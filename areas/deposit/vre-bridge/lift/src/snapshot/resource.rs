//! Resources (§2), their edges (§3–§8) and their annotation structure (§10),
//! for the selected resources and values only, and without links no live
//! LinkValue or live target backs (§11).

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::model::{Literal, NamedNode, NamedNodeRef, Term};

use super::annotation::{self, Ledger};
use super::index::{Entry, Index, Ontology};
use super::kinds::{self, Handling, Kind, Subjects};
use super::link::{self, Links, Memberships};
use super::list::LiveLists;
use super::value::{self, ValueKind};
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, representation, vocab};
use crate::LiftError;
use crate::guard::{AccessRights, Rights};
use crate::model::{self, nn};
use crate::selection::{Dropped, Selection};

/// Emit `a dao:Resource`, `dao:sourceClass`, the plain `rdfs:label` and the
/// rights of every selected resource, its edges and, for an annotation, its §10
/// structure, and check each of its predicates against §6 and §11. `lists` holds
/// the live list nodes (§4.7), `links` what decides which links are carried (§6).
pub(crate) fn emit(
    index: &Index,
    subjects: &Subjects,
    selection: &Selection,
    rights: &Rights,
    ontology: &Ontology,
    lists: &LiveLists,
    links: &Links,
    snapshot: &mut Snapshot,
    stats: &mut Stats,
    violations: &mut Violations,
) -> Result<(), LiftError> {
    let mut run = Run {
        index,
        subjects,
        selection,
        rights,
        ontology,
        lists,
        links,
        snapshot,
        stats,
        violations,
        minted: BTreeMap::new(),
        parts: Memberships::new(),
    };
    for entry in index.entries() {
        let Some(subject) = subjects.get(entry.node.as_str()) else {
            continue;
        };
        if subject.kind != Kind::Resource {
            continue;
        }
        let class = subject
            .class
            .as_ref()
            .ok_or_else(|| internal("a resource is classified by its one rdf:type", entry.node.as_str()))?;
        if !selection.has_resource(entry.node.as_str()) {
            run.omit("deleted resource");
            continue;
        }
        run.resource(entry, class)?;
    }
    run.finish();
    Ok(())
}

/// One pass over the resources: what every edge rule reads, the output, and the
/// state the checks after the last resource need.
struct Run<'i, 'o> {
    index: &'i Index,
    subjects: &'i Subjects,
    selection: &'i Selection,
    rights: &'i Rights,
    ontology: &'i Ontology,
    lists: &'i LiveLists,
    links: &'i Links,
    snapshot: &'o mut Snapshot,
    stats: &'o mut Stats,
    violations: &'o mut Violations,
    /// Each value or representation node, and the dump values that minted it: two
    /// is a UUID clash (§3, §8). The prefixes keep the two kinds of node apart.
    minted: BTreeMap<String, BTreeSet<&'i str>>,
    /// Every emitted `dao:isPartOf`, for the cycle check (§7).
    parts: Memberships,
}

/// What one resource's predicates contribute that is decided only once all of
/// them are read (§7, §8, §10).
#[derive(Default)]
struct Pending<'i> {
    /// The carried links and live values its annotation structure reads.
    annotation: Ledger<'i>,
    /// Each live seqnum value.
    seqnums: Vec<Seqnum>,
    live_files: usize,
    /// The representation node the last live file value gave, and that file value.
    representation: Option<(NamedNode, &'i str)>,
}

/// A live seqnum edge's value (§7).
enum Seqnum {
    /// Not a valid seqnum: already recorded.
    Invalid,
    Integer(Literal),
}

