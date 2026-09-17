//! Corpus-wide tests over the committed data in `modules/dpe/server/data`.
//!
//! These live in `dpe-api-oai`, beside the data they read, because the mapping functions and the
//! XML builder they exercise are crate-private and a shared crate holds no path into a service
//! module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dpe_core::CachedContributorLookup;
// `coverage_name` is the same lookup-key derivation `resolve_temporal_coverage_in`
// uses (Reference → `text`; Text map → `get_multilingual_value`), shared via
// shared-metadata so the two can't drift apart.
use shared_metadata::temporal_coverage::coverage_name;
use shared_metadata::{ProjectRaw, Record};

use super::{to_oai_record, to_oai_record_from_record, OaiRecord};
use crate::handlers::test_utils::normalize;
use crate::xml::OaiXmlBuilder;

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data");

/// The baseline is gitignored (`.claude/tmp/`) rather than committed: it is ~102,000 lines of
/// several MB, and every intermediate commit of the refactor it guards needs the file present for
/// the comparison to mean anything, so it could not be squashed back out of `main`'s history
/// afterwards.
const BASELINE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.claude/tmp/oai-baseline-hashes.txt");

const WRITE_ENV: &str = "OAI_HASH_BASELINE_WRITE";

const PREFIXES: [&str; 2] = ["oai_dc", "oai_datacite"];

/// FNV-1a 64-bit. Spelled out here rather than taken from `DefaultHasher`, whose output is not
/// stable across Rust versions — this baseline has to survive a toolchain bump.
fn fnv1a_64(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn hash_of(record: &OaiRecord) -> String {
    let mut builder = OaiXmlBuilder::new();
    builder.write_record(record);
    // `OaiXmlBuilder::new()` stamps `Utc::now()` into the envelope, so the raw string is not
    // deterministic; `normalize` drops that one line.
    fnv1a_64(normalize(&builder.finish()).as_bytes())
}

/// `read_dir` order is arbitrary, so the paths are sorted before anything reads them.
fn sorted_json_files(dir: &Path) -> Vec<PathBuf> {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("data directory {} should be readable: {e}", dir.display()));
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
}

/// Computes one `identifier \t prefix \t hash` line per committed project and record, in both
/// metadata prefixes, sorted lexicographically.
///
/// Lines, not a map: the five `0801` project files share one PID and so one OAI identifier, and
/// keying on `identifier \t prefix` would drop four of them from the baseline entirely.
fn compute_lines() -> Vec<String> {
    dpe_core::set_data_dir(DATA_DIR);
    assert_eq!(
        dpe_core::get_data_dir(),
        DATA_DIR,
        "another test initialised the process-global data dir first; the caches below would be \
         keyed on the wrong directory and the hashes would be meaningless"
    );
    assert!(
        !dpe_core::temporal_enrichment_cache::all_enriched().is_empty(),
        "committed temporal-coverage enrichment table should load and be non-empty"
    );

    let data_dir = Path::new(DATA_DIR);
    let clusters = dpe_core::cluster_cache::all_clusters();
    let lookup = CachedContributorLookup;

    let mut lines = Vec::new();

    for path in sorted_json_files(&data_dir.join("projects")) {
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("project file {} should be readable: {e}", path.display()));
        let raw = serde_json::from_str::<ProjectRaw>(&json)
            .unwrap_or_else(|e| panic!("project file {} should parse: {e}", path.display()));
        for prefix in PREFIXES {
            let oai_record = to_oai_record(&raw, prefix, clusters, &lookup);
            lines.push(format!(
                "{}\t{}\t{}",
                oai_record.header.identifier,
                prefix,
                hash_of(&oai_record)
            ));
        }
    }

    for path in sorted_json_files(&data_dir.join("records")) {
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("record dump {} should be readable: {e}", path.display()));
        let records = serde_json::from_str::<Vec<Record>>(&json)
            .unwrap_or_else(|e| panic!("record dump {} should parse: {e}", path.display()));
        for record in &records {
            for prefix in PREFIXES {
                let oai_record = to_oai_record_from_record(record, prefix, clusters);
                lines.push(format!(
                    "{}\t{}\t{}",
                    oai_record.header.identifier,
                    prefix,
                    hash_of(&oai_record)
                ));
            }
        }
    }

    lines.sort();
    lines
}

