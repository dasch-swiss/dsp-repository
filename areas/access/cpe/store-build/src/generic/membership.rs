//! Membership: a resource of a class with `member-of` stands in one parent, at a position.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::ClassIndex;
use cpe_ports::Resource;

use crate::draft::{Draft, EntryClass, Member};
use crate::error::{BuildError, Rule};
use crate::generic::order::sort_positional;

/// Places every resource of a class with `member-of` in its parent. A member has exactly one
/// parent, a resource of the parent class, and a `seqnum`; one that has not is refused. Both
/// are read before either is required, so that a member lacking both is reported for both.
pub(crate) fn read<'a>(draft: &mut Draft<'a>, errors: &mut Vec<BuildError>) {
    let mut places: Vec<(&'a str, Option<Member<'a>>)> = Vec::new();
    for entry in draft.entries() {
        let EntryClass::Mapped(_, class) = entry.class else {
            continue;
        };
        let Some(membership) = &class.member_of else {
            continue;
        };
        let resource = entry.resource;
        let parent = parent(resource, membership.parent, draft, errors);
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

/// The one parent of `resource`, which must be of `parent_class`. `part_of` may repeat a parent.
/// A parent that was itself refused is still the parent: its member is not refused for it.
fn parent<'a>(
    resource: &'a Resource,
    parent_class: ClassIndex,
    draft: &Draft<'_>,
    errors: &mut Vec<BuildError>,
) -> Option<&'a str> {
    let parents: BTreeSet<&str> = resource.part_of.iter().map(|parent| parent.as_str()).collect();
    let mut iter = parents.iter();
    let of_parent_class = |parent: &str| draft.get(parent).is_some_and(|entry| entry.is_of(parent_class));
    match (iter.next(), iter.next()) {
        (Some(parent), None) if of_parent_class(parent) => Some(parent),
        _ => {
            let parents: Vec<&str> = parents.into_iter().collect();
            let subject = resource.iri.as_str();
            errors.push(BuildError::refused(Rule::PageParent, subject, parents.join(", ")));
            None
        }
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

        read(&mut draft, &mut errors);
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
