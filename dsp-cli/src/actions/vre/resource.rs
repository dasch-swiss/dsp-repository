//! Actions for `dsp vre resource { list | describe }`.
//!
//! Phase 8a implements `list` — the first instance-side read command. Introduces
//! pagination (`--page` / `--all`) and the first real use of
//! `MetaContext.filter_warning` (dsp-cli/ADR-0007 silent-filter disclosure). Class-IRI
//! resolution follows D1 (plan 022): full-IRI bypass, optional `--data-model`
//! scope, bare-name scan across all project data-models.
//!
//! Phase 8b implements `describe` — fetches a single resource's envelope metadata
//! by its internal IRI. Includes the D2 cross-project guard and D3 filter_warning.

use std::path::Path;

use crate::actions::auth_state::read_auth_state;
use crate::cli::{ResourceDescribeArgs, ResourceListArgs};
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::model::ResourceSummary;
use crate::render::{MetaContext, Renderer, ResourceListPagination, ResourceListView};
use crate::util::text::strip_control_chars;

/// Resolved resource-type reference: IRI, local name, and optionally the
/// data-model IRI (absent on path A when the class IRI has no `#`).
struct ResourceTypeRef {
    iri: String,
    name: String,
    data_model_iri: Option<String>,
}

/// List resource instances of a given type within a project.
///
/// Authentication is optional (instance-side read; anonymous callers see only
/// publicly-visible resources per dsp-cli/ADR-0007). Reads `DSP_TOKEN` from the
/// environment and delegates all work to `run_list_impl` with injectable seams
/// for deterministic testing.
pub fn list(
    args: &ResourceListArgs,
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
/// with a `tracing::warn!` — NEVER returns `Err`. For an instance-side read,
/// a corrupt or missing `auth.toml` must still list anonymously.
pub(crate) fn run_list_impl(
    args: &ResourceListArgs,
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

    // ── 2. --resource-type required (fail-fast, BEFORE any cache/IO) ─────────
    let resource_type_arg = args
        .resource_type
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--resource-type <name-or-IRI> is required".to_string()))?;

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
                "auth cache load failed; falling back to anonymous for resource list"
            );
            AuthCache::default()
        }
    };

    // ── 4. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 5. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 6. D3: filter_warning — always Some for instance-side commands ────────
    //
    // Anonymous callers see only public resources; authenticated callers are
    // still bounded by their permissions. Both warrant disclosure.
    let filter_warning = if resolved.is_none() {
        Some("results may be filtered; login to see private resources".to_string())
    } else {
        Some("results limited to your permissions".to_string())
    };

    // ── 7. Resolve project ────────────────────────────────────────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 8. D1: Resolve resource-type IRI ─────────────────────────────────────
    //
    // Three paths (plan 022 D1):
    //   a. Full IRI (heuristic: contains `://`) → use directly; no scan.
    //   b. `--data-model` given (name or IRI) → scope resolution to one DM.
    //   c. Bare name, no `--data-model` → scan all project data-models.
    let rt_ref = resolve_resource_type_iri(
        resource_type_arg,
        args.data_model.as_deref(),
        &cfg.server,
        &pref.iri,
        client,
        token,
    )?;

    // ── 8b. Resolve --order-by field name to property IRI ────────────────────
    let order_by: Option<String> = if let Some(ob) = args.order_by.as_deref() {
        Some(resolve_order_by_property(ob, &rt_ref, &cfg.server, client, token)?)
    } else {
        None
    };

    // ── 9. D5: Fetch pages ────────────────────────────────────────────────────
    let (all_resources, pagination) = if args.all {
        // Fetch all pages: 0, 1, 2, … until may_have_more is false.
        let mut all: Vec<ResourceSummary> = Vec::new();
        let mut page = 0u32;
        loop {
            let page_result =
                client.list_resources(&cfg.server, &pref.iri, &rt_ref.iri, order_by.as_deref(), page, token)?;
            all.extend(page_result.resources);
            if !page_result.may_have_more_results {
                break;
            }
            page += 1;
        }
        let pages_fetched = page + 1;
        (all, ResourceListPagination::AllPages { pages_fetched })
    } else {
        // Single-page mode: fetch exactly the resolved page (default: 0).
        let page = args.page.unwrap_or(0);
        let page_result =
            client.list_resources(&cfg.server, &pref.iri, &rt_ref.iri, order_by.as_deref(), page, token)?;
        let may_have_more = page_result.may_have_more_results;
        (
            page_result.resources,
            ResourceListPagination::SinglePage { page, may_have_more },
        )
    };

    // ── 10. Capture total BEFORE client-side filter ───────────────────────────
    let total = all_resources.len();

    // ── 11. Apply --filter (case-insensitive substring over label) ────────────
    let mut items = all_resources;
    if let Some(ref f) = args.filter {
        let lower = f.to_lowercase();
        items.retain(|r| r.label.to_lowercase().contains(&lower));
    }

    // ── 12. Build view + meta and render ─────────────────────────────────────
    let view = ResourceListView {
        items,
        total,
        filter: args.filter.clone(),
        resource_type: rt_ref.name,
        pagination,
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning,
        count_caveat: None,
        count_cost: None,
    };
    renderer.resources(&view, &meta)
}

