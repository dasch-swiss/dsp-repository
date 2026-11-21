//! Membership: a resource of a class with `member-of` stands in one parent, at a position. A
//! class's `members` link a resource to the resources whose `part_of` names it.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::{ClassIndex, PropertyIndex, Remodel};
use cpe_ports::Resource;

use crate::draft::{Draft, EntryClass, Member, Value};
use crate::error::{BuildError, Rule};
use crate::generic::kinds::link_of;
use crate::generic::order::sort_positional;

/// The resources linking to each resource, by the linking resource's class and the archive
/// property they link under, then by the target's IRI.
type Linkers<'a> = BTreeMap<(ClassIndex, &'a str), BTreeMap<&'a str, BTreeSet<&'a str>>>;

/// Places every resource of a class with `member-of` in its parent. A member has exactly one
/// parent, a resource of the parent class, and a `seqnum`; one that has not is refused. Both
/// are read before either is required, so that a member lacking both is reported for both.
///
/// The parent is the resource its `part_of` names or, under a `via`, the resource of the parent
/// class whose link under `via` points at it, as the snapshot serves the link: this runs before
/// any value is mapped.
pub(crate) fn read<'a>(draft: &mut Draft<'a>, mapping: &'a Remodel, errors: &mut Vec<BuildError>) {
    let linkers = linkers(draft, mapping);
    let mut places: Vec<(&'a str, Option<Member<'a>>)> = Vec::new();
    for entry in draft.entries() {
        let EntryClass::Mapped(_, class) = entry.class else {
            continue;
        };
        let Some(membership) = &class.member_of else {
            continue;
        };
        let resource = entry.resource;
        let parent = match via_iri(mapping, membership.parent, membership.via) {
            None => {
                let parents: BTreeSet<&str> = resource.part_of.iter().map(|parent| parent.as_str()).collect();
                let of_parent_class =
                    |parent: &&str| draft.get(parent).is_some_and(|entry| entry.is_of(membership.parent));
                let fit = parents.iter().all(of_parent_class);
                one_parent(resource, &parents, fit, errors)
            }
            Some(property) => {
                let linking = linkers.get(&(membership.parent, property));
                let parents = linking.and_then(|targets| targets.get(resource.iri.as_str()));
                one_parent(resource, parents.unwrap_or(&BTreeSet::new()), true, errors)
            }
        };
        if resource.seqnum.is_none() {
            errors.push(BuildError {
                rule: Rule::PageWithoutSeqnum,
                subject: resource.iri.as_str().to_string(),
                detail: None,
            });
        }
        let member = parent.zip(resource.seqnum).map(|(parent, seqnum)| Member { parent, seqnum });
        places.push((resource.iri.as_str(), member));
    }
    for (iri, member) in places {
        draft.place(iri, member);
    }
}

/// The one parent of `resource` among `parents`, which are of the parent class when `fit`: none,
/// several, or one of another class is refused, naming them. A parent that was itself refused is
/// still the parent: its member is not refused for it.
fn one_parent<'a>(
    resource: &Resource,
    parents: &BTreeSet<&'a str>,
    fit: bool,
    errors: &mut Vec<BuildError>,
) -> Option<&'a str> {
    let only = parents.first().copied().filter(|_| parents.len() == 1 && fit);
    if only.is_none() {
        let named: Vec<&str> = parents.iter().copied().collect();
        let subject = resource.iri.as_str();
        errors.push(BuildError::refused(Rule::PageParent, subject, named.join(", ")));
    }
    only
}

/// The archive property of the `via` link of a membership whose parent class is `parent`.
fn via_iri(mapping: &Remodel, parent: ClassIndex, via: Option<PropertyIndex>) -> Option<&str> {
    let property = mapping.class(parent).property(via?);
    Some(property.source.iri()?.as_str())
}

/// Every link that makes a parent under a `via`: for each membership with one, the resources
/// of its parent class that link under the `via` property, by the resource each links to.
fn linkers<'a>(draft: &Draft<'a>, mapping: &'a Remodel) -> Linkers<'a> {
    let mut linkers = Linkers::new();
    for class in &mapping.classes {
        let Some(membership) = &class.member_of else {
            continue;
        };
        if let Some(property) = via_iri(mapping, membership.parent, membership.via) {
            linkers.entry((membership.parent, property)).or_default();
        }
    }
    for entry in draft.entries() {
        let EntryClass::Mapped(class, _) = entry.class else {
            continue;
        };
        for value in &entry.resource.values {
            let key = (class, value.property.as_str());
            let (Some(targets), Some(target)) = (linkers.get_mut(&key), link_of(&value.kind)) else {
                continue;
            };
            targets.entry(target).or_default().insert(entry.resource.iri.as_str());
        }
    }
    linkers
}

