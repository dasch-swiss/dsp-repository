//! Ids. A resource's id is the slug the editors curated for it, or is made from its label, from
//! the one value of one of its properties, or from its IRI, as its class's `id` node says.
//!
//! An id is part of a public URL. Every rule here gives the same id for the same snapshot, and
//! a resource whose id cannot be made is refused, never given another.

use std::collections::BTreeMap;

use cpe_core::remodel::{ClassIndex, IdPolicy, IdSource, Remodel, RemodelClass, RemodelProperty};
use cpe_ports::Resource;

use crate::curation_file::CuratedValue;
use crate::draft::{names, Draft, Entry, EntryClass, Value};
use crate::error::{refuse_shared_keys, BuildError, Rule};
use crate::generic::curation::Curation;
use crate::generic::membership;
use crate::generic::order::iri_local;
use crate::generic::slug::slug;
use crate::generic::text::py_strip;
use crate::mapping::label;
use crate::mapping::values::{self, NodeRefusals, Placed, Sources};

/// What one resource's id is made of, before the ids are compared and the members' are joined
/// to their parents'.
enum Made<'a> {
    /// The curated slug, which no other resource may have.
    Curated(&'a str),
    /// The whole id.
    Whole(String),
    /// The part after the parent's id, and the text it was made from. A part that is empty is
    /// refused only once the parent has an id: a member of a parent without one is not
    /// reported for an id it would not have got.
    UnderParent { local: String, from: &'a str },
    /// A refusal says why, or the label the id is made from was refused.
    Nothing,
}

/// What reading one resource for its id found.
struct Read<'a> {
    iri: &'a str,
    label_refused: bool,
    /// The property the id is made from and its values, when it was mapped.
    source: Option<(&'a RemodelProperty, Vec<Value<'a>>)>,
    held: Vec<BuildError>,
    made: Made<'a>,
}

/// Gives every resource of a declared class its id, or refuses it. The label is checked here,
/// for whatever reads it, and the property an id is made from is mapped here: a resource that
/// gets no id reports that and nothing else.
///
/// Two resources with one curated slug are both refused. A member's id starts with its
/// parent's, so parents come first. A member of a parent without an id gets none; what the
/// property its id is made from was refused for is still reported.
pub(crate) fn mint<'a>(
    draft: &mut Draft<'a>,
    curation: &Curation<'a>,
    mapping: &'a Remodel,
    errors: &mut Vec<BuildError>,
) {
    let mut reads = Vec::new();
    for (entry, index, class) in draft.kept() {
        let sources = Sources { mapping, curation, draft };
        reads.push(read(entry, (index, class), &sources, errors));
    }

    let mut slugs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut under_parent: BTreeMap<&str, (String, &str)> = BTreeMap::new();
    for read in reads {
        let iri = read.iri;
        if read.label_refused {
            draft.refuse_label(iri);
        }
        if let Some((node, mapped)) = read.source {
            draft.give(iri, names(node), &mapped);
        }
        draft.hold(iri, read.held);
        match read.made {
            Made::Curated(slug) => {
                slugs.entry(slug).or_default().push(iri);
                draft.name(iri, slug.to_string());
            }
            Made::Whole(id) => draft.name(iri, id),
            Made::UnderParent { local, from } => {
                under_parent.insert(iri, (local, from));
            }
            Made::Nothing => draft.refuse(iri),
        }
    }
    refuse_shared_keys(Rule::DuplicateSlug, &slugs, errors);
    for owner in slugs.values().filter(|owners| owners.len() > 1).flatten() {
        draft.refuse(owner);
    }
    mint_under_parents(draft, mapping, under_parent, errors);
}

/// Reads the resource of `entry`, which is of `class`, for its id.
fn read<'a>(
    entry: &Entry<'a>,
    (class_index, class): (ClassIndex, &'a RemodelClass),
    sources: &Sources<'a, '_>,
    errors: &mut Vec<BuildError>,
) -> Read<'a> {
    let resource = entry.resource;
    let iri = resource.iri.as_str();
    let mut read = Read {
        iri,
        label_refused: false,
        source: None,
        held: Vec::new(),
        made: Made::Nothing,
    };

    // A resource whose curated slug is refused is reported for that alone, so the slug is read
    // before the label is checked.
    let Some(origin) = origin(iri, class, sources.curation, errors) else {
        return read;
    };
    let as_text = label::is_read_as_text(resource, class, sources.curation);
    if as_text || matches!(origin, Origin::Source(IdSource::Label)) {
        let sentinel = sources.mapping.sentinel.as_deref();
        read.label_refused = label::refuse(resource, class, sentinel, as_text, errors);
    }
    let source = match origin {
        Origin::Slug(slug) => {
            read.made = Made::Curated(slug);
            return read;
        }
        Origin::Source(source) => source,
    };

    let made = local_part(resource, (class_index, class), source, &mut read, sources, errors);
    let under_parent = matches!(class.id, IdPolicy::Minted { under_parent: true, .. });
    read.made = match made {
        None => Made::Nothing,
        Some((local, from)) if under_parent => Made::UnderParent { local, from },
        Some((id, from)) => match refuse_empty(iri, id, from, errors) {
            Some(id) => Made::Whole(id),
            None => Made::Nothing,
        },
    };
    read
}

/// Where the id of one resource comes from.
enum Origin<'a> {
    /// The slug the editors curated.
    Slug(&'a str),
    Source(IdSource),
}

