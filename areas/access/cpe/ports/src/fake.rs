use std::collections::BTreeMap;
use std::io;
use std::sync::{Mutex, PoisonError};

use crate::{
    is_file_name, is_shortcode, ArchiveProjection, DepositedFiles, FilesError, ProjectSnapshot, ProjectionError,
};

/// An in-memory [`ArchiveProjection`] for CPE's tests.
///
/// Serves each registered snapshot exactly as given, every `Vec` in its original order, and can
/// simulate an unknown or an unavailable project. The last registration for a shortcode wins.
#[derive(Debug, Clone, Default)]
pub struct FakeArchiveProjection {
    projects: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone)]
enum Entry {
    Snapshot(ProjectSnapshot),
    Unavailable,
}

impl FakeArchiveProjection {
    /// Serve `snapshot` under its own `shortcode`.
    #[must_use]
    pub fn with_project(mut self, snapshot: ProjectSnapshot) -> Self {
        self.projects.insert(snapshot.shortcode.clone(), Entry::Snapshot(snapshot));
        self
    }

    /// Answer `shortcode` with [`ProjectionError::Unavailable`].
    #[must_use]
    pub fn with_unavailable(mut self, shortcode: &str) -> Self {
        self.projects.insert(shortcode.to_string(), Entry::Unavailable);
        self
    }
}

impl ArchiveProjection for FakeArchiveProjection {
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
        match self.projects.get(shortcode) {
            Some(Entry::Snapshot(snapshot)) => Ok(snapshot.clone()),
            Some(Entry::Unavailable) => Err(ProjectionError::unavailable(io::Error::other("simulated outage"))),
            None => Err(ProjectionError::UnknownProject { shortcode: shortcode.to_string() }),
        }
    }
}

/// An in-memory [`DepositedFiles`] for CPE's tests.
///
/// Serves each registered file exactly as given, can simulate an unavailable project or file, and
/// records every request. The last registration for a project wins: a file registered after
/// [`with_unavailable`](Self::with_unavailable) makes the project available again with that file
/// alone, and `with_unavailable` after a file drops the project's files. A shortcode that fails
/// [`is_shortcode`] and a name that fails [`is_file_name`] are refused whatever was registered,
/// as [`DepositedFiles::file`] requires of every adapter.
#[derive(Debug, Default)]
pub struct FakeDepositedFiles {
    projects: BTreeMap<String, Project>,
    requests: Mutex<Vec<(String, String)>>,
}

#[derive(Debug)]
enum Project {
    Files(BTreeMap<String, FileEntry>),
    Unavailable,
}

#[derive(Debug)]
enum FileEntry {
    Bytes(Vec<u8>),
    Unavailable,
}

impl FakeDepositedFiles {
    /// Serve `bytes` as `shortcode`'s file `name`.
    #[must_use]
    pub fn with_file(self, shortcode: &str, name: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.with_entry(shortcode, name, FileEntry::Bytes(bytes.into()))
    }

    /// Answer every file of `shortcode` with [`FilesError::Unavailable`].
    #[must_use]
    pub fn with_unavailable(mut self, shortcode: &str) -> Self {
        self.projects.insert(shortcode.to_string(), Project::Unavailable);
        self
    }

    /// Answer `shortcode`'s file `name` with [`FilesError::Unavailable`], and no other.
    #[must_use]
    pub fn with_unavailable_file(self, shortcode: &str, name: &str) -> Self {
        self.with_entry(shortcode, name, FileEntry::Unavailable)
    }

