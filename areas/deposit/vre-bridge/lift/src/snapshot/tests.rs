//! Fixture tests for the snapshot skeleton. Fixture IRIs are deliberately not in
//! alphabetical insertion order (ZEBRA before APPLE), so no test passes by
//! accident of the store's iteration order.

use oxigraph::io::RdfFormat;
use oxigraph::model::{GraphName, Literal, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;

use super::index::{Index, Ontology};
use super::violations::{Rule, Violations};
use super::*;
use crate::LiftError;
use crate::model::{self, kb, nn};
use crate::test_support::TempDir;

const DATA: &str = "http://www.knora.org/data/0803/incunabula";
const ONTO: &str = "http://www.knora.org/ontology/0803/incunabula";
pub(crate) const ZEBRA: &str = "http://rdfh.ch/0803/ZEBRA";
pub(crate) const APPLE: &str = "http://rdfh.ch/0803/APPLE";
const MANGO: &str = "http://rdfh.ch/0803/MANGO";

pub(crate) fn onto(local: &str) -> String {
    format!("{ONTO}#{local}")
}

/// A dump under construction: quads in insertion order, split into one data and
/// one ontology graph, with Page and Book declared as project classes.
pub(crate) struct Fixture {
    data: String,
    onto: String,
    pub(crate) quads: Vec<Quad>,
}

impl Fixture {
    pub(crate) fn new() -> Fixture {
        Fixture::with_graphs(DATA, ONTO)
    }

    fn with_graphs(data: &str, onto: &str) -> Fixture {
        let mut f = Fixture {
            data: data.to_string(),
            onto: onto.to_string(),
            quads: Vec::new(),
        };
        f.class("Page");
        f.class("Book");
        f
    }

    pub(crate) fn class(&mut self, local: &str) {
        let class = format!("{}#{local}", self.onto);
        self.onto_triple(&class, model::RDF_TYPE, nn(model::OWL_CLASS));
    }

    pub(crate) fn onto_triple(&mut self, s: &str, p: &str, o: impl Into<Term>) {
        let g = GraphName::NamedNode(nn(&self.onto));
        self.quads.push(Quad::new(nn(s), nn(p), o, g));
    }

    pub(crate) fn add(&mut self, s: &str, p: &str, o: impl Into<Term>) {
        let g = GraphName::NamedNode(nn(&self.data));
        self.quads.push(Quad::new(nn(s), nn(p), o, g));
    }

    /// A live resource of project class `local` with one plain label.
    pub(crate) fn resource(&mut self, iri: &str, local: &str, label: &str) {
        let class = format!("{}#{local}", self.onto);
        self.add(iri, model::RDF_TYPE, nn(&class));
        self.add(iri, model::RDFS_LABEL, Literal::new_simple_literal(label));
        self.add(iri, kb::IS_DELETED, boolean("false"));
    }

    fn dump_from(quads: impl Iterator<Item = Quad>) -> Dump {
        let store = Store::new().unwrap();
        for q in quads {
            store.insert(q.as_ref()).unwrap();
        }
        Dump::from_store(store).unwrap()
    }

    pub(crate) fn dump(&self) -> Dump {
        Fixture::dump_from(self.quads.iter().cloned())
    }

    fn dump_reversed(&self) -> Dump {
        Fixture::dump_from(self.quads.iter().rev().cloned())
    }
}

pub(crate) fn boolean(v: &str) -> Literal {
    Literal::new_typed_literal(v, nn(model::XSD_BOOLEAN))
}

/// The graph term every fixture line ends in.
pub(crate) const GRAPH: &str = "<urn:dsp:project:0803>";

pub(crate) fn plain(v: &str) -> Literal {
    Literal::new_simple_literal(v)
}

pub(crate) fn integer(v: &str) -> Literal {
    Literal::new_typed_literal(v, nn(model::XSD_INTEGER))
}

/// The snapshot of the dump's selection, without the guard: fixtures carry no
/// `kb:hasPermissions`, so every resource and representation is written public.
pub(crate) fn build_public(dump: &Dump, stats: &mut Stats) -> Result<Snapshot, LiftError> {
    let read = Read::of(dump)?;
    let selection = crate::selection::select(&read.index, &read.subjects);
    let rights = crate::guard::Rights::all_public(&selection);
    build(read, &selection, &rights, stats)
}

/// The fixture's output and the stats the run gathered.
pub(crate) fn run(f: &Fixture) -> (String, Stats) {
    let mut stats = Stats::default();
    let snapshot = build_public(&f.dump(), &mut stats).unwrap();
    let mut bytes = Vec::new();
    snapshot.write_to(&mut bytes).unwrap();
    (String::from_utf8(bytes).unwrap(), stats)
}

pub(crate) fn output(dump: &Dump) -> String {
    let snapshot = build_public(dump, &mut Stats::default()).unwrap();
    let mut bytes = Vec::new();
    snapshot.write_to(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

pub(crate) fn report(dump: &Dump) -> Vec<(Rule, String, String)> {
    match build_public(dump, &mut Stats::default()) {
        Err(LiftError::ClosedWorld(report)) => report.iter().cloned().collect(),
        Err(other) => panic!("not a violation report: {other}"),
        Ok(_) => panic!("fixture must fail"),
    }
}

pub(crate) fn resource_lines(iri: &str, class: &str, label: &str) -> Vec<String> {
    let g = "<urn:dsp:project:0803>";
    let mut lines = vec![
        format!("<{iri}> <{}> \"{label}\" {g} .", model::RDFS_LABEL),
        format!("<{iri}> <{}> <{}> {g} .", model::RDF_TYPE, vocab::DAO_RESOURCE),
        format!("<{iri}> <{}> <{class}> {g} .", vocab::DAO_SOURCE_CLASS),
    ];
    lines.extend(rights_lines(iri, "public"));
    lines
}

/// The `dao:accessRights` and `dao:dataClassification` lines of a resource or
/// representation node.
pub(crate) fn rights_lines(iri: &str, rights: &str) -> Vec<String> {
    let g = "<urn:dsp:project:0803>";
    vec![
        format!("<{iri}> <{}> \"{rights}\" {g} .", vocab::DAO_ACCESS_RIGHTS),
        format!("<{iri}> <{}> \"unclassified\" {g} .", vocab::DAO_DATA_CLASSIFICATION),
    ]
}

/// A fixture exercising every subject kind and every §6 rule without a violation.
fn every_rule_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.onto_triple(&onto("isPartOfBook"), model::RDFS_SUBPROPERTYOF, nn(kb::IS_PART_OF));
    f.onto_triple(&onto("hasSeqnum"), model::RDFS_SUBPROPERTYOF, nn(kb::SEQNUM));
    // A cycle and a blank-node object: the closure must terminate and skip it.
    f.onto_triple(&onto("hasA"), model::RDFS_SUBPROPERTYOF, nn(&onto("hasB")));
    f.onto_triple(&onto("hasB"), model::RDFS_SUBPROPERTYOF, nn(&onto("hasA")));
    f.onto_triple(&onto("hasA"), model::RDFS_SUBPROPERTYOF, oxigraph::model::BlankNode::default());

    f.resource(ZEBRA, "Page", "b7v");
    f.resource(APPLE, "Book", "Narrenschiff");
    let text = format!("{ZEBRA}/values/TEXT");
    let int = format!("{ZEBRA}/values/INT");
    let lv = format!("{ZEBRA}/values/LV");
    let file = format!("{ZEBRA}/values/FILE");
    let tag = format!("{text}/standoff/1");
    f.add(ZEBRA, &onto("isPartOfBook"), nn(APPLE));
    f.add(ZEBRA, &onto("hasSeqnum"), nn(&int));
    f.add(ZEBRA, kb::HAS_STANDOFF_LINK_TO, nn(APPLE));
    f.add(ZEBRA, &onto("hasBandTypeL"), nn(APPLE));
    f.add(ZEBRA, &onto("hasBandTypeLValue"), nn(&lv));
    f.add(ZEBRA, kb::HAS_STILL_IMAGE_FILE_VALUE, nn(&file));
    f.add(ZEBRA, &onto("hasDescription"), nn(&text));
    f.add(&text, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&text, kb::VALUE_HAS_UUID, Literal::new_simple_literal("TEXT"));
    f.add(&text, kb::VALUE_HAS_STRING, Literal::new_simple_literal("x"));
    f.add(&text, kb::VALUE_HAS_STANDOFF, nn(&tag));
    f.add(
        &tag,
        model::RDF_TYPE,
        nn("http://www.knora.org/ontology/standoff#StandoffRootTag"),
    );
    f.add(&int, model::RDF_TYPE, nn(kb::INT_VALUE));
    f.add(
        &int,
        kb::VALUE_HAS_INTEGER,
        Literal::new_typed_literal("1", nn(model::XSD_INTEGER)),
    );
    let part_lv = format!("{ZEBRA}/values/PARTLV");
    f.add(ZEBRA, &onto("isPartOfBookValue"), nn(&part_lv));
    f.add(&part_lv, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&part_lv, model::RDF_SUBJECT, nn(ZEBRA));
    f.add(&part_lv, model::RDF_PREDICATE, nn(&onto("isPartOfBook")));
    f.add(&part_lv, model::RDF_OBJECT, nn(APPLE));
    f.add(&lv, model::RDF_TYPE, nn(kb::LINK_VALUE));
    f.add(&lv, model::RDF_SUBJECT, nn(ZEBRA));
    f.add(&lv, model::RDF_PREDICATE, nn(&onto("hasBandTypeL")));
    f.add(&lv, model::RDF_OBJECT, nn(APPLE));
    f.add(&file, model::RDF_TYPE, nn(kb::STILL_IMAGE_FILE_VALUE));
    f.add(&file, kb::VALUE_HAS_UUID, Literal::new_simple_literal("FILE"));
    f.add(&file, kb::INTERNAL_FILENAME, Literal::new_simple_literal("f.jp2"));
    for dim in [kb::DIM_X, kb::DIM_Y] {
        f.add(&file, dim, Literal::new_typed_literal("10", nn(model::XSD_INTEGER)));
    }
    f.add(MANGO, model::RDF_TYPE, nn(kb::LIST_NODE));
    f.add(MANGO, kb::IS_ROOT_NODE, boolean("true"));
    f
}

#[test]
fn page_and_book_emit_type_source_class_and_plain_label_in_the_project_graph() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f.resource(APPLE, "Book", "Narrenschiff");

    let mut expected = resource_lines(ZEBRA, &onto("Page"), "b7v");
    expected.extend(resource_lines(APPLE, &onto("Book"), "Narrenschiff"));
    expected.sort();
    let expected: String = expected.iter().map(|l| format!("{l}\n")).collect();
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn the_graph_comes_from_the_data_graph_shortcode() {
    let mut f = Fixture::with_graphs(
        "http://www.knora.org/data/0854/alice",
        "http://www.knora.org/ontology/0854/alice",
    );
    f.resource("http://rdfh.ch/0854/ZEBRA", "Page", "x");

    let out = output(&f.dump());
    assert_eq!(out.lines().count(), 5);
    assert!(out.lines().all(|l| l.ends_with(" <urn:dsp:project:0854> .")), "{out}");
}

#[test]
fn is_deleted_counts_only_an_xsd_boolean_true() {
    let mut f = Fixture::new();
    f.add(ZEBRA, model::RDF_TYPE, nn(&onto("Page")));
    f.add(ZEBRA, model::RDFS_LABEL, Literal::new_simple_literal("one"));
    f.add(ZEBRA, kb::IS_DELETED, boolean("1"));
    // A plain "true" is not an xsd:boolean, so it deletes nothing.
    f.resource(APPLE, "Book", "kept");
    f.add(APPLE, kb::IS_DELETED, Literal::new_simple_literal("true"));

    let out = output(&f.dump());
    assert!(!out.contains(ZEBRA), "{out}");
    assert!(out.contains(APPLE), "{out}");
}

#[test]
fn a_blank_node_in_the_data_graph_is_reported_the_same_on_every_load() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f.add(ZEBRA, &onto("hasNote"), oxigraph::model::BlankNode::default());
    f.add(APPLE, model::RDFS_LABEL, Literal::new_simple_literal("x"));
    let blank_subject = Quad::new(
        oxigraph::model::BlankNode::default(),
        nn(model::RDFS_LABEL),
        Literal::new_simple_literal("x"),
        GraphName::NamedNode(nn(DATA)),
    );
    f.quads.push(blank_subject);

    let first = report(&f.dump());
    assert_eq!(first, report(&f.dump_reversed()));
    assert!(first.contains(&(Rule::BlankNode, ZEBRA.to_string(), onto("hasNote"))));
    assert!(first.contains(&(Rule::BlankNode, "_:".to_string(), model::RDFS_LABEL.to_string())));
}

#[test]
fn a_deleted_resource_emits_nothing() {
    let mut f = Fixture::new();
    f.resource(APPLE, "Book", "kept");
    f.add(ZEBRA, model::RDF_TYPE, nn(&onto("Page")));
    f.add(ZEBRA, model::RDFS_LABEL, Literal::new_simple_literal("gone"));
    f.add(ZEBRA, kb::IS_DELETED, boolean("true"));

    let out = output(&f.dump());
    assert!(!out.contains(ZEBRA), "{out}");
    assert!(out.contains(APPLE));
}

#[test]
fn admin_triples_emit_nothing() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f.add(ZEBRA, kb::ATTACHED_TO_USER, nn("http://rdfh.ch/users/U"));
    f.add(ZEBRA, kb::ATTACHED_TO_PROJECT, nn("http://rdfh.ch/projects/P"));
    f.add(
        ZEBRA,
        kb::HAS_PERMISSIONS,
        Literal::new_simple_literal("V knora-admin:UnknownUser"),
    );
    let date = nn("http://www.w3.org/2001/XMLSchema#dateTime");
    f.add(
        ZEBRA,
        kb::CREATION_DATE,
        Literal::new_typed_literal("2011-04-14T07:15:28Z", date.clone()),
    );
    f.add(
        ZEBRA,
        kb::LAST_MODIFICATION_DATE,
        Literal::new_typed_literal("2012-01-01T00:00:00Z", date),
    );

    let mut expected = resource_lines(ZEBRA, &onto("Page"), "b7v");
    expected.sort();
    let expected: String = expected.iter().map(|l| format!("{l}\n")).collect();
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn a_missing_repeated_or_tagged_label_is_a_violation_naming_the_resource() {
    let mut f = Fixture::new();
    f.add(ZEBRA, model::RDF_TYPE, nn(&onto("Page")));
    f.add(APPLE, model::RDF_TYPE, nn(&onto("Page")));
    f.add(APPLE, model::RDFS_LABEL, Literal::new_simple_literal("one"));
    f.add(APPLE, model::RDFS_LABEL, Literal::new_simple_literal("two"));
    f.add(MANGO, model::RDF_TYPE, nn(&onto("Page")));
    let tagged = Literal::new_language_tagged_literal("Seite", "de").unwrap();
    f.add(MANGO, model::RDFS_LABEL, tagged);

    let rules: Vec<(Rule, String)> = report(&f.dump()).into_iter().map(|(r, s, _)| (r, s)).collect();
    assert_eq!(
        rules,
        vec![
            (Rule::MissingLabel, ZEBRA.to_string()),
            (Rule::RepeatedLabel, APPLE.to_string()),
            (Rule::NonPlainLabel, MANGO.to_string()),
        ]
    );
}

#[test]
fn a_class_that_is_neither_a_project_nor_a_known_kb_class_is_a_violation_naming_it() {
    let mut f = Fixture::new();
    f.add(ZEBRA, model::RDF_TYPE, nn("http://example.org/Unicorn"));
    // In the project namespace, but not declared in the ontology.
    f.add(APPLE, model::RDF_TYPE, nn(&onto("Undeclared")));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::UnknownClass, APPLE.to_string(), onto("Undeclared")),
            (Rule::UnknownClass, ZEBRA.to_string(), "http://example.org/Unicorn".to_string()),
        ]
    );
}

