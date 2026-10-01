//! The interim-DAO quads of one project, mapped to the port's snapshot.
//!
//! The order is drop, then validate, then map: annotations are never served, so a broken fact
//! inside one is never checked; only the file-wide rules of `Index::build` (graph, blank nodes)
//! apply to them. A served fact that breaks `FORMAT.md` is an [`Invalid`]; what the crate doc lists
//! as omitted is omitted.
//!
//! A newly served fact reads a predicate `FORMAT.md` allows once with [`one`], and each way the
//! file can break it gets an [`InvalidFact`] variant and a test in `snapshot_tests.rs` that fails
//! without the check.

use std::collections::BTreeMap;
use std::str::FromStr;

use cpe_ports::ProjectSnapshot;
use oxrdf::{GraphNameRef, NamedNodeRef, NamedOrBlankNodeRef, Quad, TermRef};

use crate::vocab::{PROJECT_GRAPH_PREFIX, RDF_TYPE, XSD_INTEGER};
use crate::InvalidFact;

mod dates;
mod files;
mod list_nodes;
mod resources;
mod values;

/// A broken fact: the node that carries it and the rule it breaks.
pub(crate) struct Invalid {
    pub subject: String,
    pub reason: InvalidFact,
}

fn invalid(subject: &str, reason: InvalidFact) -> Invalid {
    Invalid { subject: subject.to_string(), reason }
}

pub(crate) fn map(shortcode: &str, quads: &[Quad]) -> Result<ProjectSnapshot, Invalid> {
    let index = Index::build(shortcode, quads)?;
    Ok(ProjectSnapshot {
        shortcode: shortcode.to_string(),
        list_nodes: list_nodes::map(&index)?,
        resources: resources::map(&index)?,
    })
}

/// One subject's facts, sorted by predicate and object, each fact once.
type Facts<'a> = [(NamedNodeRef<'a>, TermRef<'a>)];

/// The file's facts by subject IRI, borrowed from the parsed quads.
struct Index<'a> {
    subjects: BTreeMap<&'a str, Vec<(NamedNodeRef<'a>, TermRef<'a>)>>,
}

impl<'a> Index<'a> {
    /// Rejects a quad outside the project graph and a blank node; deduplicates the rest.
    fn build(shortcode: &str, quads: &'a [Quad]) -> Result<Self, Invalid> {
        let graph = format!("{PROJECT_GRAPH_PREFIX}{shortcode}");
        let mut subjects: BTreeMap<&str, Vec<_>> = BTreeMap::new();
        for quad in quads {
            let quad = quad.as_ref();
            let subject = match quad.subject {
                NamedOrBlankNodeRef::NamedNode(node) => node.as_str(),
                NamedOrBlankNodeRef::BlankNode(node) => {
                    return Err(invalid(&node.to_string(), InvalidFact::BlankNodeSubject));
                }
            };
            match quad.graph_name {
                GraphNameRef::NamedNode(name) if name.as_str() == graph => {}
                GraphNameRef::NamedNode(name) => {
                    return Err(invalid(subject, InvalidFact::ForeignGraph { graph: name.as_str().to_string() }));
                }
                GraphNameRef::BlankNode(name) => {
                    return Err(invalid(subject, InvalidFact::ForeignGraph { graph: name.to_string() }));
                }
                GraphNameRef::DefaultGraph => {
                    return Err(invalid(
                        subject,
                        InvalidFact::ForeignGraph { graph: "the default graph".to_string() },
                    ));
                }
            }
            if let TermRef::BlankNode(_) = quad.object {
                return Err(invalid(subject, InvalidFact::BlankNodeObject));
            }
            subjects.entry(subject).or_default().push((quad.predicate, quad.object));
        }
        for facts in subjects.values_mut() {
            facts.sort_unstable_by(|a, b| (a.0, term_key(a.1)).cmp(&(b.0, term_key(b.1))));
            facts.dedup();
        }
        Ok(Self { subjects })
    }

