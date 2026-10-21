//! The census of the store built from the committed 0803 data: what a reader of the site can
//! count. The numbers are the data's own; a change means the committed data changed.
//!
//! And the build's determinism over that data: the same file from a second build, and from the
//! snapshot in another order. And the properties the archive has and the project's block does
//! not map, against the ones it maps, each of which the build reads.

use std::collections::BTreeSet;

use cpe_ports::{File, Motivation, Resource, ValueKind};
use cpe_store_build::{is_url_safe, sql};

use crate::support::{Fixture, BOOK_CLASS, HAS_COMMENT, HAS_DESCRIPTION, HAS_PAGENUM, IS_REGION_OF, PAGE_CLASS};
use crate::{build, COMMITTED, CURATION, HEADER, MAPPING, SNAPSHOT, SQL, STORE};

/// The store's classes, in the order the counts below are written.
const CLASSES: [&str; 4] = ["Book", "Page", "BorderStrip", "Image"];

fn ids_of(class: &str) -> Vec<&'static str> {
    STORE
        .resources
        .iter()
        .filter(|resource| resource.class == class)
        .map(|resource| resource.id.as_str())
        .collect()
}

/// How many of `parts` (Book id, Page id) have an id that is another Page id of the same Book
/// followed by `-2`, `-3`, …: the Pages whose pagenum occurs a second or later time in their Book.
fn suffixed(parts: &[(String, String)]) -> usize {
    let all: BTreeSet<(&str, &str)> = parts.iter().map(|(book, page)| (book.as_str(), page.as_str())).collect();
    all.iter()
        .filter(|(book, page)| {
            page.rsplit_once('-').is_some_and(|(first, number)| {
                number.parse::<u32>().is_ok_and(|number| number >= 2)
                    && !number.starts_with('0')
                    && all.contains(&(*book, first))
            })
        })
        .count()
}

/// [`suffixed`] of the store file the Python pipeline wrote, held as a number: counted from the
/// builder's own output, it would test the builder against itself.
const SUFFIXED_PAGE_IDS: usize = 408;

#[test]
fn test_store_resources_are_the_census_by_class() {
    let counts = CLASSES.map(|class| ids_of(class).len());

    assert_eq!(counts, [19, 4022, 38, 4060]);
    assert_eq!(STORE.resources.len(), 19 + 4022 + 38 + 4060);
}

#[test]
fn test_store_every_book_page_and_strip_has_an_ark_and_no_image_has() {
    let with_ark = CLASSES.map(|class| {
        STORE
            .resources
            .iter()
            .filter(|resource| resource.class == class && resource.ark.is_some())
            .count()
    });

    assert_eq!(with_ark, [19, 4022, 38, 0]);
}

fn served_pages() -> Vec<&'static Resource> {
    SNAPSHOT
        .resources
        .iter()
        .filter(|resource| resource.class.as_str() == PAGE_CLASS)
        .collect()
}

/// The served Pages whose pagenum is the sentinel: the calibration scans, which get no row.
fn calibration_scans() -> Vec<&'static Resource> {
    let is_scan = |page: &&Resource| {
        page.values.iter().any(|value| {
            value.property.as_str() == HAS_PAGENUM
                && matches!(&value.kind, ValueKind::Text { text, .. } if text == "[missing]")
        })
    };
    served_pages().into_iter().filter(is_scan).collect()
}

#[test]
fn test_store_drops_the_two_calibration_scans() {
    let calibration_scans = calibration_scans().len();

    assert_eq!(calibration_scans, 2);
    assert_eq!(ids_of("Page").len(), served_pages().len() - calibration_scans);
}

/// How many of the served annotations have one of `motivations`.
fn served_annotations(motivations: &[Motivation]) -> usize {
    SNAPSHOT
        .resources
        .iter()
        .filter_map(|resource| resource.annotation.as_ref())
        .filter(|annotation| motivations.contains(&annotation.motivation))
        .count()
}

