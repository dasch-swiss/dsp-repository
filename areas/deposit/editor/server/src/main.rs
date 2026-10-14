//! `editor-server`, the metadata editor's composition root.
//!
//! A separate service from `dpe-server` on purpose: authenticated and writing, where DPE is
//! public and read-only. They share crates, never a process, an image or an origin.

use std::process::ExitCode;

use clap::Parser;

mod assets;
mod cli;
mod config;
#[cfg(test)]
mod cross_capability;
mod csrf;
mod editor;
mod identity;
mod observability;
mod page_url;
#[cfg(test)]
mod removed_account;
mod router;
mod serve;
mod shell;
#[cfg(test)]
mod test_support;
mod traceparent;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();

    match parsed.command {
        None => {
            use clap::CommandFactory;
            cli::Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
        Some(cli::Commands::Serve) => serve::serve(),
        Some(cli::Commands::Healthcheck { url }) => cli::healthcheck(&url),
    }
}
