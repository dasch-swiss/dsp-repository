//! CSV renderer — comma-separated values output.
//!
//! CSV output includes a header row followed by one data row per record.
//! Fields containing commas or double-quotes are quoted per RFC 4180.
//! Hand-rolled — the `csv` crate is not a dependency; our fields
//! (URLs, emails, ISO timestamps, state strings) are simple enough that
//! the `csv_field` helper covers all quoting cases with minimal code.
//!
//! Auth column sets (per PRD):
//! - `auth_login` / `auth_status`: `server,user,expires_at,state`
//! - `auth_logout`: `server,was_cached`

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
    DATA_MODELS_COLUMNS, DataModelListView, MetaContext, PROJECT_DUMP_COLUMNS, PROJECT_DUMP_DELETED_COLUMNS,
    PROJECTS_COLUMNS, ProjectListView, QuoteMode, RESOURCE_DESCRIBE_COLUMNS, RESOURCE_DESCRIBE_VALUES_COLUMNS,
    RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS, RESOURCE_LIST_COLUMNS, RESOURCE_TYPE_DESCRIBE_COLUMNS,
    RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS, RESOURCE_TYPES_COLUMNS, RESOURCE_TYPES_DEFAULT_COLUMNS, Renderer,
    ResourceListView, ResourceTypeListView, TableOptions, TableSpec, VOCABULARIES_COLUMNS,
    VOCABULARIES_COUNTED_DEFAULT_COLUMNS, VOCABULARIES_DEFAULT_COLUMNS, VOCABULARY_DESCRIBE_COLUMNS,
    VOCABULARY_DESCRIBE_DEFAULT_COLUMNS, VocabularyListView, render_table,
};

/// Renders output as comma-separated values with a header row.
pub struct CsvRenderer {
    out: Box<dyn Write>,
    /// Auth-state disclosure sink. In production (`new()`), this is
    /// `io::stderr()`. In tests using `with_writer`, it is `io::sink()` so
    /// existing tests don't emit to real stderr. Use `with_writers` to capture
    /// both streams in new tabular-disclosure tests.
    err: Box<dyn Write>,
    /// Per-invocation tabular options (column projection, header mode).
    /// Defaults to all columns, header on — matches today's unflagged behaviour.
    options: TableOptions,
}

