//! The checks a draft must pass to become a pending submission.
//!
//! Today that is one rule beyond the type-level gate in
//! [`ProjectDraft::to_raw`]: every `temporalCoverage` entry has to resolve to a
//! structured date (REQ-1.14). `dpe-server validate` does not enforce it as a
//! blocker, and OAI-PMH needs it, so a submission carrying an unresolvable
//! period would open a pull request that fails CI in a crate the editor never
//! touches.
//!
//! REQ-1.15 asked whether to refuse such a submission or carry the enrichment
//! row through to the pull request. Refusal, with a field-level error: Success
//! Criterion 2 says so, and the depositor is not stranded, because
//! `temporalCoverage`'s `Reference` variant is always a resolvable path for
//! recording a period the enrichment table does not know.
//!
//! The rule is [`platform_metadata::temporal_coverage::completeness_gap`], the
//! same decision `dpe-server validate` and `dpe-api-oai`'s
//! `every_committed_temporal_coverage_resolves` apply, so the three cannot drift
//! on what counts as a gap. Nothing needed extracting: that function was already
//! pure over the two tables. What this module adds is the entry index, which a
//! command-line report does not need and a form field does.
//!
//! Turning these into rendered errors is the submit path's job (DEV-6913).
//!
//! [`ProjectDraft::to_raw`]: crate::draft::ProjectDraft::to_raw

use std::collections::HashMap;

use platform_metadata::project::ProjectRaw;
use platform_metadata::temporal_coverage;
use platform_metadata::temporal_enrichment::EnrichedDate;
use platform_metadata::w3cdtf::W3cdtfRange;

/// A `temporalCoverage` entry that cannot resolve to a structured date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedCoverage {
    /// Index into `temporalCoverage`, so the form can mark the offending row
    /// rather than the whole field.
    pub index: usize,
    /// The display name the entry resolves under, and the key an enrichment row
    /// would have to use.
    pub name: String,
}

impl UnresolvedCoverage {
    /// The field path a per-field error is keyed by.
    #[must_use]
    pub fn field_path(&self) -> String {
        format!("temporalCoverage[{}]", self.index)
    }
}

/// Every `temporalCoverage` entry in `project` that has a name but no resolvable
/// date, and is not an enrichment row explicitly reviewed as a non-period label.
///
/// Empty means the draft passes this rule. Entries are returned in field order,
/// and duplicates are kept: two rows with the same unresolvable name are two
/// rows the depositor has to fix.
#[must_use]
pub fn unresolved_temporal_coverage(
    project: &ProjectRaw,
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
) -> Vec<UnresolvedCoverage> {
    project
        .temporal_coverage
        .iter()
        .enumerate()
        .filter_map(|(index, coverage)| {
            temporal_coverage::completeness_gap(coverage, periods, enrichment)
                .map(|name| UnresolvedCoverage { index, name })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use platform_metadata::project::TemporalCoverage;
    use platform_metadata::utils::Multilingual;
    use platform_metadata::AuthorityFileReference;

    use super::*;
    use crate::test_support::sample_raw;

    fn project_with(coverage: Vec<TemporalCoverage>) -> ProjectRaw {
        let mut project = sample_raw();
        project.temporal_coverage = coverage;
        project
    }

    fn free_text(name: &str) -> TemporalCoverage {
        TemporalCoverage::Text(Multilingual::from([("en".to_string(), name.to_string())]))
    }

    fn reference(url: &str, text: &str) -> TemporalCoverage {
        TemporalCoverage::Reference(AuthorityFileReference {
            type_: "Chronontology".to_string(),
            url: url.to_string(),
            text: Some(text.to_string()),
        })
    }

    fn enrichment(rows: &[(&str, Option<&str>, &str)]) -> HashMap<String, EnrichedDate> {
        rows.iter()
            .map(|(key, date, source)| {
                (
                    (*key).to_string(),
                    EnrichedDate {
                        date: date.map(str::to_string),
                        original_name: (*key).to_string(),
                        source: (*source).to_string(),
                    },
                )
            })
            .collect()
    }

    fn periods() -> HashMap<String, W3cdtfRange> {
        HashMap::from([(
            "0vGXxVln724L".to_string(),
            platform_metadata::w3cdtf::to_w3cdtf_range(Some("98"), Some("117")).expect("a range"),
        )])
    }

    /// Success Criterion 2: a submission whose `temporalCoverage` cannot resolve
    /// is rejected.
    #[test]
    fn an_unresolvable_free_text_period_is_reported() {
        let project = project_with(vec![free_text("A period nobody has enriched")]);
        let unresolved = unresolved_temporal_coverage(&project, &periods(), &HashMap::new());
        assert_eq!(
            unresolved,
            [UnresolvedCoverage { index: 0, name: "A period nobody has enriched".to_string() }]
        );
        assert_eq!(unresolved[0].field_path(), "temporalCoverage[0]");
    }

    #[test]
    fn a_period_resolved_through_the_enrichment_table_passes() {
        let project = project_with(vec![free_text("Early Christianity")]);
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "llm")]);
        assert!(unresolved_temporal_coverage(&project, &HashMap::new(), &enrich).is_empty());
    }

    /// The escape route the refusal decision relies on: a ChronOntology
    /// reference always resolves, so a depositor is never stuck with a period
    /// the enrichment table does not know.
    #[test]
    fn a_chronontology_reference_passes() {
        let project = project_with(vec![reference(
            "https://chronontology.dainst.org/period/0vGXxVln724L",
            "Trajanic",
        )]);
        assert!(unresolved_temporal_coverage(&project, &periods(), &HashMap::new()).is_empty());
    }

    /// A reviewed non-period label ("Swiss") is not a gap, so the editor must
    /// not refuse it either.
    #[test]
    fn an_intentionally_unresolved_label_passes() {
        let project = project_with(vec![free_text("Swiss")]);
        let enrich = enrichment(&[("Swiss", None, "unresolved")]);
        assert!(unresolved_temporal_coverage(&project, &HashMap::new(), &enrich).is_empty());
    }

    #[test]
    fn no_temporal_coverage_at_all_passes() {
        let project = project_with(vec![]);
        assert!(unresolved_temporal_coverage(&project, &periods(), &HashMap::new()).is_empty());
    }

    /// The index is what makes this a field-level error rather than a
    /// whole-field one, so a resolvable entry must not shift the reported index
    /// of an unresolvable one.
    #[test]
    fn the_reported_index_is_the_entry_position() {
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "llm")]);
        let project = project_with(vec![
            free_text("Early Christianity"),
            free_text("Mysterious Era"),
            free_text("Another Mystery"),
        ]);
        let unresolved = unresolved_temporal_coverage(&project, &HashMap::new(), &enrich);
        assert_eq!(unresolved.iter().map(|u| u.index).collect::<Vec<_>>(), [1, 2], "{unresolved:?}");
    }

    /// Two rows carrying the same unresolvable name are two rows to fix. The
    /// command-line report deduplicates by name; a form cannot.
    #[test]
    fn duplicate_unresolvable_names_are_reported_once_per_entry() {
        let project = project_with(vec![free_text("Mysterious Era"), free_text("Mysterious Era")]);
        let unresolved = unresolved_temporal_coverage(&project, &HashMap::new(), &HashMap::new());
        assert_eq!(unresolved.len(), 2);
        assert_eq!(unresolved[1].field_path(), "temporalCoverage[1]");
    }
}
