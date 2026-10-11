//! Renderer for the `compound-viewer` component (two-column reader).
//!
//! Takes a `CompoundPage` and renders an `<article id="compound-viewer">` with:
//! - An empty-state line when `total == 0`.
//! - `total == 1`: a member head (generic `{class} · NN` eyebrow + title linked to its detail page
//!   via `LinkResolver`) + properties. No contents rail, no nav (mirrors "single image → no
//!   `<nav>`").
//! - `total >= 2`, `index="rail"` (the `Contents` layout's default): a two-column `div.cv-toc` — a
//!   sticky `nav.cv-toc__contents` listing every member (current one marked `aria-current="page"`)
//!   beside `div.cv-toc__reader` (member head + properties + the prev/next `.pg` pager, shared with
//!   `component::media`'s plate — see below — at the foot). `index="collapsed"`/`"none"` skip the
//!   two-column `div.cv-toc` entirely.
//! - `layout="pages"` (the book reader): an identity head (no member properties), the stage, and
//!   the `.pg--reader` pager row — see `render_pages_head` and `render_reader_pager`. Paired with a
//!   `resource-detail`, it renders inside that record's article (`component::render_body`).
//!
//! ## Navigation pattern
//!
//! `?page=<member-id>` full-page reloads, no `#compound-viewer` fragment in
//! the emitted URL (see below;
//! `?member=N` is still READ, so old links land on the
//! same member; see `ProjectReads::compound_parts` for the `page` resolver); the
//! `article` receives `tabindex="-1"` as the forward-click swap's focus target (see
//! `render_cv_root`): no focus lands on the member block after a no-JS reload (mirrors
//! `#record-figure`). Contents-rail links use the same `?page=<id>` href (via
//! `build_compound_href`) — clicking a rail entry navigates like a prev/next
//! step, just to an arbitrary member.
//!
//! Every active (non-inert) rail link and pager step is a plain `<a href>`
//! navigation (the no-JS/initial-load path)
//! that ALSO carries `data-on:click__prevent="@get(…)"`
//! (the `_compound` fragment
//! swap): the member shown is URL-bearing state, so a JS-enhanced
//! click swaps `#compound-viewer` (the DOM element id — never present in any
//! emitted href/action, see below) in place AND
//! updates the address bar via `history.pushState`, while the plain `href`
//! remains the byte-identical
//! no-JS fallback (a full page load, still tracked by Back/reload). Inert
//! boundary spans carry neither `href` nor `data-on` — nothing to enhance.
//! The `pages` reader's page picker entries are plain links too, with no
//! per-entry `data-on`: one delegated handler on the panel makes the same
//! `@get` — see `render_picker`.
//!
//! **Back/Forward is a real page load.** The `_compound` swap above only ever runs on a FORWARD
//! click; browser Back/Forward never calls it. Instead, a plain
//! `window`-level `popstate` listener (`data-on:popstate__window`, a fixed
//! STATIC expression with no per-render interpolation — see
//! `render_cv_root`) compares `location.pathname + location.search`
//! against `window.__cpeReaderUrl` (a one-way, plain `window` property, NOT
//! a DataStar signal — see `render_cv_root`'s doc comment for why): a
//! mismatch means the browser has actually navigated to a different
//! path+query since this page last showed one (a genuine Back/Forward
//! across the reader's own pushed history entries), so the listener does a
//! plain `location.reload()` — a real page load of the URL the browser
//! already moved to, landing on the correct member via the ordinary
//! `?page=`/`?member=` resolution `render_page` always does. Comparing only
//! `pathname + search` (never the full URL/hash) is what excludes a
//! same-document hash-only history move (the skip link, an incoming-links
//! row's in-page anchor) from triggering a reload — those change
//! `location.hash` alone, which the comparison never reads.
//!
//! ## a11y idioms (mirror the `.pg` primitive)
//!
//! - `nav[aria-label]` carries the **static sequence extent** (total count), **not** the mutable
//!   position (screen reader doesn't re-announce on nav). The `pages` layout's `.pg--reader` row
//!   uses a static "Turn pages" label instead (`ChromeMsg::CompoundPagerLabel`).
//! - `span.pg__count` holds the mutable position as plain visible text, and carries
//!   `aria-live="polite" aria-atomic="true"`: member paging is a same-document `pushState`/fragment
//!   swap, so the "n / N" string is not announced by a full-page reload; the live region announces
//!   it instead, complementing (not substituting for) the focus-restore call. The SAME idiom as
//!   `component::media`'s own `.pg__count` (a separate hand-rolled copy), whose paging is a
//!   same-page `$plate` signal flip.
//! - Boundary ends are inert `span.pg__step[aria-disabled aria-hidden]` (no wrap-around); the
//!   `aria-hidden` removes the decorative glyph from the a11y tree. The `pages` layout's
//!   `.pg--reader` row differs: its inert end keeps its word and is not `aria-hidden` (see
//!   `render_reader_pager`).
//! - The contents rail's current entry carries `aria-current="page"`; the `nav.cv-toc__contents`
//!   `aria-label` is localized "Contents" (static — the caption `p.cv-toc__cap` carries the mutable
//!   "Contents · N parts" extent as plain visible text, not an `aria-live` region — the rail itself
//!   is not expected to change shape across a member swap).
//!
//! The prev/next foot nav has the `.pg` primitive's markup shape (the SAME
//! shape `component::media`'s plate pager uses), rendered by this
//! module's own `render_compound_pager`: the active steps
//! here additionally carry `data-on:click__prevent`.
//! The href composition and the
//! `?page=<member-id>` (vs. the plate's `?img=`) param semantics stay
//! compound-viewer-specific regardless.
use cpe_core::iiif::iiif_plate_url;
use cpe_core::ir::{
    ClassLabel, CompoundLayout, CompoundPage, MemberIndex, PickerMode, PropertyLabel, PropertyValue, Resource,
};
use cpe_core::lang::Lang;
use cpe_core::util::encode_get_arg;
use maud::{html, Markup, PreEscaped};

use crate::chrome::{
    anno_count_text, anno_group_marked_name, anno_marked_name, anno_marked_suffix, compound_contents_cap,
    compound_nav_label, compound_no_members_text, compound_stale_shown, compound_step_labels, picker_collision_name,
    picker_gathering_name, picker_leaves_meta, picker_live_count, picker_page_at, picker_pages_meta, picker_range_name,
    picker_summary_name, picker_unlabelled_name, ChromeMsg,
};
use crate::component::annotation::{self, render_overlay, ANNO_DOT};
use crate::component::dom::{PAGES_TITLE_ID, PICKER_ESCAPE_EXPR, PICKER_FOCUSOUT_EXPR};
use crate::component::links::{member_href_for, reader_query, LinkResolver};
use crate::component::media::{image_repr, overlay_will_draw, render_viewer_unit};
use crate::component::page_picker::{self, Display, Entry, GroupBody, GroupKind, PickerGroup};
use crate::component::resource_detail::{render_property_block, render_value};
use crate::component::values::{member_text, resolve_title_str};

/// The URL path segment that identifies the compound-viewer fragment
/// endpoint, a URL-writing `pushState` swap for FORWARD
/// clicks only (Back/Forward is a
/// real page load and never reaches this endpoint at all; see the
/// `data-on:popstate__window` doc comment below). The fragment endpoint
/// lives at `{page-path}/{COMPOUND_FRAGMENT_SEGMENT}?…`. Both the engine (to
/// build the `data-on:click__prevent` `@get` URLs) and `dsp-incubator cpe/bin` (to register
/// the Axum route) reference this constant so no magic-string mismatch is
/// possible. Mirrors `FACET_FRAGMENT_SEGMENT` / `SORT_FRAGMENT_SEGMENT` /
/// `SEARCH_FRAGMENT_SEGMENT`.
pub const COMPOUND_FRAGMENT_SEGMENT: &str = "_compound";

// ── Config borrow struct ──────────────────────────────────────────────────────

/// Borrowed context needed by `render_compound_viewer`.
///
/// Mirrors `ResourceDetailCfg`/`ResourceListCfg` in its shape: thin borrow
/// struct, no IR cloning. Built by `render_component` in `dsp-incubator cpe/web`'s `page` module
/// from the `RenderContext` fields.
pub struct CompoundViewerCfg<'a> {
    /// Optional link resolver for turning the member's class into a detail href.
    ///
    /// `None` → the member title renders as plain text (no link).
    /// `Some(resolver)` → the member's class is resolved into a
    /// `/<project>/<route>/<id>` href; unbound class → plain text fallback.
    #[doc(hidden)]
    pub links: Option<LinkResolver<'a>>,
    /// URI path of the current request (e.g. `/alice-in-daschland/story`).
    /// Used to build compound prev/next hrefs.
    #[doc(hidden)]
    pub current_path: &'a str,
    /// Raw query string of the current request (everything after `?`, no `?`).
    /// Used to build compound hrefs that preserve lang/appearance params while
    /// setting `page=<member-id>` and dropping any stale `member=` (see
    /// `reader_query`; `?member=N` is still READ, never re-emitted).
    /// `None` when the request had no query string.
    #[doc(hidden)]
    pub current_query: Option<&'a str>,
    /// The media link property name for the member's class. Taken from the
    /// `compound-viewer media="…"` attribute when set,
    /// else resolved by the caller from
    /// `RenderContext.media_properties` keyed by `member.class`.
    /// Either way it is excluded from the
    /// member's plain property list — suppressed exactly like
    /// `resource_detail::render_resource_detail`'s `.props` list. Whether the
    /// member's scan is SHOWN is decided by `CompoundPage.member_media`, not
    /// by this field: only the opt-in `media=` attribute fills it, so a
    /// reader without it keeps the "compound-reader members show no
    /// images" behaviour.
    pub media_property: Option<&'a str>,
    /// Declared class labels (`labels { class "…" { … } }`), used to resolve
    /// (a) the per-member eyebrow's class name
    /// and (b) on the
    /// empty branch, the parent's own eyebrow. Empty slice ⇒ humanized
    /// fallback — same contract as
    /// `ResourceDetailCfg.class_labels`.
    #[doc(hidden)]
    pub class_labels: &'a [ClassLabel],
    /// Declared property labels (`labels { property "…" { … } }`), used by
    /// both the empty branch's parent `.props` list and the populated
    /// branches' member property rows,
    /// via `resource_detail`'s own label resolution —
    /// "same as the detail page". Empty slice ⇒ humanized fallback.
    #[doc(hidden)]
    pub property_labels: &'a [PropertyLabel],
    /// The compound's parent Resource, for the empty state,
    /// threaded via `RenderContext.compound_parents` — resolved by the
    /// host only when a compound resolved to `total == 0`.
    ///
    /// `None` on every populated branch (`total >= 1`) and on the empty
    /// branch when the parent's source id itself doesn't resolve to a
    /// Resource — the renderer degrades to the sentence alone in that case
    /// (boot validation already warns `EmptyCompoundSource` for the sibling
    /// "source resolves but has no parts" case; a nonexistent source id is a
    /// distinct, not-currently-boot-validated condition). Never causes a
    /// panic.
    #[doc(hidden)]
    pub parent: Option<&'a Resource>,
    /// The `pages` reader's page picker (`compound-viewer picker="…"`),
    /// drawn in the pager row's centre at `total >= 2`
    /// (see `render_picker`). `None` ⇒ the centre keeps its count.
    #[doc(hidden)]
    pub picker: Option<PickerMode>,
    /// The declared reader layout (`compound-viewer layout="…"`):
    /// `Contents` (head → scan → props → pager) or `Pages` (an
    /// identity-only head, the stage, then the `.pg--reader` pager row; the
    /// member's properties are not shown — see `render_pages_head`).
    #[doc(hidden)]
    pub layout: CompoundLayout,
    /// How the member index is shown (`compound-viewer index="…"`),
    /// honoured at `total >= 2`:
    /// - `Rail` — the sticky `nav.cv-toc__contents` beside the reader;
    /// - `Collapsed` — no `div.cv-toc`: the reader, then a closed `details.cv-index` holding the
    ///   list;
    /// - `Off` — no `div.cv-toc`, no list: the reader alone.
    #[doc(hidden)]
    pub index: MemberIndex,
    /// Whether this compound-viewer is paired with a same-`id`
    /// `resource-detail` on its page (`ir::Component::CompoundViewer.paired`).
    /// `true` narrows the empty (`total == 0`) branch to
    /// the "no members" sentence alone, even when `parent` resolves — the
    /// paired record already renders that resource's own title/eyebrow/
    /// properties, so the empty branch must not repeat them (`dsp-incubator
    /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md`).
    #[doc(hidden)]
    pub paired: bool,
    /// `site.date_precision_word`: threaded
    /// into every member/parent property row's `render_property_block`, so
    /// both reader layouts render dates exactly as the record page does.
    #[doc(hidden)]
    pub date_precision_word: bool,
    /// The `member-links` row: its resolved caption and
    /// the member `link` properties it lists. `None` ⇒ no row. Rendered by
    /// the `pages` layout only (see `render_member_links`).
    #[doc(hidden)]
    pub member_links: Option<(&'a str, &'a [String])>,
    /// Whether a stale `?page=` gets its `history.replaceState` correction.
    /// `true` only on the full-page render: the
    /// `_compound` fragment never corrects the address (it has just written
    /// it itself). See `stale_url_correction`.
    #[doc(hidden)]
    pub emit_url_correction: bool,
}

/// The `data-on:popstate__window` directive shared by every populated
/// compound-viewer root:
/// a single
/// FIXED, STATIC expression with no per-render interpolation at all (not
/// even a `COMPOUND_FRAGMENT_SEGMENT` reference — this listener
/// never calls `@get` or touches the fragment endpoint).
///
/// Back/Forward navigation is a real page load: on `popstate`, if
/// `location.pathname + location.search` differs from
/// `window.__cpeReaderUrl` (the plain, one-way "URL the reader currently
/// shows" value — see [`render_cv_root`]'s doc comment for where it's set
/// and updated), the listener does a plain `location.reload()` — the
/// browser has already moved `location` to its Back/Forward target BEFORE
/// firing `popstate`, so reloading re-requests exactly that URL, and the
/// ordinary `?page=`/`?member=` resolution `render_page` always does lands
/// on the right member. No `@get`, no SSE, no history-API write of any
/// kind here — the write side lives entirely in `compound_fragment_get`'s
/// forward-click swap (`dsp-incubator cpe/bin/src/router.rs`'s `compound_fragment_response`,
/// its own `pushState`).
///
/// Comparing only `pathname + search` (never the full URL, never
/// `location.hash`) is what excludes a same-document hash-only navigation
/// (the skip-link, an incoming-links row's in-page anchor) from
/// triggering a reload — those change `location.hash` alone, which this
/// comparison never reads, so the listener no-ops and the hash-only jump
/// behaves like ordinary same-document anchor navigation.
///
/// Do not discriminate `popstate` by a request header or a DataStar signal:
/// a reload has no swap request to disambiguate.
const POPSTATE_RELOAD_EXPR: &str =
    "if (location.pathname + location.search !== window.__cpeReaderUrl) { location.reload(); }";

/// The one-time inline script every populated compound-viewer root emits as
/// its FIRST child, seeding `window.__cpeReaderUrl` — the plain, one-way
/// "URL the reader currently shows" value [`POPSTATE_RELOAD_EXPR`] compares
/// against.
///
/// Deliberately a plain `window` PROPERTY, not a DataStar signal
/// — no two-way signal syncing, and never a
/// reactively-written `data-*` attribute (`dsp-incubator cpe/CLAUDE.md`'s DataStar directive-
/// scope constraint: a `data-*` attribute is for markup, not a scratch
/// variable). Two writers only, by design:
///
/// 1. This inline `<script>`, on every NORMAL document parse — a fresh full-page load (including
///    the reload [`POPSTATE_RELOAD_EXPR`] itself triggers) always re-executes it, seeding the value
///    from whatever `location` the browser is actually showing at that moment. On a `_compound`
///    fragment swap the vendored DataStar bundle's morph normally WOULD re-execute a patched-in
///    inline `<script>` (it tracks already-loaded scripts in a `WeakSet` and skips only those);
///    this one does not re-run in practice because the morph reuses the SAME script DOM node across
///    the swap (same tag, same position — nothing about it diffs), so it is treated as
///    already-tracked and skipped. This is not "patched-in scripts never execute" — it's DOM-node
///    identity being preserved by the morph. The swap's own `ExecuteScript`
///    (`compound_fragment_response`) is the SECOND writer, updating the value explicitly, in the
///    same script that calls `pushState`, so the two can never drift regardless of whether this
///    seed script re-runs (re-seeding from `location` is idempotent either way). This doc fix
///    changes no code path.
/// 2. `compound_fragment_response`'s `pushState` `ExecuteScript`, right after the
///    `history.pushState` call — since a swap never re-runs this inline script, the handler must
///    set `window.__cpeReaderUrl` itself to keep it in sync with the address bar it just wrote.
const READER_URL_INIT_SCRIPT: &str = "window.__cpeReaderUrl = location.pathname + location.search;";

/// Wraps `body` in the `<article id="compound-viewer" tabindex="-1">` root
/// shared by every populated (`total >= 1`) branch of [`render_compound_viewer`],
/// so the root markup and its `.page` rationale live in exactly one place.
///
/// `pages_keys: Some` selects the `.compound-viewer--pages` variant (with
/// its optional `data-class="wide"` attribute, `wide`); the `None` variant
/// never carries `data-class` at all. They are two static Maud shapes, not
/// one with a conditionally-appended class.
///
/// `id="compound-viewer" tabindex="-1"` is a real focus target, but no
/// emitted href/action carries a `#compound-viewer` fragment,
/// so a plain real navigation (a no-JS page
/// step, Back/Forward, an incoming-links row) does
/// not land focus here; the only mechanism that does is the
/// JS-enhanced forward-click swap's programmatic focus-restore call
/// (`dsp-incubator cpe/bin/src/router.rs`'s `compound_fragment_response`, `preventScroll:
/// true`). See the `tabindex="-1"` Non-obvious constraint in `dsp-incubator cpe/CLAUDE.md`.
/// `.page` is the shared centered max-width + gutter
/// box every
/// top-level component root must carry (like `article.record.page` /
/// `section.resource-list-section.page`) — without it content hugs the
/// viewport edges.
///
/// Both [`READER_URL_INIT_SCRIPT`] (an inline `<script>`, first child) and
/// [`POPSTATE_RELOAD_EXPR`] (`data-on:popstate__window`) are FIXED,
/// per-render-invariant strings — no `cfg` value is interpolated into
/// either — so the emitted markup, this fn's own open-tag bytes included,
/// stays byte-identical across every render, unlike the
/// per-member `@get(...)` expressions the pager/rail carry.
///
/// `stale` (the `pages` layout only): the reader is `aria-describedby` its
/// status line, so a screen-reader user hears the stale-address line when
/// focus reaches the reader — a status region's parse-time text is never
/// announced on arrival.
///
/// `url_fix`: a `history.replaceState` statement prepended to the
/// init script, so the seed then reads the corrected `location` — see
/// [`stale_url_correction`].
///
/// `pages_keys` is the `pages` variant's `keydown` expression, from
/// [`pages_keydown`].
fn render_cv_root(
    pages_keys: Option<&str>,
    wide: Option<&str>,
    stale: bool,
    url_fix: Option<String>,
    body: Markup,
) -> Markup {
    let init_script = PreEscaped(format!("{}{READER_URL_INIT_SCRIPT}", url_fix.unwrap_or_default()));
    let popstate = POPSTATE_RELOAD_EXPR;
    if let Some(keys) = pages_keys {
        html! {
            article
                .compound-viewer
                .compound-viewer--pages
                .page
                id="compound-viewer"
                tabindex="-1"
                aria-labelledby=(PAGES_TITLE_ID)
                aria-describedby=[stale.then_some(PAGES_STATUS_ID)]
                data-class=[wide]
                data-on:popstate__window=(popstate)
                data-on:keydown=(keys)
            {
                script { (init_script) }
                (body)
            }
        }
    } else {
        html! {
            article
                .compound-viewer
                .page
                id="compound-viewer"
                tabindex="-1"
                data-on:popstate__window=(popstate)
            {
                script { (init_script) }
                (body)
            }
        }
    }
}

