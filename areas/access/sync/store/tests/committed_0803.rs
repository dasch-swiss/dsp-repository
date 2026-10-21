//! The committed 0803 snapshot, read through the adapter. The expected values are facts of the
//! 2026-07-19 stage dump that `PROVENANCE` names, so a failure here is a `dao-lift` or `sync-store`
//! bug: never change an expectation to fit the file.

use std::collections::BTreeMap;
use std::fs;
use std::sync::OnceLock;

use cpe_ports::contract::violations;
use cpe_ports::{
    ArchiveProjection, Calendar, DateBound, DatePrecision, DateValue, File, Motivation, ProjectSnapshot, Resource,
    ResourceIri, ValueKind,
};
use sync_store::LiveArchiveProjection;

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");

const BOOK: &str = "http://www.knora.org/ontology/0803/incunabula#Book";
const PAGE: &str = "http://www.knora.org/ontology/0803/incunabula#Page";
const BAND: &str = "http://www.knora.org/ontology/0803/incunabula#Band";
const KB: &str = "http://www.knora.org/ontology/knora-base#";
const REGION: &str = "http://www.knora.org/ontology/knora-base#Region";
const LINK_OBJ: &str = "http://www.knora.org/ontology/knora-base#LinkObj";

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

fn of_class(class: &str) -> Vec<&'static Resource> {
    snapshot()
        .resources
        .iter()
        .filter(|resource| resource.class.as_str() == class)
        .collect()
}

fn kinds_under<'r>(resource: &'r Resource, property: &str) -> Vec<&'r ValueKind> {
    resource
        .values
        .iter()
        .filter(|value| value.property.as_str() == property)
        .map(|value| &value.kind)
        .collect()
}

fn link_targets<'r>(resource: &'r Resource, property: &str) -> Vec<&'r ResourceIri> {
    kinds_under(resource, property)
        .into_iter()
        .map(|kind| match kind {
            ValueKind::Link(target) => target,
            other => panic!("{}: {property} holds {other:?}", resource.iri.as_str()),
        })
        .collect()
}

fn targets(resource: &Resource) -> Vec<&ResourceIri> {
    let annotation = resource
        .annotation
        .as_ref()
        .unwrap_or_else(|| panic!("{} is an annotation", resource.iri.as_str()));
    annotation.targets.iter().collect()
}

