//! The selection and the public guard, through [`crate::lift`] on synthetic dumps
//! written to a temporary directory.

use oxigraph::model::Literal;

use crate::guard::{Problem, SubjectKind, Violation};
use crate::model::{self, kb, nn};
use crate::snapshot::tests::{APPLE, Fixture, ZEBRA, boolean, onto, plain, rights_lines, write_dump};
use crate::snapshot::vocab;
use crate::test_support::TempDir;
use crate::{LiftError, Lifted, SelectedFile};

/// The anonymous user may view; written in the order DSP-API usually writes it.
const PUBLIC: &str =
    "CR knora-admin:Creator|M knora-admin:ProjectMember|V knora-admin:KnownUser,knora-admin:UnknownUser";
/// The anonymous user gets nothing.
const MEMBERS_ONLY: &str = "CR knora-admin:Creator|V knora-admin:ProjectMember";
const RESTRICTED: &str = "CR knora-admin:Creator|RV knora-admin:UnknownUser";

fn lift(f: &Fixture) -> Result<Lifted, LiftError> {
    let dir = TempDir::new();
    write_dump(f, &dir);
    crate::lift(dir.path())
}

fn output(f: &Fixture) -> String {
    let lifted = lift(f).unwrap_or_else(|e| panic!("the fixture must lift: {e}"));
    let mut bytes = Vec::new();
    lifted.snapshot.write_to(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn refusal(f: &Fixture) -> Vec<Violation> {
    match lift(f) {
        Err(LiftError::Refused { violations }) => violations,
        Err(other) => panic!("not a refusal: {other}"),
        Ok(_) => panic!("the fixture must be refused"),
    }
}

fn violation(subject: &str, kind: SubjectKind, class: &str, problem: Problem) -> Violation {
    Violation {
        subject: subject.to_string(),
        kind,
        class: class.to_string(),
        problem,
    }
}

/// A Page with `permissions` (none for `None`).
fn page(f: &mut Fixture, iri: &str, permissions: Option<&str>) {
    f.resource(iri, "Page", "p");
    if let Some(p) = permissions {
        f.add(iri, kb::HAS_PERMISSIONS, plain(p));
    }
}

/// A text value `<resource>/values/<id>` under `hasDescription`, with UUID `id`.
fn text(f: &mut Fixture, resource: &str, id: &str, permissions: Option<&str>) -> String {
    let v = format!("{resource}/values/{id}");
    f.add(resource, &onto("hasDescription"), nn(&v));
    f.add(&v, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&v, kb::VALUE_HAS_UUID, plain(id));
    f.add(&v, kb::VALUE_HAS_STRING, plain(id));
    if let Some(p) = permissions {
        f.add(&v, kb::HAS_PERMISSIONS, plain(p));
    }
    v
}

/// A file value of `class` on `resource`, with UUID `id`.
fn file(f: &mut Fixture, resource: &str, id: &str, class: &str, permissions: &str) -> String {
    let v = format!("{resource}/values/{id}");
    f.add(resource, kb::HAS_STILL_IMAGE_FILE_VALUE, nn(&v));
    f.add(&v, model::RDF_TYPE, nn(class));
    f.add(&v, kb::VALUE_HAS_UUID, plain(id));
    f.add(&v, kb::INTERNAL_FILENAME, plain(&format!("{id}.jp2")));
    for dim in [kb::DIM_X, kb::DIM_Y] {
        f.add(&v, dim, Literal::new_typed_literal("10", nn(model::XSD_INTEGER)));
    }
    f.add(&v, kb::HAS_PERMISSIONS, plain(permissions));
    v
}

/// Every subject kind the guard checks, all viewable, a still image under
/// restricted view, and a superseded, a deleted and an orphaned value and a
/// deleted resource that grant the anonymous user nothing.
fn rich_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.onto_triple(&onto("isPartOfBook"), model::RDFS_SUBPROPERTYOF, nn(kb::IS_PART_OF));
    f.onto_triple(&onto("hasSeqnum"), model::RDFS_SUBPROPERTYOF, nn(kb::SEQNUM));
    page(&mut f, ZEBRA, Some(PUBLIC));
    f.resource(APPLE, "Book", "Narrenschiff");
    f.add(APPLE, kb::HAS_PERMISSIONS, plain("V knora-admin:UnknownUser"));

    let current = text(&mut f, ZEBRA, "TEXT2", Some(PUBLIC));
    let old = text(&mut f, ZEBRA, "TEXT1", Some(MEMBERS_ONLY));
    f.add(&current, kb::PREVIOUS_VALUE, nn(&old));
    let deleted = text(&mut f, ZEBRA, "GONE", Some(MEMBERS_ONLY));
    f.add(&deleted, kb::IS_DELETED, boolean("true"));
    file(&mut f, ZEBRA, "IMAGE", kb::STILL_IMAGE_FILE_VALUE, RESTRICTED);

    let seqnum = format!("{ZEBRA}/values/SEQ");
    f.add(ZEBRA, &onto("hasSeqnum"), nn(&seqnum));
    f.add(&seqnum, model::RDF_TYPE, nn(kb::INT_VALUE));
    f.add(&seqnum, kb::VALUE_HAS_UUID, plain("SEQ"));
    f.add(
        &seqnum,
        kb::VALUE_HAS_INTEGER,
        Literal::new_typed_literal("7", nn(model::XSD_INTEGER)),
    );
    f.add(&seqnum, kb::HAS_PERMISSIONS, plain(PUBLIC));

    let part = format!("{ZEBRA}/values/PART");
    f.add(ZEBRA, &onto("isPartOfBook"), nn(APPLE));
    f.add(ZEBRA, &onto("isPartOfBookValue"), nn(&part));
    f.add(&part, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&part, model::RDF_SUBJECT, nn(ZEBRA));
    f.add(&part, model::RDF_PREDICATE, nn(&onto("isPartOfBook")));
    f.add(&part, model::RDF_OBJECT, nn(APPLE));
    f.add(&part, kb::HAS_PERMISSIONS, plain(PUBLIC));

    let link = format!("{ZEBRA}/values/LINK");
    f.add(ZEBRA, &onto("hasBandTypeL"), nn(APPLE));
    f.add(ZEBRA, &onto("hasBandTypeLValue"), nn(&link));
    f.add(&link, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&link, model::RDF_SUBJECT, nn(ZEBRA));
    f.add(&link, model::RDF_PREDICATE, nn(&onto("hasBandTypeL")));
    f.add(&link, model::RDF_OBJECT, nn(APPLE));
    f.add(&link, kb::HAS_PERMISSIONS, plain(PUBLIC));

    let root = "http://rdfh.ch/lists/0803/ROOT";
    f.add(root, model::RDF_TYPE, nn(kb::LIST_NODE));
    f.add(root, kb::IS_ROOT_NODE, boolean("true"));
    f.add(root, model::RDFS_LABEL, plain("Bandtypen"));

    let mango = "http://rdfh.ch/0803/MANGO";
    page(&mut f, mango, Some(MEMBERS_ONLY));
    f.add(mango, kb::IS_DELETED, boolean("true"));
    text(&mut f, mango, "OFDELETED", Some(MEMBERS_ONLY));
    f
}

#[test]
fn a_fully_public_dump_lifts() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    text(&mut f, ZEBRA, "TEXT", Some(PUBLIC));

    let out = output(&f);

    assert!(out.contains("<urn:dsp:value:TEXT>"), "{out}");
}

