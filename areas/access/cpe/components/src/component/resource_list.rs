//! Renderer for the `resource-list` component (surf-head + annotated rows).
//!
//! Takes a `ResourceListPage` and renders a `<ul class="rl-annot">` with
//! one `<li>` per resource row, plus an optional `<nav class="pager">` for
//! multi-page PerPage pagination.
//!
//! ## Surf-head
//! A `.surf-head` sits above the list — in both the populated AND the empty
//! branch (without it the `span.surf-head__count` live region leaves the DOM on a
//! facet-narrowed-to-zero morph, and the "0 results" state is never
//! announced): class label + result count ("N `<class plural>`", or "N of M
//! `<class plural>`" when
//! `list.unfiltered_total` is `Some(m)` and `m != list.total`) on the left,
//! the shared ordering control (`render_order_control`) on the right. The
//! class label is resolved from `cfg.class` (the IR `ResourceList.class`) —
//! independent of `list.items` — so
//! `span.surf-head__title` is populated in the empty branch too. When a
//! search scope note renders, a `span.surf-head__sep[aria-hidden] "·"`
//! separates it from the count.
//!
//! ## Row structure
//! Each row is text-first in markup (CSS shows the thumbnail left of the text
//! on desktop, right of the gloss on phones):
//! - A name resolved via `resolve_title_str(resource, Some("name"), lang)`.
//! - An optional thumbnail, looked up in `cfg.media_thumbs` by the resource's id (a safe-URI
//!   id→raw-URI map the store builds, `PageData.media_thumbs`, after the list's own read). No
//!   per-row eyebrow — the class label is in the surf-head.
//!
//! When the resource's class is bound in `cfg.links` the row is wrapped in
//! `<a href="…">` (accessible name = the name text). Unbound class → plain
//! `<div>` row, no link.
//!
//! ## Ordering control
//! `render_order_control` (private) renders a `<form method="get">`
//! (`.orderctl`) + one hidden `<input>` per preserved query pair (everything
//! except `sort`/`page`) + a `<details id="orderctl">` disclosure (summary +
//! four `<button type="submit" name="sort">` rows: As curated / Name A–Z /
//! Name Z–A / Identifier) + a dedicated `aria-live` status span outside the
//! `<details>`. When `cfg.list_id` is `Some`, the `<form>` carries
//! `data-on:submit__prevent`: a submitter-initiated submit closes the panel,
//! refocuses the summary, and fires a DataStar `@get` against the `_sort`
//! fragment endpoint (`SORT_FRAGMENT_SEGMENT`) with the clicked row's value;
//! a submitter-less submit falls back to `el.submit()` (a never-a-dead-
//! control floor — see the renderer's doc comment). When `cfg.list_id` is
//! `None`, no `data-on:submit__prevent` is emitted and the form is a plain
//! GET. The `<details>` always renders without `open` (server-closed on
//! every render).
//!
//! ## Pager
//! `render_browse_pager` (a different markup contract from the `.pg` primitive's).
//! Rendered
//! only when `list.pagination == Pagination::PerPage(n)` AND `last > 1` (more
//! than one page exists). The nav `aria-label` always carries the position
//! ("Page navigation, Page 2 of 3"), so position is conveyed once even when
//! the middle is hidden at narrow widths. Ends are real `<a href>` (words
//! *Previous*/*Next*) via `build_pager_href`, or an inert
//! `<span aria-disabled="true">` (no `aria-hidden` — the word stays readable)
//! at the first/last page. Middle: `div.pager__nums` of `.pager__num` links
//! (one `aria-current="page"` self-link) when `last <= PAGER_NUMBERED_MAX`;
//! otherwise a single `span.pager__of` "Page n of N". `span.pager__of` is
//! ALWAYS emitted — at `last <= PAGER_NUMBERED_MAX` it follows `.pager__nums`
//! with `aria-hidden="true"` (redundant with the nav label, hidden above
//! 640px by CSS not written here); at `last > PAGER_NUMBERED_MAX` it has no
//! `aria-hidden` (it's the only position indicator).
use std::borrow::Cow;
use std::collections::HashMap;

use cpe_core::iiif::{iiif_grid_tile_url_clamped, iiif_tall_tile_url, iiif_thumb_url_clamped};
use cpe_core::ir::{ChipKind, ChipSpec, MediaThumb, Pagination, PropertyValue, Resource, ResourceListPage, TileRatio};
use cpe_core::labels::{OrderRow, ResolvedGloss, ResolvedGlossPart};
use cpe_core::lang::Lang;
use cpe_core::util::{encode_get_arg, is_safe_external_url};
use maud::{html, Markup};

use crate::chrome::{count_of_text, count_resources_text, pager_page_n, pager_page_of, ChromeMsg};
use crate::component::links::LinkResolver;
use crate::component::values::{
    composed_part_texts, format_date_text, pick_values, resolve_title_str, split_keyword_label,
};

/// The URL path segment that identifies the resource-list's sort fragment
/// endpoint.
///
/// The fragment endpoint lives at `{page-path}/{SORT_FRAGMENT_SEGMENT}?…`.
/// Both the engine (to build the `data-on:submit` `@get` URL) and `dsp-incubator cpe/bin` (to
/// register the Axum route) reference this constant so no magic-string
/// mismatch is possible. Mirrors `FACET_FRAGMENT_SEGMENT` /
/// `SEARCH_FRAGMENT_SEGMENT`.
pub const SORT_FRAGMENT_SEGMENT: &str = "_sort";

// ── View mode ─────────────────────────────────────

/// The resource-list's current presentation: rows (`List`, the default) or
/// tiles (`Grid`). `pub(crate)` — nothing outside `cpe-components` names this type;
/// the host never reads `?view=` at all (see [`parse_view_param`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    List,
    Grid,
}

/// Parse the closed-vocabulary `?view=` value out of `current_query`.
///
/// Mirrors `sort_param`'s tiny reader exactly — `view` is a closed-vocabulary
/// identifier like `sort` (see the `query_pairs` Non-obvious constraint), so
/// no percent-decode point is needed. Only the literal value `"grid"` maps to
/// [`ViewMode::Grid`]; absent, empty, or any unrecognized value maps to
/// [`ViewMode::List`] — the same posture as an unknown `?sort=` value.
pub(crate) fn parse_view_param(current_query: Option<&str>) -> ViewMode {
    let Some(q) = current_query else {
        return ViewMode::List;
    };
    for pair in q.split('&') {
        let (k, v) = match pair.find('=') {
            Some(idx) => (&pair[..idx], &pair[idx + 1..]),
            None => (pair, ""),
        };
        if k == "view" {
            return if v == "grid" { ViewMode::Grid } else { ViewMode::List };
        }
    }
    ViewMode::List
}

/// Build a query string with `key` dropped entirely, preserving all other
/// params. Unlike [`override_query_param`] (set-or-append only), this
/// removes the key when present; mirrors `facet_panel::clear_group_query`'s
/// filter shape but scoped to a single named key (not a `page` + property
/// pair).
///
/// Returns `None` when the resulting query is empty — callers should then
/// emit the bare `current_path` with no trailing `?` (mirrors
/// `facet_panel::toggle_href`'s `qs.is_empty()` guard).
#[must_use]
pub(crate) fn drop_query_param(current_query: Option<&str>, key: &str) -> Option<String> {
    let q = current_query.unwrap_or("");
    let kept: Vec<&str> = q
        .split('&')
        .filter(|pair| {
            if pair.is_empty() {
                return false;
            }
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            k != key
        })
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept.join("&"))
    }
}

// ── Search-aware rendering surface ──────────────────

/// The search-aware rendering surface for a `resource-list` whose sibling
/// `search-bar` has an **active** query (≥1 usable term after tokenization —
/// [`cpe_core::util::is_active_search_query`], never raw non-emptiness).
///
/// **Populated-list scope note ONLY** (the empty-branch
/// message/hint/clear-actions are carried by
/// [`NarrowedSurface`], whose activity gate is
/// `query active || filters active` instead of query-only). Constructing a
/// `SearchSurface` is only possible via [`SearchSurface::for_query`], whose
/// smart constructor enforces the "active query" invariant: no call site can
/// build a `Some` with a dead query, so `ResourceListCfg.search` (and the
/// `span.search-scope` note it drives) only ever appears next to an active
/// search.
///
/// **`pub`, not `pub(crate)`** — `SearchSurface` appears in
/// `ResourceListCfg::with_links`'s (a `pub fn`) parameter list, which `dsp-incubator cpe/bin`
/// calls across the crate boundary; a `pub(crate)` type there is a hard
/// compile error ("private type in public interface"), not just a lint.
/// Its field stays `pub(crate)`, and [`SearchSurface::for_query`] is
/// `#[doc(hidden)] pub` for page assembly only, the one place the
/// active-search gate lives. So the host can only ever hold or pass an
/// `Option<SearchSurface>` opaquely: it can neither construct one with a
/// dead query nor read the field directly.
pub struct SearchSurface<'a> {
    /// The mono caption appended after the surf-head count:
    /// `"matching name and description"` / `"in Name und Beschreibung"`.
    /// A project-declared `chrome { string }` id referenced by `search-bar
    /// scope="…"` (mirrors `placeholder`'s "id, not literal" convention —
    /// resolved by the caller via `chrome_lookup`, never composed from
    /// `searches`/property names by the engine). Empty when undeclared or
    /// unresolvable, in which case `render_surf_head` omits the whole span.
    pub(crate) scope_note: &'a str,
}

impl<'a> SearchSurface<'a> {
    /// Smart constructor: returns `None` when `q` is inactive (the ONLY gate
    /// this type needs — `scope_note` is caller-precomputed, so a call site
    /// never has to duplicate the activity check).
    #[doc(hidden)]
    pub fn for_query(q: &str, scope_note: &'a str) -> Option<Self> {
        if !cpe_core::util::is_active_search_query(q) {
            return None;
        }
        Some(Self { scope_note })
    }
}

// ── Narrowed-to-zero rendering surface ──

/// The narrowed-to-zero rendering surface for a `resource-list` empty branch
/// (an active search query and/or active facet narrowing produced zero
/// results), a generalization of the search-only
/// `SearchSurface` above. Distinct from it in one crucial way: `SearchSurface`
/// exists to carry the POPULATED-list scope note and is gated on an active
/// search query ALONE (unchanged, still `ResourceListCfg.search`);
/// `NarrowedSurface` is gated on `query active || filters active`, so it can
/// exist — and drive the empty-branch body — on a facet-only page with zero
/// matches and NO search-bar at all (`ResourceListCfg.narrowed`, a NEW,
/// separate field; see that field's doc comment for why `cfg.search` alone
/// cannot carry this).
///
/// Every field is caller-precomputed, exactly like `SearchSurface` — this
/// type carries no resolution logic beyond the activity gate in
/// [`NarrowedSurface::build`].
///
/// **`pub`, not `pub(crate)`** — same rationale as `SearchSurface`: it
/// appears in `ResourceListCfg::with_links`'s (a `pub fn`) parameter list.
pub struct NarrowedSurface<'a> {
    /// The `.search-empty__msg` no-match message (query-only / filters-only /
    /// both, via `chrome::narrowed_to_zero_text`) — safe to interpolate
    /// verbatim (auto-escaped internally, `<em>`-wrapped query included).
    pub(crate) no_match_msg: Markup,
    /// The `.search-empty__hint` mono hint line — `Some` iff a search query
    /// is active (a filters-only narrowing has no hint: the hint is about
    /// prefix-matching search behaviour, not facets).
    pub(crate) no_match_hint: Option<&'a str>,
    /// Href for the "Clear search" action — `Some` iff a search query is
    /// active.
    pub(crate) clear_search_href: Option<&'a str>,
    /// Href for the "Clear all filters" action — `Some` iff at least one
    /// facet option is currently selected ([`cpe_core::ir::any_facet_selected`]),
    /// never merely "a facet-panel is configured on this page".
    pub(crate) clear_all_href: Option<&'a str>,
}

impl<'a> NarrowedSurface<'a> {
    /// Smart constructor: returns `None` when NEITHER a search query nor a
    /// facet is active (the empty-class case has its own message and
    /// never reaches this type). `clear_all_href.is_some()` IS "filters
    /// active" — the caller (`SearchFragmentSurface::narrowed_surface`)
    /// already derives it from `any_facet_selected`, so this constructor
    /// doesn't take a separate bool and re-derive it.
    #[doc(hidden)]
    pub fn build(
        query_active: bool,
        no_match_msg: Markup,
        no_match_hint: &'a str,
        clear_search_href: &'a str,
        clear_all_href: Option<&'a str>,
    ) -> Option<Self> {
        if !query_active && clear_all_href.is_none() {
            return None;
        }
        Some(Self {
            no_match_msg,
            no_match_hint: query_active.then_some(no_match_hint),
            clear_search_href: query_active.then_some(clear_search_href),
            clear_all_href,
        })
    }
}

// ── Config borrow struct ──────────────────────────────────────────────────────

/// Borrowed context needed by `render_resource_list`.
///
/// Mirrors `ResourceDetailCfg` in its shape: thin borrow struct, no IR
/// cloning. Built by `render_component` in `dsp-incubator cpe/web`'s `page` module from the
/// `RenderContext` fields.
pub struct ResourceListCfg<'a> {
    /// Optional link resolver for turning resource class → detail href.
    ///
    /// `None` → all rows render without a link (the pre-wiring state).
    /// `Some(resolver)` → rows whose class is bound in the map become
    /// clickable; unbound classes still render as plain text rows.
    #[doc(hidden)]
    pub links: Option<LinkResolver<'a>>,
    /// URI path of the current request (e.g. `/alice-in-daschland/characters`).
    /// Used to build pager prev/next hrefs.
    #[doc(hidden)]
    pub current_path: &'a str,
    /// Raw query string of the current request (everything after `?`, no `?`).
    /// Used to build pager hrefs that preserve lang/appearance params while
    /// overriding only `page`. `None` when the request had no query string.
    #[doc(hidden)]
    pub current_query: Option<&'a str>,
    /// Optional id for the list section element (`id="<list-id>"`).
    ///
    /// When `Some`, emitted as `id=` on the `section.resource-list-section.page`
    /// root so the element can be a DataStar patch selector
    /// (`#<list-id>`) in the `_facets` fragment endpoint. `None` for lists
    /// without a configured id (no facet panel binds to them).
    #[doc(hidden)]
    pub list_id: Option<&'a str>,
    /// The row gloss (`p.rl-annot__gloss`), already resolved for the request
    /// language by [`cpe_core::labels::resolve_gloss`]. `None` ⇒ no gloss line. From the
    /// IR `ResourceList.gloss`.
    #[doc(hidden)]
    pub gloss: Option<&'a ResolvedGloss<'a>>,
    /// Row chips (`.rl-annot__meta`), each with an optional shape override:
    /// an absent `kind` derives the shape from cardinality — a
    /// single-valued list value → `.term`, multi-valued → `.keyword` (same
    /// single-vs-multi split as `resource-detail`); an explicit `kind`
    /// overrides that for every label of that property. Empty ⇒ no chips.
    /// From the IR `ResourceList.chips`.
    #[doc(hidden)]
    pub chips: &'a [cpe_core::ir::ChipSpec],
    /// The raw (unresolved) `Class` name this list renders — from the IR
    /// `ResourceList.class` — used by `render_surf_head` to resolve the
    /// eyebrow class label **independent of `list.items`**:
    /// `list.items.first()` would leave the eyebrow blank on every empty
    /// branch (nothing to read a class from).
    #[doc(hidden)]
    pub class: &'a str,
    /// Declared class-label translations, used to resolve the surf-head class
    /// title (`render_surf_head`): the declared translation for the current
    /// request language when a `labels { class "…" { … } }` entry exists and
    /// has one, else the raw/humanized class name (`humanize_property_name`
    /// fallback) — unchanged behaviour for projects that declare no labels.
    #[doc(hidden)]
    pub class_labels: &'a [cpe_core::ir::ClassLabel],
    /// The search-aware rendering surface: `Some`
    /// iff a sibling `search-bar`'s query is active (see
    /// [`SearchSurface::for_query`]). `None` ⇒ no scope note is shown next to
    /// the count. Note: this field does not decide the empty-branch body —
    /// see [`Self::narrowed`] — it exists purely for the populated-list scope
    /// note.
    #[doc(hidden)]
    pub search: Option<SearchSurface<'a>>,
    /// The narrowed-to-zero rendering surface:
    /// `Some` iff an active search query and/or an active facet
    /// narrowing is in effect (see [`NarrowedSurface::build`]), so
    /// a facet-only page with NO search-bar at
    /// all still shows the narrowed-to-zero body rather than degrading to the
    /// genuinely-empty-class message. `None` ⇒ a zero-item list is a
    /// genuinely empty class (`p.resource-list__empty` +
    /// `ChromeMsg::ListEmptyClass`).
    #[doc(hidden)]
    pub narrowed: Option<NarrowedSurface<'a>>,
    /// Id → raw representation-URI map for the browse-row thumbnail.
    /// Built by the caller via
    /// `ProjectReads::media_thumbs(class_media_property, &ids)` after
    /// `list_resources`, keyed by the SAME `list.items[i].id` values this cfg
    /// renders. An id absent from the map (no media property configured, or
    /// the resource has no media values) renders no thumbnail — never a
    /// placeholder. `render_row` looks the raw URI up here and calls
    /// `iiif_thumb_url_clamped` on it (with the entry's `width`) to get the
    /// sized `<img src>` (the `representations` field is always empty for non-Image classes). The
    /// width clamp avoids requesting a IIIF size wider than the target image's own
    /// native width, which Sipi (IIIF v3) 400s.
    #[doc(hidden)]
    pub media_thumbs: &'a HashMap<String, MediaThumb>,
    /// Grid-mode opt-in, from the IR `ResourceList.grid`.
    /// `false` ⇒ the view toggle never renders and `?view=grid` is silently
    /// ignored (renders rows) — a `grid: false` list can never render tiles,
    /// even with `view=grid` in the query. `true` ⇒ the toggle renders and
    /// `render_resource_list` derives the active `ViewMode` from
    /// `current_query` via `parse_view_param`.
    #[doc(hidden)]
    pub grid: bool,
    /// The grid tile's aspect ratio, from the IR `ResourceList.tile_ratio`.
    /// `None` ⇒ the stylesheet's default (3:4). A fragment handler must pass
    /// the same value as the full page, or a swap returns a grid without it.
    #[doc(hidden)]
    pub tile_ratio: Option<TileRatio>,
    /// Resolved chrome-**id** override for the "Name A–Z" ordering label
    /// — id only, never literal text — already run through
    /// `chrome_lookup` by the caller.
    /// `None` ⇒ use the `ChromeMsg::OrderNameAsc` default — never an empty
    /// label.
    #[doc(hidden)]
    pub order_name_label: Option<&'a str>,
    /// Resolved chrome-**id** override for the "Name Z–A" ordering label
    /// — id only, never literal text — already run through
    /// `chrome_lookup` by the caller.
    /// `None` ⇒ use the `ChromeMsg::OrderNameDesc` default — never an empty
    /// label.
    #[doc(hidden)]
    pub order_name_desc_label: Option<&'a str>,
    /// Resolved `order-default-label` override for the "As curated" row —
    /// already run through `chrome_lookup` by the caller. `None` ⇒ use the
    /// `ChromeMsg::OrderCurated` default — never an empty label.
    #[doc(hidden)]
    pub order_default_label: Option<&'a str>,
    /// The list's declared `order-by` options as
    /// already-resolved `(key, label)` rows, in config order — built by
    /// [`cpe_core::labels::resolve_order_options`], never raw chrome ids. Rendered after
    /// the four built-in rows. Empty ⇒ the panel is exactly the four
    /// built-ins.
    #[doc(hidden)]
    pub order_options: &'a [OrderRow<'a>],
}

impl<'a> ResourceListCfg<'a> {
    /// Construct a `ResourceListCfg` with a `LinkResolver` built from the provided
    /// class-to-route map. `dsp-incubator cpe/bin` calls this (in the `_facets` fragment handler)
    /// so it never needs to name `LinkResolver` directly (which is `#[doc(hidden)]` and
    /// not part of the intended public surface).
    ///
    /// Pass `link_routes = &HashMap::new()` for tests or contexts without route
    /// bindings; the resolver will return `None` for every class, causing rows to fall
    /// back to plain-text (no link).
    #[allow(clippy::too_many_arguments)]
    pub fn with_links(
        project_id: &'a str,
        link_routes: &'a std::collections::HashMap<String, String>,
        current_path: &'a str,
        current_query: Option<&'a str>,
        list_id: Option<&'a str>,
        gloss: Option<&'a ResolvedGloss<'a>>,
        chips: &'a [cpe_core::ir::ChipSpec],
        class: &'a str,
        class_labels: &'a [cpe_core::ir::ClassLabel],
        search: Option<SearchSurface<'a>>,
        narrowed: Option<NarrowedSurface<'a>>,
        media_thumbs: &'a HashMap<String, MediaThumb>,
        grid: bool,
        tile_ratio: Option<TileRatio>,
        order_name_label: Option<&'a str>,
        order_name_desc_label: Option<&'a str>,
        order_default_label: Option<&'a str>,
        order_options: &'a [OrderRow<'a>],
    ) -> Self {
        Self {
            links: Some(LinkResolver::new(project_id, link_routes)),
            current_path,
            current_query,
            list_id,
            gloss,
            chips,
            class,
            class_labels,
            search,
            narrowed,
            media_thumbs,
            grid,
            tile_ratio,
            order_name_label,
            order_name_desc_label,
            order_default_label,
            order_options,
        }
    }
}

