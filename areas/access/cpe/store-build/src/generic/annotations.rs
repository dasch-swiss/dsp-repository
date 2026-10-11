//! Annotations: the shapes editors drew on resources, and the links that tie annotations and
//! resources together.
//!
//! Every annotation is checked, whether the editors keep it or not: what they do not keep is
//! still the archive's, and a defect there is one. Rows are written for the kept ones. Three
//! rules ask for `keep` first and refuse a kept one only: an annotation on a resource that a
//! hook dropped, a link to such a resource, and a link that has nothing to be named by.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::{ClassIndex, Remodel, RemodelAnnotations};
use cpe_ports::Resource;

use crate::draft::{Draft, Entry, EntryClass};
use crate::error::{refuse_shared_keys, BuildError, Rule};
use crate::generic::curation::{Curation, Found, KEEP, LANG, NAME};
use crate::generic::geometry::{self, Geometry, GeometryError};
use crate::generic::kinds::{colour_of, geometry_of, link_of, of_kind, text_of};
use crate::generic::knora::{HAS_COLOR, HAS_COMMENT, HAS_GEOMETRY, HAS_LINK_TO, IS_REGION_OF};
use crate::generic::order::iri_local;
use crate::generic::slug;
use crate::generic::text::py_strip;
use crate::store::{AnnotationLinkRow, AnnotationLinkTargetRow, AnnotationRow, Store};

/// What the rules of one build read.
struct Inputs<'s, 'a> {
    draft: &'s Draft<'a>,
    curation: &'s Curation<'a>,
    mapping: &'s Remodel,
    block: &'s RemodelAnnotations,
}

/// A resource an annotation can point at, by its id.
#[derive(Clone, Copy)]
struct Pointed<'s> {
    id: &'s str,
    /// A hook dropped the resource: it has an id and no row.
    dropped: bool,
}

/// What an annotation can point at.
enum Place<'s> {
    /// A resource of a declared class; `None` for one that was refused.
    Resource(ClassIndex, Option<Pointed<'s>>),
    /// An annotation, with its id.
    Annotation(&'s str),
    /// An annotation link: nothing points at one.
    Link,
    /// A resource that was refused for its class: what points at it is not refused again.
    Refused,
}

impl<'s> Inputs<'s, '_> {
    fn place(&self, iri: &str) -> Place<'s> {
        match self.draft.get(iri) {
            Some(entry) => match entry.class {
                EntryClass::Mapped(class, _) => {
                    let kept = entry.kept_id().map(|id| (id, false));
                    let dropped = entry.dropped_id().map(|id| (id, true));
                    let pointed = kept.or(dropped);
                    Place::Resource(class, pointed.map(|(id, dropped)| Pointed { id, dropped }))
                }
                EntryClass::Annotation => Place::Annotation(id_of(entry.resource)),
                EntryClass::AnnotationLink => Place::Link,
            },
            None => Place::Refused,
        }
    }
}

/// Fills the three annotation tables of `store`, each in the order the store file lists it.
/// A mapping without an `annotations` block has no annotation.
pub(crate) fn fill(
    draft: &Draft<'_>,
    curation: &Curation<'_>,
    mapping: &Remodel,
    store: &mut Store,
    errors: &mut Vec<BuildError>,
) {
    let Some(block) = &mapping.annotations else {
        return;
    };
    let sources = Inputs { draft, curation, mapping, block };
    let of_class = |wanted: fn(&EntryClass<'_>) -> bool| {
        let entries = draft.entries().filter(move |entry| wanted(&entry.class));
        entries.collect::<Vec<&Entry<'_>>>()
    };
    let annotations = of_class(|class| matches!(class, EntryClass::Annotation));
    let links = of_class(|class| matches!(class, EntryClass::AnnotationLink));
    refuse_repeated_ids(annotations.iter().chain(&links), errors);

    let mut kept = BTreeSet::new();
    for entry in annotations {
        if let Some(row) = annotation(entry.resource, &sources, errors) {
            kept.insert(id_of(entry.resource));
            store.annotations.push(row);
        }
    }
    for entry in links {
        link(entry.resource, &sources, &kept, store, errors);
    }
    refuse_unsafe_ids(store, errors);
    store.annotations.sort_by(|a, b| a.id.cmp(&b.id));
    store.annotation_links.sort_by(|a, b| a.id.cmp(&b.id));
    store
        .annotation_link_targets
        .sort_by(|a, b| (&a.link_id, a.seq).cmp(&(&b.link_id, b.seq)));
}

