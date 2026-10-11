//! Renderer for the `search-bar` component.
//!
//! Emits a labelled `form.search-row` — the `SEARCH` band that sits above the
//! filterbar on a faceted browse page, or directly above the list on a
//! non-faceted one (see `render_body`'s doc comment in `dsp-incubator cpe/web`'s `page` module for
//! the two placements and the `.page`/no-`.page` split between them).
//!
//! ## "Active" is visual, not the data-layer tokenizer test
//!
//! This renderer's own `data-active`/`×`/`aria-disabled`/`aria-describedby`
//! micro-interactions gate on raw non-emptiness of the decoded `q` string
//! (`!q.is_empty()`) — NOT `util::is_active_search_query` (the FTS-tokenizer
//! "has ≥1 usable term" test that governs the `WHERE` clause and the
//! no-match/scope-note state, steps 3/6/8). A visitor who has typed a single
//! `(` should still see the box read as "in use" even though that character
//! produces zero usable search terms. Do not import
//! `util::is_active_search_query` into this module.
//!
//! ## Reflected-input safety
//!
//! `q`'s rendered `value=`, the carry-over hidden inputs' `name=`/`value=`,
//! and every other piece of markup here go through Maud's own `(expr)`
//! interpolation (auto-escaping). Never wrap any of it in `PreEscaped` or
//! hand-build it with `format!` — the `format!` calls in this module build
//! JS-string/URL fragments for the `@get(...)` DataStar expressions, a
//! separate, non-HTML-markup concern.

use cpe_core::lang::Lang;
use cpe_core::util::encode_get_arg;
use maud::{html, Markup};

use crate::chrome::ChromeMsg;

/// The URL path segment that identifies the search-bar fragment endpoint.
///
/// The fragment endpoint lives at `{page-path}/{SEARCH_FRAGMENT_SEGMENT}?…`.
/// Both the engine (to build the live-fetch/clear `@get` URLs) and `dsp-incubator cpe/bin` (to
/// register the Axum route) reference this constant so no
/// magic-string mismatch is possible. Mirrors `FACET_FRAGMENT_SEGMENT` /
/// `SORT_FRAGMENT_SEGMENT`.
pub const SEARCH_FRAGMENT_SEGMENT: &str = "_search";

// ── Config borrow struct ──────────────────────────────────────────────────────

/// Borrowed context needed by `render_search_bar`.
///
/// All fields are `pub` (no `LinkResolver` — the search row has no
/// cross-resource links, matching `FacetPanelCfg`'s simplicity rather than
/// `ResourceListCfg`'s `with_links` factory pattern).
pub struct SearchBarCfg<'a> {
    /// URI path of the current request (e.g. `/alice-in-daschland/characters`).
    /// Used as the no-JS form `action` and as the prefix of the DataStar
    /// `@get` fragment URL.
    pub action_path: &'a str,
    /// Raw query string of the current request (everything after `?`, no
    /// leading `?`). `None` when the request had no query string.
    pub current_query: Option<&'a str>,
    /// The ALREADY-RESOLVED, translated placeholder text (the caller resolves
    /// `Component::SearchBar.placeholder`'s chrome-string id via
    /// `chrome_lookup` before constructing this cfg) — `None` when
    /// undeclared or unresolved, in which case no `placeholder` attribute is
    /// emitted.
    pub placeholder: Option<&'a str>,
    /// `true` when this row renders standalone (no `.browse` wrapper —
    /// the non-faceted case, e.g. Alice's `/chapters`): the root
    /// carries `.page` for its own centering. `false` when rendered as a
    /// flat grid child of `section.browse.page` (mirrors `.filterbar`, which
    /// also deliberately carries no `.page` — see `record.css`): the
    /// enclosing wrapper already supplies the centered box.
    pub standalone: bool,
}

