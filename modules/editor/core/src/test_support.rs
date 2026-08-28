//! Shared test fixtures for the project representation.
//!
//! One place holding the path into the published corpus, so moving the data
//! directory breaks one constant rather than every module that reads a sample.

use platform_metadata::project::ProjectRaw;

/// A committed project file used as the starting point for unit tests.
///
/// `0102_tanner.json` in particular: it has a `spatialCoverage` reference, grant
/// funding, and no `imageCredit`, which is the last-declared field and so the
/// one that shows whether a member landed in declaration order or was sorted.
pub const SAMPLE_PROJECT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../dpe/server/data/projects/0102_tanner.json");

/// The sample project, parsed.
pub fn sample_raw() -> ProjectRaw {
    let json = std::fs::read_to_string(SAMPLE_PROJECT).expect("sample project file should be readable");
    serde_json::from_str(&json).expect("sample project file should parse")
}