/// Refuses every id of a row of `store` that a URL does not carry unchanged. Only a row the
/// build writes has an id to check.
fn refuse_unsafe_ids(store: &Store, errors: &mut Vec<BuildError>) {
    let annotations = store.annotations.iter().map(|row| (row.id.as_str(), "annotation"));
    let links = store.annotation_links.iter().map(|row| (row.id.as_str(), "annotation link"));
    let ids = annotations.chain(links).map(|(id, kind)| (id, id, kind));
    slug::refuse_unsafe_ids(ids, errors);
}

/// An annotation's and an annotation link's id: the last part of its IRI.
fn id_of(resource: &Resource) -> &str {
    iri_local(resource.iri.as_str())
}

/// Refuses every id that two of `entries` have. The annotations and their links are one id
/// space, apart from the resources'.
fn refuse_repeated_ids<'s>(entries: impl Iterator<Item = &'s &'s Entry<'s>>, errors: &mut Vec<BuildError>) {
    let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for entry in entries {
        let iri = entry.resource.iri.as_str();
        owners.entry(id_of(entry.resource)).or_default().push(iri);
    }
    refuse_shared_keys(Rule::DuplicateAnnotationId, &owners, errors);
}

/// What the editors curated about an annotation or a link.
struct Curated<'a> {
    /// Whether it is written to the store.
    keep: bool,
    /// The language of its texts, where the editors named one.
    lang: Option<&'a str>,
}

/// Reads what is curated about `annotation`: one `keep`, which is `yes` or `no`, and at most
/// one language, which is one of the site's. `None` when either was refused.
fn curated<'a>(annotation: &str, sources: &Inputs<'_, 'a>, errors: &mut Vec<BuildError>) -> Option<Curated<'a>> {
    let keep = match sources.curation.at_most_one(annotation, KEEP, errors) {
        Found::One("yes", _) => Some(true),
        Found::One("no", _) => Some(false),
        Found::One(other, _) => {
            errors.push(BuildError::refused(Rule::KeepNotYesOrNo, annotation, other));
            None
        }
        Found::Absent => {
            errors.push(BuildError::refused(Rule::CuratedCount, annotation, KEEP));
            None
        }
        Found::Refused => None,
    };
    let languages = &sources.mapping.languages;
    let lang = match sources.curation.at_most_one(annotation, LANG, errors) {
        Found::One(lang, _) if languages.iter().any(|known| known == lang) => Some(Some(lang)),
        Found::One(lang, _) => {
            let rule = Rule::UnknownAnnotationLanguage;
            errors.push(BuildError::refused(rule, annotation, lang));
            None
        }
        Found::Absent => Some(None),
        Found::Refused => None,
    };
    Some(Curated { keep: keep?, lang: lang? })
}

/// `found`, the values of `resource` under `property`, as at most one. The outer `None` when
/// there are several, which is refused.
fn at_most_one<T>(
    resource: &Resource,
    property: &str,
    mut found: Vec<T>,
    errors: &mut Vec<BuildError>,
) -> Option<Option<T>> {
    if found.len() > 1 {
        let subject = resource.iri.as_str();
        let refusal = BuildError::refused(Rule::AnnotationValueCount, subject, property);
        errors.push(refusal);
        return None;
    }
    Some(found.pop())
}

/// [`at_most_one`], with none refused too.
fn exactly_one<T>(resource: &Resource, property: &str, found: Vec<T>, errors: &mut Vec<BuildError>) -> Option<T> {
    let one = at_most_one(resource, property, found, errors)?;
    if one.is_none() {
        let subject = resource.iri.as_str();
        let refusal = BuildError::refused(Rule::AnnotationValueCount, subject, property);
        errors.push(refusal);
    }
    one
}

