//! The carriers a render takes in besides the per-slot data
//! (`view_data::ComponentData`): the per-request context (`RenderContext`)
//! and the project's narrative files (`NarrativeStore`).
//!
//! A leaf of the component modules, pure data over `ir`, `lang` and `theme`: every renderer
//! and page assembly read them, so they sit below both. Nothing here renders
//! or touches SQL.

use std::collections::{BTreeMap, HashMap};

use cpe_core::ir::Resource;
use cpe_core::lang::Lang;
use cpe_core::theme::Appearance;

/// Context threaded through the renderer for a single HTTP request.
///
/// `route_params` is carried empty this slice; it is forward-shaped for
/// param-page wiring (task two) without requiring a signature change then.
pub struct RenderContext<'a> {
    pub lang: Lang,
    pub project_id: &'a str,
    /// URI path used to build language-switcher and appearance-toggle hrefs.
    /// Excludes the query string — both switchers reconstruct the query via
    /// `resource_list::override_query_param(current_query, …)` rather than
    /// dropping it, so the current request's other params (facets, sort, …)
    /// survive the switch.
    pub current_path: &'a str,
    /// Raw query string for the current request (everything after `?`, no leading `?`).
    ///
    /// Used by the carousel renderer to build prev/next hrefs that preserve all
    /// existing params (e.g. `lang`, `appearance`) while overriding only `img`.
    /// `None` when the request had no query string.
    pub current_query: Option<&'a str>,
    /// Route parameters extracted by the router (empty this slice).
    pub route_params: &'a [(String, String)],
    /// Resolved colour scheme for this request (default [`Appearance::Light`]).
    ///
    /// `Dark` emits `data-appearance="dark"` on `<html>`; `Light` emits no
    /// attribute (it is the CSS baseline). CPE does not follow the OS
    /// `prefers-color-scheme` — the header toggle is the only switch.
    pub appearance: Appearance,
    /// Selected image index for the carousel (from `?img=<index>` query param).
    ///
    /// Parsed and clamped by the handler; defaults to 0. The renderer uses this
    /// to pick which representation to show. Out-of-range values are clamped to
    /// the last valid index by the renderer.
    pub selected_image: usize,
    /// Class-to-route map for turning `Link` values into clickable hrefs.
    ///
    /// Borrowed from `Project::class_routes()`. The renderer builds a
    /// `LinkResolver` from this map and `project_id` for each `resource-detail`
    /// component. Pass an empty map (or `&HashMap::new()`) when the caller has
    /// no project context (e.g. engine unit tests that only test narrative pages).
    pub link_routes: &'a HashMap<String, String>,
    /// Class-to-media-property map (embedded-media plan step 5), threaded the
    /// same way as `link_routes`.
    ///
    /// Borrowed from `Project::media_properties()`. Consumed at the
    /// `resource-detail` and compound-member property-block call sites to
    /// suppress the media link property from plain property-row rendering
    /// (the record viewer renders it separately) — see `component::media`.
    pub media_properties: &'a HashMap<String, String>,
    /// Pre-hydrated Image resources linked from the current page's single
    /// `resource-detail` resource via its class's media property (embedded-media
    /// plan step 6c). Empty when the resource's class has no media property, or
    /// when the property has no values. The store fetches it
    /// (`PageData.media_items`, through `ProjectReads::media_items`) and the
    /// bin passes it in — renderers have no DB access of their own. A page has at most one
    /// `resource-detail` (boot-enforced `DuplicateSingletonComponent`), so one slice suffices —
    /// no per-component indexing is needed.
    pub media_items: &'a [Resource],
    /// Fold state of the record viewer's thumb strip (`?strip=` query param,
    /// default open/`true`). The record viewer's own strip fold is a
    /// client-side `$stripOpen` signal (never a URL write, `dsp-incubator
    /// cpe/docs/adr/0007-fragment-swaps-are-transient.md`); `?strip=` stays only the
    /// initial-load / zero-JS full-reload condition (the retired `_plate` fragment endpoint
    /// this comment used to describe is gone — record-viewer plan step 12).
    pub strip_open: bool,
    /// Id → thumb (raw representation-URI + stored width) map for the page's
    /// `resource-list` browse-row thumbnails (embedded-media plan step 6d),
    /// threaded the same way as `media_items`.
    ///
    /// The store fetches it (`PageData.media_thumbs`, through
    /// `ProjectReads::media_thumbs(class_media_property, &ids)`) after the
    /// list's own read (`ids` = the resolved page's item ids), and the bin
    /// passes it in — renderers have no DB access of their own. Empty
    /// when the page has no `resource-list`, its `media` attribute is absent, or
    /// no row resolved a thumbnail. A page has at most one `resource-list`
    /// (boot-enforced `DuplicateSingletonComponent`), so one map suffices — no
    /// per-component indexing is needed. `render_list_component` reads this and
    /// threads it into `ResourceListCfg.media_thumbs`. The `width` field feeds
    /// the IIIF request-size clamp — Sipi (IIIF v3) 400s a size request wider
    /// than the image's own native width.
    pub media_thumbs: &'a HashMap<String, cpe_core::ir::MediaThumb>,
    /// Lightweight member headers for the page's (at most one, boot-enforced
    /// `DuplicateSingletonComponent`) `resource-detail` resource, when that
    /// resource is itself a compound parent (record-viewer plan step 9 —
    /// `.rv-contents`, AC11). Empty when the resource has no `compound_parts`
    /// rows of its own (the common case) or the page has no resource-detail at
    /// all. The store fetches it (`PageData.compound_members`, through
    /// `ProjectReads::compound_member_headers`) and the bin passes it in —
    /// renderers have no DB access of their own. Threaded the same way as
    /// `media_items`.
    pub compound_members: &'a [cpe_core::ir::MemberHeader],
    /// Compound-parent Resource lookup, keyed by the compound's resolved
    /// source id (empty-states-not-found plan step 7, E5).
    ///
    /// Borrowed like `link_routes`/`media_properties` (never owned — an owned
    /// map would force clones at every literal). The store
    /// (`PageData.compound_parents`) populates an entry ONLY when a
    /// `CompoundViewer` component resolved to `total == 0`, so this map is
    /// empty for every page with no empty compound. `render_component`'s
    /// `CompoundViewer` arm re-resolves the same source id (literal id or
    /// `$param` via `route_params`) the store's page builder
    /// (`query::page`) used, and looks it up here — `CompoundPage` itself carries no source id.
    /// `None` here (key absent) on the empty branch means the parent's source id doesn't
    /// resolve to a Resource at all; the renderer degrades to the sentence
    /// alone rather than panicking.
    pub compound_parents: &'a HashMap<String, Resource>,
    /// Class-to-list-route map (N3, empty-states-not-found plan step 8),
    /// threaded the same way as `link_routes`. Unused by any renderer in
    /// step 7 (`#[allow(dead_code)]` on the placeholder value at every bin
    /// construction site until step 8 wires `render_not_found`/the
    /// breadcrumb to it) — added here alongside `compound_parents` so both
    /// new `RenderContext` fields ripple through the ~7 bin literals and the
    /// engine test fixtures in one coordinated pass (see the plan's step 7
    /// "Touch-point warning").
    pub class_list_routes: &'a HashMap<String, String>,
    /// This REQUEST's resolved public origin (share-view plan step 1;
    /// **revised 2026-09-01**, owner verification round): `CPE_PUBLIC_ORIGIN`
    /// when set (env override, unconditional), else derived fresh for THIS
    /// request from `X-Forwarded-Proto`/`Host` — no longer a single value
    /// resolved once at bin boot and shared across every request/environment
    /// (see `bin/src/lib.rs`'s `OriginConfig` and
    /// `docs/plans/share-view/implementation-plan.md`'s decision 4). No
    /// trailing slash. Threaded the same way as `current_path`/
    /// `current_query`; passed to `share_link::compose_absolute_url` when
    /// building the "Copy link" control's href.
    pub public_origin: &'a str,
    /// The resolved incoming-links rows for the page's (at most one)
    /// `resource-detail` resource (Incunabula Slice 5, G7), when that
    /// component declares an `incoming-links` child. Threaded exactly like
    /// `compound_members`: the store fetches them (`PageData.incoming_links`,
    /// through `ProjectReads::incoming_links`) and the bin passes them in.
    ///
    /// `None` ⇒ no list renders at all — either the page declares no
    /// `incoming-links`, or the lookup failed (a query failure must never
    /// claim "nothing links here"). `Some(&IncomingLinks { rows: vec![], .. })`
    /// ⇒ the caption + the empty sentence. `IncomingLinks.total` (item 6,
    /// Backlog hardening batch 3) is the UNCAPPED match count; the renderer
    /// shows an honest "N of M" line only when `rows.len() < total`.
    pub incoming_links: Option<&'a cpe_core::ir::IncomingLinks>,
    /// The resource-level annotation-link partners of the page's (at most
    /// one) `resource-detail` resource (`ProjectReads::resource_links`;
    /// Incunabula Slice 6, D9), for a `group links-label=` row. The store
    /// fetches them (`PageData.resource_links`) only for a full-page request
    /// (`PageScope::Page`) whose record declares such a group; empty
    /// otherwise, which renders no row.
    pub resource_links: &'a [cpe_core::ir::ResourceLink],
}

/// Bare-filename → UTF-8 content store for a project's narrative files.
///
/// Built at `Project::load` time from the pairs the bin supplies (the bin
/// reads the embedded `narrative/` directory; the engine stays I/O-free).
/// Keys are bare filenames (e.g. `"home.md"`, `"home.de.md"`); values are
/// the full UTF-8 content. `BTreeMap` gives deterministic iteration order,
/// which keeps test assertions stable.
///
/// Immutable post-boot — do NOT wrap in a `Mutex` for symmetry with `Project`'s `conn`.
/// Rendering reads this concurrently from multiple handlers; the shared
/// `Arc<Project>` is the only lock boundary.
#[derive(Debug)]
pub struct NarrativeStore(BTreeMap<String, String>);

impl NarrativeStore {
    /// Build a store from an slice of `(filename, content)` pairs.
    ///
    /// Duplicate filenames keep the last occurrence (BTreeMap insert
    /// semantics); the bin deduplicates at the source so this is a
    /// belt-and-suspenders guard.
    pub fn from_pairs(pairs: &[(String, String)]) -> Self {
        let map = pairs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Self(map)
    }

    /// Return the content for a bare filename, or `None` if absent.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}
