//! The digest of a project's shipped inputs, recorded in its Ready tokens so that a change to them
//! forces a refresh.
//!
//! Over the regular files below the project's inputs directory, sorted by their relative path's
//! bytes with `/` separators: for each, the path's UTF-8 bytes, one `0x00`, the content's length
//! as 8 bytes big-endian, the content. SHA-256 over the concatenation, written `sha256:` and 64
//! lowercase hex digits. Changing any of this changes every project's digest, and so refreshes
//! every project once.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use shared_names::is_file_name;

/// File names a `media` generation reserves for what the bridge writes itself; no shipped input
/// may have one, in any case and at any depth. `vre-bridge-store`'s `RIGHTS_RECORD_FILE` is the
/// first; a new file the bridge writes into a generation's root is added here. A stored file may
/// not take DaTEI's `file-layout.xml` either (`FILE_LAYOUT` in `datei.rs`).
pub const RESERVED_NAMES: [&str; 2] = ["access-rights.json", crate::FILE_MANIFEST];

/// A shipped input the bridge cannot use: a configuration error. Paths are relative to the
/// project's inputs directory (`.` for the directory itself).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    #[error("shipped input `{0}` is a symbolic link")]
    Symlink(String),
    #[error("shipped input `{0}` is neither a regular file nor a directory")]
    NotAFile(String),
    #[error("shipped input `{0}` does not have a name a project's file may have")]
    Name(String),
    #[error("shipped input `{0}` has a name the bridge reserves for a file it writes")]
    Reserved(String),
    #[error("shipped input `{0}` changed while it was read")]
    Changed(String),
    #[error("shipped input `{path}` cannot be read: {kind}")]
    Io { path: String, kind: io::ErrorKind },
}

/// The digest of the shipped inputs in `dir`, `sha256:` and 64 lowercase hex digits; the digest
/// of the empty sequence when `dir` does not exist.
///
/// Blocking: an async caller runs it in `spawn_blocking`.
///
/// # Errors
///
/// [`DigestError`] for a symbolic link (`dir` itself included), anything but a regular file or
/// directory, a path `is_file_name` refuses, an entry of any kind named as one of
/// [`RESERVED_NAMES`] in any ASCII case, or a file that cannot be read, is no regular file once
/// opened, or changes its length while it is read.
pub fn inputs_digest(dir: &Path) -> Result<String, DigestError> {
    let mut hasher = Sha256::new();
    for (relative, path) in &walk(dir)? {
        let mut file = File::open(path).map_err(|e| io_error(relative, &e))?;
        // `File::open` follows a symlink the walk did not see: check what was opened.
        let meta = file.metadata().map_err(|e| io_error(relative, &e))?;
        if !meta.is_file() {
            return Err(DigestError::NotAFile(relative.clone()));
        }
        let len = meta.len();
        hasher.update(relative.as_bytes());
        hasher.update([0u8]);
        hasher.update(len.to_be_bytes());
        // One byte past `len`, so a file that grew is told apart.
        let copied = io::copy(&mut (&mut file).take(len + 1), &mut hasher).map_err(|e| io_error(relative, &e))?;
        if copied != len {
            return Err(DigestError::Changed(relative.clone()));
        }
    }

    let mut digest = String::from("sha256:");
    for byte in hasher.finalize() {
        let _ = write!(digest, "{byte:02x}");
    }
    Ok(digest)
}

/// The relative paths of the shipped inputs in `dir`, sorted by their bytes; none when `dir`
/// does not exist. Blocking.
///
/// # Errors
///
/// As [`inputs_digest`], without reading the files' contents.
pub fn shipped_names(dir: &Path) -> Result<Vec<String>, DigestError> {
    Ok(walk(dir)?.into_iter().map(|(relative, _)| relative).collect())
}

/// Every regular file below `dir` as `(relative path, path)`, sorted by the relative path's bytes.
fn walk(dir: &Path) -> Result<Vec<(String, PathBuf)>, DigestError> {
    let mut files = Vec::new();
    match fs::symlink_metadata(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error(".", &e)),
        Ok(meta) if meta.file_type().is_symlink() => return Err(DigestError::Symlink(".".to_owned())),
        Ok(meta) if meta.is_dir() => collect(dir, "", &mut files)?,
        Ok(_) => return Err(DigestError::NotAFile(".".to_owned())),
    }
    files.sort_unstable_by(|(a, _), (b, _)| a.as_bytes().cmp(b.as_bytes()));
    Ok(files)
}

/// Every regular file below `dir` as `(relative path, path)`; `prefix` is `dir`'s relative path.
fn collect(dir: &Path, prefix: &str, files: &mut Vec<(String, PathBuf)>) -> Result<(), DigestError> {
    let shown = if prefix.is_empty() { "." } else { prefix };
    for entry in fs::read_dir(dir).map_err(|e| io_error(shown, &e))? {
        let entry = entry.map_err(|e| io_error(shown, &e))?;
        let name = entry.file_name();
        let relative = match (prefix, name.to_str()) {
            (_, None) => return Err(DigestError::Name(name.to_string_lossy().into_owned())),
            ("", Some(name)) => name.to_owned(),
            (prefix, Some(name)) => format!("{prefix}/{name}"),
        };
        if !is_file_name(&relative) {
            return Err(DigestError::Name(relative));
        }
        // `DirEntry::file_type` does not follow a symlink.
        let kind = entry.file_type().map_err(|e| io_error(&relative, &e))?;
        // Any kind, any case: a case-insensitive file system would map it onto the reserved name.
        let reserved = name
            .to_str()
            .is_some_and(|name| RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(name)));
        if reserved {
            return Err(DigestError::Reserved(relative));
        }
        if kind.is_symlink() {
            return Err(DigestError::Symlink(relative));
        } else if kind.is_dir() {
            collect(&entry.path(), &relative, files)?;
        } else if !kind.is_file() {
            return Err(DigestError::NotAFile(relative));
        } else {
            files.push((relative, entry.path()));
        }
    }
    Ok(())
}

fn io_error(path: &str, e: &io::Error) -> DigestError {
    DigestError::Io { path: path.to_owned(), kind: e.kind() }
}