/// The row of an annotation the editors keep. Every check runs before their answer is asked
/// for.
fn annotation(resource: &Resource, sources: &Inputs<'_, '_>, errors: &mut Vec<BuildError>) -> Option<AnnotationRow> {
    let before = errors.len();
    let subject = resource.iri.as_str();
    let curated = curated(subject, sources, errors);
    let geometry = geometry(resource, errors);
    let colour = colour(resource, errors);
    let remark = comment(resource, errors);
    if remark.flatten().is_some_and(|remark| remark.contains('\u{1e}')) {
        let rule = Rule::RemarkHoldsRecordSeparator;
        errors.push(BuildError::refused(rule, subject, HAS_COMMENT));
    }
    let on = annotated(resource, sources, errors);
    if errors.len() > before {
        return None;
    }
    // Where no error was pushed, a `None` is a resource that was refused, for a defect of its
    // own or for its class.
    let (curated, geometry, colour, remark, on) = (curated?, geometry?, colour?, remark?, on?);

    if !curated.keep {
        return None;
    }
    if on.dropped {
        let refusal = BuildError::refused(Rule::RegionOnDroppedPage, subject, on.id);
        errors.push(refusal);
        return None;
    }
    Some(AnnotationRow {
        id: id_of(resource).to_string(),
        resource_id: on.id.to_string(),
        shape: geometry.shape.name().to_string(),
        x: geometry.x,
        y: geometry.y,
        w: geometry.w,
        h: geometry.h,
        points: geometry.points,
        color: Some(colour),
        remark: remark.map(str::to_string),
        remark_lang: remark.and(curated.lang).map(str::to_string),
        ark: Some(resource.ark.as_str().to_string()),
    })
}

fn geometry(resource: &Resource, errors: &mut Vec<BuildError>) -> Option<Geometry> {
    let served = of_kind(resource, HAS_GEOMETRY, errors, geometry_of)?;
    let json = exactly_one(resource, HAS_GEOMETRY, served, errors)?;
    match geometry::normalise(json) {
        Ok(geometry) => Some(geometry),
        Err(error) => {
            let rule = match error {
                GeometryError::NotRead => Rule::GeometryNotRead,
                GeometryError::UnknownShape(_) => Rule::UnknownShape,
                GeometryError::PointCount { .. } => Rule::ShapePointCount,
                GeometryError::EmptyBox(_) => Rule::EmptyBox,
                GeometryError::OutsideImage(_) => Rule::BoxOutsideImage,
                GeometryError::Number(_) => Rule::UnwritableNumber,
            };
            let subject = resource.iri.as_str();
            errors.push(BuildError::refused(rule, subject, error.to_string()));
            None
        }
    }
}

/// The annotation's colour in lower case, which must be `#rrggbb`.
fn colour(resource: &Resource, errors: &mut Vec<BuildError>) -> Option<String> {
    let served = of_kind(resource, HAS_COLOR, errors, colour_of)?;
    let served = exactly_one(resource, HAS_COLOR, served, errors)?;
    let colour = py_strip(served).to_ascii_lowercase();
    let is_hex = |digit: &u8| matches!(digit, b'0'..=b'9' | b'a'..=b'f');
    match colour.as_bytes() {
        [b'#', digits @ ..] if digits.len() == 6 && digits.iter().all(is_hex) => Some(colour),
        _ => {
            let subject = resource.iri.as_str();
            errors.push(BuildError::refused(Rule::ColourNotHex, subject, served));
            None
        }
    }
}

/// The one comment of an annotation or a link, without the whitespace around it. The inner
/// `None` where it has none, or one that is empty then.
fn comment<'a>(resource: &'a Resource, errors: &mut Vec<BuildError>) -> Option<Option<&'a str>> {
    let served = of_kind(resource, HAS_COMMENT, errors, text_of)?;
    let comment = at_most_one(resource, HAS_COMMENT, served, errors)?;
    Some(comment.map(|(text, _)| py_strip(text)).filter(|text| !text.is_empty()))
}

/// The one resource the annotation sits on, which is of a class the block names under `on`.
/// `None` without an error when that resource was refused, for a defect of its own or for its
/// class. The links are counted as served: the same resource twice is two, and refused, where
/// a link's repeated target is one.
fn annotated<'s>(resource: &Resource, sources: &Inputs<'s, '_>, errors: &mut Vec<BuildError>) -> Option<Pointed<'s>> {
    let targets = of_kind(resource, IS_REGION_OF, errors, link_of)?;
    let place = match targets.as_slice() {
        [target] => Some(sources.place(target)),
        _ => None,
    };
    match place {
        Some(Place::Resource(class, pointed)) if sources.block.on.contains(&class) => pointed,
        Some(Place::Refused) => None,
        _ => {
            let subject = resource.iri.as_str();
            let detail = targets.join(", ");
            errors.push(BuildError::refused(Rule::RegionTarget, subject, detail));
            None
        }
    }
}

