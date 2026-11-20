//! Fixture tests for membership and seqnum (§7).

use super::link_tests::{line, link};
use super::tests::{Fixture, GRAPH, boolean, integer, onto, plain, report, run};
use super::violations::Rule;
use super::{link, vocab};
use crate::model::{self, kb, nn};

const PAGE: &str = "http://rdfh.ch/0803/PAGE";
const BOOK: &str = "http://rdfh.ch/0803/BOOK";
const ATLAS: &str = "http://rdfh.ch/0803/ATLAS";

/// Declare `incunabula:<local> rdfs:subPropertyOf <parent>`; returns the property.
fn sub_property(f: &mut Fixture, local: &str, parent: &str) -> String {
    let p = onto(local);
    f.onto_triple(&p, model::RDFS_SUBPROPERTYOF, nn(parent));
    p
}

/// A live IntValue `id` holding `n`, attached to `resource` by `property`.
fn int_value(f: &mut Fixture, resource: &str, property: &str, id: &str, n: &str) -> String {
    let v = format!("{resource}/values/{id}");
    f.add(resource, property, nn(&v));
    f.add(&v, model::RDF_TYPE, nn(kb::INT_VALUE));
    f.add(&v, kb::VALUE_HAS_UUID, plain(id));
    f.add(&v, kb::VALUE_HAS_INTEGER, integer(n));
    f.add(&v, kb::VALUE_HAS_STRING, plain(n));
    f.add(&v, kb::IS_DELETED, boolean("false"));
    v
}

/// A Page, two Books, and 0803's `isPartOfBook ⊑ kb:isPartOf`, `hasSeqnum ⊑ kb:seqnum`.
fn page_and_books() -> Fixture {
    let mut f = Fixture::new();
    sub_property(&mut f, "isPartOfBook", kb::IS_PART_OF);
    sub_property(&mut f, "hasSeqnum", kb::SEQNUM);
    f.resource(PAGE, "Page", "b7v");
    f.resource(BOOK, "Book", "Narrenschiff");
    f.resource(ATLAS, "Book", "Weltchronik");
    f
}

fn seqnum_line(s: &str, n: &str) -> String {
    format!("<{s}> <{}> \"{n}\"^^<{}> {GRAPH} .", vocab::DAO_SEQNUM, model::XSD_INTEGER)
}

#[test]
fn two_sub_properties_naming_one_parent_give_one_dao_is_part_of_counted_once() {
    let mut f = page_and_books();
    let in_volume = sub_property(&mut f, "inVolume", kb::IS_PART_OF);
    link(&mut f, PAGE, &onto("isPartOfBook"), BOOK, "LV1");
    link(&mut f, PAGE, &in_volume, BOOK, "LV2");

    let (out, stats) = run(&f);
    assert_eq!(out.matches(vocab::DAO_IS_PART_OF).count(), 1, "{out}");
    assert_eq!(stats.memberships, 1);
}

#[test]
fn a_deleted_seqnum_that_is_not_an_int_value_is_dropped_not_reported() {
    let mut f = page_and_books();
    let text = format!("{PAGE}/values/TEXT");
    f.add(PAGE, &onto("hasSeqnum"), nn(&text));
    f.add(&text, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&text, kb::IS_DELETED, boolean("true"));

    let (out, _) = run(&f);
    assert!(!out.contains(vocab::DAO_SEQNUM), "{out}");
}
#[test]
fn is_part_of_book_emits_dao_is_part_of_and_no_triple_under_the_sub_property() {
    let mut f = page_and_books();
    link(&mut f, PAGE, &onto("isPartOfBook"), BOOK, "LV1");

    let (out, stats) = run(&f);
    assert!(out.contains(&line(PAGE, vocab::DAO_IS_PART_OF, BOOK)), "{out}");
    assert!(!out.contains(&onto("isPartOfBook")), "{out}");
    assert_eq!((stats.memberships, stats.links), (1, 0));
}

#[test]
fn has_seqnum_emits_dao_seqnum_on_the_child_and_no_value_node() {
    let mut f = page_and_books();
    int_value(&mut f, PAGE, &onto("hasSeqnum"), "SEQ", "7");

    let (out, stats) = run(&f);
    assert!(out.contains(&seqnum_line(PAGE, "7")), "{out}");
    assert!(!out.contains(vocab::VALUE_PREFIX), "{out}");
    assert!(!out.contains(&onto("hasSeqnum")), "{out}");
    assert_eq!(stats.seqnums, 1);
}

#[test]
fn kb_seqnum_used_directly_also_emits_dao_seqnum() {
    let mut f = page_and_books();
    int_value(&mut f, PAGE, kb::SEQNUM, "SEQ", "3");

    let (out, _) = run(&f);
    assert!(out.contains(&seqnum_line(PAGE, "3")), "{out}");
    assert!(!out.contains(vocab::VALUE_PREFIX), "{out}");
}

