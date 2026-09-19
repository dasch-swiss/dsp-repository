//! CLI parsing — layer 1 of dsp-cli/ADR-0008.
//!
//! Produces typed argument structs from `argv`. The structs flow through the
//! action layer; no business logic lives here.

use clap::{Args, Parser, Subcommand};

use crate::diagnostic::Diagnostic;
use crate::render::{Format, HeaderMode, TableOptions};

// ── after_help column-doc strings ─────────────────────────────────────────────
//
// Each tabular leaf command carries a static `after_help` line that documents
// its column set.  These literals are derived from the per-noun column-set
// consts in `crate::render` (step 5, D9).  They are intentionally NOT built
// at runtime from the consts — clap requires `&'static str`.  A drift-guard
// unit test (`test_after_help_matches_consts`) asserts that each literal's
// column list exactly matches the joined `crate::render` const, so adding a
// column without updating the literal fails CI.

const AFTER_HELP_PROJECT_LIST: &str = "Columns (--columns): shortcode, shortname, longname, status, data_models, iri";

const AFTER_HELP_PROJECT_DESCRIBE: &str =
    "Columns (--columns): shortcode, shortname, longname, status, data_models, iri";

const AFTER_HELP_PROJECT_DUMP: &str = "Columns (--columns): path  (--delete mode: deleted)";

const AFTER_HELP_DATA_MODEL_LIST: &str = "Columns (--columns): name, iri, label, last_modified, is_builtin";

const AFTER_HELP_DATA_MODEL_DESCRIBE: &str = "Columns (--columns): name, iri, label, last_modified, resource_types";

const AFTER_HELP_DATA_MODEL_STRUCTURE: &str = "Columns (--columns): source, target, kind, field, target_data_model";

const AFTER_HELP_RESOURCE_TYPE_LIST: &str = "Columns (--columns): name, iri, label, is_builtin, count";

/// Full 8-column set for `resource-type describe` (one row per field).
/// `iri` is accessible via `--columns iri` (hidden from the default csv/tsv
/// output by the lean-default mechanism, but present in `all_columns`).
const AFTER_HELP_RESOURCE_TYPE_DESCRIBE: &str =
    "Columns (--columns): name, iri, value_type, link_target, cardinality, label, is_builtin, data_model";

const AFTER_HELP_RESOURCE_LIST: &str = "Columns (--columns): label, iri, ark_url, creation_date, last_modified, resource_type\n\n\
     Scan behaviour: a bare --resource-type name (no ://) scans all project data-models; \
     use --data-model or a full IRI to skip the scan. See also: `dsp docs concepts`\n\n\
     --order-by: field name (e.g. title) or full field IRI; sorts ascending; \
     targets project-defined fields (full IRI passed verbatim; bare name resolved to its field IRI)";

const AFTER_HELP_RESOURCE_DESCRIBE: &str = "Columns (--columns): label, iri, resource_type, ark_url, creation_date, last_modified, attached_project, owner, visibility, your_access\n\
     Columns (--columns) with --values: label, iri, field, field_label, value_type, value, comment\n\
     Default columns with --values: field, field_label, value_type, value\n\n\
     See also: `dsp docs concepts`";

const AFTER_HELP_VOCABULARY_LIST: &str = "Columns (--columns): name, iri, label_en, label_de, label_fr, label_it, label_rm, label, comment_en, comment_de, comment_fr, comment_it, comment_rm, comment, nodes, depth\n\n\
     --filter matches name and every label value in every language (all languages kept — no language preference); comments are not scanned.\n\
     --count fetches each vocabulary's full tree (one extra request PER vocabulary, sequential — this can be dozens of calls on a project with many large vocabularies); a failed per-tree fetch degrades that row's nodes/depth to empty rather than failing the whole command, and is disclosed.";

const AFTER_HELP_VOCABULARY_DESCRIBE: &str = "Columns (--columns): node_iri, number, name, label_en, label_de, label_fr, label_it, label_rm, label, comment_en, comment_de, comment_fr, comment_it, comment_rm, comment, path, position, depth, parent_iri\n\n\
     Emits the WHOLE vocabulary tree with no depth limit and no pagination — the largest vocabulary on prod has ~4400 nodes.\n\
     `number` is a 1-based dotted outline in DFS order; it is NOT a sort key (`1.10` sorts before `1.2` lexicographically) — never re-sort tabular output on this column.\n\
     A node IRI (e.g. pasted from `resource describe --values`) resolves upward to its vocabulary automatically; the addressed node is marked in prose/json output (not in tabular columns — match on node_iri instead).\n\
     --subtree narrows output to the addressed node's own branch; it requires a node IRI — a bare name or a root IRI with --subtree is a usage error.";

const AFTER_HELP_AUTH_LOGIN: &str = "Columns (--columns): server, user, expires_at, state";

const AFTER_HELP_AUTH_STATUS: &str = "Columns (--columns): server, user, expires_at, state";

const AFTER_HELP_AUTH_LOGOUT: &str = "Columns (--columns): server, was_cached";

const AFTER_HELP_AUTH_SET_TOKEN: &str = "Columns (--columns): server, user, expires_at, state";

/// No columns line — this leaf has no tabular output (D2, plan 035): no
/// `--format`/`-j`/`-l`/`--columns`, the same carve-out as `dsp auth token`.
const AFTER_HELP_SPARQL_QUERY: &str = "--accept aliases: json (default, application/sparql-results+json), \
     xml (application/sparql-results+xml), csv (text/csv), tsv (text/tab-separated-values), \
     turtle (text/turtle), ntriples (application/n-triples), jsonld (application/ld+json). \
     A value containing '/' is forwarded verbatim as a raw media type.\n\n\
     Caveat: Fuseki does not 406 on an Accept it cannot satisfy — it silently falls back to its \
     own default serialization (application/sparql-results+xml), so a wrong --accept shows up as \
     unexpected output, not as an error.\n\n\
     Requires a system-administrator (SystemAdmin) token.\n\n\
     --query-file accepts a leading-dash path as-is (allow_hyphen_values).\n\n\
     --query on the command line is the LEAST private input: it lands in shell history and is \
     readable via `ps`/`/proc/<pid>/cmdline` for the life of the process, AND dsp-api logs the \
     full query text together with your username on every call. Prefer stdin or --query-file for \
     anything sensitive. See also: dsp docs sparql";

// ── output format ─────────────────────────────────────────────────────────────

