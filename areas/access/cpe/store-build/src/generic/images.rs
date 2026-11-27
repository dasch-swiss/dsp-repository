//! Images, each with its one representation and its rights, in one of two ways. When the
//! `images` node names a declared class, each kept resource of that class is an image of its own,
//! its file its representation. Otherwise each file of a resource whose class requires an image
//! is an Image of its own, with the file's stem as its id, and the resource links to it.
//!
//! A class that does not require an image gets none, whatever files its resources have. A
//! class with a `cover` has the Image of one of its members.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::{
    ClassIndex, ImageRule, Remodel, RemodelImages, IMAGE_AUTHORSHIP, IMAGE_COPYRIGHT, IMAGE_LICENSE, IMAGE_LINK,
};
use cpe_ports::{File, FileLegal, Resource};

use crate::draft::{Draft, EntryClass, Value};
use crate::error::{refuse_shared_keys, BuildError, Rule};
use crate::generic::curation::Curation;
use crate::generic::text::{self, py_strip, Limits, Sentinel};
use crate::generic::{iiif, rows, slug};
use crate::store::{PropertyRow, RepresentationRow, ResourceRow};

/// The file of a resource whose class requires an image, or of a resource of the declared
/// images class: a JPEG 2000 still image (`.jp2` or `.jpx`) of at least one pixel.
#[derive(Clone, Copy)]
pub(crate) struct Scan<'a> {
    asset: &'a str,
    width: i64,
    height: i64,
    legal: &'a FileLegal,
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
        Some(File::StillImage { asset, width, height, legal }) => {
            if iiif::stem(asset).is_empty() {
                Rule::EmptyStem
            } else if !asset.ends_with(".jp2") && !asset.ends_with(".jpx") {
                Rule::NotJp2
            } else if *width == 0 || *height == 0 {
                Rule::ZeroDimension
            } else {
                let (width, height) = (i64::from(*width), i64::from(*height));
                return Some(Scan { asset, width, height, legal });
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

/// The declared class the `images` node names, if it names one.
fn declared(mapping: &Remodel) -> Option<ClassIndex> {
    mapping.images.as_ref().and_then(|images| images.declared)
}

/// Reads the file of every resource of a class that requires an image, and gives the resource
/// its link to the file's Image; reads the file of every resource of the declared images class,
/// which is its own image and links to none. A resource whose file is refused is refused with it.
///
/// This runs before the ids are made: a member that is refused for its file takes no part in
/// the numbering of its parent's other members.
pub(crate) fn read(draft: &mut Draft<'_>, mapping: &Remodel, errors: &mut Vec<BuildError>) {
    let images = declared(mapping);
    let mut read = Vec::new();
    for (entry, index, class) in draft.kept() {
        let links = class.image == ImageRule::Required;
        if links || images == Some(index) {
            read.push((entry.resource.iri.as_str(), scan(entry.resource, errors), links));
        }
    }
    for (iri, scan, links) in read {
        match scan {
            Some(scan) => {
                draft.scan(iri, scan);
                if links {
                    draft.give(iri, [IMAGE_LINK], &[Value::Image(scan.image_id())]);
                }
            }
            None => draft.refuse(iri),
        }
    }
}

/// Refuses an Image id that would stand for two things: the file of two resources of one
/// class, the file of resources of two classes, or an id that a resource already has. A
/// declared images class has no Image ids: its resources' own ids are checked as any others.
///
/// Every resource that has an id is checked, whether or not it gets a row.
pub(crate) fn refuse_shared(draft: &Draft<'_>, mapping: &Remodel, errors: &mut Vec<BuildError>) {
    if declared(mapping).is_some() {
        return;
    }
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
            let EntryClass::Mapped(_, member_class) = member.class else {
                return false;
            };
            let Some(membership) = member_class.member_of.as_ref().filter(|m| m.parent == class) else {
                return false;
            };
            // A member placed through its parent's link (`via`) is that parent's alone, whatever
            // its `part_of` names.
            match membership.via {
                Some(_) => member.member.is_some_and(|placed| placed.parent == owner),
                None => member.resource.part_of.iter().any(|parent| parent.as_str() == owner),
            }
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

/// One image the store gets: its file, and the label of the resource that is the image, which
/// is its alternative text. An Image of a file has no such resource and no alternative text.
struct Image<'d> {
    scan: Scan<'d>,
    alt: Option<&'d str>,
}

impl Image<'_> {
    /// Who made the image: its file's authors, else the block's `authorship`.
    fn authorship(&self, images: &RemodelImages) -> String {
        match self.scan.legal.authorship.as_slice() {
            [] => images.authorship.clone(),
            authors => authors.join(", "),
        }
    }
}

/// The images the build kept, by id: each kept resource of the declared images class, or the
/// file of each kept resource that has one.
fn kept<'d>(draft: &'d Draft<'_>, images: &RemodelImages) -> BTreeMap<&'d str, Image<'d>> {
    let kept = draft.entries().filter(|entry| entry.is_kept());
    match images.declared {
        Some(class) => kept
            .filter(|entry| entry.is_of(class))
            .filter_map(|entry| {
                let image = Image { scan: entry.scan?, alt: Some(entry.resource.label.as_str()) };
                Some((entry.kept_id()?, image))
            })
            .collect(),
        None => kept
            .filter_map(|entry| entry.scan)
            .map(|scan| (scan.image_id(), Image { scan, alt: None }))
            .collect(),
    }
}