#[test]
fn a_resource_level_predicate_the_emitter_does_not_know_is_a_violation_naming_it() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f.add(ZEBRA, "http://purl.org/dc/terms/title", Literal::new_simple_literal("x"));
    f.add(ZEBRA, kb::VALUE_HAS_STRING, Literal::new_simple_literal("x"));
    // A project predicate with a literal object fits no §6 rule either.
    f.add(ZEBRA, &onto("hasTitle"), Literal::new_simple_literal("x"));

    let mut expected = vec![
        (
            Rule::UnknownPredicate,
            ZEBRA.to_string(),
            "http://purl.org/dc/terms/title".to_string(),
        ),
        (Rule::UnknownPredicate, ZEBRA.to_string(), kb::VALUE_HAS_STRING.to_string()),
        (Rule::UnknownPredicate, ZEBRA.to_string(), onto("hasTitle")),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn three_different_violations_fail_the_run_once_listing_all_sorted() {
    let mut f = Fixture::new();
    f.add(ZEBRA, model::RDF_TYPE, nn(&onto("Page")));
    f.resource(APPLE, "Book", "b");
    f.add(APPLE, "http://purl.org/dc/terms/title", Literal::new_simple_literal("x"));
    f.add(MANGO, model::RDF_TYPE, nn("http://example.org/Unicorn"));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::UnknownClass, MANGO.to_string(), "http://example.org/Unicorn".to_string()),
            (
                Rule::UnknownPredicate,
                APPLE.to_string(),
                "http://purl.org/dc/terms/title".to_string()
            ),
            (Rule::MissingLabel, ZEBRA.to_string(), model::RDFS_LABEL.to_string()),
        ]
    );
}

