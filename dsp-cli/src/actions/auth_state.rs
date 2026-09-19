//! Shared dsp-cli/ADR-0007 auth-state disclosure helper, used by all commands that
//! produce `_meta.auth`.

use crate::config::{AuthCache, ResolvedToken, TokenOrigin};

/// Build the dsp-cli/ADR-0007 read-command auth-state disclosure string from token
/// resolution. Uses dsp-cli/ADR-0007 disclosure vocabulary ("anonymous"/"authenticated"),
/// which is the uniform `_meta.auth` contract across all commands.
///
/// Cases:
///   no token             → "anonymous"
///   env token            → "authenticated via DSP_TOKEN"
///   cache token + user   → "authenticated as {user}"
///   cache token, no user → "authenticated"
///
/// **Expiry note (v1):** reflects token *presence/origin*, not validity — does
/// not check `expires_at`. An expired cached token is sent and reported as
/// "authenticated as {user}". This is a deliberate v1 simplification (the server
/// ignores an expired token on a public endpoint anyway).
///
/// **Username source:** `ResolvedToken` carries only `token + origin`. The `{user}`
/// for the cache arm is read from `cache.user(server)` — exactly as `auth status`
/// does it. The `cache` param is needed solely for this username lookup.
pub(crate) fn read_auth_state(resolved: Option<&ResolvedToken>, cache: &AuthCache, server: &str) -> String {
    match resolved {
        None => "anonymous".to_string(),
        Some(r) if r.origin == TokenOrigin::Env => "authenticated via DSP_TOKEN".to_string(),
        Some(_) => {
            // Cache origin — username comes from the cache entry, not the resolved token.
            match cache.user(server) {
                Some(user) => format!("authenticated as {user}"),
                None => "authenticated".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthCache;
    use crate::config::token::{ResolvedToken, TokenOrigin};

    const SERVER: &str = "https://api.test.dasch.swiss";

    fn cache_with_user(server: &str, token: &str, user: &str) -> AuthCache {
        let mut cache = AuthCache::default();
        cache.set_entry(
            server,
            crate::config::auth_cache::ServerEntry {
                token: token.to_string(),
                user: Some(user.to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache
    }

    fn cache_with_token_no_user(server: &str, token: &str) -> AuthCache {
        let mut cache = AuthCache::default();
        cache.set_token(server.to_string(), token.to_string());
        cache
    }

    fn env_token() -> ResolvedToken {
        ResolvedToken { token: "env-tok".to_string(), origin: TokenOrigin::Env }
    }

    fn cache_token() -> ResolvedToken {
        ResolvedToken { token: "cache-tok".to_string(), origin: TokenOrigin::Cache }
    }

    /// Case 1: no token → "anonymous"
    #[test]
    fn no_token_is_anonymous() {
        let cache = AuthCache::default();
        let state = read_auth_state(None, &cache, SERVER);
        assert_eq!(state, "anonymous");
    }

    /// Case 2: env token → "authenticated via DSP_TOKEN"
    #[test]
    fn env_token_disclosure() {
        let cache = AuthCache::default();
        let tok = env_token();
        let state = read_auth_state(Some(&tok), &cache, SERVER);
        assert_eq!(state, "authenticated via DSP_TOKEN");
    }

    /// Case 3: cache token + user → "authenticated as {user}"
    #[test]
    fn cache_token_with_user_shows_username() {
        let cache = cache_with_user(SERVER, "cache-tok", "alice@example.com");
        let tok = cache_token();
        let state = read_auth_state(Some(&tok), &cache, SERVER);
        assert_eq!(state, "authenticated as alice@example.com");
    }

    /// Case 4: cache token, no user → "authenticated"
    #[test]
    fn cache_token_without_user_is_authenticated() {
        let cache = cache_with_token_no_user(SERVER, "cache-tok");
        let tok = cache_token();
        let state = read_auth_state(Some(&tok), &cache, SERVER);
        assert_eq!(state, "authenticated");
    }
}
