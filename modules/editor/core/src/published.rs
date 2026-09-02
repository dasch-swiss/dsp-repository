//! The published project set, as baked into the deployment.
//!
//! REQ-1.1 pre-fills the form with "the current published metadata", and REQ-2.3
//! compares the published set against local records at startup. Both need the
//! `projects/*.json` files the image carries, read once and held in memory: the
//! set cannot change without a redeployment, so nothing polls and nothing
//! invalidates.
//!
//! Filesystem access rather than a repository port. The ports in
//! [`crate::repository`] exist because the editor *writes* through them and a
//! test has to be able to make a write fail; this is a read of an immutable
//! snapshot, so a trait would buy an indirection with one implementation. The
//! loader takes a directory, which is all a test needs.
//!
//! ## The `shortcode` field is the key, not the filename
//!
//! Five of the 85 committed files disagree with the shortcode they hold —
//! `projects/0801_bebb.json` is project `0801d`, and its four siblings under
//! `0801_*` are `0801a` through `0801e`. Keying on the filename stem would file
//! all five under `0801`, a shortcode no project actually has: all five would be
//! unreachable by the code they are addressed by, and four would be dropped as
//! duplicates of the first.
//!
//! ## Lookup folds case
//!
//! 24 of the 85 shortcodes are mixed case (`080C`, `081B`, `085F`). Folding
//! matches [`User::may_reach`](crate::records::User::may_reach), which folds for
//! the same reason: which half of a shortcode is capitalised is not something a
//! person typing one can be expected to get right. No two committed shortcodes
//! collide when folded, so nothing is made ambiguous by it — asserted by
//! [`PublishedProjects::load_from`] returning a
//! [`LoadError::DuplicateShortcode`] rather than letting one file quietly
//! replace another.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use platform_metadata::project::ProjectRaw;

/// A published project could not be read.
///
/// One per file, so a single bad file names itself instead of collapsing the
/// whole load. None of these is fatal: a service that refuses to start because
/// one of 85 snapshots is malformed is worse than one serving the other 84 and
/// saying which is missing.
#[derive(Debug)]
pub enum LoadError {
    /// The directory itself could not be listed.
    Directory { path: PathBuf, message: String },
    /// One file could not be read or parsed.
    File { path: PathBuf, message: String },
    /// Two files claim the same shortcode, ignoring case. The second is dropped;
    /// silently overwriting would make which project answers depend on
    /// directory order.
    DuplicateShortcode { path: PathBuf, shortcode: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory { path, message } => {
                write!(f, "could not list {}: {message}", path.display())
            }
            Self::File { path, message } => {
                write!(f, "could not read {}: {message}", path.display())
            }
            Self::DuplicateShortcode { path, shortcode } => write!(
                f,
                "{} claims shortcode {shortcode}, which another file already holds; it was ignored",
                path.display()
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// One project as a list needs it: enough to render a row, without the caller
/// holding a whole `ProjectRaw` per line.
///
/// Borrowed from the loaded set rather than owned, so building a list allocates
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectSummary<'a> {
    /// As written in the file, not folded — this is what a link and a heading
    /// show.
    pub shortcode: &'a str,
    pub name: &'a str,
    /// `"ongoing"` or `"finished"`.
    pub status: &'a str,
}

/// The published projects, keyed by case-folded shortcode.
///
/// A `BTreeMap` so iteration is shortcode order, which is the order a list of
/// projects should appear in and one fewer thing for a caller to sort.
#[derive(Debug, Default)]
pub struct PublishedProjects {
    by_shortcode: BTreeMap<String, ProjectRaw>,
}

impl PublishedProjects {
    /// Read every `*.json` file directly under `dir`.
    ///
    /// Returns whatever loaded plus one [`LoadError`] per file that did not, so
    /// the caller can log each and carry on. An unreadable directory yields an
    /// empty set and one error — the deployment has no snapshot, which is a
    /// condition to report rather than a reason to refuse to start.
    ///
    /// Only the top level is read: `dir` also holds `persons/`,
    /// `organizations/` and `clusters/`, and recursing would file an
    /// organization under whatever shortcode its JSON happened to parse as.
    #[must_use]
    pub fn load_from(dir: &Path) -> (Self, Vec<LoadError>) {
        let mut errors = Vec::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(LoadError::Directory { path: dir.to_path_buf(), message: error.to_string() });
                return (Self::default(), errors);
            }
        };

        // Collected and sorted before parsing so that which of two files
        // claiming one shortcode wins does not depend on directory order — the
        // report has to name the same file every run to be actionable.
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "json") {
                        paths.push(path);
                    }
                }
                Err(error) => errors.push(LoadError::Directory { path: dir.to_path_buf(), message: error.to_string() }),
            }
        }
        paths.sort();

        let mut by_shortcode = BTreeMap::new();
        for path in paths {
            match read_project(&path) {
                Ok(project) => {
                    let key = fold(&project.shortcode);
                    if by_shortcode.contains_key(&key) {
                        errors.push(LoadError::DuplicateShortcode { path, shortcode: project.shortcode.clone() });
                        continue;
                    }
                    by_shortcode.insert(key, project);
                }
                Err(message) => errors.push(LoadError::File { path, message }),
            }
        }
        (Self { by_shortcode }, errors)
    }

    /// One published project, or `None` when the set has no such shortcode.
    ///
    /// `None` does **not** mean the project does not exist: REQ-2.3 allows a
    /// project that exists only locally, which has no published counterpart and
    /// whose form opens blank. Callers deciding a 404 have to consult local
    /// records too.
    #[must_use]
    pub fn get(&self, shortcode: &str) -> Option<&ProjectRaw> {
        self.by_shortcode.get(&fold(shortcode))
    }

    /// Every project, in shortcode order.
    pub fn summaries(&self) -> impl Iterator<Item = ProjectSummary<'_>> {
        self.by_shortcode.values().map(summary)
    }

    /// The projects named by `shortcodes`, in shortcode order, skipping any the
    /// set does not hold.
    ///
    /// For a depositor's list: the assignments are the user's, the order and the
    /// names are the set's. An assignment naming no published project is skipped
    /// rather than rendered as a broken row — a project assigned before it is
    /// published is a real state, and REQ-2.3's local-only project is the same
    /// shape.
    pub fn summaries_for<'a>(&'a self, shortcodes: &'a [String]) -> impl Iterator<Item = ProjectSummary<'a>> {
        self.by_shortcode
            .iter()
            .filter(|(key, _)| shortcodes.iter().any(|assigned| &fold(assigned) == *key))
            .map(|(_, project)| summary(project))
    }

    /// How many projects loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_shortcode.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_shortcode.is_empty()
    }
}