#[test]
fn the_report_prints_a_count_per_rule_then_at_most_twenty_of_each() {
    let mut v = Violations::default();
    for i in (0..25).rev() {
        v.record(Rule::UnknownPredicate, &format!("s{i:02}"), "p");
    }
    v.record(Rule::MissingLabel, "m2", "");
    v.record(Rule::MissingLabel, "m1", "");
    let report = v.into_result().expect_err("violations were recorded");

    assert_eq!(report.iter().count(), 27);
    let mut expected = String::from(
        "snapshot: 27 violation(s), no snapshot (FORMAT.md §11)\n\
         \x20 unknown-predicate (FORMAT.md §6): 25\n\
         \x20 missing-label (FORMAT.md §2): 2\n\
         unknown-predicate (FORMAT.md §6):\n",
    );
    for i in 0..20 {
        expected.push_str(&format!("  s{i:02} p\n"));
    }
    expected.push_str("  … and 5 more\nmissing-label (FORMAT.md §2):\n  m1\n  m2\n");
    assert_eq!(report.to_string(), expected);
    assert_eq!(format!("{report:?}"), expected);
}

/// `f` written as an unzipped dump under `dir`: every quad in `data.nq`.
pub(crate) fn write_dump(f: &Fixture, dir: &TempDir) {
    let rdf = dir.path().join("data").join("rdf");
    std::fs::create_dir_all(&rdf).unwrap();
    let lines: String = f.quads.iter().map(|q| format!("{q} .\n")).collect();
    std::fs::write(rdf.join("data.nq"), lines).unwrap();
}

