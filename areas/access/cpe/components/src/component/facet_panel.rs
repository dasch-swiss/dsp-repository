//! Renderer for the `facet-panel` component.
//!
//! Takes a `&[FacetGroup]` (built by `ProjectReads::facet_options` in the store's
//! two-pass page builder, `query::page`) and renders a `<section class="facet-panel page">`
//! with one `<section class="facet-group">` per group and toggle links per option.
//!
//! ## Toggle links
//!
//! Each option renders as `<a class="facet-option__toggle" href="…"
//! data-on:click__prevent="@get('…')">`. The `href` is the no-JS fallback: a full-page reload with
//! the toggled query string. The `@get` URL is the DataStar fragment URL (`{path}/_facets?{qs}`),
//! built with `shared_datastar::get`, which keeps it inside the `@get('…')` JS string.
//!
//! ## URL encoding
//!
//! `toggle_query_param` does NOT percent-encode keys/values — Alice's facet keys
//! (`role`, `keyword`, `location-type`) and list-node-id values (`R_01`, `K_005`,
//! `lt-wonderland`) are ASCII-safe, consistent with the existing `page`/`lang`/`img`
//! params in `override_query_param`. Percent-encoding for non-ASCII real-project node
//! ids and for query-structural chars (`&`, `=`, `+`) is not done.
//! Note this distinction: Maud's HTML-attribute
//! escaping handles the `href` value; only the `@get('…')` JS-string context needs
//! the path encoding `shared_datastar::get` applies.
//!
//! ## Empty branches
//!
//! Both empty branches carry the `.page` root (LAYOUT CONTRACT):
//! - (a) `groups` is empty (no facets configured).
//! - (b) All groups have zero options.
//!
//! Both emit a localized "no filters available" note.
//!
//! ## Per-group disclosure
//!
//! A group with more than `FACET_GROUP_COLLAPSE_THRESHOLD` options shows its
//! `FACET_GROUP_VISIBLE_COUNT` densest options plus every selected option
//! hoisted out of the tail, and folds the unselected remainder into a per-group
//! `<details class="facet-group__more">`. See `split_group_options` for the
//! rule and the two constants' doc comments for why 16 and 12 differ. Groups at
//! or below the threshold render byte-identically to how they always have.
//!
//! ## `aria-current`
//!
//! Selected options carry `aria-current="true"` (the `<a>` link role),
//! not `aria-pressed` (invalid on a link element). Same idiom as the
//! language switcher.
//!
//! ## Per-group clear + active-filter chips
//!
//! Each group heading sits in a `div.facet-group__titlerow` alongside an
//! `a.facet-group__clear` "clear" link, shown only when that group has ≥1
//! selected option (`:hover`/`:focus-within`-revealed, record.css). It clears
//! only that group's params via `clear_group_query` — other groups'
//! selections and unrelated params (lang, appearance) are preserved.
//!
//! [`render_active_filters`] renders a **separate** `div.filterbar` — a second
//! view of the same `groups` data, composed by `render_body` alongside
//! `render_facet_panel`'s own output. It is
//! NOT called from inside `render_facet_panel`.
//!
//! ## Query chip
//!
//! [`render_active_filters`] also takes an optional `query: Option<&str>` —
//! the sibling search-bar's ACTIVE query (never a raw-non-empty string; the
//! caller gates on the same `SearchSurface`/tokenizer test as the list
//! narrowing itself). When `Some`, a `.filterchip--query` chip renders
//! **before** the facet chips, and the filterbar
//! is non-empty even when no facet is selected. Unlike the facet chips (plain
//! `<a href>`), the query chip's no-JS fallback is a real `<form method=get>`
//! wrapping a `type="submit" name="q" value=""` button,
//! since a link alone cannot post an empty `q` — `.filterchip-form { display:
//! contents; }` (record.css) makes the wrapper invisible to the `.filterbar`
//! flex layout. Clearing it hits the *same* `_search` fragment endpoint (2-event
//! swap: filterbar + list) the search row's own `×` uses — facet counts stay
//! static, so the panel is never re-rendered by this action.

use cpe_core::ir::{any_facet_selected, FacetGroup, FacetOption};
use cpe_core::lang::Lang;
use maud::{html, Markup};
use shared_datastar as datastar;

use super::search_bar;
use crate::chrome::{facet_empty_group_text, facet_group_remainder_text, remove_filter_label, ChromeMsg};

// Per-group disclosure split

/// A facet group with **more than** this many options collapses: only
/// [`FACET_GROUP_VISIBLE_COUNT`] options stay in the flat list, the rest go
/// behind a per-group `<details>`. A group with exactly this many (or fewer)
/// renders byte-identically to how it always has.
///
/// ⚠️ **16 and [`FACET_GROUP_VISIBLE_COUNT`]'s 12 are two DELIBERATELY DIFFERENT
/// numbers — do not "simplify" them into one.** The HdM
/// 94-node season group shows its 12 densest options behind a
/// disclosure, *and* the 13-node `Jahrzehnt` (decade) group shows flat. A single
/// number of 12 would collapse the decade group; a
/// single number of 16 would show 16 seasons instead of 12. The
/// gap between the two is the point: a group only slightly over the visible
/// count is not worth a disclosure.
pub(crate) const FACET_GROUP_COLLAPSE_THRESHOLD: usize = 16;

/// How many options a collapsed group shows above its `<details>` — the 12
/// **densest**, since `query::list_facet_values`' `ORDER BY cnt DESC, label,
/// target_id` already hands the renderer its options in descending-count order
/// (no re-sort here; the prefix *is* the densest 12). Every **selected** option
/// outside that prefix is additionally hoisted into the visible set, so a
/// visitor never has to open a disclosure to see what they have already
/// filtered by — which means the visible count can legitimately exceed 12.
///
/// ⚠️ See [`FACET_GROUP_COLLAPSE_THRESHOLD`]: 12 and 16 are two deliberately
/// different numbers, not a redundancy to collapse.
pub(crate) const FACET_GROUP_VISIBLE_COUNT: usize = 12;

/// Split a group's options into (visible, hidden).
///
/// At or below [`FACET_GROUP_COLLAPSE_THRESHOLD`] the group is left completely
/// alone — `(all, &[])` — which is what guarantees byte-identical markup for
/// every group that renders flat today. Above it, the visible set is the
/// densest [`FACET_GROUP_VISIBLE_COUNT`] prefix **plus every selected option
/// after it**, and the hidden set is the remaining unselected tail. Hoisting can
/// legitimately empty the hidden set (e.g. a 17-option group with all 5 tail
/// options selected → 17 visible, 0 hidden); the caller must then render **no**
/// `<details>` at all rather than a "Show 0 more" summary.
///
/// The split is computed here, locally, from `options.len()` and
/// `FacetOption::selected` — both already at hand at the render site. No
/// `ir::FacetGroup`/`ir::FacetOption` field carries it (deliberately: an IR
/// field would ripple through every literal construction in the crate for a
/// decision the renderer can make on its own).
fn split_group_options(options: &[FacetOption]) -> (Vec<&FacetOption>, Vec<&FacetOption>) {
    if options.len() <= FACET_GROUP_COLLAPSE_THRESHOLD {
        return (options.iter().collect(), Vec::new());
    }
    let (head, tail) = options.split_at(FACET_GROUP_VISIBLE_COUNT);
    let mut visible: Vec<&FacetOption> = head.iter().collect();
    let mut hidden: Vec<&FacetOption> = Vec::new();
    for opt in tail {
        if opt.selected {
            visible.push(opt);
        } else {
            hidden.push(opt);
        }
    }
    (visible, hidden)
}

