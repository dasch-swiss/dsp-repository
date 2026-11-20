//! The one read of the store: the data graph as a sorted per-subject index, and
//! the ontology facts the rules need. Every later step reads these, never the
//! store, so no output or "first X" choice depends on Oxigraph's iteration order.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use oxigraph::model::{GraphNameRef, NamedNode, NamedOrBlankNode, Term};

use super::violations::{Rule, Violations};
use crate::dump::Dump;
use crate::model::{self, kb, nn};

/// One data-graph subject and its `(predicate, object)` pairs, sorted.
pub(crate) struct Entry {
    pub(crate) node: NamedNode,
    pub(crate) triples: Vec<(NamedNode, Term)>,
}

/// The data graph keyed by subject IRI. Blank nodes are recorded as violations
/// and left out (§1).
pub(crate) struct Index(BTreeMap<String, Entry>);

impl Index {
    pub(crate) fn build(dump: &Dump, violations: &mut Violations) -> Result<Index, Box<dyn Error>> {
        let mut subjects: BTreeMap<String, Entry> = BTreeMap::new();
        let graph = GraphNameRef::NamedNode(dump.data_graph.as_ref());
        for quad in dump.store.quads_for_pattern(None, None, None, Some(graph)) {
            let quad = quad?;
            let subject = match quad.subject {
                NamedOrBlankNode::NamedNode(n) => n,
                // Loading renames blank nodes at random, so a blank node's id is never
                // written into the report: that would make it differ from run to run.
                NamedOrBlankNode::BlankNode(_) => {
                    violations.record(Rule::BlankNode, "_:", quad.predicate.as_str());
                    continue;
                }
            };
            if let Term::BlankNode(_) = &quad.object {
                violations.record(Rule::BlankNode, subject.as_str(), quad.predicate.as_str());
                continue;
            }
            subjects
                .entry(subject.as_str().to_string())
                .or_insert_with(|| Entry { node: subject, triples: Vec::new() })
                .triples
                .push((quad.predicate, quad.object));
        }
        for entry in subjects.values_mut() {
            entry
                .triples
                .sort_by_cached_key(|(p, o)| (p.as_str().to_string(), o.to_string()));
        }
        Ok(Index(subjects))
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.0.values()
    }

    pub(crate) fn get(&self, iri: &str) -> Option<&Entry> {
        self.0.get(iri)
    }
}

/// A reflexive `*` closure (§6, §10) over one ontology relation, computed once for a
/// fixed set of knora-base seeds: every IRI that reaches a seed, the seed included.
pub(crate) struct Closure(BTreeMap<&'static str, BTreeSet<String>>);

impl Closure {
    fn build(dump: &Dump, relation: &str, seeds: &[&'static str]) -> Result<Closure, Box<dyn Error>> {
        let relation = nn(relation);
        let mut children: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for graph in &dump.ontology_graphs {
            let graph = GraphNameRef::NamedNode(graph.as_ref());
            for quad in dump.store.quads_for_pattern(None, Some(relation.as_ref()), None, Some(graph)) {
                let quad = quad?;
                // Only IRI edges: the ontology graphs also hold blank-node restrictions.
                if let (NamedOrBlankNode::NamedNode(child), Term::NamedNode(parent)) = (&quad.subject, &quad.object) {
                    children
                        .entry(parent.as_str().to_string())
                        .or_default()
                        .insert(child.as_str().to_string());
                }
            }
        }
        Ok(Closure(seeds.iter().map(|&seed| (seed, reachers(&children, seed))).collect()))
    }

    /// Whether `iri` reaches `seed`. `seed` must be one the closure was built for:
    /// any other reaches nothing, and the tests' debug build fails on it.
    pub(crate) fn reaches(&self, iri: &str, seed: &str) -> bool {
        debug_assert!(
            self.0.contains_key(seed),
            "closure queried for a seed it was not built with: {seed}"
        );
        self.0.get(seed).is_some_and(|members| members.contains(iri))
    }
}

/// `seed` and every IRI that reaches it through `children`.
fn reachers(children: &BTreeMap<String, BTreeSet<String>>, seed: &str) -> BTreeSet<String> {
    // The visited set is what makes cycles and diamonds terminate.
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut todo = vec![seed.to_string()];
    while let Some(iri) = todo.pop() {
        if visited.insert(iri.clone()) {
            todo.extend(children.get(&iri).into_iter().flatten().cloned());
        }
    }
    visited
}

/// Class seeds of `rdfs:subClassOf*`: the knora-base classes a project resource may use directly.
const CLASS_SEEDS: &[&str] = &[kb::REGION, kb::LINK_OBJ];
/// Property seeds of `rdfs:subPropertyOf*`: the §6 rules decided by predicate
/// alone, and §10's target properties and comment property.
const PROPERTY_SEEDS: &[&str] = &[
    kb::IS_PART_OF,
    kb::SEQNUM,
    kb::HAS_STANDOFF_LINK_TO,
    kb::IS_REGION_OF,
    kb::HAS_LINK_TO,
    kb::HAS_COMMENT,
];

/// What the project ontology graphs contribute: the declared classes, the term
/// namespaces and the two closures (§6). Both also drive §10: the class closure
/// decides which resources are annotations, the property closure their motivation
/// and targets. Nothing from them is emitted (§11).
pub(crate) struct Ontology {
    classes: BTreeSet<String>,
    namespaces: Vec<String>,
    pub(crate) sub_class: Closure,
    pub(crate) sub_property: Closure,
}

impl Ontology {
    pub(crate) fn build(dump: &Dump) -> Result<Ontology, Box<dyn Error>> {
        let mut classes = BTreeSet::new();
        for graph in &dump.ontology_graphs {
            let graph = GraphNameRef::NamedNode(graph.as_ref());
            let pattern = dump.store.quads_for_pattern(
                None,
                Some(nn(model::RDF_TYPE).as_ref()),
                Some(nn(model::OWL_CLASS).as_ref().into()),
                Some(graph),
            );
            for quad in pattern {
                if let NamedOrBlankNode::NamedNode(class) = quad?.subject {
                    classes.insert(class.as_str().to_string());
                }
            }
        }
        Ok(Ontology {
            classes,
            namespaces: dump.ontology_graphs.iter().map(|g| format!("{}#", g.as_str())).collect(),
            sub_class: Closure::build(dump, model::RDFS_SUBCLASSOF, CLASS_SEEDS)?,
            sub_property: Closure::build(dump, model::RDFS_SUBPROPERTYOF, PROPERTY_SEEDS)?,
        })
    }

    /// §2: a class declared in a project ontology, or `kb:Region`/`kb:LinkObj`
    /// or anything reaching them by `rdfs:subClassOf*`.
    pub(crate) fn is_resource_class(&self, class: &str) -> bool {
        self.classes.contains(class) || CLASS_SEEDS.iter().any(|seed| self.sub_class.reaches(class, seed))
    }

    /// Whether `iri` lies in a project ontology's term namespace (§11).
    pub(crate) fn is_project_term(&self, iri: &str) -> bool {
        self.namespaces.iter().any(|ns| iri.starts_with(ns.as_str()))
    }
}