/// The `pages` reader's `keydown`: on a page that draws annotation shapes,
/// Esc (unpin and fit) goes in front of [`PAGES_KEYS_EXPR`], which returns
/// on any other key and on targets inside the viewer. Any other page keeps
/// exactly [`PAGES_KEYS_EXPR`].
fn pages_keydown(page: &CompoundPage) -> std::borrow::Cow<'static, str> {
    if annotation::has_shapes(&page.annotations) {
        format!("{} {PAGES_KEYS_EXPR}", annotation::esc_unpin_expr()).into()
    } else {
        PAGES_KEYS_EXPR.into()
    }
}

// ── Public renderer entry point ───────────────────────────────────────────────

/// Render a `CompoundPage` as a component-level block.
///
/// - `total == 0` → localized empty-state line (no nav).
/// - `total == 1` → `<article id="compound-viewer" tabindex="-1">` with the member head (eyebrow +
///   title, linked to its detail page or plain text when the class has no binding) + properties. No
///   contents rail, no nav (mirrors single-image carousel).
/// - `total >= 2` → `<article id="compound-viewer" tabindex="-1"><div class="cv-toc">` two-column
///   layout: `nav.cv-toc__contents` (the contents rail, one entry per `page.members`, current one
///   marked `aria-current="page"`) beside `div.cv-toc__reader` (member head + properties +
///   `render_compound_pager`'s prev/next pager at the foot — an `aria-live="polite"
///   aria-atomic="true"` position indicator, since member paging is a same-document fragment swap
///   again (`reader-paging-fragment-swap`); boundary ends use inert `span`s).
pub fn render_compound_viewer(page: &CompoundPage, cfg: &CompoundViewerCfg<'_>, lang: &Lang) -> Markup {
    // ── Empty compound (total == 0) ──────────────────────────────────────────
    if page.total == 0 {
        // `id="compound-viewer" tabindex="-1"` matches the populated branches
        // below, even when the compound resolves to zero parts — see
        // `render_cv_root`'s doc comment for the current (JS-swap-only)
        // focus-landing mechanism.
        //
        // Paired: a sibling `resource-detail` on this page
        // already renders the SAME parent resource's title/eyebrow/properties
        // (`dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`) — showing
        // `cfg.parent`'s head + `.props` here too would duplicate that record. Degrade to
        // the sentence alone. The store still resolves `cfg.parent` for a paired source
        // (`PageData.compound_parents`), only so the sentence can name the
        // parent's class ("no parts are recorded for this book") instead of
        // the generic "compound" wording.
        if cfg.paired {
            let msg = match cfg.parent {
                Some(parent) => {
                    let class_label = cpe_core::labels::resolve_class_label(cfg.class_labels, &parent.class, lang);
                    compound_no_members_text(&class_label, lang)
                }
                None => ChromeMsg::CompoundEmptyState.text(lang).to_owned(),
            };
            return html! {
                section.compound-viewer.page id="compound-viewer" tabindex="-1" {
                    p.compound-viewer__empty { (msg) }
                }
            };
        }
        return match cfg.parent {
            // The compound's parent Resource resolved — degrade to a plain
            // record: `.cv-compound-head` (eyebrow + big italic title, built
            // for the first time here — CSS in record.css), one serif
            // sentence, then the parent's own `.props` in the reading measure
            // (reusing `render_property_block`, same as resource_detail).
            Some(parent) => {
                let class_label = cpe_core::labels::resolve_class_label(cfg.class_labels, &parent.class, lang);
                let title = resolve_title_str(parent, Some("name"), lang);
                let msg = compound_no_members_text(&class_label, lang);
                let media_property = cfg.media_property;
                html! {
                    section.compound-viewer.page id="compound-viewer" tabindex="-1" {
                        header.cv-compound-head {
                            p.eyebrow.cv-compound-head__eyebrow { (class_label) }
                            h2.cv-compound-head__title { (title) }
                        }
                        p.compound-viewer__empty { (msg) }
                        div.cv-member__props {
                            @for prop in &parent.properties {
                                @if {
                                    prop.name != "name"
                                        && media_property.is_none_or(|mp| prop.name != mp)
                                } {
                                    ({
                                        render_property_block(
                                            prop,
                                            lang,
                                            cfg.property_labels,
                                            cfg.links.as_ref(),
                                            cfg.date_precision_word,
                                        )
                                    })
                                }
                            }
                        }
                    }
                }
            }
            // Parent missing (source id doesn't resolve to a Resource) —
            // degrade to the sentence alone. No class is known to interpolate
            // into `compound_no_members_text`, so fall back to the generic
            // message. Never panic.
            None => {
                let msg = ChromeMsg::CompoundEmptyState.text(lang);
                html! {
                    section.compound-viewer.page id="compound-viewer" tabindex="-1" {
                        p.compound-viewer__empty { (msg) }
                    }
                }
            }
        };
    }

    // Invariant: member.is_some() when total >= 1
    let member: &Resource = page
        .member
        .as_ref()
        .expect("CompoundPage invariant: member is Some when total >= 1");

    // ── Member title ──────────────────────────────────────────────────────────
    let title = resolve_title_str(member, Some("name"), lang);

    // Try to resolve the member's class into a detail href.
    let title_href: Option<String> = cfg.links.as_ref().and_then(|r| r.href(&member.id, Some(&member.class)));

    // Generic `{class} · NN` eyebrow (1-based ordinal, zero-padded to ≥2
    // digits) — deliberately NOT an Alice-specific `chapter-number`/"Prologue"
    // label; keeps the component generic (`dsp-incubator cpe/CLAUDE.md`'s
    // vocabulary rule). `{:02}` is fine at Alice's 13 chapters; noted for
    // larger compounds. The class name is resolved through declared
    // labels, so this eyebrow and the empty branch's parent eyebrow
    // resolve identically.
    let member_class_label = cpe_core::labels::resolve_class_label(cfg.class_labels, &member.class, lang);
    let head_markup = match cfg.layout {
        CompoundLayout::Contents => html! {
            header.cv-member__head {
                p.eyebrow.cv-member__eyebrow { (format!("{member_class_label} · {:02}", page.index)) }
                h2.cv-member__title {
                    @if let Some(ref href) = title_href {
                        a href=(href) { (title) }
                    } @else { (title) }
                }
            }
        },
        CompoundLayout::Pages => render_pages_head(member, page, &member_class_label, title_href.as_deref(), lang),
    };

    // ── Member properties (the `contents` layout only — built lazily, since
    // the `pages` layout renders neither these nor the scan figure) ────────
    let properties_markup = || {
        html! {
            @for prop in &member.properties {
                // Skip the "name" property (shown as the title above) and the
                // member class's media link property, if any (never
                // printed as a plain int-link/unresolved
                // list here; the member's own detail page owns its plate).
                @if {
                    prop.name != "name"
                        && cfg.media_property.is_none_or(|mp| prop.name != mp)
                } {
                    // Member property labels resolve through the declared
                    // `labels {}`, same as the
                    // detail page and this component's own empty branch.
                    ({
                        render_property_block(
                            prop,
                            lang,
                            cfg.property_labels,
                            cfg.links.as_ref(),
                            cfg.date_precision_word,
                        )
                    })
                }
            }
        }
    };

    // ── Member scan (opt-in `media=`) ───────────────────────────
    let scan_markup = || render_member_scan(page.member_media.first(), title);

    // ── `pages` layout, total == 1: head + stage, no pager row ──────────────
    if page.total == 1 && cfg.layout == CompoundLayout::Pages {
        let stage = render_pages_stage(page, title, lang);
        let wide = stage.is_some().then_some(PAGES_WIDE_CLASS_ATTR);
        let body = html! {
            (head_markup)
            (render_pages_status(page, member, lang))
            @if let Some(stage) = stage { (stage) }
        };
        return render_cv_root(
            Some(&pages_keydown(page)),
            wide,
            page.stale_page.is_some(),
            stale_url_correction(page, cfg),
            body,
        );
    }

    // ── total == 1: member head + props only — no rail, no nav ───────────────
    if page.total == 1 {
        let body = html! {
            (head_markup)
            (scan_markup())
            div.cv-member__props { (properties_markup()) }
        };
        return render_cv_root(None, None, false, None, body);
    }

    // ── total >= 2: two-column reader (contents rail + foot nav) ────────
    let index = page.index;
    // 1-based prev/next positions.
    let prev_member: Option<u32> = index.checked_sub(1).filter(|&n| n >= 1);
    let total = page.total;
    // `index < total` here, and `total` is a `Vec` length, so `index + 1`
    // cannot overflow `u32`; `saturating_add` makes that explicit.
    let next_member: Option<u32> =
        (usize::try_from(index).unwrap_or(usize::MAX) < total).then(|| index.saturating_add(1));

    // Links address a member by its id (`?page=<id>`); map the
    // 1-based position to `members[n - 1]`. `members.len() == total`, and
    // both positions lie in `1..=total`, so the lookup never misses.
    let member_id_at = |n: u32| -> Option<&str> {
        let pos = usize::try_from(n).ok()?.checked_sub(1)?;
        page.members.get(pos).map(|m| m.id.as_str())
    };
    let prev_id: Option<&str> = prev_member.and_then(member_id_at);
    let next_id: Option<&str> = next_member.and_then(member_id_at);

    let prev_href: Option<String> = prev_id.map(|id| build_compound_href(cfg.current_path, cfg.current_query, id));
    let next_href: Option<String> = next_id.map(|id| build_compound_href(cfg.current_path, cfg.current_query, id));
    // The `data-on:click__prevent` `@get(...)` counterpart to each plain
    // href above — `None`
    // exactly when `*_href` is `None` (an inert boundary end gets neither).
    let prev_get: Option<String> = prev_id.map(|id| compound_fragment_get(cfg.current_path, cfg.current_query, id));
    let next_get: Option<String> = next_id.map(|id| compound_fragment_get(cfg.current_path, cfg.current_query, id));

    let count_text = format!("{index} / {total}");

    // Contents-rail aria-label (static "Contents") + caption ("Contents · N
    // parts") — mirrors the nav's static-extent/mutable-position split above.
    let contents_label = ChromeMsg::CompoundContentsLabel.text(lang);
    let contents_cap = compound_contents_cap(total, lang);

    // The contents list, shared by the rail and the collapsed index — built
    // lazily: `index="none"` (`MemberIndex::Off`) never emits this list in
    // ANY layout, so it must not be built unconditionally up front. A
    // closure (not a `let … = html! {…}` up front) so each of the
    // Collapsed/Rail call sites below builds it only when it actually
    // renders — never called at all on the `Off` path.
    let list_markup = || {
        html! {
            ol.cv-toc__list {
                @for (i, m) in page.members.iter().enumerate() {
                    // 1-based ordinal from the 0-based enumerate index;
                    // bounded by the compound's own member count, so
                    // this always fits in u32.
                    @let ordinal = u32::try_from(i + 1)
                        .expect("member ordinal is bounded by compound size — fits in u32");
                    @let current = ordinal == page.index;
                    li.cv-toc__item {
                        a   .cv-toc__link
                            href=({
                                build_compound_href(
                                    cfg.current_path,
                                    cfg.current_query,
                                    &m.id,
                                )
                            })
                            data-on:click__prevent=({
                                if current {
                                    CURRENT_ENTRY_CLICK_EXPR.to_owned()
                                } else {
                                    compound_fragment_get(
                                        cfg.current_path,
                                        cfg.current_query,
                                        &m.id,
                                    )
                                }
                            })
                            aria-current=[current.then_some("page")]
                        {
                            span.cv-toc__num { (format!("{ordinal:02}")) }
                            span { (m.name) }
                        }
                    }
                }
            }
        }
    };

    // ── `pages` layout ─────────────────────
    // One column: the identity head, the stage, the `.pg--reader` pager row
    // directly under it, then — with no rail — a collapsed index.
    // `index="rail"` wraps it in the same `div.cv-toc` as the contents
    // layout.
    if cfg.layout == CompoundLayout::Pages {
        let step = |n: Option<u32>, href: Option<&str>, get: Option<&str>| -> Option<ReaderStep> {
            let pos = usize::try_from(n?).ok()?.checked_sub(1)?;
            let target = page.members.get(pos);
            Some(ReaderStep {
                href: href?.to_owned(),
                get: get?.to_owned(),
                target: target.map_or("", |m| m.name.as_str()).to_owned(),
                annotated: target.is_some_and(|m| page.is_annotated(&m.id)),
            })
        };
        let centre = match cfg.picker {
            Some(mode) => render_picker(page, mode, cfg, lang),
            None => html! {
                span.pg__count aria-live="polite" aria-atomic="true" { (count_text) }
            },
        };
        let nav_markup = render_reader_pager(
            step(prev_member, prev_href.as_deref(), prev_get.as_deref()),
            step(next_member, next_href.as_deref(), next_get.as_deref()),
            centre,
            lang,
        );
        let under_pager = html! {
            @if cfg.index == MemberIndex::Collapsed {
                details.cv-index {
                    summary.cv-index__cap { (contents_cap) }
                    (list_markup())
                }
            }
        };
        let stage = render_pages_stage(page, title, lang);
        // WIDER's host: the unit's `$expand` toggles
        // `.is-wide` here, exactly as `article.record--viewer` does — only
        // when the stage (and so the `$expand` signal) renders.
        let wide = stage.is_some().then_some(PAGES_WIDE_CLASS_ATTR);
        let body = html! {
            (head_markup)
            (render_pages_status(page, member, lang))
            @if let Some(stage) = stage { (stage) }
            (nav_markup)
            (render_member_links(member, cfg, lang))
            (under_pager)
        };
        let content = html! {
            @if cfg.index == MemberIndex::Rail {
                div.cv-toc {
                    nav.cv-toc__contents aria-label=(contents_label) {
                        p.cv-toc__cap { (contents_cap) }
                        (list_markup())
                    }
                    div.cv-toc__reader { (body) }
                }
            } @else { (body) }
        };
        return render_cv_root(
            Some(&pages_keydown(page)),
            wide,
            page.stale_page.is_some(),
            stale_url_correction(page, cfg),
            content,
        );
    }

    // Nav aria-label carries the STATIC extent (total) — localized.
    let nav_aria = compound_nav_label(total, lang);
    let (prev_label, next_label) = compound_step_labels(lang);

    // Mirrors the shared `.pg` primitive's markup shape (same
    // inert-ends idiom — the
    // links address members by id, `?page=<id>`) via this module's
    // own `render_compound_pager`. At
    // the foot of `.cv-toc__reader`.
    let nav_markup = render_compound_pager(
        &nav_aria,
        prev_label,
        next_label,
        &count_text,
        prev_href.as_deref().zip(prev_get.as_deref()),
        next_href.as_deref().zip(next_get.as_deref()),
    );

    // ── Collapsed / no index ──────────────────────────────
    // No `div.cv-toc`: the reader's blocks sit directly in the article, then
    // (collapsed only) a closed `details.cv-index` with the rail's caption as
    // its summary.
    if cfg.index != MemberIndex::Rail {
        let body = html! {
            (head_markup)
            (scan_markup())
            div.cv-member__props { (properties_markup()) }
            (nav_markup)
            @if cfg.index == MemberIndex::Collapsed {
                details.cv-index {
                    summary.cv-index__cap { (contents_cap) }
                    (list_markup())
                }
            }
        };
        return render_cv_root(None, None, false, None, body);
    }

    // ── Member container ──────────────────────────────────────────────────────
    let body = html! {
        div.cv-toc {
            nav.cv-toc__contents aria-label=(contents_label) {
                p.cv-toc__cap { (contents_cap) }
                (list_markup())
            }
            div.cv-toc__reader {
                (head_markup)
                (scan_markup())
                div.cv-member__props { (properties_markup()) }
                (nav_markup)
            }
        }
    };
    render_cv_root(None, None, false, None, body)
}

// ── Page picker ────────────────

/// The picker's fixed `<details>` id: a page holds at most one navigator.
const PICKER_ID: &str = "cv-picker";

/// On opening, the panel scrolls so the current entry sits a third of the
/// way down. Instant, never `scrollIntoView`: that would scroll the window
/// and drag the sticky reader.
const PICKER_TOGGLE_EXPR: &str = "if (!el.open) return; const p = el.querySelector('.pk__panel'); const c = p.querySelector('[aria-current=page]'); if (c) p.scrollTop = c.offsetTop - p.clientHeight / 3;";

/// The panel's one delegated click handler. A jump mark
/// (`href="#cv-pk-<n>"`) scrolls the panel to its group's heading below the
/// sticky band and focuses it — the window never scrolls. Any other link is
/// an entry: close the picker, return focus to its summary, and make the
/// steps' own `_compound` swap for the entry's query — except the current
/// page's entry, which only closes: its swap would push the address already
/// shown, leaving a Back press that goes nowhere. A modified click
/// (Ctrl/Cmd/Shift/Alt) is left to the browser, so "open in a new tab" still
/// works. `{base}` is the `encode_get_arg`-escaped
/// `{path}/{COMPOUND_FRAGMENT_SEGMENT}`.
fn picker_click_expr(base: &str) -> String {
    format!(
        "const a = evt.target.closest('a[href]'); \
         if (!a || evt.ctrlKey || evt.metaKey || evt.shiftKey || evt.altKey) return; evt.preventDefault(); \
         const href = a.getAttribute('href'); \
         if (href.startsWith('#')) {{ const h = document.getElementById(href.slice(1)); \
         if (h) {{ el.scrollTop = h.offsetTop - el.querySelector('.pk__band').offsetHeight; \
         h.focus({{preventScroll: true}}); }} return; }} \
         const d = document.getElementById('{PICKER_ID}'); d.open = false; \
         d.querySelector('summary').focus({{preventScroll: true}}); \
         if (a.getAttribute('aria-current') === 'page') return; \
         @get('{base}' + a.search)"
    )
}

/// The contents rail's current entry: its click is prevented and does
/// nothing. A `_compound` swap for it would push the address already shown,
/// leaving a Back press that goes nowhere. `data-on` needs an expression, so
/// the no-op is spelled out.
const CURRENT_ENTRY_CLICK_EXPR: &str = "void 0";

/// The in-panel anchor id of the `n`-th (1-based) group's heading.
fn picker_group_id(n: usize) -> String {
    format!("cv-pk-{n}")
}

/// A group's visible mark (band and heading) and its full name, which is
/// what assistive technology reads.
fn picker_group_names(group: &PickerGroup, lang: &Lang) -> (String, String) {
    match &group.kind {
        GroupKind::Gathering { key } => (key.clone(), picker_gathering_name(key, lang)),
        GroupKind::Vorsatz => (
            ChromeMsg::PickerFrontMark.text(lang).to_owned(),
            ChromeMsg::PickerFrontName.text(lang).to_owned(),
        ),
        GroupKind::Nachsatz => (
            ChromeMsg::PickerBackMark.text(lang).to_owned(),
            ChromeMsg::PickerBackName.text(lang).to_owned(),
        ),
        GroupKind::Weitere => {
            let other = ChromeMsg::PickerOther.text(lang).to_owned();
            (other.clone(), other)
        }
        GroupKind::Range { first, last } => (first.to_string(), picker_range_name(*first, *last, lang)),
    }
}

