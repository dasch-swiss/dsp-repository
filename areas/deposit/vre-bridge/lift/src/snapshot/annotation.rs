//! Annotations (§10): a resource whose class is `kb:Region` or `kb:LinkObj`, or
//! reaches one by `rdfs:subClassOf*`, is also `a oa:Annotation` with one
//! `oa:motivatedBy` and one `oa:hasTarget` per distinct target of the carried
//! links of its target property.

use std::collections::BTreeSet;

use oxigraph::model::NamedNodeRef;

use super::index::Ontology;
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, vocab};
use crate::model::{kb, nn};

/// What a resource's edges contribute to its annotation, gathered while its
/// predicates are read. Targets come from here, never from a second
/// `Links::carried` call, which would count and report each link twice.
#[derive(Default)]
pub(crate) struct Ledger<'i> {
    /// Each carried link (§6): predicate and target.
    links: Vec<(NamedNodeRef<'i>, NamedNodeRef<'i>)>,
    /// The source property of each value that is not deleted or superseded (§11).
    values: Vec<NamedNodeRef<'i>>,
}

impl<'i> Ledger<'i> {
    pub(crate) fn link(&mut self, p: NamedNodeRef<'i>, target: NamedNodeRef<'i>) {
        self.links.push((p, target));
    }

    pub(crate) fn value(&mut self, p: NamedNodeRef<'i>) {
        self.values.push(p);
    }
}

/// Emit `a oa:Annotation`, `oa:motivatedBy` and the `oa:hasTarget`s of `s` when
/// its `class` is an annotation class; nothing for any other resource.
pub(crate) fn emit(
    s: NamedNodeRef<'_>,
    class: &str,
    ledger: &Ledger<'_>,
    ontology: &Ontology,
    snapshot: &mut Snapshot,
    stats: &mut Stats,
    violations: &mut Violations,
) {
    let region = ontology.sub_class.reaches(class, kb::REGION);
    let link_obj = ontology.sub_class.reaches(class, kb::LINK_OBJ);
    let (motivation, target_property) = match (region, link_obj) {
        (false, false) => return,
        (true, true) => {
            violations.record(Rule::AmbiguousAnnotationClass, s.as_str(), class);
            return;
        }
        (false, true) => (vocab::OA_LINKING, kb::HAS_LINK_TO),
        (true, false) => {
            let sub = &ontology.sub_property;
            let commented = ledger.values.iter().any(|p| sub.reaches(p.as_str(), kb::HAS_COMMENT));
            let motivation = if commented {
                vocab::OA_COMMENTING
            } else {
                vocab::OA_HIGHLIGHTING
            };
            (motivation, kb::IS_REGION_OF)
        }
    };
    // Standoff links never reach the ledger: §6 rule 3 omits them before rule 4.
    let targets: BTreeSet<NamedNodeRef<'_>> = ledger
        .links
        .iter()
        .filter(|(p, _)| ontology.sub_property.reaches(p.as_str(), target_property))
        .map(|&(_, target)| target)
        .collect();
    if targets.is_empty() {
        violations.record(Rule::MissingAnnotationTarget, s.as_str(), target_property);
        return;
    }
    let has_target = nn(vocab::OA_HAS_TARGET);
    for target in &targets {
        snapshot.put(s, has_target.as_ref(), *target);
    }
    snapshot.put(s, nn(vocab::RDF_TYPE).as_ref(), nn(vocab::OA_ANNOTATION).as_ref());
    snapshot.put(s, nn(vocab::OA_MOTIVATED_BY).as_ref(), nn(motivation).as_ref());
    *stats.annotations_by_motivation.entry(motivation).or_default() += 1;
    stats.annotation_targets += targets.len();
}
