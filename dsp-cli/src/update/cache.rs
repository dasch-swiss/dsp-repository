//! On-disk cache for the update check at `~/.config/dsp-cli/update_check.toml`.
//! See ADR-0015.
//!
//! Modeled on [`crate::config::auth_cache::AuthCache`], with two deliberate
//! differences:
//! - This cache is a single flat struct (one file, two fields), not a per-server `BTreeMap`.
//! - **A malformed/oversize/unreadable file is never an error.** Every load failure degrades to
//!   [`UpdateCheckCache::default`] (logged at `tracing::debug!`). This cache is a disposable
//!   convenience — a corrupt copy must never break an unrelated command.
//! - Saves are plain atomic writes (temp-sibling + rename) with no `0600` permission tightening:
//!   this file holds no secret, unlike `auth.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;

/// Hard cap on `update_check.toml` size. A real cache holds a handful of
/// bytes; anything past 1 MiB is almost certainly a misconfigured symlink
/// and must not be slurped into memory. Unlike `AuthCache`, breaching this
/// cap degrades to `Self::default()` rather than erroring — see the module
/// doc comment.
const MAX_CACHE_FILE_BYTES: u64 = 1 << 20;

/// In-memory view of `~/.config/dsp-cli/update_check.toml`.
///
/// Load with [`UpdateCheckCache::load`] (or [`UpdateCheckCache::load_from`]
/// in tests). Persist with [`UpdateCheckCache::save`] (or
/// [`UpdateCheckCache::save_to`] in tests).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UpdateCheckCache {
    /// When the crates.io sparse index was last fetched (successfully or
    /// not — a failed attempt still counts, so the 24h backoff holds).
    pub last_checked: Option<DateTime<Utc>>,

    /// The highest stable version last observed on the index, as a plain
    /// string (re-parsed through `semver::Version` before use — see
    /// `run_check_and_notify`'s sanitisation guard).
    pub latest_seen: Option<String>,
}

impl UpdateCheckCache {
    /// The canonical cache path: `~/.config/dsp-cli/update_check.toml`.
    ///
    /// Returns an error if the home directory cannot be resolved. Note this
    /// is the one failure `default_path` itself can produce; callers of
    /// [`Self::load`] still degrade it to `Self::default()` rather than
    /// propagating it, per this cache's disposable-on-failure contract.
    pub fn default_path() -> Result<PathBuf, Diagnostic> {
        // Same home-dir resolution as `AuthCache::default_path` — do not
        // substitute `dirs::config_dir()`, which returns a different
        // platform-specific path on macOS.
        let home =
            dirs::home_dir().ok_or_else(|| Diagnostic::Internal("could not resolve home directory".to_string()))?;
        Ok(home.join(".config").join("dsp-cli").join("update_check.toml"))
    }

    /// Load the cache from [`Self::default_path`].
    ///
    /// Every failure — including an unresolvable home directory — degrades
    /// to `Self::default()` (logged at `tracing::debug!`). This cache is
    /// disposable; no failure here is ever surfaced as an error.
    pub fn load() -> Self {
        match Self::default_path() {
            Ok(path) => Self::load_from(&path),
            Err(e) => {
                tracing::debug!(error = %e, "could not resolve update check cache path; using default");
                Self::default()
            }
        }
    }

    /// Load the cache from an explicit path.
    ///
    /// Unlike [`crate::config::auth_cache::AuthCache::load_from`], every
    /// failure path here — missing file, oversize file, unreadable file,
    /// malformed TOML — degrades to `Self::default()` rather than an `Err`.
    pub fn load_from(path: &Path) -> Self {
        let metadata = match fs::metadata(path) {
            Ok(md) => md,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "update check cache not found; using default");
                return Self::default();
            }
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "failed to stat update check cache; using default");
                return Self::default();
            }
        };

        if metadata.len() > MAX_CACHE_FILE_BYTES {
            tracing::debug!(
                path = %path.display(),
                size = metadata.len(),
                max = MAX_CACHE_FILE_BYTES,
                "update check cache too large; using default"
            );
            return Self::default();
        }

        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "failed to read update check cache; using default");
                return Self::default();
            }
        };

        match toml::from_str(&contents) {
            Ok(cache) => {
                tracing::debug!(path = %path.display(), "loaded update check cache");
                cache
            }
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "failed to parse update check cache; using default");
                Self::default()
            }
        }
    }

    /// Save the cache to [`Self::default_path`].
    ///
    /// Unlike `load`, a save failure is real and is returned as `Err` — the
    /// caller (a later plan step) decides whether to swallow it.
    pub fn save(&self) -> Result<(), Diagnostic> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save the cache to an explicit path.
    ///
    /// Creates the parent directory if it does not exist. Writes atomically
    /// via a `<filename>.<pid>` sibling file followed by a `rename`. No
    /// `0600` permission tightening — this file holds no secret.
    pub fn save_to(&self, path: &Path) -> Result<(), Diagnostic> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::Internal(format!(
                    "failed to create update check cache directory at {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let contents = toml::to_string_pretty(self).map_err(|e| {
            Diagnostic::Internal(format!("failed to serialise update check cache for {}: {}", path.display(), e))
        })?;

        write_atomically(path, &contents)?;
        tracing::debug!(path = %path.display(), "saved update check cache");
        Ok(())
    }
}

