//! Images: one per file of a resource whose class requires an image, with the file's stem as its
//! id, its one representation and its rights.
//!
//! A class that does not require an image gets none, whatever files its resources have. A
//! class with a `cover` has the Image of one of its members.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::{ClassIndex, ImageRule, Remodel, RemodelImages, IMAGE_LINK};
use cpe_ports::{File, Resource};

use crate::draft::{Draft, EntryClass, Value};
use crate::error::{refuse_shared_keys, BuildError, Rule};
use crate::generic::curation::Curation;
use crate::generic::text::{self, py_strip, Limits, Sentinel};
use crate::generic::{iiif, rows, slug};
use crate::store::{PropertyRow, RepresentationRow, ResourceRow};

/// The file of a resource whose class requires an image: a `.jp2` still image of at least one
/// pixel.
#[derive(Clone, Copy)]
pub(crate) struct Scan<'a> {
    asset: &'a str,
    width: i64,
    height: i64,
}

impl<'a> Scan<'a> {
    /// The id of the file's Image; never empty.
    fn image_id(&self) -> &'a str {
        iiif::stem(self.asset)
    }
}

/// The file of `resource`, refused unless it is what [`Scan`] says.
fn scan<'a>(resource: &'a Resource, errors: &mut Vec<BuildError>) -> Option<Scan<'a>> {
    let rule = match &resource.file {
        Some(File::StillImage { asset, width, height }) => {
            if iiif::stem(asset).is_empty() {
                Rule::EmptyStem
            } else if !asset.ends_with(".jp2") {
                Rule::NotJp2
            } else if *width == 0 || *height == 0 {
                Rule::ZeroDimension
            } else {
                return Some(Scan { asset, width: i64::from(*width), height: i64::from(*height) });
            }
        }
        Some(File::Audio { .. } | File::MovingImage { .. } | File::Document { .. }) | None => Rule::NotStillImage,
    };
    errors.push(BuildError {
        rule,
        subject: resource.iri.as_str().to_string(),
        detail: None,
    });
    None
}

/// Reads the file of every resource of a class that requires an image, and gives the resource
/// its link to the file's Image. A resource whose file is refused is refused with it.
///
/// This runs before the ids are made: a member that is refused for its file takes no part in
/// the numbering of its parent's other members.
pub(crate) fn read(draft: &mut Draft<'_>, errors: &mut Vec<BuildError>) {
    let mut read = Vec::new();
    for (entry, _, class) in draft.kept() {
        if class.image == ImageRule::Required {
            read.push((entry.resource.iri.as_str(), scan(entry.resource, errors)));
        }
    }
    for (iri, scan) in read {
        match scan {
            Some(scan) => {
                draft.scan(iri, scan);
                draft.give(iri, [IMAGE_LINK], &[Value::Image(scan.image_id())]);
            }
            None => draft.refuse(iri),
        }
    }
}

/// Refuses an Image id that would stand for two things: the file of two resources of one
/// class, the file of resources of two classes, or an id that a resource already has.
///
/// Every resource that has an id is checked, whether or not it gets a row.
pub(crate) fn refuse_shared(draft: &Draft<'_>, mapping: &Remodel, errors: &mut Vec<BuildError>) {
    let mut resource_ids = BTreeSet::new();
    // The owners of each Image id, class by class in declared order.
    let mut owners: BTreeMap<&str, Vec<Vec<&str>>> = BTreeMap::new();
    for (class, _) in mapping.indexed_classes() {
        let mut of_class: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for entry in draft.entries().filter(|entry| entry.is_of(class)) {
            let Some(id) = entry.id.as_deref() else {
                continue;
            };
            resource_ids.insert(id);
            if let Some(scan) = entry.scan {
                let iri = entry.resource.iri.as_str();
                of_class.entry(scan.image_id()).or_default().push(iri);
            }
        }
        refuse_shared_keys(Rule::SharedFile, &of_class, errors);
        for (id, iris) in of_class {
            owners.entry(id).or_default().push(iris);
        }
    }
    for (id, by_class) in owners {
        if by_class.len() > 1 || resource_ids.contains(id) {
            let detail = by_class.concat().join(", ");
            errors.push(BuildError::refused(Rule::ImageIdTaken, id, detail));
        }
    }
}

