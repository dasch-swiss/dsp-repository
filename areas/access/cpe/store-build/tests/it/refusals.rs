//! Refusal cases, at least one per rule, over hand-built snapshots. A rule without a case fails
//! `test_cases_produce_every_rule`.
//!
//! A case changes the minimal snapshot in one place, and where it can it adds a resource of its
//! own: a defect on the minimal Book or its Page would also break the rules that rely on them.
//!
//! After the cases, two tests of the order of refusals: two defects come back together in the
//! rules' order, and which of two refused curated values is reported does not follow the order
//! the snapshot lists them in.

use std::collections::BTreeSet;

use cpe_ports::{DatePrecision, DateValue, File, Motivation, Resource, ResourceIri, ValueKind};
use cpe_store_build::error::Rule;

use crate::build;
use crate::support::{
    self, annotation, band, changed, link_obj, minimal, page, recurated, region, resource, with_annotation, with_band,
    with_curation, with_curation_in, with_label, with_link, with_list_node, with_page, with_resource, with_strip,
    with_text, with_value, Fixture, ADDED, BOOK, BOOK_CLASS, HAS_BAND_TYPE_L, HAS_BAND_TYPE_R, HAS_COLOR, HAS_COMMENT,
    HAS_CREATOR, HAS_DESCRIPTION, HAS_GEOMETRY, HAS_LINK_TO, HAS_LOCATION, HAS_PAGENUM, HAS_PHYSICAL_DESC, HAS_PUBDATE,
    HAS_PUBLOC, HAS_TITLE, HAS_URI, IS_REGION_OF, KB_HAS_COMMENT, OTHER, PAGE, PAGE_ASSET, RECTANGLE, REGION_CLASS,
    STRIP_ASSET,
};

/// An IRI no resource of the minimal snapshot has.
const ABSENT: &str = "http://rdfh.ch/0803/absent";

/// `snapshot` with a second Book, [`OTHER`], that has `slug` and no cover.
fn with_book(snapshot: Fixture, slug: &str) -> Fixture {
    support::with_book(snapshot, OTHER, slug, "Zweites Buch")
}

/// `snapshot` with [`ADDED`] left without a value.
fn without_title(snapshot: Fixture) -> Fixture {
    changed(snapshot, ADDED, |band| band.values.clear())
}

/// A Book whose catalogue address the editors override, the address the override replaces, and
/// the address written in its place.
pub const COLOGNE_BOOK: &str = "http://rdfh.ch/0803/L9wVqnhfXA-7c5qmKBTylg";
pub const DEAD_COLOGNE_RECORD: &str =
    "http://www.ub.uni-koeln.de/cdm/compoundobject/collection/inkunabeln/id/1878/rec/1";
const COLOGNE_RECORD: &str =
    "https://search.ub.uni-koeln.de/portal/inkunabeln/databases/id/inkunabeln/titles/id/cdm_inkunabeln_1878.html?l=de";

/// `snapshot` with a curated catalogue override on `book`: `replacement` in place of `dead`.
pub fn with_override(snapshot: Fixture, book: &str, dead: &str, replacement: &str) -> Fixture {
    let snapshot = with_curation(snapshot, book, "catalogue_uri", replacement);
    with_curation(snapshot, book, "catalogue_uri_replaces", dead)
}

/// The minimal snapshot with [`COLOGNE_BOOK`], its cover [`ADDED`], its curated catalogue
/// override, and `uri` as its catalogue address.
pub fn with_cologne_book(uri: Option<&str>) -> Fixture {
    let snapshot = support::with_book(minimal(), COLOGNE_BOOK, "lob-der-glieder", "Lob der Glieder Mariens");
    let cover = page(ADDED, COLOGNE_BOOK, 1, "a1r", "ScanOfTheCologneBook.jp2");
    let snapshot = with_curation(with_resource(snapshot, cover), COLOGNE_BOOK, "cover_page", ADDED);
    let snapshot = with_override(snapshot, COLOGNE_BOOK, DEAD_COLOGNE_RECORD, COLOGNE_RECORD);
    match uri {
        Some(uri) => with_value(snapshot, COLOGNE_BOOK, HAS_URI, ValueKind::Uri(uri.to_string())),
        None => snapshot,
    }
}

/// The minimal snapshot with its Book's publication date changed by `change`.
fn with_pub_date(change: fn(&mut DateValue)) -> Fixture {
    changed(minimal(), BOOK, |book| {
        for value in &mut book.values {
            if let ValueKind::Date(date) = &mut value.kind {
                change(date);
            }
        }
    })
}

/// The minimal snapshot with a Region on its Page, [`ADDED`], changed by `change`. The editors
/// do not keep it: a check that refuses it runs before their answer is asked for.
fn with_unkept_region(change: impl FnOnce(&mut Resource)) -> Fixture {
    let mut region = region(ADDED, PAGE);
    change(&mut region);
    with_annotation(minimal(), region, "no")
}

/// [`with_unkept_region`] with `geometry` as the Region's geometry.
fn with_geometry(geometry: &str) -> Fixture {
    with_unkept_region(|region| {
        for value in &mut region.values {
            if value.property.as_str() == HAS_GEOMETRY {
                value.kind = ValueKind::Geometry(geometry.to_string());
            }
        }
    })
}

