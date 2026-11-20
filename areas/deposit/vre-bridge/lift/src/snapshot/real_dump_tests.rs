//! Assertions against the pinned 0803 stage dump (2026-11-20, DSP-API v40.7.0,
//! every formatted text value with its `kb:valueHasXml`).
//! Every test is `#[ignore]`d, so CI never needs the dump; run them with
//! `just vre-bridge-lift-real-dump <absolute dump dir> <reference snapshot>`, which
//! sets `VRE_BRIDGE_LIFT_DUMP` and `VRE_BRIDGE_LIFT_REFERENCE`.
//! An unset variable panics: these tests never pass without a dump.
//!
//! The tests parse the emitted N-Quads, never the emitter's own structures, and
//! compare full IRIs written out here, so they check the file a consumer reads.
//! The expected numbers mirror the plan's Success Metrics, derived from `data.nq`:
//! on a mismatch re-derive them there, never copy them from the output.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{Literal, NamedNode, NamedOrBlankNode, Quad, Term};

use super::vocab;
use crate::LiftError;

const DAO: &str = "https://ontology.dasch.swiss/dao#";
const OA: &str = "http://www.w3.org/ns/oa#";
const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const KB: &str = "http://www.knora.org/ontology/knora-base#";
const INC: &str = "http://www.knora.org/ontology/0803/incunabula#";
const RES: &str = "http://rdfh.ch/0803/";
const VALUE: &str = "urn:dsp:value:";
const GRAPH: &str = "urn:dsp:project:0803";

fn iri(prefix: &str, local: &str) -> String {
    format!("{prefix}{local}")
}

/// The dump's snapshot bytes, lifted in this process from `VRE_BRIDGE_LIFT_DUMP`.
/// A refusal prints every violation of the public guard before failing.
fn emit() -> Vec<u8> {
    let dir = std::env::var("VRE_BRIDGE_LIFT_DUMP")
        .expect("VRE_BRIDGE_LIFT_DUMP must name an unzipped dump directory (absolute path)");
    let lifted = match crate::lift(Path::new(&dir)) {
        Ok(lifted) => lifted,
        Err(err @ LiftError::Refused { .. }) => {
            println!("{err}");
            panic!("the public guard refused the dump; its violations are printed above");
        }
        Err(err) => panic!("the dump gives no snapshot: {err}"),
    };
    let mut bytes = Vec::new();
    lifted.snapshot.write_to(&mut bytes).expect("writing to memory");
    bytes
}

/// The parsed snapshot, indexed by subject. Shared by every test in the run, so
/// the dump is loaded once.
struct Parsed {
    bytes: Vec<u8>,
    quads: Vec<Quad>,
    by_subject: HashMap<String, Vec<(String, Term)>>,
}

fn snapshot() -> &'static Parsed {
    static PARSED: OnceLock<Parsed> = OnceLock::new();
    PARSED.get_or_init(|| {
        let bytes = emit();
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NQuads)
            .for_slice(&bytes)
            .collect::<Result<_, _>>()
            .expect("the snapshot parses as N-Quads");
        let mut by_subject: HashMap<String, Vec<(String, Term)>> = HashMap::new();
        for q in &quads {
            by_subject
                .entry(q.subject.to_string())
                .or_default()
                .push((q.predicate.as_str().to_string(), q.object.clone()));
        }
        Parsed { bytes, quads, by_subject }
    })
}

/// A subject's key in [`Parsed::by_subject`]: an IRI in angle brackets.
fn key(iri: &str) -> String {
    format!("<{iri}>")
}

impl Parsed {
    fn objects(&self, s: &str, p: &str) -> Vec<&Term> {
        self.by_subject
            .get(&key(s))
            .into_iter()
            .flatten()
            .filter(|(pred, _)| pred == p)
            .map(|(_, o)| o)
            .collect()
    }