#[test]
fn a_resource_granting_the_anonymous_user_nothing_is_refused_naming_it() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(MEMBERS_ONLY));
    page(&mut f, APPLE, Some(PUBLIC));

    assert_eq!(
        refusal(&f),
        vec![violation(
            ZEBRA,
            SubjectKind::Resource,
            &onto("Page"),
            Problem::NotViewable
        )]
    );
}

#[test]
fn restricted_view_on_a_text_file_value_is_refused() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let v = file(&mut f, ZEBRA, "TXT", kb::TEXT_FILE_VALUE, RESTRICTED);

    assert_eq!(
        refusal(&f),
        vec![violation(
            &v,
            SubjectKind::Value,
            kb::TEXT_FILE_VALUE,
            Problem::RestrictedViewNotAllowed
        )]
    );
}

#[test]
fn restricted_view_on_a_resource_is_refused() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(RESTRICTED));

    assert_eq!(
        refusal(&f),
        vec![violation(
            ZEBRA,
            SubjectKind::Resource,
            &onto("Page"),
            Problem::RestrictedViewNotAllowed
        )]
    );
}

#[test]
fn a_still_image_under_restricted_view_lifts_with_a_restricted_view_representation() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    file(&mut f, ZEBRA, "IMAGE", kb::STILL_IMAGE_FILE_VALUE, RESTRICTED);

    let out = output(&f);

    for line in rights_lines("urn:dsp:representation:IMAGE", "restricted-view") {
        assert!(out.contains(&line), "missing {line}\n{out}");
    }
    for line in rights_lines(ZEBRA, "public") {
        assert!(out.contains(&line), "missing {line}\n{out}");
    }
}

