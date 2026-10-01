//! A resource's values and links (`FORMAT.md` §3–§6), in the port's order.

use std::collections::BTreeSet;

use cpe_ports::{ListNodeIri, PropertyIri, ResourceIri, Value, ValueKind};
use oxrdf::vocab::xsd;
use oxrdf::{NamedNodeRef, TermRef};

use super::{canonical_decimal, dates, integer, invalid, lexical, named, one, unfit, Facts, Index, Invalid};
use crate::vocab::{
    DAO_DATE, DAO_NAMESPACE, DAO_SOURCE_LIST_NODE, DAO_SOURCE_PROPERTY, DAO_VALUE, DAO_VALUE_HAS_ORDER,
    DAO_VALUE_HAS_UUID, OA_NAMESPACE, RDFS_NAMESPACE, RDF_NAMESPACE, RDF_VALUE, SKOS_CONCEPT, VALUE_IRI_PREFIX,
    XSD_ANY_URI, XSD_BOOLEAN, XSD_DECIMAL, XSD_INTEGER,
};
use crate::InvalidFact;

/// Within one property: order ascending, a missing order counting as 0, ties by UUID; links, which
/// have neither, by target IRI. Properties are sorted only so the output is deterministic.
///
/// `claimed` holds the nodes of the resources mapped so far: a value node belongs to one resource.
pub(super) fn map<'a>(
    index: &Index<'a>,
    resource: &str,
    facts: &Facts<'a>,
    served: &BTreeSet<&str>,
    claimed: &mut BTreeSet<&'a str>,
) -> Result<Vec<Value>, Invalid> {
    let mut ordered: Vec<(i64, Value)> = Vec::new();
    for &(predicate, object) in facts {
        let Some(target) = named(object) else {
            if is_link(predicate) {
                return Err(unfit(resource, predicate, object));
            }
            continue;
        };
        // A value edge, a link to a served resource, or dropped: the object of a format predicate,
        // an annotation, or the one accepted violation the crate doc names.
        if index.is_a(target.as_str(), DAO_VALUE) {
            let value = value_node(index, target.as_str(), predicate)?;
            if !claimed.insert(target.as_str()) {
                return Err(invalid(
                    resource,
                    InvalidFact::DuplicateValueUuid { uuid: value.uuid.to_string() },
                ));
            }
            if let Some(kind) = value.kind {
                ordered.push((
                    value.order,
                    Value {
                        property: PropertyIri(predicate.as_str().to_string()),
                        uuid: Some(value.uuid.to_string()),
                        kind,
                    },
                ));
            }
        } else if is_link(predicate) && served.contains(target.as_str()) {
            ordered.push((
                0,
                Value {
                    property: PropertyIri(predicate.as_str().to_string()),
                    uuid: None,
                    kind: ValueKind::Link(ResourceIri(target.as_str().to_string())),
                },
            ));
        }
    }
    ordered.sort_unstable_by(|(a_order, a), (b_order, b)| {
        (a.property.as_str(), a_order, tie(a)).cmp(&(b.property.as_str(), b_order, tie(b)))
    });
    Ok(ordered.into_iter().map(|(_, value)| value).collect())
}

/// A link's predicate lies outside the format's own namespaces (`FORMAT.md` §3.1).
fn is_link(predicate: NamedNodeRef<'_>) -> bool {
    let iri = predicate.as_str();
    ![DAO_NAMESPACE, OA_NAMESPACE, RDF_NAMESPACE, RDFS_NAMESPACE]
        .iter()
        .any(|namespace| iri.starts_with(namespace))
}

fn tie(value: &Value) -> &str {
    match (&value.uuid, &value.kind) {
        (Some(uuid), _) => uuid,
        (None, ValueKind::Link(target)) => target.as_str(),
        (None, _) => "",
    }
}

struct ValueNode<'a> {
    uuid: &'a str,
    order: i64,
    /// `None` for a kind the port does not list, which is omitted.
    kind: Option<ValueKind>,
}

