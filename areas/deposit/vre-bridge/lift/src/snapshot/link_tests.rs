//! Fixture tests for links (§6) and the LinkValues that decide them.

use super::tests::{Fixture, GRAPH, boolean, integer, onto, output, plain, report, resource_lines, run};
use super::violations::Rule;
use super::{Stats, link, vocab};
use crate::model::{self, kb, nn};

const PAGE: &str = "http://rdfh.ch/0803/PAGE";
const BAND: &str = "http://rdfh.ch/0803/BAND";

pub(super) fn line(s: &str, p: &str, o: &str) -> String {
    format!("<{s}> <{p}> <{o}> {GRAPH} .")
}

fn file(mut lines: Vec<String>) -> String {
    lines.sort();
    lines.iter().map(|l| format!("{l}\n")).collect()
}

/// A LinkValue `id` describing `<s> <p> <o>`, attached to `s` by `p`'s companion
/// predicate, with every predicate 0803's LinkValues carry; returns its IRI.
pub(super) fn link_value(f: &mut Fixture, (s, p, o): (&str, &str, &str), id: &str, deleted: bool) -> String {
    let lv = format!("{s}/values/{id}");
    f.add(s, &format!("{p}Value"), nn(&lv));
    f.add(&lv, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&lv, model::RDF_SUBJECT, nn(s));
    f.add(&lv, model::RDF_PREDICATE, nn(p));
    f.add(&lv, model::RDF_OBJECT, nn(o));
    f.add(&lv, kb::IS_DELETED, boolean(if deleted { "true" } else { "false" }));
    f.add(&lv, kb::VALUE_HAS_UUID, plain(id));
    f.add(&lv, kb::VALUE_HAS_ORDER, integer("0"));
    f.add(&lv, kb::VALUE_HAS_REF_COUNT, integer("1"));
    f.add(&lv, kb::VALUE_HAS_STRING, plain(o));
    f.add(&lv, kb::ATTACHED_TO_USER, nn("http://rdfh.ch/users/U"));
    lv
}

/// The direct triple `<s> <p> <o>` and one live LinkValue for it.
pub(super) fn link(f: &mut Fixture, s: &str, p: &str, o: &str, id: &str) -> String {
    f.add(s, p, nn(o));
    link_value(f, (s, p, o), id, false)
}

/// A Page and a Band, with `incunabula:hasBandTypeL ⊑ kb:hasLinkTo` as in 0803.
fn page_and_band() -> Fixture {
    let mut f = Fixture::new();
    f.class("Band");
    f.onto_triple(&onto("hasBandTypeL"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_LINK_TO));
    f.resource(PAGE, "Page", "b7v");
    f.resource(BAND, "Band", "Randleiste 3");
    f
}

fn omitted(stats: &Stats, reason: &str) -> usize {
    stats.omitted.get(reason).copied().unwrap_or(0)
}

#[test]
fn a_link_emits_its_direct_triple_and_no_companion_link_value_uuid_or_order() {
    let mut f = page_and_band();
    let band_l = onto("hasBandTypeL");
    link(&mut f, PAGE, &band_l, BAND, "LV1");

    let mut expected = resource_lines(PAGE, &onto("Page"), "b7v");
    expected.extend(resource_lines(BAND, &onto("Band"), "Randleiste 3"));
    expected.push(line(PAGE, &band_l, BAND));
    let (out, stats) = run(&f);
    assert_eq!(out, file(expected));
    assert_eq!(stats.links, 1);
}

#[test]
fn a_link_to_a_deleted_resource_emits_nothing_and_is_counted() {
    let mut f = Fixture::new();
    f.class("Band");
    f.resource(PAGE, "Page", "b7v");
    f.add(BAND, model::RDF_TYPE, nn(&onto("Band")));
    f.add(BAND, model::RDFS_LABEL, plain("gone"));
    f.add(BAND, kb::IS_DELETED, boolean("true"));
    link(&mut f, PAGE, &onto("hasBandTypeL"), BAND, "LV1");

    let (out, stats) = run(&f);
    assert!(!out.contains(BAND), "{out}");
    assert_eq!(stats.links, 0);
    assert_eq!(omitted(&stats, link::TO_DELETED_RESOURCE), 1);
}

#[test]
fn a_link_to_an_iri_that_is_not_a_resource_of_the_dump_emits_nothing_and_is_counted() {
    let mut f = page_and_band();
    let other = "http://rdfh.ch/0801/OTHER";
    link(&mut f, PAGE, &onto("hasBandTypeL"), other, "LV1");

    let (out, stats) = run(&f);
    assert!(!out.contains(other), "{out}");
    assert_eq!(stats.links, 0);
    assert_eq!(omitted(&stats, link::TO_NON_RESOURCE), 1);
}

#[test]
fn a_link_whose_only_link_value_is_deleted_emits_nothing_though_the_direct_triple_is_there() {
    let mut f = page_and_band();
    let band_l = onto("hasBandTypeL");
    f.add(PAGE, &band_l, nn(BAND));
    let lv = link_value(&mut f, (PAGE, &band_l, BAND), "LV1", true);
    // A dropped LinkValue's predicates are not checked (§11).
    f.add(&lv, "http://example.org/unchecked", plain("x"));

    let (out, stats) = run(&f);
    assert!(!out.contains(&line(PAGE, &band_l, BAND)), "{out}");
    assert_eq!(stats.links, 0);
    assert_eq!(omitted(&stats, link::LINK_VALUE_DROPPED), 1);
}