    /// Every `(shortcode, name)` asked for, in order, refused names included.
    #[must_use]
    pub fn requests(&self) -> Vec<(String, String)> {
        self.requests.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn with_entry(mut self, shortcode: &str, name: &str, entry: FileEntry) -> Self {
        match self.projects.entry(shortcode.to_string()).or_insert(Project::Unavailable) {
            Project::Files(files) => {
                files.insert(name.to_string(), entry);
            }
            project @ Project::Unavailable => *project = Project::Files(BTreeMap::from([(name.to_string(), entry)])),
        }
        self
    }
}

impl DepositedFiles for FakeDepositedFiles {
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((shortcode.to_string(), name.to_string()));
        let unknown_project = || FilesError::UnknownProject { shortcode: shortcode.to_string() };
        let unknown_file = || FilesError::UnknownFile { shortcode: shortcode.to_string(), name: name.to_string() };
        if !is_shortcode(shortcode) {
            return Err(unknown_project());
        }
        if !is_file_name(name) {
            return Err(unknown_file());
        }
        match self.projects.get(shortcode) {
            None => Err(unknown_project()),
            Some(Project::Unavailable) => Err(FilesError::unavailable(io::Error::other("simulated outage"))),
            Some(Project::Files(files)) => match files.get(name) {
                Some(FileEntry::Bytes(bytes)) => Ok(bytes.clone()),
                Some(FileEntry::Unavailable) => Err(FilesError::unavailable(io::Error::other("simulated outage"))),
                None => Err(unknown_file()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::{ClassIri, DataArk, PropertyIri, Resource, ResourceIri, Value, ValueKind};

    fn text(property: &str, uuid: &str, text: &str) -> Value {
        Value {
            property: PropertyIri(property.to_string()),
            uuid: Some(uuid.to_string()),
            kind: ValueKind::Text { text: text.to_string(), lang: None },
        }
    }

    /// Values neither alphabetical nor by IRI, with three under one property.
    fn book_snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            shortcode: "0803".to_string(),
            resources: vec![Resource {
                iri: ResourceIri("http://rdfh.ch/0803/zz-book".to_string()),
                ark: DataArk("https://ark.dasch.swiss/ark:/72163/1/0803/zz=booko".to_string()),
                class: ClassIri("http://www.knora.org/ontology/0803/incunabula#book".to_string()),
                label: "Zeitglöcklein".to_string(),
                values: vec![
                    text(
                        "http://www.knora.org/ontology/0803/incunabula#title",
                        "rN2fkX0aQ",
                        "Zeitglöcklein",
                    ),
                    text(
                        "http://www.knora.org/ontology/0803/incunabula#publisher",
                        "8bTqWm3Lc",
                        "Kessler",
                    ),
                    text(
                        "http://www.knora.org/ontology/0803/incunabula#title",
                        "Hk5vPz7Ey",
                        "Andachtsbuch",
                    ),
                    text(
                        "http://www.knora.org/ontology/0803/incunabula#title",
                        "0cYw9RjuT",
                        "Mittagsgebet",
                    ),
                ],
                file: None,
                part_of: vec![],
                seqnum: None,
                annotation: None,
            }],
            list_nodes: vec![],
        }
    }

    #[test]
    fn test_snapshot_known_shortcode_returns_registered_snapshot_unchanged() {
        let fake = FakeArchiveProjection::default().with_project(book_snapshot());
        let projection: &dyn ArchiveProjection = &fake;

        let served = projection.snapshot("0803").expect("0803 is registered");

        assert_eq!(served, book_snapshot());
    }

    #[test]
    fn test_snapshot_unknown_shortcode_returns_unknown_project() {
        let fake = FakeArchiveProjection::default().with_project(book_snapshot());
        let projection: &dyn ArchiveProjection = &fake;

        let error = projection.snapshot("0001").unwrap_err();

        assert!(
            matches!(&error, ProjectionError::UnknownProject { shortcode } if shortcode == "0001"),
            "{error:?}"
        );
    }

    #[test]
    fn test_snapshot_unavailable_shortcode_returns_unavailable_with_source() {
        let fake = FakeArchiveProjection::default().with_unavailable("0803");
        let projection: &dyn ArchiveProjection = &fake;

        let error = projection.snapshot("0803").unwrap_err();

        assert!(matches!(error, ProjectionError::Unavailable(_)), "{error:?}");
        assert!(error.source().is_some());
    }

    #[test]
    fn test_snapshot_project_registered_after_unavailable_serves_snapshot() {
        let fake = FakeArchiveProjection::default()
            .with_unavailable("0803")
            .with_project(book_snapshot());
        let projection: &dyn ArchiveProjection = &fake;

        let served = projection.snapshot("0803").expect("the later registration wins");

        assert_eq!(served, book_snapshot());
    }

    #[test]
    fn test_snapshot_unavailable_registered_after_project_returns_unavailable() {
        let fake = FakeArchiveProjection::default()
            .with_project(book_snapshot())
            .with_unavailable("0803");
        let projection: &dyn ArchiveProjection = &fake;

        let error = projection.snapshot("0803").unwrap_err();

        assert!(matches!(error, ProjectionError::Unavailable(_)), "{error:?}");
    }

    /// Bytes that are not UTF-8, so the fake cannot be decoding or re-encoding them.
    const KDL: &[u8] = b"remodel {\n    shortcode \"0803\"\n}\n\xff\n";

    #[test]
    fn test_file_registered_file_served_whole() {
        let fake = FakeDepositedFiles::default().with_file("0803", "config.kdl", KDL);
        let files: &dyn DepositedFiles = &fake;

        let served = files.file("0803", "config.kdl").expect("config.kdl is registered");

        assert_eq!(served, KDL);
    }

    #[test]
    fn test_file_registered_after_unavailable_project_serves_file() {
        let fake = FakeDepositedFiles::default()
            .with_unavailable("0803")
            .with_file("0803", "config.kdl", KDL);

        let served = fake.file("0803", "config.kdl").expect("the later registration wins");

        assert_eq!(served, KDL);
    }

    #[test]
    fn test_file_unavailable_registered_after_file_returns_unavailable() {
        let fake = FakeDepositedFiles::default()
            .with_file("0803", "config.kdl", KDL)
            .with_unavailable("0803");

        let error = fake.file("0803", "config.kdl").unwrap_err();

        assert!(matches!(error, FilesError::Unavailable(_)), "{error:?}");
    }

    #[test]
    fn test_file_unknown_shortcode_returns_unknown_project() {
        let fake = FakeDepositedFiles::default().with_file("0803", "config.kdl", KDL);

        let error = fake.file("0001", "config.kdl").unwrap_err();

        assert!(
            matches!(&error, FilesError::UnknownProject { shortcode } if shortcode == "0001"),
            "{error:?}"
        );
    }

    #[test]
    fn test_file_unknown_name_returns_unknown_file() {
        let fake = FakeDepositedFiles::default().with_file("0803", "config.kdl", KDL);

        let error = fake.file("0803", "curation.csv").unwrap_err();

        assert!(
            matches!(&error, FilesError::UnknownFile { shortcode, name } if shortcode == "0803" && name == "curation.csv"),
            "{error:?}"
        );
    }

    #[test]
    fn test_file_name_failing_is_file_name_returns_unknown_file_even_when_registered() {
        let fake = FakeDepositedFiles::default()
            .with_file("0803", "../0804/config.kdl", KDL)
            .with_file("0803", "narrative/.hidden.md", KDL);

        for name in ["../0804/config.kdl", "narrative/.hidden.md"] {
            let error = fake.file("0803", name).unwrap_err();

            assert!(
                matches!(&error, FilesError::UnknownFile { name: refused, .. } if refused == name),
                "{name}: {error:?}"
            );
        }
    }

    #[test]
    fn test_file_shortcode_failing_is_shortcode_returns_unknown_project_even_when_registered() {
        let fake = FakeDepositedFiles::default().with_file("../x", "config.kdl", KDL);

        for name in ["config.kdl", "../config.kdl"] {
            let error = fake.file("../x", name).unwrap_err();

            assert!(
                matches!(&error, FilesError::UnknownProject { shortcode } if shortcode == "../x"),
                "{name}: {error:?}"
            );
        }
    }

    #[test]
    fn test_file_unavailable_project_answers_unavailable_for_every_file() {
        let fake = FakeDepositedFiles::default().with_unavailable("0803");

        for name in ["config.kdl", "curation.csv", "narrative/home.de.md"] {
            let error = fake.file("0803", name).unwrap_err();

            assert!(matches!(error, FilesError::Unavailable(_)), "{name}: {error:?}");
            assert!(error.source().is_some(), "{name}");
        }
    }

    #[test]
    fn test_file_unavailable_file_answers_unavailable_for_that_file_only() {
        let fake = FakeDepositedFiles::default()
            .with_file("0803", "config.kdl", KDL)
            .with_unavailable_file("0803", "curation.csv");

        let error = fake.file("0803", "curation.csv").unwrap_err();

        assert!(matches!(error, FilesError::Unavailable(_)), "{error:?}");
        assert_eq!(fake.file("0803", "config.kdl").expect("config.kdl stays available"), KDL);
    }

    #[test]
    fn test_requests_lists_every_request_in_order() {
        let fake = FakeDepositedFiles::default().with_file("0803", "config.kdl", KDL);

        let _ = fake.file("0803", "config.kdl");
        let _ = fake.file("0803", "narrative/home.md");
        let _ = fake.file("0001", "config.kdl");
        let _ = fake.file("0803", "../x");
        let _ = fake.file("0803", "config.kdl");

        assert_eq!(
            fake.requests(),
            [
                ("0803", "config.kdl"),
                ("0803", "narrative/home.md"),
                ("0001", "config.kdl"),
                ("0803", "../x"),
                ("0803", "config.kdl"),
            ]
            .map(|(shortcode, name)| (shortcode.to_string(), name.to_string()))
        );
    }
}
