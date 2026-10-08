//! The Access Area's `sync` capability at its minimum: [`LiveArchiveProjection`] serves CPE's
//! [`ArchiveProjection`] from a committed snapshot file per known project, re-read on every call.
//!
//! The file is interim-DAO N-Quads written by `dao-lift`, in the format `FORMAT.md` defines at the
//! `dao-lift` commit `areas/access/sync/data/PROVENANCE` pins; `vocab.rs` is transcribed from it.
//! Only the shortcodes in `KNOWN` are served; any other is `UnknownProject`.
//!
//! A `FORMAT.md` violation on a fact the port serves makes the whole call `Unavailable`, naming the
//! rule as an [`InvalidFact`], as do a missing, unreadable, malformed or empty file. The mapped
//! snapshot then goes through `cpe_ports::contract::violations`, and any violation is `Unavailable`
//! as well ([`SnapshotError::Contract`]), whether the file or the mapping is at fault.
//!
//! Where a check goes: a rule the contract states over served facts compared with one another, an
//! inverted date, a membership or list-node cycle and two siblings sharing a position, is the
//! contract's alone and never reimplemented in the mapping. Every other `FORMAT.md` rule is the
//! mapping's, as an `InvalidFact`, including a value node two resources share (the contract checks
//! UUIDs only within one resource). Where the contract checks a mapping rule again (a missing value
//! UUID, a dangling parent, list node or annotation target, an annotation without a target), the
//! mapping's rule names the fault first.
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

use cpe_ports::{contract, ArchiveProjection, ProjectSnapshot, ProjectionError};
use oxrdf::Quad;
use oxttl::NQuadsParser;

mod error;
mod mapping;
mod vocab;

pub use error::{InvalidFact, SnapshotError};

/// The projects `sync` holds. A shortcode outside this set is `UnknownProject`, whatever the
/// directory contains; one inside it whose file cannot be served is `Unavailable`.
const KNOWN: &[&str] = &["0803"];

/// `sync`'s adapter for CPE's port: the committed interim-DAO snapshot of each known project,
/// read from `<dir>/<shortcode>.nq` on every call.
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
        let snapshot = mapping::map(shortcode, &quads).map_err(|invalid| SnapshotError::Invalid {
            path: path.clone(),
            subject: invalid.subject,
            reason: invalid.reason,
        })?;
        if snapshot.resources.is_empty() {
            return Err(SnapshotError::Empty { path });
        }
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
        // The path and the served shortcode come from `KNOWN`, never from the argument, so no
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

#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod snapshot_tests;