#[test]
fn test_store_annotations_are_the_census_of_served_and_kept() {
    let regions = served_annotations(&[Motivation::Commenting, Motivation::Highlighting]);
    let link_objs = served_annotations(&[Motivation::Linking]);
    let kept = (
        STORE.annotations.len(),
        STORE.annotation_links.len(),
        STORE.annotation_link_targets.len(),
    );
    let shapes = ["polygon", "rectangle", "circle"].map(|shape| {
        let of_shape = STORE.annotations.iter().filter(|row| row.shape == shape);
        of_shape.count()
    });

    assert_eq!((regions, link_objs), (77, 40));
    assert_eq!(kept, (64, 35, 69));
    assert_eq!(shapes, [18, 45, 1]);
}

#[test]
fn test_store_no_ark_is_stored_twice_across_resources_and_annotations() {
    let of_resources = STORE.resources.iter().filter_map(|row| row.ark.as_deref());
    let of_annotations = STORE.annotations.iter().filter_map(|row| row.ark.as_deref());
    let arks: Vec<&str> = of_resources.chain(of_annotations).collect();

    let distinct: BTreeSet<&str> = arks.iter().copied().collect();
    assert_eq!(arks.len(), 4079 + 64);
    assert_eq!(distinct.len(), arks.len());
}

/// Every annotation is on a Page the store has, and every link points at a Book or a Page it has.
#[test]
fn test_store_no_kept_annotation_sits_on_a_dropped_page() {
    let scans: BTreeSet<&str> = calibration_scans().into_iter().map(|page| page.iri.as_str()).collect();
    let on_a_scan = |region: &&Resource| {
        region.values.iter().any(|value| {
            value.property.as_str() == IS_REGION_OF
                && matches!(&value.kind, ValueKind::Link(page) if scans.contains(page.as_str()))
        })
    };
    let regions_on_scans = SNAPSHOT.resources.iter().filter(on_a_scan).count();
    let pages: BTreeSet<&str> = ids_of("Page").into_iter().collect();
    let books_and_pages: BTreeSet<&str> = ids_of("Book").into_iter().chain(ids_of("Page")).collect();

    // The archive has no Region on a scan, kept or not: the refusal of a kept one has its case
    // among the refusals, over a hand-built snapshot.
    assert_eq!(regions_on_scans, 0);
    assert_eq!(scans.len(), 2);
    for annotation in &STORE.annotations {
        assert!(pages.contains(annotation.resource_id.as_str()), "{annotation:?}");
    }
    for target in &STORE.annotation_link_targets {
        if let Some(resource) = &target.resource_id {
            assert!(books_and_pages.contains(resource.as_str()), "{target:?}");
        }
    }
}

#[test]
fn test_store_every_resource_id_is_unique() {
    let ids: BTreeSet<&str> = STORE.resources.iter().map(|resource| resource.id.as_str()).collect();

    assert_eq!(ids.len(), STORE.resources.len());
}

#[test]
fn test_store_every_resource_annotation_and_link_id_is_url_safe() {
    let resources = STORE.resources.iter().map(|row| row.id.as_str());
    let annotations = STORE.annotations.iter().map(|row| row.id.as_str());
    let links = STORE.annotation_links.iter().map(|row| row.id.as_str());
    let ids: Vec<&str> = resources.chain(annotations).chain(links).collect();

    assert!(ids.len() > 8000);
    assert_eq!(
        ids.iter().filter(|id| !is_url_safe(id)).collect::<Vec<_>>(),
        Vec::<&&str>::new()
    );
}