#[test]
fn lift_reads_the_dump_directory_and_keeps_the_runs_stats() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f.add(ZEBRA, kb::HAS_PERMISSIONS, plain("V knora-admin:UnknownUser"));
    let dir = TempDir::new();
    write_dump(&f, &dir);

    let lifted = crate::lift(dir.path()).unwrap();

    let mut bytes = Vec::new();
    lifted.snapshot.write_to(&mut bytes).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), output(&f.dump()));
    assert_eq!(lifted.stats.resources_by_class.get(&onto("Page")), Some(&1));
    assert_eq!(lifted.stats.lines, 5);
}

#[test]
fn lift_of_a_dump_that_breaks_the_format_is_its_violation_report() {
    let mut bad = Fixture::new();
    bad.add(ZEBRA, model::RDF_TYPE, nn(&onto("Page")));
    bad.add(ZEBRA, kb::HAS_PERMISSIONS, plain("V knora-admin:UnknownUser"));
    let dir = TempDir::new();
    write_dump(&bad, &dir);

    let err = crate::lift(dir.path()).expect_err("a dump without a label must fail");

    assert!(matches!(err, LiftError::ClosedWorld(_)), "{err}");
}

#[test]
fn lift_of_a_directory_without_rdf_files_is_a_dump_error() {
    let dir = TempDir::new();

    let err = crate::lift(dir.path()).expect_err("an empty directory must fail");

    assert!(matches!(err, LiftError::Dump(_)), "{err}");
}

