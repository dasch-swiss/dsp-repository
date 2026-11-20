//! Recovery of an interrupted refresh, and the removal of projects no longer on the allowlist.

use std::collections::BTreeSet;
use std::{fs, io};

use shared_names::is_shortcode;
use shared_stand_in::{Generation, TOKEN_FILE};

use crate::writer::replace_token;
use crate::{
    delete_generations_except, delete_temp_tokens, has_token, io_error, list_project_dirs, sync_dir, Result,
    StoreError, Subtree, Subtrees, TokenState,
};

#[derive(Debug, Default)]
pub struct RecoveryReport {
    /// Projects whose `media` token was pointed back to the `sync` token's provenance.
    pub repointed: Vec<String>,
    /// Projects whose generations were left in place because a token could not be read.
    pub skipped: Vec<String>,
    /// Generation directories deleted, in both subtrees together.
    pub deleted_generations: usize,
    pub failed: Vec<(String, StoreError)>,
}

#[derive(Debug, Default)]
pub struct RemovalReport {
    /// Projects whose tokens were removed; their directories go in a later run.
    pub withdrawn: Vec<String>,
    /// Projects whose directories were deleted in both subtrees.
    pub deleted: Vec<String>,
    pub failed: Vec<(String, StoreError)>,
}

/// What recovering one project did: whether its `media` token was pointed back, and how many
/// generations were deleted, `None` when deletion was skipped.
struct Recovered {
    repointed: bool,
    deleted: Option<usize>,
}

impl Subtrees {
    /// Recovers each allowlisted project as [`recover_project`](Self::recover_project) does; a
    /// project that fails is reported and the others go on.
    #[must_use]
    pub fn recover(&self, allowlist: &[String]) -> RecoveryReport {
        let mut report = RecoveryReport::default();
        for shortcode in allowlist {
            match self.recover_one(shortcode) {
                Ok(Recovered { repointed, deleted }) => {
                    if repointed {
                        report.repointed.push(shortcode.clone());
                    }
                    match deleted {
                        Some(deleted) => report.deleted_generations += deleted,
                        None => report.skipped.push(shortcode.clone()),
                    }
                }
                Err(e) => report.failed.push((shortcode.clone(), e)),
            }
        }
        report
    }

    /// Puts one project back into an agreeing state: when both tokens are valid with different
    /// provenance, the `media` token is rewritten from the `sync` token; then every generation
    /// neither token names as current or previous, and every temporary token file, is deleted.
    /// A lone token is left as it is. When a token cannot be read, no generation is deleted: what
    /// it names is unknown, and a reader may be serving it.
    ///
    /// # Errors
    ///
    /// [`StoreError::Shortcode`] for a name failing `is_shortcode`; an I/O error when a token
    /// cannot be rewritten or a deletion fails.
    pub fn recover_project(&self, shortcode: &str) -> Result<()> {
        self.recover_one(shortcode).map(|_| ())
    }

    fn recover_one(&self, shortcode: &str) -> Result<Recovered> {
        if !is_shortcode(shortcode) {
            return Err(StoreError::Shortcode(shortcode.to_owned()));
        }
        let mut repointed = false;
        let (sync, media) = match self.read_tokens(shortcode) {
            (TokenState::Valid(s), TokenState::Valid(m)) if s.provenance != m.provenance => {
                let project = self.root(Subtree::Media).join(shortcode);
                let temp = format!("{TOKEN_FILE}.{}.recover.tmp", s.provenance.generation);
                replace_token(&project, &temp, &s.to_bytes()).map_err(io_error(&project.join(TOKEN_FILE)))?;
                tracing::info!(project = shortcode, "media Ready token pointed back to the sync token");
                repointed = true;
                self.read_tokens(shortcode)
            }
            states => states,
        };
        let (Some(sync_names), Some(media_names)) = (sync.named_generations(), media.named_generations()) else {
            tracing::warn!(
                project = shortcode,
                "a Ready token cannot be read; every generation is left in place"
            );
            return Ok(Recovered { repointed, deleted: None });
        };
        let keep: Vec<Generation> = sync_names.into_iter().chain(media_names).collect();
        let mut deleted = 0;
        for root in self.roots() {
            let project = root.join(shortcode);
            deleted += delete_generations_except(&project, &keep)?;
            delete_temp_tokens(&project)?;
        }
        Ok(Recovered { repointed, deleted: Some(deleted) })
    }

    /// Withdraws the projects not on the allowlist: a shortcode directory with a token in either
    /// subtree loses its `sync` token, then its `media` token, and keeps its data; one that had no
    /// token at the run's start (`tokenless_at_start`) and still has none is deleted in both
    /// subtrees. Does nothing for an empty allowlist; never touches a name failing `is_shortcode`.
    ///
    /// # Errors
    ///
    /// When a subtree root cannot be listed; a project that fails is reported and the others go
    /// on.
    pub fn remove_unlisted(
        &self,
        allowlist: &[String],
        tokenless_at_start: &BTreeSet<String>,
    ) -> Result<RemovalReport> {
        let mut report = RemovalReport::default();
        if allowlist.is_empty() {
            return Ok(report);
        }
        let mut unlisted = BTreeSet::new();
        for root in self.roots() {
            unlisted.extend(
                list_project_dirs(root)?
                    .into_iter()
                    .filter(|shortcode| !allowlist.contains(shortcode)),
            );
        }
        for shortcode in unlisted {
            if self.roots().iter().any(|root| has_token(root, &shortcode)) {
                match self.withdraw(&shortcode) {
                    Ok(()) => report.withdrawn.push(shortcode),
                    Err(e) => report.failed.push((shortcode, e)),
                }
            } else if tokenless_at_start.contains(&shortcode) {
                match self.delete_project(&shortcode) {
                    Ok(()) => report.deleted.push(shortcode),
                    Err(e) => report.failed.push((shortcode, e)),
                }
            }
        }
        Ok(report)
    }

    /// Removes the `sync` token before the `media` token, the reverse of a replacement's order,
    /// so a reader that follows `sync` sees the project go before `media` changes.
    fn withdraw(&self, shortcode: &str) -> Result<()> {
        for root in self.roots() {
            let project = root.join(shortcode);
            let token = project.join(TOKEN_FILE);
            match fs::remove_file(&token) {
                Ok(()) => sync_dir(&project),
                Err(e) if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => {}
                Err(e) => return Err(io_error(&token)(e)),
            }
        }
        Ok(())
    }

    fn delete_project(&self, shortcode: &str) -> Result<()> {
        for root in self.roots() {
            let project = root.join(shortcode);
            match fs::symlink_metadata(&project) {
                Ok(metadata) if metadata.is_dir() => {
                    fs::remove_dir_all(&project).map_err(io_error(&project))?;
                }
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_error(&project)(e)),
                _ => {}
            }
        }
        Ok(())
    }
}