/// A Book's cover is the Image of one of its Pages, so the 19 covers add no Image: there is one
/// Image per Page and strip, with one representation, and one part per Page.
#[test]
fn test_store_each_cover_is_the_image_of_a_page() {
    let images: BTreeSet<&str> = ids_of("Image").into_iter().collect();
    let covers: BTreeSet<&str> = CURATION
        .as_slice()
        .iter()
        .filter(|value| value.key == "cover_page")
        .map(|value| {
            let page = SNAPSHOT
                .resources
                .iter()
                .find(|resource| resource.iri.as_str() == value.text)
                .expect("a cover is a resource of the snapshot");
            match &page.file {
                Some(File::StillImage { asset, .. }) => asset.strip_suffix(".jp2").expect("a scan is a .jp2 file"),
                other => panic!("a cover has a still image, not {other:?}"),
            }
        })
        .collect();

    assert_eq!(covers.len(), 19);
    assert!(covers.is_subset(&images));
    assert_eq!(images.len(), ids_of("Page").len() + ids_of("BorderStrip").len());
    assert_eq!(STORE.representations.len(), images.len());
    assert_eq!(STORE.compound_parts.len(), ids_of("Page").len());
}

/// The three Books whose catalogue override the editors curated are written with the address
/// that replaces the dead one.
#[test]
fn test_store_every_overridden_catalogue_address_is_written() {
    let overridden: Vec<&str> = STORE
        .properties
        .iter()
        .filter(|row| {
            let uri = row.value_text.as_deref().unwrap_or_default();
            row.name == "catalogue-uri" && uri.starts_with("https://search.ub.uni-koeln.de/")
        })
        .map(|row| row.resource_id.as_str())
        .collect();

    assert_eq!(overridden, ["lob-der-glieder", "orationes", "zeitgloecklein-1492"]);
}

/// The file's header names the address the project's block declares as `iiif-base`, and every
/// scan is served from it: one changed without the other fails here.
#[test]
fn test_store_file_header_names_the_address_every_scan_is_served_from() {
    let base = MAPPING
        .images
        .as_ref()
        .map(|images| images.iiif_base.as_str())
        .expect("the project's block has an `images` node");
    let line = SQL
        .lines()
        .find(|line| line.starts_with("-- IIIF:"))
        .expect("the header has its IIIF line");

    assert_eq!(line, format!("-- IIIF: {base}"));
    assert!(base.ends_with('/'), "{base}");
    assert!(!STORE.representations.is_empty());
    for representation in &STORE.representations {
        assert!(
            representation.uri.starts_with(base),
            "the store file's header names {base} and the `iiif-base` of the project's mapping \
             gives {}: they must be one address",
            representation.uri
        );
    }
}

/// The Books that have a value under `name`, each once per value.
fn books_with(name: &str) -> Vec<&'static str> {
    let books = ids_of("Book");
    STORE
        .properties
        .iter()
        .filter(|row| row.name == name && books.contains(&row.resource_id.as_str()))
        .map(|row| row.resource_id.as_str())
        .collect()
}

#[test]
fn test_store_every_book_has_one_of_each_value_every_book_has() {
    for name in ["sort-key", "office", "pub-date", "name", "extent", "image"] {
        assert_eq!(books_with(name), ids_of("Book"), "{name}");
    }
}

#[test]
fn test_store_four_books_have_a_teaser_in_both_languages() {
    let languages: Vec<(&str, Option<&str>)> = STORE
        .properties
        .iter()
        .filter(|row| row.name == "teaser")
        .map(|row| (row.resource_id.as_str(), row.value_lang.as_deref()))
        .collect();

    assert_eq!(
        languages,
        [
            ("bereitung", Some("de")),
            ("bereitung", Some("en")),
            ("narrenschiff-dt", Some("de")),
            ("narrenschiff-dt", Some("en")),
            ("postilla", Some("de")),
            ("postilla", Some("en")),
            ("zeitgloecklein-1492", Some("de")),
            ("zeitgloecklein-1492", Some("en")),
        ]
    );
}