impl<'i> Run<'i, '_> {
    fn resource(&mut self, entry: &'i Entry, class: &NamedNode) -> Result<(), LiftError> {
        let s = entry.node.as_ref();
        let mut pending = Pending::default();
        for (p, o) in &entry.triples {
            let Some(handling) = kinds::classify_resource_predicate(p.as_str(), o, self.ontology, self.subjects) else {
                self.violations.record(Rule::UnknownPredicate, s.as_str(), p.as_str());
                continue;
            };
            *self.stats.resource_predicates.entry(handling).or_default() += 1;
            let p = p.as_ref();
            match handling {
                Handling::Value => self.value(s, p, iri(s, o)?, &mut pending)?,
                Handling::Link => self.link(s, p, iri(s, o)?, &mut pending),
                Handling::Membership => self.membership(s, p, iri(s, o)?),
                Handling::Seqnum => pending.seqnums.extend(self.seqnum(s.as_str(), iri(s, o)?)?),
                Handling::FileValue => self.file_value(iri(s, o)?, &mut pending)?,
                // §2 structure is emitted below; §11's filter and omitted predicates,
                // standoff links and LinkValue companions are never emitted.
                Handling::Type
                | Handling::Label
                | Handling::Deleted
                | Handling::Admin
                | Handling::StandoffLink
                | Handling::LinkValueCompanion => {}
            }
        }
        self.finish_resource(entry, class, pending)
    }

    /// §3: the value's node and the edge to it, when the value is selected.
    fn value(
        &mut self,
        s: NamedNodeRef<'_>,
        p: NamedNodeRef<'i>,
        o: NamedNodeRef<'_>,
        pending: &mut Pending<'i>,
    ) -> Result<(), LiftError> {
        let (value, kind) = value_subject(o, self.index, self.subjects)?;
        if let Some(dropped) = self.unselected(value)? {
            self.omit(dropped.value_reason());
            return Ok(());
        }
        pending.annotation.value(p);
        let emitted = value::emit(p, value, kind, self.lists, self.snapshot, self.stats, self.violations);
        if let Some(node) = emitted {
            self.snapshot.put(s, p, node.as_ref());
            self.mint(node.as_str(), value.node.as_str());
        }
        Ok(())
    }

    /// §6: the direct triple itself, when it is carried.
    fn link(&mut self, s: NamedNodeRef<'_>, p: NamedNodeRef<'i>, o: NamedNodeRef<'i>, pending: &mut Pending<'i>) {
        let carried = self.links.carried(s.as_str(), p.as_str(), o, self.stats, self.violations);
        if let Some(target) = carried {
            self.snapshot.put(s, p, target);
            self.stats.links += 1;
            pending.annotation.link(p, target);
        }
    }

    /// §7: `dao:isPartOf` for a carried membership. Two sub-properties of
    /// `kb:isPartOf` naming one parent give one edge, counted once.
    fn membership(&mut self, s: NamedNodeRef<'_>, p: NamedNodeRef<'_>, o: NamedNodeRef<'_>) {
        let carried = self.links.carried(s.as_str(), p.as_str(), o, self.stats, self.violations);
        let Some(parent) = carried else {
            return;
        };
        let parents = self.parts.entry(s.as_str().to_string()).or_default();
        if parents.insert(parent.as_str().to_string()) {
            self.snapshot.put(s, nn(vocab::DAO_IS_PART_OF).as_ref(), parent);
            self.stats.memberships += 1;
        }
    }

    /// §7: the value of the seqnum edge `<s> … <o>`, or `None` when it is a
    /// deleted or superseded value (counted, and not one of the resource's seqnums).
    fn seqnum(&mut self, s: &str, o: NamedNodeRef<'_>) -> Result<Option<Seqnum>, LiftError> {
        let Some(value) = self.index.get(o.as_str()) else {
            self.violations.record(Rule::InvalidSeqnum, s, o.as_str());
            return Ok(Some(Seqnum::Invalid));
        };
        // A dump subject without a kind is already a violation (§11).
        let Some(subject) = self.subjects.get(o.as_str()) else {
            return Ok(None);
        };
        if subject.kind == Kind::Value {
            if let Some(dropped) = self.unselected(value)? {
                self.omit(dropped.value_reason());
                return Ok(None);
            }
        }
        let class = subject.class.as_ref().map(|c| c.as_str());
        if class.and_then(ValueKind::of_class) != Some(ValueKind::Integer) {
            self.violations.record(Rule::InvalidSeqnum, s, o.as_str());
            return Ok(Some(Seqnum::Invalid));
        }
        Ok(Some(match value::seqnum(value, self.violations) {
            Some(n) => Seqnum::Integer(n),
            None => Seqnum::Invalid,
        }))
    }

    /// §8: the representation node of a selected file value, into `pending`.
    fn file_value(&mut self, o: NamedNodeRef<'_>, pending: &mut Pending<'i>) -> Result<(), LiftError> {
        let (file, class) = file_value_subject(o, self.index, self.subjects)?;
        if let Some(dropped) = self.unselected(file)? {
            self.omit(dropped.file_value_reason());
            return Ok(());
        }
        pending.live_files += 1;
        let rights = rights_of(self.rights, file.node.as_str())?;
        let emitted = representation::emit(file, class, rights, self.snapshot, self.stats, self.violations);
        pending.representation = emitted.map(|node| (node, file.node.as_str()));
        Ok(())
    }

    /// The resource's seqnum, representation and annotation structure, then its
    /// §2 structure.
    fn finish_resource(&mut self, entry: &'i Entry, class: &NamedNode, pending: Pending<'i>) -> Result<(), LiftError> {
        let s = entry.node.as_ref();
        match pending.seqnums.as_slice() {
            [Seqnum::Integer(n)] => {
                self.snapshot.put(s, nn(vocab::DAO_SEQNUM).as_ref(), n);
                self.stats.seqnums += 1;
            }
            [] | [Seqnum::Invalid] => {}
            _ => self.violations.record(Rule::RepeatedSeqnum, s.as_str(), ""),
        }
        if pending.live_files > 1 {
            self.violations.record(Rule::MultipleRepresentations, s.as_str(), "");
        } else if let Some((node, file)) = pending.representation {
            self.snapshot.put(s, nn(vocab::DAO_HAS_REPRESENTATION).as_ref(), node.as_ref());
            self.mint(node.as_str(), file);
        }
        annotation::emit(
            s,
            class.as_str(),
            &pending.annotation,
            self.ontology,
            self.snapshot,
            self.stats,
            self.violations,
        );
        if let Some(label) = plain_label(entry, self.violations) {
            self.snapshot.put(s, nn(vocab::RDFS_LABEL).as_ref(), &label);
        }
        self.snapshot
            .put(s, nn(vocab::RDF_TYPE).as_ref(), nn(vocab::DAO_RESOURCE).as_ref());
        self.snapshot.put(s, nn(vocab::DAO_SOURCE_CLASS).as_ref(), class.as_ref());
        self.snapshot.put_rights(s, rights_of(self.rights, s.as_str())?);
        *self.stats.resources_by_class.entry(class.as_str().to_string()).or_default() += 1;
        Ok(())
    }

    /// `None` for a selected value, else why it is not selected. A value a
    /// selected resource names is selected unless it is deleted or superseded.
    fn unselected(&self, value: &Entry) -> Result<Option<Dropped>, LiftError> {
        let iri = value.node.as_str();
        if self.selection.has_value(iri) {
            return Ok(None);
        }
        match self.selection.dropped(iri) {
            Some(dropped) => Ok(Some(dropped)),
            None => Err(internal("a value of a selected resource is selected or dropped", iri)),
        }
    }

    fn omit(&mut self, reason: &'static str) {
        *self.stats.omitted.entry(reason).or_default() += 1;
    }

    fn mint(&mut self, node: &str, source: &'i str) {
        self.minted.entry(node.to_string()).or_default().insert(source);
    }

    /// The checks that need every resource: UUID clashes (§3, §8) and
    /// `dao:isPartOf` cycles (§7).
    fn finish(self) {
        for (node, sources) in &self.minted {
            if sources.len() > 1 {
                for source in sources {
                    self.violations.record(Rule::DuplicateUuid, source, node);
                }
            }
        }
        link::check_membership_cycles(&self.parts, self.violations);
    }
}

