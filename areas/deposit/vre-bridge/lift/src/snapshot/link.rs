//! Links (§6) and memberships (§7): a direct resource-to-resource triple is
//! carried only when a live LinkValue describes the same triple and its target
//! is a live resource (§11). LinkValues themselves are read here and never emitted.

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::model::{NamedNodeRef, Term};

use super::Stats;
use super::index::{Entry, Index};
use super::kinds::{self, Kind, Subjects};
use super::violations::{Rule, Violations};
use crate::model::{self, kb};

/// §11: the predicates a LinkValue may carry.
const PREDICATES: &[&str] = &[
    model::RDF_TYPE,
    model::RDF_SUBJECT,
    model::RDF_PREDICATE,
    model::RDF_OBJECT,
    kb::IS_DELETED,
    kb::PREVIOUS_VALUE,
    kb::VALUE_HAS_UUID,
    kb::VALUE_HAS_ORDER,
    kb::VALUE_HAS_STRING,
    kb::VALUE_HAS_REF_COUNT,
    kb::VALUE_HAS_COMMENT,
    kb::VALUE_CREATION_DATE,
    kb::ATTACHED_TO_USER,
    kb::HAS_PERMISSIONS,
];

/// The stats lines a direct triple that is not carried is counted under.
pub(crate) const TO_DELETED_RESOURCE: &str = "link or membership to a deleted resource (§11)";
pub(crate) const TO_NON_RESOURCE: &str = "link or membership to an IRI that is not a resource of the dump (§11)";
pub(crate) const LINK_VALUE_DROPPED: &str = "link or membership whose LinkValues are all deleted or superseded (§6)";

/// `(subject, predicate, object)` of a direct triple, as IRIs.
type Triple = (String, String, String);

/// What the LinkValues and resources of the dump say about direct triples.
pub(crate) struct Links {
    /// Triples a live LinkValue describes.
    live: BTreeSet<Triple>,
    /// Triples only deleted or superseded LinkValues describe (a live one may also).
    dropped: BTreeSet<Triple>,
    /// Resources that are not deleted: the only targets a link may have.
    resources: BTreeSet<String>,
    /// Deleted resources, so a link to one is counted apart from other IRIs.
    deleted: BTreeSet<String>,
    /// Dump subjects with no kind: already a violation, so a link to one is not
    /// reported a second time.
    unclassified: BTreeSet<String>,
    /// `(subject, predicate)` of live LinkValues with no single IRI `rdf:object`: the
    /// LinkValue is the violation, so no direct triple under that subject and predicate is reported
    /// again.
    malformed: BTreeSet<(String, String)>,
}

impl Links {
    /// Read every LinkValue, checking the live ones against §11's list and their
    /// direct triple against the dump, and every live resource.
    pub(crate) fn read(index: &Index, subjects: &Subjects, violations: &mut Violations) -> Links {
        let mut links = Links {
            live: BTreeSet::new(),
            dropped: BTreeSet::new(),
            resources: BTreeSet::new(),
            deleted: BTreeSet::new(),
            unclassified: BTreeSet::new(),
            malformed: BTreeSet::new(),
        };
        for entry in index.entries() {
            match subjects.kind(entry.node.as_str()) {
                None => {
                    links.unclassified.insert(entry.node.as_str().to_string());
                }
                Some(Kind::Resource) => {
                    let set = if kinds::is_deleted(entry) {
                        &mut links.deleted
                    } else {
                        &mut links.resources
                    };
                    set.insert(entry.node.as_str().to_string());
                }
                Some(Kind::LinkValue) => links.read_link_value(entry, index, subjects, violations),
                _ => {}
            }
        }
        links
    }

    fn read_link_value(&mut self, entry: &Entry, index: &Index, subjects: &Subjects, violations: &mut Violations) {
        let lv = entry.node.as_str();
        // A dropped LinkValue's predicates are not checked (§11).
        let dropped = subjects.dropped(entry).is_some();
        if !dropped {
            for (p, _) in &entry.triples {
                if !PREDICATES.contains(&p.as_str()) {
                    violations.record(Rule::UnknownLinkValuePredicate, lv, p.as_str());
                }
            }
        }
        let triple = match (
            one_iri(entry, model::RDF_SUBJECT),
            one_iri(entry, model::RDF_PREDICATE),
            one_iri(entry, model::RDF_OBJECT),
        ) {
            (Ok(s), Ok(p), Ok(o)) => (s, p, o),
            (s, p, o) => {
                if !dropped {
                    // Direct triples under this subject and predicate are not reported
                    // again as missing their LinkValue: this LinkValue is the violation.
                    // A dropped one is no violation, so it must not silence them.
                    if let (Ok(s), Ok(p), Err(_)) = (&s, &p, &o) {
                        self.malformed.insert((s.clone(), p.clone()));
                    }
                    for missing in [s.err(), p.err(), o.err()].into_iter().flatten() {
                        violations.record(Rule::InvalidLinkValue, lv, missing);
                    }
                }
                return;
            }
        };
        if dropped {
            // Recorded, so its direct link is dropped, not reported as unexplained.
            self.dropped.insert(triple);
            return;
        }
        let (s, p, o) = &triple;
        let in_dump = index.get(s).is_some_and(|e| {
            e.triples
                .iter()
                .any(|(q, obj)| q.as_str() == p && matches!(obj, Term::NamedNode(n) if n.as_str() == o))
        });
        if !in_dump {
            violations.record(Rule::UnpairedLinkValue, lv, &format!("{s} {p} {o}"));
        }
        self.live.insert(triple);
    }

