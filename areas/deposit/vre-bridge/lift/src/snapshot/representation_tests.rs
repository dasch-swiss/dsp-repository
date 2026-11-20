//! Fixture tests for representations (§8).

use oxigraph::model::Literal;

use super::link_tests::line;
use super::tests::{Fixture, GRAPH, boolean, integer, plain, report, run};
use super::violations::Rule;
use super::{representation, vocab};
use crate::model::{self, KB, kb, nn};

const PAGE: &str = "http://rdfh.ch/0803/PAGE";
const BOOK: &str = "http://rdfh.ch/0803/BOOK";

fn node(uuid: &str) -> String {
    format!("{}{uuid}", vocab::REPRESENTATION_PREFIX)
}

/// A live file value `id` of knora-base class `class` with UUID `uuid`, attached
/// to `resource` by `kb:has<class>`, carrying 0803's omitted predicates too.
fn file_value(f: &mut Fixture, resource: &str, class: &str, id: &str, uuid: &str) -> String {
    let file = format!("{resource}/values/{id}");
    f.add(resource, &format!("{KB}has{class}"), nn(&file));
    f.add(&file, model::RDF_TYPE, nn(&format!("{KB}{class}")));
    f.add(&file, kb::VALUE_HAS_UUID, plain(uuid));
    f.add(&file, kb::INTERNAL_FILENAME, plain(&format!("{uuid}.bin")));
    f.add(&file, kb::IS_DELETED, boolean("false"));
    f.add(&file, kb::INTERNAL_MIME_TYPE, plain("application/octet-stream"));
    f.add(&file, kb::ORIGINAL_FILENAME, plain("scan.tif"));
    f.add(&file, kb::ORIGINAL_MIME_TYPE, plain("image/tiff"));
    f.add(&file, kb::VALUE_HAS_STRING, plain("scan.tif"));
    f.add(&file, kb::VALUE_HAS_ORDER, integer("0"));
    file
}

/// A still-image file value with dimensions `x` × `y`.
fn still_image(f: &mut Fixture, resource: &str, id: &str, uuid: &str, (x, y): (&str, &str)) -> String {
    let file = file_value(f, resource, "StillImageFileValue", id, uuid);
    f.add(&file, kb::DIM_X, integer(x));
    f.add(&file, kb::DIM_Y, integer(y));
    file
}

fn page() -> Fixture {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "b7v");
    f
}

#[test]
fn a_still_image_beside_an_archive_file_value_is_two_live_file_values() {
    // FORMAT.md §8 counts every live file value, the ones omitted whole included.
    let mut f = page();
    still_image(&mut f, PAGE, "ONE", "ONE", ("1", "1"));
    file_value(&mut f, PAGE, "ArchiveFileValue", "ZIP", "ZIP");

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MultipleRepresentations, PAGE.to_string(), String::new())]
    );
}
fn typed_line(s: &str, p: &str, literal: &Literal) -> String {
    format!("<{s}> <{p}> {literal} {GRAPH} .")
}

#[test]
fn a_still_image_emits_its_representation_node_with_type_filename_and_dimensions() {
    let mut f = page();
    still_image(&mut f, PAGE, "FILE", "sdSIAoxiS4", ("3297", "5141"));

    let n = node("sdSIAoxiS4");
    let (out, stats) = run(&f);
    for expected in [
        line(PAGE, vocab::DAO_HAS_REPRESENTATION, &n),
        line(&n, model::RDF_TYPE, vocab::DAO_REPRESENTATION),
        line(&n, vocab::DAO_REPRESENTATION_TYPE, vocab::DAO_STILL_IMAGE_REPRESENTATION),
        typed_line(&n, vocab::DAO_INTERNAL_FILENAME, &plain("sdSIAoxiS4.bin")),
        typed_line(&n, vocab::DAO_DIM_X, &integer("3297")),
        typed_line(&n, vocab::DAO_DIM_Y, &integer("5141")),
    ] {
        assert!(out.contains(&expected), "missing {expected}\n{out}");
    }
    assert_eq!(out.matches(&format!("<{n}>")).count(), 6, "{out}");
    assert!(!out.contains("hasStillImageFileValue"), "{out}");
    assert_eq!(
        stats.representations_by_type.into_iter().collect::<Vec<_>>(),
        vec![(vocab::DAO_STILL_IMAGE_REPRESENTATION, 1)]
    );
}