/// One picker entry: a plain link with the steps' own href (no
/// per-entry `data-on`), `aria-current="page"` on the current page. Its name
/// is the full label; where the visible text differs (a signature shown
/// without its key, a dash, or one of several pages stacked in one cell)
/// the name is an `aria-label` and the label is also the `title`. A repeat
/// of the same visible text in one cell carries a superscript ordinal
/// (`5v`, `5v²`, `5v³`); the name stays the disambiguated label. A page with
/// a register is accent-coloured with a trailing dot, and its name is
/// [`anno_marked_name`].
fn render_picker_entry(
    entry: &Entry,
    stacked: bool,
    ordinal: usize,
    page: &CompoundPage,
    cfg: &CompoundViewerCfg<'_>,
    lang: &Lang,
) -> Markup {
    let member = page
        .members
        .get(entry.member_index)
        .expect("page_picker::group indexes page.members, so every entry has its member");
    let position = entry.member_index + 1;
    let current = usize::try_from(page.index).is_ok_and(|i| i == position);
    let has_label = !entry.label.trim().is_empty();
    let visible = entry.visible();
    let dash = entry.display == Display::Dash;
    let name = if !has_label {
        Some(picker_unlabelled_name(position, lang))
    } else if stacked {
        Some(picker_collision_name(&entry.label, position, lang))
    } else if visible != entry.label {
        Some(entry.label.clone())
    } else {
        None
    };
    let annotated = page.is_annotated(&member.id);
    let name = if annotated {
        Some(anno_marked_name(name.as_deref().unwrap_or(&entry.label), lang))
    } else {
        name
    };
    let title = (has_label && visible != entry.label).then_some(entry.label.as_str());
    html! {
        a   .pk__entry
            .pk__entry--dash[dash]
            .pk__entry--anno[annotated]
            href=(build_compound_href(cfg.current_path, cfg.current_query, &member.id))
            aria-current=[current.then_some("page")]
            aria-label=[name]
            title=[title]
        {
            (visible)
            (page_picker::ordinal_mark(ordinal))
            @if annotated { (ANNO_DOT) }
        }
    }
}

/// A gathering or flat group's body.
fn render_picker_body(group: &PickerGroup, page: &CompoundPage, cfg: &CompoundViewerCfg<'_>, lang: &Lang) -> Markup {
    let cell = |entries: &[Entry]| {
        let stacked = entries.len() > 1;
        let ordinals = page_picker::repeat_ordinals(entries);
        html! {
            @for (e, n) in entries.iter().zip(ordinals) {
                (render_picker_entry(e, stacked, n, page, cfg, lang))
            }
        }
    };
    match &group.body {
        GroupBody::Leaves(leaves) => html! {
            ul.pk__leaves {
                @for leaf in leaves {
                    li.pk__leaf {
                        div.pk__cell { (cell(&leaf.recto)) }
                        div.pk__cell { (cell(&leaf.verso)) }
                    }
                }
            }
        },
        GroupBody::Flat(entries) => html! {
            ul.pk__flat.pk__flat--num[group.number_grid()] {
                @for e in entries {
                    li { (render_picker_entry(e, false, 1, page, cfg, lang)) }
                }
            }
        },
    }
}

/// The page picker in the `.pg--reader` centre: a
/// `details#cv-picker` whose panel opens upwards over the stage, anchored to
/// the pager row. The panel holds a sticky band of jump marks (one per group,
/// the current group's `aria-current="true"`), then one `section` per group:
/// an `h3[tabindex=-1]` (visible mark aria-hidden, full name visually hidden,
/// meta) and its body. Beside it, a visually hidden `span.pg__count` live
/// region announces the page a swap lands on (the `aria-live` constraint).
///
/// Zero-JS it is a native disclosure of plain links; the DataStar
/// directives add Escape, click-away and focus-out closing (the `.orderctl`
/// idiom), the open-at-current scroll, and the one delegated click handler
/// (see [`picker_click_expr`]).
fn render_picker(page: &CompoundPage, mode: PickerMode, cfg: &CompoundViewerCfg<'_>, lang: &Lang) -> Markup {
    let groups = page_picker::group(&page.members, mode);
    let current_index = usize::try_from(page.index).ok().and_then(|i| i.checked_sub(1));
    let current_group = groups
        .iter()
        .position(|g| g.entries().iter().any(|e| Some(e.member_index) == current_index));
    let current_label = current_index
        .and_then(|i| page.members.get(i))
        .map(|m| m.name.as_str())
        .filter(|l| !l.trim().is_empty() && *l != "[missing]");
    let position = usize::try_from(page.index).unwrap_or(0);
    let summary_name = picker_summary_name(
        &current_label.map_or_else(|| picker_page_at(position, lang), str::to_owned),
        page.total,
        lang,
    );
    let live_label = current_label.map_or_else(|| position.to_string(), str::to_owned);
    let index_label = if groups.iter().any(|g| matches!(g.kind, GroupKind::Gathering { .. })) {
        ChromeMsg::PickerIndexGatherings.text(lang)
    } else {
        ChromeMsg::PickerIndexPages.text(lang)
    };
    let base = encode_get_arg(&format!("{}/{COMPOUND_FRAGMENT_SEGMENT}", cfg.current_path));
    // A group holding a page with a register carries a dot on its band mark
    // and its heading, and says so in its name.
    let names: Vec<(String, String, bool)> = groups
        .iter()
        .map(|g| {
            let (mark, name) = picker_group_names(g, lang);
            let annotated = g
                .entries()
                .iter()
                .any(|e| page.members.get(e.member_index).is_some_and(|m| page.is_annotated(&m.id)));
            let name = if annotated {
                anno_group_marked_name(&name, lang)
            } else {
                name
            };
            (mark, name, annotated)
        })
        .collect();
    html! {
        div.pg__centre {
            span.pg__count.visually-hidden aria-live="polite" aria-atomic="true" {
                (picker_live_count(&live_label, page.index, page.total, lang))
            }
            details
                .pk
                id=(PICKER_ID)
                data-on:keydown__window=(PICKER_ESCAPE_EXPR)
                data-on:click__outside="el.open = false"
                data-on:focusout=(PICKER_FOCUSOUT_EXPR)
                data-on:toggle=(PICKER_TOGGLE_EXPR)
            {
                summary.pk__summary aria-label=(summary_name) {
                    (ChromeMsg::PickerSummary.text(lang))
                    " "
                    span.pk__caret aria-hidden="true" { "▴" }
                }
                div.pk__panel data-on:click=(picker_click_expr(&base)) {
                    div.pk__band {
                        span.pk__index { (index_label) }
                        @for (i, (mark, name, annotated)) in names.iter().enumerate() {
                            a   .pk__mark
                                href={ "#" (picker_group_id(i + 1)) }
                                aria-current=[(current_group == Some(i)).then_some("true")]
                            {
                                span aria-hidden="true" { (mark) }
                                @if *annotated { (ANNO_DOT) }
                                span.visually-hidden { (name) }
                            }
                        }
                        @if !page.annotated_members.is_empty() {
                            span.pk__legend { (ANNO_DOT) (ChromeMsg::AnnoMarked.text(lang)) }
                        }
                    }
                    @for (i, (g, (mark, name, annotated))) in groups.iter().zip(&names).enumerate() {
                        // A run's heading shows its span; its band mark, the
                        // first number alone.
                        @let mark = match g.kind {
                            GroupKind::Range { first, last } => format!("{first}–{last}"),
                            _ => mark.clone(),
                        };
                        @let meta = match &g.body {
                            GroupBody::Leaves(leaves) => {
                                picker_leaves_meta(leaves.len(), lang)
                            }
                            GroupBody::Flat(entries) => {
                                picker_pages_meta(entries.len(), lang)
                            }
                        };
                        section.pk__group {
                            h3  .pk__head
                                .pk__head--key[matches!(g.kind, GroupKind::Gathering { .. })]
                                id=(picker_group_id(i + 1))
                                tabindex="-1"
                            {
                                span.pk__headmark aria-hidden="true" {
                                    (mark)
                                    @if *annotated { (ANNO_DOT) }
                                }
                                span.visually-hidden { (name) }
                                " "
                                span.pk__meta { (meta) }
                            }
                            (render_picker_body(g, page, cfg, lang))
                        }
                    }
                }
            }
        }
    }
}

// ── Member scan ───────────────────────────────────────────────────────────────

/// Render the current member's scan: the
/// FIRST linked Image's safe `Image` representation as a plain,
/// non-interactive `figure.cv-member__scan > img`.
///
/// Deliberately NOT the record viewer (`component::media`): no
/// `<image-viewer>`, no `data-signals`, no element id — the host's own record
/// viewer is usually on the same page, and its `id="record-figure"` and
/// page-global signals would collide.
///
/// - `src` is `iiif_plate_url` (fetch width `PLATE_WIDTH`, paired with the `.cv-member__scan img`
///   rule in `style/record.css` — the display-box↔fetch-size coupling rule); a non-IIIF-shaped safe
///   URI is used verbatim, mirroring the record viewer's `.rv__stand` fallback.
/// - `width`/`height` come from the representation (CLS-safe with no CSS dependency), omitted when
///   unknown.
/// - `alt` is the representation's own `alt`, else `member_title` (the member's resolved name — the
///   same host-title fallback `media.rs` uses).
///
/// No image, or no representation with a safe `http(s)` URI ⇒ nothing at all
/// (no absent state).
fn render_member_scan(image: Option<&Resource>, member_title: &str) -> Markup {
    let Some(img) = member_scan_img(image, member_title) else {
        return html! {};
    };
    html! {
        figure.cv-member__scan { (img) }
    }
}

/// The member scan's `<img>` for the `contents` layout's
/// `figure.cv-member__scan`. `None` when there is no Image or no
/// representation with a safe `http(s)` URI (see [`render_member_scan`] for
/// the attribute rules).
fn member_scan_img(image: Option<&Resource>, member_title: &str) -> Option<Markup> {
    let repr = image.and_then(image_repr)?;
    let src = iiif_plate_url(&repr.uri, repr.width).unwrap_or_else(|| repr.uri.clone());
    let alt = repr.alt.as_deref().filter(|s| !s.is_empty()).unwrap_or(member_title);
    Some(html! {
        img src=(src)
            alt=(alt)
            loading="eager"
            fetchpriority="high"
            decoding="async"
            width=[repr.width]
            height=[repr.height];
    })
}

// ── `pages` layout ─────────────────────────────────

/// `data-class` binding for the `pages` article when its stage renders: the
/// record viewer unit's `$expand` (WIDER) toggles `.is-wide`, the same idiom
/// `article.record--viewer` uses (a class, never a reactive `data-*` write).
const PAGES_WIDE_CLASS_ATTR: &str = "{'is-wide': $expand}";

/// Keyboard paging (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`'s
/// sub-item 3): ←/→ page the `pages` reader while focus is inside it (the article's own `keydown`,
/// not `__window`, so the scoping is free). It clicks the step's own
/// `a[rel=prev|next]`, so the steps' handler runs; at an inert end there is
/// no link and nothing happens. It stands aside for a held key
/// (`evt.repeat` — one page per press), an already-handled event, any
/// modifier (Alt+← is the browser's Back), and focus in a form control, a
/// slider, the page picker (`details`) or the viewer unit (the canvas pans
/// with the arrows; its shadow DOM retargets to `image-viewer`).
const PAGES_KEYS_EXPR: &str = "if (evt.key !== 'ArrowLeft' && evt.key !== 'ArrowRight') return; \
     if (evt.repeat || evt.defaultPrevented || evt.shiftKey || evt.ctrlKey || evt.altKey || evt.metaKey) return; \
     if (evt.target.closest('input,select,textarea,[contenteditable],[role=slider],details,image-viewer,.rv')) return; \
     el.querySelector(evt.key === 'ArrowLeft' ? 'a[rel=prev]' : 'a[rel=next]')?.click();";

/// The id of the `pages` reader's status region, which the reader is
/// `aria-describedby` when the address is stale. Fixed, like
/// `id="compound-viewer"`.
const PAGES_STATUS_ID: &str = "cv-status";

/// Query keys the stale-address correction drops: the stale `page`, a
/// `member` beside it, and the fragment-request marker.
const URL_CORRECTION_DROP: &[&str] = &["page", "member", "datastar"];

/// Stale-address correction (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`'s
/// sub-item 1): on a full-page render of a stale `?page=`, a `history.replaceState(null, '',
/// '<url>');` that corrects the address to the page actually shown — the current path
/// and query minus [`URL_CORRECTION_DROP`]: the bare book URL on page 1 (what
/// the Copy link gives there), else that plus `page=<shown member id>`, since
/// a `?member=N` beside the stale value decides the page shown and must not
/// be lost. Replace, not push: Back still leaves the
/// book. It runs before [`READER_URL_INIT_SCRIPT`] in the same script, so
/// the seed reads the corrected `location` and there is no third
/// `__cpeReaderUrl` writer. Relative and JS-string-escaped, like the
/// router's `pushState` URL. `None` when not stale or on the fragment path.
fn stale_url_correction(page: &CompoundPage, cfg: &CompoundViewerCfg<'_>) -> Option<String> {
    if page.stale_page.is_none() || !cfg.emit_url_correction {
        return None;
    }
    let kept = cfg
        .current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| {
            let key = pair.split_once('=').map_or(*pair, |(k, _)| k);
            !pair.is_empty() && !URL_CORRECTION_DROP.contains(&key)
        })
        .collect::<Vec<&str>>()
        .join("&");
    let url = match page.member.as_ref() {
        Some(member) if page.index != 1 => member_href_for(cfg.current_path, Some(&kept), &member.id),
        _ if kept.is_empty() => cfg.current_path.to_owned(),
        _ => format!("{}?{kept}", cfg.current_path),
    };
    Some(format!(
        "history.replaceState(null, '', '{}');",
        cpe_core::util::js_string_escape(&url)
    ))
}

/// The `pages` reader's status region,
/// between the head and the stage. Always rendered, empty unless the
/// address's `?page=` named no page of this book; then it says so, with the
/// value (escaped), and names the page actually shown. HTTP stays 200 and
/// the page shown is the `?member=` fallback — the first page in practice.
fn render_pages_status(page: &CompoundPage, member: &Resource, lang: &Lang) -> Markup {
    html! {
        div.cv-status id=(PAGES_STATUS_ID) role="status" aria-live="polite" {
            @if let Some(value) = &page.stale_page {
                @let label = member_text(member, "name", lang)
                    .map_or_else(|| page.index.to_string(), str::to_owned);
                p.cv-status__msg {
                    (ChromeMsg::CompoundStaleAddress.text(lang))
                    " "
                    em { "?page=" (value) }
                }
                p.cv-status__meta { (compound_stale_shown(&label, page.index == 1, lang)) }
            }
        }
    }
}

/// The member property the `pages` head shows as its italic aside, by the
/// same naming convention that makes `name` the title.
const PAGES_ASIDE_PROPERTY: &str = "description";

/// Where CSS cannot tell whether the aside overflows its one line (no
/// scroll-driven animations), the "more" expander shows for an aside longer
/// than this many characters: about one line of the `--size-4` italic aside
/// beside the expander in the 543px reader at a 1440px window.
const ASIDE_FALLBACK_BUDGET: usize = 60;

/// A target name renders beside its step word only up to this many
/// characters; a longer one lives in the step's `title` alone, so the row
/// never wraps.
const PAGER_NAME_MAX_CHARS: usize = 10;

/// The `pages` reader head: identity only, and the
/// position shows exactly once. The label is the member's `name`:
///
/// - label ≠ position: eyebrow `{class} · n / N`, `h2` = the label;
/// - label = position: eyebrow `{class}`, `h2` = `n` plus a mono `/ N`;
/// - no label: eyebrow `{class} · no label`, `h2` = the position.
///
/// The `h2` keeps its detail link (when the member's class is bound) and
/// carries [`PAGES_TITLE_ID`]. The member's [`PAGES_ASIDE_PROPERTY`] follows
/// as `p.cv-member__aside`, its leading dash `aria-hidden`.
///
/// A page with a register adds its count to the eyebrow, a plain in-page
/// link to the register (`#annotations`, outside the swapped reader), so the
/// head still carries no control.
fn render_pages_head(
    member: &Resource,
    page: &CompoundPage,
    class_label: &str,
    title_href: Option<&str>,
    lang: &Lang,
) -> Markup {
    let position = page.index.to_string();
    let label = member_text(member, "name", lang);
    let (eyebrow, h2_text, of_total) = match label {
        None => (
            format!("{class_label} · {}", ChromeMsg::CompoundMemberUnlabelled.text(lang)),
            position.as_str(),
            false,
        ),
        Some(l) if l == position => (class_label.to_owned(), l, true),
        Some(l) => (format!("{class_label} · {} / {}", page.index, page.total), l, false),
    };
    let aside = member_text(member, PAGES_ASIDE_PROPERTY, lang);
    let long = aside.is_some_and(|a| a.chars().count() > ASIDE_FALLBACK_BUDGET);
    html! {
        // The arrival scroll's stacked target: `#cv-title`'s parent
        // (`annotation::ARRIVAL_INIT_EXPR`).
        header.cv-member__head {
            p.eyebrow.cv-member__eyebrow {
                (eyebrow)
                @if !page.annotations.is_empty() {
                    " "
                    a.linklike.cv-anno-count href={ "#" (annotation::REGISTER_ID) } {
                        (ANNO_DOT)
                        (anno_count_text(page.annotations.len(), lang))
                    }
                }
            }
            h2.cv-member__title id=(PAGES_TITLE_ID) {
                @if let Some(href) = title_href {
                    a href=(href) { (h2_text) }
                } @else { (h2_text) }
                @if of_total {
                    span.cv-member__of { " / " (page.total) }
                }
            }
            // Always rendered: an empty aside still reserves its one line,
            // so the stage and the pager sit at the same height on every
            // page.
            div.cv-member__aside.cv-member__aside--long[long] {
                @if let Some(aside) = aside {
                    span.cv-aside__text {
                        span aria-hidden="true" { "— " }
                        (aside)
                    }
                    details.cv-aside__more {
                        summary {
                            span.cv-aside__open { (ChromeMsg::CompoundAsideMore.text(lang)) }
                            span.cv-aside__close { (ChromeMsg::CompoundAsideLess.text(lang)) }
                        }
                    }
                }
            }
        }
    }
}

/// The `member-links` row under the `pages` pager:
/// `p.cv-links`, its caption, then every `link` value of the listed member
/// properties in order, each through `resource_detail::render_value` (an
/// `a.int-link` to the target's detail page when its class is bound).
/// Inside `#compound-viewer`, so it swaps with the page; below the pager, so
/// its presence never moves the pager. No link value ⇒ no row.
fn render_member_links(member: &Resource, cfg: &CompoundViewerCfg<'_>, lang: &Lang) -> Markup {
    let Some((caption, properties)) = cfg.member_links else {
        return html! {};
    };
    let values: Vec<&PropertyValue> = properties
        .iter()
        .filter_map(|name| member.properties.iter().find(|p| &p.name == name))
        .flat_map(|p| p.values.iter())
        .filter(|v| matches!(v, PropertyValue::Link { .. }))
        .collect();
    if values.is_empty() {
        return html! {};
    }
    html! {
        p.cv-links {
            span.cv-links__label { (caption) }
            " "
            @for (i, value) in values.iter().enumerate() {
                @if i > 0 {
                    span aria-hidden="true" { " · " }
                }
                (render_value(value, lang, cfg.links.as_ref()))
            }
        }
    }
}

/// The `pages` layout's stage: `div.cv-stage`, a plain sizing wrapper holding
/// the record viewer's own unit (`media::render_viewer_unit`:
/// the reader's scan is the full IIIF viewer exactly as Alice's
/// record pages have it, reused, never a fork). The unit's `.rv__banner`
/// carries the Image's legal run above the canvas.
///
/// One member image per document: the unit gets `member_media[..1]`, so it
/// takes the single-image path — no `$plate` paging, no source swap. Its id
/// follows the record viewer's own `rv-{first image id}` rule. A non-IIIF URI
/// degrades inside the unit to its `.rv__stand` plain `img`, exactly as on a
/// record page.
///
/// The page's drawn annotations ride in the unit's fallback as the overlay
/// layer, so the zero-JS copy and the viewer's adopted copy are one markup;
/// the unit draws them only when its `<image-viewer>` mounts and the image
/// has both dimensions.
///
/// No Image / no safe representation ⇒ `None`: no stage, no absent state,
/// and so no WIDER host wiring either.
fn render_pages_stage(page: &CompoundPage, member_title: &str, lang: &Lang) -> Option<Markup> {
    let image = page.member_media.first()?;
    image_repr(image)?;
    let viewer_id = format!("rv-{}", image.id);
    let overlay = render_overlay(&page.annotations);
    // Only a stage whose viewer draws the shapes wires them: the effect hands
    // the toggle and focus to the viewer, and the viewer's bubbling overlay
    // events set hover and pin (the `Viewer` builder has no attribute hook).
    let drawn = overlay.is_some() && overlay_will_draw(image);
    let effect = drawn.then(|| annotation::overlay_effect_expr(&viewer_id));
    let on_hover = drawn.then_some(annotation::OVERLAY_HOVER_EXPR);
    let on_click = drawn.then(annotation::overlay_click_expr);
    // A page-level arrival has a note but no shape, so this follows the
    // register, not the overlay.
    let on_zoom = (!page.annotations.is_empty()).then(annotation::zoom_away_expr);
    Some(html! {
        div .cv-stage
            data-effect=[effect]
            data-on:viewer-overlay-hover=[on_hover]
            data-on:viewer-overlay-click=[on_click]
            "data-on:viewer-zoom__debounce.250ms"=[on_zoom]
        {
            ({
                render_viewer_unit(
                    &page.member_media[..1],
                    0,
                    &viewer_id,
                    member_title,
                    false,
                    overlay.as_ref(),
                    lang,
                )
            })
        }
    })
}

