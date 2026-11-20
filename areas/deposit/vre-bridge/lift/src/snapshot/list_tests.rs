//! Fixture tests for list nodes (§9) and list values (§4.7). Node ids are not in
//! alphabetical insertion order, and labels do not sort in tree order.

use oxigraph::model::Literal;

use super::list::DELETED_LIST_NODE;
use super::tests::{Fixture, ZEBRA, boolean, onto, output, report, resource_lines};
use super::violations::Rule;
use super::{Stats, build, vocab};
use crate::model::{self, kb, nn};

const GRAPH: &str = "<urn:dsp:project:0803>";

fn iri(id: &str) -> String {
    format!("http://rdfh.ch/lists/0803/{id}")
}

fn line(s: &str, p: &str, o: &str) -> String {
    format!("<{s}> <{p}> {o} {GRAPH} .")
}

fn file(mut lines: Vec<String>) -> String {
    lines.sort();
    lines.iter().map(|l| format!("{l}\n")).collect()
}

fn tagged(v: &str, tag: &str) -> Literal {
    Literal::new_language_tagged_literal(v, tag).unwrap()
}

fn integer(v: &str) -> Literal {
    Literal::new_typed_literal(v, nn(model::XSD_INTEGER))
}

/// A list node with the given labels; returns its IRI.
fn node(f: &mut Fixture, id: &str, labels: &[Literal]) -> String {
    let n = iri(id);
    f.add(&n, model::RDF_TYPE, nn(kb::LIST_NODE));
    for label in labels {
        f.add(&n, model::RDFS_LABEL, label.clone());
    }
    n
}

fn root(f: &mut Fixture, id: &str, labels: &[Literal]) -> String {
    let n = node(f, id, labels);
    f.add(&n, kb::IS_ROOT_NODE, boolean("true"));
    n
}

/// A non-root under `parent`, at `position` when given.
fn child(f: &mut Fixture, parent: &str, id: &str, position: Option<&str>, labels: &[Literal]) -> String {
    let n = node(f, id, labels);
    f.add(parent, kb::HAS_SUB_LIST_NODE, nn(&n));
    if let Some(position) = position {
        f.add(&n, kb::LIST_NODE_POSITION, integer(position));
    }
    n
}

fn concept(n: &str) -> String {
    line(n, model::RDF_TYPE, &format!("<{}>", vocab::SKOS_CONCEPT))
}

fn pref_label(n: &str, label: &Literal) -> String {
    line(n, vocab::SKOS_PREF_LABEL, &label.to_string())
}

fn broader(n: &str, parent: &str) -> String {
    line(n, vocab::SKOS_BROADER, &format!("<{parent}>"))
}

fn position(n: &str, p: &str) -> String {
    line(n, vocab::DAO_LIST_NODE_POSITION, &integer(p).to_string())
}

#[test]
fn a_three_level_list_emits_every_node_as_a_concept_with_labels_broader_and_position() {
    let mut f = Fixture::new();
    let (r_de, r_en) = (tagged("Zeta", "de"), tagged("Omega", "en"));
    let r = root(&mut f, "ROOT", &[r_de.clone(), r_en.clone()]);
    // Position 1 before position 0, and "Beta" at position 1 before "Alpha".
    let (b_de, b_en) = (tagged("Beta", "de"), tagged("Bêta", "fr"));
    let b = child(&mut f, &r, "ZZ", Some("1"), &[b_de.clone(), b_en.clone()]);
    let (a_de, a_fr) = (tagged("Yankee", "de"), tagged("Alpha", "fr"));
    let a = child(&mut f, &r, "AA", Some("0"), &[a_de.clone(), a_fr.clone()]);
    let (c_de, c_en) = (tagged("Xray", "de"), tagged("Gamma", "en"));
    let c = child(&mut f, &b, "MM", Some("0"), &[c_de.clone(), c_en.clone()]);
    for n in [&b, &a, &c] {
        f.add(n, kb::HAS_ROOT_NODE, nn(&r));
    }

    let expected = file(vec![
        concept(&r),
        pref_label(&r, &r_de),
        pref_label(&r, &r_en),
        concept(&b),
        pref_label(&b, &b_de),
        pref_label(&b, &b_en),
        broader(&b, &r),
        position(&b, "1"),
        concept(&a),
        pref_label(&a, &a_de),
        pref_label(&a, &a_fr),
        broader(&a, &r),
        position(&a, "0"),
        concept(&c),
        pref_label(&c, &c_de),
        pref_label(&c, &c_en),
        broader(&c, &b),
        position(&c, "0"),
    ]);
    let dump = f.dump();
    assert_eq!(output(&dump), expected);
    let mut stats = Stats::default();
    build(&dump, &mut stats).unwrap();
    assert_eq!(stats.list_nodes, 4);
}

