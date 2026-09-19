//! Token resolution — resolves the effective bearer token from env or cache.
//!
//! See ADR-0007 for the full resolution order (flag → env → `.env` → cache).
//! In v1 there is no `--token` flag, so the effective order is **env → cache**.

use std::fmt;

use super::AuthCache;

/// Where the effective bearer token came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOrigin {
    /// Supplied via the `DSP_TOKEN` environment variable.
    Env,
    /// Read from the on-disk auth cache (`auth.toml`).
    Cache,
}

/// The bearer token a command should use for a server, plus its origin.
///
/// Holds a secret — `Debug` is manual and redacts the token value (mirrors
/// the `ServerEntry` pattern in `auth_cache.rs`).
pub struct ResolvedToken {
    /// The raw bearer token string.
    ///
    /// Do NOT log or display this field. Use it only to build the
    /// `Authorization` header or to read the `exp` claim for display.
    /// `pub(crate)` enforces that contract structurally — the secret cannot be
    /// read by any out-of-crate consumer, only by in-crate callers (the status
    /// action, the `dsp auth token` action — which prints it verbatim to
    /// stdout, its one sanctioned purpose — and the bearer-header plumbing for
    /// data commands such as `dump` and `list`).
    pub(crate) token: String,
    /// Where the token came from.
    pub origin: TokenOrigin,
}

impl fmt::Debug for ResolvedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedToken")
            .field("token", &"[REDACTED]")
            .field("origin", &self.origin)
            .finish()
    }
}

/// Resolve the effective bearer token for `server`.
///
/// Precedence per ADR-0007: a non-blank `DSP_TOKEN` env token wins over any
/// cached token; a blank or unset `DSP_TOKEN` falls through to the cache.
/// Returns `None` when neither source has a token.
///
/// The env token is server-agnostic and wins regardless of the cache contents
/// for any server. Surrounding whitespace in the env value is trimmed: a JWT
/// never contains whitespace, so `DSP_TOKEN="   "` is treated as absent.
pub fn resolve_token(
    env_token: Option<String>,
    cache: &AuthCache,
    server: &str,
) -> Option<ResolvedToken> {
    let t = env_token.as_deref().map(str::trim).unwrap_or("");
    if !t.is_empty() {
        return Some(ResolvedToken {
            token: t.to_owned(),
            origin: TokenOrigin::Env,
        });
    }
    if let Some(c) = cache.token(server) {
        return Some(ResolvedToken {
            token: c.to_owned(),
            origin: TokenOrigin::Cache,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "https://api.dasch.swiss";
    const OTHER_SERVER: &str = "https://api.stage.dasch.swiss";
    const ENV_TOKEN: &str = "env-token-abc";
    const CACHE_TOKEN: &str = "cache-token-xyz";

    fn cache_with_token(server: &str, token: &str) -> AuthCache {
        let mut cache = AuthCache::default();
        cache.set_token(server.to_string(), token.to_string());
        cache
    }

    /// 1. env non-empty wins over a populated cache for the same server.
    #[test]
    fn env_nonempty_wins_over_cache() {
        let cache = cache_with_token(SERVER, CACHE_TOKEN);
        let result = resolve_token(Some(ENV_TOKEN.to_string()), &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Env);
        assert_eq!(rt.token, ENV_TOKEN);
    }

    /// 2a. env whitespace-only falls through; cache present → origin Cache.
    #[test]
    fn env_whitespace_only_falls_through_to_cache() {
        let cache = cache_with_token(SERVER, CACHE_TOKEN);
        let result = resolve_token(Some("   ".to_string()), &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Cache);
        assert_eq!(rt.token, CACHE_TOKEN);
    }

    /// 2b. env whitespace-only with no cache → None.
    #[test]
    fn env_whitespace_only_no_cache_returns_none() {
        let cache = AuthCache::default();
        let result = resolve_token(Some("   ".to_string()), &cache, SERVER);
        assert!(result.is_none());
    }

    /// 3a. env empty string ("") falls through; cache present → origin Cache.
    #[test]
    fn env_empty_string_falls_through_to_cache() {
        let cache = cache_with_token(SERVER, CACHE_TOKEN);
        let result = resolve_token(Some(String::new()), &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Cache);
        assert_eq!(rt.token, CACHE_TOKEN);
    }

    /// 3b. env empty string with no cache → None.
    #[test]
    fn env_empty_string_no_cache_returns_none() {
        let cache = AuthCache::default();
        let result = resolve_token(Some(String::new()), &cache, SERVER);
        assert!(result.is_none());
    }

    /// 4. env None + cache has token → origin Cache.
    #[test]
    fn env_none_uses_cache() {
        let cache = cache_with_token(SERVER, CACHE_TOKEN);
        let result = resolve_token(None, &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Cache);
        assert_eq!(rt.token, CACHE_TOKEN);
    }

    /// 5. env set + cache has a token for a DIFFERENT server → still origin Env (server-agnostic).
    #[test]
    fn env_set_cache_for_different_server_still_env() {
        let cache = cache_with_token(OTHER_SERVER, CACHE_TOKEN);
        let result = resolve_token(Some(ENV_TOKEN.to_string()), &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Env);
        assert_eq!(rt.token, ENV_TOKEN);
    }

    /// 6. env set + empty cache → origin Env.
    #[test]
    fn env_set_empty_cache_returns_env() {
        let cache = AuthCache::default();
        let result = resolve_token(Some(ENV_TOKEN.to_string()), &cache, SERVER);
        let rt = result.unwrap();
        assert_eq!(rt.origin, TokenOrigin::Env);
        assert_eq!(rt.token, ENV_TOKEN);
    }

    /// 7. both absent (env None + empty cache) → None.
    #[test]
    fn both_absent_returns_none() {
        let cache = AuthCache::default();
        let result = resolve_token(None, &cache, SERVER);
        assert!(result.is_none());
    }

    /// 8. Debug-redaction: the token value must not appear in the Debug output.
    #[test]
    fn debug_redacts_token() {
        let secret = "super-secret-bearer-token";
        let rt = ResolvedToken {
            token: secret.to_string(),
            origin: TokenOrigin::Env,
        };
        let rendered = format!("{rt:?}");
        assert!(
            !rendered.contains(secret),
            "Debug impl leaked the token: {rendered}"
        );
        assert!(
            rendered.contains("REDACTED"),
            "expected redaction marker in Debug output, got: {rendered}"
        );
    }
}
