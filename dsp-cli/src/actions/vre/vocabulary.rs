//! Actions for `dsp vre vocabulary { list | describe }` (plan 034).
//!
//! Mirrors `src/actions/vre/resource_type.rs`'s shape: public entry points
//! delegate to `run_list_impl`/`run_describe_impl` with injectable
//! `env_token`/`cache_path` seams for deterministic testing. Both commands are
//! schema-side reads (vocabularies are public, D2) — `filter_warning` always
//! stays `None`, matching every other schema-side command.

use std::path::Path;

use crate::cli::{VocabularyDescribeArgs, VocabularyListArgs};
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::model::{Vocabulary, VocabularyDetail};
use crate::render::{MetaContext, Renderer, VocabularyListView};

use crate::actions::auth_state::read_auth_state;

/// Disclosure note for `vocabulary list --count` (plan 034) — schema-side
/// COST disclosure, distinct from `resource_type.rs`'s `COUNT_CAVEAT`, which
/// is about permission-filtering accuracy. That does not apply here:
/// vocabularies are public and their counts are exact. This note instead
/// discloses that `--count` is N+1 by construction (one extra tree fetch per
/// vocabulary, sequential, unthrottled). `run_list_impl` appends a
/// failure-count clause when one or more per-vocabulary fetches failed.
const COUNT_COST: &str = "--count issues one extra tree fetch per vocabulary (sequential; can be dozens of calls on projects with many vocabularies).";

/// List a project's vocabularies.
///
/// Authentication is optional (public endpoint, D2/schema-side read). Reads
/// `DSP_TOKEN` from the environment (env wins over cache per ADR-0007), and
/// delegates all work to `run_list_impl` with injectable seams for
/// deterministic testing.
pub fn list(
    args: &VocabularyListArgs,
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
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty
/// cache with a `tracing::warn!` — NEVER returns `Err`. Vocabularies are
/// public data (D2); a corrupt or missing `auth.toml` must still list
/// anonymously.
fn run_list_impl(
    args: &VocabularyListArgs,
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
                "auth cache load failed; falling back to anonymous for vocabulary list"
            );
            AuthCache::default()
        }
    };

    // ── 3. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 4. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 5. Resolve project ────────────────────────────────────────────────────
    let pref = client.resolve_project(&cfg.server, project)?;

    // ── 6. Fetch the project's vocabularies ───────────────────────────────────
    let mut items = client.list_vocabularies(&cfg.server, &pref.iri, token)?;

    // ── 7. Capture total BEFORE filter ─────────────────────────────────────────
    let total = items.len();

    // ── 8. Apply --filter: case-insensitive substring over `name` AND every
    //      label value in every language (D4 — no language preference; a
    //      name-only or single-language filter would be a silent bias).
    //      Comments are NOT scanned — disclosed in --help.
    if let Some(ref f) = args.filter {
        let lower = f.to_lowercase();
        items.retain(|item| {
            item.header
                .name
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&lower)
                || item
                    .header
                    .labels
                    .iter()
                    .any(|l| l.value.to_lowercase().contains(&lower))
        });
    }

    // ── 9. Sort by name ascending, case-insensitive, None-named roots LAST,
    //      tie-broken by iri ascending. DSP-API models no order for
    //      project-level vocabularies (see the plan's Step 4 for why), so
    //      dsp-cli must choose a deterministic one.
    items.sort_by(|a, b| match (&a.header.name, &b.header.name) {
        (Some(an), Some(bn)) => an
            .to_lowercase()
            .cmp(&bn.to_lowercase())
            .then_with(|| a.header.iri.cmp(&b.header.iri)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.header.iri.cmp(&b.header.iri),
    });

    // ── 10. --count: fetch each surviving vocabulary's full tree, sequentially.
    //       A failed per-tree fetch degrades that row's counts to `None` and
    //       is tallied — it must NOT abort the whole command (the calls are
    //       sequential and unthrottled against a shared production server).
    let mut failed = 0usize;
    if args.count {
        for item in &mut items {
            match client.describe_vocabulary(&cfg.server, &item.header.iri, token) {
                Ok(tree) => {
                    let (n, d) = tree.count_and_depth(None);
                    item.node_count = Some(n);
                    item.depth = Some(d);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        vocabulary_iri = %item.header.iri,
                        "per-vocabulary tree fetch failed under --count; degrading this row to no counts"
                    );
                    failed += 1;
                }
            }
        }
    }

    // ── 11. Build the --count cost-disclosure message ─────────────────────────
    let count_cost = if args.count {
        let mut msg = COUNT_COST.to_string();
        if failed > 0 {
            msg.push(' ');
            msg.push_str(&format!(
                "{failed} of {} per-vocabulary tree fetches failed; affected rows show no node/depth counts.",
                items.len()
            ));
        }
        Some(msg)
    } else {
        None
    };

    // ── 12. Build view + meta and render ───────────────────────────────────────
    let view = VocabularyListView {
        items,
        total,
        filter: args.filter.clone(),
        counted: args.count,
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost,
    };
    renderer.vocabularies(&view, &meta)
}

