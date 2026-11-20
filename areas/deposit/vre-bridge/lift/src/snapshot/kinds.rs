//! Closed-world classification (§6, §11): every data-graph subject gets exactly
//! one kind, and every resource-level predicate exactly one §6 rule, or a violation.

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::model::{NamedNode, Term};

use super::index::{Entry, Index, Ontology};
use super::value::ValueKind;
use super::violations::{Rule, Violations};
use crate::model::{self, kb};

/// The kind of a data-graph subject (§11's closed list, keyed by subject kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Resource,
    Value,
    FileValue,
    LinkValue,
    ListNode,
    Standoff,
}

/// File-value classes (§8), including the kinds omitted whole.
const FILE_VALUE_CLASSES: &[&str] = &[
    kb::STILL_IMAGE_FILE_VALUE,
    kb::AUDIO_FILE_VALUE,
    kb::MOVING_IMAGE_FILE_VALUE,
    kb::DOCUMENT_FILE_VALUE,
    kb::ARCHIVE_FILE_VALUE,
    kb::TEXT_FILE_VALUE,
    kb::STILL_IMAGE_EXTERNAL_FILE_VALUE,
];

pub(crate) struct Subject {
    pub(crate) kind: Kind,
    /// The one `rdf:type`; `None` for standoff nodes, which are known by position.
    pub(crate) class: Option<NamedNode>,
}

/// The kind of every data-graph subject that has one.
pub(crate) struct Subjects {
    kinds: BTreeMap<String, Subject>,
}

impl Subjects {
    pub(crate) fn get(&self, iri: &str) -> Option<&Subject> {
        self.kinds.get(iri)
    }

    pub(crate) fn kind(&self, iri: &str) -> Option<Kind> {
        self.kinds.get(iri).map(|s| s.kind)
    }
}

/// §11's deleted filter: `kb:isDeleted` with an `xsd:boolean` true. The
/// selection reads it for resources and values (`crate::selection`), and list
/// nodes for their subtree (§9).
pub(crate) fn is_deleted(entry: &Entry) -> bool {
    is_true(entry, kb::IS_DELETED)
}

/// Every object of `p` on the subject.
pub(crate) fn objects<'e>(entry: &'e Entry, p: &str) -> Vec<&'e Term> {
    entry.triples.iter().filter(|(q, _)| q.as_str() == p).map(|(_, o)| o).collect()
}

/// Whether the subject carries `p` with an `xsd:boolean` true.
pub(crate) fn is_true(entry: &Entry, p: &str) -> bool {
    entry.triples.iter().any(|(q, o)| {
        q.as_str() == p
            && matches!(o, Term::Literal(l)
                if l.datatype().as_str() == model::XSD_BOOLEAN && matches!(l.value(), "true" | "1"))
    })
}

/// Every IRI object of `p` anywhere in the data graph.
fn objects_of<'i>(index: &'i Index, p: &str) -> BTreeSet<&'i str> {
    index
        .entries()
        .flat_map(|e| &e.triples)
        .filter(|(q, _)| q.as_str() == p)
        .filter_map(|(_, o)| match o {
            Term::NamedNode(n) => Some(n.as_str()),
            _ => None,
        })
        .collect()
}

/// Classify every subject by its one `rdf:type`. Standoff nodes are known by
/// position and never checked (§11): the direct objects of `kb:valueHasStandoff`.
/// DSP standoff tags hang flat off their value, so this is §11's whole subtree;
/// a nested tag would fail as an unknown class, never be dropped silently.
pub(crate) fn classify_subjects(index: &Index, ontology: &Ontology, violations: &mut Violations) -> Subjects {
    let standoff = objects_of(index, kb::VALUE_HAS_STANDOFF);

    let mut subjects = BTreeMap::new();
    for entry in index.entries() {
        let s = entry.node.as_str();
        if standoff.contains(s) {
            subjects.insert(s.to_string(), Subject { kind: Kind::Standoff, class: None });
            continue;
        }
        let types: Vec<&Term> = entry
            .triples
            .iter()
            .filter(|(p, _)| p.as_str() == model::RDF_TYPE)
            .map(|(_, o)| o)
            .collect();
        let class = match types.as_slice() {
            [] => {
                violations.record(Rule::UntypedSubject, s, "");
                continue;
            }
            [Term::NamedNode(class)] => class,
            [other] => {
                violations.record(Rule::UnknownClass, s, &other.to_string());
                continue;
            }
            many => {
                let names: Vec<String> = many.iter().map(|t| t.to_string()).collect();
                violations.record(Rule::MultipleTypes, s, &names.join(" "));
                continue;
            }
        };
        match kind_of_class(class.as_str(), ontology) {
            Some(kind) => {
                subjects.insert(s.to_string(), Subject { kind, class: Some(class.clone()) });
            }
            None => violations.record(Rule::UnknownClass, s, class.as_str()),
        }
    }
    Subjects { kinds: subjects }
}

