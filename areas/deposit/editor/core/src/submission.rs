//! The checks a draft must pass to become a pending submission.
//!
//! [`check`] runs nine gates in a fixed order and returns the first refusal as
//! data, so the order is part of the contract: a draft failing two gates shows
//! the depositor the earlier one.
//!
//! 1. every required field is answered ([`SubmitRefusal::Unanswered`]);
//! 2. the draft converts to a [`ProjectRaw`] ([`SubmitRefusal::Incomplete`]);
//! 3. every agent reference resolves ([`SubmitRefusal::UnknownAgent`]);
//! 4. the project's own live proposals are complete ([`SubmitRefusal::ProposalIncomplete`]);
//! 5. no typed placeholder sentinel is left ([`SubmitRefusal::Sentinel`]);
//! 6. every value has the format its kind needs ([`SubmitRefusal::Format`]);
//! 7. every data language is a listed code or already published
//!    ([`SubmitRefusal::UnlistedLanguage`]);
//! 8. every `temporalCoverage` entry resolves to a structured date ([`SubmitRefusal::Invalid`]);
//! 9. something differs from the published project ([`SubmitRefusal::Unchanged`]).
//!
//! The eighth rule is [`shared_metadata::temporal_coverage::completeness_gap`],
//! the same function `access-server validate` and `dpe-api-oai` apply, so the
//! three cannot drift on what counts as a gap. `access-server validate` does not
//! block on it and OAI-PMH needs it, so a submission carrying an unresolvable
//! period would open a pull request that fails CI. The depositor is never
//! stranded: `temporalCoverage`'s `Reference` variant always resolves.
//!
//! Core returns data only. Every refusal's wording, per-field message, span
//! outcome and log event belongs to the caller.

use std::collections::HashMap;

use shared_metadata::project::ProjectRaw;
use shared_metadata::temporal_coverage;
use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::W3cdtfRange;

use crate::agents::{AgentScope, Agents};
use crate::draft::{DraftError, ProjectDraft};
use crate::form::format::{self, Finding};
use crate::form::obligation::unsatisfied_required;
use crate::form::registry::{sections_for, Audience, Field};
use crate::form::submit::{typed_sentinels, unlisted_languages, unresolved_agents};
use crate::proposals::{problems as proposal_problems, EntityProposal, ProposalProblem};
use crate::review::diff;