// ── Public renderer ───────────────────────────────────────────────────────────

/// Render a `ResourceListPage` as a `<section class="resource-list-section">`.
///
/// Structure:
/// - Populated branch: `.surf-head` (class label + result count + sort control) above `<ul
///   class="rl-annot">` with one `<li class="rl-annot__item">` per resource (link when class is
///   bound, plain div when not, optional thumbnail) + optional pager.
/// - Empty branch: `.surf-head` (same markup as the populated branch, showing the "0 `<class
///   plural>`"/"0 of M `<class plural>`" count, and a class-label eyebrow that does not depend on
///   `list.items`) above one of two bodies — **`cfg.narrowed.is_some()`** (an active search query
///   and/or an active facet narrowed the list to zero) ⇒ `.search-empty` (message + clear actions +
///   hint, per [`NarrowedSurface`]'s activity gates), else (the genuinely-empty class)
///   `p.resource-list__empty` with `ChromeMsg::ListEmptyClass` — no pager either way.
///   `list.items.is_empty()` alone cannot distinguish these two cases (both reach this branch);
///   `cfg.narrowed` is the only signal that does (NOT `cfg.search`, which is search-only and would
///   miss a facet-only narrowing on a page with no search-bar). The `.surf-head` is rendered here
///   too — not only in the populated branch — so that `span.surf-head__count` (the live region)
///   exists at the same leading position in both branches; idiomorph then preserves+updates that
///   node across a populated↔empty morph instead of removing/inserting it, which is what makes the
///   "0 results" transition announced to a screen reader (a11y fix — a live-region node inserted
///   fresh by a mutation is generally not announced).
/// - Pager (`<nav class="pager">`) shown iff `pagination == PerPage(n)` and total pages > 1.
///
/// This section is the DataStar swap target of both the `_facets` and
/// `_sort` fragment endpoints. The root itself does **not** carry
/// `aria-live` — with up to 24 rows (name + gloss + chips + pager), an
/// atomic live region on the whole section would make a screen reader
/// re-announce the entire list on every filter/sort change (WCAG
/// 4.1.3-territory verbosity; a11y review finding). Instead, only the small
/// `span.surf-head__count` result-count element carries `aria-live="polite"
/// aria-atomic="true"` (see `render_surf_head`) — no focus/URL change
/// (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`).
pub fn render_resource_list(list: &ResourceListPage, cfg: &ResourceListCfg<'_>, lang: &Lang) -> Markup {
    // ONE derivation point: `view=grid` with
    // `cfg.grid: false` can never render tiles — an ungridded class silently
    // ignores `?view=grid`, same posture as an unknown `?sort=` value. The
    // computed `ViewMode` is passed into `render_surf_head` as a parameter so
    // the toggle's `aria-current` never re-reads `current_query` itself.
    let view = if cfg.grid {
        parse_view_param(cfg.current_query)
    } else {
        ViewMode::List
    };

    if list.items.is_empty() {
        // Render `.surf-head` here too (a11y fix) — see the module doc
        // comment and this function's rustdoc "Empty branch" note. Reusing
        // `render_surf_head` keeps the count-text logic (0 resources / 0 of
        // M) and the `span.surf-head__count` live region identical to the
        // populated branch.
        let surf_head_markup = render_surf_head(list, cfg, view, lang);
        // `cfg.narrowed.is_some()` is the ONLY signal that distinguishes a
        // narrowed-to-zero list (an active search query and/or active
        // facet) from a genuinely-empty
        // class — `list.items.is_empty()` alone is true in both cases.
        let body_markup = match &cfg.narrowed {
            Some(surface) => html! {
                div.search-empty {
                    p.search-empty__msg { (surface.no_match_msg) }
                    div.search-empty__acts {
                        @if let Some(href) = surface.clear_search_href {
                            a.search-empty__act href=(href) { (ChromeMsg::SearchClear.text(lang)) }
                        }
                        @if let Some(href) = surface.clear_all_href {
                            a.search-empty__act href=(href) {
                                (ChromeMsg::EmptyClearFilters.text(lang))
                            }
                        }
                    }
                    @if let Some(hint) = surface.no_match_hint {
                        p.search-empty__hint { (hint) }
                    }
                }
            },
            None => {
                let empty_msg = ChromeMsg::ListEmptyClass.text(lang);
                html! {
                    p.resource-list__empty { (empty_msg) }
                }
            }
        };
        return html! {
            // `.page` supplies the shared centered max-width + gutter box (same
            // as `article.record.page` on detail pages); `.resource-list-section`
            // is the browse-specific hook. `id` is emitted when `list_id` is
            // Some, making this section a DataStar patch target (#<list-id>).
            section.resource-list-section.page id=[cfg.list_id] { (surf_head_markup) (body_markup) }
        };
    }

    let surf_head_markup = render_surf_head(list, cfg, view, lang);

    // Grid view renders `ul.gm` tiles instead of `ul.rl-annot` rows — same
    // resources, same order, just a different per-item markup shape. The
    // pager renders after either body exactly the same way (no early return
    // from the grid branch).
    let body_markup = match view {
        // `data-tile="tall"` selects grid.css's tall-tile rules.
        ViewMode::Grid => html! {
            ul  .gm
                data-tile=[cfg.tile_ratio.is_some_and(TileRatio::is_tall).then_some("tall")]
                style=[cfg.tile_ratio.map(|r| format!("--tile-ratio: {r}"))]
            {
                @for resource in &list.items { (render_grid_tile(resource, cfg, lang)) }
            }
        },
        ViewMode::List => html! {
            ul.rl-annot {
                @for resource in &list.items { (render_row(resource, cfg, lang)) }
            }
        },
    };

    let pager_markup = render_browse_pager(list, cfg, lang);

    html! {
        // `id` is emitted when `list_id` is Some, making this section a
        // DataStar patch target for the `_facets`/`_sort` fragment endpoints.
        section.resource-list-section.page id=[cfg.list_id] {
            (surf_head_markup)
            (body_markup)
            (pager_markup)
        }
    }
}

// ── Surf-head (result count + sort control) ──────────────────────────────────

/// Render the `.surf-head` bar above the list: class label + result count on
/// the left, the sort control on the right. Called from both the populated
/// and the empty branch of `render_resource_list` (a11y fix — see that
/// function's rustdoc).
///
/// The class label is resolved from `cfg.class` (the IR `ResourceList.class`
/// — always resolvable, independent of `list.items`)
/// via `resolve_class_label` — the declared translation for
/// the current request language when a `labels { class "…" { … } }` entry
/// exists and has one, else the raw/humanized class name. Reading
/// `list.items.first()` instead would leave `span.surf-head__title` blank on
/// EVERY empty branch (nothing to read a class from),
/// and the genuinely-empty-class message needs a
/// titled head to read as "this Class has no resources", not "‹blank› has no
/// resources". The count text is "N <class plural>" (e.g. "8’942
/// Aufführungen" — the class's own declared noun, grouped by
/// [`crate::chrome::group_thousands`]) or, when `list.unfiltered_total` is
/// `Some(m)` with `m != list.total` (a facet panel has narrowed the list), the
/// localized "N of M <class plural>" form.
///
/// `span.surf-head__count` carries `id="browse-count"` (the search input's
/// `aria-describedby` target while a query is active)
/// plus `aria-live="polite" aria-atomic="true"`
/// — this small element, not the whole
/// `resource-list-section` root, is the live region a screen reader
/// announces after a `_facets`/`_sort` morph. It is preserved as the same DOM
/// node across the morph (both fragment endpoints patch the section whose
/// `id` is unchanged, and DataStar's morph keeps matching nodes in place —
/// same mechanism already proven by `figure__count`/`compound-count`), so the
/// text-content change inside a persisting live-region node is exactly what
/// triggers an SR announcement — no extra JS/focus/URL machinery needed.
///
/// When `cfg.search` is `Some` (an active search query) AND its `scope_note`
/// is non-empty, a `span.search-scope` mono caption is appended right after
/// the count (the scope note — e.g. `"3 of 20 · matching name
/// and description"`), rendering the caller-precomputed
/// `SearchSurface::scope_note` verbatim (Maud-escaped, never `PreEscaped`).
/// `scope_note` is the project-declared `search-bar scope="…"` chrome-string
/// id, resolved like `placeholder` — an undeclared/unresolvable id yields an
/// empty string, in which case the whole span is omitted (never an empty
/// `<span class="search-scope">` with a dangling "· " bullet from the CSS
/// `::before`).
fn render_surf_head(list: &ResourceListPage, cfg: &ResourceListCfg<'_>, view: ViewMode, lang: &Lang) -> Markup {
    let class_label = cpe_core::labels::resolve_class_label(cfg.class_labels, cfg.class, lang);
    // The count line's noun is the CLASS noun, never a generic engine noun
    // ("8’942 Aufführungen", not "8942 Einträge") — singular from
    // `resolve_class_label` (same value as the head title above), plural from
    // `resolve_class_plural`, both already available from `cfg`. The browse
    // breadcrumb's accessible name (`dsp-incubator cpe/web`'s `page` module) resolves the SAME two
    // nouns through the SAME split; keep the two in lockstep.
    let class_plural = cpe_core::labels::resolve_class_plural(cfg.class_labels, cfg.class, lang);
    let count_text = match list.unfiltered_total {
        Some(m) if m != list.total => count_of_text(
            list.total,
            m,
            &class_plural,
            cpe_core::labels::class_plural_declared(cfg.class_labels, cfg.class),
            lang,
        ),
        _ => count_resources_text(list.total, &class_label, &class_plural, lang),
    };
    let current_sort = sort_param(cfg.current_query);

    html! {
        div.surf-head {
            div.surf-head__group {
                span.surf-head__title { (class_label) }
                span.surf-head__count id="browse-count" aria-live="polite" aria-atomic="true" {
                    (count_text)
                }
                @if let Some(surface) = &cfg.search {
                    @if !surface.scope_note.is_empty() {
                        span.surf-head__sep aria-hidden="true" { "\u{00B7}" }
                        span.search-scope { (surface.scope_note) }
                    }
                }
            }
            div.surf-head__ctl {
                @if cfg.grid { (render_view_toggle(cfg, view, lang)) }
                (render_order_control(cfg, current_sort, lang))
            }
        }
    }
}

/// Render the List↔Grid view toggle. Only called
/// when `cfg.grid` is `true` — an ungridded class never renders this markup
/// at all (absent, not disabled).
///
/// `role="group"` + `aria-labelledby` points at the visible `span.viewtoggle__lbl`
/// (fixed `id="viewtoggle-lbl"` — safe because `DuplicateSingletonComponent`
/// already hard-fails a second `resource-list` on one page). The List option's
/// href drops `view` entirely via [`drop_query_param`]; the Grid option's href
/// sets `view=grid` via [`override_query_param`]. Both hrefs preserve every
/// other query param (facets, `q`, `sort`, `page`) — the view is a
/// presentation of the same narrowed window. The option matching `view`
/// carries `aria-current="true"`.
fn render_view_toggle(cfg: &ResourceListCfg<'_>, view: ViewMode, lang: &Lang) -> Markup {
    let list_href = match drop_query_param(cfg.current_query, "view") {
        Some(qs) => format!("{}?{}", cfg.current_path, qs),
        None => cfg.current_path.to_owned(),
    };
    let grid_qs = override_query_param(cfg.current_query, "view", "grid");
    let grid_href = format!("{}?{}", cfg.current_path, grid_qs);

    html! {
        div.viewtoggle {
            span.viewtoggle__lbl id="viewtoggle-lbl" { (ChromeMsg::ViewLabel.text(lang)) }
            div.viewtoggle__opts role="group" aria-labelledby="viewtoggle-lbl" {
                a   .viewtoggle__opt
                    href=(list_href)
                    aria-current=[(view == ViewMode::List).then_some("true")]
                { (ChromeMsg::ViewOptionList.text(lang)) }
                a   .viewtoggle__opt
                    href=(grid_href)
                    aria-current=[(view == ViewMode::Grid).then_some("true")]
                { (ChromeMsg::ViewOptionGrid.text(lang)) }
            }
        }
    }
}

/// Extract the raw `?sort=` value from `current_query`, if present with a
/// non-empty value.
///
/// Mirrors `query_param_one` in `dsp-incubator cpe/bin/src/router.rs`, which is private to
/// that file; this crate cannot depend on the host crate.
fn sort_param(current_query: Option<&str>) -> Option<&str> {
    let q = current_query?;
    for pair in q.split('&') {
        let (k, v) = match pair.find('=') {
            Some(idx) => (&pair[..idx], &pair[idx + 1..]),
            None => (pair, ""),
        };
        if k == "sort" && !v.is_empty() {
            return Some(v);
        }
    }
    None
}

// ── Row rendering ─────────────────────────────────────────────────────────────

/// Render one resource as a `<li class="rl-annot__item">`.
///
/// Row inner content ("annotated rows"): the text-first
/// `div.rl-annot__main` carries the name (no per-row eyebrow — the class label
/// is in the surf-head), an optional one-line `p.rl-annot__gloss` (from
/// `cfg.gloss`), and an optional `div.rl-annot__meta` chip row (from
/// `cfg.chips`). An optional thumbnail follows in markup; CSS shows it left of
/// the text on desktop (`order: -1`) and right of the gloss on phones. A record
/// with no image renders no thumbnail (never a placeholder).
/// When the class has a bound route, the inner content is wrapped in `<a>`;
/// otherwise it's a plain `<div>` (no link).
///
/// `.rl-annot__main` is a `<div>` (not `<span>`) so it can legally contain the
/// block-level `p.rl-annot__gloss`.
fn render_row(resource: &Resource, cfg: &ResourceListCfg<'_>, lang: &Lang) -> Markup {
    let name = resolve_title_str(resource, Some("name"), lang);
    let gloss = row_gloss(resource, cfg.gloss, lang);
    let chips_markup = render_row_chips(resource, cfg.chips, lang);
    let thumb = cfg.media_thumbs.get(&resource.id).filter(|t| is_safe_external_url(&t.uri));

    let inner = html! {
        div.rl-annot__main {
            div.rl-annot__name { (name) }
            @if let Some(g) = gloss {
                @if g.has_teaser {
                    p.rl-annot__gloss.rl-annot__gloss--teaser { (g.text) }
                } @else {
                    p.rl-annot__gloss { (g.text) }
                }
            }
            (chips_markup)
        }
        @if let Some(t) = thumb {
            img .rl-annot__thumb
                src=(iiif_thumb_url_clamped(&t.uri, t.width).as_ref())
                alt=""
                loading="lazy";
        }
    };

    let href = cfg.links.as_ref().and_then(|r| r.href(&resource.id, Some(&resource.class)));

    html! {
        li.rl-annot__item {
            @if let Some(ref url) = href {
                a.rl-annot__row href=(url) { (inner) }
            } @else {
                div.rl-annot__row { (inner) }
            }
        }
    }
}

// ── Grid tile rendering (grid-mode) ─────────────────────────────────────────────

/// Render one resource as a `<li>` grid tile:
/// `a.gm__tile` (or `div.gm__tile` non-link fallback, mirroring `render_row`'s
/// class-has-no-detail-binding degradation) wrapping `div.gm__box` (image or
/// text-tile fallback) + `div.gm__name` (the resolved title, same
/// title-resolution path rows use).
///
/// Image tile: the resource's `cfg.media_thumbs` URI, filtered through
/// `is_safe_external_url` (same as `render_row`), sized via
/// `iiif_grid_tile_url_clamped`/`GRID_TILE_WIDTH`, or via
/// `iiif_tall_tile_url` on a list whose `tile_ratio` is tall. `alt=""` is DELIBERATE (the
/// anchor already contains the name in `.gm__name`, so a named alt
/// double-announces "Alice Alice"; mirrors `render_row`'s decorative `alt=""`).
///
/// Text tile (no safe media URI): `div.gm__box.gm__box--text > div.gm__text >
/// span.k[aria-hidden="true"]` (`ChromeMsg::GridNoImage`, decorative chrome
/// identical across every text tile — hidden so the link's accessible name
/// stays gloss + name) `+ span.g[gloss]` when `cfg.gloss` is configured for
/// this resource (mirrors the row's existing gloss resolution). No chips, no
/// gloss on image tiles — gloss only appears in the text-tile fallback.
fn render_grid_tile(resource: &Resource, cfg: &ResourceListCfg<'_>, lang: &Lang) -> Markup {
    let name = resolve_title_str(resource, Some("name"), lang);
    let thumb = cfg.media_thumbs.get(&resource.id).filter(|t| is_safe_external_url(&t.uri));

    let box_markup = match thumb {
        Some(t) => {
            let src = if cfg.tile_ratio.is_some_and(TileRatio::is_tall) {
                iiif_tall_tile_url(&t.uri, t.width, t.height)
            } else {
                iiif_grid_tile_url_clamped(&t.uri, t.width)
            };
            html! {
                div.gm__box {
                    img src=(src.as_ref()) alt="" loading="lazy";
                }
            }
        }
        None => {
            let gloss = row_gloss_text(resource, cfg.gloss, lang);
            html! {
                div.gm__box.gm__box--text {
                    div.gm__text {
                        span.k aria-hidden="true" { (ChromeMsg::GridNoImage.text(lang)) }
                        @if let Some(g) = gloss {
                            span.g { (g) }
                        }
                    }
                }
            }
        }
    };

    let inner = html! {
        (box_markup)
        div.gm__name { (name) }
    };

    let href = cfg.links.as_ref().and_then(|r| r.href(&resource.id, Some(&resource.class)));

    html! {
        li {
            @if let Some(ref url) = href {
                a.gm__tile href=(url) { (inner) }
            } @else {
                div.gm__tile { (inner) }
            }
        }
    }
}

// ── Row gloss + chips (annotated rows) ────────────────────────────────────

/// A rendered row gloss line.
struct RowGloss<'a> {
    text: Cow<'a, str>,
    /// A teaser rendered, so the row gets the 3-line clamp modifier.
    has_teaser: bool,
}

/// The row gloss line for either gloss form.
fn row_gloss<'a>(resource: &'a Resource, gloss: Option<&ResolvedGloss<'_>>, lang: &Lang) -> Option<RowGloss<'a>> {
    match gloss? {
        g @ ResolvedGloss::Property(_) => {
            row_gloss_text(resource, Some(g), lang).map(|text| RowGloss { text, has_teaser: false })
        }
        ResolvedGloss::Composed { parts, teaser } => composed_gloss_text(resource, parts, *teaser, lang),
    }
}

/// A composed gloss: each part's language-picked values joined with ", ",
/// the non-empty parts joined with " · ", then the teaser's first
/// language-picked text appended after " — " (or standing alone when no part
/// rendered). `None` when nothing at all rendered. Unlike the legacy form, a
/// part renders every value, not only the first.
fn composed_gloss_text(
    resource: &Resource,
    parts: &[ResolvedGlossPart<'_>],
    teaser: Option<&str>,
    lang: &Lang,
) -> Option<RowGloss<'static>> {
    let property = |name: &str| resource.properties.iter().find(|p| p.name == name);
    let rendered = composed_part_texts(resource, parts, false, lang);
    let teaser_text = teaser.and_then(property).and_then(|p| {
        pick_values(p, lang).into_iter().find_map(|v| match v {
            PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } if !text.is_empty() => {
                Some(text.as_str())
            }
            _ => None,
        })
    });
    let mut text = rendered.join(" · ");
    if let Some(t) = teaser_text {
        if !text.is_empty() {
            text.push_str(" — ");
        }
        text.push_str(t);
    }
    (!text.is_empty()).then_some(RowGloss { text: Cow::Owned(text), has_teaser: teaser_text.is_some() })
}

/// Resolve the legacy one-line row gloss (`gloss property="…"`): the first
/// non-empty `Text`/`Richtext`/`Date` value of the property, language-picked
/// via `pick_values` (the same requested-lang → universal → any fallback the
/// detail page uses). A composed gloss yields `None` here: this is also the
/// grid text tile's gloss, which never shows a composed line.
///
/// Returns `None` when no gloss property is configured, the property is absent
/// from the resource, or it has no non-empty text value — a record with no
/// gloss simply renders no gloss line (never the resource id, unlike the
/// title's id fallback). Richtext is rendered as **plain text** (Maud-escaped);
/// the gloss is a scannable summary, not rendered Markdown. A `Date` value
/// renders through `values::format_date_text`
/// — the same plain date text the detail page shows, and never
/// the precision word (that is `span.prec` markup, detail-page only).
fn row_gloss_text<'a>(resource: &'a Resource, gloss: Option<&ResolvedGloss<'_>>, lang: &Lang) -> Option<Cow<'a, str>> {
    let prop_name = match gloss? {
        ResolvedGloss::Property(prop_name) => *prop_name,
        ResolvedGloss::Composed { .. } => return None,
    };
    let property = resource.properties.iter().find(|p| p.name == prop_name)?;
    pick_values(property, lang).into_iter().find_map(|v| match v {
        PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } if !text.is_empty() => {
            Some(Cow::Borrowed(text.as_str()))
        }
        PropertyValue::Date { .. } => Some(format_date_text(v, lang)).filter(|t| !t.is_empty()).map(Cow::Owned),
        _ => None,
    })
}

