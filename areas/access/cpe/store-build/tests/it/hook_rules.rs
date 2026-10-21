//! Where a rule of the project's hook meets another rule, over hand-built snapshots: which of
//! the two reports, and that neither reports twice. The cases of `refusals.rs` each hold one
//! rule against the minimal snapshot; these hold a snapshot that two rules could claim.
//!
//! Every expectation is the rule and the subject of each refusal. The detail is compared only
//! where it names the curated key at fault.

use cpe_ports::ValueKind;
use cpe_store_build::error::Rule;

use crate::build;
use crate::refusals::{with_cologne_book, with_override, COLOGNE_BOOK, DEAD_COLOGNE_RECORD};
use crate::support::{
    band, built, changed, minimal, texts, with_curation, with_curation_in, with_label, with_link, with_page,
    with_resource, with_text, with_value, Fixture, ADDED, BOOK, HAS_BAND_TYPE_L, HAS_DESCRIPTION, HAS_URI, OTHER,
};

/// (rule, subject) of every refusal of `snapshot`, sorted.
fn refused(snapshot: &Fixture) -> Vec<(Rule, String)> {
    let mut refused: Vec<(Rule, String)> = match build(snapshot) {
        Ok(_) => Vec::new(),
        Err(errors) => {
            let errors = errors.as_slice().iter();
            errors.map(|error| (error.rule, error.subject.clone())).collect()
        }
    };
    refused.sort();
    refused
}

/// (rule, subject, detail) of every refusal of `snapshot`, sorted.
fn refused_with_detail(snapshot: &Fixture) -> Vec<(Rule, String, Option<String>)> {
    let mut refused: Vec<_> = match build(snapshot) {
        Ok(_) => Vec::new(),
        Err(errors) => {
            let errors = errors.as_slice().iter();
            errors
                .map(|error| (error.rule, error.subject.clone(), error.detail.clone()))
                .collect()
        }
    };
    refused.sort();
    refused
}

/// The minimal snapshot with `title` as its Book's label and as the Book's one title.
fn titled(title: &str) -> Fixture {
    changed(minimal(), BOOK, |book| {
        book.label = title.to_string();
        for value in &mut book.values {
            if let ValueKind::Text { text, .. } = &mut value.kind {
                *text = title.to_string();
            }
        }
    })
}

/// A sort key is judged by its first character once the whitespace before it is gone.
#[test]
fn test_build_title_that_starts_outside_ascii_behind_whitespace_is_no_sort_key() {
    assert_eq!(refused(&titled("\u{1f}Übung")), [(Rule::SortKeyNotAscii, BOOK.to_string())]);
    // The control: the same whitespace before an ASCII title is no defect, and the key is the
    // title as served.
    let store = built(&titled("\u{1f}Bereitung"));
    assert_eq!(texts(&store, "bereitung", "sort-key"), [(0, "\u{1f}Bereitung", None)]);
}

/// An overridden Book whose address the mapping refused, for its number or its kind, is not
/// also stale.
#[test]
fn test_build_overridden_book_whose_address_is_refused_is_not_also_stale() {
    let second = ValueKind::Uri("https://example.org/zweite".to_string());
    let two_addresses = with_cologne_book(Some(DEAD_COLOGNE_RECORD));
    let two_addresses = with_value(two_addresses, COLOGNE_BOOK, HAS_URI, second);
    let no_address = with_text(with_cologne_book(None), COLOGNE_BOOK, HAS_URI, "keine");

    assert_eq!(refused(&two_addresses), [(Rule::ValueCount, COLOGNE_BOOK.to_string())]);
    assert_eq!(refused(&no_address), [(Rule::UnexpectedValueKind, COLOGNE_BOOK.to_string())]);
}

/// The override is tested on the address as served, without the whitespace around it.
#[test]
fn test_build_overridden_address_with_whitespace_around_it_is_replaced() {
    let padded = format!(" {DEAD_COLOGNE_RECORD}\n");

    let store = built(&with_cologne_book(Some(&padded)));

    let addresses = texts(&store, "lob-der-glieder", "catalogue-uri");
    assert_eq!(addresses.len(), 1, "{addresses:?}");
    assert!(addresses[0].1.starts_with("https://search.ub.uni-koeln.de/"), "{addresses:?}");
}

