//! Writing one generation into both subtrees and making it current.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use shared_names::{is_file_name, is_shortcode};
use shared_stand_in::{read_token, Generation, ReadyToken, TOKEN_FILE};

use crate::{delete_generations_except, io_error, set_dir_mode, sync_dir, Result, StoreError, Subtree, Subtrees};

/// The rights record's name in every `media` generation; a shipped input may not take it.
pub const RIGHTS_RECORD_FILE: &str = "access-rights.json";

/// The rights record's bytes: every file of the generation is public and unclassified.
pub const RIGHTS_RECORD: &[u8] = br#"{"format":"access-rights","version":1,"default":{"access_rights":"public","data_classification":"unclassified"}}"#;

/// The bytes a writer has written so far, per subtree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sizes {
    pub sync: u64,
    pub media: u64,
}

/// A generation being written in both subtrees. Dropped or aborted before [`commit`](Self::commit),
/// it deletes what it created; no token names it before then.
pub struct GenerationWriter {
    subtrees: Subtrees,
    shortcode: String,
    generation: Generation,
    /// The generation directories this writer created, one per subtree.
    generation_dirs: Vec<PathBuf>,
    /// Project directories this writer created; removed again on abort if left empty.
    project_dirs: Vec<PathBuf>,
    /// Every directory this writer created, in creation order, so parents precede children.
    created_dirs: Vec<PathBuf>,
    sizes: Sizes,
    /// Set once nothing is to be deleted on drop: on abort, and once `commit` starts renaming,
    /// after which a token may name the generation.
    settled: bool,
    /// Runs between the `media` and the `sync` rename; an error stands for a failed `sync` rename.
    #[cfg(test)]
    pub(crate) before_sync_rename: Option<Box<dyn FnOnce() -> io::Result<()> + Send>>,
}

impl Subtrees {
    /// Creates `<root>/<shortcode>/<generation>/` in both subtrees, and the project directory
    /// where it is missing.
    ///
    /// # Errors
    ///
    /// [`StoreError::GenerationExists`] when the generation's directory exists in either subtree
    /// (the caller draws a new name); [`StoreError::Shortcode`] for a name failing
    /// `is_shortcode`; [`StoreError::NotRegular`] when the project's entry is not a directory.
    pub fn begin(&self, shortcode: &str, generation: Generation) -> Result<GenerationWriter> {
        if !is_shortcode(shortcode) {
            return Err(StoreError::Shortcode(shortcode.to_owned()));
        }
        let mut writer = GenerationWriter {
            subtrees: self.clone(),
            shortcode: shortcode.to_owned(),
            generation,
            generation_dirs: Vec::new(),
            project_dirs: Vec::new(),
            created_dirs: Vec::new(),
            sizes: Sizes::default(),
            settled: false,
            #[cfg(test)]
            before_sync_rename: None,
        };
        // On an early return the writer is dropped and deletes what it created.
        for root in self.roots() {
            let project = root.join(shortcode);
            match fs::create_dir(&project) {
                Ok(()) => {
                    writer.project_dirs.push(project.clone());
                    writer.created_dirs.push(project.clone());
                    set_dir_mode(&project)?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if !fs::symlink_metadata(&project).map_err(io_error(&project))?.is_dir() {
                        return Err(StoreError::NotRegular(project));
                    }
                }
                Err(e) => return Err(io_error(&project)(e)),
            }
            let dir = project.join(writer.generation.as_str());
            match fs::create_dir(&dir) {
                Ok(()) => {
                    writer.generation_dirs.push(dir.clone());
                    writer.created_dirs.push(dir.clone());
                    set_dir_mode(&dir)?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    return Err(StoreError::GenerationExists(writer.generation.clone()));
                }
                Err(e) => return Err(io_error(&dir)(e)),
            }
        }
        Ok(writer)
    }
}

impl GenerationWriter {
    #[must_use]
    pub fn generation(&self) -> &Generation {
        &self.generation
    }

    #[must_use]
    pub fn sizes(&self) -> Sizes {
        self.sizes
    }

    fn generation_root(&self, subtree: Subtree) -> PathBuf {
        self.subtrees.root(subtree).join(&self.shortcode).join(self.generation.as_str())
    }

    /// Writes a new file `name` (`/`-separated, creating its parent directories) into the
    /// generation and flushes it to disk.
    ///
    /// # Errors
    ///
    /// [`StoreError::FileName`] when `name` fails `is_file_name`; an I/O error when the file
    /// exists or cannot be written.
    pub fn write(&mut self, subtree: Subtree, name: &str, bytes: &[u8]) -> Result<()> {
        self.add_file(subtree, name, &mut &*bytes)
    }

