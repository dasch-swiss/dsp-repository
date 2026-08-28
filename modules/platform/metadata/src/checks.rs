//! The rules `dpe-server validate` applies to a single project, callable.
//!
//! `validate` grew these rules inline in a binary crate, where nothing could
//! depend on them, and reported every result as a string keyed by the file it
//! came from. The editor validates the same project against the same rules but
//! renders per field, so it needs to know *which member* a finding is about, not
//! which file.
//!
//! ## What is here and what is not
//!
//! Only the rules that need nothing but one project. `validate`'s other checks
//! are corpus-level — records parse, the data directories exist — and stay in
//! `dpe-server`, which is the thing that walks a corpus.
//!
//! The rule that a project's JSON parses as [`ProjectRaw`] is not here either,
//! and cannot be: a checker taking `&ProjectRaw` is downstream of it. It stays
//! with whoever does the parsing. That is also the honest place for it — serde's
//! `Path` for an `#[serde(untagged)]` enum names the last variant it tried, not
//! the one the data meant, so a malformed `funding` reports against
//! `Funding::Text` regardless and the path would be a lie.
//!
//! ## Why a finding is not a path string
//!
//! A finding names a member and, for a repeatable one, an ordinal:
//! `("temporalCoverage", Some(2))` rather than `"temporalCoverage[2]"`. The
//! ordinal is a position in document order, which is all this module can
//! honestly know. It is deliberately not an identity: the editor's form keys
//! repeatable rows by an opaque server-generated id precisely because an index
//! shifts when a row is removed, so baking `[2]` into a path string would hand
//! the form an address it cannot use. The form builds its rows from the same
//! ordered project, so it can map ordinal to row key itself.
//!
//! ## Why the corpus half of the contributor rule is inverted
//!
//! `validate` checks that every contributor id resolves to a person or
//! organization on disk. The editor applies the same rule against the published
//! corpus *plus* entity proposals that exist only in its database. So this
//! module reports which ids a project references and where
//! ([`contributor_refs`]), and leaves "is this id known" to the caller, which is
//! the only party that knows what its corpus is. Splitting there keeps the
//! per-project half shared without this crate learning about either caller's
//! storage.

use std::collections::HashMap;

use crate::project::ProjectRaw;
use crate::temporal_coverage;
use crate::temporal_enrichment::EnrichedDate;
use crate::w3cdtf::W3cdtfRange;

/// Something wrong with one member of a project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The `ProjectRaw` member, spelled as it is in JSON (`"temporalCoverage"`).
    pub field: &'static str,
    /// Position within a repeatable member, in document order; `None` for a
    /// scalar one. Not an identity — see the module docs.
    pub index: Option<usize>,
    /// The offending value, where the finding is about one. Carried separately
    /// from `message` so a caller can group or de-duplicate on it without
    /// parsing prose back out of a formatted string.
    pub value: Option<String>,
    /// What is wrong, with no file, project or field prefix — a caller that
    /// reports per file prepends its own.
    pub message: String,
}

/// A contributor id a project references, and the member it came from.
///
/// Whether the id resolves is the caller's question; see the module docs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorRef {
    /// `"attributions"` or `"contactPoint"`.
    pub field: &'static str,
    /// Position within that member, in document order.
    pub index: usize,
    pub id: String,
}

/// Every contributor id `raw` references, in document order: its `attributions`
/// first, then its `contactPoint`.
///
/// Duplicates are kept. A project naming the same id twice references it twice,
/// and a caller reporting per reference would otherwise lose one.
pub fn contributor_refs(raw: &ProjectRaw) -> Vec<ContributorRef> {
    let attributions = raw.attributions.iter().enumerate().map(|(index, attribution)| ContributorRef {
        field: "attributions",
        index,
        id: attribution.contributor.clone(),
    });

    let contact_points = raw
        .contact_point
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, id)| ContributorRef { field: "contactPoint", index, id: id.clone() });

    attributions.chain(contact_points).collect()
}