/// The node `edge` points at, which `FORMAT.md` §3 names after its UUID and gives the edge's
/// predicate as its one source property.
fn value_node<'a>(index: &Index<'a>, node: &str, edge: NamedNodeRef<'_>) -> Result<ValueNode<'a>, Invalid> {
    let facts = index.facts(node);
    let uuid = match one(node, facts, DAO_VALUE_HAS_UUID)? {
        None => return Err(invalid(node, InvalidFact::MissingValueUuid)),
        Some(TermRef::Literal(uuid)) if uuid.datatype() == xsd::STRING => uuid.value(),
        Some(other) => return Err(unfit(node, DAO_VALUE_HAS_UUID, other)),
    };
    let base64url = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if uuid.is_empty() || !uuid.chars().all(base64url) || node.strip_prefix(VALUE_IRI_PREFIX) != Some(uuid) {
        return Err(invalid(node, InvalidFact::UnfitValueUuid { uuid: uuid.to_string() }));
    }
    let content = one(node, facts, RDF_VALUE).map_err(|_| invalid(node, InvalidFact::RepeatedValue))?;
    let property = one(node, facts, DAO_SOURCE_PROPERTY)
        .map_err(|_| invalid(node, InvalidFact::RepeatedSourceProperty))?
        .ok_or_else(|| invalid(node, InvalidFact::MissingSourceProperty))?;
    if named(property) != Some(edge) {
        return Err(invalid(
            node,
            InvalidFact::UnfitSourceProperty { edge: edge.as_str().to_string() },
        ));
    }
    let kind = match (one(node, facts, DAO_SOURCE_LIST_NODE)?, content) {
        (Some(_), Some(_)) => return Err(invalid(node, InvalidFact::ListValueWithContent)),
        (Some(list_node), None) => match named(list_node) {
            Some(list_node) if index.is_a(list_node.as_str(), SKOS_CONCEPT) => {
                Some(ValueKind::ListNode(ListNodeIri(list_node.as_str().to_string())))
            }
            _ => return Err(invalid(node, InvalidFact::UnknownListNode { node: lexical(list_node) })),
        },
        (None, Some(content)) => kind(node, facts, content)?,
        (None, None) => return Err(invalid(node, InvalidFact::MissingValueContent)),
    };
    let order = one(node, facts, DAO_VALUE_HAS_ORDER)?
        .map(|order| integer(node, DAO_VALUE_HAS_ORDER, order))
        .transpose()?
        .unwrap_or(0);
    Ok(ValueNode { uuid, order, kind })
}

/// The kind is the datatype of `rdf:value` (`FORMAT.md` §4); one the port does not list is
/// omitted, never read as text.
fn kind(node: &str, facts: &Facts<'_>, content: TermRef<'_>) -> Result<Option<ValueKind>, Invalid> {
    let TermRef::Literal(literal) = content else {
        return Err(unfit(node, RDF_VALUE, content));
    };
    let text = literal.value();
    if let Some(lang) = literal.language() {
        return Ok(Some(ValueKind::Text { text: text.to_string(), lang: Some(lang.to_string()) }));
    }
    let datatype = literal.datatype();
    Ok(Some(if datatype == xsd::STRING {
        ValueKind::Text { text: text.to_string(), lang: None }
    } else if datatype == XSD_INTEGER {
        ValueKind::Integer(integer(node, RDF_VALUE, content)?)
    } else if datatype == XSD_DECIMAL {
        if !canonical_decimal(text) {
            return Err(unfit(node, RDF_VALUE, content));
        }
        ValueKind::Decimal(text.to_string())
    } else if datatype == XSD_BOOLEAN {
        ValueKind::Boolean(match text {
            "true" => true,
            "false" => false,
            _ => return Err(unfit(node, RDF_VALUE, content)),
        })
    } else if datatype == XSD_ANY_URI {
        ValueKind::Uri(text.to_string())
    } else if datatype == DAO_DATE {
        ValueKind::Date(dates::map(node, facts)?)
    } else {
        return Ok(None);
    }))
}