/// An address that is no longer the dead one is stale, and is not also refused for the
/// sentinel it holds.
#[test]
fn test_build_overridden_book_whose_address_holds_the_sentinel_is_stale_and_nothing_else() {
    let snapshot = with_cologne_book(Some("http://example.org/[missing]"));

    assert_eq!(refused(&snapshot), [(Rule::StaleCatalogueOverride, COLOGNE_BOOK.to_string())]);
    // The control: on a Book without an override the sentinel is what is refused.
    let plain = ValueKind::Uri("http://example.org/[missing]".to_string());
    let plain = with_value(minimal(), BOOK, HAS_URI, plain);
    assert_eq!(refused(&plain), [(Rule::MissingInValue, BOOK.to_string())]);
}

/// Two replacements for one Book are too many, and are neither unpaired nor stale; the refusal
/// names the key. Nothing is overridden: the address the Book has is still checked, here for the
/// sentinel it holds. The port takes one value per key and language, so the second has a
/// language.
#[test]
fn test_build_book_with_two_curated_catalogue_addresses_is_refused_for_their_number_alone() {
    let second = |snapshot| {
        let address = "https://example.org/zweite";
        with_curation_in(snapshot, COLOGNE_BOOK, "catalogue_uri", "de", address)
    };
    let dead = second(with_cologne_book(Some(DEAD_COLOGNE_RECORD)));
    let sentinel = second(with_cologne_book(Some("http://example.org/[missing]")));

    assert_eq!(
        refused_with_detail(&dead),
        [(Rule::CuratedCount, COLOGNE_BOOK.to_string(), Some("catalogue_uri".to_string()))]
    );
    assert_eq!(
        refused(&sentinel),
        [
            (Rule::MissingInValue, COLOGNE_BOOK.to_string()),
            (Rule::CuratedCount, COLOGNE_BOOK.to_string()),
        ]
    );
}

/// Two dead addresses for one Book are too many as well, and the refusal names that key.
#[test]
fn test_build_book_with_two_curated_dead_addresses_is_refused_for_their_number_alone() {
    let snapshot = with_curation_in(
        with_cologne_book(Some(DEAD_COLOGNE_RECORD)),
        COLOGNE_BOOK,
        "catalogue_uri_replaces",
        "de",
        "http://example.org/zweite",
    );

    assert_eq!(
        refused_with_detail(&snapshot),
        [(
            Rule::CuratedCount,
            COLOGNE_BOOK.to_string(),
            Some("catalogue_uri_replaces".to_string())
        )]
    );
}

/// One key of the pair without the other is refused, and the refusal names the key that is
/// missing, not the one that is there.
#[test]
fn test_build_unpaired_catalogue_override_names_the_missing_key() {
    let replacement_alone = "https://example.org/katalog";
    let replacement_alone = with_curation(minimal(), BOOK, "catalogue_uri", replacement_alone);
    let dead_alone = "http://example.org/tot";
    let dead_alone = with_curation(minimal(), BOOK, "catalogue_uri_replaces", dead_alone);

    assert_eq!(
        refused_with_detail(&replacement_alone),
        [(
            Rule::UnpairedCatalogueOverride,
            BOOK.to_string(),
            Some("catalogue_uri_replaces".to_string())
        )]
    );
    assert_eq!(
        refused_with_detail(&dead_alone),
        [(
            Rule::UnpairedCatalogueOverride,
            BOOK.to_string(),
            Some("catalogue_uri".to_string())
        )]
    );
}

/// The override is the curated pair, on whichever Book the editors put it: the rule names none.
#[test]
fn test_build_curated_catalogue_override_replaces_the_address_of_any_book() {
    let dead = "http://old.example/record/7";
    let snapshot = with_value(minimal(), BOOK, HAS_URI, ValueKind::Uri(dead.to_string()));
    let snapshot = with_override(snapshot, BOOK, dead, "https://new.example/record/7");

    let store = built(&snapshot);

    assert_eq!(
        texts(&store, "bereitung", "catalogue-uri"),
        [(0, "https://new.example/record/7", None)]
    );
}