/// Gives every kept resource of a class with `members` the links to the kept resources of each
/// node's `from` class whose `part_of` names it, in positional order: by `seqnum`, one without
/// after those with one, then by the last part of the IRI. A resource without such a member
/// gets no value under the name.
///
/// This runs after a hook, so that it sees the hook's drops.
pub(crate) fn members<'a>(draft: &mut Draft<'a>, mapping: &'a Remodel) {
    let mut given: Vec<(&'a str, &'a str, Vec<Value<'a>>)> = Vec::new();
    for (index, class) in mapping.indexed_classes() {
        for node in &class.members {
            let mut of_owner: BTreeMap<&str, Vec<(i64, &'a str)>> = BTreeMap::new();
            for (member, _, _) in draft.kept().filter(|(entry, ..)| entry.is_of(node.from)) {
                let resource = member.resource;
                let parents: BTreeSet<&str> = resource.part_of.iter().map(|parent| parent.as_str()).collect();
                let placed = (resource.seqnum.unwrap_or(i64::MAX), resource.iri.as_str());
                for parent in parents {
                    of_owner.entry(parent).or_default().push(placed);
                }
            }
            for (owner, _, _) in draft.kept().filter(|(entry, ..)| entry.is_of(index)) {
                let Some(mut linked) = of_owner.remove(owner.resource.iri.as_str()) else {
                    continue;
                };
                sort_positional(&mut linked, |&(seqnum, iri)| (seqnum, iri));
                let links = linked.into_iter().map(|(_, iri)| Value::Link(iri)).collect();
                given.push((owner.resource.iri.as_str(), node.name.as_str(), links));
            }
        }
    }
    for (owner, name, links) in given {
        draft.give(owner, [name], &links);
    }
}

/// The kept members of every parent that has one, by the parent's IRI, each parent's in
/// positional order: by `seqnum`, then by the last part of the IRI.
pub(crate) fn positional<'a>(draft: &Draft<'a>) -> BTreeMap<&'a str, Vec<&'a str>> {
    let mut by_parent: BTreeMap<&str, Vec<(i64, &str)>> = BTreeMap::new();
    for entry in draft.entries() {
        if let Some(member) = entry.member.filter(|_| entry.is_kept()) {
            let placed = (member.seqnum, entry.resource.iri.as_str());
            by_parent.entry(member.parent).or_default().push(placed);
        }
    }
    let ordered = by_parent.into_iter().map(|(parent, mut members)| {
        sort_positional(&mut members, |&(seqnum, iri)| (seqnum, iri));
        let iris = members.into_iter().map(|(_, iri)| iri).collect();
        (parent, iris)
    });
    ordered.collect()
}

#[cfg(test)]
mod tests {
    use cpe_core::validate::load_remodel;
    use cpe_ports::{ClassIri, DataArk, ProjectSnapshot, ResourceIri, DATA_ARK_PREFIX};

    use super::*;
    use crate::generic::classes::classify;

    const CONFIG: &str = r#"
site {
    languages "en"
}
remodel {
    shortcode "0123"
    class "Work" iri="http://example.org/onto#Work" {
        id from="iri"
    }
    class "Leaf" iri="http://example.org/onto#Leaf" {
        member-of "Work"
        id from="iri"
    }
}
"#;

    fn resource(iri: &str, class: &str, parent: Option<&str>, seqnum: Option<i64>) -> Resource {
        Resource {
            iri: ResourceIri(iri.to_string()),
            ark: DataArk(format!("{DATA_ARK_PREFIX}0123/x")),
            class: ClassIri(format!("http://example.org/onto#{class}")),
            label: String::new(),
            values: Vec::new(),
            file: None,
            part_of: parent.map(|parent| ResourceIri(parent.to_string())).into_iter().collect(),
            seqnum,
            annotation: None,
        }
    }

    /// Two Works. The first has four Leaves, listed against their order: two share a `seqnum`,
    /// and one has none.
    #[test]
    fn test_positional_orders_each_parents_kept_members_by_seqnum_then_local_part() {
        let mapping = load_remodel(CONFIG).expect("the test's block lowers");
        let snapshot = ProjectSnapshot {
            shortcode: "0123".to_string(),
            resources: vec![
                resource("http://a.example/x/late", "Leaf", Some("urn:w1"), Some(7)),
                resource("urn:w2", "Work", None, None),
                resource("http://a.example/x/n", "Leaf", Some("urn:w1"), Some(2)),
                resource("http://a.example/x/lost", "Leaf", Some("urn:w1"), None),
                resource("urn:w1", "Work", None, None),
                resource("http://b.example/x/m", "Leaf", Some("urn:w1"), Some(2)),
                resource("http://a.example/x/only", "Leaf", Some("urn:w2"), Some(1)),
            ],
            list_nodes: Vec::new(),
        };
        let mut errors = Vec::new();
        let mut draft = classify(&snapshot, &mapping, &mut errors);

        read(&mut draft, &mapping, &mut errors);
        let members = positional(&draft);

        let refusals: Vec<(Rule, &str)> = errors.iter().map(|error| (error.rule, error.subject.as_str())).collect();
        assert_eq!(refusals, [(Rule::PageWithoutSeqnum, "http://a.example/x/lost")]);
        let expected: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
            (
                "urn:w1",
                vec![
                    "http://b.example/x/m",
                    "http://a.example/x/n",
                    "http://a.example/x/late",
                ],
            ),
            ("urn:w2", vec!["http://a.example/x/only"]),
        ]);
        assert_eq!(members, expected);
    }
}
