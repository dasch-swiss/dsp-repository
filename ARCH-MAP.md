---
dune_map: true
schema_version: 1
last_verified_commit: a3a1b0dc015d839ac0e162d92b2b695ca7cc3d3b
date: 2026-09-24
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
another service. The shared root has moved to `shared/` at the repository root and the
metadata editor to `areas/deposit/editor/` (ADR-0002); DPE and Mosaic still live under
`modules/`, and three of the four planned components hold only a `CONTEXT.md` at their target
path (`areas/access/cpe` has no files yet). The rest of ADR-0002 (accepted, migration pending)
moves DPE under `areas/access/` and Mosaic to the root, beside `shared/`, `vitrinli/` and
`chischtli/`, so read the globs here as current state. `dsp-cli/` is a sixth root,
outside the `services → shared, mosaic` arrow entirely (ADR-0002 amendment): it is a client of
every area rather than a member of one, no area depends on it, and it depends on no area
crate. Vocabulary: root [`CONTEXT.md`](CONTEXT.md)
(the context index and the shared contract terms), `areas/deposit/editor/CONTEXT.md`,
`modules/dpe/CONTEXT.md`, `areas/archive/CONTEXT.md`, `vitrinli/CONTEXT.md`,
`chischtli/CONTEXT.md`, `dsp-cli/CONTEXT.md`. Decisions: [`docs/adr/`](docs/adr/).

## Components

### modules/dpe

- **Paths:** `:(glob)modules/dpe/**`
- **Purpose:** The Discovery and Presentation Environment — the Access Area's first service.
  Four crates: `dpe-core` (view model, `OnceLock` caches, filesystem repositories, a
  DSP-API client), `dpe-api-oai` (OAI-PMH 2.0), `dpe-web` (Maud pages and components),
  `dpe-server` (the binary: routes, page shell, Datastar SSE fragments, the `validate`
  CLI). Also owns the published **corpus** under `server/data/` (projects, persons and
  organizations, whose sizes `corpus-manifest.json` records; plus 5 clusters, 3 record
  dumps, two lookup tables) and the cover images under `public/assets/images/`.
- **Key entities:** `Project`, `ProjectQuery`, `VALID_TABS`, `all_projects`,
  `project_by_shortcode`, `cover_image_url`, `ClusterRaw`, `ClusterRef`, `CollectionRef`,
  `ProjectRepository` / `FsProjectRepository`, `RecordRepository` / `FsRecordRepository`,
  `OaiRecord`, `CachedContributorLookup`, `resolve_inputs`, `records_for_shortcode`,
  `project_oai_identifier`, `oai_handler`, `set_base_url`, `build_router`,
  `tab_fragment_handler`, `search_fragment_handler`, `record_file_handler`, `HeadExtras`,
  `landing_page` / `LandingPage` / `render`, `get_data_dir` / `set_data_dir`
- **Public interface:** the HTTP routes of `dpe-server` (`/dpe/projects`, `/dpe/projects/{id}`,
  `/dpe/projects/{id}/metadata.jsonld` and `/dpe/projects/{id}/metadata.datacite.json`,
  `/dpe/projects/{id}/tab/{tab}` and `/dpe/projects/search` as SSE, `/dpe/records/{shortcode}/{record_id}/file`,
  `/dpe/oai`, `/dpe/api/v2/projects[/{id}]`, `/dpe/about`, `/healthz`, `POST /telemetry/collect`); the
  `dpe-server serve | validate <data_dir> | healthcheck <url>` CLI; and the corpus files under
  `server/data/`, which the editor consumes as an image-baked snapshot through `EDITOR_DATA_DIR`.
  No `dpe-*` crate is depended on by any crate outside this component.
