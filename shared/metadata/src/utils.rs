use std::collections::BTreeMap;

use serde_json::Value;

use crate::models::AuthorityFileReference;

/// A multilingual value: IETF language tag -> text.
///
/// `BTreeMap` rather than `HashMap` so serialization is deterministic and
/// alphabetical by language tag. Two things depend on that: the canonical
/// project writer's byte-identical round-trip, and DPE's `/projects.json`, whose
/// language keys would otherwise differ between processes.
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

fn make_ref(url: String) -> AuthorityFileReference {
    AuthorityFileReference { type_: "URL".to_string(), url, text: None }
}

/// Parses the `"url"` JSON value — either a structured object (new format)
/// or a legacy string array — into primary and secondary references.
pub fn parse_url_value(value: Option<Value>) -> (Option<AuthorityFileReference>, Option<AuthorityFileReference>) {
    match value {
        Some(Value::Object(_)) => {
            let reference = serde_json::from_value::<AuthorityFileReference>(value.unwrap())
                .ok()
                .filter(|r| !is_placeholder(&r.url));
            (reference, None)
        }
        Some(Value::Array(arr)) => {
            // Placeholders ("MISSING"/"CALCULATED") signal "no URL yet" and must not
            // become live links — e.g. a "Discover Project Data" button to nowhere.
            let mut strings = arr
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .filter(|s| !is_placeholder(s));
            (strings.next().map(make_ref), strings.next().map(make_ref))
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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

    #[test]
    fn array_placeholder_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(Some(json!(["MISSING"])));
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    #[test]
    fn array_real_url_yields_primary_reference() {
        let (primary, secondary) = parse_url_value(Some(json!(["https://example.org/data"])));
        assert_eq!(primary.unwrap().url, "https://example.org/data");
        assert!(secondary.is_none());
    }

    #[test]
    fn array_filters_placeholders_keeps_real_urls() {
        // A placeholder primary must not shift a real URL into the primary slot
        // incorrectly, nor become a link itself.
        let (primary, secondary) = parse_url_value(Some(json!(["MISSING", "https://example.org/site"])));
        assert_eq!(primary.unwrap().url, "https://example.org/site");
        assert!(secondary.is_none());
    }

    #[test]
    fn array_calculated_placeholder_is_filtered() {
        let (primary, _) = parse_url_value(Some(json!(["CALCULATED"])));
        assert!(primary.is_none());
    }

    #[test]
    fn object_placeholder_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(Some(json!({"type": "URL", "url": "MISSING"})));
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    #[test]
    fn object_real_url_yields_primary_reference() {
        let (primary, _) = parse_url_value(Some(json!({"type": "URL", "url": "https://example.org/data"})));
        assert_eq!(primary.unwrap().url, "https://example.org/data");
    }

    #[test]
    fn missing_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(None);
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }
}
