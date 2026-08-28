use std::collections::BTreeMap;

/// A multilingual value: IETF language tag -> text.
///
/// `BTreeMap` rather than `HashMap` so serialization is deterministic and
/// alphabetical by language tag. Two things depend on that: the canonical
/// project writer, whose byte-identical round-trip over the 85 committed files
/// would otherwise be flaky, and DPE's `/projects.json`, which emitted language
/// keys in `HashMap` iteration order and so differed between processes.
///
/// The tag is an open `String`, not a de/en/fr/it enum: `ar` is live in two
/// committed files, and a closed set would silently drop it.
pub type Multilingual = BTreeMap<String, String>;

/// Returns true if the value is a data placeholder ("MISSING" or "CALCULATED").
pub fn is_placeholder(value: &str) -> bool {
    value == "MISSING" || value == "CALCULATED"
}

/// Extracts a value from a multilingual map, preferring English.
///
/// When `en` is absent the entry with the lexicographically smallest language
/// code is chosen. The choice is explicit rather than incidental: this value is
/// used as a lookup key, notably for the temporal-coverage enrichment table,
/// where collection and lookup must agree, so it must not depend on which tags
/// happen to be present or on how the map was built.
///
/// Distinct from `dpe_core::utils::lang_value`: that one prioritizes a fixed
/// language order (en -> de -> fr -> it) with a non-deterministic fallback,
/// which is fine for display but unsafe as a lookup key.
pub fn multilingual_value(map: &Multilingual) -> Option<String> {
    map.get("en")
        .or_else(|| map.iter().min_by(|(a, _), (b, _)| a.cmp(b)).map(|(_, v)| v))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_placeholder_missing() {
        assert!(is_placeholder("MISSING"));
    }

    #[test]
    fn test_is_placeholder_calculated() {
        assert!(is_placeholder("CALCULATED"));
    }

    #[test]
    fn test_is_placeholder_normal_values() {
        assert!(!is_placeholder("CC BY-SA 4.0"));
        assert!(!is_placeholder("2021-09-02"));
        assert!(!is_placeholder("person-001"));
        assert!(!is_placeholder(""));
        assert!(!is_placeholder("missing")); // case-sensitive
        assert!(!is_placeholder("calculated")); // case-sensitive
    }

    #[test]
    fn multilingual_value_prefers_english() {
        let map = Multilingual::from([
            ("de".to_string(), "Hallo".to_string()),
            ("en".to_string(), "Hello".to_string()),
        ]);
        assert_eq!(multilingual_value(&map).as_deref(), Some("Hello"));
    }

    #[test]
    fn multilingual_value_falls_back_to_lexicographically_smallest_key() {
        let map = Multilingual::from([
            ("it".to_string(), "Ciao".to_string()),
            ("fr".to_string(), "Bonjour".to_string()),
        ]);
        // No "en" entry: "fr" sorts before "it", regardless of insertion order.
        assert_eq!(multilingual_value(&map).as_deref(), Some("Bonjour"));
    }

    #[test]
    fn multilingual_value_empty_map_is_none() {
        assert_eq!(multilingual_value(&Multilingual::new()), None);
    }
}