impl CsvRenderer {
    /// Creates a renderer writing to stdout (data) and stderr (disclosure).
    ///
    /// Stdout is wrapped in `BrokenPipeWriter` so `dsp ... | head` exits 0
    /// silently instead of surfacing a broken pipe as `Diagnostic::Internal`.
    pub fn new() -> Self {
        Self {
            out: Box::new(crate::util::BrokenPipeWriter::new(io::stdout())),
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

impl Default for CsvRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CsvRenderer {
    fn diagnostic(&mut self, diag: &Diagnostic, _meta: &MetaContext) -> Result<(), Diagnostic> {
        eprintln!("Error: {diag}"); // errors go to stderr for non-JSON formats (dsp-cli/ADR-0012)
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        let rows = vec![vec![outcome.server.clone(), outcome.was_cached.to_string()]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: AUTH_LOGOUT_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)
    }

    fn project_dump(&mut self, outcome: &DumpOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV format: path only; cleanup disclosure is omitted (prose/json only).
        // reused/created_at are also prose/json only.
        // RFC-4180 escaping via csv_field handles paths that contain commas or quotes.
        let rows = vec![vec![outcome.path.display().to_string()]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECT_DUMP_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)
    }

    fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV format: header "deleted" + one row "true" or "false".
        let rows = vec![vec![outcome.deleted.to_string()]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECT_DUMP_DELETED_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)
    }

    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic> {
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
                    data_models_str,
                    item.iri.clone(),
                ]
            })
            .collect();
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECTS_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV format: header + one data row. Columns mirror `project list` CSV.
        // `data_models` is the count (rich fields like names stay prose/json only).
        let longname = project.longname.as_deref().unwrap_or("").to_string();
        let data_models_str = project.data_models.len().to_string();
        let rows = vec![vec![
            project.shortcode.clone(),
            project.shortname.clone(),
            longname,
            data_models_str,
            project.iri.clone(),
        ]];
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: PROJECTS_COLUMNS,
            rows: &rows,
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV format: header + one data row.
        // `resource_types` carries the count (names are nested — tabular carries count,
        // mirroring how `project_describe` csv carries the `data_models` count).
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic> {
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV header: name,iri,label,is_builtin,count. No last_modified —
        // resource-types have none. `count` auto-shows only when at least one
        // item carries one (plan 030) — see `default_columns` below.
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
        let has_counts = view.items.iter().any(|rt| rt.count.is_some());
        let default_columns = if has_counts {
            None
        } else {
            Some(RESOURCE_TYPES_DEFAULT_COLUMNS)
        };
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: RESOURCE_TYPES_COLUMNS,
            rows: &rows,
            default_columns,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
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
        // CSV format: header row + one row per field.
        // Full column set: name, iri, value_type, link_target, cardinality, label,
        // is_builtin, data_model (8 columns; `iri` at position 1).
        // Default (unflagged) output uses `RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS` —
        // the 7-column set without `iri`, byte-identical to the pre-020 csv output.
        // `--columns iri` unlocks the iri column alongside any other selection.
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
            default_columns: Some(RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS),
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn data_model_structure(&mut self, structure: &DataModelStructure, meta: &MetaContext) -> Result<(), Diagnostic> {
        // CSV format: header row + one row per relation.
        // Columns: source, target, kind, field, target_data_model.
        // Empty cell when field/target_data_model is None.
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
            default_columns: None,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // label, iri, ark_url, creation_date, last_modified, resource_type
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
            quote: QuoteMode::Csv,
            header: self.options.header,
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
        // CSV format: `detail.values` selects the shape (dsp-cli/ADR-0013 D1).
        // None → header + metadata row (all ten RESOURCE_DESCRIBE_COLUMNS), unchanged.
        // Some(fields) → header + one row per value (long-format), metadata dropped.
        match &detail.values {
            None => {
                let rows = vec![build_metadata_row(detail)];
                let projected = self.options.projected();
                let spec = TableSpec {
                    all_columns: RESOURCE_DESCRIBE_COLUMNS,
                    rows: &rows,
                    default_columns: None,
                    projected,
                    quote: QuoteMode::Csv,
                    header: self.options.header,
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
                    quote: QuoteMode::Csv,
                    header: self.options.header,
                };
                render_table(&mut *self.out, &spec)?;
            }
        }
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn vocabularies(&mut self, view: &VocabularyListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        let rows = build_vocabulary_list_rows(&view.items);
        // `nodes`/`depth` auto-show only when at least one item actually
        // carries a count (mirrors `resource_types`'s `has_counts` pattern) —
        // not merely on `view.counted`, so a `--count` run whose every
        // per-tree fetch failed keeps the plain default.
        let has_counts = view.items.iter().any(|v| v.node_count.is_some());
        let default_columns = if has_counts {
            Some(VOCABULARIES_COUNTED_DEFAULT_COLUMNS)
        } else {
            Some(VOCABULARIES_DEFAULT_COLUMNS)
        };
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: VOCABULARIES_COLUMNS,
            rows: &rows,
            default_columns,
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
        };
        render_table(&mut *self.out, &spec)?;
        render_table_disclosure(&mut *self.err, meta)?;
        Ok(())
    }

    fn vocabulary_describe(&mut self, detail: &VocabularyDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        let rows = build_vocabulary_rows(detail);
        let projected = self.options.projected();
        let spec = TableSpec {
            all_columns: VOCABULARY_DESCRIBE_COLUMNS,
            rows: &rows,
            default_columns: Some(VOCABULARY_DESCRIBE_DEFAULT_COLUMNS),
            projected,
            quote: QuoteMode::Csv,
            header: self.options.header,
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
        DataModel, DataModelDetail, DataModelSummary, Project, ProjectDetail, ResourceType, ResourceTypeSummary,
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
                data_models: 2,
            },
            Project {
                iri: "http://rdfh.ch/projects/0002".into(),
                shortcode: "0002".into(),
                shortname: "images".into(),
                longname: None,
                data_models: 0,
            },
            Project {
                iri: "http://rdfh.ch/projects/0803".into(),
                shortcode: "0803".into(),
                shortname: "daschland".into(),
                longname: Some("=Formula,Project".into()),
                data_models: 1,
            },
        ]
    }