#[test]
fn a_link_deleted_and_re_created_is_emitted_once() {
    let mut f = page_and_band();
    let band_l = onto("hasBandTypeL");
    f.add(PAGE, &band_l, nn(BAND));
    link_value(&mut f, (PAGE, &band_l, BAND), "OLD", true);
    let new = link_value(&mut f, (PAGE, &band_l, BAND), "NEW", false);
    // The live one is itself the head of a version chain: `kb:previousValue`
    // is on the LinkValue list, and the superseded version is dropped.
    let v1 = link_value(&mut f, (PAGE, &band_l, BAND), "V1", false);
    f.add(&new, kb::PREVIOUS_VALUE, nn(&v1));

    let (out, stats) = run(&f);
    assert_eq!(out.matches(&line(PAGE, &band_l, BAND)).count(), 1, "{out}");
    assert_eq!(stats.links, 1);
}

#[test]
fn a_link_without_link_value_and_a_link_value_without_link_are_violations() {
    let mut f = page_and_band();
    let band_l = onto("hasBandTypeL");
    let band_r = onto("hasBandTypeR");
    f.add(PAGE, &band_l, nn(BAND));
    let unpaired = link_value(&mut f, (PAGE, &band_r, BAND), "UNPAIRED", false);
    f.add(&unpaired, "http://example.org/unknown", plain("x"));
    // A LinkValue without its object is the one violation: the direct triple it
    // was meant to describe is not also reported as missing a LinkValue.
    let band_x = onto("hasBandTypeX");
    f.add(PAGE, &band_x, nn(BAND));
    let broken = format!("{PAGE}/values/BROKEN");
    f.add(&broken, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&broken, model::RDF_SUBJECT, nn(PAGE));
    f.add(&broken, model::RDF_PREDICATE, nn(&band_x));
    f.add(&broken, kb::IS_DELETED, boolean("false"));

    let mut expected = vec![
        (Rule::MissingLinkValue, PAGE.to_string(), format!("{band_l} {BAND}")),
        (Rule::UnpairedLinkValue, unpaired.clone(), format!("{PAGE} {band_r} {BAND}")),
        (
            Rule::UnknownLinkValuePredicate,
            unpaired,
            "http://example.org/unknown".to_string(),
        ),
        (Rule::InvalidLinkValue, broken, model::RDF_OBJECT.to_string()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_dropped_link_value_without_its_object_leaves_its_direct_triple_missing_a_link_value() {
    let mut f = page_and_band();
    let band_l = onto("hasBandTypeL");
    f.add(PAGE, &band_l, nn(BAND));
    let broken = format!("{PAGE}/values/BROKEN");
    f.add(&broken, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&broken, model::RDF_SUBJECT, nn(PAGE));
    f.add(&broken, model::RDF_PREDICATE, nn(&band_l));
    f.add(&broken, kb::IS_DELETED, boolean("true"));

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MissingLinkValue, PAGE.to_string(), format!("{band_l} {BAND}"),)]
    );
}

#[test]
fn a_standoff_link_and_its_link_value_emit_nothing() {
    let mut f = page_and_band();
    link(&mut f, PAGE, kb::HAS_STANDOFF_LINK_TO, BAND, "LV1");
    // §6 rule 3 omits a standoff link whatever its object.
    f.add(PAGE, kb::HAS_STANDOFF_LINK_TO, plain("not an IRI"));

    let mut expected = resource_lines(PAGE, &onto("Page"), "b7v");
    expected.extend(resource_lines(BAND, &onto("Band"), "Randleiste 3"));
    assert_eq!(output(&f.dump()), file(expected));
}

#[test]
fn kb_has_comment_and_incunabula_has_comment_keep_their_distinct_full_iris() {
    let mut f = page_and_band();
    f.onto_triple(&onto("hasComment"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_COMMENT));
    for (resource, property, uuid) in [
        (PAGE, onto("hasComment"), "PAGECOMMENT"),
        (BAND, kb::HAS_COMMENT.to_string(), "BANDCOMMENT"),
    ] {
        let value = format!("{resource}/values/{uuid}");
        f.add(resource, &property, nn(&value));
        f.add(&value, model::RDF_TYPE, nn(kb::TEXT_VALUE));
        f.add(&value, kb::VALUE_HAS_UUID, plain(uuid));
        f.add(&value, kb::VALUE_HAS_STRING, plain("a comment"));
        f.add(&value, kb::IS_DELETED, boolean("false"));
    }

    let out = output(&f.dump());
    for (property, uuid) in [
        (onto("hasComment"), "PAGECOMMENT"),
        (kb::HAS_COMMENT.to_string(), "BANDCOMMENT"),
    ] {
        let node = format!("{}{uuid}", vocab::VALUE_PREFIX);
        let expected = line(&node, vocab::DAO_SOURCE_PROPERTY, &property);
        assert!(out.contains(&expected), "{out}");
    }
}
