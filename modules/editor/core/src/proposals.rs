//! Proposals to add or change a person or organisation.
//!
//! A project field that refers to a contributor by id can only name something
//! that already exists, so a depositor who needs a new person or organisation,
//! or a correction to one, cannot express that in the project draft. A proposal
//! carries such a request through its own review. `payload` is opaque JSON for
//! the reason [`DraftRecord::payload`](crate::records::DraftRecord::payload) is:
//! a proposal can be half-filled, and only [`check_person`] and
//! [`check_organization`] decide whether it is complete enough to submit.
//!
//! [`next_entity_id`] never reuses a number, so the sequence has gaps by design:
//! reuse would let two entities carry one id at different times, and anything
//! collected while the first was live would resolve to the second. The editor
//! architecture documentation carries the rest.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::records::UnknownVariant;

/// Which entity store a proposal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProposalKind {
    Person,
    Organization,
}

impl ProposalKind {
    /// The stored form. Pinned by a `CHECK` constraint in the schema, so this
    /// and the migration have to agree.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organization => "organization",
        }
    }

    /// The word shown beside a proposal, so a list mixing both kinds says which
    /// is which. British spelling, matching
    /// [`AgentKind::label`](crate::agents::AgentKind::label).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::Organization => "Organisation",
        }
    }
}

impl fmt::Display for ProposalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProposalKind {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "person" => Ok(Self::Person),
            "organization" => Ok(Self::Organization),
            other => Err(UnknownVariant { kind: "proposal kind", value: other.to_string() }),
        }
    }
}

/// What a proposal does to the entity store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProposalOperation {
    /// A person or organisation that does not exist yet.
    New,
    /// A change to an entity the project already references.
    Change,
}

impl ProposalOperation {
    /// The stored form, pinned by a `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Change => "change",
        }
    }
}

impl fmt::Display for ProposalOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProposalOperation {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "new" => Ok(Self::New),
            "change" => Ok(Self::Change),
            other => Err(UnknownVariant { kind: "proposal operation", value: other.to_string() }),
        }
    }
}

/// Where a proposal sits in review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProposalStatus {
    Draft,
    Submitted,
    Accepted,
    Rejected,
    Withdrawn,
}

impl ProposalStatus {
    /// The stored form, pinned by a `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Submitted => "submitted",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
        }
    }
}

impl fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProposalStatus {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "submitted" => Ok(Self::Submitted),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(UnknownVariant { kind: "proposal status", value: other.to_string() }),
        }
    }
}
/// What RDU recorded about a proposal in the round now running.
///
/// Not [`review::Decision`](crate::review::Decision), whose opposite of accept
/// is "keep the published value": a proposed new entity has no published value,
/// so the only honest opposite is rejecting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProposalDecision {
    /// Take the proposal: on approval its entity becomes a file.
    Accept,
    /// Refuse it. Anything still referring to the entity blocks the approval
    /// rather than shipping a dangling reference — see the referential-integrity
    /// gate on the review surface.
    Reject,
}

impl ProposalDecision {
    /// The stored form, and the value a decision control posts. Pinned by a
    /// `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
        }
    }
}

impl fmt::Display for ProposalDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProposalDecision {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "accept" => Ok(Self::Accept),
            "reject" => Ok(Self::Reject),
            other => Err(UnknownVariant { kind: "proposal decision", value: other.to_string() }),
        }
    }
}