    fn facts(&self, subject: &str) -> &Facts<'a> {
        self.subjects.get(subject).map_or(&[], Vec::as_slice)
    }

    fn is_a(&self, subject: &str, class: NamedNodeRef<'static>) -> bool {
        objects(self.facts(subject), RDF_TYPE).any(|object| object == class.into())
    }

    /// Every subject typed `class`, in IRI order.
    fn typed<'s>(&'s self, class: NamedNodeRef<'static>) -> impl Iterator<Item = (&'a str, &'s Facts<'a>)> + 's {
        self.subjects
            .iter()
            .filter(move |(subject, _)| self.is_a(subject, class))
            .map(|(subject, facts)| (*subject, facts.as_slice()))
    }
}

/// A total order over terms, which `oxrdf` does not provide.
fn term_key(term: TermRef<'_>) -> (u8, &str, &str, &str) {
    match term {
        TermRef::NamedNode(node) => (0, node.as_str(), "", ""),
        TermRef::BlankNode(node) => (1, node.as_str(), "", ""),
        TermRef::Literal(literal) => (
            2,
            literal.value(),
            literal.datatype().as_str(),
            literal.language().unwrap_or_default(),
        ),
    }
}

fn objects<'f, 'a>(facts: &'f Facts<'a>, predicate: NamedNodeRef<'static>) -> impl Iterator<Item = TermRef<'a>> + 'f {
    facts
        .iter()
        .filter(move |(fact, _)| *fact == predicate)
        .map(|(_, object)| *object)
}

/// The object under a predicate `FORMAT.md` allows once on `subject`, if any; a second is
/// [`InvalidFact::RepeatedPredicate`]. `rdf:value` and `dao:sourceProperty` map that to their own
/// variants.
fn one<'a>(subject: &str, facts: &Facts<'a>, predicate: NamedNodeRef<'static>) -> Result<Option<TermRef<'a>>, Invalid> {
    let mut found = objects(facts, predicate);
    let object = found.next();
    if found.next().is_some() {
        return Err(invalid(
            subject,
            InvalidFact::RepeatedPredicate { predicate: predicate.as_str().to_string() },
        ));
    }
    Ok(object)
}

fn named(term: TermRef<'_>) -> Option<NamedNodeRef<'_>> {
    match term {
        TermRef::NamedNode(node) => Some(node),
        TermRef::BlankNode(_) | TermRef::Literal(_) => None,
    }
}

/// A term as text: a literal's lexical form, a node's IRI or blank-node id.
fn lexical(term: TermRef<'_>) -> String {
    match term {
        TermRef::NamedNode(node) => node.as_str().to_string(),
        TermRef::BlankNode(node) => node.to_string(),
        TermRef::Literal(literal) => literal.value().to_string(),
    }
}

/// An `xsd:integer` literal in canonical form that fits `T`; anything else is
/// [`InvalidFact::UnfitLiteral`].
fn integer<T: FromStr>(subject: &str, predicate: NamedNodeRef<'_>, term: TermRef<'_>) -> Result<T, Invalid> {
    match term {
        TermRef::Literal(literal) if literal.datatype() == XSD_INTEGER && canonical_integer(literal.value()) => {
            literal.value().parse().ok()
        }
        _ => None,
    }
    .ok_or_else(|| unfit(subject, predicate, term))
}

/// The XSD 1.1 canonical `xsd:integer` lexical form, the only one `FORMAT.md` §1 writes.
fn canonical_integer(lexical: &str) -> bool {
    match lexical.strip_prefix('-') {
        Some(digits) => digits != "0" && canonical_digits(digits),
        None => canonical_digits(lexical),
    }
}

/// The XSD 1.1 canonical `xsd:decimal` lexical form: an integral value has no fraction, any other
/// no trailing zero.
fn canonical_decimal(lexical: &str) -> bool {
    match lexical.split_once('.') {
        None => canonical_integer(lexical),
        Some((whole, fraction)) => {
            canonical_digits(whole.strip_prefix('-').unwrap_or(whole))
                && !fraction.is_empty()
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
                && !fraction.ends_with('0')
        }
    }
}

/// Decimal digits with no leading zero, or `0` itself.
fn canonical_digits(digits: &str) -> bool {
    digits == "0"
        || (digits.starts_with(|c: char| matches!(c, '1'..='9')) && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn unfit(subject: &str, predicate: NamedNodeRef<'_>, term: TermRef<'_>) -> Invalid {
    invalid(
        subject,
        InvalidFact::UnfitLiteral {
            predicate: predicate.as_str().to_string(),
            lexical: lexical(term),
        },
    )
}
