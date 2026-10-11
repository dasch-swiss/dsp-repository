use crate::ir::SiteConfig;

/// The four built-in engine languages, in canonical order. Documents the
/// chrome catalog's coverage in one place; **completeness is
/// compile-time-guaranteed only across these four**. A project MAY declare a
/// further language (`docs/src/cpe/config-language.md` permits it if its own
/// config-chrome completeness holds); such a page gets English engine chrome via the
/// catalog's `L4::pick` `_ => en` arm, which is not "complete" in the sense the catalog
/// guarantees.
pub const BUILTIN_LANGUAGES: [&str; 4] = ["en", "de", "fr", "it"];

/// A resolved, valid language code.
///
/// Built by `resolve` (against a specific project's declared
/// `site.languages`, falling back to `site.default_language`) or by
/// `resolve_builtin` (against the engine's own [`BUILTIN_LANGUAGES`], no
/// `SiteConfig` involved — used by the host's site index, which has no
/// per-project language declaration to resolve against). Callers should not
/// construct this type directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lang(String);

impl Lang {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Shared preference-list reconciliation, factored out so `resolve` and
/// `resolve_builtin` can never drift: return the first `preferences` entry
/// present in `candidates` (plain equality — callers are responsible for
/// lowercasing and stripping region subtags before calling), or `default`
/// when none match.
fn resolve_against(preferences: &[String], candidates: &[&str], default: &str) -> Lang {
    for pref in preferences {
        if candidates.contains(&pref.as_str()) {
            return Lang(pref.clone());
        }
    }
    Lang(default.to_owned())
}

/// Resolve an ordered preference list against a project's declared languages.
///
/// Returns the first `preferences` entry that matches one of
/// `site.languages` (plain equality — the host's middleware is responsible
/// for lowercasing and stripping region subtags before calling this
/// function). Falls back to `site.default_language` when no preference
/// matches.
pub fn resolve(preferences: &[String], site: &SiteConfig) -> Lang {
    let candidates: Vec<&str> = site.languages.iter().map(String::as_str).collect();
    resolve_against(preferences, &candidates, &site.default_language)
}

/// Resolve an ordered preference list against the engine's built-in chrome
/// languages ([`BUILTIN_LANGUAGES`]), with no project `SiteConfig` involved.
/// Falls back to `"en"` when no preference matches.
///
/// This is what lets the host's site index (which has no project to resolve
/// a language against) obtain a [`Lang`] and reuse the `pub` engine chrome catalog
/// (`chrome` in `cpe-components`) directly, rather than mirroring it
/// in the host.
pub fn resolve_builtin(preferences: &[String]) -> Lang {
    resolve_against(preferences, &BUILTIN_LANGUAGES, "en")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::SiteConfig;

    fn make_site() -> SiteConfig {
        SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: crate::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        }
    }

    #[test]
    fn resolves_declared_language() {
        let site = make_site();
        let lang = resolve(&["de".to_owned()], &site);
        assert_eq!(lang.as_str(), "de");
    }

    #[test]
    fn falls_back_to_default_when_undeclared() {
        let site = make_site();
        // "fr" is not in site.languages → default "en"
        let lang = resolve(&["fr".to_owned()], &site);
        assert_eq!(lang.as_str(), "en");
    }

    #[test]
    fn resolves_en_exactly() {
        let site = make_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert_eq!(lang.as_str(), "en");
    }

    /// "en-us" does not match "en" here because `resolve` uses plain equality.
    /// Region-stripping is the host middleware's responsibility (tested
    /// in the host), so after stripping "en-US" → "en" before calling
    /// `resolve`.
    #[test]
    fn does_not_match_regional_subtag() {
        let site = make_site();
        // "en-us" ≠ "en" — no match → falls back to default "en"
        let lang = resolve(&["en-us".to_owned()], &site);
        assert_eq!(lang.as_str(), "en");
    }

    #[test]
    fn empty_preferences_returns_default() {
        let site = make_site();
        let lang = resolve(&[], &site);
        assert_eq!(lang.as_str(), "en");
    }

    // ── resolve_builtin ───────────────────────────────────────────────────────

    #[test]
    fn resolve_builtin_picks_a_declared_builtin_language() {
        let lang = resolve_builtin(&["de".to_owned()]);
        assert_eq!(lang.as_str(), "de");
        let lang = resolve_builtin(&["fr".to_owned()]);
        assert_eq!(lang.as_str(), "fr");
        let lang = resolve_builtin(&["it".to_owned()]);
        assert_eq!(lang.as_str(), "it");
    }

    #[test]
    fn resolve_builtin_falls_back_to_en_for_unknown_code() {
        let lang = resolve_builtin(&["zz".to_owned()]);
        assert_eq!(lang.as_str(), "en");
    }

    #[test]
    fn resolve_builtin_prefers_first_matching_preference() {
        let lang = resolve_builtin(&["zz".to_owned(), "fr".to_owned(), "de".to_owned()]);
        assert_eq!(lang.as_str(), "fr");
    }

    #[test]
    fn resolve_builtin_empty_preferences_returns_en() {
        let lang = resolve_builtin(&[]);
        assert_eq!(lang.as_str(), "en");
    }
}