/// Every per-project rule applied to `raw`, as findings in document order.
///
/// Today that is one rule: a `temporalCoverage` entry which is a genuine
/// completeness gap. The gap decision itself is
/// [`temporal_coverage::completeness_gap`] — the same one the OAI-PMH
/// `every_committed_temporal_coverage_resolves` test applies, so the two
/// enforcement points cannot drift apart.
///
/// The contributor rule is deliberately not here: it needs corpus knowledge this
/// crate does not have, and its per-project half is [`contributor_refs`].
pub fn check_project(
    raw: &ProjectRaw,
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
) -> Vec<Finding> {
    raw.temporal_coverage
        .iter()
        .enumerate()
        .filter_map(|(index, tc)| {
            let name = temporal_coverage::completeness_gap(tc, periods, enrichment)?;
            Some(Finding {
                field: "temporalCoverage",
                index: Some(index),
                message: format!(
                    "temporalCoverage '{name}' has no resolved date \
                     (add a W3CDTF range to temporal-coverage-enrichment.json, \
                     or mark source=\"unresolved\" if not a time period)"
                ),
                value: Some(name),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal_enrichment::EnrichedDate;

    /// A project whose `temporalCoverage` and `attributions` the caller supplies
    /// as raw JSON, so a test states only what it is about.
    fn project(temporal_coverage: &str, attributions: &str, contact_point: &str) -> ProjectRaw {
        let json = format!(
            r#"{{
                "id": "0000", "pid": "MISSING", "name": "Test Project", "shortcode": "0000",
                "officialName": "Test Project", "status": "Finished", "shortDescription": "test",
                "description": {{}}, "startDate": "MISSING", "endDate": "MISSING",
                "howToCite": "test", "accessRights": {{ "accessRights": "Full Open Access" }},
                "legalInfo": [], "keywords": [], "disciplines": [],
                "temporalCoverage": {temporal_coverage}, "spatialCoverage": [],
                "attributions": {attributions}, "contactPoint": {contact_point},
                "funding": "No funding"
            }}"#
        );
        serde_json::from_str(&json).expect("fixture must parse")
    }

    fn enrichment(entries: &[(&str, Option<&str>, &str)]) -> HashMap<String, EnrichedDate> {
        entries
            .iter()
            .map(|(key, date, source)| {
                (
                    key.to_string(),
                    EnrichedDate {
                        date: date.map(str::to_string),
                        original_name: key.to_string(),
                        source: source.to_string(),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn unresolved_temporal_coverage_is_a_finding_against_its_ordinal() {
        let raw = project(r#"[{"en": "Early Christianity"}, {"en": "Mysterious Era"}]"#, "[]", "[]");
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "llm")]);
        assert_eq!(
            check_project(&raw, &HashMap::new(), &enrich),
            vec![Finding {
                field: "temporalCoverage",
                // The second entry, not the first — the ordinal is the position
                // in the project, not a count of findings.
                index: Some(1),
                value: Some("Mysterious Era".to_string()),
                message: "temporalCoverage 'Mysterious Era' has no resolved date \
                          (add a W3CDTF range to temporal-coverage-enrichment.json, \
                          or mark source=\"unresolved\" if not a time period)"
                    .to_string(),
            }]
        );
    }

    #[test]
    fn a_resolved_temporal_coverage_is_no_finding() {
        let raw = project(r#"[{"en": "Early Christianity"}]"#, "[]", "[]");
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "llm")]);
        assert!(check_project(&raw, &HashMap::new(), &enrich).is_empty());
    }

    #[test]
    fn an_intentionally_unresolved_label_is_no_finding() {
        let raw = project(r#"[{"en": "Swiss"}]"#, "[]", "[]");
        let enrich = enrichment(&[("Swiss", None, "unresolved")]);
        assert!(check_project(&raw, &HashMap::new(), &enrich).is_empty());
    }

    #[test]
    fn contributor_refs_are_attributions_then_contact_points() {
        let raw = project(
            "[]",
            r#"[{"contributor": "ada", "contributorType": ["Project Leader"]},
                {"contributor": "unibas", "contributorType": ["Funder"]}]"#,
            r#"["ada"]"#,
        );
        assert_eq!(
            contributor_refs(&raw),
            vec![
                ContributorRef { field: "attributions", index: 0, id: "ada".to_string() },
                ContributorRef { field: "attributions", index: 1, id: "unibas".to_string() },
                // Indices restart per member: this is `contactPoint[0]`, and the
                // repeated id is kept rather than folded into the one above.
                ContributorRef { field: "contactPoint", index: 0, id: "ada".to_string() },
            ]
        );
    }

    #[test]
    fn an_absent_contact_point_contributes_no_refs() {
        let raw = project("[]", r#"[{"contributor": "ada", "contributorType": ["Creator"]}]"#, "null");
        assert_eq!(
            contributor_refs(&raw),
            vec![ContributorRef { field: "attributions", index: 0, id: "ada".to_string() }]
        );
    }
}
