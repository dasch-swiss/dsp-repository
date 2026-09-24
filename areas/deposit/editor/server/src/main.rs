//! `editor-server`, the metadata editor's composition root.
//!
//! A separate service from `dpe-server` on purpose: authenticated and writing, where DPE is
//! public and read-only. They share crates, never a process, an image or an origin.

use std::process::ExitCode;

use clap::Parser;

mod accounts;
mod assets;
mod auth;
mod cli;
mod collection;
mod config;
mod csrf;
mod db;
mod depositors;
mod entities;
mod mail;
mod observability;
mod page_url;
mod projects;
mod reconcile;
mod review;
mod router;
mod sections;
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
