//! The committed 0803 snapshot, read through the adapter. The expected values are facts of the
//! 2026-07-19 stage dump that `PROVENANCE` names, so a failure here is a `dao-lift` or `sync-store`
//! bug: never change an expectation to fit the file.

use std::collections::BTreeMap;
use std::fs;
use std::sync::OnceLock;

use cpe_ports::contract::violations;
use cpe_ports::{
    ArchiveProjection, Calendar, DateBound, DatePrecision, DateValue, File, ProjectSnapshot, Resource, ValueKind,
};
use sync_store::LiveArchiveProjection;

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");

const BOOK: &str = "http://www.knora.org/ontology/0803/incunabula#Book";
const PAGE: &str = "http://www.knora.org/ontology/0803/incunabula#Page";
const BAND: &str = "http://www.knora.org/ontology/0803/incunabula#Band";

/// The served snapshot, parsed once per test binary. It never checks that the file exists first,
/// so a missing file fails every test that reads it.
fn snapshot() -> &'static ProjectSnapshot {
    static SNAPSHOT: OnceLock<ProjectSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        LiveArchiveProjection::new(DATA_DIR)
            .snapshot("0803")
            .expect("the committed 0803 file is served")
    })
}

fn resource(iri: &str) -> &'static Resource {
    snapshot()
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == iri)
        .unwrap_or_else(|| panic!("{iri} is served"))
}

#[test]
fn test_committed_0803_snapshot_passes_contract() {
    let found = violations("0803", snapshot());

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn test_committed_0803_resources_by_class_serves_books_pages_and_bands() {
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    for resource in &snapshot().resources {
        *by_class.entry(resource.class.as_str()).or_default() += 1;
    }

    assert_eq!(by_class, BTreeMap::from([(PAGE, 4_024), (BOOK, 19), (BAND, 38)]));
}

#[test]
fn test_committed_0803_annotations_serves_none_of_them() {
    let snapshot = snapshot();

    assert_eq!(snapshot.resources.len(), 4_081);
    let region = "http://rdfh.ch/0803/GOkuI_IxVuSKMRZmCypz7Q";
    let link_obj = "http://rdfh.ch/0803/00bnHlmDVIq_Blb4DvKGiQ";
    assert!(snapshot
        .resources
        .iter()
        .all(|resource| ![region, link_obj].contains(&resource.iri.as_str())));
}

#[test]
fn test_committed_0803_value_with_superseded_versions_serves_one_current_value() {
    let page = resource("http://rdfh.ch/0803/-34FYx0jVMGTEi8aItQFxQ");

    let kinds: Vec<&ValueKind> = page
        .values
        .iter()
        .filter(|value| value.uuid.as_deref() == Some("U_J3GLDmTJuj24vddKrJMA"))
        .map(|value| &value.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![&ValueKind::Text { text: "a1r; Titelblatt".to_string(), lang: None }]
    );
}

#[test]
fn test_committed_0803_book_title_with_superseded_versions_serves_current_under_source_property() {
    let book = resource("http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ");

    let titles: Vec<_> = book
        .values
        .iter()
        .filter(|value| value.property.as_str() == "http://www.knora.org/ontology/0803/incunabula#hasTitle")
        .collect();
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0].uuid.as_deref(), Some("FYQyJ2K3RT6yyqxuAE6EIg"));
    assert_eq!(
        titles[0].kind,
        ValueKind::Text {
            text: "Bereitung zu dem Heiligen Sakrament".to_string(),
            lang: None
        }
    );
    let on_region = "xdvQATb3TOeS875AuWi6Rw";
    assert!(snapshot()
        .resources
        .iter()
        .flat_map(|resource| &resource.values)
        .all(|value| value.uuid.as_deref() != Some(on_region)));
}