/// The URL path segment that identifies the facet-panel fragment endpoint.
///
/// The fragment endpoint lives at `{page-path}/{FACET_FRAGMENT_SEGMENT}?…`.
/// Both the engine (to build `data-on:click__prevent` `@get` URLs) and `dsp-incubator cpe/bin`
/// (to register the Axum route) reference this constant so no magic-string
/// mismatch is possible. Mirrors `SORT_FRAGMENT_SEGMENT` and
/// `SEARCH_FRAGMENT_SEGMENT`.
pub const FACET_FRAGMENT_SEGMENT: &str = "_facets";

// Config borrow struct

/// Borrowed context needed by `render_facet_panel`.
///
/// All fields are `pub` (unlike `ResourceListCfg`/`CompoundViewerCfg`): the
/// facet panel carries no `LinkResolver` (facet toggles are same-page
/// query-param links, not detail-page links), so all fields are public types
/// (`&str` / `Option<&str>`). Both the engine's `render_component` arm AND the
/// `dsp-incubator cpe/bin`'s `_facets` fragment handler can construct this struct directly, without
/// a factory method.
pub struct FacetPanelCfg<'a> {
    /// The `id` of the target `resource-list` component. Used to build
    /// `id="facets-{list_id}"` on the panel root for DataStar swap targeting.
    pub list_id: &'a str,
    /// URI path of the current request (e.g. `/alice-in-daschland/characters`).
    /// Used to build toggle hrefs.
    pub current_path: &'a str,
    /// Raw query string of the current request (everything after `?`, no `?`).
    /// `None` when the request had no query string.
    pub current_query: Option<&'a str>,
}

// Public renderer