/// One active step of the `pages` pager row: the plain href, its
/// `@get(...)` fragment counterpart, the target member's label, and whether
/// that member has a register.
struct ReaderStep {
    href: String,
    get: String,
    target: String,
    annotated: bool,
}

/// The `pages` reader's pager row: `nav.pg.pg--reader`,
/// three tracks — Zurück, `centre` (the live count, or the page picker with
/// its own hidden live count), Weiter.
///
/// - **Names (WCAG 2.5.3).** No `aria-label` on a step: the arrow is `aria-hidden`, so the
///   accessible name is the visible word plus the visible `span.pg__name` ("Zurück a1v"); the `·`
///   separator is `aria-hidden` too. The name shows only for a target of at most
///   [`PAGER_NAME_MAX_CHARS`] characters; the full target is always the step's `title`.
/// - **Inert end.** `span.pg__step[aria-disabled="true"]` keeps its word so the row never changes
///   width, and is NOT `aria-hidden`: a screen reader reads it as unavailable, like the browse
///   pager's inert step.
/// - Active steps are `rel="prev"`/`rel="next"` links that also carry the `_compound` swap, exactly
///   as [`render_compound_pager`]'s.
/// - **Annotated neighbour.** The accent dot sits at the step's end nearer the centre and stays
///   when the name is hidden; the fact is the visually hidden [`anno_marked_suffix`] at the end of
///   the step's name, never an `aria-label` on the dot.
fn render_reader_pager(prev: Option<ReaderStep>, next: Option<ReaderStep>, centre: Markup, lang: &Lang) -> Markup {
    let (prev_word, next_word) = compound_step_labels(lang);
    let short = |s: &ReaderStep| {
        let n = s.target.chars().count();
        n > 0 && n <= PAGER_NAME_MAX_CHARS
    };
    html! {
        nav.pg.pg--reader aria-label=(ChromeMsg::CompoundPagerLabel.text(lang)) {
            @if let Some(step) = &prev {
                a   .pg__step
                    href=(step.href)
                    rel="prev"
                    title=[(!step.target.is_empty()).then_some(&step.target)]
                    data-on:click__prevent=(step.get)
                {
                    span aria-hidden="true" { "←" }
                    " "
                    span.pg__word { (prev_word) }
                    @if short(step) {
                        " "
                        span.pg__name {
                            span aria-hidden="true" { "· " }
                            (step.target)
                        }
                    }
                    @if step.annotated {
                        (ANNO_DOT)
                        span.visually-hidden { (anno_marked_suffix(lang)) }
                    }
                }
            } @else {
                span.pg__step aria-disabled="true" {
                    span aria-hidden="true" { "←" }
                    " "
                    span.pg__word { (prev_word) }
                }
            }
            (centre)
            @if let Some(step) = &next {
                a   .pg__step
                    href=(step.href)
                    rel="next"
                    title=[(!step.target.is_empty()).then_some(&step.target)]
                    data-on:click__prevent=(step.get)
                {
                    @if step.annotated { (ANNO_DOT) }
                    @if short(step) {
                        span.pg__name {
                            (step.target)
                            span aria-hidden="true" { " ·" }
                        }
                        " "
                    }
                    span.pg__word { (next_word) }
                    @if step.annotated {
                        span.visually-hidden { (anno_marked_suffix(lang)) }
                    }
                    " "
                    span aria-hidden="true" { "→" }
                }
            } @else {
                span.pg__step aria-disabled="true" {
                    span.pg__word { (next_word) }
                    " "
                    span aria-hidden="true" { "→" }
                }
            }
        }
    }
}

// ── Prev/next pager (DataStar-enhanced, compound-local) ────────────────────────

/// The compound-viewer's own `.pg` prev/next pager renderer:
/// the `.pg` markup shape with `data-on:click__prevent`
/// wired onto each active (non-inert) step, beside the plain href
/// (`component::media` hand-rolls its own `.pg` markup too — see `pager.rs`'s module doc).
///
/// Each `Some((href, get))` pair is one active step: `href` is the plain
/// no-JS/initial-load `<a href>` (unchanged), `get` is the ALREADY-BUILT
/// `@get(...)` expression (see [`compound_fragment_get`]) for
/// `data-on:click__prevent`. `None` renders the inert boundary
/// `span.pg__step[aria-disabled][aria-hidden]` — carrying neither `href`
/// nor `data-on`, since there is nothing to enhance.
fn render_compound_pager(
    nav_aria: &str,
    prev_label: &str,
    next_label: &str,
    count_text: &str,
    prev: Option<(&str, &str)>,
    next: Option<(&str, &str)>,
) -> Markup {
    html! {
        nav.pg aria-label=(nav_aria) {
            @if let Some((h, get)) = prev {
                a.pg__step href=(h) data-on:click__prevent=(get) aria-label=(prev_label) { "←" }
            } @else {
                span.pg__step aria-disabled="true" aria-hidden="true" { "←" }
            }
            span.pg__count aria-live="polite" aria-atomic="true" { (count_text) }
            @if let Some((h, get)) = next {
                a.pg__step href=(h) data-on:click__prevent=(get) aria-label=(next_label) { "→" }
            } @else {
                span.pg__step aria-disabled="true" aria-hidden="true" { "→" }
            }
        }
    }
}

// ── Query-string builder ──────────────────────────────────────────────────────

/// Build a full href for a compound prev/next or contents-rail link:
/// [`member_href_for`] with the current query — see [`reader_query`] for the
/// `page=<member_id>` / stale-`member=` rule.
fn build_compound_href(current_path: &str, current_query: Option<&str>, member_id: &str) -> String {
    member_href_for(current_path, current_query, member_id)
}

/// The fragment-endpoint counterpart to [`build_compound_href`]:
/// the SAME [`reader_query`] (so
/// `page=<member_id>` and every other preserved param match the plain href
/// exactly), but rooted at `{path}/{COMPOUND_FRAGMENT_SEGMENT}` instead of
/// `path` itself. Neither this nor [`member_href_for`] ever appends a
/// `#compound-viewer` suffix — the
/// fragment response patches `#compound-viewer` directly by DOM id, not by
/// following an anchor, and a forward-click swap's focus lands there via a
/// programmatic `ExecuteScript` call, not a browser anchor jump. Shares
/// its query-building base with `build_compound_href`/`member_href_for` on
/// purpose: the plain-nav href and the fragment href must never drift on
/// which params they carry.
fn compound_fragment_href(current_path: &str, current_query: Option<&str>, member_id: &str) -> String {
    let qs = reader_query(current_query, member_id);
    format!("{current_path}/{COMPOUND_FRAGMENT_SEGMENT}?{qs}")
}