/// Gives every resource of a class with a `cover` the Image of the member its curated key
/// names. The values under the key are counted as served: there is exactly one, and it does not
/// hold `sentinel`. Without the whitespace around it, it is the IRI of a member of the resource
/// itself, and the cover of no other resource.
///
/// A cover two resources name is refused once, and looked at no further for either. A member
/// that was refused gives its owner no image and no second report. A member that a hook
/// dropped has no Image, and its owner is refused for naming it.
///
/// This runs after a hook, so that it sees the hook's drops.
pub(crate) fn covers<'a>(
    draft: &mut Draft<'a>,
    curation: &Curation<'a>,
    mapping: &Remodel,
    errors: &mut Vec<BuildError>,
) {
    let limits = Limits {
        not_empty: false,
        no_addresses: false,
        sentinel: mapping.sentinel.as_deref().map(Sentinel::anywhere),
    };
    // (owner, its class, the cover it names)
    let mut named: Vec<(&str, ClassIndex, &str)> = Vec::new();
    for (entry, index, class) in draft.kept() {
        let ImageRule::Cover(key) = &class.image else {
            continue;
        };
        let owner = entry.resource.iri.as_str();
        let (rule, detail) = match curation.served(owner, key.as_str()).as_slice() {
            [value] => match text::offence(&value.text, limits) {
                Some(rule) => (rule, key.as_str().to_string()),
                None => {
                    named.push((owner, index, py_strip(&value.text)));
                    continue;
                }
            },
            served => (Rule::CoverCount, served.len().to_string()),
        };
        errors.push(BuildError::refused(rule, owner, detail));
    }

    let mut users: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &(owner, _, cover) in &named {
        users.entry(cover).or_default().push(owner);
    }
    let mut images = Vec::new();
    for (owner, class, cover) in named {
        if users.get(cover).is_some_and(|owners| owners.len() > 1) {
            continue;
        }
        let member = draft.get(cover).filter(|member| {
            let of_member_class = matches!(member.class, EntryClass::Mapped(_, member_class)
            if member_class.member_of.as_ref().is_some_and(|membership| {
                membership.parent == class
            }));
            let part_of = &member.resource.part_of;
            of_member_class && part_of.iter().any(|parent| parent.as_str() == owner)
        });
        match member {
            Some(member) if member.dropped_id().is_some() => {
                let refusal = BuildError::refused(Rule::CoverIsCalibrationScan, owner, cover);
                errors.push(refusal);
            }
            Some(member) => {
                if let Some(scan) = member.scan.filter(|_| member.is_kept()) {
                    images.push((owner, scan.image_id()));
                }
            }
            None => errors.push(BuildError::refused(Rule::CoverNotOwnPage, owner, cover)),
        }
    }
    refuse_shared_keys(Rule::SharedCover, &users, errors);
    for (owner, image) in images {
        draft.give(owner, [IMAGE_LINK], &[Value::Image(image)]);
    }
}

/// The files that get an Image, by its id: those of the resources the build kept.
fn kept<'a>(draft: &Draft<'a>) -> BTreeMap<&'a str, Scan<'a>> {
    let scans = draft.entries().filter(|entry| entry.is_kept()).filter_map(|entry| entry.scan);
    scans.map(|scan| (scan.image_id(), scan)).collect()
}

/// Refuses every Image id that a URL does not carry unchanged: the stem of a kept file. The
/// subject is the id, as for the other refusals about an Image id.
pub(crate) fn refuse_unsafe_ids(draft: &Draft<'_>, errors: &mut Vec<BuildError>) {
    let images = kept(draft);
    let ids = images.iter().map(|(id, scan)| (*id, *id, scan.asset));
    slug::refuse_unsafe_ids(ids, errors);
}

/// One row per Image, by id. An Image has no ARK: it is no resource of the archive.
pub(crate) fn rows(draft: &Draft<'_>, images: &RemodelImages) -> Vec<ResourceRow> {
    kept(draft)
        .into_keys()
        .map(|id| ResourceRow {
            id: id.to_string(),
            class: images.class.as_str().to_string(),
            ark: None,
        })
        .collect()
}

/// Every Image's rights: who made it and the licence it is under, the same for all.
pub(crate) fn properties(draft: &Draft<'_>, images: &RemodelImages) -> Vec<PropertyRow> {
    kept(draft)
        .into_keys()
        .flat_map(|id| {
            [
                rows::text(id, "authorship", &images.authorship, None),
                rows::text(id, "license", &images.license, None),
            ]
        })
        .collect()
}

/// One row per Image, by its id: the full-size address of its file. No source has an
/// alternative text for a file.
pub(crate) fn representations(draft: &Draft<'_>, images: &RemodelImages) -> Vec<RepresentationRow> {
    kept(draft)
        .into_iter()
        .map(|(id, scan)| RepresentationRow {
            resource_id: id.to_string(),
            kind: "image".to_string(),
            uri: iiif::uri(&images.iiif_base, scan.asset),
            width: Some(scan.width),
            height: Some(scan.height),
            alt: None,
            attribution: Some(format!("{} · {}", images.authorship, images.license)),
            seq: 0,
        })
        .collect()
}