/// Output format selection for vre data commands.
///
/// Flattened into each of the six vre leaf Args structs. Provides `--format`,
/// `-j`/`--json`, and `-l`/`--lines` as parallel selection paths.
///
/// Precedence (resolved by [`FormatArgs::resolve`]): `-j` > `-l` > `--format`.
/// No `conflicts_with` is used — clap treats `default_value_t` as "implicitly
/// set", which would make valid calls like `dsp vre project list -j` collide
/// with the prose default. The helper-method precedence is simpler and correct.
///
/// See [dsp-cli/ADR-0003](../../docs/adr/0003-chaining-and-output.md) for the output
/// format specification and the design-decisions section of the 003 plan for
/// why this is per-leaf rather than global.
#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Output format (default: prose).
    #[arg(
        long,
        value_enum,
        default_value_t = Format::Prose,
        value_name = "FORMAT"
    )]
    pub format: Format,

    /// Shortcut for --format=json.
    #[arg(short = 'j', long = "json")]
    pub json: bool,

    /// Shortcut for --format=lines.
    #[arg(short = 'l', long = "lines")]
    pub lines: bool,

    /// Output columns for tabular formats (csv, tsv, lines): comma-separated list
    /// that selects and reorders columns. Valid names are listed in the Columns
    /// line below. Duplicates are rejected.
    #[arg(long, value_name = "COLS")]
    pub columns: Option<String>,

    /// Omit the csv/tsv header row, e.g. appending rows to an existing file.
    #[arg(long, conflicts_with = "header_only")]
    pub no_header: bool,

    /// Emit only the csv/tsv header row, no data rows. Note: the command still
    /// contacts the server.
    #[arg(long)]
    pub header_only: bool,
}

impl FormatArgs {
    /// Resolve the effective format. Precedence: `-j` > `-l` > `--format`.
    pub fn resolve(&self) -> Format {
        if self.json {
            Format::Json
        } else if self.lines {
            Format::Lines
        } else {
            self.format
        }
    }

    /// Validate and resolve tabular options from the CLI flags.
    ///
    /// `format` must be the **resolved** format (call [`FormatArgs::resolve`]
    /// first — never pass `self.format`, which is always `Prose` when `-j`/`-l`
    /// are used).
    ///
    /// ## Validation rules
    ///
    /// - `--columns` is only valid with `csv`, `tsv`, or `lines` output. Any other format →
    ///   `Diagnostic::Usage`.
    /// - `--no-header` / `--header-only` are only valid with `csv` or `tsv`. `lines` has no header
    ///   concept. Any other format → `Diagnostic::Usage`.
    /// - `--columns` value: the string must be non-empty; each comma-separated token must be
    ///   non-blank (no `a,,b`); no duplicates allowed. Unknown column names are validated later by
    ///   the engine (which knows the per-noun set).
    ///
    /// Returns a `TableOptions` whose `columns` field is guaranteed to be
    /// syntactically valid (non-empty `Some(Vec)` with no blank entries and no
    /// duplicates), or `None` if `--columns` was not supplied.
    pub fn table_options(&self, format: Format) -> Result<TableOptions, Diagnostic> {
        // Validate --columns scope.
        if self.columns.is_some() && !matches!(format, Format::Csv | Format::Tsv | Format::Lines) {
            return Err(Diagnostic::Usage("--columns works with csv, tsv, and lines output".to_string()));
        }

        // Validate --no-header / --header-only scope.
        if (self.no_header || self.header_only) && !matches!(format, Format::Csv | Format::Tsv) {
            return Err(Diagnostic::Usage(
                "--no-header and --header-only work with csv and tsv output only (lines has no header concept)"
                    .to_string(),
            ));
        }

        // Parse --columns value.
        let columns = if let Some(ref raw) = self.columns {
            if raw.is_empty() {
                return Err(Diagnostic::Usage("--columns requires at least one column name".to_string()));
            }
            let parts: Vec<&str> = raw.split(',').collect();
            // Reject blank segments (e.g. "a,,b" or trailing comma).
            for part in &parts {
                if part.is_empty() {
                    return Err(Diagnostic::Usage(format!(
                        "--columns contains a blank segment in \"{raw}\"; \
                         use a comma-separated list with no empty entries"
                    )));
                }
            }
            // Reject duplicates.
            let mut seen = std::collections::HashSet::new();
            for part in &parts {
                if !seen.insert(*part) {
                    return Err(Diagnostic::Usage(format!(
                        "--columns contains duplicate column \"{part}\"; \
                         each column may appear at most once"
                    )));
                }
            }
            Some(parts.iter().map(|s| s.to_string()).collect())
        } else {
            None
        };

        let header = if self.header_only {
            HeaderMode::Only
        } else if self.no_header {
            HeaderMode::Off
        } else {
            HeaderMode::On
        };

        Ok(TableOptions { columns, header })
    }
}

/// `dsp` — AI-agent-friendly CLI for the DaSCH Service Platform.
#[derive(Debug, Parser)]
#[command(
    name = "dsp",
    version,
    about = "AI-agent-friendly CLI for the DaSCH Service Platform.",
    long_about = "AI-agent-friendly CLI for the DaSCH Service Platform (DSP).\n\
Run `dsp docs` to list available documentation topics.",
    max_term_width = 100
)]
pub struct Cli {
    /// Increase log verbosity (-v=info, -vv=debug, -vvv=trace). RUST_LOG overrides.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: TopLevel,
}

impl Cli {
    /// The effective output format of this invocation, per command. `None`
    /// marks a command with no user-facing formatted output whose result must
    /// not drive format-gated side effects (e.g. `auth token`, a raw-credential
    /// pipe). `docs` has no `--format`: `-j` → Json, else Prose.
    ///
    /// Layer-neutral by design: the parser resolves the format; it does not
    /// name or know what consumes it (the update-notice gate is one consumer).
    pub fn output_format(&self) -> Option<Format> {
        match &self.command {
            TopLevel::Auth { cmd } => match cmd {
                AuthCmd::Login(args) => Some(args.format.resolve()),
                AuthCmd::Status(args) => Some(args.format.resolve()),
                AuthCmd::Logout(args) => Some(args.format.resolve()),
                AuthCmd::SetToken(args) => Some(args.format.resolve()),
                AuthCmd::Token(_) => None,
            },
            TopLevel::Vre { cmd } => match cmd {
                VreCmd::Project { cmd } => match cmd {
                    ProjectCmd::List(args) => Some(args.format.resolve()),
                    ProjectCmd::Describe(args) => Some(args.format.resolve()),
                    ProjectCmd::Dump(args) => Some(args.format.resolve()),
                },
                VreCmd::DataModel { cmd } => match cmd {
                    DataModelCmd::List(args) => Some(args.format.resolve()),
                    DataModelCmd::Describe(args) => Some(args.format.resolve()),
                    DataModelCmd::Structure(args) => Some(args.format.resolve()),
                },
                VreCmd::ResourceType { cmd } => match cmd {
                    ResourceTypeCmd::List(args) => Some(args.format.resolve()),
                    ResourceTypeCmd::Describe(args) => Some(args.format.resolve()),
                },
                VreCmd::Resource { cmd } => match cmd {
                    ResourceCmd::List(args) => Some(args.format.resolve()),
                    ResourceCmd::Describe(args) => Some(args.format.resolve()),
                },
                VreCmd::Vocabulary { cmd } => match cmd {
                    VocabularyCmd::List(args) => Some(args.format.resolve()),
                    VocabularyCmd::Describe(args) => Some(args.format.resolve()),
                },
                // No Renderer, no --format (D2, plan 035) — same carve-out as
                // AuthCmd::Token(_): stdout is a store-authored byte stream,
                // not dsp-cli's envelope, so the update-notice gate stays silent.
                VreCmd::Sparql { cmd } => match cmd {
                    SparqlCmd::Query(_) => None,
                },
            },
            TopLevel::Docs(args) => Some(if args.json { Format::Json } else { Format::Prose }),
        }
    }