/// Describe a single vocabulary's full tree.
///
/// Authentication is optional (public endpoint, D2/schema-side read). Reads
/// `DSP_TOKEN` from the environment (env wins over cache per ADR-0007), and
/// delegates all work to `run_describe_impl` with injectable seams for
/// deterministic testing.
pub fn describe(
    args: &VocabularyDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_describe_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `describe` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `describe`
///   entry point before calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in
///   production to use the default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty
/// cache with a `tracing::warn!` — NEVER returns `Err`. Vocabularies are
/// public data (D2); a corrupt or missing `auth.toml` must still describe
/// anonymously.
fn run_describe_impl(
    args: &VocabularyDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --vocabulary required (fail-fast, BEFORE any cache/IO) ────────────
    let vocabulary = args
        .vocabulary
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--vocabulary <name-or-IRI> is required".to_string()))?;

    // ── 2. Bare name requires --project (fail-fast, BEFORE any cache/IO) ─────
    // A full IRI needs no project (`describe_vocabulary` takes none); only
    // bare-name resolution does, since it must scan one project's roots.
    if !vocabulary.contains("://") && args.project.is_none() {
        return Err(Diagnostic::Usage(
            "--project <shortcode|shortname|IRI> is required to resolve a vocabulary by name; \
             pass a full IRI instead, or supply --project"
                .to_string(),
        ));
    }

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
                "auth cache load failed; falling back to anonymous for vocabulary describe"
            );
            AuthCache::default()
        }
    };

    // ── 4. Resolve token (optional); filter_warning stays None (schema-side) ──
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 5. Resolve --project, if given, BEFORE the fetch — mirrors
    //      `resource.rs`'s describe cross-project guard setup, since
    //      resolution can also fail fast. `-p` is optional on `describe`
    //      (required only for bare-name resolution, per step 2).
    let resolved_project = if let Some(p) = args.project.as_deref() {
        Some(client.resolve_project(&cfg.server, p)?)
    } else {
        None
    };

    // ── 6. Resolve the vocabulary address to a concrete IRI ────────────────────
    // Contains `://` → used directly (root or node; `describe_vocabulary`
    // resolves a node upward internally, D2). Otherwise a bare name — scan
    // the project's roots (bare names resolve ROOTS ONLY; node names are
    // unreliable, per the plan).
    let resolved_iri = if vocabulary.contains("://") {
        vocabulary.to_string()
    } else {
        // `resolved_project` is guaranteed `Some` here per step 2's fail-fast
        // check. `ok_or_else` (rather than `expect`) keeps this branch
        // panic-free per this crate's no-unwrap/no-expect convention, even
        // though the error path below is unreachable from real CLI input.
        let proj = resolved_project.as_ref().ok_or_else(|| {
            Diagnostic::Usage(
                "--project <shortcode|shortname|IRI> is required to resolve a vocabulary by name; \
                 pass a full IRI instead, or supply --project"
                    .to_string(),
            )
        })?;
        let candidates = client.list_vocabularies(&cfg.server, &proj.iri, token)?;
        let matches: Vec<&Vocabulary> = candidates
            .iter()
            .filter(|v| {
                v.header
                    .name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(vocabulary))
            })
            .collect();
        match matches.len() {
            0 => {
                let name_disp: String = vocabulary.chars().take(80).collect();
                let name_suffix = if vocabulary.chars().count() > 80 {
                    "…"
                } else {
                    ""
                };
                return Err(Diagnostic::NotFound(format!(
                    "vocabulary '{name_disp}{name_suffix}' not found in project '{}' on {server}. \
                     Run `dsp vre vocabulary list --project {} --server {server}` \
                     to see available vocabularies.",
                    proj.shortcode,
                    proj.shortcode,
                    server = cfg.server,
                )));
            }
            1 => matches[0].header.iri.clone(),
            _ => {
                let iris: Vec<String> = matches
                    .iter()
                    .take(5)
                    .map(|v| v.header.iri.clone())
                    .collect();
                let more = if matches.len() > 5 {
                    format!(" (+{} more)", matches.len() - 5)
                } else {
                    String::new()
                };
                return Err(Diagnostic::Usage(format!(
                    "vocabulary name '{vocabulary}' is ambiguous in project '{}': {} vocabularies match: {}{more}. \
                     Use a full IRI instead.",
                    proj.shortcode,
                    matches.len(),
                    iris.join(", "),
                )));
            }
        }
    };

    // ── 7. Fetch the vocabulary's full tree ─────────────────────────────────────
    let tree = client.describe_vocabulary(&cfg.server, &resolved_iri, token)?;

    // ── 8. Cross-project guard — runs whenever --project was given,
    //      UNCONDITIONALLY. `project_iri` is a plain, non-optional `String`
    //      on `VocabularyTree` (the root's `listinfo` always carries it), so
    //      there is no "absent" case that could let a mismatch pass silently.
    if let Some(ref proj) = resolved_project
        && tree.project_iri != proj.iri
    {
        let actual_name = local_name(&tree.project_iri);
        let expected_name = local_name(&proj.iri);
        let display_iri: String = resolved_iri.chars().take(80).collect();
        let iri_suffix = if resolved_iri.chars().count() > 80 {
            "…"
        } else {
            ""
        };
        return Err(Diagnostic::Usage(format!(
            "vocabulary '{display_iri}{iri_suffix}' belongs to project {actual_name}, not {expected_name}"
        )));
    }

    // ── 9. --subtree validation (POST-fetch: nothing distinguishes a root
    //      from a node before the response arrives, D14a). ───────────────────
    let subtree_of = if args.subtree {
        match &tree.requested_node {
            Some(node_iri) => Some(node_iri.clone()),
            None => {
                return Err(Diagnostic::Usage(
                    "--subtree requires a node IRI; the address you gave resolved to a \
                     vocabulary ROOT, which has no meaningful subtree — pass a node IRI \
                     instead (e.g. from `resource describe --values`)"
                        .to_string(),
                ));
            }
        }
    } else {
        // A node-IRI address WITHOUT --subtree still renders the whole
        // vocabulary with the node merely marked (D2) — subtree_of stays
        // None even when tree.requested_node is Some.
        None
    };

    // ── 10. Derive node_count/depth over what will actually be RENDERED
    //       (whole vocabulary, or the --subtree branch) — D15 invariant. ─────
    let (node_count, depth) = tree.count_and_depth(subtree_of.as_deref());

    // ── 11. Build detail + meta and render ──────────────────────────────────────
    let detail = VocabularyDetail {
        tree,
        subtree_of,
        node_count,
        depth,
    };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.vocabulary_describe(&detail, &meta)
}