#[test]
fn the_same_fixture_twice_gives_byte_identical_output() {
    let f = every_rule_fixture();
    assert_eq!(output(&f.dump()), output(&f.dump()));
}

#[test]
fn reversed_insertion_order_gives_byte_identical_output() {
    let f = every_rule_fixture();
    let forward = output(&f.dump());
    assert!(!forward.is_empty());
    assert_eq!(output(&f.dump_reversed()), forward);
}

#[test]
fn lines_are_strictly_ascending_bytewise_and_newline_terminated() {
    let out = output(&every_rule_fixture().dump());
    assert!(out.ends_with('\n'));
    assert!(!out.contains('\r'));
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() > 3);
    assert!(lines.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes()), "{out}");
}

#[test]
fn the_output_parses_as_n_quads_without_blank_nodes() {
    let out = output(&every_rule_fixture().dump());
    let store = Store::new().unwrap();
    store.load_from_slice(RdfFormat::NQuads, out.as_bytes()).unwrap();

    assert_eq!(store.len().unwrap(), out.lines().count());
    for q in store.iter() {
        let q = q.unwrap();
        assert!(matches!(q.subject, NamedOrBlankNode::NamedNode(_)));
        assert!(!matches!(q.object, Term::BlankNode(_)));
        assert_eq!(q.graph_name, GraphName::NamedNode(nn("urn:dsp:project:0803")));
    }
}

