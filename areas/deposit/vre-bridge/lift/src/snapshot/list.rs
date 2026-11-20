//! List nodes (§9): every live `kb:ListNode` as a `skos:Concept`, after §11's
//! deleted-subtree filter, with every breach of the tree's shape reported.

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::model::{Literal, Term};

use super::index::{Entry, Index};
use super::kinds::{self, Kind, Subjects, objects};
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, mint, value, vocab};
use crate::model::{self, kb, nn};

/// §11: the predicates a list node may carry.
const PREDICATES: &[&str] = &[
    model::RDF_TYPE,
    model::RDFS_LABEL,
    kb::LIST_NODE_POSITION,
    kb::IS_ROOT_NODE,
    kb::HAS_SUB_LIST_NODE,
    kb::IS_DELETED,
    kb::HAS_ROOT_NODE,
    kb::LIST_NODE_NAME,
    model::RDFS_COMMENT,
    kb::ATTACHED_TO_PROJECT,
];

/// The stats line dropped list nodes are counted under.
pub(crate) const DELETED_LIST_NODE: &str = "deleted list node (§9)";

/// The live list nodes, the only ones a list value may name (§4.7). Only
/// [`emit`] builds one, after the deleted-subtree filter, so no emitter can
/// name a node that filter dropped.
pub(crate) struct LiveLists(BTreeSet<String>);

impl LiveLists {
    pub(crate) fn contains(&self, iri: &str) -> bool {
        self.0.contains(iri)
    }

    /// No live list node: for tests that call an emitter directly.
    #[cfg(test)]
    pub(crate) fn none() -> LiveLists {
        LiveLists(BTreeSet::new())
    }
}

/// Every list node of the dump, by IRI.
type Nodes<'i> = BTreeMap<&'i str, &'i Entry>;
/// One direction of `kb:hasSubListNode` between list nodes: node → its children or its parents.
type Tree<'i> = BTreeMap<&'i str, BTreeSet<&'i str>>;

/// Emit every live list node and return them.
pub(crate) fn emit(
    index: &Index,
    subjects: &Subjects,
    snapshot: &mut Snapshot,
    stats: &mut Stats,
    violations: &mut Violations,
) -> LiveLists {
    let nodes: Nodes = index
        .entries()
        .filter(|e| subjects.kind(e.node.as_str()) == Some(Kind::ListNode))
        .map(|e| (e.node.as_str(), e))
        .collect();
    // Parent → children and child → parents, over every node, dropped or not.
    let mut children: Tree = BTreeMap::new();
    let mut parents: Tree = BTreeMap::new();
    for (&n, entry) in &nodes {
        for child in objects(entry, kb::HAS_SUB_LIST_NODE) {
            if let Some(c) = list_node(child, &nodes) {
                children.entry(n).or_default().insert(c);
                parents.entry(c).or_default().insert(n);
            }
        }
    }

    let dropped = dropped_subtrees(&nodes, &children);
    if !dropped.is_empty() {
        *stats.omitted.entry(DELETED_LIST_NODE).or_default() += dropped.len();
    }
    let live: Nodes = nodes
        .iter()
        .filter(|(n, _)| !dropped.contains(*n))
        .map(|(&n, &e)| (n, e))
        .collect();

    // Two parents is a breach even when one is deleted: the node would
    // otherwise vanish with that parent's subtree.
    for (&c, ps) in &parents {
        if ps.len() > 1 && ps.iter().any(|p| live.contains_key(p)) {
            for p in ps {
                violations.record(Rule::MultipleParents, c, p);
            }
        }
    }

    let mut siblings: BTreeMap<(&str, u32), Vec<&str>> = BTreeMap::new();
    for (&n, entry) in &live {
        for (p, o) in &entry.triples {
            if !PREDICATES.contains(&p.as_str()) {
                violations.record(Rule::UnknownListNodePredicate, n, p.as_str());
            } else if p.as_str() == kb::HAS_SUB_LIST_NODE && list_node(o, &nodes).is_none() {
                violations.record(Rule::DanglingSubListNode, n, &o.to_string());
            }
        }
        let node = entry.node.as_ref();
        snapshot.put(node, nn(vocab::RDF_TYPE).as_ref(), nn(vocab::SKOS_CONCEPT).as_ref());
        for label in labels(entry, violations) {
            snapshot.put(node, nn(vocab::SKOS_PREF_LABEL).as_ref(), &label);
        }

        let node_parents = parents.get(n);
        if kinds::is_true(entry, kb::IS_ROOT_NODE) {
            for parent in node_parents.into_iter().flatten() {
                violations.record(Rule::RootWithParent, n, parent);
            }
            if !objects(entry, kb::LIST_NODE_POSITION).is_empty() {
                violations.record(Rule::RootWithPosition, n, kb::LIST_NODE_POSITION);
            }
            continue;
        }
        let position = position(entry, violations);
        let Some(&parent) = node_parents.and_then(|p| p.first()) else {
            violations.record(Rule::OrphanListNode, n, "");
            continue;
        };
        if node_parents.is_some_and(|p| p.len() > 1) {
            // Recorded above as `multiple-parents`.
            continue;
        }
        snapshot.put(node, nn(vocab::SKOS_BROADER).as_ref(), nn(parent).as_ref());
        if let Some(position) = position {
            let lexical = Literal::new_typed_literal(position.to_string(), nn(vocab::XSD_INTEGER));
            snapshot.put(node, nn(vocab::DAO_LIST_NODE_POSITION).as_ref(), &lexical);
            siblings.entry((parent, position)).or_default().push(n);
        }
    }
    for ((_, position), group) in &siblings {
        if group.len() > 1 {
            for n in group {
                violations.record(Rule::DuplicateSiblingPosition, n, &position.to_string());
            }
        }
    }
    // A cycle through a dropped node is dropped whole (every member is below
    // it), so only live cycles are reported.
    for n in cycle_members(&parents).into_iter().filter(|n| live.contains_key(n)) {
        violations.record(Rule::ListNodeCycle, n, "");
    }
    stats.list_nodes += live.len();
    LiveLists(live.keys().map(|n| n.to_string()).collect())
}

