//! The Access Area's `media` capability at its minimum: [`LiveDepositedFiles`] serves CPE's bytes
//! port, [`DepositedFiles`] (ADR-0012), from a directory, re-read on every call.
//!
//! The directory stands in for the Access bucket. Each project is a subdirectory named by its
//! shortcode, holding the files it deposited by name: `<dir>/<shortcode>/<name>`. There is no list
//! of known projects: the directory is the list, as the bucket's prefixes will be.
//!
//! The adapter serves every file it holds and refuses none for want of a right: in the target each
//! file comes with its rights attached and the reader decides at request time with the shared
//! decision function (ADR-0012, ADR-0013). Not yet in code: rights on the port. The stand-in holds
//! public files only, and until an authorisation mechanism exists the reader decides as the
//! anonymous consumer.
//!
//! The composition root constructs the adapter over the directory `MEDIA_DATA_DIR` names, once, at
//! start: data ships in the image, so an edit to it reaches a reader with a deploy. Not yet in
//! code: the port announces change (ADR-0012; not scheduled).

use std::path::{Path, PathBuf};
use std::{fs, io};

use cpe_ports::{is_file_name, is_shortcode, DepositedFiles, FilesError};

/// `media`'s adapter for CPE's bytes port: `<dir>/<shortcode>/<name>`, read whole on every call.
///
/// A caller's string reaches the file system only after [`is_shortcode`] and [`is_file_name`]
/// accept it, and no component below `<dir>` may be a symlink: the project directory and every
/// directory of the name must be real directories, and the file a regular file. The window between
/// that check and the read is accepted, since only an operator writes the directory; the real
/// bucket's adapter resolves without following symlinks instead (`openat2`, `cap-std`).
pub struct LiveDepositedFiles {
    dir: PathBuf,
}

impl LiveDepositedFiles {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
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

        let mut path = self.dir.join(shortcode);
        if entry(&path)? != Entry::Directory {
            return Err(unknown_project());
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
}

/// What a path below `<dir>` is, read without following a symlink.
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