#[test]
fn audio_moving_image_and_document_file_values_emit_their_types_and_no_dimensions() {
    let mut f = Fixture::new();
    let cases = [
        ("AudioFileValue", vocab::DAO_AUDIO_REPRESENTATION),
        ("MovingImageFileValue", vocab::DAO_MOVING_IMAGE_REPRESENTATION),
        ("DocumentFileValue", vocab::DAO_DOCUMENT_REPRESENTATION),
    ];
    for (class, _) in cases {
        let resource = format!("http://rdfh.ch/0803/{class}");
        f.resource(&resource, "Page", class);
        file_value(&mut f, &resource, class, "FILE", class);
    }

    let (out, _) = run(&f);
    for (class, representation_type) in cases {
        let n = node(class);
        assert!(
            out.contains(&line(&n, vocab::DAO_REPRESENTATION_TYPE, representation_type)),
            "{out}"
        );
        let filename = typed_line(&n, vocab::DAO_INTERNAL_FILENAME, &plain(&format!("{class}.bin")));
        assert!(out.contains(&filename), "{out}");
    }
    assert!(!out.contains(vocab::DAO_DIM_X), "{out}");
    assert!(!out.contains(vocab::DAO_DIM_Y), "{out}");
}

#[test]
fn an_archive_file_value_emits_no_representation_and_the_run_succeeds() {
    let mut f = page();
    let file = file_value(&mut f, PAGE, "ArchiveFileValue", "FILE", "ZIP");
    // Omitted whole: its predicates are not checked (§11).
    f.add(&file, "http://example.org/unchecked", plain("x"));

    let (out, stats) = run(&f);
    assert!(!out.contains(vocab::DAO_HAS_REPRESENTATION), "{out}");
    assert_eq!(stats.omitted.get(representation::OMITTED_FILE_VALUE), Some(&1));
}

#[test]
fn a_deleted_and_a_superseded_file_value_emit_no_representation() {
    let mut f = page();
    f.resource(BOOK, "Book", "Narrenschiff");
    let gone = still_image(&mut f, PAGE, "GONE", "GONE", ("1", "1"));
    f.add(&gone, kb::IS_DELETED, boolean("true"));
    let current = still_image(&mut f, BOOK, "CURRENT", "CURRENT", ("2", "2"));
    let old = still_image(&mut f, BOOK, "OLD", "OLD", ("1", "1"));
    f.add(&current, kb::PREVIOUS_VALUE, nn(&old));

    let (out, stats) = run(&f);
    assert!(!out.contains(&node("GONE")), "{out}");
    assert!(!out.contains(&node("OLD")), "{out}");
    assert!(
        out.contains(&line(BOOK, vocab::DAO_HAS_REPRESENTATION, &node("CURRENT"))),
        "{out}"
    );
    assert_eq!(out.matches(vocab::DAO_HAS_REPRESENTATION).count(), 1, "{out}");
    assert_eq!(stats.omitted.get("deleted file value (§8)"), Some(&1));
    assert_eq!(stats.omitted.get("superseded file value (§8)"), Some(&1));
}

#[test]
fn a_resource_with_two_live_file_values_is_a_violation() {
    let mut f = page();
    still_image(&mut f, PAGE, "ONE", "ONE", ("1", "1"));
    file_value(&mut f, PAGE, "DocumentFileValue", "TWO", "TWO");

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MultipleRepresentations, PAGE.to_string(), String::new())]
    );
}

#[test]
fn the_representation_iri_uses_the_uuid_not_the_file_value_iri_suffix() {
    let mut f = page();
    let file = still_image(&mut f, PAGE, "IRISUFFIX", "REALUUID", ("1", "1"));
    assert!(file.ends_with("IRISUFFIX"));

    let (out, _) = run(&f);
    assert!(
        out.contains(&line(PAGE, vocab::DAO_HAS_REPRESENTATION, &node("REALUUID"))),
        "{out}"
    );
    assert!(!out.contains("IRISUFFIX"), "{out}");
}