/// Refuses every kept file whose stem a URL does not carry unchanged: the stem goes into the
/// representation's IIIF URL. Of a file's Image the stem is its id, and the subject is that id,
/// as for the other refusals about an Image id. A resource of a declared images class has its own
/// id, checked as any other; its file's stem is checked here, with the resource's id as the
/// subject.
pub(crate) fn refuse_unsafe_ids(draft: &Draft<'_>, images: &RemodelImages, errors: &mut Vec<BuildError>) {
    let images = kept(draft, images);
    let stems = images.iter().map(|(id, image)| (image.scan.image_id(), *id, image.scan.asset));
    slug::refuse_unsafe_ids(stems, errors);
}

/// One row per Image of a file, by id. An Image has no ARK: it is no resource of the archive. A
/// resource of a declared images class has its row as a resource of its class.
pub(crate) fn rows(draft: &Draft<'_>, images: &RemodelImages) -> Vec<ResourceRow> {
    if images.declared.is_some() {
        return Vec::new();
    }
    kept(draft, images)
        .into_keys()
        .map(|id| ResourceRow {
            id: id.to_string(),
            class: images.class.as_str().to_string(),
            ark: None,
        })
        .collect()
}

/// Every image's rights: who made it, who holds its copyright, where its file names them, and
/// the block's licence.
pub(crate) fn properties(draft: &Draft<'_>, images: &RemodelImages) -> Vec<PropertyRow> {
    let mut found = Vec::new();
    for (id, image) in kept(draft, images) {
        found.push(rows::text(id, IMAGE_AUTHORSHIP, &image.authorship(images), None));
        if let Some(holder) = &image.scan.legal.copyright_holder {
            found.push(rows::text(id, IMAGE_COPYRIGHT, holder, None));
        }
        found.push(rows::text(id, IMAGE_LICENSE, &images.license, None));
    }
    found
}

/// One row per image, by its id: the full-size address of its file, and its alternative text
/// where it has one.
pub(crate) fn representations(draft: &Draft<'_>, images: &RemodelImages) -> Vec<RepresentationRow> {
    kept(draft, images)
        .into_iter()
        .map(|(id, image)| RepresentationRow {
            resource_id: id.to_string(),
            kind: "image".to_string(),
            uri: iiif::uri(&images.iiif_base, image.scan.asset),
            width: Some(image.scan.width),
            height: Some(image.scan.height),
            alt: image.alt.map(str::to_string),
            attribution: Some(format!("{} · {}", image.authorship(images), images.license)),
            seq: 0,
        })
        .collect()
}
