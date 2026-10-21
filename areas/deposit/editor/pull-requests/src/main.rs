//! Turns the editor's approved records into one pull request per project.
//!
//! `docs/src/editor/publishing-to-git.md` is the contract this implements and wins on
//! any difference. The two things it is easiest to get wrong, and which the
//! modules below hold: pull-request state is keyed on the project's shortcode,
//! never on the record id, and an **open** pull request is force-pushed onto
//! rather than joined by a second one.

mod config;
mod editor;
mod enrichment;
mod forge;
mod json;
mod layout;
mod manifest;
mod renumber;
mod run;

use std::process::ExitCode;

use config::Config;

const BASE_BRANCH: &str = "main";

fn main() -> ExitCode {
    match execute() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<ExitCode, String> {
    let config = Config::from_env()?;

    let editor = editor::Editor::new(&config.editor_base_url)?;
    let forge = forge::CommandForge::new(&config.repository, BASE_BRANCH, std::path::Path::new("."));

    let summary = run::open_pull_requests(&config, &editor, &forge)?;

    println!(
        "{} records considered: {} opened, {} updated, {} skipped, {} failed",
        summary.considered, summary.opened, summary.updated, summary.skipped, summary.failed
    );

    // A failed record fails the job: a green run is what would stop anyone
    // reading the job summary.
    if summary.failed > 0 {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}
