//! What the workflow passes in.

use std::path::PathBuf;

/// The project metadata this run writes into.
///
/// A cross-module path, deliberately: the pull-request run executes inside a checkout of
/// this repository and its whole job is to write that directory. It is a field
/// on [`Config`] rather than a constant at the write sites so a test can point
/// the run loop at a fixture.
const DATA_DIR: &str = editor_core::DPE_DATA_DIR;

/// The environment one run needs.
pub struct Config {
    /// The editor's origin, without a trailing slash.
    pub editor_base_url: String,
    /// `owner/name`.
    pub repository: String,
    /// The file GitHub Actions renders as the job summary. Absent outside
    /// Actions, where a run reports to the log alone.
    pub step_summary: Option<PathBuf>,
    pub data_dir: PathBuf,
}

impl Config {
    /// Reads the environment, naming every missing variable at once.
    ///
    /// The workflow guards `EDITOR_BASE_URL` before checkout, so this is the
    /// second of two gates rather than the only one: GitHub substitutes an empty
    /// string for an unset variable rather than failing the run.
    pub fn from_env() -> Result<Self, String> {
        let mut missing = Vec::new();
        let editor_base_url = required("EDITOR_BASE_URL", &mut missing);
        let repository = required("PULL_REQUESTS_REPOSITORY", &mut missing);

        if !missing.is_empty() {
            return Err(format!("not configured: {}", missing.join(", ")));
        }

        Ok(Self {
            editor_base_url: editor_base_url.trim_end_matches('/').to_string(),
            repository,
            step_summary: std::env::var_os("GITHUB_STEP_SUMMARY").map(PathBuf::from),
            data_dir: PathBuf::from(DATA_DIR),
        })
    }
}

/// An environment variable that must be present and non-empty, recording its
/// name when it is neither.
fn required(name: &str, missing: &mut Vec<String>) -> String {
    let value = std::env::var(name).unwrap_or_default();
    if value.trim().is_empty() {
        missing.push(name.to_string());
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both are refused in one message: an operator fixing repository settings
    /// should not have to re-run to discover the second one.
    #[test]
    fn every_missing_variable_is_named_at_once() {
        let mut missing = Vec::new();
        required("PULL_REQUESTS_TEST_ABSENT_ONE", &mut missing);
        required("PULL_REQUESTS_TEST_ABSENT_TWO", &mut missing);
        assert_eq!(missing, ["PULL_REQUESTS_TEST_ABSENT_ONE", "PULL_REQUESTS_TEST_ABSENT_TWO"]);
    }
}
