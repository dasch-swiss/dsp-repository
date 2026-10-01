use std::fs;
use std::path::Path;

use cpe_ports::contract::Violation;
use cpe_ports::{ArchiveProjection, ListNodeIri, ProjectionError, PropertyIri, ResourceIri};
use tempfile::TempDir;

use crate::{InvalidFact, LiveArchiveProjection, SnapshotError, KNOWN};

/// Writes `nquads` as `<dir>/0803.nq`; the one fixture helper of the crate's tests.
pub(crate) fn write_0803(dir: &TempDir, nquads: &str) {
    fs::write(dir.path().join("0803.nq"), nquads).expect("write the fixture");
}

const _: () = {
    const fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<SnapshotError>();
};

// Serving

#[test]
fn test_snapshot_known_shortcode_with_two_resources_serves_both() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.w3.org/2000/01/rdf-schema#label> "Titelblatt" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    assert_eq!(snapshot.shortcode, "0803");
    let mut labels: Vec<(&str, &str)> = snapshot
        .resources
        .iter()
        .map(|resource| (resource.iri.as_str(), resource.label.as_str()))
        .collect();
    labels.sort_unstable();
    assert_eq!(
        labels,
        vec![
            ("http://rdfh.ch/0803/a3Lm", "Titelblatt"),
            ("http://rdfh.ch/0803/zR8c", "Zeitglöcklein")
        ]
    );
}

#[test]
fn test_snapshot_quad_written_twice_serves_one_fact() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a repeated quad is one fact");

    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.resources[0].label, "Zeitglöcklein");
    assert_eq!(snapshot.resources[0].values.len(), 1);
}

#[test]
fn test_snapshot_file_changed_between_calls_serves_changed_content() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    let projection = LiveArchiveProjection::new(dir.path());
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Erste Fassung" <urn:dsp:project:0803> .
        "#,
    );
    let first = projection.snapshot("0803").expect("the first file is served");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zweite Fassung" <urn:dsp:project:0803> .
        "#,
    );

    let second = projection.snapshot("0803").expect("the changed file is served");

    assert_eq!(first.resources[0].label, "Erste Fassung");
    assert_eq!(second.resources[0].label, "Zweite Fassung");
}

#[test]
fn test_snapshot_unknown_shortcode_with_file_present_returns_unknown_project() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    fs::write(
        dir.path().join("0804.nq"),
        r#"
        <http://rdfh.ch/0804/kP2s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0804> .
        <http://rdfh.ch/0804/kP2s> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0804/dokubib#Bild> <urn:dsp:project:0804> .
        <http://rdfh.ch/0804/kP2s> <http://www.w3.org/2000/01/rdf-schema#label> "Bild" <urn:dsp:project:0804> .
        "#,
    )
    .expect("write the fixture");

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0804")
        .expect_err("0804 is not a known project");

    assert!(
        matches!(&error, ProjectionError::UnknownProject { shortcode } if shortcode == "0804"),
        "{error:?}"
    );
}

#[test]
fn test_snapshot_every_known_shortcode_has_committed_file() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");

    for shortcode in KNOWN {
        let path = data.join(format!("{shortcode}.nq"));
        assert!(path.is_file(), "{} is missing", path.display());
    }
}

#[test]
fn test_snapshot_missing_file_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a missing file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(matches!(source, SnapshotError::Read { .. }), "{source:?}");
}

#[test]
fn test_snapshot_directory_in_place_of_file_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    fs::create_dir(dir.path().join("0803.nq")).expect("create the directory");

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a directory is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(matches!(source, SnapshotError::Read { .. }), "{source:?}");
}

#[test]
fn test_snapshot_syntax_error_on_later_line_returns_unavailable_naming_line() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        concat!(
            r#"<http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> ."#,
            "\n",
            r#"<http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> ."#,
            "\n",
            r#"<http://rdfh.ch/0803/zR8c <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> ."#,
            "\n",
            r#"<http://rdfh.ch/0803/a3Lm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> ."#,
            "\n",
        ),
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a syntax error is not served");

    assert!(error.to_string().contains("line 3"), "{error}");
    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(matches!(source, SnapshotError::Syntax { line: 3, .. }), "{source:?}");
}

#[test]
fn test_snapshot_quad_in_second_graph_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0804> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a second graph is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path, subject, reason: InvalidFact::ForeignGraph { graph } }
                if *path == dir.path().join("0803.nq") && subject == "http://rdfh.ch/0803/a3Lm" && graph == "urn:dsp:project:0804"
        ),
        "{source:?}"
    );
    assert!(error.to_string().contains("0803.nq"), "{error}");
}