#[test]
fn an_untagged_list_label_emits_a_plain_pref_label() {
    let mut f = Fixture::new();
    let label = Literal::new_simple_literal("Rubrik");
    let r = root(&mut f, "ROOT", &[label.clone(), tagged("Rubric", "en")]);

    let out = output(&f.dump());
    assert!(out.lines().any(|l| l == pref_label(&r, &label)), "{out}");
}

#[test]
fn a_non_root_without_a_position_emits_no_position() {
    let mut f = Fixture::new();
    let r = root(&mut f, "ROOT", &[]);
    let c = child(&mut f, &r, "C", None, &[]);

    assert_eq!(output(&f.dump()), file(vec![concept(&r), concept(&c), broader(&c, &r)]));
}

#[test]
fn list_node_name_comment_root_node_and_project_emit_nothing_and_other_predicates_are_violations() {
    let mut f = Fixture::new();
    let r = root(&mut f, "ROOT", &[]);
    f.add(&r, kb::LIST_NODE_NAME, Literal::new_simple_literal("root"));
    f.add(&r, model::RDFS_COMMENT, tagged("Kommentar", "de"));
    f.add(&r, kb::ATTACHED_TO_PROJECT, nn("http://rdfh.ch/projects/0803"));
    f.add(&r, kb::IS_DELETED, boolean("false"));
    let c = child(&mut f, &r, "C", None, &[]);
    f.add(&c, kb::HAS_ROOT_NODE, nn(&r));
    assert_eq!(output(&f.dump()), file(vec![concept(&r), concept(&c), broader(&c, &r)]));

    f.add(&c, "http://purl.org/dc/terms/title", Literal::new_simple_literal("x"));
    assert_eq!(
        report(&f.dump()),
        vec![(Rule::UnknownListNodePredicate, c, "http://purl.org/dc/terms/title".to_string())]
    );
}

#[test]
fn a_deleted_list_node_is_dropped_with_its_subtree() {
    let mut f = Fixture::new();
    let r = root(&mut f, "ROOT", &[]);
    let gone = child(&mut f, &r, "GONE", Some("0"), &[]);
    f.add(&gone, kb::IS_DELETED, boolean("true"));
    // Below the deleted node, and unchecked: its unlisted predicate is no violation.
    let under = child(&mut f, &gone, "UNDER", Some("0"), &[]);
    f.add(&under, "http://purl.org/dc/terms/title", Literal::new_simple_literal("x"));
    let kept = child(&mut f, &r, "KEPT", Some("1"), &[]);

    let dump = f.dump();
    assert_eq!(
        output(&dump),
        file(vec![concept(&r), concept(&kept), broader(&kept, &r), position(&kept, "1"),])
    );
    let mut stats = Stats::default();
    build(&dump, &mut stats).unwrap();
    assert_eq!(stats.omitted.get(DELETED_LIST_NODE), Some(&2));
    assert_eq!(stats.list_nodes, 2);
}