/// The first gate a draft failed, and what it found there.
#[derive(Debug)]
pub enum SubmitRefusal {
    /// Required fields the draft does not answer, in form order.
    Unanswered(Vec<&'static Field>),
    /// The draft is not a [`ProjectRaw`]: a member of the wrong JSON kind, which
    /// the form cannot produce and which no field-level rule can name.
    Incomplete(DraftError),
    /// Agent references that resolve to nobody, with the field holding each.
    UnknownAgent(Vec<(&'static Field, String)>),
    /// One entry per problem in the project's own live proposals, in proposal
    /// then finding order.
    ProposalIncomplete(Vec<ProposalProblem>),
    /// Fields holding a placeholder sentinel the depositor typed.
    Sentinel(Vec<&'static Field>),
    /// Values without the format their kind needs, in form order, each keyed by
    /// stored position.
    Format(Vec<Finding>),
    /// Data-language values that are neither a listed code nor already published, with their field.
    UnlistedLanguage(Vec<(&'static Field, String)>),
    /// `temporalCoverage` entries that cannot resolve to a structured date.
    Invalid(Vec<UnresolvedCoverage>),
    /// The draft is identical to the published project.
    Unchanged,
}

/// Run the submit gates in order and return the first refusal.
///
/// `draft` is the in-memory draft after the posted body was applied. `agents` is
/// the published store: the scope agent references resolve against is built here
/// from it plus `proposals`, which is every proposal of the project whatever its
/// status. `published` is the project as the published set holds it, `None` for a
/// project that is not published yet.
///
/// # Errors
///
/// The first failing gate's [`SubmitRefusal`].
pub fn check(
    audience: Audience,
    draft: &ProjectDraft,
    agents: &Agents,
    proposals: &[EntityProposal],
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
    published: Option<&ProjectDraft>,
) -> Result<(), SubmitRefusal> {
    // The obligation gate reads the draft, so it needs nothing from the contract conversion below
    // and must stay ahead of it: every way a required value can be missing — absent, `[]`,
    // `{}`, `""`, a placeholder sentinel — is reachable from the form, and the conversion can
    // only report the first of them, without naming a field. Presence is read through
    // `obligation::is_satisfied`, the same function the section rail counts with, so the gate
    // cannot refuse a submission the rail has just called complete.
    let unanswered = unsatisfied_required(audience, draft);
    if !unanswered.is_empty() {
        return Err(SubmitRefusal::Unanswered(unanswered));
    }

    // Do not move this ahead of the obligation gate. Emptying a required list to zero rows removes
    // the member — `keywords`, `attributions`, `disciplines`, `temporalCoverage` and
    // `spatialCoverage` are non-`Option` `Vec`s and the row appliers use absent as their empty
    // state — so the conversion fails first with a generic "missing field" and this branch
    // reports it naming no field at all.
    let raw = draft.to_raw().map_err(SubmitRefusal::Incomplete)?;

    // Every agent reference must resolve. The applier stores whatever arrives,
    // because a draft may hold a value that does not validate — this
    // is what stops an unresolvable id reaching a published file, where it
    // renders as a bare `person-001` on the public project page.
    let scope = AgentScope::with_proposals(agents, proposals);
    let unknown = unresolved_agents(audience, draft, &scope);
    if !unknown.is_empty() {
        return Err(SubmitRefusal::UnknownAgent(unknown));
    }

    // This project's own live proposals must satisfy the same rules the entity form enforces: a
    // `Draft`/`Submitted` proposal riding into review with an incomplete payload would let RDU
    // accept it into a broken committed file, with no field-level check left to stop
    // it once this project is approved. After the agent gate, which resolves them.
    let problems = proposal_problems(proposals, |id| scope.published_body(id));
    if !problems.is_empty() {
        return Err(SubmitRefusal::ProposalIncomplete(problems));
    }

    // A sentinel a depositor typed, which the shape is what recognises: a
    // sentinel is the *correct* stored value wherever clearing the field writes
    // one, so this refuses only the shapes where clearing means something else.
    // Left through, the field reads as empty, an empty submit will not clear it,
    // and the platform reads the stored word as no value at all.
    let sentinels = typed_sentinels(audience, draft);
    if !sentinels.is_empty() {
        return Err(SubmitRefusal::Sentinel(sentinels));
    }

    // Only the fields this audience sees: a depositor cannot fix an RDU-only field,
    // and a refusal naming one would strand the submission.
    let findings = format::findings(
        sections_for(audience).flat_map(|section| section.fields_for(audience)),
        draft,
        None,
    );
    if !findings.is_empty() {
        return Err(SubmitRefusal::Format(findings));
    }

    let unlisted = unlisted_languages(audience, draft, published);
    if !unlisted.is_empty() {
        return Err(SubmitRefusal::UnlistedLanguage(unlisted));
    }

    // Re-run on every submit, which is what makes a resubmission revalidated rather than trusted
    // because it was reviewed once.
    let unresolved = unresolved_temporal_coverage(&raw, periods, enrichment);
    if !unresolved.is_empty() {
        return Err(SubmitRefusal::Invalid(unresolved));
    }

    // Nothing to review. Allowed through, this locks the depositor's own form
    // on a submission a reviewer can only clear by rejecting it. The comparison
    // is `review::diff`, the one the review surface itself renders from, counted as
    // `status::Comparison::classify` counts it: a draft's stale copy of a git-owned
    // member is not a change the pull-request run would write.
    if !diff(published, draft)
        .iter()
        .any(|row| row.counts_as_change(published.is_some()))
    {
        return Err(SubmitRefusal::Unchanged);
    }

    Ok(())
}

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
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use shared_metadata::project::TemporalCoverage;
    use shared_metadata::utils::Multilingual;
    use shared_metadata::AuthorityFileReference;
    use uuid::Uuid;

    use super::*;
    use crate::proposals::{ProposalFault, ProposalKind, ProposalOperation, ProposalStatus};
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
            shared_metadata::w3cdtf::to_w3cdtf_range(Some("98"), Some("117")).expect("a range"),
        )])
    }

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

    fn committed_agents() -> Agents {
        let dir = crate::checkout_dpe_data_dir();
        let (agents, errors) = Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert!(errors.is_empty(), "{errors:?}");
        agents
    }

    fn proposal(payload: &str, status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            id: Uuid::nil(),
            shortcode: "0102".to_string(),
            entity_id: "person-900".to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::New,
            payload: payload.to_string(),
            status,
            decision: None,
            proposed_by: None,
            created_at: DateTime::<Utc>::MIN_UTC,
            updated_at: DateTime::<Utc>::MIN_UTC,
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    /// A required member of the wrong JSON kind is still an answer to the
    /// obligation gate (`has_value` counts a number), so only the conversion to
    /// the contract type can refuse it.
    #[test]
    fn a_required_member_of_the_wrong_kind_is_refused_as_incomplete() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("name", json!(7));
        assert!(unsatisfied_required(Audience::Everyone, &draft).is_empty());

        let refusal = check(
            Audience::Everyone,
            &draft,
            &Agents::default(),
            &[],
            &periods(),
            &HashMap::new(),
            None,
        )
        .expect_err("a draft that is not a project is refused");
        assert!(matches!(refusal, SubmitRefusal::Incomplete(_)), "{refusal:?}");
    }

    /// With no agents known, every reference in the sample is unresolvable, and the reference gate
    /// refuses ahead of the proposal gate even though the live proposal is unreadable too.
    #[test]
    fn the_agent_gate_refuses_ahead_of_the_proposal_gate() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        let proposals = [proposal("not json", ProposalStatus::Draft)];

        let refusal = check(
            Audience::Everyone,
            &draft,
            &Agents::default(),
            &proposals,
            &periods(),
            &HashMap::new(),
            None,
        )
        .expect_err("an unresolvable reference is refused");
        assert!(matches!(refusal, SubmitRefusal::UnknownAgent(_)), "{refusal:?}");
    }

