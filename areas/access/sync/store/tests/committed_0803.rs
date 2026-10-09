//! The committed 0803 snapshot, read through the adapter. The expected values are facts of the
//! 2026-07-19 stage dump that `PROVENANCE` names, so a failure here is a `dao-lift` or `sync-store`
//! bug: never change an expectation to fit the file.
//!
//! The curation expectations are facts of the incubator's CSV files at the commit `PROVENANCE`
//! names for `0803-curation.csv`. That file is edited by hand, so such an expectation changes only
//! together with it. These tests know Incunabula's keys; `sync-store` itself names none (ADR-0010).

use std::collections::{BTreeMap, BTreeSet};
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

/// The one Band whose label is `[missing]`; its id and name are curated.
const UNSIGNED_BAND: &str = "http://rdfh.ch/0803/EJZQcYisXECHxchG_KQ27w";

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

/// The served curation under `key` in `lang`, as text by resource IRI.
fn curated(key: &str, lang: Option<&str>) -> BTreeMap<&'static str, &'static str> {
    let mut by_resource = BTreeMap::new();
    for value in &snapshot().curation {
        if value.key == key && value.lang.as_deref() == lang {
            let earlier = by_resource.insert(value.resource.as_str(), value.text.as_str());
            assert!(earlier.is_none(), "{}: two values under {key}", value.resource.as_str());
        }
    }
    by_resource
}

fn iris_of_class(class: &str) -> BTreeSet<&'static str> {
    of_class(class).into_iter().map(|resource| resource.iri.as_str()).collect()
}

fn resources_of(values: &BTreeMap<&'static str, &'static str>) -> BTreeSet<&'static str> {
    values.keys().copied().collect()
}

fn count_by_text(values: &BTreeMap<&'static str, &'static str>) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for text in values.values() {
        *counts.entry(*text).or_default() += 1;
    }
    counts
}

#[test]
fn test_committed_0803_curation_serves_341_values_on_137_resources() {
    let curation = &snapshot().curation;

    let resources: BTreeSet<&str> = curation.iter().map(|value| value.resource.as_str()).collect();
    assert_eq!(curation.len(), 341);
    assert_eq!(resources.len(), 137);
}

#[test]
fn test_committed_0803_curation_serves_eleven_keys_and_languages_with_their_counts() {
    let mut counts: BTreeMap<(&str, Option<&str>), usize> = BTreeMap::new();
    for value in &snapshot().curation {
        *counts.entry((value.key.as_str(), value.lang.as_deref())).or_default() += 1;
    }

    assert_eq!(
        counts,
        BTreeMap::from([
            (("cover_page", None), 19),
            (("date_display", Some("de")), 19),
            (("keep", None), 117),
            (("lang", None), 97),
            (("name", None), 35),
            (("office", None), 19),
            (("slug", None), 20),
            (("sort_key", None), 3),
            (("teaser", Some("de")), 4),
            (("teaser", Some("en")), 4),
            (("title_override", None), 4),
        ])
    );
}

#[test]
fn test_committed_0803_curation_slug_serves_one_per_book_and_the_unsigned_band() {
    let slugs = curated("slug", None);

    let mut expected = iris_of_class(BOOK);
    assert_eq!(expected.len(), 19);
    expected.insert(UNSIGNED_BAND);
    assert_eq!(resources_of(&slugs), expected);
    assert_eq!(slugs[UNSIGNED_BAND], "strip-38");
    assert_eq!(slugs.values().collect::<BTreeSet<_>>().len(), 20);
}

#[test]
fn test_committed_0803_curation_office_serves_the_five_lanes_on_books_only() {
    let offices = curated("office", None);

    assert_eq!(resources_of(&offices), iris_of_class(BOOK));
    assert_eq!(
        count_by_text(&offices),
        BTreeMap::from([
            ("amerbach", 5),
            ("bergmann", 3),
            ("furter", 4),
            ("other", 4),
            ("ysenhut", 3)
        ])
    );
}

#[test]
fn test_committed_0803_curation_date_display_serves_one_german_value_per_book() {
    let dates = curated("date_display", Some("de"));

    assert_eq!(resources_of(&dates), iris_of_class(BOOK));
    assert_eq!(dates["http://rdfh.ch/0803/PZcNpxILXBuKqGfglAZ4Vg"], "19. Februar 1491");
}

