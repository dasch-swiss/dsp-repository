//! Corpus-wide tests over the committed data in `modules/dpe/server/data`.
//!
//! They live in `dpe-api-oai`, beside the data they read: a shared crate holds no path into a
//! service module, which `.github/scripts/check-shared-paths.sh` enforces.

use std::path::Path;

// `coverage_name` is the same lookup-key derivation `ProjectGraph::build` uses
// (Reference → `text`; Text map → `multilingual_value`), shared via
// shared-metadata so the two can't drift apart.
use shared_metadata::temporal_coverage::coverage_name;
use shared_metadata::ProjectRaw;

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data");

#[test]
fn every_committed_temporal_coverage_resolves() {
    let data_dir = Path::new(DATA_DIR);
    let projects_dir = data_dir.join("projects");

    let periods = shared_metadata::chronontology::load_from(data_dir);
    let enriched = shared_metadata::temporal_enrichment::load_from(data_dir);
    assert!(!enriched.is_empty(), "committed enrichment table should load and be non-empty");

    let entries = std::fs::read_dir(&projects_dir).expect("projects data directory should be readable");

    let mut seen = std::collections::HashSet::new();
    let mut unresolved = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let json = std::fs::read_to_string(&path).expect("project file should be readable");
        let raw =
            serde_json::from_str::<ProjectRaw>(&json).unwrap_or_else(|e| panic!("failed to parse {filename}: {e}"));

        for tc in &raw.temporal_coverage {
            let Some(name) = coverage_name(tc) else {
                continue; // no name to key on; nothing to resolve.
            };
            if !seen.insert(name) {
                continue; // already checked this distinct name.
            }

            // The same gap decision `dpe-server validate` applies, so the
            // two can't drift apart.
            if let Some(name) = shared_metadata::temporal_coverage::completeness_gap(tc, &periods, &enriched) {
                unresolved.push(name);
            }
        }
    }

    unresolved.sort();
    assert!(
        unresolved.is_empty(),
        "temporalCoverage names with no resolved date (add a W3CDTF range to \
         {ENRICHMENT_FILE}, or mark source=\"unresolved\" if not a time period):\n{}",
        unresolved.join("\n"),
    );
}

const ENRICHMENT_FILE: &str = "temporal-coverage-enrichment.json";