    /// The target of the direct triple `<s> <p> <o>` when it is carried (§6):
    /// a live LinkValue describes it and `o` is a live resource. Otherwise it is
    /// counted as dropped, or recorded as a violation when no LinkValue describes it.
    pub(crate) fn carried<'o>(
        &self,
        s: &str,
        p: &str,
        o: NamedNodeRef<'o>,
        stats: &mut Stats,
        violations: &mut Violations,
    ) -> Option<NamedNodeRef<'o>> {
        if self.unclassified.contains(o.as_str()) {
            return None;
        }
        let triple = (s.to_string(), p.to_string(), o.as_str().to_string());
        let reason = if !self.live.contains(&triple) {
            if !self.dropped.contains(&triple) {
                if !self.malformed.contains(&(triple.0, triple.1)) {
                    violations.record(Rule::MissingLinkValue, s, &format!("{p} {}", o.as_str()));
                }
                return None;
            }
            LINK_VALUE_DROPPED
        } else if self.resources.contains(o.as_str()) {
            return Some(o);
        } else if self.deleted.contains(o.as_str()) {
            TO_DELETED_RESOURCE
        } else {
            TO_NON_RESOURCE
        };
        *stats.omitted.entry(reason).or_default() += 1;
        None
    }
}

/// The LinkValue's one IRI object of `p`, or `Err(p)` when it has none, several
/// or a literal.
fn one_iri(entry: &Entry, p: &'static str) -> Result<String, &'static str> {
    match kinds::objects(entry, p).as_slice() {
        [Term::NamedNode(n)] => Ok(n.as_str().to_string()),
        _ => Err(p),
    }
}

/// Emitted `dao:isPartOf` edges: child → its parents.
pub(crate) type Memberships = BTreeMap<String, BTreeSet<String>>;

/// §7: record every resource on a `dao:isPartOf` cycle.
pub(crate) fn check_membership_cycles(parts: &Memberships, violations: &mut Violations) {
    for member in cycle_members(parts) {
        violations.record(Rule::MembershipCycle, member, "");
    }
}

/// Every node of a strongly connected component with a cycle: Tarjan's algorithm,
/// iterative so a deep membership chain cannot overflow the stack. Each node and
/// edge is visited once, and a node that only reaches a cycle is not a member.
fn cycle_members(graph: &Memberships) -> BTreeSet<&str> {
    let mut t = Tarjan {
        graph,
        order: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        members: BTreeSet::new(),
    };
    for root in graph.keys() {
        if t.order.contains_key(root.as_str()) {
            continue;
        }
        let mut work = vec![(root.as_str(), t.visit(root))];
        while let Some((v, edges)) = work.last_mut() {
            let v = *v;
            if let Some(w) = edges.next() {
                let w = w.as_str();
                if !t.order.contains_key(w) {
                    let edges = t.visit(w);
                    work.push((w, edges));
                } else if t.on_stack.contains(w) {
                    t.lower(v, t.order[w]);
                }
                continue;
            }
            work.pop();
            if let Some(&(u, _)) = work.last() {
                t.lower(u, t.low[v]);
            }
            if t.low[v] == t.order[v] {
                t.pop_component(v);
            }
        }
    }
    t.members
}

/// The state of one [`cycle_members`] run.
struct Tarjan<'g> {
    graph: &'g Memberships,
    order: BTreeMap<&'g str, usize>,
    low: BTreeMap<&'g str, usize>,
    stack: Vec<&'g str>,
    on_stack: BTreeSet<&'g str>,
    members: BTreeSet<&'g str>,
}

impl<'g> Tarjan<'g> {
    /// Number `v`, push it, and return its outgoing edges.
    fn visit(&mut self, v: &'g str) -> std::collections::btree_set::Iter<'g, String> {
        let n = self.order.len();
        self.order.insert(v, n);
        self.low.insert(v, n);
        self.stack.push(v);
        self.on_stack.insert(v);
        self.graph.get(v).map(|p| p.iter()).unwrap_or_default()
    }

    fn lower(&mut self, v: &'g str, to: usize) {
        let low = self.low[v].min(to);
        self.low.insert(v, low);
    }

    /// Pop the component rooted at `v`; keep it when it holds a cycle.
    fn pop_component(&mut self, v: &'g str) {
        let mut component = Vec::new();
        while let Some(w) = self.stack.pop() {
            self.on_stack.remove(w);
            component.push(w);
            if w == v {
                break;
            }
        }
        let self_loop = self.graph.get(v).is_some_and(|p| p.contains(v));
        if component.len() > 1 || self_loop {
            self.members.extend(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Memberships, cycle_members};

    fn graph(edges: &[(&str, &str)]) -> Memberships {
        let mut g = Memberships::new();
        for (child, parent) in edges {
            g.entry(child.to_string()).or_default().insert(parent.to_string());
        }
        g
    }

    #[test]
    fn every_node_of_a_cycle_is_a_member_even_one_reached_after_the_cycle_closed() {
        // a → b → c → a closes first; d lies on the second cycle b → d → c → a → b.
        let g = graph(&[("a", "b"), ("b", "c"), ("c", "a"), ("b", "d"), ("d", "c"), ("e", "a")]);
        let members: BTreeSet<&str> = ["a", "b", "c", "d"].into();
        assert_eq!(cycle_members(&g), members);
    }

    #[test]
    fn a_self_loop_is_a_cycle_and_a_chain_is_not() {
        let g = graph(&[("a", "a"), ("b", "c"), ("c", "d")]);
        assert_eq!(cycle_members(&g), ["a"].into());
    }
}
