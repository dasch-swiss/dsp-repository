//! Corpus-wide tests over the committed data in `modules/dpe/server/data`.
//!
//! They live in `dpe-api-oai`, beside the data they read: a shared crate holds no path into a
//! service module, which `.github/scripts/check-shared-paths.sh` enforces.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use shared_fair::{
    coar_access_right, project_to_datacite, project_to_datacite_json, project_to_dublin_core,
    project_to_dublin_core_meta, project_to_link_set, project_to_schema_org, record_to_datacite, record_to_dublin_core,
    script_safe_json, LinkSet, ProjectGraph, RecordGraph, ResolveContext, SchemaOrgOptions, UrlLayout,
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

/// Every representation of a committed object agrees with every other about
/// the facts they share.
///
/// This replaces the "survives all four writers" smoke test: running the
/// writers over the real corpus is still half the point — they index, slice and
/// unwrap over data no fixture reproduces, and `extract_year`'s byte-slice
/// panic is the shape of bug that catches — but from this phase on there is
/// something to compare. One graph feeds seven writers, and the guard is that
/// no two of them can describe the same object differently.
///
/// Where a writer's output legitimately differs, the comparison is against the
/// *graph*, not against another writer, and the reason is named at the
/// assertion:
///
/// - Dublin Core reads `creators` raw while everything else reads `creators_with_fallback`, because
///   `oai_dc` must not name DaSCH as the creator of a project nobody is credited with (ADR-0005).
/// - `oai_dc` prefers `officialName` and adds only the recorded alternative names; DataCite and the
///   JSON-LD take the longer of the two titles. So each title is checked for membership in the
///   graph's title set rather than for equality with another writer's pick.
/// - The meta-tag writer drops placeholders; the OAI writers carry them through. So a placeholder
///   value is compared only where both keep it.
/// - DataCite's `publicationYear` is mandatory and reads `publication_year_with_fallback`; the
///   JSON-LD's `datePublished` is optional and reads the raw `Option`, so an object with no usable
///   date gets a fallback year in DataCite and no key at all in the JSON-LD.
#[test]
fn every_representation_of_a_committed_object_agrees_with_the_others() {
    let data_dir = Path::new(DATA_DIR);
    let periods = shared_metadata::chronontology::load_from(data_dir);
    let enriched = shared_metadata::temporal_enrichment::load_from(data_dir);
    let lookup = CorpusContributorLookup::load(data_dir);
    assert!(
        !lookup.persons.is_empty(),
        "committed person corpus should load and be non-empty"
    );
    let ctx = ResolveContext::new(&lookup, &periods, &enriched);
    let datacite_schema = datacite_json_validator();

    let mut projects = 0usize;
    for path in sorted_json_files(&data_dir.join("projects")) {
        // Per file, not per identifier: the five `0801` files share one PID.
        let raw = read_json::<ProjectRaw>(&path);
        // No records: `parts` feeds only `hasPart`, which the size check below
        // covers with the largest dump there is.
        let graph = ProjectGraph::build(&raw, &ctx, std::iter::empty());
        assert_project_representations_agree(&graph, &path, &datacite_schema);
        projects += 1;
    }
    assert!(projects > 0, "committed project corpus should not be empty");

    let mut records = 0usize;
    for path in sorted_json_files(&data_dir.join("records")) {
        let dump = read_json::<Vec<Record>>(&path);
        let head = dump.iter().take(RECORD_SAMPLE);
        let tail = dump.iter().skip(dump.len().saturating_sub(RECORD_SAMPLE));
        for record in head.chain(tail) {
            assert_record_representations_agree(&RecordGraph::build(record), &path);
            records += 1;
        }
    }
    assert!(records > 0, "committed record dumps should not be empty");
}

/// A layout with stand-in URLs. The agreement is about the graph; which host
/// serves it is the consuming service's business and is tested there.
fn test_layout() -> UrlLayout {
    UrlLayout {
        landing: "https://example.test/dpe/projects/0000".to_string(),
        catalog: "https://example.test/dpe/projects".to_string(),
        representations: Vec::new(),
        oai_records: Vec::new(),
    }
}