/// Render a facet-panel as a `<section class="facet-panel page">`.
///
/// Structure:
/// - Empty branch (a): `groups` is empty → renders **nothing** (`html!{}`) — a rail with no groups
///   to offer is furniture with nothing to do. This is the raw render function's own fallback for a
///   defensive/direct-dispatch caller; the normal production path (`render_body`) doesn't call this
///   function at all in that case — it switches the whole page layout to the non-faceted shape
///   instead (see `render_body`), dropping the `.browse` grid's rail column entirely rather than
///   emitting an empty section for it.
/// - Empty branch (b): groups present but a given group has zero options → that group still renders
///   `<section.facet-group>` with its heading + a localized "{group} has no options yet" note
///   (`facet_empty_group_text`) — slot scale.
/// - Populated branch: groups rendered as flex-row wrappers with toggle-link options inside.
///
/// The populated and per-group-empty branches emit
/// `id="facets-{cfg.list_id}"` and `aria-label` (localized "Filters") on the
/// root for DataStar swap targeting and accessibility.
///
/// The root does **not** carry `aria-live` — the `_facets` DataStar fragment
/// endpoint morphs this whole section (every group/option) in place, so an
/// atomic live region here would make a screen reader re-announce the full
/// panel on every toggle (WCAG 4.1.3-territory verbosity; a11y review
/// finding). The concise, already-scoped live region for this swap lives on
/// the separate `div.filterbar` chip row (see [`render_active_filters`]),
/// per `dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md` —
/// no focus/URL change accompanies the swap either way.
pub fn render_facet_panel(groups: &[FacetGroup], cfg: &FacetPanelCfg<'_>, lang: &Lang) -> Markup {
    let panel_label = ChromeMsg::FacetFiltersWord.text(lang);
    let panel_id = format!("facets-{}", cfg.list_id);

    // Empty branch (a): no groups at all — render nothing
    if groups.is_empty() {
        return html! {};
    }

    html! {
        // `.page` must appear on EVERY branch root (LAYOUT CONTRACT, grid.css).
        section.facet-panel.page id=(panel_id) aria-label=(panel_label) {
            // Native `<details>` disclosure — zero-JS, keyboard-accessible for
            // free, no new interaction code. Rendered ALWAYS `open`: above the
            // 860px browse breakpoint the `<summary>` is hidden by CSS and this
            // reads exactly as the flat panel it has always been. Below 860px
            // the browse grid collapses to document order, which put the whole
            // facet vocabulary between the visitor and the results — there the
            // summary becomes a tappable bar and the panel starts open but can
            // be folded away.
            //
            // Because the server always emits `open`, any page load at desktop
            // width is open. The one residual case is collapsing at narrow
            // width and then widening WITHOUT a reload; `::details-content` is
            // overridden in record.css to cover that on engines that support
            // it, and a reload resolves it everywhere else.
            // `open` iff something is actually selected. This is what makes the
            // disclosure behave under the `_facets` DataStar swap, which
            // replaces this whole `<section>`: a flat `open` would never
            // auto-collapse, while a flat closed would SNAP SHUT the moment a
            // visitor tapped a facet (the re-render would drop the open state
            // mid-interaction). Keying it to selection means a fresh narrow
            // load starts collapsed, stays open while filtering, and folds
            // again on Clear all. Above 861px CSS forces it open regardless.
            @let has_selection = any_facet_selected(groups);
            details.facet-panel__disclosure open[has_selection] {
                summary.facet-panel__toggle {
                    span.facet-panel__toggle-label { (panel_label) }
                }
                @for group in groups {
                    section.facet-group {
                        @let group_has_selection = group.options.iter().any(|o| o.selected);
                        div.facet-group__titlerow {
                            h2.facet-group__title { (group.label) }
                            // Per-group clear — only when THIS group has ≥1 selected
                            // option. Clears only this group's params (+ page), leaving
                            // other groups' selections and unrelated params (lang,
                            // appearance) intact. Same no-JS href + `@get` enhancement
                            // idiom as the option toggles below — revealed on
                            // `:hover`/`:focus-within` (record.css).
                            @if group_has_selection {
                                @let g_href = group_clear_href(cfg, &group.property);
                                @let g_frag = group_clear_fragment_href(cfg, &group.property);
                                a   .facet-group__clear
                                    href=(g_href)
                                    data-on:click__prevent=(datastar::get(g_frag))
                                { (ChromeMsg::FacetGroupClearLabel.text(lang)) }
                            }
                        }
                        @if group.options.is_empty() {
                            // Empty branch (b): group present but no
                            // options. Render heading + the group-specific
                            // note (`facet_empty_group_text`), NOT a bare empty <ul>.
                            p.facet-panel__empty {
                                (facet_empty_group_text(&group.label.to_lowercase(), lang))
                            }
                        } @else {
                            // Per-group disclosure. `visible` is the whole
                            // group at or below FACET_GROUP_COLLAPSE_THRESHOLD
                            // (so those groups keep byte-identical markup);
                            // above it, the 12 densest plus every selected
                            // option hoisted out of the tail.
                            @let (visible, hidden) = split_group_options(&group.options);
                            ul.facet-options {
                                @for opt in &visible {
                                    (render_facet_option(cfg, &group.property, opt))
                                }
                            }
                            // Remainder 0 → NO <details> at all (hoisting every
                            // selected option can empty the tail); "Show 0 more"
                            // is never a thing.
                            @if !hidden.is_empty() {
                                // `open` iff THIS group has a selection — derived,
                                // never a hardcoded `false`. `_facets` morphs the
                                // whole `#facets-{list_id}` section (this function's
                                // entire output), so a flat `false` would snap the
                                // group shut the moment the visitor tapped any facet
                                // anywhere in the panel — re-opening and re-scrolling
                                // the 82-season remainder on every multi-select. That
                                // is the exact bug the panel-level disclosure above
                                // already documents and already solves with
                                // `open[has_selection]`; this is the same idiom scoped
                                // to one group. Still fully derived, still no JS state.
                                //
                                // Accepted residual: a visitor who opens a remainder
                                // WITHOUT selecting anything in that group loses the
                                // open state on the next `_facets` morph. Closing that
                                // would mean carrying the open state as a query param —
                                // URL-bearing state for a transient refinement, against
                                // `dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`.
                                // Accepted deliberately.
                                details.facet-group__more open[group_has_selection] {
                                    summary.facet-group__more-toggle {
                                        span.facet-group__more-label {
                                            (facet_group_remainder_text(hidden.len(), lang))
                                        }
                                    }
                                    ul.facet-options.facet-options--rest {
                                        @for opt in &hidden {
                                            (render_facet_option(cfg, &group.property, opt))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One `<li class="facet-option">` toggle row.
///
/// Factored out of `render_facet_panel`'s option loop only so the visible list
/// and the per-group `<details>` remainder list emit **the same** markup from
/// one place.
fn render_facet_option(cfg: &FacetPanelCfg<'_>, property: &str, opt: &FacetOption) -> Markup {
    let href = toggle_href(cfg, property, &opt.value);
    let frag = fragment_href(cfg, property, &opt.value);
    html! {
        li.facet-option {
            a   .facet-option__toggle
                href=(href)
                data-on:click__prevent=(datastar::get(frag))
                aria-current=[opt.selected.then_some("true")]
            {
                // CSS check-glyph box; removed from a11y tree.
                span.facet-option__box aria-hidden="true" {}
                span.facet-option__label { (opt.label) }
                span.facet-option__count { (opt.count) }
            }
        }
    }
}

/// Render the active-filter chip row: `<div class="filterbar" id="filters-{list_id}">`.
///
/// This is a **second view** of the same `groups` data `render_facet_panel` renders:
/// a flat `.browse` grid child
/// placed by the composition layer (`render_body`), and a **third** DataStar patch
/// region of the `_facets` fragment endpoint alongside the panel and the list — see
/// `FACET_FRAGMENT_SEGMENT` and `dsp-incubator cpe/bin`'s `make_facet_fragment_handler`.
///
/// - **Empty** (no option across any group is `selected` AND `query` is `None`): emits the bare
///   `<div id="filters-{list_id}">` with **no children** — a stable morph target that collapses via
///   `.filterbar:empty { display: none; }` (record.css), rather than being omitted entirely (an
///   absent element cannot be a DataStar patch target). An active query alone (no facet selected)
///   is enough to make the bar non-empty (the query chip renders with no facet chips beside it).
/// - **Populated**: an optional `.filterchip--query` chip (`query.is_some()`, see the module doc
///   comment) **first**, then one ink-fill `.filterchip` `<a>` per selected option — the chip's
///   `href`/`@get` **toggles that option OFF** (same `toggle_href`/`fragment_href` the rail uses),
///   so removing a chip mirrors clicking the same option in the panel — followed by a
///   `.filterbar__clear` "Clear all" `<a>` that drops every facet param (+ `page` + `q`) via
///   `clear_filters_query`.
///
/// Zero-JS: every facet-chip/Clear-all control here is a plain `<a href=…>`;
/// `data-on:click__prevent` only enhances it through the shared `_facets`
/// fragment `@get`, built with `shared_datastar::get`. The query chip is the one exception
/// — see the module doc comment.
///
/// **No `.page` class (deliberate, unlike every other component root):** the
/// LAYOUT CONTRACT's `.page` marker exists so `grid.css`'s `.page > *` blanket
/// rule can lay out a component's own children; this element instead sits as
/// a *sibling* grid child of `.browse.page` and is placed by `record.css`'s
/// `.browse > .filterbar` rule (specificity 0,2,0 — out-specifies the 0,1,0
/// blanket rule that would otherwise snap it full-width).
///
/// The root also carries `aria-live="polite" aria-atomic="true"` (a11y
/// review fix) — this small chip row, not the large `facet-panel` section
/// root, is the concise live region for the `_facets` swap: removing/adding
/// a chip is announced politely as one atomic unit, without re-announcing
/// every group/option in the panel and without moving focus or the URL
/// (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`).
pub fn render_active_filters(
    groups: &[FacetGroup],
    cfg: &FacetPanelCfg<'_>,
    lang: &Lang,
    query: Option<&str>,
) -> Markup {
    let filterbar_id = format!("filters-{}", cfg.list_id);
    let has_selected = any_facet_selected(groups);

    if !has_selected && query.is_none() {
        return html! {
            div.filterbar id=(filterbar_id) aria-live="polite" aria-atomic="true" {}
        };
    }

    let filters_label_text = ChromeMsg::FacetFiltersWord.text(lang);
    let clear_all_text = ChromeMsg::FacetClearAllLabel.text(lang);
    let facet_keys: Vec<&str> = groups.iter().map(|g| g.property.as_str()).collect();
    let clear_all_href = match clear_filters_query(cfg.current_query, &facet_keys) {
        Some(qs) => format!("{}?{}", cfg.current_path, qs),
        None => cfg.current_path.to_owned(),
    };
    let clear_all_frag = clear_all_fragment_href(cfg, &facet_keys);
    // "Clear all" drops `q` too, so it must also EMPTY the
    // search field — but `#q` lives in the search row, OUTSIDE this filter bar's
    // `_facets`/`_search` swap, so it is never re-rendered and would otherwise
    // keep the stale typed text after the reset. Reference `#q` document-wide
    // (`document.getElementById` — a boot-enforced page singleton; `el.form`
    // can't reach it since this control is in a different form than the input).
    // Gated on an active query, which guarantees a search-bar (hence `#q`) is on
    // the page, so the lookup is never null (a facet-only page has no `#q`).
    let clear_all_click = match query {
        Some(_) => {
            format!("document.getElementById('q').value = ''; {}", datastar::get(clear_all_frag))
        }
        None => datastar::get(clear_all_frag).to_string(),
    };

    html! {
        div.filterbar id=(filterbar_id) aria-live="polite" aria-atomic="true" {
            span.filterbar__lbl { (filters_label_text) }
            @if let Some(q) = query { (render_query_chip(cfg, q, lang)) }
            @for group in groups {
                @for opt in &group.options {
                    @if opt.selected {
                        @let href = toggle_href(cfg, &group.property, &opt.value);
                        @let frag = fragment_href(cfg, &group.property, &opt.value);
                        @let remove_label = remove_filter_label(lang, &opt.label);
                        a   .filterchip
                            href=(href)
                            data-on:click__prevent=(datastar::get(frag))
                            aria-label=(remove_label)
                        {
                            (opt.label)
                            span.filterchip__x aria-hidden="true" { "×" }
                        }
                    }
                }
            }
            a.filterbar__clear href=(clear_all_href) data-on:click__prevent=(clear_all_click) {
                (clear_all_text)
            }
        }
    }
}

/// Render the query chip: `<form method=get>` (a mini no-JS form — the chip
/// button needs `name="q" value=""`, unlike the facet chips' plain `<a
/// href>`, which cannot post an empty value) wrapping a `button.filterchip
/// .filterchip--query` (the query is quoted in straight ASCII `"…"`, a fixed
/// typographic treatment set by CSS italics — not a per-language quote glyph
/// like `search_no_match_text`'s DE „…"/FR «…»).
///
/// Rendered **before** the facet chips (see [`render_active_filters`]).
/// Clearing it is functionally identical to the search-bar's own `×` clear
/// button: same preserved-query rule (drops `q`/`page`/`datastar` via
/// [`search_bar::preserved_query_dropping_q_and_page`] /
/// [`cpe_core::util::hidden_pairs_dropping`] + [`search_bar::SEARCH_HIDDEN_DROP`]) and the same
/// `SEARCH_FRAGMENT_SEGMENT` fragment endpoint (a 2-event swap: filterbar +
/// list — facet counts stay static, so this action never
/// re-renders the facet panel). The aria-label reuses `ChromeMsg::SearchClear`
/// ("Clear search") — the same string the search row's own `×` carries — no
/// new chrome message needed for a control with identical behaviour.
///
/// **Reflected-input safety:** `q` renders via Maud's `(q)` interpolation
/// (auto-escaped) inside `span.q` — never `PreEscaped`/`format!`.
fn render_query_chip(cfg: &FacetPanelCfg<'_>, q: &str, lang: &Lang) -> Markup {
    let hidden_pairs = cpe_core::util::hidden_pairs_dropping(cfg.current_query, search_bar::SEARCH_HIDDEN_DROP);
    let preserved_qs = search_bar::preserved_query_dropping_q_and_page(cfg.current_query);
    // `search_fragment_prefix` only prepends `&` before `q=` when
    // `preserved_qs` is non-empty — shared with `search_bar::render_search_bar`
    // so the two `_search` URL builders can never drift on the empty-query
    // edge case (a stray leading `?&q=` when no facets/sort are active).
    let frag = search_bar::search_fragment_prefix(cfg.current_path, &preserved_qs);
    let clear_label = ChromeMsg::SearchClear.text(lang);

    html! {
        form.filterchip-form method="get" action=(cfg.current_path) {
            @for (k, v) in &hidden_pairs {
                input type="hidden" name=(k) value=(v);
            }
            button
                .filterchip
                .filterchip--query
                type="submit"
                name="q"
                value=""
                data-on:click__prevent=({
                    format!(
                        "document.getElementById('q').value = ''; {}",
                        datastar::get(frag),
                    )
                })
                aria-label=(clear_label)
            {
                span.q { "\"" (q) "\"" }
                span.filterchip__x aria-hidden="true" { "×" }
            }
        }
    }
}

// Query-string helpers

/// Extract the key portion of a `key=value` (or keyless) query-string pair.
///
/// Returns the whole pair when it carries no `=` (a keyless/flag pair), and
/// the substring before the first `=` otherwise (so `"a=b=c"` yields `"a"`).
fn pair_key(pair: &str) -> &str {
    match pair.find('=') {
        Some(idx) => &pair[..idx],
        None => pair,
    }
}

/// Split `current_query` on `&`, drop empty segments and every pair whose key
/// satisfies `drop`, and rejoin what's left.
///
/// Returns `None` when nothing is kept (caller should use the bare path);
/// returns `Some(qs)` (no leading `?`) otherwise. Shared by
/// [`clear_filters_query`] and [`clear_group_query`].
fn kept_pairs(current_query: Option<&str>, drop: impl Fn(&str) -> bool) -> Option<String> {
    let q = current_query.unwrap_or("");
    let kept: Vec<&str> = q.split('&').filter(|pair| !pair.is_empty() && !drop(pair_key(pair))).collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept.join("&"))
    }
}

/// Build the query string for the filterbar's "Clear all" link.
///
/// Drops every `key=value` pair whose key appears in `facet_keys`, AND drops
/// the `page` param (resetting to page 1), AND drops `q` ("Clear all"
/// clears everything, the active search too). All other params (e.g. `lang`,
/// `appearance`) are preserved so clicking "Clear all" doesn't strip
/// unrelated user preferences — consistent with the individual toggle links.
/// Dropping `q` is unconditional and harmless when absent (a no-op filter).
///
/// Returns `None` when the preserved query string is empty (caller should
/// use the bare path); returns `Some(qs)` (no leading `?`) otherwise.
#[must_use]
pub fn clear_filters_query(current_query: Option<&str>, facet_keys: &[&str]) -> Option<String> {
    kept_pairs(current_query, |k| k == "page" || k == "q" || facet_keys.contains(&k))
}

/// Build the query string for a single group's "clear" link.
///
/// Drops every `key=value` pair whose key equals `property` (that group's
/// facet param only — other groups' selections are preserved), and drops the
/// `page` param (resetting to page 1). All other params (lang, appearance,
/// other facet groups) are preserved. Mirrors [`clear_filters_query`] but
/// scoped to one property instead of a `facet_keys` slice.
///
/// Returns `None` when the preserved query string is empty (caller should
/// use the bare path); returns `Some(qs)` (no leading `?`) otherwise.
#[must_use]
pub(crate) fn clear_group_query(current_query: Option<&str>, property: &str) -> Option<String> {
    kept_pairs(current_query, |k| k == "page" || k == property)
}

/// Build the post-toggle query string.
///
/// If the `key=value` pair is **absent** from `current_query`, add it.
/// If it is **present**, remove it (toggle off).
/// Always **drop** the `page` param (a facet change resets to page 1).
/// Preserve all other params (lang, appearance, other facets).
///
/// `None` / empty query + adding: returns `"{key}={value}"`.
/// `None` / empty query + toggling nothing: returns `""`.
///
/// Value encoding: keys/values are assumed ASCII-safe (Alice's facet ids are
/// ASCII — `role`, `R_01`, `keyword`, `K_005`, `lt-wonderland`). Percent-
/// encoding for non-ASCII node ids and for query-structural chars (`&`, `=`,
/// `+`) is not done (see module doc). A future hardening pass must
/// encode before this function is called, not inside it.
#[must_use]
pub(crate) fn toggle_query_param(current_query: Option<&str>, key: &str, value: &str) -> String {
    let target_pair = format!("{key}={value}");
    let mut parts: Vec<String> = Vec::new();
    let mut found = false;

    if let Some(q) = current_query {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let k = pair_key(pair);
            // Drop the `page` param — a facet toggle always resets to page 1.
            if k == "page" {
                continue;
            }
            if pair == target_pair {
                // This exact key=value is present → toggle it OFF by skipping.
                found = true;
            } else {
                parts.push(pair.to_owned());
            }
        }
    }

    // If the pair was not present, toggle it ON by appending.
    if !found {
        parts.push(target_pair);
    }

    parts.join("&")
}

/// Build a facet-panel href/fragment-URL of one of four shapes:
/// `path`, `path?qs`, `path/{FACET_FRAGMENT_SEGMENT}`, or
/// `path/{FACET_FRAGMENT_SEGMENT}?qs`. Shared shape behind the five named
/// href builders below.
#[must_use]
fn facet_href(path: &str, fragment: bool, qs: Option<&str>) -> String {
    match (fragment, qs) {
        (false, None) => path.to_owned(),
        (false, Some(qs)) => format!("{path}?{qs}"),
        (true, None) => format!("{path}/{FACET_FRAGMENT_SEGMENT}"),
        (true, Some(qs)) => format!("{path}/{FACET_FRAGMENT_SEGMENT}?{qs}"),
    }
}

/// Build the no-JS fallback href for toggling a facet option.
///
/// Returns `{current_path}?{post-toggle query}`. The toggle direction (add or
/// remove) is computed inside `toggle_query_param` from `current_query` —
/// no external `selected` flag is needed here.
#[must_use]
fn toggle_href(cfg: &FacetPanelCfg<'_>, property: &str, value: &str) -> String {
    let qs = toggle_query_param(cfg.current_query, property, value);
    let qs = if qs.is_empty() { None } else { Some(qs.as_str()) };
    facet_href(cfg.current_path, false, qs)
}

/// Build the DataStar `@get` fragment URL for toggling a facet option.
///
/// Returns `{current_path}/{FACET_FRAGMENT_SEGMENT}?{post-toggle query}`.
/// No `#…` anchor (unlike the compound `href`; the morph handles positioning).
/// The toggle direction is computed inside `toggle_query_param` — no external
/// `selected` flag is needed here.
#[must_use]
fn fragment_href(cfg: &FacetPanelCfg<'_>, property: &str, value: &str) -> String {
    let qs = toggle_query_param(cfg.current_query, property, value);
    let qs = if qs.is_empty() { None } else { Some(qs.as_str()) };
    facet_href(cfg.current_path, true, qs)
}

/// Build the no-JS fallback href for a per-group "clear" link.
///
/// Returns `{current_path}?{query with only `property` + `page` dropped}`.
#[must_use]
fn group_clear_href(cfg: &FacetPanelCfg<'_>, property: &str) -> String {
    let qs = clear_group_query(cfg.current_query, property);
    facet_href(cfg.current_path, false, qs.as_deref())
}

/// Build the DataStar `@get` fragment URL for a per-group "clear" link.
#[must_use]
fn group_clear_fragment_href(cfg: &FacetPanelCfg<'_>, property: &str) -> String {
    let qs = clear_group_query(cfg.current_query, property);
    facet_href(cfg.current_path, true, qs.as_deref())
}

/// Build the DataStar `@get` fragment URL for the filterbar's "Clear all" link.
///
/// Drops every facet param in `facet_keys` (+ `page`) — same query as the
/// no-JS `clear_all_href`, just under `FACET_FRAGMENT_SEGMENT`.
#[must_use]
fn clear_all_fragment_href(cfg: &FacetPanelCfg<'_>, facet_keys: &[&str]) -> String {
    let qs = clear_filters_query(cfg.current_query, facet_keys);
    facet_href(cfg.current_path, true, qs.as_deref())
}

// Tests

#[cfg(test)]
mod tests {
    use cpe_core::ir::{FacetKind, FacetOption};

    use super::*;

    // Test helpers

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

    fn make_cfg<'a>(list_id: &'a str, current_path: &'a str, current_query: Option<&'a str>) -> FacetPanelCfg<'a> {
        FacetPanelCfg { list_id, current_path, current_query }
    }

    fn make_group(property: &str, label: &str, options: Vec<FacetOption>) -> FacetGroup {
        FacetGroup {
            property: property.to_owned(),
            label: label.to_owned(),
            kind: FacetKind::List,
            options,
        }
    }

    fn make_option(value: &str, label: &str, count: usize, selected: bool) -> FacetOption {
        FacetOption {
            value: value.to_owned(),
            label: label.to_owned(),
            count,
            selected,
        }
    }

    /// `n` unselected options in **descending count** order — the order
    /// `query::list_facet_values` already returns (`ORDER BY cnt DESC, label,
    /// target_id`), so index 0..12 really is the "12 densest" prefix the
    /// split takes.
    fn make_options(n: usize) -> Vec<FacetOption> {
        (0..n)
            .map(|i| make_option(&format!("V_{i:03}"), &format!("Option {i}"), 1000 - i, false))
            .collect()
    }

    /// `(options rendered before the per-group `<details>`, options inside it)`
    /// for a **single-group** panel. A plain page-wide `li.facet-option` count
    /// is blind to the collapse — every remainder option is still in the DOM,
    /// just inside the disclosure — so the split is what has to be asserted.
    fn rendered_split(out: &str) -> (usize, usize) {
        const OPT: &str = "<li class=\"facet-option\">";
        match out.find("<details class=\"facet-group__more\"") {
            Some(i) => (out[..i].matches(OPT).count(), out[i..].matches(OPT).count()),
            None => (out.matches(OPT).count(), 0),
        }
    }

    // Layer-1: toggle_query_param

    /// Adding a new key=value when the pair is absent.
    #[test]
    fn toggle_query_param_adds_when_absent() {
        let qs = toggle_query_param(Some("lang=en"), "role", "R_01");
        assert_eq!(qs, "lang=en&role=R_01");
    }

    /// Removing an exact key=value pair when it is present (toggle off).
    #[test]
    fn toggle_query_param_removes_when_present() {
        let qs = toggle_query_param(Some("lang=en&role=R_01"), "role", "R_01");
        assert_eq!(qs, "lang=en");
    }

    /// A different value under the same key is preserved (multi-select: adding
    /// role=R_02 while role=R_01 is active keeps both).
    #[test]
    fn toggle_query_param_preserves_different_value_same_key() {
        let qs = toggle_query_param(Some("lang=en&role=R_01"), "role", "R_02");
        assert_eq!(qs, "lang=en&role=R_01&role=R_02");
    }

    /// `page` param is always dropped, regardless of the toggle action.
    #[test]
    fn toggle_query_param_drops_page() {
        let qs = toggle_query_param(Some("lang=en&page=3&role=R_01"), "keyword", "K_005");
        // `page` must not appear; new keyword pair is appended.
        assert_eq!(qs, "lang=en&role=R_01&keyword=K_005");
    }

    /// `page` is dropped even when it is the only param besides the toggled one.
    #[test]
    fn toggle_query_param_drops_page_only_other_param() {
        let qs = toggle_query_param(Some("page=2"), "role", "R_01");
        assert_eq!(qs, "role=R_01");
    }

    /// `None` query + adding: returns the single `key=value` pair.
    #[test]
    fn toggle_query_param_none_query_adds() {
        let qs = toggle_query_param(None, "role", "R_01");
        assert_eq!(qs, "role=R_01");
    }

    /// All other params are preserved unchanged.
    #[test]
    fn toggle_query_param_preserves_other_params() {
        let qs = toggle_query_param(Some("lang=de&appearance=dark&role=R_01"), "keyword", "K_005");
        assert_eq!(qs, "lang=de&appearance=dark&role=R_01&keyword=K_005");
    }

    /// `view=grid` survives a facet toggle (each
    /// query-param-preserving helper needs its own explicit assertion).
    #[test]
    fn toggle_query_param_preserves_view() {
        let qs = toggle_query_param(Some("lang=de&view=grid&role=R_01"), "keyword", "K_005");
        assert_eq!(qs, "lang=de&view=grid&role=R_01&keyword=K_005");
    }

    /// Removing the only non-page param yields an empty string.
    #[test]
    fn toggle_query_param_remove_last_non_page_param() {
        let qs = toggle_query_param(Some("role=R_01"), "role", "R_01");
        assert_eq!(qs, "");
    }

    // Layer-1: kept_pairs / pair_key

    #[test]
    fn kept_pairs_skips_empty_segments() {
        assert_eq!(kept_pairs(Some("a=1&&b=2&"), |_| false).as_deref(), Some("a=1&b=2"));
    }

    #[test]
    fn kept_pairs_all_dropped_is_none() {
        assert_eq!(kept_pairs(Some("page=2"), |k| k == "page"), None);
        assert_eq!(kept_pairs(None, |_| false), None);
    }

    #[test]
    fn pair_key_keyless_pair_is_whole_pair() {
        assert_eq!(pair_key("flag"), "flag");
        assert_eq!(pair_key("a=b=c"), "a");
    }

    // Layer-1: empty branches

    /// `groups` is empty — `render_facet_panel` renders literally
    /// nothing (no rail, no placeholder section, no `.page`). The "no rail"
    /// composition itself lives in `render_body` (`dsp-incubator cpe/web`'s `page` module); this
    /// tests the raw render function's own defensive fallback.
    #[test]
    fn empty_branch_no_groups_renders_nothing() {
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&[], &cfg, &lang).into_string();
        assert_eq!(out, "", "E3b (no groups) must render nothing, got: {out}");
    }

    /// Groups present but all have zero options — root still carries `.page`.
    #[test]
    fn empty_branch_groups_no_options_has_page_class() {
        let groups = vec![make_group("role", "Role", vec![])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();

        assert!(
            out.contains("class=\"facet-panel page\""),
            "empty-branch (E3a) root must carry class=\"facet-panel page\", got: {out}"
        );
        // Heading still appears.
        assert!(
            out.contains("Role"),
            "group heading must appear even with no options, got: {out}"
        );
        // Group-specific note appears (not a bare empty <ul>).
        assert!(
            out.contains("No role are recorded for this class."),
            "group with no options must show the group-specific empty-state note, got: {out}"
        );
    }

    /// DE empty-state message (German uses the non-inflecting fallback).
    #[test]
    fn empty_branch_de_message() {
        let groups = vec![make_group("role", "Rolle", vec![])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("de");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            out.contains("Für diese Klasse sind keine Werte erfasst."),
            "DE empty-state must appear, got: {out}"
        );
    }

    /// Panel id is `facets-{list_id}` on a populated panel.
    #[test]
    fn panel_id_is_facets_prefix_plus_list_id() {
        let groups = vec![make_group("role", "Role", vec![])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            out.contains("id=\"facets-character-list\""),
            "panel id must be 'facets-character-list', got: {out}"
        );
    }

    // Layer-3: insta snapshots

    /// (a) Populated multi-group panel with some options selected.
    #[test]
    fn snapshot_facet_panel_populated_multi_group() {
        let groups = vec![
            make_group(
                "role",
                "Role",
                vec![
                    make_option("R_01", "Main character", 12, true),
                    make_option("R_02", "Secondary character", 28, false),
                    make_option("R_03", "Minor character", 14, false),
                ],
            ),
            make_group(
                "keyword",
                "Keyword",
                vec![
                    make_option("K_001", "Wonderland", 18, false),
                    make_option("K_002", "Adventure", 22, true),
                    make_option("K_003", "Tea party", 5, false),
                ],
            ),
        ];
        let cfg = make_cfg(
            "character-list",
            "/alice-in-daschland/characters",
            Some("lang=en&role=R_01&keyword=K_002"),
        );
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (b) Single-group panel (no active selection).
    #[test]
    fn snapshot_facet_panel_single_group() {
        let groups = vec![make_group(
            "location-type",
            "Location Type",
            vec![
                make_option("lt-wonderland", "Wonderland", 20, false),
                make_option("lt-real-world", "Real world", 12, false),
            ],
        )];
        let cfg = make_cfg("location-list", "/alice-in-daschland/locations", Some("lang=en"));
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (c) No groups — renders nothing (empty string).
    #[test]
    fn snapshot_facet_panel_empty_no_groups() {
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&[], &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (d) Group present but zero options.
    #[test]
    fn snapshot_facet_panel_empty_group_no_options() {
        let groups = vec![make_group("role", "Role", vec![])];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    /// (e) A group ABOVE `FACET_GROUP_COLLAPSE_THRESHOLD`, collapsed.
    ///
    /// Built from **36** options on purpose: 36 is Alice's real distinct
    /// `keyword` list-node count on `/alice-in-daschland/characters` (281
    /// `keyword` rows over 54 Characters in
    /// `projects/alice-in-daschland/data.sql`), the one live
    /// surface this engine-wide threshold changes. The synthetic fixtures are
    /// never built from Alice's data, so without this pin the collapse of a
    /// real browse would be a silent consequence. Summary must read "24 more options".
    #[test]
    fn snapshot_facet_panel_group_above_threshold_collapsed() {
        let groups = vec![make_group("keyword", "Keyword", make_options(36))];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("lang=en"));
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        insta::assert_snapshot!(out);
    }

    // Layer-2: per-group disclosure split

    /// The 94-option season group: 12 visible, 82 behind the `<details>`.
    #[test]
    fn facet_group_94_options_renders_12_visible_and_82_hidden() {
        let groups = vec![make_group("season", "Season", make_options(94))];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(
            rendered_split(&out),
            (12, 82),
            "94 options must render 12 visible + 82 in the disclosure, got: {out}"
        );
        assert!(
            out.contains("Show 82 more options"),
            "summary must name the remainder, got: {out}"
        );
        assert!(
            !out.contains("<details class=\"facet-group__more\" open>"),
            "no selection in this group → the disclosure is closed, got: {out}"
        );
    }

    /// Selecting option #90 (1-based) hoists it into the visible set: 13
    /// visible, 81 hidden — and the disclosure is now `open`, because THIS
    /// group has a selection (the `_facets` morph would otherwise snap it
    /// shut mid-interaction).
    #[test]
    fn facet_group_94_options_selected_tail_option_is_hoisted() {
        let mut options = make_options(94);
        options[89].selected = true;
        let groups = vec![make_group("season", "Season", options)];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(
            rendered_split(&out),
            (13, 81),
            "the selected tail option must be hoisted, got: {out}"
        );
        assert!(
            out.contains("<details class=\"facet-group__more\" open>"),
            "a group with a selection keeps its disclosure open, got: {out}"
        );
        // The hoisted option is in the VISIBLE list, not the remainder.
        let head = &out[..out.find("<details class=\"facet-group__more\"").unwrap()];
        assert!(
            head.contains("Option 89"),
            "the selected option must appear above the disclosure, got: {head}"
        );
    }

    /// 13 options (the `Jahrzehnt` decade group) render flat — this is
    /// exactly why the threshold (16) and the visible count (12) are two
    /// different numbers.
    #[test]
    fn facet_group_13_options_renders_flat_no_disclosure() {
        let groups = vec![make_group("decade", "Decade", make_options(13))];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(rendered_split(&out), (13, 0), "got: {out}");
        assert!(!out.contains("facet-group__more"), "got: {out}");
    }

    /// 16 options — exactly AT the threshold — still render flat.
    #[test]
    fn facet_group_16_options_at_threshold_renders_flat() {
        let groups = vec![make_group("category", "Category", make_options(16))];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(rendered_split(&out), (16, 0), "got: {out}");
        assert!(!out.contains("facet-group__more"), "got: {out}");
    }

    /// 17 options — one over the threshold — collapse to 12 + 5.
    #[test]
    fn facet_group_17_options_renders_12_plus_5() {
        let groups = vec![make_group("category", "Category", make_options(17))];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(rendered_split(&out), (12, 5), "got: {out}");
        assert!(out.contains("Show 5 more options"), "got: {out}");
    }

    /// Remainder 0: a 17-option group with ALL five tail options selected
    /// hoists every one of them — 17 flat options and NO `<details>` at all.
    /// "Show 0 more" must never be rendered.
    #[test]
    fn facet_group_17_options_all_remainder_selected_renders_flat() {
        let mut options = make_options(17);
        for opt in options.iter_mut().skip(12) {
            opt.selected = true;
        }
        let groups = vec![make_group("category", "Category", options)];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(rendered_split(&out), (17, 0), "got: {out}");
        assert!(
            !out.contains("facet-group__more"),
            "remainder 0 must render no disclosure, got: {out}"
        );
        assert!(!out.contains("0 more"), "never 'Show 0 more', got: {out}");
    }

    /// Remainder 1 takes the SINGULAR plural arm. This assertion is only
    /// meaningful because the chosen copy names the thing counted
    /// ("option"/"options", "Option"/"Optionen") — a noun-less
    /// "Show N more" / "N weitere anzeigen" would have made One and Other
    /// byte-identical apart from the numeral, i.e. a test that cannot fail.
    #[test]
    fn facet_group_remainder_one_uses_the_singular_arm() {
        let mut options = make_options(17);
        for opt in options.iter_mut().skip(12).take(4) {
            opt.selected = true;
        }
        let groups = vec![make_group("category", "Category", options)];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));

        let out_en = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(rendered_split(&out_en), (16, 1), "got: {out_en}");
        assert!(out_en.contains("Show 1 more option"), "got: {out_en}");
        assert!(
            !out_en.contains("Show 1 more options"),
            "singular arm must not carry the plural noun, got: {out_en}"
        );

        let out_de = render_facet_panel(&groups, &cfg, &make_lang("de")).into_string();
        assert!(out_de.contains("1 weitere Option anzeigen"), "got: {out_de}");
        assert!(
            !out_de.contains("Optionen"),
            "singular arm must not carry the plural noun, got: {out_de}"
        );
    }

    /// The selected-option count is unaffected by the split: `aria-current`
    /// markers are counted across BOTH the visible list and the remainder
    /// `<details>` (the same thing `smoke.rs`'s `selected_facet_option_count`
    /// counts page-wide).
    #[test]
    fn selected_option_count_survives_the_group_split() {
        let mut options = make_options(94);
        // One inside the visible prefix, two deep in the tail.
        options[3].selected = true;
        options[50].selected = true;
        options[93].selected = true;
        let groups = vec![make_group("season", "Season", options)];
        let cfg = make_cfg("perf-list", "/hdm/performances", Some("lang=en"));
        let out = render_facet_panel(&groups, &cfg, &make_lang("en")).into_string();
        assert_eq!(
            out.matches("aria-current=\"true\"").count(),
            3,
            "every selected option must still render exactly one marker, got: {out}"
        );
        // One was already in the densest-12 prefix, two are hoisted out of the
        // tail → 14 visible, 80 hidden.
        assert_eq!(rendered_split(&out), (14, 80), "got: {out}");
    }

    // Layer-1: toggle link href correctness

    /// Toggling an unselected option appends the pair to the query.
    #[test]
    fn toggle_href_adds_unselected_option() {
        let cfg = make_cfg("character-list", "/alice/characters", Some("lang=en&role=R_01"));
        let href = toggle_href(&cfg, "role", "R_02");
        assert_eq!(href, "/alice/characters?lang=en&role=R_01&role=R_02");
    }

    /// Toggling a selected option removes the pair from the query.
    #[test]
    fn toggle_href_removes_selected_option() {
        let cfg = make_cfg("character-list", "/alice/characters", Some("lang=en&role=R_01"));
        let href = toggle_href(&cfg, "role", "R_01");
        assert_eq!(href, "/alice/characters?lang=en");
    }

    /// Removing the last facet param yields the bare path (no `?`).
    #[test]
    fn toggle_href_removing_last_facet_yields_bare_path() {
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01"));
        let href = toggle_href(&cfg, "role", "R_01");
        assert_eq!(href, "/alice/characters");
    }

    /// fragment_href includes the FACET_FRAGMENT_SEGMENT.
    #[test]
    fn fragment_href_includes_segment() {
        let cfg = make_cfg("character-list", "/alice/characters", Some("lang=en"));
        let fhref = fragment_href(&cfg, "role", "R_01");
        assert_eq!(fhref, "/alice/characters/_facets?lang=en&role=R_01");
    }

    // Layer-1: selected option carries aria-current="true"

    #[test]
    fn selected_option_has_aria_current_true() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)])];
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01"));
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            out.contains("aria-current=\"true\""),
            "selected option must carry aria-current=\"true\", got: {out}"
        );
    }

    /// Unselected option must NOT carry aria-current.
    #[test]
    fn unselected_option_has_no_aria_current() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_02", "Secondary", 28, false)],
        )];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            !out.contains("aria-current"),
            "unselected option must NOT carry aria-current, got: {out}"
        );
    }

    // ── Layer-1: clear_filters_query (used by render_active_filters' Clear-all) ─

    /// clear_filters_query preserves lang and drops facet key + page.
    #[test]
    fn clear_filters_query_preserves_non_facet_params() {
        let result = clear_filters_query(Some("lang=de&role=R_01&page=2"), &["role", "keyword"]);
        assert_eq!(result, Some("lang=de".to_owned()));
    }

    /// clear_filters_query returns None when only facet + page params remain.
    #[test]
    fn clear_filters_query_returns_none_when_nothing_remains() {
        let result = clear_filters_query(Some("role=R_01&page=2"), &["role", "keyword"]);
        assert_eq!(result, None);
    }

    /// clear_filters_query with None current_query returns None.
    #[test]
    fn clear_filters_query_none_query_returns_none() {
        let result = clear_filters_query(None, &["role"]);
        assert_eq!(result, None);
    }

    /// `view=grid` survives "Clear all" (this is
    /// DELIBERATE, not incidental: "Clear all" clears *filters*, and the view
    /// is not a filter, so `view` must be preserved here unlike `q`/facets/`page`).
    #[test]
    fn clear_filters_query_preserves_view() {
        let result = clear_filters_query(Some("lang=de&view=grid&role=R_01&page=2"), &["role", "keyword"]);
        assert_eq!(result, Some("lang=de&view=grid".to_owned()));
    }

    // Layer-1: clear_group_query (per-group clear)

    /// clear_group_query drops only the named property's pairs + page,
    /// preserving other groups' selections and unrelated params.
    #[test]
    fn clear_group_query_drops_only_named_property_and_page() {
        let result = clear_group_query(Some("lang=de&role=R_01&role=R_02&keyword=K_001&page=2"), "role");
        assert_eq!(result, Some("lang=de&keyword=K_001".to_owned()));
    }

    /// `view=grid` survives a per-group clear.
    #[test]
    fn clear_group_query_preserves_view() {
        let result = clear_group_query(Some("lang=de&view=grid&role=R_01&page=2"), "role");
        assert_eq!(result, Some("lang=de&view=grid".to_owned()));
    }

    /// clear_group_query returns None when only that property + page remain.
    #[test]
    fn clear_group_query_returns_none_when_nothing_remains() {
        let result = clear_group_query(Some("role=R_01&page=2"), "role");
        assert_eq!(result, None);
    }

    /// clear_group_query with None current_query returns None.
    #[test]
    fn clear_group_query_none_query_returns_none() {
        let result = clear_group_query(None, "role");
        assert_eq!(result, None);
    }

    // Layer-1: per-group clear presence

    /// Per-group clear link appears only for a group with ≥1 selected option.
    #[test]
    fn per_group_clear_present_only_for_selected_group() {
        let groups = vec![
            make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)]),
            make_group("keyword", "Keyword", vec![make_option("K_001", "Wonderland", 18, false)]),
        ];
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01"));
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        let clear_count = out.matches("class=\"facet-group__clear\"").count();
        assert_eq!(
            clear_count, 1,
            "exactly one group (Role) has a selection, so exactly one \
             .facet-group__clear must appear, got {clear_count} in: {out}"
        );
    }

    /// Per-group clear is absent entirely when no group has a selection.
    #[test]
    fn per_group_clear_absent_when_no_selection() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, false)])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            !out.contains("facet-group__clear"),
            "no group has a selection, so no .facet-group__clear must appear, got: {out}"
        );
    }

    /// Each group heading is wrapped in `.facet-group__titlerow`.
    #[test]
    fn group_heading_wrapped_in_titlerow() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, false)])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            out.contains("class=\"facet-group__titlerow\""),
            "group heading must be wrapped in .facet-group__titlerow, got: {out}"
        );
    }

    /// The `facet-panel` section root must NOT carry `aria-live` — the
    /// concise live region for the `_facets` swap lives on the separate
    /// filterbar (a11y review fix: an atomic live region on the whole panel
    /// would re-announce every group/option on each toggle).
    #[test]
    fn facet_panel_root_has_no_aria_live() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, false)])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_facet_panel(&groups, &cfg, &lang).into_string();
        assert!(
            !out.contains("aria-live"),
            "facet-panel root must NOT carry aria-live, got: {out}"
        );
    }

    // Layer-1: render_active_filters

    /// Empty (childless) `div.filterbar` with the id when nothing is selected.
    #[test]
    fn active_filters_empty_when_nothing_selected() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, false)])];
        let cfg = make_cfg("character-list", "/alice/characters", None);
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();
        assert_eq!(
            out, r#"<div class="filterbar" id="filters-character-list" aria-live="polite" aria-atomic="true"></div>"#,
            "filterbar must be a childless stable morph target when nothing \
             is selected, got: {out}"
        );
    }

    /// One chip per selected option, with the remove aria-label, the
    /// decorative aria-hidden "×", and a toggle-off href; plus "Clear all".
    #[test]
    fn active_filters_one_chip_per_selected_option() {
        let groups = vec![
            make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)]),
            make_group(
                "keyword",
                "Keyword",
                vec![
                    make_option("K_001", "Wonderland", 18, false),
                    make_option("K_002", "Adventure", 22, true),
                ],
            ),
        ];
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01&keyword=K_002"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();

        let chip_count = out.matches("class=\"filterchip\"").count();
        assert_eq!(
            chip_count, 2,
            "exactly 2 options selected → 2 chips, got {chip_count} in: {out}"
        );
        assert!(
            out.contains(r#"aria-label="Remove filter: Main""#),
            "chip must carry the remove aria-label, got: {out}"
        );
        assert!(
            out.contains(r#"aria-label="Remove filter: Adventure""#),
            "chip must carry the remove aria-label, got: {out}"
        );
        assert!(
            out.contains(r#"<span class="filterchip__x" aria-hidden="true">×</span>"#),
            "chip must contain the decorative aria-hidden × glyph, got: {out}"
        );
        // Toggle-off href for the Role chip: removes role=R_01, keeps keyword=K_002.
        assert!(
            out.contains(r#"href="/alice/characters?keyword=K_002""#),
            "Role chip href must toggle role=R_01 OFF (keeping keyword=K_002), got: {out}"
        );
        assert!(
            out.contains("class=\"filterbar__clear\""),
            "Clear all link must be present, got: {out}"
        );
        assert!(
            out.contains(">Clear all<"),
            "Clear all link text must read 'Clear all', got: {out}"
        );
    }

    /// filterbar id is always `filters-{list_id}`, populated or empty.
    #[test]
    fn active_filters_id_is_filters_prefix_plus_list_id() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)])];
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();
        assert!(
            out.contains(r#"id="filters-character-list""#),
            "filterbar id must be 'filters-character-list', got: {out}"
        );
    }

    // Layer-3: insta snapshots — render_active_filters

    /// Populated filterbar: two chips (from two groups) + Clear all.
    #[test]
    fn snapshot_active_filters_populated() {
        let groups = vec![
            make_group("role", "Role", vec![make_option("R_01", "Main character", 12, true)]),
            make_group("keyword", "Keyword", vec![make_option("K_002", "Adventure", 22, true)]),
        ];
        let cfg = make_cfg(
            "character-list",
            "/alice-in-daschland/characters",
            Some("lang=en&role=R_01&keyword=K_002"),
        );
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();
        insta::assert_snapshot!(out);
    }

    /// Empty filterbar: no option selected anywhere.
    #[test]
    fn snapshot_active_filters_empty() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_01", "Main character", 12, false)],
        )];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("lang=en"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();
        insta::assert_snapshot!(out);
    }

    // Layer-1/3: query chip

    /// An active query with NO facet selected still makes the filterbar
    /// non-empty (the query chip renders
    /// alone). Guards the `!has_selected && query.is_none()` gate.
    #[test]
    fn active_filters_non_empty_with_query_only_no_facets_selected() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_01", "Main character", 12, false)],
        )];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        assert!(
            out.contains(r#"class="filterchip filterchip--query""#),
            "an active query with no facet selected must still render the \
             query chip, got: {out}"
        );
        assert!(
            !out.contains(
                r#"<div class="filterbar" id="filters-character-list" aria-live="polite" aria-atomic="true"></div>"#
            ),
            "the filterbar must NOT be the empty childless stable target when \
             a query is active, got: {out}"
        );
    }

    /// `query: None` (no active search) preserves the pre-step-9 empty-bar
    /// behaviour exactly — no query chip, no `<form>` wrapper.
    #[test]
    fn active_filters_no_query_chip_when_query_none() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)])];
        let cfg = make_cfg("character-list", "/alice/characters", Some("role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, None).into_string();
        assert!(
            !out.contains("filterchip--query") && !out.contains("filterchip-form"),
            "query: None must render no query chip at all, got: {out}"
        );
    }

    /// Query chip renders BEFORE the facet chips.
    #[test]
    fn active_filters_query_chip_renders_before_facet_chips() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_01", "Main character", 12, true)],
        )];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab&role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        let query_chip_pos = out.find("filterchip--query").expect("query chip must be present");
        let facet_chip_pos = out
            .find(r#"aria-label="Remove filter: Main character""#)
            .expect("facet chip must be present");
        assert!(
            query_chip_pos < facet_chip_pos,
            "the query chip must render before the facet chips, got: {out}"
        );
    }

    /// The query chip's no-JS fallback is a real `<form method=get>` (not a
    /// plain `<a>`, unlike the facet chips) wrapping a `type="submit"
    /// name="q" value=""` button — a link alone cannot post an empty value.
    #[test]
    fn active_filters_query_chip_is_a_form_with_submit_button() {
        let groups: Vec<FacetGroup> = vec![];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        assert!(
            out.contains(r#"<form class="filterchip-form" method="get" action="/alice-in-daschland/characters">"#),
            "the query chip must be wrapped in its own mini no-JS form, got: {out}"
        );
        assert!(
            out.contains(r#"type="submit" name="q" value="""#),
            "the query chip button must be a submit posting an empty q, got: {out}"
        );
    }

    /// The query chip's `×` `@get` targets the `_search` fragment endpoint
    /// (2-event filterbar+list swap), NOT `_facets` — clearing the query
    /// never re-renders the facet panel (facet counts stay
    /// static).
    #[test]
    fn active_filters_query_chip_fragment_targets_search_segment() {
        let groups: Vec<FacetGroup> = vec![];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab&role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        assert!(
            out.contains("/alice-in-daschland/characters/_search?role=R_01&amp;q="),
            "the query chip must @get the _search endpoint, dropping q and \
             preserving role, got: {out}"
        );
    }

    /// `clear_filters_query` drops `q` too — the
    /// filterbar's own "Clear all" href reflects this.
    #[test]
    fn clear_filters_query_now_drops_q_too() {
        let result = clear_filters_query(Some("lang=de&role=R_01&q=rab&page=2"), &["role"]);
        assert_eq!(result, Some("lang=de".to_owned()));
    }

    /// "Clear all"'s href in `render_active_filters` drops `q`, the facet
    /// key, and `page`, leaving the bare path when nothing else remains.
    #[test]
    fn active_filters_clear_all_href_drops_query_and_facet() {
        let groups = vec![make_group("role", "Role", vec![make_option("R_01", "Main", 12, true)])];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab&role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        assert!(
            out.contains(r#"class="filterbar__clear" href="/alice-in-daschland/characters" "#),
            "Clear all must drop both q and role, leaving the bare path, got: {out}"
        );
    }

    /// A `<script>`-bearing query renders escaped inside the chip's `.q`
    /// span (reflected-input safety — never `PreEscaped`).
    #[test]
    fn active_filters_query_chip_escapes_hostile_query() {
        let groups: Vec<FacetGroup> = vec![];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=%3Cscript%3E"));
        let lang = make_lang("en");
        let hostile_q = "<script>alert(1)</script>";
        let out = render_active_filters(&groups, &cfg, &lang, Some(hostile_q)).into_string();
        assert!(
            !out.contains("<script>alert(1)</script>"),
            "a hostile query must never render as raw, unescaped markup, got: {out}"
        );
        assert!(
            out.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "the hostile query must render HTML-escaped inside the chip, got: {out}"
        );
    }

    // Layer-3: insta snapshots — query chip

    /// Query chip alone (no facet selected).
    #[test]
    fn snapshot_active_filters_query_chip_only() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_01", "Main character", 12, false)],
        )];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        insta::assert_snapshot!(out);
    }

    /// Query chip + a selected facet chip side by side.
    #[test]
    fn snapshot_active_filters_query_and_facet_chips() {
        let groups = vec![make_group(
            "role",
            "Role",
            vec![make_option("R_01", "Main character", 12, true)],
        )];
        let cfg = make_cfg("character-list", "/alice-in-daschland/characters", Some("q=rab&role=R_01"));
        let lang = make_lang("en");
        let out = render_active_filters(&groups, &cfg, &lang, Some("rab")).into_string();
        insta::assert_snapshot!(out);
    }
}