    /// The `--server`/`-s` value for this invocation, if the command has one and it
    /// was supplied. `None` for `docs` (no server flag) or when unset. Used by the
    /// top-level error handler for best-effort `_meta.server` (D3 of plan 032).
    pub fn server_flag(&self) -> Option<&str> {
        match &self.command {
            TopLevel::Auth { cmd } => match cmd {
                AuthCmd::Login(args) => args.server.as_deref(),
                AuthCmd::Status(args) => args.server.as_deref(),
                AuthCmd::Logout(args) => args.server.as_deref(),
                AuthCmd::SetToken(args) => args.server.as_deref(),
                AuthCmd::Token(args) => args.server.as_deref(),
            },
            TopLevel::Vre { cmd } => match cmd {
                VreCmd::Project { cmd } => match cmd {
                    ProjectCmd::List(args) => args.server.as_deref(),
                    ProjectCmd::Describe(args) => args.server.as_deref(),
                    ProjectCmd::Dump(args) => args.server.as_deref(),
                },
                VreCmd::DataModel { cmd } => match cmd {
                    DataModelCmd::List(args) => args.server.as_deref(),
                    DataModelCmd::Describe(args) => args.server.as_deref(),
                    DataModelCmd::Structure(args) => args.server.as_deref(),
                },
                VreCmd::ResourceType { cmd } => match cmd {
                    ResourceTypeCmd::List(args) => args.server.as_deref(),
                    ResourceTypeCmd::Describe(args) => args.server.as_deref(),
                },
                VreCmd::Resource { cmd } => match cmd {
                    ResourceCmd::List(args) => args.server.as_deref(),
                    ResourceCmd::Describe(args) => args.server.as_deref(),
                },
                VreCmd::Vocabulary { cmd } => match cmd {
                    VocabularyCmd::List(args) => args.server.as_deref(),
                    VocabularyCmd::Describe(args) => args.server.as_deref(),
                },
                VreCmd::Sparql { cmd } => match cmd {
                    SparqlCmd::Query(args) => args.server.as_deref(),
                },
            },
            TopLevel::Docs(_) => None,
        }
    }
}

// Top-level command groups — areas (`vre`, `repo`) and meta-groups
// (`auth`, `docs`). See dsp-cli/ADR-0006.
#[derive(Debug, Subcommand)]
pub enum TopLevel {
    /// Authentication management.
    Auth {
        #[command(subcommand)]
        cmd: AuthCmd,
    },

    /// Virtual Research Environment (VRE) operations.
    Vre {
        #[command(subcommand)]
        cmd: VreCmd,
    },

    /// Embedded end-user documentation; `dsp docs` lists topics.
    Docs(DocsArgs),
}

// ── auth ─────────────────────────────────────────────────────────────────────

/// Auth subcommands.
#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Log in to a DSP server and cache the session token.
    ///
    /// See also: dsp docs connecting
    Login(LoginArgs),

    /// Show authentication status for a DSP server.
    Status(StatusArgs),

    /// Log out from a DSP server and clear the cached session token.
    Logout(LogoutArgs),

    /// Cache a pre-issued bearer token read from stdin.
    ///
    /// Reads a JWT from stdin, verifies it against the server with a live
    /// probe, and — only if the probe succeeds — writes it into the auth
    /// cache. Subsequent commands then reuse the token until it expires.
    ///
    /// See also: dsp docs connecting
    #[command(name = "set-token")]
    SetToken(SetTokenArgs),

    /// Print the resolved bearer token to stdout, for piping.
    ///
    /// Prints a bearer credential to stdout — see the cautions in `dsp docs
    /// connecting`.
    ///
    /// See also: dsp docs connecting
    Token(TokenArgs),
}

/// Arguments for `dsp auth login`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_AUTH_LOGIN)]
pub struct LoginArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// User identifier for authentication: an email address, a username, or a user IRI.
    /// Can also be set via the `DSP_USER` environment variable or a `.env` file.
    #[arg(short = 'u', long, env = "DSP_USER")]
    pub user: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp auth status`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_AUTH_STATUS)]
pub struct StatusArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp auth logout`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_AUTH_LOGOUT)]
pub struct LogoutArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp auth set-token`.
///
/// No `--token` flag: the token is read from stdin to avoid leaking it into
/// the shell history, `ps` output, or audit logs. See dsp-cli/ADR-0007.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_AUTH_SET_TOKEN)]
pub struct SetTokenArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp auth token`.
///
/// No `--format`/`-j`/`-l`: the token is printed verbatim, bare, with no
/// envelope. No `after_help` columns line either — there is no tabular output.
#[derive(Debug, Args)]
pub struct TokenArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,
}

// ── vre ───────────────────────────────────────────────────────────────────────

/// VRE noun-group subcommands.
#[derive(Debug, Subcommand)]
pub enum VreCmd {
    /// Manage DSP projects.
    ///
    /// A project is the top-level container on DSP. Every data-model and
    /// resource belongs to exactly one project. See also: dsp docs concepts
    Project {
        #[command(subcommand)]
        cmd: ProjectCmd,
    },

    /// Manage data-models within a project.
    ///
    /// A data-model (called "ontology" in DSP-API) defines the schema for a
    /// project's resources: the resource-types, their fields, and value-types.
    // Explicit name is load-bearing — the CLI surface is stable (dsp-cli/ADR-0002);
    // don't strip this as "redundant with default kebab-case."
    #[command(name = "data-model")]
    DataModel {
        #[command(subcommand)]
        cmd: DataModelCmd,
    },

    /// Manage resource-types within a data-model.
    ///
    /// A resource-type (called "class" in DSP-API) defines the structure of
    /// one kind of scholarly object: its fields, value-types, and cardinalities.
    // Explicit name is load-bearing — the CLI surface is stable (dsp-cli/ADR-0002);
    // don't strip this as "redundant with default kebab-case."
    #[command(name = "resource-type")]
    ResourceType {
        #[command(subcommand)]
        cmd: ResourceTypeCmd,
    },

    /// List resource instances within a project.
    ///
    /// Fetches the actual data instances (scholarly objects) stored in the DSP
    /// server for a given resource-type, with optional pagination. See also:
    /// dsp docs concepts
    Resource {
        #[command(subcommand)]
        cmd: ResourceCmd,
    },

    /// Manage controlled vocabularies within a project (DSP-API "list").
    ///
    /// See also: dsp docs concepts
    Vocabulary {
        #[command(subcommand)]
        cmd: VocabularyCmd,
    },

    /// Raw SPARQL 1.1 query passthrough to the server's own triplestore.
    ///
    /// Deliberately does NOT abstract DSP-API: the response is the store's
    /// own document, byte-exact, in whatever media type it negotiated. No
    /// Renderer, no --format. Requires a SystemAdmin token. See also: dsp
    /// docs sparql
    Sparql {
        #[command(subcommand)]
        cmd: SparqlCmd,
    },
}

// ── vre project ───────────────────────────────────────────────────────────────