/// Every deleted node and everything below it (§9). The visited set ends a cycle.
fn dropped_subtrees<'i>(nodes: &Nodes<'i>, children: &Tree<'i>) -> BTreeSet<&'i str> {
    let mut dropped = BTreeSet::new();
    let mut todo: Vec<&str> = nodes.iter().filter(|(_, e)| kinds::is_deleted(e)).map(|(&n, _)| n).collect();
    while let Some(n) = todo.pop() {
        if dropped.insert(n) {
            todo.extend(children.get(n).into_iter().flatten());
        }
    }
    dropped
}

/// The IRI of `term` when it is a list node of the dump.
fn list_node<'i>(term: &Term, nodes: &Nodes<'i>) -> Option<&'i str> {
    match term {
        Term::NamedNode(n) => nodes.get_key_value(n.as_str()).map(|(&k, _)| k),
        _ => None,
    }
}

/// §9: one `skos:prefLabel` per `rdfs:label`, verbatim, at most one per language
/// tag, "untagged" counting as one tag.
fn labels(entry: &Entry, violations: &mut Violations) -> Vec<Literal> {
    let s = entry.node.as_str();
    let mut tags: BTreeSet<Option<String>> = BTreeSet::new();
    let mut labels = Vec::new();
    for label in objects(entry, model::RDFS_LABEL) {
        let Some((literal, tag)) = label_literal(label, s, violations) else {
            continue;
        };
        if !tags.insert(tag.clone()) {
            violations.record(Rule::DuplicateListLabel, s, tag.as_deref().unwrap_or(""));
        }
        labels.push(literal);
    }
    labels
}

/// A label as its `skos:prefLabel` and its lowercased tag (`None` when untagged),
/// or `None` after recording why it is no string label.
fn label_literal(label: &Term, s: &str, violations: &mut Violations) -> Option<(Literal, Option<String>)> {
    match label {
        Term::Literal(l) => match l.language() {
            Some(tag) => mint::lang_literal(l.value(), tag, s, violations)
                .map(|literal| (literal, Some(tag.to_ascii_lowercase()))),
            None if value::is_plain_string(l) => Some((Literal::new_simple_literal(l.value()), None)),
            None => {
                violations.record(Rule::InvalidListLabel, s, &label.to_string());
                None
            }
        },
        _ => {
            violations.record(Rule::InvalidListLabel, s, &label.to_string());
            None
        }
    }
}

/// §9: a non-root's `kb:listNodePosition`, which must be one `xsd:integer`
/// fitting a `u32`; `None` when absent or after recording why it is invalid.
fn position(entry: &Entry, violations: &mut Violations) -> Option<u32> {
    let position = match objects(entry, kb::LIST_NODE_POSITION).as_slice() {
        [] => return None,
        [Term::Literal(l)] => value::xsd_integer_as::<u32>(l),
        _ => None,
    };
    if position.is_none() {
        violations.record(Rule::InvalidListPosition, entry.node.as_str(), kb::LIST_NODE_POSITION);
    }
    position
}

/// Every node on a `skos:broader` cycle, following each node's one parent. Each
/// node is walked once: a walk stops at a node an earlier walk already settled.
fn cycle_members<'i>(parents: &Tree<'i>) -> BTreeSet<&'i str> {
    let parent_of = |n: &str| match parents.get(n) {
        Some(p) if p.len() == 1 => p.first().copied(),
        _ => None,
    };
    let mut settled: BTreeSet<&str> = BTreeSet::new();
    let mut members = BTreeSet::new();
    for &start in parents.keys() {
        let mut path: Vec<&str> = Vec::new();
        let mut at: BTreeMap<&str, usize> = BTreeMap::new();
        let mut next = Some(start);
        while let Some(n) = next {
            if settled.contains(n) {
                break;
            }
            if let Some(&i) = at.get(n) {
                members.extend(&path[i..]);
                break;
            }
            at.insert(n, path.len());
            path.push(n);
            next = parent_of(n);
        }
        settled.extend(path);
    }
    members
}