/// What a link points at. The derived order is the store's: an annotation before a resource,
/// the resources by the place of their class in the mapping, each kind by id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Target<'s> {
    Annotation(&'s str),
    /// The resource's class, its id, and whether a hook dropped it.
    Resource(ClassIndex, &'s str, bool),
}

/// Every target of the link, sorted: annotations, and resources of the classes the block names
/// under `link-targets`. `None` without an error when one of them was refused, for a defect of
/// its own or for its class.
fn targets<'s>(resource: &Resource, sources: &Inputs<'s, '_>, errors: &mut Vec<BuildError>) -> Option<Vec<Target<'s>>> {
    let iris = of_kind(resource, HAS_LINK_TO, errors, link_of)?;
    let subject = resource.iri.as_str();
    if iris.is_empty() {
        errors.push(BuildError {
            rule: Rule::LinkWithoutTarget,
            subject: subject.to_string(),
            detail: None,
        });
        return None;
    }
    // A target the snapshot names twice is one target.
    let iris: BTreeSet<&str> = iris.into_iter().collect();
    let mut found = Vec::new();
    let mut whole = true;
    for iri in iris {
        let target = match sources.place(iri) {
            Place::Annotation(id) => Some(Target::Annotation(id)),
            Place::Resource(class, _) if !sources.block.link_targets.contains(&class) => {
                errors.push(BuildError::refused(Rule::LinkTarget, subject, iri));
                None
            }
            Place::Resource(class, pointed) => {
                pointed.map(|pointed| Target::Resource(class, pointed.id, pointed.dropped))
            }
            Place::Link => {
                errors.push(BuildError::refused(Rule::LinkTarget, subject, iri));
                None
            }
            Place::Refused => None,
        };
        whole &= target.is_some();
        found.extend(target);
    }
    found.sort();
    whole.then_some(found)
}

/// Writes the rows of a link the editors keep: the link, and its targets numbered from 1.
///
/// A target that is an annotation without a row is left out, and a link that is left with no
/// target is not written. `kept` are the ids of the annotations that have a row.
fn link(
    resource: &Resource,
    sources: &Inputs<'_, '_>,
    kept: &BTreeSet<&str>,
    store: &mut Store,
    errors: &mut Vec<BuildError>,
) {
    let before = errors.len();
    let iri = resource.iri.as_str();
    let curated = curated(iri, sources, errors);
    let name = sources.curation.at_most_one(iri, NAME, errors);
    let comment = comment(resource, errors);
    let targets = targets(resource, sources, errors);
    if errors.len() > before {
        return;
    }
    let (Some(curated), Some(comment), Some(mut targets)) = (curated, comment, targets) else {
        return;
    };

    if !curated.keep {
        return;
    }
    targets.retain(|target| match target {
        Target::Annotation(id) => kept.contains(id),
        Target::Resource(..) => true,
    });
    if targets.is_empty() {
        return;
    }
    let dropped = targets.iter().find_map(|target| match target {
        Target::Resource(_, id, true) => Some(*id),
        Target::Resource(_, _, false) | Target::Annotation(_) => None,
    });
    if let Some(id) = dropped {
        errors.push(BuildError::refused(Rule::LinkToDroppedPage, iri, id));
        return;
    }
    let name = match (name, comment) {
        (Found::One(name, _), _) => name,
        (Found::Absent, Some(comment)) => comment,
        (Found::Absent, None) => {
            errors.push(BuildError {
                rule: Rule::LinkWithoutName,
                subject: iri.to_string(),
                detail: None,
            });
            return;
        }
        (Found::Refused, _) => return,
    };

    let id = id_of(resource);
    store.annotation_links.push(AnnotationLinkRow {
        id: id.to_string(),
        name: name.to_string(),
        name_lang: curated.lang.map(str::to_string),
        comment: comment.map(str::to_string),
        comment_lang: comment.and(curated.lang).map(str::to_string),
    });
    for (seq, target) in (1..).zip(targets) {
        let (annotation_id, resource_id) = match target {
            Target::Annotation(id) => (Some(id), None),
            Target::Resource(_, id, _) => (None, Some(id)),
        };
        store.annotation_link_targets.push(AnnotationLinkTargetRow {
            link_id: id.to_string(),
            seq,
            annotation_id: annotation_id.map(str::to_string),
            resource_id: resource_id.map(str::to_string),
        });
    }
}
