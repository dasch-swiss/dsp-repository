//! Corpus-wide tests over the committed data in `modules/dpe/server/data`.
//!
//! They live in `dpe-api-oai`, beside the data they read: a shared crate holds no path into a
//! service module, which `.github/scripts/check-shared-paths.sh` enforces.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use shared_fair::{
    project_to_datacite, project_to_dublin_core, record_to_datacite, record_to_dublin_core, ProjectGraph, RecordGraph,
    ResolveContext,
};
// `coverage_name` is the same lookup-key derivation `ProjectGraph::build` uses
// (Reference → `text`; Text map → `multilingual_value`), shared via
// shared-metadata so the two can't drift apart.
use shared_metadata::temporal_coverage::coverage_name;
use shared_metadata::{ContributorLookup, Organization, Person, ProjectRaw, Record};

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data");

/// How many records are taken from each end of a committed dump.
///
/// The largest dump holds 27 026 records; building a graph and running both
/// writers over every one of them would dominate `just test`. The ends are where
/// the shape of a dump differs, and this is the bound the plan sets for Phase 2's
/// agreement test, which grows out of this one.
const RECORD_SAMPLE: usize = 100;

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

/// Loads every JSON file in a corpus directory into a map keyed by the entity's
/// own `id`, the way `dpe-core`'s caches do.
fn load_by_id<T: serde::de::DeserializeOwned>(dir: &Path, id_of: fn(&T) -> String) -> HashMap<String, T> {
    sorted_json_files(dir)
        .into_iter()
        .map(|path| {
            let json =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
            let entity =
                serde_json::from_str::<T>(&json).unwrap_or_else(|e| panic!("{} should parse: {e}", path.display()));
            (id_of(&entity), entity)
        })
        .collect()
}

/// The committed contributor corpus, read straight off disk.
///
/// `dpe_core::CachedContributorLookup` would do the same job, but only through
/// the process-global data dir — which is what forced the hash test this
/// replaces to be `#[ignore]`d, because a test that sets it cannot share a
/// process with the handler tests. Reading the same files directly keeps this
/// guard in `just test`.
struct CorpusContributorLookup {
    persons: HashMap<String, Person>,
    organizations: HashMap<String, Organization>,
}

impl CorpusContributorLookup {
    fn load(data_dir: &Path) -> Self {
        Self {
            persons: load_by_id(&data_dir.join("persons"), |p: &Person| p.id.clone()),
            organizations: load_by_id(&data_dir.join("organizations"), |o: &Organization| o.id.clone()),
        }
    }
}

impl ContributorLookup for CorpusContributorLookup {
    fn person(&self, id: &str) -> Option<Person> {
        self.persons.get(id).cloned()
    }

    fn organization(&self, id: &str) -> Option<Organization> {
        self.organizations.get(id).cloned()
    }
}

/// Every committed project and a sample of every committed record dump, built
/// into a graph and put through all four writers.
///
/// It asserts nothing about the output. The point is the corpus itself: the
/// graphs and the writers index, slice and unwrap over data no unit-test fixture
/// reproduces, and until Phase 2's agreement test lands nothing else runs them
/// over the real files. `extract_year`'s byte-slice panic is the shape of bug
/// this catches.
#[test]
fn every_committed_project_and_a_record_sample_survive_all_four_writers() {
    let data_dir = Path::new(DATA_DIR);
    let periods = shared_metadata::chronontology::load_from(data_dir);
    let enriched = shared_metadata::temporal_enrichment::load_from(data_dir);
    let lookup = CorpusContributorLookup::load(data_dir);
    assert!(
        !lookup.persons.is_empty(),
        "committed person corpus should load and be non-empty"
    );
    let ctx = ResolveContext::new(&lookup, &periods, &enriched);

    let mut projects = 0usize;
    for path in sorted_json_files(&data_dir.join("projects")) {
        let json =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
        let raw = serde_json::from_str::<ProjectRaw>(&json)
            .unwrap_or_else(|e| panic!("{} should parse: {e}", path.display()));

        // No records: `parts` feeds no writer, which is why the OAI call site
        // passes an empty slice too.
        let graph = ProjectGraph::build(&raw, &ctx, &[]);
        project_to_datacite(&graph);
        project_to_dublin_core(&graph);
        projects += 1;
    }
    assert!(projects > 0, "committed project corpus should not be empty");

    let mut records = 0usize;
    for path in sorted_json_files(&data_dir.join("records")) {
        let json =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
        let dump = serde_json::from_str::<Vec<Record>>(&json)
            .unwrap_or_else(|e| panic!("{} should parse: {e}", path.display()));

        let head = dump.iter().take(RECORD_SAMPLE);
        let tail = dump.iter().skip(dump.len().saturating_sub(RECORD_SAMPLE));
        for record in head.chain(tail) {
            let graph = RecordGraph::build(record);
            record_to_datacite(&graph);
            record_to_dublin_core(&graph);
            records += 1;
        }
    }
    assert!(records > 0, "committed record dumps should not be empty");
}

/// The shortcode index serves exactly what the scan it replaced served.
///
/// `records_for_shortcode` repoints the OAI `set=project:{shortcode}` filter
/// from an O(corpus) scan of the flat record vector to an index built once. The
/// sequence must not move: `ListRecords` pages it, and a resumption token is an
/// offset into that page sequence. So this runs the index builder over the
/// committed dumps — the same flat vector the cache builds from them — and
/// compares it, record for record, against the filter expression it replaced.
#[test]
fn the_shortcode_index_serves_what_the_scan_served() {
    let mut all: Vec<Record> = Vec::new();
    let mut largest_dump = 0usize;
    for path in sorted_json_files(&Path::new(DATA_DIR).join("records")) {
        let json =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
        let dump = serde_json::from_str::<Vec<Record>>(&json)
            .unwrap_or_else(|e| panic!("{} should parse: {e}", path.display()));
        largest_dump = largest_dump.max(dump.len());
        all.extend(dump);
    }
    assert!(!all.is_empty(), "committed record dumps should not be empty");

    let index = dpe_core::record_cache::index_by_shortcode(&all);
    let shortcodes: std::collections::BTreeSet<String> = all.iter().map(|r| r.pid.shortcode.to_uppercase()).collect();
    for shortcode in &shortcodes {
        let scanned: Vec<String> = all
            .iter()
            .filter(|r| r.pid.shortcode.eq_ignore_ascii_case(shortcode))
            .map(|r| r.pid.as_url())
            .collect();
        let indexed: Vec<String> = index
            .get(shortcode)
            .unwrap_or_else(|| panic!("{shortcode} should have an index entry"))
            .iter()
            .map(|r| r.pid.as_url())
            .collect();
        assert_eq!(indexed, scanned, "index and scan disagree for {shortcode}");
    }

    // The largest committed dump is served whole out of its own entry, while the
    // flat vector it sits in is several times longer. That difference is the
    // scan the index removes from every `project:{shortcode}` request and from
    // every landing page.
    assert_eq!(
        index.values().map(Vec::len).max(),
        Some(largest_dump),
        "the largest index entry should be the largest committed dump"
    );
    assert!(
        all.len() > largest_dump,
        "the flat vector should hold more than the largest dump, or the index saves nothing"
    );
}
