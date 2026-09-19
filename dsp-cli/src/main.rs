use std::process::ExitCode;

use clap::Parser;
use dsp_cli::cli::Cli;
use dsp_cli::config::Config;
use dsp_cli::diagnostic::ExitCategory;
use dsp_cli::render::MetaContext;

fn main() -> ExitCode {
    // Snapshot the raw process environment before dotenvy can touch it — see
    // `cross_context_token_risk`'s doc for why this before/after pair is enough
    // to tell "came from .env" from "came from the caller's shell" apart.
    let dsp_token_before_dotenv = std::env::var("DSP_TOKEN").ok();
    let dsp_server_present_before_dotenv = std::env::var_os("DSP_SERVER").is_some();

    // Must precede `Cli::parse()` so clap's `env =` attributes see `.env` values.
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    dsp_cli::diagnostic::init_tracing(cli.verbose);

    if cross_context_token_risk(
        dsp_token_before_dotenv.as_deref(),
        dsp_server_present_before_dotenv,
        std::env::var_os("DSP_SERVER").is_some(),
    ) {
        // One-shot, before dispatch (and so before any authenticated request):
        // DSP_TOKEN is the caller's own, but the server it would be sent to was
        // just pinned by a .env file in this directory, not by the caller.
        tracing::warn!(
            "DSP_TOKEN is set in your shell environment, but DSP_SERVER comes from a .env file in \
the current directory — you may be sending a token acquired for one server to a different one. \
See `dsp docs connecting`."
        );
    }

    // Computed before `run(cli)` moves `cli`.
    let notice_fmt = cli.output_format();
    let server_flag = cli.server_flag().map(str::to_owned);
    let allow_insecure = cli.allow_insecure_server;
    let result = dsp_cli::run(cli);

    let code = match result {
        Ok(()) => ExitCategory::Success as u8,
        Err(diag) => {
            let code = diag.exit_category() as u8;
            match notice_fmt {
                Some(fmt) => {
                    // Best-effort server label; re-resolving is side-effect-free (flag/env/.env,
                    // no network) and matches the success-path label. Unresolvable → omitted (D3).
                    // NB: this is a deliberate second call to the same resolver run() used —
                    // it must stay in lockstep with run()'s resolution or the error-path label
                    // could diverge from what a successful run would show. Acceptable for a
                    // top-level-only concern; do not plumb Config out of run() for this.
                    let server_label = Config::resolve(server_flag.as_deref(), allow_insecure)
                        .ok()
                        .map(|c| c.server)
                        .unwrap_or_default();
                    let meta = MetaContext {
                        server_label,
                        auth_state: String::new(), // omitted at top level (D3)
                        filter_warning: None,
                        count_caveat: None,
                        count_cost: None,
                    };
                    let mut renderer = fmt.into_renderer();
                    let _ = renderer.diagnostic(&diag, &meta); // must not mask the original error
                }
                None => eprintln!("Error: {diag}"), // auth token (D4)
            }
            code
        }
    };

    // Runs regardless of the command's outcome (gated + rate-limited internally);
    // the advisory is deliberately the last thing written to stderr (dsp-cli/ADR-0015).
    // Never touches `code` or the exit path.
    dsp_cli::update::maybe_notify(notice_fmt);

    ExitCode::from(code)
}

/// True when `DSP_TOKEN` came from the caller's own shell environment but
/// `DSP_SERVER` was newly introduced by a `.env` file in the current
/// directory — i.e. the effective bearer token and the effective server were
/// picked from two different contexts, and the token may not be meant for
/// that server.
///
/// `dotenvy::dotenv()` never overrides an already-set variable (dsp-cli/ADR-0007
/// "`.env` loading from CWD"), so a before/after snapshot of the raw process
/// environment is sufficient to tell the two sources apart: a `DSP_SERVER`
/// absent before `dotenvy::dotenv()` ran and present after it must have come
/// from `.env`. This is one targeted comparison, not a general-purpose
/// provenance framework — `config::token::TokenOrigin` tracks a different
/// axis (env vs. on-disk auth cache) and its consumers live outside this
/// crate area's file list, so it is deliberately left untouched.
///
/// `dsp_token_before_dotenv` mirrors `resolve_token`'s trim-and-empty rule
/// (`src/config/token.rs`): a blank `DSP_TOKEN=""` in the ambient shell does
/// not count as "set".
fn cross_context_token_risk(
    dsp_token_before_dotenv: Option<&str>,
    dsp_server_present_before_dotenv: bool,
    dsp_server_present_after_dotenv: bool,
) -> bool {
    let token_from_process_env = dsp_token_before_dotenv.map(|t| !t.trim().is_empty()).unwrap_or(false);
    let server_newly_from_dotenv = !dsp_server_present_before_dotenv && dsp_server_present_after_dotenv;
    token_from_process_env && server_newly_from_dotenv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_when_token_from_process_env_and_server_newly_from_dotenv() {
        assert!(cross_context_token_risk(Some("a-real-token"), false, true));
    }

    #[test]
    fn no_warning_when_server_was_already_present_before_dotenv() {
        // DSP_SERVER was already in the ambient shell — not sourced from .env,
        // even though it's also present afterwards.
        assert!(!cross_context_token_risk(Some("a-real-token"), true, true));
    }

    #[test]
    fn no_warning_when_dotenv_introduced_no_server() {
        assert!(!cross_context_token_risk(Some("a-real-token"), false, false));
    }

    #[test]
    fn no_warning_when_token_absent() {
        assert!(!cross_context_token_risk(None, false, true));
    }

    #[test]
    fn no_warning_when_token_blank() {
        assert!(!cross_context_token_risk(Some("   "), false, true));
    }
}