fn kind_of_class(class: &str, ontology: &Ontology) -> Option<Kind> {
    if ValueKind::of_class(class).is_some() {
        Some(Kind::Value)
    } else if FILE_VALUE_CLASSES.contains(&class) {
        Some(Kind::FileValue)
    } else if class == kb::LINK_VALUE {
        Some(Kind::LinkValue)
    } else if class == kb::LIST_NODE {
        Some(Kind::ListNode)
    } else if ontology.is_resource_class(class) {
        Some(Kind::Resource)
    } else {
        None
    }
}

/// How a resource-level predicate is handled: §2 structure, §11's filter and
/// omitted predicates, then the §6 precedence rules in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Handling {
    Type,
    Label,
    Deleted,
    /// Users, project, permissions, creation and modification dates (§11).
    Admin,
    Membership,
    Seqnum,
    StandoffLink,
    Link,
    LinkValueCompanion,
    FileValue,
    Value,
}

/// §11's resource list: the knora-base predicates that go through the §6 rules
/// when a project uses them directly.
const KB_RESOURCE_PREDICATES: &[&str] = &[
    kb::IS_PART_OF,
    kb::SEQNUM,
    kb::HAS_STANDOFF_LINK_TO,
    kb::HAS_LINK_TO,
    kb::IS_REGION_OF,
    kb::HAS_LINK_TO_VALUE,
    kb::IS_REGION_OF_VALUE,
    kb::IS_PART_OF_VALUE,
    kb::HAS_STANDOFF_LINK_TO_VALUE,
    kb::HAS_STILL_IMAGE_FILE_VALUE,
    kb::HAS_COMMENT,
    kb::HAS_COLOR,
    kb::HAS_GEOMETRY,
];

const KB_ADMIN_PREDICATES: &[&str] = &[
    kb::ATTACHED_TO_PROJECT,
    kb::ATTACHED_TO_USER,
    kb::CREATION_DATE,
    kb::LAST_MODIFICATION_DATE,
    kb::HAS_PERMISSIONS,
];

/// The handling of `<resource> p o`, or `None` when it fits no rule (a violation).
pub(crate) fn classify_resource_predicate(
    p: &str,
    o: &Term,
    ontology: &Ontology,
    subjects: &Subjects,
) -> Option<Handling> {
    match p {
        model::RDF_TYPE => return Some(Handling::Type),
        model::RDFS_LABEL => return Some(Handling::Label),
        kb::IS_DELETED => return Some(Handling::Deleted),
        _ if KB_ADMIN_PREDICATES.contains(&p) => return Some(Handling::Admin),
        _ => {}
    }
    let object_kind = match o {
        Term::NamedNode(n) => subjects.kind(n.as_str()),
        _ => None,
    };
    let listed =
        ontology.is_project_term(p) || KB_RESOURCE_PREDICATES.contains(&p) || object_kind == Some(Kind::FileValue);
    if !listed {
        return None;
    }
    // Every emitted rule needs an IRI object, so a literal fits none of them; a
    // standoff link is always omitted (§6 rule 3), whatever its object.
    let is_iri = matches!(o, Term::NamedNode(_));
    let sub = &ontology.sub_property;
    if sub.reaches(p, kb::IS_PART_OF) {
        return is_iri.then_some(Handling::Membership);
    }
    if sub.reaches(p, kb::SEQNUM) {
        return is_iri.then_some(Handling::Seqnum);
    }
    if sub.reaches(p, kb::HAS_STANDOFF_LINK_TO) {
        return Some(Handling::StandoffLink);
    }
    if !is_iri {
        return None;
    }
    Some(match object_kind {
        Some(Kind::LinkValue) => Handling::LinkValueCompanion,
        Some(Kind::FileValue) => Handling::FileValue,
        Some(Kind::Value) => Handling::Value,
        _ => Handling::Link,
    })
}