    fn add_file(&mut self, subtree: Subtree, name: &str, content: &mut dyn Read) -> Result<()> {
        if !is_file_name(name) {
            return Err(StoreError::FileName(name.to_owned()));
        }
        let mut path = self.generation_root(subtree);
        let (parents, file) = name.rsplit_once('/').unwrap_or(("", name));
        for segment in parents.split('/').filter(|segment| !segment.is_empty()) {
            path.push(segment);
            self.ensure_dir(&path)?;
        }
        path.push(file);
        let written = create_file(&path, content).map_err(io_error(&path))?;
        let size = match subtree {
            Subtree::Sync => &mut self.sizes.sync,
            Subtree::Media => &mut self.sizes.media,
        };
        *size += written;
        Ok(())
    }

    fn ensure_dir(&mut self, path: &Path) -> Result<()> {
        match fs::create_dir(path) {
            Ok(()) => {
                self.created_dirs.push(path.to_owned());
                set_dir_mode(path)
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                if fs::symlink_metadata(path).map_err(io_error(path))?.is_dir() {
                    Ok(())
                } else {
                    Err(StoreError::NotRegular(path.to_owned()))
                }
            }
            Err(e) => Err(io_error(path)(e)),
        }
    }

    /// Copies the files below `src` into the generation under their relative names, streaming
    /// each. The whole tree is checked before anything is written, and each file opened must be
    /// the regular file the check saw.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotRegular`] for a symlink, or anything that is neither a file nor a
    /// directory, at or below `src`, or a file replaced since the check;
    /// [`StoreError::FileName`] for a relative name failing `is_file_name`.
    pub fn copy_tree(&mut self, subtree: Subtree, src: &Path) -> Result<()> {
        if !fs::symlink_metadata(src).map_err(io_error(src))?.is_dir() {
            return Err(StoreError::NotRegular(src.to_owned()));
        }
        let mut files = Vec::new();
        collect_files(src, "", &mut files)?;
        for (name, path, checked) in files {
            let mut source = File::open(&path).map_err(io_error(&path))?;
            // The file opened must be the regular file the check saw, not a symlink put there
            // since.
            let opened = source.metadata().map_err(io_error(&path))?;
            if !opened.is_file() || (opened.dev(), opened.ino()) != checked {
                return Err(StoreError::NotRegular(path));
            }
            self.add_file(subtree, &name, &mut source)?;
        }
        Ok(())
    }

    /// Writes [`RIGHTS_RECORD`] as [`RIGHTS_RECORD_FILE`] into the `media` generation.
    ///
    /// # Errors
    ///
    /// As [`write`](Self::write).
    pub fn write_rights_record(&mut self) -> Result<()> {
        self.write(Subtree::Media, RIGHTS_RECORD_FILE, RIGHTS_RECORD)
    }

    /// Makes the generation current: flushes its directories, replaces the `media` token, then
    /// the `sync` token, each by one rename, and deletes every generation but `token`'s current
    /// and previous one and those the replaced tokens named.
    ///
    /// If the `sync` token cannot be replaced, the `media` token is restored to the bytes it had
    /// before (or removed, if there was none). Once a rename has been tried, the new generation
    /// is left for recovery to delete on any failure: a rename reported failed may have taken
    /// effect on NFS, and a failed restore leaves a token naming it.
    ///
    /// # Errors
    ///
    /// [`StoreError::Internal`] when `token` names another generation or does not survive its own
    /// encoding; an I/O error when the `media` token cannot be read before, or either token cannot
    /// be replaced.
    pub fn commit(mut self, token: &ReadyToken) -> Result<()> {
        if token.provenance.generation != self.generation {
            return Err(StoreError::Internal(format!(
                "the token names {}, not the generation written, {}",
                token.provenance.generation, self.generation
            )));
        }
        let bytes = token.to_bytes();
        // A token a reader would refuse is never written.
        if ReadyToken::parse(&bytes).as_ref() != Ok(token) {
            return Err(StoreError::Internal("the token does not parse back to itself".to_owned()));
        }
        let sync_project = self.subtrees.root(Subtree::Sync).join(&self.shortcode);
        let media_project = self.subtrees.root(Subtree::Media).join(&self.shortcode);
        let captured = capture_token(&media_project.join(TOKEN_FILE))?;
        // A generation a readable token names until now stays, whatever `previous_generation`
        // says: a reader may be serving it.
        let replaced = [
            read_token(self.subtrees.root(Subtree::Sync), &self.shortcode).ok().flatten(),
            captured.as_deref().and_then(|bytes| ReadyToken::parse(bytes).ok()),
        ];

        for dir in self.created_dirs.iter().rev() {
            sync_dir(dir);
        }
        sync_dir(&sync_project);
        sync_dir(&media_project);

        let temp = format!("{TOKEN_FILE}.{}.tmp", self.generation);
        self.settled = true;
        let media_token = media_project.join(TOKEN_FILE);
        replace_token(&media_project, &temp, &bytes).map_err(io_error(&media_token))?;

        #[cfg(test)]
        let hooked = self.before_sync_rename.take().map_or(Ok(()), |hook| hook());
        #[cfg(not(test))]
        let hooked = Ok(());
        if let Err(e) = hooked.and_then(|()| replace_token(&sync_project, &temp, &bytes)) {
            self.restore_media_token(&media_project, captured.as_deref());
            return Err(io_error(&sync_project.join(TOKEN_FILE))(e));
        }

        let keep: Vec<Generation> = replaced
            .into_iter()
            .flatten()
            .map(|old| old.provenance.generation)
            .chain(
                [Some(self.generation.clone()), token.previous_generation.clone()]
                    .into_iter()
                    .flatten(),
            )
            .collect();
        for project in [&sync_project, &media_project] {
            if let Err(e) = delete_generations_except(project, &keep) {
                tracing::warn!(project = %self.shortcode, error = %e, "deleting old generations failed");
            }
        }
        Ok(())
    }