#[test]
fn test_snapshot_valid_file_without_resources_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a file without resources is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(matches!(source, SnapshotError::Empty { .. }), "{source:?}");
}

// Invalid facts

#[test]
fn test_snapshot_value_node_without_uuid_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::MissingValueUuid } if subject == "urn:dsp:value:mK2x"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_two_labels_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeytglöcklein" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedLabel } if subject == "http://rdfh.ch/0803/zR8c"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_without_class_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::MissingClass } if subject == "http://rdfh.ch/0803/zR8c"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_on_two_resources_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Qx7d> <urn:dsp:project:0803> .
        <urn:dsp:value:Qx7d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Qx7d> <https://ontology.dasch.swiss/dao#valueHasUUID> "Qx7d" <urn:dsp:project:0803> .
        <urn:dsp:value:Qx7d> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:Qx7d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "GW 4168" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.w3.org/2000/01/rdf-schema#label> "Narrenschiff" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/a3Lm> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Qx7d> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::DuplicateValueUuid { uuid } }
                if subject == "http://rdfh.ch/0803/zR8c" && uuid == "Qx7d"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_order_not_an_integer_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasOrder> "1.5"^^<http://www.w3.org/2001/XMLSchema#decimal> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:mK2x" && predicate == "https://ontology.dasch.swiss/dao#valueHasOrder" && lexical == "1.5"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_without_calendar_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteDate { missing } }
                if subject == "urn:dsp:value:EDo-" && missing == "https://ontology.dasch.swiss/dao#dateCalendar"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_value_naming_absent_node_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:value:Lq9t> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#valueHasUUID> "Lq9t" <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceListNode> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownListNode { node } }
                if subject == "urn:dsp:value:Lq9t" && node == "http://rdfh.ch/lists/0803/rot7"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_node_with_absent_parent_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownListParent { parent } }
                if subject == "http://rdfh.ch/lists/0803/rot7" && parent == "http://rdfh.ch/lists/0803/Fk3a"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_blank_node_resource_subject_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        _:q3 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        _:q3 <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        _:q3 <http://www.w3.org/2000/01/rdf-schema#label> "Titelblatt" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { reason: InvalidFact::BlankNodeSubject, .. }),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_without_label_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::MissingLabel } if subject == "http://rdfh.ch/0803/zR8c"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_typed_label_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "7"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "http://rdfh.ch/0803/zR8c" && predicate == "http://www.w3.org/2000/01/rdf-schema#label" && lexical == "7"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_language_tagged_label_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::TaggedLabel } if subject == "http://rdfh.ch/0803/zR8c"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_with_two_values_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(source, SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedValue } if subject == "urn:dsp:value:mK2x"),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_with_two_source_properties_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedSourceProperty } if subject == "urn:dsp:value:mK2x"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_jdn_beyond_i64_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "9223372036854775808"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:EDo-" && predicate == "https://ontology.dasch.swiss/dao#dateStartJDN" && lexical == "9223372036854775808"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_without_end_bound_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteDate { missing } }
                if subject == "urn:dsp:value:EDo-" && missing == "https://ontology.dasch.swiss/dao#dateEndJDN"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_with_unknown_calendar_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "ISLAMIC:897"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "ISLAMIC" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownDateTerm { predicate, lexical } }
                if subject == "urn:dsp:value:EDo-" && predicate == "https://ontology.dasch.swiss/dao#dateCalendar" && lexical == "ISLAMIC"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_with_unknown_precision_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "CENTURY" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownDateTerm { predicate, lexical } }
                if subject == "urn:dsp:value:EDo-" && predicate == "https://ontology.dasch.swiss/dao#dateEndPrecision" && lexical == "CENTURY"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_without_value_or_list_node_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::MissingValueContent } if subject == "urn:dsp:value:mK2x"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_broken_value_node_on_dropped_annotation_serves_snapshot() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/knora-base#Region> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/2000/01/rdf-schema#label> "Randnotiz" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.knora.org/ontology/knora-base#isRegionOf> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/ns/oa#motivatedBy> <http://www.w3.org/ns/oa#commenting> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:value:c0mM> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Holzschnitt" <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Holzschnitt, koloriert" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a broken fact inside an annotation is dropped");

    let iris: Vec<&str> = snapshot.resources.iter().map(|resource| resource.iri.as_str()).collect();
    assert_eq!(iris, vec!["http://rdfh.ch/0803/pG4w"]);
}