/// A description that holds the sentinel is content of a calibration scan, and no value to
/// refuse for the sentinel: the scan gets no row.
#[test]
fn test_build_calibration_scan_whose_description_holds_the_sentinel_is_refused_for_its_content() {
    let snapshot = with_text(with_page("[missing]"), ADDED, HAS_DESCRIPTION, "[missing]");

    assert_eq!(refused(&snapshot), [(Rule::CalibrationScanContent, ADDED.to_string())]);
    // The control: on a Page that is no calibration scan the sentinel is what is refused.
    let page = with_text(with_page("a1v"), ADDED, HAS_DESCRIPTION, "[missing]");
    assert_eq!(refused(&page), [(Rule::MissingInValue, ADDED.to_string())]);
}

/// A description of whitespace alone is no content: the scan is dropped without a word.
#[test]
fn test_build_calibration_scan_whose_description_is_whitespace_is_dropped() {
    let snapshot = with_text(with_page("[missing]"), ADDED, HAS_DESCRIPTION, " \u{1f}");

    let store = built(&snapshot);

    let pages = store.resources.iter().filter(|row| row.class == "Page");
    assert_eq!(pages.count(), 1);
}

/// A Page's strip that was refused, here for its scan, is not also a strip on the wrong side.
#[test]
fn test_build_strip_that_was_refused_is_not_held_against_the_page_that_links_to_it() {
    let snapshot = with_resource(with_page("a1v"), band(OTHER, "r_1", "Strip.tif"));
    let snapshot = with_link(snapshot, ADDED, HAS_BAND_TYPE_L, OTHER);

    assert_eq!(refused(&snapshot), [(Rule::NotJp2, OTHER.to_string())]);
}

/// A curated sort key is refused once: for the address it is, or for the whitespace around
/// it, which is found before the sentinel it holds.
#[test]
fn test_build_curated_sort_key_is_refused_by_one_rule() {
    let address = with_curation(minimal(), BOOK, "sort_key", "http://a.example");
    let padded_sentinel = with_curation(minimal(), BOOK, "sort_key", " [missing]");

    assert_eq!(refused(&address), [(Rule::HttpInValue, BOOK.to_string())]);
    assert_eq!(refused(&padded_sentinel), [(Rule::PaddedSortKey, BOOK.to_string())]);
}

/// The label is the name of a Book whose curated title is refused. With a curated sort key
/// that refusal is all; without one the label is the key, and one that starts outside ASCII
/// is refused as well.
#[test]
fn test_build_book_whose_curated_title_is_refused_takes_its_label_as_its_sort_key() {
    let refused_title = |snapshot| with_curation(snapshot, BOOK, "title_override", "[missing]");
    let with_key = with_curation(titled("Übung zum Sakrament"), BOOK, "sort_key", "Uebung");

    assert_eq!(refused(&refused_title(with_key)), [(Rule::MissingInValue, BOOK.to_string())]);
    assert_eq!(
        refused(&refused_title(titled("Übung zum Sakrament"))),
        [
            (Rule::MissingInValue, BOOK.to_string()),
            (Rule::SortKeyNotAscii, BOOK.to_string()),
        ]
    );
    // The control: under an accepted curated title the key is that title, not the label.
    let accepted = with_curation(titled("Bereitung"), BOOK, "title_override", "Vorbereitung");
    let store = built(&accepted);
    assert_eq!(texts(&store, "bereitung", "sort-key"), [(0, "Vorbereitung", None)]);
}

/// A label that may be no name is no sort key either: a Book whose curated title is refused and
/// whose label is empty, holds an address or holds the sentinel is refused for the title and
/// for the label, and not for a sort key. Each label would be refused as a key that starts
/// outside ASCII if it were taken for one.
#[test]
fn test_build_book_whose_curated_title_and_label_are_refused_is_not_refused_for_a_sort_key() {
    let cases = [
        ("", Rule::EmptyValue),
        ("Ünter http://a.example", Rule::HttpInValue),
        ("Über [missing]", Rule::MissingInValue),
    ];
    for (label, label_rule) in cases {
        let snapshot = with_label(minimal(), BOOK, label);
        let snapshot = with_curation(snapshot, BOOK, "title_override", "[missing]");

        let mut expected = vec![(label_rule, BOOK.to_string()), (Rule::MissingInValue, BOOK.to_string())];
        expected.sort();
        assert_eq!(refused(&snapshot), expected, "{label:?}");
    }
}