    #[test]
    fn projects_csv_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let view = ProjectListView { items: make_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let stdout = out.string();
        assert!(stdout.starts_with("shortcode,shortname,longname,data_models,iri\n"));
        // Plain field
        assert!(stdout.contains("0001,anything,Anything Project,2,http://rdfh.ch/projects/0001"));
        // None longname → empty field
        assert!(stdout.contains("0002,images,,0,http://rdfh.ch/projects/0002"));
        // Leading `=` longname — formula injection quoting; also contains comma
        assert!(stdout.contains(r#"0803,daschland,"=Formula,Project",1,http://rdfh.ch/projects/0803"#));
    }

    #[test]
    fn projects_csv_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let view = ProjectListView { items: make_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        // Disclosure lands on stderr, not stdout
        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        // stdout must NOT contain the disclosure
        assert!(!out.string().contains("[anonymous"));
    }

    #[test]
    fn auth_status_csv_stderr_disclosure() {
        // AuthenticatedViaEnv — the env-vs-cache gap this fix closes
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
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
        let mut renderer2 = CsvRenderer::with_writers(out2.clone(), err2.clone());
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
    fn projects_csv_with_writer_no_stderr_leak() {
        // with_writer → err = io::sink(); should not panic, just discard disclosure.
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let view = ProjectListView { items: vec![], total: 0, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();
        // Only header row on stdout
        assert_eq!(out.string(), "shortcode,shortname,longname,data_models,iri\n");
    }

    #[test]
    fn project_describe_csv_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            description: vec![],
            keywords: vec![],
            data_models: vec![
                DataModelSummary {
                    name: "beol".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                },
                DataModelSummary {
                    name: "biblio".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                },
            ],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // Header row
        assert!(stdout.starts_with("shortcode,shortname,longname,data_models,iri\n"));
        // Data row: data_models = count (2)
        assert!(stdout.contains("0801,beol,Bernoulli-Euler Online,2,http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF"));
    }

    #[test]
    fn project_describe_csv_no_longname() {
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0000".into(),
            shortcode: "0000".into(),
            shortname: "minimal".into(),
            longname: None,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // None longname → empty field
        assert!(stdout.contains("0000,minimal,,0,http://rdfh.ch/projects/0000"));
    }

    #[test]
    fn project_describe_csv_formula_injection_longname() {
        // A longname starting with `=` or containing a comma should be quoted.
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0803".into(),
            shortcode: "0803".into(),
            shortname: "daschland".into(),
            longname: Some("=Formula,Project".into()),
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // Formula injection and comma → quoted field
        assert!(stdout.contains(r#"0803,daschland,"=Formula,Project",0"#));
    }

    #[test]
    fn project_describe_csv_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0001".into(),
            shortcode: "0001".into(),
            shortname: "test".into(),
            longname: None,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
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
    fn data_models_csv_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let stdout = out.string();
        // Header row
        assert!(stdout.starts_with("name,iri,label,last_modified,is_builtin\n"));
        // beol: has label and last_modified, not builtin
        assert!(stdout.contains(
            "beol,http://api.dasch.swiss/ontology/0801/beol/v2,The BEOL data-model,2024-05-27T13:43:26.233048Z,false"
        ));
        // biblio: label None → empty, last_modified None → empty
        assert!(stdout.contains("biblio,http://api.dasch.swiss/ontology/0801/biblio/v2,,,false"));
        // knora-api: builtin → "true"
        assert!(stdout.contains("knora-api,http://api.knora.org/ontology/knora-api/v2,,,true"));
    }

    #[test]
    fn data_models_csv_formula_injection_label() {
        // A label starting with `=` or containing a comma must be quoted.
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let view = DataModelListView {
            items: vec![DataModel {
                name: "test".into(),
                iri: "http://api.dasch.swiss/ontology/0001/test/v2".into(),
                label: Some("=Formula,Label".into()),
                last_modified: None,
                is_builtin: false,
            }],
            total: 1,
            filter: None,
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let stdout = out.string();
        // Formula injection + comma → quoted field
        assert!(
            stdout.contains(r#"test,http://api.dasch.swiss/ontology/0001/test/v2,"=Formula,Label",,false"#),
            "formula-injected label must be quoted; got:\n{stdout}"
        );
    }

    #[test]
    fn data_models_csv_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        assert!(!out.string().contains("[anonymous"));
    }

    // ── data_model_describe CSV tests ─────────────────────────────────────────

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
    fn data_model_describe_csv_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let stdout = out.string();
        // Header row
        assert!(
            stdout.starts_with("name,iri,label,last_modified,resource_types\n"),
            "header row missing; got:\n{stdout}"
        );
        // Data row: resource_types = count (3)
        assert!(
            stdout.contains(
                "beol,http://api.dasch.swiss/ontology/0801/beol/v2,The BEOL data-model,2024-05-27T13:43:26.233048Z,3"
            ),
            "data row missing; got:\n{stdout}"
        );
    }

    #[test]
    fn data_model_describe_csv_no_label_no_last_modified() {
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "minimal".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2".into(),
            label: None,
            last_modified: None,
            resource_types: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // label None → empty, last_modified None → empty, resource_types = 0
        assert!(
            stdout.contains("minimal,http://api.dasch.swiss/ontology/0000/minimal/v2,,,0"),
            "None → empty fields must be handled; got:\n{stdout}"
        );
    }

    #[test]
    fn data_model_describe_csv_formula_injection_label() {
        // A label starting with `=` or containing a comma must be quoted (formula injection).
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "test".into(),
            iri: "http://api.dasch.swiss/ontology/0001/test/v2".into(),
            label: Some("=Formula,Label".into()),
            last_modified: None,
            resource_types: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let stdout = out.string();
        // Formula injection + comma → quoted field
        assert!(
            stdout.contains(r#"test,http://api.dasch.swiss/ontology/0001/test/v2,"=Formula,Label",,0"#),
            "formula-injected label must be quoted; got:\n{stdout}"
        );
    }

    #[test]
    fn data_model_describe_csv_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let err_str = err.string();
        assert_eq!(err_str.trim(), "[anonymous on https://api.test.dasch.swiss]");
        assert!(!out.string().contains("[anonymous"));
    }

    // ── resource_types CSV tests ──────────────────────────────────────────────

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
                label: None,
                is_builtin: false,
                count: None,
            },
        ]
    }

    #[test]
    fn resource_types_csv_output() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let view = ResourceTypeListView {
            items: make_resource_type_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        // Header row (no last_modified column)
        assert!(
            stdout.starts_with("name,iri,label,is_builtin\n"),
            "header row missing; got:\n{stdout}"
        );
        // Archive: has label, not builtin
        assert!(
            stdout.contains("Archive,http://api.dasch.swiss/ontology/0801/beol/v2#Archive,Archive,false"),
            "Archive row missing; got:\n{stdout}"
        );
        // letter: label None → empty field
        assert!(
            stdout.contains("letter,http://api.dasch.swiss/ontology/0801/beol/v2#letter,,false"),
            "letter row missing; got:\n{stdout}"
        );
    }

    #[test]
    fn resource_types_csv_with_builtins() {
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![ResourceType {
                name: "Region".into(),
                iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
                label: Some("Region".into()),
                is_builtin: true,
                count: None,
            }],
            total: 1,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        // built-in row with is_builtin = "true"
        assert!(
            stdout.contains("Region,http://api.knora.org/ontology/knora-api/v2#Region,Region,true"),
            "built-in row missing; got:\n{stdout}"
        );
    }

    #[test]
    fn resource_types_csv_with_filter() {
        // filter does not affect the CSV output shape; just verify it renders cleanly
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![make_resource_type_fixture().remove(0)], // just Archive
            total: 2,
            filter: Some("arch".to_string()),
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let stdout = out.string();
        assert!(stdout.starts_with("name,iri,label,is_builtin\n"));
        assert!(stdout.contains("Archive"));
    }

    #[test]
    fn resource_types_csv_empty() {
        let out = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![],
            total: 0,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        // Only header row
        assert_eq!(out.string(), "name,iri,label,is_builtin\n");
    }

    #[test]
    fn resource_types_csv_stderr_disclosure() {
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
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
    fn resource_types_csv_with_count_auto_shows_column() {
        // plan 030: when at least one item carries a count, the `count`
        // column auto-shows in the unflagged (no --columns) header, and the
        // count_caveat is appended to the stderr disclosure line.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
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
        assert!(
            stdout.starts_with("name,iri,label,is_builtin,count\n"),
            "header must auto-show count column; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Archive,http://api.dasch.swiss/ontology/0801/beol/v2#Archive,Archive,false,10"),
            "Archive row must show count 10; got:\n{stdout}"
        );
        // letter has no count -> empty cell, trailing comma.
        assert!(
            stdout.contains("letter,http://api.dasch.swiss/ontology/0801/beol/v2#letter,,false,\n"),
            "letter row must show empty count cell; got:\n{stdout}"
        );
        let err_str = err.string();
        assert!(
            err_str.contains("counts are not permission-filtered"),
            "stderr must carry count_caveat; got: {err_str:?}"
        );
    }

    #[test]
    fn resource_type_describe_csv_with_count_no_column_but_disclosure_carries_caveat() {
        // Part F scope note: resource_type_describe gets ZERO csv/tsv changes —
        // `count` is per-resource-type, not per-field, so this row-per-field
        // table has no column for it. The only user-visible effect of a
        // `count`/`count_caveat` here is the stderr disclosure note, which
        // flows automatically through the Part B1 `disclosure_suffix` helper.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
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
        let meta = crate::render::MetaContext {
            server_label: "https://api.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts exclude deleted resources".into()),
            count_cost: None,
        };
        renderer.resource_type_describe(&detail, &meta).unwrap();

        // Table shape unchanged: header only, no `count` column.
        assert!(
            out.string()
                .starts_with("name,value_type,link_target,cardinality,label,is_builtin,data_model\n")
        );
        // Disclosure note carries the caveat.
        assert!(
            err.string().contains("counts exclude deleted resources"),
            "stderr must carry count_caveat; got: {:?}",
            err.string()
        );
    }

