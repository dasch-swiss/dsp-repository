//! Actions for `dsp vre resource-type { list | describe }`.
//!
//! Phase 5 implements `list` (the first resource-type read command), mirroring
//! `data-model list` (013) and `data-model describe` (014). The action resolves
//! the data-model via `describe_data_model` (no new `DspClient` method) and maps
//! `ResourceTypeSummary` → `ResourceType { is_builtin: false }`, optionally
//! appending built-ins when `--include-builtins` is set. See dsp-cli/ADR-0008.

use std::path::Path;

use crate::actions::auth_state::read_auth_state;
use crate::cli::{ResourceTypeDescribeArgs, ResourceTypeListArgs};
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::model::ResourceType;
use crate::render::{MetaContext, Renderer, ResourceTypeListView};

/// Disclosure note for `--count` (plan 030) — schema-side, distinct from the
/// instance-side `MetaContext.filter_warning` (dsp-cli/ADR-0007). Emitted via
/// `MetaContext.count_caveat` whenever `--count` is passed on `resource-type
/// list`/`describe`.
const COUNT_CAVEAT: &str = "counts include resources you may not be permitted to see and exclude deleted resources.";

/// List all resource-types in a data-model.
///
/// Authentication is optional (public endpoint per dsp-cli/ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per dsp-cli/ADR-0007), and delegates all
/// work to `run_list_impl` with injectable seams for deterministic testing.
pub fn list(
    args: &ResourceTypeListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_list_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `list` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `list` entry point before calling
///   this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in production to use the
///   default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. This differs deliberately from
/// `dump`, which requires auth and propagates errors. For a public endpoint, a
/// corrupt or missing `auth.toml` must still list anonymously.
fn run_list_impl(
    args: &ResourceTypeListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args
        .project
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string()))?;

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
            crate::util::warn_auth_cache_load_failed(&e, "falling back to anonymous for resource-type list");
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

    // ── 7. Fetch project's own data-models (pass token — auth users see private ontologies) ──
    let data_models = client.list_data_models(&cfg.server, &pref.iri, token)?;

    // ── 8. Match --data-model against the project's data-models ──────────────
    // Bind both iri and name from the same find — do not re-scan.
    // Truncation is for display only — never truncate before the equality check.
    let (dm_iri, dm_name) = data_models
        .iter()
        .find(|dm| dm.iri == data_model || dm.name.eq_ignore_ascii_case(data_model))
        .map(|dm| (dm.iri.clone(), dm.name.clone()))
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

    // ── 10. Map ResourceTypeSummary → ResourceType { is_builtin: false } ──────
    // `detail` is owned and only its `resource_types` are needed, so move the
    // fields out rather than cloning.
    let mut items: Vec<ResourceType> = detail
        .resource_types
        .into_iter()
        .map(|rt| ResourceType {
            name: rt.name,
            iri: rt.iri,
            label: rt.label,
            is_builtin: false,
            count: None,
        })
        .collect();

    // ── 11. Append builtins if requested ──────────────────────────────────────
    if args.include_builtins {
        items.extend(crate::client::builtin_resource_types());
    }

    // ── 12. Fetch and merge instance counts if --count ────────────────────────
    if args.count {
        let counts = client.resource_counts(&cfg.server, &pref.iri, token)?;
        for item in &mut items {
            item.count = counts.get(&item.iri).copied();
        }
    }

    // ── 13. Capture total AFTER builtins, BEFORE filter ───────────────────────
    let total = items.len();

    // ── 14. Apply --filter (case-insensitive substring over name + label) ─────
    if let Some(ref f) = args.filter {
        let lower = f.to_lowercase();
        items.retain(|rt| {
            rt.name.to_lowercase().contains(&lower) || rt.label.as_deref().unwrap_or("").to_lowercase().contains(&lower)
        });
    }

    // ── 15. Sort surviving items by name ascending ────────────────────────────
    items.sort_by(|a, b| a.name.cmp(&b.name));

    // ── 16. Build view + meta and render ─────────────────────────────────────
    let view = ResourceTypeListView {
        items,
        total,
        filter: args.filter.clone(),
        data_model: dm_name,
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: if args.count {
            Some(COUNT_CAVEAT.to_string())
        } else {
            None
        },
        count_cost: None,
    };
    renderer.resource_types(&view, &meta)
}

