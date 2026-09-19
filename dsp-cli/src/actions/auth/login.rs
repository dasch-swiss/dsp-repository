//! Actions for `dsp auth login`.
//!
//! Reads the user's password via a `PasswordSource` indirection so tests can
//! inject a static password without touching the real TTY. See Step 5 of the
//! implementation plan.

use std::path::Path;

use chrono::Utc;

use crate::actions::auth_state::read_auth_state;
use crate::cli::LoginArgs;
use crate::client::DspClient;
use crate::config::auth_cache::ServerEntry;
use crate::config::{AuthCache, Config, ResolvedToken, TokenOrigin};
use crate::diagnostic::Diagnostic;
use crate::render::auth::AuthLoginOutcome;
use crate::render::{MetaContext, Renderer};

// PasswordSource is a deliberate trait-injected seam for tests; keep it.
trait PasswordSource {
    fn read(&self, prompt: &str) -> Result<String, Diagnostic>;
}

struct TtyPasswordSource;

impl PasswordSource for TtyPasswordSource {
    fn read(&self, prompt: &str) -> Result<String, Diagnostic> {
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            // rpassword opens /dev/tty itself. If that fails (sandboxed/container
            // env without /dev/tty), surface a user-actionable message rather
            // than Diagnostic::Internal.
            rpassword::prompt_password(prompt).map_err(|_| {
                Diagnostic::Usage(
                    "could not open terminal for password prompt; pipe the password via stdin instead".into(),
                )
            })
        } else {
            let mut line = String::new();
            // A read failure here (closed pipe, no data on stdin) is a bad
            // invocation, not a CLI bug — surface it as Usage rather than the
            // From<io::Error> default of Internal.
            std::io::stdin().read_line(&mut line).map_err(|e| {
                Diagnostic::Usage(format!("could not read password from stdin: {e}"))
            })?;
            // Strip a single trailing line terminator via the shared helper.
            // See `crate::actions::auth::trim_line_ending` for the rationale:
            // a bare trailing `\r` is NOT stripped (it may be a legitimate
            // password char), so we must not use `trim_end_matches`.
            crate::actions::auth::trim_line_ending(&mut line);
            Ok(line)
        }
    }
}

/// Resolve the password to use for login.
///
/// A non-empty `DSP_PASSWORD` value takes precedence over the interactive
/// prompt / stdin. The env value is passed in (not read here) so this stays
/// pure and unit-testable without mutating process env.
///
/// SECURITY: `DSP_PASSWORD` is a plaintext password, typically living in a
/// `.env` file on disk. Use it only for **local / dev / test** setups —
/// **never a production password**. For non-interactive use against real
/// environments, prefer a scoped, expiring token (`DSP_TOKEN`) over a
/// durable master credential. See ADR-0007.
fn resolve_password(
    env_password: Option<String>,
    source: &dyn PasswordSource,
) -> Result<String, Diagnostic> {
    match env_password {
        Some(p) if !p.is_empty() => Ok(p),
        _ => source.read("Password: "),
    }
}

/// Log in to a DSP server.
///
/// Authenticates with the DSP-API, stores the token in the auth cache, and
/// renders the outcome. Password resolution order: `DSP_PASSWORD` env var
/// (local/dev only — see [`resolve_password`]), then the TTY prompt, then
/// stdin when stdin is not a terminal (see ADR-0007).
pub fn run(
    args: &LoginArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_password = std::env::var("DSP_PASSWORD").ok();
    run_impl(
        args,
        cfg,
        client,
        renderer,
        &TtyPasswordSource,
        env_password,
        None,
    )
}

