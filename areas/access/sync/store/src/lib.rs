//! The Access Area's `sync` capability at its minimum: [`LiveArchiveProjection`] serves CPE's
//! [`ArchiveProjection`] from a committed snapshot file and a committed curation file per known
//! project, both re-read on every call.
//!
//! The snapshot file is interim-DAO N-Quads written by `dao-lift`, in the format `FORMAT.md`
//! defines at the `dao-lift` commit `areas/access/sync/data/PROVENANCE` pins; `vocab.rs` is
//! transcribed from it. Only the shortcodes in `KNOWN` are served; any other is `UnknownProject`.
//!
//! The curation file, `<shortcode>-curation.csv`, holds what the project's editors authored about
//! single resources, in the format `curation.rs` defines. It is hand-authored and edited in place,
//! never generated. Its values are served as the snapshot's
//! `curation`, beside the archive's facts and never as one of them. Its keys are opaque here: the
//! adapter names no project's key and gives none a meaning. A known project always has the file,
//! one with no curation a file holding the header alone: an absent file is `Unavailable`, as is
//! one that breaks the format, naming the first fault and its line as a [`CurationFault`].
//!
//! A `FORMAT.md` violation on a fact the port serves, or a resource IRI with no data ARK, makes the
//! whole call `Unavailable`, naming the rule as an [`InvalidFact`], as do a missing, unreadable,
//! malformed or empty snapshot file. The mapped snapshot then goes through
//! `cpe_ports::contract::violations`, and any violation is `Unavailable` as well
//! ([`SnapshotError::Contract`]), whether the snapshot file or the mapping is at fault.
//!
//! Where a check goes: a rule the contract states over served facts compared with one another, an
//! inverted date, a membership or list-node cycle and two siblings sharing a position, is the
//! contract's alone and never reimplemented in the mapping. Every other `FORMAT.md` rule is the
//! mapping's, as an `InvalidFact`, including a value node two resources share (the contract checks
//! UUIDs only within one resource). Where the contract checks a mapping rule again (a missing value
//! UUID, a dangling parent, list node or annotation target, an annotation without a target), the
//! mapping's rule names the fault first. The ARK is derived from the resource IRI, not read from
//! the file; the mapping's derivation refuses an IRI with no data ARK, and the contract checks the
//! served ARK's shape again. The curation file's rules are the curation reader's, each an
//! [`InvalidCuration`]. Three of them the contract checks again (a value for a resource that is
//! not in the snapshot, a repeated resource, key and language, and a key or language that is no
//! curation name), and the reader names the fault first, with its line.
//!
//! Only what `FORMAT.md` lets a reader omit is omitted, never an error: a value whose `rdf:value`
//! datatype the port has no kind for, a representation whose type it has no `File` for, and facts
//! the port does not carry. Annotations (`oa:Annotation`) are served as resources.
//!
//! One violation is accepted, by decision: an edge to a value node that lacks `rdf:type dao:Value`.
//! Telling it from a link to an IRI outside the file, also a violation, would rest on the IRI's
//! shape, so both are dropped. An annotation target is never a value node, so one that is not a
//! resource of the file is refused ([`InvalidFact::UnknownTarget`]).
//!
//! Nothing constructs the adapter yet; wiring it into `access-server` is DEV-7400.

use std::fs;
use std::path::{Path, PathBuf};

use cpe_ports::{contract, ArchiveProjection, CuratedValue, ProjectSnapshot, ProjectionError, Resource};
use oxrdf::Quad;
use oxttl::NQuadsParser;

mod ark;
mod curation;
mod error;
mod mapping;
mod vocab;

pub use ark::ArkError;
pub use error::{CurationFault, InvalidCuration, InvalidFact, SnapshotError};

/// The projects `sync` holds. A shortcode outside this set is `UnknownProject`, whatever the
/// directory contains; one inside it whose files cannot be served is `Unavailable`.
const KNOWN: &[&str] = &["0803"];

/// `sync`'s adapter for CPE's port: the committed interim-DAO snapshot of each known project and
/// its committed curation, read from `<dir>/<shortcode>.nq` and `<dir>/<shortcode>-curation.csv` on
/// every call.
pub struct LiveArchiveProjection {
    dir: PathBuf,
}

impl LiveArchiveProjection {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn serve(&self, shortcode: &str) -> Result<ProjectSnapshot, SnapshotError> {
        let path = self.dir.join(format!("{shortcode}.nq"));
        let quads = read_quads(&path)?;
        let mut snapshot = mapping::map(shortcode, &quads).map_err(|invalid| SnapshotError::Invalid {
            path: path.clone(),
            subject: invalid.subject,
            reason: invalid.reason,
        })?;
        if snapshot.resources.is_empty() {
            return Err(SnapshotError::Empty { path });
        }
        let curation_path = self.dir.join(format!("{shortcode}-curation.csv"));
        snapshot.curation = read_curation(&curation_path, &snapshot.resources)?;
        let violations = contract::violations(shortcode, &snapshot);
        if !violations.is_empty() {
            return Err(SnapshotError::Contract { path, violations });
        }
        Ok(snapshot)
    }
}

impl ArchiveProjection for LiveArchiveProjection {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
        // Both paths and the served shortcode come from `KNOWN`, never from the argument, so no
        // caller string reaches the filesystem.
        let Some(known) = KNOWN.iter().copied().find(|known| *known == shortcode) else {
            return Err(ProjectionError::UnknownProject { shortcode: shortcode.to_string() });
        };
        self.serve(known).map_err(ProjectionError::unavailable)
    }
}

/// The file's quads in file order, strictly parsed. Duplicates are left to the mapping.
fn read_quads(path: &Path) -> Result<Vec<Quad>, SnapshotError> {
    let bytes = fs::read(path).map_err(|source| SnapshotError::Read { path: path.to_path_buf(), source })?;
    NQuadsParser::new()
        .for_slice(&bytes)
        .collect::<Result<_, _>>()
        .map_err(|source: oxttl::TurtleSyntaxError| SnapshotError::Syntax {
            path: path.to_path_buf(),
            line: source.location().start.line + 1,
            source,
        })
}

/// The curated values of the curation file at `path`, each on one of `resources`.
fn read_curation(path: &Path, resources: &[Resource]) -> Result<Vec<CuratedValue>, SnapshotError> {
    let bytes = fs::read(path).map_err(|source| SnapshotError::Read { path: path.to_path_buf(), source })?;
    let known = resources.iter().map(|resource| resource.iri.as_str()).collect();
    curation::parse(&bytes, &known).map_err(|fault| SnapshotError::Curation { path: path.to_path_buf(), fault })
}

#[cfg(test)]
mod ark_tests;
#[cfg(test)]
mod curation_tests;
#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod snapshot_tests;