/// Describe a single resource-type, including its fields and value-types.
///
/// Authentication is optional (public endpoint per dsp-cli/ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per dsp-cli/ADR-0007), and delegates all
/// work to `run_describe_impl` with injectable seams for deterministic testing.
pub fn describe(
    args: &ResourceTypeDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_describe_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `describe` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `describe` entry point before
///   calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in production to use the
///   default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. This differs deliberately from
/// `dump`, which requires auth and propagates errors. For a public endpoint, a
/// corrupt or missing `auth.toml` must still describe anonymously.
fn run_describe_impl(
    args: &ResourceTypeDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&std::path::Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args
        .project
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string()))?;

    // ── 2. --data-model required (fail-fast, BEFORE any cache/IO) ────────────
    let data_model = args
        .data_model
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--data-model <name-or-IRI> is required".to_string()))?;

    // ── 3. --resource-type required (fail-fast, BEFORE any cache/IO) ─────────
    let resource_type = args
        .resource_type
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--resource-type <name-or-IRI> is required".to_string()))?;

    // ── 4. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            crate::util::warn_auth_cache_load_failed(&e, "falling back to anonymous for resource-type describe");
            AuthCache::default()
        }
    };

    // ── 5. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 6. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 7. Resolve project ────────────────────────────────────────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 8. Fetch project's own data-models (pass token — auth users see private ontologies) ──
    let data_models = client.list_data_models(&cfg.server, &pref.iri, token)?;

    // ── 9. Match --data-model against the project's data-models ──────────────
    // Bind both iri and name from the same find — do not re-scan.
    // Truncation is for display only — never truncate before the equality check.
    let (dm_iri, dm_name) = data_models
        .iter()
        .find(|dm| dm.iri == data_model || dm.name.eq_ignore_ascii_case(data_model))
        .map(|dm| (dm.iri.clone(), dm.name.clone()))
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

    // ── 10. Fetch the resource-type detail ────────────────────────────────────
    // On NotFound, re-frame with a resource-type list hint (keeps hint text in
    // CLI vocab — dsp-cli/ADR-0001; the client only knows it didn't find the class).
    let detail = client
        .describe_resource_type(&cfg.server, &dm_iri, resource_type, token)
        .map_err(|e| match e {
            Diagnostic::NotFound(_) => {
                let rt_disp: String = resource_type.chars().take(80).collect();
                let rt_suffix = if resource_type.chars().count() > 80 { "…" } else { "" };
                let proj_disp: String = project.chars().take(80).collect();
                let proj_suffix = if project.chars().count() > 80 { "…" } else { "" };
                Diagnostic::NotFound(format!(
                    "resource-type '{rt_disp}{rt_suffix}' not found in data-model '{dm_name}' of project '{proj_disp}{proj_suffix}' on {server}. \
                     Run `dsp vre resource-type list --project {proj_disp}{proj_suffix} --data-model {dm_name} --server {server}` \
                     to see available resource-types.",
                    server = cfg.server,
                ))
            }
            other => other,
        })?;

    // ── 11. Filter built-in fields unless --include-builtins ──────────────────
    let mut detail = detail;
    if !args.include_builtins {
        detail.fields.retain(|f| !f.is_builtin);
    }

    // ── 12. Fetch and merge the instance count if --count ─────────────────────
    if args.count {
        let counts = client.resource_counts(&cfg.server, &pref.iri, token)?;
        detail.count = counts.get(&detail.iri).copied();
    }

    // ── 13. Build meta and render ─────────────────────────────────────────────
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: if args.count {
            Some(COUNT_CAVEAT.to_string())
        } else {
            None
        },
        count_cost: None,
    };
    renderer.resource_type_describe(&detail, &meta)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::{run_describe_impl, run_list_impl};
    use crate::cli::{FormatArgs, ResourceTypeDescribeArgs, ResourceTypeListArgs};
    use crate::client::DspClient;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::{
        Cardinality, DataModel, DataModelDetail, Field, ProjectRef, ResourceTypeDetail, ResourceTypeSummary, ValueType,
    };
    use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
    use crate::render::{
        DataModelListView, DumpDeleteOutcome, DumpOutcome, Format, MetaContext, ProjectListView, Renderer,
        ResourceTypeListView,
    };

    // ── MockDspClient ─────────────────────────────────────────────────────────

    struct MockDspClient {
        resolve_result: Option<Result<ProjectRef, Diagnostic>>,
        resolve_calls: RefCell<u32>,
        list_data_models_result: Option<Result<Vec<DataModel>, Diagnostic>>,
        list_data_models_calls: RefCell<u32>,
        list_data_models_iri: RefCell<Option<String>>,
        list_data_models_token: RefCell<Option<Option<String>>>,
        describe_data_model_result: Option<Result<DataModelDetail, Diagnostic>>,
        describe_data_model_calls: RefCell<u32>,
        describe_data_model_iri: RefCell<Option<String>>,
        describe_data_model_token: RefCell<Option<Option<String>>>,
        // Fields for describe_resource_type
        describe_resource_type_result: Option<Result<crate::model::ResourceTypeDetail, Diagnostic>>,
        describe_resource_type_calls: RefCell<u32>,
        describe_resource_type_iri: RefCell<Option<String>>,
        describe_resource_type_resource_type: RefCell<Option<String>>,
        describe_resource_type_token: RefCell<Option<Option<String>>>,
        resource_counts_result: Option<Result<HashMap<String, u64>, Diagnostic>>,
        resource_counts_calls: RefCell<u32>,
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
                describe_data_model_calls: RefCell::new(0),
                describe_data_model_iri: RefCell::new(None),
                describe_data_model_token: RefCell::new(None),
                describe_resource_type_result: None,
                describe_resource_type_calls: RefCell::new(0),
                describe_resource_type_iri: RefCell::new(None),
                describe_resource_type_resource_type: RefCell::new(None),
                describe_resource_type_token: RefCell::new(None),
                resource_counts_result: None,
                resource_counts_calls: RefCell::new(0),
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

        /// Set the result returned by [`DspClient::describe_resource_type`].
        fn with_describe_resource_type(mut self, result: Result<crate::model::ResourceTypeDetail, Diagnostic>) -> Self {
            self.describe_resource_type_result = Some(result);
            self
        }

        /// Set the result returned by [`DspClient::resource_counts`].
        fn with_resource_counts(mut self, result: Result<HashMap<String, u64>, Diagnostic>) -> Self {
            self.resource_counts_result = Some(result);
            self
        }

        fn resolve_calls(&self) -> u32 {
            *self.resolve_calls.borrow()
        }

        fn list_data_models_calls(&self) -> u32 {
            *self.list_data_models_calls.borrow()
        }

        fn list_data_models_token(&self) -> Option<Option<String>> {
            self.list_data_models_token.borrow().clone()
        }

        fn describe_data_model_calls(&self) -> u32 {
            *self.describe_data_model_calls.borrow()
        }

        fn describe_data_model_token(&self) -> Option<Option<String>> {
            self.describe_data_model_token.borrow().clone()
        }

        fn describe_resource_type_calls(&self) -> u32 {
            *self.describe_resource_type_calls.borrow()
        }

        fn describe_resource_type_iri(&self) -> Option<String> {
            self.describe_resource_type_iri.borrow().clone()
        }

        fn describe_resource_type_token(&self) -> Option<Option<String>> {
            self.describe_resource_type_token.borrow().clone()
        }

        fn resource_counts_calls(&self) -> u32 {
            *self.resource_counts_calls.borrow()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in resource-type action tests")
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
            unimplemented!("create_project_dump not used in resource-type action tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used in resource-type action tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used in resource-type action tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used in resource-type action tests")
        }

        fn list_projects(&self, _server: &str, _token: Option<&str>) -> Result<Vec<crate::model::Project>, Diagnostic> {
            unimplemented!("list_projects not used in resource-type action tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used in resource-type action tests")
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
            token: Option<&str>,
        ) -> Result<DataModelDetail, Diagnostic> {
            *self.describe_data_model_calls.borrow_mut() += 1;
            *self.describe_data_model_iri.borrow_mut() = Some(data_model_iri.to_string());
            *self.describe_data_model_token.borrow_mut() = Some(token.map(str::to_owned));
            self.describe_data_model_result
                .clone()
                .expect("describe_data_model_result must be set when describe_data_model is called")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            data_model_iri: &str,
            resource_type: &str,
            token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            *self.describe_resource_type_calls.borrow_mut() += 1;
            *self.describe_resource_type_iri.borrow_mut() = Some(data_model_iri.to_string());
            *self.describe_resource_type_resource_type.borrow_mut() = Some(resource_type.to_string());
            *self.describe_resource_type_token.borrow_mut() = Some(token.map(str::to_owned));
            self.describe_resource_type_result
                .clone()
                .expect("describe_resource_type_result must be set when describe_resource_type is called")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in resource-type action tests")
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
            unimplemented!("list_resources not used in resource-type action tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in resource-type action tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used in resource-type action tests")
        }

        fn resource_counts(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<HashMap<String, u64>, Diagnostic> {
            *self.resource_counts_calls.borrow_mut() += 1;
            self.resource_counts_result
                .clone()
                .expect("resource_counts_result must be set when resource_counts is called")
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
        resource_types_view: Option<ResourceTypeListView>,
        resource_types_meta: Option<MetaContext>,
        resource_type_describe_detail: Option<ResourceTypeDetail>,
        resource_type_describe_meta: Option<MetaContext>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                resource_types_view: None,
                resource_types_meta: None,
                resource_type_describe_detail: None,
                resource_type_describe_meta: None,
            }
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

        fn auth_set_token(&mut self, _outcome: &AuthSetTokenOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump(&mut self, _outcome: &DumpOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump_deleted(
            &mut self,
            _outcome: &DumpDeleteOutcome,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn projects(&mut self, _view: &ProjectListView, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_describe(
            &mut self,
            _project: &crate::model::ProjectDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_models(&mut self, _view: &DataModelListView, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_model_describe(
            &mut self,
            _detail: &crate::model::DataModelDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.resource_types_view = Some(view.clone());
            self.resource_types_meta = Some(meta.clone());
            Ok(())
        }

        fn resource_type_describe(
            &mut self,
            detail: &crate::model::ResourceTypeDetail,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.resource_type_describe_detail = Some(detail.clone());
            self.resource_type_describe_meta = Some(meta.clone());
            Ok(())
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in resource-type action tests")
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
        Config { server: SERVER.to_string() }
    }

    fn make_project_ref() -> ProjectRef {
        ProjectRef {
            iri: "http://rdfh.ch/projects/0801".to_string(),
            shortcode: "0801".to_string(),
            shortname: "beol".to_string(),
        }
    }

    fn make_args(project: Option<&str>, data_model: Option<&str>) -> ResourceTypeListArgs {
        ResourceTypeListArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            data_model: data_model.map(str::to_owned),
            filter: None,
            include_builtins: false,
            count: false,
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

    fn make_detail(name: &str, resource_types: Vec<ResourceTypeSummary>) -> DataModelDetail {
        DataModelDetail {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/{name}/v2"),
            label: Some(format!("The {name} data-model")),
            last_modified: Some("2024-05-27T12:00:00Z".to_string()),
            resource_types,
        }
    }

    fn make_rt_summary(name: &str, label: Option<&str>) -> ResourceTypeSummary {
        ResourceTypeSummary {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/beol/v2#{name}"),
            label: label.map(str::to_owned),
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

    /// Project-only list: resource-types are mapped from ResourceTypeSummary,
    /// all have is_builtin=false, and the result is sorted by name.
    #[test]
    fn test_list_project_only_sorted_by_name() {
        let rts = vec![
            make_rt_summary("Zebra", Some("Zebra class")),
            make_rt_summary("Apple", Some("Apple class")),
        ];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);

        let view = renderer.resource_types_view.expect("resource_types must have been called");
        // Sorted: Apple < Zebra
        assert_eq!(view.items.len(), 2);
        assert_eq!(view.items[0].name, "Apple");
        assert_eq!(view.items[1].name, "Zebra");
        // All non-builtin
        for item in &view.items {
            assert!(!item.is_builtin, "'{}' should be is_builtin=false", item.name);
        }
        assert_eq!(view.total, 2);
        assert!(view.filter.is_none());
        assert_eq!(view.data_model, "beol");
    }

    /// --include-builtins appends exactly the 4 builtins, all have is_builtin=true,
    /// and they are sorted in among project types.
    #[test]
    fn test_list_include_builtins_appended_and_sorted() {
        let rts = vec![make_rt_summary("Manuscript", Some("Manuscript"))];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.include_builtins = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("resource_types must have been called");

        // 1 project + 4 builtins = 5
        assert_eq!(view.total, 5, "total must include builtins before filter");
        assert_eq!(view.items.len(), 5);

        // All 4 builtins present with is_builtin=true
        let builtin_names = ["AudioSegment", "LinkObj", "Region", "VideoSegment"];
        for name in &builtin_names {
            let found = view.items.iter().find(|rt| rt.name == *name);
            assert!(found.is_some(), "builtin '{}' should be present with --include-builtins", name);
            assert!(found.unwrap().is_builtin, "builtin '{}' should have is_builtin=true", name);
        }

        // Project type has is_builtin=false
        let manuscript = view.items.iter().find(|rt| rt.name == "Manuscript");
        assert!(manuscript.is_some());
        assert!(!manuscript.unwrap().is_builtin);

        // Sorted: AudioSegment, LinkObj, Manuscript, Region, VideoSegment
        let names: Vec<&str> = view.items.iter().map(|rt| rt.name.as_str()).collect();
        assert_eq!(names, vec!["AudioSegment", "LinkObj", "Manuscript", "Region", "VideoSegment"]);
    }

    /// --filter narrows items; total reflects pre-filter/post-builtins count.
    #[test]
    fn test_list_filter_narrows_and_total_is_prefilter() {
        let rts = vec![
            make_rt_summary("Letter", Some("A letter")),
            make_rt_summary("Archive", Some("An archive")),
            make_rt_summary("Person", None),
        ];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.include_builtins = true;
        args.filter = Some("let".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("resource_types must have been called");

        // total = 3 project + 4 builtins = 7, before filter
        assert_eq!(view.total, 7, "total must be post-builtins pre-filter");
        // Only "Letter" matches "let"
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].name, "Letter");
        assert_eq!(view.filter.as_deref(), Some("let"));
    }

    /// --project missing → Usage error; no client calls made.
    #[test]
    fn test_list_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_args(None, Some("beol")); // no --project
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

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
        assert!(renderer.resource_types_view.is_none());
    }

    /// --data-model missing → Usage error; no client calls made.
    #[test]
    fn test_list_missing_data_model_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), None); // no --data-model
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

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
        assert!(renderer.resource_types_view.is_none());
    }

    /// data-model not found → NotFound with hint pointing at data-model list.
    #[test]
    fn test_list_data_model_not_found() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("nonexistent-dm"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "unknown data-model must yield NotFound, got {:?}",
            result
        );
        // The hint must mention data-model list
        if let Err(Diagnostic::NotFound(msg)) = &result {
            assert!(
                msg.contains("data-model list"),
                "NotFound hint must mention 'data-model list', got: {msg}"
            );
        }
        // describe must not be called
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must not be called when data-model is not found"
        );
        assert!(renderer.resource_types_view.is_none());
    }

    /// Empty resource_types list: renders with empty items but view.data_model is still populated.
    #[test]
    fn test_list_empty_resource_types_data_model_name_present() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("resource_types must have been called");
        assert!(view.items.is_empty(), "items should be empty");
        assert_eq!(view.total, 0);
        assert_eq!(
            view.data_model, "beol",
            "data_model header name must still be populated when items is empty"
        );
    }

    /// list_data_models returns Err → error propagates; describe_data_model never called.
    #[test]
    fn test_list_data_models_error_propagates_describe_not_called() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Err(Diagnostic::ServerError("server is broken".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "list_data_models error must propagate, got {:?}",
            result
        );
        assert_eq!(
            client.describe_data_model_calls(),
            0,
            "describe_data_model must NOT be called when list_data_models fails"
        );
        assert!(renderer.resource_types_view.is_none());
    }

    // ── auth-state seam tests ─────────────────────────────────────────────────

    /// Anonymous: no env_token + no cache → auth_state is "anonymous".
    #[test]
    fn test_list_auth_anonymous_when_no_token() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_types_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        // Token passed to list_data_models must be None
        assert_eq!(client.list_data_models_token(), Some(None));
    }

    /// DSP_TOKEN set (via env_token seam) → auth_state is "authenticated via DSP_TOKEN".
    #[test]
    fn test_list_auth_via_env_token() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        run_list_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        let meta = renderer.resource_types_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
        assert_eq!(client.list_data_models_token(), Some(Some("env-jwt-token".to_string())));
        // The same token must reach describe_data_model, not just list_data_models.
        assert_eq!(client.describe_data_model_token(), Some(Some("env-jwt-token".to_string())));
    }

    /// Cache token set → auth_state is "authenticated as <user>".
    #[test]
    fn test_list_auth_via_cache_with_user() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        let meta = renderer.resource_types_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated as user@example.com");
        assert_eq!(client.list_data_models_token(), Some(Some("cache-token-xyz".to_string())));
    }

    /// Corrupt/missing cache → falls back to anonymous, never returns Err.
    #[test]
    fn test_list_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&bad_path));
        assert!(result.is_ok(), "corrupt/missing cache must not fail: {:?}", result);

        let meta = renderer.resource_types_meta.expect("meta must be present");
        assert_eq!(
            meta.auth_state, "anonymous",
            "corrupt cache + no env token must fall back to anonymous"
        );
    }

    /// Match data-model by exact IRI (not just name).
    #[test]
    fn test_list_match_data_model_by_iri() {
        let dm_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(dm_iri));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "exact IRI match must succeed, got {:?}", result);
        let view = renderer.resource_types_view.expect("must have been called");
        assert_eq!(view.data_model, "beol");
    }

    /// Match data-model by case-insensitive name.
    #[test]
    fn test_list_match_data_model_case_insensitive() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", vec![])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("BEOL"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "case-insensitive name match must succeed, got {:?}", result);
        let view = renderer.resource_types_view.expect("must have been called");
        assert_eq!(view.data_model, "beol");
    }

    /// Filter matches label (case-insensitive).
    #[test]
    fn test_list_filter_matches_label() {
        let rts = vec![
            make_rt_summary("Letter", Some("Epistolary document")),
            make_rt_summary("Person", Some("A human person")),
        ];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.filter = Some("EPISTOLARY".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("must have been called");
        assert_eq!(view.total, 2, "total is pre-filter");
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].name, "Letter");
    }

    /// ResourceType label from ResourceTypeSummary is preserved in the mapped ResourceType.
    #[test]
    fn test_list_resource_type_label_preserved() {
        let rts = vec![
            make_rt_summary("Letter", Some("A letter type")),
            make_rt_summary("Archive", None),
        ];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("must have been called");
        // Sorted: Archive, Letter
        let archive = view.items.iter().find(|rt| rt.name == "Archive").unwrap();
        assert_eq!(archive.label, None);
        let letter = view.items.iter().find(|rt| rt.name == "Letter").unwrap();
        assert_eq!(letter.label.as_deref(), Some("A letter type"));
    }

    // ── --count tests (plan 030) ──────────────────────────────────────────────

    /// --count merges instance counts onto matching rows by exact IRI.
    #[test]
    fn test_list_count_merges_onto_matching_rows() {
        let rts = vec![
            make_rt_summary("Letter", Some("A letter")),
            make_rt_summary("Archive", Some("An archive")),
        ];
        let letter_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2#Letter".to_string();
        let archive_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2#Archive".to_string();
        let mut counts = HashMap::new();
        counts.insert(letter_iri, 42u64);
        counts.insert(archive_iri, 7u64);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)))
            .with_resource_counts(Ok(counts));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.count = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resource_types_view.expect("must have been called");
        let archive = view.items.iter().find(|rt| rt.name == "Archive").unwrap();
        assert_eq!(archive.count, Some(7));
        let letter = view.items.iter().find(|rt| rt.name == "Letter").unwrap();
        assert_eq!(letter.count, Some(42));
    }

    /// A class absent from the count map gets count=None, not an error.
    #[test]
    fn test_list_count_absent_class_is_none_not_error() {
        let rts = vec![
            make_rt_summary("Letter", Some("A letter")),
            make_rt_summary("Archive", Some("An archive")),
        ];
        let letter_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2#Letter".to_string();
        let mut counts = HashMap::new();
        counts.insert(letter_iri, 42u64);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)))
            .with_resource_counts(Ok(counts));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.count = true;
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "missing class in count map must not error");
        let view = renderer.resource_types_view.expect("must have been called");
        let letter = view.items.iter().find(|rt| rt.name == "Letter").unwrap();
        assert_eq!(letter.count, Some(42));
        let archive = view.items.iter().find(|rt| rt.name == "Archive").unwrap();
        assert_eq!(
            archive.count, None,
            "class absent from the count map must be None, not an error"
        );
    }

    /// --count triggers exactly one resource_counts call.
    #[test]
    fn test_list_count_flag_triggers_one_resource_counts_call() {
        let rts = vec![make_rt_summary("Letter", None)];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)))
            .with_resource_counts(Ok(HashMap::new()));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.count = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.resource_counts_calls(),
            1,
            "resource_counts must be called exactly once when --count is set"
        );
    }

    /// Omitting --count must never call resource_counts (a stray call would
    /// panic via the mock's `.expect(...)`, failing the test loudly).
    #[test]
    fn test_list_no_count_flag_triggers_zero_resource_counts_calls() {
        let rts = vec![make_rt_summary("Letter", None)];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol")); // count: false by default
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.resource_counts_calls(),
            0,
            "resource_counts must not be called when --count is omitted"
        );
        let view = renderer.resource_types_view.expect("must have been called");
        for item in &view.items {
            assert_eq!(item.count, None, "count must be None when --count is omitted");
        }
    }

    /// count_caveat is Some(...) when --count is used.
    #[test]
    fn test_list_count_caveat_present_when_count_true() {
        let rts = vec![make_rt_summary("Letter", None)];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)))
            .with_resource_counts(Ok(HashMap::new()));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_args(Some("0801"), Some("beol"));
        args.count = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_types_meta.expect("meta must be present");
        let caveat = meta.count_caveat.expect("count_caveat must be Some when --count is used");
        assert!(
            caveat.contains("permitted"),
            "count_caveat should disclose permission/visibility limits, got: {caveat}"
        );
    }

    /// count_caveat is None when --count is not used.
    #[test]
    fn test_list_count_caveat_absent_when_count_false() {
        let rts = vec![make_rt_summary("Letter", None)];
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_data_model(Ok(make_detail("beol", rts)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("beol")); // count: false by default
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_types_meta.expect("meta must be present");
        assert_eq!(meta.count_caveat, None, "count_caveat must be None when --count is not used");
    }

    // ── describe helpers ──────────────────────────────────────────────────────

    fn make_describe_args(
        project: Option<&str>,
        data_model: Option<&str>,
        resource_type: Option<&str>,
    ) -> ResourceTypeDescribeArgs {
        ResourceTypeDescribeArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            data_model: data_model.map(str::to_owned),
            resource_type: resource_type.map(str::to_owned),
            include_builtins: false,
            count: false,
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

    fn make_rt_detail(name: &str, fields: Vec<Field>) -> ResourceTypeDetail {
        ResourceTypeDetail {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/beol/v2#{name}"),
            label: Some(format!("{name} type")),
            data_model: "beol".to_string(),
            representation: None,
            super_types: vec![],
            fields,
            count: None,
        }
    }

    fn make_project_field(name: &str, value_type: ValueType) -> Field {
        Field {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/beol/v2#{name}"),
            label: Some(format!("{name} label")),
            value_type,
            link_target: None,
            cardinality: Cardinality::ZeroOrMore,
            is_builtin: false,
            data_model: Some("beol".to_string()),
        }
    }

    fn make_builtin_field(name: &str) -> Field {
        Field {
            name: name.to_string(),
            iri: format!("http://api.knora.org/ontology/knora-api/v2#{name}"),
            label: None,
            value_type: ValueType::Uri,
            link_target: None,
            cardinality: Cardinality::One,
            is_builtin: true,
            data_model: None,
        }
    }

    // ── describe tests ────────────────────────────────────────────────────────

    /// Success path: returns Ok, renderer received the expected detail.
    #[test]
    fn test_describe_success_returns_ok_and_renders_detail() {
        let fields = vec![make_project_field("hasTitle", ValueType::Text)];
        let detail = make_rt_detail("Manuscript", fields.clone());
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        // Built-in filter is applied: no builtins in detail fields, so same count
        assert_eq!(rendered.name, "Manuscript");
        assert_eq!(rendered.fields.len(), 1);
        assert_eq!(rendered.fields[0].name, "hasTitle");
    }

    /// --project missing → Usage error; no client calls made.
    #[test]
    fn test_describe_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(None, Some("beol"), Some("Manuscript")); // no --project
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
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when --project is missing"
        );
        assert!(renderer.resource_type_describe_detail.is_none());
    }

    /// --data-model missing → Usage error; no client calls made.
    #[test]
    fn test_describe_missing_data_model_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), None, Some("Manuscript")); // no --data-model
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
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when --data-model is missing"
        );
        assert!(renderer.resource_type_describe_detail.is_none());
    }

    /// --resource-type missing → Usage error; no client calls made.
    #[test]
    fn test_describe_missing_resource_type_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), None); // no --resource-type
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --resource-type must yield Diagnostic::Usage, got {:?}",
            result
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --resource-type is missing"
        );
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "list_data_models must not be called when --resource-type is missing"
        );
        assert_eq!(
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when --resource-type is missing"
        );
        assert!(renderer.resource_type_describe_detail.is_none());
    }

    /// data-model not found → NotFound, hint mentions data-model list;
    /// describe_resource_type NOT called.
    #[test]
    fn test_describe_data_model_not_found() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("nonexistent-dm"), Some("Manuscript"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "unknown data-model must yield NotFound, got {:?}",
            result
        );
        if let Err(Diagnostic::NotFound(msg)) = &result {
            assert!(
                msg.contains("data-model list"),
                "NotFound hint must mention 'data-model list', got: {msg}"
            );
        }
        assert_eq!(
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when data-model is not found"
        );
        assert!(renderer.resource_type_describe_detail.is_none());
    }

    /// Client returns NotFound for resource-type → action re-frames to mention resource-type list.
    #[test]
    fn test_describe_resource_type_not_found_reframes_to_resource_type_list_hint() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Err(Diagnostic::NotFound("resource type not found".to_string())));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("NonExistentType"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "resource type not found must yield NotFound, got {:?}",
            result
        );
        if let Err(Diagnostic::NotFound(msg)) = &result {
            assert!(
                msg.contains("resource-type list"),
                "re-framed NotFound hint must mention 'resource-type list', got: {msg}"
            );
        }
        assert!(renderer.resource_type_describe_detail.is_none());
    }

    /// --include-builtins=false drops is_builtin fields from rendered detail.
    #[test]
    fn test_describe_include_builtins_false_drops_builtin_fields() {
        let fields = vec![
            make_project_field("hasTitle", ValueType::Text),
            make_builtin_field("arkUrl"),
        ];
        let detail = make_rt_detail("Manuscript", fields);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        args.include_builtins = false;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(
            rendered.fields.len(),
            1,
            "builtin fields must be filtered out when include_builtins=false"
        );
        assert_eq!(rendered.fields[0].name, "hasTitle");
    }

    /// --include-builtins=true keeps is_builtin fields in rendered detail.
    #[test]
    fn test_describe_include_builtins_true_keeps_builtin_fields() {
        let fields = vec![
            make_project_field("hasTitle", ValueType::Text),
            make_builtin_field("arkUrl"),
        ];
        let detail = make_rt_detail("Manuscript", fields);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        args.include_builtins = true;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(
            rendered.fields.len(),
            2,
            "all fields (including builtins) must be present when include_builtins=true"
        );
    }

    /// Link target / representation / super_types survive into the rendered detail.
    #[test]
    fn test_describe_link_target_representation_and_super_types_preserved() {
        use crate::model::Representation;

        let link_field = Field {
            name: "hasAuthor".to_string(),
            iri: "http://api.test.dasch.swiss/ontology/0801/beol/v2#hasAuthor".to_string(),
            label: Some("Author".to_string()),
            value_type: ValueType::Link,
            link_target: Some("person".to_string()),
            cardinality: Cardinality::ZeroOrMore,
            is_builtin: false,
            data_model: Some("beol".to_string()),
        };

        let detail = ResourceTypeDetail {
            name: "letter".to_string(),
            iri: "http://api.test.dasch.swiss/ontology/0801/beol/v2#letter".to_string(),
            label: Some("Letter".to_string()),
            data_model: "beol".to_string(),
            representation: Some(Representation::StillImage),
            super_types: vec!["writtenSource".to_string()],
            fields: vec![link_field],
            count: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("letter"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(rendered.representation, Some(Representation::StillImage));
        assert_eq!(rendered.super_types, vec!["writtenSource"]);
        assert_eq!(rendered.fields[0].link_target.as_deref(), Some("person"));
        assert_eq!(rendered.fields[0].value_type, ValueType::Link);
    }

    /// Cross-DM data_model source tag survives into the rendered detail.
    #[test]
    fn test_describe_cross_dm_field_source_tag_preserved() {
        let cross_dm_field = Field {
            name: "isPartOfCollection".to_string(),
            iri: "http://api.test.dasch.swiss/ontology/0801/biblio/v2#isPartOfCollection".to_string(),
            label: Some("is part of".to_string()),
            value_type: ValueType::Link,
            link_target: Some("Collection".to_string()),
            cardinality: Cardinality::ZeroOrMore,
            is_builtin: false,
            data_model: Some("biblio".to_string()), // cross-DM: biblio, not beol
        };

        let detail = ResourceTypeDetail {
            name: "manuscript".to_string(),
            iri: "http://api.test.dasch.swiss/ontology/0801/beol/v2#manuscript".to_string(),
            label: Some("Manuscript".to_string()),
            data_model: "beol".to_string(),
            representation: None,
            super_types: vec![],
            fields: vec![cross_dm_field],
            count: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("manuscript"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        // The cross-DM field's source tag must survive into the rendered detail
        assert_eq!(rendered.fields[0].data_model.as_deref(), Some("biblio"));
    }

    // ── describe auth-state seam tests ────────────────────────────────────────

    /// Anonymous: no env_token + no cache → auth_state is "anonymous".
    #[test]
    fn test_describe_auth_anonymous_when_no_token() {
        let detail = make_rt_detail("Manuscript", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        // Token passed to all three client calls must be None
        assert_eq!(client.list_data_models_token(), Some(None));
        assert_eq!(client.describe_resource_type_token(), Some(None));
    }

    /// DSP_TOKEN set (via env_token seam) → auth_state is "authenticated via DSP_TOKEN".
    #[test]
    fn test_describe_auth_via_env_token() {
        let detail = make_rt_detail("Manuscript", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
        // Token must be forwarded to the two token-accepting calls.
        // resolve_project is called but takes no token (no assertion needed).
        assert_eq!(client.list_data_models_token(), Some(Some("env-jwt-token".to_string())));
        assert_eq!(client.describe_resource_type_token(), Some(Some("env-jwt-token".to_string())));
    }

    /// Cache token set → auth_state is "authenticated as <user>".
    #[test]
    fn test_describe_auth_via_cache_with_user() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let detail = make_rt_detail("Manuscript", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "authenticated as user@example.com");
        assert_eq!(client.list_data_models_token(), Some(Some("cache-token-xyz".to_string())));
        assert_eq!(client.describe_resource_type_token(), Some(Some("cache-token-xyz".to_string())));
    }

    /// Token is forwarded to the two token-accepting calls: list_data_models AND
    /// describe_resource_type. resolve_project is also called but takes no token
    /// (its signature has no token parameter). The comment "all three client calls"
    /// in earlier drafts was misleading — this test only asserts token-forwarding
    /// on the two calls that accept it.
    #[test]
    fn test_describe_token_forwarded_to_all_three_client_calls() {
        let detail = make_rt_detail("Manuscript", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("my-test-token".to_string()),
            None,
        )
        .expect("expected Ok");

        // resolve_project is called once
        assert_eq!(client.resolve_calls(), 1, "resolve_project must be called");
        // list_data_models receives the token
        assert_eq!(
            client.list_data_models_token(),
            Some(Some("my-test-token".to_string())),
            "list_data_models must receive the token"
        );
        // describe_resource_type receives the token
        assert_eq!(
            client.describe_resource_type_token(),
            Some(Some("my-test-token".to_string())),
            "describe_resource_type must receive the token"
        );
    }

    /// Corrupt/missing cache → falls back to anonymous, never returns Err.
    #[test]
    fn test_describe_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let detail = make_rt_detail("Manuscript", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&bad_path));
        assert!(result.is_ok(), "corrupt/missing cache must not fail: {:?}", result);

        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        assert_eq!(
            meta.auth_state, "anonymous",
            "corrupt cache + no env token must fall back to anonymous"
        );
    }

    /// describe_resource_type IRI matches the resolved data-model IRI.
    #[test]
    fn test_describe_passes_dm_iri_to_describe_resource_type() {
        let detail = make_rt_detail("Manuscript", vec![]);
        let expected_dm_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Manuscript"));
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.describe_resource_type_iri().as_deref(),
            Some(expected_dm_iri),
            "describe_resource_type must be called with the resolved data-model IRI"
        );
    }

    // ── --count tests (plan 030) ──────────────────────────────────────────────

    /// --count sets the described resource-type's count.
    #[test]
    fn test_describe_count_sets_count() {
        let detail = make_rt_detail("Page", vec![]);
        let page_iri = detail.iri.clone();
        let mut counts = HashMap::new();
        counts.insert(page_iri, 123u64);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail))
            .with_resource_counts(Ok(counts));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Page"));
        args.count = true;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(rendered.count, Some(123));
    }

    /// A count map missing the described class's IRI ⇒ None, not an error.
    #[test]
    fn test_describe_count_absent_class_is_none_not_error() {
        let detail = make_rt_detail("Page", vec![]);
        let mut counts = HashMap::new();
        counts.insert(
            "http://api.test.dasch.swiss/ontology/0801/beol/v2#SomeOtherType".to_string(),
            9u64,
        );

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail))
            .with_resource_counts(Ok(counts));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Page"));
        args.count = true;
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "missing class in count map must not error");
        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(
            rendered.count, None,
            "described class absent from the count map must be None, not an error"
        );
    }

    /// --count triggers exactly one resource_counts call.
    #[test]
    fn test_describe_count_flag_triggers_one_resource_counts_call() {
        let detail = make_rt_detail("Page", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail))
            .with_resource_counts(Ok(HashMap::new()));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Page"));
        args.count = true;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.resource_counts_calls(),
            1,
            "resource_counts must be called exactly once when --count is set"
        );
    }

    /// Omitting --count must never call resource_counts (a stray call would
    /// panic via the mock's `.expect(...)`, failing the test loudly); count
    /// stays None and count_caveat stays None.
    #[test]
    fn test_describe_no_count_flag_triggers_zero_resource_counts_calls() {
        let detail = make_rt_detail("Page", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("0801"), Some("beol"), Some("Page")); // count: false
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.resource_counts_calls(),
            0,
            "resource_counts must not be called when --count is omitted"
        );
        let rendered = renderer
            .resource_type_describe_detail
            .expect("resource_type_describe must have been called");
        assert_eq!(rendered.count, None);
        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        assert_eq!(meta.count_caveat, None);
    }

    /// count_caveat is Some(...) when --count is used on describe.
    #[test]
    fn test_describe_count_caveat_present_when_count_true() {
        let detail = make_rt_detail("Page", vec![]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol", None)]))
            .with_describe_resource_type(Ok(detail))
            .with_resource_counts(Ok(HashMap::new()));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("0801"), Some("beol"), Some("Page"));
        args.count = true;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_type_describe_meta.expect("meta must be present");
        let caveat = meta.count_caveat.expect("count_caveat must be Some when --count is used");
        assert!(
            caveat.contains("permitted"),
            "count_caveat should disclose permission/visibility limits, got: {caveat}"
        );
    }
}