/// Internal entry point that accepts an explicit cache path (for tests) and an
/// injectable `PasswordSource`. Production callers use `run`; tests use this
/// directly to inject a tempdir-backed cache path and a static password.
fn run_impl(
    args: &LoginArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    password_source: &dyn PasswordSource,
    env_password: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    let user = args.user.as_deref().ok_or_else(|| {
        Diagnostic::Usage("--user (email, username, or IRI) is required for login".to_string())
    })?;

    let password = resolve_password(env_password, password_source)?;

    let response = client.login(&cfg.server, user, &password)?;

    let entry = ServerEntry {
        token: response.token.clone(),
        user: Some(response.user.clone()),
        acquired_at: Some(Utc::now()),
        expires_at: response.expires_at,
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

    // Build the ADR-0007 auth-state via the shared helper. After a successful
    // login the token is stored in the cache as a Cache-origin token with the
    // returned user name. Synthesize a Cache-origin ResolvedToken so that
    // `read_auth_state` picks the correct branch and looks up the user from the
    // cache (which now contains `response.user`).
    let resolved_for_meta = ResolvedToken {
        token: response.token.clone(),
        origin: TokenOrigin::Cache,
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state: read_auth_state(Some(&resolved_for_meta), &cache, &cfg.server),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };

    let outcome = AuthLoginOutcome {
        server: cfg.server.clone(),
        user: response.user,
        expires_at: response.expires_at,
    };

    renderer.auth_login(&outcome, &meta)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    use super::{PasswordSource, resolve_password, run_impl};
    use crate::cli::{FormatArgs, LoginArgs};
    use crate::client::DspClient;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::LoginResponse;
    use crate::render::auth::{
        AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome,
    };
    use crate::render::{Format, MetaContext, Renderer};

    // ── local mock client ─────────────────────────────────────────────────────

    struct MockDspClient {
        result: Result<LoginResponse, Diagnostic>,
    }

    impl MockDspClient {
        fn ok(token: &str, user: &str, expires_at: Option<chrono::DateTime<Utc>>) -> Self {
            Self {
                result: Ok(LoginResponse {
                    token: token.to_string(),
                    user: user.to_string(),
                    expires_at,
                }),
            }
        }

        fn err(diag: Diagnostic) -> Self {
            Self { result: Err(diag) }
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<LoginResponse, Diagnostic> {
            self.result.clone()
        }

        fn resolve_project(
            &self,
            _server: &str,
            _project: &str,
        ) -> Result<crate::model::ProjectRef, Diagnostic> {
            unimplemented!("resolve_project not used by login tests")
        }

        fn create_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _skip_assets: bool,
            _token: &str,
        ) -> Result<crate::model::CreateDumpOutcome, Diagnostic> {
            unimplemented!("create_project_dump not used by login tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used by login tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used by login tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used by login tests")
        }

        fn list_projects(
            &self,
            _server: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Project>, Diagnostic> {
            Err(Diagnostic::NotImplemented(
                "list_projects not used in login.rs tests".into(),
            ))
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            Err(Diagnostic::NotImplemented(
                "describe_project not used in login.rs tests".into(),
            ))
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::DataModel>, Diagnostic> {
            Err(Diagnostic::NotImplemented(
                "list_data_models not used in login.rs tests".into(),
            ))
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            unimplemented!("describe_data_model not used in login tests")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in login tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in login tests")
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
            unimplemented!("list_resources not used in login tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in login tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used by login tests")
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

    // ── static password source ────────────────────────────────────────────────

    struct StaticPasswordSource(String);

    impl PasswordSource for StaticPasswordSource {
        fn read(&self, _prompt: &str) -> Result<String, Diagnostic> {
            Ok(self.0.clone())
        }
    }

    // ── recording renderer ────────────────────────────────────────────────────

    struct RecordingRenderer {
        login_outcome: Option<AuthLoginOutcome>,
        login_auth_state: Option<String>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                login_outcome: None,
                login_auth_state: None,
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
            outcome: &AuthLoginOutcome,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.login_outcome = Some(AuthLoginOutcome {
                server: outcome.server.clone(),
                user: outcome.user.clone(),
                expires_at: outcome.expires_at,
            });
            self.login_auth_state = Some(meta.auth_state.clone());
            Ok(())
        }

        fn auth_status(
            &mut self,
            _outcome: &AuthStatusOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
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
            unimplemented!("resource_type_describe not used in login tests")
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in login tests")
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

    fn fixed_expires() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 25, 12, 34, 56).unwrap()
    }

    fn make_args(server: &str) -> (LoginArgs, Config) {
        let args = LoginArgs {
            server: Some(server.to_string()),
            user: Some("u@x.test".to_string()),
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

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn happy_path_stores_entry_in_cache() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::ok("tok-abc", "u@x.test", Some(fixed_expires()));
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("hunter2".to_string());

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token("https://api.test.dasch.swiss"),
            Some("tok-abc")
        );
        assert_eq!(
            loaded.user("https://api.test.dasch.swiss"),
            Some("u@x.test")
        );
        assert_eq!(
            loaded.expires_at("https://api.test.dasch.swiss"),
            Some(fixed_expires())
        );
        assert!(
            loaded.acquired_at("https://api.test.dasch.swiss").is_some(),
            "acquired_at should be set to Some(Utc::now()) after login"
        );
    }

    #[test]
    fn happy_path_renderer_receives_correct_outcome() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::ok("tok-abc", "u@x.test", Some(fixed_expires()));
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("hunter2".to_string());

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap();

        let outcome = renderer.login_outcome.unwrap();
        assert_eq!(outcome.server, "https://api.test.dasch.swiss");
        assert_eq!(outcome.user, "u@x.test");
        assert_eq!(outcome.expires_at, Some(fixed_expires()));
        // _meta.auth must reflect the post-login state using ADR-0007 vocabulary.
        assert_eq!(
            renderer.login_auth_state.as_deref(),
            Some("authenticated as u@x.test")
        );
    }

    #[test]
    fn error_auth_required_propagates_unchanged() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::err(Diagnostic::AuthRequired(
            "Authentication failed on https://api.test.dasch.swiss".into(),
        ));
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("bad-pw".to_string());

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap_err();
        assert!(
            matches!(err, Diagnostic::AuthRequired(_)),
            "expected AuthRequired, got {err:?}"
        );
        // Must not include the username (ADR-0007 / PRD acceptance criterion 7).
        assert!(
            !err.to_string().contains("u@x.test"),
            "error message must not contain the username; got: {err}"
        );
    }

    #[test]
    fn error_network_propagates_unchanged() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::err(Diagnostic::Network("connection refused".into()));
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("pw".to_string());

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap_err();
        assert!(
            matches!(err, Diagnostic::Network(_)),
            "expected Network, got {err:?}"
        );
    }

    #[test]
    fn error_server_error_propagates_unchanged() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::err(Diagnostic::ServerError("server returned 500".into()));
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("pw".to_string());

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap_err();
        assert!(
            matches!(err, Diagnostic::ServerError(_)),
            "expected ServerError, got {err:?}"
        );
    }

    #[test]
    fn static_password_source_reaches_client() {
        // Verifies that the PasswordSource indirection works end-to-end:
        // a mock client that always succeeds combined with a static password
        // source must complete without error and store the expected entry.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");
        let client = MockDspClient::ok("tok-xyz", "u@x.test", None);
        let mut renderer = RecordingRenderer::new();
        let pw = StaticPasswordSource("hunter2".to_string());

        // Should complete without touching the real TTY.
        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &pw,
            None,
            Some(&cache_path),
        )
        .unwrap();

        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(
            loaded.token("https://api.test.dasch.swiss"),
            Some("tok-xyz")
        );
    }

    #[test]
    fn resolve_password_prefers_nonempty_env_value() {
        let src = StaticPasswordSource("from-prompt".to_string());
        let pw = resolve_password(Some("from-env".to_string()), &src).unwrap();
        assert_eq!(
            pw, "from-env",
            "non-empty DSP_PASSWORD must win over the prompt"
        );
    }

    #[test]
    fn resolve_password_ignores_empty_env_value() {
        let src = StaticPasswordSource("from-prompt".to_string());
        let pw = resolve_password(Some(String::new()), &src).unwrap();
        assert_eq!(
            pw, "from-prompt",
            "an empty DSP_PASSWORD must fall through to the prompt"
        );
    }

    #[test]
    fn resolve_password_falls_through_when_env_absent() {
        let src = StaticPasswordSource("from-prompt".to_string());
        let pw = resolve_password(None, &src).unwrap();
        assert_eq!(pw, "from-prompt");
    }
}