fn parse(contents: &str) -> Vec<String> {
    let mut lines: Vec<String> = contents.lines().filter(|l| !l.is_empty()).map(str::to_string).collect();
    lines.sort();
    lines
}

/// Groups baseline lines by their `identifier \t prefix` key, so a mismatch can be reported per
/// entity rather than as two large line lists.
fn by_key(lines: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in lines {
        let (key, hash) = line
            .rsplit_once('\t')
            .unwrap_or_else(|| panic!("baseline line is not tab-separated: {line}"));
        grouped.entry(key.replace('\t', " ")).or_default().push(hash.to_string());
    }
    grouped
}

fn report_mismatch(expected: &[String], actual: &[String]) -> String {
    let expected = by_key(expected);
    let actual = by_key(actual);

    let mut differing = Vec::new();
    let mut missing = 0usize;
    for (key, expected_hashes) in &expected {
        match actual.get(key) {
            Some(actual_hashes) if actual_hashes == expected_hashes => {}
            Some(actual_hashes) => differing.push((key.clone(), expected_hashes.join(","), actual_hashes.join(","))),
            None => missing += 1,
        }
    }
    let extra = actual.keys().filter(|key| !expected.contains_key(*key)).count();

    let mut message = format!(
        "OAI output no longer matches the hash baseline at {BASELINE_PATH}\n\
         {} differing, {missing} missing (in baseline, not produced), {extra} extra (produced, not in baseline)\n",
        differing.len()
    );
    for (key, expected_hashes, actual_hashes) in differing.iter().take(20) {
        message.push_str(&format!("  {key}: expected {expected_hashes}, got {actual_hashes}\n"));
    }
    if differing.len() > 20 {
        message.push_str(&format!("  ... and {} more\n", differing.len() - 20));
    }
    message
}

/// Pins the DataCite and Dublin Core XML of every committed project and record so a refactor can
/// prove byte-identical output.
///
/// Three modes: with `OAI_HASH_BASELINE_WRITE` set it writes the baseline; with the baseline
/// present it compares; with neither it skips, because CI has no `.claude/tmp/`.
#[test]
#[ignore = "run explicitly with --ignored: this test sets the process-global data dir and so cannot \
            share a test process with the handler tests, which initialise it to the relative default \
            on their first DataCite render"]
fn oai_output_matches_hash_baseline() {
    let started = std::time::Instant::now();
    let path = Path::new(BASELINE_PATH);

    if std::env::var_os(WRITE_ENV).is_some() {
        let lines = compute_lines();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("baseline directory {} should be creatable: {e}", parent.display()));
        }
        let mut contents = lines.join("\n");
        contents.push('\n');
        std::fs::write(path, contents)
            .unwrap_or_else(|e| panic!("baseline file {BASELINE_PATH} should be writable: {e}"));
        println!("wrote {} entries to {BASELINE_PATH}", lines.len());
        println!("took {:.1?}", started.elapsed());
        return;
    }

    let Ok(contents) = std::fs::read_to_string(path) else {
        eprintln!("oai hash baseline absent at {BASELINE_PATH}; skipping. Set {WRITE_ENV}=1 to create it.");
        return;
    };

    let expected = parse(&contents);
    let actual = compute_lines();
    assert!(expected == actual, "{}", report_mismatch(&expected, &actual));
    println!("compared {} entries", actual.len());
    println!("took {:.1?}", started.elapsed());
}

/// Completeness guard over the committed project data: every distinct
/// `temporalCoverage` entry across all in-repo project files must resolve to a
/// usable date through the *real* period and enrichment tables.
///
/// "Resolved" means a non-empty `date` (a ChronOntology timespan or an
/// enrichment range). A name-only fallback (empty `date`) counts as
/// UNRESOLVED and fails the test — the point of the check is that every path
/// carries a machine-readable range, not merely a label.
///
/// The sole exception is a name explicitly reviewed as *not a time period*: an
/// enrichment row with no `date` and `source == "unresolved"` (e.g. "Swiss",
/// "English (culture or style)"). Those are intentionally emitted as
/// `dateInformation`-only, so they are allowed to stay name-only. Any other
/// empty-date entry is a genuine gap in the enrichment table.
///
/// Data, periods, and enrichment are loaded through the same parse logic as
/// production (`ProjectRaw`, `chronontology_cache::load_from`,
/// `temporal_enrichment_cache::load_from`), resolved relative to this crate so
/// the test does not depend on the process working directory or on global
/// cache state.
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