/// Build the preserved query string for the search row's DataStar `@get`
/// prefix and the no-JS carry-over hidden inputs: every param from
/// `current_query` EXCEPT `q`, `page`, and `datastar` (a search change always
/// resets to page 1; `datastar` is the reserved DataStar-fetch marker key —
/// see `validate.rs`'s `RESERVED_FACET_PROPERTIES` / `dsp-incubator cpe/bin/src/router.rs`'s
/// `strip_datastar_marker`). Mirrors `resource_list::preserved_query_dropping_sort_and_page`
/// exactly, with `q`/`page`/`datastar` dropped instead of `sort`/`page`.
///
/// `pub(crate)` — reused by `facet_panel::render_query_chip`,
/// so the filterbar's query-chip `×` builds the *same* preserved
/// query and hits the *same* `_search` fragment endpoint as this row's own
/// `×` clear button, rather than a second, possibly-divergent computation.
pub(crate) fn preserved_query_dropping_q_and_page(current_query: Option<&str>) -> String {
    current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter(|pair| {
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            k != "q" && k != "page" && k != "datastar"
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// The keys a search-row (or query-chip) no-JS form must NOT carry over as
/// hidden inputs — the same drop-set as
/// [`preserved_query_dropping_q_and_page`], for
/// [`cpe_core::util::hidden_pairs_dropping`].
///
/// `pub(crate)` — shared with `facet_panel::render_query_chip`'s mini no-JS
/// form.
pub(crate) const SEARCH_HIDDEN_DROP: &[&str] = &["q", "page", "datastar"];

/// Build the `_search` fragment endpoint URL prefix:
/// `{action_path}/{SEARCH_FRAGMENT_SEGMENT}?{preserved_qs}&q=` — or, when
/// `preserved_qs` is empty, `{action_path}/{SEARCH_FRAGMENT_SEGMENT}?q=` (no
/// stray leading `&` with nothing before it). Used alone this IS the "clear"
/// URL (nothing appended after `q=`); with a value appended it's the live
/// debounced fetch prefix.
///
/// `pub(crate)` — shared with `facet_panel::render_query_chip`'s mirrored
/// construction of the SAME fragment URL (the filterbar's query chip `×`
/// hits the identical endpoint this row's own `×` does), so the two can never
/// drift on the empty-preserved-query edge case.
pub(crate) fn search_fragment_prefix(action_path: &str, preserved_qs: &str) -> String {
    if preserved_qs.is_empty() {
        format!("{action_path}/{SEARCH_FRAGMENT_SEGMENT}?q=")
    } else {
        format!("{action_path}/{SEARCH_FRAGMENT_SEGMENT}?{preserved_qs}&q=")
    }
}

/// Render the search row: `form.search-row role="search" method="get"` with
/// carry-over hidden inputs, the `SEARCH` label, and the `div.search-bar`
/// composite (input + conditional `×` clear + `→` submit).
///
/// - **Carry-over hidden inputs** — every current-query PAIR except `q`/`page`/`datastar` (facets,
///   sort, lang, appearance, …), so the no-JS `<form method=get>` round trip preserves the rest of
///   the request. Reflected-input safety: emitted only via Maud's `name=(k)`/`value=(v)`
///   interpolation (auto-escaped) — never hand-built.
/// - **Live debounced fetch** — `data-on:input__debounce.250ms` GETs a static `@get` prefix
///   (`{action_path}/{SEARCH_FRAGMENT_SEGMENT}?{preserved}&q=`, escaped for the DataStar
///   string-literal context via `encode_get_arg`) with the TYPED value appended
///   `encodeURIComponent`-wrapped at the JS layer — unlike the ordering control's per-row
///   `<button>` submits (a closed vocabulary, safe with `encode_get_arg` alone), free text with a
///   space/`&`/`+`/`#` would corrupt the fragment URL without the additional URL-encoding.
/// - **Clear `×`** — a real `type="submit" name="q" value=""` control (works with no JS); under
///   DataStar it ALSO fires the same static prefix (no value appended = `q=` empty) via
///   `data-on:click__prevent`, so a JS visitor gets the in-place swap too. The SAME click handler
///   also blanks the input's own `.value` directly (`el.form.querySelector('#q')` —
///   element-relative to the clicked button's form, and matched by the unique `id="q"` rather than
///   the `name="q"` the input SHARES with this clear button (so a `form.q` name-lookup would
///   ambiguously return both); `id="q"` is a boot-enforced page singleton, see
///   `DiagnosticKind::DuplicateSingletonComponent`), since `#q` sits OUTSIDE the `_search`
///   fragment's swapped region (only `#<list-id>` — and, on a faceted page, `#filters-<list-id>` —
///   get patched) and would otherwise keep showing the stale typed text after the results reset.
///   This is a click-time DOM write, not part of the morph, and writes no URL/focus (`dsp-incubator
///   cpe/docs/adr/0007-fragment-swaps-are-transient.md`-compatible). The expression is a FIXED
///   template string (never built from reflected user input), so it never needs Maud escaping.
/// - **Submit `→`** — a REAL plain form submit, deliberately with NO `data-on:click` override, even
///   under JS (`→` remains a real submit for visitors who prefer to commit — a deliberate choice,
///   not an oversight, distinct from clearing's "return to default" transient framing per
///   `dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`). It does NOT carry
///   `aria-disabled` while empty (dropped — an a11y review finding: the control remains fully
///   operable, just a benign reload, so announcing it as disabled was inconsistent) — the
///   empty-state dimming is still visual-only, driven by `grid.css`'s
///   `.search-bar:not([data-active="true"]) .search-bar__go` selector against the SAME
///   `data-active` state.
///
/// "Active" here is the VISUAL test (`!q.is_empty()`, raw non-emptiness) —
/// see this module's doc comment for why this must NOT be
/// `util::is_active_search_query`.
pub fn render_search_bar(cfg: &SearchBarCfg<'_>, lang: &Lang) -> Markup {
    let q = cpe_core::util::search_query_param(cfg.current_query);
    let active = !q.is_empty();

    // Carry-over hidden inputs: every current-query PAIR (not a deduped map)
    // except q/page/datastar, so a repeated facet key (e.g.
    // `role=R_01&role=R_02`) round-trips both values — same pattern as
    // `resource_list::render_order_control`'s `hidden_pairs`. Values are
    // decoded once (the browser re-encodes them on submit).
    let hidden_pairs = cpe_core::util::hidden_pairs_dropping(cfg.current_query, SEARCH_HIDDEN_DROP);

    // Static @get prefix — used alone (nothing appended) it IS the "clear"
    // URL (empty q=); with `+ encodeURIComponent(evt.target.value)` appended
    // by the browser at input time, it's the live debounced fetch.
    // `encode_get_arg` escapes JS-string-breakout chars (`'`, `` ` ``, `\`)
    // in the STATIC part only, mirroring `render_order_control`'s
    // `data_on_submit` construction exactly. `search_fragment_prefix` only
    // prepends `&` before `q=` when `preserved_qs` is non-empty, so an
    // unfaceted/unfiltered page never emits a stray `?&q=` (leading `&` with
    // nothing before it).
    let preserved_qs = preserved_query_dropping_q_and_page(cfg.current_query);
    let raw_prefix = search_fragment_prefix(cfg.action_path, &preserved_qs);
    let encoded_prefix = encode_get_arg(&raw_prefix);
    let live_fetch_expr = format!("@get('{encoded_prefix}' + encodeURIComponent(evt.target.value))");
    // Blank the input's own value (outside the SSE swap region) in the SAME
    // click, then fire the fragment fetch — see this fn's doc comment.
    let clear_click_expr = format!("el.form.querySelector('#q').value = ''; @get('{encoded_prefix}')");

    // The form's children — built once and interpolated into whichever root
    // (`.search-row.page` or `.search-row`) `cfg.standalone` selects, so the
    // whole body is never duplicated (`standalone` can't be resolved inside this fn
    // alone and must arrive pre-decided via `cfg`).
    let body = html! {
        @for (k, v) in &hidden_pairs {
            input type="hidden" name=(k) value=(v);
        }
        label.search-row__lbl for="q" { (ChromeMsg::SearchLabel.text(lang)) }
        div.search-bar data-active=[active.then_some("true")] {
            input
                .search-bar__input
                id="q"
                type="search"
                name="q"
                value=(q)
                autocomplete="off"
                placeholder=[cfg.placeholder]
                aria-describedby=[active.then_some("browse-count")]
                "data-on:input__debounce.250ms"=(live_fetch_expr);
            // Always in the DOM; revealed only when the field has text, via a
            // CSS `:placeholder-shown` sibling rule in grid.css. The row sits
            // outside the DataStar swap
            // (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`),
            // so a server `@if active` gate
            // could never make the × appear during live typing — CSS visibility
            // does. (Empty + JS-hidden `display:none` keeps it out of the a11y
            // tree; the no-JS submit is unaffected since a display:none submit
            // button can't be the activating control.)
            button
                .search-bar__clear
                type="submit"
                name="q"
                value=""
                title=(ChromeMsg::SearchClear.text(lang))
                aria-label=(ChromeMsg::SearchClear.text(lang))
                data-on:click__prevent=(clear_click_expr)
            { "×" }
            button
                .search-bar__go
                type="submit"
                title=(ChromeMsg::SearchSubmit.text(lang))
                aria-label=(ChromeMsg::SearchSubmit.text(lang))
            { "→" }
        }
    };

    if cfg.standalone {
        html! {
            form.search-row.page role="search" method="get" action=(cfg.action_path) { (body) }
        }
    } else {
        html! {
            form.search-row role="search" method="get" action=(cfg.action_path) { (body) }
        }
    }
}

#[cfg(test)]
mod tests {
    use cpe_core::ir::SiteConfig;
    use cpe_core::lang::resolve;

    use super::*;

    fn make_site() -> SiteConfig {
        SiteConfig {
            title: "Alice in DaSCHland".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned()],
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
        }
    }

    fn lang_en() -> Lang {
        resolve(&["en".to_owned()], &make_site())
    }

    // ── Layer-1: preserved_query_dropping_q_and_page ──────────────────────────

    /// `view=grid` survives while `q`/`page`/`datastar` drop (each
    /// query-param-preserving helper needs its own
    /// explicit assertion).
    #[test]
    fn preserved_query_dropping_q_and_page_preserves_view() {
        let qs = preserved_query_dropping_q_and_page(Some("view=grid&q=rab&page=2&datastar=true&lang=de"));
        assert_eq!(qs, "view=grid&lang=de");
    }

    /// `view=grid` survives while `q`/`page`/`datastar` drop
    /// — the pairs-based twin of
    /// `preserved_query_dropping_q_and_page_preserves_view` above.
    #[test]
    fn search_hidden_drop_preserves_view() {
        let pairs = cpe_core::util::hidden_pairs_dropping(
            Some("view=grid&role=R_01&q=alice&page=2&datastar=%7B%7D"),
            SEARCH_HIDDEN_DROP,
        );
        assert!(pairs.contains(&("view".to_owned(), "grid".to_owned())));
        assert!(pairs.contains(&("role".to_owned(), "R_01".to_owned())));
        assert!(!pairs.iter().any(|(k, _)| k == "datastar"));
        assert!(!pairs.iter().any(|(k, _)| *k == "q"));
        assert!(!pairs.iter().any(|(k, _)| *k == "page"));
    }

    #[test]
    fn empty_query_is_inactive() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: None,
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            !out.contains("data-active"),
            "empty query must not carry data-active, got: {out}"
        );
        assert!(
            out.contains("search-bar__clear"),
            "the × clear button is now ALWAYS in the DOM (its visibility is CSS-driven \
             via a `:placeholder-shown` sibling rule, so it can appear during live \
             typing without re-rendering the row); it must be present even for an \
             empty query, got: {out}"
        );
        assert!(
            !out.contains("aria-disabled"),
            "the → submit must NOT carry aria-disabled even while empty — it \
             stays fully operable (a benign reload); dimming is CSS-only, got: {out}"
        );
        assert!(
            !out.contains("aria-describedby"),
            "empty query must not carry aria-describedby, got: {out}"
        );
    }

    #[test]
    fn active_query_shows_active_state() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("q=rab"),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"data-active="true""#),
            "active query must carry data-active=\"true\", got: {out}"
        );
        assert!(
            out.contains("search-bar__clear"),
            "active query must render the × clear button, got: {out}"
        );
        assert!(
            !out.contains("aria-disabled"),
            "the → submit must never carry aria-disabled, got: {out}"
        );
        assert!(
            out.contains(r#"aria-describedby="browse-count""#),
            "active query must carry aria-describedby=\"browse-count\", got: {out}"
        );
        assert!(
            out.contains(r#"value="rab""#),
            "the input must echo the decoded query value, got: {out}"
        );
    }

    /// The `×` clear button's click handler blanks the input's own value (via
    /// its fixed page-singleton `id="q"`) in the SAME expression that fires
    /// the `_search` fragment fetch — `#q` sits outside the SSE swap region,
    /// so without this the stale typed text would linger after the results
    /// reset.
    #[test]
    fn clear_click_handler_also_blanks_the_input() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("q=rab"),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains("el.form.querySelector('#q').value = ''"),
            "the clear button's data-on:click must blank #q's value, got: {out}"
        );
        assert!(
            out.contains("/alice-in-daschland/characters/_search?q="),
            "the clear button must still fire the _search fragment fetch, got: {out}"
        );
    }

    /// When no other query params are preserved, the `_search` fragment URL
    /// must read `?q=` — never a stray leading `?&q=` (cheap fix).
    #[test]
    fn no_stray_ampersand_when_nothing_else_preserved() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/chapters",
            current_query: Some("q=rab"),
            placeholder: None,
            standalone: true,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains("/alice-in-daschland/chapters/_search?q="),
            "expected a bare '?q=' fragment prefix with no leading '&', got: {out}"
        );
        assert!(
            !out.contains("_search?&q="),
            "must never emit a stray '?&q=' separator, got: {out}"
        );
    }

    #[test]
    fn carry_over_param_with_quote_and_ampersand_renders_escaped() {
        // A raw hidden-input VALUE containing a literal `"` (percent-encoded
        // in the wire query string, decoded back to the raw char by the
        // split-on-`&`/`=` reader — same "no percent-decoding for non-q
        // params" behaviour as `render_order_control`'s hidden_pairs) must not
        // break out of the `value="..."` attribute.
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some(r#"role=R"01&q=x"#),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains("&quot;") || !out.contains(r#"value="R"01""#),
            "the raw '\"' in the carried-over value must be escaped, not \
             break the attribute, got: {out}"
        );
        // The attribute must still be well-formed: no bare unescaped `"`
        // inside what was meant to be one attribute value.
        assert!(
            !out.contains(r#"name="role" value="R"01"""#),
            "an unescaped embedded quote would prematurely terminate the \
             value attribute, got: {out}"
        );
    }

    /// Regression: a carried-over (non-`q`) pair's hidden
    /// input holds the DECODED value — the browser re-encodes on GET submit,
    /// so a raw `%20` would round-trip as `%2520`.
    #[test]
    fn carried_over_encoded_value_decodes_in_hidden_input() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("x=a%20b&role=R_01&q=cat"),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"input type="hidden" name="x" value="a b""#),
            "carried `x=a%20b` must decode once to `a b`, got: {out}"
        );
        assert!(
            out.contains(r#"input type="hidden" name="role" value="R_01""#),
            "a plain facet value must still round-trip unchanged, got: {out}"
        );
        // The `@get` fragment URL keeps the raw wire form (it IS a URL); only
        // the form-encoded hidden input must hold the decoded text.
        assert!(
            !out.contains(r#"value="a%20b""#),
            "no raw `%20` may survive into a hidden input, got: {out}"
        );
    }

    #[test]
    fn encoded_query_value_decodes_in_rendered_value_attr() {
        // Reuses `util::search_query_param`'s wiring — a light integration
        // check that this renderer calls through to it, not a re-test of
        // that fn's own unit tests.
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("q=quantum%20cat"),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"value="quantum cat""#),
            "an already-percent-encoded q must decode in the rendered value, got: {out}"
        );
    }

    #[test]
    fn standalone_true_adds_page_class() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/chapters",
            current_query: None,
            placeholder: None,
            standalone: true,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"class="search-row page""#),
            "standalone=true must carry the .page class, got: {out}"
        );
    }

    #[test]
    fn standalone_false_omits_page_class() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: None,
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"class="search-row""#),
            "standalone=false must NOT carry the .page class, got: {out}"
        );
    }

    #[test]
    fn placeholder_renders_when_present() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: None,
            placeholder: Some("name or description"),
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        assert!(
            out.contains(r#"placeholder="name or description""#),
            "resolved placeholder text must render, got: {out}"
        );
    }

    // ── Snapshots ────────────────────────────────────────────────────────────

    #[test]
    fn snapshot_search_bar_inactive_standalone() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/chapters",
            current_query: None,
            placeholder: None,
            standalone: true,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_search_bar_inactive_in_browse_grid() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: None,
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_search_bar_active_with_clear() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("q=rabbit"),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_search_bar_with_carry_over_hidden_inputs() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: Some("role=R_01&role=R_02&page=2&datastar=..."),
            placeholder: None,
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_search_bar_with_placeholder() {
        let cfg = SearchBarCfg {
            action_path: "/alice-in-daschland/characters",
            current_query: None,
            placeholder: Some("name or description"),
            standalone: false,
        };
        let out = render_search_bar(&cfg, &lang_en()).into_string();
        insta::assert_snapshot!(out);
    }
}