/// One resolved row chip: a single controlled term (`.term`) or one node of a
/// multi-valued list (`.keyword`).
enum Chip<'a> {
    /// A property with exactly one list value → a single `.term` pill.
    Term(&'a str),
    /// One of several list values → a `.keyword` pill (path/leaf split).
    Keyword(&'a str),
}

/// Collect the row chips from the configured chip specs, in config order.
///
/// For each chip spec, the resource's language-picked list-value labels
/// decide the shape: an explicit `kind` overrides the shape for
/// EVERY label of that property (`Term` → one `.term` per label, not "first
/// only"; `Keyword` → one `.keyword` per label). An absent `kind` derives the
/// shape from cardinality (mirroring `resource-detail`'s single-vs-multi
/// split): exactly one label → one `.term`; several → a run of `.keyword`s.
/// Non-list values and unresolved (`label: None`) list values contribute
/// nothing.
fn collect_row_chips<'a>(resource: &'a Resource, chip_specs: &[ChipSpec], lang: &Lang) -> Vec<Chip<'a>> {
    let mut chips = Vec::new();
    for spec in chip_specs {
        let Some(property) = resource.properties.iter().find(|p| p.name == spec.property) else {
            continue;
        };
        let labels: Vec<&str> = pick_values(property, lang)
            .into_iter()
            .filter_map(|v| match v {
                PropertyValue::ListValue { label: Some(l), .. } => Some(l.as_str()),
                _ => None,
            })
            .collect();
        match spec.kind {
            // NOTE: `cap_row_chips` (below) deliberately never caps `.term`
            // chips — "roles are identity — always shown". An explicit
            // `kind="term"` on a MULTI-valued property therefore emits one
            // `.term` per label, uncapped, and can overflow the single-line
            // `.rl-annot__meta` row if a resource has many values. No shipped
            // project's config does this today (every declared `kind="term"`
            // chip targets a single-valued property); if one ever does, this
            // is the fan-out that needs a cap or a different render treatment.
            Some(ChipKind::Term) => chips.extend(labels.into_iter().map(Chip::Term)),
            Some(ChipKind::Keyword) => chips.extend(labels.into_iter().map(Chip::Keyword)),
            None => match labels.len() {
                0 => {}
                1 => chips.push(Chip::Term(labels[0])),
                _ => chips.extend(labels.into_iter().map(Chip::Keyword)),
            },
        }
    }
    chips
}

/// Maximum number of `.keyword` (theme) chips shown in a row before the rest
/// collapse into a `+N` count. `.term` chips (roles) are always shown — the
/// single-value identity chip is never hidden. Tuned so a role plus this many
/// themes fits one line at typical widths; the `.rl-annot__meta` CSS also
/// nowraps + clips as a belt-and-suspenders. Bump this to show more themes.
const MAX_THEME_CHIPS: usize = 3;

/// Cap the collected chips to a single line's worth: every `.term` is kept,
/// `.keyword`s are kept up to [`MAX_THEME_CHIPS`], and the remaining keyword
/// count is returned as the `+N` overflow tally. Returns `(visible, hidden)`.
fn cap_row_chips(all: Vec<Chip<'_>>) -> (Vec<Chip<'_>>, usize) {
    let mut visible = Vec::with_capacity(all.len().min(MAX_THEME_CHIPS + 1));
    let mut themes_shown = 0usize;
    let mut hidden = 0usize;
    for chip in all {
        match chip {
            // Roles (single-value terms) are identity — always shown.
            Chip::Term(_) => visible.push(chip),
            Chip::Keyword(_) => {
                if themes_shown < MAX_THEME_CHIPS {
                    visible.push(chip);
                    themes_shown += 1;
                } else {
                    hidden += 1;
                }
            }
        }
    }
    (visible, hidden)
}

/// Render the `div.rl-annot__meta` chip row from the configured chip
/// properties, or empty markup when nothing renders (so no empty `.meta` box
/// is emitted for a record with no chip values).
///
/// The chip row is a single line: `.term` (role) chips always render, theme
/// chips are capped at [`MAX_THEME_CHIPS`], and any remainder collapses into a
/// trailing `span.rl-annot__more` "+N" so row heights never vary with chip
/// count.
fn render_row_chips(resource: &Resource, chip_specs: &[ChipSpec], lang: &Lang) -> Markup {
    let (visible, hidden) = cap_row_chips(collect_row_chips(resource, chip_specs, lang));
    if visible.is_empty() {
        return html! {};
    }
    html! {
        div.rl-annot__meta {
            @for chip in &visible {
                @match chip {
                    Chip::Term(label) => span.term { (label) }
                    Chip::Keyword(label) => (render_chip_keyword(label))
                }
            }
            @if hidden > 0 {
                span.rl-annot__more { "+" (hidden) }
            }
        }
    }
}

/// Render one multi-valued list label as a `span.keyword` (path/leaf split),
/// reusing `values::split_keyword_label`. A flat label (no separator)
/// renders as a single `.leaf` with no `.path`.
fn render_chip_keyword(label: &str) -> Markup {
    let (path_parts, leaf) = split_keyword_label(label);
    html! {
        span.keyword {
            @for part in &path_parts {
                span.path { (part) }
            }
            span.leaf { (leaf) }
        }
    }
}

// ── Ordering control ─────────────────────────────────────────────────────────

/// Normalize a raw `?sort=` value to one of the recognized display keys —
/// the three non-default built-ins, or a key the list declares via
/// `order-by` (`options`) — or `"default"` when the
/// value is `None`, `Some("default")`, or unrecognized (`class` is not recognized for DISPLAY,
/// though `Sort::parse_wire` still parses it at the wire level; a hand-typed
/// `?sort=class` renders the summary as "As curated" with no active mark —
/// documented limitation).
fn normalize_sort_key<'a>(current: Option<&'a str>, options: &[OrderRow<'_>]) -> &'a str {
    match current {
        Some("name") => "name",
        Some("name-desc") => "name-desc",
        Some("id") => "id",
        Some(k) if options.iter().any(|o| o.key == k) => k,
        _ => "default",
    }
}

/// The four built-in ordering-panel row keys, in panel order. The list's
/// declared `order-by` options render after them, in config order.
const BUILTIN_ORDER_KEYS: [&str; 4] = ["default", "name", "name-desc", "id"];

/// Resolve the visible label for a normalized ordering key
/// (`As curated` / `Name A–Z` / `Name Z–A` / `Identifier`,
/// plus a declared option's resolved label). `key` must be one of
/// [`BUILTIN_ORDER_KEYS`] or a declared option's key; any other value falls
/// back to the `default` label (mirrors `normalize_sort_key`'s own
/// fallback).
///
/// `default_override`/`name_override`/`name_desc_override` are the per-list
/// chrome-string-**id** overrides for the "As curated", "Name A–Z" and
/// "Name Z–A" rows (`order-default-label`, and the name pair) — id only, never
/// literal text — already resolved by the caller via
/// [`cpe_core::labels::resolve_order_name_labels`]. `None` (absent attribute, or an id
/// that failed to resolve) falls back to the `ChromeMsg` default — never an
/// empty label. The "Identifier" row is never overridable. `options` are the
/// list's declared rows, already resolved by [`cpe_core::labels::resolve_order_options`].
fn order_label_for_key<'a>(
    key: &str,
    lang: &Lang,
    default_override: Option<&'a str>,
    name_override: Option<&'a str>,
    name_desc_override: Option<&'a str>,
    options: &[OrderRow<'a>],
) -> std::borrow::Cow<'a, str> {
    match key {
        "name" => name_override
            .map(std::borrow::Cow::Borrowed)
            .unwrap_or_else(|| std::borrow::Cow::Borrowed(ChromeMsg::OrderNameAsc.text(lang))),
        "name-desc" => name_desc_override
            .map(std::borrow::Cow::Borrowed)
            .unwrap_or_else(|| std::borrow::Cow::Borrowed(ChromeMsg::OrderNameDesc.text(lang))),
        "id" => std::borrow::Cow::Borrowed(ChromeMsg::OrderId.text(lang)),
        _ => match options.iter().find(|o| o.key == key) {
            Some(o) => std::borrow::Cow::Borrowed(o.label),
            None => std::borrow::Cow::Borrowed(default_override.unwrap_or_else(|| ChromeMsg::OrderCurated.text(lang))),
        },
    }
}