#[test]
fn each_breach_of_the_list_tree_is_a_violation() {
    let mut f = Fixture::new();
    let mut expected = Vec::new();

    let same_tag = root(&mut f, "SAMETAG", &[tagged("a", "de"), tagged("b", "de")]);
    expected.push((Rule::DuplicateListLabel, same_tag, "de".to_string()));
    let untagged = [Literal::new_simple_literal("a"), Literal::new_simple_literal("b")];
    let untagged = root(&mut f, "UNTAGGED", &untagged);
    expected.push((Rule::DuplicateListLabel, untagged, String::new()));
    let iri_label = root(&mut f, "IRILABEL", &[]);
    f.add(&iri_label, model::RDFS_LABEL, nn("http://example.org/label"));
    expected.push((Rule::InvalidListLabel, iri_label, "<http://example.org/label>".to_string()));

    let (p1, p2) = (root(&mut f, "P1", &[]), root(&mut f, "P2", &[]));
    let twice = child(&mut f, &p1, "TWICE", None, &[]);
    f.add(&p2, kb::HAS_SUB_LIST_NODE, nn(&twice));
    expected.push((Rule::MultipleParents, twice.clone(), p1));
    expected.push((Rule::MultipleParents, twice, p2));

    let placed = root(&mut f, "PLACED", &[]);
    f.add(&placed, kb::LIST_NODE_POSITION, integer("0"));
    expected.push((Rule::RootWithPosition, placed, kb::LIST_NODE_POSITION.to_string()));

    let p3 = root(&mut f, "P3", &[]);
    let huge = child(&mut f, &p3, "HUGE", Some("4294967296"), &[]);
    expected.push((Rule::InvalidListPosition, huge, kb::LIST_NODE_POSITION.to_string()));

    let p4 = root(&mut f, "P4", &[]);
    for id in ["SIB2", "SIB1"] {
        let sibling = child(&mut f, &p4, id, Some("0"), &[]);
        expected.push((Rule::DuplicateSiblingPosition, sibling, "0".to_string()));
    }
    // The positive canary: a third sibling at its own position.
    child(&mut f, &p4, "SIB3", Some("1"), &[]);

    let x = node(&mut f, "CYCX", &[]);
    let y = child(&mut f, &x, "CYCY", None, &[]);
    f.add(&y, kb::HAS_SUB_LIST_NODE, nn(&x));
    expected.push((Rule::ListNodeCycle, x, String::new()));
    expected.push((Rule::ListNodeCycle, y, String::new()));

    let orphan = node(&mut f, "ORPHAN", &[]);
    expected.push((Rule::OrphanListNode, orphan, String::new()));

    let p5 = root(&mut f, "P5", &[]);
    let adopted = root(&mut f, "ADOPTED", &[]);
    f.add(&p5, kb::HAS_SUB_LIST_NODE, nn(&adopted));
    expected.push((Rule::RootWithParent, adopted, p5));

    let p6 = root(&mut f, "P6", &[]);
    f.add(&p6, kb::HAS_SUB_LIST_NODE, nn("http://rdfh.ch/0803/NOTALIST"));
    expected.push((Rule::DanglingSubListNode, p6, "<http://rdfh.ch/0803/NOTALIST>".to_string()));

    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

/// A Page carrying one live list value naming `target`; returns the value's IRI.
fn page_with_list_value(f: &mut Fixture, id: &str, target: &str) -> String {
    let v = page_with_bare_list_value(f, id);
    f.add(&v, kb::VALUE_HAS_LIST_NODE, nn(target));
    v
}

/// A Page carrying one live list value with no `kb:valueHasListNode` yet.
fn page_with_bare_list_value(f: &mut Fixture, id: &str) -> String {
    let v = format!("{ZEBRA}/values/{id}");
    f.add(ZEBRA, &onto("hasOffice"), nn(&v));
    f.add(&v, model::RDF_TYPE, nn(kb::LIST_VALUE));
    f.add(&v, kb::VALUE_HAS_UUID, Literal::new_simple_literal(id));
    f.add(&v, kb::IS_DELETED, boolean("false"));
    f.add(&v, kb::VALUE_HAS_STRING, Literal::new_simple_literal(id));
    v
}

#[test]
fn a_list_value_emits_dao_source_list_node_and_no_rdf_value() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    let r = root(&mut f, "ROOT", &[]);
    let c = child(&mut f, &r, "C", Some("0"), &[tagged("Basel", "de")]);
    page_with_list_value(&mut f, "LV", &c);

    let (p, n) = (onto("hasOffice"), "urn:dsp:value:LV");
    let mut expected = resource_lines(ZEBRA, &onto("Page"), "b7v");
    expected.extend([
        concept(&r),
        concept(&c),
        broader(&c, &r),
        position(&c, "0"),
        pref_label(&c, &tagged("Basel", "de")),
        line(ZEBRA, &p, &format!("<{n}>")),
        line(n, model::RDF_TYPE, &format!("<{}>", vocab::DAO_VALUE)),
        line(n, vocab::DAO_VALUE_HAS_UUID, &Literal::new_simple_literal("LV").to_string()),
        line(n, vocab::DAO_SOURCE_PROPERTY, &format!("<{p}>")),
        line(n, vocab::DAO_SOURCE_LIST_NODE, &format!("<{c}>")),
    ]);
    let out = output(&f.dump());
    assert_eq!(out, file(expected));
    assert!(!out.contains(model::RDF_VALUE), "{out}");
}

