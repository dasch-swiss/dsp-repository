//! The selection: what an interim snapshot is made from (`FORMAT.md` §11). The
//! public guard checks exactly this set and the snapshot is built from it, so
//! nothing reaches the output that the guard did not see.

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::model::Term;

use crate::model::{self, kb};
use crate::snapshot::index::Index;
use crate::snapshot::kinds::{self, Kind, Subjects};

/// The dump's current resources and values: resources not deleted; values
/// (value, file value and LinkValue nodes) neither deleted nor superseded, of a
/// selected resource.
pub(crate) struct Selection {
    resources: BTreeSet<String>,
    values: BTreeSet<String>,
    /// Every versioned subject that is not current state, and why.
    dropped: BTreeMap<String, Dropped>,
}

/// Why a versioned subject is not current state (§11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dropped {
    Deleted,
    Superseded,
}

impl Dropped {
    /// The stats line a dropped value is counted under.
    pub(crate) fn value_reason(self) -> &'static str {
        match self {
            Dropped::Deleted => "deleted value",
            Dropped::Superseded => "superseded value",
        }
    }

    /// The stats line a dropped file value is counted under.
    pub(crate) fn file_value_reason(self) -> &'static str {
        match self {
            Dropped::Deleted => "deleted file value (§8)",
            Dropped::Superseded => "superseded file value (§8)",
        }
    }
}

/// Select from the classified data graph. A value belongs to every resource that
/// names it as an object, and a LinkValue also to its `rdf:subject`; a value no
/// selected resource owns is not selected, whatever its own flags say.
pub(crate) fn select(index: &Index, subjects: &Subjects) -> Selection {
    // Every object of `kb:previousValue`: a version a newer one replaced. A deleted
    // head leaves nothing of its chain, since every older version is such an object.
    let superseded: BTreeSet<&str> = index
        .entries()
        .flat_map(|e| &e.triples)
        .filter(|(p, _)| p.as_str() == kb::PREVIOUS_VALUE)
        .filter_map(|(_, o)| match o {
            Term::NamedNode(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();

    let mut resources = BTreeSet::new();
    let mut dropped = BTreeMap::new();
    for entry in index.entries() {
        let iri = entry.node.as_str();
        match subjects.kind(iri) {
            Some(Kind::Resource) if !kinds::is_deleted(entry) => {
                resources.insert(iri.to_string());
            }
            Some(Kind::Value | Kind::FileValue | Kind::LinkValue) => {
                if kinds::is_deleted(entry) {
                    dropped.insert(iri.to_string(), Dropped::Deleted);
                } else if superseded.contains(iri) {
                    dropped.insert(iri.to_string(), Dropped::Superseded);
                }
            }
            _ => {}
        }
    }

    let current = |iri: &str| {
        let versioned = matches!(subjects.kind(iri), Some(Kind::Value | Kind::FileValue | Kind::LinkValue));
        versioned && !dropped.contains_key(iri)
    };
    let mut values = BTreeSet::new();
    // Every current value a selected resource names.
    for iri in &resources {
        let Some(entry) = index.get(iri) else {
            continue;
        };
        for (_, o) in &entry.triples {
            if let Term::NamedNode(n) = o {
                if current(n.as_str()) {
                    values.insert(n.as_str().to_string());
                }
            }
        }
    }
    // Every current LinkValue whose `rdf:subject` is a selected resource.
    for entry in index.entries() {
        let iri = entry.node.as_str();
        if subjects.kind(iri) == Some(Kind::LinkValue) && current(iri) {
            let owned = kinds::objects(entry, model::RDF_SUBJECT)
                .into_iter()
                .any(|o| matches!(o, Term::NamedNode(n) if resources.contains(n.as_str())));
            if owned {
                values.insert(iri.to_string());
            }
        }
    }
    Selection { resources, values, dropped }
}

/// The current file values of each selected resource, by resource IRI: a key
/// exists exactly for the resources the snapshot holds.
#[derive(Debug, Default)]
pub struct SelectedFiles(BTreeMap<String, Vec<SelectedFile>>);

/// One current file value a selected resource names, of any file-value class
/// (§8), the kinds omitted whole included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFile {
    pub class: String,
    /// `None` unless the value has exactly one `kb:internalFilename`.
    pub internal_filename: Option<String>,
}

impl SelectedFiles {
    /// The file values of `iri`, sorted by value IRI; `None` when the snapshot
    /// does not hold `iri` as a resource.
    pub fn of(&self, iri: &str) -> Option<&[SelectedFile]> {
        self.0.get(iri).map(Vec::as_slice)
    }
}

/// The selected file values each selected resource names.
pub(crate) fn files(index: &Index, subjects: &Subjects, selection: &Selection) -> SelectedFiles {
    let file = |iri: &str| {
        let class = subjects.get(iri)?.class.as_ref()?.as_str().to_string();
        let names = index
            .get(iri)
            .map(|e| kinds::objects(e, kb::INTERNAL_FILENAME))
            .unwrap_or_default();
        let internal_filename = match names.as_slice() {
            [Term::Literal(name)] => Some(name.value().to_string()),
            _ => None,
        };
        Some(SelectedFile { class, internal_filename })
    };
    let files = selection.resources().map(|resource| {
        let named: BTreeSet<&str> = index
            .get(resource)
            .into_iter()
            .flat_map(|e| &e.triples)
            .filter_map(|(_, o)| match o {
                Term::NamedNode(n) => Some(n.as_str()),
                _ => None,
            })
            .filter(|o| selection.has_value(o) && subjects.kind(o) == Some(Kind::FileValue))
            .collect();
        (resource.to_string(), named.into_iter().filter_map(file).collect())
    });
    SelectedFiles(files.collect())
}

impl Selection {
    /// The selected resources, sorted.
    pub(crate) fn resources(&self) -> impl Iterator<Item = &str> {
        self.resources.iter().map(String::as_str)
    }

    /// The selected values, sorted.
    pub(crate) fn values(&self) -> impl Iterator<Item = &str> {
        self.values.iter().map(String::as_str)
    }

    pub(crate) fn has_resource(&self, iri: &str) -> bool {
        self.resources.contains(iri)
    }

    pub(crate) fn has_value(&self, iri: &str) -> bool {
        self.values.contains(iri)
    }

    /// Why a versioned subject is not current state, or `None` when it is.
    pub(crate) fn dropped(&self, iri: &str) -> Option<Dropped> {
        self.dropped.get(iri).copied()
    }
}
