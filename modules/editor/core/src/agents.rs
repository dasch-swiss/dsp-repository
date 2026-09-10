//! The persons and organizations a project's fields refer to by id.
//!
//! `contactPoint`, `attributions[].contributor` and `funding[].funders` hold ids rather than
//! values, so a form has to resolve one to a name and a submission naming one has to be checked
//! against what exists. Read-only: nothing here proposes a new person or organization.
//!
//! Loaded per `AppState` rather than through `dpe-core`'s caches, which are a process-wide
//! `OnceLock` keyed on a global data directory — under those, two tests with different fixtures see
//! each other's entities.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use platform_metadata::{Organization, Person};

use crate::proposals::{entity_id_number, EntityProposal, ProposalKind};
use crate::published::LoadError;

/// One agent as a form needs it: the id it is stored as, and a name to show.
///
/// Do not split this into an enum per kind: the two contract types have different name shapes, but
/// every consumer needs exactly the same two fields from both, so a variant per kind buys a match
/// at each call site and no invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    /// `person-001`, `organization-008`.
    pub id: String,
    /// What a reader is shown.
    pub label: String,
    pub kind: AgentKind,
}

/// Which set an agent came from.
///
/// Carried rather than inferred from the id prefix, which is only a filename convention in the
/// committed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentKind {
    Person,
    Organization,
}

impl AgentKind {
    /// The word shown beside an agent, so a list mixing both says which is which — an
    /// organization's name can read like a person's.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::Organization => "Organisation",
        }
    }
}

/// The persons and organizations available to refer to, keyed by id.
///
/// A `BTreeMap` so iteration is id order: an option order that depended on directory listing would
/// give a different page every restart and a different snapshot every test run.
#[derive(Debug, Default)]
pub struct Agents {
    by_id: BTreeMap<String, Agent>,
    /// Each entity's file body, verbatim.
    ///
    /// Kept whole, and not folded into [`Agent`] — whose docs argue against widening it — because
    /// accepting a `change` proposal writes its payload as the entity file. A payload seeded with
    /// only the members some form renders drops the rest silently, which is the property
    /// `ProjectDraft` gives a project (REQ-1.7, REQ-1.8). It is also the published side
    /// [`crate::proposals::check_organization`] compares an address against.
    bodies: BTreeMap<String, serde_json::Value>,
}

impl Agents {
    /// Read the `*.json` files in both directories into one set.
    ///
    /// Both are always loaded together: every field that refers to a person may refer to an
    /// organization, so a caller holding only half would refuse valid ids. A missing directory
    /// is one error and an empty set, not a failure to start.
    #[must_use]
    pub fn load_from(persons: &Path, organizations: &Path) -> (Self, Vec<LoadError>) {
        let mut by_id = BTreeMap::new();
        let mut bodies = BTreeMap::new();
        let mut errors = Vec::new();
        for (dir, kind) in [(persons, AgentKind::Person), (organizations, AgentKind::Organization)] {
            read_agents_into(dir, kind, &mut by_id, &mut bodies, &mut errors);
        }
        (Self { by_id, bodies }, errors)
    }

