//! `dpe-server`, DPE's composition root.

use std::process::ExitCode;

use clap::Parser;

mod ark;
mod assets;
mod cli;
mod config;
#[cfg(feature = "dev")]
mod dev_reload;
pub(crate) mod downloads;
pub(crate) mod fragments;
mod metadata;
mod observability;
mod page_url;
mod router;
mod serve;
mod shell;
#[cfg(test)]
pub(crate) mod test_support;
mod traceparent;
mod validate;
mod view;

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
        Some(cli::Commands::Validate { data_dir }) => validate::validate(data_dir),
        Some(cli::Commands::Healthcheck { url }) => cli::healthcheck(&url),
    }
}
