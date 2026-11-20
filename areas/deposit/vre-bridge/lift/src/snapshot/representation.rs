//! Representations (§8): a resource's one live file value as a
//! `dao:Representation` node, minted from the file value's `kb:valueHasUUID`.

use oxigraph::model::{Literal, NamedNode, Term};

use super::index::Entry;
use super::value::{self, one_literal};
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, kinds, mint, vocab};
use crate::model::{self, kb, nn};

/// §11: the predicates a carried file value may carry.
const PREDICATES: &[&str] = &[
    model::RDF_TYPE,
    kb::VALUE_HAS_UUID,
    kb::INTERNAL_FILENAME,
    kb::DIM_X,
    kb::DIM_Y,
    kb::HAS_AUTHORSHIP,
    kb::HAS_COPYRIGHT_HOLDER,
    kb::IS_DELETED,
    kb::PREVIOUS_VALUE,
    kb::INTERNAL_MIME_TYPE,
    kb::ORIGINAL_FILENAME,
    kb::ORIGINAL_MIME_TYPE,
    kb::VALUE_HAS_STRING,
    kb::VALUE_HAS_ORDER,
    kb::VALUE_CREATION_DATE,
    kb::ATTACHED_TO_USER,
    kb::HAS_PERMISSIONS,
];

/// The stats line the file values omitted whole are counted under.
pub(crate) const OMITTED_FILE_VALUE: &str = "archive, text or external still-image file value (§8)";

/// §8's table: the representation type of a file-value class and whether it
/// carries dimensions; `None` for the classes omitted whole.
fn representation_type(class: &str) -> Option<(&'static str, bool)> {
    Some(match class {
        kb::STILL_IMAGE_FILE_VALUE => (vocab::DAO_STILL_IMAGE_REPRESENTATION, true),
        kb::AUDIO_FILE_VALUE => (vocab::DAO_AUDIO_REPRESENTATION, false),
        kb::MOVING_IMAGE_FILE_VALUE => (vocab::DAO_MOVING_IMAGE_REPRESENTATION, false),
        kb::DOCUMENT_FILE_VALUE => (vocab::DAO_DOCUMENT_REPRESENTATION, false),
        _ => return None,
    })
}

/// Emit the representation node of a live file value of `class` and return it,
/// or `None` for a class omitted whole (counted) or after recording every reason
/// it cannot be emitted. The caller writes `<resource> dao:hasRepresentation <node>`.
#[must_use = "the caller writes the edge from the resource to the returned node"]
pub(crate) fn emit(
    file: &Entry,
    class: &str,
    snapshot: &mut Snapshot,
    stats: &mut Stats,
    violations: &mut Violations,
) -> Option<NamedNode> {
    let Some((representation_type, dimensions)) = representation_type(class) else {
        *stats.omitted.entry(OMITTED_FILE_VALUE).or_default() += 1;
        return None;
    };
    let s = file.node.as_str();
    for (p, _) in &file.triples {
        if !PREDICATES.contains(&p.as_str()) {
            violations.record(Rule::UnknownFileValuePredicate, s, p.as_str());
        }
    }
    let uuid = one_literal(file, kb::VALUE_HAS_UUID, violations);
    let node = uuid.and_then(|u| mint::minted_node(vocab::REPRESENTATION_PREFIX, u.value(), s, violations));
    let filename = filename(file, violations);
    // `Some(None)`: a type without dimensions; `None`: a missing or invalid one.
    let dims = if dimensions {
        let x = dimension(file, kb::DIM_X, violations);
        let y = dimension(file, kb::DIM_Y, violations);
        x.zip(y).map(Some)
    } else {
        Some(None)
    };
    let legal = legal(file, violations);
    let (Some(node), Some(filename), Some(dims), Some(legal)) = (node, filename, dims, legal) else {
        return None;
    };

    let n = node.as_ref();
    snapshot.put(n, nn(vocab::RDF_TYPE).as_ref(), nn(vocab::DAO_REPRESENTATION).as_ref());
    snapshot.put(n, nn(vocab::DAO_REPRESENTATION_TYPE).as_ref(), nn(representation_type).as_ref());
    snapshot.put(n, nn(vocab::DAO_INTERNAL_FILENAME).as_ref(), &filename);
    if let Some((x, y)) = &dims {
        snapshot.put(n, nn(vocab::DAO_DIM_X).as_ref(), x);
        snapshot.put(n, nn(vocab::DAO_DIM_Y).as_ref(), y);
    }
    for author in &legal.authors {
        snapshot.put(n, nn(vocab::DAO_AUTHORSHIP).as_ref(), author);
    }
    if let Some(holder) = &legal.copyright_holder {
        snapshot.put(n, nn(vocab::DAO_COPYRIGHT_HOLDER).as_ref(), holder);
    }
    *stats.representations_by_type.entry(representation_type).or_default() += 1;
    Some(node)
}

/// `kb:internalFilename`, which must be one plain string.
fn filename(file: &Entry, violations: &mut Violations) -> Option<Literal> {
    let literal = one_literal(file, kb::INTERNAL_FILENAME, violations)?;
    if !value::is_plain_string(literal) {
        violations.record(Rule::InvalidLiteral, file.node.as_str(), kb::INTERNAL_FILENAME);
        return None;
    }
    Some(Literal::new_simple_literal(literal.value()))
}

/// A file's legal information (§8, D14).
struct Legal {
    authors: Vec<Literal>,
    copyright_holder: Option<Literal>,
}

/// Every `kb:hasAuthorship` and at most one `kb:hasCopyrightHolder`, each a
/// plain string; `None` after recording every violation among them.
fn legal(file: &Entry, violations: &mut Violations) -> Option<Legal> {
    let authors: Vec<Option<Literal>> = kinds::objects(file, kb::HAS_AUTHORSHIP)
        .into_iter()
        .map(|object| plain_string(file, kb::HAS_AUTHORSHIP, object, violations))
        .collect();
    // The outer `None` is a violation; `Some(None)` a file without a holder.
    let copyright_holder = match value::at_most_one_literal(file, kb::HAS_COPYRIGHT_HOLDER, violations) {
        Ok(Some(literal)) => {
            let object = Term::Literal(literal.clone());
            plain_string(file, kb::HAS_COPYRIGHT_HOLDER, &object, violations).map(Some)
        }
        Ok(None) => Some(None),
        Err(()) => None,
    };
    Some(Legal {
        authors: authors.into_iter().collect::<Option<_>>()?,
        copyright_holder: copyright_holder?,
    })
}

/// `object` as a plain string literal, or `None` after recording `invalid-literal`.
fn plain_string(file: &Entry, p: &str, object: &Term, violations: &mut Violations) -> Option<Literal> {
    match object {
        Term::Literal(literal) if value::is_plain_string(literal) => Some(Literal::new_simple_literal(literal.value())),
        _ => {
            violations.record(Rule::InvalidLiteral, file.node.as_str(), p);
            None
        }
    }
}

/// A still image's `kb:dimX` or `kb:dimY`: one `xsd:integer` fitting a `u32`.
fn dimension(file: &Entry, p: &str, violations: &mut Violations) -> Option<Literal> {
    let literal = one_literal(file, p, violations)?;
    if value::xsd_integer_as::<u32>(literal).is_none() {
        violations.record(Rule::InvalidLiteral, file.node.as_str(), p);
        return None;
    }
    Some(value::integer_literal(literal))
}
