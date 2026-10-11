//! The numbers a mapping derives for a resource: its position among its parent's members, how
//! many members it has, and how many links point at it.
//!
//! A count is taken over what the store holds: the members that get a row, and the links that
//! are written. It is written also when it is 0.

use std::collections::BTreeMap;

use cpe_core::remodel::{ClassIndex, Membership, Remodel};

use crate::draft::{names, Draft, Entry, Value};

/// Gives every resource the build kept its numbers: under the `seq` name of its class's
/// `member-of` its `seqnum`, as served; under each `member-count` the number of its members of
/// that class; under each `incoming-count` the number of links to it under the `via`
/// properties. A resource that links to another under two of them counts twice.
///
/// This runs after the properties are mapped and after a hook: it counts the links they gave,
/// and neither a resource the hook dropped nor a link to or from one.
pub(crate) fn fill<'a>(draft: &mut Draft<'a>, mapping: &'a Remodel) {
    // By (parent, the member's class).
    let mut members: BTreeMap<(&str, ClassIndex), i64> = BTreeMap::new();
    // By (target, the linking resource's class, the property's name).
    let mut links: BTreeMap<(&str, ClassIndex, &str), i64> = BTreeMap::new();
    for (entry, class, _) in draft.kept() {
        if let Some(member) = entry.member {
            *members.entry((member.parent, class)).or_insert(0) += 1;
        }
        for (name, given) in &entry.values {
            for value in given {
                let Value::Link(target) = value else {
                    continue;
                };
                if draft.get(target).is_some_and(Entry::is_kept) {
                    *links.entry((target, class, name)).or_insert(0) += 1;
                }
            }
        }
    }

    let mut numbers: Vec<(&str, &str, i64)> = Vec::new();
    for (entry, _, class) in draft.kept() {
        let iri = entry.resource.iri.as_str();
        if let (Some(Membership { seq: Some(seq), .. }), Some(member)) = (&class.member_of, entry.member) {
            numbers.push((iri, seq.as_str(), member.seqnum));
        }
        for count in &class.member_counts {
            let counted = members.get(&(iri, count.of)).copied().unwrap_or(0);
            numbers.push((iri, count.name.as_str(), counted));
        }
        for count in &class.incoming_counts {
            let from = mapping.class(count.from);
            let via = count.via.iter().filter_map(|&property| {
                let name = names(from.property(property)).next()?;
                links.get(&(iri, count.from, name)).copied()
            });
            numbers.push((iri, count.name.as_str(), via.sum()));
        }
    }
    for (iri, name, number) in numbers {
        draft.give(iri, [name], &[Value::Integer(number)]);
    }
}
