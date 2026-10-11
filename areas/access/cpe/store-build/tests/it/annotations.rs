//! The annotation tables, over hand-built snapshots: the shape of each row, and what follows
//! from an annotation the editors do not keep.

use cpe_ports::{ProjectSnapshot, ValueKind};
use cpe_store_build::store::{AnnotationLinkRow, AnnotationRow, Store};

use crate::build;
use crate::support::{
    self, built, link_obj, minimal, region, with_annotation, with_curation, with_page, with_text, ADDED, BOOK,
    HAS_COLOR, HAS_LINK_TO, KB_HAS_COMMENT, OTHER, PAGE,
};

/// The ids of the minimal snapshot's Book and Page.
const BOOK_ID: &str = "bereitung";
const PAGE_ID: &str = "bereitung-a1r";

const REGION_A: &str = "http://rdfh.ch/0803/region-a";
const REGION_B: &str = "http://rdfh.ch/0803/region-b";
const LINK: &str = "http://rdfh.ch/0803/link";

/// The minimal snapshot with a Region on its Page, [`ADDED`], that the editors keep, with
/// `remark` and the curated language `lang` where one is given.
fn with_region(remark: Option<&str>, lang: Option<&str>) -> ProjectSnapshot {
    let snapshot = with_annotation(minimal(), region(ADDED, PAGE), "yes");
    let snapshot = match remark {
        Some(remark) => with_text(snapshot, ADDED, KB_HAS_COMMENT, remark),
        None => snapshot,
    };
    match lang {
        Some(lang) => with_curation(snapshot, ADDED, "lang", lang),
        None => snapshot,
    }
}

fn annotation(store: &Store) -> &AnnotationRow {
    let [row] = &store.annotations[..] else {
        panic!("one annotation: {:?}", store.annotations);
    };
    row
}

/// (remark, its language) of the one annotation.
fn remark(store: &Store) -> (Option<&str>, Option<&str>) {
    let row = annotation(store);
    (row.remark.as_deref(), row.remark_lang.as_deref())
}

#[test]
fn test_build_kept_region_is_an_annotation_on_its_page() {
    let store = built(&with_region(None, None));

    let row = annotation(&store);
    assert_eq!(
        (row.id.as_str(), row.resource_id.as_str(), row.shape.as_str()),
        ("added", PAGE_ID, "rectangle")
    );
    let numbers = [&row.x, &row.y, &row.w, &row.h].map(|real| real.as_str());
    assert_eq!(numbers, ["0.25", "0.25", "0.5", "0.25"]);
    assert_eq!(row.points, None);
    assert_eq!(row.color.as_deref(), Some("#ff3333"));
    assert_eq!(remark(&store), (None, None));
    assert_eq!(row.ark.as_deref(), Some("https://ark.dasch.swiss/ark:/72163/1/0803/added"));
    assert_eq!(store.annotation_links, []);
    assert_eq!(store.annotation_link_targets, []);
}

#[test]
fn test_build_region_the_editors_do_not_keep_has_no_row() {
    let snapshot = with_annotation(minimal(), region(ADDED, PAGE), "no");

    assert_eq!(built(&snapshot).annotations, []);
}

/// The control of the refusal case `RegionOnDroppedPage`: the same Region, not kept.
#[test]
fn test_build_region_on_a_calibration_scan_that_is_not_kept_is_not_refused() {
    let on_scan = |keep| with_annotation(with_page("[missing]"), region(OTHER, ADDED), keep);

    assert!(build(&on_scan("yes")).is_err());
    assert_eq!(built(&on_scan("no")).annotations, []);
}

#[test]
fn test_build_remark_is_stripped_and_takes_the_curated_language() {
    // U+001F and U+001E are whitespace to Python and not to `str::trim`.
    let store = built(&with_region(
        Some("\u{1f} Siehe  b8v\nin der deutschen Ausgabe.\u{1e}\n"),
        Some("de"),
    ));

    assert_eq!(remark(&store), (Some("Siehe  b8v\nin der deutschen Ausgabe."), Some("de")));
}

