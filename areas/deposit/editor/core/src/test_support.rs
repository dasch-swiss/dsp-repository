use std::path::PathBuf;

use shared_metadata::project::ProjectRaw;

/// A committed project file used as the starting point for unit tests.
///
/// `0102_tanner.json` in particular: it has a `spatialCoverage` reference, grant
/// funding, and no `imageCredit`, which is the last-declared field and so the
/// one that shows whether a member landed in declaration order or was sorted.
pub fn sample_project() -> PathBuf {
    crate::checkout_dpe_data_dir().join("projects/0102_tanner.json")
}

pub fn sample_raw() -> ProjectRaw {
    let json = std::fs::read_to_string(sample_project()).expect("sample project file should be readable");
    serde_json::from_str(&json).expect("sample project file should parse")
}