/// A request to add or change one person or organisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityProposal {
    pub id: Uuid,
    /// The project this proposal rides with, as [`crate::records::normalize_shortcode`] keys it.
    pub shortcode: String,
    /// The allocated `person-NNN` / `organization-NNN`, or the existing entity's
    /// id for a [`ProposalOperation::Change`].
    pub entity_id: String,
    pub kind: ProposalKind,
    pub operation: ProposalOperation,
    /// The proposed entity as JSON, a `platform_metadata::Person` or
    /// `Organization` body. Opaque here, like
    /// [`DraftRecord::payload`](crate::records::DraftRecord::payload): a
    /// half-filled proposal cannot deserialize as the contract type, and
    /// [`check_person`] and [`check_organization`] decide completeness.
    ///
    /// A producer must leave `id` out. It lives in [`Self::entity_id`], the
    /// column the uniqueness index and the allocator work on; readers fill it in
    /// from there and overwrite whatever they find.
    pub payload: String,
    pub status: ProposalStatus,
    /// `None` once that account is removed; see
    /// [`DraftRecord::updated_by`](crate::records::DraftRecord::updated_by).
    pub proposed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// What RDU recorded this round, `None` while undecided. Retained across a
    /// request-changes return, for the reason `review::ReviewState` is.
    pub decision: Option<ProposalDecision>,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
}

impl EntityProposal {
    /// Whether this proposal is still in play: the depositor may edit it and a
    /// reviewer may still decide it. True for [`ProposalStatus::Draft`] and
    /// [`ProposalStatus::Submitted`]; [`ProposalStatus::Accepted`] is excluded
    /// because nothing about it can change any more.
    ///
    /// This says nothing about the allocated id: every row holds its id
    /// permanently, terminal ones included, which is why they stay in the
    /// uniqueness index.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self.status, ProposalStatus::Draft | ProposalStatus::Submitted)
    }

    /// Whether a project field may name this proposal's `entity_id` without
    /// submit refusing the reference.
    ///
    /// Wider than [`Self::is_live`] by [`ProposalStatus::Accepted`], whose entity
    /// is not a file yet but will exist. Narrower than every row by the two
    /// terminal statuses: a rejected entity must make the approval refuse rather
    /// than resolve.
    #[must_use]
    pub fn is_referenceable(&self) -> bool {
        matches!(
            self.status,
            ProposalStatus::Draft | ProposalStatus::Submitted | ProposalStatus::Accepted
        )
    }
}

/// The numeric suffix of `id`, if `id` has the shape `{kind.as_str()}-N`.
/// `None` for anything else: a different kind's prefix, a missing or non-numeric
/// suffix, or a number too large for `u32`.
#[must_use]
pub fn entity_id_number(kind: ProposalKind, id: &str) -> Option<u32> {
    let suffix = id.strip_prefix(kind.as_str())?.strip_prefix('-')?;
    suffix.parse().ok()
}

/// Format an allocated id: `person-417`, `organization-143`.
///
/// Zero-padded to at least three digits, as the committed corpus spells them; a
/// number past 999 is written in full. The padding is cosmetic, not an ordering
/// guarantee: string order stops agreeing with numeric order at `person-1000`,
/// which is why allocation parses ids back to numbers and the store must not
/// answer "the highest id" with a TEXT `MAX`.
#[must_use]
pub fn format_entity_id(kind: ProposalKind, number: u32) -> String {
    format!("{}-{number:03}", kind.as_str())
}

/// The next id to allocate for `kind`, given every id already taken: the union
/// of the published store and every id this editor has ever allocated, terminal
/// proposals included.
///
/// One past the highest number found for `kind`, or `1` if none is. Entries of
/// the other kind, or that do not parse as this kind's shape, are ignored:
/// `taken` is an unfiltered union of two id spaces.
#[must_use]
pub fn next_entity_id<'a>(kind: ProposalKind, taken: impl Iterator<Item = &'a str>) -> String {
    let highest = taken.filter_map(|id| entity_id_number(kind, id)).max().unwrap_or(0);
    // Saturating, not `+ 1`: `taken` includes ids read from committed files, so a
    // hand-authored `person-4294967295.json` would overflow here. Saturating
    // leaves the allocation to fail closed on the store's uniqueness constraint.
    format_entity_id(kind, highest.saturating_add(1))
}

/// One thing wrong with a proposed entity, keyed by the member it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityFinding {
    /// The member as it is spelled in JSON: `"name"`, `"url"`, `"address"`,
    /// `"jobTitles"`, `"givenNames"`, `"familyNames"`.
    pub field: &'static str,
    /// Position within a repeatable member, in document order; `None` for a
    /// scalar. Not an identity — same reasoning as
    /// [`platform_metadata::checks::Finding`], whose docs state it.
    pub index: Option<usize>,
    pub message: String,
}