/// No date is by the month, and none of the Gregorian calendar is by the day.
#[test]
fn test_store_publication_dates_are_the_census_by_precision_and_calendar() {
    let dates: Vec<_> = STORE.properties.iter().filter(|row| row.name == "pub-date").collect();
    let count = |precision: &str, calendar: &str, length: usize| {
        dates
            .iter()
            .filter(|row| {
                let bounds = [&row.date_start, &row.date_end];
                row.date_precision.as_deref() == Some(precision)
                    && row.date_calendar.as_deref() == Some(calendar)
                    && bounds
                        .iter()
                        .all(|bound| bound.as_ref().is_some_and(|text| text.len() == length))
            })
            .count()
    };

    assert_eq!(dates.len(), ids_of("Book").len());
    assert_eq!(count("year", "julian", 4), 10);
    assert_eq!(count("year", "gregorian", 4), 2);
    assert_eq!(count("day", "julian", 10), 7);
    assert_eq!(dates.len(), 10 + 2 + 7, "a date of another precision or calendar");
    assert!(dates
        .iter()
        .all(|row| row.date_display.is_some() && row.value_lang.as_deref() == Some("de")));
}

#[test]
fn test_store_no_text_is_empty_or_the_sentinel() {
    let texts: Vec<&str> = STORE
        .properties
        .iter()
        .filter_map(|row| row.value_text.as_deref().or(row.date_display.as_deref()))
        .collect();

    assert!(
        texts.len() >= STORE.resources.len(),
        "the measure finds less than a text per resource"
    );
    for text in texts {
        // The whitespace the builder trims by.
        let blank = cpe_core::remodel::is_strip_space;
        assert!(!text.chars().all(blank), "an empty text");
        assert!(!text.contains("[missing]"), "{text}");
    }
}

/// One Book's place is the sentinel in the archive, and the store has no place for it.
#[test]
fn test_store_book_whose_place_is_unknown_has_no_place() {
    let unknown = SNAPSHOT
        .resources
        .iter()
        .flat_map(|resource| &resource.values)
        .filter(|value| {
            value.property.as_str().ends_with("#hasPubloc")
                && matches!(&value.kind, cpe_ports::ValueKind::Text { text, .. } if text == "[missing]")
        })
        .count();

    assert_eq!(unknown, 1);
    assert_eq!(books_with("pub-place").len(), ids_of("Book").len() - unknown);
}

/// (resource id, target) of every value under `name`.
fn targets(name: &str) -> Vec<(&'static str, &'static str)> {
    STORE
        .properties
        .iter()
        .filter(|row| row.name == name)
        .map(|row| {
            let target = row.target_id.as_deref().expect("the value has a target");
            (row.resource_id.as_str(), target)
        })
        .collect()
}

#[test]
fn test_store_strip_links_are_the_census_by_side() {
    let strips = ids_of("BorderStrip");
    let pages = ids_of("Page");
    let [left, right] = ["strip-left", "strip-right"].map(targets);

    assert_eq!([left.len(), right.len()], [207, 208]);
    for (page, strip) in left.iter().chain(&right) {
        assert!(pages.contains(page), "{page}");
        assert!(strips.contains(strip), "{strip}");
    }
    let uses: i64 = STORE
        .properties
        .iter()
        .filter(|row| row.name == "page-uses")
        .filter_map(|row| row.value_int)
        .sum();
    assert_eq!(uses, 207 + 208);
}

#[test]
fn test_store_strips_are_the_census_by_side() {
    let sides = targets("side");
    let on = |node: &str| sides.iter().filter(|(_, side)| *side == node).count();
    let sided: BTreeSet<&str> = sides.iter().map(|(strip, _)| *strip).collect();
    let unsided: Vec<&str> = ids_of("BorderStrip")
        .into_iter()
        .filter(|strip| !sided.contains(strip))
        .collect();

    assert_eq!([on("side-left"), on("side-right")], [18, 19]);
    assert_eq!(sides.len(), 18 + 19, "a side of another node");
    assert_eq!(unsided, ["strip-38"]);
}

#[test]
fn test_store_suffixed_page_ids_are_the_census() {
    let built: Vec<(String, String)> = STORE
        .compound_parts
        .iter()
        .map(|part| (part.parent_id.clone(), part.child_id.clone()))
        .collect();

    assert_eq!(suffixed(&built), SUFFIXED_PAGE_IDS);
}