#[test]
fn test_committed_0803_curation_cover_page_serves_a_page_of_each_book_alone() {
    let covers = curated("cover_page", None);

    assert_eq!(resources_of(&covers), iris_of_class(BOOK));
    for (book, cover) in &covers {
        let page = resource(cover);
        let parents: Vec<&str> = page.part_of.iter().map(|parent| parent.as_str()).collect();
        assert_eq!(page.class.as_str(), PAGE, "{book}: cover {cover}");
        assert_eq!(parents, vec![*book], "{book}: cover {cover}");
    }
    assert_eq!(covers.values().collect::<BTreeSet<_>>().len(), 19);
}

#[test]
fn test_committed_0803_curation_cover_page_serves_the_page_with_the_picked_label() {
    let slugs = curated("slug", None);
    let covers = curated("cover_page", None);

    let labels: BTreeMap<&str, &str> = covers
        .iter()
        .map(|(book, cover)| (slugs[book], resource(cover).label.as_str()))
        .collect();
    assert_eq!(
        labels,
        BTreeMap::from([
            ("bereitung", "a1r, Titelblatt, recto"),
            ("brandan", "a1r"),
            ("de-generatione", "a2v"),
            ("itinerarius-peregrinarius", "a1r, Titelblatt"),
            ("itinerarius-peregrinatio", "1 recto"),
            ("lob-der-glieder", "a1r, Titelblatt"),
            ("melusine", "5"),
            ("methodij", "A1r; Titelblatt, recto"),
            ("narrenschiff-dt", "b1v"),
            ("narrenschiff-lat-august", "a1r; Titelblatt, recto"),
            ("narrenschiff-lat-maerz", "a1r; Titelblatt, recto"),
            ("orationes", "a1r, Titelblatt, recto"),
            ("passio-meynrhadi", "a1r, Titelblatt"),
            ("postilla", "a1r; Titelblatt"),
            ("quadragesimale", "a1r; Titelblatt recto"),
            ("reise", "6r"),
            ("walfart", "a1r, Titelblatt"),
            ("zeitgloecklein-1490", "a1r"),
            ("zeitgloecklein-1492", "a1r, Titelblatt"),
        ])
    );
}

#[test]
fn test_committed_0803_curation_keep_serves_99_yes_and_18_no_on_every_region_and_link_obj() {
    let keep = curated("keep", None);

    let regions = iris_of_class(REGION);
    let link_objs = iris_of_class(LINK_OBJ);
    let annotations: BTreeSet<&str> = regions.union(&link_objs).copied().collect();
    let dropped: BTreeSet<&str> = keep.iter().filter(|(_, text)| **text == "no").map(|(iri, _)| *iri).collect();
    assert_eq!(annotations.len(), 117);
    assert_eq!(resources_of(&keep), annotations);
    assert_eq!(count_by_text(&keep), BTreeMap::from([("no", 18), ("yes", 99)]));
    assert_eq!(dropped.intersection(&regions).count(), 13);
    assert_eq!(dropped.intersection(&link_objs).count(), 5);
}

#[test]
fn test_committed_0803_curation_lang_serves_93_de_and_4_en_on_kept_annotations_only() {
    let langs = curated("lang", None);
    let keep = curated("keep", None);

    let english: BTreeSet<&str> = langs.iter().filter(|(_, text)| **text == "en").map(|(iri, _)| *iri).collect();
    assert_eq!(count_by_text(&langs), BTreeMap::from([("de", 93), ("en", 4)]));
    assert_eq!(
        english,
        BTreeSet::from([
            "http://rdfh.ch/0803/GgE0hrMoUpWc4VAW299D6Q",
            "http://rdfh.ch/0803/O-R69zOkWRyCUXE7d5k-PA",
            "http://rdfh.ch/0803/YFEZah_aUQq6IilZwtRGIQ",
            "http://rdfh.ch/0803/zFblYIEZXqauJirfO-7D8A",
        ])
    );
    for iri in english {
        assert_eq!(resource(iri).class.as_str(), REGION, "{iri}");
    }
    for iri in langs.keys() {
        assert_eq!(keep.get(iri), Some(&"yes"), "{iri}");
    }
}

