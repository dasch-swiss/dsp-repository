//! Builds a project's CPE store from the `cpe-ports` snapshot of the project and from the
//! project's mapping, the `remodel` block of its config.
//!
//! [`build`] is the one entry. What a project's mapping says is done by `mapping` and
//! `generic`, which name no project. What it cannot say is the project's hook, a module under
//! `projects` with a row in [`hooks::HOOKS`]; a hook reaches the build only through the seam
//! of [`hook`].
//!
//! The library is pure: it reads nothing but what it is given. Reading a config, getting the
//! snapshot from an adapter and writing a file are its caller's.

use cpe_core::remodel::Remodel;
use cpe_ports::contract::{self, Violation};
use cpe_ports::{File, ProjectSnapshot, ResourceIri, ValueKind};

use crate::error::{BuildError, BuildErrors, Rule};
use crate::generic::curation::{self, Curation};
use crate::hook::Hook;
use crate::store::Store;

pub(crate) mod draft;
pub mod error;
pub(crate) mod finalise;
pub(crate) mod generic;
pub mod hook;
pub mod hooks;
pub(crate) mod mapping;
pub(crate) mod projects;
pub mod sql;
pub mod store;

pub use crate::generic::slug::is_url_safe;

/// `cpe-store`'s schema (`store/schema.sql`), which every built store must load under.
pub const SCHEMA_SQL: &str = include_str!("../../store/schema.sql");

/// The store of the project `mapping` describes, built from `snapshot`, from `mapping` and from
/// nothing else. It checks a resource's class, the keys of the curated values and a member's
/// parent, position and file, gives every resource of a declared class its id, the values of its
/// `property` and `curated` nodes, its image and its counts, checks every annotation, and
/// builds every table of the store. A table whose block the mapping does not have stays empty.
///
/// `hook` is the project's hook, as [`hooks::resolve`] gives it for the project's folder. It is
/// held to the mapping: a mapping that declares a hook needs one, and a mapping that declares
/// none takes none. A declared hook is called once, after the mapping gave every resource its
/// id and the values of its nodes, and before anything is derived from what the build keeps:
/// covers, counts, images, annotations and rows.
///
/// The port's contract is checked first, against the mapping's shortcode, and its violations
/// are returned alone: the rules rely on it, and only an adapter is certain to have checked
/// it. Every other refusal is collected. A resource that is refused takes no part in the rules
/// after its own, so a refusal that needs two resources may show only once the first is
/// mended.
pub fn build(snapshot: &ProjectSnapshot, mapping: &Remodel, hook: Option<&Hook>) -> Result<Store, BuildErrors> {
    let violations = contract::violations(mapping.shortcode.as_str(), snapshot);
    if let Some(errors) = BuildErrors::new(violations.iter().map(contract_error).collect()) {
        return Err(errors);
    }

    let mut errors = Vec::new();
    refuse_interior_nuls(snapshot, &mut errors);
    refuse_mapping_nuls(mapping, &mut errors);
    // A hook is called only when it is given and declared; either without the other is refused.
    let declared_hook = match (hook, &mapping.hook) {
        (Some(hook), Some(declared)) => Some((hook, declared)),
        (None, Some(_)) => {
            let project = mapping.shortcode.as_str();
            errors.push(BuildError::refused(
                Rule::HookNotRegistered,
                project,
                "the remodel block declares a hook",
            ));
            None
        }
        (Some(hook), None) => {
            errors.push(BuildError::refused(
                Rule::HookNotDeclared,
                hook.project,
                "the remodel block declares no hook",
            ));
            None
        }
        (None, None) => None,
    };

    let mut store = Store::default();
    generic::lists::fill(&mapping.lists, &mut store);
    let mut draft = generic::classes::classify(snapshot, mapping, &mut errors);
    let curation = Curation::read(snapshot, mapping.sentinel.as_deref());
    curation::refuse_misplaced_keys(&curation, &draft, mapping, &mut errors);
    generic::membership::read(&mut draft, &mut errors);
    generic::images::read(&mut draft, &mut errors);
    crate::mapping::ids::mint(&mut draft, &curation, mapping, &mut errors);
    generic::images::refuse_shared(&draft, mapping, &mut errors);
    crate::mapping::values::map(&mut draft, &curation, mapping, &mut errors);
    crate::mapping::curated::map(&mut draft, &curation, mapping);

    let (mut draft, curation) = match declared_hook {
        Some((hook, declared)) => hook::call(hook, draft, curation, mapping, declared, &mut errors),
        None => (draft, curation),
    };
    crate::mapping::values::after_hook(&mut draft, mapping, &mut errors);

    generic::images::covers(&mut draft, &curation, mapping, &mut errors);
    generic::counts::fill(&mut draft, mapping);
    finalise::fill(&draft, mapping, &mut store, &mut errors);
    generic::annotations::fill(&draft, &curation, mapping, &mut store, &mut errors);

    match BuildErrors::new(errors) {
        Some(errors) => Err(errors),
        None => Ok(store),
    }
}

