//! The invariants every [`ArchiveProjection`](crate::ArchiveProjection) adapter's output satisfies.
//!
//! [`violations`] checks only what one snapshot shows. Value order, `lang`, the syntax of
//! `Decimal`, `Uri`, `Geometry` and `Color`, and whether an omitted fact was omitted correctly are
//! invisible to it, so an empty result is a consistency check, not proof of fidelity; an adapter
//! pins those in its own tests. Of what [`Annotation::targets`](crate::Annotation::targets)
//! promises, it checks only that there is at least one and each is a resource of the snapshot. It
//! checks the shape of a resource's data ARK, not that the ARK belongs to its IRI; that is the
//! adapter's promise.
//!
//! Of a project's curation it checks that a value names a resource of the snapshot, that resource,
//! key and language occur once, that a key and a language are curation names
//! ([`is_curation_name`]), and that a text is not empty. What a key means, what a non-empty text
//! holds, and whether a resource has every value its project requires are the reader's rules.
//!
//! A violation is always an adapter bug, or bad data an adapter failed to refuse. `sync-store` runs
//! [`violations`] on every snapshot it serves and refuses one with any violation, so a new
//! invariant here can take a project offline at run time, not only fail a test.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{DataArk, ListNodeIri, ProjectSnapshot, PropertyIri, ResourceIri, ValueKind, DATA_ARK_PREFIX};

/// One broken invariant, naming the IRIs involved.
///
/// [`violations`] returns them sorted by `Ord`: variant order as declared, then fields (the first
/// field is the IRI the violation names first). Keep that when adding a variant or a field.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    /// The snapshot's `shortcode` is not the one requested.
    ShortcodeMismatch { requested: String, served: String },
    /// Two or more resources share this IRI.
    DuplicateResource { resource: ResourceIri },
    /// A resource's `ark` is not a plain data ARK:
    /// `https://ark.dasch.swiss/ark:/72163/1/<shortcode>/<id>` with four uppercase hex digits as
    /// shortcode and an id of `[A-Za-z0-9_=]+`.
    MalformedDataArk { resource: ResourceIri, ark: DataArk },
    /// A `Link` value's target is not a resource in the snapshot.
    DanglingLink {
        resource: ResourceIri,
        property: PropertyIri,
        target: ResourceIri,
    },
    /// An annotation's target is not a resource in the snapshot.
    DanglingTarget { resource: ResourceIri, target: ResourceIri },
    /// An annotation has no target.
    UntargetedAnnotation { resource: ResourceIri },
    /// A `part_of` target is not a resource in the snapshot.
    DanglingParent { resource: ResourceIri, parent: ResourceIri },
    /// Resources that are their own ancestors through `part_of`: one per strongly connected
    /// component, named by its smallest IRI, so two cycles sharing a resource are one report.
    MembershipCycle { resource: ResourceIri },
    /// A `ListNode` value, or a node's `parent`, names no node in `list_nodes`.
    DanglingListNode { referrer: ListNodeReferrer, missing: ListNodeIri },
    /// Two or more list nodes share this IRI.
    DuplicateListNode { node: ListNodeIri },
    /// List nodes that are their own ancestors through `parent`: one per strongly connected
    /// component, named by its smallest IRI, so two cycles sharing a node are one report.
    ListNodeCycle { node: ListNodeIri },
    /// Distinct siblings under one parent share a position; `nodes` is sorted. A repeated IRI is
    /// [`Violation::DuplicateListNode`] instead. Roots are separate lists and never compared with
    /// each other.
    DuplicateSiblingPosition { parent: ListNodeIri, position: u32, nodes: Vec<ListNodeIri> },
    /// A date whose start JDN is greater than its end JDN.
    InvertedDate { resource: ResourceIri, property: PropertyIri },
    /// A value that is not a link has no UUID.
    MissingValueUuid { resource: ResourceIri, property: PropertyIri },
    /// Two or more values of one resource share this UUID; reported once per UUID.
    DuplicateValueUuid { resource: ResourceIri, uuid: String },
    /// A link carries a UUID, which a link never has.
    LinkWithValueUuid { resource: ResourceIri, property: PropertyIri },
    /// A curated value names a resource that is not in the snapshot; reported once per IRI.
    DanglingCuration { resource: ResourceIri },
    /// Two or more curated values share this resource, key and language; reported once per triple.
    DuplicateCuration { resource: ResourceIri, key: String, lang: Option<String> },
    /// A curated value whose `key`, or whose `lang` where it has one, is no curation name
    /// ([`is_curation_name`]), or whose `text` is empty; reported once per entry of `curation`.
    MalformedCuration { resource: ResourceIri, key: String, lang: Option<String> },
}

