//! Actions for `dsp vre data-model { list | describe | structure }`.
//!
//! Phase 5 implements `list` (the first data-model read command) establishing
//! the per-noun `Renderer::data_models` + `DataModelListView` view-struct pattern.
//! See ADR-0008.

use std::path::Path;

use crate::cli::{DataModelDescribeArgs, DataModelListArgs, DataModelStructureArgs};
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::render::{DataModelListView, MetaContext, Renderer};

use crate::actions::auth_state::read_auth_state;

/// List all data-models in a project.
///
/// Authentication is optional (public endpoint per ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per ADR-0007), and delegates all
/// work to `run_list_impl` with injectable seams for deterministic testing.
pub fn list(
    args: &DataModelListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_list_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `list` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `list` entry
///   point before calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in
///   production to use the default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. This differs deliberately from
/// `dump`, which requires auth and propagates errors. For a public endpoint, a
/// corrupt or missing `auth.toml` must still list anonymously (PRD AC 2).
fn run_list_impl(
    args: &DataModelListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args.project.as_deref().ok_or_else(|| {
        Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string())
    })?;

    // ── 2. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "auth cache load failed; falling back to anonymous for data-model list"
            );
            AuthCache::default()
        }
    };

    // ── 3. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 4. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 5. Resolve project (public endpoint; no token needed) ─────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 6. Fetch project's own data-models ───────────────────────────────────
    let mut items = client.list_data_models(&cfg.server, &pref.iri, token)?;

    // ── 7. Append builtins if requested ──────────────────────────────────────
    if args.include_builtins {
        items.extend(crate::client::builtin_data_models());
    }

    // ── 8. Capture total AFTER builtins, BEFORE filter ───────────────────────
    let total = items.len();

    // ── 9. Apply --filter (case-insensitive substring over name + label) ──────
    if let Some(ref f) = args.filter {
        let lower = f.to_lowercase();
        items.retain(|dm| {
            dm.name.to_lowercase().contains(&lower)
                || dm
                    .label
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&lower)
        });
    }

    // ── 10. Sort surviving items by name ascending ────────────────────────────
    items.sort_by(|a, b| a.name.cmp(&b.name));

    // ── 11. Build view + meta and render ─────────────────────────────────────
    let view = DataModelListView {
        items,
        total,
        filter: args.filter.clone(),
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.data_models(&view, &meta)
}

/// Describe a single data-model.
///
/// Authentication is optional (public endpoint per ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per ADR-0007), and delegates all
/// work to `run_describe_impl` with injectable seams for deterministic testing.
pub fn describe(
    args: &DataModelDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_describe_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `describe` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `describe` entry
///   point before calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in
///   production to use the default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. The data-model endpoint is
/// public (ADR-0007), so a corrupt or missing `auth.toml` must still describe
/// anonymously.
fn run_describe_impl(
    args: &DataModelDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args.project.as_deref().ok_or_else(|| {
        Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string())
    })?;

    // ── 2. --data-model required (fail-fast, BEFORE any cache/IO) ────────────
    let data_model = args
        .data_model
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--data-model <name-or-IRI> is required".to_string()))?;

    // ── 3. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "auth cache load failed; falling back to anonymous for data-model describe"
            );
            AuthCache::default()
        }
    };

    // ── 4. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 5. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 6. Resolve project ────────────────────────────────────────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 7. Fetch project's own data-models ───────────────────────────────────
    let data_models = client.list_data_models(&cfg.server, &pref.iri, token)?;

    // ── 8. Match --data-model against the project's data-models ──────────────
    // Truncation is for display only — never truncate before the equality check.
    let dm_iri = data_models
        .iter()
        .find(|dm| {
            dm.iri == data_model || dm.name.eq_ignore_ascii_case(data_model)
        })
        .map(|dm| dm.iri.clone())
        .ok_or_else(|| {
            let dm_disp: String = data_model.chars().take(80).collect();
            let dm_suffix = if data_model.chars().count() > 80 { "…" } else { "" };
            let proj_disp: String = project.chars().take(80).collect();
            let proj_suffix = if project.chars().count() > 80 { "…" } else { "" };
            Diagnostic::NotFound(format!(
                "data-model '{dm_disp}{dm_suffix}' not found in project '{proj_disp}{proj_suffix}' on {server}. \
                 Run `dsp vre data-model list --project {proj_disp}{proj_suffix} --server {server}` \
                 to see available data-models.",
                server = cfg.server,
            ))
        })?;

    // ── 9. Fetch the data-model detail ────────────────────────────────────────
    let detail = client.describe_data_model(&cfg.server, &dm_iri, token)?;

    // ── 10. Build meta and render ─────────────────────────────────────────────
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.data_model_describe(&detail, &meta)
}

