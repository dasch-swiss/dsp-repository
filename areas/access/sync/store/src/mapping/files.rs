//! A resource's representation (`FORMAT.md` §8). One whose `dao:representationType` the port has no
//! `File` for is omitted, as §8 lets a reader do; any other broken representation is invalid.

use std::collections::BTreeSet;

use cpe_ports::File;
use oxrdf::vocab::xsd;
use oxrdf::{NamedNodeRef, TermRef};

use super::{integer, invalid, lexical, named, one, unfit, Facts, Index, Invalid};
use crate::vocab::{
    DAO_AUDIO_REPRESENTATION, DAO_DIM_X, DAO_DIM_Y, DAO_DOCUMENT_REPRESENTATION, DAO_HAS_REPRESENTATION,
    DAO_INTERNAL_FILENAME, DAO_MOVING_IMAGE_REPRESENTATION, DAO_REPRESENTATION, DAO_REPRESENTATION_TYPE,
    DAO_STILL_IMAGE_REPRESENTATION,
};
use crate::InvalidFact;

/// `claimed` holds the nodes of the resources mapped so far: a representation belongs to one
/// resource.
pub(super) fn map<'a>(
    index: &Index<'a>,
    iri: &str,
    resource: &Facts<'a>,
    claimed: &mut BTreeSet<&'a str>,
) -> Result<Option<File>, Invalid> {
    let Some(object) = one(iri, resource, DAO_HAS_REPRESENTATION)? else {
        return Ok(None);
    };
    let node = match named(object) {
        Some(node) if index.is_a(node.as_str(), DAO_REPRESENTATION) => node.as_str(),
        _ => return Err(invalid(iri, InvalidFact::UnknownRepresentation { node: lexical(object) })),
    };
    if !claimed.insert(node) {
        return Err(invalid(iri, InvalidFact::DuplicateRepresentation { node: node.to_string() }));
    }
    let facts = index.facts(node);
    let kind = required(node, facts, DAO_REPRESENTATION_TYPE)?;
    let Some(kind) = named(kind) else {
        return Err(unfit(node, DAO_REPRESENTATION_TYPE, kind));
    };
    Ok(Some(if kind == DAO_STILL_IMAGE_REPRESENTATION {
        File::StillImage {
            asset: asset(node, facts)?,
            width: dimension(node, facts, DAO_DIM_X)?,
            height: dimension(node, facts, DAO_DIM_Y)?,
        }
    } else if kind == DAO_AUDIO_REPRESENTATION {
        File::Audio { asset: asset(node, facts)? }
    } else if kind == DAO_MOVING_IMAGE_REPRESENTATION {
        File::MovingImage { asset: asset(node, facts)? }
    } else if kind == DAO_DOCUMENT_REPRESENTATION {
        File::Document { asset: asset(node, facts)? }
    } else {
        return Ok(None);
    }))
}

fn asset(node: &str, facts: &Facts<'_>) -> Result<String, Invalid> {
    match required(node, facts, DAO_INTERNAL_FILENAME)? {
        TermRef::Literal(asset) if asset.datatype() == xsd::STRING => Ok(asset.value().to_string()),
        other => Err(unfit(node, DAO_INTERNAL_FILENAME, other)),
    }
}

fn dimension(node: &str, facts: &Facts<'_>, predicate: NamedNodeRef<'static>) -> Result<u32, Invalid> {
    integer(node, predicate, required(node, facts, predicate)?)
}

fn required<'a>(node: &str, facts: &Facts<'a>, predicate: NamedNodeRef<'static>) -> Result<TermRef<'a>, Invalid> {
    one(node, facts, predicate)?.ok_or_else(|| {
        invalid(
            node,
            InvalidFact::IncompleteRepresentation { missing: predicate.as_str().to_string() },
        )
    })
}
