//! Actions for `dsp auth logout`.
//!
//! Removes the cached token for a server. If no token is cached, reports that
//! gracefully without error — logout is idempotent.

use std::path::Path;

use crate::cli::LogoutArgs;
use crate::config::{AuthCache, Config};
use crate::diagnostic::Diagnostic;
use crate::render::auth::AuthLogoutOutcome;
use crate::render::{MetaContext, Renderer};

/// Log out from a DSP server.
///
/// Loads the auth cache, removes the server entry if present, persists the
/// change, and renders the outcome. Always returns `Ok(())`.
pub fn run(args: &LogoutArgs, cfg: &Config, renderer: &mut dyn Renderer) -> Result<(), Diagnostic> {
    run_impl(args, cfg, renderer, None)
}

fn run_impl(
    _args: &LogoutArgs,
    cfg: &Config,
    renderer: &mut dyn Renderer,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    let mut cache = match cache_path {
        Some(p) => AuthCache::load_from(p)?,
        None => AuthCache::load()?,
    };

    let was_cached = cache.remove(&cfg.server);

    // Only write to disk if there was actually something to remove —
    // avoid creating a cache file where none existed before.
    if was_cached {
        match cache_path {
            Some(p) => cache.save_to(p)?,
            None => cache.save()?,
        }
    }

    // After logout no token remains → "anonymous" per dsp-cli/ADR-0007 vocabulary.
    // Route through the shared helper for consistency with every other command;
    // `cache` has had this server's entry removed above, so it resolves to
    // "anonymous" regardless.
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state: crate::actions::auth_state::read_auth_state(None, &cache, &cfg.server),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };

    let outcome = AuthLogoutOutcome { server: cfg.server.clone(), was_cached };

    renderer.auth_logout(&outcome, &meta)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::run_impl;
    use crate::cli::{FormatArgs, LogoutArgs};
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
    use crate::render::{Format, MetaContext, Renderer};

    // ── recording renderer ────────────────────────────────────────────────────

    struct RecordingRenderer {
        logout_outcome: Option<(String, bool)>,
        logout_auth_state: Option<String>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self { logout_outcome: None, logout_auth_state: None }
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

        fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.logout_outcome = Some((outcome.server.clone(), outcome.was_cached));
            self.logout_auth_state = Some(meta.auth_state.clone());
            Ok(())
        }

        fn auth_set_token(&mut self, _outcome: &AuthSetTokenOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
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
            unimplemented!("resource_type_describe not used in logout tests")
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in logout tests")
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

    fn make_args(server: &str) -> (LogoutArgs, Config) {
        let args = LogoutArgs {
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
        let cfg = Config { server: server.to_string() };
        (args, cfg)
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn logout_removes_entry_and_reports_was_cached() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        // Pre-populate cache.
        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "tok-abc".to_string(),
                user: Some("u@x.test".to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path)).unwrap();

        // Renderer must report was_cached = true.
        let (server, was_cached) = renderer.logout_outcome.unwrap();
        assert_eq!(server, "https://api.test.dasch.swiss");
        assert!(was_cached, "expected was_cached=true when entry existed");

        // Entry must actually be gone.
        let loaded = AuthCache::load_from(&cache_path).unwrap();
        assert_eq!(loaded.token("https://api.test.dasch.swiss"), None);
    }

    #[test]
    fn logout_on_empty_cache_reports_was_cached_false() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path)).unwrap();

        let (_, was_cached) = renderer.logout_outcome.unwrap();
        assert!(!was_cached, "expected was_cached=false when cache was empty");
    }

    #[test]
    fn logout_on_empty_cache_does_not_create_cache_file() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path)).unwrap();

        assert!(!cache_path.exists(), "logout on empty cache must not create the cache file");
    }

    #[test]
    fn run_always_returns_ok() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        let result = run_impl(&args, &cfg, &mut renderer, Some(&cache_path));
        assert!(result.is_ok(), "logout should always return Ok; got {result:?}");
    }

    #[test]
    fn logout_meta_auth_is_anonymous() {
        // Guards the dsp-cli/ADR-0007 harmonization: after logout, _meta.auth must be
        // "anonymous" (no token present), not the old "not_logged_in" string.
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args("https://api.test.dasch.swiss");

        let mut renderer = RecordingRenderer::new();
        run_impl(&args, &cfg, &mut renderer, Some(&cache_path)).unwrap();

        assert_eq!(
            renderer.logout_auth_state.as_deref(),
            Some("anonymous"),
            "_meta.auth after logout must be 'anonymous' (dsp-cli/ADR-0007 vocabulary)"
        );
    }
}
