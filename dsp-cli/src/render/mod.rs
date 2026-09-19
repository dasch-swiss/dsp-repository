//! Renderer layer (3b of dsp-cli/ADR-0008) — output formatting.
//!
//! The `Renderer` trait has explicit per-noun methods (prose is irreducibly
//! per-noun); each format impl is a separate struct (prose, json, lines,
//! csv, tsv). `MetaContext` threads the auth-state disclosure from dsp-cli/ADR-0007
//! through every call.
//!
//! `Format` is the user-facing enum that maps `--format` flag values to
//! concrete renderer instances via `Format::into_renderer`. It derives
//! `clap::ValueEnum` so the CLI can parse it directly.

pub mod auth;
pub mod csv;
pub mod dump;
pub mod format;
pub mod json;
pub mod lines;
pub mod progress;
pub mod prose;
pub(crate) mod table;
#[cfg(test)]
mod test_support;
pub mod tsv;
pub(crate) mod value;
pub(crate) mod vocabulary;

pub use auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
pub use dump::{DumpDeleteOutcome, DumpEvent, DumpOutcome};
pub use format::Format;
pub use progress::{HumanProgress, JsonProgress, ProgressReporter};
// Re-exported for plan 020 steps 2–5: renderer methods, engine error hints,
// and CLI `after_help` drift-guard tests.
pub(crate) use table::{
    AUTH_LOGIN_COLUMNS, AUTH_LOGOUT_COLUMNS, DATA_MODEL_DESCRIBE_COLUMNS, DATA_MODEL_STRUCTURE_COLUMNS,
    DATA_MODELS_COLUMNS, PROJECT_DUMP_COLUMNS, PROJECT_DUMP_DELETED_COLUMNS, PROJECTS_COLUMNS, QuoteMode,
    RESOURCE_DESCRIBE_COLUMNS, RESOURCE_DESCRIBE_VALUES_COLUMNS, RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS,
    RESOURCE_LIST_COLUMNS, RESOURCE_TYPE_DESCRIBE_COLUMNS, RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS,
    RESOURCE_TYPES_COLUMNS, RESOURCE_TYPES_DEFAULT_COLUMNS, TableSpec, VOCABULARIES_COLUMNS,
    VOCABULARIES_COUNTED_DEFAULT_COLUMNS, VOCABULARIES_DEFAULT_COLUMNS, VOCABULARY_DESCRIBE_COLUMNS,
    VOCABULARY_DESCRIBE_DEFAULT_COLUMNS, render_table,
};
// TableOptions and HeaderMode are public: integration tests in `tests/` construct
// renderers with specific options via `with_options`. The column-set consts are
// pub(crate) only (used in renderer bodies and the engine error hints, not the
// test API).
//
// `with_options` is `pub` (not `pub(crate)`) because integration tests are a
// separate crate and need to call it to exercise the new --columns and header
// flags end-to-end; bypassing FormatArgs::table_options validation via
// with_options is an internal/test-only concern that does not affect the
// production dispatch path.
pub use table::{HeaderMode, TableOptions};

use crate::diagnostic::Diagnostic;
use crate::model::{
    DataModel, DataModelDetail, DataModelStructure, Project, ProjectDetail, ResourceDetail, ResourceSummary,
    ResourceType, ResourceTypeDetail, Vocabulary, VocabularyDetail,
};

/// The data a renderer needs to render a `project list` result.
///
/// `total` is the pre-filter count; `filter` is the applied substring (if any),
/// used only by prose for the "(m of n matching "…")" count line.
///
/// Owns its data (no borrow/lifetime): it is a per-call view, never stored, and
/// the action builds it by moving the already-sorted `Vec<Project>` in. Owning
/// avoids a `'a` parameter leaking into the `Renderer::projects` signature (and
/// the latent lifetime-elision trap a future caching renderer would hit). The
/// clone cost is nil — the vec is moved, not copied.
#[derive(Debug)]
pub struct ProjectListView {
    pub items: Vec<Project>,
    /// Pre-filter total count (before `--filter` was applied). Feeds the
    /// "(m of total matching …)" prose count line.
    pub total: usize,
    /// The `--filter` substring the user supplied, if any.
    pub filter: Option<String>,
}