fn summary(project: &ProjectRaw) -> ProjectSummary<'_> {
    ProjectSummary {
        shortcode: &project.shortcode,
        name: &project.name,
        status: project.status.as_str(),
    }
}

/// The lookup key: a shortcode with case folded away.
fn fold(shortcode: &str) -> String {
    shortcode.trim().to_ascii_lowercase()
}

fn read_project(path: &Path) -> Result<ProjectRaw, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::test_support::sample_raw;

    /// The committed corpus, which the round-trip test also reads.
    fn corpus() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects")
    }

    /// A directory holding `files` as `<name>.json`, removed by the caller.
    fn dir_with(name: &str, files: &[(&str, String)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("editor-published-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        for (file, body) in files {
            std::fs::write(dir.join(format!("{file}.json")), body).expect("write");
        }
        dir
    }

    fn project_json(shortcode: &str, name: &str) -> String {
        let mut project = sample_raw();
        project.shortcode = shortcode.to_string();
        project.name = name.to_string();
        serde_json::to_string(&project).expect("serializes")
    }

    #[test]
    fn the_whole_committed_corpus_loads_with_no_errors() {
        // Enumerated, not sampled: a file the editor cannot read is a project
        // whose form cannot be pre-filled, and the count is the only thing that
        // would show it.
        let (published, errors) = PublishedProjects::load_from(&corpus());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(published.len(), 85);
    }

    #[test]
    fn a_project_is_keyed_by_its_shortcode_field_not_its_filename() {
        // `projects/0801_bebb.json` holds project `0801d`, and its four siblings
        // under `0801_*` are `0801a`..`0801e`. Keying on the filename stem would
        // file all five under `0801` — a shortcode no project actually has — so
        // all five would be unreachable by the code they are addressed by, and
        // four of them would be dropped as duplicates on top.
        let (published, _) = PublishedProjects::load_from(&corpus());
        for shortcode in ["0801a", "0801b", "0801c", "0801d", "0801e"] {
            assert!(published.get(shortcode).is_some(), "{shortcode} should be reachable");
        }
        assert_eq!(published.get("0801d").map(|p| p.shortcode.as_str()), Some("0801d"));
        assert!(published.get("0801").is_none(), "no project has the bare shortcode 0801");
    }

    #[test]
    fn lookup_folds_case_because_24_committed_shortcodes_are_mixed_case() {
        let (published, _) = PublishedProjects::load_from(&corpus());
        assert!(published.get("080C").is_some());
        assert!(published.get("080c").is_some());
        assert_eq!(
            published.get("080c").map(|p| p.shortcode.as_str()),
            published.get("080C").map(|p| p.shortcode.as_str())
        );
        // The stored spelling is what a heading and a link show, not the key.
        assert_eq!(published.get("080c").map(|p| p.shortcode.as_str()), Some("080C"));
    }

    #[test]
    fn a_missing_directory_is_an_empty_set_and_one_error_not_a_panic() {
        // A deployment with no snapshot is a condition to report, not a reason
        // to refuse to start.
        let (published, errors) = PublishedProjects::load_from(Path::new("no-such-directory"));
        assert!(published.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], LoadError::Directory { .. }), "{errors:?}");
    }

    #[test]
    fn one_malformed_file_does_not_take_the_others_with_it() {
        let dir = dir_with(
            "malformed",
            &[
                ("good", project_json("0901", "A Good Project")),
                ("broken", "{ not json".to_string()),
            ],
        );
        let (published, errors) = PublishedProjects::load_from(&dir);
        assert_eq!(published.len(), 1);
        assert!(published.get("0901").is_some());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("broken.json"), "{}", errors[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_files_claiming_one_shortcode_is_reported_rather_than_silently_resolved() {
        // Overwriting would make which project answers depend on directory
        // order, so the same request could serve different data per deployment.
        let dir = dir_with(
            "duplicate",
            &[
                ("a_first", project_json("0902", "First")),
                ("b_second", project_json("0902", "Second")),
            ],
        );
        let (published, errors) = PublishedProjects::load_from(&dir);
        assert_eq!(published.len(), 1);
        assert_eq!(published.get("0902").map(|p| p.name.as_str()), Some("First"));
        assert!(
            matches!(&errors[..], [LoadError::DuplicateShortcode { shortcode, .. }] if shortcode == "0902"),
            "{errors:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_case_folded_duplicate_is_a_duplicate() {
        let dir = dir_with(
            "folded-duplicate",
            &[
                ("a_lower", project_json("090c", "Lower")),
                ("b_upper", project_json("090C", "Upper")),
            ],
        );
        let (published, errors) = PublishedProjects::load_from(&dir);
        assert_eq!(published.len(), 1);
        assert_eq!(errors.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_json_files_directly_in_the_directory_are_read() {
        // The data directory also holds `persons/`, `organizations/` and
        // `clusters/`; recursing would file an organization under whatever
        // shortcode its JSON happened to parse as.
        let dir = dir_with("mixed", &[("project", project_json("0903", "A Project"))]);
        std::fs::write(dir.join("notes.txt"), "not a project").expect("write");
        std::fs::create_dir_all(dir.join("persons")).expect("subdir");
        std::fs::write(dir.join("persons/someone.json"), "{}").expect("write");
        let (published, errors) = PublishedProjects::load_from(&dir);
        assert_eq!(published.len(), 1);
        assert!(errors.is_empty(), "{errors:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn summaries_are_in_shortcode_order() {
        let (published, _) = PublishedProjects::load_from(&corpus());
        let codes: Vec<&str> = published.summaries().map(|s| s.shortcode).collect();
        let mut sorted = codes.clone();
        sorted.sort_by_key(|code| code.to_ascii_lowercase());
        assert_eq!(codes, sorted);
        assert_eq!(codes.len(), 85);
    }

    #[test]
    fn a_summary_carries_what_a_list_row_needs() {
        let (published, _) = PublishedProjects::load_from(&corpus());
        let summary = published.summaries().find(|s| s.shortcode == "0801d").expect("0801d");
        assert!(!summary.name.is_empty());
        assert!(matches!(summary.status, "ongoing" | "finished"), "{}", summary.status);
    }

    #[test]
    fn summaries_for_returns_only_the_assigned_projects_folding_case() {
        let (published, _) = PublishedProjects::load_from(&corpus());
        let assigned = vec!["080c".to_string(), "0801d".to_string()];
        let codes: Vec<&str> = published.summaries_for(&assigned).map(|s| s.shortcode).collect();
        assert_eq!(codes, ["0801d", "080C"]);
    }

    #[test]
    fn an_assignment_naming_no_published_project_is_skipped_not_rendered_broken() {
        // A project assigned before it is published is a real state, and so is
        // REQ-2.3's local-only project.
        let (published, _) = PublishedProjects::load_from(&corpus());
        let assigned = vec!["0801d".to_string(), "9999".to_string()];
        let codes: Vec<&str> = published.summaries_for(&assigned).map(|s| s.shortcode).collect();
        assert_eq!(codes, ["0801d"]);
    }

    #[test]
    fn an_unpublished_shortcode_is_absent_rather_than_an_error() {
        // REQ-2.3: absent from the published set is not "does not exist".
        let (published, _) = PublishedProjects::load_from(&corpus());
        assert!(published.get("9999").is_none());
    }
}