/// Show the structure of a single data-model (links + inheritance relations).
///
/// Authentication is optional (public endpoint per ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per ADR-0007), and delegates all
/// work to `run_structure_impl` with injectable seams for deterministic testing.
pub fn structure(
    args: &DataModelStructureArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_structure_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `structure` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `structure` entry
///   point before calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in
///   production to use the default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. The data-model endpoint is
/// public (ADR-0007), so a corrupt or missing `auth.toml` must still show structure
/// anonymously.
fn run_structure_impl(
    args: &DataModelStructureArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args.project.as_deref().ok_or_else(|| {
        Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string())
    })?;

    // ── 2. --data-model required (fail-fast, BEFORE any cache/IO) ────────────
    let data_model = args
        .data_model
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--data-model <name-or-IRI> is required".to_string()))?;

    // ── 3. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "auth cache load failed; falling back to anonymous for data-model structure"
            );
            AuthCache::default()
        }
    };

    // ── 4. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 5. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 6. Resolve project ────────────────────────────────────────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 7. Fetch project's own data-models ───────────────────────────────────
    let data_models = client.list_data_models(&cfg.server, &pref.iri, token)?;

    // ── 8. Match --data-model against the project's data-models ──────────────
    let dm_iri = data_models
        .iter()
        .find(|dm| {
            dm.iri == data_model || dm.name.eq_ignore_ascii_case(data_model)
        })
        .map(|dm| dm.iri.clone())
        .ok_or_else(|| {
            let dm_disp: String = data_model.chars().take(80).collect();
            let dm_suffix = if data_model.chars().count() > 80 { "…" } else { "" };
            let proj_disp: String = project.chars().take(80).collect();
            let proj_suffix = if project.chars().count() > 80 { "…" } else { "" };
            Diagnostic::NotFound(format!(
                "data-model '{dm_disp}{dm_suffix}' not found in project '{proj_disp}{proj_suffix}' on {server}. \
                 Run `dsp vre data-model list --project {proj_disp}{proj_suffix} --server {server}` \
                 to see available data-models.",
                server = cfg.server,
            ))
        })?;

    // ── 9. Fetch the data-model structure ─────────────────────────────────────
    let mut structure = client.data_model_structure(&cfg.server, &dm_iri, token)?;

    // ── 10. Filter built-in relations unless --include-builtins ───────────────
    if !args.include_builtins {
        structure.relations.retain(|r| !r.is_builtin);
    }

    // ── 11. Build meta and render ─────────────────────────────────────────────
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.data_model_structure(&structure, &meta)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use tempfile::TempDir;

    use super::{run_describe_impl, run_list_impl, run_structure_impl};
    use crate::cli::{
        DataModelDescribeArgs, DataModelListArgs, DataModelStructureArgs, FormatArgs,
    };
    use crate::client::DspClient;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::ProjectDetail;
    use crate::model::{
        DataModel, DataModelDetail, DataModelStructure, ProjectRef, ResourceTypeSummary,
    };
    use crate::render::Format;
    use crate::render::auth::{
        AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome,
    };
    use crate::render::{
        DataModelListView, DumpDeleteOutcome, DumpOutcome, MetaContext, ProjectListView, Renderer,
    };

    // ── MockDspClient ─────────────────────────────────────────────────────────

    struct MockDspClient {
        resolve_result: Option<Result<ProjectRef, Diagnostic>>,
        resolve_calls: RefCell<u32>,
        /// Records the project_iri passed to list_data_models.
        list_data_models_result: Option<Result<Vec<DataModel>, Diagnostic>>,
        list_data_models_calls: RefCell<u32>,
        list_data_models_iri: RefCell<Option<String>>,
        list_data_models_token: RefCell<Option<Option<String>>>,
        /// Canned result returned by describe_data_model.
        describe_data_model_result: Option<Result<DataModelDetail, Diagnostic>>,
        /// Records the data_model_iri passed to describe_data_model.
        describe_data_model_iri: RefCell<Option<String>>,
        describe_data_model_calls: RefCell<u32>,
        /// Canned result returned by data_model_structure (Step 6 action tests).
        data_model_structure_result: Option<Result<DataModelStructure, Diagnostic>>,
        /// Records the data_model_iri passed to data_model_structure.
        data_model_structure_iri: RefCell<Option<String>>,
        /// Records the token passed to data_model_structure.
        data_model_structure_token: RefCell<Option<Option<String>>>,
        data_model_structure_calls: RefCell<u32>,
    }

    impl MockDspClient {
        fn new() -> Self {
            Self {
                resolve_result: None,
                resolve_calls: RefCell::new(0),
                list_data_models_result: None,
                list_data_models_calls: RefCell::new(0),
                list_data_models_iri: RefCell::new(None),
                list_data_models_token: RefCell::new(None),
                describe_data_model_result: None,
                describe_data_model_iri: RefCell::new(None),
                describe_data_model_calls: RefCell::new(0),
                data_model_structure_result: None,
                data_model_structure_iri: RefCell::new(None),
                data_model_structure_token: RefCell::new(None),
                data_model_structure_calls: RefCell::new(0),
            }
        }

        fn with_resolve_project(mut self, result: Result<ProjectRef, Diagnostic>) -> Self {
            self.resolve_result = Some(result);
            self
        }

        fn with_list_data_models(mut self, result: Result<Vec<DataModel>, Diagnostic>) -> Self {
            self.list_data_models_result = Some(result);
            self
        }

        fn with_describe_data_model(mut self, result: Result<DataModelDetail, Diagnostic>) -> Self {
            self.describe_data_model_result = Some(result);
            self
        }

        fn with_data_model_structure(
            mut self,
            result: Result<DataModelStructure, Diagnostic>,
        ) -> Self {
            self.data_model_structure_result = Some(result);
            self
        }

        fn resolve_calls(&self) -> u32 {
            *self.resolve_calls.borrow()
        }

        fn list_data_models_calls(&self) -> u32 {
            *self.list_data_models_calls.borrow()
        }

        fn list_data_models_iri(&self) -> Option<String> {
            self.list_data_models_iri.borrow().clone()
        }

        fn list_data_models_token(&self) -> Option<Option<String>> {
            self.list_data_models_token.borrow().clone()
        }

        fn describe_data_model_calls(&self) -> u32 {
            *self.describe_data_model_calls.borrow()
        }

        fn describe_data_model_iri(&self) -> Option<String> {
            self.describe_data_model_iri.borrow().clone()
        }

        fn data_model_structure_calls(&self) -> u32 {
            *self.data_model_structure_calls.borrow()
        }

        fn data_model_structure_iri(&self) -> Option<String> {
            self.data_model_structure_iri.borrow().clone()
        }

        fn data_model_structure_token(&self) -> Option<Option<String>> {
            self.data_model_structure_token.borrow().clone()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in data-model action tests")
        }

        fn resolve_project(&self, _server: &str, _project: &str) -> Result<ProjectRef, Diagnostic> {
            *self.resolve_calls.borrow_mut() += 1;
            self.resolve_result
                .clone()
                .expect("resolve_result must be set when resolve_project is called")
        }

        fn create_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _skip_assets: bool,
            _token: &str,
        ) -> Result<crate::model::CreateDumpOutcome, Diagnostic> {
            unimplemented!("create_project_dump not used in data-model action tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used in data-model action tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used in data-model action tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used in data-model action tests")
        }

        fn list_projects(
            &self,
            _server: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Project>, Diagnostic> {
            unimplemented!("list_projects not used in data-model action tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used in data-model action tests")
        }

        fn list_data_models(
            &self,
            _server: &str,
            project_iri: &str,
            token: Option<&str>,
        ) -> Result<Vec<DataModel>, Diagnostic> {
            *self.list_data_models_calls.borrow_mut() += 1;
            *self.list_data_models_iri.borrow_mut() = Some(project_iri.to_string());
            *self.list_data_models_token.borrow_mut() = Some(token.map(str::to_owned));
            self.list_data_models_result
                .clone()
                .expect("list_data_models_result must be set when list_data_models is called")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<DataModelDetail, Diagnostic> {
            *self.describe_data_model_calls.borrow_mut() += 1;
            *self.describe_data_model_iri.borrow_mut() = Some(data_model_iri.to_string());
            self.describe_data_model_result
                .clone()
                .expect("describe_data_model_result must be set when describe_data_model is called")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in data-model action tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            data_model_iri: &str,
            token: Option<&str>,
        ) -> Result<DataModelStructure, Diagnostic> {
            *self.data_model_structure_calls.borrow_mut() += 1;
            *self.data_model_structure_iri.borrow_mut() = Some(data_model_iri.to_string());
            *self.data_model_structure_token.borrow_mut() = Some(token.map(str::to_owned));
            self.data_model_structure_result.clone().unwrap_or_else(|| {
                unimplemented!("data_model_structure not configured in data-model action tests")
            })
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
            unimplemented!("list_resources not used in data-model action tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in data-model action tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used in data-model action tests")
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

    // ── RecordingRenderer ─────────────────────────────────────────────────────

    struct RecordingRenderer {
        data_models_view: Option<DataModelListView>,
        data_models_meta: Option<MetaContext>,
        /// Records the `detail` passed to `data_model_describe` for assertion in Step 4.
        data_model_describe_detail: Option<DataModelDetail>,
        /// Records the `meta` passed to `data_model_describe` for assertion in Step 4.
        data_model_describe_meta: Option<MetaContext>,
        /// Records the `structure` passed to `data_model_structure` for assertion in Step 6.
        data_model_structure_val: Option<crate::model::DataModelStructure>,
        /// Records the `meta` passed to `data_model_structure` for assertion in Step 6.
        data_model_structure_meta: Option<MetaContext>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                data_models_view: None,
                data_models_meta: None,
                data_model_describe_detail: None,
                data_model_describe_meta: None,
                data_model_structure_val: None,
                data_model_structure_meta: None,
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
            _outcome: &DumpOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump_deleted(
            &mut self,
            _outcome: &DumpDeleteOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn projects(
            &mut self,
            _view: &ProjectListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_describe(
            &mut self,
            _project: &ProjectDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_models(
            &mut self,
            view: &DataModelListView,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.data_models_view = Some(view.clone());
            self.data_models_meta = Some(meta.clone());
            Ok(())
        }

        fn data_model_describe(
            &mut self,
            detail: &crate::model::DataModelDetail,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.data_model_describe_detail = Some(detail.clone());
            self.data_model_describe_meta = Some(meta.clone());
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
            unimplemented!("resource_type_describe not used in data-model action tests")
        }

        fn data_model_structure(
            &mut self,
            structure: &crate::model::DataModelStructure,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.data_model_structure_val = Some(structure.clone());
            self.data_model_structure_meta = Some(meta.clone());
            Ok(())
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

    const SERVER: &str = "https://api.test.dasch.swiss";

    fn make_cfg() -> Config {
        Config {
            server: SERVER.to_string(),
        }
    }

    fn make_project_ref() -> ProjectRef {
        ProjectRef {
            iri: "http://rdfh.ch/projects/0801".to_string(),
            shortcode: "0801".to_string(),
            shortname: "beol".to_string(),
        }
    }

    fn make_args(project: Option<&str>) -> DataModelListArgs {
        DataModelListArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            filter: None,
            include_builtins: false,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        }
    }

    fn make_data_model(name: &str, label: Option<&str>) -> DataModel {
        DataModel {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/{name}/v2"),
            label: label.map(str::to_owned),
            last_modified: None,
            is_builtin: false,
        }
    }

    fn cache_with_entry(server: &str, token: &str, user: &str) -> AuthCache {
        let mut cache = AuthCache::default();
        cache.set_entry(
            server,
            ServerEntry {
                token: token.to_string(),
                user: Some(user.to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache
    }

    fn write_cache(dir: &TempDir, cache: &AuthCache) -> std::path::PathBuf {
        let path = dir.path().join("auth.toml");
        cache.save_to(&path).expect("failed to write test cache");
        path
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_list_data_models_success_sorted_view() {
        let project_ref = make_project_ref();
        let items = vec![
            make_data_model("zebra", None),
            make_data_model("apple", Some("Apple data-model")),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(project_ref.clone()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);

        // Mock was called with the resolved IRI
        assert_eq!(client.list_data_models_calls(), 1);
        assert_eq!(
            client.list_data_models_iri().as_deref(),
            Some(project_ref.iri.as_str())
        );
        // No token (anonymous)
        assert_eq!(
            client.list_data_models_token(),
            Some(None),
            "token should be None for anonymous call"
        );

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        // Sorted by name: apple < zebra
        assert_eq!(view.items.len(), 2);
        assert_eq!(view.items[0].name, "apple");
        assert_eq!(view.items[1].name, "zebra");
        assert_eq!(view.total, 2);
        assert!(view.filter.is_none());
    }

    /// This is the canonical test: default (no `--include-builtins`) renders
    /// only project data-models. Every item has `is_builtin == false`; none of
    /// knora-api / standoff / salsah-gui appear.
    #[test]
    fn test_list_data_models_excludes_builtins() {
        let items = vec![
            make_data_model("beol", Some("The BEOL data-model")),
            make_data_model("images", None),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.include_builtins = false;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");

        // total must equal the project data-model count (captured AFTER builtins
        // append, BEFORE filter — with no builtins appended, that means 2).
        assert_eq!(
            view.total, 2,
            "total must equal the number of project data-models (2); \
             a regression capturing total after filtering would break this"
        );

        // All items have is_builtin == false
        for item in &view.items {
            assert!(
                !item.is_builtin,
                "item '{}' has is_builtin == true but should be false",
                item.name
            );
        }

        // None of the built-in names appear
        let names: Vec<&str> = view.items.iter().map(|i| i.name.as_str()).collect();
        assert!(
            !names.contains(&"knora-api"),
            "knora-api must not appear without --include-builtins"
        );
        assert!(
            !names.contains(&"standoff"),
            "standoff must not appear without --include-builtins"
        );
        assert!(
            !names.contains(&"salsah-gui"),
            "salsah-gui must not appear without --include-builtins"
        );
    }

    #[test]
    fn test_list_data_models_include_builtins_appended_and_sorted() {
        let items = vec![
            make_data_model("limc", Some("LIMC")),
            make_data_model("rosetta", None),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.include_builtins = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");

        // total = 2 project + 3 builtins = 5, before filter
        assert_eq!(view.total, 5, "total must include builtins before filter");
        assert_eq!(view.items.len(), 5);

        // Sort order: knora-api, limc, rosetta, salsah-gui, standoff
        let names: Vec<&str> = view.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["knora-api", "limc", "rosetta", "salsah-gui", "standoff"]
        );

        // Builtins have is_builtin == true
        for item in &view.items {
            if matches!(item.name.as_str(), "knora-api" | "standoff" | "salsah-gui") {
                assert!(
                    item.is_builtin,
                    "'{}' should have is_builtin == true",
                    item.name
                );
            } else {
                assert!(
                    !item.is_builtin,
                    "'{}' should have is_builtin == false",
                    item.name
                );
            }
        }
    }

    #[test]
    fn test_list_data_models_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_args(None); // no --project
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --project must yield Diagnostic::Usage, got {:?}",
            result
        );
        // No client calls should be made
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --project is missing"
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when --project is missing"
        );
        // Renderer not called either
        assert!(renderer.data_models_view.is_none());
    }

    #[test]
    fn test_list_data_models_resolve_not_found_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Err(Diagnostic::NotFound("project '9999' not found".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("9999"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "resolve_project NotFound must propagate, got {:?}",
            result
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when resolve_project fails"
        );
    }

    #[test]
    fn test_list_data_models_list_error_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Err(Diagnostic::ServerError("server is broken".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "list_data_models error must propagate, got {:?}",
            result
        );
    }

    #[test]
    fn test_list_data_models_filter_case_insensitive() {
        let items = vec![
            make_data_model("beol", Some("The BEOL data-model")),
            make_data_model("images", Some("Image collection")),
            make_data_model("admin", None),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.filter = Some("beol".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");

        // total = 3 (before filter), shown = 1
        assert_eq!(view.total, 3, "total must be pre-filter count");
        assert_eq!(view.items.len(), 1, "only the matching item should remain");
        assert_eq!(view.items[0].name, "beol");
        assert_eq!(view.filter.as_deref(), Some("beol"));
    }

    #[test]
    fn test_list_data_models_filter_matches_label() {
        let items = vec![
            make_data_model("ont1", Some("Awesome Label")),
            make_data_model("ont2", Some("Other label")),
            make_data_model("ont3", None),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        // "awesome" only matches ont1 via label
        args.filter = Some("AWESOME".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        assert_eq!(view.total, 3);
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].name, "ont1");
    }

    #[test]
    fn test_list_data_models_filter_no_match_yields_empty() {
        let items = vec![make_data_model("beol", None)];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.filter = Some("zzznomatch".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        assert_eq!(view.total, 1, "total is pre-filter");
        assert_eq!(view.items.len(), 0, "no items match the filter");
    }

    #[test]
    fn test_list_data_models_filter_all_pass_still_renders() {
        let items = vec![
            make_data_model("beol", Some("beol label")),
            make_data_model("images", Some("images label")),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        // "label" appears in all items' labels
        args.filter = Some("label".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        // Both items pass — shown == total (the "(m of total matching …)" prose branch)
        assert_eq!(view.total, 2);
        assert_eq!(view.items.len(), 2);
        assert!(view.filter.is_some());
    }

    #[test]
    fn test_list_data_models_filter_matches_only_builtins() {
        let items = vec![
            make_data_model("beol", None),
            make_data_model("images", None),
        ];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.include_builtins = true;
        // "knora" only matches the knora-api builtin name
        args.filter = Some("knora".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        // total = 2 project + 3 builtins = 5 (before filter)
        assert_eq!(view.total, 5, "total must include builtins before filter");
        // Only knora-api survives the filter
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].name, "knora-api");
        assert!(view.items[0].is_builtin);
    }

    #[test]
    fn test_list_data_models_filter_excludes_builtins_total_still_counts_them() {
        let items = vec![make_data_model("beol", None)];

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(items));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"));
        args.include_builtins = true;
        // "beol" only matches the project item, not builtins
        args.filter = Some("beol".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .data_models_view
            .expect("data_models must have been called");
        // total = 1 project + 3 builtins = 4 (before filter), even though builtins are filtered out
        assert_eq!(
            view.total, 4,
            "total must count builtins even when filtered out"
        );
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].name, "beol");
    }

    // ── auth-state seam tests ─────────────────────────────────────────────────

    #[test]
    fn test_list_data_models_auth_anonymous_when_no_token() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![]));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.data_models_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        // Token passed to list_data_models must be None
        assert_eq!(client.list_data_models_token(), Some(None));
    }

    #[test]
    fn test_list_data_models_auth_via_env_token() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![]));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        // Pass env token via seam
        run_list_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        let meta = renderer.data_models_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
        // Token passed to list_data_models must be the env token
        assert_eq!(
            client.list_data_models_token(),
            Some(Some("env-jwt-token".to_string()))
        );
    }

    #[test]
    fn test_list_data_models_auth_via_cache_with_user() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![]));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        run_list_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            None,
            Some(&path),
        )
        .expect("expected Ok");

        let meta = renderer.data_models_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated as user@example.com");
        assert_eq!(
            client.list_data_models_token(),
            Some(Some("cache-token-xyz".to_string()))
        );
    }

    #[test]
    fn test_list_data_models_corrupt_cache_falls_back_to_anonymous() {
        // Point cache_path at a non-existent path to simulate corrupt/missing cache
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![]));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"));
        // Must succeed (no Err) even with missing cache
        let result = run_list_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            None,
            Some(&bad_path),
        );
        assert!(
            result.is_ok(),
            "corrupt/missing cache must not fail: {:?}",
            result
        );

        let meta = renderer.data_models_meta.expect("meta must be present");
        assert_eq!(
            meta.auth_state, "anonymous",
            "corrupt cache + no env token must fall back to anonymous"
        );
    }

    // ── describe helpers ──────────────────────────────────────────────────────

    fn make_describe_args(
        project: Option<&str>,
        data_model: Option<&str>,
    ) -> DataModelDescribeArgs {
        DataModelDescribeArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            data_model: data_model.map(str::to_owned),
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        }
    }

    fn make_data_model_detail(name: &str) -> DataModelDetail {
        DataModelDetail {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/{name}/v2"),
            label: Some(format!("The {name} data-model")),
            last_modified: Some("2024-05-27T12:00:00Z".to_string()),
            resource_types: vec![ResourceTypeSummary {
                name: "Letter".to_string(),
                iri: format!("http://api.test.dasch.swiss/ontology/0801/{name}/v2#Letter"),
                label: Some("Letter".to_string()),
            }],
        }
    }

    // ── describe tests ────────────────────────────────────────────────────────

    #[test]
    fn test_describe_data_model_success() {
        let data_model_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let detail = make_data_model_detail("beol");
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model(
                "beol",
                Some("The BEOL data-model"),
            )]))
            .with_describe_data_model(Ok(detail.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(client.describe_data_model_calls(), 1);
        assert_eq!(
            client.describe_data_model_iri().as_deref(),
            Some(data_model_iri),
            "describe_data_model must be called with the matched IRI"
        );
        let recorded = renderer
            .data_model_describe_detail
            .expect("data_model_describe must have been called");
        assert_eq!(recorded, detail);
    }

    #[test]
    fn test_describe_data_model_match_by_name_case_insensitive() {
        // "BEOL" (uppercase) should match the data-model named "beol"
        let data_model_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("BEOL"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            result.is_ok(),
            "expected Ok for case-insensitive match, got {:?}",
            result
        );
        assert_eq!(
            client.describe_data_model_iri().as_deref(),
            Some(data_model_iri)
        );
    }

    #[test]
    fn test_describe_data_model_match_by_exact_iri() {
        let data_model_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        // Pass the full IRI as --data-model
        let args = make_describe_args(Some("0801"), Some(data_model_iri));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            result.is_ok(),
            "expected Ok for exact IRI match, got {:?}",
            result
        );
        assert_eq!(
            client.describe_data_model_iri().as_deref(),
            Some(data_model_iri)
        );
    }

    #[test]
    fn test_describe_data_model_not_found() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("nonexistent-dm"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "unknown data-model must yield NotFound, got {:?}",
            result
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when the data-model is not found"
        );
    }

    #[test]
    fn test_describe_data_model_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(None, Some("beol")); // no --project
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --project must yield Diagnostic::Usage, got {:?}",
            result
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --project is missing"
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when --project is missing"
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when --project is missing"
        );
    }

    #[test]
    fn test_describe_data_model_missing_data_model_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), None); // no --data-model
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --data-model must yield Diagnostic::Usage, got {:?}",
            result
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --data-model is missing"
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when --data-model is missing"
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when --data-model is missing"
        );
    }

    #[test]
    fn test_describe_data_model_resolve_project_not_found_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Err(Diagnostic::NotFound("project '9999' not found".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("9999"), Some("beol"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "resolve_project NotFound must propagate, got {:?}",
            result
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when resolve_project fails"
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when resolve_project fails"
        );
    }

    #[test]
    fn test_describe_data_model_list_error_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Err(Diagnostic::ServerError("server is broken".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "list_data_models error must propagate, got {:?}",
            result
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when list_data_models fails"
        );
    }

    #[test]
    fn test_describe_data_model_describe_error_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Err(Diagnostic::ServerError("allentities failed".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "describe_data_model error must propagate, got {:?}",
            result
        );
    }

    #[test]
    fn test_describe_data_model_auth_anonymous() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let meta = renderer
            .data_model_describe_meta
            .expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
    }

    #[test]
    fn test_describe_data_model_auth_via_env_token() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        let meta = renderer
            .data_model_describe_meta
            .expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
    }

    #[test]
    fn test_describe_data_model_auth_via_cache_with_user() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            None,
            Some(&path),
        )
        .expect("expected Ok");

        let meta = renderer
            .data_model_describe_meta
            .expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated as user@example.com");
    }

    #[test]
    fn test_describe_data_model_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_data_model_detail("beol")));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"));
        let result = run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            None,
            Some(&bad_path),
        );
        assert!(
            result.is_ok(),
            "corrupt/missing cache must not fail: {:?}",
            result
        );

        let meta = renderer
            .data_model_describe_meta
            .expect("meta must be present");
        assert_eq!(
            meta.auth_state, "anonymous",
            "corrupt cache + no env token must fall back to anonymous"
        );
    }

    // ── structure helpers ─────────────────────────────────────────────────────

    fn make_structure_args(
        project: Option<&str>,
        data_model: Option<&str>,
        include_builtins: bool,
    ) -> DataModelStructureArgs {
        DataModelStructureArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            data_model: data_model.map(str::to_owned),
            include_builtins,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        }
    }

    fn make_link_relation(
        source: &str,
        field: &str,
        target: &str,
        target_data_model: Option<&str>,
        is_builtin: bool,
    ) -> crate::model::Relation {
        crate::model::Relation {
            source: source.to_string(),
            target: target.to_string(),
            kind: crate::model::RelationKind::Link,
            field: Some(field.to_string()),
            target_data_model: target_data_model.map(str::to_owned),
            is_builtin,
        }
    }

    fn make_inherits_relation(
        source: &str,
        target: &str,
        target_data_model: Option<&str>,
        is_builtin: bool,
    ) -> crate::model::Relation {
        crate::model::Relation {
            source: source.to_string(),
            target: target.to_string(),
            kind: crate::model::RelationKind::Inherits,
            field: None,
            target_data_model: target_data_model.map(str::to_owned),
            is_builtin,
        }
    }

    fn make_structure(
        data_model: &str,
        relations: Vec<crate::model::Relation>,
    ) -> DataModelStructure {
        DataModelStructure {
            data_model: data_model.to_string(),
            relations,
        }
    }

    // ── structure tests ───────────────────────────────────────────────────────

    #[test]
    fn test_structure_link_only() {
        let structure = make_structure(
            "beol",
            vec![
                make_link_relation("letter", "hasSender", "person", None, false),
                make_link_relation("letter", "hasRecipient", "person", None, false),
            ],
        );
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(client.data_model_structure_calls(), 1);
        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert_eq!(recorded.relations.len(), 2);
        assert!(
            recorded
                .relations
                .iter()
                .all(|r| r.kind == crate::model::RelationKind::Link)
        );
    }

    #[test]
    fn test_structure_inherits_only() {
        let structure = make_structure(
            "beol",
            vec![make_inherits_relation(
                "letter",
                "writtenSource",
                None,
                false,
            )],
        );
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert_eq!(recorded.relations.len(), 1);
        assert_eq!(
            recorded.relations[0].kind,
            crate::model::RelationKind::Inherits
        );
    }

    #[test]
    fn test_structure_mixed_link_and_inherits() {
        let structure = make_structure(
            "beol",
            vec![
                make_link_relation("letter", "hasSender", "person", None, false),
                make_inherits_relation("letter", "writtenSource", None, false),
            ],
        );
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert_eq!(recorded.relations.len(), 2);
        let kinds: Vec<_> = recorded.relations.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&crate::model::RelationKind::Link));
        assert!(kinds.contains(&crate::model::RelationKind::Inherits));
    }

    #[test]
    fn test_structure_cross_model_target_tag_carried_through() {
        // A link field pointing to a target in a sibling data-model carries
        // target_data_model = Some("biblio"), which the renderer will tag [to biblio].
        let structure = make_structure(
            "beol",
            vec![make_link_relation(
                "letter",
                "cites",
                "Book",
                Some("biblio"),
                false,
            )],
        );
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert_eq!(recorded.relations.len(), 1);
        assert_eq!(
            recorded.relations[0].target_data_model.as_deref(),
            Some("biblio"),
            "cross-model target_data_model must be preserved through the action"
        );
    }

    #[test]
    fn test_structure_asymmetric_builtin_project_link_to_builtin_target_shown_by_default() {
        // A project-defined link field pointing to a built-in target has is_builtin=false
        // (keyed off the FIELD's prefix). It must NOT be filtered by default.
        let project_link_to_builtin = make_link_relation(
            "letter",
            "hasRelation",
            "Resource",
            None,  // system target → target_data_model None
            false, // is_builtin=false: the FIELD is project-defined
        );
        // An inherits relation to a system super is is_builtin=true, filtered by default.
        let system_inherit = make_inherits_relation(
            "letter", "Resource", None, // system target → target_data_model None
            true, // is_builtin=true: system superclass
        );
        let structure = make_structure("beol", vec![project_link_to_builtin, system_inherit]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false); // no --include-builtins
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        // Only the project link field (is_builtin=false) must survive; the system inherit is dropped.
        assert_eq!(
            recorded.relations.len(),
            1,
            "only the project link (is_builtin=false) should survive default filtering"
        );
        assert_eq!(recorded.relations[0].kind, crate::model::RelationKind::Link);
        assert_eq!(recorded.relations[0].field.as_deref(), Some("hasRelation"));
    }

    #[test]
    fn test_structure_include_builtins_shows_all() {
        // With --include-builtins, both the project link AND the system inherit are shown.
        let project_link_to_builtin =
            make_link_relation("letter", "hasRelation", "Resource", None, false);
        let system_inherit = make_inherits_relation("letter", "Resource", None, true);
        let structure = make_structure("beol", vec![project_link_to_builtin, system_inherit]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), true); // --include-builtins
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert_eq!(
            recorded.relations.len(),
            2,
            "--include-builtins must show all relations including is_builtin=true ones"
        );
    }

    #[test]
    fn test_structure_zero_relations() {
        let structure = make_structure("beol", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            result.is_ok(),
            "zero relations must render OK, got {:?}",
            result
        );
        let recorded = renderer
            .data_model_structure_val
            .expect("data_model_structure must have been called");
        assert!(
            recorded.relations.is_empty(),
            "zero relations must be empty after filtering"
        );
    }

    #[test]
    fn test_structure_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(None, Some("beol"), false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --project must yield Diagnostic::Usage, got {:?}",
            result
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --project is missing"
        );
        assert_eq!(
            client.data_model_structure_calls(),
            0,
            "data_model_structure must not be called when --project is missing"
        );
    }

    #[test]
    fn test_structure_missing_data_model_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), None, false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --data-model must yield Diagnostic::Usage, got {:?}",
            result
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --data-model is missing"
        );
        assert_eq!(
            client.data_model_structure_calls(),
            0,
            "data_model_structure must not be called when --data-model is missing"
        );
    }

    #[test]
    fn test_structure_project_not_found_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Err(Diagnostic::NotFound("project '9999' not found".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("9999"), Some("beol"), false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "resolve_project NotFound must propagate, got {:?}",
            result
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when resolve_project fails"
        );
        assert_eq!(
            client.data_model_structure_calls(),
            0,
            "data_model_structure must not be called when resolve_project fails"
        );
    }

    #[test]
    fn test_structure_data_model_not_found_reframed_to_not_found_diagnostic() {
        // When the data-model name is not in the project's data-model list, the action
        // reframes the miss as a NotFound Diagnostic (not a server error or panic).
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("nonexistent-dm"), false);
        let result = run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "unknown data-model must yield NotFound Diagnostic, got {:?}",
            result
        );
        assert_eq!(
            client.data_model_structure_calls(),
            0,
            "data_model_structure must not be called when the data-model is not found"
        );
    }

    #[test]
    fn test_structure_data_model_iri_forwarded() {
        // Confirm the resolved IRI (not the user-provided name) is passed to data_model_structure.
        let dm_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let structure = make_structure("beol", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        assert_eq!(
            client.data_model_structure_iri().as_deref(),
            Some(dm_iri),
            "data_model_structure must be called with the resolved IRI"
        );
    }

    #[test]
    fn test_structure_token_forwarded_to_client() {
        // The resolved token (from env seam) must be forwarded to data_model_structure.
        let structure = make_structure("beol", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        assert_eq!(
            client.data_model_structure_token(),
            Some(Some("env-jwt-token".to_string())),
            "env token must be forwarded to data_model_structure"
        );
        let meta = renderer
            .data_model_structure_meta
            .expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
    }

    #[test]
    fn test_structure_auth_anonymous_when_no_token() {
        let structure = make_structure("beol", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let meta = renderer
            .data_model_structure_meta
            .expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        assert_eq!(client.data_model_structure_token(), Some(None));
    }

    #[test]
    fn test_structure_auth_via_cache_with_user() {
        // Parallel to test_describe_data_model_auth_via_cache_with_user and
        // test_list_data_models_auth_via_cache_with_user: the cached token must be
        // resolved and forwarded to data_model_structure, and the auth-state disclosure
        // must reflect the cached user identity.
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let structure = make_structure("beol", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_data_model_structure(Ok(structure));

        let mut renderer = RecordingRenderer::new();
        let args = make_structure_args(Some("0801"), Some("beol"), false);
        run_structure_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            None,
            Some(&path),
        )
        .expect("expected Ok");

        let meta = renderer
            .data_model_structure_meta
            .expect("meta must be present");
        assert_eq!(
            meta.auth_state, "authenticated as user@example.com",
            "cached-token path must produce 'authenticated as <user>' auth state"
        );
        assert_eq!(
            client.data_model_structure_token(),
            Some(Some("cache-token-xyz".to_string())),
            "cached token must be forwarded to data_model_structure"
        );
    }
}
