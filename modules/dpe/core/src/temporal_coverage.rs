//! Resolution of a `temporalCoverage` entry to a W3CDTF date range.
//!
//! Shared by `dpe-api-oai` (DataCite `Coverage` dates) and `dpe-server`'s
//! `validate` command, over the same ChronOntology period cache and offline
//! enrichment table, so the two can never disagree about what counts as
//! "resolved".

use std::collections::HashMap;

use super::chronontology_cache;
use super::project::TemporalCoverage;
use super::temporal_enrichment_cache::{self, EnrichedDate};
use super::utils::multilingual_value;
use super::w3cdtf::W3cdtfRange;

/// The outcome of resolving one `temporalCoverage` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// W3CDTF date range, or empty when only a name could be carried (a
    /// name-only fallback, e.g. DataCite's `dateInformation`-only case).
    pub date: String,
    /// The display name backing the entry (e.g. DataCite's `dateInformation`).
    pub date_information: Option<String>,
}

impl Resolution {
    /// Whether resolution produced a machine-readable date, as opposed to a
    /// name-only fallback.
    pub fn has_date(&self) -> bool {
        !self.date.is_empty()
    }
}

/// The display name / enrichment lookup key for a `temporalCoverage` entry: a
/// Reference's authority-file label, or a Text map's preferred-language value.
pub fn coverage_name(tc: &TemporalCoverage) -> Option<String> {
    match tc {
        TemporalCoverage::Reference(ref_data) => ref_data.text.clone(),
        TemporalCoverage::Text(text_map) => multilingual_value(text_map),
    }
}

/// Resolve one `temporalCoverage` entry over the given period and enrichment
/// tables (pure, for testability without the process-global caches).
///
/// Resolution order:
/// 1. A ChronOntology reference URL -> its timespan range (`periods`).
/// 2. Otherwise (or on a cache miss) the offline enrichment table (`enrichment`), keyed by the same
///    name `coverage_name` computes.
/// 3. Otherwise a name-only outcome (empty `date`), so the original information is never silently
///    dropped.
///
/// Returns `None` only when there is neither a resolvable range nor any name
/// to carry — nothing to report.
pub fn resolve_in(
    tc: &TemporalCoverage,
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
) -> Option<Resolution> {
    let name = coverage_name(tc);

    // 1. ChronOntology URL -> timespan.
    if let TemporalCoverage::Reference(ref_data) = tc {
        if !ref_data.url.is_empty() {
            if let Some(range) = chronontology_cache::timespan_for_in(periods, &ref_data.url) {
                return Some(Resolution { date: range.into(), date_information: name });
            }
        }
    }

    // 2. Enrichment table, keyed by the display name.
    if let Some(ref key) = name {
        if let Some(enriched) = temporal_enrichment_cache::enriched_for_in(enrichment, key) {
            return Some(Resolution {
                date: enriched.date.unwrap_or_default(),
                date_information: Some(enriched.original_name),
            });
        }
    }

    // 3. Name-only fallback.
    name.map(|n| Resolution { date: String::new(), date_information: Some(n) })
}

/// Whether an unresolved (empty-date) `temporalCoverage` entry is an
/// intentionally reviewed non-period label rather than a genuine gap: an
/// enrichment row with no `date` and `source == "unresolved"` (e.g. "Swiss",
/// "English (culture or style)").
pub fn is_intentionally_unresolved(name: &str, enrichment: &HashMap<String, EnrichedDate>) -> bool {
    enrichment
        .get(name)
        .is_some_and(|e| e.date.is_none() && e.source == "unresolved")
}