/// The data a renderer needs to render a `data-model list` result.
///
/// `total` is the pre-filter count (it includes any built-ins the action
/// appended); `filter` the applied substring (if any). Whether built-ins are
/// present is read off the items themselves (`is_builtin`), so it is not carried
/// as a separate field.
#[derive(Debug, Clone)]
pub struct DataModelListView {
    pub items: Vec<DataModel>,
    /// Pre-filter total count (before `--filter` was applied). Feeds the
    /// "(m of total matching …)" prose count line.
    pub total: usize,
    /// The `--filter` substring the user supplied, if any.
    pub filter: Option<String>,
}

/// The data a renderer needs to render a `resource-type list` result.
///
/// `items` is the post-filter, sorted list; `total` is the pre-filter count
/// (after any built-ins were appended, before `--filter` was applied); `filter`
/// is the applied substring (if any). Whether built-ins are present is read off
/// the items themselves (`is_builtin`), so it is not carried as a separate field.
///
/// `data_model` carries the resolved parent data-model's **name** for the prose
/// header (`resource-types in <data_model> on <server>`). This is a deliberate
/// extension beyond `ProjectListView`/`DataModelListView` (which carry no parent):
/// resource-type list is sub-scoped to one data-model, and the name must appear
/// even when `items` is empty (so it cannot be derived from `items[0].iri`).
/// Threading it through `MetaContext` was rejected — that struct is auth/server
/// disclosure only (dsp-cli/ADR-0007), so overloading it is a worse coupling than this
/// explicit field. Tabular and JSON renderers ignore `data_model`.
///
/// `Clone` is required because the test `RecordingRenderer` stores the view in
/// an `Option<ResourceTypeListView>`.
#[derive(Debug, Clone)]
pub struct ResourceTypeListView {
    pub items: Vec<ResourceType>,
    /// Pre-filter total count (after built-ins were appended, before `--filter`
    /// was applied). Feeds the "(m of total matching …)" prose count line.
    pub total: usize,
    /// The `--filter` substring the user supplied, if any.
    pub filter: Option<String>,
    /// The resolved parent data-model's NAME, for the prose header. Tabular/json
    /// ignore it. See struct-level doc for why this field exists.
    pub data_model: String,
}

/// Pagination state for a `resource list` render call (D5 of the plan).
///
/// Models two mutually exclusive modes as an enum rather than two loose
/// `Option<u32>` fields that admit invalid combinations (e.g. both set).
/// The json renderer matches on this to build `_meta` pagination keys;
/// prose reads `may_have_more` (SinglePage only) for the "use --all" hint.
///
/// `AllPages` hard-codes `may_have_more_results: false` because the `--all`
/// loop only exits when the server reports `false` — the variant carries no
/// flag because it is guaranteed.
#[derive(Debug, Clone)]
pub enum ResourceListPagination {
    /// A single page was fetched (default or `--page N`).
    SinglePage {
        /// The page number that was fetched (zero-based).
        page: u32,
        /// Whether the server reported more pages after this one.
        may_have_more: bool,
    },
    /// All pages were fetched (`--all`).
    AllPages {
        /// Total number of pages fetched.
        pages_fetched: u32,
    },
}

/// The data a renderer needs to render a `resource list` result.
///
/// `items` is the post-filter list; `total` is the pre-filter count (capture
/// BEFORE applying `--filter`, mirroring `project list`); `filter` is the
/// applied substring (if any). `resource_type` carries the resolved class name
/// for the prose header. `pagination` carries the D5 mode struct (single page
/// vs. all pages).
///
/// `Clone` is required because the test `RecordingRenderer` stores the view in
/// an `Option<ResourceListView>`.
#[derive(Debug, Clone)]
pub struct ResourceListView {
    /// Post-filter, sorted resource summaries.
    pub items: Vec<ResourceSummary>,
    /// Pre-filter total count. Feeds the "(m of total matching "…")" prose line.
    pub total: usize,
    /// The `--filter` substring the user supplied, if any.
    pub filter: Option<String>,
    /// Local name of the resource type, for the prose header.
    pub resource_type: String,
    /// Pagination state (single page vs. all-pages drain).
    pub pagination: ResourceListPagination,
}

