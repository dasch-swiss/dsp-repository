//! The end of a build: what was held on the resources is reported, the ids are compared, and
//! the store's rows are written from the draft, each table in the order the store file has it.

use std::collections::BTreeMap;

use cpe_core::remodel::Remodel;

use crate::draft::{Draft, Entry, Value};
use crate::error::{refuse_shared_keys, BuildError, Rule};
use crate::generic::{images, membership, rows, slug};
use crate::store::{CompoundPartRow, PropertyRow, ResourceRow, Store};

/// Fills `resources`, `properties`, `representations` and `compound_parts` of `store` from
/// `draft`, and collects in `errors` the refusals held on its resources and the ids that two
/// of them have. With a refusal the store is incomplete and must not be used.
pub(crate) fn fill(draft: &Draft<'_>, mapping: &Remodel, store: &mut Store, errors: &mut Vec<BuildError>) {
    for entry in draft.entries() {
        errors.extend(entry.held.iter().cloned());
    }
    refuse_repeated_ids(draft, errors);
    refuse_unsafe_ids(draft, mapping, errors);
    store.resources = resources(draft, mapping);
    store.properties = properties(draft, mapping);
    if let Some(block) = &mapping.images {
        store.representations = images::representations(draft, block);
    }
    store.compound_parts = compound_parts(draft);
}

/// One row per member the build kept, in a parent the build kept: a parent's members numbered
/// from 1 in positional order, whatever their `seqnum`s. The rows are by parent id and number.
fn compound_parts(draft: &Draft<'_>) -> Vec<CompoundPartRow> {
    let mut found = Vec::new();
    for (parent, members) in membership::positional(draft) {
        let Some(parent_id) = draft.get(parent).and_then(Entry::kept_id) else {
            continue;
        };
        let kept = members.iter().filter_map(|member| draft.get(member)?.kept_id());
        found.extend((1..).zip(kept).map(|(seq, child_id)| CompoundPartRow {
            parent_id: parent_id.to_string(),
            child_id: child_id.to_string(),
            seq,
        }));
    }
    found.sort_by(|a, b| (&a.parent_id, a.seq).cmp(&(&b.parent_id, b.seq)));
    found
}

/// Refuses every id that two resources of declared classes have. The classes are one id space.
fn refuse_repeated_ids(draft: &Draft<'_>, errors: &mut Vec<BuildError>) {
    let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for entry in draft.entries() {
        if let Some(id) = entry.id.as_deref() {
            let iri = entry.resource.iri.as_str();
            owners.entry(id).or_default().push(iri);
        }
    }
    refuse_shared_keys(Rule::DuplicateId, &owners, errors);
}

/// Refuses every id of a resource the build kept, and of an Image, that a URL does not carry
/// unchanged. A resource that was refused has no id to check: its refusal is the report.
fn refuse_unsafe_ids(draft: &Draft<'_>, mapping: &Remodel, errors: &mut Vec<BuildError>) {
    let kept = draft.entries().filter_map(|entry| {
        let id = entry.kept_id()?;
        Some((id, entry.resource.iri.as_str(), id))
    });
    slug::refuse_unsafe_ids(kept, errors);
    if mapping.images.is_some() {
        images::refuse_unsafe_ids(draft, errors);
    }
}

/// One row per resource the build kept and per Image: the classes in the order the mapping
/// declares them, each class by id, and the Images, by id, where the `images` block stands
/// among the classes.
fn resources(draft: &Draft<'_>, mapping: &Remodel) -> Vec<ResourceRow> {
    let mut found = Vec::new();
    let images = mapping.images.as_ref();
    let image_rows = |found: &mut Vec<ResourceRow>, classes_before: usize| {
        if let Some(block) = images.filter(|block| block.classes_before == classes_before) {
            found.append(&mut images::rows(draft, block));
        }
    };
    for (classes_before, (index, class)) in mapping.indexed_classes().enumerate() {
        image_rows(&mut found, classes_before);
        let mut rows = Vec::new();
        for entry in draft.entries().filter(|entry| entry.is_of(index)) {
            if let Some(id) = entry.kept_id() {
                rows.push(ResourceRow {
                    id: id.to_string(),
                    class: class.name.as_str().to_string(),
                    ark: Some(entry.resource.ark.as_str().to_string()),
                });
            }
        }
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        found.append(&mut rows);
    }
    image_rows(&mut found, mapping.classes.len());
    found
}

/// The values of every resource the build kept and of every Image, by resource, property and
/// number, whatever the class. A property's values are numbered from 0 in the order the mapping
/// gave them.
fn properties(draft: &Draft<'_>, mapping: &Remodel) -> Vec<PropertyRow> {
    let mut found = Vec::new();
    if let Some(block) = &mapping.images {
        found = images::properties(draft, block);
    }
    for entry in draft.entries() {
        let Some(id) = entry.kept_id() else {
            continue;
        };
        for (name, values) in &entry.values {
            let written = values.iter().filter_map(|value| row(draft, id, name, value));
            found.extend((0..).zip(written).map(|(seq, row)| PropertyRow { seq, ..row }));
        }
    }
    found.sort_by(|a, b| (&a.resource_id, &a.name, a.seq).cmp(&(&b.resource_id, &b.name, b.seq)));
    found
}

/// The row of one value. A link to a resource that gets no row gives none: that resource was
/// refused, and its refusal is the one report, or a hook dropped it, and the link is refused
/// where the hook's drops are followed up.
fn row(draft: &Draft<'_>, id: &str, name: &str, value: &Value<'_>) -> Option<PropertyRow> {
    match value {
        Value::Text(text, lang) => Some(rows::text(id, name, text, *lang)),
        Value::Date { start, end, precision, calendar, display } => {
            let bounds = (start.clone(), end.clone());
            Some(rows::date(id, name, bounds, *precision, *calendar, *display))
        }
        Value::Link(target) => {
            let target = draft.get(target)?.kept_id()?;
            Some(rows::link(id, name, target))
        }
        Value::ListNode(node) => Some(rows::list_value(id, name, node)),
        Value::Image(image) => Some(rows::link(id, name, image)),
        Value::Integer(number) => Some(rows::integer(id, name, *number)),
    }
}