    fn restore_media_token(&self, media_project: &Path, captured: Option<&[u8]>) {
        let restored = match captured {
            Some(bytes) => {
                replace_token(media_project, &format!("{TOKEN_FILE}.{}.rollback.tmp", self.generation), bytes)
            }
            None => match fs::remove_file(media_project.join(TOKEN_FILE)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
        };
        match restored {
            Ok(()) => sync_dir(media_project),
            Err(e) => tracing::error!(
                project = %self.shortcode,
                error = %e,
                "restoring the media Ready token failed; recovery points it back to the sync token"
            ),
        }
    }

    /// Deletes the new generation and leaves the tokens untouched; as dropping the writer does.
    pub fn abort(mut self) {
        self.clean_up();
    }

    fn clean_up(&mut self) {
        self.settled = true;
        for dir in &self.generation_dirs {
            if let Err(e) = fs::remove_dir_all(dir) {
                if e.kind() != io::ErrorKind::NotFound {
                    tracing::warn!(path = %dir.display(), error = %e, "deleting an uncommitted generation failed");
                }
            }
        }
        for dir in &self.project_dirs {
            // Only if empty: nothing else may be lost.
            let _ = fs::remove_dir(dir);
        }
    }
}

impl Drop for GenerationWriter {
    fn drop(&mut self) {
        if !self.settled {
            self.clean_up();
        }
    }
}

/// The token's bytes, or `None` if there is none.
fn capture_token(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(path)(e)),
        Ok(metadata) if !metadata.is_file() => Err(StoreError::NotRegular(path.to_owned())),
        Ok(_) => fs::read(path).map(Some).map_err(io_error(path)),
    }
}

/// Replaces `<project>/.ready` with `bytes`: written to `<project>/<temp>`, flushed, renamed.
pub(crate) fn replace_token(project: &Path, temp: &str, bytes: &[u8]) -> io::Result<()> {
    let temp = project.join(temp);
    let replaced = write_new(&temp, bytes).and_then(|()| fs::rename(&temp, project.join(TOKEN_FILE)));
    if replaced.is_err() {
        let _ = fs::remove_file(&temp);
    }
    sync_dir(project);
    replaced
}

/// Writes `bytes` to a new file at `path`, replacing a leftover file but never following a
/// symlink there. Only for temporary token and record files, which a crash may leave behind;
/// a file in a generation is never replaced (`add_file`).
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    create_file(path, &mut &*bytes).map(|_| ())
}

/// Creates the file at `path` from `content` with mode `0644`, flushed, and returns its size.
/// `create_new` neither follows nor replaces anything already there; the mode is set on the
/// open file, since the umask masks a mode given at creation.
fn create_file(path: &Path, content: &mut dyn Read) -> io::Result<u64> {
    let mut out = OpenOptions::new().write(true).create_new(true).open(path)?;
    let written = io::copy(content, &mut out)?;
    out.set_permissions(fs::Permissions::from_mode(0o644))?;
    out.sync_all()?;
    Ok(written)
}

/// The regular files below `dir`, as (`/`-separated name relative to the tree's root, path,
/// device and inode).
fn collect_files(dir: &Path, prefix: &str, files: &mut Vec<(String, PathBuf, (u64, u64))>) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .map_err(io_error(dir))?
        .collect::<io::Result<Vec<_>>>()
        .map_err(io_error(dir))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let name = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if !is_file_name(&name) {
            return Err(StoreError::FileName(name));
        }
        // `DirEntry::file_type` does not follow a symlink.
        let kind = entry.file_type().map_err(io_error(&path))?;
        if kind.is_dir() {
            collect_files(&path, &name, files)?;
        } else if kind.is_file() {
            // `DirEntry::metadata` does not follow a symlink either.
            let metadata = entry.metadata().map_err(io_error(&path))?;
            files.push((name, path, (metadata.dev(), metadata.ino())));
        } else {
            return Err(StoreError::NotRegular(path));
        }
    }
    Ok(())
}