/// The data a renderer needs to render a `vocabulary list` result.
#[derive(Debug, Clone)]
pub struct VocabularyListView {
    pub items: Vec<Vocabulary>,
    /// Pre-filter total count (before `--filter` was applied). Feeds the
    /// "(m of total matching …)" prose count line.
    pub total: usize,
    /// The `--filter` substring the user supplied, if any.
    pub filter: Option<String>,
    /// Whether `--count` was passed (drives which tabular default-column set
    /// applies and the prose "· N nodes · M levels" suffix per item). NOT the
    /// same test as "any item carries a count" — an item can carry `None`
    /// after a failed per-tree fetch even when `--count` WAS passed; `counted`
    /// records the flag, not the outcome.
    pub counted: bool,
}

/// Auth and server context attached to every rendered response.
/// See dsp-cli/ADR-0007.
#[derive(Debug, Clone)]
pub struct MetaContext {
    pub server_label: String,
    pub auth_state: String,
    /// dsp-cli/ADR-0007 silent-filter disclosure for instance-side reads.
    ///
    /// Set to `Some(message)` by instance-side commands (`resource list`,
    /// `resource describe`) to disclose that results may be filtered by the
    /// caller's authentication state (anonymous → only public resources visible;
    /// authenticated → bounded by permissions). `None` for schema-side commands
    /// (`project list/describe`, `data-model list/describe`, `resource-type
    /// list/describe`, `data-model structure`) that are not affected by caller
    /// identity.
    pub filter_warning: Option<String>,
    /// Schema-side `--count` disclosure note (`resource-type list`/`describe`).
    ///
    /// Set to `Some(message)` by the action layer when `--count` was passed,
    /// disclosing that the v3 `resourcesPerOntology` counts are NOT
    /// permission-filtered (unlike `resource list`'s `filter_warning`, which IS
    /// permission-filtered) and exclude deleted resources. `None` when `--count`
    /// was not used, and always `None` for every command other than
    /// resource-type list/describe. Deliberately a DISTINCT field from
    /// `filter_warning` — a different semantic contract, not reused (see design plan
    /// 030-resource-type-count in the dsp-incubator archive).
    pub count_caveat: Option<String>,
    /// `dsp vre vocabulary list --count` cost-disclosure note (plan 034).
    ///
    /// Set to `Some(message)` by the action layer when `--count` was passed on
    /// `vocabulary list`, disclosing that `--count` costs one extra tree fetch
    /// PER vocabulary (sequential, unthrottled). Deliberately a DISTINCT field
    /// from `count_caveat` — `count_caveat`'s message is about permission-filtering
    /// accuracy, which does not apply here: vocabularies are public and their
    /// counts are exact. `None` when `--count` was not used, and always `None`
    /// for every command other than `vocabulary list`.
    pub count_cost: Option<String>,
}