#[test]
fn test_suffixed_counts_a_repeat_and_not_a_number_that_belongs_to_the_pagenum() {
    let part = |page: &str| ("b".to_string(), page.to_string());

    let count = suffixed(&[
        part("b-a1r"),
        part("b-a1r-2"),
        part("b-a1r-3"),
        part("b-12"),
        part("b-a2r-1"),
        part("b-a3r-02"),
        part("b-a3r"),
    ]);

    assert_eq!(count, 2);
}

/// The properties the archive has values under, on a class the project's block declares, that
/// the block declares no `property` node for, each (class, property).
///
/// The list is closed on purpose. A property that is not here and not mapped would be dropped
/// without a word, and the store file would not change.
const IGNORED: [(&str, &str); 6] = [
    (BOOK_CLASS, "http://www.knora.org/ontology/0803/incunabula#hasCitation"),
    (BOOK_CLASS, HAS_COMMENT),
    (BOOK_CLASS, HAS_DESCRIPTION),
    (BOOK_CLASS, "http://www.knora.org/ontology/0803/incunabula#hasNote"),
    (PAGE_CLASS, "http://www.knora.org/ontology/0803/incunabula#hasCitation"),
    (PAGE_CLASS, "http://www.knora.org/ontology/0803/incunabula#hasOrigname"),
];

/// The committed data without the values that resources of `class` have under `property`.
fn without(class: &str, property: &str) -> Fixture {
    let mut snapshot = COMMITTED.clone();
    for resource in &mut snapshot.facts.resources {
        if resource.class.as_str() == class {
            resource.values.retain(|value| value.property.as_str() != property);
        }
    }
    snapshot
}

/// (class, property) of every value the snapshot serves on a resource of a class the project's
/// block declares.
fn served_on_declared_classes() -> BTreeSet<(&'static str, &'static str)> {
    let declared: BTreeSet<&str> = MAPPING.classes.iter().map(|class| class.iri.as_str()).collect();
    SNAPSHOT
        .resources
        .iter()
        .filter(|resource| declared.contains(resource.class.as_str()))
        .flat_map(|resource| {
            let class = resource.class.as_str();
            let properties = resource.values.iter().map(|value| value.property.as_str());
            properties.map(move |property| (class, property))
        })
        .collect()
}

/// (class, property) of every `property` node of the project's block that reads the archive.
fn mapped_pairs() -> BTreeSet<(&'static str, &'static str)> {
    let mut mapped = BTreeSet::new();
    for class in &MAPPING.classes {
        for node in &class.properties {
            if let Some(property) = node.source.iri() {
                mapped.insert((class.iri.as_str(), property.as_str()));
            }
        }
    }
    mapped
}

#[test]
fn test_snapshot_unmapped_properties_are_the_listed_six() {
    let served = served_on_declared_classes();
    let mapped = mapped_pairs();

    let unmapped: Vec<(&str, &str)> = served.difference(&mapped).copied().collect();

    assert!(unmapped.len() < served.len(), "the measure finds no property that is mapped");
    assert_eq!(
        unmapped, IGNORED,
        "the properties the archive serves and the block does not map are not the listed ones. \
         For each difference decide: map the property in the project's remodel block, or list \
         it in IGNORED here and in the sentence about ignored properties in \
         store-build/README.md; a property that is mapped now leaves both"
    );
}

/// A `property` node the build did not read would leave the store as it is when the archive's
/// values under it are gone. Only a pair the snapshot serves can show it.
#[test]
fn test_build_reads_every_declared_property() {
    let served = served_on_declared_classes();
    let declared: Vec<(&str, &str)> = mapped_pairs().intersection(&served).copied().collect();
    assert!(declared.len() > 10, "{declared:?}");
    assert!(
        build(&COMMITTED).is_ok_and(|store| store == *STORE),
        "the data the cases change builds the store"
    );

    let unread: Vec<(&str, &str)> = declared
        .into_iter()
        .filter(|(class, property)| build(&without(class, property)).is_ok_and(|store| store == *STORE))
        .collect();

    assert!(unread.is_empty(), "{unread:?}");
}

