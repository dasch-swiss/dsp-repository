//! On-disk token cache at `~/.config/dsp-cli/auth.toml`. See ADR-0007 and ADR-0012.
//!
//! Tokens are keyed by server URL. The file is written atomically via a
//! `<filename>.<pid>` sibling file followed by a `rename`, so the original
//! file is intact if a crash interrupts mid-write. Concurrent writes use a
//! last-writer-wins policy: whichever `rename` runs last wins. The `<pid>`
//! suffix prevents temp-file collisions between concurrent invocations.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;

/// Hard cap on `auth.toml` size. A real cache holds tens of bytes per server
/// entry; anything past 1 MiB is almost certainly a misconfigured symlink and
/// must not be slurped into memory before the TOML parse can reject it.
const MAX_CACHE_FILE_BYTES: u64 = 1 << 20;

/// One entry in the cache — token plus optional metadata.
///
/// The `Debug` impl below is manual to redact the token. The struct holds a
/// secret; `#[derive(Debug)]` would expose it via any future `tracing::debug!`
/// or panic message that captured a `ServerEntry`. The `user` and timestamp
/// fields are not secrets and are shown in cleartext.
///
/// All new fields (`user`, `acquired_at`, `expires_at`) are `Option<...>` so
/// legacy `auth.toml` files from before v2 (token-only entries) still parse.
#[derive(Deserialize, Serialize)]
pub struct ServerEntry {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquired_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl fmt::Debug for ServerEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerEntry")
            .field("token", &"[REDACTED]")
            .field("user", &self.user)
            .field("acquired_at", &self.acquired_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// In-memory view of `~/.config/dsp-cli/auth.toml`.
///
/// Load with [`AuthCache::load`] (or [`AuthCache::load_from`] in tests).
/// Mutate with [`AuthCache::set_token`] and [`AuthCache::remove`].
/// Persist with [`AuthCache::save`] (or [`AuthCache::save_to`] in tests).
///
/// Internal layout is a `BTreeMap` (not `HashMap`) so the on-disk TOML has
/// deterministic key order. The `toml` crate serialises a `BTreeMap<String, T>`
/// at the root as a sequence of standalone `[key]` tables, which is the shape
/// ADR-0007 specifies — no wrapper struct or `#[serde(flatten)]` needed.
#[derive(Debug, Default)]
pub struct AuthCache {
    entries: BTreeMap<String, ServerEntry>,
}

impl AuthCache {
    /// The canonical cache path: `~/.config/dsp-cli/auth.toml`.
    ///
    /// Returns an error if the home directory cannot be resolved.
    pub fn default_path() -> Result<PathBuf, Diagnostic> {
        // ADR-0007 specifies the literal `~/.config/dsp-cli/auth.toml`. Do not
        // substitute `dirs::config_dir()` — that returns `~/Library/Application
        // Support/dsp-cli` on macOS, which contradicts the ADR.
        let home = dirs::home_dir()
            .ok_or_else(|| Diagnostic::Internal("could not resolve home directory".to_string()))?;
        Ok(home.join(".config").join("dsp-cli").join("auth.toml"))
    }

    /// Load the cache from [`Self::default_path`].
    ///
    /// A missing file is not an error — it means no tokens are cached yet.
    pub fn load() -> Result<Self, Diagnostic> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load the cache from an explicit path.
    ///
    /// A missing file is not an error — returns an empty cache. Files whose
    /// reported size exceeds `MAX_CACHE_FILE_BYTES` are rejected before the
    /// contents are read, so a symlink pointing at a huge file fails fast.
    pub fn load_from(path: &Path) -> Result<Self, Diagnostic> {
        let metadata = match fs::metadata(path) {
            Ok(md) => md,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "auth cache not found; starting empty");
                return Ok(Self::default());
            }
            Err(e) => {
                return Err(Diagnostic::Internal(format!(
                    "failed to stat auth cache at {}: {}",
                    path.display(),
                    e
                )));
            }
        };

        if metadata.len() > MAX_CACHE_FILE_BYTES {
            return Err(Diagnostic::Internal(format!(
                "auth cache at {} is too large ({} bytes, max {} bytes); refusing to read",
                path.display(),
                metadata.len(),
                MAX_CACHE_FILE_BYTES
            )));
        }

        let contents = fs::read_to_string(path).map_err(|e| {
            Diagnostic::Internal(format!(
                "failed to read auth cache at {}: {}",
                path.display(),
                e
            ))
        })?;
        let entries: BTreeMap<String, ServerEntry> = toml::from_str(&contents).map_err(|e| {
            Diagnostic::Internal(format!(
                "failed to parse auth cache at {}: {}",
                path.display(),
                e
            ))
        })?;
        tracing::debug!(path = %path.display(), "loaded auth cache");
        Ok(Self { entries })
    }

