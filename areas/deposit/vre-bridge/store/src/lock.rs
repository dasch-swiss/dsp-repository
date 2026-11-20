//! The lock every refresh runs under, the nightly-run record beside it, and the sweep of dump
//! directories a killed process left behind.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::writer::write_new;
use crate::{io_error, sync_dir, Result, StoreError};

/// The lock the nightly run and the operator's refresh share, held until dropped. It is an
/// advisory lock on an open file description (`flock`), so a second open of the same path
/// conflicts even within one process; the file must be on a local disk.
#[derive(Debug)]
pub struct Lock {
    _file: File,
    path: PathBuf,
}

impl Lock {
    /// Takes the lock at `path`, creating the file if needed, without waiting: `None` while
    /// another holder has it. A waiting caller retries with an async sleep, never blocking a
    /// runtime thread.
    ///
    /// # Errors
    ///
    /// When the file cannot be opened or locking fails for another reason than a holder.
    pub fn try_acquire(path: &Path) -> Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(io_error(path))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file, path: path.to_owned() })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => Err(io_error(path)(e)),
        }
    }

    /// The nightly-run record, `<lock file>.nightly`.
    fn record_path(&self) -> PathBuf {
        let mut path = OsString::from(self.path.as_os_str());
        path.push(".nightly");
        PathBuf::from(path)
    }

    /// Whether the nightly run of `date` (local date) has started already. A record that cannot
    /// be read counts as no run.
    #[must_use]
    pub fn nightly_done_on(&self, date: NaiveDate) -> bool {
        let path = self.record_path();
        match fs::read_to_string(&path) {
            Ok(text) => NaiveDate::parse_from_str(text.trim_end(), "%Y-%m-%d") == Ok(date),
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "the nightly-run record cannot be read");
                false
            }
        }
    }

    /// Records that the nightly run of `date` has started, replacing the record by one rename.
    ///
    /// # Errors
    ///
    /// When the record cannot be written.
    pub fn record_nightly(&self, date: NaiveDate) -> Result<()> {
        let path = self.record_path();
        let mut temp = OsString::from(path.as_os_str());
        temp.push(".tmp");
        let temp = PathBuf::from(temp);
        let text = format!("{}\n", date.format("%Y-%m-%d"));
        write_new(&temp, text.as_bytes())
            .and_then(|()| fs::rename(&temp, &path))
            .map_err(io_error(&path))?;
        if let Some(parent) = path.parent() {
            sync_dir(parent);
        }
        Ok(())
    }
}

fn swept_name(name: &std::ffi::OsStr, prefix: &str) -> bool {
    !prefix.is_empty() && name.as_encoded_bytes().starts_with(prefix.as_bytes())
}

/// Whether [`sweep_work_dir`]`(work_dir, prefix)` would delete `path`: it is, or lies below, an
/// entry of `work_dir` whose name starts with `prefix`. Both paths are compared as given, so a
/// caller passes canonical ones.
#[must_use]
pub fn is_swept(work_dir: &Path, prefix: &str, path: &Path) -> bool {
    path.strip_prefix(work_dir)
        .ok()
        .and_then(|rest| rest.components().next())
        .is_some_and(|first| swept_name(first.as_os_str(), prefix))
}

/// Deletes every entry directly below `work_dir` whose name starts with `prefix`, judged without
/// following a symlink: a directory is removed recursively, anything else unlinked. Returns how
/// many entries it deleted. The caller holds the [`Lock`], so no running refresh's directory goes.
///
/// # Errors
///
/// [`StoreError::Internal`] for an empty `prefix`, which would match everything; an I/O error
/// when the directory cannot be listed, or the first one when an entry cannot be deleted, after
/// every other entry was tried.
pub fn sweep_work_dir(work_dir: &Path, prefix: &str) -> Result<usize> {
    if prefix.is_empty() {
        return Err(StoreError::Internal(
            "an empty prefix would sweep the whole work directory".to_owned(),
        ));
    }
    let mut deleted = 0;
    // An entry that cannot be deleted does not stop the sweep: the others may hold a dump too.
    let mut first_failure = None;
    for entry in fs::read_dir(work_dir).map_err(io_error(work_dir))? {
        let entry = entry.map_err(io_error(work_dir))?;
        if !swept_name(&entry.file_name(), prefix) {
            continue;
        }
        let path = entry.path();
        let removed = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(&path),
            Ok(_) => fs::remove_file(&path),
            Err(e) => Err(e),
        };
        match removed {
            Ok(()) => deleted += 1,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "deleting a leftover dump directory failed");
                first_failure.get_or_insert_with(|| io_error(&path)(e));
            }
        }
    }
    first_failure.map_or(Ok(deleted), Err)
}