/// DataCite's own JSON schema, compiled once for the whole corpus run.
///
/// The copy is `shared/fair/testdata/schemas/`, refreshed by the
/// `download-schemas.sh` beside it, which also records why a kernel-4.3 schema
/// checks 4.6 output and what it patches. Compiled with no remote resolution:
/// every `$ref` in it is local.
///
/// Format checking is off. This is a check of the *shape* the writer produces,
/// and `format` is an annotation in draft-07 — the crate validates it by
/// default, which is stricter than the specification. One committed license URI
/// carries a trailing space, and that is a defect in the corpus, not in the
/// writer: the XML representation and the `Link` header carry it too.
fn datacite_json_validator() -> jsonschema::Validator {
    const SCHEMA: &str = include_str!("../../../../../shared/fair/testdata/schemas/datacite-4.3-schema.json");
    let schema: serde_json::Value = serde_json::from_str(SCHEMA).expect("the DataCite JSON schema should parse");
    jsonschema::options()
        .should_validate_formats(false)
        .build(&schema)
        .expect("the DataCite JSON schema should compile")
}

fn assert_project_representations_agree(graph: &ProjectGraph, path: &Path, datacite_schema: &jsonschema::Validator) {
    let file = path.display();
    let datacite = project_to_datacite(graph);
    let datacite_json = project_to_datacite_json(&datacite);
    let dublin_core = project_to_dublin_core(graph);
    let meta = project_to_dublin_core_meta(graph);
    let urls = test_layout();
    let json_ld = project_to_schema_org(graph, &urls, SchemaOrgOptions { has_part_cap: Some(100) });
    let links = project_to_link_set(graph, &urls);

    // --- the DataCite JSON representation is a DataCite document ---
    // The one writer whose output a third party parses to a published schema
    // rather than to our own reading of it. Every error, not the first: a
    // rejected document usually breaks in more than one place, and one at a
    // time would be as many runs as there are mistakes.
    let violations: Vec<String> = datacite_schema
        .iter_errors(&datacite_json)
        .map(|error| format!("  {}: {error}", error.instance_path()))
        .collect();
    assert!(
        violations.is_empty(),
        "{file}: DataCite JSON does not validate:\n{}",
        violations.join("\n")
    );

    // --- the identifier ---
    assert_eq!(json_ld["@id"], graph.ark.as_str(), "{file}: JSON-LD @id");
    // `identifier` holds two entries: the ARK as a `PropertyValue`, and the
    // landing page URL, which is the one a FAIR assessor pointed at the page
    // has to match. Picked by shape, not by position.
    let identifiers = json_ld_values(&json_ld, "identifier");
    assert_eq!(
        identifiers
            .iter()
            .find(|entry| entry.get("@type").and_then(|kind| kind.as_str()) == Some("PropertyValue"))
            .and_then(|entry| entry.get("value"))
            .and_then(|value| value.as_str()),
        Some(graph.ark.as_str()),
        "{file}: JSON-LD identifier carries the ARK as a PropertyValue"
    );
    assert!(
        identifiers.iter().any(|entry| entry.as_str() == Some(urls.landing.as_str())),
        "{file}: JSON-LD identifier carries the landing page"
    );
    assert_eq!(datacite.identifier, graph.ark, "{file}: DataCite identifier");
    assert_eq!(
        datacite_json["identifiers"][0]["identifier"],
        graph.ark.as_str(),
        "{file}: DataCite JSON identifier"
    );
    assert_eq!(
        datacite_json["identifiers"][0]["identifierType"], "ARK",
        "{file}: DataCite JSON identifierType"
    );
    assert_eq!(dublin_core.identifiers, vec![graph.ark.clone()], "{file}: dc:identifier");
    assert_eq!(values(&meta, "DC.identifier"), vec![graph.ark.clone()], "{file}: DC.identifier");
    assert_eq!(
        hrefs(&links, "cite-as"),
        vec![graph.ark.clone()],
        "{file}: cite-as is the ARK, exactly once"
    );

    // --- titles ---
    let (title, alternatives) = graph.titles();
    let known: Vec<&str> = std::iter::once(title.as_str())
        .chain(alternatives.iter().map(String::as_str))
        .chain(graph.official_name.as_deref())
        .collect();
    assert_eq!(datacite.titles[0].title, title, "{file}: DataCite primary title");
    assert_eq!(
        datacite_json["titles"][0]["title"], title,
        "{file}: DataCite JSON primary title"
    );
    // Absent rather than a placeholder, which is the meta-tag writer's rule.
    match json_ld.get("name") {
        Some(name) => assert_eq!(name, &title.as_str(), "{file}: JSON-LD name"),
        None => assert!(
            shared_metadata::is_placeholder(&title) || title.is_empty(),
            "{file}: JSON-LD dropped a real title"
        ),
    }
    for emitted in values(&meta, "DC.title") {
        assert!(known.contains(&emitted.as_str()), "{file}: DC.title {emitted:?} invented");
    }

    // --- creators ---
    let attributed: Vec<&str> = graph.creators.iter().map(|a| a.name.as_str()).collect();
    let credited: Vec<String> = graph.creators_with_fallback().iter().map(|a| a.name.clone()).collect();
    assert_eq!(
        datacite.creators.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
        credited,
        "{file}: DataCite creators"
    );
    assert_eq!(json_ld_names(&json_ld, "creator"), credited, "{file}: JSON-LD creators");
    // `json_ld_names` reads a `name` member off each entry, which is what a
    // DataCite creator carries too — the helper is about the shape, not the
    // vocabulary.
    assert_eq!(
        json_ld_names(&datacite_json, "creators"),
        credited,
        "{file}: DataCite JSON creators"
    );
    // Dublin Core deliberately stops at the attributed creators.
    assert_eq!(dublin_core.creators, attributed, "{file}: dc:creator");
    assert_eq!(values(&meta, "DC.creator"), attributed, "{file}: DC.creator");

    let orcids: Vec<String> = graph
        .creators_with_fallback()
        .iter()
        .flat_map(|agent| agent.name_identifiers.iter())
        .filter(|id| id.scheme == "ORCID")
        .map(|id| id.identifier.clone())
        .collect();
    assert_eq!(hrefs(&links, "author"), orcids, "{file}: author links");
    assert_eq!(
        json_ld_identifiers(&json_ld, "creator", "ORCID"),
        orcids,
        "{file}: JSON-LD ORCIDs"
    );
    assert_eq!(
        datacite
            .creators
            .iter()
            .flat_map(|creator| creator.name_identifiers.iter())
            .filter(|id| id.scheme == "ORCID")
            .map(|id| id.identifier.clone())
            .collect::<Vec<_>>(),
        orcids,
        "{file}: DataCite ORCIDs"
    );

    // --- licenses ---
    let licensed: Vec<String> = distinct(
        graph
            .legal_info
            .iter()
            .map(|legal| legal.license_uri.clone())
            .filter(|uri| !uri.is_empty() && !shared_metadata::is_placeholder(uri)),
    );
    // The JSON-LD carries each licence as a node object, so the URI is read out
    // of `@id` rather than off a string.
    assert_eq!(json_ld_ids(&json_ld, "license"), licensed, "{file}: JSON-LD licenses");
    // DataCite emits one entry per `legalInfo` element and keeps an empty URI,
    // so the comparison is against the same filtered, deduplicated set.
    assert_eq!(
        distinct(
            datacite
                .rights_list
                .iter()
                .filter_map(|rights| rights.rights_uri.clone())
                .filter(|uri| !uri.is_empty() && !shared_metadata::is_placeholder(uri))
        ),
        licensed,
        "{file}: DataCite rights URIs"
    );
    assert_eq!(
        distinct(
            json_ld_values(&datacite_json, "rightsList")
                .into_iter()
                .filter_map(|rights| rights.get("rightsUri")?.as_str().map(str::to_string))
                // The same filter the XML comparison above applies: the writer
                // emits the record's empty `rightsURI` verbatim in both shapes.
                .filter(|uri| !uri.is_empty() && !shared_metadata::is_placeholder(uri))
        ),
        licensed,
        "{file}: DataCite JSON rights URIs"
    );
    // The profile allows at most one `license` link, so several means none.
    let expected_link = if licensed.len() == 1 {
        licensed.clone()
    } else {
        Vec::new()
    };
    assert_eq!(hrefs(&links, "license"), expected_link, "{file}: license link");

    // --- the publication year ---
    // DataCite's field is mandatory and falls back; schema.org's is optional
    // and omits rather than invents. Where the graph has a year, both carry it.
    match graph.publication_year {
        Some(ref year) => {
            assert_eq!(json_ld["datePublished"], year.as_str(), "{file}: JSON-LD datePublished");
            assert_eq!(datacite.publication_year, *year, "{file}: DataCite publicationYear");
        }
        None => {
            assert!(
                json_ld.get("datePublished").is_none(),
                "{file}: JSON-LD invented a datePublished"
            );
            assert_eq!(datacite.publication_year, "2015", "{file}: DataCite fallback year");
        }
    }

    // --- access rights ---
    assert_eq!(
        values(&meta, "DC.accessRights"),
        vec![coar_access_right(&graph.access_rights).to_string()],
        "{file}: DC.accessRights"
    );
    assert!(json_ld.get("conditionsOfAccess").is_some(), "{file}: conditionsOfAccess");
    assert!(json_ld.get("isAccessibleForFree").is_some(), "{file}: isAccessibleForFree");

    // Nothing is invented for a score: there is no project-level download.
    assert!(json_ld.get("distribution").is_none(), "{file}: distribution invented");
    // And no placeholder reaches a landing page.
    let rendered = script_safe_json(&json_ld);
    assert!(!rendered.contains("MISSING"), "{file}: MISSING in the JSON-LD");
    assert!(!rendered.contains("CALCULATED"), "{file}: CALCULATED in the JSON-LD");
    for (name, content) in &meta {
        assert!(!shared_metadata::is_placeholder(content), "{file}: placeholder in {name}");
    }
}