/// [`with_unkept_region`] with the Region's values under `property` taken away.
fn without_value(property: &str) -> Fixture {
    with_unkept_region(|region| {
        region.values.retain(|value| value.property.as_str() != property);
    })
}

/// A class the builder does not map.
const SIDEBAND_CLASS: &str = "http://www.knora.org/ontology/0803/incunabula#Sideband";

/// A LinkObj whose id is the id of the Region [`ADDED`].
const LINK_WITH_THE_REGIONS_ID: &str = "http://rdfh.ch/0803/links/added";

pub(crate) struct Case {
    pub(crate) rule: Rule,
    pub(crate) snapshot: fn() -> Fixture,
    /// What the one error names.
    pub(crate) subject: &'static str,
}

pub(crate) const CASES: &[Case] = &[
    Case {
        rule: Rule::ContractViolation,
        snapshot: || with_link(minimal(), PAGE, HAS_BAND_TYPE_L, ABSENT),
        subject: PAGE,
    },
    // On a label no rule reads: a Book's label with a NUL is also none of its titles.
    Case {
        rule: Rule::InteriorNul,
        snapshot: || with_label(minimal(), PAGE, "a1r\0"),
        subject: PAGE,
    },
    Case {
        rule: Rule::UnmappedClass,
        snapshot: || with_resource(minimal(), resource(ADDED, SIDEBAND_CLASS)),
        subject: ADDED,
    },
    Case {
        rule: Rule::AnnotationClassMismatch,
        snapshot: || {
            let link = annotation(ADDED, REGION_CLASS, Motivation::Linking, BOOK);
            with_resource(minimal(), link)
        },
        subject: ADDED,
    },
    // The same Page twice is two Pages to a Region.
    Case {
        rule: Rule::RegionTarget,
        snapshot: || with_link(with_unkept_region(|_| {}), ADDED, IS_REGION_OF, PAGE),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_list_node(minimal(), PAGE, HAS_PAGENUM),
        subject: PAGE,
    },
    Case {
        rule: Rule::BookWithoutCuration,
        snapshot: || with_resource(minimal(), resource(ADDED, BOOK_CLASS)),
        subject: ADDED,
    },
    Case {
        rule: Rule::SlugCount,
        snapshot: || with_curation_in(minimal(), BOOK, "slug", "de", "bereitung-de"),
        subject: BOOK,
    },
    // U+001F is whitespace to Python and not to `str::trim`.
    Case {
        rule: Rule::EmptySlug,
        snapshot: || recurated(minimal(), BOOK, "slug", Some(" \u{1f}")),
        subject: BOOK,
    },
    Case {
        rule: Rule::DuplicateSlug,
        snapshot: || with_book(minimal(), "bereitung"),
        subject: "bereitung",
    },
    Case {
        rule: Rule::HttpInValue,
        snapshot: || recurated(minimal(), BOOK, "slug", Some("http-bereitung")),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || recurated(minimal(), BOOK, "slug", Some("bereitung-[missing]")),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || recurated(minimal(), BOOK, "cover_page", Some("[missing]")),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || with_resource(minimal(), band(ADDED, "[missing]", STRIP_ASSET)),
        subject: ADDED,
    },
    Case {
        rule: Rule::CoverCount,
        snapshot: || recurated(minimal(), BOOK, "cover_page", None),
        subject: BOOK,
    },
    // The second Book's cover is a Page of the first that the first does not use.
    Case {
        rule: Rule::CoverNotOwnPage,
        snapshot: || {
            let snapshot = with_book(with_page("a1v"), "zweites");
            with_curation(snapshot, OTHER, "cover_page", ADDED)
        },
        subject: OTHER,
    },
    Case {
        rule: Rule::CoverIsCalibrationScan,
        snapshot: || recurated(with_page("[missing]"), BOOK, "cover_page", Some(ADDED)),
        subject: BOOK,
    },
    Case {
        rule: Rule::SharedCover,
        snapshot: || {
            let snapshot = with_book(minimal(), "zweites");
            with_curation(snapshot, OTHER, "cover_page", PAGE)
        },
        subject: PAGE,
    },
    Case {
        rule: Rule::PageParent,
        snapshot: || changed(with_page("a1v"), ADDED, |page| page.part_of.clear()),
        subject: ADDED,
    },
    Case {
        rule: Rule::PageParent,
        snapshot: || {
            changed(with_page("a1v"), ADDED, |page| {
                page.part_of.push(ResourceIri(PAGE.to_string()));
            })
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::PageParent,
        snapshot: || {
            changed(with_page("a1v"), ADDED, |page| {
                page.part_of = vec![ResourceIri(PAGE.to_string())];
            })
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::PageWithoutSeqnum,
        snapshot: || changed(with_page("a1v"), ADDED, |page| page.seqnum = None),
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || with_text(with_page("a1v"), ADDED, HAS_PAGENUM, "a2r"),
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || changed(with_page("a1v"), ADDED, |page| page.values.clear()),
        subject: ADDED,
    },
    Case {
        rule: Rule::EmptyValue,
        snapshot: || with_page(" \u{1c}"),
        subject: ADDED,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || with_page("a1v [missing]"),
        subject: ADDED,
    },
    Case {
        rule: Rule::CalibrationScanContent,
        snapshot: || with_text(with_page("[missing]"), ADDED, HAS_DESCRIPTION, "Farbkarte"),
        subject: ADDED,
    },
    Case {
        rule: Rule::CalibrationScanContent,
        snapshot: || with_text(with_page("[missing]"), ADDED, HAS_COMMENT, "Farbkarte"),
        subject: ADDED,
    },
    Case {
        rule: Rule::CalibrationScanContent,
        snapshot: || {
            let strip = band(OTHER, "l_1", STRIP_ASSET);
            let snapshot = with_resource(with_page("[missing]"), strip);
            with_link(snapshot, ADDED, HAS_BAND_TYPE_L, OTHER)
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::EmptyIdSlug,
        snapshot: || with_page("[ ; ]"),
        subject: ADDED,
    },
    Case {
        rule: Rule::EmptyIdSlug,
        snapshot: || with_resource(minimal(), band(ADDED, "[ ]", STRIP_ASSET)),
        subject: ADDED,
    },
    // The second `a1r` is suffixed `-2`, and `a1r 2` slugs to the same id.
    Case {
        rule: Rule::DuplicateId,
        snapshot: || {
            let third = page(OTHER, BOOK, 3, "a1r 2", "ScanOfTheThirdPage.jp2");
            with_resource(with_page("a1r"), third)
        },
        subject: "bereitung-a1r-2",
    },
    Case {
        rule: Rule::DuplicateId,
        snapshot: || with_resource(minimal(), band(ADDED, "Bereitung", STRIP_ASSET)),
        subject: "bereitung",
    },
    Case {
        rule: Rule::NotStillImage,
        snapshot: || changed(with_page("a1v"), ADDED, |page| page.file = None),
        subject: ADDED,
    },
    Case {
        rule: Rule::NotStillImage,
        snapshot: || {
            changed(with_page("a1v"), ADDED, |page| {
                page.file = Some(File::Document { asset: "ScanOfTheSecondPage.pdf".to_string() });
            })
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::EmptyStem,
        snapshot: || with_resource(minimal(), band(ADDED, "l_1", ".jp2")),
        subject: ADDED,
    },
    Case {
        rule: Rule::NotJp2,
        snapshot: || with_resource(minimal(), band(ADDED, "l_1", "ScanOfTheStrip.tif")),
        subject: ADDED,
    },
    Case {
        rule: Rule::ZeroDimension,
        snapshot: || {
            changed(with_page("a1v"), ADDED, |page| {
                page.file = Some(File::StillImage {
                    asset: "ScanOfTheSecondPage.jp2".to_string(),
                    width: 1200,
                    height: 0,
                });
            })
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::SharedFile,
        snapshot: || with_resource(minimal(), page(ADDED, BOOK, 2, "a1v", PAGE_ASSET)),
        subject: "ScanOfThePage",
    },
    Case {
        rule: Rule::SharedFile,
        snapshot: || {
            let snapshot = with_resource(minimal(), band(ADDED, "l_1", STRIP_ASSET));
            with_resource(snapshot, band(OTHER, "r_1", STRIP_ASSET))
        },
        subject: "ScanOfTheStrip",
    },
    Case {
        rule: Rule::ImageIdTaken,
        snapshot: || with_resource(minimal(), page(ADDED, BOOK, 2, "a1v", "bereitung.jp2")),
        subject: "bereitung",
    },
    Case {
        rule: Rule::ImageIdTaken,
        snapshot: || with_resource(minimal(), band(ADDED, "l_1", PAGE_ASSET)),
        subject: "ScanOfThePage",
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_list_node(minimal(), BOOK, HAS_CREATOR),
        subject: BOOK,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_text(minimal(), BOOK, HAS_URI, "im Katalog der Bibliothek"),
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedCount,
        snapshot: || recurated(minimal(), BOOK, "office", None),
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedCount,
        snapshot: || recurated(minimal(), BOOK, "date_display", None),
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedCount,
        snapshot: || {
            let snapshot = with_curation(minimal(), BOOK, "title_override", "Bereitung");
            with_curation_in(snapshot, BOOK, "title_override", "de", "Die Bereitung")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::UnknownListNode,
        snapshot: || recurated(minimal(), BOOK, "office", Some("froben")),
        subject: BOOK,
    },
    Case {
        rule: Rule::DateDisplayWithoutLanguage,
        snapshot: || recurated(minimal(), BOOK, "date_display", Some("1489")),
        subject: BOOK,
    },
    Case {
        rule: Rule::UnknownDateDisplayLanguage,
        snapshot: || {
            let snapshot = recurated(minimal(), BOOK, "date_display", None);
            with_curation_in(snapshot, BOOK, "date_display", "fr", "1489")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::PaddedSortKey,
        snapshot: || with_curation(minimal(), BOOK, "sort_key", "Bereitung "),
        subject: BOOK,
    },
    Case {
        rule: Rule::SortKeyNotAscii,
        snapshot: || with_curation(minimal(), BOOK, "sort_key", "Übung"),
        subject: BOOK,
    },
    // Without a curated sort key, the title is the key.
    Case {
        rule: Rule::SortKeyNotAscii,
        snapshot: || with_curation(minimal(), BOOK, "title_override", "Übung zum Sakrament"),
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedLanguages,
        snapshot: || with_curation_in(minimal(), BOOK, "teaser", "de", "Der umfangreichste Band."),
        subject: BOOK,
    },
    Case {
        rule: Rule::HttpInValue,
        snapshot: || with_curation(minimal(), BOOK, "title_override", "http://example.org/titel"),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = with_curation_in(minimal(), BOOK, "teaser", "de", "[missing]");
            with_curation_in(snapshot, BOOK, "teaser", "en", "The largest volume.")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::EmptyValue,
        snapshot: || with_label(minimal(), BOOK, " \u{1f}"),
        subject: BOOK,
    },
    Case {
        rule: Rule::LabelNotATitle,
        snapshot: || with_label(minimal(), BOOK, "Bereitung"),
        subject: BOOK,
    },
    Case {
        rule: Rule::HttpInValue,
        snapshot: || with_label(minimal(), BOOK, "http://example.org/titel"),
        subject: BOOK,
    },
    Case {
        rule: Rule::HttpInValue,
        snapshot: || with_text(minimal(), BOOK, HAS_PHYSICAL_DESC, "siehe http://example.org"),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || with_text(minimal(), BOOK, HAS_CREATOR, "[missing]"),
        subject: BOOK,
    },
    // Only the sentinel alone stands for an unknown place.
    Case {
        rule: Rule::MissingInValue,
        snapshot: || with_text(minimal(), BOOK, HAS_PUBLOC, "Basel [missing]"),
        subject: BOOK,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || {
            changed(minimal(), BOOK, |book| {
                book.values.retain(|value| value.property.as_str() != HAS_PUBDATE);
            })
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || {
            let snapshot = with_text(minimal(), BOOK, HAS_LOCATION, "Basel UB, Sign: Aleph D III 13");
            with_text(snapshot, BOOK, HAS_LOCATION, "Basel UB, Sign: FP I 2")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::StaleCatalogueOverride,
        snapshot: || with_cologne_book(Some("https://example.org/mended")),
        subject: COLOGNE_BOOK,
    },
    Case {
        rule: Rule::StaleCatalogueOverride,
        snapshot: || with_cologne_book(None),
        subject: COLOGNE_BOOK,
    },
    // Each of the two curated keys without the other.
    Case {
        rule: Rule::UnpairedCatalogueOverride,
        snapshot: || with_curation(minimal(), BOOK, "catalogue_uri", "https://example.org/katalog"),
        subject: BOOK,
    },
    Case {
        rule: Rule::UnpairedCatalogueOverride,
        snapshot: || with_curation(minimal(), BOOK, "catalogue_uri_replaces", "http://example.org/tot"),
        subject: BOOK,
    },
    Case {
        rule: Rule::MixedDatePrecision,
        snapshot: || with_pub_date(|date| date.end.precision = DatePrecision::Day),
        subject: BOOK,
    },
    // A value of the wrong kind is no missing value: one refusal, not two.
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            changed(minimal(), BOOK, |book| {
                for value in &mut book.values {
                    if value.property.as_str() == HAS_PUBDATE {
                        value.kind = ValueKind::Uri("1489".to_string());
                    }
                }
            })
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            changed(minimal(), BOOK, |book| {
                for value in &mut book.values {
                    if value.property.as_str() == HAS_TITLE {
                        value.kind = ValueKind::Uri("Bereitung".to_string());
                    }
                }
            })
        },
        subject: BOOK,
    },
    // Two values of one property that break one rule are one refusal.
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = with_text(minimal(), BOOK, HAS_CREATOR, "[missing]");
            with_text(snapshot, BOOK, HAS_CREATOR, "[missing] ")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedLanguages,
        snapshot: || {
            let snapshot = with_curation_in(minimal(), BOOK, "teaser", "de", "Der Band.");
            let snapshot = with_curation_in(snapshot, BOOK, "teaser", "en", "The volume.");
            with_curation(snapshot, BOOK, "teaser", "A volume.")
        },
        subject: BOOK,
    },
    // The first day of the year 10000, in the Julian calendar: only the end is out of range.
    Case {
        rule: Rule::DateOutOfRange,
        snapshot: || with_pub_date(|date| date.end.jdn = 5_373_558),
        subject: BOOK,
    },
    // The last day of the year before 1, in the Julian calendar.
    Case {
        rule: Rule::DateOutOfRange,
        snapshot: || with_pub_date(|date| date.start.jdn = 1_721_423),
        subject: BOOK,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || with_text(with_page("a1v"), ADDED, HAS_DESCRIPTION, "Blatt [missing]"),
        subject: ADDED,
    },
    // Two comments that hold the sentinel are one refusal.
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = with_text(with_page("a1v"), ADDED, HAS_COMMENT, "[missing]");
            with_text(snapshot, ADDED, HAS_COMMENT, "Rubrizierung: [missing]")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || {
            let snapshot = with_text(with_page("a1v"), ADDED, HAS_DESCRIPTION, "Titelblatt");
            with_text(snapshot, ADDED, HAS_DESCRIPTION, "Holzschnitt")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || {
            let second = band("http://rdfh.ch/0803/second", "l_2", "ScanOfTheSecondStrip.jp2");
            let snapshot = with_resource(with_strip("l_1", HAS_BAND_TYPE_L), second);
            with_link(snapshot, ADDED, HAS_BAND_TYPE_L, "http://rdfh.ch/0803/second")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::StripLinkTarget,
        snapshot: || with_link(with_page("a1v"), ADDED, HAS_BAND_TYPE_L, BOOK),
        subject: ADDED,
    },
    Case {
        rule: Rule::StripSideMismatch,
        snapshot: || with_strip("r_1", HAS_BAND_TYPE_L),
        subject: ADDED,
    },
    // A strip of no side stands on neither.
    Case {
        rule: Rule::StripSideMismatch,
        snapshot: || with_strip("Randleiste", HAS_BAND_TYPE_R),
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || without_title(with_band("l_1")),
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || with_text(with_band("l_1"), ADDED, HAS_TITLE, "l_1a"),
        subject: ADDED,
    },
    // The label names the side and makes the id; only the title is the sentinel.
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = without_title(with_band("l_1"));
            with_text(snapshot, ADDED, HAS_TITLE, "[missing]\n")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::CuratedCount,
        snapshot: || {
            let snapshot = with_curation(with_band("l_1"), ADDED, "name", "Randleiste 1");
            with_curation_in(snapshot, ADDED, "name", "de", "Erste Randleiste")
        },
        subject: ADDED,
    },
    // A strip whose curated name is refused is not also reported for the title it lacks.
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = without_title(with_band("l_1"));
            with_curation(snapshot, ADDED, "name", "[missing]")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::HttpInValue,
        snapshot: || with_curation(with_band("l_1"), ADDED, "name", "http://example.org/l_1"),
        subject: ADDED,
    },
    Case {
        rule: Rule::ValueCount,
        snapshot: || {
            let snapshot = with_text(with_band("l_1"), ADDED, HAS_DESCRIPTION, "Fünf Narren.");
            with_text(snapshot, ADDED, HAS_DESCRIPTION, "2.2 x 16.6 cm.")
        },
        subject: ADDED,
    },
    // U+001F is whitespace to Python and not to `str::trim`.
    Case {
        rule: Rule::EmptyValue,
        snapshot: || with_text(with_band("l_1"), ADDED, HAS_COMMENT, " \u{1f}"),
        subject: ADDED,
    },
    Case {
        rule: Rule::MissingInValue,
        snapshot: || {
            let snapshot = with_text(with_band("l_1"), ADDED, HAS_COMMENT, "Auch in Blatt a2r.");
            with_text(snapshot, ADDED, HAS_COMMENT, "[missing]\n")
        },
        subject: ADDED,
    },
    // A value of the wrong kind is one refusal: the texts beside it are not judged without it.
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(with_page("a1v"), ADDED, HAS_DESCRIPTION, BOOK);
            let snapshot = with_text(snapshot, ADDED, HAS_DESCRIPTION, "Titelblatt");
            with_text(snapshot, ADDED, HAS_DESCRIPTION, "Holzschnitt")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(with_page("a1v"), ADDED, HAS_COMMENT, BOOK);
            with_text(snapshot, ADDED, HAS_COMMENT, "Rubrizierung: [missing]")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(with_band("l_1"), ADDED, HAS_DESCRIPTION, BOOK);
            let snapshot = with_text(snapshot, ADDED, HAS_DESCRIPTION, "Fünf Narren.");
            with_text(snapshot, ADDED, HAS_DESCRIPTION, "2.2 x 16.6 cm.")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(with_band("l_1"), ADDED, HAS_COMMENT, BOOK);
            with_text(snapshot, ADDED, HAS_COMMENT, "[missing]")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(minimal(), BOOK, HAS_PHYSICAL_DESC, PAGE);
            let snapshot = with_text(snapshot, BOOK, HAS_PHYSICAL_DESC, "4°");
            with_text(snapshot, BOOK, HAS_PHYSICAL_DESC, "siehe http://example.org")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || {
            let snapshot = with_link(minimal(), BOOK, HAS_CREATOR, PAGE);
            with_text(snapshot, BOOK, HAS_CREATOR, "[missing]")
        },
        subject: BOOK,
    },
    // A strip whose title is of the wrong kind is not also reported for having none.
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_link(with_band("l_1"), ADDED, HAS_TITLE, BOOK),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_text(with_page("a1v"), ADDED, HAS_BAND_TYPE_L, "l_1"),
        subject: ADDED,
    },
    // One strip on both sides of a Page stands on the wrong one once.
    Case {
        rule: Rule::StripSideMismatch,
        snapshot: || with_link(with_strip("l_1", HAS_BAND_TYPE_L), ADDED, HAS_BAND_TYPE_R, OTHER),
        subject: ADDED,
    },
    // The strip was refused for its scan: its Page is not refused for the link to it.
    Case {
        rule: Rule::NotJp2,
        snapshot: || {
            let strip = band(OTHER, "l_1", "ScanOfTheStrip.tif");
            let snapshot = with_resource(with_page("a1v"), strip);
            with_link(snapshot, ADDED, HAS_BAND_TYPE_L, OTHER)
        },
        subject: OTHER,
    },
    Case {
        rule: Rule::UnknownCuratedKey,
        snapshot: || with_curation(minimal(), BOOK, "sulg", "bereitung"),
        subject: BOOK,
    },
    // In two languages, the key is refused once.
    Case {
        rule: Rule::UnknownCuratedKey,
        snapshot: || {
            let snapshot = with_curation_in(minimal(), BOOK, "blurb", "de", "Der Band.");
            with_curation_in(snapshot, BOOK, "blurb", "en", "The volume.")
        },
        subject: BOOK,
    },
    Case {
        rule: Rule::CuratedKeyOnWrongClass,
        snapshot: || with_curation(minimal(), PAGE, "office", "amerbach"),
        subject: PAGE,
    },
    Case {
        rule: Rule::CuratedKeyOnWrongClass,
        snapshot: || with_curation(minimal(), BOOK, "keep", "yes"),
        subject: BOOK,
    },
    // A name is a Band's and a LinkObj's.
    Case {
        rule: Rule::CuratedKeyOnWrongClass,
        snapshot: || {
            let name = "identischer Holzschnitt";
            with_curation(with_unkept_region(|_| {}), ADDED, "name", name)
        },
        subject: ADDED,
    },
    // The annotation cases. Where a check is the archive's own, the editors do not keep the
    // annotation: the check runs before their answer is asked for.
    Case {
        rule: Rule::CuratedCount,
        snapshot: || with_resource(minimal(), region(ADDED, PAGE)),
        subject: ADDED,
    },
    Case {
        rule: Rule::KeepNotYesOrNo,
        snapshot: || with_annotation(minimal(), region(ADDED, PAGE), "maybe"),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnknownAnnotationLanguage,
        snapshot: || with_curation(with_unkept_region(|_| {}), ADDED, "lang", "fr"),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnknownAnnotationLanguage,
        snapshot: || {
            let link = link_obj(ADDED, &[PAGE]);
            with_curation(with_annotation(minimal(), link, "no"), ADDED, "lang", "fr")
        },
        subject: ADDED,
    },
    // A link's id and a Region's are one id space.
    Case {
        rule: Rule::DuplicateAnnotationId,
        snapshot: || {
            let link = link_obj(LINK_WITH_THE_REGIONS_ID, &[PAGE]);
            with_annotation(with_unkept_region(|_| {}), link, "no")
        },
        subject: "added",
    },
    Case {
        rule: Rule::AnnotationValueCount,
        snapshot: || {
            let geometry = ValueKind::Geometry(RECTANGLE.to_string());
            with_value(with_unkept_region(|_| {}), ADDED, HAS_GEOMETRY, geometry)
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::AnnotationValueCount,
        snapshot: || without_value(HAS_COLOR),
        subject: ADDED,
    },
    Case {
        rule: Rule::AnnotationValueCount,
        snapshot: || {
            let snapshot = with_annotation(minimal(), link_obj(ADDED, &[PAGE]), "no");
            let snapshot = with_text(snapshot, ADDED, KB_HAS_COMMENT, "Derselbe Holzschnitt.");
            with_text(snapshot, ADDED, KB_HAS_COMMENT, "Siehe b8v.")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::GeometryNotRead,
        snapshot: || with_geometry(r#"{"type":"rectangle"}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnknownShape,
        snapshot: || with_geometry(r#"{"type":"ellipse","points":[{"x":0.25,"y":0.25},{"x":0.75,"y":0.5}]}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::ShapePointCount,
        snapshot: || with_geometry(r#"{"type":"rectangle","points":[{"x":0.25,"y":0.25}]}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::EmptyBox,
        snapshot: || with_geometry(r#"{"type":"rectangle","points":[{"x":0.25,"y":0.25},{"x":0.25,"y":0.5}]}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::BoxOutsideImage,
        snapshot: || with_geometry(r#"{"type":"rectangle","points":[{"x":0.25,"y":0.25},{"x":1.5,"y":0.5}]}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::UnwritableNumber,
        snapshot: || with_geometry(r#"{"type":"rectangle","points":[{"x":0.00001,"y":0.25},{"x":0.75,"y":0.5}]}"#),
        subject: ADDED,
    },
    Case {
        rule: Rule::ColourNotHex,
        snapshot: || {
            with_unkept_region(|region| {
                for value in &mut region.values {
                    if value.property.as_str() == HAS_COLOR {
                        value.kind = ValueKind::Color("red".to_string());
                    }
                }
            })
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::RemarkHoldsRecordSeparator,
        snapshot: || {
            let remark = "Derselbe\u{1e}Holzschnitt.";
            with_text(with_unkept_region(|_| {}), ADDED, KB_HAS_COMMENT, remark)
        },
        subject: ADDED,
    },
    // The Region is the region of nothing: its target is under another property.
    Case {
        rule: Rule::RegionTarget,
        snapshot: || without_value(IS_REGION_OF),
        subject: ADDED,
    },
    Case {
        rule: Rule::RegionTarget,
        snapshot: || with_link(with_unkept_region(|_| {}), ADDED, IS_REGION_OF, BOOK),
        subject: ADDED,
    },
    Case {
        rule: Rule::RegionTarget,
        snapshot: || with_annotation(minimal(), region(ADDED, BOOK), "no"),
        subject: ADDED,
    },
    // [`ADDED`] is a calibration scan, and the editors keep the Region on it.
    Case {
        rule: Rule::RegionOnDroppedPage,
        snapshot: || with_annotation(with_page("[missing]"), region(OTHER, ADDED), "yes"),
        subject: OTHER,
    },
    // What points at a resource of an unmapped class is not refused for it a second time.
    Case {
        rule: Rule::UnmappedClass,
        snapshot: || {
            let snapshot = with_resource(minimal(), resource(OTHER, SIDEBAND_CLASS));
            with_annotation(snapshot, region(ADDED, OTHER), "no")
        },
        subject: OTHER,
    },
    Case {
        rule: Rule::UnmappedClass,
        snapshot: || {
            let snapshot = with_resource(minimal(), resource(OTHER, SIDEBAND_CLASS));
            with_annotation(snapshot, link_obj(ADDED, &[OTHER, PAGE]), "no")
        },
        subject: OTHER,
    },
    // Nor what points at an annotation whose class does not fit its motivation.
    Case {
        rule: Rule::AnnotationClassMismatch,
        snapshot: || {
            let misfit = annotation(ADDED, REGION_CLASS, Motivation::Linking, BOOK);
            with_annotation(with_resource(minimal(), misfit), link_obj(OTHER, &[ADDED]), "no")
        },
        subject: ADDED,
    },
    // The same unknown target twice is one mistake.
    Case {
        rule: Rule::LinkTarget,
        snapshot: || {
            let link = link_obj(OTHER, &[ADDED, PAGE]);
            let snapshot = with_annotation(with_band("l_1"), link, "no");
            with_link(snapshot, OTHER, HAS_LINK_TO, ADDED)
        },
        subject: OTHER,
    },
    Case {
        rule: Rule::LinkTarget,
        snapshot: || with_annotation(with_band("l_1"), link_obj(OTHER, &[ADDED, PAGE]), "no"),
        subject: OTHER,
    },
    // The link has a target and links to nothing: its target is under another property.
    Case {
        rule: Rule::LinkWithoutTarget,
        snapshot: || {
            let mut link = link_obj(ADDED, &[PAGE]);
            link.values.clear();
            with_annotation(minimal(), link, "no")
        },
        subject: ADDED,
    },
    Case {
        rule: Rule::LinkToDroppedPage,
        snapshot: || {
            let link = link_obj(OTHER, &[ADDED, PAGE]);
            let snapshot = with_annotation(with_page("[missing]"), link, "yes");
            with_curation(snapshot, OTHER, "name", "identischer Holzschnitt")
        },
        subject: OTHER,
    },
    Case {
        rule: Rule::LinkWithoutName,
        snapshot: || with_annotation(minimal(), link_obj(ADDED, &[PAGE]), "yes"),
        subject: ADDED,
    },
    // A link's name is curated like every value: no address in it.
    Case {
        rule: Rule::HttpInValue,
        snapshot: || {
            let snapshot = with_annotation(minimal(), link_obj(ADDED, &[PAGE]), "no");
            with_curation(snapshot, ADDED, "name", "http://example.org/name")
        },
        subject: ADDED,
    },
    // A curated slug that is empty is refused, also where the label could make the id.
    Case {
        rule: Rule::EmptySlug,
        snapshot: || with_curation(with_band("l_1"), ADDED, "slug", " \u{1f}"),
        subject: ADDED,
    },
    // A label of whitespace alone makes no id; the Band is refused for its id, not for a value.
    Case {
        rule: Rule::EmptyIdSlug,
        snapshot: || with_band(" \u{1f}"),
        subject: ADDED,
    },
    // A calibration scan's strip link is content whatever it points at: the scan is refused for
    // the link alone, and not also for a target that is no Band.
    Case {
        rule: Rule::CalibrationScanContent,
        snapshot: || with_link(with_page("[missing]"), ADDED, HAS_BAND_TYPE_L, BOOK),
        subject: ADDED,
    },
    // A Book has one sort key, whatever the languages of two.
    Case {
        rule: Rule::CuratedCount,
        snapshot: || {
            let snapshot = with_curation_in(minimal(), BOOK, "sort_key", "de", "Bereitung");
            with_curation_in(snapshot, BOOK, "sort_key", "en", "Preparation")
        },
        subject: BOOK,
    },
    // A Band's slug, not a Book's: a Book's slug is also the start of every Page's id.
    Case {
        rule: Rule::IdNotUrlSafe,
        snapshot: || with_curation(with_band("l_1"), ADDED, "slug", "A/b c"),
        subject: ADDED,
    },
    // An Image is named after its file's stem.
    Case {
        rule: Rule::IdNotUrlSafe,
        snapshot: || with_resource(minimal(), band(ADDED, "l_1", "Scan Of The Strip.jp2")),
        subject: "Scan Of The Strip",
    },
];

/// (rule, subject) of every error `build` returns for `snapshot`, in the order returned.
fn refused(snapshot: &Fixture) -> Vec<(Rule, String)> {
    match build(snapshot) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .as_slice()
            .iter()
            .map(|error| (error.rule, error.subject.clone()))
            .collect(),
    }
}

/// [`refused`] for the case's snapshot.
fn refusals(case: &Case) -> Vec<(Rule, String)> {
    refused(&(case.snapshot)())
}

/// The control of every case: unchanged, the snapshot is accepted, and so is each addition the
/// cases build on.
#[test]
fn test_build_minimal_snapshot_and_the_cases_additions_are_accepted() {
    assert!(build(&minimal()).is_ok());
    assert!(build(&with_page("a1v")).is_ok());
    assert!(build(&with_page("[missing]")).is_ok());
    assert!(build(&with_resource(minimal(), band(ADDED, "l_1", STRIP_ASSET))).is_ok());
    let second_book = with_book(with_page("a1v"), "zweites");
    let second_book = changed(second_book, ADDED, |page| {
        page.part_of = vec![ResourceIri(OTHER.to_string())];
    });
    assert!(build(&with_curation(second_book, OTHER, "cover_page", ADDED)).is_ok());
    assert!(build(&with_cologne_book(Some(DEAD_COLOGNE_RECORD))).is_ok());
    assert!(build(&with_strip("l_1", HAS_BAND_TYPE_L)).is_ok());
    assert!(build(&with_curation(with_band("l_1"), ADDED, "name", "Randleiste 1")).is_ok());
    assert!(build(&with_unkept_region(|_| {})).is_ok());
    assert!(build(&with_annotation(minimal(), region(ADDED, PAGE), "yes")).is_ok());
    assert!(build(&with_annotation(minimal(), link_obj(ADDED, &[PAGE]), "no")).is_ok());
    // The calibration scan is no defect of the Region or the link the editors do not keep.
    let scan = with_page("[missing]");
    assert!(build(&with_annotation(scan.clone(), region(OTHER, ADDED), "no")).is_ok());
    assert!(build(&with_annotation(scan, link_obj(OTHER, &[ADDED]), "no")).is_ok());
}

#[test]
fn test_build_each_case_is_refused_by_its_rule_alone_naming_its_subject() {
    for (index, case) in CASES.iter().enumerate() {
        assert_eq!(refusals(case), [(case.rule, case.subject.to_string())], "case {index}");
    }
}

/// The misspelt key is found before the Page's pagenum is read, and its rule stands after the
/// other in [`Rule::ALL`]: the two come back in the rules' order, not in the order found.
#[test]
fn test_build_two_defects_are_both_refused_in_the_rules_order() {
    let snapshot = with_list_node(minimal(), PAGE, HAS_PAGENUM);
    let snapshot = with_curation(snapshot, BOOK, "sulg", "bereitung");

    assert_eq!(
        refused(&snapshot),
        [
            (Rule::UnexpectedValueKind, PAGE.to_string()),
            (Rule::UnknownCuratedKey, BOOK.to_string())
        ]
    );
}

/// The port leaves the order of the curated values open. Of two values under one key that are
/// each refused, the one reported is the first by language, whichever the snapshot lists first.
/// The German text is the later of the two as a text, so the language decides here.
#[test]
fn test_build_refusal_does_not_follow_the_order_of_the_curated_values() {
    let address = |snapshot| with_curation_in(snapshot, BOOK, "teaser", "de", "http://a.example");
    let sentinel = |snapshot| with_curation_in(snapshot, BOOK, "teaser", "en", "[missing]");

    let address_first = refused(&sentinel(address(minimal())));
    let sentinel_first = refused(&address(sentinel(minimal())));

    assert_eq!(address_first, [(Rule::HttpInValue, BOOK.to_string())]);
    assert_eq!(sentinel_first, [(Rule::HttpInValue, BOOK.to_string())]);
}

/// `annotation` at the IRI `iri` and with the ARK it had: an ARK is checked against its own
/// shape, an IRI's last part against none.
fn at_iri(mut annotation: Resource, iri: &str) -> Resource {
    annotation.iri = ResourceIri(iri.to_string());
    annotation
}

/// The id of a Region or a LinkObj is the last part of its IRI.
#[test]
fn test_build_annotation_and_link_ids_that_are_not_url_safe_are_refused() {
    let region = at_iri(region(ADDED, PAGE), "http://rdfh.ch/0803/regions/a%20b");
    let snapshot = with_annotation(minimal(), region, "yes");
    assert_eq!(refused(&snapshot), [(Rule::IdNotUrlSafe, "a%20b".to_string())]);

    let link_iri = "http://rdfh.ch/0803/links/a%2Fb";
    let link = at_iri(link_obj(ADDED, &[PAGE]), link_iri);
    let snapshot = with_annotation(minimal(), link, "yes");
    let snapshot = with_curation(snapshot, link_iri, "name", "ein Name");
    assert_eq!(refused(&snapshot), [(Rule::IdNotUrlSafe, "a%2Fb".to_string())]);
}

/// An annotation the editors do not keep has no row, and so no id to check.
#[test]
fn test_build_annotation_that_the_editors_do_not_keep_has_no_id_to_check() {
    let region = at_iri(region(ADDED, PAGE), "http://rdfh.ch/0803/regions/a%20b");
    assert!(build(&with_annotation(minimal(), region, "no")).is_ok());
}

#[test]
fn test_cases_produce_every_rule() {
    let produced: BTreeSet<Rule> = CASES
        .iter()
        .flat_map(refusals)
        .map(|(rule, _)| rule)
        .chain(crate::hookless::produced_rules())
        .chain(crate::hooks::produced_rules())
        .collect();
    let all: BTreeSet<Rule> = Rule::ALL.iter().copied().collect();

    assert_eq!(produced, all);
}

#[test]
fn test_build_calibration_scans_get_no_row_no_image_and_no_number() {
    let third = page(OTHER, BOOK, 3, "[missing]", "ScanOfTheThirdPage.jp2");
    // U+001F is whitespace to Python and not to `str::trim`.
    let snapshot = with_resource(with_page("\u{1f}[missing] "), third);

    let store = build(&snapshot).expect("two calibration scans are no defect");

    let ids: Vec<&str> = store.resources.iter().map(|resource| resource.id.as_str()).collect();
    assert_eq!(ids, ["bereitung", "ScanOfThePage", "bereitung-a1r"]);
    assert_eq!(store.representations.len(), 1);
    assert_eq!(store.compound_parts.len(), 1);
}