    /// Save the cache to [`Self::default_path`].
    ///
    /// Creates the parent directory if it does not exist. On Unix, the file is
    /// created with mode `0600`.
    pub fn save(&self) -> Result<(), Diagnostic> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save the cache to an explicit path.
    ///
    /// Creates the parent directory if it does not exist. On Unix, the file is
    /// created with mode `0600`.
    pub fn save_to(&self, path: &Path) -> Result<(), Diagnostic> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::Internal(format!(
                    "failed to create auth cache directory at {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let contents = toml::to_string_pretty(&self.entries).map_err(|e| {
            Diagnostic::Internal(format!(
                "failed to serialise auth cache for {}: {}",
                path.display(),
                e
            ))
        })?;

        write_atomically(path, &contents)?;
        tracing::debug!(path = %path.display(), "saved auth cache");
        Ok(())
    }

    /// Return the cached token for `server`, if any.
    pub fn token(&self, server: &str) -> Option<&str> {
        self.entries.get(server).map(|e| e.token.as_str())
    }

    /// Return the cached user for `server`, if any.
    pub fn user(&self, server: &str) -> Option<&str> {
        self.entries.get(server).and_then(|e| e.user.as_deref())
    }

    /// Return the `acquired_at` timestamp for `server`, if any.
    pub fn acquired_at(&self, server: &str) -> Option<DateTime<Utc>> {
        self.entries.get(server).and_then(|e| e.acquired_at)
    }

    /// Return the `expires_at` timestamp for `server`, if any.
    pub fn expires_at(&self, server: &str) -> Option<DateTime<Utc>> {
        self.entries.get(server).and_then(|e| e.expires_at)
    }

    /// Insert or replace a fully-populated entry for `server`.
    ///
    /// Prefer this over `set_token` when the full entry shape is available
    /// (e.g. after a login that returns user + expiry). `set_token` remains
    /// as a thin convenience for callers that only have a token.
    pub fn set_entry(&mut self, server: impl Into<String>, entry: ServerEntry) {
        self.entries.insert(server.into(), entry);
    }

    /// Insert or replace the token for `server`.
    ///
    /// Convenience wrapper that constructs a partial `ServerEntry` with only
    /// the token set, leaving `user`, `acquired_at`, and `expires_at` as
    /// `None`. Use `set_entry` when the full shape is available.
    pub fn set_token(&mut self, server: String, token: String) {
        self.set_entry(
            server,
            ServerEntry {
                token,
                user: None,
                acquired_at: None,
                expires_at: None,
            },
        );
    }

    /// Remove the token for `server`.
    ///
    /// Returns `true` if an entry was present and removed, `false` otherwise.
    pub fn remove(&mut self, server: &str) -> bool {
        self.entries.remove(server).is_some()
    }

    /// Return `true` if no tokens are cached.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Write `contents` to `path` atomically via a `<filename>.<pid>` temp file
/// in the same directory, then `rename`.
///
/// On Unix the temp file is created with mode `0600` at creation time,
/// eliminating the brief window where the file could be readable under the
/// caller's umask. On non-Unix the file is written without permission
/// tightening (Windows support is a known limitation — see ADR-0007).
fn write_atomically(path: &Path, contents: &str) -> Result<(), Diagnostic> {
    let tmp_path = temp_sibling_path(path)?;

    write_temp_file(&tmp_path, contents).map_err(|e| {
        Diagnostic::Internal(format!(
            "failed to write auth cache temp file at {}: {}",
            tmp_path.display(),
            e
        ))
    })?;

    if let Err(e) = fs::rename(&tmp_path, path) {
        // Rename failed but the temp file (mode 0600) is still on disk. Make a
        // best-effort attempt to remove it so the directory does not accrete
        // `auth.toml.<pid>` residue across failed writes. The cleanup result
        // is intentionally discarded — the original rename error is what we
        // want to surface to the caller.
        let _ = fs::remove_file(&tmp_path);
        return Err(Diagnostic::Internal(format!(
            "failed to rename auth cache temp file to {}: {}",
            path.display(),
            e
        )));
    }
    Ok(())
}

/// `<path>` → `<dirname>/<filename>.<pid>`. Derived from the actual filename
/// (not via `Path::with_extension`, which would replace the existing extension
/// and silently misbehave if `path` ever ended in something other than `.toml`).
fn temp_sibling_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    let mut name = path
        .file_name()
        .ok_or_else(|| {
            Diagnostic::Internal(format!(
                "auth cache path has no filename component: {}",
                path.display()
            ))
        })?
        .to_os_string();
    name.push(format!(".{}", std::process::id()));
    Ok(path.with_file_name(name))
}