/// Project verb subcommands.
#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// List all projects on the DSP server.
    ///
    /// See also: dsp docs concepts
    List(ProjectListArgs),

    /// Describe a single DSP project.
    ///
    /// See also: dsp docs concepts
    Describe(ProjectDescribeArgs),

    /// Trigger and download a project dump (a server-produced bagit-zip archive).
    ///
    /// Connects to the DSP server, triggers a server-side dump of the specified
    /// project, polls until the dump is ready, and downloads the resulting
    /// bagit-zip archive to a local file. Binary assets (images, audio, video,
    /// etc.) are included by default; pass `--skip-assets` to download only
    /// the structured RDF data.
    ///
    /// **Requires a system-administrator token.** Obtain one via
    /// `dsp auth login --server <server>` or set the `DSP_TOKEN` environment
    /// variable.
    Dump(ProjectDumpArgs),
}

/// Arguments for `dsp vre project list`.
///
/// Lists all projects on the DSP server. Use `--filter` to narrow results by a
/// case-insensitive substring match over shortcode, shortname, and longname.
/// Authentication is optional: an anonymous caller sees all public projects; an
/// authenticated caller may see additional ones depending on server policy.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_PROJECT_LIST)]
pub struct ProjectListArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Filter projects by case-insensitive substring over shortcode, shortname,
    /// and longname.
    #[arg(long)]
    pub filter: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre project describe`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_PROJECT_DESCRIBE)]
pub struct ProjectDescribeArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project to describe.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre project dump`.
///
/// Triggers a server-side project dump (a bagit-zip archive of the project's
/// data) and downloads it to a local file. Assets (images, audio, video, etc.)
/// are included by default; use `--skip-assets` to download only the
/// structured RDF data.
///
/// **Requires a system-administrator token.** Obtain one via
/// `dsp auth login --server <server>` or set the `DSP_TOKEN` environment
/// variable.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_PROJECT_DUMP)]
pub struct ProjectDumpArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project to dump.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Skip binary assets (images, audio, video, etc.); download only the
    /// project's structured RDF data. Assets are included by default.
    #[arg(long)]
    pub skip_assets: bool,

    /// Write the dump to this path instead of the default
    /// `./<shortcode>-<timestamp>.zip`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// Overwrite an existing output file. Without this flag, the command
    /// refuses to overwrite an existing path.
    #[arg(long)]
    pub force: bool,

    /// Delete the server-side dump after a successful download.
    #[arg(long)]
    pub cleanup: bool,

    /// Abort if the dump has not completed within this many seconds
    /// (must be at least 1).
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,

    /// Discard this project's existing dump and create a fresh one. If the
    /// server's single dump slot is held by a **different** project, this refuses
    /// unless `--discard-other-project` is also given.
    #[arg(long, conflicts_with = "delete")]
    pub replace: bool,

    /// Remove this project's dump without downloading. If the slot is held by a
    /// different project, this is a no-op (it never removes another project's dump).
    #[arg(
        long,
        conflicts_with_all = ["replace", "output", "force", "skip_assets", "cleanup"]
    )]
    pub delete: bool,

    /// Only valid with `--replace`. The DSP-API holds one dump server-wide; if
    /// the slot is held by a **different** project, also discard *that* project's
    /// dump to make room. Without this, `--replace` refuses when the slot belongs
    /// to another project. (Distinct from `--force`, which only governs
    /// overwriting the local output file.)
    #[arg(long, requires = "replace", conflicts_with = "delete")]
    pub discard_other_project: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

// ── vre data-model ────────────────────────────────────────────────────────────

/// Data-model verb subcommands.
#[derive(Debug, Subcommand)]
pub enum DataModelCmd {
    /// List all data-models in a project.
    ///
    /// See also: dsp docs concepts
    List(DataModelListArgs),

    /// Describe a single data-model.
    ///
    /// See also: dsp docs concepts
    Describe(DataModelDescribeArgs),

    /// Show the relations (links + inheritance) between a data-model's resource-types.
    ///
    /// See also: dsp docs concepts
    Structure(DataModelStructureArgs),
}

/// Arguments for `dsp vre data-model list`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_DATA_MODEL_LIST)]
pub struct DataModelListArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project whose data-models to list.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Filter data-models by case-insensitive substring over name and label.
    #[arg(long)]
    pub filter: Option<String>,

    /// Also list the platform built-in data-models (knora-api, standoff,
    /// salsah-gui) that every project inherits. Off by default.
    #[arg(long)]
    pub include_builtins: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre data-model describe`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_DATA_MODEL_DESCRIBE)]
pub struct DataModelDescribeArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project containing the data-model.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or IRI of the data-model to describe.
    #[arg(long = "data-model")]
    pub data_model: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre data-model structure`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_DATA_MODEL_STRUCTURE)]
pub struct DataModelStructureArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project containing the data-model.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or IRI of the data-model whose structure to show.
    #[arg(long = "data-model")]
    pub data_model: Option<String>,

    /// Also show relations to/from the platform built-in resource-types
    /// (e.g. inherits edges to `Resource` or `StillImageRepresentation`).
    /// Off by default.
    #[arg(long)]
    pub include_builtins: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

// ── vre resource-type ─────────────────────────────────────────────────────────

/// Resource-type verb subcommands.
#[derive(Debug, Subcommand)]
pub enum ResourceTypeCmd {
    /// List all resource-types in a data-model.
    ///
    /// See also: dsp docs concepts
    List(ResourceTypeListArgs),

    /// Describe a single resource-type, including its fields and value-types.
    ///
    /// See also: dsp docs concepts
    Describe(ResourceTypeDescribeArgs),
}

