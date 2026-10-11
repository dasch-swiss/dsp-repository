//! `access-server`, the Access Area's composition root: CLI, config loading,
//! observability, the OTel layers and the untraced routes. Mounts DPE's
//! router (`dpe-server`) and, under its base, CPE's (`cpe-server`), which it
//! activates over `sync`'s archive projection (`sync-store`).

use std::process::ExitCode;

use clap::Parser;

mod cli;
mod observability;
mod serve;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();

    match parsed.command {
        None => {
            // No subcommand: print help and exit
            use clap::CommandFactory;
            cli::Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
        Some(cli::Commands::Serve) => serve::serve(),
        Some(cli::Commands::Validate { data_dir }) => dpe_server::validate(data_dir),
        Some(cli::Commands::Healthcheck { url }) => cli::healthcheck(&url),
    }
}
