//! Actions for `dsp auth set-token`.
//!
//! Reads a pre-issued JWT from stdin, verifies it against the server with a
//! live probe, and — only if the probe succeeds — writes it into the auth
//! cache. Subsequent commands then reuse the token until it expires.
//!
//! See plan 008 for the full validation flow and design decisions.

use std::path::Path;

use chrono::Utc;

use crate::actions::auth_state::read_auth_state;
use crate::client::DspClient;
use crate::config::auth_cache::ServerEntry;
use crate::config::{AuthCache, Config, ResolvedToken, TokenOrigin};
use crate::diagnostic::Diagnostic;
use crate::render::auth::AuthSetTokenOutcome;
use crate::render::{MetaContext, Renderer};

/// Cache a pre-issued bearer token read from stdin.
///
/// Reads a JWT from stdin, then delegates to [`run_from_line`] for the
/// trim→empty-guard→decode→probe→cache→render flow.
pub fn run(cfg: &Config, client: &dyn DspClient, renderer: &mut dyn Renderer) -> Result<(), Diagnostic> {
    let mut line = String::new();
    // `read_line` returns `Ok(0)` on EOF (not an error); the empty-guard in
    // `run_from_line` handles that case. A real I/O failure (broken pipe, etc.)
    // is a bad invocation rather than a CLI bug, so we surface it as Usage
    // rather than Internal — consistent with login.rs's stdin error handling.
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| Diagnostic::Usage(format!("could not read token from stdin: {e}")))?;
    run_from_line(line, cfg, client, renderer, None)
}

/// Trim, guard empty, then decode→probe→cache→render.
///
/// Extracted as a plain-function seam so tests can exercise the empty-guard
/// and newline-trim path without touching stdin. Production callers use `run`;
/// tests call this directly with an injected string and a tempdir cache path.
fn run_from_line(
    mut line: String,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // Strip a single trailing line terminator first (shared with the password
    // path; see `crate::actions::auth::trim_line_ending`).
    crate::actions::auth::trim_line_ending(&mut line);

    // Then trim all surrounding whitespace for both the empty-guard and the
    // value forwarded onward. A JWT never contains surrounding whitespace, so
    // this is safe and keeps the guard and the cached token symmetric (a pasted
    // "  <jwt>  " decodes/caches cleanly rather than failing as "not a valid
    // JWT"). Unlike `trim_line_ending`, `trim()` also eats a bare surrounding
    // `\r` — fine here, since the forwarded token must not carry whitespace.
    let token = line.trim();
    if token.is_empty() {
        return Err(Diagnostic::Usage("no token provided on stdin".to_string()));
    }

    run_impl(token, cfg, client, renderer, cache_path)
}