#[test]
fn a_selected_value_without_permissions_is_refused_as_missing_permissions() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let v = text(&mut f, ZEBRA, "TEXT", None);

    assert_eq!(
        refusal(&f),
        vec![violation(
            &v,
            SubjectKind::Value,
            kb::TEXT_VALUE,
            Problem::MissingPermissions
        )]
    );
}

#[test]
fn a_view_grant_to_known_users_alone_is_refused() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some("V knora-admin:KnownUser"));

    assert_eq!(
        refusal(&f),
        vec![violation(
            ZEBRA,
            SubjectKind::Resource,
            &onto("Page"),
            Problem::NotViewable
        )]
    );
}

#[test]
fn the_entries_of_a_permission_literal_pass_in_either_order() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some("V knora-admin:UnknownUser|CR knora-admin:Creator"));
    page(&mut f, APPLE, Some("CR knora-admin:Creator|V knora-admin:UnknownUser"));

    let out = output(&f);

    assert!(out.contains(ZEBRA) && out.contains(APPLE), "{out}");
}

#[test]
fn a_still_image_granting_view_besides_restricted_view_is_public() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let both = "RV knora-admin:UnknownUser|V knora-admin:UnknownUser";
    file(&mut f, ZEBRA, "IMAGE", kb::STILL_IMAGE_FILE_VALUE, both);

    let out = output(&f);

    for line in rights_lines("urn:dsp:representation:IMAGE", "public") {
        assert!(out.contains(&line), "missing {line}\n{out}");
    }
}

#[test]
fn a_link_value_owned_only_through_its_rdf_subject_is_checked() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    page(&mut f, APPLE, Some(PUBLIC));
    let link = format!("{ZEBRA}/values/LINK");
    f.add(ZEBRA, &onto("hasBandTypeL"), nn(APPLE));
    f.add(&link, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&link, model::RDF_SUBJECT, nn(ZEBRA));
    f.add(&link, model::RDF_PREDICATE, nn(&onto("hasBandTypeL")));
    f.add(&link, model::RDF_OBJECT, nn(APPLE));
    f.add(&link, kb::HAS_PERMISSIONS, plain(MEMBERS_ONLY));

    assert_eq!(
        refusal(&f),
        vec![violation(
            &link,
            SubjectKind::Value,
            kb::LINK_VALUE,
            Problem::NotViewable
        )]
    );
}

