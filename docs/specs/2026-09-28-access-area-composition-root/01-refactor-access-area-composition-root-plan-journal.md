---
plan: docs/specs/2026-09-28-access-area-composition-root/01-refactor-access-area-composition-root-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7397
base_commit: a8e3df90
branch: worktree-DEV-7397
started: 2026-09-28
problem: >
  CPE (DEV-7400) is the Access Area's second capability and needs a composition root at
  areas/access/server to be mounted into. DPE's process-global state (five config OnceLocks
  and ten static data caches in dpe-core) is first-call-wins and process-wide, so a second
  capability or a second test corpus in the same binary silently gets DPE's values. The
  work makes that state capability-owned values and extracts the root from dpe-server.
status: complete
---

# Execution Journal: 01-refactor-access-area-composition-root-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | a8e3df90 | worktree-DEV-7397 | rebase | shipped | https://github.com/dasch-swiss/dsp-repository/pull/447 |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@47f59a19 | 1 |
| 2 | reviewed | dsp-repository@52cf82c0 | 1 |
| 3 | reviewed | dsp-repository@6d6110af | 1 |

## Chunk queue

Each phase is one compile unit (removing a global breaks every caller at once) and lands as one
commit, per the plan; chunks within a phase are sequential worker dispatches into the same
working tree, committed together once `just check && just test` pass.

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | eng.yaml | — | (intake fix) | eng.yaml validates | stale modules/*/{web,style,server} overrides | — |
| 1.2 | dsp-repository | api-oai/src/{lib,xml,handlers/*,metadata/*}.rs, server/src/{router,serve,shell,fragments,test_support}.rs, web/src/components/placeholder_value.rs + callers, core/src/{utils,lib}.rs | 1.1 | Phase 1 all code/test checkboxes | no set_base_url / set_show_placeholder_values; snapshots and golden files unchanged; just check && just test green | plan § OAI state, § Render context | — |
| 2.1 | dsp-repository | core/src/*_cache.rs, core/src/{lib,utils,ark}.rs, core tests | 1.2 | Phase 2 dpe-core checkboxes | Corpus value, two corpora coexist | plan § Corpus shape | — |
| 2.2 | dsp-repository | api-oai, web, server call sites; fuzz; docs/src/dpe/project_structure.md | 2.1 | Phase 2 remaining checkboxes | no static OnceLock outside tests; snapshots unchanged | plan § Corpus shape | — |
| 3.1 | dsp-repository | areas/access/server/**, dpe/server lib.rs, Cargo.toml | 2.2 | Phase 3 "The code" | access-server builds and serves; tests green | plan § public surface | — |
| 3.2 | dsp-repository | Dockerfile, .github/**, justfile, bacon.toml, playwright config, eng.yaml, check-composition-root-deps.sh | 3.1 | Phase 3 "Build, deploy and CI" | gates pass | plan Phase 3 | — |
| 3.3 | dsp-repository | CLAUDE.md files, ADRs, docs/src/**, ARCH-MAP.md | 3.1 | Phase 3 "Docs and records" | docs name access-server | plan Phase 3 | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | f691fefd | drop stale modules/* eng.yaml overrides (folded into the Phase 1 commit) | none |
| 1.2 | dsp-repository | complete | f691fefd | OaiState replaces dpe-api-oai's BASE_URL global; RenderContext replaces the placeholder global; dropped now-unused tracing dep | none |
| 1.3 | dsp-repository | complete | 52cf82c0 | Phase 1 review fixes amended into the phase commit (f691fefd → 52cf82c0): ARCH-MAP key entities, project_structure.md placeholder sentence, xml.rs base-URL clone helper | none |
| 2.1 | dsp-repository | complete | 10d1f056 | dpe-core: Corpus + CorpusSettings own the ten caches (instance OnceLocks, &'static self accessors); setters, statics and get_data_dir env fallbacks removed | none |
| 2.2 | dsp-repository | complete | 10d1f056 | callers: OaiState/RenderContext/AppState carry the corpus; serve.rs leaks one; ListInputs replaces the 7-arg paged handlers; test corpora per crate; docs updated | none |
| 2.3 | dsp-repository | complete | 6d6110af | Phase 2 review fixes amended (10d1f056 → 6d6110af): AppState::render_context, stale process-global comments, oai_state test corpus memoised, CONTEXT.md Corpus entry, project_structure resolve_inputs | none |
| 3.1 | dsp-repository | complete | 99a620b5 | access-server crate (main/cli/serve/observability moved with history); dpe-server lib.rs with Dpe::{new,router,warm}; OTel layers at the root; TELEMETRY_NAME pinned; traceparent split tests (mutation-checked) | none |
| 3.2 | dsp-repository | complete | 99a620b5 | Dockerfile moved, /app/dpe-server alias; build-dpe, a11y, cloud-run, scout workflows; justfile, bacon, playwright, dependabot, eng.yaml; check-composition-root-deps.sh with an 8-case test | none |
| 3.3 | dsp-repository | complete | 99a620b5 | areas/access/server/CLAUDE.md; dpe CLAUDE/README; ADR-0002/0007 amendments; docs/src sweep; ARCH-MAP via dune-map; `dpe-server validate` → `access-server validate` in editor/shared comments; .dockerignore | none |
| 3.4 | dsp-repository | complete | 5e9a72c1 | Phase 3 review fixes amended (99a620b5 → 5e9a72c1): gate catches renamed deps (`package = "…"`, 2 cases, mutation-checked) and its header states which roles it forbids; bacon watches areas/access/server; ARCH-MAP enforcement label split; observability-reviewer bound to observability.rs; Dpe::new leak-per-call doc; ADR-0003 pointer | none |

## Deferrals

- Phase 1 review: the 7-argument `handle_list_*_paged` signatures (clippy allow) → Phase 2 checkbox, since the corpus in `OaiState` subsumes most of those arguments.

## Side findings

- eng.yaml failed static validation at intake: the `modules/*/web/**`, `modules/*/style/*.css` and
  `modules/*/server/src/**` overrides matched nothing since DPE moved to `areas/`. Removed; folded
  into the Phase 1 commit (user decision).
- Phase 1 checkpoint: 6 reviewers (rust, simplicity, consistency, ivan, dune, accessibility), no Critical in code; browser pass over /dpe/projects, /dpe/projects/0803, OAI Identify clean after `just css` (a fresh worktree has no built CSS; not a regression).
- Phase 2: the OAI golden-file tests had been reading empty temporal tables (relative default data dir resolved from the crate cwd, where it does not exist); their test corpus uses an empty dir to keep bytes identical.
- Phase 2: `Pid` deserialises from a string but serialises as a struct, so on-disk record fixtures must be written as raw JSON, not serialised `Record`s.
- Phase 2 checkpoint: 6 reviewers (rust, simplicity, consistency, ivan, dune, performance); no Critical. Browser pass clean.
- `dsp-cli` `corrupt_cache_does_not_leak_token_bytes_at_default_verbosity` failed once under the full `just test` (empty captured log) and passed on three isolated re-runs and the next full run: a flaky log-capture test in an untouched crate, not this work.
- Pre-existing: `dpe-api-oai/src/metadata/corpus.rs` `PREVIEW_ORIGIN` / `ark_paths` are dead code under `cargo clippy -p dpe-api-oai` (since a8e3df90); `just check` does not flag them.
- The same dsp-cli test failed again on a Phase 3 run (2 of ~5 full runs overall); the next run passed.
- Phase 3 end-to-end: `just test-a11y-dpe` 5/5 against access-server; `just run` routes 200, traceparent on
  pages and not on /healthz; `access-server healthcheck` defaults to :8080 (the container port), so
  locally it needs `--url`; an image built from a musl binary compiled in rust:1.93-alpine answers
  healthcheck at both /app/access-server and /app/dpe-server.
- Phase 3 checkpoint: 7 reviewers (rust, simplicity, consistency, ivan, dune, devops, observability); no
  Critical survived. Declined: dune's call to forbid every non-`server` capability crate in the root.
  ADR-0003 has the root wire `Live<Port>` adapters from a provider's store crate, so `store`/`ports`
  stay allowed; the gate's header now says so. Browser check at this checkpoint = the Playwright a11y
  run plus live probes against access-server.
- Pre-existing, not this work: justfile `CARGO_VERSION` (dev-otel) takes every workspace package's
  version, not one crate's.
- Final review, catch-up round (user decision, after the session named its deviations: no skeptic
  agents, maud-datastar and later accessibility reviewers skipped, fixes not re-reviewed, reduced
  final pass): 9 reviewers over the full diff (rust, simplicity, consistency, dune, ivan,
  maud-datastar, accessibility, devops, observability), a skeptic on the DUNE-001 decision (holds:
  ADR-0003 has the root construct adapters itself), direct checks on each Critical/Warning, one
  fixup commit (129a2fb8) and a re-review of it. Fixed: lost "OAI-PMH base URL set" log (Phase 3),
  stale `shared/README.md` get_data_dir text (Phase 2), `({ project_tabs(..) })` splice (Phase 1),
  gate header over the block budget, `init_test_otel` doc length, store/ports rule promoted into
  ARCH-MAP's convention and CPE entry, CONTEXT.md names access-server. All landed in the Phase 3
  fixup to avoid rebase conflicts across phases.
- Pre-existing, left for follow-up: no CI workflow runs any `.github/scripts/*.test.sh` (they sit
  under `just test`; CI runs `just check` only), so the new gate's test suite is local-only like
  its siblings; the record cache's dev-API fetch with a placeholder bearer (already in ARCH-MAP);
  `collect_cluster`'s Vec-scan dedup; golden files auto-written on first run; `build-docker-dpe`
  building from the repo root (the plan leaves it).
- A fresh worktree needs `just css` and `npm ci` in web-e2e-tests before a browser or a11y check.
- Intake: browser verification (eng:test-browser) runs unattended at every phase checkpoint (user decision).

## Closeout

- root_cause: DPE was built as the only thing in its process. Its configuration (data dir, public dir,
  ARK host, placeholder flag, OAI base URL) and its ten data caches were `static` `OnceLock`s set or
  lazily loaded first-call-wins, and `dpe-server` was both DPE's router and the process's `main`. A
  second capability (CPE) or a second test corpus in the same binary would silently inherit DPE's
  values, and there was no area-level root to mount CPE into.
- investigation: The plan's claims held at intake (ten statics in nine cache modules, five setters,
  one ops-deploy healthcheck on `/app/dpe-server`). Three things surfaced during execution: the OAI
  golden-file tests had been running against empty temporal tables, because the relative default
  data dir resolved from the crate's cwd did not exist; `Pid` serialises asymmetrically, so record
  fixtures must be raw JSON; and git pathspec `*` crosses `/`, so the composition-root gate needed
  depth filters. Review found that the move had dropped the "OAI-PMH base URL set" startup log,
  that bacon no longer watched the root, and that the gate could be bypassed with a renamed
  dependency. A proposal to forbid every non-`server` capability crate in the root was declined
  against ADR-0003, which has the root construct `Live<Port>` adapters from store crates; a skeptic
  pass confirmed it.
- solution: Three commits. (1) `OaiState` carries the OAI base URL on the `/dpe/oai` sub-router and
  `dpe_web::RenderContext` carries the placeholder flag, replacing two setters. (2)
  `dpe_core::Corpus` owns every cache as instance `OnceLock`s with `&'static self` accessors, built
  from `CorpusSettings` and leaked once; handlers, the OAI endpoint and views receive it through
  `AppState`, `OaiState` and `RenderContext`. (3) `areas/access/server` (`access-server`) is the
  composition root: CLI, OTel and Pyroscope init, the OTel layers, the untraced routes, mounting
  `dpe_server::Dpe::router()`. `dpe-server` is a library. Telemetry names are pinned, the image
  keeps `daschswiss/dpe` and ships a `/app/dpe-server` alias until ops-deploy switches (H1).
- prevention: Tests that two corpora and two OAI base URLs coexist in one binary; traceparent-split
  tests on the assembled router (mutation-checked); `check-composition-root-deps.sh` in `just check`
  with a ten-case test (renamed-dependency cases mutation-checked); no `static` `OnceLock` outside
  `#[cfg(test)]` in `areas/access`. Anti-pattern to avoid: process-global setters for per-deployment
  configuration; a new capability's state goes in its own value, handed in by the root. Open:
  CI does not run `.github/scripts/*.test.sh`.
