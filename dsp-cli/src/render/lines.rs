//! Lines renderer — one identifier per line.
//!
//! Lines output emits one identifier per line for easy shell pipeline
//! consumption (e.g. piping into `xargs`). Auth methods emit:
//! - `auth_login` / `auth_status` / `auth_set_token`: the server URL.
//! - `auth_logout`: `<server>\twas_cached=true|false` (tab-separated, bespoke decorated default;
//!   projected path goes through the engine).
//!
//! Per-noun methods arrive with real data: `project_dump` in Phase 3,
//! `projects` (list) in Phase 4; other noun-groups (data-models, etc.) in Phase 5.

use std::io::{self, Write};

use crate::diagnostic::Diagnostic;
use crate::model::{DataModelDetail, DataModelStructure, ProjectDetail, VocabularyDetail};
use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
use crate::render::dump::{DumpDeleteOutcome, DumpOutcome};
use crate::render::table::render_table_disclosure;
use crate::render::value::{build_metadata_row, build_value_rows};
use crate::render::vocabulary::{build_vocabulary_list_rows, build_vocabulary_rows};
use crate::render::{
    AUTH_LOGIN_COLUMNS, AUTH_LOGOUT_COLUMNS, DATA_MODEL_DESCRIBE_COLUMNS, DATA_MODEL_STRUCTURE_COLUMNS,
    DATA_MODELS_COLUMNS, DataModelListView, HeaderMode, MetaContext, PROJECT_DUMP_COLUMNS,
    PROJECT_DUMP_DELETED_COLUMNS, PROJECTS_COLUMNS, ProjectListView, QuoteMode, RESOURCE_DESCRIBE_COLUMNS,
    RESOURCE_DESCRIBE_VALUES_COLUMNS, RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS, RESOURCE_LIST_COLUMNS,
    RESOURCE_TYPE_DESCRIBE_COLUMNS, RESOURCE_TYPES_COLUMNS, Renderer, ResourceListView, ResourceTypeListView,
    TableOptions, TableSpec, VOCABULARIES_COLUMNS, VOCABULARY_DESCRIBE_COLUMNS, VocabularyListView, render_table,
};
use crate::util::text::replace_control_chars;

/// Renders output as one identifier per line.
pub struct LinesRenderer {
    out: Box<dyn Write>,
    /// Auth-state disclosure sink. In production (`new()`), this is
    /// `io::stderr()`. In tests using `with_writer`, it is `io::sink()` so
    /// existing tests don't emit to real stderr. Use `with_writers` to capture
    /// both streams in new tabular-disclosure tests.
    err: Box<dyn Write>,
    /// Per-invocation tabular options (column projection).
    /// Defaults to all columns selected from the lean default subset — matches
    /// today's unflagged lean behaviour.
    options: TableOptions,
}

impl LinesRenderer {
    /// Creates a renderer writing to stdout (data) and stderr (disclosure).
    pub fn new() -> Self {
        Self {
            out: Box::new(io::stdout()),
            err: Box::new(io::stderr()),
            options: TableOptions::default(),
        }
    }

    /// Creates a renderer writing data to `w`; discards disclosure (stderr
    /// becomes `io::sink()`). Keeps all existing tabular tests working without
    /// emitting to real stderr.
    pub fn with_writer(w: impl Write + 'static) -> Self {
        Self {
            out: Box::new(w),
            err: Box::new(io::sink()),
            options: TableOptions::default(),
        }
    }

    /// Creates a renderer writing data to `out` and disclosure to `err`.
    /// Used by tests that assert the stderr auth-state line.
    pub fn with_writers(out: impl Write + 'static, err: impl Write + 'static) -> Self {
        Self {
            out: Box::new(out),
            err: Box::new(err),
            options: TableOptions::default(),
        }
    }

    /// Returns a new renderer with the given tabular options applied.
    /// Used by `Format::into_renderer_with_options` and tests.
    pub fn with_options(mut self, opts: TableOptions) -> Self {
        self.options = opts;
        self
    }
}

impl Default for LinesRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for LinesRenderer {
    fn diagnostic(&mut self, diag: &Diagnostic, _meta: &MetaContext) -> Result<(), Diagnostic> {
        eprintln!("Error: {diag}"); // errors go to stderr for non-JSON formats (ADR-0012)
        Ok(())
    }