/// [`compound_fragment_href`] wrapped as a DataStar `@get(...)` expression,
/// ready to drop straight into a `data-on:click__prevent` attribute.
fn compound_fragment_get(current_path: &str, current_query: Option<&str>, member_id: &str) -> String {
    format!(
        "@get('{}')",
        encode_get_arg(&compound_fragment_href(current_path, current_query, member_id))
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{CompoundPage, MemberHeader, Property, PropertyValue, Resource};

    use super::*;
    use crate::component::pager::override_query_param;

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

    fn make_chapter_resource(id: &str, name: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: "StoryChapter".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: name.to_owned(), lang: Some("en".to_owned()) }],
            }],
            representations: vec![],
        }
    }

    fn make_chapter_resource_with_desc(id: &str, name: &str, desc: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: "StoryChapter".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: name.to_owned(), lang: Some("en".to_owned()) }],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![PropertyValue::Text { text: desc.to_owned(), lang: Some("en".to_owned()) }],
                },
            ],
            representations: vec![],
        }
    }

    fn make_routes() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("StoryChapter".to_owned(), "/chapters/:id".to_owned());
        m
    }

    fn make_resolver<'a>(project_id: &'a str, routes: &'a HashMap<String, String>) -> LinkResolver<'a> {
        LinkResolver::new(cpe_core::url::ROOT.project(project_id), routes)
    }

    fn make_cfg<'a>(
        routes: Option<&'a HashMap<String, String>>,
        current_path: &'a str,
        current_query: Option<&'a str>,
    ) -> CompoundViewerCfg<'a> {
        CompoundViewerCfg {
            links: routes.map(|r| make_resolver("alice-in-daschland", r)),
            current_path,
            current_query,
            media_property: None,
            class_labels: &[],
            property_labels: &[],
            parent: None,
            picker: None,
            layout: CompoundLayout::Contents,
            index: MemberIndex::Rail,
            paired: false,
            date_precision_word: false,
            member_links: None,
            emit_url_correction: false,
        }
    }

    /// Like `make_cfg`, but with a compound parent Resource set (empty-state tests).
    fn make_cfg_with_parent<'a>(
        routes: Option<&'a HashMap<String, String>>,
        current_path: &'a str,
        current_query: Option<&'a str>,
        parent: Option<&'a Resource>,
    ) -> CompoundViewerCfg<'a> {
        CompoundViewerCfg { parent, ..make_cfg(routes, current_path, current_query) }
    }

    /// The markup of the first element opening with `open` up to its first
    /// `close` — scopes a "no `data-on`" assertion to one element (the pager
    /// `nav.pg`, the contents rail), never the whole article.
    fn element<'o>(out: &'o str, open: &str, close: &str) -> &'o str {
        let start = out.find(open).unwrap_or_else(|| panic!("`{open}` missing, got: {out}"));
        let end = out[start..]
            .find(close)
            .unwrap_or_else(|| panic!("`{close}` after `{open}` missing, got: {out}"));
        &out[start..start + end + close.len()]
    }

    /// The prev/next pager (`nav.pg`).
    fn pager(out: &str) -> &str {
        element(out, "<nav class=\"pg\"", "</nav>")
    }

    /// The contents rail (`nav.cv-toc__contents`).
    fn rail(out: &str) -> &str {
        element(out, "<nav class=\"cv-toc__contents\"", "</nav>")
    }

    /// Test builder for `CompoundPage`, which avoids repeating the field list at ~20 call sites.
    /// Pass `vec![]` for `members` in tests that don't exercise the contents
    /// rail; pass a `members` vec consistent with `total` for rail-focused
    /// tests (see `make_story_headers`).
    ///
    /// The prev/next hrefs read the
    /// neighbour's id from `members`, so an empty `members` with
    /// `total >= 2` would break the `members.len() == total` invariant the
    /// renderer relies on. Such a call is filled with `total` generic
    /// `SCH_001…` headers (the ids the tests' current-member resources use).
    fn make_compound_page(
        member: Option<Resource>,
        index: u32,
        total: usize,
        members: Vec<MemberHeader>,
    ) -> CompoundPage {
        let members = if members.is_empty() && total >= 2 {
            (1..=total)
                .map(|n| MemberHeader {
                    id: format!("SCH_{n:03}"),
                    class: "StoryChapter".to_owned(),
                    name: format!("Chapter {n}"),
                })
                .collect()
        } else {
            members
        };
        CompoundPage {
            member,
            index,
            total,
            members,
            member_media: vec![],
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        }
    }

    /// Thirteen `MemberHeader`s mirroring Alice's Story chapters, in
    /// `SCH_001..SCH_013` id order. Position 1 ("Down the Rabbit Hole"),
    /// position 7 ("A Mad Tea-Party"), and position 13 ("Alice's Evidence")
    /// intentionally match the current-member resources used by the
    /// first/middle/last snapshot tests below.
    fn make_story_headers() -> Vec<MemberHeader> {
        const TITLES: [&str; 13] = [
            "Down the Rabbit Hole",
            "The Pool of Tears",
            "A Caucus-Race and a Long Tale",
            "The Rabbit Sends in a Little Bill",
            "Advice from a Caterpillar",
            "Pig and Pepper",
            "A Mad Tea-Party",
            "The Queen's Croquet-Ground",
            "The Mock Turtle's Story",
            "The Lobster Quadrille",
            "Who Stole the Tarts?",
            "The Rabbit-Hole Revisited",
            "Alice's Evidence",
        ];
        TITLES
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("SCH_{:03}", i + 1),
                class: "StoryChapter".to_owned(),
                name: (*name).to_owned(),
            })
            .collect()
    }

    // ── Layer-1: override_query_param ─────────────────────────────────────────

    #[test]
    fn override_query_param_appends_when_absent() {
        let qs = override_query_param(Some("lang=de"), "member", "2");
        assert_eq!(qs, "lang=de&member=2");
    }

    #[test]
    fn override_query_param_replaces_existing() {
        let qs = override_query_param(Some("lang=de&member=1"), "member", "3");
        assert_eq!(qs, "lang=de&member=3");
    }

    #[test]
    fn override_query_param_none_query_produces_single_param() {
        let qs = override_query_param(None, "member", "2");
        assert_eq!(qs, "member=2");
    }

    #[test]
    fn override_query_param_preserves_lang_and_appearance() {
        let qs = override_query_param(Some("lang=fr&appearance=dark&member=1"), "member", "2");
        assert_eq!(qs, "lang=fr&appearance=dark&member=2");
    }

    // ── Layer-1: build_compound_href ──────────────────────────────────────────

    #[test]
    fn build_compound_href_no_existing_query() {
        let href = build_compound_href("/alice-in-daschland/story", None, "SCH_002");
        assert_eq!(href, "/alice-in-daschland/story?page=SCH_002");
    }

    #[test]
    fn build_compound_href_preserves_lang_and_clears_stale_member() {
        let href = build_compound_href("/alice-in-daschland/story", Some("lang=de&member=1"), "SCH_003");
        assert_eq!(href, "/alice-in-daschland/story?lang=de&page=SCH_003");
    }

    #[test]
    fn build_compound_href_carries_no_compound_viewer_fragment() {
        let href = build_compound_href("/alice-in-daschland/story", Some("lang=de"), "SCH_001");
        assert!(
            !href.contains('#'),
            "href must carry no fragment (owner decision 2026-09-29), got: {href}"
        );
    }

    /// `compound_fragment_get`'s `@get('…')` must pass the whole href through
    /// `encode_get_arg` before embedding it — a `current_query` value
    /// containing `'`, `` ` `` or `\` must come out percent-encoded in the
    /// `data-on:click__prevent` expression, never spliced in raw (this would
    /// otherwise let a crafted query value break out of the single-quoted
    /// JS string literal inside `@get(...)`).
    #[test]
    fn pager_click_prevent_escapes_query_special_chars() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_002", "Middle Chapter")), 2, 3, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("q=a'b`c\\d"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            pager(&out).contains(
                "data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?\
                 q=a%27b%60c%5Cd&amp;page=SCH_001')\""
            ),
            "the @get expression must be encode_get_arg-escaped, got: {out}"
        );
        assert!(
            !out.contains("@get('/alice-in-daschland/story/_compound?q=a'b`c\\d"),
            "the raw, unescaped query value must never appear inside a @get \
             expression (the plain href fallback is unaffected — it's a \
             regular URL attribute, not a JS string literal), got: {out}"
        );
    }

    // ── Layer-1: inert-end behaviour ──────────────────────────────────────────

    /// At index == 1 (first member), prev must be an inert span (aria-disabled).
    #[test]
    fn inert_prev_at_first_member() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Down the Rabbit Hole")), 1, 3, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        // First position: prev must be inert.
        assert!(
            out.contains("aria-disabled=\"true\""),
            "first member must have inert prev span, got: {out}"
        );
        // Next must be active: a plain href — a real page navigation.
        assert!(
            out.contains("href=\"/alice-in-daschland/story?page=SCH_002\""),
            "first member next must point to page=SCH_002, got: {out}"
        );
        assert!(
            pager(&out).contains("data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?page=SCH_002')\""),
            "the active next step also carries the fragment-swap @get, got: {out}"
        );
        // Negative: links address members by id — no emitted `member=` at all.
        assert!(
            !out.contains("member="),
            "no emitted href may carry the retired member= key, got: {out}"
        );
        // Inert prev span must NOT carry data-on.
        let inert_span_start = out.find("aria-disabled=\"true\"").expect("aria-disabled present");
        // Find the opening tag that contains aria-disabled.
        let tag_before = &out[..inert_span_start];
        let span_open = tag_before.rfind('<').expect("inert span opening tag");
        let inert_tag_slice = &out[span_open..inert_span_start + "aria-disabled=\"true\"".len()];
        assert!(
            !inert_tag_slice.contains("data-on"),
            "inert prev span must NOT carry data-on, got tag slice: {inert_tag_slice}"
        );
    }

    /// At index == total (last member), next must be an inert span (aria-disabled).
    #[test]
    fn inert_next_at_last_member() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_003", "The Third Chapter")), 3, 3, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        // Last position: next must be inert.
        assert!(
            out.contains("aria-disabled=\"true\""),
            "last member must have inert next span, got: {out}"
        );
        // Prev must be active: a plain href — a real page navigation.
        assert!(
            out.contains("href=\"/alice-in-daschland/story?page=SCH_002\""),
            "last member prev must point to page=SCH_002, got: {out}"
        );
        assert!(
            pager(&out).contains("data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?page=SCH_002')\""),
            "the active prev step also carries the fragment-swap @get, got: {out}"
        );
        // Negative: must not have href beyond total (no 4th member id exists).
        assert!(!out.contains("SCH_004"), "href must not point beyond total, got: {out}");
    }

    /// Middle member: both prev and next are active plain anchors.
    #[test]
    fn both_active_at_middle_member() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_002", "Middle Chapter")), 2, 3, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("lang=en"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        // Both prev (SCH_001) and next (SCH_003) must be active.
        // Maud HTML-escapes `&` to `&amp;` in href attributes.
        assert!(
            out.contains("href=\"/alice-in-daschland/story?lang=en&amp;page=SCH_001\""),
            "middle member prev must point to page=SCH_001 with lang preserved, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/story?lang=en&amp;page=SCH_003\""),
            "middle member next must point to page=SCH_003 with lang preserved, got: {out}"
        );
        assert!(
            pager(&out).contains(
                "data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?lang=en&amp;page=SCH_001')\""
            ) && pager(&out).contains(
                "data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?lang=en&amp;page=SCH_003')\""
            ),
            "both active steps carry the fragment-swap @get, got: {out}"
        );
        // No inert spans.
        assert!(
            !out.contains("aria-disabled=\"true\""),
            "middle member must have no inert spans, got: {out}"
        );
    }

    // ── Layer-1: real navigation + fragment-swap enhancement ──────────────────

    /// Member paging is a real page load (every active pager step / rail
    /// link carries a plain `href` — the byte-identical no-JS/initial-load
    /// path, unchanged) AND is progressively enhanced: every active step
    /// ALSO carries `data-on:click__prevent="@get(…/_compound?…)"`, while
    /// inert boundary spans carry neither.
    #[test]
    fn rail_links_and_pager_steps_carry_both_href_and_data_on() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_007", "A Mad Tea-Party")),
            7,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("lang=en"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        let pager = pager(&out);
        let rail = rail(&out);
        assert_eq!(pager.matches("<a ").count(), 2, "both steps active, got: {pager}");
        assert_eq!(rail.matches("<a ").count(), 13, "one rail link per member, got: {rail}");
        for (name, el) in [("pager", pager), ("rail", rail)] {
            assert!(
                el.contains("data-on:click__prevent=\"@get('"),
                "{name} active links must carry the fragment-swap @get, got: {el}"
            );
            assert_eq!(
                el.matches("data-on:click__prevent").count(),
                el.matches("<a ").count(),
                "{name}: every active <a> gets exactly one data-on, got: {el}"
            );
        }
        // Every emitted @get targets the revived _compound fragment endpoint.
        assert!(
            out.contains("/story/_compound?"),
            "active links must address the _compound endpoint, got: {out}"
        );
    }

    /// Inert boundary spans must NOT carry `data-on:click__prevent`.
    #[test]
    fn inert_span_has_no_data_on() {
        let routes = make_routes();
        // At index=1 prev is inert; at index=total next is inert.
        let page_first = make_compound_page(Some(make_chapter_resource("SCH_001", "First")), 1, 2, vec![]);
        let page_last = make_compound_page(Some(make_chapter_resource("SCH_002", "Last")), 2, 2, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);

        let out_first = render_compound_viewer(&page_first, &cfg, &lang).into_string();
        let out_last = render_compound_viewer(&page_last, &cfg, &lang).into_string();

        // Each output has exactly one inert span; that span must not carry data-on.
        for out in [&out_first, &out_last] {
            let inert_start = out.find("aria-disabled=\"true\"").expect("inert span present");
            let tag_start = out[..inert_start].rfind('<').expect("tag start");
            // Grab from the tag open to just past aria-disabled (covers the full inert tag).
            let tag_end = out[inert_start..].find('>').expect("tag close");
            let inert_tag = &out[tag_start..inert_start + tag_end];
            assert!(
                !inert_tag.contains("data-on"),
                "inert span must not carry data-on, got: {inert_tag}"
            );
        }
    }

    /// Single member (total=1): no nav rendered at all, so no
    /// `data-on:click__prevent`/`data-on:submit__prevent` — but the root
    /// still unconditionally carries its own `data-on:popstate__window`
    /// (every populated
    /// compound-viewer root needs Back/Forward to work, regardless of
    /// whether it has a pager or rail at all), and the plain
    /// `window.__cpeReaderUrl` init script the listener compares against.
    #[test]
    fn single_member_no_data_on() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Only Chapter")), 1, 1, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        assert!(
            !out.contains("data-on:click") && !out.contains("data-on:submit"),
            "total=1 must have no nav data-on (no nav rendered), got: {out}"
        );
        assert!(
            out.contains(
                r#"data-on:popstate__window="if (location.pathname + location.search !== window.__cpeReaderUrl) { location.reload(); }""#
            ),
            "the root must still carry the reload-on-popstate listener, got: {out}"
        );
        assert!(
            out.contains("<script>window.__cpeReaderUrl = location.pathname + location.search;</script>"),
            "the root must seed window.__cpeReaderUrl via an inline script, got: {out}"
        );
        assert!(
            !out.contains("data-signals") && !out.contains("currentPageQuery") && !out.contains("X-CPE-Popstate"),
            "Option B must not reintroduce the retired DataStar-signal/header \
             machinery, got: {out}"
        );
    }

    /// Option B: the popstate listener compares `location.pathname +
    /// location.search` only — never `location.hash` — so a same-document
    /// hash-only history move (the skip-link, an incoming-links row's
    /// in-page anchor) never triggers a reload.
    #[test]
    fn popstate_listener_never_compares_location_hash() {
        assert!(
            !POPSTATE_RELOAD_EXPR.contains("location.hash"),
            "the popstate listener must never read location.hash, got: {POPSTATE_RELOAD_EXPR}"
        );
        assert!(
            POPSTATE_RELOAD_EXPR.contains("location.reload()"),
            "Back/Forward must reload the page (Option B), got: {POPSTATE_RELOAD_EXPR}"
        );
        assert!(
            !POPSTATE_RELOAD_EXPR.contains('@'),
            "the popstate listener must never call @get any more, got: {POPSTATE_RELOAD_EXPR}"
        );
    }

    /// Empty compound (total=0): no nav, no data-on.
    #[test]
    fn empty_compound_no_data_on() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(None, "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        assert!(
            !out.contains("data-on"),
            "total=0 must have no data-on (empty state, no nav), got: {out}"
        );
    }

    // ── Layer-1: total == 1 → no nav ─────────────────────────────────────────

    #[test]
    fn total_one_renders_no_nav() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Only Chapter")), 1, 1, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(!out.contains("class=\"pg\""), "total=1 must not have a .pg pager, got: {out}");
        // Member title must still appear.
        assert!(out.contains("Only Chapter"), "total=1 must still show member title, got: {out}");
        // Layout contract: the root MUST carry `.page` (the shared centered
        // box). Without it the content hugs the viewport edges — see the
        // LAYOUT CONTRACT note in style/grid.css.
        assert!(
            out.contains("class=\"compound-viewer page\""),
            "member root must carry `.page` (centered box), got: {out}"
        );
        // total==1 must NOT render the contents rail.
        assert!(
            !out.contains("cv-toc"),
            "total=1 must not render the .cv-toc contents rail, got: {out}"
        );
    }

    // ── Layer-1: total == 0 → empty-state ────────────────────────────────────

    #[test]
    fn total_zero_renders_empty_state_no_nav() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(None, "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("This compound has no members."),
            "empty compound must show EN empty-state message, got: {out}"
        );
        assert!(
            !out.contains("class=\"pg\""),
            "empty compound must not have a .pg pager, got: {out}"
        );
        // Layout contract: even the empty-state root carries `.page` so the
        // message is centered, not edge-hugging (see grid.css LAYOUT CONTRACT).
        assert!(
            out.contains("class=\"compound-viewer page\""),
            "empty-state root must carry `.page` (centered box), got: {out}"
        );
        assert!(
            !out.contains("<article"),
            "empty compound must not have an article container, got: {out}"
        );
        // The empty branch's root carries the SAME no-JS focus anchor as the
        // populated branches.
        assert!(
            out.contains("id=\"compound-viewer\""),
            "empty-branch root must carry id=\"compound-viewer\", got: {out}"
        );
        assert!(
            out.contains("tabindex=\"-1\""),
            "empty-branch root must carry tabindex=\"-1\", got: {out}"
        );
    }

    #[test]
    fn total_zero_empty_state_de() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("de");
        let cfg = make_cfg(None, "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        assert!(
            out.contains("Dieses Compound hat keine Elemente."),
            "DE empty-state must appear, got: {out}"
        );
    }

    // ── Layer-1: empty compound with a resolved parent ──────────────────

    /// A resolved compound parent renders `.cv-compound-head` (eyebrow + big
    /// italic title) + the class-aware `compound_no_members_text` sentence + the
    /// parent's own `.props` — no `.cv-toc`, no member head, no pager.
    #[test]
    fn empty_compound_with_parent_renders_head_and_props() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let parent = make_chapter_resource_with_desc("ST_001", "Alice's Adventures in Wonderland", "The first story.");
        let cfg = make_cfg_with_parent(None, "/alice-in-daschland/wonderland", None, Some(&parent));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("class=\"cv-compound-head\""),
            ".cv-compound-head must render when the parent resolves, got: {out}"
        );
        assert!(
            out.contains("Alice's Adventures in Wonderland"),
            "parent's title must appear, got: {out}"
        );
        assert!(
            out.contains("StoryChapter"),
            "eyebrow must show the (humanized, undeclared-label) class name, got: {out}"
        );
        assert!(
            out.contains("No parts are recorded for this StoryChapter."),
            "must show the class-aware compound.no_members sentence, got: {out}"
        );
        assert!(
            out.contains("The first story."),
            "parent's own .props (its description property) must render, got: {out}"
        );
        assert!(
            !out.contains("cv-toc") && !out.contains("class=\"pg\""),
            "empty branch must render no contents rail and no pager, got: {out}"
        );
        assert!(
            out.contains("id=\"compound-viewer\"") && out.contains("tabindex=\"-1\""),
            "empty-branch root must carry the no-JS focus anchor, got: {out}"
        );
    }

    /// The empty branch renders the parent's property rows in its own
    /// `div.cv-member__props`, outside any record: `grid.css`'s plain-record
    /// rules select on `.record` and on `.props`, and must reach neither.
    #[test]
    fn empty_compound_with_parent_renders_its_rows_outside_any_record() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let parent = make_chapter_resource_with_desc("ST_001", "Wonderland", "The first story.");
        let cfg = make_cfg_with_parent(None, "/alice-in-daschland/wonderland", None, Some(&parent));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(
            out.contains(r#"<div class="cv-member__props"><div class="prop">"#),
            "canary: the rows render: {out}"
        );
        assert!(!out.contains(r#"class="props""#), "{out}");
        assert!(!out.contains(r#"class="record"#), "{out}");
        assert!(!out.contains("record__"), "{out}");
    }

    /// Parent's "name" property is excluded from `.props` (shown as the title
    /// instead) — mirrors the populated-branch member-props exclusion.
    #[test]
    fn empty_compound_with_parent_excludes_name_from_props() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let parent = make_chapter_resource("ST_001", "Alice's Adventures in Wonderland");
        let cfg = make_cfg_with_parent(None, "/alice-in-daschland/wonderland", None, Some(&parent));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        // The title appears once (in the head), and the props block must not
        // ALSO render a "name" prop row.
        assert!(
            !out.contains("class=\"prop__label\">Name<"),
            "the parent's \"name\" property must not appear as a plain .props row, got: {out}"
        );
    }

    /// Parent missing (source id doesn't resolve) — degrades to the sentence
    /// alone: no `.cv-compound-head`, no `.props`, but still the no-JS focus
    /// anchor.
    #[test]
    fn empty_compound_parent_absent_degrades_to_sentence_alone() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg_with_parent(None, "/alice-in-daschland/wonderland", None, None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("This compound has no members."),
            "parent-absent must show the generic (non-class-aware) empty message, got: {out}"
        );
        assert!(
            !out.contains("cv-compound-head"),
            "parent-absent must not render .cv-compound-head, got: {out}"
        );
        assert!(
            !out.contains("class=\"cv-member__props\""),
            "parent-absent must not render a .props block, got: {out}"
        );
        assert!(
            out.contains("id=\"compound-viewer\"") && out.contains("tabindex=\"-1\""),
            "parent-absent root must still carry the no-JS focus anchor, got: {out}"
        );
    }

    /// A PAIRED compound-viewer (a sibling `resource-detail`
    /// on the same page already renders this parent resource) must degrade
    /// to the sentence alone even when the parent DOES resolve — no
    /// `.cv-compound-head`, no `h2`, no `.props` — because that record
    /// already owns the title/eyebrow/properties (`dsp-incubator
    /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md`).
    #[test]
    fn empty_compound_paired_degrades_to_sentence_alone_even_with_parent() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let parent = make_chapter_resource_with_desc("B1", "Der Ritter vom Turn", "A book with no pages yet.");
        let routes = make_routes();
        let cfg = CompoundViewerCfg {
            paired: true,
            ..make_cfg_with_parent(Some(&routes), "/incunabula/books/B1", None, Some(&parent))
        };
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("No parts are recorded"),
            "paired empty branch must still show a 'no members' sentence, got: {out}"
        );
        assert!(
            !out.contains("cv-compound-head"),
            "paired empty branch must not render .cv-compound-head, got: {out}"
        );
        assert!(
            !out.contains("<h2"),
            "paired empty branch must not render an h2 (the sibling record already has one), got: {out}"
        );
        assert!(
            !out.contains("class=\"cv-member__props\""),
            "paired empty branch must not render a .props block, got: {out}"
        );
        assert!(
            out.contains("id=\"compound-viewer\"") && out.contains("tabindex=\"-1\""),
            "paired empty branch root must still carry the no-JS focus anchor, got: {out}"
        );
    }

    /// The unpaired case is unchanged: no sibling `resource-detail`, parent
    /// resolves ⇒ still the full `.cv-compound-head` + `.props` degrade
    /// (mirrors `snapshot_compound_viewer_empty_with_parent`, asserted
    /// directly rather than via snapshot so the two branches' behaviour is
    /// pinned side-by-side in one place).
    #[test]
    fn empty_compound_unpaired_still_renders_parent_head_and_props() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let parent = make_chapter_resource_with_desc("ST_001", "Alice's Adventures in Wonderland", "The first story.");
        let routes = make_routes();
        let cfg = make_cfg_with_parent(Some(&routes), "/alice-in-daschland/wonderland", None, Some(&parent));
        assert!(!cfg.paired, "precondition: make_cfg defaults to unpaired");
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("cv-compound-head"),
            "unpaired empty branch must still render .cv-compound-head, got: {out}"
        );
        assert!(
            out.contains("class=\"cv-member__props\""),
            "unpaired empty branch must still render a .props block, got: {out}"
        );
    }

    // ── Layer-3: empty-compound snapshots ──────────────────────────────────────────────────

    /// (f) Empty compound WITH a resolved parent — `.cv-compound-head` + props.
    #[test]
    fn snapshot_compound_viewer_empty_with_parent() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let parent = make_chapter_resource_with_desc("ST_001", "Alice's Adventures in Wonderland", "The first story.");
        let routes = make_routes();
        let cfg = make_cfg_with_parent(Some(&routes), "/alice-in-daschland/wonderland", None, Some(&parent));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (g) Empty compound with a missing parent — degrade to sentence alone.
    #[test]
    fn snapshot_compound_viewer_empty_parent_absent() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg_with_parent(None, "/alice-in-daschland/wonderland", None, None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    // ── Layer-1: link-resolver binding ───────────────────────────────────────

    /// Member with a bound class → title renders as an `<a>`.
    #[test]
    fn bound_class_renders_title_as_link() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Down the Rabbit Hole")), 1, 1, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("href=\"/alice-in-daschland/chapters/SCH_001\""),
            "bound class must render title as a link, got: {out}"
        );
    }

    /// Member with no link resolver → title renders as plain text (no `<a>`).
    #[test]
    fn no_resolver_renders_title_as_plain_text() {
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Down the Rabbit Hole")), 1, 1, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(None, "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            !out.contains("<a ") && !out.contains("<a>"),
            "no resolver must render title as plain text (no <a> element), got: {out}"
        );
        assert!(out.contains("Down the Rabbit Hole"), "title text must still appear, got: {out}");
    }

    // ── Layer-1: count display ────────────────────────────────────────────────

    #[test]
    fn count_text_shows_n_of_total() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_005", "Chapter Five")), 5, 13, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        assert!(out.contains("5 / 13"), "count text must show '5 / 13', got: {out}");
    }

    /// `span.pg__count` carries `aria-live="polite" aria-atomic="true"`
    /// — member paging is a same-document
    /// swap, so the live region is what announces a page step. Same
    /// idiom as `media.rs`'s own reactive `.pg__count`.
    #[test]
    fn compound_count_has_aria_live() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_005", "Chapter Five")), 5, 13, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        assert!(out.contains("class=\"pg__count\""), "expected a pg__count span, got: {out}");
        assert!(
            out.contains(r#"aria-live="polite" aria-atomic="true">5 / 13"#),
            "pg__count must carry aria-live=\"polite\" aria-atomic=\"true\", got: {out}"
        );
    }

    // ── Layer-1: contents rail ───────────────────────────────────────────

    /// `total >= 2` renders one `.cv-toc__item` per `page.members`, in order,
    /// with the current member marked `aria-current="page"` and the rest not.
    #[test]
    fn toc_renders_all_members_with_current_marked() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_007", "A Mad Tea-Party")),
            7,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert_eq!(
            out.matches("class=\"cv-toc__item\"").count(),
            13,
            "must render exactly one .cv-toc__item per member, got: {out}"
        );
        assert_eq!(
            out.matches("aria-current=\"page\"").count(),
            1,
            "exactly one rail entry must carry aria-current=\"page\", got: {out}"
        );
        // The current member's own name (from `page.members`, ordinal 07) must
        // appear in the rail alongside its zero-padded ordinal.
        assert!(
            out.contains("A Mad Tea-Party") && out.contains("07"),
            "rail must show the current member's name + zero-padded ordinal, got: {out}"
        );
        // A non-current entry's name must also appear (rail lists every member).
        assert!(
            out.contains("Down the Rabbit Hole"),
            "rail must list non-current members too, got: {out}"
        );
    }

    /// The current rail entry makes no `_compound` request (its swap would
    /// push the address already shown); every other entry does. The no-JS
    /// href stays on both.
    #[test]
    fn toc_current_entry_click_makes_no_request() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_007", "A Mad Tea-Party")),
            7,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        let rail = element(&out, r#"<ol class="cv-toc__list""#, "</ol>");
        let current = rail
            .split("<li ")
            .find(|li| li.contains(r#"aria-current="page""#))
            .unwrap_or_else(|| panic!("no current rail entry, got: {rail}"));
        assert!(current.contains(r#"data-on:click__prevent="void 0""#), "got: {current}");
        assert!(current.contains("href=\"/alice-in-daschland/story?page="), "got: {current}");
        assert!(!current.contains("@get("), "got: {current}");
        assert_eq!(rail.matches("@get(").count(), 12, "got: {rail}");
    }

    /// Rail links address each entry's OWN member id, NOT the current
    /// `page.index` — a click on rail entry 3 must navigate to `page=SCH_003`
    /// regardless of which member is currently shown.
    #[test]
    fn toc_link_href_uses_ordinal_not_current_index() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_001", "Down the Rabbit Hole")),
            1,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("href=\"/alice-in-daschland/story?page=SCH_003\""),
            "rail entry 3 must link to ?page=SCH_003 even though the current member is 1, got: {out}"
        );
        assert!(
            rail(&out).contains("data-on:click__prevent=\"@get('/alice-in-daschland/story/_compound?page=SCH_003')\""),
            "rail entry 3's fragment-swap @get must also target page=SCH_003, got: {out}"
        );
    }

    /// The contents-rail caption shows the localized "Contents · N parts"
    /// text with the correct member count, and the nav carries the static
    /// "Contents" aria-label.
    #[test]
    fn toc_caption_and_aria_label() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_001", "Down the Rabbit Hole")),
            1,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("Contents · 13 parts"),
            "rail caption must show 'Contents · 13 parts', got: {out}"
        );
        assert!(
            out.contains("aria-label=\"Contents\""),
            "rail nav must carry the static aria-label 'Contents', got: {out}"
        );
    }

    /// The member eyebrow is the generic `{class} · NN` form
    /// — not an Alice-specific chapter label.
    #[test]
    fn member_eyebrow_is_generic_class_and_ordinal() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource("SCH_007", "A Mad Tea-Party")),
            7,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();

        assert!(
            out.contains("StoryChapter · 07"),
            "eyebrow must be the generic '{{class}} · NN' form, got: {out}"
        );
    }

    // ── Layer-3: insta snapshots ──────────────────────────────────────────────

    /// (a) Member 1 — prev inert, next active.
    #[test]
    fn snapshot_compound_viewer_first_member() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource_with_desc(
                "SCH_001",
                "Down the Rabbit Hole",
                "Alice begins her journey.",
            )),
            1,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("lang=en"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (b) Middle member — both prev and next active.
    #[test]
    fn snapshot_compound_viewer_middle_member() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource_with_desc(
                "SCH_007",
                "A Mad Tea-Party",
                "The famous tea-party scene.",
            )),
            7,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("lang=en"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (c) Last member — next inert, prev active.
    #[test]
    fn snapshot_compound_viewer_last_member() {
        let routes = make_routes();
        let page = make_compound_page(
            Some(make_chapter_resource_with_desc(
                "SCH_013",
                "Alice's Evidence",
                "The final chapter.",
            )),
            13,
            13,
            make_story_headers(),
        );
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", Some("lang=en"));
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (d) Single member (total = 1) — no nav.
    #[test]
    fn snapshot_compound_viewer_single_member() {
        let routes = make_routes();
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "The Only Chapter")), 1, 1, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(Some(&routes), "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (e) Empty compound (total = 0) — empty-state, no nav.
    #[test]
    fn snapshot_compound_viewer_empty_compound() {
        let page = make_compound_page(None, 1, 0, vec![]);
        let lang = make_lang("en");
        let cfg = make_cfg(None, "/alice-in-daschland/story", None);
        let out = render_compound_viewer(&page, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    // ── Member scan (`media=`) ────────────────

    const SCAN_URI: &str = "https://iiif.example.org/0803/p1.jpx/full/max/0/default.jpg";

    /// A Page member carrying `name`, `description` and an `image` link to
    /// `I_P1` — the shape the `media=` reader renders.
    fn make_page_member() -> Resource {
        Resource {
            id: "P_001".to_owned(),
            class: "Page".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "a1r".to_owned(), lang: None }],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![PropertyValue::Text { text: "Title page.".to_owned(), lang: Some("en".to_owned()) }],
                },
                Property {
                    name: "image".to_owned(),
                    values: vec![PropertyValue::Link {
                        target_id: "I_P1".to_owned(),
                        target_label: Some("p1".to_owned()),
                        target_class: Some("Image".to_owned()),
                    }],
                },
            ],
            representations: vec![],
        }
    }

    /// The member's linked Image with one image representation.
    fn make_scan_image(uri: &str, alt: Option<&str>) -> Resource {
        Resource {
            id: "I_P1".to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![cpe_core::ir::Representation {
                kind: cpe_core::ir::RepresentationKind::Image,
                uri: uri.to_owned(),
                width: Some(1200),
                height: Some(1800),
                alt: alt.map(str::to_owned),
                attribution: None,
                seq: 0,
            }],
        }
    }

    fn make_scan_page(total: usize, image: Option<Resource>) -> CompoundPage {
        let members = if total >= 2 {
            vec![
                MemberHeader {
                    id: "P_001".to_owned(),
                    class: "Page".to_owned(),
                    name: "a1r".to_owned(),
                },
                MemberHeader {
                    id: "P_002".to_owned(),
                    class: "Page".to_owned(),
                    name: "a1v".to_owned(),
                },
            ]
        } else {
            vec![]
        };
        CompoundPage {
            member: Some(make_page_member()),
            index: 1,
            total,
            members,
            member_media: image.into_iter().collect(),
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        }
    }

    fn make_scan_cfg<'a>(media_property: Option<&'a str>) -> CompoundViewerCfg<'a> {
        CompoundViewerCfg {
            media_property,
            ..make_cfg(None, "/incunabula/books/B1", None)
        }
    }

    /// Snapshot: `media` set, member has a scan — `figure.cv-member__scan >
    /// img` present (plate-sized src, eager + high priority, explicit dims, representation
    /// alt), and the `image` property row is absent.
    #[test]
    fn snapshot_compound_viewer_member_with_media_scan() {
        let page = make_scan_page(2, Some(make_scan_image(SCAN_URI, Some("Folio a1r"))));
        let cfg = make_scan_cfg(Some("image"));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(
            out.contains(
                "<figure class=\"cv-member__scan\"><img src=\"https://iiif.example.org/0803/p1.jpx/full/900,/0/default.jpg\" \
                 alt=\"Folio a1r\" loading=\"eager\" fetchpriority=\"high\" decoding=\"async\" \
                 width=\"1200\" height=\"1800\"></figure>"
            ),
            "scan figure must render inside the reader, got: {out}"
        );
        assert!(
            !out.contains("I_P1") && !out.contains(">Image<"),
            "the media property must not print as a property row, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// The `total == 1` branch renders the scan too (no rail, no reader div).
    #[test]
    fn member_scan_renders_in_single_member_branch() {
        let page = make_scan_page(1, Some(make_scan_image(SCAN_URI, None)));
        let cfg = make_scan_cfg(Some("image"));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(!out.contains("cv-toc__reader"), "total==1 has no reader column");
        assert!(
            out.contains("<figure class=\"cv-member__scan\">"),
            "total==1 must still render the scan, got: {out}"
        );
    }

    /// No representation `alt` ⇒ the member's resolved name is the alt.
    #[test]
    fn member_scan_alt_falls_back_to_member_name() {
        let page = make_scan_page(2, Some(make_scan_image(SCAN_URI, Some(""))));
        let cfg = make_scan_cfg(Some("image"));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(
            out.contains("alt=\"a1r\""),
            "empty representation alt must fall back to the member name, got: {out}"
        );
    }

    /// A non-`http(s)` representation URI ⇒ nothing rendered (no absent state).
    #[test]
    fn member_scan_unsafe_uri_renders_nothing() {
        let page = make_scan_page(2, Some(make_scan_image("javascript:alert(1)", None)));
        let cfg = make_scan_cfg(Some("image"));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(
            !out.contains("cv-member__scan") && !out.contains("javascript:"),
            "an unsafe URI must render no scan at all, got: {out}"
        );
    }

    /// `member_media` empty (no `media=`, or a member with no image) ⇒ no
    /// figure, even when a media property is suppressed from the rows.
    #[test]
    fn member_scan_absent_when_member_media_empty() {
        let page = make_scan_page(2, None);
        let cfg = make_scan_cfg(Some("image"));
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(
            !out.contains("cv-member__scan") && !out.contains("<img"),
            "no member_media ⇒ no scan figure, got: {out}"
        );
    }

    /// Snapshot: member property rows resolve through declared `labels {}`
    /// — `description` renders its declared DE
    /// label `Seitenbeschreibung`, not the humanized `Description`. No `media`
    /// ⇒ no scan, and the (unsuppressed) `image` row still prints.
    #[test]
    fn snapshot_compound_viewer_member_declared_label_de() {
        let labels = vec![PropertyLabel {
            property: "description".to_owned(),
            translations: vec![
                ("de".to_owned(), "Seitenbeschreibung".to_owned()),
                ("en".to_owned(), "Page description".to_owned()),
            ],
        }];
        let page = make_scan_page(2, None);
        let cfg = CompoundViewerCfg { property_labels: &labels, ..make_scan_cfg(None) };
        let out = render_compound_viewer(&page, &cfg, &make_lang("de")).into_string();
        assert!(
            out.contains("Seitenbeschreibung"),
            "declared DE label must be used for the member row, got: {out}"
        );
        assert!(!out.contains("cv-member__scan"), "no media ⇒ no scan, got: {out}");
        insta::assert_snapshot!(out);
    }

    /// No emitted `href` or `action` attribute
    /// anywhere in the compound-viewer's own markup — the pager, the
    /// contents rail, or the page picker's entries — may carry a
    /// `#compound-viewer` (or any other) fragment. The picker's in-panel jump
    /// marks (`#cv-pk-<n>`) are the one deliberate exception: they scroll the
    /// panel and never navigate. The element `id="compound-viewer"` itself is
    /// unaffected (asserted separately elsewhere).
    #[test]
    fn no_emitted_href_or_action_carries_a_fragment() {
        let contents = render_compound_viewer(
            &make_scan_page(3, None),
            &make_cfg(None, "/incunabula/books/B1", Some("lang=de")),
            &make_lang("en"),
        )
        .into_string();
        let pages = render_picker_book(&["a1r", "a1v", "a2r"], 2, PickerMode::Signatures, "en");
        for out in [&contents, &pages] {
            for attr in ["href=\"", "action=\""] {
                let mut rest = out.as_str();
                while let Some(at) = rest.find(attr) {
                    let after = &rest[at + attr.len()..];
                    let value = after.split('"').next().expect("closing quote");
                    assert!(
                        value.starts_with("#cv-pk-") || !value.contains('#'),
                        "{attr}{value:?} must carry no fragment, got full markup: {out}"
                    );
                    rest = &after[value.len()..];
                }
            }
        }
    }

    // ── Member index modes ─────────────

    /// A synthetic 3-member compound on member 2.
    fn render_index_mode(index: MemberIndex) -> String {
        let page = make_compound_page(
            Some(make_chapter_resource_with_desc("SCH_002", "Chapter 2", "The second chapter.")),
            2,
            3,
            vec![],
        );
        let cfg = CompoundViewerCfg {
            index,
            ..make_cfg(None, "/alice-in-daschland/wonderland", Some("lang=en"))
        };
        render_compound_viewer(&page, &cfg, &make_lang("en")).into_string()
    }

    #[test]
    fn index_rail_is_the_two_column_toc() {
        let out = render_index_mode(MemberIndex::Rail);
        assert!(out.contains(r#"<div class="cv-toc">"#), "got: {out}");
        assert!(!out.contains("cv-index"), "got: {out}");
    }

    #[test]
    fn index_collapsed_has_no_toc_and_a_closed_details_after_the_pager() {
        let out = render_index_mode(MemberIndex::Collapsed);
        assert!(out.starts_with(r#"<article class="compound-viewer page""#), "got: {out}");
        assert!(!out.contains("cv-toc__contents") && !out.contains(r#"class="cv-toc""#));
        assert!(!out.contains("cv-toc__reader"), "got: {out}");
        let pager_at = out.find("<nav class=\"pg\"").expect("pager");
        let details_at = out
            .find(r#"<details class="cv-index"><summary class="cv-index__cap">Contents · 3 parts</summary><ol class="cv-toc__list">"#)
            .unwrap_or_else(|| panic!("closed details.cv-index missing, got: {out}"));
        assert!(pager_at < details_at, "pager → index");
        assert!(!out.contains("<details open"), "the index is closed by default");
        assert_eq!(out.matches("class=\"cv-toc__item\"").count(), 3);
        assert!(out.contains(r#"aria-current="page""#), "current member marked");
    }

    #[test]
    fn index_off_has_no_toc_no_list_and_ends_on_the_pager() {
        let out = render_index_mode(MemberIndex::Off);
        assert!(out.starts_with(r#"<article class="compound-viewer page""#), "got: {out}");
        assert!(!out.contains("cv-toc"), "no toc, no list, got: {out}");
        assert!(!out.contains("cv-index") && !out.contains("<details"));
        assert!(!out.contains("<form"), "got: {out}");
        assert!(out.ends_with("</nav></article>"), "got: {out}");
    }

    // ── `pages` layout ─────────────────────

    /// A synthetic Page Image carrying `authorship` + `license` (no
    /// `copyright`) — the legal run is `"Test Library · CC BY 4.0"`.
    fn make_legal_scan_image() -> Resource {
        let mut img = make_scan_image(SCAN_URI, Some("Folio a1r"));
        for (name, text) in [("authorship", "Test Library"), ("license", "CC BY 4.0")] {
            img.properties.push(Property {
                name: name.to_owned(),
                values: vec![PropertyValue::Text { text: text.to_owned(), lang: None }],
            });
        }
        img
    }

    /// A synthetic 3-member Page compound on member 2, with the legal-bearing
    /// scan, rendered in the `pages` layout with the given index mode.
    fn render_pages_mode(index: MemberIndex) -> String {
        render_pages_scan(index, make_legal_scan_image(), vec![])
    }

    /// [`render_pages_mode`] with a given scan and register entries.
    fn render_pages_scan(
        index: MemberIndex,
        scan: Resource,
        annotations: Vec<cpe_core::ir::AnnotationEntry>,
    ) -> String {
        let members = ["a1r", "a1v", "a2r"]
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("P_{:03}", i + 1),
                class: "Page".to_owned(),
                name: (*name).to_owned(),
            })
            .collect();
        let page = CompoundPage {
            member: Some(make_page_member()),
            index: 2,
            total: 3,
            members,
            member_media: vec![scan],
            stale_page: None,
            annotations,
            annotated_members: std::collections::BTreeSet::new(),
        };
        let class_labels = vec![ClassLabel {
            class: "Page".to_owned(),
            translations: vec![
                ("en".to_owned(), "Page".to_owned()),
                ("de".to_owned(), "Seite".to_owned()),
            ],
            plural: vec![],
        }];
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index,
            media_property: Some("image"),
            class_labels: &class_labels,
            ..make_cfg(None, "/incunabula/books/B1", None)
        };
        render_compound_viewer(&page, &cfg, &make_lang("en")).into_string()
    }

    /// (h) The `contents` layout renders a
    /// member's date through `render_property_block`, so the
    /// `site.date_precision_word` flag reaches it. The `pages` layout shows
    /// no member property rows at all, date included.
    #[test]
    fn contents_layout_renders_member_dates_and_pages_renders_no_rows() {
        let mut member = make_page_member();
        member.properties.push(Property {
            name: "pub-date".to_owned(),
            values: vec![PropertyValue::Date {
                start: "1494-02-11".to_owned(),
                end: Some("1494-02-11".to_owned()),
                precision: cpe_core::ir::DatePrecision::Day,
                display: None,
                display_lang: None,
                calendar: None,
            }],
        });
        let members: Vec<MemberHeader> = ["a1r", "a1v", "a2r"]
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("P_{:03}", i + 1),
                class: "Page".to_owned(),
                name: (*name).to_owned(),
            })
            .collect();
        let page = CompoundPage {
            member: Some(member),
            index: 1,
            total: 3,
            members,
            member_media: vec![make_legal_scan_image()],
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        };
        let render = |layout, flag| {
            let cfg = CompoundViewerCfg {
                layout,
                media_property: Some("image"),
                date_precision_word: flag,
                ..make_cfg(None, "/incunabula/books/B1", None)
            };
            render_compound_viewer(&page, &cfg, &make_lang("de")).into_string()
        };
        let contents = render(CompoundLayout::Contents, true);
        assert_eq!(
            element(&contents, "<span class=\"v-date\">", "</span></span>"),
            "<span class=\"v-date\">1494-02-11 <span class=\"prec\">Tag</span></span>"
        );
        let contents_off = render(CompoundLayout::Contents, false);
        assert!(
            contents_off.contains("<span class=\"v-date\">1494-02-11</span>"),
            "got: {contents_off}"
        );
        assert!(!contents_off.contains("class=\"prec\""), "got: {contents_off}");

        let pages = render(CompoundLayout::Pages, true);
        assert!(!pages.contains("v-date") && !pages.contains("class=\"prop\""), "got: {pages}");
    }

    /// The `pages` article's open tag, WIDER host wiring included.
    const PAGES_ROOT_OPEN: &str = r#"<article class="compound-viewer compound-viewer--pages page" id="compound-viewer" tabindex="-1" aria-labelledby="cv-title" data-class="{'is-wide': $expand}" data-on:popstate__window="if (location.pathname + location.search !== window.__cpeReaderUrl) { location.reload(); }""#;

    /// The ←/→ handler sits on the `pages` article only
    /// (focus-scoped, not `__window`), with every guard, and clicks the
    /// steps' own `rel` links.
    #[test]
    fn pages_root_pages_on_arrow_keys_with_guards() {
        let out = render_pages_mode(MemberIndex::Off);
        let open = &out[..out.find('>').expect("open tag")];
        assert_eq!(open.matches("data-on:keydown").count(), 1, "got: {open}");
        assert!(!out.contains("keydown__window=\"if (evt.key !== 'ArrowLeft'"));
        for guard in [
            "evt.key !== 'ArrowLeft' &amp;&amp; evt.key !== 'ArrowRight'",
            "evt.repeat || evt.defaultPrevented || evt.shiftKey || evt.ctrlKey || evt.altKey || evt.metaKey",
            "evt.target.closest('input,select,textarea,[contenteditable],[role=slider],details,image-viewer,.rv')",
            "el.querySelector(evt.key === 'ArrowLeft' ? 'a[rel=prev]' : 'a[rel=next]')?.click()",
        ] {
            assert!(open.contains(guard), "missing `{guard}` in: {open}");
        }
        let contents = render_compound_viewer(
            &make_scan_page(3, None),
            &make_cfg(None, "/incunabula/books/B1", None),
            &make_lang("en"),
        )
        .into_string();
        assert!(!contents.contains("data-on:keydown="), "contents layout: {contents}");
    }

    /// `annotation::READER_VIEWER_EXPR` finds the reader's viewer by the
    /// article's id: the rendered `pages` root carries it and holds the
    /// viewer.
    #[test]
    fn reader_viewer_expr_selects_the_rendered_reader() {
        let out = render_pages_mode(MemberIndex::Off);
        let expr = annotation::READER_VIEWER_EXPR;
        let id = expr
            .split_once("'#")
            .and_then(|(_, rest)| rest.split_once(" image-viewer'"))
            .map(|(id, _)| id)
            .expect("READER_VIEWER_EXPR selects `#<id> image-viewer`");
        let open = &out[..out.find('>').expect("open tag")];
        assert!(open.contains(&format!(r#" id="{id}""#)), "got: {open}");
        assert!(out.contains("<image-viewer"), "got: {out}");
    }

    #[test]
    fn snapshot_compound_viewer_pages_off() {
        let out = render_pages_mode(MemberIndex::Off);
        assert!(out.starts_with(PAGES_ROOT_OPEN), "got: {out}");
        assert!(!out.contains("cv-toc"), "no rail, no list, got: {out}");
        assert!(out.contains(r#"<p class="eyebrow cv-member__eyebrow">Page · 2 / 3</p>"#));
        assert!(
            out.contains(r#"<h2 class="cv-member__title" id="cv-title">a1r</h2>"#),
            "got: {out}"
        );
        // The head is identity only: no scan head, no member-props track.
        assert!(
            !out.contains("em-head") && !out.contains("cv-member__props") && !out.contains("cv-pages"),
            "got: {out}"
        );
        // The legal run sits in the unit's banner, above the
        // canvas — the record viewer's own markup, no separate legal band.
        let banner_at = out.find(r#"<div class="rv__banner""#).expect("banner");
        let canvas_at = out.find(r#"class="rv__canvas""#).expect("canvas");
        let run_at = out
            .find(r#"<span class="em-legal__run">Test Library<span class="em-legal__dot"> · </span>CC BY 4.0</span>"#)
            .expect("legal run");
        assert!(banner_at < run_at && run_at < canvas_at, "got: {out}");
        assert!(!out.contains("cv-stage__legal") && !out.contains("<figure"));
        // IIIF URI ⇒ the unit mounts `<image-viewer>` with its fallback img
        // inside, the bar's zoom cluster and WIDER.
        assert_eq!(out.matches("<image-viewer").count(), 1, "got: {out}");
        assert!(out.contains(r#"id="rv-I_P1""#), "got: {out}");
        let viewer = element(&out, "<image-viewer", "</image-viewer>");
        assert!(viewer.contains("<img"), "fallback img inside, got: {viewer}");
        assert!(out.contains(r#"<span class="vgroup">"#), "zoom cluster, got: {out}");
        assert!(out.contains(r#"id="rv-I_P1-wider-toggle""#), "WIDER, got: {out}");
        assert!(
            !out.contains("goToPage") && !out.contains("rv__pg"),
            "one image per document: no client-side paging, got: {out}"
        );
        assert!(
            !out.contains(">Image<"),
            "the media property must not print as a property row, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_compound_viewer_pages_rail() {
        let out = render_pages_mode(MemberIndex::Rail);
        let toc_at = out.find(r#"<div class="cv-toc">"#).expect("rail wrapper");
        let reader_at = out.find(r#"<div class="cv-toc__reader">"#).expect("reader");
        let head_at = out.find(r#"<header class="cv-member__head">"#).expect("head");
        assert!(toc_at < reader_at && reader_at < head_at, "got: {out}");
        assert!(!out.contains("<form"), "no jump form, got: {out}");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_compound_viewer_pages_collapsed() {
        let out = render_pages_mode(MemberIndex::Collapsed);
        assert!(!out.contains(r#"class="cv-toc""#), "got: {out}");
        let pager_at = out.find(r#"<nav class="pg pg--reader""#).expect("pager");
        let details_at = out.find(r#"<details class="cv-index">"#).expect("index");
        assert!(pager_at < details_at, "pager → index, got: {out}");
        assert!(out.ends_with("</details></article>"), "got: {out}");
        insta::assert_snapshot!(out);
    }

    /// The stage's debounced zoom listener as Maud writes it.
    fn zoom_away_attr() -> String {
        format!(
            r#"data-on:viewer-zoom__debounce.250ms="{}""#,
            html! { (annotation::zoom_away_expr()) }.into_string()
        )
    }

    /// The reader's stage is `div.cv-stage` wrapping EXACTLY the record
    /// viewer's unit for the member's first Image (no fork): the same bytes
    /// `media::render_viewer_unit` emits for that image, id `rv-{image id}`.
    fn expected_stage(image: &Resource, alt: &str) -> String {
        let unit = render_viewer_unit(
            std::slice::from_ref(image),
            0,
            &format!("rv-{}", image.id),
            alt,
            false,
            None,
            &make_lang("en"),
        )
        .into_string();
        format!(r#"<div class="cv-stage">{unit}</div>"#)
    }

    #[test]
    fn pages_stage_is_the_record_viewer_unit_unforked() {
        let out = render_pages_mode(MemberIndex::Off);
        let stage = expected_stage(&make_legal_scan_image(), "a1r");
        assert!(out.contains(&stage), "stage must equal the unit, got: {out}");
    }

    fn one_rectangle() -> Vec<cpe_core::ir::AnnotationEntry> {
        vec![cpe_core::ir::AnnotationEntry::Annotation {
            number: 1,
            annotation: cpe_core::ir::Annotation {
                id: "a1".to_owned(),
                resource_id: "P_002".to_owned(),
                shape: cpe_core::ir::AnnotationShape::Rectangle,
                x: 0.17,
                y: 0.15,
                w: 0.4,
                h: 0.3,
                points: vec![],
                color: Some("#ff3333".to_owned()),
                remark: None,
                remark_lang: None,
            },
            partners: vec![],
        }]
    }

    fn one_page_level_entry() -> cpe_core::ir::AnnotationEntry {
        cpe_core::ir::AnnotationEntry::Link {
            link: cpe_core::ir::AnnotationLink {
                id: "l1".to_owned(),
                name: "Wiederholung".to_owned(),
                name_lang: None,
                comment: None,
                comment_lang: None,
            },
            partners: vec![],
        }
    }

    /// The eyebrow counts every register entry, numbered and page-level
    /// alike, and links to the register; a page without one keeps the bare
    /// eyebrow.
    #[test]
    fn pages_eyebrow_counts_the_register() {
        let eyebrow = |out: &str| element(out, "<p class=\"eyebrow cv-member__eyebrow\">", "</p>").to_owned();
        let one = render_pages_scan(MemberIndex::Off, make_legal_scan_image(), one_rectangle());
        assert_eq!(
            eyebrow(&one),
            r##"<p class="eyebrow cv-member__eyebrow">Page · 2 / 3 <a class="linklike cv-anno-count" href="#annotations"><span class="anno-dot" aria-hidden="true"></span>1 annotation</a></p>"##
        );
        let mut both = one_rectangle();
        both.push(one_page_level_entry());
        let two = render_pages_scan(MemberIndex::Off, make_legal_scan_image(), both);
        assert!(eyebrow(&two).contains(">2 annotations</a></p>"), "got: {two}");
        let page_level = render_pages_scan(MemberIndex::Off, make_legal_scan_image(), vec![one_page_level_entry()]);
        assert!(eyebrow(&page_level).contains(">1 annotation</a></p>"), "got: {page_level}");
        let none = render_pages_mode(MemberIndex::Off);
        assert_eq!(eyebrow(&none), r#"<p class="eyebrow cv-member__eyebrow">Page · 2 / 3</p>"#);
    }

    /// The `<image-viewer>` of the reader's stage, open tag to close tag.
    fn stage_viewer(out: &str) -> &str {
        element(out, "<image-viewer", "</image-viewer>")
    }

    /// An annotated page's fallback is a fit box holding the plate and the
    /// overlay, sized by the image's own ratio, inside the mounted viewer.
    #[test]
    fn snapshot_pages_stage_draws_the_overlay_in_a_fit_box() {
        let out = render_pages_scan(MemberIndex::Off, make_legal_scan_image(), one_rectangle());
        let viewer = stage_viewer(&out);
        assert!(
            viewer.contains(r#"<div class="anno-fit" style="--r:1200 / 1800"><img "#),
            "got: {viewer}"
        );
        insta::assert_snapshot!(viewer);
    }

    /// Without both dimensions there is no ratio: the stage is the plain
    /// unit, with no fit box and no overlay.
    #[test]
    fn pages_stage_without_dimensions_draws_no_overlay() {
        for (width, height) in [(Some(1200), None), (None, Some(1800)), (Some(0), Some(1800))] {
            let scan = || {
                let mut scan = make_legal_scan_image();
                scan.representations[0].width = width;
                scan.representations[0].height = height;
                scan
            };
            let out = render_pages_scan(MemberIndex::Off, scan(), one_rectangle());
            assert!(
                !out.contains("anno-fit") && !out.contains("data-iv-overlay"),
                "{width:?} × {height:?}, got: {out}"
            );
            // The page still has a register, so a zoom still clears its
            // arrival note.
            let stage = expected_stage(&scan(), "a1r").replacen(
                r#"<div class="cv-stage">"#,
                &format!(r#"<div class="cv-stage" {}>"#, zoom_away_attr()),
                1,
            );
            assert!(out.contains(&stage), "{width:?} × {height:?}, got: {out}");
        }
    }

    /// A page that draws shapes wires them: the stage carries the overlay
    /// effect (naming its own viewer), the two overlay events and the
    /// zoom-away note clear, and the reader's keydown handles Esc in front of
    /// ←/→. Without annotations the stage is plain and the keydown is exactly
    /// [`PAGES_KEYS_EXPR`].
    #[test]
    fn pages_stage_and_keydown_wire_the_annotation_directives() {
        let out = render_pages_scan(MemberIndex::Off, make_legal_scan_image(), one_rectangle());
        let stage_open = format!(
            r#"<div class="cv-stage" data-effect="document.getElementById('rv-I_P1')?.setOverlayState?.({{visible: $_annoShow, focus: ($_annoPin || $_annoHover || $_annoArrive)}})" data-on:viewer-overlay-hover="$_annoHover = evt.detail.id ?? ''" data-on:viewer-overlay-click="{}" {}>"#,
            annotation::overlay_click_expr(),
            zoom_away_attr()
        );
        assert!(out.contains(&stage_open), "got: {out}");
        assert!(
            annotation::overlay_click_expr().contains("evt.target?.zoomToShape?.((evt.detail.id ?? ''))"),
            "{}",
            annotation::overlay_click_expr()
        );
        // Maud's own attribute escaping, not a hand-rolled copy.
        let attr = |expr: String| format!(r#"data-on:keydown="{}""#, html! { (expr) }.into_string());
        let keydown = attr(format!("{} {PAGES_KEYS_EXPR}", annotation::esc_unpin_expr()));
        assert!(out.contains(&keydown), "got: {out}");

        let plain = render_pages_mode(MemberIndex::Off);
        assert!(!plain.contains("_annoShow") && !plain.contains("_annoPin"), "got: {plain}");
        assert!(plain.contains(&attr(PAGES_KEYS_EXPR.to_owned())), "got: {plain}");
    }

    /// The head is followed by the status region then the stage, the pager row is the
    /// stage's next sibling, and the article ends on the pager.
    #[test]
    fn pages_head_stage_pager_in_document_order() {
        let out = render_pages_mode(MemberIndex::Off);
        let stage = expected_stage(&make_legal_scan_image(), "a1r");
        let stage_at = out.find(&stage).expect("stage");
        assert!(
            out[..stage_at]
                .ends_with(r#"</header><div class="cv-status" id="cv-status" role="status" aria-live="polite"></div>"#),
            "the stage follows the head and its (empty) status region, got: {out}"
        );
        let stage_end = stage_at + stage.len();
        assert!(
            out[stage_end..].starts_with(r#"<nav class="pg pg--reader""#),
            "the pager must be the stage's next sibling, got: {}",
            &out[stage_end..]
        );
        let pager_end = stage_end + reader_pager(&out[stage_end..]).len();
        assert_eq!(&out[pager_end..], "</article>");
    }

    /// WIDER's host wiring: with a stage, the article toggles `.is-wide`
    /// from `$expand` in every index mode, and nothing in the reader's own
    /// head or pager row goes `inert` when it does.
    #[test]
    fn pages_with_a_stage_wires_the_wider_host() {
        for index in [MemberIndex::Off, MemberIndex::Rail, MemberIndex::Collapsed] {
            let out = render_pages_mode(index);
            assert!(out.starts_with(PAGES_ROOT_OPEN), "got: {out}");
            let head = element(&out, r#"<header class="cv-member__head""#, "</header>");
            let pager = reader_pager(&out);
            for (name, el) in [("head", head), ("pager", pager)] {
                assert!(
                    !el.contains("data-attr") && !el.contains("inert"),
                    "{name} must not be hidden by WIDER, got: {el}"
                );
            }
            assert!(!out.contains(crate::component::media::HIDDEN_WHEN_WIDE_ATTR), "got: {out}");
        }
    }

    /// No safe Image ⇒ no stage (no absent state); the pager row follows the
    /// head directly.
    #[test]
    fn pages_without_a_safe_image_has_no_stage() {
        let page = CompoundPage {
            member_media: vec![make_scan_image("javascript:alert(1)", None)],
            ..make_scan_page(2, None)
        };
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            ..make_scan_cfg(Some("image"))
        };
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(!out.contains("cv-stage") && !out.contains("<img"));
        assert!(
            out.contains(r#"</header><div class="cv-status" id="cv-status" role="status" aria-live="polite"></div><nav class="pg pg--reader""#),
            "got: {out}"
        );
        // No stage ⇒ no `$expand` signal ⇒ no WIDER host wiring.
        assert!(
            !out.contains("is-wide") && !out.contains("$expand") && !out.contains("rv-unit"),
            "got: {out}"
        );
    }

    /// A non-IIIF URI degrades inside the unit exactly as on a record page:
    /// the `.rv__stand` plain `img`, no `<image-viewer>`, the Details toggle
    /// still reachable, no zoom cluster, no Adjust.
    #[test]
    fn snapshot_compound_viewer_pages_non_iiif() {
        const URI: &str = "https://example.org/scans/p1.jpg";
        let image = make_scan_image(URI, Some("Folio a1r"));
        let page = make_scan_page(2, Some(make_scan_image(URI, Some("Folio a1r"))));
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            ..make_scan_cfg(Some("image"))
        };
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(out.contains(&expected_stage(&image, "a1r")), "got: {out}");
        assert!(!out.contains("<image-viewer"), "got: {out}");
        assert!(out.contains("rv__stand"), "got: {out}");
        assert!(out.contains(r#"id="rv-I_P1-details-toggle""#), "got: {out}");
        assert!(
            !out.contains(r#"class="vgroup""#) && !out.contains("rv__scale") && !out.contains("-adjust-toggle"),
            "no zoom cluster, no Adjust, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// `total == 1` in the `pages` layout: the head and stage, no pager row.
    #[test]
    fn pages_single_member_has_the_stage_and_no_pager() {
        let page = make_scan_page(1, Some(make_scan_image(SCAN_URI, None)));
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            ..make_scan_cfg(Some("image"))
        };
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        assert!(out.contains("compound-viewer--pages") && out.contains("cv-stage"));
        assert!(!out.contains("<nav"), "got: {out}");
        assert!(out.contains("· 1 / 1"), "got: {out}");
    }

    // ── `pages` reader head + pager row ─────

    /// The `pages` pager row of `out`.
    fn reader_pager(out: &str) -> &str {
        element(out, r#"<nav class="pg pg--reader""#, "</nav>")
    }

    /// Render a `pages` reader over members labelled `labels`, on the
    /// 1-based position `index`, in `lang`. The current member carries its
    /// label as `name` (omitted when empty) and, optionally, a description.
    fn render_pages_at(labels: &[&str], index: u32, description: Option<&str>, lang: &str) -> String {
        render_pages_with(labels, index, description, vec![], None, lang)
    }

    /// [`render_pages_at`] with extra member properties and a
    /// `member-links` row.
    fn render_pages_with(
        labels: &[&str],
        index: u32,
        description: Option<&str>,
        extra: Vec<Property>,
        member_links: Option<(&str, &[String])>,
        lang: &str,
    ) -> String {
        let members: Vec<MemberHeader> = labels
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("P_{:03}", i + 1),
                class: "Page".to_owned(),
                name: (*name).to_owned(),
            })
            .collect();
        let pos = usize::try_from(index).expect("fits") - 1;
        let mut properties = Vec::new();
        if !labels[pos].is_empty() {
            properties.push(Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: labels[pos].to_owned(), lang: None }],
            });
        }
        if let Some(d) = description {
            properties.push(Property {
                name: "description".to_owned(),
                values: vec![PropertyValue::Text { text: d.to_owned(), lang: None }],
            });
        }
        properties.extend(extra);
        let member = Resource {
            id: members[pos].id.clone(),
            class: "Page".to_owned(),
            properties,
            representations: vec![],
        };
        let mut routes = HashMap::new();
        routes.insert("BorderStrip".to_owned(), "/strips/:id".to_owned());
        let page = CompoundPage {
            member: Some(member),
            index,
            total: labels.len(),
            members,
            member_media: vec![make_legal_scan_image()],
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        };
        let class_labels = vec![ClassLabel {
            class: "Page".to_owned(),
            translations: vec![
                ("en".to_owned(), "Page".to_owned()),
                ("de".to_owned(), "Seite".to_owned()),
            ],
            plural: vec![],
        }];
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            media_property: Some("image"),
            class_labels: &class_labels,
            links: Some(LinkResolver::new(cpe_core::url::ROOT.project("incunabula"), &routes)),
            member_links,
            ..make_cfg(None, "/incunabula/books/B1", None)
        };
        render_compound_viewer(&page, &cfg, &make_lang(lang)).into_string()
    }

    /// A `pages` reader over members labelled `labels` (ids `P_001`…), on
    /// position `index`, with the page picker in `mode`.
    fn render_picker_book(labels: &[&str], index: u32, mode: PickerMode, lang: &str) -> String {
        render_signposted_book(labels, index, Some(mode), &[], lang)
    }

    /// [`render_picker_book`] with an optional picker and the 1-based
    /// positions of the members that have a register.
    fn render_signposted_book(
        labels: &[&str],
        index: u32,
        mode: Option<PickerMode>,
        annotated: &[usize],
        lang: &str,
    ) -> String {
        let members: Vec<MemberHeader> = labels
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("P_{:03}", i + 1),
                class: "Page".to_owned(),
                name: (*name).to_owned(),
            })
            .collect();
        let pos = usize::try_from(index).expect("fits") - 1;
        let page = CompoundPage {
            member: Some(Resource {
                id: members[pos].id.clone(),
                class: "Page".to_owned(),
                properties: vec![Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: labels[pos].to_owned(), lang: None }],
                }],
                representations: vec![],
            }),
            index,
            total: labels.len(),
            members,
            member_media: vec![make_legal_scan_image()],
            stale_page: None,
            annotations: vec![],
            annotated_members: annotated.iter().map(|n| format!("P_{n:03}")).collect(),
        };
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            media_property: Some("image"),
            picker: mode,
            ..make_cfg(None, "/incunabula/books/B1", Some("lang=de"))
        };
        render_compound_viewer(&page, &cfg, &make_lang(lang)).into_string()
    }

    /// A two-page `pages` reader on page 1 (`Vorsatzblatt, recto`) whose
    /// `?page=` was `stale`.
    fn render_stale(stale: Option<&str>, lang: &str) -> String {
        let members: Vec<MemberHeader> = ["Vorsatzblatt, recto", "a1r"]
            .iter()
            .enumerate()
            .map(|(i, name)| MemberHeader {
                id: format!("P_{:03}", i + 1),
                class: "Page".to_owned(),
                name: (*name).to_owned(),
            })
            .collect();
        let page = CompoundPage {
            member: Some(Resource {
                id: "P_001".to_owned(),
                class: "Page".to_owned(),
                properties: vec![Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "Vorsatzblatt, recto".to_owned(), lang: None }],
                }],
                representations: vec![],
            }),
            index: 1,
            total: 2,
            members,
            member_media: vec![make_legal_scan_image()],
            stale_page: stale.map(str::to_owned),
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        };
        let cfg = CompoundViewerCfg {
            layout: CompoundLayout::Pages,
            index: MemberIndex::Off,
            media_property: Some("image"),
            ..make_cfg(None, "/incunabula/books/B1", None)
        };
        render_compound_viewer(&page, &cfg, &make_lang(lang)).into_string()
    }

    /// The status region always renders between the head and the stage,
    /// empty when the address is fine; the reader is described by it only
    /// when it has something to say.
    #[test]
    fn pages_status_region_is_present_and_empty_when_not_stale() {
        let out = render_stale(None, "de");
        assert!(
            out.contains(r#"</header><div class="cv-status" id="cv-status" role="status" aria-live="polite"></div><div class="cv-stage">"#),
            "got: {out}"
        );
        assert!(!out.contains("aria-describedby"), "got: {out}");
    }

    /// A stale `?page=` says so, names the page shown, and describes the
    /// reader. The value is escaped.
    #[test]
    fn pages_status_names_the_stale_value_and_the_page_shown() {
        let out = render_stale(Some("zzz"), "de");
        assert!(
            out.contains(r#"<div class="cv-status" id="cv-status" role="status" aria-live="polite"><p class="cv-status__msg">Die Adresse nennt keine Seite dieses Buchs: <em>?page=zzz</em></p><p class="cv-status__meta">Gezeigt wird die erste Seite, Vorsatzblatt, recto</p></div>"#),
            "got: {out}"
        );
        assert!(out.contains(r#"aria-labelledby="cv-title" aria-describedby="cv-status""#));
        let en = render_stale(Some("zzz"), "en");
        assert!(en.contains("The address names no page of this book: <em>?page=zzz</em>"));
        assert!(en.contains(">Showing the first page, Vorsatzblatt, recto<"));
        let hostile = render_stale(Some("<script>alert(1)</script>"), "de");
        assert!(!hostile.contains("<script>alert"), "got: {hostile}");
        assert!(hostile.contains("<em>?page=&lt;script&gt;alert(1)&lt;/script&gt;</em>"));
    }

    #[test]
    fn snapshot_compound_viewer_pages_stale_address() {
        insta::assert_snapshot!(render_stale(Some("zzz"), "de"));
    }

    /// The picker takes the pager's centre: a `details#cv-picker` whose
    /// summary is named by its visible text plus the current page and the
    /// extent, a visually hidden live count beside it, and no visible count.
    #[test]
    fn picker_replaces_the_visible_count_with_a_named_summary() {
        let out = render_picker_book(&["a1r", "a1v", "a2r"], 2, PickerMode::Signatures, "de");
        let row = reader_pager(&out);
        assert!(
            row.contains(r#"<span class="pg__count visually-hidden" aria-live="polite" aria-atomic="true">Seite a1v, 2 von 3</span>"#),
            "got: {row}"
        );
        assert!(!row.contains(">2 / 3<"), "got: {row}");
        assert!(
            row.contains(r#"<summary class="pk__summary" aria-label="Seite wählen (aktuell a1v, 3 Seiten)">Seite wählen <span class="pk__caret" aria-hidden="true">▴</span></summary>"#),
            "got: {row}"
        );
        assert_eq!(row.matches(r#"id="cv-picker""#).count(), 1);
    }

    /// Entries are plain links with the steps' own href and no per-entry
    /// `data-on`; the panel carries the one delegated handler; exactly one
    /// entry is current; a signature under its gathering shows `1r` and is
    /// named `a1r`.
    #[test]
    fn picker_entries_are_plain_links_with_one_delegated_handler() {
        let out = render_picker_book(&["a1r", "a1v", "a2r"], 2, PickerMode::Signatures, "de");
        let panel = element(&out, r#"<div class="pk__panel""#, "</details>");
        assert_eq!(panel.matches("data-on:").count(), 1, "got: {panel}");
        assert!(panel.starts_with(r#"<div class="pk__panel" data-on:click=""#));
        assert!(panel.contains("@get('/incunabula/books/B1/_compound' + a.search)"));
        // The current entry only closes the picker: no swap that would push
        // the address already shown. The guard sits after the close and
        // focus return, right before the request.
        assert!(
            panel.contains("if (a.getAttribute('aria-current') === 'page') return; @get("),
            "got: {panel}"
        );
        assert!(
            panel.contains(r#"<a class="pk__entry" href="/incunabula/books/B1?lang=de&amp;page=P_001" aria-label="a1r" title="a1r">1r</a>"#),
            "got: {panel}"
        );
        assert_eq!(panel.matches(r#"aria-current="page""#).count(), 1);
        assert!(panel.contains(r#"page=P_002" aria-current="page" aria-label="a1v""#));
    }

    /// One jump mark per group, each an in-panel `#cv-pk-<n>` anchor to a
    /// focusable `h3` of that id; the current group's mark is current; the
    /// visible mark is hidden from AT in favour of the full name.
    #[test]
    fn picker_marks_address_unique_group_headings() {
        let out = render_picker_book(
            &["Vorsatzblatt, recto", "a1r", "a1v", "b1r", "b1v", "Nachsatzblatt"],
            4,
            PickerMode::Signatures,
            "en",
        );
        let band = element(&out, r#"<div class="pk__band">"#, "</div>");
        assert!(band.starts_with(r#"<div class="pk__band"><span class="pk__index">Gatherings</span>"#));
        for (n, mark, name) in [
            (1, "Front", "Front flyleaves"),
            (2, "a", "Gathering a"),
            (3, "b", "Gathering b"),
            (4, "Back", "Back flyleaves"),
        ] {
            let current = if n == 3 { r#" aria-current="true""# } else { "" };
            assert!(
                band.contains(&format!(r##"<a class="pk__mark" href="#cv-pk-{n}"{current}><span aria-hidden="true">{mark}</span><span class="visually-hidden">{name}</span></a>"##)),
                "mark {n}, got: {band}"
            );
            assert_eq!(out.matches(&format!(r#"id="cv-pk-{n}" tabindex="-1""#)).count(), 1);
        }
        assert!(
            out.contains(r#"<h3 class="pk__head pk__head--key" id="cv-pk-2" tabindex="-1"><span class="pk__headmark" aria-hidden="true">a</span><span class="visually-hidden">Gathering a</span> <span class="pk__meta">1 leaf</span></h3>"#),
            "got: {out}"
        );
        assert!(
            out.contains(r#"<span class="pk__meta">1 page</span>"#),
            "Vorsatz meta, got: {out}"
        );
    }

    /// A numbered book: the band reads "Pages", each run is marked by its
    /// first number, and entries show their label verbatim.
    #[test]
    fn picker_numbered_book_marks_runs_by_first_number() {
        let labels: Vec<String> = (1..=25).map(|n| n.to_string()).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let out = render_picker_book(&labels, 1, PickerMode::Numbered, "de");
        assert!(out.contains(r#"<span class="pk__index">Seiten</span>"#), "got: {out}");
        assert!(out.contains(r#"<span aria-hidden="true">21</span><span class="visually-hidden">Seiten 21–25</span>"#));
        assert!(
            out.contains(r#"<ul class="pk__flat pk__flat--num">"#),
            "number grid, got: {out}"
        );
        assert!(out.contains(r#"page=P_025">25</a>"#), "got: {out}");
    }

    /// Stacked pages in one cell are told apart by position; a dash entry
    /// keeps its verbatim label as its name, and an unlabelled page is named
    /// by position.
    #[test]
    fn picker_names_collisions_dashes_and_unlabelled_pages() {
        let out = render_picker_book(
            &[
                "D5r",
                "D5v (doppelt gescannt???)",
                "D5v (doppelt gescannt???)",
                "158",
                "D6v",
            ],
            1,
            PickerMode::Signatures,
            "de",
        );
        assert!(
            out.contains(
                r#"aria-label="D5v (doppelt gescannt???) (Seite 2)" title="D5v (doppelt gescannt???)">5v</a>"#
            ),
            "got: {out}"
        );
        assert!(
            out.contains(
                r#"aria-label="D5v (doppelt gescannt???) (Seite 3)" title="D5v (doppelt gescannt???)">5v²</a>"#
            ),
            "the repeat carries a visible ordinal, got: {out}"
        );
        assert!(
            out.contains(r#"<a class="pk__entry pk__entry--dash" href="/incunabula/books/B1?lang=de&amp;page=P_004" aria-label="158" title="158">—</a>"#),
            "got: {out}"
        );
        let numbered = render_picker_book(&["1", "", "3"], 1, PickerMode::Numbered, "de");
        assert!(
            numbered.contains(r#"aria-label="Seite 2, ohne Bezeichnung">—</a>"#),
            "got: {numbered}"
        );
    }

    /// No picker at one page (no pager row at all) or without `picker=`.
    #[test]
    fn picker_absent_at_one_page_and_without_the_attribute() {
        let one = render_picker_book(&["a1r"], 1, PickerMode::Signatures, "de");
        assert!(!one.contains("cv-picker") && !one.contains("pg--reader"), "got: {one}");
        let off = render_pages_at(&["a1r", "a1v"], 1, None, "de");
        assert!(!off.contains("cv-picker"), "got: {off}");
        assert!(reader_pager(&off)
            .contains(r#"<span class="pg__count" aria-live="polite" aria-atomic="true">1 / 2</span>"#));
    }

    #[test]
    fn snapshot_compound_viewer_pages_picker_signatures() {
        let out = render_picker_book(
            &["Vorsatzblatt, recto", "a1r", "a1v", "158", "a2v", "b1r", "b1r", "b1v"],
            2,
            PickerMode::Signatures,
            "de",
        );
        insta::assert_snapshot!(out);
    }

    /// The picker's panel with marks: the legend, accent entries with a dot
    /// and "– mit Annotation" (also on the current page and on a dash), and
    /// a dot on the band mark and heading of each group holding one; group b
    /// and the flyleaves hold none. The snapshot starts at the band, so the
    /// panel's click handler is not part of it.
    #[test]
    fn snapshot_picker_marks_annotated_pages() {
        let labels = [
            "Vorsatzblatt, recto",
            "a1r",
            "a1v",
            "158",
            "a2v",
            "b1r",
            "b1r",
            "b1v",
            "c1r",
        ];
        let out = render_signposted_book(&labels, 2, Some(PickerMode::Signatures), &[2, 4, 9], "de");
        let panel = element(&out, r#"<div class="pk__band">"#, "</details>");
        assert!(
            panel.contains(
                r#"<span class="pk__legend"><span class="anno-dot" aria-hidden="true"></span>mit Annotation</span>"#
            ),
            "got: {panel}"
        );
        assert!(
            panel.contains(
                r#"aria-current="page" aria-label="a1r – mit Annotation" title="a1r">1r<span class="anno-dot""#
            ),
            "got: {panel}"
        );
        assert_eq!(
            panel
                .matches(r#"<span class="visually-hidden">Lage a – enthält annotierte Seiten</span>"#)
                .count(),
            2,
            "band + heading, got: {panel}"
        );
        assert!(panel.contains(r#"<span class="visually-hidden">Lage b</span>"#), "got: {panel}");
        insta::assert_snapshot!(panel);
    }

    /// A book with no annotated page has no legend and no marks.
    #[test]
    fn picker_without_annotated_pages_has_no_marks() {
        let out = render_signposted_book(&["a1r", "a1v"], 1, Some(PickerMode::Signatures), &[], "de");
        assert!(
            !out.contains("pk__legend") && !out.contains("anno-dot") && !out.contains("mit Annotation"),
            "got: {out}"
        );
    }

    #[test]
    fn snapshot_compound_viewer_pages_picker_numbered() {
        let out = render_picker_book(&["1 recto", "1 verso", "2 recto"], 3, PickerMode::Signatures, "en");
        insta::assert_snapshot!(out);
    }

    /// An annotated neighbour's step carries the dot beside its label (kept
    /// when the label is too long to show) and ", mit Annotation" at the end
    /// of its name.
    #[test]
    fn snapshot_reader_pager_marks_annotated_neighbours() {
        let out = render_signposted_book(&["Vorsatzblatt, recto", "a1r", "a1v"], 2, None, &[1, 3], "de");
        let pager = reader_pager(&out);
        assert_eq!(
            pager.matches(r#"<span class="anno-dot" aria-hidden="true"></span>"#).count(),
            2,
            "got: {pager}"
        );
        assert_eq!(
            pager
                .matches(r#"<span class="visually-hidden">, mit Annotation</span>"#)
                .count(),
            2,
            "got: {pager}"
        );
        insta::assert_snapshot!(pager);
    }

    /// Only a neighbour's register marks a step: the current page's own does
    /// not, and an inert end has none to mark.
    #[test]
    fn snapshot_reader_pager_without_annotated_neighbours() {
        let out = render_signposted_book(&["a1r", "a1v"], 1, None, &[1], "de");
        let pager = reader_pager(&out);
        assert!(!pager.contains("anno-dot") && !pager.contains("mit Annotation"), "got: {pager}");
        insta::assert_snapshot!(pager);
    }

    fn strip_link(prop: &str, id: &str, label: &str) -> Property {
        Property {
            name: prop.to_owned(),
            values: vec![PropertyValue::Link {
                target_id: id.to_owned(),
                target_label: Some(label.to_owned()),
                target_class: Some("BorderStrip".to_owned()),
            }],
        }
    }

    /// The `member-links` row renders under the pager, inside the
    /// reader article, listing each link value in property order as an
    /// `int-link` to the target's `/strips/:id` page.
    #[test]
    fn pages_member_links_render_below_the_pager() {
        let props = ["strip-left".to_owned(), "strip-right".to_owned()];
        let out = render_pages_with(
            &["a1r", "a1v"],
            1,
            None,
            vec![
                strip_link("strip-right", "r-07", "r_07"),
                strip_link("strip-left", "l-03", "l_03"),
            ],
            Some(("Randleisten", &props)),
            "de",
        );
        let pager_end = out.find("</nav>").expect("pager renders");
        let row = out.find(r#"<p class="cv-links">"#).expect("the row renders");
        assert!(row > pager_end, "the row follows the pager: {out}");
        assert!(
            out.contains(r#"<p class="cv-links"><span class="cv-links__label">Randleisten</span> <a class="int-link" href="/incunabula/strips/l-03"><span class="ref">l_03</span></a><span aria-hidden="true"> · </span><a class="int-link" href="/incunabula/strips/r-07"><span class="ref">r_07</span></a></p>"#),
            "got: {out}"
        );
        assert!(out.ends_with("</p></article>"), "inside #compound-viewer: {out}");
    }

    /// No link value on the page ⇒ no row; no `member-links` ⇒ no row.
    #[test]
    fn pages_member_links_absent_without_links() {
        let props = ["strip-left".to_owned()];
        let out = render_pages_with(&["a1r", "a1v"], 1, None, vec![], Some(("R", &props)), "de");
        assert!(!out.contains("cv-links"), "got: {out}");
        let out = render_pages_with(
            &["a1r", "a1v"],
            1,
            None,
            vec![strip_link("strip-left", "l-03", "l_03")],
            None,
            "de",
        );
        assert!(!out.contains("cv-links"), "got: {out}");
    }

    /// Label ≠ position: the eyebrow carries `n / N`, the
    /// h2 the label — the position shows once.
    #[test]
    fn pages_head_label_differs_from_position() {
        let out = render_pages_at(&["Vorsatzblatt, recto", "a1r", "a1v"], 2, None, "de");
        let head = element(&out, r#"<header class="cv-member__head""#, "</header>");
        assert_eq!(
            head,
            r#"<header class="cv-member__head"><p class="eyebrow cv-member__eyebrow">Seite · 2 / 3</p><h2 class="cv-member__title" id="cv-title">a1r</h2><div class="cv-member__aside"></div></header>"#
        );
    }

    /// Label = position: the eyebrow is the class alone and
    /// the h2 carries `n` plus a mono `/ N`.
    #[test]
    fn pages_head_label_equals_position() {
        let labels: Vec<String> = (1..=194).map(|n| n.to_string()).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let out = render_pages_at(&labels, 5, None, "de");
        let head = element(&out, r#"<header class="cv-member__head""#, "</header>");
        assert_eq!(
            head,
            r#"<header class="cv-member__head"><p class="eyebrow cv-member__eyebrow">Seite</p><h2 class="cv-member__title" id="cv-title">5<span class="cv-member__of"> / 194</span></h2><div class="cv-member__aside"></div></header>"#
        );
    }

    /// No label: the eyebrow says so and the h2 is the
    /// position.
    #[test]
    fn pages_head_without_label_shows_the_position() {
        let out = render_pages_at(&["a1r", "", "a2r"], 2, None, "de");
        let head = element(&out, r#"<header class="cv-member__head""#, "</header>");
        assert_eq!(
            head,
            r#"<header class="cv-member__head"><p class="eyebrow cv-member__eyebrow">Seite · ohne Bezeichnung</p><h2 class="cv-member__title" id="cv-title">2</h2><div class="cv-member__aside"></div></header>"#
        );
        let en = render_pages_at(&["a1r", "", "a2r"], 2, None, "en");
        assert!(en.contains(">Page · no label</p>"), "got: {en}");
    }

    /// A `[missing]` label is a label: rendered verbatim as the h2.
    #[test]
    fn pages_head_missing_placeholder_is_a_label() {
        let out = render_pages_at(&["[missing]", "a1v"], 1, None, "de");
        assert!(
            out.contains(r#"<p class="eyebrow cv-member__eyebrow">Seite · 1 / 2</p><h2 class="cv-member__title" id="cv-title">[missing]</h2>"#),
            "got: {out}"
        );
    }

    /// The description follows the h2 as a one-line italic aside whose dash
    /// is hidden from assistive technology, with the native "more"
    /// disclosure beside it. A short text is not `--long`,
    /// so without scroll-driven animations the expander stays hidden.
    #[test]
    fn pages_head_description_is_an_aside() {
        let out = render_pages_at(&["a1r", "a1v"], 1, Some("leer"), "de");
        assert!(
            out.contains(r#"</h2><div class="cv-member__aside"><span class="cv-aside__text"><span aria-hidden="true">— </span>leer</span><details class="cv-aside__more"><summary><span class="cv-aside__open">mehr</span><span class="cv-aside__close">weniger</span></summary></details></div></header>"#),
            "got: {out}"
        );
        let en = render_pages_at(&["a1r", "a1v"], 1, Some("empty"), "en");
        assert!(
            en.contains(r#"<span class="cv-aside__open">more</span><span class="cv-aside__close">less</span>"#),
            "got: {en}"
        );
    }

    /// A page with no description still renders the aside, empty: its one
    /// line is reserved so the stage and pager never move while paging.
    #[test]
    fn pages_head_reserves_the_aside_line_without_description() {
        let out = render_pages_at(&["a1r", "a1v"], 1, None, "de");
        assert!(
            out.contains(r#"</h2><div class="cv-member__aside"></div></header>"#),
            "got: {out}"
        );
        assert!(!out.contains("cv-aside__more"), "got: {out}");
    }

    /// Past `ASIDE_FALLBACK_BUDGET` characters the aside carries `--long`,
    /// the fallback's signal to show the expander where CSS cannot measure
    /// the overflow; at the budget it does not.
    #[test]
    fn pages_head_long_description_is_flagged_for_the_fallback() {
        let at_budget = "x".repeat(ASIDE_FALLBACK_BUDGET);
        let out = render_pages_at(&["a1r", "a1v"], 1, Some(&at_budget), "de");
        assert!(out.contains(r#"<div class="cv-member__aside">"#), "got: {out}");
        let over = "x".repeat(ASIDE_FALLBACK_BUDGET + 1);
        let out = render_pages_at(&["a1r", "a1v"], 1, Some(&over), "de");
        assert!(
            out.contains(r#"<div class="cv-member__aside cv-member__aside--long">"#),
            "got: {out}"
        );
    }

    /// The article is named by its h2: `aria-labelledby` resolves to the one
    /// element carrying that id.
    #[test]
    fn pages_root_is_labelled_by_the_h2() {
        let out = render_pages_at(&["a1r", "a1v", "a2r"], 2, None, "en");
        assert!(out.contains(r#"aria-labelledby="cv-title""#), "got: {out}");
        assert_eq!(out.matches(r#"id="cv-title""#).count(), 1, "got: {out}");
        assert!(out.contains(r#"<h2 class="cv-member__title" id="cv-title">"#));
    }

    /// The pager row: words and short target names are the visible (and so
    /// the accessible) names — no `aria-label` on a step, arrows and the `·`
    /// separator `aria-hidden`, `rel` set, the full target in `title`.
    #[test]
    fn pages_pager_steps_carry_words_names_and_rel() {
        let out = render_pages_at(&["a1r", "a1v", "a2r"], 2, None, "de");
        let pager = reader_pager(&out);
        assert!(
            pager.starts_with(r#"<nav class="pg pg--reader" aria-label="Blättern">"#),
            "got: {pager}"
        );
        assert!(
            pager.contains(r#"rel="prev" title="a1r" data-on:click__prevent="#)
                && pager.contains(r#"<span aria-hidden="true">←</span> <span class="pg__word">Zurück</span> <span class="pg__name"><span aria-hidden="true">· </span>a1r</span></a>"#),
            "got: {pager}"
        );
        assert!(
            pager.contains(r#"rel="next" title="a2r" data-on:click__prevent="#)
                && pager.contains(r#"<span class="pg__name">a2r<span aria-hidden="true"> ·</span></span> <span class="pg__word">Weiter</span> <span aria-hidden="true">→</span></a>"#),
            "got: {pager}"
        );
        assert!(
            !pager
                .trim_start_matches(r#"<nav class="pg pg--reader" aria-label="Blättern">"#)
                .contains("aria-label"),
            "no step carries an aria-label, got: {pager}"
        );
        assert!(
            pager.contains(r#"<span class="pg__count" aria-live="polite" aria-atomic="true">2 / 3</span>"#),
            "got: {pager}"
        );
    }

    /// A target longer than the name threshold shows the word alone; the
    /// full target stays in `title`.
    #[test]
    fn pages_pager_long_target_name_lives_in_title_only() {
        let out = render_pages_at(&["Vorsatzblatt, recto", "Vorsatzblatt, verso", "a1r"], 2, None, "de");
        let pager = reader_pager(&out);
        assert!(pager.contains(r#"rel="prev" title="Vorsatzblatt, recto""#), "got: {pager}");
        let prev = element(pager, r#"<a class="pg__step""#, "</a>");
        assert!(!prev.contains("pg__name"), "got: {prev}");
        assert!(pager.contains(r#"<span class="pg__name">a1r<span"#), "got: {pager}");
    }

    /// The inert end keeps its word, is `aria-disabled`, and is NOT
    /// `aria-hidden`: a screen reader reads it as unavailable.
    #[test]
    fn pages_pager_inert_end_keeps_its_word() {
        let first = render_pages_at(&["a1r", "a1v"], 1, None, "de");
        assert!(
            reader_pager(&first).contains(r#"<span class="pg__step" aria-disabled="true"><span aria-hidden="true">←</span> <span class="pg__word">Zurück</span></span>"#),
            "got: {first}"
        );
        let last = render_pages_at(&["a1r", "a1v"], 2, None, "de");
        let pager = reader_pager(&last);
        assert!(
            pager.contains(r#"<span class="pg__step" aria-disabled="true"><span class="pg__word">Weiter</span> <span aria-hidden="true">→</span></span>"#),
            "got: {pager}"
        );
        assert!(!pager.contains(r#"aria-disabled="true" aria-hidden"#), "got: {pager}");
    }

    /// The `contents` layout keeps the plain `.pg` pager: no `pg--reader`,
    /// arrow-only steps named by `aria-label`, an `aria-hidden` inert end.
    #[test]
    fn contents_pager_is_unchanged() {
        let page = make_compound_page(Some(make_chapter_resource("SCH_001", "Chapter 1")), 1, 3, vec![]);
        let cfg = make_cfg(None, "/alice-in-daschland/wonderland", None);
        let out = render_compound_viewer(&page, &cfg, &make_lang("en")).into_string();
        let pager = pager(&out);
        assert!(!out.contains("pg--reader") && !out.contains("cv-title"), "got: {out}");
        assert!(
            pager.contains(r#"<span class="pg__step" aria-disabled="true" aria-hidden="true">←</span>"#)
                && pager.contains(r#"aria-label="Next">→</a>"#),
            "got: {pager}"
        );
    }
}