/// Where the id of the resource `iri`, which is of `class`, comes from. The source behind a
/// curated key is read only when the editors curated nothing under it. `None` when the curated
/// slug was refused.
fn origin<'a>(
    iri: &str,
    class: &RemodelClass,
    curation: &Curation<'a>,
    errors: &mut Vec<BuildError>,
) -> Option<Origin<'a>> {
    let (key, served) = match &class.id {
        IdPolicy::Minted { source, .. } => return Some(Origin::Source(*source)),
        IdPolicy::Curated { key, fallback } => {
            let served = curation.served(iri, key.as_str());
            match fallback {
                Some(fallback) if served.is_empty() => return Some(Origin::Source(*fallback)),
                _ => (key.as_str(), served),
            }
        }
    };
    curated_slug(iri, key, &served, curation, errors).map(Origin::Slug)
}

/// The part of the id of `resource` that is made from `source`, and the text it was made from.
/// `None` when a refusal says why, and when the label it is made from was refused. A property
/// the id is made from is mapped here, and `read` keeps what it gave and what was held.
fn local_part<'a>(
    resource: &'a Resource,
    (class_index, class): (ClassIndex, &'a RemodelClass),
    source: IdSource,
    read: &mut Read<'a>,
    sources: &Sources<'a, '_>,
    errors: &mut Vec<BuildError>,
) -> Option<(String, &'a str)> {
    let iri = resource.iri.as_str();
    match source {
        IdSource::Label if read.label_refused => None,
        IdSource::Label => Some((slug(&resource.label), resource.label.as_str())),
        IdSource::Iri => Some((iri_local(iri).to_string(), iri)),
        IdSource::Property(index) => {
            let node = class.property(index);
            let reported = errors.len();
            let mut refusals = NodeRefusals { kinds: errors, held: &mut read.held };
            let label_refused = read.label_refused;
            let placed = Placed { class_index, class, index, node };
            let mapped = values::of_node(sources, resource, label_refused, placed, &mut refusals);
            let text = match mapped.as_deref() {
                Some([Value::Text(text, _)]) => Some(*text),
                _ => None,
            };
            let silent = errors.len() == reported && read.held.is_empty() && !label_refused;
            read.source = mapped.map(|mapped| (node, mapped));
            match text {
                Some(text) => Some((slug(text), text)),
                // A property that may be without a value has none: nothing was refused, and
                // the resource must not go missing without a word.
                None if silent => Some((String::new(), "")),
                None => None,
            }
        }
    }
}

/// The slug the editors curated for the resource `iri` under `key`, without the whitespace
/// around it. The values are counted as served: there is exactly one, it holds no address and
/// no sentinel, on a resource of any class, and it is not empty.
fn curated_slug<'a>(
    iri: &str,
    key: &str,
    served: &[&'a CuratedValue],
    curation: &Curation<'_>,
    errors: &mut Vec<BuildError>,
) -> Option<&'a str> {
    let (rule, detail) = match served {
        [] if !curation.has_any(iri) => (Rule::BookWithoutCuration, None),
        [value] => {
            let value: &'a CuratedValue = value;
            match (curation.offence(&value.text), py_strip(&value.text)) {
                (Some(rule), _) => (rule, Some(key.to_string())),
                (None, "") => (Rule::EmptySlug, Some(key.to_string())),
                (None, slug) => return Some(slug),
            }
        }
        served => (Rule::SlugCount, Some(served.len().to_string())),
    };
    errors.push(BuildError { rule, subject: iri.to_string(), detail });
    None
}

/// `local`, the part of the id of the resource `iri` that was made from `from`; refused when
/// it is empty: `from` gave no character that can stand in an id.
fn refuse_empty(iri: &str, local: String, from: &str, errors: &mut Vec<BuildError>) -> Option<String> {
    if local.is_empty() {
        errors.push(BuildError::refused(Rule::EmptyIdSlug, iri, from));
        return None;
    }
    Some(local)
}

/// Gives each member in `under_parent`, by its IRI, the id `<parent id>-<local part>`. Among
/// one parent's members a local part that repeats gets `-2`, `-3`, … in positional order, so
/// the order of the members decides which of them keeps the bare id.
///
/// A parent that is itself a member has its id by the time its members get theirs: the parents
/// are taken by how many parents stand above them. A member of a parent without an id gets
/// none and is not refused for its own part; under a parent with one, a member whose part is
/// empty is refused and takes no number.
fn mint_under_parents<'a>(
    draft: &mut Draft<'a>,
    mapping: &Remodel,
    mut under_parent: BTreeMap<&'a str, (String, &'a str)>,
    errors: &mut Vec<BuildError>,
) {
    let members = membership::positional(draft);
    let mut parents: Vec<(usize, &str, &Vec<&str>)> = Vec::new();
    for (parent, members) in &members {
        if let Some(EntryClass::Mapped(_, class)) = draft.get(parent).map(|entry| entry.class) {
            parents.push((parents_above(mapping, class), *parent, members));
        }
    }
    parents.sort();
    for (_, parent, members) in parents {
        let prefix = draft.get(parent).and_then(Entry::kept_id).map(str::to_string);
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for &member in members {
            let Some((local, from)) = under_parent.remove(member) else {
                continue;
            };
            let Some(prefix) = &prefix else {
                draft.refuse(member);
                continue;
            };
            let Some(local) = refuse_empty(member, local, from, errors) else {
                draft.refuse(member);
                continue;
            };
            let count = seen.entry(local.clone()).or_insert(0);
            *count += 1;
            let id = if *count == 1 {
                format!("{prefix}-{local}")
            } else {
                format!("{prefix}-{local}-{count}")
            };
            draft.name(member, id);
        }
    }
}

/// How many parents stand above a resource of `class`. A mapping's memberships have no cycle.
fn parents_above(mapping: &Remodel, class: &RemodelClass) -> usize {
    let mut above = 0;
    let mut class = class;
    while let Some(membership) = &class.member_of {
        above += 1;
        class = mapping.class(membership.parent);
    }
    above
}
