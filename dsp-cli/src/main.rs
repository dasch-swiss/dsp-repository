use std::process::ExitCode;

use clap::Parser;

use dsp_cli::cli::Cli;
use dsp_cli::config::Config;
use dsp_cli::diagnostic::ExitCategory;
use dsp_cli::render::MetaContext;

fn main() -> ExitCode {
    // Must precede `Cli::parse()` so clap's `env =` attributes see `.env` values.
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    dsp_cli::diagnostic::init_tracing(cli.verbose);

    // Computed before `run(cli)` moves `cli`.
    let notice_fmt = cli.output_format();
    let server_flag = cli.server_flag().map(str::to_owned);
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
                    let server_label = Config::resolve(server_flag.as_deref())
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
    // the advisory is deliberately the last thing written to stderr (ADR-0015).
    // Never touches `code` or the exit path.
    dsp_cli::update::maybe_notify(notice_fmt);

    ExitCode::from(code)
}