#[test]
fn a_resource_with_two_parents_emits_two_dao_is_part_of() {
    let mut f = page_and_books();
    link(&mut f, PAGE, &onto("isPartOfBook"), BOOK, "LV1");
    link(&mut f, PAGE, &onto("isPartOfBook"), ATLAS, "LV2");

    let (out, stats) = run(&f);
    assert!(out.contains(&line(PAGE, vocab::DAO_IS_PART_OF, BOOK)), "{out}");
    assert!(out.contains(&line(PAGE, vocab::DAO_IS_PART_OF, ATLAS)), "{out}");
    assert_eq!(stats.memberships, 2);
}

#[test]
fn a_membership_to_a_deleted_parent_is_dropped_and_the_child_keeps_its_seqnum() {
    let mut f = Fixture::new();
    sub_property(&mut f, "isPartOfBook", kb::IS_PART_OF);
    sub_property(&mut f, "hasSeqnum", kb::SEQNUM);
    f.resource(PAGE, "Page", "b7v");
    f.add(BOOK, model::RDF_TYPE, nn(&onto("Book")));
    f.add(BOOK, model::RDFS_LABEL, plain("gone"));
    f.add(BOOK, kb::IS_DELETED, boolean("true"));
    link(&mut f, PAGE, &onto("isPartOfBook"), BOOK, "LV1");
    int_value(&mut f, PAGE, &onto("hasSeqnum"), "SEQ", "7");

    let (out, stats) = run(&f);
    assert!(!out.contains(vocab::DAO_IS_PART_OF), "{out}");
    assert!(out.contains(&seqnum_line(PAGE, "7")), "{out}");
    assert_eq!(stats.omitted.get(link::TO_DELETED_RESOURCE), Some(&1));
}

#[test]
fn two_live_seqnums_and_a_seqnum_that_is_not_an_int_value_are_violations() {
    let mut f = page_and_books();
    let has_seqnum = onto("hasSeqnum");
    int_value(&mut f, PAGE, &has_seqnum, "SEQ1", "7");
    int_value(&mut f, PAGE, &has_seqnum, "SEQ2", "8");
    // A deleted third one is dropped, not counted as a third.
    let gone = int_value(&mut f, PAGE, &has_seqnum, "SEQ3", "9");
    f.add(&gone, kb::IS_DELETED, boolean("true"));
    let text = format!("{BOOK}/values/TEXT");
    f.add(BOOK, &has_seqnum, nn(&text));
    f.add(&text, model::RDF_TYPE, nn(kb::TEXT_VALUE));

    let mut expected = vec![
        (Rule::RepeatedSeqnum, PAGE.to_string(), String::new()),
        (Rule::InvalidSeqnum, BOOK.to_string(), text),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_membership_cycle_is_a_violation_naming_only_the_resources_on_it() {
    let mut f = page_and_books();
    let part_of = onto("isPartOfBook");
    link(&mut f, BOOK, &part_of, ATLAS, "LV1");
    link(&mut f, ATLAS, &part_of, BOOK, "LV2");
    // The page reaches the cycle but is not on it.
    link(&mut f, PAGE, &part_of, BOOK, "LV3");

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::MembershipCycle, ATLAS.to_string(), String::new()),
            (Rule::MembershipCycle, BOOK.to_string(), String::new()),
        ]
    );
}

#[test]
fn sub_property_cycles_and_diamonds_terminate_and_a_diamond_yields_one_dao_is_part_of() {
    let mut f = page_and_books();
    // A cycle that never reaches kb:isPartOf: its members stay plain links.
    let p1 = sub_property(&mut f, "hasP1", &onto("hasP2"));
    sub_property(&mut f, "hasP2", &p1);
    // A diamond: two paths from `inVolume` to kb:isPartOf.
    let in_volume = sub_property(&mut f, "inVolume", &onto("partOfA"));
    f.onto_triple(&in_volume, model::RDFS_SUBPROPERTYOF, nn(&onto("partOfB")));
    sub_property(&mut f, "partOfA", kb::IS_PART_OF);
    sub_property(&mut f, "partOfB", kb::IS_PART_OF);
    link(&mut f, PAGE, &in_volume, BOOK, "LV1");
    link(&mut f, PAGE, &p1, ATLAS, "LV2");

    let (out, stats) = run(&f);
    assert_eq!(out.matches(vocab::DAO_IS_PART_OF).count(), 1, "{out}");
    assert!(out.contains(&line(PAGE, &p1, ATLAS)), "{out}");
    assert_eq!((stats.memberships, stats.links), (1, 1));
}