fn assert_record_representations_agree(graph: &RecordGraph, path: &Path) {
    let file = path.display();
    let datacite = record_to_datacite(graph);
    let dublin_core = record_to_dublin_core(graph);

    // Both carry the resolvable ARK URL, the one form the graph holds.
    assert_eq!(datacite.identifier, graph.ark, "{file}: DataCite identifier");
    assert_eq!(dublin_core.identifiers, vec![graph.ark.clone()], "{file}: dc:identifier");

    match graph.title {
        Some(ref title) => {
            assert_eq!(datacite.titles[0].title, *title, "{file}: DataCite title");
            assert_eq!(dublin_core.titles, vec![title.clone()], "{file}: dc:title");
        }
        None => assert!(dublin_core.titles.is_empty(), "{file}: dc:title without a title"),
    }

    assert_eq!(
        datacite.creators.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
        graph
            .creators_with_fallback()
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
        "{file}: DataCite creators"
    );
    assert_eq!(
        dublin_core.creators,
        graph.creators.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
        "{file}: dc:creator stays at the attributed authorship"
    );

    // The record path applies the same rule: DataCite falls back, the graph
    // records no year rather than inventing one.
    match graph.publication_year {
        Some(ref year) => assert_eq!(datacite.publication_year, *year, "{file}: DataCite publicationYear"),
        None => assert_eq!(datacite.publication_year, "2015", "{file}: DataCite fallback year"),
    }

    let uri = (!graph.license_uri.is_empty()).then(|| graph.license_uri.clone());
    assert_eq!(datacite.rights_list[0].rights_uri, uri, "{file}: DataCite rights URI");
    assert_eq!(
        dublin_core.rights.contains(&graph.license_uri),
        uri.is_some(),
        "{file}: dc:rights URI"
    );
}

