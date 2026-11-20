//! The writing side of the stand-in contract, the VRE bridge's only code that writes the `sync`
//! and `media` subtrees: a new generation per refresh, the Ready tokens that make it current,
//! the rollback of a half-replaced pair, recovery, removal, and the lock and nightly-run record.
//!
//! The reading side, the layout and the token's encoding, is `shared-stand-in`'s; this crate
//! writes what it reads. A token is replaced by writing `.ready.<generation>.tmp` and renaming it
//! over `.ready`, `media` before `sync`; that rename within one directory is the only step
//! assumed atomic, since the subtrees may be NFS. No symlink is ever created. Nothing is written
//! outside `<root>/<shortcode>/` but the lock file and its record (with the record's temporary
//! file), and nothing deleted outside it but the leftover dump directories `sweep_work_dir`
//! removes from the work directory.
//!
//! Every call is synchronous `std::fs`: an async caller runs it in `spawn_blocking`.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use chrono::{DateTime, Utc};
use shared_names::is_shortcode;
use shared_stand_in::{read_token, Generation, ReadyToken, TokenError, TOKEN_FILE};

mod lock;
mod recovery;
mod writer;

pub use lock::{is_swept, sweep_work_dir, Lock};
pub use recovery::{RecoveryReport, RemovalReport};
pub use writer::{GenerationWriter, Sizes, RIGHTS_RECORD, RIGHTS_RECORD_FILE};

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not a shortcode: {0:?}")]
    Shortcode(String),
    #[error("not a file name a generation may hold: {0:?}")]
    FileName(String),
    /// A source tree holds a symlink or an entry that is neither a file nor a directory.
    #[error("not a regular file or directory: {}", .0.display())]
    NotRegular(PathBuf),
    /// The generation's directory exists in a subtree already: draw a new name.
    #[error("generation {0} exists already")]
    GenerationExists(Generation),
    #[error("{}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("internal: {0}")]
    Internal(String),
}

pub(crate) fn io_error(path: &Path) -> impl FnOnce(io::Error) -> StoreError + '_ {
    move |source| StoreError::Io { path: path.to_owned(), source }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtree {
    Sync,
    Media,
}

/// A project's Ready token in one subtree, as a refresh and recovery judge it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenState {
    Missing,
    Broken(Broken),
    Valid(ReadyToken),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Broken {
    /// The token is unreadable or invalid, a newer `token_version` included.
    Token(TokenError),
    /// The token is valid but its generation's directory is not there; kept, so the generations
    /// it names stay known.
    MissingGeneration(ReadyToken),
}

impl TokenState {
    /// The generations the token names as current and previous, or `None` when they cannot be
    /// known (a missing token names none).
    pub(crate) fn named_generations(&self) -> Option<Vec<Generation>> {
        match self {
            Self::Missing => Some(Vec::new()),
            Self::Broken(Broken::Token(_)) => None,
            Self::Valid(token) | Self::Broken(Broken::MissingGeneration(token)) => Some(
                [
                    Some(token.provenance.generation.clone()),
                    token.previous_generation.clone(),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
        }
    }
}

/// The two subtree roots. Both exist before the bridge starts; this crate creates nothing above
/// `<root>/<shortcode>/`.
#[derive(Debug, Clone)]
pub struct Subtrees {
    sync: PathBuf,
    media: PathBuf,
}

impl Subtrees {
    #[must_use]
    pub fn new(sync: PathBuf, media: PathBuf) -> Self {
        Self { sync, media }
    }

    #[must_use]
    pub fn root(&self, subtree: Subtree) -> &Path {
        match subtree {
            Subtree::Sync => &self.sync,
            Subtree::Media => &self.media,
        }
    }

    pub(crate) fn roots(&self) -> [&Path; 2] {
        [&self.sync, &self.media]
    }

    /// The project's tokens, `sync`'s first.
    #[must_use]
    pub fn read_tokens(&self, shortcode: &str) -> (TokenState, TokenState) {
        (token_state(&self.sync, shortcode), token_state(&self.media, shortcode))
    }

    /// The shortcode directories that hold a token in neither subtree, taken at a run's start so
    /// that removal deletes only what was already without a token then.
    #[must_use]
    pub fn tokenless_projects(&self) -> BTreeSet<String> {
        self.project_dirs()
            .into_iter()
            .filter(|shortcode| !self.roots().iter().any(|root| has_token(root, shortcode)))
            .collect()
    }

    /// The names passing `is_shortcode` of the real directories directly below either root; a
    /// root that cannot be listed adds none.
    pub(crate) fn project_dirs(&self) -> BTreeSet<String> {
        self.roots()
            .iter()
            .filter_map(|root| list_project_dirs(root).ok())
            .flatten()
            .collect()
    }
}

/// The names passing `is_shortcode` of the real directories directly below `root`.
pub(crate) fn list_project_dirs(root: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(root).map_err(io_error(root))? {
        let entry = entry.map_err(io_error(root))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        // `DirEntry::file_type` does not follow a symlink.
        if is_shortcode(&name) && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            names.push(name);
        }
    }
    Ok(names)
}

/// Whether anything is at the project's token path. An error other than "not there" counts as a
/// token, so nothing is deleted on a transient read error.
pub(crate) fn has_token(root: &Path, shortcode: &str) -> bool {
    match fs::symlink_metadata(root.join(shortcode).join(TOKEN_FILE)) {
        Ok(_) => true,
        Err(e) => !matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory),
    }
}

