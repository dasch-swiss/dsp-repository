---
dune_map: true
schema_version: 2
date: 2026-10-15
---

# Architecture Map

## Overview

`dsp-repository` is a Rust monorepo (a Cargo workspace today; Bazel per ADR-0001 once the migration lands) of
server-rendered hypermedia services on Axum + Maud + Datastar, plus the crates they share.
Two services exist — **DPE**, the public read-only Discovery and Presentation Environment,
and the authenticated **metadata editor** — each a separate deployable on its own origin;
they share `shared-metadata` (the research-metadata contract), `shared-telemetry` (the
browser-beacon collector) and `mosaic-tiles` (the design system), and nothing else. The
dependency arrow is one-way: `services → shared, mosaic`, and a service never imports
another service. The shared root has moved to `shared/` at the repository root, the
metadata editor to `areas/deposit/editor/` with its area's composition root `deposit-server` at `areas/deposit/server/`, and DPE to `areas/access/dpe/` (ADR-0002); Mosaic
still lives under `modules/`; `areas/access/sync` exists at its minimum, the crate `sync-store` and
the committed snapshot and curation it serves; `areas/access/cpe` holds its `CONTEXT.md`, the port crate
`cpe-ports`, the domain crate `cpe-core`, the component library `cpe-components`, the interim image viewer
`cpe-image-viewer`, the read-side store `cpe-store`, the store builder `cpe-store-build`, the page assembly `cpe-web`, the host crate `cpe-server`, Incunabula's project folder and CPE's shared static files, and `access-server` mounts it under its base; of the five planned components,
`areas/access/media` holds nothing yet, and the
other four hold only a `CONTEXT.md` at their target path. The rest of ADR-0002 (accepted, migration pending) moves Mosaic to the root,
beside `shared/`, `vitrinli/` and `chischtli/`, so read the globs here as current state.
`dsp-cli/` is a sixth root, outside the `services → shared, mosaic` arrow entirely (ADR-0002
amendment): it is a client of every area rather than a member of one, no area depends on it, and it depends on no area
crate. Vocabulary: root [`CONTEXT.md`](CONTEXT.md)
(the context index and the shared contract terms), `areas/deposit/editor/CONTEXT.md`,
`areas/access/CONTEXT.md`, `areas/access/cpe/CONTEXT.md`, `areas/access/sync/CONTEXT.md`,
`areas/access/datei/CONTEXT.md`,
`areas/archive/CONTEXT.md`, `vitrinli/CONTEXT.md`,
`chischtli/CONTEXT.md`, `dsp-cli/CONTEXT.md`. Decisions: [`docs/adr/`](docs/adr/).

## Components

### areas/access/dpe

- **Paths:** `:(glob)areas/access/dpe/**`, `:(glob)areas/access/CONTEXT.md` (the Access Area's
  vocabulary, DPE's until it is split; CPE's own terms are in `areas/access/cpe/CONTEXT.md`)
- **Purpose:** The Discovery and Presentation Environment — the Access Area's first service.
  Four crates: `dpe-core` (view model, the `Corpus` value that owns the lazily-loaded caches,
  filesystem repositories, a DSP-API client), `dpe-api-oai` (OAI-PMH 2.0), `dpe-web` (Maud pages
  and components), `dpe-server` (a library: routes, page shell, Datastar SSE fragments, config,
  the `validate` logic; the binary is areas/access/server's). Also owns the published **corpus** under `server/data/` (projects, persons and
  organizations, whose sizes `corpus-manifest.json` records; plus 5 clusters, 3 record
  dumps, two lookup tables) and the cover images under `public/assets/images/`.
- **Key entities:** `Project`, `ProjectQuery`, `VALID_TABS`, `all_projects`,
  `project_by_shortcode`, `cover_image_url`, `ClusterRaw`, `ClusterRef`, `CollectionRef`,
  `ProjectRepository` / `FsProjectRepository`, `RecordRepository` / `FsRecordRepository`,
  `OaiRecord`, `CachedContributorLookup`, `resolve_inputs`, `records_for_shortcode`,
  `project_oai_identifier`, `oai_handler`, `OaiState`, `RenderContext`, `build_router`,
  `tab_fragment_handler`, `search_fragment_handler`, `record_file_handler`, `HeadExtras`,
  `landing_page` / `LandingPage` / `render`, `Corpus` / `CorpusSettings`, `Dpe`, `DpeConfig`
- **Public interface:** the HTTP routes of `dpe-server` (`/dpe/projects`, `/dpe/projects/{id}`,
  `/dpe/projects/{id}/metadata.jsonld` and `/dpe/projects/{id}/metadata.datacite.json`,
  `/dpe/projects/{id}/tab/{tab}` and `/dpe/projects/search` as SSE, `/dpe/records/{shortcode}/{record_id}/file`,
  `/dpe/oai`, `/dpe/api/v2/projects[/{id}]`, `/dpe/about`); to areas/access/server only,
  `dpe-server`'s library surface `DpeConfig`, `Dpe::{new, router, warm}`, `validate`,
  `normalize_page_url`, `RightmostXffKeyExtractor`, which names no `dpe-core`/`dpe-web`/`dpe-api-oai`
  type; and the corpus files under `server/data/`, which the editor consumes as an image-baked
  snapshot through `EDITOR_DATA_DIR`. No `dpe-*` crate other than `dpe-server` is depended on by
  any crate outside this component, and `dpe-server` only by areas/access/server.