/// Internal entry point that accepts an explicit cache path (for tests).
///
/// Production callers use `run`; tests call this directly to inject a
/// tempdir-backed cache path and a pre-built token string.
///
/// # Security note
///
/// The `token` parameter is a bearer secret and must **never** be logged,
/// printed to stderr, or included in any `tracing::debug!` / `tracing::info!`
/// call. It is stored into `auth.toml` (which uses a manual `Debug` impl that
/// redacts the token) and forwarded to `client.verify_token` only.
fn run_impl(
    token: &str,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // 1. Local decode: extract metadata without verifying the signature. Failure means the input is
    //    not structurally a JWT → Usage error (exit 2). We do NOT locally enforce `exp`; a
    //    locally-expired token may still pass the live probe if the server's clock differs, and the
    //    probe is the trust boundary in any case.
    let meta = crate::client::jwt::extract_meta(token)
        .ok_or_else(|| Diagnostic::Usage("input on stdin is not a valid JWT".to_string()))?;

    // 2. Live probe: verify the token is currently accepted by the server. This is the
    //    authoritative validity gate — no token is cached before this succeeds. 401/403 →
    //    AuthRequired (exit 3); other failures propagate.
    client.verify_token(&cfg.server, token)?;

    // 3. Cache (only reached when probe returned Ok).
    let entry = ServerEntry {
        token: token.to_string(),
        user: meta.sub.clone(),
        acquired_at: Some(Utc::now()),
        expires_at: meta.exp,
    };

    let mut cache = match cache_path {
        Some(p) => AuthCache::load_from(p)?,
        None => AuthCache::load()?,
    };
    cache.set_entry(&cfg.server, entry);
    match cache_path {
        Some(p) => cache.save_to(p)?,
        None => cache.save()?,
    }

    // 4. Build MetaContext reflecting the post-set-token auth state using the
    // shared ADR-0007 helper. The token was just stored in the cache as a
    // Cache-origin entry with `meta.sub` as the user. Synthesize a Cache-origin
    // ResolvedToken so `read_auth_state` picks the correct branch.
    let resolved_for_meta = ResolvedToken { token: token.to_string(), origin: TokenOrigin::Cache };
    let meta_ctx = MetaContext {
        server_label: cfg.server.clone(),
        auth_state: read_auth_state(Some(&resolved_for_meta), &cache, &cfg.server),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };

    // 5. Render the outcome.
    let outcome = AuthSetTokenOutcome {
        server: cfg.server.clone(),
        user: meta.sub,
        expires_at: meta.exp,
    };

    renderer.auth_set_token(&outcome, &meta_ctx)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use tempfile::TempDir;

    use super::{run_from_line, run_impl};
    use crate::client::DspClient;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::{CreateDumpOutcome, DumpTask, LoginResponse, ProjectRef};
    use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
    use crate::render::{MetaContext, Renderer};

    // ── local mock client ─────────────────────────────────────────────────────

    struct MockDspClient {
        verify_token_result: Result<(), Diagnostic>,
    }

    impl MockDspClient {
        fn ok() -> Self {
            Self { verify_token_result: Ok(()) }
        }

        fn err(diag: Diagnostic) -> Self {
            Self { verify_token_result: Err(diag) }
        }
    }

    impl DspClient for MockDspClient {
        fn login(&self, _server: &str, _user: &str, _password: &str) -> Result<LoginResponse, Diagnostic> {
            unimplemented!("login not used by set-token tests")
        }

        fn resolve_project(&self, _server: &str, _project: &str) -> Result<ProjectRef, Diagnostic> {
            unimplemented!("resolve_project not used by set-token tests")
        }

        fn create_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _skip_assets: bool,
            _token: &str,
        ) -> Result<CreateDumpOutcome, Diagnostic> {
            unimplemented!("create_project_dump not used by set-token tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used by set-token tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used by set-token tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used by set-token tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used by set-token tests")
        }

        fn list_projects(&self, _server: &str, _token: Option<&str>) -> Result<Vec<crate::model::Project>, Diagnostic> {
            Err(Diagnostic::NotImplemented(
                "list_projects not used in set_token.rs tests".into(),
            ))
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::DataModel>, Diagnostic> {
            unimplemented!("list_data_models not used by set-token tests")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            unimplemented!("describe_data_model not used in set-token tests")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in set-token tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in set-token tests")
        }

        fn list_resources(
            &self,
            _server: &str,
            _project_iri: &str,
            _resource_type_iri: &str,
            _order_by: Option<&str>,
            _page: u32,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourcePage, Diagnostic> {
            unimplemented!("list_resources not used in set-token tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in set-token tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            self.verify_token_result.clone()
        }

        fn resource_counts(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<std::collections::HashMap<String, u64>, Diagnostic> {
            Ok(std::collections::HashMap::new())
        }

        fn list_vocabularies(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Vocabulary>, Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn describe_vocabulary(
            &self,
            _server: &str,
            _iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::VocabularyTree, Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn sparql_query(
            &self,
            _server: &str,
            _token: &str,
            _query: &str,
            _accept: &str,
            _timeout_secs: u64,
        ) -> Result<crate::client::sparql::SparqlResponse, Diagnostic> {
            Err(Diagnostic::Internal("not used in this test".into()))
        }
    }

    // ── recording renderer ─────────────────────────────────────────────────────

    struct RecordingRenderer {
        set_token_outcome: Option<AuthSetTokenOutcome>,
        set_token_auth_state: Option<String>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self { set_token_outcome: None, set_token_auth_state: None }
        }
    }

    impl Renderer for RecordingRenderer {
        fn diagnostic(&mut self, _diag: &Diagnostic, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_login(&mut self, _outcome: &AuthLoginOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_status(&mut self, _outcome: &AuthStatusOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_logout(&mut self, _outcome: &AuthLogoutOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_set_token(&mut self, outcome: &AuthSetTokenOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.set_token_outcome = Some(AuthSetTokenOutcome {
                server: outcome.server.clone(),
                user: outcome.user.clone(),
                expires_at: outcome.expires_at,
            });
            self.set_token_auth_state = Some(meta.auth_state.clone());
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

        fn projects(&mut self, _view: &crate::render::ProjectListView, _meta: &MetaContext) -> Result<(), Diagnostic> {
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
            unimplemented!("resource_type_describe not used in set-token tests")
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in set-token tests")
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

    // ── helpers ────────────────────────────────────────────────────────────────

    /// Produce a minimal JWT with the given JSON payload.
    /// The secret is arbitrary — `extract_meta` disables signature validation.
    fn make_jwt(payload: &serde_json::Value) -> String {
        encode(&Header::new(Algorithm::HS256), payload, &EncodingKey::from_secret(b"unused"))
            .expect("test JWT encoding should not fail")
    }

    fn fixed_expires() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2099, 6, 25, 12, 34, 56).unwrap()
    }

    fn fixed_past_expires() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()
    }

    fn make_cfg(server: &str) -> Config {
        Config { server: server.to_string() }
    }

    const SERVER: &str = "https://api.test.dasch.swiss";

    // ── tests ──────────────────────────────────────────────────────────────────

    #[test]
    fn happy_path_full_jwt_caches_entry_and_notifies_renderer() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap();

        // Cache assertions.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), Some(token.as_str()));
        assert_eq!(
            loaded.user(SERVER),
            Some("http://rdfh.ch/users/root"),
            "user should be set to the JWT sub claim"
        );
        assert!(
            loaded.acquired_at(SERVER).is_some(),
            "acquired_at should be set after set-token"
        );
        assert_eq!(
            loaded.expires_at(SERVER),
            Some(fixed_expires()),
            "expires_at should match the JWT exp claim"
        );

        // Renderer assertions.
        let outcome = renderer.set_token_outcome.unwrap();
        assert_eq!(outcome.server, SERVER);
        assert_eq!(outcome.user.as_deref(), Some("http://rdfh.ch/users/root"));
        assert_eq!(outcome.expires_at, Some(fixed_expires()));
        assert_eq!(
            renderer.set_token_auth_state.as_deref(),
            Some("authenticated as http://rdfh.ch/users/root"),
            "auth_state should use ADR-0007 vocabulary and include the sub IRI"
        );
    }

    #[test]
    fn space_padded_jwt_is_trimmed_before_decode_and_cache() {
        // A JWT pasted with surrounding whitespace must be trimmed for both the
        // empty-guard and the value forwarded onward, so it decodes and caches
        // cleanly rather than failing as "not a valid JWT". Goes through
        // `run_from_line` (the trim seam), not `run_impl`.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        run_from_line(format!("  {token}  "), &cfg, &client, &mut renderer, Some(&cache_path)).unwrap();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token(SERVER),
            Some(token.as_str()),
            "the cached token must be the whitespace-trimmed JWT"
        );
    }

    #[test]
    fn happy_path_sub_absent_caches_user_none() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        // JWT with exp but no sub.
        let token = make_jwt(&serde_json::json!({
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.user(SERVER), None, "user should be None when sub absent");

        let outcome = renderer.set_token_outcome.unwrap();
        assert_eq!(outcome.user, None, "outcome.user should be None when sub absent");
        assert_eq!(
            renderer.set_token_auth_state.as_deref(),
            Some("authenticated"),
            "auth_state should be 'authenticated' (no sub, ADR-0007 vocabulary)"
        );
    }

    #[test]
    fn probe_200_with_locally_expired_token_still_cached() {
        // Guards the "don't pre-check exp locally" invariant from the plan:
        // if the live probe returns Ok, we cache regardless of exp value.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_past_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok(); // probe returns 200
        let mut renderer = RecordingRenderer::new();

        // Must succeed and cache the token — exp is not locally enforced.
        run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token(SERVER),
            Some(token.as_str()),
            "locally-expired token should still be cached when probe returns Ok"
        );
    }

    #[test]
    fn non_jwt_stdin_returns_usage_error_nothing_cached() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        let err = run_impl("not-a-jwt", &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage diagnostic for non-JWT input, got {err:?}"
        );
        // Pin the distinct decode-failure message so it can't silently collapse
        // into the empty-guard message.
        assert!(
            err.to_string().contains("not a valid JWT"),
            "decode-failure message must contain 'not a valid JWT', got: {err}"
        );

        // Nothing should have been cached.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), None, "non-JWT input must not write anything to the cache");
    }

    #[test]
    fn empty_stdin_returns_usage_no_token_nothing_cached() {
        // Guards the real empty-guard path in `run_from_line` (previously untested:
        // the old test called `run_impl("")` which hit the JWT-decode branch with a
        // DIFFERENT message — "not a valid JWT" — not the empty-guard path).
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        let err = run_from_line(String::new(), &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage for empty stdin line, got {err:?}"
        );
        assert!(
            err.to_string().contains("no token provided"),
            "empty-guard message must contain 'no token provided', got: {err}"
        );

        // Nothing should have been cached.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), None, "empty stdin must not write anything to the cache");
    }

    #[test]
    fn newline_only_stdin_returns_usage_no_token() {
        // A bare newline from stdin (e.g. user pressed Enter) trims to empty string
        // and must hit the empty-guard path, not the JWT-decode path.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        let err = run_from_line("\n".to_string(), &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage for newline-only stdin line, got {err:?}"
        );
        assert!(
            err.to_string().contains("no token provided"),
            "newline-only stdin must hit the empty-guard path; message must contain 'no token provided', got: {err}"
        );
    }

    #[test]
    fn whitespace_only_stdin_returns_usage_no_token() {
        // A spaces-only line (no newline) must hit the empty-guard path.
        // Before the fix, `line.is_empty()` let "   " fall through to JWT-decode;
        // after the fix, `line.trim().is_empty()` catches it and returns the
        // "no token provided" message.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::ok();
        let mut renderer = RecordingRenderer::new();

        let err = run_from_line("   ".to_string(), &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage for whitespace-only stdin, got {err:?}"
        );
        assert!(
            err.to_string().contains("no token provided"),
            "whitespace-only stdin must hit the empty-guard path; message must contain 'no token provided', got: {err}"
        );

        // Nothing should have been cached.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token(SERVER),
            None,
            "whitespace-only stdin must not write anything to the cache"
        );
    }

    #[test]
    fn probe_401_returns_auth_required_nothing_cached() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::err(Diagnostic::AuthRequired(format!(
            "token rejected by {SERVER} — it may be expired, revoked, or for a different environment"
        )));
        let mut renderer = RecordingRenderer::new();

        let err = run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::AuthRequired(_)),
            "expected AuthRequired for probe 401, got {err:?}"
        );

        // Nothing should have been cached.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), None, "probe 401 must not write anything to the cache");
    }

    #[test]
    fn probe_403_returns_auth_required_nothing_cached() {
        // At the action layer the mock injects an identical `Diagnostic::AuthRequired`
        // as the 401 test — this only proves AuthRequired propagates and nothing is
        // cached. The real 401-vs-403 HTTP-status distinction is covered by
        // `tests/set_token_http.rs`.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::err(Diagnostic::AuthRequired(format!(
            "token rejected by {SERVER} — it may be expired, revoked, or for a different environment"
        )));
        let mut renderer = RecordingRenderer::new();

        let err = run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::AuthRequired(_)),
            "expected AuthRequired for probe 403, got {err:?}"
        );

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), None, "probe 403 must not write anything to the cache");
    }

    #[test]
    fn probe_server_error_returns_server_error_nothing_cached() {
        // Closes the action-layer ServerError gap: when verify_token returns
        // Err(ServerError), run_impl must propagate it and leave the cache clean.
        // The wiremock test already covers the HTTP-level mapping; this test covers
        // the action layer independently.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client =
            MockDspClient::err(Diagnostic::ServerError("server returned 500 Internal Server Error".to_string()));
        let mut renderer = RecordingRenderer::new();

        let err = run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::ServerError(_)),
            "expected ServerError when verify_token returns ServerError, got {err:?}"
        );

        // Nothing should have been cached.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token(SERVER), None, "server error must not write anything to the cache");
    }

    #[test]
    fn probe_network_error_propagates_nothing_cached() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::err(Diagnostic::Network("connection refused".to_string()));
        let mut renderer = RecordingRenderer::new();

        let err = run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Network(_)),
            "expected Network diagnostic, got {err:?}"
        );

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token(SERVER),
            None,
            "network failure must not write anything to the cache"
        );
    }

    #[test]
    fn probe_failure_does_not_overwrite_existing_entry() {
        // Guards that a pre-existing valid token is not replaced by a failed set-token.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");

        // Pre-populate with a good entry.
        let mut cache = AuthCache::default();
        cache.set_entry(
            SERVER,
            ServerEntry {
                token: "existing-valid-tok".to_string(),
                user: Some("existing@user.test".to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/new",
            "exp": fixed_expires().timestamp(),
        }));
        let cfg = make_cfg(SERVER);
        let client = MockDspClient::err(Diagnostic::AuthRequired("rejected".to_string()));
        let mut renderer = RecordingRenderer::new();

        run_impl(&token, &cfg, &client, &mut renderer, Some(&cache_path)).unwrap_err();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token(SERVER),
            Some("existing-valid-tok"),
            "failed set-token must not overwrite an existing valid cache entry"
        );
    }
}
