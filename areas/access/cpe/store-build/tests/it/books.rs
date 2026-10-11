//! A Book's values, over hand-built snapshots: what the committed 0803 data has no case of, and
//! the shape of each row.

use cpe_ports::{Calendar, DateBound, DatePrecision, DateValue, ValueKind};

use crate::refusals::{with_cologne_book, DEAD_COLOGNE_RECORD};
use crate::support::{
    built, changed, minimal, rows, texts, with_curation, with_curation_in, with_text, with_value, BOOK, BOOK_TITLE,
    HAS_CREATOR, HAS_PHYSICAL_DESC, HAS_PUBLISHER, HAS_PUBLOC, HAS_TITLE,
};

/// The id of the minimal snapshot's Book.
const ID: &str = "bereitung";

fn text_in(text: &str, lang: &str) -> ValueKind {
    ValueKind::Text { text: text.to_string(), lang: Some(lang.to_string()) }
}

#[test]
fn test_build_minimal_book_has_the_values_every_book_has() {
    let store = built(&minimal());

    let names: Vec<&str> = store
        .properties
        .iter()
        .filter(|row| row.resource_id == ID)
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["extent", "image", "name", "office", "pub-date", "sort-key"]);
    assert_eq!(texts(&store, ID, "name"), [(0, BOOK_TITLE, None)]);
    assert_eq!(texts(&store, ID, "sort-key"), [(0, BOOK_TITLE, None)]);

    let [extent] = rows(&store, ID, "extent")[..] else {
        panic!("one extent");
    };
    assert_eq!((extent.value_type.as_str(), extent.value_int), ("integer", Some(1)));
    let [image] = rows(&store, ID, "image")[..] else {
        panic!("one image");
    };
    assert_eq!(
        (image.value_type.as_str(), image.target_id.as_deref()),
        ("link", Some("ScanOfThePage"))
    );
    let [office] = rows(&store, ID, "office")[..] else {
        panic!("one office");
    };
    assert_eq!(
        (office.value_type.as_str(), office.target_id.as_deref()),
        ("list_value", Some("office-amerbach"))
    );
    let [date] = rows(&store, ID, "pub-date")[..] else {
        panic!("one publication date");
    };
    assert_eq!(date.value_type, "date");
    assert_eq!(
        [&date.date_start, &date.date_end, &date.date_precision],
        [
            &Some("1489".to_string()),
            &Some("1489".to_string()),
            &Some("year".to_string())
        ]
    );
    assert_eq!(
        [&date.date_calendar, &date.date_display, &date.value_lang],
        [
            &Some("julian".to_string()),
            &Some("1489".to_string()),
            &Some("de".to_string())
        ]
    );
    assert_eq!(date.value_text, None);
}

#[test]
fn test_build_date_is_written_in_its_own_calendar_at_its_precision() {
    // 15 October 1582 of the Gregorian calendar to 1 January 2000, by the month.
    let range = DateValue {
        calendar: Calendar::Gregorian,
        start: DateBound { jdn: 2_299_161, precision: DatePrecision::Month },
        end: DateBound { jdn: 2_451_545, precision: DatePrecision::Month },
    };
    let snapshot = changed(minimal(), BOOK, |book| {
        for value in &mut book.values {
            if matches!(value.kind, ValueKind::Date(_)) {
                value.kind = ValueKind::Date(range.clone());
            }
        }
    });

    let store = built(&snapshot);

    let [date] = rows(&store, ID, "pub-date")[..] else {
        panic!("one publication date");
    };
    assert_eq!(
        [
            &date.date_start,
            &date.date_end,
            &date.date_precision,
            &date.date_calendar
        ],
        [
            &Some("1582-10".to_string()),
            &Some("2000-01".to_string()),
            &Some("month".to_string()),
            &Some("gregorian".to_string())
        ]
    );
}

/// The port's order within a property is the store's, whatever stands between the values and
/// whatever their texts are.
#[test]
fn test_build_values_of_a_property_are_numbered_in_the_ports_order() {
    let creators = [
        "Sebastian Brant",
        "Jakob Locher",
        "Thomas Beccadelli",
        "Anonymus",
        "Zwingli",
        "Bergmann de Olpe",
        "anonymus",
        "Meynradus Sanctus",
        "Brant",
        "Albrecht von Bonstetten",
    ];
    let mut snapshot = minimal();
    for (index, creator) in creators.into_iter().enumerate() {
        snapshot = with_text(snapshot, BOOK, HAS_CREATOR, creator);
        if index % 3 == 0 {
            snapshot = with_text(snapshot, BOOK, HAS_PUBLISHER, creator);
        }
    }

    let store = built(&snapshot);

    let expected: Vec<(i64, &str, Option<&str>)> =
        (0..).zip(creators).map(|(seq, creator)| (seq, creator, None)).collect();
    assert_eq!(texts(&store, ID, "creator"), expected);
    let mut by_text = creators;
    by_text.sort_unstable();
    assert_ne!(by_text, creators, "the fixture is not in text order");
    assert_eq!(texts(&store, ID, "publisher").len(), 4);
}

