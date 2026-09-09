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
        let mut errors = Vec::new();
        for (dir, kind) in [(persons, AgentKind::Person), (organizations, AgentKind::Organization)] {
            read_agents_into(dir, kind, &mut by_id, &mut errors);
        }
        (Self { by_id }, errors)
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

    /// Every agent, in id order.
    pub fn all(&self) -> impl Iterator<Item = &Agent> {
        self.by_id.values()
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

/// Read one directory of agents of one kind, adding an error per file that did not load and per id
/// already held.
fn read_agents_into(dir: &Path, kind: AgentKind, by_id: &mut BTreeMap<String, Agent>, errors: &mut Vec<LoadError>) {
    let paths = match read_dir_sorted(dir) {
        Ok(paths) => paths,
        Err(message) => {
            errors.push(LoadError::Directory { path: dir.to_path_buf(), message });
            return;
        }
    };
    for file in paths {
        match read_agent(&file, kind) {
            Ok(agent) if by_id.contains_key(&agent.id) => errors.push(LoadError::File {
                path: file,
                message: format!("{} is already loaded; this file was ignored", agent.id),
            }),
            Ok(agent) => {
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

fn read_agent(path: &Path, kind: AgentKind) -> Result<Agent, String> {
    let json = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    match kind {
        AgentKind::Person => {
            let person: Person = serde_json::from_str(&json).map_err(|error| error.to_string())?;
            Ok(Agent { label: person_label(&person), id: person.id, kind })
        }
        AgentKind::Organization => {
            let organization: Organization = serde_json::from_str(&json).map_err(|error| error.to_string())?;
            Ok(Agent { label: organization.name, id: organization.id, kind })
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
    use super::*;

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
}