#[test]
fn a_list_value_naming_a_deleted_node_or_one_in_a_deleted_subtree_is_a_violation() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    let r = root(&mut f, "ROOT", &[]);
    let gone = child(&mut f, &r, "GONE", Some("0"), &[]);
    f.add(&gone, kb::IS_DELETED, boolean("true"));
    let under = child(&mut f, &gone, "UNDER", Some("0"), &[]);
    let kept = child(&mut f, &r, "KEPT", Some("1"), &[]);
    let at_gone = page_with_list_value(&mut f, "AT-GONE", &gone);
    let at_under = page_with_list_value(&mut f, "AT-UNDER", &under);
    // The positive canary: a value naming a live node is no violation.
    page_with_list_value(&mut f, "AT-KEPT", &kept);

    let mut expected = vec![
        (Rule::ListValueTarget, at_gone, gone),
        (Rule::ListValueTarget, at_under, under),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_list_value_naming_a_node_that_is_not_in_the_dump_is_a_violation() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    let missing = iri("MISSING");
    let v = page_with_list_value(&mut f, "LV", &missing);

    assert_eq!(report(&f.dump()), vec![(Rule::ListValueTarget, v, missing)]);
}

#[test]
fn a_list_value_with_no_one_list_node_iri_is_a_violation() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    let r = root(&mut f, "ROOT", &[]);
    let p = kb::VALUE_HAS_LIST_NODE;
    let none = page_with_bare_list_value(&mut f, "NONE");
    let literal = page_with_bare_list_value(&mut f, "LITERAL");
    f.add(&literal, p, Literal::new_simple_literal("Basel"));
    let twice = page_with_list_value(&mut f, "TWICE", &r);
    f.add(&twice, p, nn(&iri("OTHER")));

    let mut expected = vec![
        (Rule::MissingSourceLiteral, none, p.to_string()),
        (Rule::InvalidLiteral, literal, p.to_string()),
        (Rule::RepeatedSourceLiteral, twice, p.to_string()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_cycle_through_a_deleted_node_is_dropped_with_it_and_not_reported() {
    let mut f = Fixture::new();
    let r = root(&mut f, "ROOT", &[]);
    // X ⇄ Y, and X is deleted: Y is below X, so the whole cycle is dropped.
    let x = node(&mut f, "CYCX", &[]);
    f.add(&x, kb::IS_DELETED, boolean("true"));
    let y = child(&mut f, &x, "CYCY", None, &[]);
    f.add(&y, kb::HAS_SUB_LIST_NODE, nn(&x));

    let dump = f.dump();
    assert_eq!(output(&dump), file(vec![concept(&r)]));
    let mut stats = Stats::default();
    build(&dump, &mut stats).unwrap();
    assert_eq!(stats.omitted.get(DELETED_LIST_NODE), Some(&2));
}

#[test]
fn a_node_named_by_a_live_and_a_deleted_parent_is_a_violation_not_silently_dropped() {
    let mut f = Fixture::new();
    let live = root(&mut f, "LIVE", &[]);
    let gone = root(&mut f, "GONE", &[]);
    f.add(&gone, kb::IS_DELETED, boolean("true"));
    let shared = child(&mut f, &live, "SHARED", Some("0"), &[]);
    f.add(&gone, kb::HAS_SUB_LIST_NODE, nn(&shared));

    let mut expected = vec![
        (Rule::MultipleParents, shared.clone(), live),
        (Rule::MultipleParents, shared, gone),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}