/// The `Renderer` trait. Methods grow as new noun-groups land.
///
/// All methods return `Result<(), Diagnostic>`. I/O errors in renderer impls
/// are converted via `impl From<std::io::Error> for Diagnostic`, which means
/// `writeln!(self.out, "...")?;` Just Works against this return type.
pub trait Renderer {
    /// Render a diagnostic in the appropriate shape for this format.
    fn diagnostic(&mut self, diag: &Diagnostic, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a successful `dsp auth login` outcome.
    fn auth_login(&mut self, outcome: &AuthLoginOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp auth status` outcome (logged-in or not-logged-in).
    fn auth_status(&mut self, outcome: &AuthStatusOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp auth logout` outcome.
    fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a successful `dsp auth set-token` outcome.
    fn auth_set_token(&mut self, outcome: &AuthSetTokenOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre project dump` outcome.
    fn project_dump(&mut self, outcome: &DumpOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre project dump --delete` outcome.
    ///
    /// `outcome.deleted = false` means no completed/failed dump existed and a
    /// probe created an in-progress dump — NOT a delete failure (failures are
    /// `Err(Diagnostic)`).
    fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre project list` result (possibly empty).
    ///
    /// `view` carries the items (post-filter, sorted), the pre-filter total,
    /// and the filter string. `meta` carries auth/server disclosure (dsp-cli/ADR-0007).
    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre project describe` result (a single project).
    ///
    /// `project` is passed directly — no view wrapper, since there is no
    /// aggregate context (no `total`/`filter`) for a single-object describe.
    /// `meta` carries auth/server disclosure per dsp-cli/ADR-0007.
    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre data-model list` result (possibly empty).
    ///
    /// `view` carries the items (post-filter, sorted), the pre-filter total,
    /// and the filter string. `meta` carries auth/server disclosure (dsp-cli/ADR-0007).
    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre data-model describe` result (a single data-model).
    ///
    /// `detail` is passed directly — no view wrapper, since there is no aggregate
    /// context (no `total`/`filter`) for a single-object describe. `meta` carries
    /// auth/server disclosure per dsp-cli/ADR-0007.
    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre resource-type list` result (possibly empty).
    ///
    /// `view` carries the items (post-filter, sorted), the pre-filter total, the
    /// filter string, and the parent data-model name. `meta` carries auth/server
    /// disclosure (dsp-cli/ADR-0007).
    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre resource-type describe` result (a single resource-type).
    ///
    /// `detail` is passed directly — no view wrapper, since there is no aggregate
    /// context (no `total`/`filter`) for a single-object describe. `meta` carries
    /// auth/server disclosure per dsp-cli/ADR-0007. Built-in field filtering is applied by
    /// the action (via `--include-builtins`) before this method is called — the
    /// renderer receives only the fields it should render.
    fn resource_type_describe(&mut self, detail: &ResourceTypeDetail, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre data-model structure` result (a single data-model's relations).
    ///
    /// `structure` is passed directly — no view wrapper (describe-shaped, like
    /// `data_model_describe`). Built-in relation filtering via `--include-builtins`
    /// is applied by the action before this method is called; the renderer receives
    /// only the relations it should render. `meta` carries auth/server disclosure
    /// per dsp-cli/ADR-0007.
    fn data_model_structure(&mut self, structure: &DataModelStructure, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre resource list` result (possibly empty).
    ///
    /// `view` carries the items (post-filter), the pre-filter total, the filter
    /// string, the resource type name, and the pagination state. `meta` carries
    /// auth/server disclosure (dsp-cli/ADR-0007) including the always-present
    /// `filter_warning` for instance-side commands (D3).
    fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre resource describe` result (a single resource's envelope).
    ///
    /// `detail` is passed directly — no view wrapper, since there is no aggregate
    /// context for a single-object describe. `meta` carries auth/server disclosure
    /// per dsp-cli/ADR-0007 including the always-present `filter_warning` for instance-side
    /// commands (D3).
    fn resource_describe(&mut self, detail: &ResourceDetail, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre vocabulary list` result (possibly empty).
    ///
    /// `view` carries the items (post-filter, sorted by name), the pre-filter
    /// total, the filter string, and whether `--count` was requested. `meta`
    /// carries auth/server disclosure (dsp-cli/ADR-0007) plus the `--count` cost
    /// disclosure (`MetaContext.count_cost`, plan 034).
    fn vocabularies(&mut self, view: &VocabularyListView, meta: &MetaContext) -> Result<(), Diagnostic>;

    /// Render a `dsp vre vocabulary describe` result (a single vocabulary's tree).
    ///
    /// `detail` is passed directly — no view wrapper, since there is no
    /// aggregate context for a single-object describe.
    ///
    /// **This method deliberately INVERTS the established describe convention.**
    /// Other describe methods document that "built-in field filtering is
    /// applied by the action ... before this method is called — the renderer
    /// receives only the fields it should render" (see `resource_type_describe`
    /// above). This one is the opposite: it receives the WHOLE tree
    /// (`detail.tree`, never pruned) and filters to `detail.subtree_of`'s branch
    /// itself. That is required, not sloppy — pruning before the renderer would
    /// delete the ancestor chain that the absolute `number` and `path` columns
    /// are derived from. Do not "fix" this by narrowing the tree in the action
    /// layer.
    fn vocabulary_describe(&mut self, detail: &VocabularyDetail, meta: &MetaContext) -> Result<(), Diagnostic>;
}