#[test]
fn a_label_with_control_characters_is_escaped_and_round_trips() {
    let label = "a\u{1E}b\r\t\u{8}\u{C}\"c\\d";
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", label);

    let out = output(&f.dump());
    let rendered = "\"a\\u001Eb\\r\\t\\b\\f\\\"c\\\\d\"";
    let line = out.lines().find(|l| l.contains(model::RDFS_LABEL)).expect("a label line");
    assert!(line.contains(rendered), "{line}");

    let store = Store::new().unwrap();
    store.load_from_slice(RdfFormat::NQuads, line.as_bytes()).unwrap();
    let parsed = store.iter().next().unwrap().unwrap();
    let Term::Literal(literal) = parsed.object else {
        panic!("the label is a literal");
    };
    assert_eq!(literal.value(), label);
    assert_eq!(literal.to_string(), rendered);
}

#[test]
fn an_untyped_subject_and_a_twice_typed_resource_are_violations() {
    let mut f = Fixture::new();
    f.add(ZEBRA, model::RDFS_LABEL, Literal::new_simple_literal("no type"));
    f.resource(APPLE, "Book", "b");
    f.add(APPLE, model::RDF_TYPE, nn(&onto("Page")));

    let rules: Vec<(Rule, String)> = report(&f.dump()).into_iter().map(|(r, s, _)| (r, s)).collect();
    assert_eq!(
        rules,
        vec![
            (Rule::UntypedSubject, ZEBRA.to_string()),
            (Rule::MultipleTypes, APPLE.to_string()),
        ]
    );
}

#[test]
fn every_subject_kind_and_rule_classifies_without_a_violation() {
    let dump = every_rule_fixture().dump();
    let mut stats = Stats::default();
    build_public(&dump, &mut stats).unwrap();

    assert_eq!(
        stats.subjects_by_kind.into_iter().collect::<Vec<_>>(),
        vec![
            (Kind::Resource, 2),
            (Kind::Value, 2),
            (Kind::FileValue, 1),
            (Kind::LinkValue, 2),
            (Kind::ListNode, 1),
            (Kind::Standoff, 1),
        ]
    );
    let handled = |h| stats.resource_predicates.get(&h).copied().unwrap_or(0);
    assert_eq!(handled(Handling::Membership), 1);
    assert_eq!(handled(Handling::Seqnum), 1);
    assert_eq!(handled(Handling::StandoffLink), 1);
    assert_eq!(handled(Handling::Link), 1);
    assert_eq!(handled(Handling::Value), 1);
}

#[test]
fn a_link_value_companion_and_a_file_value_never_reach_the_value_path() {
    let mut f = every_rule_fixture();
    // Drop the one real value edge, so any Value count comes from misclassification.
    f.quads.retain(|q| q.predicate.as_str() != onto("hasDescription"));
    let dump = f.dump();
    let mut stats = Stats::default();
    let snapshot = build_public(&dump, &mut stats).unwrap();

    let handled = |h| stats.resource_predicates.get(&h).copied().unwrap_or(0);
    assert_eq!(handled(Handling::LinkValueCompanion), 2);
    assert_eq!(handled(Handling::FileValue), 1);
    assert_eq!(handled(Handling::Value), 0);
    let mut bytes = Vec::new();
    snapshot.write_to(&mut bytes).unwrap();
    let out = String::from_utf8(bytes).unwrap();
    assert!(!out.contains("hasBandTypeLValue"), "{out}");
    assert!(!out.contains(kb::HAS_STILL_IMAGE_FILE_VALUE), "{out}");
}