/// Platform-specific temp file write. On Unix, opens with mode `0600`
/// at creation; on other platforms, uses a plain write.
#[cfg(unix)]
fn write_temp_file(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())
}

#[cfg(not(unix))]
fn write_temp_file(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_from_missing_file_returns_empty_cache() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");
        let cache = AuthCache::load_from(&path).unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn set_then_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token(
            "https://api.dasch.swiss".to_string(),
            "tok-abc123".to_string(),
        );
        cache.save_to(&path).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(loaded.token("https://api.dasch.swiss"), Some("tok-abc123"));
    }

    #[test]
    fn multiple_servers_coexist() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token(
            "https://api.dasch.swiss".to_string(),
            "tok-prod".to_string(),
        );
        cache.set_token(
            "https://api.test.dasch.swiss".to_string(),
            "tok-test".to_string(),
        );
        cache.save_to(&path).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(loaded.token("https://api.dasch.swiss"), Some("tok-prod"));
        assert_eq!(
            loaded.token("https://api.test.dasch.swiss"),
            Some("tok-test")
        );
    }

    #[test]
    fn set_overwrites_existing_token() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token(
            "https://api.dasch.swiss".to_string(),
            "old-token".to_string(),
        );
        cache.save_to(&path).unwrap();

        let mut cache2 = AuthCache::load_from(&path).unwrap();
        cache2.set_token(
            "https://api.dasch.swiss".to_string(),
            "new-token".to_string(),
        );
        cache2.save_to(&path).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(loaded.token("https://api.dasch.swiss"), Some("new-token"));
    }

    #[test]
    fn remove_clears_entry() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token(
            "https://api.dasch.swiss".to_string(),
            "tok-prod".to_string(),
        );
        cache.save_to(&path).unwrap();

        let mut cache2 = AuthCache::load_from(&path).unwrap();
        // remove returns true when the key is present
        assert!(cache2.remove("https://api.dasch.swiss"));
        // remove returns false when the key is absent
        assert!(!cache2.remove("https://api.dasch.swiss"));
        cache2.save_to(&path).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(loaded.token("https://api.dasch.swiss"), None);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token("https://api.dasch.swiss".to_string(), "tok".to_string());
        cache.save_to(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn save_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token("https://api.dasch.swiss".to_string(), "tok".to_string());
        cache.save_to(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected 0600, got {mode:o}");
    }

    #[test]
    fn malformed_toml_returns_internal_diagnostic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        fs::write(&path, b"not valid toml [[[").unwrap();

        let err = AuthCache::load_from(&path).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Internal(_)),
            "expected Diagnostic::Internal, got {:?}",
            err
        );
        let msg = err.to_string();
        assert!(
            msg.contains(&path.to_string_lossy().to_string()),
            "error message should contain the path; got: {msg}"
        );
    }

    #[test]
    fn atomic_write_does_not_leave_temp_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token("https://api.dasch.swiss".to_string(), "tok".to_string());
        cache.save_to(&path).unwrap();

        let tmp_path = temp_sibling_path(&path).unwrap();
        assert!(
            !tmp_path.exists(),
            "temp file should not exist after save: {}",
            tmp_path.display()
        );
    }

    #[test]
    fn on_disk_shape_uses_standalone_tables() {
        // Pins the contract from ADR-0007: each server URL is a top-level
        // standalone table, not an inline table. This catches the
        // `#[serde(flatten)]` / `BTreeMap` interaction risk the plan flagged.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let mut cache = AuthCache::load_from(&path).unwrap();
        cache.set_token("https://api.dasch.swiss".to_string(), "tok-abc".to_string());
        cache.save_to(&path).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("[\"https://api.dasch.swiss\"]"),
            "expected standalone table header, got:\n{raw}"
        );
        assert!(
            raw.contains("token = \"tok-abc\""),
            "expected token on its own line, got:\n{raw}"
        );
        assert!(
            !raw.contains("= {"),
            "did not expect inline-table shape, got:\n{raw}"
        );
    }

    #[test]
    fn server_entry_debug_redacts_token() {
        // The cache holds secrets; the Debug impl must not leak the token if
        // something accidentally formats a ServerEntry (e.g. tracing::debug!).
        let entry = ServerEntry {
            token: "super-secret-jwt".to_string(),
            user: None,
            acquired_at: None,
            expires_at: None,
        };
        let rendered = format!("{entry:?}");
        assert!(
            !rendered.contains("super-secret-jwt"),
            "Debug impl leaked the token: {rendered}"
        );
        assert!(
            rendered.contains("REDACTED"),
            "expected redaction marker, got: {rendered}"
        );
    }

    #[test]
    fn server_entry_debug_redacts_token_when_all_fields_populated() {
        // Regression guard: adding user/timestamp fields to ServerEntry must not
        // accidentally cause the Debug impl to reveal the token via a derived impl.
        use chrono::TimeZone;
        let entry = ServerEntry {
            token: "super-secret-jwt-full".to_string(),
            user: Some("user@example.com".to_string()),
            acquired_at: Some(Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap()),
            expires_at: Some(Utc.with_ymd_and_hms(2026, 6, 25, 12, 34, 56).unwrap()),
        };
        let rendered = format!("{entry:?}");
        assert!(
            !rendered.contains("super-secret-jwt-full"),
            "Debug impl leaked the token when all fields are set: {rendered}"
        );
        assert!(
            rendered.contains("REDACTED"),
            "expected redaction marker, got: {rendered}"
        );
        // User and timestamps should appear in cleartext.
        assert!(
            rendered.contains("user@example.com"),
            "expected user in debug output, got: {rendered}"
        );
    }

    #[test]
    fn round_trip_entry_with_all_fields() {
        // Verify serialise → deserialise round-trip for a fully-populated ServerEntry.
        use chrono::TimeZone;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        let expires = Utc.with_ymd_and_hms(2026, 6, 25, 12, 34, 56).unwrap();
        let acquired = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "tok-full".to_string(),
                user: Some("user@example.com".to_string()),
                acquired_at: Some(acquired),
                expires_at: Some(expires),
            },
        );
        cache.save_to(&path).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(
            loaded.token("https://api.test.dasch.swiss"),
            Some("tok-full")
        );
        assert_eq!(
            loaded.user("https://api.test.dasch.swiss"),
            Some("user@example.com")
        );
        assert_eq!(
            loaded.acquired_at("https://api.test.dasch.swiss"),
            Some(acquired)
        );
        assert_eq!(
            loaded.expires_at("https://api.test.dasch.swiss"),
            Some(expires)
        );
    }

    #[test]
    fn load_from_rejects_oversize_file() {
        // Regression guard for the symlink-to-huge-file slurp surfaced during the
        // 004 security review: stat the file first; reject before reading if it's
        // past the cap. Writes just over the cap (~1 MiB + 1 B).
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");
        let oversize = vec![b'x'; (MAX_CACHE_FILE_BYTES + 1) as usize];
        fs::write(&path, &oversize).unwrap();

        let err = AuthCache::load_from(&path).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Internal(_)),
            "expected Diagnostic::Internal, got {:?}",
            err
        );
        let msg = err.to_string();
        assert!(
            msg.contains("too large"),
            "expected 'too large' in error message; got: {msg}"
        );
    }

    #[test]
    fn write_atomically_cleans_temp_file_on_rename_failure() {
        // Force `rename` to fail by making the target path a directory: POSIX
        // rename of a regular file onto a directory is invalid. The cleanup
        // branch must remove the temp sibling so directories don't accrete
        // `auth.toml.<pid>` residue across failed writes.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");
        fs::create_dir(&path).unwrap();

        let err = write_atomically(&path, "irrelevant").unwrap_err();
        assert!(
            matches!(err, Diagnostic::Internal(_)),
            "expected Diagnostic::Internal on rename-onto-directory; got {:?}",
            err
        );

        let tmp = temp_sibling_path(&path).unwrap();
        assert!(
            !tmp.exists(),
            "temp file should be cleaned up after rename failure: {}",
            tmp.display()
        );
    }

    #[test]
    fn round_trip_legacy_entry_token_only() {
        // Backward compat: an auth.toml written before the schema bump (token-only)
        // must still parse, with the new Option fields deserialising as None.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.toml");

        // Write a legacy-style TOML (only `token` field, no user/acquired_at/expires_at).
        let legacy_toml = "[\"https://api.dasch.swiss\"]\ntoken = \"legacy-tok\"\n";
        fs::write(&path, legacy_toml).unwrap();

        let loaded = AuthCache::load_from(&path).unwrap();
        assert_eq!(loaded.token("https://api.dasch.swiss"), Some("legacy-tok"));
        assert_eq!(loaded.user("https://api.dasch.swiss"), None);
        assert_eq!(loaded.acquired_at("https://api.dasch.swiss"), None);
        assert_eq!(loaded.expires_at("https://api.dasch.swiss"), None);
    }
}