#[test]
fn test_snapshot_resource_with_two_classes_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "http://rdfh.ch/0803/zR8c" && predicate == "https://ontology.dasch.swiss/dao#sourceClass"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_with_two_uuids_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "Bw4e" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "urn:dsp:value:mK2x" && predicate == "https://ontology.dasch.swiss/dao#valueHasUUID"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_with_two_orders_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasOrder> "2"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasOrder> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "urn:dsp:value:mK2x" && predicate == "https://ontology.dasch.swiss/dao#valueHasOrder"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_two_seqnums_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#seqnum> "7"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#seqnum> "3"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "http://rdfh.ch/0803/pG4w" && predicate == "https://ontology.dasch.swiss/dao#seqnum"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_with_two_start_jdns_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo3> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo3" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266010"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "urn:dsp:value:EDo3" && predicate == "https://ontology.dasch.swiss/dao#dateStartJDN"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_value_with_two_list_nodes_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:value:Lq9t> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#valueHasUUID> "Lq9t" <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceListNode> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/blau2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/blau2> <http://www.w3.org/2004/02/skos/core#prefLabel> "Blau"@de <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceListNode> <http://rdfh.ch/lists/0803/blau2> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "urn:dsp:value:Lq9t" && predicate == "https://ontology.dasch.swiss/dao#sourceListNode"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_node_with_two_parents_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Aw1c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Aw1c> <http://www.w3.org/2004/02/skos/core#prefLabel> "Signale"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Aw1c> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "http://rdfh.ch/lists/0803/rot7" && predicate == "http://www.w3.org/2004/02/skos/core#broader"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_node_with_two_positions_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "3"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "http://rdfh.ch/lists/0803/rot7" && predicate == "https://ontology.dasch.swiss/dao#listNodePosition"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_without_filename_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteRepresentation { missing } }
                if subject == "urn:dsp:representation:sdSI" && missing == "https://ontology.dasch.swiss/dao#internalFilename"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_still_image_without_width_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#StillImageRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "8F1KTUpKKra-CaFgCYlt8im.jp2" <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#dimY> "5141"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteRepresentation { missing } }
                if subject == "urn:dsp:representation:sdSI" && missing == "https://ontology.dasch.swiss/dao#dimX"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_still_image_without_height_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#dimX> "3297"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#StillImageRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "8F1KTUpKKra-CaFgCYlt8im.jp2" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteRepresentation { missing } }
                if subject == "urn:dsp:representation:sdSI" && missing == "https://ontology.dasch.swiss/dao#dimY"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_without_type_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::IncompleteRepresentation { missing } }
                if subject == "urn:dsp:representation:sdSI" && missing == "https://ontology.dasch.swiss/dao#representationType"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_not_typed_representation_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownRepresentation { node } }
                if subject == "http://rdfh.ch/0803/pG4w" && node == "urn:dsp:representation:sdSI"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_as_literal_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownRepresentation { node } }
                if subject == "http://rdfh.ch/0803/pG4w" && node == "Kd2bWq9-dok.pdf"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_two_representations_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:Ab7c> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ab7c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ab7c> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ab7c> <https://ontology.dasch.swiss/dao#internalFilename> "Zt5mPq2-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "http://rdfh.ch/0803/pG4w" && predicate == "https://ontology.dasch.swiss/dao#hasRepresentation"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_with_two_filenames_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Zt5mPq2-dok.pdf" <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::RepeatedPredicate { predicate } }
                if subject == "urn:dsp:representation:sdSI" && predicate == "https://ontology.dasch.swiss/dao#internalFilename"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_filename_not_plain_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:representation:sdSI" && predicate == "https://ontology.dasch.swiss/dao#internalFilename" && lexical == "Kd2bWq9-dok.pdf"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_type_as_literal_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> "DocumentRepresentation" <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:representation:sdSI" && predicate == "https://ontology.dasch.swiss/dao#representationType" && lexical == "DocumentRepresentation"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_without_source_property_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::MissingSourceProperty }
                if subject == "urn:dsp:value:mK2x"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_source_property_differing_from_edge_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitSourceProperty { edge } }
                if subject == "urn:dsp:value:mK2x" && edge == "http://www.knora.org/ontology/0803/incunabula#title"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_node_under_two_edges_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitSourceProperty { edge } }
                if subject == "urn:dsp:value:mK2x" && edge == "http://www.knora.org/ontology/0803/incunabula#hasCitation"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_uuid_differing_from_node_iri_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "Qx7d" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitValueUuid { uuid } }
                if subject == "urn:dsp:value:mK2x" && uuid == "Qx7d"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_uuid_not_base64url_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x.1> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x.1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x.1> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x.1" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x.1> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x.1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitValueUuid { uuid } }
                if subject == "urn:dsp:value:mK2x.1" && uuid == "mK2x.1"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_uuid_not_plain_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x"@de <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:mK2x" && predicate == "https://ontology.dasch.swiss/dao#valueHasUUID" && lexical == "mK2x"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_value_with_rdf_value_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:value:Lq9t> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#valueHasUUID> "Lq9t" <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Rot" <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceListNode> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::ListValueWithContent }
                if subject == "urn:dsp:value:Lq9t"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_value_as_iri_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Bw4e> <urn:dsp:project:0803> .
        <urn:dsp:value:Bw4e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Bw4e> <https://ontology.dasch.swiss/dao#valueHasUUID> "Bw4e" <urn:dsp:project:0803> .
        <urn:dsp:value:Bw4e> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:Bw4e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> <http://rdfh.ch/0803/a3Lm> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:Bw4e" && predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" && lexical == "http://rdfh.ch/0803/a3Lm"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_boolean_not_canonical_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#isColoured> <urn:dsp:value:Hc5r> <urn:dsp:project:0803> .
        <urn:dsp:value:Hc5r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Hc5r> <https://ontology.dasch.swiss/dao#valueHasUUID> "Hc5r" <urn:dsp:project:0803> .
        <urn:dsp:value:Hc5r> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#isColoured> <urn:dsp:project:0803> .
        <urn:dsp:value:Hc5r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "1"^^<http://www.w3.org/2001/XMLSchema#boolean> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:Hc5r" && predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" && lexical == "1"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_integer_not_canonical_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasLeaves> <urn:dsp:value:Np8s> <urn:dsp:project:0803> .
        <urn:dsp:value:Np8s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Np8s> <https://ontology.dasch.swiss/dao#valueHasUUID> "Np8s" <urn:dsp:project:0803> .
        <urn:dsp:value:Np8s> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasLeaves> <urn:dsp:project:0803> .
        <urn:dsp:value:Np8s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "+12"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:Np8s" && predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" && lexical == "+12"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_decimal_not_canonical_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPrice> <urn:dsp:value:Wd3k> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <https://ontology.dasch.swiss/dao#valueHasUUID> "Wd3k" <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPrice> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "1.50"^^<http://www.w3.org/2001/XMLSchema#decimal> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:Wd3k" && predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" && lexical == "1.50"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_decimal_not_a_number_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPrice> <urn:dsp:value:Wd3k> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <https://ontology.dasch.swiss/dao#valueHasUUID> "Wd3k" <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPrice> <urn:dsp:project:0803> .
        <urn:dsp:value:Wd3k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "zwölf"^^<http://www.w3.org/2001/XMLSchema#decimal> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "urn:dsp:value:Wd3k" && predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" && lexical == "zwölf"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_label_typed_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "7"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "http://rdfh.ch/lists/0803/Fk3a" && predicate == "http://www.w3.org/2004/02/skos/core#prefLabel" && lexical == "7"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_label_as_iri_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "http://rdfh.ch/lists/0803/Fk3a" && predicate == "http://www.w3.org/2004/02/skos/core#prefLabel" && lexical == "http://rdfh.ch/lists/0803/rot7"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_node_with_two_labels_in_one_language_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Colours"@en <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farbtöne"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::DuplicateListLabel }
                if subject == "http://rdfh.ch/lists/0803/Fk3a"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_list_node_with_two_untagged_labels_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farbtöne" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::DuplicateListLabel }
                if subject == "http://rdfh.ch/lists/0803/Fk3a"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_date_calendar_not_plain_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo3> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo3" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN"@en <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo3> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownDateTerm { predicate, lexical } }
                if subject == "urn:dsp:value:EDo3" && predicate == "https://ontology.dasch.swiss/dao#dateCalendar" && lexical == "JULIAN"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_blank_node_object_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasAuthor> _:b0 <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::BlankNodeObject }
                if subject == "http://rdfh.ch/0803/zR8c"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_resource_with_second_type_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::ExtraResourceType { class } }
                if subject == "http://rdfh.ch/0803/zR8c" && class == "http://www.w3.org/2004/02/skos/core#Concept"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_literal_under_link_predicate_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnfitLiteral { predicate, lexical } }
                if subject == "http://rdfh.ch/0803/zR8c" && predicate == "http://www.knora.org/ontology/0803/incunabula#title" && lexical == "Zeitglöcklein des Lebens"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_part_of_absent_resource_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/gOnE> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#seqnum> "12"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownParent { parent } }
                if subject == "http://rdfh.ch/0803/pG4w" && parent == "http://rdfh.ch/0803/gOnE"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_part_of_literal_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> "Zeitglöcklein" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::UnknownParent { parent } }
                if subject == "http://rdfh.ch/0803/pG4w" && parent == "Zeitglöcklein"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_root_list_node_with_position_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <https://ontology.dasch.swiss/dao#listNodePosition> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::PositionedListRoot }
                if subject == "http://rdfh.ch/lists/0803/Fk3a"
        ),
        "{source:?}"
    );
}