/// Arguments for `dsp vre resource-type list`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_RESOURCE_TYPE_LIST)]
pub struct ResourceTypeListArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or IRI of the data-model containing the resource-types.
    #[arg(long = "data-model")]
    pub data_model: Option<String>,

    /// Filter resource-types by case-insensitive substring over name and label.
    #[arg(long)]
    pub filter: Option<String>,

    /// Also list the platform built-in resource-types a user can instantiate
    /// (Region, AudioSegment, VideoSegment, LinkObj) that every project inherits.
    /// Off by default.
    #[arg(long)]
    pub include_builtins: bool,

    /// Also fetch and show instance counts per resource-type (one extra HTTP
    /// call). Off by default. Counts are non-deleted but NOT permission-filtered
    /// (unlike `resource list`) — a disclosure note is emitted when this flag is
    /// used.
    #[arg(long)]
    pub count: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre resource-type describe`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_RESOURCE_TYPE_DESCRIBE)]
pub struct ResourceTypeDescribeArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or IRI of the data-model containing the resource-type.
    #[arg(long = "data-model")]
    pub data_model: Option<String>,

    /// Name or IRI of the resource-type to describe.
    #[arg(long = "resource-type")]
    pub resource_type: Option<String>,

    /// Also show the built-in (platform) fields every resource inherits
    /// (arkUrl, permissions, timestamps, …). Off by default.
    #[arg(long)]
    pub include_builtins: bool,

    /// Also fetch and show instance counts per resource-type (one extra HTTP
    /// call). Off by default. Counts are non-deleted but NOT permission-filtered
    /// (unlike `resource list`) — a disclosure note is emitted when this flag is
    /// used.
    #[arg(long)]
    pub count: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

// ── vre resource ──────────────────────────────────────────────────────────────

/// Resource verb subcommands.
#[derive(Debug, Subcommand)]
pub enum ResourceCmd {
    /// List resource instances of a given type within a project.
    ///
    /// Fetches the actual data instances stored in DSP for a resource-type.
    /// Supports single-page (`--page N`) and all-pages (`--all`) modes.
    /// Authentication is optional; anonymous callers see only public resources.
    ///
    /// See also: dsp docs concepts
    List(ResourceListArgs),

    /// Fetch the envelope metadata of a single resource by its internal IRI.
    ///
    /// Returns the resource's label, resource-type, IRI, ARK URL, creation and
    /// last-modification dates, owning project, owner, visibility, and your
    /// access level. Field values (the actual data) are omitted by default;
    /// pass `--values` to include them.
    ///
    /// Use `--resource` with the resource's internal IRI. ARK addressing is not
    /// supported in v1 — use the internal IRI directly. Optionally, supply
    /// `--project` to guard that the resource belongs to the expected project.
    ///
    /// Authentication is optional; anonymous callers see only public resources.
    ///
    /// See also: dsp docs concepts
    Describe(ResourceDescribeArgs),
}

/// Arguments for `dsp vre resource list`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_RESOURCE_LIST)]
pub struct ResourceListArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or full IRI of the resource-type to list instances of.
    /// A bare name triggers a scan across all project data-models; use
    /// `--data-model` or a full IRI (`://` heuristic) to skip the scan.
    #[arg(long = "resource-type")]
    pub resource_type: Option<String>,

    /// Name or IRI of the data-model to scope the resource-type search.
    /// Optional; narrows the bare-name scan to one data-model.
    #[arg(long = "data-model")]
    pub data_model: Option<String>,

    /// Page number to fetch (zero-based). Cannot be combined with `--all`.
    /// Defaults to page 0 when neither `--page` nor `--all` is given.
    #[arg(long, value_parser = clap::value_parser!(u32), conflicts_with = "all")]
    pub page: Option<u32>,

    /// Fetch all pages until the server reports no more results.
    /// Cannot be combined with `--page`.
    #[arg(long)]
    pub all: bool,

    /// Filter resources by case-insensitive substring over the label.
    #[arg(long)]
    pub filter: Option<String>,

    /// Field name (e.g. `title`) or full field IRI to sort by (ascending).
    /// A bare field name is resolved to the resource-type's field IRI;
    /// a full IRI (contains `://`) is passed to the server verbatim.
    /// Targets project-defined fields; ascending order only.
    #[arg(long = "order-by")]
    pub order_by: Option<String>,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre resource describe`.
///
/// Fetches the envelope metadata of a single resource by its internal IRI.
/// Authentication is optional; anonymous callers see only publicly-visible
/// resources. Use `--project` to assert that the resource belongs to the
/// expected project (a cross-project guard — fails if the resource's
/// attached project does not match).
///
/// **ARK addressing is not supported in v1.** Use the resource's internal IRI
/// (e.g. `http://rdfh.ch/0803/AbCdEf`) as returned by `dsp vre resource list`.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_RESOURCE_DESCRIBE)]
pub struct ResourceDescribeArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Internal IRI of the resource to describe (e.g. `http://rdfh.ch/0803/AbCdEf`).
    /// ARK addressing is not supported in v1 — use the internal IRI directly.
    /// Run `dsp vre resource list` to discover resource IRIs.
    #[arg(long)]
    pub resource: Option<String>,

    /// Shortcode, shortname, or IRI of the expected project. When given, the
    /// command fails with a usage error unless the resource's attached project
    /// matches this value. Omit to describe a resource regardless of project.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Include the resource's field values in the output (off by default; metadata
    /// envelope only when omitted). In `prose` and `json` this adds a values
    /// section on top of the metadata; in tabular formats (`csv`, `tsv`, `lines`)
    /// the output becomes one row per value instead of the metadata row.
    /// Resolving field and vocabulary-item labels requires additional server requests.
    /// See also: `dsp docs concepts`
    #[arg(long)]
    pub values: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

// ── vre vocabulary ───────────────────────────────────────────────────────────

/// Vocabulary verb subcommands.
#[derive(Debug, Subcommand)]
pub enum VocabularyCmd {
    /// List a project's vocabularies.
    ///
    /// See also: dsp docs concepts
    List(VocabularyListArgs),

    /// Describe a single vocabulary's full tree.
    ///
    /// See also: dsp docs concepts
    Describe(VocabularyDescribeArgs),
}

/// Arguments for `dsp vre vocabulary list`.
///
/// Lists a project's vocabularies (DSP-API "lists"). Authentication is
/// optional — vocabularies are public data (D2/schema-side read).
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_VOCABULARY_LIST)]
pub struct VocabularyListArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project.
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Filter vocabularies by case-insensitive substring over name and every label.
    #[arg(long)]
    pub filter: Option<String>,

    /// Also fetch each vocabulary's full tree to show node/depth counts (one
    /// extra HTTP request per vocabulary). Off by default.
    #[arg(long)]
    pub count: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

/// Arguments for `dsp vre vocabulary describe`.
///
/// Describes a single vocabulary's full tree, with no depth limit and no
/// pagination. A node IRI (e.g. pasted from `resource describe --values`)
/// resolves upward to its vocabulary automatically and is marked in the
/// output; use `--subtree` to narrow output to that node's own branch.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_VOCABULARY_DESCRIBE)]
pub struct VocabularyDescribeArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// Shortcode, shortname, or IRI of the project. Required only when
    /// --vocabulary is a bare name (a full IRI needs no project).
    #[arg(short = 'p', long)]
    pub project: Option<String>,

    /// Name or IRI of the vocabulary (or one of its nodes) to describe.
    #[arg(long = "vocabulary")]
    pub vocabulary: Option<String>,

    /// Narrow output to the addressed node's own branch. Requires a node
    /// IRI (a bare name or a root IRI is a usage error).
    #[arg(long)]
    pub subtree: bool,

    #[command(flatten)]
    pub format: FormatArgs,
}

// ── vre sparql ───────────────────────────────────────────────────────────────

/// Sparql verb subcommands.
#[derive(Debug, Subcommand)]
pub enum SparqlCmd {
    /// Run a raw SPARQL 1.1 query against the server's triplestore.
    ///
    /// See also: dsp docs sparql
    Query(SparqlQueryArgs),
}

/// Arguments for `dsp vre sparql query`.
///
/// No `--format`/`-j`/`-l`/`--columns`/`--no-header` (D2, plan 035): the
/// response body is a store-authored document in a store-negotiated media
/// type, and dsp-cli's envelope would corrupt or double-encode it. Copied
/// from `TokenArgs`' precedent (D2) — the third no-Renderer command.
#[derive(Debug, Args)]
#[command(after_help = AFTER_HELP_SPARQL_QUERY)]
pub struct SparqlQueryArgs {
    /// DSP server URL or shortcut (e.g. `https://api.example.org` or `prod`).
    /// Can also be set via the `DSP_SERVER` environment variable or a `.env` file.
    #[arg(short = 's', long, env = "DSP_SERVER")]
    pub server: Option<String>,