- **Local-context kit:** `modules/dpe/CLAUDE.md`, `modules/dpe/server/src/router.rs`,
  `modules/dpe/server/src/shell.rs`, `modules/dpe/core/src/lib.rs`,
  `modules/dpe/core/src/project.rs`, `modules/dpe/server/src/fragments.rs`,
  `modules/dpe/api-oai/src/lib.rs` (the OAI crate's whole surface; the contract it reads is
  shared/metadata's own kit). The kit is at its seven-file budget, so
  `modules/dpe/server/src/serve.rs` is named here rather than added: it holds the order-sensitive
  startup sequence and the two untraced routes.
- **Depends on:** shared/metadata (all four crates), shared/fair (`dpe-api-oai`, `dpe-server`), shared/telemetry
  (`dpe-server`), modules/mosaic (`dpe-web`, `dpe-server`); third-party: axum, tokio, tower /
  tower-http / tower_governor, maud, datastar, clap, figment, serde / serde_json, quick-xml,
  ureq, the OpenTelemetry stack, pyroscope, insta
- **Used by:** areas/deposit/editor — data only, never code: the corpus is copied into the editor image
  (`.github/actions/build-editor/action.yml`, `justfile`) and read by the editor's tests through
  `editor_core::checkout_dpe_data_dir()` (`DPE_DATA_DIR`, the one Rust definition of the path); shared/metadata's and shared/fair's
  committed-data tests live here (`core/src/temporal_enrichment_cache.rs`,
  `api-oai/src/metadata/corpus.rs`); `editor-collector` writes the corpus through a pull request
  (see **Durable state** above), still never through a `dpe-*` import
- **Boundary rules:**
  - Never depends on an `editor-*` crate; the editor never depends on a `dpe-*` crate
    (**review** today; **structure** via Bazel visibility after ADR-0001).
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
  - Page, fragment and API routes live in `server/src/router.rs::build_router`; `serve.rs`
    declares only the two untraced routes, `/healthz` and `POST /telemetry/collect`, after the
    OTel layers on purpose.
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
  In-process `OnceLock` caches load once and never invalidate.

### areas/deposit/editor

- **Paths:** `:(glob)areas/deposit/editor/**`
- **Purpose:** The metadata editor — the Deposit Area's first service. Four crates:
  `editor-core` (the draft model, validation, the canonical project and entity writers, the
  persistence ports), `editor-web` (the document shell, pages, the form's field registry and
  widgets), `editor-server` (the binary: config, auth, routing, the SQLite implementations of
  the ports), `editor-collector` (the CI binary that turns an approved record into a pull
  request against this repository — outside the service's request path, and the only crate here
  that writes anything under `modules/dpe/`).
  Depositors edit their projects section by section; RDU reviews field by field; approve
  writes an approved record, which the collector publishes as a pull request. The Deposit
  Area's target is two capabilities, `identity` and `editor`, under `areas/deposit/server`
  (`areas/deposit/ADR-0004`); the code has one, and `docs/src/editor/architecture.md` lists
  the divergence. The entry below describes the code as it is.
- **Key entities:** `ProjectDraft`, `ProjectState`, `SubmissionState`, `ReviewState`,
  `FieldReview`, `Decision`, `EntityProposal`, `Transition`, `PublishedProjects`, `Agents`,
  `Repositories`, `ReviewRoundRepository`, `RecordClassification`, `classify_record`,
  `registry::FIELDS`, `registry::SECTIONS`, `Shape`,
  `apply`, `write_project`, `write_entity`, `normalize_shortcode`, `Authenticated`, `Rdu`,
  `KNOWN_ROUTES`, `EditorConfig`, `Forge`, `CollectionReport`
- **Public interface:** the HTTP routes of `editor-server`, root-mounted on its own hostname
  (`/login`, `/login/code`, `/logout`, `/projects`, `/projects/{shortcode}`,
  `/projects/{shortcode}/sections/{section}` and its row-action `POST`s,
  `/projects/{shortcode}/entities/{proposal}`, `/review`, `/review/{shortcode}`, `/depositors…`,
  `/collection`, `/collection/{id}/discard`, `/states`, `/healthz`, `POST /telemetry/collect`,
  `GET /api/v1/approved-records`, `POST /api/v1/collection-report`); the
  `editor-server serve | healthcheck` CLI; the `editor-collector collect | refresh` CLI, invoked
  only by `.github/workflows/collect-editor-records.yml`. No crate outside this component depends
  on an `editor-*` crate; `editor-server` exports nothing.
- **Local-context kit:** `areas/deposit/editor/CLAUDE.md`, `areas/deposit/editor/server/src/router.rs`,
  `areas/deposit/editor/web/src/form/registry.rs`, `areas/deposit/editor/core/src/form.rs`,
  `areas/deposit/editor/core/src/status.rs`, `docs/src/editor/collection.md`,
  `docs/src/editor/architecture.md`
- **Depends on:** shared/metadata (all three crates), shared/telemetry
  (`editor-server`), modules/mosaic (`editor-web`); modules/dpe's corpus as data (see above),
  never its code — `editor-collector` additionally *writes* that corpus, through a pull request
  and never at runtime (`docs/src/editor/collection.md`); third-party: axum, maud, rusqlite
  (`bundled`) + deadpool-sqlite, figment, lettre, clap, ureq, rand + subtle, tower_governor,
  reqwest (`editor-collector`), the OpenTelemetry stack, pyroscope, insta
- **Used by:** — (top of the dependency graph; a separate deployable)
- **Boundary rules:**
  - Never depends on a `dpe-*` crate (**review** today; **structure** after ADR-0001).
    `core/src/agents.rs` states why it avoids `dpe-core`'s process-wide caches.
  - `EDITOR_DATA_DIR` has no default on purpose: the only plausible one would be a relative path
    into DPE's tree; unset is a configured state (the PR preview runs without a snapshot), so
    nothing refuses to start (**review**).
  - Separate origin from DPE; every state-changing route is `POST` and passes the
    `Sec-Fetch-Site` same-origin check; a write URL also answers `GET` (**review**; the
    traced/untraced split in `router.rs` is pinned by tests — **static-analysis**).
  - `Authenticated` / `Rdu` are extractors, not middleware: a public handler is public in its
    signature (**structure**).
  - Reader connections are `query_only=ON`; the only write path is `Database::write`, one
    connection, `BEGIN IMMEDIATE` (**structure**).
  - Every contract member is either a registry `Field` or listed in `OMITTED`
    (**static-analysis** — `every_contract_field_is_either_placed_in_the_form_or_deliberately_omitted`).
  - Depositor-facing vocabulary is closed (REQ-2.1 / REQ-2.2): five states, no "export", "JSON",
    "transfer", "commit", "pull request" (**static-analysis** — `depositor_vocabulary.rs` plus the
    E2E pass).
  - One SQLite file holds identity's tables (`users`, `user_shortcodes`, `sessions`,
    `login_codes`, `mail_sends`) beside the editor's, with seven foreign keys into `users`;
    falls short of ADR-0003's data sovereignty and `areas/deposit/ADR-0004`'s one file per
    capability (**review**; **structure** once the files split).
  - `Authenticated` / `Rdu` and the user-name reads in `review.rs` and `sections.rs` call the
    session and user repositories directly; `areas/deposit/ADR-0004` puts them behind
    `editor-ports`, which does not exist yet (**review**).
  - Handlers, routes and the SQLite implementations live in `editor-server`; ADR-0003's anatomy
    puts them in the capability's web crate and an `editor-store` crate, and the composition
    root at `areas/deposit/server` (**review**; no crate-graph gate exists yet — ADR-0003,
    amendment of 2026-09-25).