#[test]
fn an_unknown_token_is_refused_as_malformed() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some("CR knora-admin:Creator|VV knora-admin:UnknownUser"));

    assert_eq!(
        refusal(&f),
        vec![violation(
            ZEBRA,
            SubjectKind::Resource,
            &onto("Page"),
            Problem::MalformedPermissions
        )]
    );
}

#[test]
fn a_permission_that_is_not_one_plain_string_is_refused_as_malformed() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    f.add(ZEBRA, kb::HAS_PERMISSIONS, plain("V knora-admin:UnknownUser"));
    page(&mut f, APPLE, None);
    let tagged = Literal::new_language_tagged_literal("V knora-admin:UnknownUser", "en").unwrap();
    f.add(APPLE, kb::HAS_PERMISSIONS, tagged);

    let violations = refusal(&f);

    let problems: Vec<(&str, Problem)> = violations.iter().map(|v| (v.subject.as_str(), v.problem)).collect();
    assert_eq!(
        problems,
        vec![
            (APPLE, Problem::MalformedPermissions),
            (ZEBRA, Problem::MalformedPermissions)
        ]
    );
}

#[test]
fn a_superseded_value_granting_nothing_is_neither_checked_nor_written() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let current = text(&mut f, ZEBRA, "NEW", Some(PUBLIC));
    let old = text(&mut f, ZEBRA, "OLD", Some(MEMBERS_ONLY));
    f.add(&current, kb::PREVIOUS_VALUE, nn(&old));

    let out = output(&f);

    assert!(out.contains("<urn:dsp:value:NEW>"), "{out}");
    assert!(!out.contains("OLD"), "{out}");
}

#[test]
fn a_deleted_value_granting_nothing_is_neither_checked_nor_written() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let gone = text(&mut f, ZEBRA, "GONE", Some(MEMBERS_ONLY));
    f.add(&gone, kb::IS_DELETED, boolean("true"));

    let out = output(&f);

    assert!(!out.contains("GONE"), "{out}");
}

#[test]
fn a_value_of_a_deleted_resource_is_neither_checked_nor_written() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(MEMBERS_ONLY));
    f.add(ZEBRA, kb::IS_DELETED, boolean("true"));
    text(&mut f, ZEBRA, "ORPHANED", None);
    page(&mut f, APPLE, Some(PUBLIC));

    let out = output(&f);

    assert!(!out.contains("ORPHANED"), "{out}");
    assert!(!out.contains(ZEBRA), "{out}");
}

#[test]
fn two_violations_in_one_dump_are_both_reported_sorted_by_subject() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(MEMBERS_ONLY));
    page(&mut f, APPLE, None);

    let violations = refusal(&f);

    assert_eq!(
        violations,
        vec![
            violation(APPLE, SubjectKind::Resource, &onto("Page"), Problem::MissingPermissions),
            violation(ZEBRA, SubjectKind::Resource, &onto("Page"), Problem::NotViewable),
        ]
    );
    let text = LiftError::Refused { violations }.to_string();
    assert_eq!(text.lines().count(), 3, "{text}");
    assert!(text.lines().nth(1).is_some_and(|l| l.contains(APPLE)), "{text}");
}

#[test]
fn every_resource_and_representation_carries_one_access_right_and_unclassified() {
    let out = output(&rich_fixture());

    let count = |subject: &str, predicate: &str| {
        out.lines()
            .filter(|l| l.starts_with(&format!("<{subject}> <{predicate}> ")))
            .count()
    };
    let nodes = [
        (ZEBRA, "public"),
        (APPLE, "public"),
        ("urn:dsp:representation:IMAGE", "restricted-view"),
    ];
    for (subject, rights) in nodes {
        for line in rights_lines(subject, rights) {
            assert!(out.contains(&line), "missing {line}\n{out}");
        }
        assert_eq!(count(subject, vocab::DAO_ACCESS_RIGHTS), 1, "{subject}\n{out}");
        assert_eq!(count(subject, vocab::DAO_DATA_CLASSIFICATION), 1, "{subject}\n{out}");
    }
    let rights = out.lines().filter(|l| l.contains("#accessRights>")).count();
    assert_eq!(rights, 3, "only resources and representations carry rights\n{out}");
}