    /// The SPARQL query text. Mutually exclusive with --query-file; if
    /// neither is given, the query is read from stdin. See --help's note on
    /// shell-history/`ps` exposure.
    #[arg(long, conflicts_with = "query_file")]
    pub query: Option<String>,

    /// Path to a file containing the SPARQL query text. Mutually exclusive
    /// with --query. A leading-dash path is accepted as-is
    /// (`--query-file -foo.rq`), via `allow_hyphen_values`.
    #[arg(long, allow_hyphen_values = true)]
    pub query_file: Option<String>,

    /// Requested response media type: an alias (json/xml/csv/tsv/turtle/
    /// ntriples/jsonld) or a raw media type containing '/'. Defaults to
    /// `json` (application/sparql-results+json) — the store's own default,
    /// with no Accept sent, is XML. See --help for the full alias table and
    /// the Fuseki silent-fallback caveat.
    #[arg(long)]
    pub accept: Option<String>,

    /// Client-side request timeout, in whole seconds. Bounds the request
    /// DOWN from the server's own guardrail; it cannot raise it. Default:
    /// 3600 (one hour) — the server's own ~135s deadline is what ends a slow
    /// query in practice.
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,
}

// ── docs ──────────────────────────────────────────────────────────────────────

/// Arguments for `dsp docs [topic]`.
#[derive(Debug, Args)]
pub struct DocsArgs {
    /// Topic to display. Omit to list all topics. Topics: dsp-cli, dsp,
    /// concepts, identifiers, connecting, output, workflows, errors,
    /// dsp-tools, sparql. Run `dsp docs` for one-line descriptions.
    pub topic: Option<String>,

    /// Page the output through $PAGER (default `less`).
    #[arg(long, conflicts_with = "json")]
    pub pager: bool,

    /// Emit the topic index as machine-readable JSON. Cannot be combined with a
    /// topic name or --pager.
    #[arg(
        short = 'j',
        long = "json",
        conflicts_with_all = ["topic", "pager"]
    )]
    pub json: bool,
}

