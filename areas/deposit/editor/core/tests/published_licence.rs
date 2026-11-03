//! Every committed project carries a licence.
//!
//! A project created in the editor is collected with an empty `legalInfo`: the member is
//! display-only, so neither the depositor nor the form can fill it in, and RDU adds the licence in
//! the collection pull request. This is the gate that keeps such a pull request from merging
//! before that, since it runs in CI's required test jobs.

use shared_metadata::project::ProjectRaw;

#[test]
fn every_committed_project_has_a_licence() {
    let dir = editor_core::checkout_dpe_data_dir().join("projects");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("the projects data directory should be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no project files were found in {}", dir.display());

    let unlicensed: Vec<String> = files
        .iter()
        .filter(|path| {
            let json = std::fs::read_to_string(path).expect("a committed project should be readable");
            let project: ProjectRaw =
                serde_json::from_str(&json).unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
            project.legal_info.is_empty()
        })
        .map(|path| path.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();
    assert!(
        unlicensed.is_empty(),
        "these project files have an empty legalInfo; add the project's licence before merging: {unlicensed:?}"
    );
}
