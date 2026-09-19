//! `dsp-cli` — AI-agent-friendly CLI for the DaSCH Service Platform.
//!
//! The orientation entry point is `idea.md` in the repository root; the
//! ubiquitous-language reference is `CONTEXT.md`. The internal architecture
//! is described in ADR-0008 (`docs/adr/0008-internal-architecture.md`).
//!
//! This crate exposes a library surface so tests can reach the action layer
//! without going through the binary; the binary itself is `src/main.rs`.

pub mod actions;
pub mod cli;
pub mod client;
pub mod config;
pub mod diagnostic;
pub mod model;
pub mod render;
pub mod update;
pub(crate) mod util;

use crate::cli::{
    AuthCmd, Cli, DataModelCmd, ProjectCmd, ResourceCmd, ResourceTypeCmd, SparqlCmd, TopLevel,
    VocabularyCmd, VreCmd,
};
use crate::client::http::HttpDspClient;
use crate::config::Config;
use crate::diagnostic::Diagnostic;

/// Top-level entry point used by `main.rs` and by integration tests.
///
/// Routes CLI commands to the appropriate action function. The match is
/// exhaustive — adding a new variant forces an explicit dispatch update.
///
/// For vre leaf commands and auth commands, the renderer is constructed from
/// the `--format` / `-j` / `-l` flags parsed into `FormatArgs` and resolved
/// via `FormatArgs::resolve().into_renderer()`. The auth commands additionally
/// build a `Config` from `--server`/`DSP_SERVER` and, for login, an
/// `HttpDspClient`.
pub fn run(cli: Cli) -> Result<(), Diagnostic> {
    match cli.command {
        TopLevel::Auth { cmd } => match cmd {
            AuthCmd::Login(args) => {
                let cfg = Config::resolve(args.server.as_deref())?;
                let client = HttpDspClient::new()?;
                let fmt = args.format.resolve();
                let opts = args.format.table_options(fmt)?;
                let mut renderer = fmt.into_renderer_with_options(opts);
                actions::auth::login::run(&args, &cfg, &client, &mut *renderer)
            }
            AuthCmd::Status(args) => {
                let cfg = Config::resolve(args.server.as_deref())?;
                let fmt = args.format.resolve();
                let opts = args.format.table_options(fmt)?;
                let mut renderer = fmt.into_renderer_with_options(opts);
                actions::auth::status::run(&args, &cfg, &mut *renderer)
            }
            AuthCmd::Logout(args) => {
                let cfg = Config::resolve(args.server.as_deref())?;
                let fmt = args.format.resolve();
                let opts = args.format.table_options(fmt)?;
                let mut renderer = fmt.into_renderer_with_options(opts);
                actions::auth::logout::run(&args, &cfg, &mut *renderer)
            }
            AuthCmd::SetToken(args) => {
                let cfg = Config::resolve(args.server.as_deref())?;
                let client = HttpDspClient::new()?;
                let fmt = args.format.resolve();
                let opts = args.format.table_options(fmt)?;
                let mut renderer = fmt.into_renderer_with_options(opts);
                actions::auth::set_token::run(&cfg, &client, &mut *renderer)
            }
            AuthCmd::Token(args) => {
                let cfg = Config::resolve(args.server.as_deref())?;
                actions::auth::token::run(&cfg)
            }
        },

        TopLevel::Vre { cmd } => match cmd {
            VreCmd::Project { cmd } => match cmd {
                ProjectCmd::List(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::project::list(&args, &cfg, &client, &mut *renderer)
                }
                ProjectCmd::Describe(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::project::describe(&args, &cfg, &client, &mut *renderer)
                }
                ProjectCmd::Dump(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    let mut reporter = fmt.into_progress_reporter();
                    actions::vre::project::dump(
                        &args,
                        &cfg,
                        &client,
                        &mut *renderer,
                        &mut *reporter,
                    )
                }
            },
            VreCmd::DataModel { cmd } => match cmd {
                DataModelCmd::List(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::data_model::list(&args, &cfg, &client, &mut *renderer)
                }
                DataModelCmd::Describe(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::data_model::describe(&args, &cfg, &client, &mut *renderer)
                }
                DataModelCmd::Structure(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::data_model::structure(&args, &cfg, &client, &mut *renderer)
                }
            },
            VreCmd::ResourceType { cmd } => match cmd {
                ResourceTypeCmd::List(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::resource_type::list(&args, &cfg, &client, &mut *renderer)
                }
                ResourceTypeCmd::Describe(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::resource_type::describe(&args, &cfg, &client, &mut *renderer)
                }
            },
            VreCmd::Resource { cmd } => match cmd {
                ResourceCmd::List(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::resource::list(&args, &cfg, &client, &mut *renderer)
                }
                ResourceCmd::Describe(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::resource::describe(&args, &cfg, &client, &mut *renderer)
                }
            },
            VreCmd::Vocabulary { cmd } => match cmd {
                VocabularyCmd::List(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::vocabulary::list(&args, &cfg, &client, &mut *renderer)
                }
                VocabularyCmd::Describe(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    let fmt = args.format.resolve();
                    let opts = args.format.table_options(fmt)?;
                    let mut renderer = fmt.into_renderer_with_options(opts);
                    actions::vre::vocabulary::describe(&args, &cfg, &client, &mut *renderer)
                }
            },
            // No renderer, no table_options — D2/D16: the response is a
            // store-authored byte stream, not a dsp-cli-rendered view.
            VreCmd::Sparql { cmd } => match cmd {
                SparqlCmd::Query(args) => {
                    let cfg = Config::resolve(args.server.as_deref())?;
                    let client = HttpDspClient::new()?;
                    actions::vre::sparql::run(&args, &cfg, &client)
                }
            },
        },

        TopLevel::Docs(args) => actions::docs::run(&args),
    }
}