/// The IRI object the §6 classifier guarantees for every rule it assigns to an
/// edge of `s`.
fn iri<'o>(s: NamedNodeRef<'_>, o: &'o Term) -> Result<NamedNodeRef<'o>, LiftError> {
    let Term::NamedNode(o) = o else {
        return Err(internal("the §6 classifier assigns a rule only to an IRI object", s.as_str()));
    };
    Ok(o.as_ref())
}

/// The rights the guard gave `iri`, a selected subject.
fn rights_of(rights: &Rights, iri: &str) -> Result<AccessRights, LiftError> {
    rights
        .of(iri)
        .ok_or_else(|| internal("the guard gave every selected subject its rights", iri))
}

/// A classifier guarantee that did not hold for `subject`, an IRI.
fn internal(what: &'static str, subject: &str) -> LiftError {
    LiftError::Internal { what, subject: subject.to_string() }
}

/// The index entry and kind of a value edge's object, which the §6 classifier
/// has already found to be a `Kind::Value` subject.
fn value_subject<'i>(
    o: NamedNodeRef<'_>,
    index: &'i Index,
    subjects: &Subjects,
) -> Result<(&'i Entry, ValueKind), LiftError> {
    let class = subjects
        .get(o.as_str())
        .and_then(|s| s.class.as_ref())
        .ok_or_else(|| internal("a value subject is classified by its one rdf:type", o.as_str()))?;
    let kind = ValueKind::of_class(class.as_str())
        .ok_or_else(|| internal("a value subject's class is a §4 kind", o.as_str()))?;
    let entry = index
        .get(o.as_str())
        .ok_or_else(|| internal("a classified subject is in the index", o.as_str()))?;
    Ok((entry, kind))
}

/// The index entry and class of a file-value edge's object, which the §6
/// classifier has already found to be a `Kind::FileValue` subject.
fn file_value_subject<'i>(
    o: NamedNodeRef<'_>,
    index: &'i Index,
    subjects: &'i Subjects,
) -> Result<(&'i Entry, &'i str), LiftError> {
    let class = subjects
        .get(o.as_str())
        .and_then(|s| s.class.as_ref())
        .ok_or_else(|| internal("a file value is classified by its one rdf:type", o.as_str()))?;
    let entry = index
        .get(o.as_str())
        .ok_or_else(|| internal("a classified subject is in the index", o.as_str()))?;
    Ok((entry, class.as_str()))
}

/// The resource's one plain `rdfs:label` (§2), or `None` after recording why not.
fn plain_label(entry: &Entry, violations: &mut Violations) -> Option<Literal> {
    let s = entry.node.as_str();
    match kinds::objects(entry, model::RDFS_LABEL).as_slice() {
        [] => violations.record(Rule::MissingLabel, s, model::RDFS_LABEL),
        [Term::Literal(l)] if value::is_plain_string(l) => {
            return Some(Literal::new_simple_literal(l.value()));
        }
        [other] => violations.record(Rule::NonPlainLabel, s, &other.to_string()),
        _ => violations.record(Rule::RepeatedLabel, s, model::RDFS_LABEL),
    }
    None
}