- **Durable state:** one SQLite database — `users`, `user_shortcodes`, `sessions`, `login_codes`,
  `mail_sends`, `drafts`, `submissions`, `review_rounds`, `approved_records`, `entity_proposals`,
  all `STRICT`, one baseline migration `0001` under `server/src/db/migrations/` (edited in place
  until the first deployment, then forward-only) guarded by `PRAGMA user_version`. **Single writer:**
  `editor-server/src/db/` through `editor-core`'s repository ports; several handlers call the
  same port, serialized by the one writer connection, and every multi-table transition is one
  repository method. `review_rounds` is append-only. `approved_records` is written only by
  approve, and deleted by two paths: the startup reconcile that derives Online, and approve's
  own transaction, which supersedes an earlier record for the same project when no pull request
  of its own is live.

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
- **Used by:** modules/dpe (`dpe-web`, `dpe-server`), areas/deposit/editor (`editor-web`); all three
  Tailwind entries (`modules/dpe/style/main.css`, `areas/deposit/editor/style/main.css`,
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
  `shared/fair/src/project_graph.rs`, `modules/dpe/api-oai/src/metadata/mod.rs` (the OAI call
  site), `modules/dpe/api-oai/src/metadata/corpus.rs` (the corpus-wide tests),
  `shared/README.md`, `docs/adr/0005-fair-landing-pages-in-the-access-area.md`. The kit is at
  its seven-file budget, so the second call site, `modules/dpe/server/src/metadata.rs`, is named
  here rather than added: it is where the landing page's writers are called and the `UrlLayout`
  is built.
- **Depends on:** shared/metadata, and `serde_json` at runtime (the JSON-LD writer's `Value` is
  its output type, and `script_safe_json` serialises it); nothing else in the workspace. No
  other third-party runtime dependency.
- **Used by:** modules/dpe — `dpe-api-oai` (the OAI writers) and `dpe-server` (the landing
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
    (`modules/dpe/api-oai/src/metadata/corpus.rs`), not here (**review**).
- **Durable state:** none. Holds no cache and reads no environment; the contributor lookup and
  the two temporal tables arrive in `ResolveContext`.

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
  `modules/dpe/core/src/project.rs`, `areas/deposit/editor/core/tests/canonical_round_trip.rs`
  (a contract member moves with all four consumer files in one commit)
- **Depends on:** nothing in the workspace; third-party: serde, serde_json (`preserve_order`,
  which the editor's canonical writer requires), tracing
- **Used by:** modules/dpe (all four crates), areas/deposit/editor (all three crates), shared/fair,
  the DPE fuzz crate
- **Boundary rules:**
  - Depends on no service crate (**structure** — Cargo cycle); holds no path into a service
    module (**static-analysis** — `.github/scripts/check-shared-paths.sh`, run by `just check`).
  - Reads no environment: both lookup tables load from a caller-supplied `data_dir`; the caches
    live in `dpe-core`, and `editor-server` loads its own (**review**).
  - Member order in `ProjectRaw` **is** the on-disk order of every committed project file
    (**static-analysis** — `every_committed_project_file_round_trips_byte_identically`).
  - Tests that read committed data live in the consumer that owns the data, not here
    (**review**).
- **Durable state:** none. Loads `chronontology-periods.json` and
  `temporal-coverage-enrichment.json` from whatever directory it is given.

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
  `shared/telemetry/src/collector.rs`, `modules/dpe/public/telemetry.js`,
  `areas/deposit/editor/public/telemetry.js`, `docs/src/dpe/observability.md`,
  `shared/README.md`, `modules/dpe/server/fuzz/fuzz_targets/beacon_payload.rs`
- **Depends on:** nothing in the workspace; third-party: serde, serde_json; axum, opentelemetry,
  tracing, url (collector only). No `tower_governor` — the rate limiter is each server's own.
- **Used by:** modules/dpe (`dpe-server`: `collect_route("dpe", page_url::normalize_page_url)`),
  areas/deposit/editor (`editor-server`: `collect_route("editor", …)`), both servers' `traceparent.rs`,
  the DPE fuzz crate
- **Boundary rules:**
  - Depends on no service crate; holds no path into one (**structure** + **static-analysis**,
    as for `shared-metadata`).
  - Page-URL normalization is each service's own `page_url.rs`, passed in as a function — a
    shared crate cannot hold one service's route table (**structure** — the argument is
    required).
  - `namespace` is a required argument so the instrumentation scope dashboards filter on
    (`dpe.browser`, `editor.browser`) cannot be omitted (**structure**).
  - The client module is a two-copy fork (`modules/dpe/public/telemetry.js`,
    `areas/deposit/editor/public/telemetry.js`); a new signal edits both, plus `beacon.rs`,
    `collector.rs` and the docs, in one commit (**review**).
  - Metric attributes stay bounded; high-cardinality data goes to logs (**review**).
- **Durable state:** none. Process-wide statics (`METER_SCOPE`, `PAGE_URL_NORMALIZER`,
  `BROWSER_METRICS`) make the crate one-service-per-process by construction. Three fuzz targets
  (`beacon_payload`, `origin_validation`, `traceparent_validation`) exist but `fuzz.yml` runs
  only DPE's two.

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
  area) and the `media` capability of the Access Area modulith (today's modules/dpe area),
  both planned; each implements Vitrinli's traits over its own store and owns the routes
- **Boundary rules:** depends on no area crate and knows no area's session, rights or storage
  (**structure** via Bazel visibility once it arrives); every trait has one implementation per
  area's `media`, so each seam is real; the Access Area's `media` feeds it only archive-made
  Service Files from the Access bucket, the Deposit Area's only Originals it holds; neither
  path reads a Preservation File (`vitrinli/CONTEXT.md` → Boundary commitments; **docs-only**
  until code exists).
- **Durable state:** none. What is servable and where is a `media` capability's tables, in its
  area's database — **single writer:** that area's `media` (target design).

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
  products `sync` rebuilds from), `modules/dpe/CONTEXT.md` (the reading side today)
- **Depends on:** nothing in this repository is expected beyond `shared-*` crates
- **Used by:** the Access Area modulith — `sync` (writer of the archive projection), `profile`
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

### areas/access/cpe

- **status: planned**
- **Paths:** `:(glob)areas/access/cpe/**` (no files yet)
- **Purpose:** CPE, the Configurable Presentation Environment — a second Access-Area
  hypermedia server rendering project-specific presentations over the same data as DPE,
  configured per project. Where the per-project configuration lives is an open item.
- **Key entities:** — (none yet)
- **Public interface:** its router, mounted by the Access Area's composition root on the area's
  origin beside DPE's routes; `cpe/ports` for anything a sibling capability needs from it.
- **Local-context kit:** `docs/adr/0002-areas-at-the-repository-root.md`, `modules/dpe/CONTEXT.md`,
  `modules/dpe/server/src/router.rs` (the shape to copy), `shared/metadata/src/lib.rs`,
  `docs/src/mosaic/component-api-conventions.md`, `docs/adr/0003-one-modulith-per-area.md`
- **Depends on:** shared/metadata, shared/telemetry, modules/mosaic
  (expected); never DPE's domain, store or web crates. Two port directions, each with its own
  consumer (ADR-0003): what CPE needs from DPE, CPE declares in `cpe/ports` and DPE implements;
  what DPE needs from CPE, DPE declares in `dpe/ports` and CPE implements, depending on
  `dpe/ports` alone. A *concept* both need lives in `shared/`; a shape never does
- **Used by:** —
- **Boundary rules:** a capability of the Access Area's modulith (ADR-0003) — its own tables,
  ports as described under Depends on, no shared shape; and the platform-wide style commitment
  in `## Conventions`, a hypermedia server serving the browser directly, no BFF, no SPA
  (**docs-only** until code exists).
- **Durable state:** its own read-side store, rebuildable from the archive's snapshot + replay
  (target design).

## Conventions

- **One-way top-level dependency direction:** `services → shared, mosaic, vitrinli, chischtli`;
  `shared-*`, `mosaic-*`, Vitrinli and Chischtli never import a service; a service never
  imports another service. *Enforcement:*
  **structure** for the shared half (a service dependency in a shared crate is a Cargo
  cycle) + **static-analysis** for hardcoded paths (`check-shared-paths.sh`); **review** for
  service→service today, **structure** via Bazel visibility after ADR-0001.
- **Areas are bounded contexts (ADR-0002):** the Deposit, Archive and Access Areas integrate
  over the wire, never by Rust import; each service is the sole reader and writer of its
  durable state; cross-area references are opaque identifiers (shortcode, ARK); share
  concepts (a `shared-*` crate), never shapes. *Enforcement:* **review**, becoming
  **structure** with ADR-0001.
- **One modulith per area (ADR-0003):** one binary per area, composed at `<area>/server` out of
  capabilities that own their tables, collaborate only through consumer-defined `ports` crates
  with `Live<Port>` adapters in the provider's store, reference each other by opaque id, and
  never span a transaction. Today each area has one capability, so the service crates are the
  seeds of the composition roots. *Enforcement:* **review**, becoming **structure** (Bazel
  visibility: `ports` public within the area; domain / store / web visible to the capability
  and the composition root only) with ADR-0001.
- **Shared code lives under `shared/` as `shared-{role}`** the moment a second
  service depends on it; the directory is what CI path filters, `bacon.toml` watch lists and
  directory-scoped `CLAUDE.md` files key on. One crate is there ahead of its second consumer:
  `shared-fair`, because ADR-0005 names the consumers to come. *Enforcement:* **review** (the
  rule), **static-analysis** (the path grep).
- **Crate naming:** `{service}-{role}`; the folder drops the prefix (`dpe/core` is `dpe-core`).
  *Enforcement:* **review**.
- **Composition root:** each service's `server` crate owns routing and config; views are
  `fn(...) -> Markup` in the `web` crate; the domain / contract crate has no framework
  dependency. *Enforcement:* **review** (DPE's views self-load from `dpe-core` caches — a
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
| A `dpe-*` dependency in an `editor-*` crate (or the reverse) | Turns two deployables with a deliberate origin split into one codebase; the next agent copies the shortcut | Move the shared concept to a `shared-*` crate; pass service-specific data in as a parameter | review → structure (ADR-0001) |
| A capability importing a sibling capability's domain, store or web crate, or querying its tables | Turns the area's modulith into a tangle; extraction becomes impossible | Declare a port in the consumer's `ports` crate; the provider implements the adapter; wire at `<area>/server` (ADR-0003) | review → static-analysis (`check-capability-deps.sh`, ADR-0003 amendment of 2026-09-25, once it lands) → structure (ADR-0001) |
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
  `verify-checksums.sh`, `check-adr-refs.sh`, `check-live-tests-ignored.sh`),
  release-please config
- **Documentation:** `docs/**` (the mdBook under `docs/src/`, ADRs under `docs/adr/`),
  `README.md`, `CLAUDE.md`, `CONVENTIONS.md`, `REVIEW.md`, `CHANGELOG.md`, `LICENSE`,
  `CONTEXT.md`, `ARCH-MAP.md`, `shared/README.md`
- **Agent configuration:** `.claude/**` (settings, the `add-mosaic-component` skill)
- **Data tooling:** `scripts/**` — the two Python scripts that write DPE's lookup tables
