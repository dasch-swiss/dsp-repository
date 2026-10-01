//! The project's list nodes (`FORMAT.md` §9). They are never omitted, so a reference to a node that
//! is not one is a broken file.

use std::collections::BTreeSet;

use cpe_ports::{LangString, ListNode, ListNodeIri};
use oxrdf::vocab::xsd;
use oxrdf::TermRef;

use super::{integer, invalid, lexical, named, objects, one, unfit, Facts, Index, Invalid};
use crate::vocab::{DAO_LIST_NODE_POSITION, SKOS_BROADER, SKOS_CONCEPT, SKOS_PREF_LABEL};
use crate::InvalidFact;

pub(super) fn map(index: &Index<'_>) -> Result<Vec<ListNode>, Invalid> {
    index
        .typed(SKOS_CONCEPT)
        .map(|(iri, facts)| list_node(index, iri, facts))
        .collect()
}

fn list_node(index: &Index<'_>, iri: &str, facts: &Facts<'_>) -> Result<ListNode, Invalid> {
    let parent = one(iri, facts, SKOS_BROADER)?
        .map(|parent| match named(parent) {
            Some(node) if index.is_a(node.as_str(), SKOS_CONCEPT) => Ok(ListNodeIri(node.as_str().to_string())),
            _ => Err(invalid(iri, InvalidFact::UnknownListParent { parent: lexical(parent) })),
        })
        .transpose()?;
    let position = one(iri, facts, DAO_LIST_NODE_POSITION)?
        .map(|position| integer(iri, DAO_LIST_NODE_POSITION, position))
        .transpose()?;
    if parent.is_none() && position.is_some() {
        return Err(invalid(iri, InvalidFact::PositionedListRoot));
    }
    Ok(ListNode {
        iri: ListNodeIri(iri.to_string()),
        parent,
        position,
        labels: labels(iri, facts)?,
    })
}

/// Plain or language-tagged literals, at most one per language, untagged counting as one.
fn labels(iri: &str, facts: &Facts<'_>) -> Result<Vec<LangString>, Invalid> {
    let mut labels = Vec::new();
    let mut languages = BTreeSet::new();
    for label in objects(facts, SKOS_PREF_LABEL) {
        let lang = match label {
            TermRef::Literal(literal) => match literal.language() {
                Some(tag) => Some(tag.to_string()),
                None if literal.datatype() == xsd::STRING => None,
                None => return Err(unfit(iri, SKOS_PREF_LABEL, label)),
            },
            TermRef::NamedNode(_) | TermRef::BlankNode(_) => return Err(unfit(iri, SKOS_PREF_LABEL, label)),
        };
        if !languages.insert(lang.clone()) {
            return Err(invalid(iri, InvalidFact::DuplicateListLabel));
        }
        labels.push(LangString { text: lexical(label), lang });
    }
    Ok(labels)
}