/// Resolve the `resource_type` argument to a [`ResourceTypeRef`].
///
/// Three paths per D1 (plan 022):
/// - Full IRI (contains `://`) → use as-is; extract local name with `local_name`; `data_model_iri`
///   derived by stripping the `#fragment` (or `None` if no `#`).
/// - `--data-model` given → scope to that one data-model.
/// - Bare name, no `--data-model` → scan all project data-models.
fn resolve_resource_type_iri(
    resource_type_arg: &str,
    data_model_arg: Option<&str>,
    server: &str,
    project_iri: &str,
    client: &dyn DspClient,
    token: Option<&str>,
) -> Result<ResourceTypeRef, Diagnostic> {
    // Path A: full IRI (heuristic: contains `://`)
    if resource_type_arg.contains("://") {
        // heuristic: looks like a full IRI → skip the scan
        let name = local_name(resource_type_arg);
        // Derive data_model_iri by stripping the #fragment, if present.
        let data_model_iri = resource_type_arg.find('#').map(|idx| resource_type_arg[..idx].to_string());
        return Ok(ResourceTypeRef { iri: resource_type_arg.to_string(), name, data_model_iri });
    }

    // Path B / C: bare name — possibly scoped by --data-model.
    if let Some(dm_arg) = data_model_arg {
        // Path B: --data-model given → scope to one data-model.
        let dm_iri = if dm_arg.contains("://") {
            // --data-model is a full IRI → use directly.
            dm_arg.to_string()
        } else {
            // --data-model is a name → resolve via list_data_models.
            let data_models = client.list_data_models(server, project_iri, token)?;
            data_models
                .iter()
                .find(|dm| dm.iri == dm_arg || dm.name.eq_ignore_ascii_case(dm_arg))
                .map(|dm| dm.iri.clone())
                .ok_or_else(|| {
                    let dm_disp: String = dm_arg.chars().take(80).collect();
                    let dm_suffix = if dm_arg.chars().count() > 80 { "…" } else { "" };
                    Diagnostic::NotFound(format!("data-model '{dm_disp}{dm_suffix}' not found in project on {server}"))
                })?
        };

        // Fetch the data-model detail and find the resource-type by name.
        let detail = client.describe_data_model(server, &dm_iri, token)?;
        let matched = detail
            .resource_types
            .iter()
            .find(|rt| rt.name.eq_ignore_ascii_case(resource_type_arg) || rt.iri == resource_type_arg)
            .ok_or_else(|| {
                let rt_disp: String = resource_type_arg.chars().take(80).collect();
                let rt_suffix = if resource_type_arg.chars().count() > 80 {
                    "…"
                } else {
                    ""
                };
                Diagnostic::NotFound(format!(
                    "resource-type '{rt_disp}{rt_suffix}' not found in data-model '{name}' on {server}; \
                     run `dsp vre resource-type list --project <project> --data-model {name} --server {server}` \
                     to see available resource-types.",
                    name = detail.name,
                ))
            })?;
        return Ok(ResourceTypeRef {
            iri: matched.iri.clone(),
            name: matched.name.clone(),
            data_model_iri: Some(dm_iri),
        });
    }

    // Path C: bare name, no --data-model → scan all project data-models.
    let data_models = client.list_data_models(server, project_iri, token)?;
    if data_models.len() > 10 {
        tracing::warn!(
            count = data_models.len(),
            "bare-name resource-type resolution will issue {} describe_data_model calls; \
             use --data-model or a full IRI to avoid scanning",
            data_models.len()
        );
    }

    // Single accumulator: (resource_type_iri, rt_name, dm_iri, dm_name).
    // Using one Vec eliminates the redundant parallel state and removes any need
    // for `.expect()` in the match arm below.
    let mut matches: Vec<(String, String, String, String)> = Vec::new(); // (resource_type_iri, rt_name, dm_iri, dm_name)

    for dm in &data_models {
        let detail = client.describe_data_model(server, &dm.iri, token)?;
        for rt in &detail.resource_types {
            if rt.name.eq_ignore_ascii_case(resource_type_arg) || rt.iri == resource_type_arg {
                matches.push((rt.iri.clone(), rt.name.clone(), dm.iri.clone(), dm.name.clone()));
            }
        }
    }

    match matches.len() {
        0 => {
            let rt_disp: String = resource_type_arg.chars().take(80).collect();
            let rt_suffix = if resource_type_arg.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::NotFound(format!(
                "resource-type '{rt_disp}{rt_suffix}' not found in any data-model of the project on {server}; \
                 run `dsp vre resource-type list --project <project> --server {server}` \
                 to see available resource-types."
            )))
        }
        1 => {
            let (resource_type_iri, rt_name, dm_iri, _dm_name) = matches.remove(0);
            Ok(ResourceTypeRef {
                iri: resource_type_iri,
                name: rt_name,
                data_model_iri: Some(dm_iri),
            })
        }
        _ => {
            // Dedup data-model names: the same DM must not appear twice if a
            // resource-type name appears multiple times within it.
            let mut seen_dm_names: Vec<String> = Vec::new();
            for (_, _, _, dm_name) in &matches {
                if !seen_dm_names.contains(dm_name) {
                    seen_dm_names.push(dm_name.clone());
                }
            }
            let dm_list = seen_dm_names.join(", ");
            let rt_disp: String = resource_type_arg.chars().take(80).collect();
            let rt_suffix = if resource_type_arg.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::Usage(format!(
                "resource-type '{rt_disp}{rt_suffix}' is ambiguous: found in data-models [{dm_list}]; \
                 use --data-model to scope the search to one data-model."
            )))
        }
    }
}

/// Resolve a `--order-by` argument to a complex-schema property IRI.
///
/// Two paths:
/// - `order_by_arg` contains `://` → full-IRI bypass; returned verbatim, `describe_resource_type`
///   is NOT called.
/// - Bare field name → look up via `describe_resource_type(server, dm_iri, rt_iri, token)`, match
///   `field.name` case-insensitively, return `field.iri`. If no match → `Diagnostic::Usage` with a
///   hint pointing at `dsp vre resource-type describe` to list field names.
///
/// Requires `rt_ref.data_model_iri` on the bare-name path; if `None`
/// (path A class IRI without `#`), returns `Diagnostic::Usage` telling the
/// user to pass a full property IRI instead.
fn resolve_order_by_property(
    order_by_arg: &str,
    rt_ref: &ResourceTypeRef,
    server: &str,
    client: &dyn DspClient,
    token: Option<&str>,
) -> Result<String, Diagnostic> {
    // Full-IRI bypass: contains `://` → return verbatim, no describe call.
    if order_by_arg.contains("://") {
        return Ok(order_by_arg.to_string());
    }

    // Bare field name — need the data-model IRI.
    let dm_iri = rt_ref.data_model_iri.as_deref().ok_or_else(|| {
        Diagnostic::Usage(
            "cannot resolve --order-by as a field name when --resource-type is a full IRI \
             that contains no '#' fragment (no data-model can be derived); \
             pass --order-by as a full field IRI instead"
                .to_string(),
        )
    })?;

    // Fetch the resource-type detail (allentities for this data-model).
    // Arg order: (server, data_model_iri, resource_type, token).
    let detail = client.describe_resource_type(server, dm_iri, &rt_ref.iri, token)?;

    // Match field by name (case-insensitive); cover ALL fields (built-ins included).
    if let Some(field) = detail.fields.iter().find(|f| f.name.eq_ignore_ascii_case(order_by_arg)) {
        return Ok(field.iri.clone());
    }

    // No match — Usage error with a hint and the echoed field name
    // (truncated to 80 chars + ellipsis, control-char-sanitised).
    let sanitised = strip_control_chars(order_by_arg);
    let field_disp: String = sanitised.chars().take(80).collect();
    let field_suffix = if sanitised.chars().count() > 80 { "…" } else { "" };
    Err(Diagnostic::Usage(format!(
        "field '{field_disp}{field_suffix}' not found in resource-type '{}'; \
         run `dsp vre resource-type describe --server <s> --project <project> \
         --resource-type {}` to see available field names.",
        rt_ref.name, rt_ref.name,
    )))
}

