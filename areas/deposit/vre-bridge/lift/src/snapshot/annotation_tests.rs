//! Fixture tests for annotations (§10): the marker, the motivation and the targets.

use oxigraph::model::{GraphName, Quad};

use super::link_tests::{line, link};
use super::tests::{Fixture, boolean, onto, plain, report, run};
use super::violations::Rule;
use super::vocab;
use crate::model::{self, kb, nn};

const PAGE: &str = "http://rdfh.ch/0803/PAGE";
const BOOK: &str = "http://rdfh.ch/0803/BOOK";
const REGION: &str = "http://rdfh.ch/0803/REGION";
const LINK_OBJ: &str = "http://rdfh.ch/0803/LINKOBJ";

/// A live resource typed `class` (a full IRI) with one plain label.
fn typed_resource(f: &mut Fixture, iri: &str, class: &str, deleted: bool) {
    f.add(iri, model::RDF_TYPE, nn(class));
    f.add(iri, model::RDFS_LABEL, plain("label"));
    f.add(iri, kb::IS_DELETED, boolean(if deleted { "true" } else { "false" }));
}

/// A Page, and a resource of `class` that is a Region of it.
fn region_of_page(class: &str) -> Fixture {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    typed_resource(&mut f, REGION, class, false);
    link(&mut f, REGION, kb::IS_REGION_OF, PAGE, "REGIONLV");
    f
}

/// A text value of `resource` under `property`; returns its IRI.
fn text(f: &mut Fixture, resource: &str, property: &str, id: &str, deleted: bool) -> String {
    let iri = format!("{resource}/values/{id}");
    f.add(resource, property, nn(&iri));
    f.add(&iri, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&iri, kb::VALUE_HAS_UUID, plain(id));
    f.add(&iri, kb::VALUE_HAS_STRING, plain("a comment"));
    f.add(&iri, kb::IS_DELETED, boolean(if deleted { "true" } else { "false" }));
    iri
}

/// The two marker lines of an annotation motivated by `motivation`.
fn marker(s: &str, motivation: &str) -> [String; 2] {
    [
        line(s, model::RDF_TYPE, vocab::OA_ANNOTATION),
        line(s, vocab::OA_MOTIVATED_BY, motivation),
    ]
}

fn target(s: &str, o: &str) -> String {
    line(s, vocab::OA_HAS_TARGET, o)
}

fn assert_has(out: &str, lines: &[String]) {
    for l in lines {
        assert!(out.contains(&format!("{l}\n")), "missing {l} in\n{out}");
    }
}

/// The `oa:` lines of the output.
fn oa_lines(out: &str) -> Vec<&str> {
    out.lines().filter(|l| l.contains("/ns/oa#")).collect()
}

#[test]
fn a_commented_region_is_a_commenting_annotation_targeting_its_page_and_keeps_its_edges() {
    let mut f = region_of_page(kb::REGION);
    text(&mut f, REGION, kb::HAS_COMMENT, "COMMENT", false);
    let color = format!("{REGION}/values/COLOR");
    f.add(REGION, kb::HAS_COLOR, nn(&color));
    f.add(&color, model::RDF_TYPE, nn(kb::COLOR_VALUE));
    f.add(&color, kb::VALUE_HAS_UUID, plain("COLOR"));
    f.add(&color, kb::VALUE_HAS_COLOR, plain("#ff3333"));
    let geometry = format!("{REGION}/values/GEOM");
    f.add(REGION, kb::HAS_GEOMETRY, nn(&geometry));
    f.add(&geometry, model::RDF_TYPE, nn(kb::GEOM_VALUE));
    f.add(&geometry, kb::VALUE_HAS_UUID, plain("GEOM"));
    f.add(&geometry, kb::VALUE_HAS_GEOMETRY, plain("{}"));

    let (out, stats) = run(&f);
    let node = |uuid: &str| format!("{}{uuid}", vocab::VALUE_PREFIX);
    let mut expected = marker(REGION, vocab::OA_COMMENTING).to_vec();
    expected.extend([
        target(REGION, PAGE),
        line(REGION, model::RDF_TYPE, vocab::DAO_RESOURCE),
        line(REGION, kb::IS_REGION_OF, PAGE),
        line(REGION, kb::HAS_COMMENT, &node("COMMENT")),
        line(REGION, kb::HAS_COLOR, &node("COLOR")),
        line(REGION, kb::HAS_GEOMETRY, &node("GEOM")),
    ]);
    assert_has(&out, &expected);
    assert_eq!(oa_lines(&out).len(), 3, "{out}");
    assert_eq!(
        stats.annotations_by_motivation.into_iter().collect::<Vec<_>>(),
        vec![(vocab::OA_COMMENTING, 1)]
    );
    assert_eq!(stats.annotation_targets, 1);
}

#[test]
fn a_region_without_a_comment_is_highlighting() {
    let f = region_of_page(kb::REGION);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
    assert!(!out.contains(vocab::OA_COMMENTING), "{out}");
}