#[test]
fn test_snapshot_representation_on_two_resources_returns_unavailable() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "Kd2bWq9-dok.pdf" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba7x> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba7x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba7x> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba7x> <http://www.w3.org/2000/01/rdf-schema#label> "a1v" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("an invalid file is not served");

    let ProjectionError::Unavailable(source) = &error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    assert!(
        matches!(
            source,
            SnapshotError::Invalid { path: _, subject, reason: InvalidFact::DuplicateRepresentation { node } }
                if subject == "http://rdfh.ch/0803/pG4w" && node == "urn:dsp:representation:sdSI"
        ),
        "{source:?}"
    );
}

// The port's contract, run on every snapshot before it is served

/// The violations of a snapshot refused as `Contract` from `<dir>/0803.nq`; panics on any other
/// result.
fn expect_contract_refusal(error: &ProjectionError) -> &[Violation] {
    let ProjectionError::Unavailable(source) = error else {
        panic!("expected Unavailable, got {error:?}")
    };
    let source = source.downcast_ref::<SnapshotError>().expect("the source is a SnapshotError");
    let SnapshotError::Contract { path, violations } = source else {
        panic!("expected Contract, got {source:?}")
    };
    assert!(path.ends_with("0803.nq"), "{path:?}");
    violations
}