/// Every rule a proposed organisation must satisfy, against the `Organization`
/// shape in `modules/platform/metadata/src/organization.rs`.
///
/// `published` is the entity as the committed store holds it, for a
/// [`ProposalOperation::Change`], and `None` for a [`ProposalOperation::New`].
///
/// An `address` byte-equal to `published`'s passes; any other incomplete one is
/// refused. The rule wants all four of `street`, `postalCode`, `locality` and
/// `country` or none, but six committed organizations satisfy neither, and
/// judging an inherited value would make them unchangeable without inventing
/// data. `tests::six_committed_organizations_carry_an_incomplete_address` pins
/// them. `sameAs` carries no rule: the contract defaults an absent one, and no
/// rule asks for one.
#[must_use]
pub fn check_organization(payload: &serde_json::Value, published: Option<&serde_json::Value>) -> Vec<EntityFinding> {
    let mut findings = Vec::new();

    if !holds_non_blank_string(payload, "name") {
        findings.push(EntityFinding {
            field: "name",
            index: None,
            message: "name is required".to_string(),
        });
    }
    if !holds_non_blank_string(payload, "url") {
        findings.push(EntityFinding {
            field: "url",
            index: None,
            message: "url is required".to_string(),
        });
    }

    // `address` is optional, but once present each of the four is required even
    // if every one is blank: a section the depositor opened and left blank must
    // not be silently dropped. Unless it is exactly what the published entity
    // held; see the grandfathering paragraph in this function's docs.
    if let Some(address) = payload.get("address").filter(|value| !value.is_null()) {
        let inherited_unchanged = published
            .and_then(|entity| entity.get("address"))
            .is_some_and(|prior| prior == address);
        if !inherited_unchanged {
            for field in ["street", "postalCode", "locality", "country"] {
                if !holds_non_blank_string(address, field) {
                    findings.push(EntityFinding {
                        field: "address",
                        index: None,
                        message: format!("address.{field} is required once address is provided"),
                    });
                }
            }
        }
    }

    findings
}

/// Every rule a proposed person must satisfy, against the `Person` shape in
/// `modules/platform/metadata/src/person.rs`, including the project-role guard.
/// `sameAs` carries no rule, as in [`check_organization`].
#[must_use]
pub fn check_person(payload: &serde_json::Value) -> Vec<EntityFinding> {
    let mut findings = Vec::new();

    for field in ["givenNames", "familyNames"] {
        if !holds_non_blank_array_entry(payload, field) {
            findings.push(EntityFinding {
                field,
                index: None,
                message: format!("{field} must hold at least one non-blank entry"),
            });
        }
    }

    // Present, and no more: a non-defaulted `Vec<String>` is satisfied by `[]`,
    // which many committed persons hold. `givenNames` and `familyNames` carry
    // the stronger rule because a person with neither renders as its own id,
    // which `agents::person_label` documents as unselectable in a picker.
    if !payload.get("jobTitles").is_some_and(serde_json::Value::is_array) {
        findings.push(EntityFinding {
            field: "jobTitles",
            index: None,
            message: "jobTitles must be present, though it may be empty".to_string(),
        });
    }

    // `dpe-server validate` rejects a committed file carrying a
    // `platform_metadata::JOB_TITLE_ROLE_WORDS` entry in `jobTitles`, because the
    // OAI-PMH creator/contributor logic reads only `attributions`. Refusing here
    // stops a proposal that would fail that validation later.
    if let Some(job_titles) = payload.get("jobTitles").and_then(|value| value.as_array()) {
        for (index, title) in job_titles.iter().enumerate() {
            if let Some(title) = title.as_str() {
                if platform_metadata::is_role_job_title(title) {
                    findings.push(EntityFinding {
                        field: "jobTitles",
                        index: Some(index),
                        message: format!(
                            "{title:?} is a project-contribution role, not a job title; it belongs in the project's attributions (contributorType)"
                        ),
                    });
                }
            }
        }
    }

    findings
}