#[test]
fn test_build_remark_without_a_curated_language_has_none() {
    let store = built(&with_region(Some("San Francisco"), None));

    assert_eq!(remark(&store), (Some("San Francisco"), None));
}

#[test]
fn test_build_region_without_a_remark_has_no_remark_language() {
    for empty in [None, Some(" \n")] {
        let store = built(&with_region(empty, Some("de")));

        assert_eq!(remark(&store), (None, None), "{empty:?}");
    }
}

#[test]
fn test_build_colour_is_written_in_lower_case() {
    let mut region = region(ADDED, PAGE);
    for value in &mut region.values {
        if value.property.as_str() == HAS_COLOR {
            value.kind = ValueKind::Color("#FF33aa".to_string());
        }
    }

    let store = built(&with_annotation(minimal(), region, "yes"));

    assert_eq!(annotation(&store).color.as_deref(), Some("#ff33aa"));
}

/// The minimal snapshot with a kept Region `region-b`, a Region `region-a` the editors answer
/// `keep_a` about, and the kept LinkObj [`LINK`] to `targets`, named "identischer Holzschnitt".
fn with_link(keep_a: &str, targets: &[&str]) -> ProjectSnapshot {
    let snapshot = with_annotation(minimal(), region(REGION_B, PAGE), "yes");
    let snapshot = with_annotation(snapshot, region(REGION_A, PAGE), keep_a);
    let snapshot = with_annotation(snapshot, link_obj(LINK, targets), "yes");
    with_curation(snapshot, LINK, "name", "identischer Holzschnitt")
}

/// (link, seq, annotation, resource) of every target row.
fn targets(store: &Store) -> Vec<(&str, i64, Option<&str>, Option<&str>)> {
    store
        .annotation_link_targets
        .iter()
        .map(|row| {
            (
                row.link_id.as_str(),
                row.seq,
                row.annotation_id.as_deref(),
                row.resource_id.as_deref(),
            )
        })
        .collect()
}

#[test]
fn test_build_link_targets_are_numbered_from_1_by_kind_then_id() {
    let store = built(&with_link("yes", &[PAGE, BOOK, REGION_B, REGION_A]));

    assert_eq!(
        targets(&store),
        [
            ("link", 1, Some("region-a"), None),
            ("link", 2, Some("region-b"), None),
            ("link", 3, None, Some(BOOK_ID)),
            ("link", 4, None, Some(PAGE_ID)),
        ]
    );
}

#[test]
fn test_build_link_target_on_a_region_without_a_row_is_left_out_and_the_rest_renumbered() {
    let store = built(&with_link("no", &[PAGE, REGION_B, REGION_A]));

    assert_eq!(
        targets(&store),
        [("link", 1, Some("region-b"), None), ("link", 2, None, Some(PAGE_ID)),]
    );
}

#[test]
fn test_build_link_target_the_snapshot_names_twice_is_written_once() {
    let snapshot = with_link("yes", &[PAGE, REGION_B]);
    let snapshot = support::with_link(snapshot, LINK, HAS_LINK_TO, PAGE);

    assert_eq!(
        targets(&built(&snapshot)),
        [("link", 1, Some("region-b"), None), ("link", 2, None, Some(PAGE_ID)),]
    );
}

#[test]
fn test_build_link_left_without_a_target_is_not_written() {
    let store = built(&with_link("no", &[REGION_A]));

    assert_eq!(store.annotation_links, []);
    assert_eq!(store.annotation_link_targets, []);
    assert_eq!(store.annotations.len(), 1);
}

#[test]
fn test_build_link_the_editors_do_not_keep_has_no_row() {
    let snapshot = with_annotation(minimal(), link_obj(LINK, &[PAGE]), "no");

    let store = built(&snapshot);

    assert_eq!(store.annotation_links, []);
    assert_eq!(store.annotation_link_targets, []);
}