/// The subject is the IRI the violation names first; the detail is the violation's own text.
fn contract_error(violation: &Violation) -> BuildError {
    let subject = match violation {
        Violation::ShortcodeMismatch { served, .. } => served.as_str(),
        Violation::DuplicateResource { resource }
        | Violation::MalformedDataArk { resource, .. }
        | Violation::DanglingLink { resource, .. }
        | Violation::DanglingTarget { resource, .. }
        | Violation::UntargetedAnnotation { resource }
        | Violation::DanglingParent { resource, .. }
        | Violation::MembershipCycle { resource }
        | Violation::InvertedDate { resource, .. }
        | Violation::MissingValueUuid { resource, .. }
        | Violation::DuplicateValueUuid { resource, .. }
        | Violation::LinkWithValueUuid { resource, .. }
        | Violation::DanglingCuration { resource }
        | Violation::DuplicateCuration { resource, .. }
        | Violation::MalformedCuration { resource, .. } => resource.as_str(),
        Violation::DanglingListNode { missing, .. } => missing.as_str(),
        Violation::DuplicateListNode { node } | Violation::ListNodeCycle { node } => node.as_str(),
        Violation::DuplicateSiblingPosition { parent, .. } => parent.as_str(),
    };
    BuildError::refused(Rule::ContractViolation, subject, violation.to_string())
}

/// Refuses U+0000 in every text of the mapping that can reach the store: the names and labels of
/// its lists, the names of its classes and of everything that yields a property, the images'
/// class, address and rights, and the site's languages. The subject is the mapping's shortcode
/// and the detail the node the text stands in. A text a hook gives is checked where it is given.
fn refuse_mapping_nuls(mapping: &Remodel, errors: &mut Vec<BuildError>) {
    let subject = mapping.shortcode.as_str();
    let mut check = |text: &str, place: &str| {
        if text.contains('\0') {
            errors.push(BuildError::refused(Rule::InteriorNul, subject, place));
        }
    };
    for language in &mapping.languages {
        check(language, "languages");
    }
    for list in &mapping.lists {
        check(&list.name, "list");
        for node in &list.nodes {
            check(&node.slug, "list-node");
            for label in &node.labels {
                check(&label.language, "list-node label");
                check(&label.text, "list-node label");
            }
        }
    }
    for class in &mapping.classes {
        check(class.name.as_str(), "class");
        let properties = class.properties.iter().flat_map(|node| &node.names);
        for name in properties {
            check(name.as_str(), "property");
        }
        for node in &class.curated {
            check(node.name.as_str(), "curated");
        }
        for count in &class.member_counts {
            check(count.name.as_str(), "member-count");
        }
        for count in &class.incoming_counts {
            check(count.name.as_str(), "incoming-count");
        }
        let seq = class.member_of.as_ref().and_then(|member| member.seq.as_ref());
        if let Some(seq) = seq {
            check(seq.as_str(), "member-of");
        }
    }
    if let Some(images) = &mapping.images {
        check(images.class.as_str(), "images");
        check(&images.iiif_base, "images");
        check(&images.authorship, "authorship");
        check(&images.license, "license");
    }
    let provides = mapping.hook.iter().flat_map(|hook| &hook.provides);
    for provided in provides {
        check(provided.name.as_str(), "provides");
    }
}

/// Refuses U+0000 in every string of the snapshot that can reach the store. The contract already
/// restricts an ARK and a curated value's key and language to characters that exclude it.
fn refuse_interior_nuls(snapshot: &ProjectSnapshot, errors: &mut Vec<BuildError>) {
    let mut check = |resource: &ResourceIri, text: &str, place: &str| {
        if text.contains('\0') {
            let subject = resource.as_str();
            errors.push(BuildError::refused(Rule::InteriorNul, subject, place));
        }
    };
    for resource in &snapshot.resources {
        check(&resource.iri, resource.iri.as_str(), "iri");
        check(&resource.iri, &resource.label, "label");
        match &resource.file {
            Some(
                File::StillImage { asset, .. }
                | File::Audio { asset }
                | File::MovingImage { asset }
                | File::Document { asset },
            ) => check(&resource.iri, asset, "file"),
            None => {}
        }
        for value in &resource.values {
            let property = value.property.as_str();
            match &value.kind {
                ValueKind::Text { text, lang } => {
                    check(&resource.iri, text, property);
                    check(&resource.iri, lang.as_deref().unwrap_or_default(), property);
                }
                ValueKind::Decimal(text)
                | ValueKind::Uri(text)
                | ValueKind::Geometry(text)
                | ValueKind::Color(text) => check(&resource.iri, text, property),
                ValueKind::Integer(_)
                | ValueKind::Boolean(_)
                | ValueKind::Date(_)
                | ValueKind::ListNode(_)
                | ValueKind::Link(_) => {}
            }
        }
    }
    for curated in &snapshot.curation {
        check(&curated.resource, &curated.text, &curated.key);
    }
}
