//! The Access Area's `media` capability at its minimum: [`LiveDepositedFiles`] serves CPE's bytes
//! port, [`DepositedFiles`] (ADR-0012), from the `media` stand-in subtree, re-read on every call.
//!
//! The subtree stands in for the Access bucket, and its only writer is the VRE bridge. A project is
//! known exactly when its Ready token exists, `<root>/<shortcode>/.ready`, read through
//! `shared_stand_in`; its files are those of the generation the token names,
//! `<root>/<shortcode>/<generation>/<name>`, and nothing outside that generation is served, the
//! token included. A project without a token is `UnknownProject`. A token that cannot be read or
//! parsed, or one naming a generation that is not a real directory, is `Unavailable`, never
//! `UnknownProject`: a broken project is not a removed one. The port's provenance of a project is
//! its token's.
//!
//! The adapter serves every file it holds and refuses none for want of a right: in the target each
//! file comes with its rights attached and the reader decides at request time with the shared
//! decision function (ADR-0012, ADR-0013). Not yet in code: rights on the port. The stand-in holds
//! public files only, and until an authorisation mechanism exists the reader decides as the
//! anonymous consumer.
//!
//! The composition root constructs the adapter over the directory `MEDIA_DATA_DIR` names, once, at
//! start: an edit to it reaches a reader only on a restart. Not yet in code: the port announces
//! change (DEV-7938). Nothing here writes to the subtree or watches it for changes: it may be NFS.

use std::path::{Path, PathBuf};
use std::{fs, io};

use cpe_ports::{DepositedFiles, FilesError, Provenance};
use shared_names::{is_file_name, is_shortcode};
use shared_stand_in::{generation_dir, read_token, ReadyToken, TokenError, TOKEN_FILE};

/// `media`'s adapter for CPE's bytes port: `<root>/<shortcode>/<generation>/<name>`, the generation
/// the project's Ready token names, read whole on every call.
///
/// A caller's string reaches the file system only after [`is_shortcode`] and [`is_file_name`]
/// accept it, and no component at or below the generation directory may be a symlink: the
/// generation directory and every directory of the name must be real directories, and the file a
/// regular file. The window between that check and the read is accepted, since only the VRE bridge
/// writes the subtree; the real bucket's adapter resolves without following symlinks instead
/// (`openat2`, `cap-std`).
pub struct LiveDepositedFiles {
    root: PathBuf,
}

impl LiveDepositedFiles {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The project's token; `Ok(None)` for an unknown project.
    fn token(&self, shortcode: &str) -> Result<Option<ReadyToken>, FilesError> {
        read_token(&self.root, shortcode).map_err(|error| {
            let kind = match error {
                TokenError::Unreadable(kind) => kind,
                TokenError::Invalid(_) => io::ErrorKind::InvalidData,
            };
            unavailable(&self.root.join(shortcode).join(TOKEN_FILE), io::Error::new(kind, error))
        })
    }
}

impl DepositedFiles for LiveDepositedFiles {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError> {
        let unknown_project = || FilesError::UnknownProject { shortcode: shortcode.to_string() };
        let unknown_file = || FilesError::UnknownFile { shortcode: shortcode.to_string(), name: name.to_string() };
        if !is_shortcode(shortcode) {
            return Err(unknown_project());
        }
        if !is_file_name(name) {
            return Err(unknown_file());
        }

        let Some(token) = self.token(shortcode)? else {
            return Err(unknown_project());
        };
        let Some(mut path) = generation_dir(&self.root, shortcode, &token.provenance.generation) else {
            return Err(unknown_project());
        };
        if entry(&path)? != Entry::Directory {
            return Err(unavailable(
                &path,
                io::Error::other("the generation the Ready token names is not a directory"),
            ));
        }
        let mut segments = name.split('/');
        let file = segments.next_back().unwrap_or(name);
        for segment in segments {
            path.push(segment);
            if entry(&path)? != Entry::Directory {
                return Err(unknown_file());
            }
        }
        path.push(file);
        match entry(&path)? {
            Entry::File => fs::read(&path).map_err(|error| unavailable(&path, error)),
            Entry::Directory => Err(unavailable(&path, io::ErrorKind::IsADirectory.into())),
            Entry::Neither => Err(unknown_file()),
        }
    }

    /// The token's provenance, `None` for an unknown project; a token that cannot be read or parsed
    /// is `Unavailable`, as for `file`.
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn provenance(&self, shortcode: &str) -> Result<Option<Provenance>, FilesError> {
        Ok(self.token(shortcode)?.map(|token| port_provenance(token.provenance)))
    }
}

/// The token's provenance as the port carries it.
fn port_provenance(provenance: shared_stand_in::Provenance) -> Provenance {
    Provenance {
        generation: provenance.generation.to_string(),
        as_of: provenance.as_of,
        server: provenance.server,
        interim_format_version: provenance.interim_format_version,
        transformation_version: provenance.transformation_version,
        inputs_digest: provenance.inputs_digest,
    }
}

/// What a path below `<root>` is, read without following a symlink.
#[derive(PartialEq)]
enum Entry {
    Directory,
    File,
    /// Nothing, a symlink, or neither a directory nor a regular file.
    Neither,
}

/// The entry at `path`. An error other than absence is `Unavailable`, so a directory the adapter
/// may not search is never taken for one that does not exist.
fn entry(path: &Path) -> Result<Entry, FilesError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(Entry::Directory),
        Ok(metadata) if metadata.is_file() => Ok(Entry::File),
        Ok(_) => Ok(Entry::Neither),
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => {
            Ok(Entry::Neither)
        }
        Err(error) => Err(unavailable(path, error)),
    }
}

/// An `Unavailable` whose source names the path, since it is logged and never served.
fn unavailable(path: &Path, error: io::Error) -> FilesError {
    FilesError::unavailable(io::Error::new(error.kind(), format!("cannot read {}: {error}", path.display())))
}

#[cfg(test)]
mod files_tests;
