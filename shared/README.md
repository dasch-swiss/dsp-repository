# Platform

Crates shared by more than one service. A crate lands here as soon as a second service depends on it, and is named `platform-{role}`; see `docs/src/repo_structure.md` → *Shared Crates* for the rule and why the directory, not just the crate name, is what matters.

```txt
platform/
├── metadata/          # Research-metadata wire contract (crate: platform-metadata)
└── telemetry/         # Browser beacon contract + collector endpoint (crate: platform-telemetry)
```

Platform crates depend on no service crate. Anything that needs to know about one service's routes, data or configuration takes it as a parameter instead of reaching for it.

Both halves of that rule are enforced, not just reviewed. A `dpe-*` or `editor-*` dependency in a platform crate's `Cargo.toml` is a Cargo cycle and fails to build. A hardcoded *path* into a service module compiles fine, so `just check` greps for one: see `.github/scripts/check-platform-paths.sh`, which fails on a `../`- or `modules/`-rooted path into any non-platform module under `modules/` appearing in a platform crate's `src/`. Prose cross-references are untouched: the crate names (`dpe-core`, `dpe_core::…`) carry no slash.

## `platform-metadata` (metadata/)

The research-metadata wire contract, shared by `dpe-*` and `editor-*`. Contains:

- **Contract types**: `ProjectRaw` and everything it is built from (`AccessRights`, `LegalInfo`, `License`, `Attribution`, `Publication`, `Grant`, `TemporalCoverage`, `Discipline`, `Funding`, …), plus `Person`, `Organization`, `Record` and `AuthorityFileReference`
- **Reading rules**: `is_placeholder` (`"MISSING"` / `"CALCULATED"`), `multilingual_value` (the deterministic lookup key), `is_valid_shortcode`, `is_role_job_title`
- **`Multilingual`**: the language-tag-to-text map every multilingual field uses. A `BTreeMap` with an open `String` tag, so serialization is alphabetical and deterministic (`ar` is live in two committed files, so a closed de/en/fr/it enum would drop content)
- **Temporal resolution**: `w3cdtf` formatting, the ChronOntology period table and the offline enrichment table, and `temporal_coverage::{resolve_in, completeness_gap}` — the single rule `dpe-server validate`, the OAI-PMH mapping and the editor all apply
- **Per-project checks**: `checks::{check_project, contributor_refs}` — the rules `dpe-server validate` applies to one project, reported as findings keyed by member and ordinal (`("temporalCoverage", Some(2))`) rather than by file, so the editor can render them per field. Whether a contributor id resolves stays with the caller: DPE's corpus is `persons/` plus `organizations/`, the editor's is the published corpus plus its own pending entity proposals

It was extracted from `dpe-core` rather than moved with it: `dpe-core` mixes the contract with DPE's caches, repositories, cluster logic and a DSP-API HTTP client, none of which is shared. What stayed behind is DPE's `Project` view model and both `From` impls (lossy on `url`'s original form and on `clusters`, which the editor must preserve), plus the process-global `OnceLock` caches — those read `DPE_DATA_DIR`, one service's configuration.

Table loading is therefore exposed as `load_from(data_dir)` and never reaches for a directory itself; `dpe-core` wraps it in the `OnceLock` and passes `get_data_dir()`. The same reason keeps the crate free of any relative path into `modules/dpe/server/data`: the test that validates the committed enrichment table lives in `dpe-core`, beside the data it reads.

Dependencies: `serde`, `serde_json`, `tracing`.

The workspace enables `serde_json`'s `preserve_order` feature, which the editor's canonical project writer requires: it round-trips `ProjectRaw` through `serde_json::Value` to strip `null` members, and `Value` alphabetises every key unless the feature is on. With it, output follows the struct's declaration order, which is what the 85 committed project files hold. Two consequences to know: `Map::remove` becomes swap-remove (use `retain` to drop members in place), and a `HashMap` field would serialize in its own random iteration order, which is why multilingual fields are `Multilingual` rather than `HashMap`.

`testdata/` holds two supported cross-crate test fixtures, not private ones: `0803-records.json` (a plain record) and `0862-records.json` (the one committed record carrying the full technical file metadata). `record.rs`'s own tests `include_str!` both; `dpe-api-oai`'s `get_record` and `test_utils` tests read `0803-records.json` by relative path. That direction (a service crate reading into platform) is the allowed one, and these are the single copy of each sample, so moving or renaming one breaks those call sites at compile time; update them in the same commit. They live here rather than under `modules/dpe/server/data/` because `record.rs` is what reads them, and a platform crate takes no path into a service module.

## `platform-telemetry` (telemetry/)

The browser telemetry contract and the endpoint that consumes it. A library crate so fuzz targets can test the real code, and so the beacon has one implementation across `dpe-server` and `editor-server` rather than a fork per service. Contains:

- **Beacon types**: `BeaconPayload`, `Signal`, `WebVitalSignal`, `ErrorSignal`, etc. (serde deserialization for browser beacons)
- **Origin validation**: `is_allowed_origin()` — validates dasch.swiss subdomains
- **Traceparent validation**: `is_valid_traceparent()` — W3C traceparent format validation
- **Collector endpoint**: `collector::collect_route(namespace, normalize_page_url)` — converts beacons to OTel metrics and structured logs. `namespace` sets the instrumentation scope (`dpe.browser`, `editor.browser`) and is a required argument so it cannot be omitted and silently rename the scope a dashboard filters on. `normalize_page_url` bounds the `page.url` metric attribute to a known set of routes

The contract modules depend on `serde` only; `collector` additionally pulls in `axum`, `opentelemetry`, `tracing` and `url`.

Page-URL normalization is **not** in this crate: a platform crate depends on no service crate, and a route table is exactly one service's data. `dpe-server` and `editor-server` each own a `page_url.rs` and pass its `normalize_page_url` fn into `collect_route`.