/// What refers to a list node in [`Violation::DanglingListNode`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ListNodeReferrer {
    /// A `ListNode` value of this resource.
    Value { resource: ResourceIri, property: PropertyIri },
    /// This node's `parent`.
    Parent { node: ListNodeIri },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShortcodeMismatch { requested, served } => {
                write!(f, "requested project {requested}, but the snapshot is of {served}")
            }
            Self::DuplicateResource { resource } => write!(f, "resource {} occurs more than once", resource.as_str()),
            Self::MalformedDataArk { resource, ark } => {
                write!(f, "resource {} has the malformed data ARK {}", resource.as_str(), ark.as_str())
            }
            Self::DanglingLink { resource, property, target } => write!(
                f,
                "resource {} links via {} to {}, which is not in the snapshot",
                resource.as_str(),
                property.as_str(),
                target.as_str()
            ),
            Self::DanglingTarget { resource, target } => write!(
                f,
                "resource {} is an annotation targeting {}, which is not in the snapshot",
                resource.as_str(),
                target.as_str()
            ),
            Self::UntargetedAnnotation { resource } => {
                write!(f, "resource {} is an annotation without a target", resource.as_str())
            }
            Self::DanglingParent { resource, parent } => write!(
                f,
                "resource {} is part of {}, which is not in the snapshot",
                resource.as_str(),
                parent.as_str()
            ),
            Self::MembershipCycle { resource } => {
                write!(f, "resource {} is its own ancestor through part_of", resource.as_str())
            }
            Self::DanglingListNode { referrer, missing } => {
                match referrer {
                    ListNodeReferrer::Value { resource, property } => write!(
                        f,
                        "resource {} has a list value via {} naming",
                        resource.as_str(),
                        property.as_str()
                    )?,
                    ListNodeReferrer::Parent { node } => write!(f, "list node {} has the parent", node.as_str())?,
                }
                write!(f, " {}, which is not in the snapshot", missing.as_str())
            }
            Self::DuplicateListNode { node } => write!(f, "list node {} occurs more than once", node.as_str()),
            Self::ListNodeCycle { node } => write!(f, "list node {} is its own ancestor", node.as_str()),
            Self::DuplicateSiblingPosition { parent, position, nodes } => {
                let nodes: Vec<&str> = nodes.iter().map(ListNodeIri::as_str).collect();
                write!(
                    f,
                    "list nodes {} under {} share position {position}",
                    nodes.join(", "),
                    parent.as_str()
                )
            }
            Self::InvertedDate { resource, property } => write!(
                f,
                "resource {} has a date via {} that ends before it starts",
                resource.as_str(),
                property.as_str()
            ),
            Self::MissingValueUuid { resource, property } => write!(
                f,
                "resource {} has a value via {} without a UUID",
                resource.as_str(),
                property.as_str()
            ),
            Self::DuplicateValueUuid { resource, uuid } => {
                write!(f, "resource {} has more than one value with UUID {uuid}", resource.as_str())
            }
            Self::LinkWithValueUuid { resource, property } => write!(
                f,
                "resource {} has a link via {} that carries a UUID",
                resource.as_str(),
                property.as_str()
            ),
            Self::DanglingCuration { resource } => {
                write!(
                    f,
                    "resource {} has curated values, but is not in the snapshot",
                    resource.as_str()
                )
            }
            Self::DuplicateCuration { resource, key, lang } => write!(
                f,
                "resource {} has more than one curated value {}",
                resource.as_str(),
                curation_name(key, lang.as_deref())
            ),
            Self::MalformedCuration { resource, key, lang } => write!(
                f,
                "resource {} has a curated value {:?} whose key or language is not a curation name, or whose text \
                 is empty",
                resource.as_str(),
                curation_name(key, lang.as_deref())
            ),
        }
    }
}

/// `key`, or `key@lang` where the value has a language.
fn curation_name(key: &str, lang: Option<&str>) -> String {
    match lang {
        Some(lang) => format!("{key}@{lang}"),
        None => key.to_string(),
    }
}

/// Every invariant `snapshot` breaks when served for `requested`, in the order [`Violation`]
/// documents.
#[must_use]
pub fn violations(requested: &str, snapshot: &ProjectSnapshot) -> Vec<Violation> {
    let mut found = Vec::new();

    if snapshot.shortcode != requested {
        found.push(Violation::ShortcodeMismatch {
            requested: requested.to_string(),
            served: snapshot.shortcode.clone(),
        });
    }

    let mut resource_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for resource in &snapshot.resources {
        *resource_counts.entry(resource.iri.as_str()).or_default() += 1;
    }
    let mut node_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for node in &snapshot.list_nodes {
        *node_counts.entry(node.iri.as_str()).or_default() += 1;
    }

    for (&iri, _) in resource_counts.iter().filter(|(_, &count)| count > 1) {
        found.push(Violation::DuplicateResource { resource: ResourceIri(iri.to_string()) });
    }
    for (&iri, _) in node_counts.iter().filter(|(_, &count)| count > 1) {
        found.push(Violation::DuplicateListNode { node: ListNodeIri(iri.to_string()) });
    }

    let mut parents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for resource in &snapshot.resources {
        if !is_plain_data_ark(resource.ark.as_str()) {
            found.push(Violation::MalformedDataArk { resource: resource.iri.clone(), ark: resource.ark.clone() });
        }

        let edges = parents.entry(resource.iri.as_str()).or_default();
        for parent in &resource.part_of {
            if resource_counts.contains_key(parent.as_str()) {
                edges.push(parent.as_str());
            } else {
                found.push(Violation::DanglingParent { resource: resource.iri.clone(), parent: parent.clone() });
            }
        }

        if let Some(annotation) = &resource.annotation {
            if annotation.targets.is_empty() {
                found.push(Violation::UntargetedAnnotation { resource: resource.iri.clone() });
            }
            for target in &annotation.targets {
                if !resource_counts.contains_key(target.as_str()) {
                    found.push(Violation::DanglingTarget { resource: resource.iri.clone(), target: target.clone() });
                }
            }
        }

        for value in &resource.values {
            match &value.kind {
                ValueKind::Link(target) if !resource_counts.contains_key(target.as_str()) => {
                    found.push(Violation::DanglingLink {
                        resource: resource.iri.clone(),
                        property: value.property.clone(),
                        target: target.clone(),
                    });
                }
                ValueKind::ListNode(node) if !node_counts.contains_key(node.as_str()) => {
                    found.push(Violation::DanglingListNode {
                        referrer: ListNodeReferrer::Value {
                            resource: resource.iri.clone(),
                            property: value.property.clone(),
                        },
                        missing: node.clone(),
                    });
                }
                ValueKind::Date(date) if date.start.jdn > date.end.jdn => {
                    found.push(Violation::InvertedDate {
                        resource: resource.iri.clone(),
                        property: value.property.clone(),
                    });
                }
                _ => {}
            }
        }
    }
    for resource in &snapshot.resources {
        let mut uuid_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for value in &resource.values {
            match (&value.uuid, &value.kind) {
                (Some(_), ValueKind::Link(_)) => found.push(Violation::LinkWithValueUuid {
                    resource: resource.iri.clone(),
                    property: value.property.clone(),
                }),
                (Some(uuid), _) => *uuid_counts.entry(uuid.as_str()).or_default() += 1,
                (None, ValueKind::Link(_)) => {}
                (None, _) => found.push(Violation::MissingValueUuid {
                    resource: resource.iri.clone(),
                    property: value.property.clone(),
                }),
            }
        }
        for (&uuid, _) in uuid_counts.iter().filter(|(_, &count)| count > 1) {
            found.push(Violation::DuplicateValueUuid { resource: resource.iri.clone(), uuid: uuid.to_string() });
        }
    }
    for resource in cycle_representatives(&parents) {
        found.push(Violation::MembershipCycle { resource: ResourceIri(resource.to_string()) });
    }

    let mut node_parents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut siblings: BTreeMap<(&str, u32), Vec<&ListNodeIri>> = BTreeMap::new();
    for node in &snapshot.list_nodes {
        let edges = node_parents.entry(node.iri.as_str()).or_default();
        let Some(parent) = &node.parent else { continue };
        if node_counts.contains_key(parent.as_str()) {
            edges.push(parent.as_str());
        } else {
            found.push(Violation::DanglingListNode {
                referrer: ListNodeReferrer::Parent { node: node.iri.clone() },
                missing: parent.clone(),
            });
        }
        if let Some(position) = node.position {
            siblings.entry((parent.as_str(), position)).or_default().push(&node.iri);
        }
    }
    for node in cycle_representatives(&node_parents) {
        found.push(Violation::ListNodeCycle { node: ListNodeIri(node.to_string()) });
    }
    for ((parent, position), mut nodes) in siblings {
        nodes.sort();
        nodes.dedup();
        if nodes.len() < 2 {
            continue;
        }
        found.push(Violation::DuplicateSiblingPosition {
            parent: ListNodeIri(parent.to_string()),
            position,
            nodes: nodes.into_iter().cloned().collect(),
        });
    }

    let mut dangling: BTreeSet<&ResourceIri> = BTreeSet::new();
    let mut curation_counts: BTreeMap<(&ResourceIri, &str, Option<&str>), usize> = BTreeMap::new();
    for value in &snapshot.curation {
        let lang = value.lang.as_deref();
        if !resource_counts.contains_key(value.resource.as_str()) {
            dangling.insert(&value.resource);
        }
        *curation_counts.entry((&value.resource, value.key.as_str(), lang)).or_default() += 1;
        if !is_curation_name(&value.key) || !lang.is_none_or(is_curation_name) || value.text.is_empty() {
            found.push(Violation::MalformedCuration {
                resource: value.resource.clone(),
                key: value.key.clone(),
                lang: value.lang.clone(),
            });
        }
    }
    for resource in dangling {
        found.push(Violation::DanglingCuration { resource: resource.clone() });
    }
    for ((resource, key, lang), _) in curation_counts.into_iter().filter(|&(_, count)| count > 1) {
        found.push(Violation::DuplicateCuration {
            resource: resource.clone(),
            key: key.to_string(),
            lang: lang.map(str::to_string),
        });
    }

    found.sort();
    found
}