#[test]
fn test_committed_0803_snapshot_passes_contract() {
    let found = violations("0803", snapshot());

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn test_committed_0803_resources_by_class_serves_books_pages_bands_regions_and_link_objs() {
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    for resource in &snapshot().resources {
        *by_class.entry(resource.class.as_str()).or_default() += 1;
    }

    assert_eq!(
        by_class,
        BTreeMap::from([(PAGE, 4_024), (BOOK, 19), (BAND, 38), (REGION, 77), (LINK_OBJ, 40)])
    );
}

#[test]
fn test_committed_0803_annotations_serves_regions_and_link_objs() {
    let snapshot = snapshot();

    assert_eq!(snapshot.resources.len(), 4_198);
    assert_eq!(resource("http://rdfh.ch/0803/GOkuI_IxVuSKMRZmCypz7Q").class.as_str(), REGION);
    assert_eq!(resource("http://rdfh.ch/0803/00bnHlmDVIq_Blb4DvKGiQ").class.as_str(), LINK_OBJ);
}

#[test]
fn test_committed_0803_annotations_serves_117_with_their_motivations() {
    let motivations: Vec<Motivation> = snapshot()
        .resources
        .iter()
        .filter_map(|resource| resource.annotation.as_ref())
        .map(|annotation| annotation.motivation)
        .collect();

    let count = |motivation: Motivation| motivations.iter().filter(|found| **found == motivation).count();
    assert_eq!(motivations.len(), 117);
    assert_eq!(count(Motivation::Commenting), 77);
    assert_eq!(count(Motivation::Linking), 40);
}

#[test]
fn test_committed_0803_regions_serves_each_one_geometry() {
    let regions = of_class(REGION);

    assert_eq!(regions.len(), 77);
    for region in regions {
        let kinds = kinds_under(region, &format!("{KB}hasGeometry"));
        assert!(
            matches!(kinds.as_slice(), [ValueKind::Geometry(_)]),
            "{}: {kinds:?}",
            region.iri.as_str()
        );
    }
}

#[test]
fn test_committed_0803_regions_serves_each_one_color() {
    let regions = of_class(REGION);

    assert_eq!(regions.len(), 77);
    for region in regions {
        let kinds = kinds_under(region, &format!("{KB}hasColor"));
        assert!(
            matches!(kinds.as_slice(), [ValueKind::Color(_)]),
            "{}: {kinds:?}",
            region.iri.as_str()
        );
    }
}

#[test]
fn test_committed_0803_regions_serves_each_one_comment() {
    let regions = of_class(REGION);

    assert_eq!(regions.len(), 77);
    for region in regions {
        let kinds = kinds_under(region, &format!("{KB}hasComment"));
        assert!(
            matches!(kinds.as_slice(), [ValueKind::Text { .. }]),
            "{}: {kinds:?}",
            region.iri.as_str()
        );
    }
}

#[test]
fn test_committed_0803_regions_serves_targets_equal_to_is_region_of_link() {
    let regions = of_class(REGION);

    assert_eq!(regions.len(), 77);
    for region in regions {
        let links = link_targets(region, &format!("{KB}isRegionOf"));
        assert_eq!(links.len(), 1, "{}", region.iri.as_str());
        assert_eq!(targets(region), links, "{}", region.iri.as_str());
    }
}

#[test]
fn test_committed_0803_link_objs_serves_79_has_link_to_links() {
    let link_objs = of_class(LINK_OBJ);

    let links: usize = link_objs
        .iter()
        .map(|link_obj| link_targets(link_obj, &format!("{KB}hasLinkTo")).len())
        .sum();
    assert_eq!(link_objs.len(), 40);
    assert_eq!(links, 79);
}

#[test]
fn test_committed_0803_link_objs_serves_targets_equal_to_has_link_to_links() {
    let link_objs = of_class(LINK_OBJ);

    assert_eq!(link_objs.len(), 40);
    for link_obj in link_objs {
        let links = link_targets(link_obj, &format!("{KB}hasLinkTo"));
        assert_eq!(targets(link_obj), links, "{}", link_obj.iri.as_str());
    }
}

#[test]
fn test_committed_0803_region_color_serves_lexical() {
    let region = resource("http://rdfh.ch/0803/089fJhP1WuylV1wftl5Y_Q");

    assert_eq!(
        kinds_under(region, &format!("{KB}hasColor")),
        vec![&ValueKind::Color("#ff3333".to_string())]
    );
}

#[test]
fn test_committed_0803_region_geometry_serves_json_verbatim() {
    let region = resource("http://rdfh.ch/0803/089fJhP1WuylV1wftl5Y_Q");

    let kinds = kinds_under(region, &format!("{KB}hasGeometry"));
    let [ValueKind::Geometry(geometry)] = kinds.as_slice() else {
        panic!("expected one geometry, got {kinds:?}")
    };
    assert!(
        geometry.starts_with(r##"{"status":"active","lineColor":"#ff3333""##),
        "{geometry}"
    );
    assert!(geometry.contains(r#""type":"rectangle""#), "{geometry}");
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
}

#[test]
fn test_committed_0803_region_comment_with_superseded_versions_serves_current_value() {
    let region = resource("http://rdfh.ch/0803/GOkuI_IxVuSKMRZmCypz7Q");

    let comments: Vec<(Option<&str>, &ValueKind)> = region
        .values
        .iter()
        .filter(|value| value.property.as_str() == format!("{KB}hasComment"))
        .map(|value| (value.uuid.as_deref(), &value.kind))
        .collect();
    assert_eq!(
        comments,
        vec![(
            Some("xdvQATb3TOeS875AuWi6Rw"),
            &ValueKind::Text {
                text: "Derselbe Holzschnitt wird auf Seite c7r der lateinischen Ausgabe des Narrenschiffs verwendet.\n        "
                    .to_string(),
                lang: None
            }
        )]
    );
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

/// `0803-arks.txt` is the incubator's copy of `dao-lift`'s ARKs (`PROVENANCE`): never regenerate it
/// from `sync-store`, or this test checks the derivation against itself.
#[test]
fn test_committed_0803_arks_serves_every_dao_lift_ark() {
    let arks = fs::read_to_string(format!("{DATA_DIR}/0803-arks.txt")).expect("read 0803-arks.txt");
    let lines: Vec<&str> = arks.lines().collect();
    assert_eq!(lines.len(), 4_143);
    assert!(lines.windows(2).all(|pair| pair[0] < pair[1]), "sorted, without repeats");

    let served: BTreeMap<&str, &str> = snapshot()
        .resources
        .iter()
        .map(|resource| (resource.iri.as_str(), resource.ark.as_str()))
        .collect();
    let mut mismatches = Vec::new();
    for line in &lines {
        // The ARK's last segment is the resource id, `-` written as `=`, plus one check digit.
        let id = line
            .strip_prefix("https://ark.dasch.swiss/ark:/72163/1/0803/")
            .and_then(|segment| segment.get(..segment.len().checked_sub(1)?))
            .unwrap_or_else(|| panic!("{line} is a 0803 data ARK"))
            .replace('=', "-");
        match served.get(format!("http://rdfh.ch/0803/{id}").as_str()) {
            Some(ark) if ark == line => {}
            Some(ark) => mismatches.push(format!("{line} expected, served {ark}")),
            None => mismatches.push(format!("{line} expected, its resource is not served")),
        }
    }

    assert!(mismatches.is_empty(), "{} mismatches: {mismatches:#?}", mismatches.len());
}

#[test]
fn test_committed_0803_book_serves_its_data_ark() {
    assert_eq!(
        resource("http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ").ark.as_str(),
        "https://ark.dasch.swiss/ark:/72163/1/0803/CDYZPN5zVVKbIcjA1DZxKQO"
    );
}

#[test]
fn test_committed_0803_region_serves_its_data_ark() {
    assert_eq!(
        resource("http://rdfh.ch/0803/089fJhP1WuylV1wftl5Y_Q").ark.as_str(),
        "https://ark.dasch.swiss/ark:/72163/1/0803/089fJhP1WuylV1wftl5Y_QL"
    );
}