/// The block declares no property of an annotation: the builder reads those itself. Every
/// property the snapshot serves on a class the block does not declare is one of them, and
/// without it the build is another store or is refused. One that is served and not read fails
/// here: it would be dropped without a word.
#[test]
fn test_build_reads_every_property_of_an_annotation() {
    let declared = served_on_declared_classes();
    let on_annotations: BTreeSet<(&str, &str)> = SNAPSHOT
        .resources
        .iter()
        .flat_map(|resource| {
            let class = resource.class.as_str();
            let properties = resource.values.iter().map(|value| value.property.as_str());
            properties.map(move |property| (class, property))
        })
        .filter(|pair| !declared.contains(pair))
        .collect();
    assert!(on_annotations.len() > 3, "{on_annotations:?}");
    assert!(
        build(&COMMITTED).is_ok_and(|store| store == *STORE),
        "the data the cases change builds the store"
    );

    let unread: Vec<(&str, &str)> = on_annotations
        .into_iter()
        .filter(|(class, property)| build(&without(class, property)).is_ok_and(|store| store == *STORE))
        .collect();

    assert!(unread.is_empty(), "{unread:?}");
}

/// The store file `build` makes of `snapshot`.
fn file_of(snapshot: &Fixture) -> String {
    let store = build(snapshot).expect("the snapshot builds");
    sql::to_sql(&store, &MAPPING, &HEADER).expect("every table of the built store has a row")
}

/// Fails with the place of the first byte in which `file` differs from [`SQL`], and the two
/// lengths: the files are too long to print.
fn assert_is_the_store_file(file: &str) {
    let differs_at = file.bytes().zip(SQL.bytes()).position(|(a, b)| a != b);

    assert_eq!((differs_at, file.len()), (None, SQL.len()));
}

#[test]
fn test_build_second_build_writes_the_same_file_byte_for_byte() {
    assert_is_the_store_file(&file_of(&COMMITTED));
}

/// Every order the port leaves open that this data can show is another one: the snapshot's
/// resources, list nodes and curated values, and a resource's properties among each other. The
/// values of one property keep their order, which is the archive's. The order of a resource's
/// parents is open too, and no resource here has two.
#[test]
fn test_build_reordered_snapshot_writes_the_same_file_byte_for_byte() {
    fn reorder<T>(items: &mut [T]) {
        items.reverse();
        items.rotate_left(items.len() / 3);
    }
    let mut snapshot = COMMITTED.clone();
    reorder(&mut snapshot.facts.resources);
    reorder(&mut snapshot.facts.list_nodes);
    reorder(&mut snapshot.curation);
    for resource in &mut snapshot.facts.resources {
        assert!(resource.part_of.len() < 2, "parents to reorder as well");
        // Stable: the properties change places, the values of each do not.
        resource.values.sort_by(|a, b| b.property.cmp(&a.property));
    }
    assert_ne!(snapshot.facts.resources[0], SNAPSHOT.resources[0]);
    assert_ne!(snapshot.facts.list_nodes[0], SNAPSHOT.list_nodes[0]);
    assert_ne!(snapshot.curation[0], COMMITTED.curation[0]);
    let by_iri = |resources: &[Resource]| {
        let mut sorted = resources.to_vec();
        sorted.sort_by(|a, b| a.iri.cmp(&b.iri));
        sorted
    };
    let moved_within = by_iri(&snapshot.facts.resources)
        .iter()
        .zip(&by_iri(&SNAPSHOT.resources))
        .filter(|(reordered, served)| reordered.values != served.values)
        .count();
    assert!(moved_within > 0, "no resource's properties changed places");

    assert_is_the_store_file(&file_of(&snapshot));
}
