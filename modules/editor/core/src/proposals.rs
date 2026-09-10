//! Proposals to add or change a person or organisation (REQ-3.x).
//!
//! A project field that refers to a contributor by id (`contactPoint`,
//! `attributions[].contributor`, `funding[].funders`) can only name something
//! that already exists, so a depositor who needs a new person or organisation,
//! or a correction to an existing one, cannot express that inside the project
//! draft. A proposal is the separate record that carries such a request
//! through its own review, independent of the project submission it may ride
//! alongside.
//!
//! Framework-free, like the rest of this crate: no `rusqlite`, no Axum, no
//! Maud. `payload` is opaque JSON for the same reason
//! [`DraftRecord::payload`](crate::records::DraftRecord::payload) is — a
//! proposal can be half-filled, and only [`check_person`]/[`check_organization`]
//! decide whether it is complete enough to submit.
//!
//! [`next_entity_id`] is monotonic and never reuses a number, so the sequence has gaps by design:
//! reuse would let two entities carry one id at different times, and anything collected while the
//! first was live would then resolve to the second. The editor architecture documentation carries
//! the rest.

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
    /// is which. British spelling for the organisation, matching
    /// [`AgentKind::label`](crate::agents::AgentKind::label) — the two label an
    /// entity of the same kind and must not disagree on how it reads.
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
    /// A person or organisation that does not exist yet (REQ-3.1).
    New,
    /// A change to an entity the project already references (REQ-3.2).
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
/// Not [`review::Decision`](crate::review::Decision), whose second variant is
/// `Revert` — "keep the published value". A proposed *new* entity has no
/// published value to keep, so the only honest opposite of accepting one is
/// rejecting it. The two vocabularies are therefore separate rather than shared,
/// and a control on the review surface posts this one for a proposal and that
/// one for a project field.
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
    /// The proposed entity as JSON — a `platform_metadata::Person` or
    /// `Organization` body. Opaque here, like
    /// [`DraftRecord::payload`](crate::records::DraftRecord::payload): a
    /// half-filled proposal cannot deserialize as the contract type, and
    /// deciding whether it is complete is [`check_person`]'s and
    /// [`check_organization`]'s job.
    ///
    /// **A producer must leave `id` out.** It lives in [`Self::entity_id`], the
    /// column the uniqueness index and the allocator work on; a second copy
    /// here would be free to drift from it. Readers fill it in from
    /// `entity_id` and overwrite whatever they find.
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
    /// reviewer may still decide it.
    ///
    /// True for [`ProposalStatus::Draft`] and [`ProposalStatus::Submitted`].
    /// [`ProposalStatus::Accepted`] is excluded because nothing about it can
    /// change any more — it is on its way into a `persons`/`organizations`
    /// file — and the two terminal statuses because they are over.
    ///
    /// **This says nothing about the allocated id.** Every row holds its id
    /// permanently, terminal ones included, which is why they stay in the
    /// uniqueness index; see the module docs on why reuse is refused. Reading
    /// this as "the id is free again" is the mistake that would hand an
    /// already-collected id to a second entity.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self.status, ProposalStatus::Draft | ProposalStatus::Submitted)
    }

    /// Whether a project field may name this proposal's `entity_id` without
    /// submit refusing the reference.
    ///
    /// Wider than [`Self::is_live`] by exactly [`ProposalStatus::Accepted`]: an
    /// accepted proposal's entity is not a file yet, so the published store
    /// cannot answer for it, and the project that proposed it is referring to
    /// something that will exist. Narrower than "every row" by the two terminal
    /// statuses, which is the whole point of the referential-integrity gate —
    /// a rejected entity must make the approval refuse rather than resolve.
    #[must_use]
    pub fn is_referenceable(&self) -> bool {
        matches!(
            self.status,
            ProposalStatus::Draft | ProposalStatus::Submitted | ProposalStatus::Accepted
        )
    }
}

/// The numeric suffix of `id`, if `id` has the shape `{kind.as_str()}-N`.
///
/// `None` for a different kind's prefix, a missing or non-numeric suffix, or a
/// number too large for `u32` — every one of those is "not an id of this
/// kind" rather than a value to recover from.
#[must_use]
pub fn entity_id_number(kind: ProposalKind, id: &str) -> Option<u32> {
    let suffix = id.strip_prefix(kind.as_str())?.strip_prefix('-')?;
    suffix.parse().ok()
}

