//! Every committed project passes the format rules. A finding on a published
//! value would refuse the next submission of that project, whatever it changed,
//! so a rule that fails one of these files is too strict for the corpus.

use std::collections::BTreeSet;
use std::path::PathBuf;

use editor_core::draft::ProjectDraft;
use editor_core::form::format::{findings, ValueKind};
use editor_core::form::registry::FIELDS;
use serde_json::{json, Value};
use shared_metadata::project::ProjectRaw;

fn project_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(editor_core::checkout_dpe_data_dir().join("projects"))
        .expect("the projects data directory should be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no project files were found");
    files
}

fn draft_of(path: &PathBuf) -> ProjectDraft {
    let committed = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let raw: ProjectRaw =
        serde_json::from_str(&committed).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
    ProjectDraft::from_raw(&raw)
}

/// The controls of `draft` holding a finding, over every field. A committed project is its own
/// published side, so a value published over its cap is no finding.
fn controls(draft: &ProjectDraft) -> Vec<String> {
    findings(FIELDS.iter(), draft, None, Some(draft))
        .into_iter()
        .map(|finding| finding.control)
        .collect()
}

#[test]
fn every_committed_project_has_no_format_finding() {
    let failing: Vec<String> = project_files()
        .iter()
        .filter_map(|path| {
            let found = controls(&draft_of(path));
            (!found.is_empty()).then(|| format!("{}: {found:?}", path.display()))
        })
        .collect();
    assert!(failing.is_empty(), "committed values a format rule refuses: {failing:#?}");
}

#[test]
fn a_stored_placeholder_reference_has_no_finding_where_a_malformed_one_does() {
    // 0110 stores the corpus placeholder as its temporal and spatial reference links.
    let path = project_files()
        .into_iter()
        .find(|path| path.file_name().is_some_and(|name| name == "0110_h-steiner.json"))
        .expect("0110_h-steiner is committed");
    let mut draft = draft_of(&path);
    assert_eq!(
        draft.get("spatialCoverage"),
        Some(&json!([{ "type": "URL", "url": "MISSING" }]))
    );
    assert!(controls(&draft).is_empty(), "{:?}", controls(&draft));

    draft.set("spatialCoverage", json!([{ "type": "URL", "url": "www.dasch.swiss" }]));
    assert_eq!(controls(&draft), ["spatialCoverage.r0.ref.url"]);
}

fn strings(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        Value::Object(members) => members.values().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn a_field_without_a_format_holds_no_committed_web_address() {
    // A field a depositor fills with web addresses needs a format declared, or a
    // malformed one reaches the public page unchecked.
    let mut undeclared = BTreeSet::new();
    for path in project_files() {
        let draft = draft_of(&path);
        for field in FIELDS.iter().filter(|field| field.shape.is_some() && field.format.is_empty()) {
            let links = draft
                .get(field.id)
                .map(strings)
                .unwrap_or_default()
                .into_iter()
                .any(|text| ValueKind::Link.accepts(text.trim()));
            if links {
                undeclared.insert(field.id);
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "fields holding web addresses with no format: {undeclared:?}"
    );
}