#[test]
fn test_committed_0803_book_citations_serves_by_order_not_uuid() {
    let book = resource("http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ");

    let citations: Vec<&ValueKind> = book
        .values
        .iter()
        .filter(|value| value.property.as_str() == "http://www.knora.org/ontology/0803/incunabula#hasCitation")
        .map(|value| &value.kind)
        .collect();
    let text = |text: &str| ValueKind::Text { text: text.to_string(), lang: None };
    assert_eq!(
        citations,
        vec![
            &text("Schramm Bd. XXI, S. 27"),
            &text("GW 4168"),
            &text("ISTC ib00512000")
        ]
    );
}

#[test]
fn test_committed_0803_book_pubdate_serves_julian_year_bounds() {
    let book = resource("http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ");

    let pubdates: Vec<&ValueKind> = book
        .values
        .iter()
        .filter(|value| value.property.as_str() == "http://www.knora.org/ontology/0803/incunabula#hasPubdate")
        .map(|value| &value.kind)
        .collect();
    assert_eq!(
        pubdates,
        vec![&ValueKind::Date(DateValue {
            calendar: Calendar::Julian,
            start: DateBound { jdn: 2_266_011, precision: DatePrecision::Year },
            end: DateBound { jdn: 2_266_376, precision: DatePrecision::Year },
        })]
    );
}

#[test]
fn test_committed_0803_text_with_standoff_serves_plain_text_verbatim() {
    let kinds: Vec<&ValueKind> = snapshot()
        .resources
        .iter()
        .flat_map(|resource| &resource.values)
        .filter(|value| value.uuid.as_deref() == Some("-UFjZccyRaWal2r0EhOACw"))
        .map(|value| &value.kind)
        .collect();

    assert_eq!(
        kinds,
        vec![&ValueKind::Text { text: "[missing]\n        ".to_string(), lang: None }]
    );
}

#[test]
fn test_committed_0803_list_nodes_serves_23_with_4_roots() {
    let list_nodes = &snapshot().list_nodes;

    assert_eq!(list_nodes.len(), 23);
    assert_eq!(list_nodes.iter().filter(|node| node.parent.is_none()).count(), 4);
}

#[test]
fn test_committed_0803_pages_and_bands_serves_each_a_still_image() {
    let scanned = snapshot()
        .resources
        .iter()
        .filter(|resource| matches!(resource.class.as_str(), PAGE | BAND));

    let mut count = 0;
    for resource in scanned {
        count += 1;
        match &resource.file {
            Some(File::StillImage { asset, width, height }) => {
                assert!(!asset.is_empty(), "{}: empty asset", resource.iri.as_str());
                assert!(*width > 0 && *height > 0, "{}: zero dimension", resource.iri.as_str());
            }
            other => panic!("{}: expected a still image, got {other:?}", resource.iri.as_str()),
        }
    }
    assert_eq!(count, 4_062);
}

#[test]
fn test_committed_0803_pages_serves_each_one_book_parent_and_a_seqnum() {
    let snapshot = snapshot();
    let books: Vec<&str> = snapshot
        .resources
        .iter()
        .filter(|resource| resource.class.as_str() == BOOK)
        .map(|resource| resource.iri.as_str())
        .collect();

    for page in snapshot.resources.iter().filter(|resource| resource.class.as_str() == PAGE) {
        let parents: Vec<&str> = page.part_of.iter().map(|parent| parent.as_str()).collect();
        assert!(
            matches!(parents.as_slice(), [parent] if books.contains(parent)),
            "{}: parents {parents:?}",
            page.iri.as_str()
        );
        assert!(page.seqnum.is_some(), "{}: no seqnum", page.iri.as_str());
    }
}

#[test]
fn test_committed_0803_provenance_pin_matches_vocab_pin() {
    let pin = |text: &str| {
        let start = text.find("dsp-incubator@").expect("a dsp-incubator@<commit> pin") + "dsp-incubator@".len();
        text[start..].chars().take_while(char::is_ascii_hexdigit).collect::<String>()
    };
    let provenance = fs::read_to_string(format!("{DATA_DIR}/PROVENANCE")).expect("read PROVENANCE");
    let vocab = include_str!("../src/vocab.rs");

    assert_eq!(pin(&provenance).len(), 40);
    assert_eq!(pin(vocab).len(), 40);
    assert_eq!(pin(&provenance), pin(vocab));
}