/// Format an allocated id: `person-417`, `organization-143`.
///
/// Zero-padded to at least three digits so an allocated id is spelled the way
/// the committed corpus spells one (`person-001` .. `person-416`); a number
/// past 999 is written in full rather than truncated.
///
/// The padding is cosmetic, **not** an ordering guarantee. It happens to make
/// string order agree with numeric order while every id has three digits, and
/// stops doing so at `person-1000` — which is why allocation parses these back
/// to numbers rather than comparing them as strings, and why the store must not
/// answer "the highest id" with a TEXT `MAX`.
#[must_use]
pub fn format_entity_id(kind: ProposalKind, number: u32) -> String {
    format!("{}-{number:03}", kind.as_str())
}

/// The next id to allocate for `kind`, given every id already taken — the union
/// of the published store and every id this editor has ever allocated,
/// terminal proposals included (see the module docs on why reuse is refused).
///
/// One past the highest number found for `kind` in `taken`, or `1` if none is.
/// Entries of the other kind, or that do not parse as this kind's shape, are
/// ignored rather than rejected: `taken` is expected to be an unfiltered union
/// of two id spaces.
#[must_use]
pub fn next_entity_id<'a>(kind: ProposalKind, taken: impl Iterator<Item = &'a str>) -> String {
    let highest = taken.filter_map(|id| entity_id_number(kind, id)).max().unwrap_or(0);
    // Saturating, not `+ 1`: `entity_id_number` accepts any `u32`, and `taken`
    // includes ids read from committed files, so a hand-authored
    // `person-4294967295.json` would overflow here — a panic in a debug build
    // and a wrap to `person-000` in a release one. Saturating leaves the
    // allocation to fail closed on the store's uniqueness constraint instead of
    // handing out an id that is already taken.
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

/// Every rule a proposed organisation must satisfy (REQ-3.4), against
/// `modules/platform/metadata/src/organization.rs`'s `Organization` shape.
///
/// `published` is the entity as the committed store holds it, for a
/// [`ProposalOperation::Change`], and `None` for a
/// [`ProposalOperation::New`] — there is nothing to compare a new entity
/// against.
///
/// **An `address` byte-equal to `published`'s passes; any other incomplete one
/// is refused.** REQ-3.4 wants all four of `street`, `postalCode`, `locality`
/// and `country` or none, but six committed organizations satisfy neither, so
/// judging an inherited value would make them unchangeable without inventing
/// data. The editor architecture documentation names the six and the precedent;
/// [`tests::six_committed_organizations_carry_an_incomplete_address`] pins them.
///
/// `sameAs` carries no rule beyond being where authority identifiers go: the
/// contract types it as `Vec<AuthorityFileReference>` with a serde default, so
/// an absent one already deserializes, and REQ-3.4 does not ask for one. The
/// omission here is therefore decided, not forgotten.
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

    // `address` itself is optional (REQ-3.4: collect all four members or omit
    // the section entirely). Once present, each of the four is required even
    // if every one is blank — a section the depositor opened and left blank is
    // not the same as one they never opened, and silently dropping it would
    // discard what they typed rather than telling them to finish it.
    //
    // Unless it is exactly what the published entity already held: see the
    // grandfathering paragraph in this function's docs for the six committed
    // organizations that rule would otherwise strand.
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

/// Every rule a proposed person must satisfy (REQ-3.5), against
/// `modules/platform/metadata/src/person.rs`'s `Person` shape, including the
/// project-role guard.
///
/// `sameAs` carries no rule for the same reason as [`check_organization`]'s:
/// REQ-3.5 places ORCID there rather than constraining it, and the contract
/// already defaults an absent one.
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

    // Present, and no more: REQ-3.5 asks that it be emitted and a non-defaulted
    // `Vec<String>` is satisfied by `[]`, which many committed persons are.
    //
    // `givenNames` and `familyNames` above do carry the stronger rule, because
    // none of the 416 is empty and a person with neither name renders as its own
    // id, which `agents::person_label` documents as unselectable in a picker.
    if !payload.get("jobTitles").is_some_and(serde_json::Value::is_array) {
        findings.push(EntityFinding {
            field: "jobTitles",
            index: None,
            message: "jobTitles must be present, though it may be empty".to_string(),
        });
    }

    // The guard the issue adds beyond the PRD: `dpe-server validate` rejects a
    // committed file carrying a `platform_metadata::JOB_TITLE_ROLE_WORDS` entry
    // in `jobTitles` (modules/dpe/server/src/main.rs:568), because it is
    // invisible there to the OAI-PMH creator/contributor logic, which only
    // reads `attributions`. Without this check the editor could hand a
    // depositor's proposal straight through to a file that fails that
    // validation later.
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
/// provided" — a form field can be absent, wrongly typed, or spaces alone, and
/// REQ-3.4/3.5 treat the three as one failure.
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
        // `review::Decision`'s stored forms are `accept` and `revert`. The two
        // vocabularies share a word, so reading either as the other has to fail
        // rather than land on whichever variant sorts first — the argument
        // `records::test_an_unknown_stored_review_outcome_is_an_error` makes
        // about `submitted` and `approved`.
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

    /// Allocation must not degrade into string comparison, which is the shape the
    /// three-digit padding invites once ids pass 999.
    ///
    /// Over these four, lexicographic order puts `person-99` last and would
    /// allocate `person-100` — an id below two already taken, which then
    /// collides on the store's uniqueness index for every later proposal.
    /// SQLite's `MAX(entity_id)` on a TEXT column answers exactly that, so this
    /// pins the arithmetic against the optimisation somebody will reach for.
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
        // Mirrors `published::tests::corpus()`: the committed store this editor
        // allocates against.
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data")
    }

    /// Every entity id in `dir`, read from each file's `id` **member**.
    ///
    /// Not from the filename stem, although all 558 committed files currently
    /// agree with theirs: `agents::Agents::load_from` keys on the `id` member,
    /// so allocating against filenames would be testing a set the runtime never
    /// sees. `published`'s module docs record what that costs when the two
    /// diverge — five of the 85 project files disagree with their own
    /// `shortcode`, and keying on the stem filed all five under a shortcode no
    /// project has.
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
        // `organization-065` (Tanta University) is committed with no
        // `postalCode`. A depositor proposing any other change to it must not be
        // made to invent one, so an address byte-equal to the published one is
        // passed through — the carve-out this function's docs set out.
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
        // `None` published side: a new entity has nothing to inherit, so an
        // incomplete address is refused however it arrived.
        let mut org = valid_organization();
        org["address"] = json!({ "street": "", "postalCode": "1015", "locality": "Lausanne", "country": "CH" });
        assert!(!check_organization(&org, None).is_empty());
    }

    /// The corpus fact the carve-out rests on, enumerated rather than assumed.
    /// If a data change completed all six addresses, the carve-out would have no
    /// reason to exist and this says so.
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
        // The depositor opened the section and typed nothing into it; that is
        // not the same as never opening it, and must not be silently dropped.
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
        // 59 of the 416 committed persons hold `"jobTitles": []`, and REQ-3.5
        // asks only that the member be emitted. Demanding an entry would refuse
        // a proposal for somebody who has no job title with no correct value to
        // type, while the published set carries 59 people in that exact state.
        let mut person = valid_person();
        person["jobTitles"] = json!([]);
        assert!(check_person(&person).is_empty(), "{:?}", check_person(&person));
    }

    /// The corpus fact the rule above rests on, enumerated rather than taken on
    /// trust: if a future data change made every committed person carry a job
    /// title, the weaker rule would no longer have a reason to exist.
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

    /// Why the role guard needs no carve-out of the kind the address rule has.
    ///
    /// None of the 416 committed persons holds a `JOB_TITLE_ROLE_WORDS` entry in
    /// `jobTitles`, which is also why `dpe-server validate` passes on the
    /// corpus. So the guard can be unconditional: applying it to a change
    /// proposal cannot refuse anybody over data they did not write. If a role
    /// word ever lands in the committed set, this fails and the guard needs the
    /// same grandfathering `check_organization`'s address rule has.
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
