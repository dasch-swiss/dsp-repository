//! The values of a resource under one property of the archive, read by kind.

use cpe_ports::{DateValue, Resource, ValueKind};

use crate::error::{BuildError, Rule};
use crate::generic::curation::Text;

/// The values of `resource` under `property` that `expected` takes, in the order the resource
/// has them. A value of another kind is refused, never skipped, each one once; the property
/// then gives `None`, so that its values are not counted or judged without it.
pub(crate) fn of_kind<'a, T>(
    resource: &'a Resource,
    property: &str,
    errors: &mut Vec<BuildError>,
    expected: impl Fn(&'a ValueKind) -> Option<T>,
) -> Option<Vec<T>> {
    let mut found = Vec::new();
    let mut all_of_the_kind = true;
    for value in &resource.values {
        if value.property.as_str() != property {
            continue;
        }
        match expected(&value.kind) {
            Some(value) => found.push(value),
            None => {
                all_of_the_kind = false;
                let subject = resource.iri.as_str();
                let refusal = BuildError::refused(Rule::UnexpectedValueKind, subject, property);
                errors.push(refusal);
            }
        }
    }
    all_of_the_kind.then_some(found)
}

// Each reader names every kind of the port, so that a kind the port gains must be placed here
// before the builder compiles.

pub(crate) fn text_of(kind: &ValueKind) -> Option<Text<'_>> {
    match kind {
        ValueKind::Text { text, lang } => Some((text.as_str(), lang.as_deref())),
        ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Date(_)
        | ValueKind::Uri(_)
        | ValueKind::Geometry(_)
        | ValueKind::Color(_)
        | ValueKind::ListNode(_)
        | ValueKind::Link(_) => None,
    }
}

/// An address is written as a text without a language.
pub(crate) fn uri_of(kind: &ValueKind) -> Option<Text<'_>> {
    match kind {
        ValueKind::Uri(uri) => Some((uri.as_str(), None)),
        ValueKind::Text { .. }
        | ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Date(_)
        | ValueKind::Geometry(_)
        | ValueKind::Color(_)
        | ValueKind::ListNode(_)
        | ValueKind::Link(_) => None,
    }
}

pub(crate) fn date_of(kind: &ValueKind) -> Option<&DateValue> {
    match kind {
        ValueKind::Date(date) => Some(date),
        ValueKind::Text { .. }
        | ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Uri(_)
        | ValueKind::Geometry(_)
        | ValueKind::Color(_)
        | ValueKind::ListNode(_)
        | ValueKind::Link(_) => None,
    }
}

pub(crate) fn link_of(kind: &ValueKind) -> Option<&str> {
    match kind {
        ValueKind::Link(target) => Some(target.as_str()),
        ValueKind::Text { .. }
        | ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Date(_)
        | ValueKind::Uri(_)
        | ValueKind::Geometry(_)
        | ValueKind::Color(_)
        | ValueKind::ListNode(_) => None,
    }
}

/// A geometry, as the archive's JSON; `None` for a value of any other kind.
pub(crate) fn geometry_of(kind: &ValueKind) -> Option<&str> {
    match kind {
        ValueKind::Geometry(json) => Some(json.as_str()),
        ValueKind::Text { .. }
        | ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Date(_)
        | ValueKind::Uri(_)
        | ValueKind::Color(_)
        | ValueKind::ListNode(_)
        | ValueKind::Link(_) => None,
    }
}

/// A colour, as the archive writes it; `None` for a value of any other kind.
pub(crate) fn colour_of(kind: &ValueKind) -> Option<&str> {
    match kind {
        ValueKind::Color(colour) => Some(colour.as_str()),
        ValueKind::Text { .. }
        | ValueKind::Integer(_)
        | ValueKind::Decimal(_)
        | ValueKind::Boolean(_)
        | ValueKind::Date(_)
        | ValueKind::Uri(_)
        | ValueKind::Geometry(_)
        | ValueKind::ListNode(_)
        | ValueKind::Link(_) => None,
    }
}