fn token_state(root: &Path, shortcode: &str) -> TokenState {
    match read_token(root, shortcode) {
        Ok(None) => TokenState::Missing,
        Err(e) => TokenState::Broken(Broken::Token(e)),
        Ok(Some(token)) => {
            let present = fs::symlink_metadata(root.join(shortcode).join(token.provenance.generation.as_str()))
                .is_ok_and(|m| m.is_dir());
            if present {
                TokenState::Valid(token)
            } else {
                TokenState::Broken(Broken::MissingGeneration(token))
            }
        }
    }
}

/// A new generation name for a refresh whose T is `as_of_utc`: `YYYYMMDDTHHMMSSZ-xxxxxxxx`, the
/// time and 8 random lowercase hex digits.
///
/// # Errors
///
/// [`StoreError::Internal`] when the name fails `Generation::parse`, as for a year past 9999.
pub fn new_generation_name(as_of_utc: &DateTime<Utc>) -> Result<Generation> {
    let name = format!("{}-{:08x}", as_of_utc.format("%Y%m%dT%H%M%SZ"), rand::random::<u32>());
    Generation::parse(&name).ok_or_else(|| StoreError::Internal(format!("{name:?} is not a generation name")))
}

/// Deletes every generation directory in `project` but those in `keep`, and returns how many it
/// deleted. Entries that are not a real directory named like a generation are left alone.
pub(crate) fn delete_generations_except(project: &Path, keep: &[Generation]) -> Result<usize> {
    let mut deleted = 0;
    for (name, kind, path) in entries(project)? {
        let Some(generation) = Generation::parse(&name) else {
            continue;
        };
        if kind.is_dir() && !keep.contains(&generation) {
            fs::remove_dir_all(&path).map_err(io_error(&path))?;
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Deletes every temporary token file, `.ready.*.tmp`, in `project`.
pub(crate) fn delete_temp_tokens(project: &Path) -> Result<()> {
    let prefix = format!("{TOKEN_FILE}.");
    for (name, kind, path) in entries(project)? {
        if name.starts_with(&prefix) && name.ends_with(".tmp") && !kind.is_dir() {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_error(&path)(e)),
                _ => {}
            }
        }
    }
    Ok(())
}

/// The UTF-8-named entries of the real directory `dir` with their kind, symlinks not followed;
/// none if `dir` does not exist.
fn entries(dir: &Path) -> Result<Vec<(String, fs::FileType, PathBuf)>> {
    match fs::symlink_metadata(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(dir)(e)),
        Ok(metadata) if !metadata.is_dir() => return Err(StoreError::NotRegular(dir.to_owned())),
        Ok(_) => {}
    }
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_error(dir))? {
        let entry = entry.map_err(io_error(dir))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let kind = entry.file_type().map_err(io_error(&entry.path()))?;
        found.push((name, kind, entry.path()));
    }
    Ok(found)
}

/// Sets `0755` on a directory this crate created: the umask masks a mode given at creation, and
/// a reader with another uid must read the subtrees.
pub(crate) fn set_dir_mode(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(io_error(path))
}

/// Flushes a directory's entries. Not every file system supports it (NFS among them), so a
/// failure is only logged.
pub(crate) fn sync_dir(path: &Path) {
    if let Err(e) = fs::File::open(path).and_then(|dir| dir.sync_all()) {
        tracing::debug!(path = %path.display(), error = %e, "directory fsync failed");
    }
}

#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod lock_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod writer_tests;