#[test]
fn test_snapshot_inverted_date_returns_unavailable_naming_violation() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a snapshot the contract rejects is not served");

    assert_eq!(
        expect_contract_refusal(&error),
        [Violation::InvertedDate {
            resource: ResourceIri("http://rdfh.ch/0803/zR8c".to_string()),
            property: PropertyIri("http://www.knora.org/ontology/0803/incunabula#hasPubdate".to_string()),
        }]
    );
}

#[test]
fn test_snapshot_part_of_cycle_returns_unavailable_naming_violation() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/zR8c> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a snapshot the contract rejects is not served");

    assert_eq!(
        expect_contract_refusal(&error),
        [Violation::MembershipCycle {
            resource: ResourceIri("http://rdfh.ch/0803/pG4w".to_string())
        }]
    );
}

#[test]
fn test_snapshot_list_node_cycle_returns_unavailable_naming_violation() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <https://ontology.dasch.swiss/dao#listNodePosition> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a snapshot the contract rejects is not served");

    assert_eq!(
        expect_contract_refusal(&error),
        [Violation::ListNodeCycle {
            node: ListNodeIri("http://rdfh.ch/lists/0803/Fk3a".to_string())
        }]
    );
}

#[test]
fn test_snapshot_siblings_sharing_position_returns_unavailable_naming_violation() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/bl4u> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/bl4u> <http://www.w3.org/2004/02/skos/core#prefLabel> "Blau"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/bl4u> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/bl4u> <https://ontology.dasch.swiss/dao#listNodePosition> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let error = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect_err("a snapshot the contract rejects is not served");

    assert_eq!(
        expect_contract_refusal(&error),
        [Violation::DuplicateSiblingPosition {
            parent: ListNodeIri("http://rdfh.ch/lists/0803/Fk3a".to_string()),
            position: 1,
            nodes: vec![
                ListNodeIri("http://rdfh.ch/lists/0803/bl4u".to_string()),
                ListNodeIri("http://rdfh.ch/lists/0803/rot7".to_string()),
            ],
        }]
    );
}
