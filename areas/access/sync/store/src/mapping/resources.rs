//! The served resources (`FORMAT.md` §2, §7, §10).

use std::collections::BTreeSet;

use cpe_ports::{Annotation, ClassIri, Motivation, Resource, ResourceIri};
use oxrdf::vocab::xsd;
use oxrdf::TermRef;

use super::{files, integer, invalid, lexical, named, objects, one, unfit, values, Facts, Index, Invalid};
use crate::vocab::{
    DAO_IS_PART_OF, DAO_RESOURCE, DAO_SEQNUM, DAO_SOURCE_CLASS, OA_ANNOTATION, OA_COMMENTING, OA_HAS_TARGET,
    OA_HIGHLIGHTING, OA_LINKING, OA_MOTIVATED_BY, RDFS_LABEL, RDF_TYPE,
};
use crate::InvalidFact;

/// Every `dao:Resource`, annotations included. An annotation is identified by `oa:Annotation`
/// alone, never by `dao:sourceClass`: a project may subclass `kb:Region` or `kb:LinkObj`
/// (`FORMAT.md` §10).
pub(super) fn map(index: &Index<'_>) -> Result<Vec<Resource>, Invalid> {
    check_annotation_markers(index)?;
    let served: BTreeSet<&str> = index.typed(DAO_RESOURCE).map(|(iri, _)| iri).collect();
    // Value and representation nodes already reached: each belongs to one resource.
    let mut claimed = BTreeSet::new();
    served
        .iter()
        .map(|iri| resource(index, iri, index.facts(iri), &served, &mut claimed))
        .collect()
}

/// Over every subject, served or not: `oa:Annotation` marks only a `dao:Resource`, and the
/// annotation predicates sit only on a subject marked `oa:Annotation`.
fn check_annotation_markers(index: &Index<'_>) -> Result<(), Invalid> {
    for (&subject, facts) in &index.subjects {
        if index.is_a(subject, OA_ANNOTATION) {
            if !index.is_a(subject, DAO_RESOURCE) {
                return Err(invalid(subject, InvalidFact::UnservedAnnotation));
            }
        } else if let Some((predicate, _)) = facts
            .iter()
            .find(|(predicate, _)| [OA_MOTIVATED_BY, OA_HAS_TARGET].contains(predicate))
        {
            return Err(invalid(
                subject,
                InvalidFact::StrayAnnotationFact { predicate: predicate.as_str().to_string() },
            ));
        }
    }
    Ok(())
}

fn resource<'a>(
    index: &Index<'a>,
    iri: &str,
    facts: &Facts<'a>,
    served: &BTreeSet<&str>,
    claimed: &mut BTreeSet<&'a str>,
) -> Result<Resource, Invalid> {
    if let Some(other) = objects(facts, RDF_TYPE)
        .find(|class| ![TermRef::from(DAO_RESOURCE), TermRef::from(OA_ANNOTATION)].contains(class))
    {
        return Err(invalid(iri, InvalidFact::ExtraResourceType { class: lexical(other) }));
    }
    let class = one(iri, facts, DAO_SOURCE_CLASS)?
        .and_then(named)
        .ok_or_else(|| invalid(iri, InvalidFact::MissingClass))?;
    let label = label(iri, facts)?;
    let values = values::map(index, iri, facts, served, claimed)?;
    let file = files::map(index, iri, facts, claimed)?;
    let part_of = part_of(iri, facts, served)?;
    let seqnum = one(iri, facts, DAO_SEQNUM)?
        .map(|seqnum| integer(iri, DAO_SEQNUM, seqnum))
        .transpose()?;
    let annotation = if index.is_a(iri, OA_ANNOTATION) {
        Some(annotation(iri, facts, served)?)
    } else {
        None
    };
    Ok(Resource {
        iri: ResourceIri(iri.to_string()),
        class: ClassIri(class.as_str().to_string()),
        label,
        values,
        file,
        part_of,
        seqnum,
        annotation,
    })
}

/// The parents; one that is not a resource of the file is [`InvalidFact::UnknownParent`].
fn part_of(iri: &str, facts: &Facts<'_>, served: &BTreeSet<&str>) -> Result<Vec<ResourceIri>, Invalid> {
    let mut parents = Vec::new();
    for object in objects(facts, DAO_IS_PART_OF) {
        match named(object) {
            Some(parent) if served.contains(parent.as_str()) => parents.push(ResourceIri(parent.as_str().to_string())),
            _ => return Err(invalid(iri, InvalidFact::UnknownParent { parent: lexical(object) })),
        }
    }
    Ok(parents)
}

/// Exactly one `oa:motivatedBy` of the three `FORMAT.md` §10 writes, and at least one
/// `oa:hasTarget`, each a resource of the file. Unlike a link, a target that is not a resource of
/// the file is refused: it can never be an untyped value node.
fn annotation(iri: &str, facts: &Facts<'_>, served: &BTreeSet<&str>) -> Result<Annotation, Invalid> {
    let term = one(iri, facts, OA_MOTIVATED_BY)?.ok_or_else(|| invalid(iri, InvalidFact::MissingMotivation))?;
    let motivation = match named(term) {
        Some(node) if node == OA_COMMENTING => Motivation::Commenting,
        Some(node) if node == OA_HIGHLIGHTING => Motivation::Highlighting,
        Some(node) if node == OA_LINKING => Motivation::Linking,
        Some(node) => {
            return Err(invalid(
                iri,
                InvalidFact::UnknownMotivation { motivation: node.as_str().to_string() },
            ));
        }
        None => return Err(unfit(iri, OA_MOTIVATED_BY, term)),
    };
    // In `Facts` order, which for named nodes is IRI order without repeats, as `Annotation`
    // promises.
    let mut targets = Vec::new();
    for object in objects(facts, OA_HAS_TARGET) {
        let target = named(object).ok_or_else(|| unfit(iri, OA_HAS_TARGET, object))?;
        if !served.contains(target.as_str()) {
            return Err(invalid(iri, InvalidFact::UnknownTarget { target: target.as_str().to_string() }));
        }
        targets.push(ResourceIri(target.as_str().to_string()));
    }
    if targets.is_empty() {
        return Err(invalid(iri, InvalidFact::MissingAnnotationTarget));
    }
    Ok(Annotation { motivation, targets })
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