    fn one(&self, s: &str, p: &str) -> &Term {
        let objects = self.objects(s, p);
        assert_eq!(objects.len(), 1, "<{s}> <{p}>: {objects:?}");
        objects[0]
    }

    fn one_iri(&self, s: &str, p: &str) -> &str {
        match self.one(s, p) {
            Term::NamedNode(n) => n.as_str(),
            other => panic!("<{s}> <{p}> is not an IRI: {other}"),
        }
    }

    fn one_literal(&self, s: &str, p: &str) -> &Literal {
        match self.one(s, p) {
            Term::Literal(l) => l,
            other => panic!("<{s}> <{p}> is not a literal: {other}"),
        }
    }

    /// Every IRI subject with `rdf:type <class>`.
    fn typed(&self, class: &str) -> BTreeSet<&str> {
        self.with(&iri(RDF, "type"), class)
    }

    /// Every IRI subject with `<p> <o>`, `o` an IRI.
    fn with(&self, p: &str, o: &str) -> BTreeSet<&str> {
        self.quads
            .iter()
            .filter(|q| q.predicate.as_str() == p)
            .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == o))
            .filter_map(|q| match &q.subject {
                NamedOrBlankNode::NamedNode(n) => Some(n.as_str()),
                NamedOrBlankNode::BlankNode(_) => None,
            })
            .collect()
    }

    fn count(&self, p: &str) -> usize {
        self.quads.iter().filter(|q| q.predicate.as_str() == p).count()
    }

    fn of_class(&self, local: &str) -> BTreeSet<&str> {
        self.with(&iri(DAO, "sourceClass"), &iri(INC, local))
    }

    fn resources(&self) -> BTreeSet<&str> {
        self.typed(&iri(DAO, "Resource"))
    }

    /// §3.1's link row: a resource subject, a predicate outside `dao:`, `oa:`,
    /// `rdf:` and `rdfs:`, and an IRI object that is not a value node.
    fn links(&self) -> Vec<(&str, &str)> {
        let resources = self.resources();
        let values = self.typed(&iri(DAO, "Value"));
        self.quads
            .iter()
            .filter(|q| matches!(&q.subject, NamedOrBlankNode::NamedNode(s) if resources.contains(s.as_str())))
            .filter(|q| {
                let p = q.predicate.as_str();
                ![DAO, OA, RDF, RDFS].iter().any(|ns| p.starts_with(ns))
            })
            .filter_map(|q| match &q.object {
                Term::NamedNode(o) if !values.contains(o.as_str()) => Some((q.predicate.as_str(), o.as_str())),
                _ => None,
            })
            .collect()
    }
}

fn plain(v: &str) -> Term {
    Term::Literal(Literal::new_simple_literal(v))
}

fn typed(v: &str, datatype: &str) -> Term {
    Term::Literal(Literal::new_typed_literal(v, NamedNode::new_unchecked(datatype)))
}