    /// One agent, or `None` for an id nothing holds.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.by_id.get(id)
    }

    /// Whether this id refers to something that exists.
    #[must_use]
    pub fn has(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// One entity's file body, verbatim, or `None` for an id nothing holds.
    ///
    /// See [`Self::bodies`] for why the whole body is kept. Callers wanting a *proposal payload*
    /// want [`Self::seed_payload`] instead — this one still carries `id`, which a payload must not.
    #[must_use]
    pub fn body(&self, id: &str) -> Option<&serde_json::Value> {
        self.bodies.get(id)
    }

    /// One entity's body as a proposal payload: the file body with `id` removed.
    ///
    /// `EntityProposal::payload` must not carry `id` — it lives in `entity_id`, the column the
    /// allocator and the uniqueness index work on, and a second copy would be free to drift from
    /// it. Stripping it here rather than at each call site is what keeps a seeded payload from
    /// being the one place that invariant is forgotten.
    #[must_use]
    pub fn seed_payload(&self, id: &str) -> Option<serde_json::Value> {
        let mut body = self.bodies.get(id)?.clone();
        if let Some(members) = body.as_object_mut() {
            members.remove("id");
        }
        Some(body)
    }

    /// Every agent, in id order.
    pub fn all(&self) -> impl Iterator<Item = &Agent> {
        self.by_id.values()
    }

    /// The highest id number this store holds for `kind`, or `0` where it holds none.
    ///
    /// This is the `published_floor` argument
    /// [`EntityProposalRepository::create_new`](crate::repository::EntityProposalRepository::create_new)
    /// takes: that layer allocates against its own table alone and cannot see this store, so the
    /// server has to pass in what it holds. Delegates to [`entity_id_number`] rather than
    /// re-parsing ids so the two cannot disagree on what shape an id of `kind` has.
    #[must_use]
    pub fn highest_id_number(&self, kind: ProposalKind) -> u32 {
        self.by_id.keys().filter_map(|id| entity_id_number(kind, id)).max().unwrap_or(0)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Agent resolution for one project: the published store, plus that project's own entity
/// proposals.
///
/// A depositor who proposes `person-417` has to be able to name it in `attributions` before it is
/// ever a file, but a proposal is per-request database state and cannot live in [`Agents`]'s
/// snapshot. This borrows the snapshot instead of cloning it: `AppState` is cloned per request and
/// the published store holds 558 agents, which is exactly why [`Agents`] sits behind an `Arc`
/// (see the `agents` field docs on `editor-server`'s `AppState`); copying it here to add a handful
/// of proposed rows would undo that.
#[derive(Debug)]
pub struct AgentScope<'a> {
    published: &'a Agents,
    proposed: Vec<Agent>,
}

impl<'a> AgentScope<'a> {
    /// The published store alone, for a surface with no project in hand.
    #[must_use]
    pub fn published_only(published: &'a Agents) -> Self {
        Self { published, proposed: Vec::new() }
    }

    /// The published store plus the referenceable proposals among `proposals`.
    ///
    /// Only [`EntityProposal::is_referenceable`] proposals contribute: a rejected or withdrawn one
    /// must not resolve, which is what lets the approval refuse a project still naming a rejected
    /// entity instead of shipping a dangling reference. A proposal whose payload cannot be parsed
    /// into the shape its kind promises contributes nothing either — see [`proposed_agent`].
    #[must_use]
    pub fn with_proposals(published: &'a Agents, proposals: &[EntityProposal]) -> Self {
        let proposed = proposals
            .iter()
            .filter(|proposal| proposal.is_referenceable())
            .filter_map(proposed_agent)
            .collect();
        Self { published, proposed }
    }

    /// One agent, or `None` for an id nothing holds.
    ///
    /// The published store wins over a proposed one: a `change` proposal names an id the store
    /// already holds, and resolving to the proposed payload would show a reviewer the proposed
    /// name where the surface is meant to show the published one beside it.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.published
            .get(id)
            .or_else(|| self.proposed.iter().find(|agent| agent.id == id))
    }

    /// Whether this id refers to something that exists.
    #[must_use]
    pub fn has(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// The **published** entity's body, ignoring any proposal for it.
    ///
    /// Deliberately not "the body this scope resolves": the two callers both want the published
    /// side specifically — one seeds a `change` proposal from it, the other compares a proposed
    /// address against it to decide whether an incomplete one was inherited or written. Answering
    /// with a proposal's payload would make a proposal grandfather itself.
    #[must_use]
    pub fn published_body(&self, id: &str) -> Option<&serde_json::Value> {
        self.published.body(id)
    }

    /// The published entity's body as a proposal payload — see [`Agents::seed_payload`].
    #[must_use]
    pub fn seed_payload(&self, id: &str) -> Option<serde_json::Value> {
        self.published.seed_payload(id)
    }

    /// Every agent: the published ones in id order, then the proposed ones the
    /// published store does not already answer for.
    ///
    /// The filter is [`Self::get`]'s precedence rule, applied to the listing so
    /// the two cannot disagree. Without it a `change` proposal — which by
    /// definition names an id the store already holds — put a second
    /// `<option>` into the shared `<datalist>` carrying the same `value` and a
    /// different label, so the picker offered one organisation twice under two
    /// names while `get` resolved only the published one.
    pub fn all(&self) -> impl Iterator<Item = &Agent> {
        self.published
            .all()
            .chain(self.proposed.iter().filter(|agent| self.published.get(&agent.id).is_none()))
    }

    /// How many distinct agents this scope resolves.
    ///
    /// Counted through [`Self::all`] rather than by adding the two sides, which
    /// would double-count every `change` proposal.
    #[must_use]
    pub fn len(&self) -> usize {
        self.all().count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.published.is_empty() && self.proposed.is_empty()
    }
}

/// One proposal's [`Agent`], or `None` when its payload does not parse into the shape its
/// [`ProposalKind`] promises.
///
/// A half-filled proposal is normal — `payload` is opaque JSON for exactly that reason — so an id
/// that resolves to a label nobody can compute is worse in a picker than an id that does not
/// resolve; the caller skips it rather than erroring. The label is built the same way
/// [`read_agent`] builds one: `person_label` for a person, `organization.name` for an
/// organisation.
///
/// `entity_id` is written in before the contract types, which require an `id`, and the insert
/// **overwrites** — so the column stays authoritative over anything the payload carries. See
/// `EntityProposal::payload` for why it should carry none.
fn proposed_agent(proposal: &EntityProposal) -> Option<Agent> {
    let mut payload: serde_json::Value = serde_json::from_str(&proposal.payload).ok()?;
    payload
        .as_object_mut()?
        .insert("id".to_string(), serde_json::Value::String(proposal.entity_id.clone()));
    let (label, kind) = match proposal.kind {
        ProposalKind::Person => {
            let person: Person = serde_json::from_value(payload).ok()?;
            (person_label(&person), AgentKind::Person)
        }
        ProposalKind::Organization => {
            let organization: Organization = serde_json::from_value(payload).ok()?;
            (organization.name, AgentKind::Organization)
        }
    };
    Some(Agent { id: proposal.entity_id.clone(), label, kind })
}

/// Read one directory of agents of one kind, adding an error per file that did not load and per id
/// already held.
fn read_agents_into(
    dir: &Path,
    kind: AgentKind,
    by_id: &mut BTreeMap<String, Agent>,
    bodies: &mut BTreeMap<String, serde_json::Value>,
    errors: &mut Vec<LoadError>,
) {
    let paths = match read_dir_sorted(dir) {
        Ok(paths) => paths,
        Err(message) => {
            errors.push(LoadError::Directory { path: dir.to_path_buf(), message });
            return;
        }
    };
    for file in paths {
        match read_agent(&file, kind) {
            Ok((agent, _)) if by_id.contains_key(&agent.id) => errors.push(LoadError::File {
                path: file,
                message: format!("{} is already loaded; this file was ignored", agent.id),
            }),
            Ok((agent, body)) => {
                bodies.insert(agent.id.clone(), body);
                by_id.insert(agent.id.clone(), agent);
            }
            Err(message) => errors.push(LoadError::File { path: file, message }),
        }
    }
}

/// The `*.json` files directly under `dir`, sorted, so which of two files claiming one id wins does
/// not depend on directory order.
fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// One agent and the file body it came from.
///
/// The body is returned beside the parsed agent rather than re-read later: this function already
/// holds the bytes, and a second read could see a different file.
fn read_agent(path: &Path, kind: AgentKind) -> Result<(Agent, serde_json::Value), String> {
    let json = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    match kind {
        AgentKind::Person => {
            let person: Person = serde_json::from_str(&json).map_err(|error| error.to_string())?;
            Ok((Agent { label: person_label(&person), id: person.id, kind }, body))
        }
        AgentKind::Organization => {
            let organization: Organization = serde_json::from_str(&json).map_err(|error| error.to_string())?;
            Ok((Agent { label: organization.name, id: organization.id, kind }, body))
        }
    }
}

/// A person's name as one line: given names then family names, in the order the file lists them.
///
/// Both members are `Vec<String>` on the contract, so joining is the only faithful rendering. A
/// person with neither falls back to the id, because a blank option in a picker is unselectable.
fn person_label(person: &Person) -> String {
    let name = person
        .given_names
        .iter()
        .chain(person.family_names.iter())
        .map(String::as_str)
        .collect::<Vec<&str>>()
        .join(" ");
    if name.trim().is_empty() {
        person.id.clone()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::proposals::{ProposalOperation, ProposalStatus};

    fn data_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data")
    }

    fn committed() -> (Agents, Vec<LoadError>) {
        let dir = data_dir();
        Agents::load_from(&dir.join("persons"), &dir.join("organizations"))
    }

    #[test]
    fn the_committed_entity_store_loads_without_errors() {
        let (agents, errors) = committed();
        assert!(errors.is_empty(), "{errors:?}");
        // A count rather than "not empty": a loader that silently read half the store shows up as
        // ids the form cannot resolve, which reads like a data problem.
        assert_eq!(agents.len(), 558, "the committed store is 416 persons plus 142 organizations");
    }

    #[test]
    fn every_id_the_committed_projects_refer_to_resolves() {
        // The property the form and the submit check both rest on: a reference that does not
        // resolve renders as a bare id and is refused on the next submit of a project
        // nobody edited.
        let (agents, _) = committed();
        let (published, errors) = crate::published::PublishedProjects::load_from(&data_dir().join("projects"));
        assert!(errors.is_empty(), "{errors:?}");

        let mut unresolved: Vec<String> = Vec::new();
        for summary in published.summaries() {
            let project = published.get(summary.shortcode).expect("a summary names a loaded project");
            let mut ids: Vec<&str> = Vec::new();
            ids.extend(project.contact_point.iter().flatten().map(String::as_str));
            ids.extend(project.attributions.iter().map(|a| a.contributor.as_str()));
            if let platform_metadata::project::Funding::Grants(grants) = &project.funding {
                ids.extend(grants.iter().flat_map(|grant| grant.funders.iter().map(String::as_str)));
            }
            for id in ids {
                if !agents.has(id) {
                    unresolved.push(format!("{}: {id}", summary.shortcode));
                }
            }
        }
        assert!(
            unresolved.is_empty(),
            "committed references that do not resolve: {unresolved:?}"
        );
    }

    #[test]
    fn a_person_and_an_organization_both_resolve_to_a_readable_label() {
        let (agents, _) = committed();
        let person = agents.get("person-001").expect("person-001 is committed");
        assert_eq!(person.kind, AgentKind::Person);
        assert_eq!(person.label, "Philippe Gonzalez");

        let organization = agents.get("organization-008").expect("organization-008 is committed");
        assert_eq!(organization.kind, AgentKind::Organization);
        assert_eq!(organization.label, "Dokumentationsbibliothek St. Moritz");
    }

    #[test]
    fn no_committed_agent_falls_back_to_its_id_for_a_label() {
        // The fallback exists because both name members are `Vec<String>` and could be empty; if it
        // fires for real data a picker shows `person-123` as an option.
        let (agents, _) = committed();
        let nameless: Vec<&str> = agents
            .all()
            .filter(|agent| agent.label == agent.id)
            .map(|agent| agent.id.as_str())
            .collect();
        assert!(nameless.is_empty(), "agents with no name: {nameless:?}");
    }

    #[test]
    fn highest_id_number_over_the_committed_store_matches_the_corpus_size() {
        // Same floors `proposals::tests` pins for `next_entity_id` against the identical corpus —
        // this is the value the server actually has in hand to pass as `published_floor`.
        let (agents, _) = committed();
        assert_eq!(agents.highest_id_number(ProposalKind::Person), 416);
        assert_eq!(agents.highest_id_number(ProposalKind::Organization), 142);
    }

    #[test]
    fn highest_id_number_over_an_empty_store_is_zero() {
        let agents = Agents::default();
        assert_eq!(agents.highest_id_number(ProposalKind::Person), 0);
        assert_eq!(agents.highest_id_number(ProposalKind::Organization), 0);
    }

    #[test]
    fn a_missing_directory_is_one_error_and_an_empty_set() {
        let (agents, errors) = Agents::load_from(
            Path::new("/nonexistent-editor-data/persons"),
            Path::new("/nonexistent-editor-data/organizations"),
        );
        assert!(agents.is_empty());
        assert_eq!(errors.len(), 2, "one per subdirectory: {errors:?}");
    }

    #[test]
    fn an_unparsable_file_names_itself_and_the_rest_still_load() {
        let dir = std::env::temp_dir().join("editor-agents-partial");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("persons")).expect("subdir");
        std::fs::create_dir_all(dir.join("organizations")).expect("subdir");
        std::fs::write(dir.join("persons/broken.json"), "{ not json").expect("write");
        std::fs::write(
            dir.join("persons/person-9.json"),
            r#"{"id":"person-9","givenNames":["A"],"familyNames":["B"],"jobTitles":[]}"#,
        )
        .expect("write");

        let (agents, errors) = Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert_eq!(agents.len(), 1, "the good file still loaded");
        assert!(agents.has("person-9"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].to_string().contains("broken.json"), "{}", errors[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn entity_proposal(
        kind: ProposalKind,
        operation: ProposalOperation,
        entity_id: &str,
        payload: &str,
        status: ProposalStatus,
    ) -> EntityProposal {
        EntityProposal {
            id: Uuid::nil(),
            shortcode: "0801".to_string(),
            entity_id: entity_id.to_string(),
            kind,
            operation,
            payload: payload.to_string(),
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
    fn published_only_resolves_a_committed_id_and_reports_the_published_count() {
        let (agents, _) = committed();
        let scope = AgentScope::published_only(&agents);
        assert_eq!(
            scope.get("person-001").map(|agent| agent.label.as_str()),
            Some("Philippe Gonzalez")
        );
        assert_eq!(scope.len(), 558);
    }

    #[test]
    fn with_proposals_resolves_a_proposed_person_to_its_payload_label() {
        let (agents, _) = committed();
        let payload = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;
        let proposal = entity_proposal(
            ProposalKind::Person,
            ProposalOperation::New,
            "person-417",
            payload,
            ProposalStatus::Submitted,
        );
        let scope = AgentScope::with_proposals(&agents, &[proposal]);
        assert_eq!(scope.get("person-417").map(|agent| agent.label.as_str()), Some("Ada Lovelace"));
        assert_eq!(scope.len(), 559);
    }

    #[test]
    fn a_rejected_or_withdrawn_proposal_does_not_resolve() {
        let (agents, _) = committed();
        let payload = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;
        for status in [ProposalStatus::Rejected, ProposalStatus::Withdrawn] {
            let proposal = entity_proposal(ProposalKind::Person, ProposalOperation::New, "person-417", payload, status);
            let scope = AgentScope::with_proposals(&agents, &[proposal]);
            assert!(!scope.has("person-417"), "{status}");
        }
    }

    #[test]
    fn a_draft_submitted_or_accepted_proposal_resolves() {
        let (agents, _) = committed();
        let payload = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;
        for status in [
            ProposalStatus::Draft,
            ProposalStatus::Submitted,
            ProposalStatus::Accepted,
        ] {
            let proposal = entity_proposal(ProposalKind::Person, ProposalOperation::New, "person-417", payload, status);
            let scope = AgentScope::with_proposals(&agents, &[proposal]);
            assert!(scope.has("person-417"), "{status}");
        }
    }

    #[test]
    fn a_change_proposal_resolves_to_the_published_name_not_the_proposed_one() {
        let (agents, _) = committed();
        let payload = r#"{"name":"A Different Name","url":"https://example.org/"}"#;
        let proposal = entity_proposal(
            ProposalKind::Organization,
            ProposalOperation::Change,
            "organization-008",
            payload,
            ProposalStatus::Submitted,
        );
        let scope = AgentScope::with_proposals(&agents, &[proposal]);
        assert_eq!(
            scope.get("organization-008").map(|agent| agent.label.as_str()),
            Some("Dokumentationsbibliothek St. Moritz")
        );
    }

    #[test]
    fn a_change_proposal_does_not_offer_its_entity_twice_in_the_listing() {
        // `all()` feeds the shared `<datalist>`, one `<option value=id>` per
        // agent. A change proposal names an id the published store already
        // holds, so listing both put two options with the same `value` and
        // different labels into the picker — one organisation offered twice
        // under two names, while `get` resolved only the published one. The
        // listing has to agree with the lookup.
        let (agents, _) = committed();
        let proposal = entity_proposal(
            ProposalKind::Organization,
            ProposalOperation::Change,
            "organization-008",
            r#"{"name":"A Different Name","url":"https://example.org/"}"#,
            ProposalStatus::Submitted,
        );
        let scope = AgentScope::with_proposals(&agents, &[proposal]);
        assert_eq!(scope.len(), 558, "a change proposal adds no agent");
        assert_eq!(
            scope.all().filter(|agent| agent.id == "organization-008").count(),
            1,
            "organization-008 must be offered exactly once"
        );
    }

    #[test]
    fn the_entity_id_column_overrides_an_id_inside_the_payload() {
        // The payload must not carry `id` at all, but if one arrives it cannot
        // be allowed to win: the allocator and the uniqueness index work on
        // `entity_id`, so an entity resolving under a payload-supplied id would
        // be resolving under one nothing claimed.
        let (agents, _) = committed();
        let proposal = entity_proposal(
            ProposalKind::Person,
            ProposalOperation::New,
            "person-417",
            r#"{"id":"person-999","givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#,
            ProposalStatus::Submitted,
        );
        let scope = AgentScope::with_proposals(&agents, &[proposal]);
        assert!(scope.get("person-417").is_some(), "the allocated id resolves");
        assert!(scope.get("person-999").is_none(), "the payload's id does not");
    }

    #[test]
    fn an_unparsable_proposal_payload_resolves_to_nothing_and_does_not_panic() {
        let (agents, _) = committed();
        for payload in ["{}", "not json"] {
            let proposal = entity_proposal(
                ProposalKind::Person,
                ProposalOperation::New,
                "person-417",
                payload,
                ProposalStatus::Submitted,
            );
            let scope = AgentScope::with_proposals(&agents, &[proposal]);
            assert!(!scope.has("person-417"), "{payload}");
            assert_eq!(scope.len(), 558);
        }
    }

    #[test]
    fn all_yields_the_published_agents_in_id_order_then_the_proposed_ones() {
        let (agents, _) = committed();
        let payload = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;
        let proposal = entity_proposal(
            ProposalKind::Person,
            ProposalOperation::New,
            "person-417",
            payload,
            ProposalStatus::Submitted,
        );
        let scope = AgentScope::with_proposals(&agents, &[proposal]);
        let ids: Vec<&str> = scope.all().map(|agent| agent.id.as_str()).collect();
        // Boundary assertion rather than re-deriving the published set's own order, which
        // `the_committed_entity_store_loads_without_errors` and its siblings already pin.
        assert_eq!(ids.first(), Some(&"organization-001"));
        assert_eq!(ids.last(), Some(&"person-417"));
    }
}
