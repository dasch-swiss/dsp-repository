---
title: "refactor: Extract the Access Area composition root areas/access/server"
type: refactor
date: 2026-09-28
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7397
linear_project: CPE establish production path
---

# refactor: Extract the Access Area composition root areas/access/server

## Overview

ADR-0003 composes each area into one binary at `areas/<area>/server`. ADR-0002 keeps that root inside the first capability
until a second one arrives, and ADR-0007 names CPE as the Access Area's second capability. This plan does the preparatory
half before CPE lands. It creates `areas/access/server` (crate `access-server`), which holds wiring only: CLI, config
loading, observability, the OTel layers, the untraced routes, mounting DPE's router. `dpe-server` becomes a library that
keeps what is DPE's.

ADR-0007's Consequences also require that DPE's process-global state become capability-owned, because a second capability
shares the process. That covers five config `OnceLock`s (`set_data_dir`, `set_public_dir`, `set_ark_resolver_base_url`,
`set_show_placeholder_values`, `dpe_api_oai::set_base_url`) and the ten `static` data caches in `dpe-core` that lazy-load
from them. All of them become values constructed by the composition root and passed in.

Visitors see no change: the same routes, bytes, env vars, image name and telemetry names.

The work builds on DEV-7396 (PR #444, merged as a8e3df90), which moved DPE to `areas/access/dpe`. It lands as three
commits, one per phase, with `allow-many-commits` ticked. This plan's own commit is squashed into the Phase 1 commit
when the branch is finalized.

ADR-0002 and ADR-0007 tie the move of the root to CPE's arrival ("when the second capability arrives"). This plan moves
it first, so that CPE lands into a finished root instead of doing the split and its own arrival in one change. The
reasons both ADRs give (the root is wiring only, and a second capability shares the process) are unchanged. Only the
timing moves, and Phase 3 records that as an amendment to both ADRs.

## Problem Statement / Motivation

- CPE (DEV-7400) and the public URL layout (DEV-7405) are blocked on this ticket. CPE needs a composition root to be mounted
  into, and it cannot share a process with DPE's `OnceLock`s. These are first-call-wins and process-wide: a second
  capability or a second test corpus in the same binary silently gets DPE's values.
- The test suite already pays for this. `areas/access/dpe/server/src/test_support.rs:12-21` explains that every test must
  set the same data dir before touching any cache, or the empty corpus stays pinned for the whole test binary. With the
  editor (`areas/deposit/editor/server/src/shell.rs:5-30`), everything is a field of `AppState`, and it has no such rule.
- `AppState.oai_base_url` and `dpe_api_oai::BASE_URL` hold the same value twice, and `shell.rs:13-16` warns that they must
  never diverge. Injecting one value removes the duplicate.

## Proposed Solution

Three phases, each a buildable, green commit:

1. **View and OAI settings become arguments.** `dpe_api_oai::set_base_url` and `set_show_placeholder_values` go away. The
   OAI base URL arrives in the OAI handler's state. The placeholder flag reaches `dpe-web` components through a small
   render context.
2. **The corpus becomes a value.** A `dpe_core::Corpus` owns every data cache today held in a `static`. It is built by
   `Corpus::new(CorpusSettings { data_dir, public_dir, ark_resolver_base_url })`. `set_data_dir`, `set_public_dir`,
   `set_ark_resolver_base_url` and every `static …: OnceLock` in `dpe-core` go away. Handlers and views receive the
   corpus through state and arguments.
3. **Extract the composition root.** The new crate `areas/access/server` (`access-server`, binary `access-server`) takes
   over `main`, `cli`, `serve`, `observability`, the OTel layers and the untraced routes. `dpe-server` becomes a library
   exposing its config, a constructor for DPE's router and `validate`. The Dockerfile moves beside the root (ADR-0002),
   the image stays `daschswiss/dpe`, and the image also carries `/app/dpe-server` as an alias, because ops-deploy's
   healthcheck calls that path.

### The capability's public surface after Phase 3

`access-server` depends on `dpe-server` only, never on `dpe-core`, `dpe-web` or `dpe-api-oai`:

```rust
// dpe-server (lib) — what the composition root may name
pub use config::DpeConfig;                       // figment loader, DPE_* env, unchanged

#[derive(Clone)]
pub struct Dpe { /* &'static Corpus, AppState, public_dir, … — private, all cheap to clone */ }

impl Dpe {
    /// Builds the corpus and DPE's state from its config. The one production `Box::leak` of a
    /// `Corpus` lives here, so the composition root never names `Corpus`. No corpus I/O
    /// (it resolves the hashed CSS href from `public_dir`, as `serve.rs:59` does today);
    /// `warm()` loads the record cache.
    pub fn new(config: &DpeConfig) -> Self;
    /// DPE's routes (pages, fragments, JSON, /dpe/oai with its limiter, /ark:/ when
    /// configured, the ServeDir fallback) — un-layered; the root applies OTel.
    pub fn router(&self) -> axum::Router;
    /// Blocking: loads the records up front, as `record_cache::warm` does today.
    pub fn warm(&self);
}

pub fn validate(data_dir: std::path::PathBuf) -> std::process::ExitCode;
pub fn normalize_page_url(path: &str) -> &'static str;   // for the telemetry mount
```

```rust
// access-server serve() — the shape, not the final code
let dpe_config = dpe_server::DpeConfig::load().expect("failed to load DPE configuration");
let dpe = dpe_server::Dpe::new(&dpe_config);
tokio::task::spawn_blocking({ let dpe = dpe.clone(); move || dpe.warm() });

let app = Router::new()
    .merge(dpe.router())
    // --- traced above, untraced below: routes declared after .layer() are not wrapped ---
    .layer(OtelInResponseLayer)
    .layer(OtelAxumLayer::default())
    .route("/healthz", get(|| async { StatusCode::OK }))
    .route("/telemetry/collect", collect_route("dpe", dpe_server::normalize_page_url).layer(governor));
```

### `Corpus` shape (Phase 2)

- One struct in `dpe-core`, with one **instance-owned** `OnceLock` field per cache: projects plus shortcode index, raw
  projects, records plus shortcode index, persons, organizations, clusters, ChronOntology periods, temporal enrichment,
  covers. Each loads lazily on first access, as today, so startup and memory behaviour stay the same, and `warm()` stays
  the one eager load.
- The existing free functions become methods with the same names (`corpus.project_by_shortcode(..)`,
  `corpus.records_for_shortcode(..)`, …), so every call site changes mechanically.
- **Lifetime choice:** `dpe-server` leaks one corpus (`serve.rs` in Phase 2, `Dpe::new` from Phase 3 on) (`Box::leak`) and holds a `&'static Corpus`. That is the only
  production leak, and it is stated in `Corpus`'s doc comment. Accessors that return borrowed data take
  **`&'static self`**, not `&self`, because a field borrowed through `self` is `'static` only if `self` is. Written
  that way, the existing `&'static` return types (`&'static Project`, `&'static [&'static Record]`) keep compiling, and
  the record index (`OnceLock<HashMap<String, Vec<&'static Record>>>`, built from `self.all_records()`) needs no
  rewrite. `Corpus` derives neither `Clone` nor `Copy`: a clone would start with empty `OnceLock`s and load everything a
  second time.
  This does not bring back a global: nothing names the corpus except whoever was handed it. Tests leak one per fixture
  directory, which today's statics make impossible. axum's docs show `Arc` state, not `Box::leak`, for app-lifetime
  data. The leak is chosen for diff size, not idiom; see Alternative Approaches.
- `FsProjectRepository` / `FsRecordRepository` take `&'static Corpus` in `new()`, and their `Default` impls
  (`project_repository.rs:16-20`, `record_repository.rs:30-34`) are deleted. No caller uses `::default()`.
  `CachedContributorLookup` changes from a unit struct to `CachedContributorLookup { corpus: &'static Corpus }`. It is
  the one `ContributorLookup` impl, and a trait method's `&self` reaches the corpus through that field.
  `resolve_inputs()` (`core/src/lib.rs:54-65`) becomes a `Corpus` method returning the lookup **by value**, since its
  `static LOOKUP` cannot hold a corpus. Callers (`api-oai/src/metadata/mod.rs:85`, `server/src/metadata.rs:196`) pass
  `&lookup`.
- Pure helpers stay free functions: `record_cache::index_by_shortcode`, `cluster_cache::projects_for_cluster_in`,
  `clusters_for_shortcode_in` and `project_cache::load_projects_from`. Only functions that read a `static` become
  methods.
- The count is ten `static` `OnceLock`s in nine cache modules, because `project_cache` and `record_cache` each also hold
  a `SHORTCODE_INDEX`. `Corpus` gets one field for each of the ten.
- `get_data_dir()`'s env fallbacks (`DPE_DATA_DIR` → `DATA_DIR` → dev default) are removed with it. `DpeConfig` becomes
  the one source. It already reads `DPE_*`, keeps its own legacy `DATA_DIR` alias
  (`server/src/config.rs:106-108`, documented in `docs/src/dpe/operations.md:61`), and defaults to the same paths. So no
  deployment's configuration changes and `operations.md` stays accurate.

### Render context (Phase 1, extended in Phase 2)

`dpe-web` gains `pub struct RenderContext { pub show_placeholder_values: bool }`, and Phase 2 adds
`pub corpus: &'static Corpus`. It has no lifetime parameter, because the corpus accessors need `&'static self`, so a
shorter lifetime could never call them. `dpe-server` builds it from `AppState` and passes it to page functions by
reference. Components that read a cache or the placeholder flag take `&RenderContext`. Pure components keep their
signature. Introducing the struct in Phase 1 means each signature changes once. Phase 2 only adds a field, and does not
turn a bare `bool` into a struct at the same ~20 sites. While touching these signatures, never pass a nested
`html! { … }` as an argument: bind it with `let` first (`CONVENTIONS.md`).

### OAI state (Phase 1, extended in Phase 2)

`dpe-api-oai` gains `pub struct OaiState { pub base_url: String }`, and Phase 2 adds `pub corpus: &'static Corpus`.
`oai_handler` extracts `State<OaiState>`. `OaiState::new(configured)` applies today's `resolve_url` exactly once. That is
the empty-to-default and trailing-slash normalisation `set_base_url` does now (`api-oai/src/lib.rs:25-40`), so the
emitted `baseURL` stays byte-identical, and `AppState.oai_base_url` takes the same normalised value.

`rate_limited_router_with` (`router.rs:58-82`) puts `/dpe/oai` and the two representation routes under one
`route_layer`, so they share one per-IP bucket. The representation handlers take `State<AppState>`, so the OAI route
gets its state on its own sub-router, and the shared limiter stays on the outside:

```rust
Router::new()
    .merge(Router::new().route("/dpe/oai", get(dpe_api_oai::oai_handler)).with_state(oai_state))
    .route("/dpe/projects/{id}/metadata.jsonld", get(crate::metadata::project_json_ld_handler))
    .route("/dpe/projects/{id}/metadata.datacite.json", get(crate::metadata::project_datacite_json_handler))
    .route_layer(limiter)
```

`with_state` returns `Router<S2>` for any `S2`, so it is inferred as `AppState` here. No `FromRef` impl is needed, and
`dpe-api-oai` never sees `AppState`, which stays `pub(crate)`. `rate_limited_router_with` gains an `OaiState`
parameter, so its test call sites and `router.rs:331` change with it. The ARCH-MAP convention "DPE's
views self-load from `dpe-core` caches — a recorded exception" narrows to "DPE's views read the corpus handed to them" and
is updated in Phase 3.

## Alternative Approaches Considered

- **Remove only the five config setters and keep the static caches behind an explicit `Corpus::init(&settings)`.**
  Rejected (decided 2026-09-28): the init is a process-global setter under another name, and it keeps the first-call-wins
  test hazard.
- **`Arc<Corpus>` with index-based record lookups instead of a leaked `&'static Corpus`.** It frees the corpus on drop,
  which no production path does: the process holds it until exit, as today. It would also turn every `&'static` return
  into a borrow of the state and rewrite the record index as `Vec<usize>`, roughly doubling Phase 2's diff for no runtime
  difference. If `sync` later needs to swap the corpus at run time, this is the way to do it, and it is a contained change
  then.
- **Binary name.** Decided 2026-09-28: rename the binary to `access-server`, keep the image `daschswiss/dpe`, ship a
  `/app/dpe-server` copy until ops-deploy switches (H1). Rejected: renaming with no alias, which forces ops-deploy to move
  in lockstep with the release, and keeping the binary named `dpe-server`, a capability's name on a two-capability area.

## Technical Considerations

- **Telemetry names must not move with the code.** `observability.rs` uses `env!("CARGO_PKG_NAME")` as the OTel tracer
  name (`observability.rs:90`) and the Pyroscope application name (`observability.rs:116`). Moved into `access-server`
  unchanged, the profiles would silently change application in Grafana. Phase 3 pins both to the literal `"dpe-server"`
  behind one named constant, with a comment saying why. The Pyroscope tag `service.namespace = "dpe"`
  (`observability.rs:122`) and the collector scope `"dpe"` are literals already. They stay, and each gets a comment
  saying the value is DPE's telemetry identity, not the crate's. `service.name` is not code-derived: deploys set
  `OTEL_SERVICE_NAME=dpe` (`ops-deploy/repository.yml:55-56`). Renaming any of them is a dashboard decision, out of
  scope. (Pyroscope's `PYROSCOPE_ENDPOINT` is commented out in prod today, so the Pyroscope half of this risk is
  currently local and dev only.)
- **Log targets move.** Events from `serve`/`observability` now carry target `access_server::…`. `RUST_LOG=info`
  (Dockerfile) is unaffected. The `dpe_server=info,…` examples in `docs/src/dpe/operations.md:60,185` are updated to
  include `access_server`.
- **Traced/untraced order is load-bearing.** Today `build_router` applies the OTel layers and `serve.rs` adds `/healthz`
  and `/telemetry/collect` after them. Phase 3 moves the layers to the root and keeps the order. The `dev` feature's
  live-reload layer (`dev_reload.rs`) is DPE's, because it watches DPE's public dir. It stays inside `Dpe::router()` and
  ends up inside the OTel layers, where today it wraps them. This is dev-only and changes no production path.
- **Env vars are unchanged.** `DPE_SITE_ADDR`, `DPE_ENV`, `DPE_*` config and `OTEL_*`/`PYROSCOPE_*` keep their names, so
  neither ops-deploy nor the Cloud Run preview changes. An `ACCESS_*` rename belongs with CPE's arrival, if anywhere.
- **Out of scope, named so it is not mistaken for done:**
  - `shared-telemetry`'s own process-globals (`collector.rs:28,49`, `METER_SCOPE`, `PAGE_URL_NORMALIZER`). They are set
    once by the one `collect_route` mount. With CPE, one endpoint needs either two normalizers or a composed one. That
    belongs to DEV-7400, which introduces the second caller.
  - DPE's `ServeDir` fallback: axum refuses to merge two routers that both have a fallback, which CPE's arrival will hit.
    DEV-7400 or DEV-7405 decides how static assets are split.
- **Crate scopes.** Commit scopes are free-form (`.commitlintrc.yml`: presence only). `access-server` is added to the scope
  vocabulary lists in `docs/src/git-conventions.md:87` and `CONVENTIONS.md:54`.
- **Gates.** `check-shared-paths.sh` buckets by the area segment (`non_shared_dirs`, `awk … print $2`), so
  `areas/access/server` falls under `access` like DPE does and cannot be misclassified. Running it in Phase 3 is a plain
  regression check.
  `eng.yaml`'s `areas/*/*/server/src/**` override (the `ivan-reviewer` binding) is one segment too deep for
  `areas/access/server/src/**`, so Phase 3 adds `areas/*/server/src/**`.
- **The boundary gets a mechanical check.** ADR-0007's *Enforced by* expects a `Cargo.toml` grep in the style of
  `check-shared-paths.sh` until Bazel `visibility` lands. Phase 3 adds `.github/scripts/check-composition-root-deps.sh`
  with this rule: *among the crates under `areas/<area>/`, `areas/<area>/server/Cargo.toml` depends only on each
  capability's `server` crate, never on a capability's `core`, `web` or `api-*` crate; `shared-*` and third-party crates
  are unrestricted.* It runs in `just check`, with a `.test.sh` in the style of
  `check-shared-paths.test.sh`.
- **Learnings applied.** From the DPE-move journal
  (`docs/specs/2026-09-28-move-dpe-to-access-area/01-refactor-move-dpe-to-access-area-plan-journal.md`): verify relative
  paths by building, not by inspection; grep for prose that names the binary without the literal string; run
  `cargo metadata` for the fuzz workspace. From `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`:
  accept snapshots with `cargo insta accept`/`review` only. No snapshot content should change in any phase. A changed
  snapshot is a regression, not something to accept.

## Implementation Phases

Conventions for every phase: tests first (`CLAUDE.md`), then the change. `areas/access/dpe/CLAUDE.md` and `CONVENTIONS.md`
apply. Reviewers for each phase review: `eng:review:rust-reviewer`, `eng:review:code-simplicity-reviewer`,
`eng:review:consistency-reviewer`, `eng:review:ivan-reviewer`, `eng:review:dune-reviewer`. Phase 3 adds
`eng:review:devops-reviewer` and `eng:review:observability-reviewer`.

#### Phase 1: OAI base URL and placeholder flag become arguments

Commit: `refactor(dpe-api-oai,dpe-web,dpe-server): pass the OAI base URL and placeholder flag instead of setting globals`

- [x] Test (`dpe-api-oai`): a handler test that builds the OAI router with base URL `https://a.example/oai` and one with `https://b.example/oai` in the same test binary; each response's `<request>` and `baseURL` carry its own value
- [x] Test (`dpe-web`): `placeholder_value("MISSING")` renders the red span under `RenderContext { show_placeholder_values: true }` and nothing under `false`, both in one test (replacing the env-dependent `placeholder_value_markup_is_empty_when_hidden`)
- [x] Test (`dpe-web`): `should_render_value` returns true for a placeholder only when the context says to show placeholders
- [x] `dpe-api-oai`: remove `BASE_URL`, `set_base_url` and `base_url()` (`api-oai/src/lib.rs:21-47`) and the `DPE_OAI_BASE_URL` env fallback; add `OaiState { base_url }`; `oai_handler` (`api-oai/src/handlers/mod.rs:59`) extracts `State<OaiState>`, and the XML writers (`xml.rs`, `metadata.rs`) take the base URL as a parameter
- [x] `dpe-api-oai`: `OaiState::new` applies `resolve_url` once; move `resolve_url`'s unit tests onto it; rewrite the identify test that calls `crate::base_url()` (`api-oai/src/handlers/identify.rs:93`) to use a state
- [x] `dpe-server`: `rate_limited_router_with` (`router.rs:58-82`) takes an `OaiState`, merges `/dpe/oai` as its own `.with_state(oai_state)` sub-router, and keeps the two representation routes and the shared `route_layer(limiter)` as they are (shape in Proposed Solution); its `Router<AppState>` return type is unchanged; update its test call sites and `router.rs:331`
- [x] `dpe-web`: add `RenderContext { show_placeholder_values }`; `placeholder_value` and `should_render_value` take it; thread it through each caller (`git grep -n placeholder_value -- areas/access/dpe/web/src`, ~20 sites) and the page functions above them
- [x] `dpe-core`: remove `SHOW_PLACEHOLDER_VALUES`, `set_show_placeholder_values` and `show_placeholder_values` (`core/src/utils.rs:52,101-123`) and their re-exports
- [x] `dpe-server`: `AppState` carries `show_placeholder_values`; handlers build the `RenderContext`; `serve.rs` stops calling the two removed setters; `AppState.oai_base_url` becomes the single copy and the "must never be set apart" comment in `shell.rs` goes
- [x] `dpe-server` tests: `test_support::test_state()` sets `show_placeholder_values: false` (the production default); the fragments tests, which today call `set_show_placeholder_values(true)` (`fragments.rs:296`), build their state with `true`, so `dpe_server__fragments__tests__project_sidebar_real_project.snap` keeps its `CALCULATED` span; `fragments.rs`'s stateless `test_app()` gains that state
- [x] `git grep -nE "set_base_url|set_show_placeholder_values|show_placeholder_values\(\)" -- areas/access` returns nothing
- [x] Run `just check && just test`; both pass, and nothing under `**/snapshots/` or `api-oai/src/oai/handlers/testdata/golden/` changes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: the corpus becomes a capability-owned value

Commit: `refactor(dpe-core,dpe-api-oai,dpe-web,dpe-server): own the corpus in a Corpus value instead of static caches`

- [x] Test (`dpe-core`): two `Corpus` values over two temporary data dirs, holding different `projects.json` content, both leaked in one test; each serves its own projects (this fails against today's statics)
- [x] Test (`dpe-core`): a corpus built with `ark_resolver_base_url: Some(host)` normalises project and record ARKs to that host, and one built with `None` leaves them as committed (moving the existing `ark.rs` tests off the setter)
- [x] Test (`dpe-core`): `cover_image_url` answers from the corpus's `public_dir`, not from the process's working directory
- [x] `dpe-core`: add `Corpus` (no `Clone`/`Copy` derive; doc comment says production leaks exactly one, in `dpe-server`) and `CorpusSettings { data_dir, public_dir, ark_resolver_base_url }`, with one instance-owned `OnceLock` per cache, loading lazily as today
- [x] `dpe-core`: move each cache's loader and accessors into `Corpus` methods (`project_cache`, `record_cache`, `person_cache`, `organization_cache`, `cluster_cache`, `chronontology_cache`, `temporal_enrichment_cache`, `cover_image_cache`; raw projects and both shortcode indices stay behind `project_cache`'s and `record_cache`'s existing accessors, as today in `project_cache.rs:19-20` and `record_cache.rs:18-19`), keeping names and return types; accessors returning borrowed data take `&'static self`; loaders read paths from `CorpusSettings`
- [x] `dpe-core`: `Corpus::warm(&'static self)` replaces `record_cache::warm` (it builds the record index, so it needs `&'static self`)
- [x] `dpe-core`: `FsProjectRepository::new` and `FsRecordRepository::new` take `&'static Corpus`; delete their `Default` impls
- [x] `dpe-core`: `CachedContributorLookup { corpus: &'static Corpus }` replaces the unit struct (literal construction at `api-oai/src/handlers/mod.rs:63` changes with it)
- [x] `dpe-core`: `resolve_inputs` becomes a `Corpus` method returning the lookup by value; its `static LOOKUP` goes; callers (`api-oai/src/metadata/mod.rs:85`, `server/src/metadata.rs:196`) pass `&lookup`
- [x] `dpe-core`: `index_by_shortcode`, `projects_for_cluster_in`, `clusters_for_shortcode_in` and `load_projects_from` stay free functions
- [x] `dpe-api-oai` golden files: the handler tests keep producing today's bytes. First establish which data dir they read today (`to_oai_record` → `resolve_inputs()` resolves the relative default from the crate's cwd, possibly to no tables at all), then build the test corpus over that same input, so `api-oai/src/oai/handlers/testdata/golden/*.xml` stay unchanged
- [x] `dpe-core`: remove `DATA_DIR`, `PUBLIC_DIR`, `ARK_RESOLVER_BASE_URL`, `set_data_dir`, `get_data_dir`, `set_public_dir`, `get_public_dir`, `set_ark_resolver_base_url`, `ark_resolver_base_url()` and their re-exports; `normalise_project`/`normalise_record` keep taking the host as an argument
- [x] `dpe-api-oai`: `OaiState` gains `corpus: &'static Corpus`; handlers and `metadata/corpus.rs` take the corpus from it
- [x] `dpe-api-oai`: `handle_list_identifiers_paged` / `handle_list_records_paged` take `&OaiState` in place of the separate repo, record-repo, clusters, lookup and base-URL arguments, and their `#[allow(clippy::too_many_arguments)]` (added in Phase 1) goes (Phase 1 review)
- [x] `dpe-web`: `RenderContext` gains `corpus: &'static Corpus`; the self-loading components (`domain/projects.rs`, `domain/contributors.rs`, `components/person.rs`, `organization_name.rs`, `project_header.rs`, `project_sidebar/legal_info.rs`, `projects/components/card.rs`, `project_list.rs`) read it from there; `domain/projects.rs:176` uses the corpus's data dir instead of `get_data_dir()`
- [x] `dpe-server`: `AppState` holds `corpus: &'static Corpus`; `serve.rs` builds it once from `DpeConfig` with `Box::leak`, replaces the three setter calls, and warms with `spawn_blocking(move || corpus.warm())`
- [x] `dpe-server`: `ark.rs`, `downloads.rs`, `fragments.rs`, `metadata.rs`, `router.rs`, `shell.rs` read the corpus from state
- [x] `dpe-server` tests: `test_support` leaks one `Corpus` over the committed data dir (behind a test-only `OnceLock`, as `areas/deposit/editor/server/src/test_support.rs:213` does), and its doc comment about first-call-wins ordering is removed; router and ark tests that called setters build state instead
- [x] Fuzz workspace: `cargo metadata --manifest-path areas/access/dpe/server/fuzz/Cargo.toml` resolves and `cargo check` in that workspace passes (it depends on `dpe-core`)
- [x] `git grep -nE "static [A-Z_]+: *(std::sync::)?(OnceLock|LazyLock)" -- areas/access` matches only `#[cfg(test)]` code
- [x] `git grep -nE "set_data_dir|set_public_dir|set_ark_resolver_base_url|get_data_dir|get_public_dir" -- areas/access docs/src` returns nothing
- [x] `docs/src/dpe/project_structure.md:82`: replace the paragraph on process-global `OnceLock`s with the `Corpus` handed in through state
- [x] `just validate-data` passes
- [x] Run `just check && just test`; both pass, and nothing under `**/snapshots/` or `api-oai/src/oai/handlers/testdata/golden/` changes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: extract areas/access/server

Commit: `refactor(access-server,dpe-server): extract the Access Area composition root areas/access/server`

The code:

- [x] Test (`access-server`, new; no such test exists today): on the router `access-server` assembles, `/healthz` and `POST /telemetry/collect` get no `traceparent` response header and a DPE page gets one. To make this testable, `serve()` delegates router assembly to a function taking `&Dpe`
- [x] Test (`access-server`): port `cli.rs`'s `is_allowed_healthcheck_url` tests unchanged
- [x] Test (`access-server`): the assembled router serves `/dpe/projects`, `/dpe/projects/0803`, `/dpe/oai?verb=Identify` and a static asset with status 200
- [x] Create `areas/access/server/` with `Cargo.toml` (package and binary `access-server`, `version.workspace = true`) and add it to the root workspace `members`
- [x] Move `main.rs`, `cli.rs`, `serve.rs` and `observability.rs` from `areas/access/dpe/server/src/` to `areas/access/server/src/` with `git mv`, so history follows
- [x] `access-server`: `serve()` loads `DpeConfig`, builds `Dpe`, warms it, merges `dpe.router()`, applies the OTel layers, then mounts `/healthz` and `/telemetry/collect` (with its governor layer and `dpe_server::normalize_page_url`)
- [x] `access-server`: `main` dispatches `serve`, `validate <data_dir>` (to `dpe_server::validate`) and `healthcheck`
- [x] `access-server`: pin the OTel tracer name and the Pyroscope application name to the literal `"dpe-server"` in one constant, with a comment stating why it is not `CARGO_PKG_NAME`
- [x] `access-server`: comment the `service.namespace = "dpe"` Pyroscope tag and the `collect_route("dpe", …)` scope as DPE's telemetry identity, not the crate's
- [x] `access-server` `Cargo.toml` depends on `dpe-server`, `shared-telemetry` and the infra crates only, not on `dpe-core`, `dpe-web` or `dpe-api-oai`
- [x] Move the OTel/Pyroscope/clap/tokio-signal dependencies that only the moved files use from `dpe-server`'s `Cargo.toml` to `access-server`'s
- [x] `dpe-server`: add `src/lib.rs` with the surface above (`DpeConfig`, `Dpe::{new, router, warm}`, `validate`, `normalize_page_url`, `RightmostXffKeyExtractor`) and delete the binary target
- [x] `dpe-server`: the `Box::leak` of the corpus moves from `serve.rs` into `Dpe::new`, and remains the only production leak
- [x] `dpe-server`: no public item of `lib.rs` names a `dpe_core`, `dpe_web` or `dpe_api_oai` type in its signature or re-exports one (`git grep -nE "dpe_(core|web|api_oai)" -- areas/access/server/src` returns nothing, and `lib.rs`'s `pub` items are checked by hand)
- [x] `dpe-server`: `build_router` no longer applies the OTel layers; `RightmostXffKeyExtractor` (`router.rs`, today `pub(crate)`) becomes `pub`, because the telemetry mount's governor moves to `access-server` and uses it
- [x] `access-server`: the root `Router::new()` sets no fallback, since `dpe.router()` carries DPE's `ServeDir` fallback and axum 0.8.9 panics on merging two
- [x] `dpe-server`: its `dev` feature is re-exported by `access-server` (`dev = ["dpe-server/dev"]`) so `bacon` and `just dev` keep live-reload

Build, deploy and CI:

- [x] `git mv areas/access/dpe/Dockerfile areas/access/server/Dockerfile`; it copies `access-server`, keeps `ENTRYPOINT ["./access-server"]` and `HEALTHCHECK … ./access-server healthcheck`, and also copies the same binary to `/app/dpe-server`, with a comment naming H1 as the condition for removing it (distroless has no shell, so a second `COPY`, not a symlink)
- [x] `.github/actions/build-dpe/action.yml`: build `-p access-server`, stage `access-server`, copy the Dockerfile from `areas/access/server/`
- [x] `.github/workflows/a11y-dpe.yml:50-56`: the build step runs `cargo build -p access-server --release`, and its comment names `target/release/access-server`
- [x] `.github/workflows/a11y-dpe.yml`, `cloud-run-dpe-pull-request.yml`, `scout-dpe.yml`: add `areas/access/server/**` to every `paths:` filter that lists `areas/access/dpe/**` (`dpe-docker-publish.yml` and `dpe-release-publish.yml` have no `paths:` filter; they change only through `build-dpe`)
- [x] `justfile` `build-docker-dpe` (`justfile:528`): `-f areas/access/server/Dockerfile`. The Dockerfile is packaging-only and expects staged artifacts, so this recipe does not build from the tree; that predates this plan and is left as is
- [x] `justfile`: `run` and `validate-data` use `--bin access-server`; `dev` and the comments that name `dpe-server` as the binary follow. The `test` recipe's `-p dpe-server --features dpe-server/dev` (`justfile:160`) stays, since it tests `dev_reload.rs`, which remains in the library; `access-server`'s tests run under the workspace `cargo test`
- [x] `bacon.toml`: `--bin access-server --features dev`
- [x] `areas/access/dpe/web-e2e-tests/playwright.config.ts`: `target/release/access-server` and the build hint
- [x] `eng.yaml`: bind `areas/access/server/**` to `areas/access/server/CLAUDE.md` and add `eng:review:devops-reviewer` for its Dockerfile
- [x] `eng.yaml`: add an `areas/*/server/src/**` override with `eng:review:ivan-reviewer`, beside the existing `areas/*/*/server/src/**`
- [x] Test first: `.github/scripts/check-composition-root-deps.test.sh` builds a throwaway repo in which `areas/access/server/Cargo.toml` depends on `dpe-core` and expects the gate to fail, and a variant that depends only on `dpe-server` and `shared-telemetry` and expects it to pass
- [x] `.github/scripts/check-composition-root-deps.sh` enforces the rule from Technical Considerations verbatim: among the crates under `areas/<area>/`, `areas/<area>/server/Cargo.toml` depends only on each capability's `server` crate, never on a capability's `core`, `web` or `api-*` crate; `shared-*` and third-party crates are unrestricted
- [x] `justfile`: add a `check-composition-root-deps` recipe to `check`'s dependencies, and run the `.test.sh` in `test` beside `check-shared-paths.test.sh`
- [x] `.github/dependabot.yml`: add `areas/access/server` wherever the DPE Cargo directory is listed
- [x] Run `just check-shared-paths`; it passes

Docs and records:

- [x] `areas/access/server/CLAUDE.md`: what the root may hold (ADR-0003's wiring list), the traced/untraced order rule, "no adapter logic", and "imports only `dpe-server`'s public surface"
- [x] `areas/access/dpe/CLAUDE.md`: the "Routing, head, and the page shell" section no longer calls `dpe-server` the composition root. It states that `dpe-server` is a library since this change, exposing DPE's router and config, with no binary, that the binary is `access-server`, and it points to `areas/access/server/CLAUDE.md`
- [x] `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`: add a dated amendment (2026-09-28, DEV-7397) recording that the composition root was extracted before CPE's arrival, to unblock DEV-7400/DEV-7405, with the decision's reasons unchanged
- [x] `docs/adr/0002-areas-at-the-repository-root.md`: add a one-line dated amendment after line 24 that the Access Area's root has moved up to `areas/access/server` ahead of the second capability, pointing to ADR-0007's amendment
- [x] Run `just check-adr-refs`; it passes
- [x] `areas/access/dpe/README.md`: layout, build and binary paths
- [x] `docs/src/dpe/operations.md`: binary `access-server` (and the `/app/dpe-server` alias), subcommands, `RUST_LOG` examples naming `access_server`
- [x] `docs/src/deployment.md`, `docs/src/dpe/architecture.md`, `docs/src/dpe/project_structure.md`, `docs/src/dpe/testing-strategy.md`, `docs/src/dpe/observability.md` (Pyroscope application stays `dpe-server`), `docs/src/repo_structure.md`: the new crate and the binary name
- [x] `docs/src/git-conventions.md:87` and `CONVENTIONS.md:54`: add `access-server` to the crate list
- [x] `ARCH-MAP.md`: run `dune:dune-map` to add the `areas/access/server` component, update the `areas/access/dpe` entry (public interface, local-context kit, `serve.rs` reference), reword the "Composition root" convention (line ~554: an area's `server` crate owns CLI, observability and mounting; each capability owns its routes and config; enforced by `check-composition-root-deps.sh`, static-analysis), and narrow the "views self-load" exception (line ~556)
- [x] `git grep -nE "\bdpe-server\b" -- ':!docs/specs/20*' ':!CHANGELOG.md' ':!*.snap' ':!Cargo.lock' ':!docs/adr'`: every remaining hit names the library crate, the telemetry constant or the image alias, never the binary
- [x] `cargo build -p access-server --release`, then `just css-release`, then `just test-a11y-dpe` against it; the suite passes
- [x] `just run` with no env overrides: `/dpe/projects`, `/dpe/projects/0803`, `/dpe/oai?verb=Identify` and `/healthz` return 200, and `access-server healthcheck` exits 0
- [x] `just build-docker-dpe`; `docker run` the image and `docker exec` both `/app/access-server healthcheck` and `/app/dpe-server healthcheck`; both exit 0
- [x] Run `just check && just test`; both pass, and nothing under `**/snapshots/` or `api-oai/src/oai/handlers/testdata/golden/` changes
- [x] Run `just commit-lint`; it passes with `allow-many-commits` in mind (three commits)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Switch ops-deploy's DPE healthcheck (`repository.yml:43`, `roles/deploy/README.md`, `defaults/main.yml`) from `/app/dpe-server` to `/app/access-server`, then open the follow-up that drops the alias `COPY` from `areas/access/server/Dockerfile` | Balduin | after ship | Deploy config in another repository, released on its own schedule |
| H2 | Open the PR for this branch after the phases are committed, with `allow-many-commits` ticked | Balduin | after ship | The owner opens PRs in this workflow |
| H3 | After the first deploy, spot-check in Grafana that a DPE span's instrumentation scope is still `dpe-server` and `service.name` is still `dpe` | Balduin | after ship | Needs a deployed build exporting to the production stack |

## Acceptance Criteria

- [x] One binary, `access-server`, serves every DPE route unchanged: handler tests, `just test` and the `test-a11y-dpe` E2E suite pass
- [x] `areas/access/server`'s `Cargo.toml` names no `dpe-core`, `dpe-web` or `dpe-api-oai` dependency, and its source names no type of theirs; `check-composition-root-deps.sh` enforces the first half in `just check`
- [x] ADR-0007 and ADR-0002 carry dated amendments recording that the root moved ahead of CPE
- [x] No process-global setter remains in `areas/access/**`: no `set_*` for data dir, public dir, ARK host, placeholder flag or OAI base URL, and no `static` `OnceLock`/`LazyLock` outside `#[cfg(test)]` code
- [x] Two `Corpus` values over different directories coexist in one test binary
- [x] Rendered HTML, JSON and OAI XML are byte-identical to before: no change under `**/snapshots/*.snap` or `api-oai/src/oai/handlers/testdata/golden/` across the three commits
- [x] The Docker image `daschswiss/dpe` answers `healthcheck` at both `/app/access-server` and `/app/dpe-server`
- [x] Pyroscope application name, OTel tracer name, `service.namespace` and the browser-metrics scope are unchanged (`dpe-server`, `dpe-server`, `dpe`, `dpe`)
- [x] `just check`, `just test` and `just commit-lint` pass on the branch

## Dependencies & Risks

- **Phase 2 is the widest diff** (about 100 call sites in `dpe-web`, `dpe-server`, `dpe-api-oai`). It is mechanical: every former free function keeps its name as a method. The snapshot suite is the check that nothing rendered changed.
- **Lazy-load behaviour.** If an accessor is accidentally made eager in `Corpus::new`, the Cloud Run start (HEALTHCHECK `--start-period=5s`) waits for the record load. The `just run` step and the Docker healthcheck step catch it.
- **ops-deploy healthcheck.** Covered by the alias until H1. Removing the alias before H1 fails the healthcheck on the next deploy.
- **Follow-ups this plan deliberately leaves** (see Technical Considerations): `shared-telemetry`'s process-globals, the `ServeDir` fallback that cannot merge with a second router, and `ACCESS_*` env names. All three become necessary with DEV-7400.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Telemetry identity changes silently (Pyroscope app, tracer name) | M | M | Pinned constant in Phase 3; acceptance criterion; observability reviewer |
| A CI path filter misses `areas/access/server/**`, so DPE jobs stop triggering on root changes | M | M | Explicit deliverable per workflow; devops reviewer; the PR itself touches the root, so jobs must trigger on it |
| Placeholder flag threading misses a component, and placeholders show or hide wrongly | L | L | Phase 1 tests on the component; the flag can no longer be read without a context, so a miss does not compile |
| Leaked corpus memory in tests grows with fixture count | L | L | One leak per fixture dir, behind a test-only `OnceLock` |

## Success Metrics

- Baseline, taken before Phase 1 on this branch: `just test` passes, and the insta snapshot set under
  `areas/access/dpe/**/snapshots/` and the OAI golden files under
  `areas/access/dpe/api-oai/src/oai/handlers/testdata/golden/` are unchanged after each phase (`git status --porcelain`
  lists neither).
- After Phase 3: `git grep -nE "static [A-Z_]+: *(std::sync::)?(OnceLock|LazyLock)" -- areas/access` matches only test code, and
  `cargo tree -p access-server --depth 1` lists `dpe-server` as its only `dpe-*` dependency.

## References

- Ticket: DEV-7397. Builds on DEV-7396 (PR #444, merged). Blocks DEV-7400 and DEV-7405.
- axum 0.8.9 `Router::merge` (panics on two fallbacks), `layer` ordering, `with_state`: https://docs.rs/axum/0.8.9/axum/struct.Router.html
- ADRs: `docs/adr/0002-areas-at-the-repository-root.md:24` (root location, Dockerfile beside it),
  `docs/adr/0003-one-modulith-per-area.md` (wiring only; Consequences), `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`
  (the composition root; process-global caches become capability-owned).
- Current wiring: `areas/access/dpe/server/src/serve.rs`, `router.rs:107-153`, `observability.rs:90,116`, `cli.rs`, `shell.rs:13-16`.
- Globals: `areas/access/dpe/core/src/utils.rs:48-123`, `core/src/ark.rs:36-63`, `api-oai/src/lib.rs:21-47`, and the ten
  `static` caches in `core/src/*_cache.rs`.
- Pattern to mirror: `areas/deposit/editor/server/src/shell.rs:5-30` (`AppState` of owned values),
  `areas/deposit/editor/server/src/test_support.rs:175-227` (test-only shared fixtures).
- Deploy coupling: `ops-deploy/repository.yml:35,43` (image `daschswiss/dpe`, healthcheck `/app/dpe-server`).
- Precedent: `docs/specs/2026-09-28-move-dpe-to-access-area/` (plan and journal).
- Learnings: `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`,
  `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`.