fn node(iri: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(iri))
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_resources_by_class() {
    let s = snapshot();
    assert_eq!(s.of_class("Book").len(), 19);
    assert_eq!(s.of_class("Page").len(), 4_024);
    assert_eq!(s.of_class("Band").len(), 38);
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_regions_and_link_objects_are_annotations() {
    let s = snapshot();
    let source_class = iri(DAO, "sourceClass");
    let regions = s.with(&source_class, &iri(KB, "Region"));
    let link_objects = s.with(&source_class, &iri(KB, "LinkObj"));
    assert_eq!(regions.len(), 77);
    assert_eq!(link_objects.len(), 40);
    let annotations = s.typed(&iri(OA, "Annotation"));
    let expected: BTreeSet<&str> = regions.union(&link_objects).copied().collect();
    assert_eq!(annotations, expected);
    let motivated_by = iri(OA, "motivatedBy");
    for r in &regions {
        assert_eq!(s.one_iri(r, &motivated_by), iri(OA, "commenting"), "{r}");
    }
    for l in &link_objects {
        assert_eq!(s.one_iri(l, &motivated_by), iri(OA, "linking"), "{l}");
    }
    assert!(s.with(&motivated_by, &iri(OA, "highlighting")).is_empty());
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_superseded_pagenum_gives_the_current_value_under_the_original_uuid() {
    let s = snapshot();
    let page = iri(RES, "-34FYx0jVMGTEi8aItQFxQ");
    let value = iri(VALUE, "U_J3GLDmTJuj24vddKrJMA");
    assert_eq!(s.one_iri(&page, &iri(INC, "hasPagenum")), value);
    assert_eq!(s.one(&value, &iri(RDF, "value")), &plain("a1r; Titelblatt"));
}

/// A `kb:Region` is an annotation (FORMAT.md §10), and it keeps its values.
#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_superseded_region_comment_is_carried_on_an_annotation() {
    let s = snapshot();
    let region = iri(RES, "GOkuI_IxVuSKMRZmCypz7Q");
    let value = iri(VALUE, "xdvQATb3TOeS875AuWi6Rw");
    assert_eq!(s.one_iri(&region, &iri(KB, "hasComment")), value);
    assert_eq!(s.one(&value, &iri(RDF, "type")), &node(&iri(DAO, "Value")));
    s.one_literal(&value, &iri(RDF, "value"));
    let types = s.objects(&region, &iri(RDF, "type"));
    assert!(types.contains(&&node(&iri(OA, "Annotation"))), "{types:?}");
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_no_value_node_is_minted_from_a_value_iri_suffix() {
    let s = snapshot();
    let terms: BTreeSet<String> = s
        .quads
        .iter()
        .flat_map(|q| [q.subject.to_string(), q.object.to_string()])
        .collect();
    for suffix in [
        "8HeOpYC8Tq2BZDFFPHex8A",
        "I0k-VF0PRFCx9oJxomJ9Xw",
        "aL0mQInVRpq8C9U9aS2tgg",
    ] {
        assert!(!terms.contains(&key(&iri(VALUE, suffix))), "{suffix}");
    }
    for uuid in ["U_J3GLDmTJuj24vddKrJMA", "xdvQATb3TOeS875AuWi6Rw"] {
        assert!(terms.contains(&key(&iri(VALUE, uuid))), "{uuid}");
    }
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_book_citations_carry_their_orders() {
    let s = snapshot();
    let book = iri(RES, "70aWaB2kWsuiN6ujYgM0ZQ");
    let mut citations: Vec<(i64, &Term)> = s
        .objects(&book, &iri(INC, "hasCitation"))
        .into_iter()
        .map(|n| match n {
            Term::NamedNode(n) => n.as_str(),
            other => panic!("a citation edge to {other}"),
        })
        .map(|n| {
            let order = s.one_literal(n, &iri(DAO, "valueHasOrder"));
            assert_eq!(order.datatype().as_str(), iri(XSD, "integer"), "{n}");
            let order = order.value().parse().expect("an integer order");
            (order, s.one(n, &iri(RDF, "value")))
        })
        .collect();
    citations.sort_by_key(|(order, _)| *order);
    assert_eq!(
        citations,
        [
            (0, &plain("Schramm Bd. XXI, S. 27")),
            (1, &plain("GW 4168")),
            (2, &plain("ISTC ib00512000")),
        ]
    );
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_book_pubdate_is_a_julian_year() {
    let s = snapshot();
    let book = iri(RES, "70aWaB2kWsuiN6ujYgM0ZQ");
    let date = s.one_iri(&book, &iri(INC, "hasPubdate"));
    let integer = iri(XSD, "integer");
    let expected = [
        ("value", typed("JULIAN:1492 CE", &iri(DAO, "date")), RDF),
        ("dateCalendar", plain("JULIAN"), DAO),
        ("dateStartJDN", typed("2266011", &integer), DAO),
        ("dateEndJDN", typed("2266376", &integer), DAO),
        ("dateStartPrecision", plain("YEAR"), DAO),
        ("dateEndPrecision", plain("YEAR"), DAO),
    ];
    for (local, object, ns) in expected {
        assert_eq!(s.one(date, &iri(ns, local)), &object, "{local}");
    }
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_text_values_drop_the_record_separator() {
    let s = snapshot();
    // The dump writes "[missing]\u{1e}\n        ": only the U+001E goes.
    let value = iri(VALUE, "-UFjZccyRaWal2r0EhOACw");
    assert_eq!(s.one(&value, &iri(RDF, "value")), &plain("[missing]\n        "));
    let rdf_value = iri(RDF, "value");
    let carriers: Vec<&Quad> = s
        .quads
        .iter()
        .filter(|q| q.predicate.as_str() == rdf_value)
        .filter(|q| matches!(&q.object, Term::Literal(l) if l.value().contains('\u{1e}')))
        .collect();
    assert!(carriers.is_empty(), "{carriers:?}");
}

/// The Success Metrics table, row by row; every mismatch is listed at once.
#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_matches_the_success_metrics() {
    let s = snapshot();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for v in s.typed(&iri(DAO, "Value")) {
        let kind = match s.one_literal(v, &iri(RDF, "value")).datatype().as_str() {
            d if d == iri(XSD, "string") || d == iri(RDF, "langString") => "text".to_string(),
            d => d.to_string(),
        };
        *kinds.entry(kind).or_default() += 1;
    }
    let mut links: BTreeMap<String, usize> = BTreeMap::new();
    for (p, _) in s.links() {
        *links.entry(p.to_string()).or_default() += 1;
    }
    let mut actual: BTreeMap<String, usize> = BTreeMap::new();
    let mut put = |k: String, n: usize| actual.insert(k, n);
    put("resources".into(), s.resources().len());
    put("annotations".into(), s.typed(&iri(OA, "Annotation")).len());
    put("value nodes".into(), s.typed(&iri(DAO, "Value")).len());
    for (kind, n) in kinds {
        put(format!("value nodes: {kind}"), n);
    }
    put("dao:valueHasOrder".into(), s.count(&iri(DAO, "valueHasOrder")));
    put("dao:valueHasXml".into(), s.count(&iri(DAO, "valueHasXml")));
    put("representations".into(), s.typed(&iri(DAO, "Representation")).len());
    put("dao:isPartOf".into(), s.count(&iri(DAO, "isPartOf")));
    put("dao:seqnum".into(), s.count(&iri(DAO, "seqnum")));
    put("links".into(), s.links().len());
    for (p, n) in links {
        put(format!("links: {p}"), n);
    }
    put("oa:hasTarget".into(), s.count(&iri(OA, "hasTarget")));
    put("list nodes".into(), s.typed(&iri(SKOS, "Concept")).len());
    put("skos:prefLabel".into(), s.count(&iri(SKOS, "prefLabel")));
    put("quads".into(), s.quads.len());

    let expected: BTreeMap<String, usize> = [
        ("resources".to_string(), 4_198),
        ("annotations".into(), 117),
        ("value nodes".into(), 12_961),
        ("value nodes: text".into(), 12_770),
        (format!("value nodes: {}", iri(DAO, "geometry")), 77),
        (format!("value nodes: {}", iri(DAO, "color")), 77),
        (format!("value nodes: {}", iri(DAO, "date")), 19),
        (format!("value nodes: {}", iri(XSD, "anyURI")), 18),
        ("dao:valueHasOrder".into(), 12_961),
        ("dao:valueHasXml".into(), 47),
        ("representations".into(), 4_062),
        ("dao:isPartOf".into(), 4_024),
        ("dao:seqnum".into(), 4_024),
        ("links".into(), 571),
        (format!("links: {}", iri(INC, "hasBandTypeL")), 207),
        (format!("links: {}", iri(INC, "hasBandTypeR")), 208),
        (format!("links: {}", iri(KB, "hasLinkTo")), 79),
        (format!("links: {}", iri(KB, "isRegionOf")), 77),
        ("oa:hasTarget".into(), 156),
        ("list nodes".into(), 23),
        ("skos:prefLabel".into(), 49),
        // 123,946 facts the incubator also writes + 2 × (4,198 + 4,062) rights lines + 47 XML
        // lines.
        ("quads".into(), 140_513),
    ]
    .into_iter()
    .collect();
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_list_nodes_have_four_roots() {
    let s = snapshot();
    let nodes = s.typed(&iri(SKOS, "Concept"));
    assert_eq!(nodes.len(), 23);
    let roots = nodes.iter().filter(|n| s.objects(n, &iri(SKOS, "broader")).is_empty()).count();
    assert_eq!(roots, 4);
    let broader = iri(SKOS, "broader");
    for n in &nodes {
        for parent in s.objects(n, &broader) {
            let parent = match parent {
                Term::NamedNode(p) => p.as_str(),
                other => panic!("{n} skos:broader {other}"),
            };
            assert!(nodes.contains(parent) && parent != *n, "{n} -> {parent}");
        }
    }
    assert_eq!(s.count(&broader), 19);
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_pages_and_bands_have_one_still_image_and_books_none() {
    let s = snapshot();
    let has_representation = iri(DAO, "hasRepresentation");
    let mut nodes = BTreeSet::new();
    for r in s.of_class("Page").union(&s.of_class("Band")) {
        let rep = s.one_iri(r, &has_representation);
        assert_eq!(s.one(rep, &iri(RDF, "type")), &node(&iri(DAO, "Representation")));
        assert_eq!(
            s.one_iri(rep, &iri(DAO, "representationType")),
            iri(DAO, "StillImageRepresentation"),
            "{rep}"
        );
        assert!(!s.one_literal(rep, &iri(DAO, "internalFilename")).value().is_empty(), "{rep}");
        for dim in ["dimX", "dimY"] {
            let lexical = s.one_literal(rep, &iri(DAO, dim));
            assert_eq!(lexical.datatype().as_str(), iri(XSD, "integer"), "{rep}");
            let n: u32 = lexical.value().parse().expect("a u32 dimension");
            assert!(n > 0, "{rep} {dim}");
        }
        nodes.insert(rep);
    }
    assert_eq!(nodes.len(), 4_062);
    for book in s.of_class("Book") {
        assert!(s.objects(book, &has_representation).is_empty(), "{book}");
    }
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_every_page_is_part_of_one_book_with_one_seqnum() {
    let s = snapshot();
    let books = s.of_class("Book");
    let pages = s.of_class("Page");
    assert_eq!(pages.len(), 4_024);
    for page in pages {
        let parent = s.one_iri(page, &iri(DAO, "isPartOf"));
        assert!(books.contains(parent), "{page} is part of {parent}");
        let seqnum = s.one_literal(page, &iri(DAO, "seqnum"));
        assert_eq!(seqnum.datatype().as_str(), iri(XSD, "integer"), "{page}");
    }
}

/// Two values sharing a UUID would share one node: it would then be the object
/// of two edges, so "one edge per node" is the uniqueness check on the file.
#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_value_uuids_are_unique_and_every_reference_is_a_resource() {
    let s = snapshot();
    let values = s.typed(&iri(DAO, "Value"));
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for q in &s.quads {
        if let Term::NamedNode(o) = &q.object {
            if values.contains(o.as_str()) {
                edges.entry(o.as_str()).or_default().push(q.predicate.as_str());
            }
        }
    }
    for v in &values {
        let uuid = s.one_literal(v, &iri(DAO, "valueHasUUID"));
        assert_eq!(Some(uuid.value()), v.strip_prefix(VALUE), "{v}");
        s.one_literal(v, &iri(DAO, "valueHasOrder"));
        let preds = edges.get(v).map(Vec::as_slice).unwrap_or_default();
        assert_eq!(preds.len(), 1, "{v} is the object of {preds:?}");
        assert_eq!(s.one_iri(v, &iri(DAO, "sourceProperty")), preds[0], "{v}");
    }

    let resources = s.resources();
    let mut references: Vec<(&str, &str)> = s.links();
    for p in [iri(DAO, "isPartOf"), iri(OA, "hasTarget")] {
        for q in s.quads.iter().filter(|q| q.predicate.as_str() == p) {
            match &q.object {
                Term::NamedNode(o) => references.push((q.predicate.as_str(), o.as_str())),
                other => panic!("{q}: object {other} is not an IRI"),
            }
        }
    }
    let dangling: Vec<_> = references.into_iter().filter(|(_, o)| !resources.contains(o)).collect();
    assert!(dangling.is_empty(), "{dangling:?}");
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_has_no_blank_node_and_no_admin_object() {
    let s = snapshot();
    let forbidden = ["rdfh.ch/users/", "rdfh.ch/projects/", "knora.org/data/"];
    assert!(!s.quads.is_empty());
    for q in &s.quads {
        assert!(matches!(&q.subject, NamedOrBlankNode::NamedNode(_)), "blank subject: {q}");
        match &q.object {
            Term::BlankNode(_) => panic!("blank object: {q}"),
            Term::NamedNode(o) => assert!(!forbidden.iter().any(|f| o.as_str().contains(f)), "forbidden object: {q}"),
            Term::Literal(_) => {}
        }
        assert_eq!(q.graph_name.to_string(), key(GRAPH), "{q}");
    }
}

#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_emits_identical_bytes_twice() {
    assert!(snapshot().bytes == emit(), "a second emission differs");
}

/// On the same dump, the snapshot holds the same quads as the one dsp-incubator's
/// dao-lift at 6106ee38166617cd8be5d58411d5bb14741ebc03 wrote
/// (`VRE_BRIDGE_LIFT_REFERENCE`), apart from the access rights, data
/// classification and `dao:valueHasXml` that producer did not write. That
/// producer refuses a dump with `kb:valueHasXml`, so its reference is written
/// from the dump without those quads. Facts, not bytes: the two link different
/// oxrdf versions, which render the lines.
#[test]
#[ignore = "needs VRE_BRIDGE_LIFT_DUMP and VRE_BRIDGE_LIFT_REFERENCE; run via `just vre-bridge-lift-real-dump`"]
fn real_dump_matches_the_incubator_lift() {
    let path = std::env::var("VRE_BRIDGE_LIFT_REFERENCE")
        .expect("VRE_BRIDGE_LIFT_REFERENCE must name the incubator lift's snapshot of the same dump");
    let reference = std::fs::read(&path).expect("the reference snapshot reads");
    let theirs: HashSet<Quad> = RdfParser::from_format(RdfFormat::NQuads)
        .for_slice(&reference)
        .collect::<Result<_, _>>()
        .expect("the reference parses as N-Quads");
    assert!(!theirs.is_empty(), "the reference snapshot is empty");
    let not_theirs = [
        vocab::DAO_ACCESS_RIGHTS,
        vocab::DAO_DATA_CLASSIFICATION,
        vocab::DAO_VALUE_HAS_XML,
    ];
    let ours: HashSet<Quad> = snapshot()
        .quads
        .iter()
        .filter(|q| !not_theirs.contains(&q.predicate.as_str()))
        .cloned()
        .collect();

    let mut only_ours: Vec<String> = ours.difference(&theirs).map(Quad::to_string).collect();
    let mut only_theirs: Vec<String> = theirs.difference(&ours).map(Quad::to_string).collect();
    only_ours.sort();
    only_theirs.sort();
    for q in &only_ours {
        println!("only in this lift: {q}");
    }
    for q in &only_theirs {
        println!("only in the reference: {q}");
    }
    assert!(
        only_ours.is_empty() && only_theirs.is_empty(),
        "{} quad(s) only in this lift, {} only in the reference",
        only_ours.len(),
        only_theirs.len()
    );
}