#[test]
fn lift_never_reads_admin_or_permission_graphs() {
    let f = rich_fixture();
    let dir = TempDir::new();
    write_dump(&f, &dir);
    let rdf = dir.path().join("data").join("rdf");
    // A valid first line (so a lenient parse would load it) before a broken one.
    let secret = "<http://rdfh.ch/users/SECRET> <http://www.w3.org/2000/01/rdf-schema#label> \"secret\" <http://www.knora.org/data/admin> .\nthis is not N-Quads\n";
    std::fs::write(rdf.join("admin.nq"), secret).unwrap();
    std::fs::write(rdf.join("permission.nq"), secret).unwrap();

    let lifted = crate::lift(dir.path()).unwrap();

    let mut bytes = Vec::new();
    lifted.snapshot.write_to(&mut bytes).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), output(&f));
}

#[test]
fn the_rich_fixture_lifts_to_the_pinned_golden_bytes() {
    let golden = include_str!("../testdata/golden.nq");

    let out = output(&rich_fixture());

    assert!(
        out == golden,
        "the lift's output changed: raise TRANSFORMATION_VERSION (now {}) and replace testdata/golden.nq \
         with the output below in the same commit\n{out}",
        crate::TRANSFORMATION_VERSION
    );
}

// ---------------------------------------------------------------- the selected files

#[test]
fn a_superseded_file_value_is_not_a_selected_file() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let current = file(&mut f, ZEBRA, "NEW", kb::STILL_IMAGE_FILE_VALUE, PUBLIC);
    let old = file(&mut f, ZEBRA, "OLD", kb::STILL_IMAGE_FILE_VALUE, PUBLIC);
    f.add(&current, kb::PREVIOUS_VALUE, nn(&old));

    let files = lift(&f).unwrap().files;

    let expected = SelectedFile {
        class: kb::STILL_IMAGE_FILE_VALUE.to_string(),
        internal_filename: Some("NEW.jp2".to_string()),
    };
    assert_eq!(files.of(ZEBRA), Some([expected].as_slice()));
}

#[test]
fn a_deleted_file_value_is_not_a_selected_file() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let gone = file(&mut f, ZEBRA, "GONE", kb::STILL_IMAGE_FILE_VALUE, PUBLIC);
    f.add(&gone, kb::IS_DELETED, boolean("true"));

    let files = lift(&f).unwrap().files;

    assert_eq!(files.of(ZEBRA), Some([].as_slice()));
}

#[test]
fn a_deleted_resource_has_no_selected_files() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    f.add(ZEBRA, kb::IS_DELETED, boolean("true"));
    file(&mut f, ZEBRA, "ORPHANED", kb::STILL_IMAGE_FILE_VALUE, PUBLIC);
    page(&mut f, APPLE, Some(PUBLIC));

    let files = lift(&f).unwrap().files;

    assert_eq!(files.of(ZEBRA), None);
    assert_eq!(files.of(APPLE), Some([].as_slice()));
}

#[test]
fn a_file_value_with_two_file_names_has_none() {
    let mut f = Fixture::new();
    page(&mut f, ZEBRA, Some(PUBLIC));
    let text = file(&mut f, ZEBRA, "TEI", kb::TEXT_FILE_VALUE, PUBLIC);
    f.add(&text, kb::INTERNAL_FILENAME, plain("other.xml"));

    let files = lift(&f).unwrap().files;

    let expected = SelectedFile {
        class: kb::TEXT_FILE_VALUE.to_string(),
        internal_filename: None,
    };
    assert_eq!(files.of(ZEBRA), Some([expected].as_slice()));
}