    // ── resource_describe csv values-note tests ───────────────────────────────

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
    fn resource_describe_csv_some_values_no_note_renders_value_rows() {
        // When values is Some, no stderr note; stdout carries the values header +
        // one row per value in the compact default columns. Metadata (label/iri)
        // is dropped by default in values mode.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
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
            "no values note must be emitted when values is Some; got:\n{err_str}"
        );
        let stdout = out.string();
        assert!(
            stdout.starts_with("field,field_label,value_type,value\n"),
            "csv values header must be the compact default set; got:\n{stdout}"
        );
        assert!(
            stdout.contains("hasTitle,,text,Hello\n"),
            "expected value row for hasTitle; got:\n{stdout}"
        );
        // Metadata is dropped in values mode by default.
        assert!(
            !stdout.contains("Test Resource"),
            "metadata row must not appear in values mode; got:\n{stdout}"
        );
    }

    #[test]
    fn resource_describe_csv_no_values_no_note() {
        // When values is None, no stderr note.
        let out = SharedBuf::new();
        let err = SharedBuf::new();
        let mut renderer = CsvRenderer::with_writers(out.clone(), err.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&make_resource_detail(None), &meta).unwrap();

        assert!(
            !err.string().contains("values not shown"),
            "no note expected when values is None; got:\n{}",
            err.string()
        );
    }
}