/// Every committed project's embedded JSON-LD stays small enough to sit in a
/// `<head>`.
///
/// The cap is the reason: without it the largest project would embed 27,026
/// `hasPart` entries, megabytes of them, on every visit, and the project with
/// the most files would add a `DataDownload` to 7,716 of them. 64 KB is a
/// budget, not a limit anything enforces, so it is asserted here rather than
/// left to be noticed.
///
/// Every dump rather than the largest one alone: the largest carries no file at
/// all, so measuring it says nothing about what `distribution` costs.
#[test]
fn every_committed_project_embeds_a_small_json_ld_block() {
    let data_dir = Path::new(DATA_DIR);
    let projects: Vec<ProjectRaw> = sorted_json_files(&data_dir.join("projects"))
        .iter()
        .map(read_json::<ProjectRaw>)
        .collect();

    let periods = shared_metadata::chronontology::load_from(data_dir);
    let enriched = shared_metadata::temporal_enrichment::load_from(data_dir);
    let lookup = CorpusContributorLookup::load(data_dir);
    let ctx = ResolveContext::new(&lookup, &periods, &enriched);

    let dumps = sorted_json_files(&data_dir.join("records"));
    assert!(!dumps.is_empty(), "there should be committed record dumps");
    let mut with_a_distribution = 0;

    for path in &dumps {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        let shortcode = stem.strip_suffix("-records").unwrap_or(stem).to_string();
        let dump = read_json::<Vec<Record>>(path);
        let raw = projects
            .iter()
            .find(|raw| raw.shortcode.eq_ignore_ascii_case(&shortcode))
            .unwrap_or_else(|| panic!("no committed project for the dump {shortcode}"));

        let graph = ProjectGraph::build(raw, &ctx, dump.iter().take(100));
        let embedded = script_safe_json(&project_to_schema_org(
            &graph,
            &test_layout(),
            SchemaOrgOptions { has_part_cap: Some(100) },
        ));
        // The cap is what is being measured, so the block has to carry it.
        assert_eq!(
            graph.parts.len(),
            100,
            "the dump {shortcode} should fill the cap, or this measures nothing"
        );
        assert!(embedded.contains("hasPart"), "no hasPart in {shortcode}'s embedded block");
        assert!(
            embedded.len() < 64 * 1024,
            "the embedded JSON-LD for {shortcode} is {} bytes, over the 64 KB budget",
            embedded.len()
        );
        if embedded.contains("DataDownload") {
            with_a_distribution += 1;
        }
    }

    // Without this the budget could be met by emitting no download at all.
    assert!(
        with_a_distribution > 0,
        "no committed project's first 100 records produced a distribution, so the budget measures nothing about one"
    );
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> T {
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{} should parse: {e}", path.display()))
}

/// The values of one `<meta name="DC.*">` name, in emission order.
fn values(meta: &[(&'static str, String)], name: &str) -> Vec<String> {
    meta.iter()
        .filter(|(tag, _)| *tag == name)
        .map(|(_, value)| value.clone())
        .collect()
}

fn hrefs(links: &LinkSet, rel: &str) -> Vec<String> {
    links
        .iter()
        .filter(|link| link.rel == rel)
        .map(|link| link.href.clone())
        .collect()
}

fn distinct(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// A JSON-LD property holding one value, an array of them, or nothing.
fn json_ld_values<'a>(doc: &'a serde_json::Value, key: &str) -> Vec<&'a serde_json::Value> {
    match doc.get(key) {
        None => Vec::new(),
        Some(serde_json::Value::Array(values)) => values.iter().collect(),
        Some(value) => vec![value],
    }
}

/// The `@id` of each node object a property holds.
fn json_ld_ids(doc: &serde_json::Value, key: &str) -> Vec<String> {
    json_ld_values(doc, key)
        .into_iter()
        .filter_map(|node| node.get("@id").and_then(|id| id.as_str()).map(str::to_string))
        .collect()
}

fn json_ld_names(doc: &serde_json::Value, key: &str) -> Vec<String> {
    json_ld_values(doc, key)
        .into_iter()
        .filter_map(|node| node.get("name").and_then(|name| name.as_str()).map(str::to_string))
        .collect()
}

fn json_ld_identifiers(doc: &serde_json::Value, key: &str, property_id: &str) -> Vec<String> {
    json_ld_values(doc, key)
        .into_iter()
        .flat_map(|node| json_ld_values(node, "identifier"))
        .filter(|id| id.get("propertyID").and_then(|p| p.as_str()) == Some(property_id))
        .filter_map(|id| id.get("value").and_then(|v| v.as_str()).map(str::to_string))
        .collect()
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