/// Build the preserved query string for the ordering control's DataStar
/// `@get` prefix: every param from `current_query` EXCEPT `sort` and `page`
/// (a sort change always resets to page 1), joined back with `&`. Empty when
/// nothing is preserved (or `current_query` is `None`).
fn preserved_query_dropping_sort_and_page(current_query: Option<&str>) -> String {
    current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter(|pair| {
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            k != "sort" && k != "page"
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Render the ordering control: a
/// `<form method="get">` (`.orderctl`) with hidden inputs preserving the
/// current query (minus `sort`/`page`), a `<details id="orderctl">`
/// disclosure holding the four ordering rows as submit buttons, and a
/// dedicated `aria-live` status span announcing the current ordering.
///
/// `current_sort` is the raw `?sort=` value for this request (`None`/absent/
/// unrecognized/`class` → the "As curated" row, no `data-active`).
///
/// The form contains no text input, so every submit has an explicit
/// `submitter` — no implicit-submission ambiguity to guard against.
///
/// When `cfg.list_id` is `Some`, the `<form>` carries
/// `data-on:submit__prevent`: a submitter-initiated submit
/// closes the panel, refocuses the summary (both at click time, before the
/// fetch — the user's own dismissal act, `dsp-incubator
/// cpe/docs/adr/0007-fragment-swaps-are-transient.md`'s disclosure-widget exception), and fires the
/// `_sort` fragment `@get` with the clicked row's value; a submitter-less submit (unreachable on
/// any engine running the v1 DataStar bundle, since `SubmitEvent.submitter` ships in Chrome
/// 81/Firefox 75/Safari 15.4) falls back to `el.submit()` — a never-a-dead-control
/// floor, NOT a working degrade: it carries no submitter, so the GET lands
/// on the default order and the visitor's pick is lost. When
/// `cfg.list_id` is `None`, no `data-on:submit__prevent` is emitted and the
/// form is a plain GET.
///
/// `encode_get_arg` is applied to the interpolated `@get` PREFIX only, never
/// to the composed expression — the static `querySelector('details')`-style
/// quotes must stay literal; the prefix's own `'` → `%27` is what blocks a
/// JS-string breakout from a crafted preserved query pair.
///
/// The `<details>` always renders WITHOUT an `open` attribute:
/// the server always renders the panel closed, and a re-render (via the
/// `_sort` swap, itself inside the swapped `#<list-id>` region) snaps it
/// shut via Idiomorph removing the absent attribute.
///
/// `data-on:focusout`'s guard checks `evt.relatedTarget.tabIndex >= 0`,
/// not just `!el.contains(evt.relatedTarget)` alone: in
/// WebKit, a mouse click on a `<button>` row does NOT move focus to that
/// button — focus lands on the nearest ancestor `main#main` (`tabindex="-1"`,
/// the skip-link target), which passes the plain `!el.contains(...)` test
/// (it's outside `<details>`) and closed the panel before the click's own
/// submit ever landed, silently eating every WebKit row click. A
/// `tabindex="-1"` element is never a legitimate focus TARGET a visitor
/// tabbed to on purpose, so excluding it (`tabIndex >= 0`) still closes the
/// panel on a real Tab-away/click-elsewhere without misfiring on
/// WebKit's own click-focus quirk.
fn render_order_control(cfg: &ResourceListCfg<'_>, current_sort: Option<&str>, lang: &Lang) -> Markup {
    let label_text = ChromeMsg::OrderLabel.text(lang);
    let key = normalize_sort_key(current_sort, cfg.order_options);
    let current_label = order_label_for_key(
        key,
        lang,
        cfg.order_default_label,
        cfg.order_name_label,
        cfg.order_name_desc_label,
        cfg.order_options,
    );
    let summary_aria_label = format!("{label_text}: {current_label}");
    let status_text = format!("{label_text}: {current_label}");
    let is_active = key != "default";

    // Hidden inputs: one per raw query PAIR (not a deduped map) except
    // `sort`/`page`, so a repeated facet key (e.g. `role=R_01&role=R_02`)
    // round-trips both values. Each key/value is decoded once — the browser
    // re-encodes hidden inputs on submit, so a raw `q=quantum+cat` would
    // otherwise re-submit as a literal `+` (see `util::hidden_pairs_dropping`).
    let hidden_pairs = cpe_core::util::hidden_pairs_dropping(cfg.current_query, &["sort", "page"]);

    // DataStar `data-on:submit__prevent`, built only when the list has a
    // patch target. PREFIX = "{current_path}/{SORT_FRAGMENT_SEGMENT}?{preserved}"
    // with a trailing `&` only when `preserved` is non-empty (mirrors
    // `search_bar::search_fragment_prefix` — today's control emits a stray
    // `?&`), passed through `encode_get_arg` (percent-encodes `'`, `` ` ``,
    // `\`); the runtime JS then concatenates `sort=' + evt.submitter.value`
    // onto it. Note Maud escapes `&` to `&amp;` in the rendered attribute
    // value, so any assertion spanning the expression's `&&` operators must
    // expect `&amp;&amp;` in the output (the browser decodes it back to `&`
    // before DataStar's JS ever sees it — same precedent as the facet
    // toggle `@get` attributes).
    let data_on_submit: Option<String> = cfg.list_id.map(|_| {
        let preserved_qs = preserved_query_dropping_sort_and_page(cfg.current_query);
        let prefix = if preserved_qs.is_empty() {
            format!("{}/{}?", cfg.current_path, SORT_FRAGMENT_SEGMENT)
        } else {
            format!("{}/{}?{}&", cfg.current_path, SORT_FRAGMENT_SEGMENT, preserved_qs)
        };
        let encoded_prefix = encode_get_arg(&prefix);
        format!(
            "evt.submitter ? (document.getElementById('orderctl').open = false, document.getElementById('orderctl').querySelector('summary').focus({{preventScroll: true}}), @get('{encoded_prefix}sort=' + evt.submitter.value)) : el.submit()"
        )
    });

    html! {
        form
            .orderctl
            method="get"
            action=(cfg.current_path)
            data-active=[is_active.then_some("true")]
            data-on:submit__prevent=[data_on_submit]
        {
            @for (k, v) in &hidden_pairs {
                input type="hidden" name=(k) value=(v);
            }
            span.viewtoggle__lbl { (label_text) }
            details
                #orderctl
                .orderctl__disc
                data-on:keydown__window="evt.key === 'Escape' && el.open && (evt.preventDefault(), el.open = false, el.querySelector('summary').focus({preventScroll: true}))"
                data-on:click__outside="el.open = false"
                data-on:focusout="evt.relatedTarget && evt.relatedTarget.tabIndex >= 0 && !el.contains(evt.relatedTarget) && (el.open = false)"
            {
                summary.orderctl__current aria-label=(summary_aria_label) {
                    (current_label)
                    span.orderctl__caret aria-hidden="true" { "▾" }
                }
                div.orderctl__panel {
                    @for row_key in BUILTIN_ORDER_KEYS {
                        button
                            .orderctl__opt
                            type="submit"
                            name="sort"
                            value=(row_key)
                            aria-current=[(row_key == key).then_some("true")]
                        {
                            ({
                                order_label_for_key(
                                    row_key,
                                    lang,
                                    cfg.order_default_label,
                                    cfg.order_name_label,
                                    cfg.order_name_desc_label,
                                    cfg.order_options,
                                )
                            })
                        }
                    }
                    @for row in cfg.order_options {
                        button
                            .orderctl__opt
                            type="submit"
                            name="sort"
                            value=(row.key)
                            aria-current=[(row.key == key).then_some("true")]
                        { (row.label) }
                    }
                }
            }
            span
                #orderctl-status
                .visually-hidden
                .orderctl__status
                aria-live="polite"
                aria-atomic="true"
            { (status_text) }
        }
    }
}

// ── Pager ─────────────────────────────────────────────────────────────────────

/// Above this many pages, the numbered-links middle (`.pager__nums`) is
/// replaced by a single "Page n of N" `span.pager__of`
/// — a wall of 100+ number links is unusable, and CSS-hiding them
/// all above this threshold instead of the position text keeps the pager
/// legible without ever emitting the whole run of links to the DOM.
pub(crate) const PAGER_NUMBERED_MAX: usize = 8;

/// Render the pagination `<nav>` when applicable, or empty markup.
///
/// Returns empty markup when:
/// - `list.pagination` is `Pagination::Limit` (no pager for limit mode).
/// - Total pages is 1 (single page — no pager needed).
/// - The list is empty (caller already returned before reaching here, but guard retained for
///   correctness).
fn render_browse_pager(list: &ResourceListPage, cfg: &ResourceListCfg<'_>, lang: &Lang) -> Markup {
    let per_page = match list.pagination {
        Pagination::PerPage(n) => usize::try_from(n).unwrap_or(usize::MAX),
        Pagination::Limit(_) => return html! {},
    };

    let last = compute_last_page(list.total, per_page);
    if last <= 1 {
        return html! {};
    }

    let page = list.page as usize;
    let of_text = pager_page_of(page, last, lang);
    let nav_aria = format!("{}, {of_text}", ChromeMsg::PagerNavLabel.text(lang));
    let prev_word = ChromeMsg::PagerPrev.text(lang);
    let next_word = ChromeMsg::PagerNext.text(lang);

    // Prev href: only when not on the first page.
    let prev_href: Option<String> = (page > 1).then(|| build_pager_href(cfg.current_path, cfg.current_query, page - 1));
    // Next href: only when not on the last page.
    let next_href: Option<String> =
        (page < last).then(|| build_pager_href(cfg.current_path, cfg.current_query, page + 1));

    let numbered = last <= PAGER_NUMBERED_MAX;

    html! {
        nav.pager aria-label=(nav_aria) {
            // Prev step: active anchor or inert span at first page. No
            // `aria-hidden` — the inert end stays readable, the AT cue is the
            // absence of a link.
            @if let Some(ref h) = prev_href {
                a.pager__step href=(h) { (prev_word) }
            } @else {
                span.pager__step aria-disabled="true" { (prev_word) }
            }
            @if numbered {
                div.pager__nums {
                    @for n in 1..=last {
                        @let href = build_pager_href(cfg.current_path, cfg.current_query, n);
                        @let num_aria = pager_page_n(n, lang);
                        @if n == page {
                            a.pager__num href=(href) aria-label=(num_aria) aria-current="page" { (n) }
                        } @else {
                            a.pager__num href=(href) aria-label=(num_aria) { (n) }
                        }
                    }
                }
                // Always emitted: the nav label already speaks the
                // position, so this is redundant here — hidden above 640px by
                // CSS.
                span.pager__of aria-hidden="true" { (of_text) }
            } @else {
                // No numbered links at this page count — the position text is
                // the only indicator, so it carries no `aria-hidden`.
                span.pager__of { (of_text) }
            }
            // Next step: active anchor or inert span at last page.
            @if let Some(ref h) = next_href {
                a.pager__step href=(h) { (next_word) }
            } @else {
                span.pager__step aria-disabled="true" { (next_word) }
            }
        }
    }
}

/// Compute the last page number (1-based, minimum 1).
///
/// `div_ceil(total, per_page)` gives the page count; clamped to ≥ 1 so an
/// empty list still yields page 1 (the caller guards against rendering a pager
/// when the list is empty, but this function must never return 0).
pub(crate) fn compute_last_page(total: usize, per_page: usize) -> usize {
    if per_page == 0 {
        return 1;
    }
    total.div_ceil(per_page).max(1)
}

/// Build a pager href: `{current_path}?{query_with_page_overridden}`.
///
/// Preserves all existing query params (lang, appearance, …) while
/// replacing (or appending) the `page` param. Mirrors the carousel's
/// `override_query_param` + `build_carousel_href` approach.
pub(crate) fn build_pager_href(current_path: &str, current_query: Option<&str>, page: usize) -> String {
    let qs = override_query_param(current_query, "page", &page.to_string());
    format!("{current_path}?{qs}")
}

/// Build a query string with `key` set to `value`, preserving all other params.
///
/// If `key` already appears, it is replaced; otherwise it is appended.
/// Empty pairs in `current_query` are skipped.
pub fn override_query_param(current_query: Option<&str>, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut found = false;
    if let Some(q) = current_query {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            if k == key {
                parts.push(format!("{key}={value}"));
                found = true;
            } else {
                parts.push(pair.to_owned());
            }
        }
    }
    if !found {
        parts.push(format!("{key}={value}"));
    }
    parts.join("&")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{MediaThumb, Pagination, Property, PropertyValue, Resource, ResourceListPage};

    use super::*;

    /// Empty id→thumb map, used by tests/helpers with no thumbnails to
    /// configure (mirrors `dsp-incubator cpe/web`'s `page` module's `EMPTY_ROUTES`/`EMPTY_MEDIA` —
    /// a `'static` empty map so it can be borrowed from a function that
    /// returns a `ResourceListCfg`, not just a `let`-bound temporary).
    static EMPTY_THUMBS: std::sync::LazyLock<HashMap<String, MediaThumb>> = std::sync::LazyLock::new(HashMap::new);

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn make_lang(code: &str) -> Lang {
        use cpe_core::ir::SiteConfig;
        let site = SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned(), "fr".to_owned(), "it".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        cpe_core::lang::resolve(&[code.to_owned()], &site)
    }

    fn make_resource(id: &str, class: &str, name: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: class.to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: name.to_owned(), lang: Some("en".to_owned()) }],
            }],
            representations: vec![],
        }
    }

    fn make_resolver(project_id: &'static str, routes: &'static HashMap<String, String>) -> LinkResolver<'static> {
        LinkResolver::new(project_id, routes)
    }

    fn make_routes() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("Character".to_owned(), "/characters/:id".to_owned());
        m
    }

    fn make_list_page(items: Vec<Resource>, total: usize, page: u32, pagination: Pagination) -> ResourceListPage {
        ResourceListPage { items, total, page, pagination, unfiltered_total: None }
    }

    // ── Row thumbnail decorative alt ────────────────────

    /// A row thumbnail is decorative — the adjacent `.rl-annot__name` is its
    /// accessible label — so it must render `alt=""`, never a meaningful alt
    /// derived from the resource.
    #[test]
    fn row_thumbnail_alt_is_empty_and_decorative() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &thumbs,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_row(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="rl-annot__thumb""#),
            "row must render a thumbnail, got: {out}"
        );
        assert!(
            out.contains(r#"alt="""#),
            "the row thumbnail must be decorative (alt=\"\"), got: {out}"
        );
    }

    /// A thumb narrower than `THUMB_WIDTH` (e.g. an Incunabula BorderStrip,
    /// 288-360px) must be requested at its own stored width, never at the
    /// unclamped `THUMB_WIDTH` (384) — Sipi (IIIF v3) 400s a size request
    /// wider than the image's native width.
    #[test]
    fn row_thumbnail_src_is_clamped_to_narrower_stored_width() {
        let resource = make_resource("STRIP_001", "BorderStrip", "Randleiste 1");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "STRIP_001".to_owned(),
            MediaThumb {
                uri: "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg".to_owned(),
                width: Some(288),
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/incunabula/strips",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "BorderStrip",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &thumbs,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_row(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains("src=\"https://iiif.dasch.swiss/0803/abc.jpx/full/288,/0/default.jpg\""),
            "must clamp to the narrower stored width (288), never request THUMB_WIDTH (384) \
             unclamped, got: {out}"
        );
    }

    // ── Unit: compute_last_page ───────────────────────────────────────────────

    #[test]
    fn compute_last_page_exact_division() {
        assert_eq!(compute_last_page(20, 10), 2);
    }

    #[test]
    fn compute_last_page_remainder() {
        assert_eq!(compute_last_page(21, 10), 3);
    }

    #[test]
    fn compute_last_page_zero_total_returns_one() {
        assert_eq!(compute_last_page(0, 10), 1);
    }

    #[test]
    fn compute_last_page_per_page_larger_than_total() {
        assert_eq!(compute_last_page(5, 24), 1);
    }

    #[test]
    fn compute_last_page_single_item() {
        assert_eq!(compute_last_page(1, 24), 1);
    }

    // ── Unit: override_query_param ────────────────────────────────────────────

    #[test]
    fn override_query_param_appends_when_absent() {
        let qs = override_query_param(Some("lang=de"), "page", "2");
        assert_eq!(qs, "lang=de&page=2");
    }

    #[test]
    fn override_query_param_replaces_existing() {
        let qs = override_query_param(Some("lang=de&page=1"), "page", "3");
        assert_eq!(qs, "lang=de&page=3");
    }

    #[test]
    fn override_query_param_none_query_produces_single_param() {
        let qs = override_query_param(None, "page", "2");
        assert_eq!(qs, "page=2");
    }

    #[test]
    fn override_query_param_preserves_appearance() {
        let qs = override_query_param(Some("lang=fr&appearance=dark&page=1"), "page", "2");
        assert_eq!(qs, "lang=fr&appearance=dark&page=2");
    }

    /// `view=grid` survives a `page=N` override on `resource_list`'s OWN
    /// `override_query_param` — this is a
    /// separate, independently-maintained copy from `component::pager`'s
    /// (see pager.rs's `override_query_param_preserves_view`), so it needs
    /// its own coverage.
    #[test]
    fn override_query_param_preserves_view() {
        let qs = override_query_param(Some("view=grid&page=1"), "page", "2");
        assert_eq!(qs, "view=grid&page=2");
    }

    // ── Unit: build_pager_href ────────────────────────────────────────────────

    #[test]
    fn build_pager_href_no_existing_query() {
        let href = build_pager_href("/alice-in-daschland/characters", None, 2);
        assert_eq!(href, "/alice-in-daschland/characters?page=2");
    }

    #[test]
    fn build_pager_href_preserves_lang_overrides_page() {
        let href = build_pager_href("/alice-in-daschland/characters", Some("lang=de&page=1"), 3);
        assert_eq!(href, "/alice-in-daschland/characters?lang=de&page=3");
    }

    // ── Unit: pager inert-end logic ──────────────────────

    fn base_cfg<'a>(
        resolver: LinkResolver<'a>,
        current_path: &'a str,
        current_query: Option<&'a str>,
    ) -> ResourceListCfg<'a> {
        ResourceListCfg {
            links: Some(resolver),
            current_path,
            current_query,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        }
    }

    #[test]
    fn pager_page_1_of_3_prev_inert_next_active() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 15 total, 5 per page → 3 pages; current = page 1.
        let list = make_list_page(items, 15, 1, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", None);
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // Prev is an inert span carrying the word "Previous", no aria-hidden.
        assert!(
            out.contains(r#"<span class="pager__step" aria-disabled="true">Previous</span>"#),
            "page 1: prev must be an inert span with the word, got: {out}"
        );
        // Next is an active <a> pointing to page 2, carrying the word "Next".
        assert!(
            out.contains(r#"href="/alice-in-daschland/characters?page=2""#) && out.contains(">Next</a>"),
            "page 1: next must be an active <a> to page 2 with the word, got: {out}"
        );
    }

    #[test]
    fn pager_page_3_of_3_next_inert_prev_active() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 15 total, 5 per page → 3 pages; current = page 3 (last).
        let list = make_list_page(items, 15, 3, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", Some("lang=en"));
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // Next is an inert span carrying the word "Next", no aria-hidden.
        assert!(
            out.contains(r#"<span class="pager__step" aria-disabled="true">Next</span>"#),
            "page 3: next must be an inert span with the word, got: {out}"
        );
        // Prev is an active <a> pointing to page 2, carrying the word "Previous".
        assert!(
            out.contains(r#"href="/alice-in-daschland/characters?lang=en&amp;page=2""#)
                && out.contains(">Previous</a>"),
            "page 3: prev must be an active <a> to page 2 with the word, got: {out}"
        );
    }

    #[test]
    fn pager_numbered_links_at_last_eq_8() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 40 total, 5 per page → 8 pages; current = page 3.
        let list = make_list_page(items, 40, 3, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", Some("lang=de&role=R_01"));
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // Exactly 8 `.pager__num` links.
        let num_count = out.matches(r#"class="pager__num""#).count();
        assert_eq!(num_count, 8, "last=8 must render exactly 8 .pager__num, got: {out}");
        // Exactly one carries aria-current="page" (the current page's self-link).
        let current_count = out.matches(r#"aria-current="page""#).count();
        assert_eq!(current_count, 1, "exactly one .pager__num must be aria-current, got: {out}");
        // Hrefs preserve lang/facet params alongside the overridden page.
        assert!(
            out.contains("lang=de&amp;role=R_01&amp;page=1"),
            "pager__num hrefs must preserve lang/facets, got: {out}"
        );
    }

    #[test]
    fn pager_last_eq_9_no_numbered_links_shows_pager_of() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 45 total, 5 per page → 9 pages; current = page 2.
        let list = make_list_page(items, 45, 2, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", None);

        let en = make_lang("en");
        let out_en = render_resource_list(&list, &cfg, &en).into_string();
        assert!(
            !out_en.contains(r#"class="pager__num""#),
            "last=9 must render no .pager__num, got: {out_en}"
        );
        assert!(
            out_en.contains(r#"<span class="pager__of">Page 2 of 9</span>"#),
            "last=9 must render .pager__of with no aria-hidden, got: {out_en}"
        );

        let de = make_lang("de");
        let out_de = render_resource_list(&list, &cfg, &de).into_string();
        assert!(
            out_de.contains(r#"<span class="pager__of">Seite 2 von 9</span>"#),
            "last=9 German .pager__of text, got: {out_de}"
        );
    }

    #[test]
    fn pager_last_eq_3_pager_of_present_with_aria_hidden_after_nums() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        let list = make_list_page(items, 15, 2, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", None);
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        let nums_idx = out.find(r#"class="pager__nums""#).expect("last=3 must render .pager__nums");
        let of_marker = r#"<span class="pager__of" aria-hidden="true">Page 2 of 3</span>"#;
        let of_idx = out
            .find(of_marker)
            .expect("last=3 .pager__of must carry aria-hidden and the position text");
        assert!(of_idx > nums_idx, ".pager__of must immediately follow .pager__nums, got: {out}");
    }

    #[test]
    fn pager_nav_aria_label_contains_position() {
        let routes = make_routes();
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        let list = make_list_page(items, 15, 2, Pagination::PerPage(5));
        let cfg = base_cfg(resolver, "/alice-in-daschland/characters", None);
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        assert!(
            out.contains("Page navigation, Page 2 of 3"),
            "nav aria-label must contain the position text, got: {out}"
        );
    }

    // ── Unit: localized helpers (sourced from the chrome catalog) ────────

    #[test]
    fn pager_nav_label_de() {
        let lang = make_lang("de");
        assert_eq!(ChromeMsg::PagerNavLabel.text(&lang), "Seitennavigation");
    }

    #[test]
    fn pager_nav_label_fr() {
        let lang = make_lang("fr");
        assert_eq!(ChromeMsg::PagerNavLabel.text(&lang), "Navigation par page");
    }

    #[test]
    fn pager_nav_label_it() {
        let lang = make_lang("it");
        assert_eq!(ChromeMsg::PagerNavLabel.text(&lang), "Navigazione pagina");
    }

    #[test]
    fn pager_nav_label_en() {
        let lang = make_lang("en");
        assert_eq!(ChromeMsg::PagerNavLabel.text(&lang), "Page navigation");
    }

    // ── Unit: render_order_control ────────────────────────────────────────────

    fn make_sort_cfg<'a>(
        current_path: &'a str,
        current_query: Option<&'a str>,
        list_id: Option<&'a str>,
    ) -> ResourceListCfg<'a> {
        ResourceListCfg {
            links: None,
            current_path,
            current_query,
            list_id,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        }
    }

    /// `None` current sort → the summary reads "As curated" and its
    /// `aria-label` reads "Order: As curated".
    #[test]
    fn order_control_summary_text_and_aria_label_when_current_none() {
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("As curated"),
            "None current sort must show 'As curated', got: {out}"
        );
        assert!(
            out.contains(r#"aria-label="Order: As curated""#),
            "None current sort must set aria-label 'Order: As curated', got: {out}"
        );
    }

    /// An unrecognized `?sort=` value falls back to "As curated" too.
    #[test]
    fn order_control_summary_text_and_aria_label_when_current_unknown() {
        let cfg = make_sort_cfg("/alice/characters", Some("sort=bogus"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("bogus"), &lang).into_string();
        assert!(
            out.contains(r#"aria-label="Order: As curated""#),
            "unrecognized sort value must fall back to 'As curated', got: {out}"
        );
    }

    /// `?sort=class` also falls back to "As curated" (`class`
    /// is not recognized for display).
    #[test]
    fn order_control_summary_text_and_aria_label_when_current_class() {
        let cfg = make_sort_cfg("/alice/characters", Some("sort=class"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("class"), &lang).into_string();
        assert!(
            out.contains(r#"aria-label="Order: As curated""#),
            "sort=class must fall back to 'As curated' (no longer recognized \
             for display), got: {out}"
        );
        assert!(
            !out.contains(r#"data-active="true""#),
            "sort=class must NOT be treated as active, got: {out}"
        );
    }

    /// Each of the four recognized keys — including `default` — marks
    /// exactly the matching row `aria-current="true"`, and no other row
    /// ("the chosen row carries aria-current=true", which
    /// applies to every row including the default "As curated" one).
    #[test]
    fn order_control_aria_current_on_matching_row_for_each_key() {
        let lang = make_lang("en");
        for key in ["name", "name-desc", "id"] {
            let qs = format!("sort={key}");
            let cfg = make_sort_cfg("/alice/characters", Some(&qs), None);
            let out = render_order_control(&cfg, Some(key), &lang).into_string();
            let current_count = out.matches(r#"aria-current="true""#).count();
            assert_eq!(
                current_count, 1,
                "sort={key} must mark exactly one row aria-current=\"true\", got: {out}"
            );
            let needle = format!(r#"value="{key}" aria-current="true""#);
            assert!(
                out.contains(&needle),
                "sort={key} must mark the '{key}' row aria-current, got: {out}"
            );
        }
        // The default row ("default" key, absent `?sort=`) marks its own
        // "As curated" row aria-current — it's still the current ordering.
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains(r#"value="default" aria-current="true""#),
            "the default ordering must mark its own row aria-current, got: {out}"
        );
        let current_count = out.matches(r#"aria-current="true""#).count();
        assert_eq!(current_count, 1, "the default ordering must mark exactly one row, got: {out}");
    }

    /// `data-active="true"` on the `<form>` only for a non-default ordering.
    #[test]
    fn order_control_data_active_only_for_non_default() {
        let lang = make_lang("en");
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            !out.contains("data-active"),
            "the default ordering must NOT carry data-active, got: {out}"
        );

        let cfg = make_sort_cfg("/alice/characters", Some("sort=name"), None);
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();
        assert!(
            out.contains(r#"data-active="true""#),
            "a non-default ordering must carry data-active=\"true\", got: {out}"
        );
    }

    /// `list_id: Some` → `data-on:submit__prevent` present, targets `/_sort?`,
    /// reads `evt.submitter.value`, closes the panel and refocuses the
    /// summary, and carries the `el.submit()` fallback.
    #[test]
    fn order_control_data_on_submit_present_with_list_id() {
        let cfg = make_sort_cfg("/alice-in-daschland/characters", Some("lang=en"), Some("character-list"));
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("data-on:submit__prevent=\""),
            "list_id: Some must emit data-on:submit__prevent (quoted colon \
             form), got: {out}"
        );
        assert!(
            out.contains("/alice-in-daschland/characters/_sort?"),
            "the expression must target the _sort fragment endpoint, got: {out}"
        );
        assert!(
            out.contains("evt.submitter.value"),
            "the expression must read evt.submitter.value, got: {out}"
        );
        assert!(
            out.contains("document.getElementById('orderctl').open = false"),
            "the expression must close the panel at click time, got: {out}"
        );
        assert!(
            out.contains("querySelector('summary').focus"),
            "the expression must refocus the summary at click time, got: {out}"
        );
        assert!(
            out.contains("el.submit()"),
            "the expression must carry the el.submit() never-a-dead-control \
             fallback, got: {out}"
        );
    }

    /// `list_id: None` → no `data-on:submit__prevent` — the form is a plain
    /// GET.
    #[test]
    fn order_control_no_data_on_submit_without_list_id() {
        let cfg = make_sort_cfg("/alice-in-daschland/characters", Some("lang=en"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            !out.contains("data-on:submit__prevent"),
            "list_id: None must NOT emit data-on:submit__prevent, got: {out}"
        );
    }

    /// No stray `?&` in the `@get` prefix when nothing is preserved.
    #[test]
    fn order_control_prefix_has_no_stray_query_ampersand_when_nothing_preserved() {
        let cfg = make_sort_cfg("/alice-in-daschland/characters", None, Some("character-list"));
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            !out.contains("?&"),
            "an empty preserved query must not leave a stray '?&', got: {out}"
        );
        assert!(
            out.contains("/alice-in-daschland/characters/_sort?sort='"),
            "with nothing preserved the prefix must end directly in '?sort=', got: {out}"
        );
    }

    /// Hidden inputs are emitted per raw query PAIR (not deduped) — a repeated
    /// facet key (`role=R_01&role=R_02`) round-trips both values — and never
    /// for `sort`/`page`.
    #[test]
    fn order_control_hidden_inputs_per_raw_pair_excluding_sort_and_page() {
        let cfg = make_sort_cfg("/alice/characters", Some("lang=en&role=R_01&role=R_02&sort=name&page=3"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();

        assert!(
            out.contains(r#"name="lang" value="en""#),
            "lang pair must round-trip as a hidden input, got: {out}"
        );
        let role_01_count = out.matches(r#"name="role" value="R_01""#).count();
        let role_02_count = out.matches(r#"name="role" value="R_02""#).count();
        assert_eq!(
            role_01_count, 1,
            "role=R_01 must appear exactly once as a hidden input, got: {out}"
        );
        assert_eq!(
            role_02_count, 1,
            "role=R_02 must appear exactly once as a hidden input, got: {out}"
        );
        assert!(
            !out.contains(r#"input type="hidden" name="sort""#),
            "sort must NOT be emitted as a hidden input, got: {out}"
        );
        assert!(
            !out.contains(r#"input type="hidden" name="page""#),
            "page must NOT be emitted as a hidden input (a sort change resets it), got: {out}"
        );
    }

    /// Regression: a carried-over pair arrives
    /// percent-/`+`-encoded on the wire; its hidden input's `value=` must
    /// hold the DECODED text, since the browser re-encodes it on the GET
    /// submit. Raw `q=quantum+cat` written verbatim would re-submit as
    /// `q=quantum%2Bcat` (a literal `+`), and `%20` as `%2520`.
    #[test]
    fn order_control_hidden_inputs_decode_carried_values_once() {
        let cfg = make_sort_cfg(
            "/alice/characters",
            Some("q=quantum+cat&x=a%20b&k%C3%A9y=caf%C3%A9&sort=name"),
            None,
        );
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();
        assert!(
            out.contains(r#"name="q" value="quantum cat""#),
            "`+` must decode to a space in the carried q, got: {out}"
        );
        assert!(
            out.contains(r#"name="x" value="a b""#),
            "`%20` must decode to a space, got: {out}"
        );
        assert!(
            out.contains(r#"name="kéy" value="café""#),
            "percent-encoded UTF-8 key and value must both decode, got: {out}"
        );
        assert!(
            !out.contains(r#"value="a%20b""#) && !out.contains(r#"value="quantum+cat""#),
            "no raw encoded form may survive into a hidden input, got: {out}"
        );
    }

    /// Hidden-input escaping: a query pair whose key AND value contain `"`
    /// and `<` must come out Maud-escaped (`&quot;`/`&lt;`), never raw.
    #[test]
    fn order_control_hidden_input_escapes_quote_and_angle_bracket() {
        let cfg = make_sort_cfg("/alice/characters", Some(r#"e"vil<key=e"vil<value&lang=en"#), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("&quot;vil&lt;key"),
            "hidden input NAME must be Maud-escaped (quote+angle-bracket), got: {out}"
        );
        assert!(
            out.contains("&quot;vil&lt;value"),
            "hidden input VALUE must be Maud-escaped (quote+angle-bracket), got: {out}"
        );
        assert!(
            !out.contains("e\"vil<key") && !out.contains("e\"vil<value"),
            "raw unescaped quote/angle-bracket must NOT appear in output, got: {out}"
        );
    }

    /// The `#orderctl-status` span sits OUTSIDE the `<details>`, carries
    /// `aria-live="polite"`/`aria-atomic="true"`, and reads "Order: <current>".
    /// The `<summary>` itself carries NO `aria-live`.
    #[test]
    fn order_control_status_span_outside_details_with_aria_live() {
        let cfg = make_sort_cfg("/alice/characters", Some("sort=name"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();

        let details_start = out.find("<details").expect("details present");
        let details_end = out[details_start..]
            .find("</details>")
            .map(|i| details_start + i + "</details>".len())
            .expect("details closes");
        let status_pos = out.find(r#"id="orderctl-status""#).expect("#orderctl-status present");
        assert!(
            status_pos > details_end,
            "#orderctl-status must sit OUTSIDE (after) the </details>, got: {out}"
        );
        assert!(
            out.contains(r#"aria-live="polite""#) && out.contains(r#"aria-atomic="true""#),
            "the status span must carry aria-live=\"polite\" aria-atomic=\"true\", got: {out}"
        );
        assert!(
            out.contains(">Order: Name A\u{2013}Z<"),
            "the status span text must read 'Order: <current label>', got: {out}"
        );

        // The <summary> itself must carry no aria-live.
        let summary_start = out.find("<summary").expect("summary present");
        let summary_end = out[summary_start..]
            .find("</summary>")
            .map(|i| summary_start + i)
            .expect("summary closes");
        let summary_markup = &out[summary_start..summary_end];
        assert!(
            !summary_markup.contains("aria-live"),
            "the <summary> must NOT carry aria-live, got: {summary_markup}"
        );
    }

    /// No `<select>`/`<noscript>` anywhere in the rendered control — the
    /// native `<select>` is dropped entirely.
    #[test]
    fn order_control_has_no_select_or_noscript() {
        let cfg = make_sort_cfg("/alice/characters", None, Some("character-list"));
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(!out.contains("<select"), "must contain no <select>, got: {out}");
        assert!(!out.contains("<noscript"), "must contain no <noscript>, got: {out}");
    }

    /// The `<details>` carries the keydown/outside/focusout dismissal
    /// attributes.
    #[test]
    fn order_control_details_carries_dismissal_attributes() {
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("data-on:keydown__window="),
            "details must carry data-on:keydown__window, got: {out}"
        );
        assert!(
            out.contains("data-on:click__outside=\"el.open = false\""),
            "details must carry data-on:click__outside, got: {out}"
        );
        assert!(
            out.contains("data-on:focusout="),
            "details must carry data-on:focusout, got: {out}"
        );
    }

    /// Exactly four `type="submit" name="sort"` row buttons.
    #[test]
    fn order_control_has_exactly_four_submit_rows() {
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        let count = out.matches(r#"type="submit" name="sort""#).count();
        assert_eq!(count, 4, "must render exactly four submit rows, got: {out}");
    }

    /// The `<details>` never carries an `open` attribute.
    #[test]
    fn order_control_details_has_no_open_attribute() {
        let cfg = make_sort_cfg("/alice/characters", Some("sort=name"), None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();
        let details_start = out.find("<details").expect("details present");
        let details_tag_end = out[details_start..]
            .find('>')
            .map(|i| details_start + i)
            .expect("details tag closes");
        let details_open_tag = &out[details_start..details_tag_end];
        assert!(
            !details_open_tag.contains(" open"),
            "the <details> open tag must never carry `open`, got: {details_open_tag}"
        );
    }

    /// `action` equals `cfg.current_path`.
    #[test]
    fn order_control_action_equals_current_path() {
        let cfg = make_sort_cfg("/alice-in-daschland/characters", None, None);
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains(r#"action="/alice-in-daschland/characters""#),
            "action must equal current_path, got: {out}"
        );
    }

    /// The expression uses `document.getElementById('orderctl')`, never
    /// `el.querySelector` — the panel is looked up by id, not relative to the
    /// form (`HTMLFormElement`'s named-property getter hazard).
    #[test]
    fn order_control_uses_get_element_by_id_not_el_query_selector() {
        let cfg = make_sort_cfg("/alice/characters", None, Some("character-list"));
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("document.getElementById('orderctl')"),
            "must look up the panel via getElementById('orderctl'), got: {out}"
        );
        assert!(
            !out.contains("el.querySelector('details')"),
            "must NOT look up the panel via el.querySelector, got: {out}"
        );
    }

    /// A preserved pair containing `'` appears as `%27` inside
    /// `data-on:submit__prevent` (the interpolated PREFIX is escaped),
    /// while the static `querySelector('summary')` quotes stay literal (the
    /// composed expression itself is never re-escaped).
    #[test]
    fn order_control_prefix_quote_is_percent_encoded_but_static_quotes_stay_literal() {
        let cfg = make_sort_cfg("/alice/characters", Some("q=alice's"), Some("character-list"));
        let lang = make_lang("en");
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert!(
            out.contains("alice%27s"),
            "the preserved pair's ' must be percent-encoded to %27, got: {out}"
        );
        assert!(
            out.contains("querySelector('summary')"),
            "the static querySelector('summary') quotes must stay literal, got: {out}"
        );
        assert!(
            out.contains("getElementById('orderctl')"),
            "the static getElementById('orderctl') quotes must stay literal, got: {out}"
        );
    }

    // ── Unit: order_label_for_key overrides ───────────────────────────

    /// An override for "name" is returned verbatim, without touching the
    /// `ChromeMsg` default.
    #[test]
    fn order_label_for_key_name_override_returned_verbatim() {
        let lang = make_lang("en");
        let out = order_label_for_key("name", &lang, None, Some("Row label A–Z"), None, &[]);
        assert_eq!(out, "Row label A–Z");
    }

    /// An override for "name-desc" is returned verbatim.
    #[test]
    fn order_label_for_key_name_desc_override_returned_verbatim() {
        let lang = make_lang("en");
        let out = order_label_for_key("name-desc", &lang, None, None, Some("Row label Z–A"), &[]);
        assert_eq!(out, "Row label Z–A");
    }

    /// No override (both `None`) → the `ChromeMsg` default, unchanged.
    #[test]
    fn order_label_for_key_no_override_falls_back_to_chrome_default() {
        let lang = make_lang("en");
        assert_eq!(
            order_label_for_key("name", &lang, None, None, None, &[]),
            ChromeMsg::OrderNameAsc.text(&lang)
        );
        assert_eq!(
            order_label_for_key("name-desc", &lang, None, None, None, &[]),
            ChromeMsg::OrderNameDesc.text(&lang)
        );
    }

    /// An override that resolved to "nothing" (the caller already collapsed a
    /// non-resolving chrome id to `None` before calling this fn) also falls
    /// back to the `ChromeMsg` default — never an empty label.
    #[test]
    fn order_label_for_key_none_override_is_never_an_empty_label() {
        let lang = make_lang("en");
        let out = order_label_for_key("name", &lang, None, None, None, &[]);
        assert!(!out.is_empty(), "must never fall back to an empty label, got: {out:?}");
    }

    /// Each override relabels only its own row: the name-row overrides never
    /// reach "default", and no override reaches "id" (never overridable).
    #[test]
    fn order_label_for_key_overrides_stay_on_their_own_rows() {
        let lang = make_lang("en");
        assert_eq!(
            order_label_for_key("default", &lang, None, Some("Row label A–Z"), Some("Row label Z–A"), &[]),
            ChromeMsg::OrderCurated.text(&lang)
        );
        assert_eq!(
            order_label_for_key("id", &lang, Some("By date"), Some("Row label A–Z"), Some("Row label Z–A"), &[]),
            ChromeMsg::OrderId.text(&lang)
        );
        assert_eq!(
            order_label_for_key("name", &lang, Some("By date"), None, None, &[]),
            ChromeMsg::OrderNameAsc.text(&lang)
        );
    }

    /// `order-default-label` relabels the "As curated" row verbatim.
    #[test]
    fn order_label_for_key_default_override_returned_verbatim() {
        let lang = make_lang("en");
        assert_eq!(
            order_label_for_key("default", &lang, Some("By date"), None, None, &[]),
            "By date"
        );
    }

    /// A default-row override the caller collapsed to `None` (absent, or an
    /// unresolvable chrome id) falls back to `OrderCurated` — never empty.
    #[test]
    fn order_label_for_key_unresolved_default_override_falls_back_to_curated() {
        let lang = make_lang("en");
        let out = order_label_for_key("default", &lang, None, None, None, &[]);
        assert_eq!(out, ChromeMsg::OrderCurated.text(&lang));
        assert!(!out.is_empty(), "must never be an empty label");
    }

    /// End-to-end through `render_order_control`: with both overrides set,
    /// the summary/status/panel rows read the overridden text, not "Name
    /// A–Z"/"Name Z–A".
    #[test]
    fn order_control_uses_name_label_overrides_when_configured() {
        let cfg = ResourceListCfg {
            order_name_label: Some("Row label A–Z"),
            order_name_desc_label: Some("Row label Z–A"),
            order_default_label: None,
            ..make_sort_cfg("/hdm/performances", None, None)
        };
        let lang = make_lang("en");
        let out = render_order_control(&cfg, Some("name"), &lang).into_string();
        assert!(
            out.contains("Row label A–Z"),
            "the override must render in place of 'Name A–Z', got: {out}"
        );
        assert!(
            out.contains("Row label Z–A"),
            "the override must render for the 'name-desc' panel row too, got: {out}"
        );
        assert!(
            !out.contains("Name A\u{2013}Z") && !out.contains("Name Z\u{2013}A"),
            "the default labels must not also render, got: {out}"
        );
    }

    // ── Unit: surf-head count text ────────────────────────────────────────────

    #[test]
    fn surf_head_shows_plain_count_when_not_narrowed() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("1 Character"),
            "unnarrowed surf-head with total=1 must show the singular CLASS noun \
             (pluralization, Step 6; A12.1 made the noun the class noun — this cfg \
             declares no ClassLabel, so it humanizes to \"Character\"), got: {out}"
        );
        assert!(
            !out.contains("1 resource"),
            "the generic engine noun must be gone (A12.1), got: {out}"
        );
        assert!(
            !out.contains(" of "),
            "unnarrowed surf-head must NOT show the 'N of M' form, got: {out}"
        );
    }

    /// Pluralization across de/fr/it for `n ∈ {0, 1, 2}`, exercised
    /// through the actual `render_surf_head` call site (not just the
    /// `chrome::count_resources_text` unit tests). French singularizes `n ==
    /// 0` too (see `chrome::plural_category`). The inflected noun
    /// is the CLASS noun, so this fixture declares per-language singular and
    /// plural `ClassLabel`s for `Character`; an invariable noun (Italian's
    /// borrowed "record") is simply declared identically in both slots.
    #[test]
    fn surf_head_count_pluralizes_across_languages_for_n_0_1_2() {
        let class_labels = vec![cpe_core::ir::ClassLabel {
            class: "Character".to_owned(),
            translations: vec![
                ("de".to_owned(), "Figur".to_owned()),
                ("fr".to_owned(), "personnage".to_owned()),
                ("it".to_owned(), "record".to_owned()),
            ],
            plural: vec![
                ("de".to_owned(), "Figuren".to_owned()),
                ("fr".to_owned(), "personnages".to_owned()),
                ("it".to_owned(), "record".to_owned()),
            ],
        }];
        let mut cfg = make_sort_cfg("/alice/characters", None, None);
        cfg.class_labels = &class_labels;

        let cases: &[(&str, usize, &str)] = &[
            ("de", 0, "0 Figuren"),
            ("de", 1, "1 Figur"),
            ("de", 2, "2 Figuren"),
            ("fr", 0, "0 personnage"),
            ("fr", 1, "1 personnage"),
            ("fr", 2, "2 personnages"),
            ("it", 0, "0 record"),
            ("it", 1, "1 record"),
            ("it", 2, "2 record"),
        ];
        for (code, n, expected) in cases {
            let items: Vec<_> = (0..*n)
                .map(|i| make_resource(&format!("CH_{i:03}"), "Character", "Alice"))
                .collect();
            let list = make_list_page(items, *n, 1, Pagination::PerPage(24));
            let lang = make_lang(code);
            let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
            assert!(
                out.contains(expected),
                "lang={code} n={n} surf-head must contain {expected:?}, got: {out}"
            );
        }
    }

    /// The count noun is the class's DECLARED plural when there is one,
    /// and the humanized class name when there is no `ClassLabel` at all —
    /// never a generic engine noun (`Einträge`/`resources`).
    /// Undeclared is a degradation
    /// `validate::check_class_plurals` already warns about
    /// (`MissingClassPlural`); it must NOT be papered over with a fallback.
    #[test]
    fn surf_head_count_noun_is_the_class_noun_declared_or_humanized() {
        let class_labels = vec![cpe_core::ir::ClassLabel {
            class: "Performance".to_owned(),
            translations: vec![("de".to_owned(), "Auff\u{00FC}hrung".to_owned())],
            plural: vec![("de".to_owned(), "Auff\u{00FC}hrungen".to_owned())],
        }];
        let list = make_list_page(vec![], 8942, 1, Pagination::PerPage(24));
        let lang = make_lang("de");

        let mut cfg = make_sort_cfg("/hdm/performances", None, None);
        cfg.class = "Performance";
        cfg.class_labels = &class_labels;
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("8\u{2019}942 Auff\u{00FC}hrungen"),
            "a declared plural must be used verbatim, grouped by \
             `chrome::group_thousands`, got: {out}"
        );

        // Same class, no `ClassLabel` at all — humanized, never generic.
        let mut bare = make_sort_cfg("/hdm/performances", None, None);
        bare.class = "Performance";
        let out = render_surf_head(&list, &bare, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("8\u{2019}942 Performance"),
            "an undeclared class must humanize its own name, got: {out}"
        );
        assert!(
            !out.contains("Eintr\u{00E4}ge") && !out.contains("resources"),
            "the generic engine noun must not survive as a fallback, got: {out}"
        );
    }

    #[test]
    fn surf_head_shows_n_of_m_when_narrowed() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let mut list = make_list_page(items, 8, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(54);
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(out.contains("8 of 54"), "narrowed surf-head must show 'N of M', got: {out}");
    }

    /// `unfiltered_total == Some(total)` (equal, not narrowed) must NOT trigger
    /// the "N of M" form — only a strictly different value counts as narrowed.
    #[test]
    fn surf_head_plain_count_when_unfiltered_equals_total() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let mut list = make_list_page(items, 24, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(24);
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("24 Character"),
            "unfiltered_total == total must NOT be treated as narrowed, got: {out}"
        );
        assert!(!out.contains(" of "), "must not show 'N of M', got: {out}");
    }

    /// `span.surf-head__count` carries `aria-live="polite"` (a11y review
    /// fix) — mirrors `figure_count_has_aria_live_polite`
    /// (`resource_detail.rs`) and the filterbar's aria-live assertion
    /// (`facet_panel.rs`). The large `resource-list-section` root must NOT
    /// carry `aria-live` at all — only this small count element does.
    #[test]
    fn surf_head_count_has_aria_live_polite() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains(
                "class=\"surf-head__count\" id=\"browse-count\" aria-live=\"polite\" \
                 aria-atomic=\"true\""
            ),
            "surf-head__count must carry aria-live=\"polite\" aria-atomic=\"true\", got: {out}"
        );
    }

    /// `count_resources_text`/`count_of_text` (via `render_surf_head`) handle
    /// `total == 0` correctly for the unnarrowed "0 <class plural>" case.
    #[test]
    fn surf_head_shows_plain_zero_count_when_empty_and_not_narrowed() {
        let list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("0 Character"),
            "an empty, unnarrowed list must show '0 <class plural>', got: {out}"
        );
        assert!(!out.contains(" of "), "must not show 'N of M', got: {out}");
    }

    /// `count_resources_text`/`count_of_text` (via `render_surf_head`) handle
    /// `total == 0` correctly for the facet-narrowed-to-zero "0 of M" case
    /// (a facet panel narrowed a non-empty class down to zero matches).
    #[test]
    fn surf_head_shows_n_of_m_zero_when_narrowed_to_zero() {
        let mut list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(54);
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains("0 of 54"),
            "a facet-narrowed-to-zero list must show '0 of 54', got: {out}"
        );
    }

    /// A11y regression guard: the EMPTY branch of `render_resource_list`
    /// (reached via `list.items.is_empty()`) must still render
    /// `span.surf-head__count` with `aria-live="polite" aria-atomic="true"` —
    /// mirrors `surf_head_count_has_aria_live_polite` (the populated-branch
    /// version) but through the public `render_resource_list` entry point
    /// with an empty items list, so the live region exists at the same
    /// leading position in both branches and a facet-narrowed-to-zero morph
    /// is announced to a screen reader (without `.surf-head` the
    /// live-region node would not exist to be
    /// preserved+updated by idiomorph across the morph).
    #[test]
    fn resource_list_empty_branch_still_has_surf_head_count_live_region() {
        let list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(
                "class=\"surf-head__count\" id=\"browse-count\" aria-live=\"polite\" \
                 aria-atomic=\"true\""
            ),
            "the empty branch must still render surf-head__count with aria-live=\"polite\" \
             aria-atomic=\"true\", got: {out}"
        );
        assert!(
            out.contains("0 Character"),
            "the empty branch's count text must read '0 <class plural>', got: {out}"
        );
        assert!(
            out.contains("This class contains no resources."),
            "the empty branch must still show the empty-state message, got: {out}"
        );
    }

    /// The `resource-list-section` root itself must NOT carry `aria-live` —
    /// only the small `surf-head__count` element does (a11y review fix:
    /// scoping the live region away from the whole up-to-24-row section).
    #[test]
    fn resource_list_section_root_has_no_aria_live() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let cfg = make_sort_cfg("/alice/characters", None, None);
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            !out.contains("class=\"resource-list-section page\" aria-live"),
            "resource-list-section root must NOT carry aria-live, got: {out}"
        );
    }

    // ── Layer-3: insta snapshots ──────────────────────────────────────────────

    /// (a) Populated + paginated (middle page — both prev & next active, linked rows).
    #[test]
    fn snapshot_resource_list_populated_middle_page() {
        let routes = make_routes();
        static ROUTES_A: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_A.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = vec![
            make_resource("CH_001", "Character", "Alice"),
            make_resource("CH_002", "Character", "The White Rabbit"),
            make_resource("CH_003", "Character", "The Cheshire Cat"),
        ];
        let mut thumbs = HashMap::new();
        // Canonical IIIF `/full/max/` URI so the snapshot freezes
        // `iiif_thumb_url_clamped`'s rewrite to `/full/{THUMB_WIDTH},/` (plain
        // `example.org/…` fixtures leave that path unexercised).
        thumbs.insert(
            "CH_002".to_owned(),
            MediaThumb {
                uri: "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        // 10 total, 3 per page, page 2 → prev to page 1, next to page 3
        let list = make_list_page(items, 10, 2, Pagination::PerPage(3));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: Some("lang=en"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &thumbs,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (b) First page of multi-page (prev inert, next active).
    #[test]
    fn snapshot_resource_list_first_page() {
        let routes = make_routes();
        static ROUTES_B: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_B.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = vec![
            make_resource("CH_001", "Character", "Alice"),
            make_resource("CH_002", "Character", "The White Rabbit"),
        ];
        // 6 total, 2 per page, page 1 → prev inert, next active
        let list = make_list_page(items, 6, 1, Pagination::PerPage(2));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (c) Last page (next inert, prev active).
    #[test]
    fn snapshot_resource_list_last_page() {
        let routes = make_routes();
        static ROUTES_C: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_C.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = vec![
            make_resource("CH_005", "Character", "The Mad Hatter"),
            make_resource("CH_006", "Character", "The Queen of Hearts"),
        ];
        // 6 total, 2 per page, page 3 → prev active, next inert
        let list = make_list_page(items, 6, 3, Pagination::PerPage(2));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: Some("lang=de"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (d) Single page PerPage — no pager rendered.
    #[test]
    fn snapshot_resource_list_single_page_no_pager() {
        let routes = make_routes();
        static ROUTES_D: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_D.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = vec![
            make_resource("CH_001", "Character", "Alice"),
            make_resource("CH_002", "Character", "The White Rabbit"),
        ];
        // 2 total, 24 per page → 1 page only, no pager
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        // Must contain list rows but no pager nav
        assert!(
            !out.contains("class=\"pager\""),
            "single-page list must not have a pager, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// (e) Limit form — no pager, ever.
    #[test]
    fn snapshot_resource_list_limit_no_pager() {
        let items: Vec<Resource> = vec![
            make_resource("CH_001", "Character", "Alice"),
            make_resource("CH_002", "Character", "The White Rabbit"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::Limit(10));
        let cfg = ResourceListCfg {
            links: None, // unbound class — plain div rows
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            !out.contains("class=\"pager\""),
            "Limit form must never have a pager, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// (f) Empty list — localized empty-state, no pager.
    #[test]
    fn snapshot_resource_list_empty() {
        let list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains("This class contains no resources."),
            "empty list must show empty-state message, got: {out}"
        );
        assert!(!out.contains("class=\"pager\""), "empty list must not have a pager, got: {out}");
        // Regression guard: a genuinely-empty list
        // with NO search (`cfg.search: None`) must keep rendering the
        // `.resource-list__empty` state — never
        // `.search-empty`, and never a scope note (only `cfg.search: Some`
        // ever appends one).
        assert!(
            !out.contains("search-empty"),
            "an empty list with no search must NOT render .search-empty, got: {out}"
        );
        assert!(
            !out.contains("search-scope"),
            "an empty list with no search must NOT render a scope note, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Unbound class (no links) renders rows as plain div, not anchor.
    /// This is the outer-None path: `cfg.links` is `None`.
    #[test]
    fn snapshot_resource_list_unbound_class_no_link() {
        let items: Vec<Resource> = vec![
            make_resource("LOC_001", "Location", "Oxford"),
            make_resource("LOC_002", "Location", "Wonderland"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: None, // no resolver → no links
            current_path: "/alice-in-daschland/locations",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Location",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            !out.contains("<a"),
            "unbound class must render rows as plain div, no <a> tag, got: {out}"
        );
        assert!(out.contains("Oxford"), "row names must appear even without links, got: {out}");
        insta::assert_snapshot!(out);
    }

    /// Inner-None fallback: `cfg.links` is `Some(resolver)` but the resolver's
    /// class→route map does NOT contain the row resource's class. The resolver
    /// binds "Character" but the items are "Location". `resolver.href(id,
    /// Some("Location"))` returns `None` → rows must render as plain `<div>`,
    /// not `<a>`. This is the real fallback path used in production when a
    /// page lists a class that has no bound detail route.
    ///
    /// Distinct from `snapshot_resource_list_unbound_class_no_link` which uses
    /// `links: None` (outer-None). This test exercises the inner-None path
    /// where the resolver is present but doesn't cover the row's class.
    #[test]
    fn unbound_class_with_present_resolver_inner_none_renders_plain() {
        // Build a resolver that binds "Character" only — NOT "Location".
        let mut routes: HashMap<String, String> = HashMap::new();
        routes.insert("Character".to_owned(), "/characters/:id".to_owned());

        static ROUTES_INNER: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_INNER.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        // Items are "Location" — NOT bound in the resolver.
        let items: Vec<Resource> = vec![
            make_resource("LOC_001", "Location", "Oxford"),
            make_resource("LOC_002", "Location", "Wonderland"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: Some(resolver), // resolver present but does not bind "Location"
            current_path: "/alice-in-daschland/locations",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Location",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // Must render as plain div, not <a> — inner-None path.
        assert!(
            !out.contains("<a"),
            "inner-None path must render rows as plain div (no <a>), got: {out}"
        );

        // Row content must still be present.
        assert!(
            out.contains("Oxford"),
            "row name 'Oxford' must appear even with unbound resolver, got: {out}"
        );
        assert!(
            out.contains("Wonderland"),
            "row name 'Wonderland' must appear even with unbound resolver, got: {out}"
        );
    }

    /// (g) Composed: `list_id: Some` (DataStar-enhanced sort control) + a
    /// facet-narrowed list (`unfiltered_total: Some(54)`, `total: 8`) + an
    /// active `?sort=` value in `current_query` — the three
    /// browse-surface features rendered together through the public
    /// `render_resource_list` entry point. `surf_head_shows_n_of_m_when_narrowed`
    /// and `order_control_data_on_submit_present_with_list_id` each exercise one
    /// of these in isolation (against `render_surf_head`/`render_order_control`
    /// directly); this snapshot closes the gap of never exercising both
    /// together through the composed renderer.
    #[test]
    fn snapshot_resource_list_narrowed_with_sort_and_list_id() {
        let routes = make_routes();
        static ROUTES_G: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_G.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = vec![
            make_resource("CH_010", "Character", "The Mock Turtle"),
            make_resource("CH_011", "Character", "The Gryphon"),
        ];
        // Facet-narrowed: 8 of an unfiltered 54; single page (8 < 24 per_page,
        // so no pager competes with the surf-head/sort-control markup).
        let mut list = make_list_page(items, 8, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(54);

        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            // An active facet (`role=n1`) plus a live sort choice (`sort=name`):
            // the ordering control's `data-on:submit__prevent` must preserve
            // `role=n1` while dropping `sort`/`page` from the `@get` prefix.
            current_query: Some("role=n1&sort=name"),
            list_id: Some("character-list"),
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // Fail-fast sanity checks (clearer failure than a bare snapshot diff)
        // before asserting the full composed markup via the snapshot below.
        assert!(
            out.contains("8 of 54"),
            "narrowed + list_id: Some composition must still show 'N of M', got: {out}"
        );
        assert!(
            out.contains("/alice-in-daschland/characters/_sort?role=n1&amp;sort='"),
            "data-on:submit__prevent @get must preserve the active facet (role=n1) \
             and target _sort, got: {out}"
        );
        assert!(
            out.contains("evt.submitter.value"),
            "data-on:submit__prevent must read the live-picked sort value \
             via evt.submitter.value, got: {out}"
        );
        assert!(
            !out.contains("<noscript>") && !out.contains("sort-form__go"),
            "the ordering control must not emit the retired <noscript>/sort-form__go \
             markup, got: {out}"
        );

        insta::assert_snapshot!(out);
    }

    /// A list declaring two `order-by` options, with
    /// one of them active (`?sort=date`) — the panel renders the four
    /// built-ins then the two declared rows in config order, the active
    /// declared row carries `aria-current`, and the summary/status read its
    /// label.
    #[test]
    fn snapshot_resource_list_with_two_declared_order_options() {
        let items: Vec<Resource> = vec![
            make_resource("B_001", "Book", "Zeitglöcklein"),
            make_resource("B_002", "Book", "Narrenschiff"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let rows = [
            OrderRow { key: "date", label: "Date" },
            OrderRow { key: "extent", label: "Extent" },
        ];
        let cfg = ResourceListCfg {
            current_query: Some("role=n1&sort=date&page=2"),
            class: "Book",
            order_options: &rows,
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert_eq!(out.matches("orderctl__opt").count(), 6);
        insta::assert_snapshot!(out);
    }

    /// `order-default-label` relabels the "As curated" row, and — because it is
    /// the current row with no `?sort=` — the summary and the live status too.
    #[test]
    fn snapshot_resource_list_with_order_default_label() {
        let items: Vec<Resource> = vec![
            make_resource("B_001", "Book", "Zeitglöcklein"),
            make_resource("B_002", "Book", "Narrenschiff"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let rows = [OrderRow { key: "extent", label: "By extent" }];
        let cfg = ResourceListCfg {
            class: "Book",
            order_default_label: Some("By date"),
            order_name_label: Some("Title A–Z"),
            order_name_desc_label: Some("Title Z–A"),
            order_options: &rows,
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"value="default" aria-current="true">By date</button>"#),
            "the default row must carry the override label, got: {out}"
        );
        assert!(
            !out.contains(ChromeMsg::OrderCurated.text(&lang)),
            "the built-in 'As curated' label must not render once overridden, got: {out}"
        );
        assert!(
            out.contains("Order: By date"),
            "the summary/status must follow the relabelled current row, got: {out}"
        );
        assert_eq!(out.matches("orderctl__opt").count(), 5);
        insta::assert_snapshot!(out);
    }

    // ── Unit: declared order-by options ──────────────

    const DECLARED_ROWS: [OrderRow<'static>; 2] = [
        OrderRow { key: "date", label: "Date" },
        OrderRow { key: "extent", label: "Extent" },
    ];

    #[test]
    fn normalize_sort_key_recognises_a_declared_key() {
        assert_eq!(normalize_sort_key(Some("date"), &DECLARED_ROWS), "date");
        assert_eq!(normalize_sort_key(Some("extent"), &DECLARED_ROWS), "extent");
        // Built-ins are unaffected by declared options.
        assert_eq!(normalize_sort_key(Some("name"), &DECLARED_ROWS), "name");
    }

    #[test]
    fn normalize_sort_key_unknown_or_undeclared_falls_back_to_default() {
        assert_eq!(normalize_sort_key(Some("bogus"), &DECLARED_ROWS), "default");
        // A declared key of ANOTHER list means nothing here.
        assert_eq!(normalize_sort_key(Some("date"), &[]), "default");
        assert_eq!(normalize_sort_key(None, &DECLARED_ROWS), "default");
    }

    #[test]
    fn order_label_for_key_returns_a_declared_rows_label() {
        let lang = make_lang("en");
        assert_eq!(order_label_for_key("extent", &lang, None, None, None, &DECLARED_ROWS), "Extent");
        assert_eq!(
            order_label_for_key("bogus", &lang, None, None, None, &DECLARED_ROWS),
            ChromeMsg::OrderCurated.text(&lang)
        );
    }

    /// The panel renders `4 + n` rows: built-ins first, then the declared
    /// options in config order.
    #[test]
    fn order_control_button_count_is_four_plus_declared() {
        let lang = make_lang("en");
        let base = make_sort_cfg("/incunabula/books", None, Some("book-list"));
        let out = render_order_control(&base, None, &lang).into_string();
        assert_eq!(out.matches("<button").count(), 4);

        let cfg = ResourceListCfg {
            order_options: &DECLARED_ROWS,
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        let out = render_order_control(&cfg, None, &lang).into_string();
        assert_eq!(out.matches("<button").count(), 6);
        let id = out.find(r#"value="id""#).expect("id row");
        let date = out.find(r#"value="date""#).expect("date row");
        let extent = out.find(r#"value="extent""#).expect("extent row");
        assert!(
            id < date && date < extent,
            "built-ins, then declared rows in config order: {out}"
        );
    }

    /// A declared key marks its own row `aria-current` (and only that row),
    /// and the summary/status read its label; the control is `data-active`.
    #[test]
    fn order_control_marks_declared_row_aria_current() {
        let lang = make_lang("en");
        let cfg = ResourceListCfg {
            order_options: &DECLARED_ROWS,
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        let out = render_order_control(&cfg, Some("extent"), &lang).into_string();
        assert_eq!(out.matches(r#"aria-current="true""#).count(), 1);
        assert!(
            out.contains(r#"value="extent" aria-current="true""#),
            "the declared row must carry aria-current, got: {out}"
        );
        assert!(
            out.contains(r#"aria-label="Order: Extent""#),
            "the summary must name the declared ordering, got: {out}"
        );
        assert!(out.contains(r#"data-active="true""#), "got: {out}");
    }

    /// The hidden inputs still drop `sort`/`page` when the active sort is a
    /// declared key.
    #[test]
    fn order_control_hidden_inputs_exclude_sort_and_page_with_declared_key() {
        let lang = make_lang("en");
        let cfg = ResourceListCfg {
            order_options: &DECLARED_ROWS,
            ..make_sort_cfg("/incunabula/books", Some("role=n1&sort=date&page=3"), Some("book-list"))
        };
        let out = render_order_control(&cfg, Some("date"), &lang).into_string();
        assert!(out.contains(r#"type="hidden" name="role" value="n1""#), "got: {out}");
        assert!(
            !out.contains(r#"type="hidden" name="sort""#) && !out.contains(r#"type="hidden" name="page""#),
            "sort/page must never be preserved as hidden inputs, got: {out}"
        );
    }

    // ── Strengthened pager inert-end assertions ───────────────────────────────

    /// First page: PREV end must be an inert `<span aria-disabled="true">` AND
    /// NEXT end must be an active `<a href=...>`. Fails if both ends are inert.
    #[test]
    fn pager_first_page_prev_inert_and_next_active() {
        let routes = make_routes();
        static ROUTES_FP: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_FP.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 50 total, 5 per page → 10 pages; current = page 1.
        let list = make_list_page(items, 50, 1, Pagination::PerPage(5));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // PREV must be an inert span.
        assert!(
            out.contains("span") && out.contains("aria-disabled=\"true\""),
            "first page: prev must be an inert span with aria-disabled, got: {out}"
        );

        // NEXT must be an active <a> with href pointing to page 2.
        assert!(
            out.contains("href=\"/alice-in-daschland/characters?page=2\""),
            "first page: next must be an active <a> pointing to page 2, got: {out}"
        );

        // Negative: must NOT have a href pointing to page 0 (prev would be nonsense).
        assert!(!out.contains("page=0"), "first page: must have no href with page=0, got: {out}");
    }

    /// Last page: NEXT end must be an inert `<span aria-disabled="true">` AND
    /// PREV end must be an active `<a href=...>`. Fails if both ends are inert.
    #[test]
    fn pager_last_page_next_inert_and_prev_active() {
        let routes = make_routes();
        static ROUTES_LP: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_LP.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);

        let items: Vec<Resource> = (1..=5)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        // 10 total, 5 per page → 2 pages; current = page 2 (last).
        let list = make_list_page(items, 10, 2, Pagination::PerPage(5));
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: Some("lang=en"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();

        // NEXT must be an inert span.
        assert!(
            out.contains("span") && out.contains("aria-disabled=\"true\""),
            "last page: next must be an inert span with aria-disabled, got: {out}"
        );

        // PREV must be an active <a> pointing to page 1 (with lang preserved).
        // Maud HTML-escapes `&` to `&amp;` in attribute values.
        assert!(
            out.contains("href=\"/alice-in-daschland/characters?lang=en&amp;page=1\""),
            "last page: prev must be an active <a> pointing to page 1 with lang=en preserved, got: {out}"
        );

        // Negative: must NOT have a href pointing to page 3 (next would be beyond last).
        assert!(!out.contains("page=3"), "last page: must have no href with page=3, got: {out}");
    }

    // ── Gloss + chips (annotated rows) ───────────────────────────────────

    /// A Character-like resource with a richtext `description` (gloss source), a
    /// single-valued `role` list value (→ `.term`), and a multi-valued `keyword`
    /// list (→ `.keyword` chips).
    fn make_annotated_resource(id: &str, name: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: "Character".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: name.to_owned(), lang: Some("en".to_owned()) }],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![PropertyValue::Richtext {
                        text: "A curious girl in a world of nonsense.".to_owned(),
                        lang: Some("en".to_owned()),
                    }],
                },
                Property {
                    name: "role".to_owned(),
                    values: vec![PropertyValue::ListValue {
                        node_id: "R_01".to_owned(),
                        label: Some("Main Character".to_owned()),
                    }],
                },
                Property {
                    name: "keyword".to_owned(),
                    values: vec![
                        PropertyValue::ListValue {
                            node_id: "K_01".to_owned(),
                            label: Some("Surrealism".to_owned()),
                        },
                        PropertyValue::ListValue {
                            node_id: "K_02".to_owned(),
                            label: Some("Nonsense".to_owned()),
                        },
                    ],
                },
            ],
            representations: vec![],
        }
    }

    /// `row_gloss_text` returns the first text/richtext value of the gloss
    /// property, language-picked.
    #[test]
    fn row_gloss_text_returns_first_text_value() {
        let r = make_annotated_resource("CH_001", "Alice");
        let lang = make_lang("en");
        assert_eq!(
            row_gloss_text(&r, Some(&ResolvedGloss::Property("description")), &lang).as_deref(),
            Some("A curious girl in a world of nonsense.")
        );
    }

    /// No gloss property configured, or the property absent → `None` (never the
    /// id fallback the title uses).
    #[test]
    fn row_gloss_text_none_when_absent_or_unconfigured() {
        let r = make_resource("CH_001", "Character", "Alice"); // name only
        let lang = make_lang("en");
        assert_eq!(row_gloss_text(&r, Some(&ResolvedGloss::Property("description")), &lang), None);
        assert_eq!(row_gloss_text(&r, None, &lang), None);
    }

    /// (g) A `date` gloss property renders
    /// `format_date_text` — the plain date, the degenerate range collapsed,
    /// the display string honoured — and NEVER the precision word.
    #[test]
    fn row_gloss_text_renders_a_date_value_as_plain_text() {
        use cpe_core::ir::{Calendar, DatePrecision, Property};
        let mut r = make_resource("B1", "Book", "Narrenschiff");
        r.properties.push(Property {
            name: "pub-date".to_owned(),
            values: vec![PropertyValue::Date {
                start: "1494-02-11".to_owned(),
                end: Some("1494-02-11".to_owned()),
                precision: DatePrecision::Day,
                display: Some("11. Februar 1494".to_owned()),
                display_lang: Some("de".to_owned()),
                calendar: Some(Calendar::Gregorian),
            }],
        });
        assert_eq!(
            row_gloss_text(&r, Some(&ResolvedGloss::Property("pub-date")), &make_lang("de")).as_deref(),
            Some("11. Februar 1494")
        );
        assert_eq!(
            row_gloss_text(&r, Some(&ResolvedGloss::Property("pub-date")), &make_lang("en")).as_deref(),
            Some("11 February 1494")
        );

        // The rendered row carries the gloss as plain text — no `span.prec`.
        static ROUTES: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes = ROUTES.get_or_init(make_routes);
        let cfg = ResourceListCfg {
            gloss: Some(&ResolvedGloss::Property("pub-date")),
            ..base_cfg(make_resolver("incunabula", routes), "/incunabula/books", None)
        };
        let row = render_row(&r, &cfg, &make_lang("de")).into_string();
        assert!(row.contains("<p class=\"rl-annot__gloss\">11. Februar 1494</p>"), "{row}");
        assert!(!row.contains("prec"), "{row}");
    }

    // ── Composed gloss ────────────────────────────────────────────────────────

    fn text(value: &str, lang: Option<&str>) -> PropertyValue {
        PropertyValue::Text { text: value.to_owned(), lang: lang.map(str::to_owned) }
    }

    /// A Book with a two-valued `publisher`, a `pub-place`, a curated-display
    /// `pub-date`, an integer `extent` and a DE/EN `teaser`.
    fn make_book() -> Resource {
        use cpe_core::ir::{Calendar, DatePrecision};
        let mut r = make_resource("bereitung", "Book", "Bereitung");
        r.properties.extend([
            Property {
                name: "publisher".to_owned(),
                values: vec![text("Johann Amerbach", None), text("Johann Petri de Langendorff", None)],
            },
            Property {
                name: "pub-place".to_owned(),
                values: vec![text("Basel", None)],
            },
            Property {
                name: "pub-date".to_owned(),
                values: vec![PropertyValue::Date {
                    start: "1489".to_owned(),
                    end: Some("1489".to_owned()),
                    precision: DatePrecision::Year,
                    display: Some("1489".to_owned()),
                    display_lang: Some("de".to_owned()),
                    calendar: Some(Calendar::Julian),
                }],
            },
            Property {
                name: "extent".to_owned(),
                values: vec![PropertyValue::Integer(511)],
            },
            Property {
                name: "teaser".to_owned(),
                values: vec![
                    text("Der umfangreichste Band.", Some("de")),
                    text("The largest volume.", Some("en")),
                ],
            },
        ]);
        r
    }

    const UNIT: Option<&str> = Some("{n} Seiten");

    fn book_parts(unit: Option<&'static str>) -> Vec<ResolvedGlossPart<'static>> {
        ["publisher", "pub-place", "pub-date", "extent"]
            .into_iter()
            .map(|property| ResolvedGlossPart {
                property,
                unit: (property == "extent").then_some(unit).flatten(),
            })
            .collect()
    }

    fn composed(unit: Option<&'static str>, teaser: Option<&'static str>) -> ResolvedGloss<'static> {
        ResolvedGloss::Composed { parts: book_parts(unit), teaser }
    }

    fn gloss_line(r: &Resource, gloss: &ResolvedGloss<'_>, lang: &str) -> Option<(String, bool)> {
        row_gloss(r, Some(gloss), &make_lang(lang)).map(|g| (g.text.into_owned(), g.has_teaser))
    }

    #[test]
    fn composed_part_texts_appends_the_gregorian_word_at_day_and_month_only() {
        use cpe_core::ir::{Calendar, DatePrecision, Property};
        let date = |start: &str, precision| Property {
            name: "pub-date".to_owned(),
            values: vec![PropertyValue::Date {
                start: start.to_owned(),
                end: None,
                precision,
                display: None,
                display_lang: None,
                calendar: Some(Calendar::Gregorian),
            }],
        };
        let parts = [ResolvedGlossPart { property: "pub-date", unit: None }];
        let en = make_lang("en");
        for (start, precision, want) in [
            ("1494-02-11", DatePrecision::Day, "1494-02-11 · Gregorian"),
            ("1494-02", DatePrecision::Month, "1494-02 · Gregorian"),
            ("1494", DatePrecision::Year, "1494"),
        ] {
            let mut r = make_resource("B1", "Book", "T");
            r.properties.push(date(start, precision));
            assert_eq!(composed_part_texts(&r, &parts, true, &en), vec![want]);
            assert_eq!(composed_part_texts(&r, &parts, false, &en), vec![start]);
        }
    }

    #[test]
    fn composed_gloss_joins_every_part_and_every_value() {
        assert_eq!(
            gloss_line(&make_book(), &composed(UNIT, None), "de"),
            Some((
                "Johann Amerbach, Johann Petri de Langendorff · Basel · 1489 · 511 Seiten".to_owned(),
                false
            ))
        );
    }

    #[test]
    fn composed_gloss_skips_a_missing_part_without_a_doubled_separator() {
        let mut r = make_book();
        r.properties.retain(|p| p.name != "pub-place");
        assert_eq!(
            gloss_line(&r, &composed(UNIT, None), "de").map(|(t, _)| t),
            Some("Johann Amerbach, Johann Petri de Langendorff · 1489 · 511 Seiten".to_owned())
        );
    }

    #[test]
    fn composed_gloss_integer_with_and_without_a_unit() {
        let only_extent = |unit| ResolvedGloss::Composed {
            parts: vec![ResolvedGlossPart { property: "extent", unit }],
            teaser: None,
        };
        assert_eq!(
            gloss_line(&make_book(), &only_extent(UNIT), "de").map(|(t, _)| t),
            Some("511 Seiten".to_owned())
        );
        assert_eq!(
            gloss_line(&make_book(), &only_extent(None), "de").map(|(t, _)| t),
            Some("511".to_owned()),
            "no unit template (none declared, or none for this language) ⇒ bare digits"
        );
    }

    #[test]
    fn composed_gloss_appends_the_teaser_in_the_request_language() {
        assert_eq!(
            gloss_line(&make_book(), &composed(UNIT, Some("teaser")), "de"),
            Some((
                "Johann Amerbach, Johann Petri de Langendorff · Basel · 1489 · 511 Seiten \
                 — Der umfangreichste Band."
                    .to_owned(),
                true
            ))
        );
        let (en, _) = gloss_line(&make_book(), &composed(Some("{n} pages"), Some("teaser")), "en").expect("renders");
        assert!(
            en.ends_with("511 pages — The largest volume."),
            "the EN request picks the en teaser: {en}"
        );
    }

    #[test]
    fn composed_gloss_teaser_stands_alone_when_no_part_renders() {
        let gloss = ResolvedGloss::Composed {
            parts: vec![ResolvedGlossPart { property: "no-such-part", unit: None }],
            teaser: Some("teaser"),
        };
        assert_eq!(
            gloss_line(&make_book(), &gloss, "de"),
            Some(("Der umfangreichste Band.".to_owned(), true))
        );
    }

    #[test]
    fn composed_gloss_renders_nothing_when_nothing_is_present() {
        let r = make_resource("x", "Book", "X");
        assert_eq!(gloss_line(&r, &composed(UNIT, Some("teaser")), "de"), None);
        let row = render_row(
            &r,
            &ResourceListCfg {
                gloss: Some(&composed(UNIT, Some("teaser"))),
                links: None,
                ..make_sort_cfg("/b", None, None)
            },
            &make_lang("de"),
        )
        .into_string();
        assert!(!row.contains("rl-annot__gloss"), "no <p> at all: {row}");
    }

    /// The legacy form renders the FIRST value of a multi-valued property;
    /// the composed form joins them all. Both behaviours are deliberate.
    #[test]
    fn legacy_gloss_first_value_only_composed_joins_all() {
        let r = make_book();
        assert_eq!(
            gloss_line(&r, &ResolvedGloss::Property("publisher"), "de"),
            Some(("Johann Amerbach".to_owned(), false))
        );
        let joined = ResolvedGloss::Composed {
            parts: vec![ResolvedGlossPart { property: "publisher", unit: None }],
            teaser: None,
        };
        assert_eq!(
            gloss_line(&r, &joined, "de").map(|(t, _)| t),
            Some("Johann Amerbach, Johann Petri de Langendorff".to_owned())
        );
    }

    /// A teaser containing " — " and "&" is escaped exactly once and its dash
    /// is not mistaken for the separator; only a row whose teaser rendered
    /// carries the 3-line clamp modifier.
    #[test]
    fn composed_gloss_row_markup_escapes_once_and_flags_the_teaser() {
        let mut r = make_book();
        r.properties.retain(|p| p.name != "teaser");
        r.properties.push(Property {
            name: "teaser".to_owned(),
            values: vec![text("Holz & Stock — zweimal.", Some("de"))],
        });
        let row_with = |gloss: &ResolvedGloss<'_>| {
            render_row(
                &r,
                &ResourceListCfg {
                    gloss: Some(gloss),
                    links: None,
                    ..make_sort_cfg("/b", None, None)
                },
                &make_lang("de"),
            )
            .into_string()
        };
        let with_teaser = row_with(&composed(UNIT, Some("teaser")));
        assert!(
            with_teaser.contains(
                r#"<p class="rl-annot__gloss rl-annot__gloss--teaser">Johann Amerbach, Johann Petri de Langendorff · Basel · 1489 · 511 Seiten — Holz &amp; Stock — zweimal.</p>"#
            ),
            "{with_teaser}"
        );
        let without = row_with(&composed(UNIT, None));
        assert!(
            without.contains(r#"<p class="rl-annot__gloss">"#) && !without.contains("--teaser"),
            "{without}"
        );
    }

    /// A grid text tile shows the legacy gloss only — never a composed line.
    #[test]
    fn grid_text_tile_renders_no_composed_gloss() {
        let r = make_book();
        let tile = |gloss: &ResolvedGloss<'_>| {
            render_grid_tile(
                &r,
                &ResourceListCfg {
                    gloss: Some(gloss),
                    links: None,
                    ..make_sort_cfg("/b", None, None)
                },
                &make_lang("de"),
            )
            .into_string()
        };
        let composed_tile = tile(&composed(UNIT, Some("teaser")));
        assert!(
            composed_tile.contains("gm__box--text") && !composed_tile.contains(r#"class="g""#),
            "{composed_tile}"
        );
        let legacy_tile = tile(&ResolvedGloss::Property("pub-place"));
        assert!(legacy_tile.contains(r#"<span class="g">Basel</span>"#), "{legacy_tile}");
    }

    #[test]
    fn snapshot_resource_list_composed_gloss_with_teaser() {
        let list = make_list_page(vec![make_book()], 1, 1, Pagination::PerPage(24));
        let gloss = composed(UNIT, Some("teaser"));
        let cfg = ResourceListCfg {
            gloss: Some(&gloss),
            links: None,
            class: "Book",
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        insta::assert_snapshot!(render_resource_list(&list, &cfg, &make_lang("de")).into_string());
    }

    #[test]
    fn snapshot_resource_list_composed_gloss_without_teaser() {
        let list = make_list_page(vec![make_book()], 1, 1, Pagination::PerPage(24));
        let gloss = composed(UNIT, None);
        let cfg = ResourceListCfg {
            gloss: Some(&gloss),
            links: None,
            class: "Book",
            ..make_sort_cfg("/incunabula/books", None, Some("book-list"))
        };
        insta::assert_snapshot!(render_resource_list(&list, &cfg, &make_lang("de")).into_string());
    }

    /// A single list value renders as `.term`; a multi-valued list renders as a
    /// run of `.keyword` chips (path/leaf split, flat labels are leaf-only).
    #[test]
    fn chips_single_renders_term_multi_renders_keywords() {
        let r = make_annotated_resource("CH_001", "Alice");
        let lang = make_lang("en");
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert!(
            out.contains(r#"<div class="rl-annot__meta">"#),
            "chip row container must render, got: {out}"
        );
        assert!(
            out.contains(r#"<span class="term">Main Character</span>"#),
            "single list value (role) must render as a .term chip, got: {out}"
        );
        assert!(
            out.contains(r#"<span class="keyword"><span class="leaf">Surrealism</span></span>"#),
            "multi list value (keyword) must render as .keyword/.leaf chips, got: {out}"
        );
        assert!(
            out.contains("Nonsense"),
            "every keyword value must render as a chip, got: {out}"
        );
    }

    /// No chip property present on the resource → empty markup (no empty
    /// `.rl-annot__meta` box).
    #[test]
    fn chips_empty_when_no_chip_property_present() {
        let r = make_resource("CH_001", "Character", "Alice"); // no role/keyword
        let lang = make_lang("en");
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert!(out.is_empty(), "no chip values → no .rl-annot__meta emitted, got: {out}");
    }

    /// Themes beyond `MAX_THEME_CHIPS` collapse into a trailing `+N`; the role
    /// `.term` always shows and never counts toward the cap.
    #[test]
    fn chips_cap_themes_and_render_plus_n() {
        let mut r = make_annotated_resource("CH_001", "Alice");
        // Replace the 2-keyword set with 5 so the cap (3) bites: 3 shown, +2.
        for p in &mut r.properties {
            if p.name == "keyword" {
                p.values = (1..=5)
                    .map(|i| PropertyValue::ListValue {
                        node_id: format!("K_{i:02}"),
                        label: Some(format!("Theme {i}")),
                    })
                    .collect();
            }
        }
        let lang = make_lang("en");
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let out = render_row_chips(&r, &chips, &lang).into_string();

        // Role term always present.
        assert!(
            out.contains(r#"<span class="term">Main Character</span>"#),
            "role .term must always render, got: {out}"
        );
        // Exactly MAX_THEME_CHIPS (3) keyword chips rendered.
        assert_eq!(
            out.matches(r#"<span class="keyword">"#).count(),
            3,
            "themes must be capped at MAX_THEME_CHIPS (3), got: {out}"
        );
        // The overflow (5 - 3 = 2) renders as +2.
        assert!(
            out.contains(r#"<span class="rl-annot__more">+2</span>"#),
            "the 2 hidden themes must collapse into a +2 overflow chip, got: {out}"
        );
    }

    /// At or below the cap, no `+N` overflow chip is emitted.
    #[test]
    fn chips_no_overflow_when_within_cap() {
        let r = make_annotated_resource("CH_001", "Alice"); // 1 role + 2 keywords
        let lang = make_lang("en");
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert!(
            !out.contains("rl-annot__more"),
            "2 themes (≤ cap) must not produce a +N overflow chip, got: {out}"
        );
    }

    /// An explicit `kind: Some(Keyword)` on a single-valued property
    /// overrides the cardinality rule: the one label still renders as
    /// `.keyword`, not `.term`.
    #[test]
    fn chips_explicit_keyword_kind_overrides_single_value_cardinality() {
        let r = make_annotated_resource("CH_001", "Alice"); // role has exactly 1 value
        let lang = make_lang("en");
        let chips = vec![ChipSpec { property: "role".to_owned(), kind: Some(ChipKind::Keyword) }];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert!(
            out.contains(r#"<span class="keyword">"#),
            "kind: Some(Keyword) on a single-valued property must render .keyword, got: {out}"
        );
        assert!(
            !out.contains(r#"<span class="term">"#),
            "kind: Some(Keyword) must never fall back to .term, got: {out}"
        );
    }

    /// An explicit `kind: Some(Term)` on a multi-valued property
    /// overrides the cardinality rule for EVERY label — N `.term` chips, not
    /// a run of `.keyword`s.
    #[test]
    fn chips_explicit_term_kind_overrides_multi_value_cardinality() {
        let r = make_annotated_resource("CH_001", "Alice"); // keyword has 2 values
        let lang = make_lang("en");
        let chips = vec![ChipSpec { property: "keyword".to_owned(), kind: Some(ChipKind::Term) }];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert_eq!(
            out.matches(r#"<span class="term">"#).count(),
            2,
            "kind: Some(Term) on a multi-valued property must render one .term \
             per label (not 'first only'), got: {out}"
        );
        assert!(
            !out.contains(r#"<span class="keyword">"#),
            "kind: Some(Term) must never fall back to .keyword, got: {out}"
        );
    }

    /// `kind: None` is the existing cardinality rule, unchanged — a
    /// regression guard so the `kind` override cannot silently move today's default
    /// behaviour (assert the exact markup, not just the enum variant).
    #[test]
    fn chips_none_kind_keeps_todays_cardinality_rule() {
        let r = make_annotated_resource("CH_001", "Alice");
        let lang = make_lang("en");
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let out = render_row_chips(&r, &chips, &lang).into_string();
        assert!(
            out.contains(r#"<span class="term">Main Character</span>"#),
            "single-valued role must still render .term with kind: None, got: {out}"
        );
        assert!(
            out.contains(r#"<span class="keyword"><span class="leaf">Surrealism</span></span>"#),
            "multi-valued keyword must still render .keyword with kind: None, got: {out}"
        );
    }

    /// (h) Full annotated row through the public renderer: name + gloss +
    /// role (.term) + keyword (.keyword) chips.
    #[test]
    fn snapshot_resource_list_annotated_rows_gloss_and_chips() {
        let items = vec![make_annotated_resource("CH_001", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let chips = vec![
            ChipSpec { property: "role".to_owned(), kind: None },
            ChipSpec { property: "keyword".to_owned(), kind: None },
        ];
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &chips,
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"<p class="rl-annot__gloss">A curious girl in a world of nonsense.</p>"#),
            "the gloss line must render inside .rl-annot__main, got: {out}"
        );
        assert!(
            out.contains(r#"<div class="rl-annot__meta">"#),
            "the chip row must render, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Combined, through the public renderer: a `kind:
    /// Some(Keyword)` chip override on a single-valued property (HdM's
    /// `source` — one label rendering `.keyword`, not `.term`), plus both
    /// ordering-label overrides set (HdM's "Zeile A–Z"/"Zeile Z–A"). This
    /// is the only chip/order snapshot that
    /// exercises these attributes.
    #[test]
    fn snapshot_resource_list_chip_kind_override_and_order_label_overrides() {
        let items = vec![make_annotated_resource("CH_001", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let chips = vec![ChipSpec { property: "role".to_owned(), kind: Some(ChipKind::Keyword) }];
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/hdm/performances",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &chips,
            class: "Performance",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: Some("Zeile A–Z"),
            order_name_desc_label: Some("Zeile Z–A"),
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"<span class="keyword">"#),
            "the single-valued role chip must render .keyword under the kind \
             override, got: {out}"
        );
        assert!(
            out.contains("Zeile A\u{2013}Z") && out.contains("Zeile Z\u{2013}A"),
            "both order-label overrides must render, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    // ── Config-declared class labels (surf-head title) ───────────────────────

    /// A declared `labels { class "…" { … } }` translation is used for the
    /// surf-head `span.surf-head__title`, in preference to the raw/humanized
    /// class name — the resolver call site in
    /// `render_surf_head`. The declared translation ("Cast Member") is
    /// deliberately different from both the raw class name ("Character") and
    /// its humanized form (a no-op here, since "Character" has no
    /// `-`/`_`/` ` separator), so a regression to either fallback is caught.
    #[test]
    fn snapshot_resource_list_surf_head_uses_declared_class_label() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let class_labels = vec![cpe_core::ir::ClassLabel {
            class: "Character".to_owned(),
            translations: vec![("en".to_owned(), "Cast Member".to_owned())],
            plural: vec![],
        }];
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &class_labels,
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"<span class="surf-head__title">Cast Member</span>"#),
            "the declared class-label translation must render as the surf-head title, \
             got: {out}"
        );
        assert!(
            !out.contains(">Character<"),
            "the raw/humanized class name must NOT appear when a translation is declared, \
             got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    // ── Search-aware rendering (SearchSurface) ───────

    /// Build a `SearchSurface` for a given (already-active) query —
    /// the populated-list scope note (the empty-branch fields are on
    /// `NarrowedSurface`, see `make_narrowed_surface` below).
    fn make_search_surface(q: &str) -> SearchSurface<'_> {
        SearchSurface::for_query(q, "matching Name and Description")
            .expect("test helper is only ever called with an active query")
    }

    /// Build a `NarrowedSurface` — the
    /// literal stand-in for what `SearchFragmentSurface::narrowed_surface`
    /// would have resolved. `query_active` gates the hint/clear-search fields
    /// exactly like `NarrowedSurface::build` does for real, AND selects
    /// between the two message stand-ins (so a filters-only test doesn't
    /// render a message that references a query it doesn't have);
    /// `clear_all_href` gates the "filters active" side.
    fn make_narrowed_surface<'a>(query_active: bool, clear_all_href: Option<&'a str>) -> NarrowedSurface<'a> {
        let no_match_msg = if query_active {
            html! {
                "No characters match "
                em { "\"xyzzy\"" }
                "."
            }
        } else {
            html! {
                "No locations match these filters."
            }
        };
        NarrowedSurface::build(
            query_active,
            no_match_msg,
            "Matching is on the beginning of words — try a shorter term.",
            "/alice-in-daschland/characters",
            clear_all_href,
        )
        .expect("test helper is only ever called with an active query or an active filter")
    }

    /// Active-query MATCH: `cfg.search: Some`, `list.items` non-empty — the
    /// count + scope note render, and `#browse-count`'s `id` is present.
    #[test]
    fn snapshot_resource_list_search_active_match() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let mut list = make_list_page(items, 3, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(20);
        let surface = make_search_surface("rab");
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("q=rab"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: Some(surface),
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"id="browse-count""#),
            "the count element must carry id=\"browse-count\", got: {out}"
        );
        assert!(
            out.contains(
                "<span class=\"surf-head__sep\" aria-hidden=\"true\">\u{b7}</span><span class=\"search-scope\">matching Name and Description</span>"
            ),
            "the separator + scope note must render right after the count, got: {out}"
        );
        assert!(
            !out.contains("search-empty"),
            "a matching search must NOT render .search-empty, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// The `span.surf-head__sep[aria-hidden]` separator is present IFF the
    /// scope note renders — never emitted when there is no scope note.
    #[test]
    fn surf_head_separator_present_iff_scope_note_present() {
        let items = vec![make_resource("CH_001", "Character", "Alice")];
        let list = make_list_page(items, 1, 1, Pagination::PerPage(24));
        let lang = make_lang("en");

        // With a non-empty scope note: the separator renders.
        let surface_with_note =
            SearchSurface::for_query("rab", "matching Name and Description").expect("active query with a scope note");
        let cfg_with_note = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("q=rab"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: Some(surface_with_note),
            narrowed: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let out_with_note = render_resource_list(&list, &cfg_with_note, &lang).into_string();
        assert!(
            out_with_note.contains(r#"class="surf-head__sep" aria-hidden="true""#),
            "a rendered scope note must be preceded by the surf-head__sep, got: {out_with_note}"
        );

        // With an empty scope note: neither the note nor the separator render.
        let surface_no_note = SearchSurface::for_query("rab", "").expect("active query with an empty scope note");
        let cfg_no_note = ResourceListCfg { search: Some(surface_no_note), ..cfg_with_note };
        let out_no_note = render_resource_list(&list, &cfg_no_note, &lang).into_string();
        assert!(
            !out_no_note.contains("surf-head__sep"),
            "an empty scope note must NOT render the separator either, got: {out_no_note}"
        );
        assert!(
            !out_no_note.contains("search-scope"),
            "an empty scope note must NOT render, got: {out_no_note}"
        );

        // With no search surface at all: neither renders (existing behavior).
        let cfg_none = ResourceListCfg { search: None, ..cfg_no_note };
        let out_none = render_resource_list(&list, &cfg_none, &lang).into_string();
        assert!(
            !out_none.contains("surf-head__sep"),
            "no search surface must NOT render the separator, got: {out_none}"
        );
    }

    /// Active-query NO-MATCH, faceted (`clear_all_href: Some`): `.search-empty`
    /// renders msg/hint/BOTH clear actions; `.resource-list__empty` does NOT.
    #[test]
    fn snapshot_resource_list_search_no_match_with_facets() {
        let mut list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(20);
        let surface = make_narrowed_surface(true, Some("/alice-in-daschland/characters?role=R_01"));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("q=xyzzy&role=R_01"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: Some(surface),
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="search-empty""#),
            "an active query with zero matches must render .search-empty, got: {out}"
        );
        assert!(
            !out.contains("resource-list__empty"),
            "must NOT fall back to .resource-list__empty, got: {out}"
        );
        assert!(
            out.contains(r#"<p class="search-empty__msg">No characters match <em>&quot;xyzzy&quot;</em>.</p>"#),
            "the no-match message must render with the query inside <em> \
             (Maud-escaped), got: {out}"
        );
        assert!(out.contains("search-empty__hint"), "the mono hint must render, got: {out}");
        assert_eq!(
            out.matches(r#"class="search-empty__act""#).count(),
            2,
            "with a facet active, BOTH Clear search and Clear all actions must render, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Active-query NO-MATCH, non-faceted (`clear_all_href: None`): only the
    /// "Clear search" action renders — never "Clear all" (that
    /// action only appears when facets are on).
    #[test]
    fn snapshot_resource_list_search_no_match_no_facets() {
        let mut list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(13);
        let surface = make_narrowed_surface(true, None);
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/chapters",
            current_query: Some("q=xyzzy"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: Some(surface),
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="search-empty""#),
            "an active query with zero matches must render .search-empty, got: {out}"
        );
        assert_eq!(
            out.matches(r#"class="search-empty__act""#).count(),
            1,
            "without a facet, only ONE action (Clear search) must render, got: {out}"
        );
        assert!(
            out.contains(&format!(">{}</a>", ChromeMsg::SearchClear.text(&lang))),
            "the sole action must be Clear search, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    // ── Grid mode ─────────────────────────────────────

    fn make_grid_cfg<'a>(
        current_path: &'a str,
        current_query: Option<&'a str>,
        grid: bool,
        media_thumbs: &'a HashMap<String, MediaThumb>,
    ) -> ResourceListCfg<'a> {
        ResourceListCfg {
            links: None,
            current_path,
            current_query,
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs,
            grid,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        }
    }

    // ── Unit: parse_view_param ────────────────────────────────────────────────

    #[test]
    fn parse_view_param_grid_value_maps_to_grid() {
        assert_eq!(parse_view_param(Some("view=grid")), ViewMode::Grid);
    }

    #[test]
    fn parse_view_param_absent_maps_to_list() {
        assert_eq!(parse_view_param(None), ViewMode::List);
    }

    #[test]
    fn parse_view_param_empty_value_maps_to_list() {
        assert_eq!(parse_view_param(Some("view=")), ViewMode::List);
    }

    #[test]
    fn parse_view_param_garbage_value_maps_to_list() {
        assert_eq!(parse_view_param(Some("view=bogus")), ViewMode::List);
    }

    #[test]
    fn parse_view_param_key_absent_from_other_params_maps_to_list() {
        assert_eq!(parse_view_param(Some("lang=de&sort=name")), ViewMode::List);
    }

    #[test]
    fn parse_view_param_preserves_view_among_other_params() {
        assert_eq!(parse_view_param(Some("role=n1&view=grid&sort=name")), ViewMode::Grid);
    }

    // ── Unit: drop_query_param ────────────────────────────────────────────────

    #[test]
    fn drop_query_param_removes_only_named_key() {
        let qs = drop_query_param(Some("lang=en&view=grid&sort=name"), "view");
        assert_eq!(qs, Some("lang=en&sort=name".to_owned()));
    }

    #[test]
    fn drop_query_param_returns_none_when_query_empties() {
        assert_eq!(drop_query_param(Some("view=grid"), "view"), None);
    }

    #[test]
    fn drop_query_param_none_query_returns_none() {
        assert_eq!(drop_query_param(None, "view"), None);
    }

    #[test]
    fn drop_query_param_key_absent_preserves_full_query() {
        let qs = drop_query_param(Some("lang=en&sort=name"), "view");
        assert_eq!(qs, Some("lang=en&sort=name".to_owned()));
    }

    // ── Unit: view toggle markup + href preservation ──────────────────────────

    #[test]
    fn view_toggle_not_rendered_when_grid_false() {
        let cfg = make_grid_cfg("/alice/characters", None, false, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_surf_head(
            &make_list_page(
                vec![make_resource("CH_001", "Character", "Alice")],
                1,
                1,
                Pagination::PerPage(24),
            ),
            &cfg,
            ViewMode::List,
            &lang,
        )
        .into_string();
        assert!(
            !out.contains("class=\"viewtoggle\""),
            "grid: false must never render the view toggle, got: {out}"
        );
    }

    #[test]
    fn view_toggle_rendered_when_grid_true_with_role_group_and_labelledby() {
        let cfg = make_grid_cfg("/alice/characters", None, true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains(r#"role="group""#) && out.contains(r#"aria-labelledby="viewtoggle-lbl""#),
            "the toggle opts group must carry role=group + aria-labelledby, got: {out}"
        );
        assert!(
            out.contains(r#"id="viewtoggle-lbl""#),
            "the visible label must carry the fixed id, got: {out}"
        );
    }

    #[test]
    fn view_toggle_list_option_current_when_view_is_list() {
        let cfg = make_grid_cfg("/alice/characters", None, true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::List, &lang).into_string();
        let list_pos = out.find("List").expect("List option label present");
        let aria_current_count = out.matches(r#"aria-current="true""#).count();
        assert_eq!(aria_current_count, 1, "exactly one option must carry aria-current, got: {out}");
        // The List option (earlier in source order) must be the one carrying it.
        let aria_pos = out.find(r#"aria-current="true""#).unwrap();
        assert!(
            aria_pos < list_pos,
            "aria-current=true must be on the List <a>, not the Grid one, got: {out}"
        );
    }

    #[test]
    fn view_toggle_grid_option_current_when_view_is_grid() {
        let cfg = make_grid_cfg("/alice/characters", None, true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::Grid, &lang).into_string();
        let grid_pos = out.rfind("Grid").expect("Grid option label present");
        let aria_pos = out.find(r#"aria-current="true""#).unwrap();
        assert!(aria_pos < grid_pos, "aria-current=true must be on the Grid <a>, got: {out}");
    }

    #[test]
    fn view_toggle_list_href_drops_view_and_preserves_other_params() {
        let cfg = make_grid_cfg("/alice/characters", Some("role=n1&view=grid&sort=name"), true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::Grid, &lang).into_string();
        assert!(
            out.contains(r#"href="/alice/characters?role=n1&amp;sort=name""#),
            "the List option href must drop view but preserve role/sort, got: {out}"
        );
    }

    #[test]
    fn view_toggle_list_href_is_bare_path_when_view_is_only_param() {
        let cfg = make_grid_cfg("/alice/characters", Some("view=grid"), true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::Grid, &lang).into_string();
        assert!(
            out.contains(r#"href="/alice/characters""#),
            "the List option href must be the bare path when view was the only param, \
             got: {out}"
        );
        assert!(
            !out.contains("href=\"/alice/characters?\""),
            "the List option href must NEVER have a trailing bare '?', got: {out}"
        );
    }

    #[test]
    fn view_toggle_grid_href_sets_view_grid_and_preserves_other_params() {
        let cfg = make_grid_cfg("/alice/characters", Some("role=n1&sort=name"), true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_view_toggle(&cfg, ViewMode::List, &lang).into_string();
        assert!(
            out.contains(r#"href="/alice/characters?role=n1&amp;sort=name&amp;view=grid""#),
            "the Grid option href must set view=grid and preserve role/sort, got: {out}"
        );
    }

    // ── Unit: grid-mode surf-head/toggle wiring via render_resource_list ──────

    #[test]
    fn resource_list_view_ignored_when_grid_false() {
        // `?view=grid` in the query but `cfg.grid: false` must still render
        // rows — a grid: false list can never render tiles.
        let cfg = make_grid_cfg("/alice/characters", Some("view=grid"), false, &EMPTY_THUMBS);
        let list = make_list_page(
            vec![make_resource("CH_001", "Character", "Alice")],
            1,
            1,
            Pagination::PerPage(24),
        );
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="rl-annot""#),
            "grid: false must render rows even with ?view=grid, got: {out}"
        );
        assert!(
            !out.contains(r#"class="gm""#),
            "grid: false must never render tiles, got: {out}"
        );
        assert!(
            !out.contains("class=\"viewtoggle\""),
            "grid: false must never render the toggle, got: {out}"
        );
    }

    #[test]
    fn resource_list_renders_grid_tiles_when_grid_true_and_view_grid() {
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = make_grid_cfg("/alice/characters", Some("view=grid"), true, &thumbs);
        let list = make_list_page(
            vec![make_resource("CH_001", "Character", "Alice")],
            1,
            1,
            Pagination::PerPage(24),
        );
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="gm""#),
            "grid: true + view=grid must render ul.gm tiles, got: {out}"
        );
        assert!(
            !out.contains(r#"class="rl-annot""#),
            "grid view must not render rows, got: {out}"
        );
    }

    /// The grid's opening tag for a list with `ratio` (`None`: no ratio set).
    fn grid_open_tag(ratio: Option<&str>) -> String {
        let thumbs = HashMap::new();
        let cfg = ResourceListCfg {
            tile_ratio: ratio.map(|r| TileRatio::parse(r).expect("a valid ratio")),
            ..make_grid_cfg("/alice/characters", Some("view=grid"), true, &thumbs)
        };
        let list = make_list_page(
            vec![make_resource("CH_001", "Character", "Alice")],
            1,
            1,
            Pagination::PerPage(24),
        );
        let out = render_resource_list(&list, &cfg, &make_lang("en")).into_string();
        let start = out.find("<ul class=\"gm\"").expect("a ul.gm");
        let end = out[start..].find('>').expect("the tag closes");
        out[start..=start + end].to_owned()
    }

    #[test]
    fn a_tall_tile_ratio_renders_the_style_and_data_tile() {
        assert_eq!(
            grid_open_tag(Some("1:9")),
            r#"<ul class="gm" data-tile="tall" style="--tile-ratio: 1 / 9">"#
        );
    }

    /// The `src` of one image tile (360 × 4264 stored) on a list with `ratio`.
    fn strip_tile_src(ratio: Option<&str>) -> String {
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://iiif.dasch.swiss/0803/l10.jp2/full/max/0/default.jpg".to_owned(),
                width: Some(360),
                height: Some(4264),
            },
        );
        let cfg = ResourceListCfg {
            tile_ratio: ratio.map(|r| TileRatio::parse(r).expect("a valid ratio")),
            ..make_grid_cfg("/incunabula/strips", Some("view=grid"), true, &thumbs)
        };
        let resource = make_resource("CH_001", "Character", "Alice");
        let out = render_grid_tile(&resource, &cfg, &make_lang("en")).into_string();
        let start = out.find("src=\"").expect("an img src") + 5;
        let end = out[start..].find('"').expect("the attribute closes");
        out[start..start + end].to_owned()
    }

    /// Only a tall list asks for the best-fit request; any other list keeps
    /// the width-sized one, whatever the image's own shape.
    #[test]
    fn only_a_tall_list_requests_its_tiles_best_fit() {
        assert_eq!(
            strip_tile_src(Some("1:9")),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/!360,1000/0/default.jpg"
        );
        let by_width = "https://iiif.dasch.swiss/0803/l10.jp2/full/360,/0/default.jpg";
        assert_eq!(strip_tile_src(Some("3:4")), by_width);
        assert_eq!(strip_tile_src(None), by_width);
    }

    #[test]
    fn a_tile_ratio_that_is_not_tall_renders_the_style_alone() {
        assert_eq!(grid_open_tag(Some("16:9")), r#"<ul class="gm" style="--tile-ratio: 16 / 9">"#);
    }

    #[test]
    fn a_grid_without_a_tile_ratio_renders_neither_attribute() {
        assert_eq!(grid_open_tag(None), r#"<ul class="gm">"#);
    }

    // ── Unit: grid tile markup (image tile) ───────────────────────────────────

    #[test]
    fn grid_tile_image_renders_box_with_sized_src_and_empty_decorative_alt() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &thumbs,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="gm__box""#),
            "image tile must render div.gm__box, got: {out}"
        );
        assert!(
            out.contains("src=\"https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/512,/0/default.jpg\""),
            "image tile <img> src must be sized via iiif_grid_tile_url_clamped/GRID_TILE_WIDTH, \
             got: {out}"
        );
        assert!(
            out.contains(r#"alt="""#),
            "the grid image tile must render alt=\"\" (decorative — .gm__name already has \
             the name), got: {out}"
        );
        assert!(
            out.contains(r#"class="gm__name""#) && out.contains("Alice"),
            "the tile must render .gm__name with the resolved title, got: {out}"
        );
        assert!(
            !out.contains("gm__box--text"),
            "an image tile must NOT render the text-tile fallback markup, got: {out}"
        );
    }

    /// A thumb narrower than `GRID_TILE_WIDTH` (e.g. an Incunabula
    /// BorderStrip, 288-360px) must be requested at its own stored width,
    /// never at the unclamped `GRID_TILE_WIDTH` (512) — Sipi (IIIF v3) 400s
    /// a size request wider than the image's native width.
    #[test]
    fn grid_tile_image_src_is_clamped_to_narrower_stored_width() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg".to_owned(),
                width: Some(288),
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/incunabula/strips",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "BorderStrip",
            narrowed: None,
            search: None,
            media_thumbs: &thumbs,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains("src=\"https://iiif.dasch.swiss/0803/abc.jpx/full/288,/0/default.jpg\""),
            "must clamp to the narrower stored width (288), never request GRID_TILE_WIDTH \
             (512) unclamped, got: {out}"
        );
    }

    /// Regression twin of `row_thumbnail_alt_is_empty_and_decorative`:
    /// the grid tile image must also be `alt=""`.
    #[test]
    fn grid_tile_thumbnail_alt_is_empty_and_decorative() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &thumbs,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(out.contains("<img"), "tile must render an <img>, got: {out}");
        assert!(
            out.contains(r#"alt="""#),
            "the grid tile image must be decorative (alt=\"\"), got: {out}"
        );
    }

    // ── Unit: grid tile markup (text tile fallback) ───────────────────────────

    #[test]
    fn grid_tile_text_fallback_renders_when_no_safe_media_uri() {
        let resource = make_annotated_resource("CH_001", "Alice");
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="gm__box gm__box--text""#),
            "no safe media URI must render the text-tile fallback, got: {out}"
        );
        assert!(
            out.contains(r#"class="k" aria-hidden="true""#),
            "the 'no illustration' label must be decorative (aria-hidden), got: {out}"
        );
        assert!(
            out.contains(ChromeMsg::GridNoImage.text(&lang)),
            "the text tile must render the GridNoImage chrome string, got: {out}"
        );
        assert!(
            out.contains(r#"class="g""#) && out.contains("A curious girl in a world of nonsense."),
            "the text tile must render the configured gloss, got: {out}"
        );
        assert!(!out.contains("<img"), "a text tile must render no <img>, got: {out}");
    }

    #[test]
    fn grid_tile_text_fallback_omits_gloss_span_when_no_gloss_configured() {
        let resource = make_resource("CH_001", "Character", "Alice"); // name only, no description
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="gm__box gm__box--text""#),
            "must still render the text-tile fallback, got: {out}"
        );
        assert!(
            !out.contains(r#"class="g""#),
            "no gloss configured must NOT render span.g, got: {out}"
        );
    }

    #[test]
    fn grid_tile_image_tile_never_shows_gloss() {
        // No chips, no gloss on IMAGE tiles — gloss only appears
        // in the text-tile fallback.
        let resource = make_annotated_resource("CH_001", "Alice");
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &thumbs,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            !out.contains("A curious girl in a world of nonsense."),
            "an image tile must never render the gloss text, got: {out}"
        );
    }

    // ── Unit: tile link semantics (mirrors render_row's non-link fallback) ────

    #[test]
    fn grid_tile_renders_plain_div_when_class_unbound() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            !out.contains("<a"),
            "an unbound class must render a plain div.gm__tile, no <a>, got: {out}"
        );
        assert!(
            out.contains(r#"class="gm__tile""#),
            "the tile wrapper must still carry .gm__tile, got: {out}"
        );
    }

    #[test]
    fn grid_tile_renders_link_when_class_bound() {
        let resource = make_resource("CH_001", "Character", "Alice");
        let mut routes: HashMap<String, String> = HashMap::new();
        routes.insert("Character".to_owned(), "/characters/:id".to_owned());
        static ROUTES_GT: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
        let routes_ref = ROUTES_GT.get_or_init(|| routes);
        let resolver = make_resolver("alice-in-daschland", routes_ref);
        let cfg = ResourceListCfg {
            links: Some(resolver),
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_grid_tile(&resource, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"<a class="gm__tile" href="/alice-in-daschland/characters/CH_001""#),
            "a bound class must render a.gm__tile with the resolved href, got: {out}"
        );
    }

    // ── Unit: pager present in grid view ──────────────────────────────────────

    #[test]
    fn pager_present_after_grid_body_on_multi_page_list() {
        let items: Vec<Resource> = (1..=3)
            .map(|i| make_resource(&format!("CH_{i:03}"), "Character", &format!("Alice {i}")))
            .collect();
        let list = make_list_page(items, 10, 1, Pagination::PerPage(3));
        let cfg = make_grid_cfg("/alice/characters", Some("view=grid"), true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(out.contains(r#"class="gm""#), "grid view must render tiles, got: {out}");
        assert!(
            out.contains(r#"class="pager""#),
            "the pager must still render after the grid body on a multi-page list, got: {out}"
        );
    }

    // ── Preserved-query assertions ────────────────────────────────

    /// `preserved_query_dropping_sort_and_page` must preserve `view` (it only
    /// drops `sort`/`page`).
    #[test]
    fn preserved_query_dropping_sort_and_page_preserves_view() {
        let qs = preserved_query_dropping_sort_and_page(Some("view=grid&sort=name&page=2"));
        assert_eq!(qs, "view=grid");
    }

    /// The view toggle's own hrefs (built via `drop_query_param`/
    /// `override_query_param`) preserve every other param — covered above by
    /// `view_toggle_list_href_drops_view_and_preserves_other_params` /
    /// `view_toggle_grid_href_sets_view_grid_and_preserves_other_params`; this
    /// assertion covers the grid-toggle-specific
    /// helper (`drop_query_param`) directly.
    #[test]
    fn drop_query_param_preserves_view_when_dropping_a_different_key() {
        let qs = drop_query_param(Some("view=grid&sort=name"), "sort");
        assert_eq!(qs, Some("view=grid".to_owned()));
    }

    // ── Layer-3 snapshots: surf-head grid states + grid populated ────────────

    /// Surf-head WITHOUT the grid toggle (grid: false) — still gets the
    /// `.surf-head__ctl` wrapper, but no
    /// `.viewtoggle__opts` (the actual view-toggle markup — the ordering
    /// control's own `span.viewtoggle__lbl` deliberately reuses the label
    /// class verbatim, so a bare
    /// "viewtoggle" substring check would false-positive on it).
    #[test]
    fn snapshot_surf_head_non_grid_class_has_ctl_wrapper_no_toggle() {
        let list = make_list_page(
            vec![make_resource("CH_001", "Character", "Alice")],
            1,
            1,
            Pagination::PerPage(24),
        );
        let cfg = make_grid_cfg("/alice-in-daschland/chapters", None, false, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(out.contains(r#"class="surf-head__ctl""#));
        assert!(!out.contains("viewtoggle__opts"));
        insta::assert_snapshot!(out);
    }

    /// Surf-head WITH the grid toggle (grid: true), List currently active.
    #[test]
    fn snapshot_surf_head_grid_class_has_toggle_list_active() {
        let list = make_list_page(
            vec![make_resource("CH_001", "Character", "Alice")],
            1,
            1,
            Pagination::PerPage(24),
        );
        let cfg = make_grid_cfg("/alice-in-daschland/characters", None, true, &EMPTY_THUMBS);
        let lang = make_lang("en");
        let out = render_surf_head(&list, &cfg, ViewMode::List, &lang).into_string();
        assert!(out.contains("viewtoggle"));
        insta::assert_snapshot!(out);
    }

    /// Grid populated state through the public `render_resource_list` entry
    /// point — image tile + text tile side by side (a Character with a media
    /// thumb, a Character without).
    #[test]
    fn snapshot_resource_list_grid_populated_mixed_image_and_text_tiles() {
        let items = vec![
            make_resource("CH_001", "Character", "Alice"),
            make_annotated_resource("CH_002", "The White Rabbit"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let mut thumbs = HashMap::new();
        thumbs.insert(
            "CH_001".to_owned(),
            MediaThumb {
                uri: "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg".to_owned(),
                width: None,
                height: None,
            },
        );
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("view=grid"),
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &thumbs,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(out.contains(r#"class="gm""#));
        assert!(out.contains("gm__box--text"));
        insta::assert_snapshot!(out);
    }

    /// Grid with text tiles only (no media configured at all) — the "no
    /// illustration" fallback for every tile.
    #[test]
    fn snapshot_resource_list_grid_all_text_tiles() {
        let items = vec![
            make_annotated_resource("CH_001", "Alice"),
            make_annotated_resource("CH_002", "The White Rabbit"),
        ];
        let list = make_list_page(items, 2, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("view=grid"),
            list_id: None,
            gloss: Some(&ResolvedGloss::Property("description")),
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(out.contains("gm__box--text"));
        assert!(!out.contains("<img"));
        insta::assert_snapshot!(out);
    }

    /// Empty-in-grid: a zero-item list rendered with the grid
    /// toggle on (`?view=grid`) must still fall into the existing
    /// empty-state branch — but the surf-head's view toggle must survive
    /// into that branch, not disappear along with the (absent) items.
    #[test]
    fn snapshot_resource_list_grid_empty() {
        let list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: Some("view=grid"),
            list_id: None,
            gloss: None,
            chips: &[],
            class_labels: &[],
            class: "Character",
            narrowed: None,
            search: None,
            media_thumbs: &EMPTY_THUMBS,
            grid: true,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(ChromeMsg::ListEmptyClass.text(&lang)),
            "empty-in-grid must still show the empty-state message, got: {out}"
        );
        assert!(
            out.contains("viewtoggle"),
            "empty-in-grid must keep the surf-head's view toggle visible, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Filters-only NO-MATCH, NO search query and NO search-bar at all
    /// (the facet-only case): the
    /// narrowed-to-zero body renders from `cfg.narrowed` alone — `cfg.search`
    /// stays `None` throughout (there is no search-bar on this page), so a
    /// `cfg.search`-only gate would miss this case.
    /// Only "Clear all filters" renders (no "Clear search", no hint — both
    /// gated on an active QUERY, which is absent here).
    #[test]
    fn snapshot_resource_list_narrowed_filters_only_no_search_bar() {
        let mut list = make_list_page(vec![], 0, 1, Pagination::PerPage(24));
        list.unfiltered_total = Some(32);
        let surface = make_narrowed_surface(false, Some("/alice-in-daschland/locations"));
        let cfg = ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/locations",
            current_query: Some("location-type=lt-wonderland"),
            list_id: None,
            gloss: None,
            chips: &[],
            class: "Location",
            class_labels: &[],
            search: None,
            narrowed: Some(surface),
            media_thumbs: &EMPTY_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let lang = make_lang("en");
        let out = render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"class="search-empty""#),
            "a filters-only narrowing must render .search-empty, got: {out}"
        );
        assert!(
            !out.contains("resource-list__empty"),
            "must NOT fall back to .resource-list__empty (E1), got: {out}"
        );
        assert_eq!(
            out.matches(r#"class="search-empty__act""#).count(),
            1,
            "filters-only: exactly ONE action (Clear all filters) must render, got: {out}"
        );
        assert!(
            out.contains(&format!(">{}</a>", ChromeMsg::EmptyClearFilters.text(&lang))),
            "the sole action must be Clear all filters, got: {out}"
        );
        assert!(
            !out.contains("search-empty__hint"),
            "no active query → no search hint, got: {out}"
        );
        assert!(
            out.contains("0 of 32"),
            "the count must reflect the unfiltered total, got: {out}"
        );
        insta::assert_snapshot!(out);
    }
}