// The atomic-write helpers below are a deliberate copy of the pattern in
// `src/config/auth_cache.rs` (`write_atomically`/`temp_sibling_path`), not a
// shared util: those are private to that module and carry `0600`
// secret-cache semantics this cache does not need. Two cache files don't yet
// justify extraction (rule of three) — extract a shared helper if a third
// on-disk cache appears.

/// Write `contents` to `path` atomically via a `<filename>.<pid>` temp file
/// in the same directory, then `rename`. No permission tightening — this
/// file holds no secret, unlike `auth.toml`.
fn write_atomically(path: &Path, contents: &str) -> Result<(), Diagnostic> {
    let tmp_path = temp_sibling_path(path)?;

    fs::write(&tmp_path, contents).map_err(|e| {
        Diagnostic::Internal(format!(
            "failed to write update check cache temp file at {}: {}",
            tmp_path.display(),
            e
        ))
    })?;

    if let Err(e) = fs::rename(&tmp_path, path) {
        // Rename failed but the temp file is still on disk. Best-effort
        // cleanup so the directory does not accrete
        // `update_check.toml.<pid>` residue across failed writes; the
        // cleanup result is intentionally discarded — the rename error is
        // what we want to surface.
        let _ = fs::remove_file(&tmp_path);
        return Err(Diagnostic::Internal(format!(
            "failed to rename update check cache temp file to {}: {}",
            path.display(),
            e
        )));
    }
    Ok(())
}

/// `<path>` → `<dirname>/<filename>.<pid>`. Derived from the actual filename
/// (not via `Path::with_extension`, which would replace the existing
/// extension and silently misbehave if `path` ever ended in something other
/// than `.toml`).
fn temp_sibling_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    let mut name = path
        .file_name()
        .ok_or_else(|| {
            Diagnostic::Internal(format!("update check cache path has no filename component: {}", path.display()))
        })?
        .to_os_string();
    name.push(format!(".{}", std::process::id()));
    Ok(path.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn round_trip_sets_both_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update_check.toml");

        let checked = Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap();
        let cache = UpdateCheckCache {
            last_checked: Some(checked),
            latest_seen: Some("0.1.5".to_string()),
        };
        cache.save_to(&path).unwrap();

        let loaded = UpdateCheckCache::load_from(&path);
        assert_eq!(loaded.last_checked, Some(checked));
        assert_eq!(loaded.latest_seen, Some("0.1.5".to_string()));
    }

    #[test]
    fn load_from_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update_check.toml");

        let cache = UpdateCheckCache::load_from(&path);
        assert_eq!(cache.last_checked, None);
        assert_eq!(cache.latest_seen, None);
    }

    #[test]
    fn load_from_malformed_toml_returns_default_not_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update_check.toml");

        fs::write(&path, b"not valid toml [[[").unwrap();

        // No `.unwrap_err()` here on purpose: `load_from` returns `Self`
        // directly, never `Result` — a malformed file must never surface as
        // an error for this disposable cache.
        let cache = UpdateCheckCache::load_from(&path);
        assert_eq!(cache.last_checked, None);
        assert_eq!(cache.latest_seen, None);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("update_check.toml");

        let cache = UpdateCheckCache { last_checked: None, latest_seen: Some("0.1.3".to_string()) };
        cache.save_to(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn atomic_write_does_not_leave_temp_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update_check.toml");

        let cache = UpdateCheckCache { last_checked: None, latest_seen: Some("0.1.3".to_string()) };
        cache.save_to(&path).unwrap();

        let tmp_path = temp_sibling_path(&path).unwrap();
        assert!(
            !tmp_path.exists(),
            "temp file should not exist after save: {}",
            tmp_path.display()
        );
    }

    #[test]
    fn load_from_oversize_file_returns_default_not_error() {
        // Behavioural contrast with `AuthCache::load_from`, which returns
        // `Err(Diagnostic::Internal(_))` on an oversize file: this cache
        // degrades to `Self::default()` instead, since a corrupt/huge
        // update-check cache must never break an unrelated command.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update_check.toml");
        let oversize = vec![b'x'; (MAX_CACHE_FILE_BYTES + 1) as usize];
        fs::write(&path, &oversize).unwrap();

        let cache = UpdateCheckCache::load_from(&path);
        assert_eq!(cache.last_checked, None);
        assert_eq!(cache.latest_seen, None);
    }
}