/// Whether `name` can be a curated key or language: `[a-z][a-z0-9_-]*`, so `key@lang` reads one
/// way. A language tag with an upper-case part, such as `de-CH`, is refused by design.
#[must_use]
pub fn is_curation_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Whether `s` is `DATA_ARK_PREFIX` followed by `<shortcode>/<id>`: a four-digit uppercase
/// hexadecimal shortcode, and an id of ASCII letters, digits, `_` and `=`. A version suffix, a
/// value segment, a lowercase shortcode, a `-` and a foreign resolver all fail.
fn is_plain_data_ark(s: &str) -> bool {
    let Some(rest) = s.strip_prefix(DATA_ARK_PREFIX) else {
        return false;
    };
    let Some((shortcode, id)) = rest.split_once('/') else {
        return false;
    };
    shortcode.len() == 4
        && shortcode.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        && !id.is_empty()
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'=')
}

/// The smallest member of every strongly connected component of `edges` that contains a cycle,
/// self-loops included; successors that are not keys of `edges` are ignored.
///
/// Tarjan's algorithm with an explicit stack, so a deep chain cannot overflow the call stack.
fn cycle_representatives<'a>(edges: &BTreeMap<&'a str, Vec<&'a str>>) -> Vec<&'a str> {
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    let mut low: BTreeMap<&str, usize> = BTreeMap::new();
    let mut component: Vec<&str> = Vec::new();
    let mut on_component: BTreeSet<&str> = BTreeSet::new();
    let mut representatives = Vec::new();

    for &root in edges.keys() {
        if index.contains_key(root) {
            continue;
        }
        let mut call: Vec<(&str, usize)> = vec![(root, 0)];
        index.insert(root, index.len());
        low.insert(root, index[root]);
        component.push(root);
        on_component.insert(root);

        while let Some(frame) = call.last_mut() {
            let node = frame.0;
            if let Some(&successor) = edges[node].get(frame.1) {
                frame.1 += 1;
                if !edges.contains_key(successor) {
                    continue;
                }
                if !index.contains_key(successor) {
                    index.insert(successor, index.len());
                    low.insert(successor, index[successor]);
                    component.push(successor);
                    on_component.insert(successor);
                    call.push((successor, 0));
                } else if on_component.contains(successor) {
                    low.insert(node, low[node].min(index[successor]));
                }
                continue;
            }

            call.pop();
            if let Some(&(caller, _)) = call.last() {
                low.insert(caller, low[caller].min(low[node]));
            }
            if low[node] == index[node] {
                let mut members = Vec::new();
                while let Some(member) = component.pop() {
                    on_component.remove(member);
                    members.push(member);
                    if member == node {
                        break;
                    }
                }
                if members.len() > 1 || edges[node].contains(&node) {
                    representatives.extend(members.into_iter().min());
                }
            }
        }
    }
    representatives
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Annotation, Calendar, ClassIri, CuratedValue, DateBound, DatePrecision, DateValue, File, LangString, ListNode,
        Motivation, Resource, Value,
    };

    const PAGE: &str = "http://www.knora.org/ontology/0803/incunabula#page";
    const BOOK: &str = "http://www.knora.org/ontology/0803/incunabula#book";
    const REGION: &str = "http://www.knora.org/ontology/knora-base#Region";

    fn res(id: &str) -> ResourceIri {
        ResourceIri(format!("http://rdfh.ch/0803/{id}"))
    }

    fn node(id: &str) -> ListNodeIri {
        ListNodeIri(format!("http://rdfh.ch/lists/0803/{id}"))
    }

    fn prop(name: &str) -> PropertyIri {
        PropertyIri(format!("http://www.knora.org/ontology/0803/incunabula#{name}"))
    }

    fn resource(id: &str, class: &str) -> Resource {
        Resource {
            iri: res(id),
            ark: DataArk(format!("https://ark.dasch.swiss/ark:/72163/1/0803/{}", id.replace('-', "="))),
            class: ClassIri(class.to_string()),
            label: id.to_string(),
            values: vec![],
            file: None,
            part_of: vec![],
            seqnum: None,
            annotation: None,
        }
    }

    fn page(id: &str, book: &str, seqnum: Option<i64>) -> Resource {
        Resource { part_of: vec![res(book)], seqnum, ..resource(id, PAGE) }
    }

    fn list_node(id: &str, parent: Option<&str>, position: Option<u32>) -> ListNode {
        ListNode {
            iri: node(id),
            parent: parent.map(node),
            position,
            labels: vec![],
        }
    }

    fn date(start: i64, end: i64) -> Value {
        Value {
            property: prop("pubdate"),
            uuid: Some(format!("date-{start}")),
            kind: ValueKind::Date(DateValue {
                calendar: Calendar::Julian,
                start: DateBound { jdn: start, precision: DatePrecision::Day },
                end: DateBound { jdn: end, precision: DatePrecision::Day },
            }),
        }
    }

    fn snapshot(resources: Vec<Resource>, list_nodes: Vec<ListNode>) -> ProjectSnapshot {
        ProjectSnapshot {
            shortcode: "0803".to_string(),
            resources,
            list_nodes,
            curation: vec![],
        }
    }

    fn curated(id: &str, key: &str, lang: Option<&str>, text: &str) -> CuratedValue {
        CuratedValue {
            resource: res(id),
            key: key.to_string(),
            lang: lang.map(str::to_string),
            text: text.to_string(),
        }
    }

    fn valid_snapshot() -> ProjectSnapshot {
        let text = |uuid: &str, text: &str| Value {
            property: prop("title"),
            uuid: Some(uuid.to_string()),
            kind: ValueKind::Text { text: text.to_string(), lang: None },
        };
        let book = Resource {
            values: vec![
                text("tQ3n", "Zeitglöcklein"),
                Value {
                    property: prop("citation"),
                    uuid: Some("Lm8x".to_string()),
                    kind: ValueKind::Text { text: "Hain 1234".to_string(), lang: Some("de".to_string()) },
                },
                text("0aZr", "Andachtsbuch"),
                text("Kc2w", "Mittagsgebet"),
                Value {
                    property: prop("pages"),
                    uuid: Some("e5Hv".to_string()),
                    kind: ValueKind::Integer(212),
                },
                Value {
                    property: prop("weight"),
                    uuid: Some("Wp1d".to_string()),
                    kind: ValueKind::Decimal("1.50".to_string()),
                },
                Value {
                    property: prop("digitised"),
                    uuid: Some("3fYs".to_string()),
                    kind: ValueKind::Boolean(true),
                },
                date(2_266_002, 2_266_366),
                Value {
                    property: prop("url"),
                    uuid: Some("gB7k".to_string()),
                    kind: ValueKind::Uri("https://example.org/b".to_string()),
                },
                Value {
                    property: prop("genre"),
                    uuid: Some("Rx4m".to_string()),
                    kind: ValueKind::ListNode(node("prayer")),
                },
                Value {
                    property: prop("hasAuthor"),
                    uuid: None,
                    kind: ValueKind::Link(res("m-person")),
                },
            ],
            file: Some(File::Document { asset: "b.pdf".to_string() }),
            ..resource("q-book", BOOK)
        };
        let two_parents = Resource {
            part_of: vec![res("q-book"), res("c-collection")],
            file: Some(File::Audio { asset: "a.mp3".to_string() }),
            ..resource("f-leaflet", PAGE)
        };
        let still = Resource {
            file: Some(File::StillImage { asset: "p.jp2".to_string(), width: 1200, height: 1800 }),
            ..page("x-page", "q-book", Some(3))
        };
        let moving = Resource {
            file: Some(File::MovingImage { asset: "v.mp4".to_string() }),
            ..page("a-page", "q-book", Some(1))
        };
        let region = Resource {
            values: vec![
                Value {
                    property: prop("hasGeometry"),
                    uuid: Some("gE0m".to_string()),
                    kind: ValueKind::Geometry(r#"{"type":"rectangle"}"#.to_string()),
                },
                Value {
                    property: prop("hasColor"),
                    uuid: Some("cO1r".to_string()),
                    kind: ValueKind::Color("#ff3333".to_string()),
                },
                Value {
                    property: prop("isRegionOf"),
                    uuid: None,
                    kind: ValueKind::Link(res("x-page")),
                },
            ],
            annotation: Some(Annotation {
                motivation: Motivation::Commenting,
                targets: vec![res("x-page")],
            }),
            ..resource("r-region", REGION)
        };
        let genre = |id: &str, parent: &str, position: u32| ListNode {
            labels: vec![LangString { text: id.to_string(), lang: Some("en".to_string()) }],
            ..list_node(id, Some(parent), Some(position))
        };

        let curation = vec![
            curated("r-region", "keep", None, "yes"),
            curated("q-book", "caption", Some("en"), "A book of hours"),
            curated("q-book", "slug", None, "zeitgloecklein"),
            curated("q-book", "caption", Some("de"), "Ein Stundenbuch"),
        ];

        let facts = snapshot(
            vec![
                book,
                still,
                page("k-page", "q-book", Some(3)),
                moving,
                page("b-page", "q-book", None),
                two_parents,
                resource("m-person", "http://www.knora.org/ontology/0803/incunabula#person"),
                resource("c-collection", "http://www.knora.org/ontology/0803/incunabula#collection"),
                Resource { seqnum: Some(7), ..resource("s-orphan", PAGE) },
                region,
            ],
            vec![
                genre("sermon", "genres", 2),
                list_node("genres", None, None),
                genre("prayer", "genres", 0),
                genre("morning-prayer", "prayer", 0),
                list_node("places", None, None),
                genre("basel", "places", 0),
            ],
        );
        ProjectSnapshot { curation, ..facts }
    }

    #[test]
    fn test_is_curation_name_accepts_lowercase_names_and_refuses_the_rest() {
        for name in ["slug", "date_display", "sort-key", "de", "a1"] {
            assert!(is_curation_name(name), "{name:?}");
        }
        for name in ["", "Slug", "1st", "_a", "-a", "a b", "a@b", "de-CH", "ü"] {
            assert!(!is_curation_name(name), "{name:?}");
        }
    }

    #[test]
    fn test_curated_values_sort_by_resource_key_and_language() {
        let mut values = vec![
            curated("b", "caption", Some("en"), "Red"),
            curated("b", "caption", Some("de"), "Rot"),
            curated("b", "caption", None, "plain"),
            curated("a", "size", None, "big"),
        ];

        values.sort();

        assert_eq!(
            values,
            vec![
                curated("a", "size", None, "big"),
                curated("b", "caption", None, "plain"),
                curated("b", "caption", Some("de"), "Rot"),
                curated("b", "caption", Some("en"), "Red"),
            ]
        );
    }

    #[test]
    fn test_violations_valid_snapshot_reports_nothing() {
        assert_eq!(violations("0803", &valid_snapshot()), vec![]);
    }

    #[test]
    fn test_violations_other_shortcode_reports_shortcode_mismatch() {
        let found = violations("0001", &snapshot(vec![], vec![]));

        assert_eq!(
            found,
            vec![Violation::ShortcodeMismatch { requested: "0001".to_string(), served: "0803".to_string() }]
        );
    }

    #[test]
    fn test_violations_repeated_resource_reports_duplicate_resource() {
        let other = Resource { label: "other".to_string(), ..resource("b", BOOK) };

        let found = violations("0803", &snapshot(vec![resource("b", BOOK), other], vec![]));

        assert_eq!(found, vec![Violation::DuplicateResource { resource: res("b") }]);
    }

    #[test]
    fn test_violations_version_ark_reports_malformed_data_ark() {
        let ark = DataArk("https://ark.dasch.swiss/ark:/72163/1/0803/b.20180604T085622Z".to_string());
        let bad = Resource { ark: ark.clone(), ..resource("b", BOOK) };

        let found = violations("0803", &snapshot(vec![bad], vec![]));

        assert_eq!(found, vec![Violation::MalformedDataArk { resource: res("b"), ark }]);
    }

    #[test]
    fn test_violations_lowercase_shortcode_ark_reports_malformed_data_ark() {
        let ark = DataArk("https://ark.dasch.swiss/ark:/72163/1/080a/b".to_string());
        let bad = Resource { ark: ark.clone(), ..resource("b", BOOK) };

        let found = violations("0803", &snapshot(vec![bad], vec![]));

        assert_eq!(found, vec![Violation::MalformedDataArk { resource: res("b"), ark }]);
    }

    #[test]
    fn test_violations_unescaped_dash_ark_reports_malformed_data_ark() {
        let ark = DataArk("https://ark.dasch.swiss/ark:/72163/1/0803/zz-booko".to_string());
        let bad = Resource { ark: ark.clone(), ..resource("zz-book", BOOK) };

        let found = violations("0803", &snapshot(vec![bad], vec![]));

        assert_eq!(found, vec![Violation::MalformedDataArk { resource: res("zz-book"), ark }]);
    }

    #[test]
    fn test_violations_foreign_resolver_ark_reports_malformed_data_ark() {
        let ark = DataArk("https://ark.example.org/ark:/72163/1/0803/b".to_string());
        let bad = Resource { ark: ark.clone(), ..resource("b", BOOK) };

        let found = violations("0803", &snapshot(vec![bad], vec![]));

        assert_eq!(found, vec![Violation::MalformedDataArk { resource: res("b"), ark }]);
    }

    #[test]
    fn test_violations_link_to_missing_resource_reports_dangling_link() {
        let linking = Resource {
            values: vec![Value {
                property: prop("hasAuthor"),
                uuid: None,
                kind: ValueKind::Link(res("gone")),
            }],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![linking], vec![]));

        assert_eq!(
            found,
            vec![Violation::DanglingLink {
                resource: res("b"),
                property: prop("hasAuthor"),
                target: res("gone")
            }]
        );
    }

    #[test]
    fn test_violations_annotation_target_missing_reports_dangling_target() {
        let annotation = Resource {
            annotation: Some(Annotation {
                motivation: Motivation::Linking,
                targets: vec![res("b"), res("gone")],
            }),
            ..resource("l", REGION)
        };

        let found = violations("0803", &snapshot(vec![annotation, resource("b", BOOK)], vec![]));

        assert_eq!(
            found,
            vec![Violation::DanglingTarget { resource: res("l"), target: res("gone") }]
        );
    }

    #[test]
    fn test_violations_annotation_without_target_reports_untargeted_annotation() {
        let annotation = Resource {
            annotation: Some(Annotation { motivation: Motivation::Commenting, targets: vec![] }),
            ..resource("r", REGION)
        };

        let found = violations("0803", &snapshot(vec![annotation], vec![]));

        assert_eq!(found, vec![Violation::UntargetedAnnotation { resource: res("r") }]);
    }

    #[test]
    fn test_violations_part_of_missing_resource_reports_dangling_parent() {
        let found = violations("0803", &snapshot(vec![page("p", "gone", Some(1))], vec![]));

        assert_eq!(
            found,
            vec![Violation::DanglingParent { resource: res("p"), parent: res("gone") }]
        );
    }

    #[test]
    fn test_violations_two_resource_part_of_cycle_reports_one_membership_cycle() {
        let found = violations("0803", &snapshot(vec![page("y", "x", None), page("x", "y", None)], vec![]));

        assert_eq!(found, vec![Violation::MembershipCycle { resource: res("x") }]);
    }

    #[test]
    fn test_violations_self_parent_reports_one_membership_cycle() {
        let found = violations("0803", &snapshot(vec![page("x", "x", None)], vec![]));

        assert_eq!(found, vec![Violation::MembershipCycle { resource: res("x") }]);
    }

    #[test]
    fn test_violations_cycle_reached_through_second_parent_reports_one_membership_cycle() {
        let looping = Resource { part_of: vec![res("root"), res("z")], ..resource("m", PAGE) };
        let resources = vec![page("z", "m", None), looping, resource("root", BOOK)];

        let found = violations("0803", &snapshot(resources, vec![]));

        assert_eq!(found, vec![Violation::MembershipCycle { resource: res("m") }]);
    }

    #[test]
    fn test_violations_cycle_entered_from_outside_reports_it_by_smallest_iri() {
        let resources = vec![page("a", "d", None), page("d", "c", None), page("c", "d", None)];

        let found = violations("0803", &snapshot(resources, vec![]));

        assert_eq!(found, vec![Violation::MembershipCycle { resource: res("c") }]);
    }

    #[test]
    fn test_violations_cycle_with_tail_reports_only_the_cycle() {
        let resources = vec![page("a", "b", None), page("b", "c", None), page("c", "b", None)];

        let found = violations("0803", &snapshot(resources, vec![]));

        assert_eq!(found, vec![Violation::MembershipCycle { resource: res("b") }]);
    }

    #[test]
    fn test_violations_two_disjoint_cycles_report_two_membership_cycles_in_order() {
        let resources = vec![
            page("y", "x", None),
            page("q", "p", None),
            page("x", "y", None),
            page("p", "q", None),
        ];

        let found = violations("0803", &snapshot(resources, vec![]));

        assert_eq!(
            found,
            vec![
                Violation::MembershipCycle { resource: res("p") },
                Violation::MembershipCycle { resource: res("x") },
            ]
        );
    }

    #[test]
    fn test_violations_deep_acyclic_chain_reports_nothing() {
        let depth = 100_000;
        let mut resources: Vec<Resource> = (0..depth)
            .map(|i| page(&format!("n{i:06}"), &format!("n{:06}", i + 1), None))
            .collect();
        resources.push(resource(&format!("n{depth:06}"), BOOK));

        assert_eq!(violations("0803", &snapshot(resources, vec![])), vec![]);
    }

    #[test]
    fn test_violations_list_value_naming_missing_node_reports_dangling_list_node() {
        let valued = Resource {
            values: vec![Value {
                property: prop("genre"),
                uuid: Some("n7Qa".to_string()),
                kind: ValueKind::ListNode(node("gone")),
            }],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![valued], vec![]));

        assert_eq!(
            found,
            vec![Violation::DanglingListNode {
                referrer: ListNodeReferrer::Value { resource: res("b"), property: prop("genre") },
                missing: node("gone"),
            }]
        );
    }

    #[test]
    fn test_violations_node_parent_missing_reports_dangling_list_node() {
        let found = violations("0803", &snapshot(vec![], vec![list_node("child", Some("gone"), Some(0))]));

        assert_eq!(
            found,
            vec![Violation::DanglingListNode {
                referrer: ListNodeReferrer::Parent { node: node("child") },
                missing: node("gone"),
            }]
        );
    }

    #[test]
    fn test_violations_repeated_list_node_reports_duplicate_list_node() {
        let other = ListNode {
            labels: vec![LangString { text: "other".to_string(), lang: None }],
            ..list_node("root", None, None)
        };

        let found = violations("0803", &snapshot(vec![], vec![list_node("root", None, None), other]));

        assert_eq!(found, vec![Violation::DuplicateListNode { node: node("root") }]);
    }

    #[test]
    fn test_violations_repeated_child_node_reports_only_duplicate_list_node() {
        let other = ListNode {
            labels: vec![LangString { text: "other".to_string(), lang: None }],
            ..list_node("x", Some("root"), Some(0))
        };
        let nodes = vec![
            list_node("root", None, None),
            list_node("x", Some("root"), Some(0)),
            other,
        ];

        let found = violations("0803", &snapshot(vec![], nodes));

        assert_eq!(found, vec![Violation::DuplicateListNode { node: node("x") }]);
    }

    #[test]
    fn test_violations_cycle_entered_above_its_smallest_iri_reports_it_by_smallest_iri() {
        let nodes = vec![
            list_node("a", Some("d"), Some(0)),
            list_node("d", Some("c"), Some(0)),
            list_node("c", Some("d"), Some(1)),
        ];

        let found = violations("0803", &snapshot(vec![], nodes));

        assert_eq!(found, vec![Violation::ListNodeCycle { node: node("c") }]);
    }

    #[test]
    fn test_violations_two_node_parent_loop_reports_one_list_node_cycle() {
        let nodes = vec![list_node("y", Some("x"), Some(0)), list_node("x", Some("y"), Some(0))];

        let found = violations("0803", &snapshot(vec![], nodes));

        assert_eq!(found, vec![Violation::ListNodeCycle { node: node("x") }]);
    }

    #[test]
    fn test_violations_node_own_parent_reports_one_list_node_cycle() {
        let found = violations("0803", &snapshot(vec![], vec![list_node("x", Some("x"), Some(0))]));

        assert_eq!(found, vec![Violation::ListNodeCycle { node: node("x") }]);
    }

    #[test]
    fn test_violations_siblings_sharing_position_report_duplicate_sibling_position() {
        let nodes = vec![
            list_node("root", None, None),
            list_node("zurich", Some("root"), Some(1)),
            list_node("basel", Some("root"), Some(0)),
            list_node("bern", Some("root"), Some(1)),
        ];

        let found = violations("0803", &snapshot(vec![], nodes));

        assert_eq!(
            found,
            vec![Violation::DuplicateSiblingPosition {
                parent: node("root"),
                position: 1,
                nodes: vec![node("bern"), node("zurich")],
            }]
        );
    }

    #[test]
    fn test_violations_roots_without_positions_report_nothing() {
        let nodes = vec![list_node("places", None, None), list_node("genres", None, None)];

        assert_eq!(violations("0803", &snapshot(vec![], nodes)), vec![]);
    }

    #[test]
    fn test_violations_single_point_date_reports_nothing() {
        let dated = Resource {
            values: vec![date(2_266_002, 2_266_002)],
            ..resource("b", BOOK)
        };

        assert_eq!(violations("0803", &snapshot(vec![dated], vec![])), vec![]);
    }

    #[test]
    fn test_violations_start_one_day_after_end_reports_inverted_date() {
        let dated = Resource {
            values: vec![date(2_266_003, 2_266_002)],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![dated], vec![]));

        assert_eq!(
            found,
            vec![Violation::InvertedDate { resource: res("b"), property: prop("pubdate") }]
        );
    }

    #[test]
    fn test_violations_several_in_unsorted_resources_return_documented_order() {
        let inverted = Resource { values: vec![date(2, 1)], ..resource("a", BOOK) };
        let linking = Resource {
            values: vec![Value {
                property: prop("hasAuthor"),
                uuid: None,
                kind: ValueKind::Link(res("gone")),
            }],
            ..resource("z", BOOK)
        };
        let text = |uuid: Option<&str>| Value {
            property: prop("title"),
            uuid: uuid.map(str::to_string),
            kind: ValueKind::Text { text: "Zeitglöcklein".to_string(), lang: None },
        };
        let uuidless = Resource { values: vec![text(None)], ..resource("y", BOOK) };
        let doubled = Resource {
            values: vec![text(Some("vX2")), text(Some("vX2"))],
            ..resource("d", BOOK)
        };
        let untargeted = Resource {
            annotation: Some(Annotation { motivation: Motivation::Commenting, targets: vec![] }),
            ..resource("b", REGION)
        };
        let targeting = Resource {
            annotation: Some(Annotation { motivation: Motivation::Linking, targets: vec![res("gone")] }),
            ..resource("x", REGION)
        };
        let bad_ark = DataArk("https://ark.dasch.swiss/ark:/72163/1/0803/q.20180604T085622Z".to_string());
        let malformed = Resource { ark: bad_ark.clone(), ..resource("q", BOOK) };
        let resources = vec![
            uuidless,
            malformed,
            untargeted,
            inverted,
            page("m", "gone", None),
            doubled,
            targeting,
            linking,
            page("c", "gone", None),
        ];

        let curation = vec![
            curated("z", "", None, "untitled"),
            curated("a", "caption", Some("de"), "Rot"),
            curated("nowhere", "caption", None, "plain"),
            curated("a", "caption", Some("de"), "Dunkelrot"),
        ];

        let found = violations("0803", &ProjectSnapshot { curation, ..snapshot(resources, vec![]) });

        let parent = |id: &str| Violation::DanglingParent { resource: res(id), parent: res("gone") };
        assert_eq!(
            found,
            vec![
                Violation::MalformedDataArk { resource: res("q"), ark: bad_ark },
                Violation::DanglingLink {
                    resource: res("z"),
                    property: prop("hasAuthor"),
                    target: res("gone")
                },
                Violation::DanglingTarget { resource: res("x"), target: res("gone") },
                Violation::UntargetedAnnotation { resource: res("b") },
                parent("c"),
                parent("m"),
                Violation::InvertedDate { resource: res("a"), property: prop("pubdate") },
                Violation::MissingValueUuid { resource: res("y"), property: prop("title") },
                Violation::DuplicateValueUuid { resource: res("d"), uuid: "vX2".to_string() },
                Violation::DanglingCuration { resource: res("nowhere") },
                Violation::DuplicateCuration {
                    resource: res("a"),
                    key: "caption".to_string(),
                    lang: Some("de".to_string())
                },
                Violation::MalformedCuration { resource: res("z"), key: String::new(), lang: None },
            ]
        );
    }

    #[test]
    fn test_violations_link_with_uuid_reports_link_with_value_uuid() {
        let linking = Resource {
            values: vec![Value {
                property: prop("hasAuthor"),
                uuid: Some("kL4".to_string()),
                kind: ValueKind::Link(res("m-person")),
            }],
            ..resource("b", BOOK)
        };
        let resources = vec![linking, resource("m-person", BOOK)];

        let found = violations("0803", &snapshot(resources, vec![]));

        assert_eq!(
            found,
            vec![Violation::LinkWithValueUuid { resource: res("b"), property: prop("hasAuthor") }]
        );
    }

    #[test]
    fn test_violations_uuids_repeated_several_times_report_one_duplicate_each_in_order() {
        let text = |uuid: &str| Value {
            property: prop("citation"),
            uuid: Some(uuid.to_string()),
            kind: ValueKind::Text { text: uuid.to_string(), lang: None },
        };
        let valued = Resource {
            values: vec![text("zz"), text("aa"), text("zz"), text("mm"), text("aa"), text("zz")],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![valued], vec![]));

        assert_eq!(
            found,
            vec![
                Violation::DuplicateValueUuid { resource: res("b"), uuid: "aa".to_string() },
                Violation::DuplicateValueUuid { resource: res("b"), uuid: "zz".to_string() },
            ]
        );
    }

    #[test]
    fn test_violations_value_without_uuid_reports_missing_value_uuid() {
        let valued = Resource {
            values: vec![Value {
                property: prop("title"),
                uuid: None,
                kind: ValueKind::Text { text: "Zeitglöcklein".to_string(), lang: None },
            }],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![valued], vec![]));

        assert_eq!(
            found,
            vec![Violation::MissingValueUuid { resource: res("b"), property: prop("title") }]
        );
    }

    #[test]
    fn test_violations_geometry_without_uuid_reports_missing_value_uuid() {
        let region = Resource {
            values: vec![Value {
                property: prop("hasGeometry"),
                uuid: None,
                kind: ValueKind::Geometry(r#"{"type":"rectangle"}"#.to_string()),
            }],
            ..resource("r", REGION)
        };

        let found = violations("0803", &snapshot(vec![region], vec![]));

        assert_eq!(
            found,
            vec![Violation::MissingValueUuid { resource: res("r"), property: prop("hasGeometry") }]
        );
    }

    #[test]
    fn test_violations_color_without_uuid_reports_missing_value_uuid() {
        let region = Resource {
            values: vec![Value {
                property: prop("hasColor"),
                uuid: None,
                kind: ValueKind::Color("#ff3333".to_string()),
            }],
            ..resource("r", REGION)
        };

        let found = violations("0803", &snapshot(vec![region], vec![]));

        assert_eq!(
            found,
            vec![Violation::MissingValueUuid { resource: res("r"), property: prop("hasColor") }]
        );
    }

    #[test]
    fn test_violations_link_without_uuid_reports_nothing() {
        let linking = Resource {
            values: vec![Value {
                property: prop("hasAuthor"),
                uuid: None,
                kind: ValueKind::Link(res("m-person")),
            }],
            ..resource("b", BOOK)
        };
        let resources = vec![linking, resource("m-person", BOOK)];

        assert_eq!(violations("0803", &snapshot(resources, vec![])), vec![]);
    }

    #[test]
    fn test_violations_two_values_sharing_uuid_report_duplicate_value_uuid() {
        let text = |text: &str| ValueKind::Text { text: text.to_string(), lang: None };
        let valued = Resource {
            values: vec![
                Value {
                    property: prop("title"),
                    uuid: Some("vX2".to_string()),
                    kind: text("Zeitglöcklein"),
                },
                Value {
                    property: prop("note"),
                    uuid: Some("aB9".to_string()),
                    kind: text("Basel"),
                },
                Value {
                    property: prop("title"),
                    uuid: Some("vX2".to_string()),
                    kind: text("Andachtsbuch"),
                },
            ],
            ..resource("b", BOOK)
        };

        let found = violations("0803", &snapshot(vec![valued], vec![]));

        assert_eq!(
            found,
            vec![Violation::DuplicateValueUuid { resource: res("b"), uuid: "vX2".to_string() }]
        );
    }

    #[test]
    fn test_violations_same_uuid_in_two_resources_reports_nothing() {
        let valued = |id: &str| Resource {
            values: vec![Value {
                property: prop("title"),
                uuid: Some("vX2".to_string()),
                kind: ValueKind::Text { text: id.to_string(), lang: None },
            }],
            ..resource(id, BOOK)
        };

        assert_eq!(violations("0803", &snapshot(vec![valued("b"), valued("a")], vec![])), vec![]);
    }

    fn curated_snapshot(curation: Vec<CuratedValue>) -> ProjectSnapshot {
        let resources = vec![resource("a", BOOK), resource("b", BOOK)];
        ProjectSnapshot { curation, ..snapshot(resources, vec![]) }
    }

    fn malformed(id: &str, key: &str, lang: Option<&str>) -> Violation {
        Violation::MalformedCuration {
            resource: res(id),
            key: key.to_string(),
            lang: lang.map(str::to_string),
        }
    }

    #[test]
    fn test_violations_curation_for_missing_resource_reports_one_dangling_curation() {
        let curation = vec![
            curated("gone", "colour", None, "red"),
            curated("gone", "size", None, "big"),
        ];

        let found = violations("0803", &curated_snapshot(curation));

        assert_eq!(found, vec![Violation::DanglingCuration { resource: res("gone") }]);
    }

    #[test]
    fn test_violations_curated_value_given_three_times_reports_one_duplicate_curation() {
        let curation = vec![
            curated("a", "caption", Some("en"), "Red"),
            curated("a", "caption", Some("en"), "Crimson"),
            curated("a", "caption", Some("en"), "Red"),
        ];

        let found = violations("0803", &curated_snapshot(curation));

        assert_eq!(
            found,
            vec![Violation::DuplicateCuration {
                resource: res("a"),
                key: "caption".to_string(),
                lang: Some("en".to_string())
            }]
        );
    }

    #[test]
    fn test_violations_empty_curated_text_reports_malformed_curation() {
        let found = violations("0803", &curated_snapshot(vec![curated("a", "colour", None, "")]));

        assert_eq!(found, vec![malformed("a", "colour", None)]);
    }

    #[test]
    fn test_violations_empty_curation_key_reports_malformed_curation() {
        let found = violations("0803", &curated_snapshot(vec![curated("a", "", None, "red")]));

        assert_eq!(found, vec![malformed("a", "", None)]);
    }

    #[test]
    fn test_violations_empty_curation_language_reports_malformed_curation() {
        let found = violations("0803", &curated_snapshot(vec![curated("a", "caption", Some(""), "Red")]));

        assert_eq!(found, vec![malformed("a", "caption", Some(""))]);
    }

    #[test]
    fn test_violations_uppercase_curation_key_reports_malformed_curation() {
        let found = violations("0803", &curated_snapshot(vec![curated("a", "Colour", None, "red")]));

        assert_eq!(found, vec![malformed("a", "Colour", None)]);
    }

    #[test]
    fn test_violations_malformed_value_given_twice_reports_two_malformed_and_one_duplicate() {
        let curation = vec![curated("a", "colour", None, ""), curated("a", "colour", None, "")];

        let found = violations("0803", &curated_snapshot(curation));

        assert_eq!(
            found,
            vec![
                Violation::DuplicateCuration { resource: res("a"), key: "colour".to_string(), lang: None },
                malformed("a", "colour", None),
                malformed("a", "colour", None),
            ]
        );
    }

    #[test]
    fn test_violations_one_key_in_two_languages_reports_nothing() {
        let curation = vec![
            curated("a", "caption", Some("de"), "Rot"),
            curated("a", "caption", Some("en"), "Red"),
        ];

        assert_eq!(violations("0803", &curated_snapshot(curation)), vec![]);
    }

    #[test]
    fn test_violations_one_key_tagged_and_untagged_reports_nothing() {
        let curation = vec![
            curated("a", "caption", None, "plain"),
            curated("a", "caption", Some("en"), "Red"),
        ];

        assert_eq!(violations("0803", &curated_snapshot(curation)), vec![]);
    }

    #[test]
    fn test_violations_one_key_on_two_resources_reports_nothing() {
        let curation = vec![curated("a", "colour", None, "red"), curated("b", "colour", None, "red")];

        assert_eq!(violations("0803", &curated_snapshot(curation)), vec![]);
    }

    #[test]
    fn test_violations_padded_curation_text_reports_nothing() {
        let curation = vec![curated("a", "colour", None, " red\t")];

        assert_eq!(violations("0803", &curated_snapshot(curation)), vec![]);
    }

    #[test]
    fn test_display_dangling_curation_names_resource() {
        let violation = Violation::DanglingCuration { resource: res("gone") };

        assert!(violation.to_string().contains("http://rdfh.ch/0803/gone"), "{violation}");
    }

    #[test]
    fn test_display_duplicate_curation_shows_key_and_language() {
        let duplicate = |key: &str, lang: Option<&str>| Violation::DuplicateCuration {
            resource: res("a"),
            key: key.to_string(),
            lang: lang.map(str::to_string),
        };

        let tagged = duplicate("teaser", Some("de")).to_string();
        let untagged = duplicate("slug", None).to_string();

        assert!(tagged.ends_with("curated value teaser@de"), "{tagged}");
        assert!(untagged.ends_with("curated value slug"), "{untagged}");
    }

    #[test]
    fn test_display_malformed_curation_quotes_empty_key() {
        let violation = malformed("a", "", None);

        assert!(violation.to_string().contains(r#"curated value "" whose"#), "{violation}");
    }
}