/// Whether `payload`'s `field` member is a JSON string that is non-blank after
/// trimming. Missing, non-string, and all-whitespace all read as "not
/// provided" — a form field can be absent, wrongly typed, or spaces alone, and the entity
/// rules treat the three as one failure.
fn holds_non_blank_string(payload: &serde_json::Value, field: &str) -> bool {
    payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

/// Whether `payload`'s `field` member is a JSON array holding at least one
/// non-blank string entry.
fn holds_non_blank_array_entry(payload: &serde_json::Value, field: &str) -> bool {
    payload
        .get(field)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| entries.iter().any(|entry| entry.as_str().is_some_and(|s| !s.trim().is_empty())))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::*;

    #[test]
    fn test_proposal_kind_round_trips_through_its_stored_form() {
        for kind in [ProposalKind::Person, ProposalKind::Organization] {
            assert_eq!(kind.as_str().parse::<ProposalKind>().unwrap(), kind);
        }
    }

    #[test]
    fn test_unknown_stored_proposal_kind_is_an_error_not_a_default() {
        assert!("company".parse::<ProposalKind>().is_err());
    }

    #[test]
    fn test_proposal_operation_round_trips_through_its_stored_form() {
        for operation in [ProposalOperation::New, ProposalOperation::Change] {
            assert_eq!(operation.as_str().parse::<ProposalOperation>().unwrap(), operation);
        }
    }

    #[test]
    fn test_unknown_stored_proposal_operation_is_an_error_not_a_default() {
        assert!("delete".parse::<ProposalOperation>().is_err());
    }

    #[test]
    fn test_proposal_status_round_trips_through_its_stored_form() {
        for status in [
            ProposalStatus::Draft,
            ProposalStatus::Submitted,
            ProposalStatus::Accepted,
            ProposalStatus::Rejected,
            ProposalStatus::Withdrawn,
        ] {
            assert_eq!(status.as_str().parse::<ProposalStatus>().unwrap(), status);
        }
    }

    #[test]
    fn test_proposal_decision_round_trips_through_its_stored_form() {
        for decision in [ProposalDecision::Accept, ProposalDecision::Reject] {
            assert_eq!(decision.as_str().parse::<ProposalDecision>().unwrap(), decision);
        }
    }

    #[test]
    fn test_a_proposal_decision_does_not_read_a_field_decision() {
        // The two vocabularies share `accept`, so reading either as the other has
        // to fail rather than land on whichever variant sorts first.
        assert!("revert".parse::<ProposalDecision>().is_err());
        assert_eq!("accept".parse::<ProposalDecision>().unwrap(), ProposalDecision::Accept);
    }

    #[test]
    fn test_unknown_stored_proposal_status_is_an_error_not_a_default() {
        assert!("in_review".parse::<ProposalStatus>().is_err());
    }

    fn proposal(status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            id: Uuid::nil(),
            shortcode: "0801".to_string(),
            entity_id: "person-417".to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::New,
            payload: "{}".to_string(),
            status,
            decision: None,
            proposed_by: None,
            created_at: DateTime::<Utc>::MIN_UTC,
            updated_at: DateTime::<Utc>::MIN_UTC,
            decided_by: None,
            decided_at: None,
        }
    }

    #[test]
    fn an_accepted_proposal_is_referenceable_but_not_live() {
        // The gap between the two predicates, and the reason there are two: an
        // accepted entity is not a file yet, so a project naming it must still
        // resolve, while nothing about the proposal can change any more.
        let accepted = proposal(ProposalStatus::Accepted);
        assert!(accepted.is_referenceable());
        assert!(!accepted.is_live());
    }

    #[test]
    fn a_rejected_or_withdrawn_proposal_is_not_referenceable() {
        // What makes the referential-integrity gate possible: a project still
        // naming a rejected entity has to fail resolution, not quietly pass.
        for status in [ProposalStatus::Rejected, ProposalStatus::Withdrawn] {
            assert!(!proposal(status).is_referenceable(), "{status}");
        }
    }

    #[test]
    fn draft_and_submitted_proposals_are_live() {
        assert!(proposal(ProposalStatus::Draft).is_live());
        assert!(proposal(ProposalStatus::Submitted).is_live());
    }

    #[test]
    fn accepted_rejected_and_withdrawn_proposals_are_not_live() {
        // Accepted is on its way to a file, so the published set will hold its
        // id next; Rejected and Withdrawn never held anything to keep live.
        for status in [
            ProposalStatus::Accepted,
            ProposalStatus::Rejected,
            ProposalStatus::Withdrawn,
        ] {
            assert!(!proposal(status).is_live(), "{status}");
        }
    }

    #[test]
    fn test_entity_id_number_parses_the_kind_it_is_asked_for() {
        assert_eq!(entity_id_number(ProposalKind::Person, "person-001"), Some(1));
        assert_eq!(entity_id_number(ProposalKind::Person, "person-417"), Some(417));
        assert_eq!(entity_id_number(ProposalKind::Organization, "organization-142"), Some(142));
    }

    #[test]
    fn test_entity_id_number_rejects_the_wrong_kinds_prefix() {
        assert_eq!(entity_id_number(ProposalKind::Person, "organization-001"), None);
        assert_eq!(entity_id_number(ProposalKind::Organization, "person-001"), None);
    }

    #[test]
    fn test_entity_id_number_rejects_a_blank_suffix() {
        assert_eq!(entity_id_number(ProposalKind::Person, "person-"), None);
        assert_eq!(entity_id_number(ProposalKind::Person, "person"), None);
    }

    #[test]
    fn test_entity_id_number_rejects_a_non_numeric_suffix() {
        assert_eq!(entity_id_number(ProposalKind::Person, "person-abc"), None);
    }

    #[test]
    fn test_entity_id_number_rejects_an_overflowing_suffix() {
        assert_eq!(entity_id_number(ProposalKind::Person, "person-99999999999"), None);
    }

    #[test]
    fn test_format_entity_id_pads_to_three_digits() {
        assert_eq!(format_entity_id(ProposalKind::Person, 1), "person-001");
        assert_eq!(format_entity_id(ProposalKind::Organization, 142), "organization-142");
    }

    #[test]
    fn test_format_entity_id_does_not_truncate_past_three_digits() {
        assert_eq!(format_entity_id(ProposalKind::Person, 1234), "person-1234");
    }

    #[test]
    fn test_next_entity_id_starts_at_one_for_an_empty_taken_set() {
        assert_eq!(next_entity_id(ProposalKind::Person, std::iter::empty()), "person-001");
    }

    #[test]
    fn test_next_entity_id_is_one_past_the_highest_number_seen() {
        let taken = ["person-001", "person-002", "person-050"];
        assert_eq!(next_entity_id(ProposalKind::Person, taken.into_iter()), "person-051");
    }

    #[test]
    fn allocation_leaves_a_hole_rather_than_filling_it() {
        // A future allocation must never reuse person-002..004 even though
        // nothing currently holds them — see the module docs on why reuse is
        // refused.
        let taken = ["person-001", "person-005"];
        assert_eq!(next_entity_id(ProposalKind::Person, taken.into_iter()), "person-006");
    }

    /// Allocation must not degrade into string comparison once ids pass 999:
    /// lexicographic order would allocate `person-100` below ids already taken,
    /// and SQLite's `MAX(entity_id)` on a TEXT column answers exactly that.
    #[test]
    fn allocation_is_numeric_not_lexicographic_past_three_digits() {
        let taken = ["person-002", "person-99", "person-417", "person-1000"];
        assert_eq!(next_entity_id(ProposalKind::Person, taken.into_iter()), "person-1001");

        // And the boundary itself: 999 rolls into four digits rather than wrapping.
        assert_eq!(next_entity_id(ProposalKind::Person, ["person-999"].into_iter()), "person-1000");
        assert_eq!(format_entity_id(ProposalKind::Person, 1000), "person-1000");
        assert_eq!(entity_id_number(ProposalKind::Person, "person-1000"), Some(1000));
    }

    #[test]
    fn test_next_entity_id_ignores_the_other_kinds_ids_and_unparseable_entries() {
        let taken = ["organization-500", "person-003", "not-an-id", "person-abc"];
        assert_eq!(next_entity_id(ProposalKind::Person, taken.into_iter()), "person-004");
    }

    fn data_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data")
    }

    /// Every entity id in `dir`, read from each file's `id` member, which is what
    /// the runtime keys on, not the filename stem.
    fn ids_in(dir: &Path) -> Vec<String> {
        let mut ids: Vec<String> = json_files(dir)
            .map(|path| {
                let json = std::fs::read_to_string(&path).expect("an entity file reads");
                let entity: serde_json::Value = serde_json::from_str(&json).expect("an entity file parses");
                let id = entity["id"].as_str().expect("an entity has an id").to_string();
                // The convention the rest of the tree assumes, asserted where
                // it is cheapest to notice: a file whose name and id disagree
                // would still load, and nothing else would say so.
                assert_eq!(
                    path.file_stem().expect("a json file has a stem").to_string_lossy(),
                    id,
                    "{} names an entity whose id is {id}",
                    path.display()
                );
                id
            })
            .collect();
        ids.sort();
        ids
    }

    /// The `*.json` files directly under `dir`, in directory order.
    fn json_files(dir: &Path) -> impl Iterator<Item = PathBuf> {
        std::fs::read_dir(dir)
            .expect("data directory reads")
            .map(|entry| entry.expect("directory entry reads").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
    }

    #[test]
    fn allocation_against_the_committed_corpus_yields_the_next_free_id_of_each_kind() {
        // Enumerated, not sampled, like
        // `published::tests::the_whole_committed_corpus_loads_with_no_errors`: a directory
        // listing under- or over-counted would silently hand out a colliding id.
        let persons = ids_in(&data_dir().join("persons"));
        let organizations = ids_in(&data_dir().join("organizations"));
        assert_eq!(persons.len(), 416, "the committed store is 416 persons");
        assert_eq!(organizations.len(), 142, "the committed store is 142 organizations");

        assert_eq!(
            next_entity_id(ProposalKind::Person, persons.iter().map(String::as_str)),
            "person-417"
        );
        assert_eq!(
            next_entity_id(ProposalKind::Organization, organizations.iter().map(String::as_str)),
            "organization-143"
        );
    }

    fn valid_organization() -> serde_json::Value {
        json!({
            "name": "Université de Lausanne",
            "url": "https://www.unil.ch/",
        })
    }

    #[test]
    fn a_valid_organisation_has_no_findings() {
        assert!(check_organization(&valid_organization(), None).is_empty());
    }

    #[test]
    fn an_organisation_missing_name_is_a_finding() {
        let mut org = valid_organization();
        org.as_object_mut().unwrap().remove("name");
        let findings = check_organization(&org, None);
        assert!(findings.iter().any(|f| f.field == "name"), "{findings:?}");
    }

    #[test]
    fn an_organisation_with_a_blank_name_is_a_finding() {
        let mut org = valid_organization();
        org["name"] = json!("   ");
        let findings = check_organization(&org, None);
        assert!(findings.iter().any(|f| f.field == "name"), "{findings:?}");
    }

    #[test]
    fn an_organisation_missing_url_is_a_finding() {
        let mut org = valid_organization();
        org.as_object_mut().unwrap().remove("url");
        let findings = check_organization(&org, None);
        assert!(findings.iter().any(|f| f.field == "url"), "{findings:?}");
    }

    #[test]
    fn an_absent_address_is_not_a_finding() {
        assert!(check_organization(&valid_organization(), None).is_empty());
    }

    #[test]
    fn an_incomplete_address_inherited_unchanged_from_the_published_entity_is_no_finding() {
        // `organization-065` is committed without a `postalCode`, so an address
        // byte-equal to the published one passes.
        let published = json!({
            "id": "organization-065",
            "name": "Tanta University",
            "url": "https://tanta.edu.eg/",
            "address": { "street": "Al-Geish St.", "postalCode": "", "locality": "Tanta", "country": "Egypt" },
        });
        let mut proposed = published.clone();
        proposed["url"] = json!("https://www.tanta.edu.eg/");
        let findings = check_organization(&proposed, Some(&published));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_incomplete_address_the_depositor_edited_is_still_a_finding() {
        // The other half of the carve-out: once they touch the address, the
        // rule applies. Grandfathering the *value* rather than the entity is
        // what keeps an edit from smuggling a new incomplete address in behind
        // a published one.
        let published = json!({
            "name": "Tanta University",
            "url": "https://tanta.edu.eg/",
            "address": { "street": "Al-Geish St.", "postalCode": "", "locality": "Tanta", "country": "Egypt" },
        });
        let mut proposed = published.clone();
        proposed["address"]["locality"] = json!("Tanta City");
        let findings = check_organization(&proposed, Some(&published));
        assert!(findings.iter().any(|f| f.field == "address"), "{findings:?}");
    }

    #[test]
    fn a_new_organisation_gets_no_grandfathering() {
        let mut org = valid_organization();
        org["address"] = json!({ "street": "", "postalCode": "1015", "locality": "Lausanne", "country": "CH" });
        assert!(!check_organization(&org, None).is_empty());
    }

    /// The corpus fact the carve-out rests on, enumerated: if a data change
    /// completed all six addresses, the carve-out would have no reason to exist.
    #[test]
    fn six_committed_organizations_carry_an_incomplete_address() {
        let mut incomplete = Vec::new();
        for file in json_files(&data_dir().join("organizations")) {
            let org: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&file).expect("an organization file reads"))
                    .expect("an organization file parses");
            if let Some(address) = org.get("address").filter(|value| !value.is_null()) {
                if ["street", "postalCode", "locality", "country"]
                    .iter()
                    .any(|member| !holds_non_blank_string(address, member))
                {
                    incomplete.push(org["id"].as_str().expect("an id").to_string());
                }
            }
        }
        incomplete.sort();
        assert_eq!(
            incomplete,
            [
                "organization-009",
                "organization-033",
                "organization-065",
                "organization-089",
                "organization-090",
                "organization-137",
            ],
            "the address carve-out is calibrated on exactly these"
        );
    }

    #[test]
    fn a_complete_address_is_not_a_finding() {
        let mut org = valid_organization();
        org["address"] = json!({
            "street": "Unicentre",
            "postalCode": "1015",
            "locality": "Lausanne",
            "country": "Switzerland",
        });
        assert!(check_organization(&org, None).is_empty());
    }

    #[test]
    fn a_partial_address_missing_one_of_four_members_is_a_finding() {
        let mut org = valid_organization();
        org["address"] = json!({
            "street": "Unicentre",
            "postalCode": "1015",
            "locality": "Lausanne",
            // country omitted
        });
        let findings = check_organization(&org, None);
        assert!(findings.iter().any(|f| f.field == "address"), "{findings:?}");
    }

    #[test]
    fn an_address_with_all_four_members_blank_is_still_a_finding() {
        let mut org = valid_organization();
        org["address"] = json!({
            "street": "",
            "postalCode": "  ",
            "locality": "",
            "country": "",
        });
        let findings = check_organization(&org, None);
        assert_eq!(findings.iter().filter(|f| f.field == "address").count(), 4, "{findings:?}");
    }

    fn valid_person() -> serde_json::Value {
        json!({
            "givenNames": ["Philippe"],
            "familyNames": ["Gonzalez"],
            "jobTitles": ["Senior lecturer"],
        })
    }

    #[test]
    fn a_valid_person_has_no_findings() {
        assert!(check_person(&valid_person()).is_empty());
    }

    #[test]
    fn a_person_missing_given_names_is_a_finding() {
        let mut person = valid_person();
        person.as_object_mut().unwrap().remove("givenNames");
        let findings = check_person(&person);
        assert!(findings.iter().any(|f| f.field == "givenNames"), "{findings:?}");
    }

    #[test]
    fn a_person_missing_family_names_is_a_finding() {
        let mut person = valid_person();
        person.as_object_mut().unwrap().remove("familyNames");
        let findings = check_person(&person);
        assert!(findings.iter().any(|f| f.field == "familyNames"), "{findings:?}");
    }

    #[test]
    fn a_person_missing_job_titles_is_a_finding() {
        let mut person = valid_person();
        person.as_object_mut().unwrap().remove("jobTitles");
        let findings = check_person(&person);
        assert!(findings.iter().any(|f| f.field == "jobTitles"), "{findings:?}");
    }

    #[test]
    fn a_person_with_an_empty_job_titles_array_is_no_finding() {
        // 59 committed persons hold `"jobTitles": []`, and the rule asks only that
        // the member be emitted.
        let mut person = valid_person();
        person["jobTitles"] = json!([]);
        assert!(check_person(&person).is_empty(), "{:?}", check_person(&person));
    }

    /// The corpus fact the rule above rests on, enumerated rather than taken on
    /// trust.
    #[test]
    fn the_committed_person_set_contains_people_with_no_job_title() {
        let mut without = 0;
        for file in json_files(&data_dir().join("persons")) {
            let person: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&file).expect("a person file reads"))
                    .expect("a person file parses");
            if person["jobTitles"].as_array().is_some_and(Vec::is_empty) {
                without += 1;
            }
        }
        assert_eq!(without, 59, "the empty-jobTitles rule is calibrated on this count");
    }

    #[test]
    fn a_person_with_only_blank_entries_in_a_required_array_is_a_finding() {
        let mut person = valid_person();
        person["givenNames"] = json!(["  ", ""]);
        let findings = check_person(&person);
        assert!(findings.iter().any(|f| f.field == "givenNames"), "{findings:?}");
    }

    #[test]
    fn a_project_leader_job_title_is_a_finding_at_its_index() {
        let mut person = valid_person();
        person["jobTitles"] = json!(["Senior lecturer", "Project Leader"]);
        let findings = check_person(&person);
        let finding = findings.iter().find(|f| f.field == "jobTitles").expect("a finding");
        assert_eq!(finding.index, Some(1));
    }

    #[test]
    fn a_full_professor_job_title_is_not_a_finding() {
        let mut person = valid_person();
        person["jobTitles"] = json!(["Full professor"]);
        assert!(check_person(&person).is_empty());
    }

    /// Why the role guard needs no carve-out of the kind the address rule has: no
    /// committed person holds a role word in `jobTitles`. If one lands, this
    /// fails and the guard needs the address rule's grandfathering.
    #[test]
    fn no_committed_person_holds_a_project_role_in_job_titles() {
        let mut offenders = Vec::new();
        for file in json_files(&data_dir().join("persons")) {
            let person: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&file).expect("a person file reads"))
                    .expect("a person file parses");
            let findings = check_person(&person);
            if findings
                .iter()
                .any(|finding| finding.field == "jobTitles" && finding.index.is_some())
            {
                offenders.push(person["id"].as_str().expect("an id").to_string());
            }
        }
        assert!(offenders.is_empty(), "{offenders:?}");
    }

    #[test]
    fn the_project_role_guard_ignores_case_and_surrounding_whitespace() {
        // `is_role_job_title` already trims and folds case; this pins that
        // `check_person` actually calls through to it rather than doing its own
        // (possibly stricter) comparison.
        let mut person = valid_person();
        person["jobTitles"] = json!(["  PROJECT LEADER  "]);
        let findings = check_person(&person);
        assert!(findings.iter().any(|f| f.field == "jobTitles"), "{findings:?}");
    }
}