#[test]
fn test_committed_0803_curation_name_serves_34_link_objs_and_the_unsigned_band_untagged() {
    let names = curated("name", None);

    let mut expected_on = iris_of_class(LINK_OBJ);
    expected_on.insert(UNSIGNED_BAND);
    let other: BTreeMap<&str, &str> = names
        .iter()
        .filter(|(_, text)| **text != "identischer Holzschnitt")
        .map(|(iri, text)| (*iri, *text))
        .collect();
    assert_eq!(names.len(), 35);
    let misplaced: Vec<&str> = resources_of(&names).difference(&expected_on).copied().collect();
    assert!(
        misplaced.is_empty(),
        "a name on a Region, Book, Page or other Band: {misplaced:?}"
    );
    assert_eq!(resource(UNSIGNED_BAND).class.as_str(), BAND);
    assert_eq!(resource(UNSIGNED_BAND).label, "[missing]");
    assert_eq!(
        other,
        BTreeMap::from([
            ("http://rdfh.ch/0803/E7VBj9uBXKyrhvFLEc7Zrg", "im selben Band gebunden"),
            (UNSIGNED_BAND, "Randleiste 38"),
            ("http://rdfh.ch/0803/Z6pN6v4FUSOdLc_ghfCfng", "Übersetzung"),
            ("http://rdfh.ch/0803/_j3T1ZACWzGNux_T6tlnCA", "gleiche Druckermarke"),
            ("http://rdfh.ch/0803/oinJvxRSWC6VCHoXjEHrtA", "Holzschnitte identisch?"),
        ])
    );
}

#[test]
fn test_committed_0803_curation_title_override_and_sort_key_serve_their_books() {
    assert_eq!(
        curated("title_override", None),
        BTreeMap::from([
            (
                "http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ",
                "Zeitglöcklein des Lebens und Leidens Christi [1492]"
            ),
            (
                "http://rdfh.ch/0803/KyeQjCqTXLqLdFcRkEO9Rw",
                "[Das] Narrenschiff (lat.) [Aug. 1497]"
            ),
            (
                "http://rdfh.ch/0803/g3cP7N0-XuGSRFI52RIvig",
                "Zeitglöcklein des Lebens und Leidens Christi [1490]"
            ),
            (
                "http://rdfh.ch/0803/oZyOub3jUm2H0AGmZL3tyQ",
                "[Das] Narrenschiff (lat.) [März 1497]"
            ),
        ])
    );
    assert_eq!(
        curated("sort_key", None),
        BTreeMap::from([
            ("http://rdfh.ch/0803/KyeQjCqTXLqLdFcRkEO9Rw", "Narrenschiff (lat.) [Aug. 1497]"),
            ("http://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg", "Narrenschiff (dt.)"),
            ("http://rdfh.ch/0803/oZyOub3jUm2H0AGmZL3tyQ", "Narrenschiff (lat.) [März 1497]"),
        ])
    );
}

#[test]
fn test_committed_0803_curation_teaser_serves_four_books_in_german_and_english() {
    let german = curated("teaser", Some("de"));
    let english = curated("teaser", Some("en"));

    let books = BTreeSet::from([
        "http://rdfh.ch/0803/2B-ew2G6Vua3qoLmH9_5nw",
        "http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ",
        "http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ",
        "http://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg",
    ]);
    assert_eq!(resources_of(&german), books);
    assert_eq!(resources_of(&english), books);
    assert_eq!(
        english["http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ"],
        "The largest volume in the corpus, with three re-used woodcuts marked across its pages."
    );
}

#[test]
fn test_committed_0803_curation_note_column_is_in_the_file_and_not_served() {
    let region = "http://rdfh.ch/0803/0JJDCMvsV_e8ZQ5C-icf3w";
    let file = fs::read_to_string(format!("{DATA_DIR}/0803-curation.csv")).expect("read 0803-curation.csv");

    let row = file
        .lines()
        .find(|line| line.starts_with(&format!("{region},")))
        .expect("the Region has a row");
    let served: Vec<(&str, Option<&str>, &str)> = snapshot()
        .curation
        .iter()
        .filter(|value| value.resource.as_str() == region)
        .map(|value| (value.key.as_str(), value.lang.as_deref(), value.text.as_str()))
        .collect();
    assert!(row.ends_with(r#","questionable, kept for the owner (D7)""#), "{row}");
    assert_eq!(resource(region).class.as_str(), REGION);
    assert_eq!(served, vec![("keep", None, "yes")]);
}

#[test]
fn test_committed_0803_curation_second_call_serves_the_same_sorted_values() {
    let first = &snapshot().curation;

    let second = LiveArchiveProjection::new(DATA_DIR)
        .snapshot("0803")
        .expect("the committed 0803 files are served")
        .curation;
    assert!(first.windows(2).all(|pair| pair[0] < pair[1]), "sorted, without repeats");
    assert_eq!(first, &second);
}