/// Strip an IRI to its local name (the segment after the last `#`, `/`, or `:`).
///
/// Intentional kept-in-sync duplicate of `local_name` in `src/client/http.rs`.
/// The client's `local_name` is private to the client module; the action layer
/// must not reach into client internals (dsp-cli/ADR-0008 layering). Do NOT introduce a
/// shared util module — the two copies are adjacent enough to audit on sight.
///
/// `rsplit` always yields at least one element so `unwrap_or` is a no-panic
/// guard rather than a live fallback, mirroring the http.rs implementation.
fn local_name(iri: &str) -> String {
    iri.rsplit(['#', '/', ':']).next().unwrap_or(iri).to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// resource describe
// ─────────────────────────────────────────────────────────────────────────────

/// Describe a single resource by its internal IRI.
///
/// Authentication is optional (instance-side read; anonymous callers see only
/// publicly-visible resources per dsp-cli/ADR-0007). Reads `DSP_TOKEN` from the
/// environment and delegates all work to `run_describe_impl` with injectable
/// seams for deterministic testing.
pub fn describe(
    args: &ResourceDescribeArgs,
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
/// with a `tracing::warn!` — NEVER returns `Err`. For an instance-side read,
/// a corrupt or missing `auth.toml` must still describe anonymously.
pub(crate) fn run_describe_impl(
    args: &ResourceDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --resource required (fail-fast, BEFORE any cache/IO) ──────────────
    let resource_iri = args
        .resource
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--resource <iri> is required".to_string()))?;

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
                "auth cache load failed; falling back to anonymous for resource describe"
            );
            AuthCache::default()
        }
    };

    // ── 3. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 4. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 5. D3: filter_warning — always Some for instance-side commands ────────
    //
    // Anonymous callers see only public resources; authenticated callers are
    // still bounded by their permissions. Both warrant disclosure.
    let filter_warning = if resolved.is_none() {
        Some("results may be filtered; login to see private resources".to_string())
    } else {
        Some("results limited to your permissions".to_string())
    };

    // ── 6. D2: Resolve project guard (optional) ───────────────────────────────
    //
    // If --project was given, resolve it to a ProjectRef so we have the IRI for
    // the guard comparison after fetching the resource.
    let resolved_project = if let Some(p) = args.project.as_deref() {
        Some(client.resolve_project(&cfg.server, p)?)
    } else {
        None
    };

    // ── 7. Fetch resource detail ──────────────────────────────────────────────
    let detail = client.describe_resource(&cfg.server, resource_iri, token, args.values)?;

    // ── 8. D2: Cross-project guard ────────────────────────────────────────────
    //
    // Fire ONLY when a project was given AND the resource's attached_project is
    // Some(actual) AND actual != resolved_project_iri. A None attached_project
    // (field absent from the response) passes through — absence ≠ mismatch.
    if let Some(ref proj) = resolved_project
        && let Some(ref actual) = detail.attached_project
        && actual != &proj.iri
    {
        let actual_name = local_name(actual);
        let expected_name = local_name(&proj.iri);
        // Cap the resource IRI at 80 chars for readability, mirroring the
        // idiom used throughout http.rs for user-supplied IRIs.
        let display_iri: String = resource_iri.chars().take(80).collect();
        let iri_suffix = if resource_iri.chars().count() > 80 { "…" } else { "" };
        return Err(Diagnostic::Usage(format!(
            "resource '{display_iri}{iri_suffix}' belongs to project {actual_name}, not {expected_name}"
        )));
    }

    // ── 9. Build meta and render ──────────────────────────────────────────────
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning,
        count_caveat: None,
        count_cost: None,
    };
    renderer.resource_describe(&detail, &meta)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use tempfile::TempDir;

    use super::{run_describe_impl, run_list_impl};
    use crate::cli::{FormatArgs, ResourceDescribeArgs, ResourceListArgs};
    use crate::client::DspClient;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::{
        Cardinality, DataModel, DataModelDetail, Field, ProjectRef, ResourceAccess, ResourceDetail, ResourcePage,
        ResourceSummary, ResourceTypeDetail, ResourceTypeSummary, ResourceVisibility,
    };
    use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
    use crate::render::{
        DataModelListView, DumpDeleteOutcome, DumpOutcome, Format, MetaContext, ProjectListView, Renderer,
        ResourceListView, ResourceTypeListView,
    };

    // ── MockDspClient ─────────────────────────────────────────────────────────
    //
    // Uses a `Vec<Result<DataModelDetail, Diagnostic>>` queue for
    // `describe_data_model` (popped in call order) so that the ambiguous-name
    // test can return different data-models for each call.

    /// Captured `list_resources` arguments: (project_iri, resource_type_iri, order_by, page,
    /// token).
    type ListResourcesArg = (String, String, Option<String>, u32, Option<String>);

    struct MockDspClient {
        resolve_result: Option<Result<ProjectRef, Diagnostic>>,
        resolve_calls: RefCell<u32>,
        list_data_models_result: Option<Result<Vec<DataModel>, Diagnostic>>,
        list_data_models_calls: RefCell<u32>,
        // Queue: each call pops the front; None = unimplemented
        describe_data_model_queue: RefCell<Vec<Result<DataModelDetail, Diagnostic>>>,
        describe_data_model_calls: RefCell<u32>,
        // Configurable describe_resource_type: returns canned detail or unimplemented
        describe_resource_type_result: Option<ResourceTypeDetail>,
        describe_resource_type_calls: RefCell<u32>,
        // Queue for list_resources: each call pops the front
        list_resources_queue: RefCell<Vec<Result<ResourcePage, Diagnostic>>>,
        list_resources_calls: RefCell<u32>,
        list_resources_args: RefCell<Vec<ListResourcesArg>>,
        // Queue for describe_resource: each call pops the front
        describe_resource_queue: RefCell<Vec<Result<ResourceDetail, Diagnostic>>>,
        describe_resource_calls: RefCell<u32>,
        describe_resource_with_values_args: RefCell<Vec<bool>>,
    }

    impl MockDspClient {
        fn new() -> Self {
            Self {
                resolve_result: None,
                resolve_calls: RefCell::new(0),
                list_data_models_result: None,
                list_data_models_calls: RefCell::new(0),
                describe_data_model_queue: RefCell::new(Vec::new()),
                describe_data_model_calls: RefCell::new(0),
                describe_resource_type_result: None,
                describe_resource_type_calls: RefCell::new(0),
                list_resources_queue: RefCell::new(Vec::new()),
                list_resources_calls: RefCell::new(0),
                list_resources_args: RefCell::new(Vec::new()),
                describe_resource_queue: RefCell::new(Vec::new()),
                describe_resource_calls: RefCell::new(0),
                describe_resource_with_values_args: RefCell::new(Vec::new()),
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

        fn with_describe_data_model(self, result: Result<DataModelDetail, Diagnostic>) -> Self {
            self.describe_data_model_queue.borrow_mut().push(result);
            self
        }

        fn with_describe_resource_type(mut self, detail: ResourceTypeDetail) -> Self {
            self.describe_resource_type_result = Some(detail);
            self
        }

        fn with_list_resources(self, result: Result<ResourcePage, Diagnostic>) -> Self {
            self.list_resources_queue.borrow_mut().push(result);
            self
        }

        fn with_describe_resource(self, result: Result<ResourceDetail, Diagnostic>) -> Self {
            self.describe_resource_queue.borrow_mut().push(result);
            self
        }

        fn resolve_calls(&self) -> u32 {
            *self.resolve_calls.borrow()
        }

        fn list_data_models_calls(&self) -> u32 {
            *self.list_data_models_calls.borrow()
        }

        fn describe_resource_type_calls(&self) -> u32 {
            *self.describe_resource_type_calls.borrow()
        }

        fn list_resources_calls(&self) -> u32 {
            *self.list_resources_calls.borrow()
        }

        fn list_resources_args(&self) -> Vec<ListResourcesArg> {
            self.list_resources_args.borrow().clone()
        }

        fn describe_resource_calls(&self) -> u32 {
            *self.describe_resource_calls.borrow()
        }

        fn describe_resource_with_values_args(&self) -> Vec<bool> {
            self.describe_resource_with_values_args.borrow().clone()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in resource action tests")
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
            unimplemented!("create_project_dump not used in resource action tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used in resource action tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used in resource action tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used in resource action tests")
        }

        fn list_projects(&self, _server: &str, _token: Option<&str>) -> Result<Vec<crate::model::Project>, Diagnostic> {
            unimplemented!("list_projects not used in resource action tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used in resource action tests")
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<DataModel>, Diagnostic> {
            *self.list_data_models_calls.borrow_mut() += 1;
            self.list_data_models_result
                .clone()
                .expect("list_data_models_result must be set when list_data_models is called")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            *self.describe_data_model_calls.borrow_mut() += 1;
            let mut queue = self.describe_data_model_queue.borrow_mut();
            if queue.is_empty() {
                panic!("describe_data_model called more times than expected (queue is empty)");
            }
            queue.remove(0)
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            *self.describe_resource_type_calls.borrow_mut() += 1;
            match &self.describe_resource_type_result {
                Some(detail) => Ok(detail.clone()),
                None => Err(Diagnostic::NotFound("describe_resource_type: no result configured".to_string())),
            }
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in resource action tests")
        }

        fn list_resources(
            &self,
            _server: &str,
            project_iri: &str,
            resource_type_iri: &str,
            order_by: Option<&str>,
            page: u32,
            token: Option<&str>,
        ) -> Result<ResourcePage, Diagnostic> {
            *self.list_resources_calls.borrow_mut() += 1;
            self.list_resources_args.borrow_mut().push((
                project_iri.to_string(),
                resource_type_iri.to_string(),
                order_by.map(str::to_owned),
                page,
                token.map(str::to_owned),
            ));
            let mut queue = self.list_resources_queue.borrow_mut();
            if queue.is_empty() {
                panic!("list_resources called more times than expected (queue is empty)");
            }
            queue.remove(0)
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            *self.describe_resource_calls.borrow_mut() += 1;
            self.describe_resource_with_values_args.borrow_mut().push(with_values);
            let mut queue = self.describe_resource_queue.borrow_mut();
            if queue.is_empty() {
                panic!("describe_resource called more times than expected (queue is empty)");
            }
            queue.remove(0)
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used in resource action tests")
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
        resources_view: Option<ResourceListView>,
        resources_meta: Option<MetaContext>,
        resource_describe_detail: Option<ResourceDetail>,
        resource_describe_meta: Option<MetaContext>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                resources_view: None,
                resources_meta: None,
                resource_describe_detail: None,
                resource_describe_meta: None,
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

        fn resource_types(&mut self, _view: &ResourceTypeListView, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_type_describe(
            &mut self,
            _detail: &crate::model::ResourceTypeDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.resources_view = Some(view.clone());
            self.resources_meta = Some(meta.clone());
            Ok(())
        }

        fn resource_describe(
            &mut self,
            detail: &crate::model::ResourceDetail,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.resource_describe_detail = Some(detail.clone());
            self.resource_describe_meta = Some(meta.clone());
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
    const PROJECT_IRI: &str = "http://rdfh.ch/projects/0801";
    const BEOL_IRI: &str = "http://api.test.dasch.swiss/ontology/0801/beol/v2";
    const LETTER_IRI: &str = "http://api.test.dasch.swiss/ontology/0801/beol/v2#Letter";

    fn make_cfg() -> Config {
        Config { server: SERVER.to_string() }
    }

    fn make_project_ref() -> ProjectRef {
        ProjectRef {
            iri: PROJECT_IRI.to_string(),
            shortcode: "0801".to_string(),
            shortname: "beol".to_string(),
        }
    }

    fn make_args(
        project: Option<&str>,
        resource_type: Option<&str>,
        data_model: Option<&str>,
        page: Option<u32>,
        all: bool,
        filter: Option<&str>,
    ) -> ResourceListArgs {
        make_args_with_order_by(project, resource_type, data_model, page, all, filter, None)
    }

    fn make_args_with_order_by(
        project: Option<&str>,
        resource_type: Option<&str>,
        data_model: Option<&str>,
        page: Option<u32>,
        all: bool,
        filter: Option<&str>,
        order_by: Option<&str>,
    ) -> ResourceListArgs {
        ResourceListArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            resource_type: resource_type.map(str::to_owned),
            data_model: data_model.map(str::to_owned),
            page,
            all,
            filter: filter.map(str::to_owned),
            order_by: order_by.map(str::to_owned),
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

    fn make_data_model(name: &str) -> DataModel {
        DataModel {
            name: name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/{name}/v2"),
            label: None,
            last_modified: None,
            is_builtin: false,
        }
    }

    fn make_detail_with_rts(dm_name: &str, rt_names: &[&str]) -> DataModelDetail {
        let resource_types = rt_names
            .iter()
            .map(|name| ResourceTypeSummary {
                name: name.to_string(),
                iri: format!("http://api.test.dasch.swiss/ontology/0801/{dm_name}/v2#{name}"),
                label: None,
            })
            .collect();
        DataModelDetail {
            name: dm_name.to_string(),
            iri: format!("http://api.test.dasch.swiss/ontology/0801/{dm_name}/v2"),
            label: None,
            last_modified: None,
            resource_types,
        }
    }

    fn make_resource(label: &str) -> ResourceSummary {
        ResourceSummary {
            label: label.to_string(),
            iri: format!("http://rdfh.ch/0801/{label}"),
            ark_url: None,
            creation_date: None,
            last_modified: None,
            resource_type: "Letter".to_string(),
        }
    }

    fn make_resource_page(labels: &[&str], may_have_more: bool) -> ResourcePage {
        ResourcePage {
            resources: labels.iter().map(|l| make_resource(l)).collect(),
            may_have_more_results: may_have_more,
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

    const RESOURCE_IRI: &str = "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw";
    const OTHER_PROJECT_IRI: &str = "http://rdfh.ch/projects/9999";

    fn make_describe_args(resource: Option<&str>, project: Option<&str>) -> ResourceDescribeArgs {
        make_describe_args_with_values(resource, project, false)
    }

    fn make_describe_args_with_values(
        resource: Option<&str>,
        project: Option<&str>,
        values: bool,
    ) -> ResourceDescribeArgs {
        ResourceDescribeArgs {
            server: Some(SERVER.to_string()),
            resource: resource.map(str::to_owned),
            project: project.map(str::to_owned),
            values,
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

    fn make_resource_detail() -> ResourceDetail {
        ResourceDetail {
            label: "Test Page".to_string(),
            iri: RESOURCE_IRI.to_string(),
            resource_type: "Page".to_string(),
            ark_url: Some("ark:/72163/1/0803/--6Esp4SVnGG1DBzFvYErw".to_string()),
            creation_date: Some("2021-01-01T00:00:00Z".to_string()),
            last_modified: Some("2021-06-15T12:00:00Z".to_string()),
            attached_project: Some(PROJECT_IRI.to_string()),
            owner: Some("http://rdfh.ch/users/root".to_string()),
            visibility: Some(ResourceVisibility::Public),
            your_access: Some(ResourceAccess::View),
            values: None,
        }
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    // ── describe tests ────────────────────────────────────────────────────────

    /// Happy path: renderer receives the expected ResourceDetail.
    #[test]
    fn test_describe_happy_path() {
        let detail = make_resource_detail();
        let client = MockDspClient::new().with_describe_resource(Ok(detail.clone()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(client.describe_resource_calls(), 1);

        let recorded = renderer
            .resource_describe_detail
            .expect("resource_describe must have been called");
        assert_eq!(recorded.label, "Test Page");
        assert_eq!(recorded.iri, RESOURCE_IRI);
        assert_eq!(recorded.resource_type, "Page");
        assert_eq!(recorded.visibility, Some(ResourceVisibility::Public));
        assert_eq!(recorded.your_access, Some(ResourceAccess::View));
    }

    /// --resource missing → Usage error; no client calls.
    #[test]
    fn test_describe_missing_resource_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(None, None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --resource must yield Usage, got {:?}",
            result
        );
        assert_eq!(client.describe_resource_calls(), 0);
    }

    /// --resource missing → Usage error message mentions --resource.
    #[test]
    fn test_describe_missing_resource_error_mentions_flag() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(None, None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        if let Err(Diagnostic::Usage(msg)) = result {
            assert!(msg.contains("--resource"), "error message must mention --resource, got: {msg}");
        } else {
            panic!("expected Usage error");
        }
    }

    /// Cross-project guard MATCH: resource.attached_project == resolved_project_iri → no error.
    #[test]
    fn test_describe_cross_project_guard_match_no_error() {
        let detail = make_resource_detail(); // attached_project = PROJECT_IRI
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref())) // iri = PROJECT_IRI
            .with_describe_resource(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), Some("0801"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "matching project must not error, got {:?}", result);
        assert!(renderer.resource_describe_detail.is_some(), "renderer must have been called");
    }

    /// Cross-project guard MISMATCH: resource.attached_project != resolved_project_iri → Usage
    /// error.
    ///
    /// The error message must name BOTH the resource IRI and the local-name tails of
    /// the actual project (OTHER_PROJECT_IRI → "9999") and the expected project
    /// (PROJECT_IRI → "0801"), so a generic "mismatch" message would fail this test.
    #[test]
    fn test_describe_cross_project_guard_mismatch_usage_error() {
        let mut detail = make_resource_detail();
        detail.attached_project = Some(OTHER_PROJECT_IRI.to_string()); // belongs to a different project

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref())) // iri = PROJECT_IRI
            .with_describe_resource(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), Some("0801"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "project mismatch must yield Usage, got {:?}",
            result
        );
        assert!(
            renderer.resource_describe_detail.is_none(),
            "renderer must NOT be called on mismatch"
        );

        // Tightened: the message must mention the resource IRI and local-name
        // tails of BOTH the actual project ("9999") and the expected project ("0801").
        if let Err(Diagnostic::Usage(msg)) = result {
            assert!(
                msg.contains(RESOURCE_IRI),
                "mismatch error must mention the resource IRI; got: {msg}"
            );
            // local_name("http://rdfh.ch/projects/9999") → "9999"
            assert!(
                msg.contains("9999"),
                "mismatch error must mention the actual project's local name '9999'; got: {msg}"
            );
            // local_name("http://rdfh.ch/projects/0801") → "0801"
            assert!(
                msg.contains("0801"),
                "mismatch error must mention the expected project's local name '0801'; got: {msg}"
            );
        } else {
            panic!("expected Err(Diagnostic::Usage(_))");
        }
    }

    /// Cross-project guard with attached_project == None → passes through (no error).
    ///
    /// Per D2: absence of attached_project does NOT trigger the guard.
    #[test]
    fn test_describe_cross_project_guard_none_passes_through() {
        let mut detail = make_resource_detail();
        detail.attached_project = None; // absent from response

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_describe_resource(Ok(detail));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), Some("0801"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "None attached_project must not error, got {:?}", result);
        assert!(renderer.resource_describe_detail.is_some(), "renderer must have been called");
    }

    /// No --project given: guard is entirely skipped, resolve_project not called.
    #[test]
    fn test_describe_no_project_guard_skipped() {
        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --project is omitted"
        );
    }

    /// D3: anonymous → filter_warning is "results may be filtered; login to see private resources".
    #[test]
    fn test_describe_filter_warning_anonymous() {
        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resource_describe_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        let fw = meta.filter_warning.expect("filter_warning must be Some for anonymous");
        assert_eq!(fw, "results may be filtered; login to see private resources");
    }

    /// D3: authenticated (cache token) → filter_warning is "results limited to your permissions".
    #[test]
    fn test_describe_filter_warning_authenticated() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        let meta = renderer.resource_describe_meta.expect("meta must be present");
        let fw = meta.filter_warning.expect("filter_warning must be Some for authenticated");
        assert_eq!(fw, "results limited to your permissions");
    }

    /// Not-found propagation: mock returns Diagnostic::NotFound → surfaced.
    #[test]
    fn test_describe_not_found_propagates() {
        let client = MockDspClient::new()
            .with_describe_resource(Err(Diagnostic::NotFound("resource not found on server".to_string())));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "not-found from client must propagate, got {:?}",
            result
        );
        assert!(
            renderer.resource_describe_detail.is_none(),
            "renderer must not be called on error"
        );
    }

    /// Token forwarded to describe_resource when authenticated.
    #[test]
    fn test_describe_token_forwarded() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "my-describe-token", "user@example.com");
        let path = write_cache(&dir, &cache);

        // We need to capture the token passed to describe_resource.
        // Use env_token injection (the public describe() fn reads DSP_TOKEN; we
        // inject via env_token parameter to run_describe_impl directly).
        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        // The auth_state reported in meta reveals that auth was used.
        let meta = renderer.resource_describe_meta.expect("meta must be present");
        // Authenticated (cache token) → filter_warning uses the authenticated wording.
        let fw = meta.filter_warning.expect("filter_warning must be present");
        assert_eq!(fw, "results limited to your permissions");
    }

    /// --values absent (default false) → describe_resource called with with_values=false.
    #[test]
    fn test_describe_values_flag_absent_passes_false() {
        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None); // values=false by default
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let with_values_args = client.describe_resource_with_values_args();
        assert_eq!(with_values_args.len(), 1);
        assert!(!with_values_args[0], "with_values must be false when --values is absent");
    }

    /// --values true → describe_resource called with with_values=true.
    #[test]
    fn test_describe_values_flag_present_passes_true() {
        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args_with_values(Some(RESOURCE_IRI), None, true);
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let with_values_args = client.describe_resource_with_values_args();
        assert_eq!(with_values_args.len(), 1);
        assert!(with_values_args[0], "with_values must be true when --values is given");
    }

    /// Corrupt/missing cache → falls back to anonymous, never returns Err.
    #[test]
    fn test_describe_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let client = MockDspClient::new().with_describe_resource(Ok(make_resource_detail()));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(RESOURCE_IRI), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&bad_path));
        assert!(result.is_ok(), "corrupt cache must not fail: {:?}", result);

        let meta = renderer.resource_describe_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
    }

    // ── list tests ────────────────────────────────────────────────────────────

    /// Full-IRI bypass: no data-model scan, list_data_models not called.
    #[test]
    fn test_list_full_iri_bypass_skips_scan() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["LetterA"], false)));

        let mut renderer = RecordingRenderer::new();
        // Full IRI → bypass scan
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "full IRI must not trigger a data-model scan"
        );
        assert_eq!(client.list_resources_calls(), 1);

        let view = renderer.resources_view.expect("resources must have been called");
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.resource_type, "Letter"); // local name extracted
    }

    /// Full-IRI bypass: correct resource_type_iri and project_iri passed to list_resources.
    #[test]
    fn test_list_full_iri_passes_correct_params() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let calls = client.list_resources_args();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, PROJECT_IRI, "project_iri must be the resolved IRI");
        assert_eq!(calls[0].1, LETTER_IRI, "resource_type_iri must be the full IRI");
        assert_eq!(calls[0].2, None, "order_by is None (placeholder)");
        assert_eq!(calls[0].3, 0, "default page is 0");
        assert_eq!(calls[0].4, None, "no token for anonymous");
    }

    /// Bare-name scan (one match): resolves correctly.
    #[test]
    fn test_list_bare_name_scan_single_match() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter", "Person"])))
            .with_list_resources(Ok(make_resource_page(&["LetterA"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("Letter"), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.resource_type, "Letter");
        assert_eq!(view.items.len(), 1);
    }

    /// Bare-name scan with zero matches → NotFound.
    #[test]
    fn test_list_bare_name_scan_not_found() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter", "Person"])));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("Nonexistent"), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::NotFound(_))),
            "unknown resource-type must yield NotFound, got {:?}",
            result
        );
        assert_eq!(client.list_resources_calls(), 0);
    }

    /// Bare-name ambiguous across two data-models → Usage error listing both DMs.
    #[test]
    fn test_list_bare_name_ambiguous_gives_usage_error() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol"), make_data_model("webern")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter", "Person"])))
            .with_describe_data_model(Ok(make_detail_with_rts("webern", &["Letter", "Score"])));

        let mut renderer = RecordingRenderer::new();
        // "Letter" exists in both data-models → ambiguous
        let args = make_args(Some("0801"), Some("Letter"), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "ambiguous resource-type must yield Usage error, got {:?}",
            result
        );
        if let Err(Diagnostic::Usage(msg)) = &result {
            assert!(
                msg.contains("beol") && msg.contains("webern"),
                "Usage error must list both data-models, got: {msg}"
            );
            assert!(
                msg.contains("--data-model"),
                "Usage error must suggest --data-model, got: {msg}"
            );
        }
        assert_eq!(client.list_resources_calls(), 0);
    }

    /// --data-model name scope: resolves only within the named data-model.
    #[test]
    fn test_list_data_model_name_scope() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_list_resources(Ok(make_resource_page(&["LetterA"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("Letter"), Some("beol"), None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.items.len(), 1);
    }

    /// --data-model full IRI scope: skips list_data_models.
    #[test]
    fn test_list_data_model_full_iri_scope_skips_list() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_list_resources(Ok(make_resource_page(&["LetterA"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some("Letter"), Some(BEOL_IRI), None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            client.list_data_models_calls(),
            0,
            "--data-model as full IRI must skip list_data_models"
        );
    }

    /// Single page with may_have_more=false: pagination is SinglePage { may_have_more: false }.
    #[test]
    fn test_list_single_page_no_more() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A", "B"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.items.len(), 2);
        assert_eq!(view.total, 2);
        match &view.pagination {
            crate::render::ResourceListPagination::SinglePage { page, may_have_more } => {
                assert_eq!(*page, 0);
                assert!(!may_have_more);
            }
            other => panic!("expected SinglePage, got {:?}", other),
        }
    }

    /// Single page with may_have_more=true.
    #[test]
    fn test_list_single_page_has_more() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A", "B"], true)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resources_view.expect("must have been called");
        match &view.pagination {
            crate::render::ResourceListPagination::SinglePage { may_have_more, .. } => {
                assert!(*may_have_more);
            }
            other => panic!("expected SinglePage, got {:?}", other),
        }
    }

    /// --page N: the correct page number is passed to list_resources.
    #[test]
    fn test_list_explicit_page_passed_to_client() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, Some(3), false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let calls = client.list_resources_args();
        assert_eq!(calls[0].3, 3, "--page 3 must be passed to list_resources");

        let view = renderer.resources_view.expect("must have been called");
        match &view.pagination {
            crate::render::ResourceListPagination::SinglePage { page, .. } => {
                assert_eq!(*page, 3);
            }
            other => panic!("expected SinglePage, got {:?}", other),
        }
    }

    /// --all fetches multiple pages; stops when may_have_more=false.
    #[test]
    fn test_list_all_multi_page() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A", "B"], true)))
            .with_list_resources(Ok(make_resource_page(&["C"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, true, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(client.list_resources_calls(), 2);
        // Pages requested: 0 then 1
        let calls = client.list_resources_args();
        assert_eq!(calls[0].3, 0);
        assert_eq!(calls[1].3, 1);

        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.items.len(), 3, "all pages merged");
        assert_eq!(view.total, 3);
        match &view.pagination {
            crate::render::ResourceListPagination::AllPages { pages_fetched } => {
                assert_eq!(*pages_fetched, 2);
            }
            other => panic!("expected AllPages, got {:?}", other),
        }
    }

    /// --all with empty final page: accumulate nothing, terminate normally.
    #[test]
    fn test_list_all_empty_final_page() {
        // Page 0 has items + more; page 1 is empty + no more.
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A"], true)))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, true, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(client.list_resources_calls(), 2);
        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.items.len(), 1, "empty final page contributes zero resources");
        match &view.pagination {
            crate::render::ResourceListPagination::AllPages { pages_fetched } => {
                assert_eq!(*pages_fetched, 2);
            }
            other => panic!("expected AllPages, got {:?}", other),
        }
    }

    /// --all propagates error mid-loop; nothing is rendered.
    #[test]
    fn test_list_all_propagates_error_mid_loop() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A"], true)))
            .with_list_resources(Err(Diagnostic::ServerError("server exploded".into())));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, true, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "mid-loop error must propagate, got {:?}",
            result
        );
        assert!(renderer.resources_view.is_none(), "no partial result must be rendered on error");
    }

    /// Empty result (zero resources): renders with total=0.
    #[test]
    fn test_list_empty_result() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resources_view.expect("must have been called");
        assert!(view.items.is_empty());
        assert_eq!(view.total, 0);
    }

    /// --filter narrows items; total reflects pre-filter count.
    #[test]
    fn test_list_filter_narrows_total_is_prefilter() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["AliceA", "BobB", "AliceC"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, Some("alice"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer.resources_view.expect("must have been called");
        assert_eq!(view.total, 3, "total is pre-filter");
        assert_eq!(view.items.len(), 2, "filter keeps only Alice items");
        assert!(view.items.iter().all(|r| r.label.to_lowercase().contains("alice")));
        assert_eq!(view.filter.as_deref(), Some("alice"));
    }

    /// --project missing → Usage error; no client calls.
    #[test]
    fn test_list_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_args(None, Some(LETTER_IRI), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --project must yield Usage, got {:?}",
            result
        );
        assert_eq!(client.resolve_calls(), 0);
    }

    /// --resource-type missing → Usage error; no client calls.
    #[test]
    fn test_list_missing_resource_type_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), None, None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "missing --resource-type must yield Usage, got {:?}",
            result
        );
        assert_eq!(client.resolve_calls(), 0);
    }

    /// D3: anonymous → filter_warning is "results may be filtered; login to see private resources".
    #[test]
    fn test_list_filter_warning_anonymous() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let meta = renderer.resources_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
        let fw = meta.filter_warning.expect("filter_warning must be Some for anonymous");
        assert_eq!(fw, "results may be filtered; login to see private resources");
    }

    /// D3: authenticated (cache token) → filter_warning is "results limited to your permissions".
    #[test]
    fn test_list_filter_warning_authenticated() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "cache-token-xyz", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        let meta = renderer.resources_meta.expect("meta must be present");
        let fw = meta.filter_warning.expect("filter_warning must be Some for authenticated");
        assert_eq!(fw, "results limited to your permissions");
    }

    /// Corrupt/missing cache → falls back to anonymous, never returns Err.
    #[test]
    fn test_list_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().expect("tempdir");
        let bad_path = dir.path().join("nonexistent_auth.toml");

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&bad_path));
        assert!(result.is_ok(), "corrupt cache must not fail: {:?}", result);

        let meta = renderer.resources_meta.expect("meta must be present");
        assert_eq!(meta.auth_state, "anonymous");
    }

    /// Mid-scan `describe_data_model` error is propagated; `list_resources` is never
    /// called.
    ///
    /// The project has two data-models. The first `describe_data_model` call succeeds
    /// (returning a detail with no matching type), but the second returns
    /// `Err(Diagnostic::ServerError(...))`. The action must propagate that error and
    /// never proceed to `list_resources`.
    #[test]
    fn test_list_mid_scan_describe_error_propagates() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol"), make_data_model("webern")]))
            // First call succeeds — but the target type is not in it.
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter", "Person"])))
            // Second call returns a server error mid-scan.
            .with_describe_data_model(Err(Diagnostic::ServerError("data-model service unavailable".into())));

        let mut renderer = RecordingRenderer::new();
        // "Score" exists only in webern, but the second call errors before we get there.
        let args = make_args(Some("0801"), Some("Score"), None, None, false, None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "mid-scan describe_data_model error must propagate as ServerError, got {:?}",
            result
        );
        assert_eq!(
            client.list_resources_calls(),
            0,
            "list_resources must never be called when describe_data_model errors mid-scan"
        );
    }

    /// Token passed to list_resources when authenticated.
    #[test]
    fn test_list_token_forwarded_to_list_resources() {
        let dir = TempDir::new().expect("tempdir");
        let cache = cache_with_entry(SERVER, "my-token", "user@example.com");
        let path = write_cache(&dir, &cache);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, Some(&path)).expect("expected Ok");

        let calls = client.list_resources_args();
        assert_eq!(
            calls[0].4,
            Some("my-token".to_string()),
            "token must be forwarded to list_resources"
        );
    }

    // ── order-by tests ────────────────────────────────────────────────────────

    /// Build a ResourceTypeDetail with a set of fields for testing.
    fn make_rt_detail_with_fields(rt_name: &str, field_names: &[(&str, &str)]) -> ResourceTypeDetail {
        let fields = field_names
            .iter()
            .map(|(name, iri)| Field {
                name: name.to_string(),
                iri: iri.to_string(),
                label: None,
                value_type: crate::model::resource_type::ValueType::Text,
                link_target: None,
                cardinality: Cardinality::ZeroOrOne,
                is_builtin: false,
                data_model: Some("beol".to_string()),
            })
            .collect();
        ResourceTypeDetail {
            name: rt_name.to_string(),
            iri: LETTER_IRI.to_string(),
            label: None,
            data_model: "beol".to_string(),
            representation: None,
            super_types: vec![],
            fields,
            count: None,
        }
    }

    const TITLE_PROP_IRI: &str = "http://api.test.dasch.swiss/ontology/0801/beol/v2#hasTitle";

    /// Bare field name: resolves to the correct property IRI passed to list_resources.
    #[test]
    fn test_order_by_bare_name_resolves_to_property_iri() {
        let rt_detail = make_rt_detail_with_fields("Letter", &[("hasTitle", TITLE_PROP_IRI)]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_describe_resource_type(rt_detail)
            .with_list_resources(Ok(make_resource_page(&["A"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args_with_order_by(Some("0801"), Some("Letter"), None, None, false, None, Some("hasTitle"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.describe_resource_type_calls(),
            1,
            "bare field name must call describe_resource_type"
        );
        let calls = client.list_resources_args();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].2,
            Some(TITLE_PROP_IRI.to_string()),
            "order_by must be the resolved property IRI"
        );
    }

    /// `://` bypass: value used verbatim, describe_resource_type NOT called.
    #[test]
    fn test_order_by_full_iri_bypass_skips_describe_resource_type() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&["A"], false)));

        let mut renderer = RecordingRenderer::new();
        let args =
            make_args_with_order_by(Some("0801"), Some(LETTER_IRI), None, None, false, None, Some(TITLE_PROP_IRI));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.describe_resource_type_calls(),
            0,
            "full-IRI bypass must NOT call describe_resource_type"
        );
        let calls = client.list_resources_args();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].2,
            Some(TITLE_PROP_IRI.to_string()),
            "order_by must be the verbatim IRI"
        );
    }

    /// Path A (full resource-type IRI with `#`) + bare order-by → resolves correctly.
    #[test]
    fn test_order_by_path_a_with_fragment_resolves() {
        // LETTER_IRI contains '#', so data_model_iri will be derived as the part before '#'.
        let rt_detail = make_rt_detail_with_fields("Letter", &[("hasTitle", TITLE_PROP_IRI)]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_describe_resource_type(rt_detail)
            .with_list_resources(Ok(make_resource_page(&["A"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args_with_order_by(
            Some("0801"),
            Some(LETTER_IRI), // full IRI with '#'
            None,
            None,
            false,
            None,
            Some("hasTitle"),
        );
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(
            client.describe_resource_type_calls(),
            1,
            "bare field name with path-A resource-type must call describe_resource_type"
        );
        let calls = client.list_resources_args();
        assert_eq!(
            calls[0].2,
            Some(TITLE_PROP_IRI.to_string()),
            "order_by must be the resolved property IRI"
        );
    }

    /// Path A resource-type IRI lacking `#` + bare order-by → Usage error.
    #[test]
    fn test_order_by_path_a_no_fragment_bare_name_is_usage_error() {
        // A class IRI without '#' — data_model_iri cannot be derived. Resolution
        // must fail before any fetch, so no `with_list_resources` is seeded.
        let no_fragment_iri = "http://api.test.dasch.swiss/ontology/0801/beol/v2/Letter";
        let client = MockDspClient::new().with_resolve_project(Ok(make_project_ref()));

        let mut renderer = RecordingRenderer::new();
        let args =
            make_args_with_order_by(Some("0801"), Some(no_fragment_iri), None, None, false, None, Some("hasTitle"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "path-A IRI lacking '#' + bare order-by must yield Usage, got {:?}",
            result
        );
        assert_eq!(
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when error is expected"
        );
    }

    /// Unknown field name → Usage error with a helpful hint.
    #[test]
    fn test_order_by_unknown_field_is_usage_error() {
        let rt_detail = make_rt_detail_with_fields("Letter", &[("hasTitle", TITLE_PROP_IRI)]);
        // No `with_list_resources` seed: resolution must fail before any fetch.
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_describe_resource_type(rt_detail);

        let mut renderer = RecordingRenderer::new();
        let args =
            make_args_with_order_by(Some("0801"), Some("Letter"), None, None, false, None, Some("nonExistentField"));
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);

        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "unknown field name must yield Usage, got {:?}",
            result
        );
        if let Err(Diagnostic::Usage(msg)) = &result {
            assert!(
                msg.contains("nonExistentField"),
                "Usage error must echo the field name, got: {msg}"
            );
            assert!(
                msg.contains("resource-type describe"),
                "Usage error must hint at resource-type describe, got: {msg}"
            );
        }
        // Resolution looked the field up, then bailed before fetching.
        assert_eq!(client.describe_resource_type_calls(), 1);
        assert_eq!(
            client.list_resources_calls(),
            0,
            "list_resources must not be called when order-by resolution fails"
        );
    }

    /// No `--order-by` → None recorded by mock.
    #[test]
    fn test_order_by_absent_passes_none_to_list_resources() {
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args(Some("0801"), Some(LETTER_IRI), None, None, false, None);
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let calls = client.list_resources_args();
        assert_eq!(calls[0].2, None, "--order-by absent must pass None to list_resources");
        assert_eq!(
            client.describe_resource_type_calls(),
            0,
            "describe_resource_type must not be called when --order-by is absent"
        );
    }

    /// `--order-by` + `--all` → resolved IRI threaded to EVERY page request.
    #[test]
    fn test_order_by_with_all_threads_iri_to_every_page() {
        let rt_detail = make_rt_detail_with_fields("Letter", &[("hasTitle", TITLE_PROP_IRI)]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_describe_resource_type(rt_detail)
            .with_list_resources(Ok(make_resource_page(&["A"], true)))
            .with_list_resources(Ok(make_resource_page(&["B"], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args_with_order_by(
            Some("0801"),
            Some("Letter"),
            None,
            None,
            true, // --all
            None,
            Some("hasTitle"),
        );
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        assert_eq!(client.list_resources_calls(), 2);
        let calls = client.list_resources_args();
        assert_eq!(
            calls[0].2,
            Some(TITLE_PROP_IRI.to_string()),
            "order_by must be threaded to page 0"
        );
        assert_eq!(
            calls[1].2,
            Some(TITLE_PROP_IRI.to_string()),
            "order_by must be threaded to page 1"
        );
    }

    /// Case-insensitive field name match: `HASTITLE` resolves to the same IRI as `hasTitle`.
    #[test]
    fn test_order_by_field_name_is_case_insensitive() {
        let rt_detail = make_rt_detail_with_fields("Letter", &[("hasTitle", TITLE_PROP_IRI)]);
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_list_data_models(Ok(vec![make_data_model("beol")]))
            .with_describe_data_model(Ok(make_detail_with_rts("beol", &["Letter"])))
            .with_describe_resource_type(rt_detail)
            .with_list_resources(Ok(make_resource_page(&[], false)));

        let mut renderer = RecordingRenderer::new();
        let args = make_args_with_order_by(Some("0801"), Some("Letter"), None, None, false, None, Some("HASTITLE"));
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let calls = client.list_resources_args();
        assert_eq!(
            calls[0].2,
            Some(TITLE_PROP_IRI.to_string()),
            "case-insensitive field name match must resolve to the property IRI"
        );
    }
}