/// Whether one `temporalCoverage` entry is a genuine completeness gap: it has
/// a name to key on, resolves to no machine-readable date, and is not
/// explicitly reviewed as a non-period label. Returns the name (for
/// reporting) when it is a gap, `None` otherwise (resolved, intentionally
/// unresolved, or nameless).
///
/// The single decision both `dpe-server validate` and the
/// `every_committed_temporal_coverage_resolves` completeness test apply per
/// entry, so the two enforcement points can't drift apart on what counts as a
/// gap even though each walks its own project data independently.
pub fn completeness_gap(
    tc: &TemporalCoverage,
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
) -> Option<String> {
    let name = coverage_name(tc)?;
    let resolved = resolve_in(tc, periods, enrichment);
    if resolved.as_ref().is_some_and(Resolution::has_date) {
        return None;
    }
    if is_intentionally_unresolved(&name, enrichment) {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AuthorityFileReference;

    fn reference(url: &str, text: Option<&str>) -> TemporalCoverage {
        TemporalCoverage::Reference(AuthorityFileReference {
            type_: "Chronontology".to_string(),
            url: url.to_string(),
            text: text.map(str::to_string),
        })
    }

    fn text(en: &str) -> TemporalCoverage {
        let mut map = HashMap::new();
        map.insert("en".to_string(), en.to_string());
        TemporalCoverage::Text(map)
    }

    fn periods() -> HashMap<String, W3cdtfRange> {
        let mut map = HashMap::new();
        map.insert(
            "0vGXxVln724L".to_string(),
            crate::w3cdtf::to_w3cdtf_range(Some("98"), Some("117")).unwrap(),
        );
        map
    }

    fn enrichment(entries: &[(&str, Option<&str>, &str, &str)]) -> HashMap<String, EnrichedDate> {
        entries
            .iter()
            .map(|(key, date, name, source)| {
                (
                    key.to_string(),
                    EnrichedDate {
                        date: date.map(str::to_string),
                        original_name: name.to_string(),
                        source: source.to_string(),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn chronontology_url_resolves_to_range() {
        let tc = reference("https://chronontology.dainst.org/period/0vGXxVln724L", Some("Trajanic"));
        let resolution = resolve_in(&tc, &periods(), &HashMap::new()).unwrap();
        assert_eq!(resolution.date, "0098/0117");
        assert_eq!(resolution.date_information.as_deref(), Some("Trajanic"));
    }

    #[test]
    fn free_text_resolves_via_enrichment() {
        let tc = text("Early Christianity");
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "Early Christianity", "llm")]);
        let resolution = resolve_in(&tc, &HashMap::new(), &enrich).unwrap();
        assert_eq!(resolution.date, "0030/0451");
    }

    #[test]
    fn unresolved_emits_name_only_empty_date() {
        let tc = text("Mysterious Era");
        let resolution = resolve_in(&tc, &periods(), &HashMap::new()).unwrap();
        assert!(!resolution.has_date());
        assert_eq!(resolution.date_information.as_deref(), Some("Mysterious Era"));
    }

    #[test]
    fn no_name_and_no_resolution_is_none() {
        let tc = reference("", None);
        assert!(resolve_in(&tc, &periods(), &HashMap::new()).is_none());
    }

    #[test]
    fn intentionally_unresolved_label_is_recognised() {
        let enrich = enrichment(&[("Swiss", None, "Swiss", "unresolved")]);
        assert!(is_intentionally_unresolved("Swiss", &enrich));
        assert!(!is_intentionally_unresolved("Unknown", &enrich));
    }

    #[test]
    fn completeness_gap_flags_genuinely_unresolved_name() {
        let tc = text("Mysterious Era");
        assert_eq!(
            completeness_gap(&tc, &HashMap::new(), &HashMap::new()).as_deref(),
            Some("Mysterious Era")
        );
    }

    #[test]
    fn completeness_gap_is_none_when_resolved() {
        let tc = text("Early Christianity");
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "Early Christianity", "llm")]);
        assert_eq!(completeness_gap(&tc, &HashMap::new(), &enrich), None);
    }

    #[test]
    fn completeness_gap_is_none_when_intentionally_unresolved() {
        let tc = text("Swiss");
        let enrich = enrichment(&[("Swiss", None, "Swiss", "unresolved")]);
        assert_eq!(completeness_gap(&tc, &HashMap::new(), &enrich), None);
    }

    #[test]
    fn completeness_gap_is_none_when_nameless() {
        let tc = reference("", None);
        assert_eq!(completeness_gap(&tc, &HashMap::new(), &HashMap::new()), None);
    }
}
