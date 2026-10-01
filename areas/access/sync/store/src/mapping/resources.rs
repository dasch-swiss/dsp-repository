//! The served resources (`FORMAT.md` §2, §7, §10).

use std::collections::BTreeSet;

use cpe_ports::{ClassIri, Resource, ResourceIri};
use oxrdf::vocab::xsd;
use oxrdf::TermRef;

use super::{files, integer, invalid, lexical, named, objects, one, unfit, values, Facts, Index, Invalid};
use crate::vocab::{DAO_IS_PART_OF, DAO_RESOURCE, DAO_SEQNUM, DAO_SOURCE_CLASS, OA_ANNOTATION, RDFS_LABEL, RDF_TYPE};
use crate::InvalidFact;

/// Every resource but the annotations, which are identified by `oa:Annotation` alone, never by
/// `dao:sourceClass`: a project may subclass `kb:Region` or `kb:LinkObj` (`FORMAT.md` §10).
pub(super) fn map(index: &Index<'_>) -> Result<Vec<Resource>, Invalid> {
    let served: BTreeSet<&str> = index
        .typed(DAO_RESOURCE)
        .filter(|(iri, _)| !index.is_a(iri, OA_ANNOTATION))
        .map(|(iri, _)| iri)
        .collect();
    // Value and representation nodes already reached: each belongs to one resource.
    let mut claimed = BTreeSet::new();
    served
        .iter()
        .map(|iri| resource(index, iri, index.facts(iri), &served, &mut claimed))
        .collect()
}

fn resource<'a>(
    index: &Index<'a>,
    iri: &str,
    facts: &Facts<'a>,
    served: &BTreeSet<&str>,
    claimed: &mut BTreeSet<&'a str>,
) -> Result<Resource, Invalid> {
    if let Some(other) = objects(facts, RDF_TYPE).find(|class| *class != TermRef::from(DAO_RESOURCE)) {
        return Err(invalid(iri, InvalidFact::ExtraResourceType { class: lexical(other) }));
    }
    let class = one(iri, facts, DAO_SOURCE_CLASS)?
        .and_then(named)
        .ok_or_else(|| invalid(iri, InvalidFact::MissingClass))?;
    let label = label(iri, facts)?;
    let values = values::map(index, iri, facts, served, claimed)?;
    let file = files::map(index, iri, facts, claimed)?;
    let part_of = part_of(index, iri, facts, served)?;
    let seqnum = one(iri, facts, DAO_SEQNUM)?
        .map(|seqnum| integer(iri, DAO_SEQNUM, seqnum))
        .transpose()?;
    Ok(Resource {
        iri: ResourceIri(iri.to_string()),
        class: ClassIri(class.as_str().to_string()),
        label,
        values,
        file,
        part_of,
        seqnum,
    })
}

/// The served parents. A parent that is an annotation is left out; one that is not a resource of
/// the file is [`InvalidFact::UnknownParent`].
fn part_of(
    index: &Index<'_>,
    iri: &str,
    facts: &Facts<'_>,
    served: &BTreeSet<&str>,
) -> Result<Vec<ResourceIri>, Invalid> {
    let mut parents = Vec::new();
    for object in objects(facts, DAO_IS_PART_OF) {
        match named(object) {
            Some(parent) if served.contains(parent.as_str()) => parents.push(ResourceIri(parent.as_str().to_string())),
            Some(parent) if index.is_a(parent.as_str(), DAO_RESOURCE) => {}
            _ => return Err(invalid(iri, InvalidFact::UnknownParent { parent: lexical(object) })),
        }
    }
    Ok(parents)
}

/// Exactly one `rdfs:label`, a plain literal.
fn label(iri: &str, facts: &Facts<'_>) -> Result<String, Invalid> {
    let mut labels = objects(facts, RDFS_LABEL);
    let reason = match (labels.next(), labels.next()) {
        (Some(_), Some(_)) => InvalidFact::RepeatedLabel,
        (Some(TermRef::Literal(label)), None) if label.language().is_some() => InvalidFact::TaggedLabel,
        (Some(TermRef::Literal(label)), None) if label.datatype() == xsd::STRING => {
            return Ok(label.value().to_string());
        }
        (Some(term), None) => return Err(unfit(iri, RDFS_LABEL, term)),
        (None, _) => InvalidFact::MissingLabel,
    };
    Err(invalid(iri, reason))
}
