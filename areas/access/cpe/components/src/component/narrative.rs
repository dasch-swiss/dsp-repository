//! Narrative component: language-variant resolution and HTML rendering.
//!
//! The resolution logic uses `util::variant_filename` (shared with `validate`).
//! Rendering delegates to `component::editorial::render_editorial`
//! (the narrative IS the editorial archetype: lede, drop-cap, epigraph,
//! invite-lists).

use cpe_core::lang::Lang;
use maud::html;

use crate::component::context::NarrativeStore;
use crate::component::editorial::{Editorial, FigureBlock, Rail};

/// Compute the resolved filename for a narrative `file` + `lang`.
///
/// Delegates to `util::variant_filename` (last-dot stem/ext split, shared
/// with `validate`'s variant probe): `Some(candidate)` builds
/// `"<stem>.<lang>.<ext>"` (e.g. `"home.de.md"`); `None` (no extension, no
/// variant form possible) falls back to `file` as-is. The caller decides
/// present-vs-fallback by looking the result up in the store.
///
/// Does NOT consult the store — pure string logic only.
pub(crate) fn resolve_variant<'a>(file: &'a str, lang: &Lang) -> std::borrow::Cow<'a, str> {
    match cpe_core::util::variant_filename(file, lang.as_str()) {
        Some(candidate) => std::borrow::Cow::Owned(candidate),
        None => std::borrow::Cow::Borrowed(file),
    }
}

/// Resolve the narrative content for `file` under a 3-step fallback chain:
/// 1. `<stem>.<requested>.<ext>` (e.g. `home.fr.md`)
/// 2. `<stem>.<default>.<ext>` (e.g. `home.en.md`) — skipped when `default == requested`
/// 3. the base `file` (e.g. `home.md`)
///
/// First hit wins. Returns `None` only if all three miss (the caller then
/// renders the empty-article fallback — unchanged behaviour).
///
/// Shared by `render_narrative` (body rendering) and `compose_page_title`
/// (title derivation) so the two can never resolve to different content.
///
/// Boot's `validate::check_page_heading_ids_unique` walks the same three
/// rungs over the raw narrative pairs, to compare the heading ids of the
/// content a visitor is served. A change to the chain here goes there too.
pub fn resolve_narrative_content<'a>(
    file: &str,
    requested: &Lang,
    default: &Lang,
    store: &'a NarrativeStore,
) -> Option<&'a str> {
    // Step 1: requested-language variant.
    let requested_candidate = resolve_variant(file, requested);
    if let Some(content) = store.get(requested_candidate.as_ref()) {
        return Some(content);
    }

    // Step 2: default-language variant (skip if identical to step 1 — already tried).
    if default.as_str() != requested.as_str() {
        let default_candidate = resolve_variant(file, default);
        if let Some(content) = store.get(default_candidate.as_ref()) {
            return Some(content);
        }
    }

    // Step 3: base file.
    store.get(file)
}

