//! In-process cache of offline-enriched temporal-coverage date ranges.
//!
//! Some `temporalCoverage` entries cannot be resolved through ChronOntology
//! (free-text names, or periods without a timespan). An offline tool produces
//! `temporal-coverage-enrichment.json`, a reviewed lookup table mapping a
//! normalized name to a W3CDTF date range, the original name, and the source of
//! the range (`chronontology` or `llm`). This module loads that table at runtime.
//!
//! The table is keyed by the same value the OAI mapping computes at request time
//! (the preferred-language / deterministic multilingual value), so collection and
//! lookup agree.
//!
//! Loaded from disk once on first access, like the other `*_cache` modules.
//!
//! Note: This module is gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::utils::get_data_dir;

const ENRICHMENT_FILE: &str = "temporal-coverage-enrichment.json";

static ENRICHMENT: OnceLock<HashMap<String, EnrichedDate>> = OnceLock::new();

/// One enriched temporal-coverage entry.
#[derive(Clone, Debug, Deserialize)]
pub struct EnrichedDate {
    /// W3CDTF date range, or `None` when the name is known but no range could be
    /// determined (the entry is then emitted with `dateInformation` only).
    #[serde(default)]
    pub date: Option<String>,
    /// The original human-readable period name.
    pub original_name: String,
    /// Provenance of the range: `"chronontology"` or `"llm"`.
    #[serde(default)]
    pub source: String,
}

fn all_enriched() -> &'static HashMap<String, EnrichedDate> {
    ENRICHMENT.get_or_init(load_all_enriched)
}

/// Look up an enriched entry in `entries` by its normalized key. Pure over the
/// given map so it can be unit-tested without the process cache.
pub fn enriched_for_in(entries: &HashMap<String, EnrichedDate>, key: &str) -> Option<EnrichedDate> {
    entries.get(key).cloned()
}

/// Look up an enriched entry (from the cache) by its normalized key.
pub fn enriched_for(key: &str) -> Option<EnrichedDate> {
    enriched_for_in(all_enriched(), key)
}

fn load_all_enriched() -> HashMap<String, EnrichedDate> {
    use std::fs;
    use std::path::PathBuf;

    let path = PathBuf::from(get_data_dir()).join(ENRICHMENT_FILE);

    // A missing enrichment file is normal (e.g. before the tool is first run);
    // callers fall back to a name-only date. Only a present-but-broken file warns.
    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(_) => return HashMap::new(),
    };

    match serde_json::from_str(&json) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(file = ?path, error = %e, "failed to parse temporal coverage enrichment file");
            HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(date: Option<&str>, name: &str, source: &str) -> EnrichedDate {
        EnrichedDate {
            date: date.map(str::to_string),
            original_name: name.to_string(),
            source: source.to_string(),
        }
    }

    fn fixtures() -> HashMap<String, EnrichedDate> {
        let mut map = HashMap::new();
        map.insert(
            "Early Christianity".to_string(),
            entry(Some("0030/0451"), "Early Christianity", "llm"),
        );
        map.insert("Mysterious Era".to_string(), entry(None, "Mysterious Era", "llm"));
        map
    }

    #[test]
    fn returns_entry_with_range() {
        let entries = fixtures();
        let got = enriched_for_in(&entries, "Early Christianity").unwrap();
        assert_eq!(got.date.as_deref(), Some("0030/0451"));
        assert_eq!(got.original_name, "Early Christianity");
    }

    #[test]
    fn returns_entry_without_range() {
        let entries = fixtures();
        let got = enriched_for_in(&entries, "Mysterious Era").unwrap();
        assert_eq!(got.date, None);
        assert_eq!(got.original_name, "Mysterious Era");
    }

    #[test]
    fn unknown_key_is_none() {
        let entries = fixtures();
        assert!(enriched_for_in(&entries, "Unknown").is_none());
    }
}