/// Intentional kept-in-sync duplicate of `local_name` in `src/client/http.rs`
/// and `src/actions/vre/resource.rs`. The client's copy is private to the
/// client module; the action layer must not reach into client internals
/// (ADR-0008 layering). Do NOT introduce a shared util module — the copies
/// are adjacent enough to audit on sight.
///
/// `rsplit` always yields at least one element so `unwrap_or` is a no-panic
/// guard rather than a live fallback, mirroring the http.rs implementation.
fn local_name(iri: &str) -> String {
    iri.rsplit(['#', '/', ':'])
        .next()
        .unwrap_or(iri)
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::{run_describe_impl, run_list_impl};
    use crate::cli::{FormatArgs, VocabularyDescribeArgs, VocabularyListArgs};
    use crate::client::DspClient;
    use crate::config::Config;
    use crate::diagnostic::Diagnostic;
    use crate::model::{
        LocalizedText, ProjectRef, Vocabulary, VocabularyDetail, VocabularyHeader, VocabularyNode,
        VocabularyTree,
    };
    use crate::render::Format;
    use crate::render::auth::{
        AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome,
    };
    use crate::render::{
        DataModelListView, DumpDeleteOutcome, DumpOutcome, MetaContext, ProjectListView, Renderer,
        ResourceTypeListView, VocabularyListView,
    };

    // ── MockDspClient ─────────────────────────────────────────────────────────

    struct MockDspClient {
        resolve_result: Option<Result<ProjectRef, Diagnostic>>,
        resolve_calls: RefCell<u32>,

        list_vocabularies_result: Option<Result<Vec<Vocabulary>, Diagnostic>>,
        list_vocabularies_calls: RefCell<u32>,
        list_vocabularies_project_iri: RefCell<Option<String>>,
        list_vocabularies_token: RefCell<Option<Option<String>>>,

        // Keyed by the queried iri — lets one test configure distinct
        // per-vocabulary results (needed for `--count`'s fan-out and for
        // partial-failure tests) as well as `describe`'s single lookup.
        describe_vocabulary_results: RefCell<HashMap<String, Result<VocabularyTree, Diagnostic>>>,
        describe_vocabulary_calls: RefCell<Vec<String>>,
        describe_vocabulary_token: RefCell<Option<Option<String>>>,
    }

    impl MockDspClient {
        fn new() -> Self {
            Self {
                resolve_result: None,
                resolve_calls: RefCell::new(0),
                list_vocabularies_result: None,
                list_vocabularies_calls: RefCell::new(0),
                list_vocabularies_project_iri: RefCell::new(None),
                list_vocabularies_token: RefCell::new(None),
                describe_vocabulary_results: RefCell::new(HashMap::new()),
                describe_vocabulary_calls: RefCell::new(Vec::new()),
                describe_vocabulary_token: RefCell::new(None),
            }
        }

        fn with_resolve_project(mut self, result: Result<ProjectRef, Diagnostic>) -> Self {
            self.resolve_result = Some(result);
            self
        }

        fn with_list_vocabularies(mut self, result: Result<Vec<Vocabulary>, Diagnostic>) -> Self {
            self.list_vocabularies_result = Some(result);
            self
        }

        fn with_describe_vocabulary(
            self,
            iri: &str,
            result: Result<VocabularyTree, Diagnostic>,
        ) -> Self {
            self.describe_vocabulary_results
                .borrow_mut()
                .insert(iri.to_string(), result);
            self
        }

        fn resolve_calls(&self) -> u32 {
            *self.resolve_calls.borrow()
        }

        fn list_vocabularies_calls(&self) -> u32 {
            *self.list_vocabularies_calls.borrow()
        }

        fn list_vocabularies_token(&self) -> Option<Option<String>> {
            self.list_vocabularies_token.borrow().clone()
        }

        fn describe_vocabulary_calls(&self) -> Vec<String> {
            self.describe_vocabulary_calls.borrow().clone()
        }

        fn describe_vocabulary_token(&self) -> Option<Option<String>> {
            self.describe_vocabulary_token.borrow().clone()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in vocabulary action tests")
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
            unimplemented!("create_project_dump not used in vocabulary action tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used in vocabulary action tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used in vocabulary action tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used in vocabulary action tests")
        }

        fn list_projects(
            &self,
            _server: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Project>, Diagnostic> {
            unimplemented!("list_projects not used in vocabulary action tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used in vocabulary action tests")
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::DataModel>, Diagnostic> {
            unimplemented!("list_data_models not used in vocabulary action tests")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            unimplemented!("describe_data_model not used in vocabulary action tests")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in vocabulary action tests")
        }

        fn resource_counts(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<HashMap<String, u64>, Diagnostic> {
            unimplemented!("resource_counts not used in vocabulary action tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in vocabulary action tests")
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
            unimplemented!("list_resources not used in vocabulary action tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in vocabulary action tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used in vocabulary action tests")
        }

        fn list_vocabularies(
            &self,
            _server: &str,
            project_iri: &str,
            token: Option<&str>,
        ) -> Result<Vec<Vocabulary>, Diagnostic> {
            *self.list_vocabularies_calls.borrow_mut() += 1;
            *self.list_vocabularies_project_iri.borrow_mut() = Some(project_iri.to_string());
            *self.list_vocabularies_token.borrow_mut() = Some(token.map(str::to_owned));
            self.list_vocabularies_result
                .clone()
                .expect("list_vocabularies_result must be set when list_vocabularies is called")
        }

        fn describe_vocabulary(
            &self,
            _server: &str,
            iri: &str,
            token: Option<&str>,
        ) -> Result<VocabularyTree, Diagnostic> {
            self.describe_vocabulary_calls
                .borrow_mut()
                .push(iri.to_string());
            *self.describe_vocabulary_token.borrow_mut() = Some(token.map(str::to_owned));
            self.describe_vocabulary_results
                .borrow()
                .get(iri)
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "describe_vocabulary_results must contain an entry for '{iri}' \
                         (call with_describe_vocabulary(\"{iri}\", ...) in the test setup)"
                    )
                })
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
        vocabularies_view: Option<VocabularyListView>,
        vocabularies_meta: Option<MetaContext>,
        vocabulary_describe_detail: Option<VocabularyDetail>,
        vocabulary_describe_meta: Option<MetaContext>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                vocabularies_view: None,
                vocabularies_meta: None,
                vocabulary_describe_detail: None,
                vocabulary_describe_meta: None,
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
            _project: &crate::model::ProjectDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_models(
            &mut self,
            _view: &DataModelListView,
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
            _view: &ResourceTypeListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
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
            view: &VocabularyListView,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.vocabularies_view = Some(view.clone());
            self.vocabularies_meta = Some(meta.clone());
            Ok(())
        }

        fn vocabulary_describe(
            &mut self,
            detail: &VocabularyDetail,
            meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            self.vocabulary_describe_detail = Some(detail.clone());
            self.vocabulary_describe_meta = Some(meta.clone());
            Ok(())
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    const SERVER: &str = "https://api.test.dasch.swiss";

    fn make_cfg() -> Config {
        Config {
            server: SERVER.to_string(),
        }
    }

    fn make_project_ref(iri: &str, shortcode: &str, shortname: &str) -> ProjectRef {
        ProjectRef {
            iri: iri.to_string(),
            shortcode: shortcode.to_string(),
            shortname: shortname.to_string(),
        }
    }

    fn text(value: &str, language: Option<&str>) -> LocalizedText {
        LocalizedText {
            value: value.to_string(),
            language: language.map(str::to_owned),
        }
    }

    fn header(iri: &str, name: Option<&str>, labels: Vec<LocalizedText>) -> VocabularyHeader {
        VocabularyHeader {
            iri: iri.to_string(),
            name: name.map(str::to_owned),
            labels,
            comments: vec![],
        }
    }

    fn vocab(iri: &str, name: Option<&str>, labels: Vec<LocalizedText>) -> Vocabulary {
        Vocabulary {
            header: header(iri, name, labels),
            node_count: None,
            depth: None,
        }
    }

    fn leaf_node(iri: &str, name: &str, position: i32) -> VocabularyNode {
        VocabularyNode {
            header: header(iri, Some(name), vec![text(name, Some("en"))]),
            position,
            children: vec![],
        }
    }

    fn parent_with_child(parent_iri: &str, child_iri: &str) -> VocabularyNode {
        VocabularyNode {
            header: header(parent_iri, Some("parent"), vec![]),
            position: 0,
            children: vec![leaf_node(child_iri, "child", 0)],
        }
    }

    fn make_tree(
        root_iri: &str,
        project_iri: &str,
        requested_node: Option<&str>,
        children: Vec<VocabularyNode>,
    ) -> VocabularyTree {
        VocabularyTree {
            root: header(root_iri, Some("root"), vec![text("Root", Some("en"))]),
            children,
            project_iri: project_iri.to_string(),
            requested_node: requested_node.map(str::to_owned),
        }
    }

    fn make_list_args(project: Option<&str>) -> VocabularyListArgs {
        VocabularyListArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            filter: None,
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

    fn make_describe_args(
        vocabulary: Option<&str>,
        project: Option<&str>,
    ) -> VocabularyDescribeArgs {
        VocabularyDescribeArgs {
            server: Some(SERVER.to_string()),
            project: project.map(str::to_owned),
            vocabulary: vocabulary.map(str::to_owned),
            subtree: false,
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

    // ── list: fail-fast ───────────────────────────────────────────────────────

    #[test]
    fn list_missing_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_list_args(None);
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "got {result:?}"
        );
        assert_eq!(client.resolve_calls(), 0);
    }

    // ── describe: fail-fast ───────────────────────────────────────────────────

    #[test]
    fn describe_missing_vocabulary_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(None, Some("0001"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "got {result:?}"
        );
        assert_eq!(client.resolve_calls(), 0);
    }

    #[test]
    fn describe_bare_name_without_project_is_usage_error() {
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("epoch"), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "bare name with no --project must be Usage, got {result:?}"
        );
        assert_eq!(
            client.resolve_calls(),
            0,
            "must fail before any client call"
        );
    }

    // ── bare-name resolution ──────────────────────────────────────────────────

    #[test]
    fn describe_bare_name_resolves_to_matching_root() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let epoch_iri = "http://rdfh.ch/lists/0001/epoch";
        let epoch = vocab(epoch_iri, Some("epoch"), vec![text("Period", Some("en"))]);
        let other = vocab("http://rdfh.ch/lists/0001/other", Some("other"), vec![]);
        let tree = make_tree(
            epoch_iri,
            &proj.iri,
            None,
            vec![leaf_node(&format!("{epoch_iri}/n1"), "n1", 0)],
        );

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![epoch, other]))
            .with_describe_vocabulary(epoch_iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("epoch"), Some("0001"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(result.is_ok(), "expected Ok, got {result:?}");

        let detail = renderer
            .vocabulary_describe_detail
            .expect("vocabulary_describe must have been called");
        assert_eq!(detail.tree.root.iri, epoch_iri);
    }

    /// Same fixture as above, but via the `DSP_TOKEN` env seam (not the auth
    /// cache) — proves the token reaches `list_vocabularies` (the bare-name
    /// scan) as well as the tree fetch, and that `list_vocabularies` is
    /// called exactly once.
    #[test]
    fn describe_bare_name_forwards_env_token_to_scan_and_fetch() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let epoch_iri = "http://rdfh.ch/lists/0001/epoch";
        let epoch = vocab(epoch_iri, Some("epoch"), vec![]);
        let tree = make_tree(epoch_iri, &proj.iri, None, vec![]);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![epoch]))
            .with_describe_vocabulary(epoch_iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("epoch"), Some("0001"));
        run_describe_impl(
            &args,
            &make_cfg(),
            &client,
            &mut renderer,
            Some("env-jwt-token".to_string()),
            None,
        )
        .expect("expected Ok");

        assert_eq!(client.list_vocabularies_calls(), 1);
        assert_eq!(
            client.list_vocabularies_token(),
            Some(Some("env-jwt-token".to_string()))
        );
        assert_eq!(
            client.describe_vocabulary_token(),
            Some(Some("env-jwt-token".to_string()))
        );
    }

    #[test]
    fn describe_bare_name_ambiguous_is_usage_error() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let v1 = vocab("http://rdfh.ch/lists/0001/epoch1", Some("epoch"), vec![]);
        // Case-insensitive duplicate name under a different IRI.
        let v2 = vocab("http://rdfh.ch/lists/0001/epoch2", Some("EPOCH"), vec![]);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![v1, v2]));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("epoch"), Some("0001"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "got {result:?}"
        );
        assert_eq!(
            client.list_vocabularies_calls(),
            1,
            "the scan itself must still run exactly once"
        );
        assert_eq!(
            client.describe_vocabulary_calls().len(),
            0,
            "describe_vocabulary must not be called on an ambiguous match"
        );
    }

    // ── --filter (list) ───────────────────────────────────────────────────────

    #[test]
    fn list_filter_matches_non_english_only_label() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        // No English label at all — proves the filter isn't accidentally
        // English-only (D4).
        let epoch = vocab(
            "http://rdfh.ch/lists/0001/epoch",
            Some("epoch"),
            vec![text("A3 Periode", Some("de"))],
        );
        let other = vocab(
            "http://rdfh.ch/lists/0001/other",
            Some("other"),
            vec![text("Something else", Some("en"))],
        );

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![epoch.clone(), other]));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_list_args(Some("0001"));
        args.filter = Some("periode".to_string());
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .vocabularies_view
            .expect("vocabularies must have been called");
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].header.iri, epoch.header.iri);
        assert_eq!(view.total, 2, "total is pre-filter");
    }

    // ── --count (list) ────────────────────────────────────────────────────────

    #[test]
    fn list_count_fills_node_count_and_depth() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let v1 = vocab("http://rdfh.ch/lists/0001/v1", Some("aaa"), vec![]);
        let v2 = vocab("http://rdfh.ch/lists/0001/v2", Some("bbb"), vec![]);
        // v1: 2 leaf children -> 2 nodes, depth 1.
        let tree1 = make_tree(
            "http://rdfh.ch/lists/0001/v1",
            &proj.iri,
            None,
            vec![
                leaf_node("http://rdfh.ch/lists/0001/v1/n1", "n1", 0),
                leaf_node("http://rdfh.ch/lists/0001/v1/n2", "n2", 1),
            ],
        );
        // v2: one parent + one child -> 2 nodes, depth 2.
        let tree2 = make_tree(
            "http://rdfh.ch/lists/0001/v2",
            &proj.iri,
            None,
            vec![parent_with_child(
                "http://rdfh.ch/lists/0001/v2/p1",
                "http://rdfh.ch/lists/0001/v2/p1/c1",
            )],
        );

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![v1, v2]))
            .with_describe_vocabulary("http://rdfh.ch/lists/0001/v1", Ok(tree1))
            .with_describe_vocabulary("http://rdfh.ch/lists/0001/v2", Ok(tree2));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_list_args(Some("0001"));
        args.count = true;
        run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None).expect("expected Ok");

        let view = renderer
            .vocabularies_view
            .expect("vocabularies must have been called");
        let item1 = view
            .items
            .iter()
            .find(|i| i.header.iri == "http://rdfh.ch/lists/0001/v1")
            .unwrap();
        assert_eq!(item1.node_count, Some(2));
        assert_eq!(item1.depth, Some(1));
        let item2 = view
            .items
            .iter()
            .find(|i| i.header.iri == "http://rdfh.ch/lists/0001/v2")
            .unwrap();
        assert_eq!(item2.node_count, Some(2));
        assert_eq!(item2.depth, Some(2));

        let meta = renderer.vocabularies_meta.expect("meta must be present");
        assert!(meta.count_cost.is_some());
    }

    #[test]
    fn list_count_partial_failure_degrades_and_discloses() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let v1 = vocab("http://rdfh.ch/lists/0001/v1", Some("aaa"), vec![]);
        let v2 = vocab("http://rdfh.ch/lists/0001/v2", Some("bbb"), vec![]);
        let tree1 = make_tree(
            "http://rdfh.ch/lists/0001/v1",
            &proj.iri,
            None,
            vec![leaf_node("http://rdfh.ch/lists/0001/v1/n1", "n1", 0)],
        );

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![v1, v2]))
            .with_describe_vocabulary("http://rdfh.ch/lists/0001/v1", Ok(tree1))
            .with_describe_vocabulary(
                "http://rdfh.ch/lists/0001/v2",
                Err(Diagnostic::ServerError("boom".to_string())),
            );

        let mut renderer = RecordingRenderer::new();
        let mut args = make_list_args(Some("0001"));
        args.count = true;
        let result = run_list_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            result.is_ok(),
            "a partial per-tree failure must not abort the whole command, got {result:?}"
        );

        let view = renderer
            .vocabularies_view
            .expect("vocabularies must have been called");
        let item1 = view
            .items
            .iter()
            .find(|i| i.header.iri == "http://rdfh.ch/lists/0001/v1")
            .unwrap();
        assert_eq!(item1.node_count, Some(1));
        let item2 = view
            .items
            .iter()
            .find(|i| i.header.iri == "http://rdfh.ch/lists/0001/v2")
            .unwrap();
        assert_eq!(item2.node_count, None);
        assert_eq!(item2.depth, None);

        let meta = renderer.vocabularies_meta.expect("meta must be present");
        let cost = meta
            .count_cost
            .expect("count_cost must be Some under --count");
        assert!(
            cost.contains("1 of 2"),
            "count_cost must disclose how many of how many failed, got: {cost}"
        );
    }

    // ── cross-project guard (describe) ───────────────────────────────────────

    #[test]
    fn describe_cross_project_guard_fires_even_for_bare_name_address() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let epoch_iri = "http://rdfh.ch/lists/0001/epoch";
        let epoch = vocab(epoch_iri, Some("epoch"), vec![]);
        // Deliberately mismatched project_iri: proves the guard is never
        // skipped just because the address was resolved via this project's
        // own list_vocabularies call (a bare-name address "trivially matches").
        let tree = make_tree(epoch_iri, "http://rdfh.ch/projects/9999", None, vec![]);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![epoch]))
            .with_describe_vocabulary(epoch_iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some("epoch"), Some("0001"));
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "cross-project guard must fire even on a bare-name address, got {result:?}"
        );
    }

    #[test]
    fn describe_project_omitted_with_full_iri_succeeds_no_guard() {
        let iri = "http://rdfh.ch/lists/0001/epoch";
        let tree = make_tree(iri, "http://rdfh.ch/projects/0001", None, vec![]);
        let client = MockDspClient::new().with_describe_vocabulary(iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let args = make_describe_args(Some(iri), None);
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(result.is_ok(), "got {result:?}");
        assert_eq!(
            client.resolve_calls(),
            0,
            "resolve_project must not be called when --project is omitted"
        );
    }

    // ── filter_warning always None ────────────────────────────────────────────

    #[test]
    fn filter_warning_is_always_none_on_list_and_describe() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");

        let list_client = MockDspClient::new()
            .with_resolve_project(Ok(proj.clone()))
            .with_list_vocabularies(Ok(vec![]));
        let mut list_renderer = RecordingRenderer::new();
        let list_args = make_list_args(Some("0001"));
        run_list_impl(
            &list_args,
            &make_cfg(),
            &list_client,
            &mut list_renderer,
            None,
            None,
        )
        .expect("expected Ok");
        let list_meta = list_renderer
            .vocabularies_meta
            .expect("meta must be present");
        assert_eq!(list_meta.filter_warning, None);

        let iri = "http://rdfh.ch/lists/0001/epoch";
        let tree = make_tree(iri, &proj.iri, None, vec![]);
        let describe_client = MockDspClient::new().with_describe_vocabulary(iri, Ok(tree));
        let mut describe_renderer = RecordingRenderer::new();
        let describe_args = make_describe_args(Some(iri), None);
        run_describe_impl(
            &describe_args,
            &make_cfg(),
            &describe_client,
            &mut describe_renderer,
            None,
            None,
        )
        .expect("expected Ok");
        let describe_meta = describe_renderer
            .vocabulary_describe_meta
            .expect("meta must be present");
        assert_eq!(describe_meta.filter_warning, None);
    }

    // ── --subtree ─────────────────────────────────────────────────────────────

    #[test]
    fn describe_subtree_with_root_address_is_usage_error_even_on_success() {
        let iri = "http://rdfh.ch/lists/0001/epoch";
        // A successful ROOT response (requested_node: None) — the action must
        // still reject --subtree here, per the plan's explicit instruction.
        let tree = make_tree(
            iri,
            "http://rdfh.ch/projects/0001",
            None,
            vec![leaf_node(&format!("{iri}/n1"), "n1", 0)],
        );
        let client = MockDspClient::new().with_describe_vocabulary(iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some(iri), None);
        args.subtree = true;
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "--subtree on a root address must be a usage error even though the fetch \
             succeeded, got {result:?}"
        );
    }

    #[test]
    fn describe_subtree_with_bare_name_address_is_usage_error() {
        let proj = make_project_ref("http://rdfh.ch/projects/0001", "0001", "geoarch");
        let epoch_iri = "http://rdfh.ch/lists/0001/epoch";
        let epoch = vocab(epoch_iri, Some("epoch"), vec![]);
        let tree = make_tree(epoch_iri, &proj.iri, None, vec![]);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(proj))
            .with_list_vocabularies(Ok(vec![epoch]))
            .with_describe_vocabulary(epoch_iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some("epoch"), Some("0001"));
        args.subtree = true;
        let result = run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "got {result:?}"
        );
    }

    #[test]
    fn describe_subtree_with_node_address_narrows_correctly() {
        let node_iri = "http://rdfh.ch/lists/0001/epoch/n1";
        let tree = make_tree(
            "http://rdfh.ch/lists/0001/epoch",
            "http://rdfh.ch/projects/0001",
            Some(node_iri),
            vec![leaf_node(node_iri, "n1", 0)],
        );
        let client = MockDspClient::new().with_describe_vocabulary(node_iri, Ok(tree));

        let mut renderer = RecordingRenderer::new();
        let mut args = make_describe_args(Some(node_iri), None);
        args.subtree = true;
        run_describe_impl(&args, &make_cfg(), &client, &mut renderer, None, None)
            .expect("expected Ok");

        let detail = renderer
            .vocabulary_describe_detail
            .expect("vocabulary_describe must have been called");
        assert_eq!(detail.subtree_of.as_deref(), Some(node_iri));
        assert_eq!(detail.node_count, 1);
        assert_eq!(detail.depth, 1);
    }
}
