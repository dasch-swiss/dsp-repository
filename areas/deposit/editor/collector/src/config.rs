//! What the workflow passes in, and the two shapes it can be asked for.

use std::path::PathBuf;

/// The project metadata this collector writes into.
///
/// A cross-module path, deliberately: the collector runs inside a checkout of
/// this repository and its whole job is to write that directory. It is a field
/// on [`Config`] rather than a constant at the write sites so a test can point
/// the run loop at a fixture.
const DATA_DIR: &str = editor_core::DPE_DATA_DIR;

/// What a run was asked to do.
///
/// `Refresh` exists because a reported state is only as fresh as the last run:
/// a pull request merged with reviewer edits after a collect would otherwise
/// read `open` forever, with nobody prompted. It writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Collect,
    Refresh,
}

impl Mode {
    /// Parses the single positional argument the workflow passes.
    pub fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [mode] if mode == "collect" => Ok(Self::Collect),
            [mode] if mode == "refresh" => Ok(Self::Refresh),
            [mode] => Err(format!("unknown mode {mode:?}; expected `collect` or `refresh`")),
            [] => Err("no mode given; expected `collect` or `refresh`".to_string()),
            _ => Err("too many arguments; expected exactly one of `collect` or `refresh`".to_string()),
        }
    }
}

/// The environment one run needs.
///
/// No `Debug`: [`Self::collection_token`] is in here, and a `{:?}` added while
/// diagnosing something would put it in a CI log.
pub struct Config {
    /// The editor's origin, without a trailing slash.
    pub editor_base_url: String,
    /// The bearer token `POST /api/v1/collection-report` verifies. Never logged.
    pub collection_token: String,
    /// `owner/name`, which also bounds the pull request URLs a report may carry.
    pub repository: String,
    pub data_dir: PathBuf,
}

impl Config {
    /// Reads the environment, naming every missing variable at once.
    ///
    /// The workflow guards the same two variables before checkout, so this is
    /// the second of two gates rather than the only one: GitHub substitutes an
    /// empty string for an unset secret rather than failing the run, and an
    /// empty token would otherwise open pull requests and then take 401 on
    /// every report.
    pub fn from_env() -> Result<Self, String> {
        let mut missing = Vec::new();
        let editor_base_url = required("EDITOR_BASE_URL", &mut missing);
        let collection_token = required("EDITOR_COLLECTION_TOKEN", &mut missing);
        let repository = required("COLLECTOR_REPOSITORY", &mut missing);

        if !missing.is_empty() {
            return Err(format!("not configured: {}", missing.join(", ")));
        }

        Ok(Self {
            editor_base_url: editor_base_url.trim_end_matches('/').to_string(),
            collection_token,
            repository,
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

    #[test]
    fn a_mode_is_the_one_positional_argument() {
        assert_eq!(Mode::parse(&["collect".to_string()]), Ok(Mode::Collect));
        assert_eq!(Mode::parse(&["refresh".to_string()]), Ok(Mode::Refresh));
    }

    #[test]
    fn anything_but_the_two_modes_is_refused_by_name() {
        let error = Mode::parse(&["publish".to_string()]).expect_err("an unknown mode is refused");
        assert!(error.contains("publish"), "{error}");

        assert!(Mode::parse(&[]).is_err());
        assert!(Mode::parse(&["collect".to_string(), "refresh".to_string()]).is_err());
    }

    /// Both are refused in one message: an operator fixing repository settings
    /// should not have to re-run to discover the second one.
    #[test]
    fn every_missing_variable_is_named_at_once() {
        let mut missing = Vec::new();
        required("COLLECTOR_TEST_ABSENT_ONE", &mut missing);
        required("COLLECTOR_TEST_ABSENT_TWO", &mut missing);
        assert_eq!(missing, ["COLLECTOR_TEST_ABSENT_ONE", "COLLECTOR_TEST_ABSENT_TWO"]);
    }
}
