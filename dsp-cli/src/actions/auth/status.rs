//! Actions for `dsp auth status`.
//!
//! Reads the auth cache and reports whether a token is present and whether it
//! has expired. No network call is made — this is a pure cache inspection.

use std::path::Path;

use chrono::Utc;

use crate::actions::auth_state::read_auth_state;
use crate::cli::StatusArgs;
use crate::config::{AuthCache, Config, TokenOrigin, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::render::auth::AuthStatusOutcome;
use crate::render::{MetaContext, Renderer};

/// Show authentication status for a DSP server.
///
/// Loads the auth cache, looks up the server, and renders the appropriate
/// outcome. Always returns `Ok(())` — missing/expired tokens are not errors.
pub fn run(args: &StatusArgs, cfg: &Config, renderer: &mut dyn Renderer) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_impl(args, cfg, renderer, None, env_token)
}

fn run_impl(
    _args: &StatusArgs,
    cfg: &Config,
    renderer: &mut dyn Renderer,
    cache_path: Option<&Path>,
    env_token: Option<String>,
) -> Result<(), Diagnostic> {
    // ADR-0007 says a non-blank `DSP_TOKEN` wins regardless of cache state.
    // A corrupt or unreadable `auth.toml` therefore must not mask the env
    // token: treat a cache-load failure as an empty cache when the env token
    // would resolve. (Matches the trim-and-empty rule in `resolve_token`.)
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

    let (outcome, resolved_opt) = match resolve_token(env_token, &cache, &cfg.server) {
        Some(resolved) if resolved.origin == TokenOrigin::Env => {
            // do not log/format this binding — it is the raw bearer secret
            let token = resolved.token.clone();
            let now = Utc::now();
            // extract_exp is display-only (status makes no HTTP call) and never
            // gates access. It reads the JWT `exp` claim without validating the
            // signature, identical to the login path's cache-store behaviour.
            let expires_at = crate::client::jwt::extract_exp(&token);
            let expired = expires_at.map(|t| t < now).unwrap_or(false);
            (
                AuthStatusOutcome::AuthenticatedViaEnv {
                    server: cfg.server.clone(),
                    expires_at,
                    expired,
                },
                Some(resolved),
            )
        }
        Some(resolved) => {
            // Cache origin — read user/expires_at from cache directly (the
            // resolver only confirms origin; the full entry still comes from cache).
            let user = cache.user(&cfg.server).map(str::to_owned);
            let expires_at = cache.expires_at(&cfg.server);
            let expired = expires_at.map(|t| t < Utc::now()).unwrap_or(false);
            (
                AuthStatusOutcome::LoggedIn {
                    server: cfg.server.clone(),
                    user,
                    expires_at,
                    expired,
                },
                Some(resolved),
            )
        }
        None => (
            AuthStatusOutcome::NotLoggedIn {
                server: cfg.server.clone(),
            },
            None,
        ),
    };

    // _meta.auth uses presence/origin semantics (ADR-0007 uniform vocabulary),
    // independent of expiry. This is the same as the read commands: an expired
    // cached token still reports "authenticated as {user}" in _meta.auth.
    // The richer expiry/not-logged-in detail lives in the data output above.
    let auth_state = read_auth_state(resolved_opt.as_ref(), &cache, &cfg.server);

    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };

    renderer.auth_status(&outcome, &meta)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use tempfile::TempDir;

    use super::run_impl;
    use crate::cli::{FormatArgs, StatusArgs};
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::render::auth::{
        AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome,
    };
    use crate::render::{Format, MetaContext, Renderer};

    // ── recording renderer ────────────────────────────────────────────────────

    struct LoggedInRecord {
        server: String,
        user: Option<String>,
        expires_at: Option<chrono::DateTime<Utc>>,
        expired: bool,
    }

    struct EnvRecord {
        server: String,
        expires_at: Option<chrono::DateTime<Utc>>,
        expired: bool,
    }

    struct RecordingRenderer {
        status_outcome: Option<LoggedInRecord>,
        env_outcome: Option<EnvRecord>,
        not_logged_in: Option<String>,
        last_auth_state: Option<String>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                status_outcome: None,
                env_outcome: None,
                not_logged_in: None,
                last_auth_state: None,
            }
        }
    }

    impl Renderer for RecordingRenderer {
        fn diagnostic(
            &mut self,
            _diag: &Diagnostic,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_login(
            &mut self,
            _outcome: &AuthLoginOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_status(
            &mut self,
            outcome: &AuthStatusOutcome,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.last_auth_state = Some(meta.auth_state.clone());
            match outcome {
                AuthStatusOutcome::LoggedIn {
                    server,
                    user,
                    expires_at,
                    expired,
                } => {
                    self.status_outcome = Some(LoggedInRecord {
                        server: server.clone(),
                        user: user.clone(),
                        expires_at: *expires_at,
                        expired: *expired,
                    });
                }
                AuthStatusOutcome::AuthenticatedViaEnv {
                    server,
                    expires_at,
                    expired,
                } => {
                    self.env_outcome = Some(EnvRecord {
                        server: server.clone(),
                        expires_at: *expires_at,
                        expired: *expired,
                    });
                }
                AuthStatusOutcome::NotLoggedIn { server } => {
                    self.not_logged_in = Some(server.clone());
                }
            }
            Ok(())
        }

        fn auth_logout(
            &mut self,
            _outcome: &AuthLogoutOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_set_token(
            &mut self,
            _outcome: &AuthSetTokenOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump(
            &mut self,
            _outcome: &crate::render::DumpOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump_deleted(
            &mut self,
            _outcome: &crate::render::DumpDeleteOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn projects(
            &mut self,
            _view: &crate::render::ProjectListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_describe(
            &mut self,
            _project: &crate::model::ProjectDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_models(
            &mut self,
            _view: &crate::render::DataModelListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_model_describe(
            &mut self,
            _detail: &crate::model::DataModelDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_types(
            &mut self,
            _view: &crate::render::ResourceTypeListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_type_describe(
            &mut self,
            _detail: &crate::model::ResourceTypeDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("resource_type_describe not used in status tests")
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in status tests")
        }

        fn resources(
            &mut self,
            _view: &crate::render::ResourceListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_describe(
            &mut self,
            _detail: &crate::model::ResourceDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn vocabularies(
            &mut self,
            _view: &crate::render::VocabularyListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn vocabulary_describe(
            &mut self,
            _detail: &crate::model::VocabularyDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_args(server: &str) -> (StatusArgs, Config) {
        let args = StatusArgs {
            server: Some(server.to_string()),
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        };
        let cfg = Config {
            server: server.to_string(),
        };
        (args, cfg)
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

    // ── existing tests (regression guard) ────────────────────────────────────

    #[test]
    fn logged_in_entry_produces_logged_in_outcome() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

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

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None).unwrap();

        let rec = renderer.status_outcome.unwrap();
        assert_eq!(rec.server, "https://api.test.dasch.swiss");
        assert_eq!(rec.user.as_deref(), Some("u@x.test"));
        assert_eq!(rec.expires_at, Some(fixed_future()));
        assert!(
            !rec.expired,
            "token with future expiry should not be expired"
        );
    }

    #[test]
    fn empty_cache_produces_not_logged_in_outcome() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None).unwrap();

        assert!(
            renderer.status_outcome.is_none(),
            "expected no LoggedIn outcome"
        );
        assert_eq!(
            renderer.not_logged_in.as_deref(),
            Some("https://api.test.dasch.swiss")
        );
    }

    #[test]
    fn expired_entry_produces_logged_in_with_expired_true() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

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

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None).unwrap();

        let rec = renderer.status_outcome.unwrap();
        assert!(rec.expired, "token with past expiry should be expired");
    }

    #[test]
    fn run_always_returns_ok_even_for_not_logged_in() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        let result = run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None);
        assert!(
            result.is_ok(),
            "status should always return Ok; got {result:?}"
        );
    }

    // ── env-token tests ───────────────────────────────────────────────────────

    #[test]
    fn env_jwt_future_exp_produces_authenticated_via_env_not_expired() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        // far-future exp: year 2099 ≈ Unix 4070908800
        let exp_ts = fixed_future().timestamp();
        let token = make_jwt_with_exp(exp_ts);

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), Some(token)).unwrap();

        let rec = renderer
            .env_outcome
            .expect("expected AuthenticatedViaEnv outcome");
        assert_eq!(rec.server, "https://api.test.dasch.swiss");
        assert!(rec.expires_at.is_some(), "expected Some(expires_at)");
        assert!(!rec.expired, "future exp should not be expired");
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
            "_meta.auth must use ADR-0007 presence/origin vocabulary (not expiry-aware)"
        );
    }

    #[test]
    fn env_jwt_past_exp_produces_authenticated_via_env_expired() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let exp_ts = fixed_past().timestamp();
        let token = make_jwt_with_exp(exp_ts);

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), Some(token)).unwrap();

        let rec = renderer
            .env_outcome
            .expect("expected AuthenticatedViaEnv outcome");
        assert!(rec.expired, "past exp should be expired");
        // _meta.auth uses presence/origin semantics — expired env token still
        // reports "authenticated via DSP_TOKEN", not an expiry string.
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
            "_meta.auth for expired env token must be 'authenticated via DSP_TOKEN'"
        );
        // The data output (AuthenticatedViaEnv.expired) correctly reflects the expiry.
        assert!(rec.expired, "data output must still report expired==true");
    }

    #[test]
    fn env_non_jwt_produces_authenticated_via_env_expiry_unknown() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some("not-a-jwt".to_string()),
        )
        .unwrap();

        let rec = renderer
            .env_outcome
            .expect("expected AuthenticatedViaEnv outcome");
        assert!(
            rec.expires_at.is_none(),
            "non-JWT env token should have expires_at == None"
        );
        assert!(!rec.expired, "non-JWT env token should not be expired");
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
            "_meta.auth for non-JWT env token must be 'authenticated via DSP_TOKEN'"
        );
    }

    #[test]
    fn env_token_wins_over_valid_cache_entry() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        // Populate cache with a valid entry for the same server.
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

        let exp_ts = fixed_future().timestamp();
        let env_token = make_jwt_with_exp(exp_ts);

        let mut renderer = RecordingRenderer::new();
        run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some(env_token),
        )
        .unwrap();

        // Env wins: must be AuthenticatedViaEnv, not LoggedIn.
        assert!(
            renderer.env_outcome.is_some(),
            "env token should win over valid cache entry"
        );
        assert!(
            renderer.status_outcome.is_none(),
            "LoggedIn should not be produced when env token is present"
        );
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
            "env win over valid cache should report 'authenticated via DSP_TOKEN'"
        );
    }

    #[test]
    fn env_token_wins_over_expired_cache_entry() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        // Populate cache with an expired entry.
        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "old-cache-tok".to_string(),
                user: Some("u@cache.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_past()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let exp_ts = fixed_future().timestamp();
        let env_token = make_jwt_with_exp(exp_ts);

        let mut renderer = RecordingRenderer::new();
        run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some(env_token),
        )
        .unwrap();

        assert!(
            renderer.env_outcome.is_some(),
            "env token should win over expired cache entry"
        );
        assert!(
            renderer.status_outcome.is_none(),
            "LoggedIn should not be produced when env token is present"
        );
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
            "env win over expired cache should report 'authenticated via DSP_TOKEN'"
        );
    }

    #[test]
    fn whitespace_only_env_with_expired_cache_falls_through_to_cache() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

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

        let mut renderer = RecordingRenderer::new();
        run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some("  ".to_string()), // whitespace-only → treated as absent
        )
        .unwrap();

        // Falls through to cache: must be LoggedIn with expired==true.
        let rec = renderer
            .status_outcome
            .expect("expected LoggedIn outcome from cache fall-through");
        assert!(
            rec.expired,
            "cache entry has past expiry; expired should be true"
        );
        assert!(
            renderer.env_outcome.is_none(),
            "whitespace env should not produce AuthenticatedViaEnv"
        );
        // _meta.auth uses presence/origin semantics — expired cached token with
        // a known user still reports "authenticated as {user}", not an expiry string.
        // The expiry detail lives in the data output (LoggedIn.expired == true).
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated as u@x.test"),
            "_meta.auth for expired cache token must be 'authenticated as <user>'"
        );
    }

    #[test]
    fn env_token_wins_when_cache_is_corrupt() {
        // ADR-0007: DSP_TOKEN wins regardless of cache state. A corrupt
        // auth.toml must not mask the env token in `dsp auth status`.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"not valid toml [[[").unwrap();

        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let exp_ts = fixed_future().timestamp();
        let env_token = make_jwt_with_exp(exp_ts);

        let mut renderer = RecordingRenderer::new();
        run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some(env_token),
        )
        .unwrap();

        assert!(
            renderer.env_outcome.is_some(),
            "env token should win over corrupt cache"
        );
        assert!(
            renderer.status_outcome.is_none(),
            "LoggedIn should not be produced when env token is present"
        );
        assert_eq!(
            renderer.last_auth_state.as_deref(),
            Some("authenticated via DSP_TOKEN"),
        );
    }

    #[test]
    fn corrupt_cache_without_env_token_still_errors() {
        // Regression guard for the other side of the env-wins fix: when no env
        // token is set, a corrupt cache must still propagate the error rather
        // than silently producing `not_logged_in`. The user needs to know the
        // cache is broken so they can fix or delete it.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"not valid toml [[[").unwrap();

        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let mut renderer = RecordingRenderer::new();
        let result = run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None);
        assert!(
            result.is_err(),
            "without env token, a corrupt cache should propagate; got {result:?}"
        );
    }

    #[test]
    fn whitespace_only_env_with_corrupt_cache_still_errors() {
        // Mirrors the cache-fall-through whitespace test: a blank env token
        // must not be enough to swallow the cache-load error. The trim rule
        // here matches `resolve_token`'s blank-handling.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"not valid toml [[[").unwrap();

        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let mut renderer = RecordingRenderer::new();
        let result = run_impl(
            &args,
            &cfg,
            &mut renderer,
            Some(&cache_path),
            Some("  ".to_string()),
        );
        assert!(
            result.is_err(),
            "whitespace env token should not swallow corrupt-cache error; got {result:?}"
        );
    }

    #[test]
    fn absent_env_with_cache_entry_produces_logged_in() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "tok-abc".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: Some(fixed_future()),
            },
        );
        cache.save_to(&cache_path).unwrap();

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path), None).unwrap();

        assert!(
            renderer.status_outcome.is_some(),
            "absent env + cache entry should produce LoggedIn"
        );
        assert!(
            renderer.env_outcome.is_none(),
            "absent env should not produce AuthenticatedViaEnv"
        );
    }
}