    fn auth_login(&mut self, outcome: &AuthLoginOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        let expires = outcome.expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let rows = vec![vec![
            outcome.server.clone(),
            outcome.user.clone(),
            expires,
            "login_success".to_string(),
        ]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: AUTH_LOGIN_COLUMNS,
            rows: &rows,
            default_columns: Some(&["server"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)
    }

    fn auth_status(&mut self, outcome: &AuthStatusOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let row = match outcome {
            AuthStatusOutcome::LoggedIn { server, user, expires_at, expired } => {
                let user_str = user.as_deref().unwrap_or("").to_string();
                let expires = expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
                let state = if *expired { "expired" } else { "logged_in" };
                vec![server.clone(), user_str, expires, state.to_string()]
            }
            // DSP_TOKEN env-override: row shape is uniform with LoggedIn (user empty,
            // expires_at rfc3339 or empty, state "logged_in"|"expired"). The source
            // is never a stdout column.
            AuthStatusOutcome::AuthenticatedViaEnv { server, expires_at, expired } => {
                let expires = expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
                let state = if *expired { "expired" } else { "logged_in" };
                vec![server.clone(), String::new(), expires, state.to_string()]
            }
            AuthStatusOutcome::NotLoggedIn { server } => {
                vec![
                    server.clone(),
                    String::new(),
                    String::new(),
                    "not_logged_in".to_string(),
                ]
            }
        };
        let rows = vec![row];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: AUTH_LOGIN_COLUMNS,
            rows: &rows,
            default_columns: Some(&["server"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        // Sanctioned engine bypass (plan 020 D6): the lines default emits the
        // decorated literal `<server>\twas_cached=true|false`, which the engine
        // cannot reproduce.  When --columns is active (projection path), plain
        // values go through the engine with replace_control_chars applied.
        //
        // The server field is passed through replace_control_chars even on the
        // bespoke path — byte-identical for all normal server URLs (which
        // contain no ASCII control chars), but correct-by-construction
        // regardless of what the server string contains.
        if self.options.columns.is_none() {
            // Default (decorated) path — bespoke writeln!.
            writeln!(
                self.out,
                "{}\twas_cached={}",
                replace_control_chars(&outcome.server),
                outcome.was_cached
            )?;
        } else {
            // Projected path — engine with plain was_cached value ("true"/"false").
            let rows = vec![vec![outcome.server.clone(), outcome.was_cached.to_string()]];
            let projected = self.options.projected();
            let spec = TableSpec {
                all_columns: AUTH_LOGOUT_COLUMNS,
                rows: &rows,
                default_columns: None,
                projected,
                quote: QuoteMode::Lines,
                header: HeaderMode::Off,
            };
            render_table(&mut *self.out, &spec)?;
        }
        Ok(())
    }

    fn auth_set_token(&mut self, outcome: &AuthSetTokenOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        let user_str = outcome.user.as_deref().unwrap_or("").to_string();
        let expires = outcome.expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let rows = vec![vec![
            outcome.server.clone(),
            user_str,
            expires,
            "token_cached".to_string(),
        ]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: AUTH_LOGIN_COLUMNS,
            rows: &rows,
            default_columns: Some(&["server"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)
    }

    fn project_dump(&mut self, outcome: &DumpOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: bare path only; cleanup disclosure is omitted (prose/json only).
        // reused/created_at are also prose/json only.
        let rows = vec![vec![outcome.path.display().to_string()]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECT_DUMP_COLUMNS,
            rows: &rows,
            default_columns: Some(&["path"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)
    }

    fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: one token — "deleted" when true, "not-deleted" otherwise.
        // Note: csv builds true/false for the same `deleted` column; lines builds
        // "deleted"/"not-deleted". Each renderer owns its own row values, so this
        // divergence is acceptable — it preserves byte-identity for both formats'
        // existing snapshots.
        let token = if outcome.deleted {
            "deleted".to_string()
        } else {
            "not-deleted".to_string()
        };
        let rows = vec![vec![token]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECT_DUMP_DELETED_COLUMNS,
            rows: &rows,
            default_columns: Some(&["deleted"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)
    }

    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: shortcode TAB shortname TAB longname per item, no header.
        // longname None → empty string (yields a trailing tab — locked by fixture).
        let rows: Vec<Vec<String>> = view
            .items
            .iter()
            .map(|item| {
                let longname = item.longname.as_deref().unwrap_or("").to_string();
                let data_models_str = item.data_models.to_string();
                vec![
                    item.shortcode.clone(),
                    item.shortname.clone(),
                    longname,
                    item.status.as_str().to_string(),
                    data_models_str,
                    item.iri.clone(),
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECTS_COLUMNS,
            rows: &rows,
            default_columns: Some(&["shortcode", "shortname", "longname"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: single row shortcode TAB shortname TAB longname, no header.
        // longname None → empty string (trailing tab — mirrors `projects` lines behaviour).
        let longname = project.longname.as_deref().unwrap_or("").to_string();
        let data_models_str = project.data_models.len().to_string();
        let rows = vec![vec![
            project.shortcode.clone(),
            project.shortname.clone(),
            longname,
            project.status.as_str().to_string(),
            data_models_str,
            project.iri.clone(),
        ]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECTS_COLUMNS,
            rows: &rows,
            default_columns: Some(&["shortcode", "shortname", "longname"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: name TAB iri per item, no header.
        // Deliberately omits label/last_modified/is_builtin — lean chaining format.
        let rows: Vec<Vec<String>> = view
            .items
            .iter()
            .map(|item| {
                let label = item.label.as_deref().unwrap_or("").to_string();
                let last_modified = item.last_modified.as_deref().unwrap_or("").to_string();
                let is_builtin_str = if item.is_builtin { "true" } else { "false" };
                vec![
                    item.name.clone(),
                    item.iri.clone(),
                    label,
                    last_modified,
                    is_builtin_str.to_string(),
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: DATA_MODELS_COLUMNS,
            rows: &rows,
            default_columns: Some(&["name", "iri"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: single row name TAB iri, no header.
        // Resource-types are omitted — lean chaining format (mirrors project_describe lines).
        let label = detail.label.as_deref().unwrap_or("").to_string();
        let last_modified = detail.last_modified.as_deref().unwrap_or("").to_string();
        let resource_types_str = detail.resource_types.len().to_string();
        let rows = vec![vec![
            detail.name.clone(),
            detail.iri.clone(),
            label,
            last_modified,
            resource_types_str,
        ]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: DATA_MODEL_DESCRIBE_COLUMNS,
            rows: &rows,
            default_columns: Some(&["name", "iri"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: name TAB iri per item, no header.
        // Deliberately omits label/is_builtin/count from the default — lean
        // chaining format (mirrors data_models). Unlike csv/tsv, the default
        // stays fixed regardless of `--count` (plan 030); `count` is reachable
        // via explicit `--columns`.
        let rows: Vec<Vec<String>> = view
            .items
            .iter()
            .map(|item| {
                let label = item.label.as_deref().unwrap_or("").to_string();
                let is_builtin_str = if item.is_builtin { "true" } else { "false" };
                let count_str = item.count.map(|c| c.to_string()).unwrap_or_default();
                vec![
                    item.name.clone(),
                    item.iri.clone(),
                    label,
                    is_builtin_str.to_string(),
                    count_str,
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: RESOURCE_TYPES_COLUMNS,
            rows: &rows,
            default_columns: Some(&["name", "iri"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resource_type_describe(
        &mut self,
        detail: &crate::model::ResourceTypeDetail,
        meta: &MetaContext,
    ) -> Result<(), Diagnostic> {
        // Lines format: one row per field — name TAB iri, no header.
        // Mirrors data_model_describe lines (single identifier rows, lean chaining).
        //
        // Uses the shared RESOURCE_TYPE_DESCRIBE_COLUMNS (which now includes `iri`
        // at position 1), with `default_columns: Some(&["name", "iri"])` for the
        // lean path. csv/tsv use the same const with their own default_columns that
        // omit `iri` from the unflagged output (byte-identical to pre-020 csv/tsv).
        let rows: Vec<Vec<String>> = detail
            .fields
            .iter()
            .map(|field| {
                let value_type_str = field.value_type.to_string();
                let link_target = field.link_target.as_deref().unwrap_or("").to_string();
                let card_str = field.cardinality.to_string();
                let label = field.label.as_deref().unwrap_or("").to_string();
                let is_builtin_str = if field.is_builtin { "true" } else { "false" };
                let data_model = field.data_model.as_deref().unwrap_or("").to_string();
                vec![
                    field.name.clone(),
                    field.iri.clone(),
                    value_type_str,
                    link_target,
                    card_str,
                    label,
                    is_builtin_str.to_string(),
                    data_model,
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: RESOURCE_TYPE_DESCRIBE_COLUMNS,
            rows: &rows,
            default_columns: Some(&["name", "iri"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_model_structure(&mut self, structure: &DataModelStructure, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: one relation per line, tab-separated: source TAB target TAB kind TAB field.
        // field is empty for inherits relations. target_data_model is intentionally omitted
        // (lean pipe-friendly format — use csv/tsv/json for the full column set). No header.
        let rows: Vec<Vec<String>> = structure
            .relations
            .iter()
            .map(|rel| {
                let kind_str = rel.kind.to_string();
                let field = rel.field.as_deref().unwrap_or("").to_string();
                let target_dm = rel.target_data_model.as_deref().unwrap_or("").to_string();
                vec![rel.source.clone(), rel.target.clone(), kind_str, field, target_dm]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: DATA_MODEL_STRUCTURE_COLUMNS,
            rows: &rows,
            default_columns: Some(&["source", "target", "kind", "field"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        let rows: Vec<Vec<String>> = view
            .items
            .iter()
            .map(|item| {
                vec![
                    item.label.clone(),
                    item.iri.clone(),
                    item.ark_url.as_deref().unwrap_or("").to_string(),
                    item.creation_date.as_deref().unwrap_or("").to_string(),
                    item.last_modified.as_deref().unwrap_or("").to_string(),
                    item.resource_type.clone(),
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: RESOURCE_LIST_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resource_describe(
        &mut self,
        detail: &crate::model::ResourceDetail,
        meta: &MetaContext,
    ) -> Result<(), Diagnostic> {
        // Lines format: `detail.values` selects the shape (ADR-0013 D1).
        // None → the metadata row (single row, label TAB iri lean default), unchanged.
        // Some(fields) → one row per value (long-format), metadata row dropped.
        match &detail.values {
            None => {
                let rows = vec![build_metadata_row(detail)];
                let projected = self.options.projected();
                let spec = TableSpec {
                    all_columns: RESOURCE_DESCRIBE_COLUMNS,
                    rows: &rows,
                    default_columns: Some(&["label", "iri"]),
                    projected,
                    quote: QuoteMode::Lines,
                    header: HeaderMode::Off,
                };
                render_table(&mut *self.out, &spec)?;
            }
            Some(fields) => {
                let rows = build_value_rows(&detail.label, &detail.iri, fields);
                let projected = self.options.projected();
                let spec = TableSpec {
                    all_columns: RESOURCE_DESCRIBE_VALUES_COLUMNS,
                    rows: &rows,
                    default_columns: Some(RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS),
                    projected,
                    quote: QuoteMode::Lines,
                    header: HeaderMode::Off,
                };
                render_table(&mut *self.out, &spec)?;
            }
        }
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn vocabularies(&mut self, view: &VocabularyListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: name TAB iri per item, no header — lean chaining
        // format, language-neutral (D4 forbids picking a language even for
        // the default). `nodes`/`depth` are reachable via `--columns`.
        let rows = build_vocabulary_list_rows(&view.items);
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: VOCABULARIES_COLUMNS,
            rows: &rows,
            default_columns: Some(&["name", "iri"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn vocabulary_describe(&mut self, detail: &VocabularyDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Lines format: one row per node — node_iri TAB number, no header.
        // `number` (language-neutral, D4) is the one column that makes the
        // hierarchy legible in a flat format.
        let rows = build_vocabulary_rows(detail);
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: VOCABULARY_DESCRIBE_COLUMNS,
            rows: &rows,
            default_columns: Some(&["node_iri", "number"]),
            projected,
            quote: QuoteMode::Lines,
            header: HeaderMode::Off,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DataModel, DataModelDetail, Project, ProjectDetail, ProjectStatus, ResourceType, ResourceTypeSummary,
    };
    use crate::render::test_support::{SharedBuf, make_meta};
    use crate::render::{DataModelListView, ResourceTypeListView};

    fn make_fixture() -> Vec<Project> {
        vec![
            Project {
                iri: "http://rdfh.ch/projects/0001".into(),
                shortcode: "0001".into(),
                shortname: "anything".into(),
                longname: Some("Anything Project".into()),
                status: ProjectStatus::Active,
                data_models: 2,
            },
            Project {
                iri: "http://rdfh.ch/projects/0002".into(),
                shortcode: "0002".into(),
                shortname: "images".into(),
                longname: None,
                status: ProjectStatus::Inactive,
                data_models: 0,
            },
            Project {
                iri: "http://rdfh.ch/projects/0803".into(),
                shortcode: "0803".into(),
                shortname: "daschland".into(),
                longname: Some("DaSCHland Project".into()),
                status: ProjectStatus::Active,
                data_models: 1,
            },
        ]
    }

    #[test]
    fn projects_lines_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = ProjectListView { items: make_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let stdout = out.string();
        // No header
        assert!(!stdout.starts_with("shortcode"));
        // Three rows: shortcode TAB shortname TAB longname
        assert!(stdout.contains("0001\tanything\tAnything Project\n"));
        // None longname → trailing tab (empty third field)
        assert!(stdout.contains("0002\timages\t\n"));
        assert!(stdout.contains("0803\tdaschland\tDaSCHland Project\n"));
    }

    #[test]
    fn projects_lines_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = ProjectListView { items: make_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        assert!(!out.string().contains("[anonymous"));
    }

    #[test]
    fn auth_status_lines_stderr_disclosure() {
        // AuthenticatedViaEnv — the env-vs-cache gap this fix closes
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("authenticated via DSP_TOKEN", "https://api.prod.dasch.swiss");
        renderer
            .auth_status(
                &AuthStatusOutcome::AuthenticatedViaEnv {
                    server: "https://api.prod.dasch.swiss".to_string(),
                    expires_at: None,
                    expired: false,
                },
                &meta,
            )
            .unwrap();
        let err_str = err.string();
        assert_eq!(err_str.trim(), "[authenticated via DSP_TOKEN on https://api.prod.dasch.swiss]");
        // Disclosure must not appear on stdout
        assert!(!out.string().contains('['));

        // NotLoggedIn — disclosure emitted for every outcome
        let out2 = SharedBuf::new();
        let err2 = SharedBuf::new();
        let mut renderer2 = LinesRenderer::with_writers(out2.clone(), err2.clone());
        let meta2 = make_meta("anonymous", "https://api.prod.dasch.swiss");
        renderer2
            .auth_status(
                &AuthStatusOutcome::NotLoggedIn { server: "https://api.prod.dasch.swiss".to_string() },
                &meta2,
            )
            .unwrap();
        let err2_str = err2.string();
        assert_eq!(err2_str.trim(), "[anonymous on https://api.prod.dasch.swiss]");
        // Disclosure must not appear on stdout (symmetry with the env case above)
        assert!(!out2.string().contains('['));
    }

    #[test]
    fn projects_lines_with_writer_no_stderr_leak() {
        let out = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writer(out.clone());
        let view = ProjectListView { items: vec![], total: 0, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();
        // Empty list → no data rows; disclosure discarded to sink
        assert_eq!(out.string(), "");
    }

    #[test]
    fn project_describe_lines_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            status: ProjectStatus::Active,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // Single row: shortcode TAB shortname TAB longname
        assert!(stdout.contains("0801\tbeol\tBernoulli-Euler Online\n"));
        // No header
        assert!(!stdout.starts_with("shortcode"));
    }

    #[test]
    fn project_describe_lines_no_longname_trailing_tab() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0000".into(),
            shortcode: "0000".into(),
            shortname: "minimal".into(),
            longname: None,
            status: ProjectStatus::Inactive,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // None longname → trailing tab (empty third field)
        assert!(stdout.contains("0000\tminimal\t\n"));
    }

    #[test]
    fn project_describe_lines_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0001".into(),
            shortcode: "0001".into(),
            shortname: "test".into(),
            longname: None,
            status: ProjectStatus::Active,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        // Disclosure must not appear on stdout
        assert!(!out.string().contains("[anonymous"));
    }

    fn make_data_model_fixture() -> Vec<DataModel> {
        vec![
            DataModel {
                name: "beol".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                label: Some("The BEOL data-model".into()),
                last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
                is_builtin: false,
            },
            DataModel {
                name: "biblio".into(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                label: None,
                last_modified: None,
                is_builtin: false,
            },
            DataModel {
                name: "knora-api".into(),
                iri: "http://api.knora.org/ontology/knora-api/v2".into(),
                label: None,
                last_modified: None,
                is_builtin: true,
            },
        ]
    }

    #[test]
    fn data_models_lines_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let stdout = out.string();
        // No header
        assert!(!stdout.starts_with("name"));
        // name TAB iri rows
        assert!(stdout.contains("beol\thttp://api.dasch.swiss/ontology/0801/beol/v2\n"));
        assert!(stdout.contains("biblio\thttp://api.dasch.swiss/ontology/0801/biblio/v2\n"));
        assert!(stdout.contains("knora-api\thttp://api.knora.org/ontology/knora-api/v2\n"));
        // label/last_modified/is_builtin are NOT in lines output
        assert!(!stdout.contains("The BEOL"));
        assert!(!stdout.contains("2024-05-27"));
        assert!(!stdout.contains("true"));
    }

    #[test]
    fn data_models_lines_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        // Disclosure must not appear on stdout
        assert!(!out.string().contains("[anonymous"));
    }

    // ── data_model_describe lines tests ──────────────────────────────────────

    fn make_beol_dm_detail() -> DataModelDetail {
        DataModelDetail {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
            label: Some("The BEOL data-model".into()),
            last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
            resource_types: vec![
                ResourceTypeSummary {
                    name: "Archive".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                    label: Some("Archive".into()),
                },
                ResourceTypeSummary {
                    name: "basicLetter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#basicLetter".into(),
                    label: None,
                },
                ResourceTypeSummary {
                    name: "letter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                    label: Some("Letter".into()),
                },
            ],
        }
    }

    #[test]
    fn data_model_describe_lines_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let stdout = out.string();
        // Single row: name TAB iri
        assert!(
            stdout.contains("beol\thttp://api.dasch.swiss/ontology/0801/beol/v2\n"),
            "expected name TAB iri row; got:\n{stdout}"
        );
        // No header
        assert!(!stdout.starts_with("name"));
        // resource-types must NOT appear in lines output
        assert!(!stdout.contains("Archive"), "resource-types must be omitted");
    }

    #[test]
    fn data_model_describe_lines_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        // Disclosure must not appear on stdout
        assert!(!out.string().contains("[anonymous"));
    }

    // ── resource_types lines tests ────────────────────────────────────────────

    fn make_resource_type_fixture() -> Vec<ResourceType> {
        vec![
            ResourceType {
                name: "Archive".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                label: Some("Archive".into()),
                is_builtin: false,
                count: None,
            },
            ResourceType {
                name: "letter".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                label: Some("Letter".into()),
                is_builtin: false,
                count: None,
            },
        ]
    }

    #[test]
    fn resource_types_lines_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = ResourceTypeListView {
            items: make_resource_type_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        // No header
        assert!(!stdout.starts_with("name"));
        // name TAB iri rows
        assert!(stdout.contains("Archive\thttp://api.dasch.swiss/ontology/0801/beol/v2#Archive\n"));
        assert!(stdout.contains("letter\thttp://api.dasch.swiss/ontology/0801/beol/v2#letter\n"));
        // label/is_builtin NOT in lines output
        assert!(!stdout.contains("Letter"));
        assert!(!stdout.contains("true"));
        assert!(!stdout.contains("false"));
    }

    #[test]
    fn resource_types_lines_with_builtins() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let mut items = make_resource_type_fixture();
        items.push(ResourceType {
            name: "Region".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
            label: Some("Region".into()),
            is_builtin: true,
            count: None,
        });
        let view = ResourceTypeListView { items, total: 3, filter: None, data_model: "beol".into() };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        // built-in appears as a plain name TAB iri row
        assert!(stdout.contains("Region\thttp://api.knora.org/ontology/knora-api/v2#Region\n"));
        // no is_builtin column
        assert!(!stdout.contains("true"));
    }

    #[test]
    fn resource_types_lines_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = ResourceTypeListView {
            items: make_resource_type_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        assert!(!out.string().contains("[anonymous"));
    }

    #[test]
    fn resource_types_lines_empty() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let view = ResourceTypeListView {
            items: vec![],
            total: 0,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        // No data rows; disclosure still emitted to stderr
        assert_eq!(out.string(), "");
        assert!(err.string().contains("[anonymous on https://api.test.dasch.swiss]"));
    }

    #[test]
    fn resource_types_lines_default_unchanged_count_only_via_columns() {
        // plan 030: lines' default stays fixed at ["name", "iri"] regardless
        // of --count (unlike csv/tsv, which auto-show). `count` is only
        // reachable via explicit --columns.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let mut items = make_resource_type_fixture();
        items[0].count = Some(10);
        let view = ResourceTypeListView { items, total: 2, filter: None, data_model: "beol".into() };
        let meta = crate::render::MetaContext {
            server_label: "https://api.test.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts are not permission-filtered".into()),
            count_cost: None,
        };
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        // Default output stays name TAB iri only — no count cell leaks in.
        assert!(stdout.contains("Archive\thttp://api.dasch.swiss/ontology/0801/beol/v2#Archive\n"));
        assert!(
            !stdout.contains("10"),
            "count must not leak into default output; got:\n{stdout}"
        );
        assert!(
            err.string().contains("counts are not permission-filtered"),
            "stderr must carry count_caveat; got: {:?}",
            err.string()
        );
    }

    #[test]
    fn resource_types_lines_columns_count_explicit_projection() {
        let out = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writer(out.clone()).with_options(TableOptions {
            columns: Some(vec!["name".to_string(), "count".to_string()]),
            header: HeaderMode::Off,
        });
        let mut items = make_resource_type_fixture();
        items[0].count = Some(10);
        let view = ResourceTypeListView { items, total: 2, filter: None, data_model: "beol".into() };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        assert!(stdout.contains("Archive\t10\n"), "got:\n{stdout}");
        // letter has no count -> empty cell after the tab.
        assert!(stdout.contains("letter\t\n"), "got:\n{stdout}");
    }

    #[test]
    fn resource_type_describe_lines_columns_no_count_column_exists() {
        // Part G scope note: resource_type_describe is untouched by this
        // step — `count` (a resource-type-level field) has no column in this
        // row-per-field table. `--columns count` must be a usage error.
        let out = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writer(out.clone()).with_options(TableOptions {
            columns: Some(vec!["count".to_string()]),
            header: HeaderMode::Off,
        });
        let detail = crate::model::ResourceTypeDetail {
            name: "manuscript".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".into(),
            label: Some("Manuscript".into()),
            data_model: "beol".into(),
            representation: None,
            super_types: vec![],
            fields: vec![],
            count: Some(7),
        };
        let meta = make_meta("anonymous", "https://api.dasch.swiss");
        let result = renderer.resource_type_describe(&detail, &meta);
        assert!(result.is_err(), "count is not a valid column for resource_type_describe");
    }

    // ── auth_logout lines tests (D6 bypass + projected path) ─────────────────

    #[test]
    fn auth_logout_lines_default_decorated_output() {
        // Default path: bespoke writeln! produces <server>\twas_cached=<bool>.
        let out = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writer(out.clone());
        renderer
            .auth_logout(
                &AuthLogoutOutcome {
                    server: "https://api.example.com".to_string(),
                    was_cached: true,
                },
                &make_meta("anonymous", "https://api.example.com"),
            )
            .unwrap();
        assert_eq!(out.string(), "https://api.example.com\twas_cached=true\n");
    }

    #[test]
    fn auth_logout_lines_projected_was_cached_plain_value() {
        // Projected path via engine: --columns was_cached → "true\n" (plain bool string,
        // no "was_cached=" prefix). Pins plan 020 D6's projected path contract.
        let out = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writer(out.clone()).with_options(TableOptions {
            columns: Some(vec!["was_cached".to_string()]),
            header: HeaderMode::Off,
        });
        renderer
            .auth_logout(
                &AuthLogoutOutcome {
                    server: "https://api.example.com".to_string(),
                    was_cached: true,
                },
                &make_meta("anonymous", "https://api.example.com"),
            )
            .unwrap();
        assert_eq!(out.string(), "true\n");
    }

    // ── resource_describe lines values-note tests ─────────────────────────────

    use crate::model::{FieldValues, ResourceAccess, ResourceDetail, ResourceVisibility, ValueContent};

    fn make_resource_detail(values: Option<Vec<FieldValues>>) -> ResourceDetail {
        ResourceDetail {
            label: "Test Resource".into(),
            iri: "http://rdfh.ch/0803/abc123".into(),
            resource_type: "Page".into(),
            ark_url: None,
            creation_date: None,
            last_modified: None,
            attached_project: None,
            owner: None,
            visibility: Some(ResourceVisibility::Public),
            your_access: Some(ResourceAccess::View),
            values,
        }
    }

    #[test]
    fn resource_describe_lines_no_values_no_note() {
        // When values is None, no stderr note is emitted; the metadata row renders
        // unchanged.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&make_resource_detail(None), &meta).unwrap();

        let err_str = err.string();
        assert!(
            !err_str.contains("values not shown"),
            "no values note must be emitted when values is None; got stderr:\n{err_str}"
        );
        // Metadata row still present on stdout
        assert!(
            out.string().contains("Test Resource"),
            "metadata row missing; got:\n{}",
            out.string()
        );
    }

    #[test]
    fn resource_describe_lines_some_values_no_note_renders_value_rows() {
        // When values is Some, no stderr note is emitted; stdout carries one row
        // per value in the compact default columns (field, field_label, value_type,
        // value) — no label/iri (opt-in only).
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer
            .resource_describe(
                &make_resource_detail(Some(vec![FieldValues {
                    name: "hasTitle".into(),
                    label: None,
                    values: vec![ValueContent::Text("Hello".into()).into()],
                }])),
                &meta,
            )
            .unwrap();

        let err_str = err.string();
        assert!(
            !err_str.contains("values not shown"),
            "no values note must be emitted when values is Some; got stderr:\n{err_str}"
        );
        let stdout = out.string();
        assert!(
            stdout.contains("hasTitle\t\ttext\tHello\n"),
            "expected default-columns value row field\\tfield_label\\tvalue_type\\tvalue; got:\n{stdout}"
        );
        // Metadata (label/iri) is dropped in values mode by default.
        assert!(
            !stdout.contains("Test Resource"),
            "metadata row must not appear in values mode; got:\n{stdout}"
        );
    }

    #[test]
    fn resource_describe_lines_columns_comment_renders_comment_column() {
        // comment is opt-in via --columns; selecting it must surface the
        // per-value comment text in the projected column position, and the
        // default-columns test above must stay unaffected (comment absent).
        use crate::model::Value;

        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone()).with_options(TableOptions {
            columns: Some(vec!["field".to_string(), "value".to_string(), "comment".to_string()]),
            header: HeaderMode::Off,
        });
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer
            .resource_describe(
                &make_resource_detail(Some(vec![FieldValues {
                    name: "hasTranscription".into(),
                    label: None,
                    values: vec![Value {
                        content: ValueContent::Text("teh book".into()),
                        comment: Some("reading uncertain".into()),
                    }],
                }])),
                &meta,
            )
            .unwrap();

        let stdout = out.string();
        assert!(
            stdout.contains("hasTranscription\tteh book\treading uncertain\n"),
            "expected projected field/value/comment row; got:\n{stdout}"
        );
    }

    #[test]
    fn resource_describe_lines_default_columns_omits_comment_even_when_present() {
        // comment is opt-in via --columns, NOT part of the default column set —
        // this must hold even when the underlying value DOES carry a comment
        // (distinct from `resource_describe_lines_some_values_no_note_renders_value_rows`,
        // whose fixture has no comment at all and so cannot demonstrate this).
        use crate::model::Value;

        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = LinesRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer
            .resource_describe(
                &make_resource_detail(Some(vec![FieldValues {
                    name: "hasTranscription".into(),
                    label: None,
                    values: vec![Value {
                        content: ValueContent::Text("teh book".into()),
                        comment: Some("reading uncertain".into()),
                    }],
                }])),
                &meta,
            )
            .unwrap();

        let stdout = out.string();
        assert!(
            stdout.contains("hasTranscription\t\ttext\tteh book\n"),
            "expected default-columns row field\\tfield_label\\tvalue_type\\tvalue \
             (no comment column); got:\n{stdout}"
        );
        assert!(
            !stdout.contains("reading uncertain"),
            "comment must NOT appear under default columns even though the value \
             has one — comment is opt-in via --columns, not in the default set; \
             got:\n{stdout}"
        );
    }
}