    /// A live proposal whose stored payload cannot be parsed is a refusal, not a
    /// skipped check; a terminal one is not looked at.
    #[test]
    fn a_live_proposal_with_an_unreadable_payload_is_refused() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        let agents = committed_agents();
        let proposals = [
            proposal("not json", ProposalStatus::Draft),
            proposal("not json", ProposalStatus::Withdrawn),
        ];

        let refusal = check(
            Audience::Everyone,
            &draft,
            &agents,
            &proposals,
            &periods(),
            &HashMap::new(),
            None,
        )
        .expect_err("an unreadable live proposal is refused");
        let SubmitRefusal::ProposalIncomplete(problems) = refusal else {
            panic!("expected ProposalIncomplete, got {refusal:?}");
        };
        assert_eq!(
            problems,
            [ProposalProblem {
                entity_id: "person-900".to_string(),
                kind: ProposalKind::Person,
                fault: ProposalFault::Unreadable,
            }]
        );
    }

    #[test]
    fn a_value_without_its_format_is_refused_for_the_audience_that_sees_it() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("additionalMaterial", json!(["www.dasch.swiss"]));
        draft.set("documentationMaterial", json!(["www.dasch.swiss"]));
        let agents = committed_agents();

        for (audience, expected) in [
            (Audience::Everyone, &["additionalMaterial.r0"][..]),
            (Audience::RduOnly, &["documentationMaterial.r0", "additionalMaterial.r0"][..]),
        ] {
            let refusal = check(audience, &draft, &agents, &[], &periods(), &HashMap::new(), None)
                .expect_err("a value without its format is refused");
            let SubmitRefusal::Format(findings) = refusal else {
                panic!("expected Format, got {refusal:?}");
            };
            let controls: Vec<&str> = findings.iter().map(|finding| finding.control.as_str()).collect();
            assert_eq!(controls, expected, "{audience:?}");
        }
    }
}