#[test]
fn test_build_text_language_reaches_the_row() {
    let snapshot = with_value(minimal(), BOOK, HAS_CREATOR, text_in("Sebastianus Brant", "la"));
    let snapshot = with_text(snapshot, BOOK, HAS_CREATOR, "Sebastian Brant");

    let store = built(&snapshot);

    assert_eq!(
        texts(&store, ID, "creator"),
        [(0, "Sebastianus Brant", Some("la")), (1, "Sebastian Brant", None)]
    );
}

#[test]
fn test_build_text_is_written_as_served_and_an_empty_one_gives_no_row() {
    let snapshot = with_text(minimal(), BOOK, HAS_PHYSICAL_DESC, "[12] Bl. : Ill. ; 8° ");
    let snapshot = with_text(snapshot, BOOK, HAS_CREATOR, " \u{1f}");
    let snapshot = with_text(snapshot, BOOK, HAS_CREATOR, "Sebastian Brant");

    let store = built(&snapshot);

    assert_eq!(texts(&store, ID, "physical"), [(0, "[12] Bl. : Ill. ; 8° ", None)]);
    assert_eq!(texts(&store, ID, "creator"), [(0, "Sebastian Brant", None)]);
}

/// U+001F is whitespace to Python and not to `str::trim`.
#[test]
fn test_build_place_that_is_the_sentinel_gives_no_row() {
    let known = with_text(minimal(), BOOK, HAS_PUBLOC, "Basel");
    let unknown = with_text(minimal(), BOOK, HAS_PUBLOC, "[missing]\u{1f}");

    assert_eq!(texts(&built(&known), ID, "pub-place"), [(0, "Basel", None)]);
    assert_eq!(texts(&built(&unknown), ID, "pub-place"), []);
}

#[test]
fn test_build_alt_titles_are_the_titles_without_the_first_that_is_the_label() {
    let snapshot = with_text(minimal(), BOOK, HAS_TITLE, "Praeparatio ad sacramentum");
    let snapshot = with_text(snapshot, BOOK, HAS_TITLE, BOOK_TITLE);

    let store = built(&snapshot);

    assert_eq!(
        texts(&store, ID, "alt-title"),
        [(0, "Praeparatio ad sacramentum", None), (1, BOOK_TITLE, None)]
    );
}

/// A record separator is cut where a title is compared with the label, never in what is written.
#[test]
fn test_build_title_that_ends_in_a_record_separator_is_written_as_served() {
    let second = "Praeparatio ad sacramentum\u{1e}\n ";
    let snapshot = with_text(minimal(), BOOK, HAS_TITLE, second);

    let store = built(&snapshot);

    assert_eq!(texts(&store, ID, "alt-title"), [(0, second, None)]);
}

#[test]
fn test_build_curated_title_and_sort_key_win_over_the_label() {
    let overridden = with_curation(minimal(), BOOK, "title_override", " Bereitung (1489) ");
    let keyed = with_curation(overridden.clone(), BOOK, "sort_key", "Sakrament, Bereitung");

    let overridden = built(&overridden);
    let keyed = built(&keyed);

    assert_eq!(texts(&overridden, ID, "name"), [(0, "Bereitung (1489)", None)]);
    assert_eq!(texts(&overridden, ID, "sort-key"), [(0, "Bereitung (1489)", None)]);
    assert_eq!(texts(&keyed, ID, "name"), [(0, "Bereitung (1489)", None)]);
    assert_eq!(texts(&keyed, ID, "sort-key"), [(0, "Sakrament, Bereitung", None)]);
    assert_eq!(texts(&keyed, ID, "alt-title"), []);
}

#[test]
fn test_build_teasers_are_numbered_by_language_not_by_curation_order() {
    let snapshot = with_curation_in(minimal(), BOOK, "teaser", "en", "The largest volume.");
    let snapshot = with_curation_in(snapshot, BOOK, "teaser", "de", "Der umfangreichste Band.");

    let store = built(&snapshot);

    assert_eq!(
        texts(&store, ID, "teaser"),
        [
            (0, "Der umfangreichste Band.", Some("de")),
            (1, "The largest volume.", Some("en"))
        ]
    );
}

#[test]
fn test_build_overridden_catalogue_address_replaces_the_dead_one() {
    let store = built(&with_cologne_book(Some(DEAD_COLOGNE_RECORD)));

    let [(0, uri, None)] = texts(&store, "lob-der-glieder", "catalogue-uri")[..] else {
        panic!("one catalogue address");
    };
    assert!(uri.starts_with("https://search.ub.uni-koeln.de/"), "{uri}");
    assert_eq!(texts(&store, ID, "catalogue-uri"), []);
}
