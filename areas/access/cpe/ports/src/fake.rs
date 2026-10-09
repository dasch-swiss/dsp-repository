use std::collections::BTreeMap;
use std::io;

use crate::{ArchiveProjection, ProjectSnapshot, ProjectionError};

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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::{ClassIri, CuratedValue, DataArk, PropertyIri, Resource, ResourceIri, Value, ValueKind};

    fn text(property: &str, uuid: &str, text: &str) -> Value {
        Value {
            property: PropertyIri(property.to_string()),
            uuid: Some(uuid.to_string()),
            kind: ValueKind::Text { text: text.to_string(), lang: None },
        }
    }

    /// Values neither alphabetical nor by IRI, with three under one property; curation listed
    /// against its `Ord`.
    fn book_snapshot() -> ProjectSnapshot {
        let book = || ResourceIri("http://rdfh.ch/0803/zz-book".to_string());
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
            curation: vec![
                CuratedValue {
                    resource: book(),
                    key: "teaser".to_string(),
                    lang: Some("en".to_string()),
                    text: "A book of hours".to_string(),
                },
                CuratedValue {
                    resource: book(),
                    key: "slug".to_string(),
                    lang: None,
                    text: "zeitgloecklein".to_string(),
                },
            ],
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
}