#[test]
fn a_region_whose_only_comment_is_deleted_is_highlighting() {
    let mut f = region_of_page(kb::REGION);
    text(&mut f, REGION, kb::HAS_COMMENT, "COMMENT", true);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
    assert!(!out.contains(vocab::OA_COMMENTING), "{out}");
}

/// `proj:DetailRegion ⊑ proj:Region2 ⊑ kb:Region`, both declared project classes.
fn region_sub_classes(f: &mut Fixture) {
    f.class("DetailRegion");
    f.class("Region2");
    f.onto_triple(&onto("DetailRegion"), model::RDFS_SUBCLASSOF, nn(&onto("Region2")));
    f.onto_triple(&onto("Region2"), model::RDFS_SUBCLASSOF, nn(kb::REGION));
}

#[test]
fn a_leaf_sub_class_of_region_is_an_annotation_keeping_its_full_source_class() {
    let mut f = region_of_page(&onto("DetailRegion"));
    region_sub_classes(&mut f);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
    assert_has(&out, &[line(REGION, vocab::DAO_SOURCE_CLASS, &onto("DetailRegion"))]);
}

#[test]
fn an_intermediate_sub_class_of_region_is_an_annotation_too() {
    let mut f = region_of_page(&onto("Region2"));
    region_sub_classes(&mut f);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
}

#[test]
fn a_sub_class_cycle_terminates_and_a_class_on_it_reaching_region_is_marked() {
    let mut f = region_of_page(&onto("A"));
    f.class("A");
    f.class("B");
    f.onto_triple(&onto("A"), model::RDFS_SUBCLASSOF, nn(&onto("B")));
    f.onto_triple(&onto("B"), model::RDFS_SUBCLASSOF, nn(&onto("A")));
    f.onto_triple(&onto("B"), model::RDFS_SUBCLASSOF, nn(kb::REGION));
    // A second cycle that reaches nothing: its resource is no annotation.
    f.class("C");
    f.class("D");
    f.onto_triple(&onto("C"), model::RDFS_SUBCLASSOF, nn(&onto("D")));
    f.onto_triple(&onto("D"), model::RDFS_SUBCLASSOF, nn(&onto("C")));
    typed_resource(&mut f, BOOK, &onto("C"), false);

    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
    assert_eq!(oa_lines(&out).len(), 3, "{out}");
}

#[test]
fn a_link_obj_with_two_links_is_linking_with_one_target_each_and_keeps_both_links() {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    f.resource(BOOK, "Book", "Narrenschiff");
    typed_resource(&mut f, LINK_OBJ, kb::LINK_OBJ, false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, BOOK, "LV2");

    let (out, stats) = run(&f);
    let mut expected = marker(LINK_OBJ, vocab::OA_LINKING).to_vec();
    expected.extend([
        target(LINK_OBJ, PAGE),
        target(LINK_OBJ, BOOK),
        line(LINK_OBJ, kb::HAS_LINK_TO, PAGE),
        line(LINK_OBJ, kb::HAS_LINK_TO, BOOK),
    ]);
    assert_has(&out, &expected);
    assert_eq!(oa_lines(&out).len(), 4, "{out}");
    assert_eq!(stats.links, 2);
    assert_eq!(stats.annotation_targets, 2);
    assert_eq!(
        stats.annotations_by_motivation.into_iter().collect::<Vec<_>>(),
        vec![(vocab::OA_LINKING, 1)]
    );
}

#[test]
fn a_project_sub_class_of_link_obj_is_linking() {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    f.class("Citation");
    f.onto_triple(&onto("Citation"), model::RDFS_SUBCLASSOF, nn(kb::LINK_OBJ));
    typed_resource(&mut f, LINK_OBJ, &onto("Citation"), false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");

    let (out, _) = run(&f);
    assert_has(&out, &marker(LINK_OBJ, vocab::OA_LINKING));
    assert_has(&out, &[target(LINK_OBJ, PAGE)]);
}

#[test]
fn a_page_is_no_annotation_even_with_a_link() {
    let mut f = Fixture::new();
    f.onto_triple(&onto("partOf"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_LINK_TO));
    f.resource(PAGE, "Page", "a1r");
    f.resource(BOOK, "Book", "Narrenschiff");
    link(&mut f, PAGE, &onto("partOf"), BOOK, "LV1");

    let (out, stats) = run(&f);
    assert_has(&out, &[line(PAGE, &onto("partOf"), BOOK)]);
    assert!(oa_lines(&out).is_empty(), "{out}");
    assert!(stats.annotations_by_motivation.is_empty());
}

#[test]
fn a_deleted_target_gives_neither_a_target_nor_a_link() {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    typed_resource(&mut f, BOOK, &onto("Book"), true);
    typed_resource(&mut f, LINK_OBJ, kb::LINK_OBJ, false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, BOOK, "LV2");

    let (out, _) = run(&f);
    assert_has(&out, &[target(LINK_OBJ, PAGE), line(LINK_OBJ, kb::HAS_LINK_TO, PAGE)]);
    assert!(!out.contains(BOOK), "{out}");
}

#[test]
fn an_annotation_whose_only_target_is_deleted_is_a_violation() {
    let mut f = Fixture::new();
    typed_resource(&mut f, PAGE, &onto("Page"), true);
    typed_resource(&mut f, REGION, kb::REGION, false);
    link(&mut f, REGION, kb::IS_REGION_OF, PAGE, "REGIONLV");

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MissingAnnotationTarget, REGION.to_string(), kb::IS_REGION_OF.to_string(),)]
    );
}

