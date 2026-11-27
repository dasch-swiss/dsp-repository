//! A resource's representation (`FORMAT.md` §8). One whose `dao:representationType` the port has no
//! `File` for is omitted, as §8 lets a reader do; any other broken representation is invalid.

use std::collections::BTreeSet;

use cpe_ports::{File, FileLegal};
use oxrdf::vocab::xsd;
use oxrdf::{NamedNodeRef, TermRef};

use super::{integer, invalid, lexical, named, objects, one, unfit, Facts, Index, Invalid};
use crate::vocab::{
    DAO_AUDIO_REPRESENTATION, DAO_AUTHORSHIP, DAO_COPYRIGHT_HOLDER, DAO_DIM_X, DAO_DIM_Y, DAO_DOCUMENT_REPRESENTATION,
    DAO_HAS_REPRESENTATION, DAO_INTERNAL_FILENAME, DAO_MOVING_IMAGE_REPRESENTATION, DAO_REPRESENTATION,
    DAO_REPRESENTATION_TYPE, DAO_STILL_IMAGE_REPRESENTATION,
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
            legal: legal(node, facts)?,
        }
    } else if kind == DAO_AUDIO_REPRESENTATION {
        File::Audio { asset: asset(node, facts)?, legal: legal(node, facts)? }
    } else if kind == DAO_MOVING_IMAGE_REPRESENTATION {
        File::MovingImage { asset: asset(node, facts)?, legal: legal(node, facts)? }
    } else if kind == DAO_DOCUMENT_REPRESENTATION {
        File::Document { asset: asset(node, facts)?, legal: legal(node, facts)? }
    } else {
        return Ok(None);
    }))
}

fn asset(node: &str, facts: &Facts<'_>) -> Result<String, Invalid> {
    plain(node, DAO_INTERNAL_FILENAME, required(node, facts, DAO_INTERNAL_FILENAME)?)
}

/// Both predicates are optional; `dao:authorship` may repeat, `dao:copyrightHolder` may not.
fn legal(node: &str, facts: &Facts<'_>) -> Result<FileLegal, Invalid> {
    let authorship = objects(facts, DAO_AUTHORSHIP)
        .map(|author| plain(node, DAO_AUTHORSHIP, author))
        .collect::<Result<_, _>>()?;
    let copyright_holder = one(node, facts, DAO_COPYRIGHT_HOLDER)?
        .map(|holder| plain(node, DAO_COPYRIGHT_HOLDER, holder))
        .transpose()?;
    Ok(FileLegal { authorship, copyright_holder })
}

/// A plain `xsd:string` literal: no datatype and no language tag (`FORMAT.md` §1).
fn plain(node: &str, predicate: NamedNodeRef<'static>, term: TermRef<'_>) -> Result<String, Invalid> {
    match term {
        TermRef::Literal(text) if text.datatype() == xsd::STRING => Ok(text.value().to_string()),
        other => Err(unfit(node, predicate, other)),
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
