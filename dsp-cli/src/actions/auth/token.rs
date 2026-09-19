//! Action for `dsp auth token`.
//!
//! Prints the resolved bearer token **verbatim to stdout**, bare — no
//! envelope, no `--format`, no `Renderer`. This is the **read-out**
//! counterpart to `dsp auth set-token` (which reads a JWT from stdin into the
//! cache): today `set-token` writes a token in, and this command is the only
//! way to read one back out for use in shell pipelines (`export
//! DSP_TOKEN=$(dsp auth token -s dev)`).
//!
//! No server round-trip is made — this is a pure cache/env inspection, like
//! `dsp auth status` (see `status.rs`), from which the resolution and expiry
//! logic below is lifted. Before printing, a **local, best-effort** expiry
//! check is done: if the resolved token is locally detected as expired, or no
//! token is cached for the server, the command returns
//! [`Diagnostic::AuthRequired`] (exit `3`, ADR-0012) instead of printing
//! anything. This check is advisory only — the JWT signature is not verified
//! (see `crate::client::jwt`) — so exit `0` means only "the token looks
//! unexpired locally," not "the server will accept it." See ADR-0007 for the
//! token-resolution/precedence model this command reuses unchanged.

use std::io::Write;
use std::path::Path;

use chrono::Utc;

use crate::config::{AuthCache, Config, TokenOrigin, resolve_token};
use crate::diagnostic::Diagnostic;

/// Print the resolved bearer token for `cfg.server` to stdout.
///
/// Reads `DSP_TOKEN` from the environment, resolves the effective token via
/// [`resolve_token`] (env wins over cache), and — unless the token is
/// locally detected as expired or absent — writes it followed by a single
/// `\n` to stdout. Returns [`Diagnostic::AuthRequired`] when no token is
/// cached for the server or the resolved token is expired; never logs,
/// formats, or otherwise displays the token value except via the final
/// `writeln!`.
pub fn run(cfg: &Config) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    let mut out = std::io::stdout().lock();
    run_impl(cfg, &mut out, None, env_token)
}