#[test]
fn a_language_tag_is_lowercased_and_an_invalid_one_is_a_violation() {
    let mut f = Fixture::new();
    // The N-Quads parser already lowercases, so only this path can carry `DE`.
    let upper = Literal::new_language_tagged_literal_unchecked("Seite", "DE");
    f.add(ZEBRA, &onto("hasLabel"), upper);
    let dump = f.dump();
    let mut violations = Violations::default();
    let index = Index::build(&dump, &mut violations).unwrap();
    let entry = index.entries().next().expect("the fixture subject");
    let Term::Literal(stored) = &entry.triples[0].1 else {
        panic!("a literal object");
    };
    let tag = stored.language().expect("a tagged literal");

    let lit = mint::lang_literal(stored.value(), tag, ZEBRA, &mut violations).unwrap();
    assert_eq!(lit.to_string(), "\"Seite\"@de");
    assert!(mint::lang_literal("x", "de_CH", ZEBRA, &mut violations).is_none());
    let report = violations.into_result().expect_err("de_CH is invalid");
    assert_eq!(
        report.iter().cloned().collect::<Vec<_>>(),
        vec![(Rule::InvalidLanguageTag, ZEBRA.to_string(), "de_CH".to_string())]
    );
}

#[test]
fn a_uuid_that_is_not_base64url_mints_no_node() {
    let mut violations = Violations::default();
    let ok = mint::minted_node("urn:dsp:value:", "EDo-E58n_Rcq", ZEBRA, &mut violations);
    assert_eq!(ok.unwrap().as_str(), "urn:dsp:value:EDo-E58n_Rcq");
    assert!(mint::minted_node("urn:dsp:value:", "", ZEBRA, &mut violations).is_none());
    assert!(mint::minted_node("urn:dsp:value:", "a#b", ZEBRA, &mut violations).is_none());
    let report = violations.into_result().expect_err("two invalid UUIDs");
    assert_eq!(report.iter().count(), 2);
}

#[test]
fn a_selected_resource_without_rights_from_the_guard_is_an_internal_error_not_public() {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    let dump = f.dump();
    let read = Read::of(&dump).unwrap();
    let selection = crate::selection::select(&read.index, &read.subjects);
    let (no_rights, violations) = crate::guard::check(&read.index, &read.subjects, &selection);
    assert_eq!(violations.len(), 1, "the fixture's resource has no permissions");

    let err = build(read, &selection, &no_rights, &mut Stats::default()).expect_err("no rights");

    assert!(matches!(err, LiftError::Internal { .. }), "{err}");
}

#[test]
fn a_shortcode_that_forms_no_graph_iri_is_an_error_not_a_panic() {
    let mut f = Fixture::with_graphs(
        "http://www.knora.org/data/08 03/incunabula",
        "http://www.knora.org/ontology/0803/incunabula",
    );
    f.resource(ZEBRA, "Page", "b7v");
    let err = build_public(&f.dump(), &mut Stats::default()).expect_err("no graph IRI");
    assert!(err.to_string().contains("08 03"), "{err}");
}

#[test]
fn the_ontology_closures_are_reflexive_and_follow_sub_properties() {
    let dump = every_rule_fixture().dump();
    let ontology = Ontology::build(&dump).unwrap();
    assert!(ontology.sub_property.reaches(kb::IS_PART_OF, kb::IS_PART_OF));
    assert!(ontology.sub_property.reaches(&onto("isPartOfBook"), kb::IS_PART_OF));
    assert!(!ontology.sub_property.reaches(&onto("hasA"), kb::IS_PART_OF));
    assert!(ontology.is_resource_class(kb::REGION));
    assert!(ontology.is_resource_class(&onto("Page")));
    assert!(!ontology.is_resource_class(&onto("Undeclared")));
}
