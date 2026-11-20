//! The snapshot: one project's current state as sorted N-Quads, in the interim,
//! not-DAO format specified by `FORMAT.md` beside this crate's `BUILD.bazel`.
//! Every `§N` in this module tree is a section of that file, which is normative:
//! where code and `FORMAT.md` disagree, the code is wrong.
//!
//! [`Read::of`] reads the store once into an [`index::Index`] and classifies every
//! subject; [`build`] writes the selection, checking every predicate against the
//! closed list (§11), and fails once with every violation before any line is returned.
//!
//! `bazel test` does not run the real-dump tests (`real_dump_tests`,
//! `#[ignore]`d); after a change to the output, run
//! `just vre-bridge-lift-real-dump <absolute dump dir> <reference snapshot>` too.

mod annotation;
#[cfg(test)]
mod annotation_tests;
#[cfg(test)]
mod format_doc_tests;
pub(crate) mod index;
pub(crate) mod kinds;
mod link;
#[cfg(test)]
mod link_tests;
mod list;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod membership_tests;
mod mint;
#[cfg(test)]
mod real_dump_tests;
mod representation;
#[cfg(test)]
mod representation_tests;
mod resource;
#[cfg(test)]
pub(crate) mod tests;
mod value;
#[cfg(test)]
mod value_tests;
mod violations;
pub(crate) mod vocab;
mod writer;

use std::collections::BTreeMap;

use index::{Index, Ontology};
use kinds::Subjects;
pub use kinds::{Handling, Kind};
use oxigraph::model::NamedNode;
pub use value::ValueKind;
pub use violations::ViolationReport;
use violations::Violations;
pub use writer::Snapshot;

use crate::LiftError;
use crate::dump::Dump;
use crate::guard::Rights;
use crate::selection::Selection;

/// The data graph read once and every subject classified, with the closed-world
/// violations found so far. The selection and the guard read it; [`build`] consumes it.
pub(crate) struct Read {
    graph: NamedNode,
    ontology: Ontology,
    pub(crate) index: Index,
    pub(crate) subjects: Subjects,
    violations: Violations,
}

impl Read {
    pub(crate) fn of(dump: &Dump) -> Result<Read, LiftError> {
        let graph = mint::project_graph(&dump.data_graph).map_err(LiftError::Dump)?;
        let mut violations = Violations::default();
        let ontology = Ontology::build(dump).map_err(|e| LiftError::Dump(e.to_string()))?;
        let index = Index::build(dump, &mut violations).map_err(|e| LiftError::Dump(e.to_string()))?;
        let subjects = kinds::classify_subjects(&index, &ontology, &mut violations);
        Ok(Read { graph, ontology, index, subjects, violations })
    }
}

/// The whole snapshot of `selection` in memory, with `rights` on its resources and
/// representations, or a [`violations::ViolationReport`] listing every closed-world breach.
pub(crate) fn build(
    read: Read,
    selection: &Selection,
    rights: &Rights,
    stats: &mut Stats,
) -> Result<Snapshot, LiftError> {
    let Read { graph, ontology, index, subjects, mut violations } = read;
    for entry in index.entries() {
        if let Some(kind) = subjects.kind(entry.node.as_str()) {
            *stats.subjects_by_kind.entry(kind).or_default() += 1;
        }
    }

    let mut snapshot = Snapshot::new(graph);
    let lists = list::emit(&index, &subjects, &mut snapshot, stats, &mut violations);
    let links = link::Links::read(&index, &subjects, selection, &mut violations);
    resource::emit(
        &index,
        &subjects,
        selection,
        rights,
        &ontology,
        &lists,
        &links,
        &mut snapshot,
        stats,
        &mut violations,
    )?;

    stats.violations = violations.len();
    stats.lines = snapshot.len();
    violations.into_result()?;
    Ok(snapshot)
}

/// What one run counted; the caller logs it.
#[derive(Debug, Default)]
pub struct Stats {
    pub subjects_by_kind: BTreeMap<Kind, usize>,
    /// Resources emitted, by source class.
    pub resources_by_class: BTreeMap<String, usize>,
    /// Resource-level predicates, by handling (§6, §11).
    pub resource_predicates: BTreeMap<Handling, usize>,
    /// Value nodes emitted, by kind (§4).
    pub value_nodes_by_kind: BTreeMap<ValueKind, usize>,
    /// List nodes emitted (§9).
    pub list_nodes: usize,
    /// Links emitted (§6).
    pub links: usize,
    /// `dao:isPartOf` emitted (§7).
    pub memberships: usize,
    /// `dao:seqnum` emitted (§7).
    pub seqnums: usize,
    /// Representations emitted, by type (§8).
    pub representations_by_type: BTreeMap<&'static str, usize>,
    /// Annotations emitted, by motivation (§10).
    pub annotations_by_motivation: BTreeMap<&'static str, usize>,
    /// `oa:hasTarget` emitted (§10).
    pub annotation_targets: usize,
    /// Edges and nodes left out, by reason.
    pub omitted: BTreeMap<&'static str, usize>,
    pub violations: usize,
    pub lines: usize,
}
