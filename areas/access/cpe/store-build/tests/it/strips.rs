//! A border strip's values, over hand-built snapshots: what the vendored data has no case of,
//! and the shape of each row.

use cpe_ports::ValueKind;

use crate::support::{
    band, built, page, rows, texts, with_band, with_curation_in, with_link, with_resource, with_strip, with_text,
    with_value, ADDED, BOOK, HAS_BAND_TYPE_L, HAS_COMMENT, HAS_DESCRIPTION, OTHER,
};

/// The id of a Band labelled `l_1`.
const ID: &str = "l-1";

fn side_of(label: &str, id: &str) -> Vec<(String, Option<String>)> {
    let store = built(&with_band(label));
    rows(&store, id, "side")
        .into_iter()
        .map(|row| (row.value_type.clone(), row.target_id.clone()))
        .collect()
}

#[test]
fn test_build_strip_has_the_values_every_strip_has() {
    let store = built(&with_band("l_1"));

    let names: Vec<&str> = store
        .properties
        .iter()
        .filter(|row| row.resource_id == ID)
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["image", "name", "page-uses", "side"]);
    assert_eq!(texts(&store, ID, "name"), [(0, "l_1", None)]);

    let [uses] = rows(&store, ID, "page-uses")[..] else {
        panic!("one count of pages");
    };
    assert_eq!((uses.value_type.as_str(), uses.value_int), ("integer", Some(0)));
    let [image] = rows(&store, ID, "image")[..] else {
        panic!("one image");
    };
    assert_eq!(
        (image.value_type.as_str(), image.target_id.as_deref()),
        ("link", Some("ScanOfTheStrip"))
    );
}

#[test]
fn test_build_strip_side_is_the_one_its_label_names_or_none() {
    let node = |node: &str| vec![("list_value".to_string(), Some(node.to_string()))];

    assert_eq!(side_of("l_1", "l-1"), node("side-left"));
    assert_eq!(side_of("r_1", "r-1"), node("side-right"));
    assert_eq!(side_of("Randleiste", "randleiste"), []);
}

// U+001F is whitespace to Python and not to `str::trim`.
#[test]
fn test_build_strip_description_that_is_empty_or_the_sentinel_gives_no_row() {
    for description in ["[missing]", " [missing]\n", "", " \u{1f}\n"] {
        let store = built(&with_text(with_band("l_1"), ADDED, HAS_DESCRIPTION, description));

        assert_eq!(texts(&store, ID, "description"), [], "{description:?}");
    }
}

/// The sentinel stands for a value that is not there only when it is the whole text.
#[test]
fn test_build_strip_description_that_holds_the_sentinel_within_a_longer_text_is_written() {
    let described = "Rand [missing] unten";
    let snapshot = with_text(with_band("l_1"), ADDED, HAS_DESCRIPTION, described);

    let store = built(&snapshot);

    assert_eq!(texts(&store, ID, "description"), [(0, described, None)]);
}

#[test]
fn test_build_strip_texts_are_written_as_served_in_the_ports_order_with_their_languages() {
    let german = ValueKind::Text {
        text: "Seitenverkehrt zu r_1.\n".to_string(),
        lang: Some("de".to_string()),
    };
    let described = "Randleiste mit fünf Narren, 2.2 x 16.6 cm.\n        ";
    let snapshot = with_text(with_band("l_1"), ADDED, HAS_DESCRIPTION, described);
    let snapshot = with_value(snapshot, ADDED, HAS_COMMENT, german);
    let snapshot = with_text(snapshot, ADDED, HAS_COMMENT, "Auch in Blatt a2r.");

    let store = built(&snapshot);

    assert_eq!(texts(&store, ID, "description"), [(0, described, None)]);
    assert_eq!(
        texts(&store, ID, "comment"),
        [
            (0, "Seitenverkehrt zu r_1.\n", Some("de")),
            (1, "Auch in Blatt a2r.", None)
        ]
    );
}

#[test]
fn test_build_strip_curated_name_stands_for_its_title_without_its_language() {
    let snapshot = with_curation_in(with_band("l_1"), ADDED, "name", "de", " Randleiste 1 ");

    let store = built(&snapshot);

    assert_eq!(texts(&store, ID, "name"), [(0, "Randleiste 1", None)]);
}

#[test]
fn test_build_strip_page_uses_counts_the_pages_that_link_to_it() {
    let third = "http://rdfh.ch/0803/third";
    let right = "http://rdfh.ch/0803/right";
    let snapshot = with_strip("l_1", HAS_BAND_TYPE_L);
    let snapshot = with_resource(snapshot, page(third, BOOK, 3, "a2r", "ScanOfTheThirdPage.jp2"));
    let snapshot = with_link(snapshot, third, HAS_BAND_TYPE_L, OTHER);
    let snapshot = with_resource(snapshot, band(right, "r_1", "ScanOfTheRightStrip.jp2"));

    let store = built(&snapshot);

    let uses =
        |id: &str| -> Vec<Option<i64>> { rows(&store, id, "page-uses").into_iter().map(|row| row.value_int).collect() };
    assert_eq!(uses("l-1"), [Some(2)]);
    assert_eq!(uses("r-1"), [Some(0)]);
}
