//! The committed curation file of project 0803, read against the committed snapshot. The
//! expected values are facts of the file, which is edited by hand, so an expectation changes only
//! together with it. These tests know Incunabula's keys; the reader itself names none.

use std::collections::{BTreeMap, BTreeSet};

use cpe_ports::{Resource, ValueKind};

use crate::support::{
    BAND_CLASS as BAND, BOOK_CLASS as BOOK, HAS_URI, LINK_OBJ_CLASS as LINK_OBJ, PAGE_CLASS as PAGE,
    REGION_CLASS as REGION,
};
use crate::{CURATION, SNAPSHOT};

/// The one Band whose label is `[missing]`; its id and name are curated.
const UNSIGNED_BAND: &str = "http://rdfh.ch/0803/EJZQcYisXECHxchG_KQ27w";

fn resource(iri: &str) -> &'static Resource {
    SNAPSHOT
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == iri)
        .unwrap_or_else(|| panic!("{iri} is served"))
}

fn of_class(class: &str) -> Vec<&'static Resource> {
    SNAPSHOT
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

/// The curated values under `key` in `lang`, as text by resource IRI.
fn curated(key: &str, lang: Option<&str>) -> BTreeMap<&'static str, &'static str> {
    let mut by_resource = BTreeMap::new();
    for value in CURATION.as_slice() {
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
fn test_committed_0803_curation_reads_347_values_on_137_resources() {
    let curation = CURATION.as_slice();

    let resources: BTreeSet<&str> = curation.iter().map(|value| value.resource.as_str()).collect();
    assert_eq!(curation.len(), 347);
    assert_eq!(resources.len(), 137);
}

#[test]
fn test_committed_0803_curation_reads_thirteen_keys_and_languages_with_their_counts() {
    let mut counts: BTreeMap<(&str, Option<&str>), usize> = BTreeMap::new();
    for value in CURATION.as_slice() {
        *counts.entry((value.key.as_str(), value.lang.as_deref())).or_default() += 1;
    }

    assert_eq!(
        counts,
        BTreeMap::from([
            (("catalogue_uri", None), 3),
            (("catalogue_uri_replaces", None), 3),
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
fn test_committed_0803_curation_slug_reads_one_per_book_and_the_unsigned_band() {
    let slugs = curated("slug", None);

    let mut expected = iris_of_class(BOOK);
    assert_eq!(expected.len(), 19);
    expected.insert(UNSIGNED_BAND);
    assert_eq!(resources_of(&slugs), expected);
    assert_eq!(slugs[UNSIGNED_BAND], "strip-38");
    assert_eq!(slugs.values().collect::<BTreeSet<_>>().len(), 20);
}

#[test]
fn test_committed_0803_curation_office_reads_the_five_lanes_on_books_only() {
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
fn test_committed_0803_curation_date_display_reads_one_german_value_per_book() {
    let dates = curated("date_display", Some("de"));

    assert_eq!(resources_of(&dates), iris_of_class(BOOK));
    assert_eq!(dates["http://rdfh.ch/0803/PZcNpxILXBuKqGfglAZ4Vg"], "19. Februar 1491");
}

#[test]
fn test_committed_0803_curation_cover_page_reads_a_page_of_each_book_alone() {
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
fn test_committed_0803_curation_cover_page_reads_the_page_with_the_picked_label() {
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
fn test_committed_0803_curation_keep_reads_99_yes_and_18_no_on_every_region_and_link_obj() {
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
fn test_committed_0803_curation_lang_reads_93_de_and_4_en_on_kept_annotations_only() {
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
fn test_committed_0803_curation_name_reads_34_link_objs_and_the_unsigned_band_untagged() {
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
fn test_committed_0803_curation_title_override_and_sort_key_read_on_their_books() {
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
fn test_committed_0803_curation_catalogue_uri_reads_the_cologne_record_on_its_three_books() {
    let cologne = "https://search.ub.uni-koeln.de/portal/inkunabeln/databases/id/inkunabeln/titles/id/cdm_inkunabeln_1878.html?l=de";

    assert_eq!(
        curated("catalogue_uri", None),
        BTreeMap::from([
            ("http://rdfh.ch/0803/6jKWDQbjU-K2n6VNEjl8sA", cologne),
            ("http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ", cologne),
            ("http://rdfh.ch/0803/L9wVqnhfXA-7c5qmKBTylg", cologne),
        ])
    );
}

/// `catalogue_uri` stands in for an address the archive holds dead, and only while the archive
/// holds exactly that address. A regenerated `0803.nq` with the address mended fails here, and
/// the Book's two cells are then emptied.
#[test]
fn test_committed_0803_curation_catalogue_uri_replaces_reads_the_address_each_book_has() {
    let replaced = curated("catalogue_uri_replaces", None);

    assert_eq!(resources_of(&replaced), resources_of(&curated("catalogue_uri", None)));
    for (book, dead) in replaced {
        assert_eq!(
            kinds_under(resource(book), HAS_URI),
            vec![&ValueKind::Uri(dead.to_string())],
            "{book}"
        );
    }
}

#[test]
fn test_committed_0803_curation_teaser_reads_four_books_in_german_and_english() {
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
fn test_committed_0803_curation_note_column_is_in_the_file_and_not_read() {
    let region = "http://rdfh.ch/0803/0JJDCMvsV_e8ZQ5C-icf3w";
    let file = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../media/data/0803/curation.csv"));

    let row = file
        .lines()
        .find(|line| line.starts_with(&format!("{region},")))
        .expect("the Region has a row");
    let served: Vec<(&str, Option<&str>, &str)> = CURATION
        .as_slice()
        .iter()
        .filter(|value| value.resource.as_str() == region)
        .map(|value| (value.key.as_str(), value.lang.as_deref(), value.text.as_str()))
        .collect();
    assert!(row.ends_with(r#","questionable, kept for the owner (D7)""#), "{row}");
    assert_eq!(resource(region).class.as_str(), REGION);
    assert_eq!(served, vec![("keep", None, "yes")]);
}

#[test]
fn test_committed_0803_curation_reads_its_values_sorted_without_repeats() {
    let values = CURATION.as_slice();

    assert!(values.windows(2).all(|pair| pair[0] < pair[1]), "sorted, without repeats");
}