// ── Unit tests for FormatArgs::table_options and after_help drift guard ──────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{
        AUTH_LOGIN_COLUMNS, AUTH_LOGOUT_COLUMNS, DATA_MODEL_DESCRIBE_COLUMNS, DATA_MODEL_STRUCTURE_COLUMNS,
        DATA_MODELS_COLUMNS, Format, HeaderMode, PROJECT_DUMP_COLUMNS, PROJECT_DUMP_DELETED_COLUMNS, PROJECTS_COLUMNS,
        RESOURCE_DESCRIBE_COLUMNS, RESOURCE_DESCRIBE_VALUES_COLUMNS, RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS,
        RESOURCE_LIST_COLUMNS, RESOURCE_TYPE_DESCRIBE_COLUMNS, RESOURCE_TYPES_COLUMNS, VOCABULARIES_COLUMNS,
        VOCABULARY_DESCRIBE_COLUMNS,
    };

    /// Helper: construct a minimal FormatArgs with only the given flags set;
    /// all others default to "unset / false / None".
    fn fmt_args(
        format: Format,
        json: bool,
        lines: bool,
        columns: Option<&str>,
        no_header: bool,
        header_only: bool,
    ) -> FormatArgs {
        FormatArgs {
            format,
            json,
            lines,
            columns: columns.map(|s| s.to_string()),
            no_header,
            header_only,
        }
    }

    // ── format-combination validation ─────────────────────────────────────────

    #[test]
    fn columns_with_prose_default_rejected() {
        // --columns with the default (prose) format → Usage error.
        let args = fmt_args(Format::Prose, false, false, Some("name"), false, false);
        let result = args.table_options(Format::Prose);
        assert!(matches!(result, Err(Diagnostic::Usage(_))), "expected Usage, got {result:?}");
    }

    #[test]
    fn columns_with_json_rejected() {
        // --columns -j → resolved format is Json → Usage error.
        let args = fmt_args(Format::Prose, true, false, Some("name"), false, false);
        let resolved = args.resolve(); // Json
        let result = args.table_options(resolved);
        assert!(matches!(result, Err(Diagnostic::Usage(_))), "expected Usage, got {result:?}");
    }

    #[test]
    fn columns_with_lines_accepted() {
        // --columns with -l (lines) → ok.
        let args = fmt_args(Format::Prose, false, true, Some("name"), false, false);
        let resolved = args.resolve(); // Lines
        let result = args.table_options(resolved);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let opts = result.unwrap();
        assert_eq!(opts.columns, Some(vec!["name".to_string()]));
    }

    #[test]
    fn columns_with_format_lines_flag_accepted() {
        // --columns --format lines → resolved via the --format branch (not -l).
        let args = fmt_args(Format::Lines, false, false, Some("iri"), false, false);
        let resolved = args.resolve(); // Lines (via --format)
        let result = args.table_options(resolved);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let opts = result.unwrap();
        assert_eq!(opts.columns, Some(vec!["iri".to_string()]));
    }

    #[test]
    fn columns_with_csv_accepted() {
        let args = fmt_args(Format::Csv, false, false, Some("shortcode,iri"), false, false);
        let result = args.table_options(Format::Csv);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let opts = result.unwrap();
        assert_eq!(opts.columns, Some(vec!["shortcode".to_string(), "iri".to_string()]));
    }

    // ── header-flag validation ────────────────────────────────────────────────

    #[test]
    fn no_header_with_prose_rejected() {
        // --no-header with the default prose format → Usage.
        let args = fmt_args(Format::Prose, false, false, None, true, false);
        let resolved = args.resolve(); // Prose
        let result = args.table_options(resolved);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for --no-header + prose, got {result:?}"
        );
    }

    #[test]
    fn header_only_with_prose_rejected() {
        // --header-only with the default prose format → Usage.
        let args = fmt_args(Format::Prose, false, false, None, false, true);
        let resolved = args.resolve(); // Prose
        let result = args.table_options(resolved);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for --header-only + prose, got {result:?}"
        );
    }

    #[test]
    fn no_header_with_lines_rejected() {
        // --no-header with lines → Usage (lines has no header concept).
        let args = fmt_args(Format::Prose, false, true, None, true, false);
        let resolved = args.resolve(); // Lines
        let result = args.table_options(resolved);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for --no-header + lines, got {result:?}"
        );
    }

    #[test]
    fn header_only_with_json_rejected() {
        // --header-only with -j → Usage.
        let args = fmt_args(Format::Prose, true, false, None, false, true);
        let resolved = args.resolve(); // Json
        let result = args.table_options(resolved);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for --header-only + json, got {result:?}"
        );
    }

    #[test]
    fn no_header_with_csv_accepted() {
        let args = fmt_args(Format::Csv, false, false, None, true, false);
        let opts = args.table_options(Format::Csv).unwrap();
        assert_eq!(opts.header, HeaderMode::Off);
    }

    #[test]
    fn header_only_with_tsv_accepted() {
        let args = fmt_args(Format::Tsv, false, false, None, false, true);
        let opts = args.table_options(Format::Tsv).unwrap();
        assert_eq!(opts.header, HeaderMode::Only);
    }

    #[test]
    fn default_gives_header_on() {
        let args = fmt_args(Format::Csv, false, false, None, false, false);
        let opts = args.table_options(Format::Csv).unwrap();
        assert_eq!(opts.header, HeaderMode::On);
    }

    // ── column syntax validation ──────────────────────────────────────────────

    #[test]
    fn empty_columns_value_rejected() {
        let args = fmt_args(Format::Csv, false, false, Some(""), false, false);
        let result = args.table_options(Format::Csv);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for empty --columns, got {result:?}"
        );
    }

    #[test]
    fn blank_segment_rejected() {
        // "a,,b" has an empty middle segment.
        let args = fmt_args(Format::Csv, false, false, Some("a,,b"), false, false);
        let result = args.table_options(Format::Csv);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for blank segment, got {result:?}"
        );
    }

    #[test]
    fn duplicate_rejected() {
        let args = fmt_args(Format::Csv, false, false, Some("iri,iri"), false, false);
        let result = args.table_options(Format::Csv);
        assert!(
            matches!(result, Err(Diagnostic::Usage(_))),
            "expected Usage for duplicate column, got {result:?}"
        );
    }

    #[test]
    fn single_column_accepted() {
        let args = fmt_args(Format::Lines, false, false, Some("iri"), false, false);
        let opts = args.table_options(Format::Lines).unwrap();
        assert_eq!(opts.columns, Some(vec!["iri".to_string()]));
    }

    #[test]
    fn multiple_columns_select_and_reorder() {
        // Columns come back in the user-supplied order (the engine honours it).
        let args = fmt_args(Format::Csv, false, false, Some("iri,shortcode,label"), false, false);
        let opts = args.table_options(Format::Csv).unwrap();
        assert_eq!(
            opts.columns,
            Some(vec!["iri".to_string(), "shortcode".to_string(), "label".to_string()])
        );
    }

    #[test]
    fn no_columns_gives_none() {
        let args = fmt_args(Format::Csv, false, false, None, false, false);
        let opts = args.table_options(Format::Csv).unwrap();
        assert_eq!(opts.columns, None);
    }

    // ── drift-guard: after_help column list must match per-noun consts ─────────
    //
    // Each assertion checks that the corresponding AFTER_HELP_* constant's column
    // list exactly matches `<CONST>.join(", ")`.  Adding a column to the const
    // without updating the literal (or vice versa) fails this test, preventing
    // silent drift between the runtime engine and the help text.
    //
    // RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS (and any other "lean default" consts)
    // are intentionally NOT listed here: they are internal engine defaults, not
    // user-facing column sets. The drift-guard covers all_columns consts only —
    // those are the valid names documented in each command's --help output.
    //
    // PROJECT_DUMP uses a bespoke two-mode literal rather than a join, so it is
    // tested separately against both component consts.

    /// Extract the column list from an after_help string of the form
    /// "Columns (--columns): col1, col2, ..."  or the dump variant
    /// "Columns (--columns): path (with --delete: deleted)".
    ///
    /// Returns everything after the ": " that follows "Columns (--columns)".
    fn extract_columns_part(after_help: &str) -> &str {
        after_help
            .strip_prefix("Columns (--columns): ")
            .expect("after_help must start with 'Columns (--columns): '")
    }

    #[test]
    fn after_help_matches_consts() {
        // Each pair: (after_help_const, column_const_as_joined_string).
        // Uses a Vec so a new pair is one line.
        let cases: &[(&str, &[&str])] = &[
            (AFTER_HELP_PROJECT_LIST, PROJECTS_COLUMNS),
            (AFTER_HELP_PROJECT_DESCRIBE, PROJECTS_COLUMNS),
            (AFTER_HELP_DATA_MODEL_LIST, DATA_MODELS_COLUMNS),
            (AFTER_HELP_DATA_MODEL_DESCRIBE, DATA_MODEL_DESCRIBE_COLUMNS),
            (AFTER_HELP_DATA_MODEL_STRUCTURE, DATA_MODEL_STRUCTURE_COLUMNS),
            (AFTER_HELP_RESOURCE_TYPE_LIST, RESOURCE_TYPES_COLUMNS),
            (AFTER_HELP_RESOURCE_TYPE_DESCRIBE, RESOURCE_TYPE_DESCRIBE_COLUMNS),
            (AFTER_HELP_AUTH_LOGIN, AUTH_LOGIN_COLUMNS),
            (AFTER_HELP_AUTH_STATUS, AUTH_LOGIN_COLUMNS),
            (AFTER_HELP_AUTH_LOGOUT, AUTH_LOGOUT_COLUMNS),
            (AFTER_HELP_AUTH_SET_TOKEN, AUTH_LOGIN_COLUMNS),
        ];

        for (help_str, const_cols) in cases {
            let extracted = extract_columns_part(help_str);
            let expected = const_cols.join(", ");
            assert_eq!(
                extracted, expected,
                "after_help drift for \"{help_str}\": \
                 help says \"{extracted}\" but const says \"{expected}\""
            );
        }

        // PROJECT_DUMP is bespoke (two-mode literal) — check it structurally.
        let dump_extracted = extract_columns_part(AFTER_HELP_PROJECT_DUMP);
        assert!(
            dump_extracted.starts_with(PROJECT_DUMP_COLUMNS[0]),
            "AFTER_HELP_PROJECT_DUMP must start with PROJECT_DUMP_COLUMNS[0] (\"path\"); \
             got \"{dump_extracted}\""
        );
        assert!(
            dump_extracted.contains(PROJECT_DUMP_DELETED_COLUMNS[0]),
            "AFTER_HELP_PROJECT_DUMP must contain PROJECT_DUMP_DELETED_COLUMNS[0] (\"deleted\"); \
             got \"{dump_extracted}\""
        );

        // RESOURCE_LIST has a multi-line after_help (columns line + scan-behaviour
        // summary). Check that the first line exactly matches the column const.
        let rl_extracted = extract_columns_part(AFTER_HELP_RESOURCE_LIST);
        let rl_first_line = rl_extracted
            .split('\n')
            .next()
            .expect("AFTER_HELP_RESOURCE_LIST must have at least one line");
        let rl_expected = RESOURCE_LIST_COLUMNS.join(", ");
        assert_eq!(
            rl_first_line, rl_expected,
            "AFTER_HELP_RESOURCE_LIST columns line must match RESOURCE_LIST_COLUMNS; \
             got \"{rl_first_line}\" but expected \"{rl_expected}\""
        );

        // RESOURCE_DESCRIBE also has a multi-line after_help (columns + "See also").
        // Check that the first line exactly matches the column const.
        let rd_extracted = extract_columns_part(AFTER_HELP_RESOURCE_DESCRIBE);
        let rd_first_line = rd_extracted
            .split('\n')
            .next()
            .expect("AFTER_HELP_RESOURCE_DESCRIBE must have at least one line");
        let rd_expected = RESOURCE_DESCRIBE_COLUMNS.join(", ");
        assert_eq!(
            rd_first_line, rd_expected,
            "AFTER_HELP_RESOURCE_DESCRIBE columns line must match RESOURCE_DESCRIBE_COLUMNS; \
             got \"{rd_first_line}\" but expected \"{rd_expected}\""
        );

        // RESOURCE_DESCRIBE also documents the --values column set (full + lean
        // default). Locate each sibling line by its distinct prefix and
        // exact-match the remainder — these prefixes don't collide with the
        // "Columns (--columns): " check above (that one anchors on the first
        // line of the whole string).
        let rd_values_prefix = "Columns (--columns) with --values: ";
        let rd_values_line = AFTER_HELP_RESOURCE_DESCRIBE
            .lines()
            .find(|l| l.starts_with(rd_values_prefix))
            .expect("AFTER_HELP_RESOURCE_DESCRIBE must have a --values columns line");
        let rd_values_extracted = rd_values_line
            .strip_prefix(rd_values_prefix)
            .expect("prefix already matched by find()");
        let rd_values_expected = RESOURCE_DESCRIBE_VALUES_COLUMNS.join(", ");
        assert_eq!(
            rd_values_extracted, rd_values_expected,
            "AFTER_HELP_RESOURCE_DESCRIBE --values columns line must match \
             RESOURCE_DESCRIBE_VALUES_COLUMNS; got \"{rd_values_extracted}\" but expected \
             \"{rd_values_expected}\""
        );

        let rd_values_default_prefix = "Default columns with --values: ";
        let rd_values_default_line = AFTER_HELP_RESOURCE_DESCRIBE
            .lines()
            .find(|l| l.starts_with(rd_values_default_prefix))
            .expect("AFTER_HELP_RESOURCE_DESCRIBE must have a default --values columns line");
        let rd_values_default_extracted = rd_values_default_line
            .strip_prefix(rd_values_default_prefix)
            .expect("prefix already matched by find()");
        let rd_values_default_expected = RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS.join(", ");
        assert_eq!(
            rd_values_default_extracted, rd_values_default_expected,
            "AFTER_HELP_RESOURCE_DESCRIBE default --values columns line must match \
             RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS; got \"{rd_values_default_extracted}\" \
             but expected \"{rd_values_default_expected}\""
        );

        // VOCABULARY_LIST also has a multi-line after_help (columns line plus
        // --filter/--count disclosure notes). Check that the first line exactly
        // matches the column const.
        let vl_extracted = extract_columns_part(AFTER_HELP_VOCABULARY_LIST);
        let vl_first_line = vl_extracted
            .split('\n')
            .next()
            .expect("AFTER_HELP_VOCABULARY_LIST must have at least one line");
        let vl_expected = VOCABULARIES_COLUMNS.join(", ");
        assert_eq!(
            vl_first_line, vl_expected,
            "AFTER_HELP_VOCABULARY_LIST columns line must match VOCABULARIES_COLUMNS; \
             got \"{vl_first_line}\" but expected \"{vl_expected}\""
        );

        // VOCABULARY_DESCRIBE also has a multi-line after_help (columns line plus
        // several explanatory paragraphs). Check that the first line exactly
        // matches the column const.
        let vd_extracted = extract_columns_part(AFTER_HELP_VOCABULARY_DESCRIBE);
        let vd_first_line = vd_extracted
            .split('\n')
            .next()
            .expect("AFTER_HELP_VOCABULARY_DESCRIBE must have at least one line");
        let vd_expected = VOCABULARY_DESCRIBE_COLUMNS.join(", ");
        assert_eq!(
            vd_first_line, vd_expected,
            "AFTER_HELP_VOCABULARY_DESCRIBE columns line must match VOCABULARY_DESCRIBE_COLUMNS; \
             got \"{vd_first_line}\" but expected \"{vd_expected}\""
        );
    }

    // ── Cli::output_format ────────────────────────────────────────────────────

    #[test]
    fn output_format_vre_project_list_prose_default() {
        let cli = Cli::try_parse_from(["dsp", "vre", "project", "list"]).unwrap();
        assert_eq!(cli.output_format(), Some(Format::Prose));
    }

    #[test]
    fn output_format_vre_project_list_json_flag() {
        let cli = Cli::try_parse_from(["dsp", "vre", "project", "list", "-j"]).unwrap();
        assert_eq!(cli.output_format(), Some(Format::Json));
    }

    #[test]
    fn output_format_vre_project_list_lines_flag() {
        let cli = Cli::try_parse_from(["dsp", "vre", "project", "list", "-l"]).unwrap();
        assert_eq!(cli.output_format(), Some(Format::Lines));
    }

    #[test]
    fn output_format_docs_topic_is_prose() {
        let cli = Cli::try_parse_from(["dsp", "docs", "concepts"]).unwrap();
        assert_eq!(cli.output_format(), Some(Format::Prose));
    }

    #[test]
    fn output_format_docs_json_flag() {
        let cli = Cli::try_parse_from(["dsp", "docs", "-j"]).unwrap();
        assert_eq!(cli.output_format(), Some(Format::Json));
    }

    #[test]
    fn output_format_auth_token_is_none() {
        let cli = Cli::try_parse_from(["dsp", "auth", "token", "-s", "dev"]).unwrap();
        assert_eq!(cli.output_format(), None);
    }

    // ── Cli::server_flag ──────────────────────────────────────────────────────

    #[test]
    fn server_flag_with_flag_supplied() {
        // An explicit --server always wins over any ambient DSP_SERVER env var
        // (clap precedence: explicit CLI arg > env), so no env guarding needed here.
        let cli = Cli::try_parse_from(["dsp", "vre", "project", "list", "--server", "dev"]).unwrap();
        assert_eq!(cli.server_flag(), Some("dev"));
    }

    #[test]
    fn server_flag_none_when_unset() {
        // Every leaf Args struct's `server` field carries `#[arg(env =
        // "DSP_SERVER")]`, so there is no way to exercise the "genuinely
        // unset" arm via `Cli::try_parse_from` without either mutating the
        // real process environment (unsound here: `cargo test` runs unit
        // tests in parallel, so one thread clearing `DSP_SERVER` while
        // another thread's `try_parse_from` reads it is a data race — which
        // is exactly why `std::env::remove_var` requires `unsafe` in current
        // Rust) or serializing this test against every other test that
        // touches `DSP_SERVER` (no such crate/pattern — e.g. `serial_test` —
        // exists in this project, and one isn't worth adding for a single
        // test).
        //
        // Instead, construct the `Cli` value directly, bypassing clap
        // parsing (and thus the env read) entirely. This still exercises the
        // real match-arm + `.as_deref()` logic in `server_flag()` — it just
        // reaches the `None` field value by construction instead of by
        // parsing absent input, so it is deterministic with zero env risk.
        let cli = Cli {
            verbose: 0,
            command: TopLevel::Vre {
                cmd: VreCmd::Project {
                    cmd: ProjectCmd::List(ProjectListArgs {
                        server: None,
                        filter: None,
                        format: fmt_args(Format::Prose, false, false, None, false, false),
                    }),
                },
            },
        };
        assert_eq!(cli.server_flag(), None);
    }

    #[test]
    fn server_flag_docs_is_none() {
        let cli = Cli::try_parse_from(["dsp", "docs", "concepts"]).unwrap();
        assert_eq!(cli.server_flag(), None);
    }
}