/// The classes are read by IRI; an id is the last part of one.
#[test]
fn test_build_annotation_rows_are_sorted_by_id_where_the_iris_sort_otherwise() {
    let mut snapshot = minimal();
    for iri in ["http://rdfh.ch/0803/a/zwei", "http://rdfh.ch/0803/b/eins"] {
        snapshot = with_annotation(snapshot, region(iri, PAGE), "yes");
    }
    for iri in ["http://rdfh.ch/0803/a/vier", "http://rdfh.ch/0803/b/drei"] {
        snapshot = with_annotation(snapshot, link_obj(iri, &[PAGE]), "yes");
        snapshot = with_curation(snapshot, iri, "name", "identischer Holzschnitt");
    }

    let store = built(&snapshot);

    let annotations: Vec<&str> = store.annotations.iter().map(|row| row.id.as_str()).collect();
    let links: Vec<&str> = store.annotation_links.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(annotations, ["eins", "zwei"]);
    assert_eq!(links, ["drei", "vier"]);
    assert_eq!(
        targets(&store),
        [("drei", 1, None, Some(PAGE_ID)), ("vier", 1, None, Some(PAGE_ID)),]
    );
}

/// The one link of the minimal snapshot with a kept LinkObj to its Page that has the curated
/// `name` and `lang` and the comment given.
fn link_row(name: Option<&str>, lang: Option<&str>, comment: Option<&str>) -> AnnotationLinkRow {
    let mut snapshot = with_annotation(minimal(), link_obj(LINK, &[PAGE]), "yes");
    for (key, text) in [("name", name), ("lang", lang)] {
        if let Some(text) = text {
            snapshot = with_curation(snapshot, LINK, key, text);
        }
    }
    if let Some(comment) = comment {
        snapshot = with_text(snapshot, LINK, KB_HAS_COMMENT, comment);
    }
    let [row] = &built(&snapshot).annotation_links[..] else {
        panic!("one link");
    };
    row.clone()
}

fn expected(
    name: &str,
    name_lang: Option<&str>,
    comment: Option<&str>,
    comment_lang: Option<&str>,
) -> AnnotationLinkRow {
    AnnotationLinkRow {
        id: "link".to_string(),
        name: name.to_string(),
        name_lang: name_lang.map(str::to_string),
        comment: comment.map(str::to_string),
        comment_lang: comment_lang.map(str::to_string),
    }
}

#[test]
fn test_build_link_name_and_comment_take_the_curated_language() {
    let comment = " Derselbe Holzschnitt wird auf den Seiten e3r und e4r verwendet.\n";

    assert_eq!(
        link_row(Some("identischer Holzschnitt"), Some("de"), Some(comment)),
        expected(
            "identischer Holzschnitt",
            Some("de"),
            Some("Derselbe Holzschnitt wird auf den Seiten e3r und e4r verwendet."),
            Some("de"),
        )
    );
}

#[test]
fn test_build_link_without_a_comment_has_a_name_language_and_no_comment_language() {
    assert_eq!(
        link_row(Some("identischer Holzschnitt"), Some("en"), None),
        expected("identischer Holzschnitt", Some("en"), None, None)
    );
    assert_eq!(
        link_row(Some("identischer Holzschnitt"), Some("en"), Some(" ")),
        expected("identischer Holzschnitt", Some("en"), None, None)
    );
}

#[test]
fn test_build_link_without_a_curated_language_has_neither_language() {
    assert_eq!(
        link_row(Some("identischer Holzschnitt"), None, Some("Siehe b8v.")),
        expected("identischer Holzschnitt", None, Some("Siehe b8v."), None)
    );
}

#[test]
fn test_build_link_without_a_curated_name_is_named_by_its_comment() {
    assert_eq!(
        link_row(None, Some("de"), Some("Siehe b8v.\n")),
        expected("Siehe b8v.", Some("de"), Some("Siehe b8v."), Some("de"))
    );
}