- **Local-context kit:** `areas/access/dpe/CLAUDE.md`, `areas/access/dpe/server/src/router.rs`,
  `areas/access/dpe/server/src/shell.rs`, `areas/access/dpe/core/src/lib.rs`,
  `areas/access/dpe/core/src/project.rs`, `areas/access/dpe/server/src/fragments.rs`,
  `areas/access/dpe/api-oai/src/lib.rs` (the OAI crate's whole surface; the contract it reads is
  shared/metadata's own kit). The kit is at its seven-file budget, so
  `areas/access/dpe/server/src/lib.rs` (the `Dpe` surface the composition root mounts, and the
  one production `Box::leak` of the corpus) is named here rather than added.
- **Depends on:** shared/metadata (all four crates), shared/fair (`dpe-api-oai`, `dpe-server`), shared/telemetry
  (`dpe-server`, `traceparent.rs`), modules/mosaic (`dpe-web`, `dpe-server`); third-party: axum, tokio,
  tower / tower-http / tower_governor, maud, datastar, figment, serde / serde_json, quick-xml,
  ureq, opentelemetry / tracing-opentelemetry, insta
- **Used by:** areas/access/server (`dpe-server`'s library surface only); areas/deposit/editor — data only, never code: the corpus is copied into the editor image
  (`.github/actions/build-editor/action.yml`, `justfile`) and read by the editor's tests through
  `editor_core::checkout_dpe_data_dir()` (`DPE_DATA_DIR`, the one Rust definition of the path); shared/metadata's and shared/fair's
  committed-data tests live here (`core/src/temporal_enrichment_cache.rs`,
  `api-oai/src/metadata/corpus.rs`); `editor-collector` writes the corpus through a pull request
  (see **Durable state** above), still never through a `dpe-*` import
- **Boundary rules:**
  - Never depends on an `editor-*` crate; the editor never depends on a `dpe-*` crate
    (**review** for this side; **static-analysis** for the editor's, `check-capability-deps.sh`;
    **structure** via Bazel visibility after ADR-0001).
  - `dpe-api-oai` depends on `dpe-core`, `shared-metadata` and `shared-fair` only — never on
    `dpe-web` or another API crate (**review**; a `Cargo.toml` check in the style of
    `check-shared-paths.sh` would make it **static-analysis** today — tracked with the
    `dpe-*`/`editor-*` check as a follow-up).
  - The editor's path is `ProjectRaw` → draft → `ProjectRaw`, never through `dpe_core::Project`,
    whose `From` impls are lossy on `url` and `clusters` (**review**).
  - Every `<a>` enhanced by Datastar keeps a working `href` (**review**).
  - The full page and the SSE fragment render `#project-tabs` through the same function
    (**static-analysis** — a test pins it).
  - `PreEscaped` has exactly three sanctioned sites: the Mosaic `IconData` SVG, the leading
    newline the Mosaic `textarea` writes back, and the JSON-LD splice in
    `server/src/metadata.rs`, whose only permitted input is `shared_fair`'s `script_safe_json`
    (**static-analysis** — a test in `dpe-server` greps every file under its own `src/` so
    `PreEscaped(` appears exactly once in the crate). The first two splice a constant. The
    search-query echo in `fragments.rs` stays an auto-escaped splice (**review**).
  - Page, fragment and API routes live in `server/src/router.rs::build_router`, un-layered;
    the OTel layers and the two untraced routes, `/healthz` and `POST /telemetry/collect`, are
    areas/access/server's.
  - No process-global state: the corpus, the OAI base URL and the placeholder flag are values
    (`Corpus`, `OaiState`, `RenderContext`, `AppState`) built from `DpeConfig` by `Dpe::new`; no
    `static` `OnceLock`/`LazyLock` outside `#[cfg(test)]` (**review**; ADR-0007).
- **Durable state:** the corpus under `server/data/`. `projects/` has **two writers** — hand
  edits and `editor-core`'s `canonical_round_trip` test under `CANONICALIZE_PROJECT_FILES=1`,
  which is the canonical formatter, so the second writer is the intended one. `records/` has
  **two writers with two upstreams** — `just fetch-records` (production API, explicit bearer) and
  `dpe_core::record_cache::load_all_records`, which on a cache miss fetches from the dev API with a
  placeholder bearer and writes into the tracked directory at startup; the shortcode list is
  duplicated between `justfile` and `record_cache.rs` (single-writer violation, recorded). The
  two lookup tables are written by `scripts/*.py`. Cover images: drop-at-path, hand-written.
  `editor-collector` is a further writer of `projects/`, the first automated writer of `persons/`
  and `organizations/`, and a second writer of `temporal-coverage-enrichment.json` and
  `corpus-manifest.json` — but never in place: it writes a disposable checkout on an
  `editor-collect/<shortcode>` branch, and the gate between that and anything served is the pull
  request it opens plus a human merge (**static-analysis** via `canonical_round_trip`,
  `every_committed_temporal_coverage_resolves` and `the_corpus_is_the_whole_published_set`, all of
  which run on it because it is opened with `secrets.GH_TOKEN`; then **review**).
  The `Corpus`'s instance `OnceLock` caches load once per process and never invalidate.
- **Fingerprint:** `4eac5ac44cd2`

### areas/access/server

- **Paths:** `:(glob)areas/access/server/**`
- **Purpose:** The Access Area's composition root (ADR-0003, ADR-0007): the one binary,
  `access-server`, holding wiring only — CLI, config loading, OTel and Pyroscope init, the OTel
  layers, the untraced routes, and mounting each capability's router. It merges DPE's router and
  nests CPE's at CPE's base (ADR-0007, amendment of 2026-10-12, DEV-7438), constructs `sync`'s adapter of CPE's
  port over `SYNC_DATA_DIR` and has CPE activate over it in the background, after the listener is bound.
- **Key entities:** `app`, `assemble`, `mount_cpe`, `name_span_by_cpe_route`, `activate_in_background`, `serve`,
  `DEFAULT_SYNC_DATA_DIR`, `shutdown_signal`, `init_otel`, `init_pyroscope`,
  `TELEMETRY_NAME`, `is_allowed_healthcheck_url`
- **Public interface:** the `access-server serve | validate <data_dir> | healthcheck <url>` CLI;
  `/healthz` and `POST /telemetry/collect` (untraced), every route DPE's router carries, and every
  path under `Cpe::base()` (`CPE_BASE`, `/cpe` in the image), which is CPE's. `DPE_SITE_ADDR` and
  `SYNC_DATA_DIR` are read here; `DPE_*` and `CPE_*` by the capabilities' own configurations. The
  Docker image stays `daschswiss/dpe`.
- **Local-context kit:** `areas/access/server/CLAUDE.md`, `areas/access/server/src/serve.rs`,
  `areas/access/server/src/observability.rs`, `areas/access/server/src/cli.rs`,
  `areas/access/dpe/server/src/lib.rs`, `areas/access/cpe/server/src/lib.rs`,
  `docs/adr/0003-one-modulith-per-area.md`, `.github/scripts/check-composition-root-deps.sh`
- **Depends on:** areas/access/dpe (`dpe-server` only), areas/access/cpe (`cpe-server`, and
  `cpe-ports` for the trait of the adapter it hands over; never `cpe-store`), areas/access/sync
  (`sync-store`, to construct `LiveArchiveProjection`), shared/telemetry; expected, as each
  arrives: areas/access/datei and areas/access/media (each one's `server`
  crate, plus its `store` / `ports` crates only to construct `Live<Port>` adapters);
  third-party:
  axum, tokio, tower, clap, ureq, tower_governor, the OpenTelemetry stack, pyroscope, tracing-subscriber
- **Used by:** — (deployment: `areas/access/server/Dockerfile`, `.github/actions/build-dpe/`)
- **Boundary rules:**
  - The image carries each capability's data: what `build-dpe` puts into `staging/`, what the Dockerfile copies
    and what its `ENV` lines name have to agree (**review**; see `areas/access/server/CLAUDE.md`).
  - Depends on each capability's `server` crate only, never a capability's `core`, `web` or
    `api-*` crate (**static-analysis**: `check-composition-root-deps.sh` in `just check`).
  - Routes declared after the OTel layers are untraced, so the order in `app` is load-bearing
    (**static-analysis**: `serve.rs` tests assert the traceparent split).
  - No adapter or domain logic: wiring only (**review**).
  - Sets no router fallback: DPE's router carries the `ServeDir` fallback, and axum panics when
    merging two (**review**). CPE's router has a fallback too, so it is nested and never merged; a
    path outside the base is answered as with no CPE in the binary (**static-analysis**: `serve.rs`
    tests).
  - Writes no path of CPE's: the mount is `Cpe::base()`, and without a base (`CPE_BASE` unset or
    refused) CPE is not mounted and DPE is served alone (**static-analysis**: `serve.rs` tests, one of
    them a search of every file of this crate's `src/` for a `"/cpe` literal outside a test module).
  - CPE never stops or delays the binary: nothing of CPE's is awaited or unwrapped in `serve`,
    activation runs on the blocking pool after the bind, and `app` builds the router with CPE inside
    `catch_unwind`: a base the router cannot be built with leaves DPE served alone, with one error that
    names `CPE_BASE`. No base under a path of DPE's or of the binary's does that (the route keeps its
    answer); the one build known to panic is a second any-method route at exactly `<base>/`
    (**static-analysis** for the router: `serve.rs` tests hold an activation open, take the data
    directory away, mount CPE at bases that are routes and at one whose mount panics; **review** for
    `serve` itself, which no test runs).
  - Depends on `cpe-server` and `cpe-ports` and on no other CPE crate (**static-analysis**:
    `check-cpe-seam.sh` rule 1 and `check-composition-root-deps.sh`, both in `just check`).
  - The OTel tracer name and Pyroscope application stay `dpe-server`, the `service.namespace` and
    collector scope `dpe`: DPE's telemetry identity, not this crate's name (**review**). CPE's
    requests are traced under it, each span named by the route CPE reports (**static-analysis**:
    `serve.rs` tests), never by a path that matched no route.
- **Durable state:** none.
- **Fingerprint:** `4ca5264600e2`

### areas/deposit/editor

- **Paths:** `:(glob)areas/deposit/editor/**`
- **Purpose:** The metadata editor — the Deposit Area's first service. Five crates:
  `editor-core` (the draft model, validation, the canonical project and entity writers, the
  persistence ports), `editor-ports` (what the editor asks of identity: `Principal`,
  `Authenticator`, `UserDirectory` and their fakes), `editor-store` (the SQLite implementation
  of the persistence ports, the file `editor.sqlite`, and `LiveAuthoredWork` behind
  `identity-ports`), `editor-web` (the editor's HTTP surface: pages, the form's field registry
  and widgets, the handlers and route table, the extractors, the startup loaders and reconcile,
  behind the `Editor` facade; the document shell around the pages is the area's
  `deposit-shell`, below), `editor-collector` (the CI binary that turns an approved record into a
  pull request against this repository — outside the service's request path, and the only crate
  here that writes anything under `areas/access/dpe/`).
  Depositors edit their projects section by section; RDU reviews field by field; approve
  writes an approved record, which the collector publishes as a pull request. The Deposit
  Area is two capabilities, `identity` and `editor`, under the composition root `deposit-server`
  (`areas/deposit/server`, below; `areas/deposit/ADR-0004`), with identity's four crates in
  `areas/deposit/identity` and the area shell in `areas/deposit/shell`.
- **Key entities:** `ProjectDraft`, `ProjectState`, `SubmissionState`, `ReviewState`,
  `FieldReview`, `Decision`, `EntityProposal`, `Transition`, `PublishedProjects`, `Agents`,
  `EditorRepositories` (behind `EditorState.editor`), `ReviewRoundRepository`, `RecordClassification`, `classify_record`,
  `registry::FIELDS`, `registry::SECTIONS`, `Shape`,
  `apply`, `write_project`, `write_entity`, `normalize_shortcode`, `Principal`, `Authenticator`,
  `UserDirectory`, `Account`, `Authenticated`, `Rdu`, `LiveAuthoredWork`, `Editor`,
  `EditorSettings`, `EditorState`, `CollectionToken`, `COLLECTION_REPORT_PATH`, `is_enhanced`,
  `routes`, `RouteKind`, `KNOWN_ROUTES`, `normalize_page_url`, `reconcile_published`,
  `Forge`, `CollectionReport`
- **Public interface:** the HTTP routes of `editor-web`'s route table, root-mounted on the
  editor's own hostname (`/` (a redirect), `/projects`, `/projects/{shortcode}`,
  `/projects/{shortcode}/sections/{section}` and its row-action `POST`s,
  `/projects/{shortcode}/entities/{proposal}` and its row-action `POST`s, `/review`, `/review/{shortcode}`,
  `/collection`, `/collection/{id}/discard`, `/states`,
  `GET /api/v1/approved-records`, `POST /api/v1/collection-report`), served only through
  `Editor::router`, which `deposit-server` merges with identity's; the
  `editor-collector collect | refresh` CLI, invoked only by
  `.github/workflows/collect-editor-records.yml`. The `editor_web` library for the root
  (`Editor::{start, router}`, `EditorSettings`, `CollectionToken`, `normalize_page_url`,
  `RightmostXffKeyExtractor`, `COLLECTION_REPORT_PATH`; `Editor::for_test` and `test_support`
  behind `test-support`). The `editor_store` library for the root (`Database`, `Source`,
  `LiveAuthoredWork`; `test_support`'s plain-value seed helpers and row readers behind
  `test-support`). The `editor_ports` library, which `identity-store` implements. Outside this
  component only `deposit-server` and `identity-store` (on `editor-ports`) depend on an `editor-*` crate.
- **Local-context kit:** `areas/deposit/editor/CLAUDE.md`, `areas/deposit/editor/web/src/lib.rs`,
  `areas/deposit/editor/web/src/routes.rs`, `areas/deposit/editor/web/src/handlers/auth.rs`,
  `areas/deposit/server/src/router.rs`, `areas/deposit/editor/core/src/form.rs`,
  `areas/deposit/editor/store/src/lib.rs`. The kit is at its seven-file budget; the next reads
  are `web/src/form/registry.rs` (a form change), `core/src/status.rs` (project state),
  `ports/src/lib.rs` (what identity answers) and `docs/src/editor/{collection,architecture}.md`.
- **Depends on:** shared/metadata (`editor-core`, `editor-web`, `editor-collector`),
  shared/telemetry (`editor-web`: `is_valid_traceparent`),
  modules/mosaic (`editor-web`), areas/deposit/shell (`editor-web`),
  areas/deposit/identity (`editor-store` on `identity-ports` alone, the port it answers;
  `editor-web` and `editor-core` on no identity crate; `editor-ports` on `std` alone); areas/access/dpe's corpus as data (see above),
  never its code — `editor-collector` additionally *writes* that corpus, through a pull request
  and never at runtime (`docs/src/editor/collection.md`); third-party: axum, maud, rusqlite
  (`bundled`) + deadpool-sqlite, tower_governor, subtle,
  reqwest (`editor-collector`), the OpenTelemetry API, insta
- **Used by:** areas/deposit/server (`editor-web`'s facade and `editor-store`; `editor-ports`
  in its tests), areas/deposit/identity (`identity-store` on `editor-ports`, the ports it answers)
- **Boundary rules:**
  - Never depends on a `dpe-*` crate (**static-analysis** — `check-capability-deps.sh` fails an edge to another
    area's crate; **structure** after ADR-0001).
    `core/src/agents.rs` states why it avoids `dpe-core`'s process-wide caches.
  - `EDITOR_DATA_DIR` has no default on purpose: the only plausible one would be a relative path
    into DPE's tree; unset is a configured state (the PR preview runs without a snapshot), so
    nothing refuses to start (**review**).
  - Separate origin from DPE; every state-changing route is `POST` and passes the
    `Sec-Fetch-Site` same-origin check in `deposit-server`; a write URL also answers `GET` (**review**).
    The one exemption, `POST /api/v1/collection-report`, is `editor-web`'s
    `COLLECTION_REPORT_PATH`, which the route table and the root's `csrf.rs` both read
    (**structure**); its handler checks a bearer token in constant time (`CollectionToken`).
  - Every `RouteKind::Page` entry of `editor-web`'s `routes.rs` normalizes to its own path
    template in `editor-web`'s `page_url.rs`, and the root's normalizer asks identity's, then
    the editor's, else `other` (**static-analysis** —
    `every_full_page_route_is_labelled_with_its_own_template`, one per web crate);
    the kind a route is given is not checked by any test (**review**).
  - `Editor::router` returns its routes without CSRF or tracing; `deposit-server` supplies
    both (**review**).
  - Only `editor-web`'s `handlers/`, route table, `shell.rs` and `lib.rs` use axum; `pages/`, `form/` and `entity/` are
    `fn -> Markup` (**review**).
  - `Authenticated` / `Rdu` are extractors, not middleware: a public handler is public in its
    signature (**structure**).
  - Reader connections are `query_only=ON`; the only write path is `Database::write`, one
    connection, `BEGIN IMMEDIATE`, in each store (**structure**).
  - Every contract member is either a registry `Field` or listed in `OMITTED`
    (**static-analysis** — `every_contract_field_is_either_placed_in_the_form_or_deliberately_omitted`).
  - Depositor-facing vocabulary is closed (REQ-2.1 / REQ-2.2): five states, no "export", "JSON",
    "transfer", "commit", "pull request" (**static-analysis** — `depositor_vocabulary.rs` plus the
    E2E pass).
  - `editor-store` has no foreign keys: the seven account columns are plain `TEXT`, and a
    deleted account leaves its id behind, which pages render as unknown (**structure**;
    `deposit-server`'s `removed_account.rs` pins the pages: **static-analysis**). Its only
    identity dependency is `identity-ports` (**static-analysis** — `check-capability-deps.sh`: a `store`
    crate may depend on another capability's `ports` crate alone).
  - `editor-web` sees identity only through `editor-ports`: `Authenticated` / `Rdu`
    (`handlers/auth.rs`, with its own GET-only sign-in destination over `deposit-shell`'s
    `safe_next` and its own `Cookie`-header scan) over `Authenticator`, the name reads in
    `review.rs` and `sections.rs` over `UserDirectory`; it depends on no identity crate
    (**static-analysis** — `check-capability-deps.sh`). Its handler tests sign in through
    `FakeAuthenticator` (`web/src/fixtures.rs`); tests that need both capabilities are
    `deposit-server`'s.
  - `editor-ports` depends on `std` alone: on no workspace crate but `shared-*`
    (**static-analysis** — `check-capability-deps.sh`), on no third-party crate (**static-analysis** — the same script).
- **Durable state:** the SQLite file `editor.sqlite` — `drafts`, `submissions`, `review_rounds`,
  `approved_records`, `entity_proposals`, all `STRICT`, one baseline migration `0001` under
  `editor/store/src/migrations/` (edited in place until the first deployment, then forward-only)
  guarded by its own `PRAGMA user_version`; identity's tables are in `identity.sqlite`
  (below). **Single writer:** `editor-store` through `editor-core`'s repository ports; several of
  `editor-web`'s handlers call the same port, serialized by the one writer connection, and every
  multi-table transition is one repository method. `review_rounds` is append-only.
  `approved_records` is created only by approve, updated only by
  `POST /api/v1/collection-report` (`mark_collected`, `report_collection`), and deleted by three
  paths: the startup reconcile that derives Online (`editor-web`'s `startup.rs`, inside
  `Editor::start`), approve's own transaction, which supersedes an earlier record for the same
  project when no pull request of its own is live, and RDU's discard
  (`POST /collection/{id}/discard`).
- **Fingerprint:** `407d95f66ff4`

### areas/deposit/shell

- **Paths:** `:(glob)areas/deposit/shell/**`
- **Purpose:** `deposit-shell`, the Deposit Area's document shell (`areas/deposit/ADR-0004`):
  the document (`view::page`), the header and footer, the 403, 404 and problem pages, the
  area's sign-in link with its one open-redirect check (`sign_in::safe_next`, `login_url`,
  `NEXT`), and the RDU-only text. A `fn -> Markup` library that both capabilities' pages are
  wrapped in; it turns nothing into a response.
- **Key entities:** `page`, `Viewer`, `header`, `footer`, `forbidden`, `not_found`,
  `unavailable`, `safe_next`, `login_url`, `NEXT`, `MAX_NEXT_LEN`, `RDU_ONLY`
- **Public interface:** the `deposit_shell` library: `view::{page, Viewer}`,
  `pages::{forbidden::forbidden, not_found::not_found, problem::unavailable}`,
  `sign_in::{safe_next, login_url, NEXT}`, `RDU_ONLY`; the header and footer are private to
  `view::page`. No routes, no binary.
- **Local-context kit:** `areas/deposit/shell/src/lib.rs`, `areas/deposit/shell/src/view.rs`,
  `areas/deposit/shell/src/sign_in.rs`, `areas/deposit/shell/src/components/global/header.rs`,
  `areas/deposit/editor/web/src/shell.rs`, `areas/deposit/identity/web/src/handlers/guard.rs`,
  `areas/deposit/editor/style/main.css`
- **Depends on:** modules/mosaic (`mosaic-tiles`, the header's sign-out button); third-party:
  maud
- **Used by:** areas/deposit/editor (`editor-web`: `shell.rs` turns its pages into
  responses, `handlers/auth.rs` builds the sign-in redirect, every handler module renders the
  problem page), areas/deposit/server (`deposit-server`: `shell.rs` renders the fallback 404), areas/deposit/identity (`identity-web`: `shell.rs` turns its pages into
  responses, `handlers/guard.rs` and `handlers/login.rs` build the sign-in redirect,
  `handlers/depositors.rs` renders the problem page)
- **Boundary rules:**
  - Depends on no capability crate, nor any workspace crate but `mosaic-tiles` and `shared-*`
    (ADR-0003, amendment of 2026-09-25) (**static-analysis** — `check-capability-deps.sh`). Has no Axum:
    each caller turns its `Markup` into a response (**static-analysis** — the same script's
    web-framework list).
  - The routes it links are string literals: `/` and `POST /logout` (header), `/projects`
    (403), `/login` (`login_url`, and the two login screens `safe_next` refuses), plus the
    assets `/logo.svg`, `/vendor/datastar.js` and `/telemetry.js`, which `deposit-server`
    serves from `areas/deposit/editor/public`. No compiler checks them against the route tables (**docs-only**, recorded in the
    crate doc).
  - One copy of the open-redirect allowlist: every request-derived redirect target goes through
    `safe_next`, and `login_url` interpolates without encoding only because `safe_next` admits
    unreserved characters alone (**static-analysis** — the nine `sign_in.rs` tests pin both
    together; **review** for new callers, `REVIEW.md`).
  - Its Tailwind classes ship only through `@source '../../shell/**/*.rs'` in the editor's
    `style/main.css`; moving the crate drops them silently (**review**).
- **Durable state:** none.
- **Fingerprint:** `e24b77ad3aa6`

### areas/deposit/identity

- **Paths:** `:(glob)areas/deposit/identity/**`
- **Purpose:** Identity's crates in the Deposit Area's `identity` capability
  (`areas/deposit/ADR-0004`). `identity-core`: accounts, sessions, login codes and mail sends
  as records and repository ports, the login rules, the session rules, the cookie names and
  `Cookie`-header scan, code and token generation, and the `Mailer` trait. `identity-ports`:
  `AuthoredWork`, what identity asks of the editor. `identity-store`: the SQLite file
  `identity.sqlite` behind the four ports, the mail transports, and `LiveAuthenticator` and
  `LiveUserDirectory` behind `editor-ports`. `identity-web`: the login and depositor pages,
  their handlers and route table, identity's own `Authenticated`/`Rdu` extractors, the
  `Set-Cookie` glue, and the `Identity` facade the composition root starts and mounts.
- **Key entities:** `User`, `Role`, `Session`, `LoginCode`, `UserRepository`,
  `SessionRepository`, `LoginCodeRepository`, `MailSendRepository`, `Attempt`, `Issued`,
  `IdentityRepositories`, `RepositoryError`, `AuthConfig`, `locked_out`,
  `is_plausible_address`, `delta`, `CODE_TTL`, `SEND_WINDOW`, `MAX_CODE_ATTEMPTS`, `begin`,
  `current`, `end`, `Viewer`, `Ended`, `TOUCH_INTERVAL`, `SESSION`, `LOGIN`, `Mailer`, `Mail`,
  `MailError`, `SmtpMailer`, `ConsoleMailer`, `AuthoredWork`, `LiveAuthenticator`,
  `LiveUserDirectory`, `Identity`, `IdentitySettings`, `SmtpPassword`, `StartupError`,
  `IdentityState`, `Authenticated`, `Rdu`, `ensure_rdu`, `spawn_sweep`, `routes`, `RouteKind`,
  `normalize_page_url`, `RecordingMailer`
- **Public interface:** the `identity_core` library (`records`, `repository`, `login`,
  `session`, `cookie`, `secret`, `mail`), the `identity_ports` library (`AuthoredWork`, which
  `editor-store` implements, `PortFuture`, `Unavailable`; `FakeAuthoredWork` behind
  `test-support`) and the `identity_store` library (`Database`, `Source`, `DbError`,
  `mail::mailer`, `SmtpSetupError`, `LiveAuthenticator`, `LiveUserDirectory`; behind
  `test-support`, `test_support`'s plain-value seed helpers and readers (`seed_user`,
  `seed_session`, `find_user`, `UserRow`, `session_exists`, …) and `FaultyStore`) and the `identity_web` library (`Identity::{start, router}`,
  `IdentitySettings`, `SmtpPassword`, `StartupError`, `normalize_page_url`, and `CODE_TTL` and
  `is_plausible_address` for the root's configuration check; `Identity::for_test`, `ISSUE_BURST` and
  `test_support::{RecordingMailer, SESSION, LOGIN}` behind `test-support`). Its HTTP routes (`/login`,
  `/login/code`, `/logout`, `/depositors…`) are served only through `Identity::router`. No
  binary.
- **Local-context kit:** `areas/deposit/identity/CLAUDE.md`,
  `areas/deposit/identity/core/src/session.rs`, `areas/deposit/identity/core/src/repository.rs`,
  `areas/deposit/identity/store/src/live.rs`, `areas/deposit/identity/web/src/lib.rs`,
  `areas/deposit/identity/web/src/routes.rs`, `docs/src/editor/authentication.md`
- **Depends on:** areas/deposit/editor (`identity-store` on `editor-ports` alone, the ports it
  answers; `identity-ports` on `std` alone), areas/deposit/shell, modules/mosaic,
  shared/metadata and shared/telemetry (`identity-web`); third-party: async-trait, base64,
  chrono, rand + subtle, thiserror, tracing, uuid; rusqlite (`bundled`) + deadpool-sqlite and
  lettre (`identity-store`); axum, maud, tower_governor, tokio, the OpenTelemetry API
  (`identity-web`)
- **Used by:** areas/deposit/server (`identity-web`'s facade and re-exports, `identity-store`'s
  stores and adapters; never `identity-core`), areas/deposit/editor (`editor-store` on
  `identity-ports`; `editor-web` on no identity crate)
- **Boundary rules:**
  - `identity-core` has no Axum, Maud or database dependency (**static-analysis** — `check-capability-deps.sh`).
  - Depends on no `editor-*` crate but `editor-ports` (from `identity-store`), and neither
    `editor-core` nor `editor-web` depends on it: capabilities share no core (ADR-0003)
    (**static-analysis** — `check-capability-deps.sh`).
  - One copy of the session rules: every caller authenticates through `session::current`,
    identity's extractor and login screens directly (`identity-web`'s `handlers/guard.rs`,
    `handlers/login.rs`) and the editor through `LiveAuthenticator`, its only caller on the
    editor's side (**review**).
  - `identity-ports` depends on `std` alone: on no workspace crate but `shared-*`
    (**static-analysis** — `check-capability-deps.sh`), on no third-party crate (**static-analysis** — the same script).
  - `identity-web` depends on no `editor-*` crate: the removal confirmation's open work comes
    through `identity-ports` (**static-analysis** — `check-capability-deps.sh`). Only its `handlers/`,
    route table, `shell.rs` and `lib.rs` use axum; `pages/` are `fn -> Markup` (**review**).
  - Every `RouteKind::Page` entry of `routes.rs` normalizes to its own template in its
    `page_url.rs` (**static-analysis** — `every_full_page_route_is_labelled_with_its_own_template`).
  - `Identity::router` returns its routes without CSRF or tracing; whoever mounts it supplies
    both (**review**).
  - Its Tailwind classes ship only through `@source '../../identity/web/**/*.rs'` in the editor's
    `style/main.css` (**review**).
- **Durable state:** the SQLite file `identity.sqlite` — `users`, `user_shortcodes`,
  `sessions`, `login_codes`, `mail_sends`, all `STRICT`, with four internal foreign keys, one
  baseline migration `0001` under `identity/store/src/migrations/` guarded by its own
  `PRAGMA user_version`. **Single writer:** `identity-store`, through `identity-core`'s
  repository ports, called by `identity-web`'s handlers, the RDU bootstrap and the hourly
  sweep, and by `session::current`'s idle touch (also through `LiveAuthenticator`).
- **Fingerprint:** `53e192a457a1`

### areas/deposit/server

- **Paths:** `:(glob)areas/deposit/server/**`
- **Purpose:** `deposit-server`, the Deposit Area's composition root and its one binary
  (`areas/deposit/ADR-0004`): configuration loading and validation, observability, the CSRF and
  OTel layers, the untraced routes, the composed page-URL normalizer and the fallback 404. It
  opens identity's store, then the editor's, constructs the adapters, starts `Identity` and then
  `Editor`, and merges their routers; it holds no business logic. Its tests are the ones that
  need both capabilities or the assembled app.
- **Key entities:** `EditorConfig`, `Secret`, `ConfigError`, `identity_settings`,
  `editor_settings`, `build_router`, `build_app`, `require_same_origin`, `normalize_page_url`,
  `TELEMETRY_NAME`, `init_otel`, `init_pyroscope`, `install_tracing_panic_hook`, `not_found`,
  `Cli`, `healthcheck`, `TestState`, `test_app`
- **Public interface:** the `deposit-server serve | healthcheck` CLI (the image's `ENTRYPOINT`
  and `HEALTHCHECK`, `areas/deposit/server/Dockerfile`, built by `just build-docker-editor` and
  `.github/actions/build-editor`); its own routes `/healthz`, `POST /telemetry/collect` and the
  `ServeDir` fallback over `areas/deposit/editor/public`. No library: nothing depends on it.
- **Local-context kit:** `areas/deposit/server/CLAUDE.md`, `areas/deposit/server/src/serve.rs`,
  `areas/deposit/server/src/router.rs`, `areas/deposit/server/src/config.rs`,
  `areas/deposit/server/src/csrf.rs`, `areas/deposit/server/src/test_support.rs`,
  `areas/deposit/server/Cargo.toml`
- **Depends on:** areas/deposit/identity (`identity-web`: `Identity`, `IdentitySettings`,
  `SmtpPassword`, `CODE_TTL`, `is_plausible_address`, `normalize_page_url`; `identity-store`:
  `Database`, `Source`, `LiveAuthenticator`, `LiveUserDirectory`), areas/deposit/editor
  (`editor-web`: `Editor`, `EditorSettings`, `CollectionToken`, `COLLECTION_REPORT_PATH`,
  `RightmostXffKeyExtractor`, `normalize_page_url`; `editor-store`: `Database`, `Source`,
  `LiveAuthoredWork`; `editor-ports` in tests only), areas/deposit/shell (the fallback 404),
  shared/telemetry (`collect_route`, `is_valid_traceparent`); third-party: axum, clap, figment,
  ureq, tokio, tower-http, tower_governor, the OpenTelemetry stack, pyroscope, insta
- **Used by:** nothing in the workspace; the top of the Deposit Area's dependency graph, a
  separate deployable (`daschswiss/metadata-editor`)
- **Boundary rules:**
  - Never depends on a capability's `core` or `api-*` crate, dev-dependencies included; it
    mounts `identity` and `editor` through their `web` crates' facades because neither has a
    `server` crate (**static-analysis** — `check-composition-root-deps.sh`, the root's one
    enforcer: `check-capability-deps.sh` skips `areas/<area>/server`). Its tests seed and
    read through the stores' plain-value `test-support` helpers instead of `core` types.
  - The root declares no capability route: `build_router` merges `Editor::router()` and
    `Identity::router()` before the OTel layers, and `build_app` declares `/healthz` and
    `POST /telemetry/collect` after them, so both stay untraced, then wraps everything in the
    CSRF layer (**static-analysis** — `router.rs`'s tests pin the order; the login `POST`s and
    a section save in `cross_capability.rs`, every other `POST` in `csrf.rs`).
  - The CSRF exemption is exactly `editor_web::COLLECTION_REPORT_PATH`, the path the route
    table also reads (**structure**).
  - The telemetry identity is pinned, not derived from the crate name: tracer and Pyroscope
    application name `"editor-server"` (`TELEMETRY_NAME`), Pyroscope `service.namespace=editor`,
    beacon scope `"editor"` (**review**; recorded in `CLAUDE.md`).
- **Durable state:** none of its own. It opens `identity.sqlite` and `editor.sqlite` in
  `EDITOR_DB_DIR` (two named in-memory databases when unset) and hands each store to its
  capability, which is the single writer.
- **Fingerprint:** `b5b211dccdfc`

### modules/mosaic

- **Paths:** `:(glob)modules/mosaic/**`
- **Purpose:** Mosaic, the DaSCH design system: `mosaic-tiles`, a library of Maud component
  functions and builders with co-located CSS and the design tokens, and `mosaic-playground`,
  a plain Axum showcase binary. Shared kernel of every hypermedia server here; other DaSCH
  codebases adopt it by copy, never by dependency, so nothing outside this repository can
  break when a tile changes.
- **Key entities:** `ComponentBuilder`; constructors `badge`, `button`, `alert`, `card`, `link`,
  `table`, `tabs`, `text_field`, `select`, `textarea`, `checkbox_group`, `radio_group`,
  `repeatable_list`, `icon`, `copy_button`, `loading`, `breadcrumb`; builders
  `BadgeBuilder`, `ButtonBuilder`, `TextFieldBuilder`, …; variant enums with `css_class()`
  (`ButtonVariant`, `BadgeVariant`, `CardVariant`, `AlertVariant`, `InputType`); `IconData`;
  playground `router`, `COMPONENT_NAV`
- **Public interface:** the `mosaic-tiles` crate (`ComponentBuilder`, `components::*`);
  two CSS files consumers `@import` — `tiles/src/components/theme_provider/tokens.css` and
  `tiles/src/components/components.css` (the barrel); the tiles `.rs` sources as a Tailwind
  `@source` glob; the playground binary.
- **Local-context kit:** `docs/src/mosaic/component-api-conventions.md`,
  `modules/mosaic/CLAUDE.md`, `modules/mosaic/tiles/src/builder.rs`,
  `modules/mosaic/tiles/src/components/badge/mod.rs`,
  `modules/mosaic/tiles/src/components/mod.rs`,
  `modules/mosaic/tiles/src/components/components.css`, `modules/mosaic/playground/src/app.rs`
- **Depends on:** nothing in the workspace; third-party: maud, icondata (tiles); axum, tokio,
  tower-http, optional tower-livereload + notify behind `dev` (playground)
- **Used by:** areas/access/dpe (`dpe-web`, `dpe-server`), areas/deposit/editor (`editor-web`),
  areas/deposit/shell (`deposit-shell`), areas/deposit/identity (`identity-web`); all three
  Tailwind entries (`areas/access/dpe/style/main.css`, `areas/deposit/editor/style/main.css`,
  `modules/mosaic/playground/style/main.css`)
- **Boundary rules:**
  - Depends on no `shared-*` or service crate (**structure** — a dependency would be a Cargo
    cycle for the services and is simply absent).
  - Every `css_class()` returns a complete literal class string so Tailwind's scan sees it;
    stated in six places, enforced by nothing — a violation surfaces as unstyled markup at
    runtime (**docs-only**; candidate for promotion).
  - Accessibility is the tile's responsibility, not the caller's; the working a11y gate is the
    editor's axe-core suite, which `a11y-editor.yml` triggers on `modules/mosaic/**`
    (**static-analysis**, indirectly).
  - Consumers use tiles through the crate API; 13 consumer source files hardcode Mosaic class strings
    instead (`card card-bordered`, `btn btn-outline`, `field-label`, and `tooltip`, which has no
    tile at all) — a reach-in, recorded (**review**).
  - Registration is in `tiles/src/components/mod.rs`, not `lib.rs`; `modules/mosaic/CLAUDE.md`
    and the `add-mosaic-component` skill say `lib.rs` — stale, fix on next touch (**docs-only**).
- **Durable state:** none. The playground E2E suite is dormant (no recipe, no CI job runs it).
- **Fingerprint:** `7e1f14442dc7`

### shared/fair

- **Paths:** `:(glob)shared/fair/**`
- **Purpose:** `shared-fair` — the FAIR exposure engine: one resolved graph per published
  object, and one writer per representation reading it. `ProjectGraph::build` applies agent
  resolution, placeholder filtering, multilingual preference and temporal-coverage resolution
  once; `RecordGraph::build` infers a record's creators from the record alone and needs no
  context. The mandatory-creator fallback is resolved once too, but as a derived accessor
  (`creators_with_fallback()` on each graph) rather than in `build`: only the representations
  DataCite's mandatory-creator rule governs apply it, and Dublin Core deliberately does not,
  because `oai_dc` names no creator an object does not have. Every representation then reads the
  same facts, so no two can disagree about one object (ADR-0005). Holds the DataCite and Dublin Core mappings that used to live in
  `dpe-api-oai`; the OAI-PMH envelope, verbs, identifiers and set specs stayed there.
- **Key entities:** `ProjectGraph`, `RecordGraph`, `PartRef`, `ResolveContext`, `AgentKind`,
  `ProjectAgent`, `RecordCreator`, `LicenseRef`, `TemporalRef`, `DisciplineRef`, `SpatialRef`,
  `FundingRef`, `PublicationRef`, `resolve_agent`, `UrlLayout`, `Candidate`, `Link`, `LinkSet`,
  the writers `project_to_datacite`, `project_to_dublin_core`, `record_to_datacite`,
  `record_to_dublin_core`, `project_to_dublin_core_meta`, `project_to_schema_org` and
  `project_to_link_set`, the embedding helper `script_safe_json`, and the output models
  `DataCiteRecord` and `DublinCoreRecord`
- **Public interface:** the root re-exports in `src/lib.rs` (the graphs and their refs, the
  seven writers — each named `{subject}_to_{format}` — the DataCite and Dublin Core models)
  plus the module paths `graph::*`,
  `project_graph::*`, `datacite::*`, `dublin_core::*`, `dublin_core_meta::*`,
  `record_datacite::*`, `record_dublin_core::*`, `schema_org::*`, `signposting::*`,
  `resolve::{resolve_agent, ResolvedAgent}` and the vocabulary helpers
  in `helpers::*`. `ResolveContext::new(lookup, periods, enriched)` is the wiring point: a
  consumer adapts its own store behind `shared_metadata::ContributorLookup` and the two
  temporal tables.
- **Local-context kit:** `shared/fair/src/lib.rs`, `shared/fair/src/graph.rs`,
  `shared/fair/src/project_graph.rs`, `areas/access/dpe/api-oai/src/metadata/mod.rs` (the OAI call
  site), `areas/access/dpe/api-oai/src/metadata/corpus.rs` (the corpus-wide tests),
  `shared/README.md`, `docs/adr/0005-fair-landing-pages-in-the-access-area.md`. The kit is at
  its seven-file budget, so the second call site, `areas/access/dpe/server/src/metadata.rs`, is named
  here rather than added: it is where the landing page's writers are called and the `UrlLayout`
  is built.
- **Depends on:** shared/metadata, and `serde_json` at runtime (the JSON-LD writer's `Value` is
  its output type, and `script_safe_json` serialises it); nothing else in the workspace. No
  other third-party runtime dependency.
- **Used by:** areas/access/dpe — `dpe-api-oai` (the OAI writers) and `dpe-server` (the landing
  page's JSON-LD, meta tags and Signposting links). `dpe-core` does not and must not: the domain
  crate never depends on the exposure engine. ADR-0005 names the consumers still to come: DPE's
  record pages, CPE, and the Deposit Area's FAIR assessment.
- **Boundary rules:**
  - Depends on no service crate (**structure** — Cargo cycle); holds no path into a service
    module (**static-analysis** — `.github/scripts/check-shared-paths.sh`, run by `just check`).
  - No web framework, no Maud, no routes: a writer returns a `String` or a
    `serde_json::Value`, and the consuming service turns that into a response (**review**).
  - Only `ProjectGraph::build`, `RecordGraph::build` and `PartRef::from_record` take a
    wire-contract root aggregate (`&ProjectRaw`, `&Record`); a builder's own private helpers may
    take one to decompose construction, and no public writer takes anything but a graph or a
    graph-derived field. Resolution happens once, at the build call, and a writer cannot reach
    past the graph to re-derive a fact (**review**).
  - Corpus-wide tests over the committed data live in the consumer that owns the data
    (`areas/access/dpe/api-oai/src/metadata/corpus.rs`), not here (**review**).
- **Durable state:** none. Holds no cache and reads no environment; the contributor lookup and
  the two temporal tables arrive in `ResolveContext`.
- **Fingerprint:** `c424b1286553`

### shared/metadata

- **Paths:** `:(glob)shared/metadata/**`
- **Purpose:** `shared-metadata` — the research-metadata wire contract and the rules for
  reading a value out of it: `ProjectRaw` and its parts, `Person`, `Organization`, `Record`,
  `Multilingual`, the placeholder rule, shortcode validation, temporal-coverage resolution,
  and the per-project checks. The published language between the Deposit and Access Areas.
- **Key entities:** `ProjectRaw`, `Multilingual`, `is_placeholder`, `multilingual_value`,
  `is_valid_shortcode`, `AuthorityFileReference`, `Person`, `Organization`, `Record`,
  `RecordPid`, `Finding`, `ContributorRef`, `ContributorLookup`, `is_organization_id`,
  `check_project`, `contributor_refs`,
  `completeness_gap`, `resolve_in`, `chronontology::load_from`, `temporal_enrichment::load_from`
- **Public interface:** the root re-exports in `src/lib.rs` plus the module-path items
  `temporal_coverage::*`, `w3cdtf::*`, `chronontology::*`, `temporal_enrichment::*`,
  `utils::parse_url_value` (the `url` reading rule, not re-exported at the root),
  `project::{CONTRIBUTOR_ROLES, ROLES_NOT_OFFERED, PROJECT_STATUS_VALUES, TYPE_OF_DATA_VALUES}`;
  the two fixtures under `testdata/` (`0803-records.json`, `0862-records.json`), read by
  `dpe-api-oai`'s tests by relative path — the single copy of each sample.
- **Local-context kit:** `shared/metadata/src/project.rs`,
  `shared/metadata/src/lib.rs`, `shared/README.md`,
  `areas/deposit/editor/web/src/form/registry.rs`, `areas/deposit/editor/core/src/draft.rs`,
  `areas/access/dpe/core/src/project.rs`, `areas/deposit/editor/core/tests/canonical_round_trip.rs`
  (a contract member moves with all four consumer files in one commit)
- **Depends on:** nothing in the workspace; third-party: serde, serde_json (`preserve_order`,
  which the editor's canonical writer requires), tracing
- **Used by:** areas/access/dpe (all four crates), areas/deposit/editor (`editor-core`, `editor-web`, `editor-collector`),
  areas/deposit/identity (`identity-web`), shared/fair,
  the DPE fuzz crate
- **Boundary rules:**
  - Depends on no service crate (**structure** — Cargo cycle); holds no path into a service
    module (**static-analysis** — `.github/scripts/check-shared-paths.sh`, run by `just check`).
  - Reads no environment: both lookup tables load from a caller-supplied `data_dir`; the caches
    live in `dpe-core`, and `editor-web` loads its own (`startup.rs`) (**review**).
  - Member order in `ProjectRaw` **is** the on-disk order of every committed project file
    (**static-analysis** — `every_committed_project_file_round_trips_byte_identically`).
  - Tests that read committed data live in the consumer that owns the data, not here
    (**review**).
- **Durable state:** none. Loads `chronontology-periods.json` and
  `temporal-coverage-enrichment.json` from whatever directory it is given.
- **Fingerprint:** `969101398529`

### shared/telemetry

- **Paths:** `:(glob)shared/telemetry/**`
- **Purpose:** `shared-telemetry` — the browser-beacon wire contract (`BeaconPayload`,
  `Signal` and its variants), origin and traceparent validation, and the `/telemetry/collect`
  collector that turns beacons into OTel metrics and structured logs. One implementation for
  both services.
- **Key entities:** `BeaconPayload`, `Signal`, `WebVitalSignal`, `ErrorSignal`, `LoafSignal`,
  `NavigationSignal`, `collect_route`, `collect_handler`, `process_signal`, `BROWSER_METRICS`,
  `PAGE_URL_NORMALIZER`, `is_allowed_origin`, `is_valid_traceparent`
- **Public interface:** `collector::collect_route(namespace, normalize_page_url)` — the intended
  wiring point (`collect_handler` is also `pub`, contradicting its doc comment); `beacon::*`,
  `origin::is_allowed_origin`, `traceparent::{is_valid_traceparent, validated_traceparent}`.
- **Local-context kit:** `shared/telemetry/src/beacon.rs`,
  `shared/telemetry/src/collector.rs`, `areas/access/dpe/public/telemetry.js`,
  `areas/deposit/editor/public/telemetry.js`, `docs/src/dpe/observability.md`,
  `shared/README.md`, `areas/access/dpe/server/fuzz/fuzz_targets/beacon_payload.rs`
- **Depends on:** nothing in the workspace; third-party: serde, serde_json; axum, opentelemetry,
  tracing, url (collector only). No `tower_governor` — the rate limiter is each server's own.
- **Used by:** areas/access/server (`collect_route("dpe", dpe_server::normalize_page_url)`), areas/access/dpe (`dpe-server`'s `traceparent.rs`),
  areas/deposit/server (`collect_route("editor", …)`), areas/deposit/editor (`editor-web`'s copy of `traceparent.rs`), both servers' `traceparent.rs`,
  areas/deposit/identity (`identity-web`'s copy of `traceparent.rs`),
  the DPE fuzz crate
- **Boundary rules:**
  - Depends on no service crate; holds no path into one (**structure** + **static-analysis**,
    as for `shared-metadata`).
  - Page-URL normalization is each service's own `page_url.rs`, passed in as a function — a
    shared crate cannot hold one service's route table (**structure** — the argument is
    required).
  - `namespace` is a required argument so the instrumentation scope dashboards filter on
    (`dpe.browser`, `editor.browser`) cannot be omitted (**structure**).
  - The client module is a two-copy fork (`areas/access/dpe/public/telemetry.js`,
    `areas/deposit/editor/public/telemetry.js`); a new signal edits both, plus `beacon.rs`,
    `collector.rs` and the docs, in one commit (**review**).
  - Metric attributes stay bounded; high-cardinality data goes to logs (**review**).
- **Durable state:** none. Process-wide statics (`METER_SCOPE`, `PAGE_URL_NORMALIZER`,
  `BROWSER_METRICS`) make the crate one-service-per-process by construction. Three fuzz targets
  (`beacon_payload`, `origin_validation`, `traceparent_validation`) exist but `fuzz.yml` runs
  only DPE's two.
- **Fingerprint:** `59aca6a4b6c2`

### dsp-cli

- **Paths:** `:(glob)dsp-cli/**`
- **Purpose:** `dsp-cli` — an AI-agent-friendly command-line client for the DaSCH Service
  Platform: it talks to a DSP-API server using researcher vocabulary (data-models,
  resource-types, fields) instead of DSP-API's raw RDF/JSON-LD surface. Read-only in v1,
  covering the VRE only; the Deposit, Archive and Access moduliths are future targets, all
  reached over the wire. A root peer of the areas (ADR-0002), not a member of one: no area
  depends on it, and it is a client of every area.
- **Key entities:** `DspClient`, `HttpDspClient`, `Renderer`, `Diagnostic`, `Config`
- **Public interface:** the `dsp` command surface (`auth`; `vre project | data-model |
  resource-type | resource | vocabulary | sparql`; `docs`); the `dsp-cli` crate published on
  crates.io.
- **Local-context kit:** `dsp-cli/CLAUDE.md`, `dsp-cli/CONTEXT.md`, `dsp-cli/src/cli/mod.rs`,
  `dsp-cli/src/client/mod.rs`, `dsp-cli/src/render/mod.rs`, `dsp-cli/src/diagnostic.rs`,
  `dsp-cli/docs/adr/0008-internal-architecture.md`
- **Depends on:** nothing in the workspace; third-party: clap, reqwest, serde, url, insta,
  wiremock
- **Used by:** — (top of the dependency graph; a published binary)
- **Boundary rules:**
  - No dependency on an area crate (**static-analysis**, `cargo publish -p dsp-cli --dry-run`
    in `check.yml`; **structure** after ADR-0001).
  - A `shared-*` dependency only if that crate is itself published to crates.io (same gate).
  - Integrates with an area over its public HTTP surface only, never by Rust import
    (**review**).
  - Live tests never run in the default suite (**static-analysis**,
    `check-live-tests-ignored.sh`).
- **Durable state:** `~/.config/dsp-cli/auth.toml` — single writer, dsp-cli.
- **Fingerprint:** `2b72a643c52f`

### areas/archive (Spycherli)

- **status: planned**
- **Paths:** `:(glob)areas/archive/**` (today only `areas/archive/CONTEXT.md`)
- **Purpose:** The Archive Area — the OAIS archive of the platform, working name Spycherli:
  the intent-protocol edge producers submit to, the validation workers, a leader-elected
  coordinator over a NATS JetStream hot log, and the sealed, URN-keyed store on two S3
  replicas plus tape. Feeds every Access-Area read side with NATS pointers plus immutable S3
  payloads. Nothing is implemented here yet; the vocabulary and the boundary commitments are
  in `areas/archive/CONTEXT.md`.
- **Key entities:** (design vocabulary) `Resource`, `Representation`, `Deposition`,
  `DepositAgreement`, `PreservationAction`, `AccessPolicy`, `Ingest Intent`, `Preservation File`,
  `Service File`, `ARK`
- **Public interface:** the intent protocol (`RegisterIngestIntent`, `CompleteUpload` over
  HTTPS + mTLS, presigned S3 upload); the read-side notification contract (NATS pointers,
  S3 snapshots and deltas); the internal `CommandAPI` for preservation admin tooling.
- **Local-context kit:** `areas/archive/CONTEXT.md`, `CONTEXT.md`,
  `docs/adr/0002-areas-at-the-repository-root.md`, `docs/adr/0003-one-modulith-per-area.md`,
  `shared/metadata/src/lib.rs`
- **Depends on:** shared/metadata (expected, for the contract at the SIP boundary);
  otherwise nothing in this repository
- **Used by:** areas/deposit/editor (target: submits SIPs over the intent protocol), the Access Area
  services (target: consume the notification stream)
- **Boundary rules:** Preservation storage is exclusive to this area — no other component
  reaches into the sealed store, the log, or Preservation File bytes; internal producers get no
  shortcut past the intent protocol; DAO is this area's language and appears only at its
  boundaries (`areas/archive/CONTEXT.md` → Boundary commitments; **docs-only** until code exists, then
  **structure** via Bazel visibility).
- **Durable state:** the sealed store, the hot log, the ingest (quarantine) and Access buckets —
  **single writer:** this area's coordinator and workers.
- **Fingerprint:** none

### vitrinli

- **status: planned**
- **Paths:** `:(glob)vitrinli/**` (today only `vitrinli/CONTEXT.md`; the code is sipi, maintained
  separately until it moves in)
- **Purpose:** Vitrinli — the media engine library, sipi under its new name and mid-way through
  a C++ to Rust rewrite: IIIF Image API rendering, range-served downloads, Service File
  derivation. Neither a service nor a capability (ADR-0003): no routes, no tables, no
  authentication; everything area-specific arrives through the traits its interface accepts.
  A `media` capability in each area depends on it — the Deposit Area's over uploaded
  Originals (and has it derive Service Files for previews), the Access Area's over the
  Service Files the archive produced — which is why it is a root peer and not an area member
  (ADR-0002).
- **Key entities:** (design vocabulary) Media delivery, Bitstream delivery, Derivation; the
  traits Byte source, Authorisation check, Derivation sink; `Original`, `Service File`
- **Public interface:** the library's Rust API — the three roles and the traits they take. No
  HTTP surface of its own; each area's `media` capability owns the routes.
- **Local-context kit:** `vitrinli/CONTEXT.md`, `docs/adr/0001-bazel-builds-the-monorepo.md`,
  `docs/adr/0002-areas-at-the-repository-root.md`, `CONTEXT.md` (the file vocabulary under
  Shared), `areas/archive/CONTEXT.md` (where Service Files come from),
  `shared/telemetry/src/lib.rs` (the beacon collector it may mount)
- **Depends on:** nothing in this repository is expected beyond `shared-*` crates
- **Used by:** the `media` capability of the Deposit Area modulith (today's areas/deposit/editor
  area, no entry yet) and areas/access/media, both planned; each implements Vitrinli's traits
  over its own store and owns the routes
- **Boundary rules:** depends on no area crate and knows no area's session, rights or storage
  (**structure** via Bazel visibility once it arrives); every trait has one implementation per
  area's `media`, so each seam is real; the Access Area's `media` feeds it only archive-made
  Service Files from the Access bucket, the Deposit Area's only Originals it holds; neither
  path reads a Preservation File (`vitrinli/CONTEXT.md` → Boundary commitments; **docs-only**
  until code exists).
- **Durable state:** none. What is servable and where is a `media` capability's tables, in its
  area's database — **single writer:** that area's `media` (target design).
- **Fingerprint:** none

### chischtli

- **status: planned**
- **Paths:** `:(glob)chischtli/**` (today only `chischtli/CONTEXT.md`; the engine is designed and
  built separately until it moves in)
- **Purpose:** Chischtli — the triplestore engine library, the in-house replacement for Fuseki:
  an embedded RDF store with graph-granular write dispatch, exact update deltas, full-text
  search and optional SHACL validation on update. Neither a service nor a capability
  (ADR-0003): no routes, no authentication, no opinion about which graphs exist. Its unit of
  ownership is the named graph; each graph has exactly one writing capability. One instance per
  area's modulith — the Access Area's holds the archive projection written by `sync` and the
  settings graphs written by `profile`; the Deposit Area's holds the working graphs of data
  creation — which is why it is a root peer and not an area member (ADR-0002).
- **Key entities:** (design vocabulary) Named graph, Owning capability, Update, Query,
  Full-text index, Shape validation, Projection, Working store
- **Public interface:** the library's Rust API — open an instance, dispatch an Update to a named
  graph, run a Query, maintain a Full-text index, validate against shapes. No HTTP surface of its
  own; the owning capabilities expose what may be read through their ports.
- **Local-context kit:** `chischtli/CONTEXT.md`, `docs/adr/0002-areas-at-the-repository-root.md`,
  `docs/adr/0003-one-modulith-per-area.md`, `CONTEXT.md`, `areas/archive/CONTEXT.md` (the data
  products `sync` rebuilds from), `areas/access/CONTEXT.md` (the reading side today)
- **Depends on:** nothing in this repository is expected beyond `shared-*` crates
- **Used by:** the Access Area modulith — `sync` (areas/access/sync; writer of the
  archive projection once it moves onto Chischtli), `profile`
  (writer of its settings graphs), with DPE, CPE and the SPARQL endpoint reading through
  `sync`'s ports; the Deposit Area modulith — the data-creation capability (writer of the working
  graphs); all planned
- **Boundary rules:** depends on no area crate and knows no area's session, rights or graph
  names (**structure** via Bazel visibility once it arrives); every named graph has one writing
  capability and non-owners read only through that owner's ports, never by opening the store
  (**review**, then **structure** once the store handle is visible only to owning capabilities);
  the Access Area's projection is written by `sync` alone and only from the archive's data
  products (`chischtli/CONTEXT.md` → Boundary commitments; **docs-only** until code exists).
- **Durable state:** the named graphs of each instance — **single writer per graph:** its owning
  capability (`sync`, `profile`, data creation); the Access-Area projection is disposable and
  rebuilt from snapshot plus replay, never repaired in place (target design).
- **Fingerprint:** none

### areas/access/cpe

- **Paths:** `:(glob)areas/access/cpe/**` — today `ports/` (crate `cpe-ports`), `core/` (crate
  `cpe-core`), `components/` (crate `cpe-components`, with its stylesheets in `components/style/`),
  `image-viewer/` (crate `cpe-image-viewer`), `store/` (crate `cpe-store`, with its schema in
  `store/schema.sql`), `store-build/` (crate `cpe-store-build`), `web/` (crate `cpe-web`), `server/` (crate `cpe-server`), `projects/` (one folder per project,
  `projects/incunabula/` so far), `public/` (CPE's shared static files, with the vendored `datastar.js` in
  `public/vendor/`) and `CONTEXT.md` (CPE's vocabulary and what its port serves)
- **Purpose:** CPE, the Configurable Presentation Environment — a second Access-Area
  hypermedia server rendering project-specific presentations over the same data as DPE,
  configured per project. The project-wide configuration is in the project's folder, its KDL
  (ADR-0007); per-resource curation comes through the port, beside the archive's facts and
  never as one (ADR-0010, proposed). `ports/`, the domain crate `core/`, the component library
  `components/`, the image viewer `image-viewer/`, the read-side store `store/`, its builder `store-build/`, the page assembly `web/`
  and the host crate `server/` are here, and areas/access/server mounts the host crate's router
  under `CPE_BASE`. The crates moved in from `dsp-incubator` one by one, in dependency order (ADR-0007, 2026-10-02 and 2026-10-11
  amendments; ADR-0011). `cpe-image-viewer` is the `<image-viewer>` web component on a vendored
  OpenSeadragon with its Maud builder: an interim, cut-down copy of `dsp-incubator
  shared/dasch-image-viewer` that the incubator's final viewer replaces. `cpe-components` renders what `cpe-core`'s IR describes, as
  Maud functions; its seven stylesheets are plain files that the crate neither embeds nor serves: `cpe-server` does. `cpe-store` is the derived read model (ADR-0008):
  one in-memory SQLite database per project, opened from SQL text over `schema.sql`, and the query layer
  over it. `cpe-store-build` is the remodelling (ADR-0007): a pure library that turns a project snapshot and
  the `remodel {}` block of the project's KDL into the store's rows, with the project's hook where the block
  declares one, and renders them as that SQL text. `cpe-web` is the page assembly: `Project`, a project loaded
  from its KDL, its store's SQL text and its narrative, and the functions that render a page, the compound
  viewer's fragment, the overview of the projects and the not-found pages from `cpe-components`' parts; it
  holds no routes. `public/` holds
  the files every CPE page refers to (two logos, the fingerprint, Datastar). `cpe-server` is the host crate, a
  facade in the shape of `dpe-server`: `CpeConfig`, and `Cpe` with its router and its activation. The router's
  root, which is the base with and without a trailing slash, is the overview of the loaded projects (ADR-0007,
  amendment of 2026-10-12 on the overview). Activation
  loads every project folder with a store built from the port's snapshot, and a project that does not
  activate answers 503 on its own prefix while the others are served (ADR-0007, amendment of 2026-10-11 on
  activation); the crate also serves the stylesheets and `public/` from a compiled-in table. A project is a
  folder under `projects/`: its KDL, its
  narrative and its assets; a hook is not in the folder but a module of the builder (ADR-0007, amendment of
  2026-10-11, DEV-7465).
- **Key entities:** `ArchiveProjection`, `ProjectSnapshot`, `Resource`, `Annotation`,
  `Motivation`, `DataArk`, `DATA_ARK_PREFIX`, `CuratedValue`, `FakeArchiveProjection`,
  `contract::violations`, `contract::is_curation_name` (in `cpe-ports`); curation is
  `ProjectSnapshot.curation`, whose keys the port gives no meaning and whose shape the contract
  checks (`Violation::DanglingCuration`, `Violation::DuplicateCuration`,
  `Violation::MalformedCuration`); an annotation (Region, LinkObj) is an ordinary `Resource`, found by
  `Resource.annotation`, never by `class`; every `Resource` carries its plain data ARK, whose
  shape the contract checks (`Violation::MalformedDataArk`) and whose derivation is the
  adapter's (`sync-store`'s `ark.rs`). In `cpe-core`: the KDL config language
  (`validate::load_config`, `validate::load_remodel`, `ConfigLoadError`, `ConfigError`),
  its validation against a project's facts (`ValidationContext`, `validate::facts::ProjectFacts`),
  the IR a valid configuration lowers to (`ir::SiteConfig`, `ir::Component`, `remodel::Remodel`),
  CPE's presentation types (`ir::Resource`, `ir::Property`, which are not `cpe_ports::Resource`) and
  the diagnostics a broken one produces (`Diagnostic`, `DiagnosticKind`, `ValidationReport`), and the URLs
  CPE writes (`url::Base`, `url::ProjectUrls`, `url::StaticDir`). In
  `cpe-image-viewer`: `Viewer` (the builder that renders the element), `Auth`, `asset` (an embedded
  file by request path, traversal-proof), `head_tag`, `ASSET_PREFIX`. In
  `cpe-components`: the component modules under `component` (`editorial`, `narrative`, `markdown`,
  `resource_list`, `facet_panel`, `search_bar`, `resource_detail`, `media`, `compound_viewer`, `annotation`,
  `data_links`, `breadcrumb`, `values`, `links`, with `context::RenderContext` as what a renderer is given),
  `chrome`'s message catalogue (`chrome::ChromeMsg`, the interface's own strings in every language) and the
  per-project components under `component::project`, found through the `(project, name)` registry
  `PROJECT_RENDERERS` (private to the crate). In `cpe-store`: `db::ProjectStore` (one project's store and the
  owner of its connection: `open` takes SQL text, `facts`, `build_search_indexes`, `data_arks` and `reads` are
  what a caller does with it), `DbError`, `query::LiveProjectFacts` (the store's `ProjectFacts`),
  `query::reads::ProjectReads` (the composite reads), `query::page::page_data` (a request's `PageData`),
  `query::search_index_name`, `QueryError`. In `cpe-store-build`: `build` (the one
  entry), `store::Store` and `store::Table` (the built rows), `sql::to_sql` and `sql::header` (the SQL text),
  `SCHEMA_SQL`, `error::BuildErrors` and `error::Rule` (every refusal has a rule), `hook::Hook` and
  `hook::Draft` (the seam a hook works through), `hooks::HOOKS` and `hooks::resolve` (the table of hooks, by
  project id, the name of the project's folder). In `cpe-web`: `project::Project` (the loaded project; `Project::load`, `page_data`,
  `get_resource`) and `project::LoadError`, `page::render_page`, `page::render_compound_fragment` and
  `page::CompoundFragmentOutput`, `page::render_site_index` and `page::ProjectTeaser` (the overview and one
  project on it, from `Project::teaser`), `page::render_not_found_project`, `page::render_not_found_site`,
  `page::render_browse_breadcrumb_for_page`, `page::SearchFragmentSurface`, `page::document_head`,
  `share_link::compose_canonical_query`, `share_link::compose_absolute_url`, `share_link::ViewSurface`. In
  `cpe-server`: `CpeConfig` (`CPE_PROJECTS_DIR`, `CPE_PUBLIC_ORIGIN`, `CPE_BASE`), `Cpe` (`new`, `activate`, `router`,
  `base`, `projects`, and the doc-hidden `from_sql_folders`, which ends with DEV-7498), `ProjectSummary` and
  `MatchedRoute` (the route template a response carries in its extensions, for the host's request span)
- **Public interface:** `cpe-server`'s `Cpe` and `CpeConfig`: the router, mounted by areas/access/server
  at `Cpe::base()` on the area's origin beside DPE's routes, and `Cpe::activate`, which takes the port. No `pub` item of
  `cpe-server` names a type of another CPE crate, `cpe-ports`' excepted. `cpe/ports` is not an interface CPE offers but the ports CPE
  consumes: today `ArchiveProjection`, which `sync` implements. `pub` in `cpe-core` and in `cpe-components` means visible
  to CPE's other crates, not to a host: no crate of this repository outside the capability depends on either. The
  same holds for `cpe-store`, whose `pub` surface names no `rusqlite` type, and for
  `cpe-web`, whose `pub` items are what `cpe-server` calls.
- **Local-context kit:** `areas/access/cpe/CONTEXT.md`, `areas/access/cpe/ports/src/lib.rs`,
  `areas/access/cpe/core/src/lib.rs`, `areas/access/cpe/components/src/lib.rs`,
  `areas/access/cpe/image-viewer/src/lib.rs`, `areas/access/cpe/store/src/lib.rs`,
  `areas/access/cpe/server/src/lib.rs`, `areas/access/cpe/server/src/activation.rs`,
  `areas/access/cpe/ports/src/snapshot.rs`, `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`,
  `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`,
  `docs/adr/0010-the-port-also-serves-per-resource-curation.md`,
  `docs/adr/0002-areas-at-the-repository-root.md`, `areas/access/CONTEXT.md`,
  `areas/access/dpe/server/src/lib.rs` (the `Dpe` surface to copy), `areas/access/server/src/serve.rs`
  (where it is mounted), `shared/metadata/src/lib.rs`,
  `docs/src/mosaic/component-api-conventions.md`, `docs/adr/0003-one-modulith-per-area.md`
- **Depends on:** `cpe-ports` depends on `std` alone. `cpe-core` depends on `kdl`,
  `pulldown-cmark`, `percent-encoding` and `thiserror`, and on no CPE crate, `cpe-ports`
  included. `cpe-image-viewer` depends on `maud` alone and on no CPE crate. `cpe-components` depends on
  `cpe-core` and `cpe-image-viewer`, and on `maud`, `pulldown-cmark` and `tracing`. `cpe-store` depends on
  `cpe-core`, and on `rusqlite`, `thiserror` and `tracing`. `cpe-store-build` depends on `cpe-core` and
  `cpe-ports`, and on `serde_json`, `thiserror` and `unicode-normalization`; it does not depend on `cpe-store`
  and reads `store/schema.sql` with `include_str!`; its tests alone depend on `sync-store` and `rusqlite`.
  `cpe-web` depends on `cpe-core`, `cpe-store`, `cpe-components` and `cpe-image-viewer`, and on `maud`
  and `thiserror`; its tests read `components/style/` and `public/vendor/datastar.js` with
  `include_str!`. `cpe-server` depends on `cpe-core`, `cpe-ports`, `cpe-store-build`, `cpe-web`, `cpe-components`
  and `cpe-image-viewer` (on `cpe-store` through `cpe-web` only; **static-analysis**: `check-cpe-seam.sh`'s
  `CPE_EDGES` lists the crate), and on `axum`, `tower`, `datastar`, `figment`,
  `maud`, `serde`, `futures-util`, `percent-encoding`, `thiserror` and `tracing`; it reads `components/style/` and
  `public/` with `include_bytes!`; its tests alone depend on `sync-store`, `sha2`, `tempfile` and `tokio`,
  and on `figment`'s `test` feature.
  No CPE crate depends on
  modules/mosaic (ADR-0007). The crates still to come: shared/metadata, shared/telemetry (expected); never
  DPE's domain, store or web crates, and never `sync`'s store or Chischtli (ADR-0008), a dev-dependency on
  `sync-store` excepted (ADR-0008, amendment of 2026-10-11). The first `cpe/ports` edge is CPE's port
  against areas/access/sync: CPE declares `ArchiveProjection` and `sync-store` implements it.
  Ports between CPE and DPE remain hypothetical, each with its own consumer (ADR-0003): what CPE needs from DPE,
  CPE declares in `cpe/ports` and DPE implements; what DPE needs from CPE, DPE declares in
  `dpe/ports` and CPE implements, depending on `dpe/ports` alone. A *concept* both need lives
  in `shared/`; a shape never does
- **Used by:** areas/access/sync (`sync-store`, the provider, depending on `cpe-ports` to
  implement `ArchiveProjection` as `LiveArchiveProjection`); `cpe-components` is the consumer of `cpe-core` and of `cpe-image-viewer`, and `cpe-web` and `cpe-server` are its consumers; `cpe-store` is a consumer of `cpe-core`, and `cpe-web` is its consumer; `cpe-web` is a consumer of `cpe-core`, `cpe-store`, `cpe-components` and `cpe-image-viewer`, and `cpe-server` is its consumer; `cpe-store-build` is a consumer of `cpe-core` and `cpe-ports`, and `cpe-server` is its consumer; areas/access/server (`cpe-server`, mounted beside DPE, and `cpe-ports` for the trait of the adapter it constructs from `sync-store`; `check-composition-root-deps.sh` forbids `core`/`web`/`api-*`, and `check-cpe-seam.sh` forbids every CPE crate but `cpe-ports` and `cpe-server`, so `cpe-store` too)
- **Boundary rules:** a capability of the Access Area's modulith (ADR-0003) — its own tables,
  ports as described under Depends on, no shared shape; and the platform-wide style commitment
  in `## Conventions`, a hypermedia server serving the browser directly, no BFF, no SPA.
  That `cpe/ports` reaches only `std` and `shared-*` is **review** until Bazel `visibility`
  (ADR-0003). The dependency seam is
  **static-analysis** (`.github/scripts/check-cpe-seam.sh`, run by `just check`; ADR-0007, amendment of
  2026-10-11, and of 2026-10-12 for the exception list): no crate of this repository outside
  `areas/access/cpe/` depends on a CPE crate other than `cpe-ports` and `cpe-server`; no manifest under `areas/access/cpe/` depends on a Mosaic crate; `cpe-core`, `cpe-ports`
  and `cpe-image-viewer` depend on no CPE crate; no manifest under `areas/access/cpe/` names a crate of
  `areas/access/sync/` outside a dev-dependencies table (rule 4; ADR-0008, amendment of 2026-10-11); and
  `cpe-store` depends on no CPE crate but `cpe-core`, `cpe-store-build` on none but `cpe-core` and `cpe-ports`,
  `cpe-server` on every CPE crate but `cpe-store`
  (rule 5, the script's `CPE_EDGES`; ADR-0007, amendments of 2026-10-11, DEV-7465, and of 2026-10-12). A rename
  (`package = "…"`) of a CPE, Mosaic or `sync` crate in the workspace
  root is reported where it is declared, whichever crate inherits it. The
  first, second and fourth rules cover a new crate by its
  `[package]` name, without an edit; a new crate that must stay a leaf has to be added to the script's
  `LEAVES`, one whose CPE dependencies are to be held to its `CPE_EDGES`, and a crate another capability or
  the composition root may depend on has to be added to its
  `PORT_EXCEPTIONS` first. What the script does not check is **review**: that no CPE crate depends on DPE's
  domain, store or web crates, which CPE crates a crate outside `LEAVES` and `CPE_EDGES` depends on, `cpe-components`' own dependency list
  (no `axum`, no `rusqlite`), `cpe-store`'s (no `maud`, no `axum`), and `cpe-web`'s (no `axum`: the crate
  holds no routes).
  `cpe-store`'s `ProjectStore` owns a project's connection in a `Mutex`, and no `pub` item hands out the
  connection or a guard, so every lock is taken inside the crate and none can be held across an `.await`
  (**structure** for a caller outside the crate: the field and `ProjectStore::lock` are not `pub`; that neither
  is widened and no `pub` function returns a guard is **review**). Inside the crate a lock is short-scoped
  and a read never calls a read while it holds one (**review**). Among CPE's `[dependencies]` tables `rusqlite`
  is named by `cpe-store` alone; `cpe-store-build` names it under `[dev-dependencies]` (**review**), and the
  driver's error reaches a caller only inside `cpe-store`'s wrapper, which has no source and names no driver
  type (**static-analysis**: two tests in `store/src/db.rs`, over a failed load and a failed query). Debt: a read runs on the async worker thread that
  serves the request, as in the prototype, and a project's requests wait on one `Mutex`; moving the reads to
  `spawn_blocking` or to `deadpool-sqlite`, as the editor does, changes concurrency and is a follow-up. The
  pages `cpe-web` renders are pinned by its 30
  snapshots, and every validation rule by a fixture project under `web/tests/projects/`
  (**static-analysis**). Every URL CPE writes is built by `cpe_core::url` from the base, `CPE_BASE`, which
  `Cpe::base()` returns and a host mounts the router at; `cpe-server`'s `base_tests` crawl Incunabula and the
  fixture projects under `/cpe` and fail on a URL outside it on a page they reach (**static-analysis**); that a
  URL is built through that module at all, and not joined by hand, is **review**. `public/vendor/datastar.js` is checked against its README by `just verify-checksums`
  (**static-analysis**); the three SVG files in `public/` are DaSCH's own and carry no checksum.
  `cpe-components` splices rendered Markdown and a few constants through `PreEscaped` (**review**; DPE's
  three-site rule does not apply here). `cpe-store-build`'s library reads nothing but what it is given, and its generic code names no project
  (**static-analysis**: the source searches of its `tests/it/hygiene.rs`. The second holds for Incunabula
  alone: the search is for the hand-kept list `PROJECT_NOUNS`, and nothing checks that a second project's
  nouns are added to it, **review**); a hook and the `hook` node of its
  project's block are held to each other by `build` (**run-time**) and by `tests/it/hooks.rs`
  (**static-analysis**). The SQL text it renders for Incunabula equals the prototype's committed `data.sql`
  byte for byte, header included (**static-analysis**: that file is not here, and a test of `cpe-server` holds the
  text to its length and SHA-256). In `cpe-server`: no `pub` item names a type of another CPE crate but
  `cpe-ports` (**static-analysis**: the crate's test `no_pub_item_names_another_cpe_crate`, a text check over
  `lib.rs` and `config.rs`); CPE never stops its host, so no constructor fails, `activate` returns nothing and
  catches a panic, and what did not load answers 503 with `Retry-After` and `Cache-Control:
  no-store`, never 404 (**static-analysis**: the tests of its `activation` module, which hold a panic inside
  one project's load; a panic outside it is caught too, and that is **review**, as is a binary built with
  `panic = "abort"`); no request path is joined
  onto a directory (structural where a request is answered, since `AssetInventory::serve` and
  `static_files::serve` are given strings; **review** in `discovery`, the one module that reads the disk);
  `sync-store` is a dev-dependency, so the crate's own code cannot name the adapter (**static-analysis**:
  `check-cpe-seam.sh`, rule 4); that a host feeds a store through `Cpe::activate` and the port, and not through
  the doc-hidden `Cpe::from_sql_folders` (a committed `data.sql`, kept for the incubator's host and for tests
  until DEV-7498), is **review**; a request's origin, where none is configured, is taken from `Host` and
  `X-Forwarded-Proto` under the rule the configured origin is held to (**static-analysis**: its tests). `cpe-core` is
  framework-free: that its manifest and sources name no `maud`, `axum` or `rusqlite` is **review**
  here, and is checked by the incubator's `just cpe crate-deps-check` at each pin move.
  That `cpe-image-viewer` depends on `maud` alone is **review**. Its vendored OpenSeadragon files
  are checked against `areas/access/cpe/image-viewer/assets/vendor/VENDOR.lock` by hand, not by
  `just verify-checksums` (**review**); a file under its `assets/` without a row in its `ASSETS`
  table fails the crate's tests (**static-analysis**).
  The markup `cpe-components` renders is pinned by its snapshot tests (**static-analysis**); that a
  stylesheet in `components/style/` still matches that markup is **review**, with no browser check here.
  `sync-store` also runs
  `contract::violations` on every snapshot it serves, so a new contract invariant can take a
  project offline. No `cpe-ports` code gives a curation key a meaning: a key's rules belong to
  the project's KDL and hook (ADR-0010, proposed; **review**).
- **Durable state:** a read model behind its port against `sync` (`ArchiveProjection`),
  single writer CPE, rebuilt from empty from the project snapshots the port serves (ADR-0008;
  the schema and the queries are in `cpe-store` and what builds a store from a snapshot in `cpe-store-build`).
  The store is built at every start and kept nowhere: `cpe-server`'s `Cpe::activate` builds each project's in
  memory, **single writer** that call (or, until DEV-7498, `Cpe::from_sql_folders` in its place, never both
  in one host), and nothing is written to disk. `projects/incunabula/` is
  committed configuration, narrative and assets, **single writer** the project's developers (**review**); the process reads it at activation and never writes it. `cpe-ports`, `cpe-core`,
  `cpe-components`, `cpe-image-viewer` and `cpe-store-build` hold none; `cpe-web`'s `Project` holds a project's store in memory, and `cpe-server` keeps the loaded projects until the next activation replaces them.
- **Fingerprint:** `3481ca0e10d9`

### areas/access/datei

- **status: planned**
- **Paths:** `:(glob)areas/access/datei/**` (today only `areas/access/datei/CONTEXT.md`; the code is
  `dsp-incubator/da-tei`, of which only the presenter and the rebuild move)
- **Purpose:** DaTEI, the Access Area's third capability (ADR-0009): a hypermedia server presenting
  TEI-based digital editions, one project per route prefix, used instead of a CPE presentation where
  an edition project wants a custom one. Its store is a derived read model (ADR-0008), rebuilt from
  empty at project activation from the archived files its port serves plus the project's TOML
  configuration. First projects: MSSL (086A), then eHKKA (083C). The URL layout and the component
  library are open.
- **Key entities:** (design vocabulary, `areas/access/datei/CONTEXT.md`) Presenter, Rebuild, Read
  model, Configuration, Edition file, ODD / Project CSS, Driving file, Table of contents; in the
  incubator `tei-serve`, `ir::ingest`, `config_doc`, `viewer`
- **Public interface:** its router (planned), mounted by areas/access/server on the area's origin.
  `datei/ports` is not an interface DaTEI offers but the port it consumes: the files of one project
  with their bytes, which areas/access/media implements.
- **Local-context kit:** `areas/access/datei/CONTEXT.md`,
  `docs/adr/0009-datei-joins-the-access-area-as-its-third-capability.md`,
  `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`,
  `docs/adr/0003-one-modulith-per-area.md`, `areas/access/cpe/ports/src/lib.rs` (the port shape to
  copy), `areas/access/server/src/serve.rs` (where it is mounted), `areas/access/CONTEXT.md`
- **Depends on:** areas/access/media through `datei/ports` (DaTEI declares the port, `media`
  implements it); shared/telemetry (expected); never DPE's, CPE's or `sync`'s crates, never
  Chischtli. Third-party, from the incubator: axum, maud, datastar, rusqlite (reconciled with the
  workspace's one pin on entry), quick-xml, roxmltree, toml
- **Used by:** areas/access/server (expected: `datei-server` mounted beside DPE and CPE;
  `datei/ports` only where the root constructs the `Live<Port>` adapter)
- **Boundary rules:** a capability of the Access Area's modulith (ADR-0003): its own tables, one
  consumer-defined port, no shared shape, and the platform-wide style commitment in `## Conventions`.
  The read model is fed by the port and the configuration only: nothing under `areas/access/datei/`
  is an Edition file, ODD, CSS, driving file or table of contents. The presenter never writes the
  read model. A project fails alone (ADR-0007). The incubator's `<cpe-{name}>` element prefix and
  its `ingest` identifier are renamed on entry (**docs-only** until code exists; then
  **static-analysis** for one rebuild-from-empty test against a `Fake<Port>`, and **structure** via
  Bazel visibility after ADR-0001).
- **Durable state:** one read model per project (SQLite), single writer the Rebuild, disposable and
  rebuilt from empty (ADR-0008, ADR-0009). Nothing committed.
- **Fingerprint:** none

### areas/access/sync

- **Paths:** `:(glob)areas/access/sync/**` — today `store/` (crate `sync-store`), `data/` (the
  committed snapshots, the committed curation files, today `0803-curation.csv`, the test oracle
  `0803-arks.txt` and their `PROVENANCE`) and `CONTEXT.md`
- **Purpose:** The Access Area's `sync` capability: the single writer of the archive projection
  and the single consumer of the archive's projection stream, rebuilding the projection from
  snapshot plus replay (ADR-0003, ADR-0008); the first Access-Area capability without a screen
  (ADR-0007). It exists at its minimum (DEV-7399): `sync-store` serves the committed snapshot of
  one known project (0803, Incunabula) behind CPE's port, with no Chischtli, replay or bus;
  `access-server` constructs it over `SYNC_DATA_DIR` and hands it to CPE's activation. Beside the snapshot it serves the project's per-resource
  curation from a hand-authored file, `data/0803-curation.csv`, as an interim (ADR-0010,
  proposed): a missing or faulty curation file refuses the whole project. Annotations (Regions, LinkObjs) are served as resources
  with their geometry and color verbatim. Each resource's data ARK is derived from its IRI with
  dsp-api's algorithm (`ark.rs`), not read from the file, until DAO carries it; the incubator's
  ARKs for 0803 are committed as `data/0803-arks.txt` and pinned by the committed-file tests.
  The snapshot is interim-DAO N-Quads written by `dao-lift`
  (ADR-0007, DEV-7399 amendment), read with a strict `oxttl` parse. DPE keeps reading its corpus
  directly; moving it onto `sync` is its own decision.
- **Key entities:** `LiveArchiveProjection`, `KNOWN`, `SnapshotError`, `InvalidFact`,
  `ArkError`, `InvalidCuration`, `CurationFault` (in `sync-store`; a resource IRI with no data
  ARK is `InvalidFact::NoDataArk`; a curation file that breaks its format is
  `SnapshotError::Curation`, a `CurationFault` naming the line and the `InvalidCuration` rule);
  (design vocabulary) Projection (`chischtli/CONTEXT.md`), Data product
  (`areas/archive/CONTEXT.md`)
- **Public interface:** nothing to a browser. The `Live<Port>` adapters for the ports it
  implements: today `LiveArchiveProjection` for CPE's `ArchiveProjection`; later DPE's own port
  when DPE moves onto `sync`, and the query ports the SPARQL endpoint reads through.
- **Local-context kit:** `areas/access/sync/CONTEXT.md`, `areas/access/sync/data/PROVENANCE`
  (the `dao-lift` commit that pins `FORMAT.md`), `areas/access/sync/store/src/lib.rs`,
  `areas/access/sync/store/src/ark.rs` (the one derivation of the data ARK),
  `areas/access/sync/store/src/vocab.rs` (the format's IRIs and the same pin),
  `areas/access/sync/store/src/curation.rs` (the curation file's format and its one reader),
  `docs/adr/0010-the-port-also-serves-per-resource-curation.md`,
  `areas/access/cpe/ports/src/lib.rs` (the port it implements),
  `areas/access/cpe/ports/src/contract.rs` (the contract its output must pass),
  `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`,
  `chischtli/CONTEXT.md`, `areas/archive/CONTEXT.md`, `docs/adr/0003-one-modulith-per-area.md`,
  `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`,
  `areas/access/server/src/serve.rs` (where its adapters are constructed)
- **Depends on:** areas/access/cpe (`cpe-ports` only, to implement `ArchiveProjection`: the one
  permitted capability-to-capability edge, ADR-0003); chischtli (planned; not in the minimum);
  shared/metadata (expected). Never CPE's or DPE's domain, store or web crates. Third-party:
  `oxttl` (the strict N-Quads parser) and `oxrdf` (its terms), with no triplestore at the
  minimum; `thiserror`, `tracing`
- **Used by:** areas/access/cpe (through `cpe-ports`; CPE's store is rebuilt from what `sync`
  serves, ADR-0008; the tests of `cpe-store-build` and of `cpe-server` build `sync-store`'s adapter over the
  committed data, as a dev-dependency, ADR-0008's amendment of 2026-10-11); areas/access/server (constructs `LiveArchiveProjection` over `SYNC_DATA_DIR` and hands it to `Cpe::activate`); later
  areas/access/dpe and the SPARQL endpoint, each through its own port
- **Boundary rules:** single writer of every projection graph and single consumer of the
  projection stream (ADR-0003, ADR-0008; whether `media`'s reading of the Access bucket counts
  against that is DEV-7442); maps its data to each consumer's DTOs, archive-shaped but for
  curation, and holds no presentation remodelling (ADR-0007's guardrail); `sync-store` knows no
  curation key: it serves keys as opaque names and no code outside its tests names one
  (ADR-0010, proposed; **static-analysis** for the committed keys as string literals,
  `test_source_outside_tests_names_no_committed_curation_key`, otherwise **review**); readers reach the projection only through the
  ports it implements, never by opening its store; a reader's tests may build `sync-store`'s adapter, as a
  dev-dependency, and read through the port it implements (ADR-0008, amendment of 2026-10-11; that it is
  a dev-dependency only is **static-analysis** for CPE, rule 4 of `check-cpe-seam.sh`; that the tests read
  through the port is **review**); `sync-store` depends on `cpe-ports` and on no
  other CPE or DPE crate (no other CPE crate: **static-analysis**, `check-cpe-seam.sh` in `just check`;
  no DPE crate: **review**; both **structure** via Bazel visibility after ADR-0001;
  `check-composition-root-deps.sh` does not scan it). A file that breaks a fact the port serves
  is refused whole, never served in part (**static-analysis**: `sync-store`'s tests), and so is
  one whose mapped snapshot fails `cpe_ports::contract::violations`, run on every call; the
  contract's cross-fact rules (inverted date, cycles, sibling positions) live only there, never
  in the mapping (**review**; the crate doc in `lib.rs` says where a check goes). A link to an IRI
  outside the file is omitted, but an annotation target that is not a resource of the file is
  refused (`InvalidFact::UnknownTarget`). A curation file is read strictly and never repaired:
  a row for an IRI the snapshot does not hold, a padded value or a missing file refuses the
  project (**static-analysis**: `sync-store`'s tests).
- **Durable state:** the archive projection, **single writer** `sync`; disposable, rebuilt from
  snapshot plus replay, never repaired in place (target design). Today the committed snapshots
  under `areas/access/sync/data/` (`0803.nq`), **single writer** `dao-lift`, run on a VRE dump at
  the commit `PROVENANCE` pins and committed unedited (**review**); `sync-store` only reads them,
  on every call, and holds nothing in between. The committed curation `0803-curation.csv`,
  **single writer** a person: hand-authored, edited in place and never generated (**review**);
  since dsp-incubator#500 (DEV-7496) it is the only copy of Incunabula's curation, and a change
  is made here and reaches the incubator when it re-vendors (`PROVENANCE`; **review**);
  read by `sync-store` on every call, and pinned by the committed-file tests: an edit to the
  file changes their expectations in the same commit (**review**). The test oracle `0803-arks.txt`, **single writer**
  the extraction `PROVENANCE` gives over the incubator's `data.sql`, never generated from `ark.rs`
  (**review**), is read only by the committed-file tests.
- **Fingerprint:** `d149ed6eb63a`

### areas/access/media

- **status: planned**
- **Paths:** `:(glob)areas/access/media/**` (nothing yet; created by ADR-0009 ahead of the Access
  bucket)
- **Purpose:** The Access Area's `media` capability (`vitrinli/CONTEXT.md`): serves archive-made
  Service Files, whole or rendered through Vitrinli, and owns the tables that say what is servable
  and where. Starts at its minimum (ADR-0009): the committed files of one shortcode read from disk,
  behind DaTEI's port, the same port the Access bucket will later stand behind.
- **Key entities:** (design vocabulary) Media capability, Access bucket, Byte source,
  Authorisation check (Vitrinli's traits); the `Live<Port>` adapter for `datei/ports`
- **Public interface:** its routes for Service Files (planned: the IIIF and download surface that
  CPE's IIIF URLs and DPE's record files will resolve to); the `Live<Port>` adapters for the ports
  it implements, today DaTEI's.
- **Local-context kit:** `vitrinli/CONTEXT.md`, `areas/access/datei/CONTEXT.md` (its first
  consumer), `docs/adr/0009-datei-joins-the-access-area-as-its-third-capability.md`,
  `docs/adr/0003-one-modulith-per-area.md`, `areas/archive/CONTEXT.md` (where Service Files come
  from), `CONTEXT.md` (the file vocabulary under Shared), `areas/access/server/src/serve.rs`
- **Depends on:** vitrinli (planned); areas/access/datei (`datei/ports` only, to implement its port:
  the one permitted capability-to-capability edge, ADR-0003); shared/telemetry (expected). Never
  `sync`'s store, never Chischtli, never a sibling's domain or web crate
- **Used by:** areas/access/datei (through `datei/ports`), areas/access/server (mounting and adapter
  construction); expected later: areas/access/cpe and areas/access/dpe for IIIF and downloads
- **Boundary rules:** reads only the Access bucket, and until it exists only its own committed data
  directory; never the sealed store, Preservation Files or Originals; never derives, because the
  archive already did; implements consumer-declared ports beside its data and never serves a
  consumer's shape (`vitrinli/CONTEXT.md` → Boundary commitments; **docs-only** until code exists,
  then **structure** via Bazel visibility). Whether it may consume the Access bucket beside `sync`'s
  consumption of the projection stream is DEV-7442.
- **Durable state:** the tables that say what is servable and where, **single writer** `media`
  (target design). Until the Access bucket: the committed files of the first project under
  `areas/access/media/`, plain git, hand-committed with their provenance recorded (ADR-0009).
- **Fingerprint:** none

## Conventions

- **One-way top-level dependency direction:** `services → shared, mosaic, vitrinli, chischtli`;
  `shared-*`, `mosaic-*`, Vitrinli and Chischtli never import a service; a service never
  imports another service. *Enforcement:*
  **structure** for the shared half (a service dependency in a shared crate is a Cargo
  cycle) + **static-analysis** for hardcoded paths (`check-shared-paths.sh`); **review** for
  service→service today (**static-analysis** from a Deposit Area capability or its shell,
  `check-capability-deps.sh`; not from its root),
  **structure** via Bazel visibility after ADR-0001.
- **Areas are bounded contexts (ADR-0002):** the Deposit, Archive and Access Areas integrate
  over the wire, never by Rust import; each service is the sole reader and writer of its
  durable state; cross-area references are opaque identifiers (shortcode, ARK); share
  concepts (a `shared-*` crate), never shapes. *Enforcement:* **review**, becoming
  **structure** with ADR-0001.
- **One modulith per area (ADR-0003):** one binary per area, composed at `<area>/server` out of
  capabilities that own their tables, collaborate only through consumer-defined `ports` crates
  with `Live<Port>` adapters in the provider's store, reference each other by opaque id, and
  never span a transaction. The Access Area's root is `areas/access/server` (moved up ahead of
  CPE, ADR-0007 amendment); the Deposit Area's is `areas/deposit/server` (`areas/deposit/ADR-0004`).
  *Enforcement:* **static-analysis** for the crate edges inside the Deposit Area (`check-capability-deps.sh`,
  `just check`: a capability's crates on its own and `shared-*`, a `store` crate also on another
  capability's `ports`, a `web` crate also on the shell and `mosaic-tiles`, a `ports` crate on
  `shared-*` alone, the shell on `mosaic-tiles` and `shared-*`, `collector` as `core`; among
  third-party crates, a `ports` crate on none, a `core` crate on no web framework, Maud or database
  crate, the shell on no web framework; the root's edges are `check-composition-root-deps.sh`'s). The Access Area is not covered yet
  (`check-cpe-seam.sh` holds CPE's edges; ADR-0003, amendment of 2026-10-13). **review** for the
  rest, becoming **structure** (Bazel
  visibility: `ports` public within the area; domain / store / web visible to the capability
  and the composition root only) with ADR-0001.
- **Shared code lives under `shared/` as `shared-{role}`** the moment a second
  service depends on it; the directory is what CI path filters, `bacon.toml` watch lists and
  directory-scoped `CLAUDE.md` files key on. One crate is there ahead of its second consumer:
  `shared-fair`, because ADR-0005 names the consumers to come. *Enforcement:* **review** (the
  rule), **static-analysis** (the path grep).
- **Crate naming:** `{service}-{role}`; the folder drops the prefix (`dpe/core` is `dpe-core`).
  *Enforcement:* **review**.
- **Composition root:** an area's `server` crate (`areas/<area>/server`) owns the CLI,
  observability and mounting; each capability owns its routes and config; the root depends on a
  capability's `server` crate, and on its `store` and `ports` crates only to construct and inject
  `Live<Port>` adapters (ADR-0003), never on its `core` or `api-*` crates, and on its `web`
  crate only when the capability has no `server` crate (the Deposit Area mounts `identity` and
  `editor` through their web facades, `areas/deposit/ADR-0004`; DPE and CPE keep theirs); views are
  `fn(...) -> Markup` in the `web` crate; the domain / contract crate has no framework
  dependency. *Enforcement:* **static-analysis** for the root's dependencies
  (`check-composition-root-deps.sh`) and for a Deposit `core` crate's ({G}), **review** for the rest (DPE's views read the corpus
  handed to them through `RenderContext` rather than taking every value as an argument — a
  recorded exception).
- **Hypermedia, server-authoritative (ADR-0004):** server-rendered HTML with Datastar SSE
  fragments; no SPA, no WASM, no client-side state store, no BFF; every enhanced link keeps a
  working `href`; every state-changing route is `POST`; a page is never rendered differently
  by header. *Enforcement:* **review** + **static-analysis** (`check-datastar-delimiters.sh`;
  the editor's no-JavaScript E2E pass).
- **FAIR landing pages (ADR-0005):** every Access-Area page a persistent identifier resolves to
  embeds standards-shaped metadata in the served HTML, carries FAIR Signposting `Link` headers,
  and offers each machine-readable representation at its own URL, written from one resolved
  graph. *Enforcement:* **static-analysis** + **review**, clause by clause: one graph feeding
  every representation by `every_representation_of_a_committed_object_agrees_with_the_others`
  (`dpe-api-oai`); the headers and the one negotiation step by the five `dpe-server` handler
  tests ADR-0005 names — `a_landing_page_carries_the_metadata_and_the_link_header`,
  `the_page_links_every_representation_it_serves`,
  `every_landing_page_answer_varies_on_accept`,
  `a_harvester_asking_for_a_representation_is_redirected_to_it` and
  `an_unknown_shortcode_never_redirects_whatever_it_was_asked_for`; `shared-fair` holding no
  hardcoded path into a service module by `check-shared-paths.sh` (`just check`), which reads
  each shared crate's `src/` and `testdata/`; and FAIRness being measured at all by
  the `just fair-check` step in `REVIEW.md` (**review**).
- **Data conventions:** project files are canonical (member order = `ProjectRaw` declaration
  order, no `null`, 4-space indent); every `temporalCoverage` resolves. *Enforcement:*
  **static-analysis** (`canonical_round_trip`, `every_committed_temporal_coverage_resolves`,
  `just validate-data`).
- **Colocated docs:** each service module carries a `CLAUDE.md` (agent runbook), except the
  `shared-*` crates, which share `shared/README.md` one directory up; each bounded context, and
  any root-level component with its own vocabulary (Vitrinli, Chischtli), a `CONTEXT.md`;
  authoritative prose lives in `docs/src/`; system-wide decisions in `docs/adr/`, and any
  root-level component with its own decision history keeps its own `docs/adr/` under the
  component directory, with its own sequence from 0001 — a bare `ADR-NNNN` always names a root
  ADR, a component ADR is always cited qualified as `<component>/ADR-NNNN`, also from inside that
  component (ADR-0006). An area is such a component: `areas/<area>/docs/adr/`, cited
  `areas/<area>/ADR-NNNN`, and a capability carries no series of its own (ADR-0006, amendment of
  2026-09-25; `areas/deposit/` is the first).
  *Enforcement:* **docs-only** for the runbook and vocabulary docs; **static-analysis**
  (`check-adr-refs.sh`, `just check`) for the citation rule.
- **Local-context kit budget:** ≤7 files per component. *Enforcement:* **docs-only**.
- **Commits:** `type(scope): subject`, scope = crate name or `dpe-data` / `ci` / `deps` / `docs`;
  one commit per PR. *Enforcement:* **static-analysis** (`just commit-lint`, the `gate` job).

### Banned constructs

| Locally-attractive pattern | Why it couples globally | Supported alternative | Enforcement |
|---|---|---|---|
| A `dpe-*` dependency in an `editor-*` crate (or the reverse) | Turns two deployables with a deliberate origin split into one codebase; the next agent copies the shortcut | Move the shared concept to a `shared-*` crate; pass service-specific data in as a parameter | static-analysis for an `editor-*` crate (`check-capability-deps.sh`), review for the reverse → structure (ADR-0001) |
| A capability importing a sibling capability's domain, store or web crate, or querying its tables | Turns the area's modulith into a tangle; extraction becomes impossible | Declare a port in the consumer's `ports` crate; the provider implements the adapter; wire at `<area>/server` (ADR-0003) | static-analysis in the Deposit Area (`check-capability-deps.sh`, ADR-0003 amendment of 2026-09-25), review in the Access Area → structure (ADR-0001) |
| Opening the area's Chischtli instance to read or write a graph another capability owns | A graph with two writers is a table with two writers; readers past the owner's ports freeze the owner's internal graph layout | Query through the owning capability's ports (`sync` for the archive projection); a new graph gets a new owner, not a second writer (ADR-0003) | review → structure (store handle visible to owners only) |
| A relative path from a `shared-*` crate into `modules/<service>/`, in a source file or a test fixture | Makes the shared crate depend on one service's layout and configuration | Take the directory or table as a parameter (`load_from(data_dir)`); the repo-root `justfile` is what may name a module path | static-analysis (`shared/*/src/*.rs` and `shared/*/testdata/**`) |
| Reading `dpe_core::Project` in the editor | The view model is lossy on `url` and `clusters`; the editor must preserve both | `ProjectRaw` → draft → `ProjectRaw` | review |
| Hardcoding a Mosaic class string (`card card-bordered`, `tooltip`) in a service's markup | Freezes the design system's CSS contract in 19 call sites; a tile rename breaks silently | Call the tile; add the missing tile (`tooltip`) rather than the string | review |
| A `css_class()` that assembles its string at runtime | Tailwind's `source(none)` scan never sees the class; the failure is unstyled markup with no error | Complete literal strings, one per variant | docs-only (promote) |
| A `GET` route that writes, or a Datastar-enhanced control without a no-JS path | Defeats the `Sec-Fetch-Site` CSRF control; strands the no-JavaScript user | `POST` for every write; `formaction` on row controls; the E2E suite's no-JS pass (ADR-0004) | static-analysis (editor E2E) |
| A client-side framework, WASM bundle, client router or BFF for a new screen | Splits UI state between two runtimes; the page stops being citable by URL and readable by a machine without a client | A Maud view in the `web` crate plus a Datastar-enhanced fragment (ADR-0004) | review |
| Rendering a landing page differently by `Accept` | Leaves machine representations without a URL to cite or link; types every error path per media type | A dedicated URL per representation, `describedby` links, a `303` from the page as the only negotiation (ADR-0005) | static-analysis (the four `dpe-server` handler tests that bear on *this* clause — `a_browser_gets_the_page`, `a_harvester_asking_for_a_representation_is_redirected_to_it`, `every_landing_page_answer_varies_on_accept`, `an_unknown_shortcode_never_redirects_whatever_it_was_asked_for`; ADR-0005's full list is in `## Conventions`) |
| A second writer for a corpus file (`records/` from the server at startup) | "Who wrote this?" has no single answer; a tracked directory changes under git | One recipe (`just fetch-records`) as the writer; the server reads only | review (open) |
| A dsp-cli dependency on an area crate | Turns the CLI into a second, out-of-process consumer of code meant to run inside one area's modulith on its own origin | Call the area's public HTTP surface, as any external client does | static-analysis (`cargo publish --dry-run`) → structure (ADR-0001) |

## Cross-cutting concerns

Not code components; they span the repo and are staleness-exempt here:

- **Build, toolchain and dev loop:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
  `.rustfmt.toml`, `deny.toml`, `bacon.toml`, `justfile`, `tailwind.pins`, `flake.nix`,
  `flake.lock`, `.envrc`, `.node-version`, `.gitignore`, `.dockerignore`, `.worktreeinclude`,
  `.commitlintrc.yml`, `.kodus-readiness.yml`
- **CI:** `.github/**` — workflows, composite actions, the gate scripts
  (`check-shared-paths.sh`, `check-datastar-delimiters.sh`, `check-commit-count.sh`,
  `verify-checksums.sh`, `check-adr-refs.sh`, `check-live-tests-ignored.sh`, `check-cpe-seam.sh`, `check-capability-deps.sh`),
  release-please config
- **Documentation:** `docs/**` (the mdBook under `docs/src/`, ADRs under `docs/adr/`), an area's decision records under `areas/<area>/docs/adr/` (`areas/deposit/docs/**` today),
  `README.md`, `CLAUDE.md`, `CONVENTIONS.md`, `REVIEW.md`, `CHANGELOG.md`, `LICENSE`,
  `CONTEXT.md`, `ARCH-MAP.md`, `shared/README.md`
- **Agent configuration:** `.claude/**` (settings, the `add-mosaic-component` skill)
- **Data tooling:** `scripts/**` — the two Python scripts that write DPE's lookup tables