fn run_impl(
    cfg: &Config,
    out: &mut dyn Write,
    cache_path: Option<&Path>,
    env_token: Option<String>,
) -> Result<(), Diagnostic> {
    // ADR-0007 says a non-blank `DSP_TOKEN` wins regardless of cache state.
    // A corrupt or unreadable `auth.toml` therefore must not mask the env
    // token: treat a cache-load failure as an empty cache when the env token
    // would resolve. (Matches the trim-and-empty rule in `resolve_token`, and
    // the identical fallthrough in `status.rs`.)
    let env_token_would_win = env_token
        .as_deref()
        .map(str::trim)
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) if env_token_would_win => {
            tracing::warn!(
                error = %e,
                "auth cache load failed; DSP_TOKEN is set, falling through to env token"
            );
            AuthCache::default()
        }
        Err(e) => return Err(e),
    };

    match resolve_token(env_token, &cache, &cfg.server) {
        None => Err(Diagnostic::AuthRequired(format!(
            "no token for {}; run `dsp auth login`, pipe one to `dsp auth set-token`, or set DSP_TOKEN",
            cfg.server
        ))),
        Some(resolved) => {
            // Exhaustive match, no wildcard: a future TokenOrigin variant must
            // fail to compile here rather than silently folding into the
            // cache case.
            let expires_at = match resolved.origin {
                TokenOrigin::Env => crate::client::jwt::extract_exp(&resolved.token),
                TokenOrigin::Cache => cache.expires_at(&cfg.server),
            };
            let expired = expires_at.map(|t| t < Utc::now()).unwrap_or(false);
            if expired {
                // The remedy differs by origin. A *cached* token is refreshed by
                // `login`. But a `DSP_TOKEN` unconditionally overrides the cache
                // (ADR-0007), so `login` would NOT fix an expired env token — the
                // freshly-cached token stays shadowed and the command keeps
                // exiting 3; the caller must refresh or unset `DSP_TOKEN` instead.
                let msg = match resolved.origin {
                    TokenOrigin::Env => format!(
                        "the DSP_TOKEN for {} has expired; export a fresh token or unset DSP_TOKEN",
                        cfg.server
                    ),
                    TokenOrigin::Cache => format!(
                        "the cached token for {} has expired; run `dsp auth login` to refresh",
                        cfg.server
                    ),
                };
                return Err(Diagnostic::AuthRequired(msg));
            }
            // Bare `?`: a stdout write failure (e.g. `dsp auth token | head`
            // closing the pipe early) routes through the blanket
            // `From<std::io::Error> for Diagnostic` to `Internal` (exit 1),
            // exactly like `docs.rs`. Never hand-roll `Diagnostic::Io` here —
            // that variant is reserved for explicit user-requested file
            // writes, not injected-writer plumbing I/O.
            writeln!(out, "{}", resolved.token)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::{TimeZone, Utc};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use tempfile::TempDir;

    use super::run_impl;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_cfg(server: &str) -> Config {
        Config {
            server: server.to_string(),
        }
    }

    fn fixed_future() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap()
    }

    fn fixed_past() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()
    }

    /// Produce a minimal JWT with the given JSON payload.
    /// The secret is arbitrary — `extract_exp` disables signature validation.
    fn make_jwt(payload: &serde_json::Value) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            payload,
            &EncodingKey::from_secret(b"unused"),
        )
        .expect("test JWT encoding should not fail")
    }

    fn make_jwt_with_exp(exp_ts: i64) -> String {
        make_jwt(&serde_json::json!({ "exp": exp_ts }))
    }

    fn run_buf(
        cfg: &Config,
        cache_path: &Path,
        env_token: Option<String>,
    ) -> (Result<(), Diagnostic>, String) {
        let mut buf: Vec<u8> = Vec::new();
        let result = run_impl(cfg, &mut buf, Some(cache_path), env_token);
        let out = String::from_utf8(buf).unwrap();
        (result, out)
    }

    // ── (a) this command's unique behaviour — print vs. exit 3 ────────────────

    #[test]
    fn cached_unexpired_prints_token() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "tok-123".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_future()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, None);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, "tok-123\n");
    }

    #[test]
    fn cached_expired_produces_auth_required() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "old-tok".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_past()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, None);
        let err = result.expect_err("expected AuthRequired for expired cached token");
        let Diagnostic::AuthRequired(msg) = &err else {
            panic!("expected AuthRequired, got {err:?}");
        };
        // Cache-origin remedy: log in to refresh the cached token.
        assert!(
            msg.contains("cached") && msg.contains("login"),
            "cache-expired message should name the cached token and the login remedy: {msg}"
        );
        assert!(out.is_empty(), "nothing should be printed on error");
    }

    #[test]
    fn no_cache_entry_produces_auth_required() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let (result, out) = run_buf(&cfg, &cache_path, None);
        let err = result.expect_err("expected AuthRequired when no token is cached");
        assert!(
            matches!(err, Diagnostic::AuthRequired(_)),
            "expected AuthRequired, got {err:?}"
        );
        assert!(out.is_empty(), "nothing should be printed on error");
    }

    #[test]
    fn env_jwt_past_exp_produces_auth_required() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let token = make_jwt_with_exp(fixed_past().timestamp());
        let (result, out) = run_buf(&cfg, &cache_path, Some(token));
        let err = result.expect_err("expected AuthRequired for expired env token");
        let Diagnostic::AuthRequired(msg) = &err else {
            panic!("expected AuthRequired, got {err:?}");
        };
        // Env-origin remedy must reference DSP_TOKEN — NOT (only) `dsp auth
        // login`, which cannot fix an expired env token (DSP_TOKEN overrides the
        // cache, ADR-0007). Guards against the origin-agnostic message regression.
        assert!(
            msg.contains("DSP_TOKEN"),
            "env-expired message must reference DSP_TOKEN, not just the cache/login remedy: {msg}"
        );
        assert!(out.is_empty(), "nothing should be printed on error");
    }

    #[test]
    fn env_non_jwt_prints_token() {
        // D5: an undecodable/non-JWT env token has expires_at == None, so it
        // is not treated as expired — it is printed.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let (result, out) = run_buf(&cfg, &cache_path, Some("not-a-jwt".to_string()));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, "not-a-jwt\n");
    }

    #[test]
    fn cached_token_with_no_expiry_prints_token() {
        // D5: a cached entry with expires_at == None is not treated as
        // expired — it is printed.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "opaque-tok".to_string(),
                user: None,
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, None);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, "opaque-tok\n");
    }

    // ── (b) shared resolution logic (mirrors resolve_token/status) ────────────

    #[test]
    fn env_future_exp_wins_over_cache_prints_env_token() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "cache-tok".to_string(),
                user: Some("u@cache.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_future()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let env_token = make_jwt_with_exp(fixed_future().timestamp());
        let (result, out) = run_buf(&cfg, &cache_path, Some(env_token.clone()));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, format!("{env_token}\n"));
    }

    #[test]
    fn whitespace_env_falls_through_to_valid_cache() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "cache-tok".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_future()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, Some("  ".to_string()));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, "cache-tok\n");
    }

    #[test]
    fn whitespace_env_with_expired_cache_produces_auth_required() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "old-tok".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_past()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, Some("  ".to_string()));
        let err = result.expect_err("expected AuthRequired for expired cache fallthrough");
        assert!(
            matches!(err, Diagnostic::AuthRequired(_)),
            "expected AuthRequired, got {err:?}"
        );
        assert!(out.is_empty(), "nothing should be printed on error");
    }

    #[test]
    fn corrupt_cache_with_valid_env_token_prints_env_token() {
        // ADR-0007: DSP_TOKEN wins regardless of cache state. A corrupt
        // auth.toml must not mask the env token.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"not valid toml [[[").unwrap();
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let env_token = make_jwt_with_exp(fixed_future().timestamp());
        let (result, out) = run_buf(&cfg, &cache_path, Some(env_token.clone()));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(out, format!("{env_token}\n"));
    }

    #[test]
    fn corrupt_cache_without_env_token_propagates_error() {
        // Without an env token, a corrupt cache must propagate the load error
        // rather than being swallowed into AuthRequired — the user needs to
        // learn the cache is broken (contrast case: cached_expired above,
        // which is AuthRequired, not a propagated load error).
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"not valid toml [[[").unwrap();
        let cfg = make_cfg("https://api.test.dasch.swiss");

        let (result, out) = run_buf(&cfg, &cache_path, None);
        let err = result.expect_err("expected the load error to propagate");
        assert!(
            !matches!(err, Diagnostic::AuthRequired(_)),
            "corrupt-cache-without-env error must NOT be AuthRequired, got {err:?}"
        );
        assert!(out.is_empty(), "nothing should be printed on error");
    }

    // ── (c) secret hygiene ──────────────────────────────────────────────────

    #[test]
    fn printed_bytes_equal_token_exactly() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");

        const SECRET: &str = "super-secret-bearer-token-xyz";
        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: SECRET.to_string(),
                user: None,
                acquired_at: None,
                expires_at: Some(fixed_future()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let (result, out) = run_buf(&cfg, &cache_path, None);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(
            out,
            format!("{SECRET}\n"),
            "output must be exactly the token plus one newline"
        );
    }

    #[test]
    fn exit_3_error_messages_never_contain_the_token() {
        const SECRET: &str = "super-secret-bearer-token-abc";

        // expired cache path
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg("https://api.test.dasch.swiss");
        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: SECRET.to_string(),
                user: None,
                acquired_at: None,
                expires_at: Some(fixed_past()),
            },
        );
        cache.save_to(&cache_path).unwrap();
        let (result, out) = run_buf(&cfg, &cache_path, None);
        let err = result.expect_err("expected AuthRequired");
        assert!(matches!(err, Diagnostic::AuthRequired(_)));
        assert!(out.is_empty(), "nothing must be written on the exit-3 path");
        assert!(
            !err.to_string().contains(SECRET),
            "expired cache-token error message must not contain the token: {err}"
        );

        // Expired env-token (DSP_TOKEN) path — the JWT string is itself the
        // credential, so a leak would surface it verbatim in the message. This
        // exercises the *env* origin's exit-3 path directly (case 4), rather
        // than leaning on the shared error-construction code with the cache case.
        let env_jwt = make_jwt_with_exp(fixed_past().timestamp());
        let dir2 = TempDir::new().unwrap();
        let cache_path2 = dir2.path().join("auth.toml"); // empty cache → env wins
        let (result2, out2) = run_buf(&cfg, &cache_path2, Some(env_jwt.clone()));
        let err2 = result2.expect_err("expected AuthRequired for expired env token");
        assert!(matches!(err2, Diagnostic::AuthRequired(_)));
        assert!(
            out2.is_empty(),
            "nothing must be written on the exit-3 path"
        );
        assert!(
            !err2.to_string().contains(&env_jwt),
            "expired env-token error message must not contain the token: {err2}"
        );
    }
}
