//! The Access Area's `sync` capability at its minimum: [`LiveArchiveProjection`] serves CPE's
//! [`ArchiveProjection`] from the `sync` stand-in subtree, re-read on every call.
//!
//! A project is known exactly when its Ready token exists: `<root>/<shortcode>/.ready`, read
//! through `shared_stand_in`. A shortcode `is_shortcode` refuses is `UnknownProject` before
//! anything touches the file system, and so is a project without a token. A token that cannot be
//! read or parsed, one naming an interim format version outside [`READABLE`], or one naming a
//! generation that is not there makes the call `Unavailable`, never `UnknownProject`: a broken
//! project is not a removed one. The snapshot is `<root>/<shortcode>/<generation>/snapshot.nq`, the
//! Interim snapshot of the generation the token names, and nothing else is read. The generation
//! directory must be a real directory and `snapshot.nq` a regular file, each checked without
//! following a symlink, or the call is `Unavailable`. The window between that check and the read is
//! accepted, since only the VRE bridge writes the subtree.
//!
//! The file is in the interim format `areas/deposit/vre-bridge/lift/FORMAT.md` defines; `vocab.rs`
//! is transcribed from it. A new format version is added to [`READABLE`] before the bridge writes
//! it.
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
//! served ARK's shape again.
//!
//! Only what `FORMAT.md` lets a reader omit is omitted, never an error: a value whose `rdf:value`
//! datatype the port has no kind for, a representation whose type it has no `File` for, and facts
//! the port does not carry, such as the access rights, data classification and legal information.
//! Annotations (`oa:Annotation`) are served as resources.
//!
//! One violation is accepted, by decision: an edge to a value node that lacks `rdf:type dao:Value`.
//! Telling it from a link to an IRI outside the file, also a violation, would rest on the IRI's
//! shape, so both are dropped. An annotation target is never a value node, so one that is not a
//! resource of the file is refused ([`InvalidFact::UnknownTarget`]).
//!
//! `access-server` constructs the adapter, over the directory `SYNC_DATA_DIR` names, and hands
//! it to CPE's activation. Nothing here writes to the subtree or watches it for changes: it may be
//! NFS.

use std::path::{Path, PathBuf};
use std::{fs, io};

use cpe_ports::{contract, ArchiveProjection, ProjectSnapshot, ProjectionError};
use oxrdf::Quad;
use oxttl::NQuadsParser;
use shared_names::is_shortcode;
use shared_stand_in::{generation_dir, read_token, ReadyToken, TOKEN_FILE};

mod ark;
mod error;
mod mapping;
mod vocab;

pub use ark::ArkError;
pub use error::{InvalidFact, SnapshotError};

/// The interim format versions `sync-store` reads: the current one and, once there is one, the
/// version before it.
const READABLE: &[u32] = &[1];

/// The snapshot's file name in a generation's directory.
const SNAPSHOT_FILE: &str = "snapshot.nq";

/// `sync`'s adapter for CPE's port: the Interim snapshot of each project whose Ready token exists
/// in the subtree at `root`, read from the generation the token names on every call.
pub struct LiveArchiveProjection {
    root: PathBuf,
}

impl LiveArchiveProjection {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn serve(&self, shortcode: &str, token: &ReadyToken, generation: &Path) -> Result<ProjectSnapshot, SnapshotError> {
        let version = token.provenance.interim_format_version;
        if !READABLE.contains(&version) {
            return Err(SnapshotError::UnreadableFormat { path: self.root.join(shortcode).join(TOKEN_FILE), version });
        }
        let path = snapshot_file(generation)?;
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
        let unknown = || ProjectionError::UnknownProject { shortcode: shortcode.to_string() };
        // No caller string reaches the file system unless it is a shortcode.
        if !is_shortcode(shortcode) {
            return Err(unknown());
        }
        let token = match read_token(&self.root, shortcode) {
            Ok(Some(token)) => token,
            Ok(None) => return Err(unknown()),
            Err(source) => {
                let path = self.root.join(shortcode).join(TOKEN_FILE);
                return Err(ProjectionError::unavailable(SnapshotError::Token { path, source }));
            }
        };
        let Some(generation) = generation_dir(&self.root, shortcode, &token.provenance.generation) else {
            return Err(unknown());
        };
        self.serve(shortcode, &token, &generation).map_err(ProjectionError::unavailable)
    }
}

/// The snapshot's path in the directory `generation`, once the directory is found to be a real
/// directory and the snapshot a regular file, a symlink at either refused unfollowed.
fn snapshot_file(generation: &Path) -> Result<PathBuf, SnapshotError> {
    match fs::symlink_metadata(generation) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(not_a(generation, "a directory")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(SnapshotError::MissingGeneration { path: generation.to_path_buf() });
        }
        Err(source) => return Err(SnapshotError::Read { path: generation.to_path_buf(), source }),
    }
    let path = generation.join(SNAPSHOT_FILE);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(path),
        Ok(_) => Err(not_a(&path, "a regular file")),
        Err(source) => Err(SnapshotError::Read { path, source }),
    }
}

/// A `Read` error for an entry of the wrong kind; a symlink is one.
fn not_a(path: &Path, kind: &str) -> SnapshotError {
    SnapshotError::Read {
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, format!("not {kind}, or a symlink")),
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
mod ark_tests;
#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod snapshot_tests;
