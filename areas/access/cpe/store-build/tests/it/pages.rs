//! A Page's values, over hand-built snapshots: what the vendored data has no case of, and the
//! shape of each row.

use std::collections::BTreeSet;

use cpe_ports::ValueKind;

use crate::support::{
    band, built, minimal, rows, texts, with_link, with_page, with_resource, with_strip, with_text, with_value, ADDED,
    HAS_BAND_TYPE_L, HAS_BAND_TYPE_R, HAS_COMMENT, HAS_DESCRIPTION,
};

/// The id of the minimal snapshot's Page, and of the second Page [`with_page`] adds as `a1v`.
const FIRST: &str = "bereitung-a1r";
const SECOND: &str = "bereitung-a1v";

#[test]
fn test_build_minimal_page_has_the_values_every_page_has() {
    let store = built(&minimal());

    let names: Vec<&str> = store
        .properties
        .iter()
        .filter(|row| row.resource_id == FIRST)
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["image", "name", "page-id", "seq-number"]);
    assert_eq!(texts(&store, FIRST, "name"), [(0, "a1r", None)]);
    assert_eq!(texts(&store, FIRST, "page-id"), [(0, "a1r", None)]);

    let [number] = rows(&store, FIRST, "seq-number")[..] else {
        panic!("one sequence number");
    };
    assert_eq!((number.value_type.as_str(), number.value_int), ("integer", Some(1)));
    let [image] = rows(&store, FIRST, "image")[..] else {
        panic!("one image");
    };
    assert_eq!(
        (image.value_type.as_str(), image.target_id.as_deref()),
        ("link", Some("ScanOfThePage"))
    );
}

// U+001C and U+001F are whitespace to Python and not to `str::trim`.
#[test]
fn test_build_page_text_that_is_empty_once_stripped_gives_no_row() {
    let snapshot = with_text(with_page("a1v"), ADDED, HAS_DESCRIPTION, " \u{1f}\n");
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "Rubrizierung: Rot.\n");
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "\u{1c} ");
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "Initiale");

    let store = built(&snapshot);

    assert_eq!(texts(&store, SECOND, "description"), []);
    assert_eq!(
        texts(&store, SECOND, "comment"),
        [(0, "Rubrizierung: Rot.\n", None), (1, "Initiale", None)]
    );
}

#[test]
fn test_build_page_texts_are_written_as_served_in_the_ports_order_with_their_languages() {
    let german = ValueKind::Text {
        text: "Zweispaltig ".to_string(),
        lang: Some("de".to_string()),
    };
    let snapshot = with_text(with_page("a1v"), ADDED, HAS_DESCRIPTION, " Titelblatt\n");
    let snapshot = with_value(snapshot, ADDED, HAS_COMMENT, german);
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "Initiale");

    let store = built(&snapshot);

    assert_eq!(texts(&store, SECOND, "description"), [(0, " Titelblatt\n", None)]);
    assert_eq!(
        texts(&store, SECOND, "comment"),
        [(0, "Zweispaltig ", Some("de")), (1, "Initiale", None)]
    );
}

/// A record separator is cut where a text is compared, never in what is written.
#[test]
fn test_build_page_text_that_ends_in_a_record_separator_is_written_as_served() {
    let snapshot = with_text(with_page("a1v"), ADDED, HAS_COMMENT, "Rubrizierung: Rot.");
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "Initiale\u{1e}\n ");

    let store = built(&snapshot);

    assert_eq!(
        texts(&store, SECOND, "comment"),
        [(0, "Rubrizierung: Rot.", None), (1, "Initiale\u{1e}\n ", None)]
    );
}

#[test]
fn test_build_page_links_to_its_strip_on_each_side() {
    let right = "http://rdfh.ch/0803/right";
    let snapshot = with_strip("l_1", HAS_BAND_TYPE_L);
    let snapshot = with_resource(snapshot, band(right, "r_1", "ScanOfTheRightStrip.jp2"));
    let snapshot = with_link(snapshot, ADDED, HAS_BAND_TYPE_R, right);

    let store = built(&snapshot);

    for (name, strip) in [("strip-left", "l-1"), ("strip-right", "r-1")] {
        let [link] = rows(&store, SECOND, name)[..] else {
            panic!("one {name}");
        };
        assert_eq!(
            (link.value_type.as_str(), link.seq, link.target_id.as_deref()),
            ("link", 0, Some(strip))
        );
    }
    assert_eq!(rows(&store, FIRST, "strip-left").len(), 0);
}

#[test]
fn test_build_calibration_scan_has_no_value() {
    let store = built(&with_page("[missing]"));

    let with_values: BTreeSet<&str> = store.properties.iter().map(|row| row.resource_id.as_str()).collect();
    assert_eq!(with_values, BTreeSet::from(["bereitung", "ScanOfThePage", FIRST]));
}
