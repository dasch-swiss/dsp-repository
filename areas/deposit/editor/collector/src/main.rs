//! Turns the editor's approved records into one pull request per project.
//!
//! `docs/src/editor/collection.md` is the contract this implements and wins on
//! any difference. The two things it is easiest to get wrong, and which the
//! modules below hold: collection state is keyed on the project's shortcode,
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

use config::{Config, Mode};

/// The branch a collection pull request targets.
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
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mode = Mode::parse(&arguments)?;
    let config = Config::from_env()?;

    let editor = editor::Editor::new(&config.editor_base_url, &config.collection_token)?;
    let forge = forge::CommandForge::new(&config.repository, BASE_BRANCH, std::path::Path::new("."));

    let summary = match mode {
        Mode::Collect => run::collect(&config, &editor, &forge)?,
        Mode::Refresh => run::refresh(&editor, &forge)?,
    };

    println!(
        "{} records considered: {} published, {} inspected, {} failed, {} unreported",
        summary.considered, summary.published, summary.skipped, summary.failed, summary.unreported
    );

    // A record that failed, or one whose outcome never reached the editor,
    // fails the job: both leave the editor's picture wrong, and a green run is
    // what would stop anyone looking.
    if summary.failed > 0 || summary.unreported > 0 {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}