#[test]
fn a_shared_uuid_a_missing_filename_a_missing_or_bad_dimension_and_an_unknown_predicate_are_violations() {
    let mut f = page();
    f.resource(BOOK, "Book", "Narrenschiff");
    let shared_page = still_image(&mut f, PAGE, "A", "SHARED", ("1", "1"));
    let shared_book = still_image(&mut f, BOOK, "B", "SHARED", ("1", "1"));
    for (i, resource) in ["NOFILE", "NODIM", "BIGDIM", "EXTRA"].iter().enumerate() {
        let iri = format!("http://rdfh.ch/0803/{resource}");
        f.resource(&iri, "Page", resource);
        let uuid = format!("U{i}");
        let file = file_value(&mut f, &iri, "StillImageFileValue", resource, &uuid);
        match *resource {
            "NOFILE" => {
                f.quads.retain(|q| {
                    !(q.subject.to_string() == format!("<{file}>") && q.predicate.as_str() == kb::INTERNAL_FILENAME)
                });
                f.add(&file, kb::DIM_X, integer("1"));
                f.add(&file, kb::DIM_Y, integer("1"));
            }
            "NODIM" => f.add(&file, kb::DIM_Y, integer("1")),
            "BIGDIM" => {
                f.add(&file, kb::DIM_X, integer("4294967296"));
                f.add(&file, kb::DIM_Y, integer("1"));
            }
            _ => {
                f.add(&file, kb::DIM_X, integer("1"));
                f.add(&file, kb::DIM_Y, integer("1"));
                f.add(&file, "http://example.org/fps", integer("25"));
            }
        }
    }
    let file = |resource: &str| format!("http://rdfh.ch/0803/{resource}/values/{resource}");

    let mut expected = vec![
        (Rule::DuplicateUuid, shared_page, node("SHARED")),
        (Rule::DuplicateUuid, shared_book, node("SHARED")),
        (Rule::MissingSourceLiteral, file("NOFILE"), kb::INTERNAL_FILENAME.to_string()),
        (Rule::MissingSourceLiteral, file("NODIM"), kb::DIM_X.to_string()),
        (Rule::InvalidLiteral, file("BIGDIM"), kb::DIM_X.to_string()),
        (
            Rule::UnknownFileValuePredicate,
            file("EXTRA"),
            "http://example.org/fps".to_string(),
        ),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_file_values_authorship_and_copyright_holder_are_emitted_on_its_representation() {
    let mut f = page();
    let file = still_image(&mut f, PAGE, "FILE", "LEGAL", ("1", "1"));
    f.add(&file, kb::HAS_AUTHORSHIP, plain("John Tenniel"));
    f.add(&file, kb::HAS_AUTHORSHIP, plain("Dalziel Brothers"));
    f.add(&file, kb::HAS_COPYRIGHT_HOLDER, plain("Public Domain"));

    // Legal information rides on every representation type, not only still images.
    f.resource(BOOK, "Book", "Narrenschiff");
    let audio = file_value(&mut f, BOOK, "AudioFileValue", "AUDIO", "AUDIO");
    f.add(&audio, kb::HAS_COPYRIGHT_HOLDER, plain("DaSCH"));

    let n = node("LEGAL");
    let (out, _) = run(&f);
    for expected in [
        typed_line(&n, vocab::DAO_AUTHORSHIP, &plain("Dalziel Brothers")),
        typed_line(&n, vocab::DAO_AUTHORSHIP, &plain("John Tenniel")),
        typed_line(&n, vocab::DAO_COPYRIGHT_HOLDER, &plain("Public Domain")),
        typed_line(&node("AUDIO"), vocab::DAO_COPYRIGHT_HOLDER, &plain("DaSCH")),
    ] {
        assert!(out.contains(&expected), "missing {expected}\n{out}");
    }
    assert_eq!(out.matches(vocab::DAO_AUTHORSHIP).count(), 2, "{out}");
    assert_eq!(out.matches(vocab::DAO_COPYRIGHT_HOLDER).count(), 2, "{out}");
}

#[test]
fn a_file_value_without_legal_information_emits_none() {
    let mut f = page();
    still_image(&mut f, PAGE, "FILE", "BARE", ("1", "1"));

    let (out, _) = run(&f);
    assert!(!out.contains(vocab::DAO_AUTHORSHIP), "{out}");
    assert!(!out.contains(vocab::DAO_COPYRIGHT_HOLDER), "{out}");
}

#[test]
fn a_second_copyright_holder_a_non_string_author_or_holder_and_a_licence_are_violations() {
    let mut f = page();
    f.resource(BOOK, "Book", "Narrenschiff");
    let two = still_image(&mut f, PAGE, "TWO", "TWO", ("1", "1"));
    f.add(&two, kb::HAS_COPYRIGHT_HOLDER, plain("A"));
    f.add(&two, kb::HAS_COPYRIGHT_HOLDER, plain("B"));
    let tagged = still_image(&mut f, BOOK, "TAGGED", "TAGGED", ("1", "1"));
    f.add(&tagged, kb::HAS_AUTHORSHIP, plain("Dalziel Brothers"));
    f.add(
        &tagged,
        kb::HAS_AUTHORSHIP,
        Literal::new_language_tagged_literal_unchecked("Tenniel", "en"),
    );
    let more = "http://rdfh.ch/0803/MORE";
    f.resource(more, "Page", "MORE");
    let typed = still_image(&mut f, more, "TYPED", "TYPED", ("1", "1"));
    f.add(&typed, kb::HAS_COPYRIGHT_HOLDER, integer("1"));
    f.add(&typed, kb::HAS_AUTHORSHIP, nn("http://example.org/tenniel"));
    // A per-file licence is not carried (§11): it stays an unknown predicate.
    let license = format!("{KB}hasLicense");
    f.add(&tagged, &license, nn("http://rdfh.ch/licenses/public-domain"));

    let mut expected = vec![
        (Rule::RepeatedSourceLiteral, two, kb::HAS_COPYRIGHT_HOLDER.to_string()),
        (Rule::InvalidLiteral, tagged.clone(), kb::HAS_AUTHORSHIP.to_string()),
        (Rule::UnknownFileValuePredicate, tagged, license),
        (Rule::InvalidLiteral, typed.clone(), kb::HAS_AUTHORSHIP.to_string()),
        (Rule::InvalidLiteral, typed, kb::HAS_COPYRIGHT_HOLDER.to_string()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}
