//! The class rule: what a resource of the snapshot is under the mapping.

use cpe_core::remodel::Remodel;
use cpe_ports::{Motivation, ProjectSnapshot, Resource};

use crate::draft::{Draft, Entry, EntryClass};
use crate::error::{BuildError, Rule};
use crate::generic::knora::{LINK_OBJ_CLASS, REGION_CLASS};

/// The draft of `snapshot`: its resources that have a class under `mapping`. A resource
/// [`class_of`] refuses is in no entry.
pub(crate) fn classify<'a>(
    snapshot: &'a ProjectSnapshot,
    mapping: &'a Remodel,
    errors: &mut Vec<BuildError>,
) -> Draft<'a> {
    let mut draft = Draft::default();
    for resource in &snapshot.resources {
        match class_of(resource, mapping) {
            Ok(class) => draft.insert(Entry::new(resource, class)),
            Err(error) => errors.push(error),
        }
    }
    draft
}

/// The class of `resource`. A class the mapping does not declare is refused, never dropped: its
/// resources would silently be missing from the store.
///
/// An annotation is known by its `annotation` field and is a link or not by its motivation; its
/// class must be the one that motivation belongs to. A mapping without an `annotations` block
/// maps no annotation, so there every annotation is of an undeclared class.
fn class_of<'a>(resource: &Resource, mapping: &'a Remodel) -> Result<EntryClass<'a>, BuildError> {
    let class = resource.class.as_str();
    let refused = |rule| BuildError::refused(rule, resource.iri.as_str(), class);
    match &resource.annotation {
        Some(_) if mapping.annotations.is_none() => Err(refused(Rule::UnmappedClass)),
        Some(annotation) => {
            let (expected, mapped) = match annotation.motivation {
                Motivation::Linking => (LINK_OBJ_CLASS, EntryClass::AnnotationLink),
                Motivation::Commenting | Motivation::Highlighting => (REGION_CLASS, EntryClass::Annotation),
            };
            if class == expected {
                Ok(mapped)
            } else {
                Err(refused(Rule::AnnotationClassMismatch))
            }
        }
        None => mapping
            .indexed_classes()
            .find(|(_, declared)| declared.iri.as_str() == class)
            .map(|(index, declared)| EntryClass::Mapped(index, declared))
            .ok_or_else(|| refused(Rule::UnmappedClass)),
    }
}