#[test]
fn a_region_targets_only_its_is_region_of_links_never_another_link_or_a_standoff_link() {
    let mut f = region_of_page(&onto("DetailRegion"));
    region_sub_classes(&mut f);
    f.resource(BOOK, "Book", "Narrenschiff");
    f.onto_triple(&onto("seeAlso"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_LINK_TO));
    link(&mut f, REGION, &onto("seeAlso"), BOOK, "SEEALSOLV");
    link(&mut f, REGION, kb::HAS_STANDOFF_LINK_TO, BOOK, "STANDOFFLV");

    let (out, _) = run(&f);
    assert_has(&out, &[target(REGION, PAGE), line(REGION, &onto("seeAlso"), BOOK)]);
    assert!(!out.contains(&target(REGION, BOOK)), "{out}");
    assert!(!out.contains(kb::HAS_STANDOFF_LINK_TO), "{out}");
}

#[test]
fn a_comment_under_a_sub_property_of_has_comment_is_commenting() {
    let mut f = region_of_page(kb::REGION);
    f.onto_triple(&onto("regionNote"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_COMMENT));
    text(&mut f, REGION, &onto("regionNote"), "NOTE", false);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_COMMENTING));
}

#[test]
fn a_text_value_under_another_property_is_no_comment() {
    let mut f = region_of_page(kb::REGION);
    text(&mut f, REGION, &onto("hasDescription"), "DESC", false);
    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
}

#[test]
fn the_sub_class_closure_reads_every_ontology_graph() {
    let second = "http://www.knora.org/ontology/0803/second";
    let class = format!("{second}#DetailRegion");
    let mut f = region_of_page(&class);
    let g = GraphName::NamedNode(nn(second));
    for (p, o) in [
        (model::RDF_TYPE, model::OWL_CLASS),
        (model::RDFS_SUBCLASSOF, kb::REGION),
    ] {
        f.quads.push(Quad::new(nn(&class), nn(p), nn(o), g.clone()));
    }

    let (out, _) = run(&f);
    assert_has(&out, &marker(REGION, vocab::OA_HIGHLIGHTING));
}

#[test]
fn a_class_reaching_both_region_and_link_obj_is_a_violation() {
    let mut f = region_of_page(&onto("Both"));
    f.class("Both");
    f.onto_triple(&onto("Both"), model::RDFS_SUBCLASSOF, nn(kb::REGION));
    f.onto_triple(&onto("Both"), model::RDFS_SUBCLASSOF, nn(kb::LINK_OBJ));

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::AmbiguousAnnotationClass, REGION.to_string(), onto("Both"),)]
    );
}

#[test]
fn two_links_to_one_target_give_one_target() {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    f.onto_triple(&onto("cites"), model::RDFS_SUBPROPERTYOF, nn(kb::HAS_LINK_TO));
    typed_resource(&mut f, LINK_OBJ, kb::LINK_OBJ, false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");
    link(&mut f, LINK_OBJ, &onto("cites"), PAGE, "LV2");

    let (out, stats) = run(&f);
    assert_eq!(out.matches(&target(LINK_OBJ, PAGE)).count(), 1, "{out}");
    assert_eq!(stats.links, 2);
    assert_eq!(stats.annotation_targets, 1);
}

#[test]
fn a_link_obj_whose_only_target_is_deleted_is_a_violation_naming_has_link_to() {
    let mut f = Fixture::new();
    typed_resource(&mut f, PAGE, &onto("Page"), true);
    typed_resource(&mut f, LINK_OBJ, kb::LINK_OBJ, false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MissingAnnotationTarget, LINK_OBJ.to_string(), kb::HAS_LINK_TO.to_string(),)]
    );
}

#[test]
fn a_standoff_link_is_never_a_link_obj_target() {
    let mut f = Fixture::new();
    f.resource(PAGE, "Page", "a1r");
    f.resource(BOOK, "Book", "Narrenschiff");
    typed_resource(&mut f, LINK_OBJ, kb::LINK_OBJ, false);
    link(&mut f, LINK_OBJ, kb::HAS_LINK_TO, PAGE, "LV1");
    link(&mut f, LINK_OBJ, kb::HAS_STANDOFF_LINK_TO, BOOK, "LV2");

    let (out, _) = run(&f);
    assert_has(&out, &[target(LINK_OBJ, PAGE)]);
    assert!(!out.contains(&target(LINK_OBJ, BOOK)), "{out}");
}