/// Render the narrative component for `file` in `lang`.
///
/// Variant resolution (3-step fallback chain, delegated to
/// `resolve_narrative_content`):
/// 1. `<stem>.<lang>.<ext>` (e.g. `"home.de.md"` for `file="home.md"`, `lang="de"`).
/// 2. `<stem>.<default_lang>.<ext>` — the site's default-language variant, skipped when
///    `default_lang == lang` (already tried in step 1).
/// 3. `file` (the base form).
///
/// If none of the three are present (should not happen after boot
/// validation), render an empty `<article class="editorial page">` — never
/// `.expect()`/panic.
///
/// The content is passed through the editorial Markdown transformer which
/// produces `article.editorial.page` with the editorial archetype structure
/// (header with h1 + lede, intro prose with drop-cap, invite-sections). This
/// applies to all narrative pages; the transformer falls back to a single
/// `div.prose` wrapper for documents that don't match the editorial shape.
///
/// `figures` are the figure blocks placed in this narrative's sections
/// (empty for every narrative no figure-block slot names). `id_offset` is
/// the number of top-level `##` headings in the page's narratives before
/// this one, and `rail` is the page rail, for the page's first narrative
/// only (see `editorial::render_editorial` for both).
pub fn render_narrative(
    file: &str,
    lang: &Lang,
    default_lang: &Lang,
    narrative: &NarrativeStore,
    figures: &[FigureBlock],
    id_offset: usize,
    rail: Option<&Rail<'_>>,
) -> Editorial {
    match resolve_narrative_content(file, lang, default_lang, narrative) {
        Some(src) => crate::component::editorial::render_editorial(src, figures, id_offset, rail),
        // All three rungs missed — an invariant violation after boot validation
        // (validation guarantees the base-or-default rung exists); emit an empty
        // article rather than panicking so the page still renders.
        None => Editorial {
            markup: html! {
                article.editorial.page {}
            },
            rail_placed: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use cpe_core::ir::SiteConfig;
    use cpe_core::lang::resolve;

    use super::*;
    use crate::component::context::NarrativeStore;

    // Helper: build a `Lang` from a language code against a known SiteConfig.
    fn make_lang(code: &str) -> Lang {
        let site = SiteConfig {
            title: "T".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned(), "fr".to_owned()],
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
        resolve(&[code.to_owned()], &site)
    }

    // Helper: build a NarrativeStore from (name, content) pairs.
    fn make_store(pairs: &[(&str, &str)]) -> NarrativeStore {
        let owned: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        NarrativeStore::from_pairs(&owned)
    }

    // ── resolve_variant tests ─────────────────────────────────────────────────

    #[test]
    fn variant_present_returns_candidate() {
        // "home.md" + lang "de" → candidate "home.de.md"
        let lang = make_lang("de");
        let store = make_store(&[("home.md", "en content"), ("home.de.md", "de content")]);
        // resolve_variant returns the candidate (caller decides present-vs-fallback)
        let candidate = resolve_variant("home.md", &lang);
        assert_eq!(candidate.as_ref(), "home.de.md");
        // render_narrative should use the de variant
        let out = render_narrative("home.md", &lang, &lang, &store, &[], 0, None)
            .markup
            .into_string();
        assert!(out.contains("de content"), "expected de content in output, got: {out}");
    }

    #[test]
    fn variant_absent_falls_back_to_base() {
        // "home.md" + lang "de" — store only has "home.md", no "home.de.md" → base
        let lang = make_lang("de");
        let store = make_store(&[("home.md", "base content")]);
        let out = render_narrative("home.md", &lang, &lang, &store, &[], 0, None)
            .markup
            .into_string();
        assert!(
            out.contains("base content"),
            "expected base content when variant absent, got: {out}"
        );
    }

    #[test]
    fn no_extension_uses_file_as_is() {
        // "readme" has no extension → resolve_variant returns "readme" unchanged
        let lang = make_lang("de");
        let candidate = resolve_variant("readme", &lang);
        assert_eq!(candidate.as_ref(), "readme");
        // render_narrative finds "readme" in the store
        let store = make_store(&[("readme", "readme content")]);
        let out = render_narrative("readme", &lang, &lang, &store, &[], 0, None)
            .markup
            .into_string();
        assert!(out.contains("readme content"), "expected readme content, got: {out}");
    }

    #[test]
    fn multi_dot_stem_resolved_correctly() {
        // "notes.intro.md" + lang "de" → candidate "notes.intro.de.md"
        // split_stem_ext splits at the LAST dot: stem="notes.intro", ext="md"
        let lang = make_lang("de");
        let candidate = resolve_variant("notes.intro.md", &lang);
        assert_eq!(candidate.as_ref(), "notes.intro.de.md");
    }

    // ── resolve_narrative_content tests ───────────────────────────────────────

    #[test]
    fn resolve_narrative_content_hits_requested_variant() {
        // requested="fr", default="en"; store has BOTH home.fr.md and home.en.md
        // (fr content differs from en) → the FR content wins (step 1
        // short-circuits, doesn't fall through to default).
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[("home.fr.md", "fr content"), ("home.en.md", "en content")]);
        let content = resolve_narrative_content("home.md", &requested, &default, &store);
        assert_eq!(
            content,
            Some("fr content"),
            "requested-language variant must win over the default variant"
        );
    }

    #[test]
    fn resolve_narrative_content_falls_back_to_default_variant() {
        // requested="fr", default="en"; store has ONLY home.en.md (no home.fr.md,
        // no bare home.md) → the EN content is returned. This is the
        // Alice-shaped case.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[("home.en.md", "en content")]);
        let content = resolve_narrative_content("home.md", &requested, &default, &store);
        assert_eq!(
            content,
            Some("en content"),
            "missing requested variant must fall back to the default-language variant"
        );
    }

    #[test]
    fn resolve_narrative_content_falls_back_to_base_file() {
        // requested="fr", default="en"; store has ONLY the bare home.md (no
        // variants at all) → the base content is returned.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[("home.md", "base content")]);
        let content = resolve_narrative_content("home.md", &requested, &default, &store);
        assert_eq!(
            content,
            Some("base content"),
            "missing both variants must fall back to the base file"
        );
    }

    #[test]
    fn resolve_narrative_content_all_absent_returns_none() {
        // requested="fr", default="en"; empty store → None.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[]);
        let content = resolve_narrative_content("home.md", &requested, &default, &store);
        assert_eq!(content, None, "all three rungs missing must return None");
    }

    #[test]
    fn resolve_narrative_content_default_equals_requested_still_resolves_correctly() {
        // requested="en", default="en" (same Lang); store has ONLY the bare
        // home.md → base content is returned, proving the default==requested
        // case (where step 2 is skipped as a redundant-probe optimization)
        // doesn't regress the chain's outcome. Note: this test cannot, by
        // itself, prove step 2 is actually SKIPPED rather than harmlessly
        // re-probed — `NarrativeStore::get` is a pure, side-effect-free
        // lookup, so "skip" vs "probe twice, same answer" are behaviorally
        // indistinguishable from the caller's side. It only guards against a
        // regression in the final resolved content for this input shape.
        let requested = make_lang("en");
        let default = make_lang("en");
        let store = make_store(&[("home.md", "base content")]);
        let content = resolve_narrative_content("home.md", &requested, &default, &store);
        assert_eq!(
            content,
            Some("base content"),
            "default==requested must still fall through to the base file"
        );
    }

    // ── render_narrative edge cases ───────────────────────────────────────────

    #[test]
    fn missing_file_renders_empty_article_editorial() {
        // Neither requested variant, default variant, nor base present (all
        // three rungs missed) → empty <article class="editorial page">. The
        // `.page` class is NOT optional on this branch: the shared layout
        // contract requires every component root to carry it on EVERY return
        // branch, empty states included (see `style/grid.css`'s LAYOUT
        // CONTRACT note). Uses
        // distinct requested/default langs to confirm the empty-article edge
        // case end-to-end through render_narrative, not just
        // resolve_narrative_content.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[]); // empty store
        let out = render_narrative("home.md", &requested, &default, &store, &[], 0, None)
            .markup
            .into_string();
        assert_eq!(
            out, r#"<article class="editorial page"></article>"#,
            "missing file should render empty article.editorial, got: {out}"
        );
    }

    #[test]
    fn render_narrative_wraps_in_editorial_article() {
        let lang = make_lang("en");
        let store = make_store(&[("home.md", "# Hello\n\nSome content.")]);
        let out = render_narrative("home.md", &lang, &lang, &store, &[], 0, None)
            .markup
            .into_string();
        assert!(
            out.contains(r#"class="editorial page""#),
            "output should be wrapped in article.editorial.page, got: {out}"
        );
        assert!(out.contains("<h1>Hello</h1>"), "heading should be rendered, got: {out}");
    }

    // ── Snapshots ──────────────────────────────────────────────────────────────

    #[test]
    fn snapshot_narrative_requested_variant_wins() {
        // requested="fr", default="en"; store has BOTH home.fr.md and home.en.md
        // with distinct content — the FR content must reach the rendered output.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[
            ("home.fr.md", "Contenu français distinct"),
            ("home.en.md", "Distinct English content"),
        ]);
        let out = render_narrative("home.md", &requested, &default, &store, &[], 0, None)
            .markup
            .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_falls_back_to_default_variant() {
        // requested="fr", default="en"; store has ONLY home.en.md — the
        // Alice-shaped fallback case.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[("home.en.md", "English only content")]);
        let out = render_narrative("home.md", &requested, &default, &store, &[], 0, None)
            .markup
            .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_all_rungs_missing_empty_article() {
        // requested="fr", default="en"; empty store — freezes the exact
        // empty-article fallback markup.
        let requested = make_lang("fr");
        let default = make_lang("en");
        let store = make_store(&[]);
        let out = render_narrative("home.md", &requested, &default, &store, &[], 0, None)
            .markup
            .into_string();
        insta::assert_snapshot!(out);
    }
}
