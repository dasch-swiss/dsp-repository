---
plan: docs/specs/2026-10-21-media-minimum/01-feat-media-minimum-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7512
base_commit: e528da68a05dcc19c81f1231cfe463f1758f94b8
branch: worktree-DEV-7512
started: 2026-10-21
problem: >
  ADR-0012 (PR #494) decides that a project's configuration, curation and narrative are deposited
  files that reach a reader as bytes through a port against `media`, and that a CPE project folder
  holds code, not data. Today CPE reads `config.kdl` and `narrative/` from its project folder and the
  curation travels inside `sync`'s snapshot. This run adds the bytes port to `cpe-ports`, builds
  `media` at its minimum (`media-store`, `MEDIA_DATA_DIR`), moves Incunabula's files there, and
  retires curation from `sync` and the snapshot (DEV-7511, DEV-7512).
status: complete
---

# Execution Journal: 01-feat-media-minimum-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | e528da68 | worktree-DEV-7512 | rebase | shipped | https://github.com/dasch-swiss/dsp-repository/pull/526 |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@b09f6af1 | 2 |
| 2 | reviewed | dsp-repository@dfbb3cd0 | 1 |
| 3 | reviewed | dsp-repository@c43ebefa | 1 |
| 4 | reviewed | dsp-repository@70237734 | 1 |
| 5 | reviewed | dsp-repository@b9433150 | 1 |
| 6 | reviewed | dsp-repository@8c7e292e | 2 |
| 7 | reviewed | dsp-repository@8a7dd1d6 | 0 |

## Chunk queue

| id  | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | areas/access/cpe/ports/src/names.rs, lib.rs | — | "is_shortcode accepts/refuses", "is_file_name accepts", "is_file_name refuses", "Implement is_shortcode and is_file_name" | both predicates' tests pass; re-exported from lib.rs | plan § The bytes port (`cpe-ports`), Names/Shortcodes bullets | — |
| 1.2 | dsp-repository | areas/access/cpe/ports/src/lib.rs, fake.rs | 1.1 | the seven FakeDepositedFiles/requests/FilesError tests, "Implement the DepositedFiles trait and FilesError", "Implement FakeDepositedFiles", "Update the crate doc" | trait, error and fake tests pass; crate doc names both ports | lib.rs:1-76 (ArchiveProjection/ProjectionError as model), fake.rs:1-45 | — |
| 1.3 | dsp-repository | — (verification, no commit) | 1.2 | "cargo clippy -p cpe-ports --all-targets", "just check", "just test" | all three pass | — | — |
| 2.0 | dsp-repository | areas/access/cpe/ports/src/names.rs | — | — (owner's Phase 1 answer, brief resume_state) | `is_shortcode` doc says why `0801a`–`0801e` are refused; `fixup!` of 1a84bf39 | names.rs:1-6 | — |
| 2.1 | dsp-repository | Cargo.toml, Cargo.lock, areas/access/media/store/{Cargo.toml,src/lib.rs,src/tests.rs} | 2.0 | the three Scaffold items, the twelve LiveDepositedFiles tests, "Implement LiveDepositedFiles" | crate builds; all twelve tests pass; manifest staged; crate doc names stand-in, anonymous consumer, ADR-0012 | plan § `media` at its minimum; sync/store/Cargo.toml + src/lib.rs as model | — |
| 2.2 | dsp-repository | .github/scripts/check-cpe-seam.sh, check-cpe-seam.test.sh | 2.1 | the five seam-gate items | `bash .github/scripts/check-cpe-seam.test.sh` passes; real gate passes | check-cpe-seam.sh rule 4, test.sh base_repo/:126/:621/rule-4 block/absence block | — |
| 2.3 | dsp-repository | — (verification, no commit) | 2.2 | "cargo clippy -p media-store --all-targets", "just check", "just test" | all three pass | — | — |
| 3.1 | dsp-repository | areas/access/cpe/server/src/activation.rs | — | the two Baseline pin items | header-stripped pin passes on unchanged code; constants never change after | activation.rs tests § incunabulas_store_text_is_the_one_the_prototype_embeds; plan § Curation moves (split at "\n\n") | — |
| 3.2 | dsp-repository | areas/access/cpe/core/src/validate.rs (+ test) | 3.1 | "cpe-core accessor lists every narrative file", "Add the narrative-name accessor" | accessor returns bases from page-body `narrative` and `narrative-for`, also when lowering fails | validate.rs load_remodel (~:220), lower_narrative, narrative-for; declared_languages (:171) | — |
| 3.3 | dsp-repository | areas/access/cpe/core/src/remodel.rs (+ test) | 3.1 | "Remodel::reads_curation holds for each of its four sources", "Add Remodel::reads_curation" | predicate true for curated node, curated-key attr, hook key, annotations; false for THINGS | remodel.rs RemodelClass::curated_keys, RemodelHook.keys, Remodel.annotations | — |
| 3.4 | dsp-repository | cpe-core validate/context.rs, validate.rs, cpe-web tests/validation.rs, cpe-server activation.rs/router.rs, docs/src/cpe/config-language.md | 3.1 | "validation with a complete, empty inventory", "Add the inventory-completeness field", "Add the new field to every literal", "Reword config-language.md:2410 and NarrativeFileNotFound" | complete+empty inventory reports NarrativeFileNotFound; for_tests = not complete | context.rs:10-50; validate.rs check_narrative_file (~:4783-4849) | — |
| 3.5 | dsp-repository | cpe-store-build src/curation.rs (new), src/curation_tests.rs, lib.rs, generic/curation.rs, tests/it/{main,support,hooks,hookless}.rs, cpe-server activation.rs, areas/access/media/data/0803/curation.csv, .gitattributes | 3.1 | "curation parser refuses dangling/duplicate/malformed/control", "Copy the curation parser", "build takes &CuratedValues", "unit-test literals generic/curation.rs", "static CURATION", "build(snapshot) wrapper", "build_with helper", "Copy curation.csv", ".gitattributes rule" | store-build tests green with CURATION from the media CSV; activation passes curation explicitly | sync/store/src/curation.rs; store-build lib.rs:37-53, :236-238; generic/curation.rs:38-47, :183-195; tests/it/main.rs:38-63 | — |
| 3.6 | dsp-repository | cpe-store-build src/sql.rs, tests/it/hookless.rs, cpe-server activation.rs | 3.5 | "Change the store header", "Re-pin INCUNABULA_STORE_*", "Rename incunabulas_store_text…" | header names both ports; full pin re-pinned; header-stripped pin unchanged | sql.rs:82-112, :523-578; hookless.rs:1205, :1355 | — |
| 3.7 | dsp-repository | areas/access/server/src/serve.rs | — | "data_dir returns the default for None and Some(\"\")", "Add data_dir … SYNC_DATA_DIR" (sync half) | empty SYNC_DATA_DIR = unset; test | serve.rs:200-240 | — |
| 3.8 | dsp-repository | cpe-server activation.rs, discovery.rs | 3.4 | — (preparatory refactor for "load_all/load take a discovery function and a per-project source") | no behaviour change; all cpe-server tests green | activation.rs:31-184 | — |
| 3.9 | dsp-repository | cpe-server {discovery,activation,lib,base_tests,router}.rs + fixtures + Cargo.toml, access-server serve.rs + Cargo.toml, media/data/0803/{config.kdl,narrative/}, cpe/projects/incunabula/SHORTCODE, store-build tests/it/{main,worked_example,hooks}.rs, docs/src/cpe/config-language.md:616 | 3.2-3.8 | every remaining Data, Tests-first and Implementation item | CPE activates from both ports; all listed tests; whole suite green | plan § CPE reads through the port; § Wiring and image | — |
| 3.10 | dsp-repository | — (verification, no commit) | 3.9 | the four Verification items | all pass, pin constants unchanged from 3.1 | — | — |
| 4.1 | dsp-repository | cpe-store-build tests/it/{main,support,hooks,hookless,committed_0803,refusals,annotations,hook_rules,rebuild,…}.rs | — | "Move the test helpers' curated values … fixture type", "Replace the reads of SNAPSHOT.curation", "Make every build derived from the committed data take CURATION's values", "Change the order-independence tests" | store-build tests green; no test build reads `snapshot.curation`; committed-data builds take CURATION; canary: empty curation in the committed fixture fails `test_build_reads_every_declared_property` | support.rs build_with/with_curation/recurated/minimal_snapshot; committed_0803.rs without/file_of/reordered; hooks.rs ~:1365-1414 | — |
| 4.2 | dsp-repository | sync-store src/{lib,curation,curation_tests,snapshot_tests,ark_tests}.rs, tests/committed_0803.rs, sync/data/{0803-curation.csv,PROVENANCE}, media/data/PROVENANCE (new), .gitattributes, sync/CONTEXT.md | 4.1 | "Remove read_curation … from sync-store", "Rewrite sync-store's crate doc", "git rm 0803-curation.csv and .gitattributes rule", "Move the curation section of PROVENANCE", "Update sync/CONTEXT.md" | sync-store tests green; snapshot curation empty; no sync file names curation | sync/store/src/lib.rs:1-136 | — |
| 4.3 | dsp-repository | cpe-ports src/{lib,snapshot,contract,fake}.rs; cpe-store-build src/{lib,curation_file,curation_file_tests,generic/*,mapping/*}.rs + tests/it; cpe-server activation.rs; access-server serve.rs; sync-store mapping/mod.rs, ark_tests.rs | 4.2 | "Move CuratedValue and is_curation_name", "Remove ProjectSnapshot.curation", "Remove curation from cpe-ports's docs", "Remove the contract's curation violations", "Remove the curated values from fake.rs's tests" | workspace builds; all tests green; `rg CuratedValue areas/access/cpe/ports/src` empty | cpe-ports snapshot.rs:66-102, contract.rs:11-14,74-79,170-195,343-380 | — |
| 4.4 | dsp-repository | areas/access/media/data/PROVENANCE | 4.2 | "Write the config.kdl and narrative/ entries of media PROVENANCE" | entries name origin, PR, how to change | git log of the moved files; projects/incunabula/PROVENANCE.md | — |
| 4.5 | dsp-repository | — (verification, no commit) | 4.1-4.4 | the four Verification items | rg empty; both pins unchanged; just check; just test | — | — |
| 5.1 | dsp-repository | justfile | — | "Stage `areas/access/cpe/projects` in `just build-docker-dpe`" | recipe stages `cpe/projects` as `build-dpe` does | justfile `build-docker-dpe` (~:553-593) | — |
| 5.2 | dsp-repository | .github/actions/build-dpe/action.yml, areas/access/server/Dockerfile, justfile, .dockerignore, docs/src/dpe/operations.md, docs/src/cpe/operations.md | 5.1 | "Stage media/data in action.yml", "COPY + ENV in Dockerfile", "Dockerfile header", "Stage media/data in just build-docker-dpe", ".dockerignore", "Update docs/src/dpe/operations.md", "Update docs/src/cpe/operations.md" | three places agree (staging, COPY, ENV); docs name `MEDIA_DATA_DIR` | action.yml Stage step; Dockerfile:1-30; dpe/operations.md:15-17, :91-102; cpe/operations.md:12-13, :21, :36-37, :53, :81-83 | — |
| 5.3 | dsp-repository | .github/workflows/{a11y-dpe,cloud-run-dpe-pull-request,scout-dpe}.yml, docs/src/deployment.md, docs/src/repo_structure.md, docs/src/security.md | — | the three path-filter items, "Update the docs that restate the path filters" | media/cpe/sync paths in each filter; docs restate them | workflow `on.*.paths`; deployment.md:20, :114, :116; repo_structure.md:117; security.md:26 | — |
| 5.4 | dsp-repository | — (verification, no commit) | 5.1-5.3 | "just build-docker-dpe builds", "docker export lists both paths", "docker run serves Incunabula", "just check", "just test" | all pass | scripts under .claude/tmp/ | — |
| 6.1 | dsp-repository | areas/access/media/CONTEXT.md (new) | — | "Write areas/access/media/CONTEXT.md" | sync/CONTEXT.md's shape; stand-in, MEDIA_DATA_DIR, layout, LiveDepositedFiles, anonymous consumer, not yet (derive, assets) | media/store/src/lib.rs; media/data/PROVENANCE | — |
| 6.2 | dsp-repository | areas/access/cpe/CONTEXT.md | — | "Update areas/access/cpe/CONTEXT.md" (+ the `:142` half of the store-build README checkbox) | no "planned"/"until DEV-7511"; SHORTCODE, from_sql_folders legacy, CpeConfig, stricter name rule, 0801a–e line, build signature | cpe/CONTEXT.md intro, Curation, Bytes port, Activation, Unavailable project, project folders, store builder, What the port serves | — |
| 6.3 | dsp-repository | areas/access/CONTEXT.md, areas/access/datei/CONTEXT.md | — | "Update areas/access/CONTEXT.md and datei/CONTEXT.md" | media exists; DaTEI's adapter waits for its port | access/CONTEXT.md:3, :14; datei/CONTEXT.md:55, :62, :82 | — |
| 6.4 | dsp-repository | vitrinli/CONTEXT.md, CONTEXT.md | — | "Update vitrinli/CONTEXT.md and the root CONTEXT.md Access Area line" | stand-in built; root line names media at its minimum | vitrinli/CONTEXT.md:32; CONTEXT.md:13 | — |
| 6.5 | dsp-repository | docs/adr/0007-…md | — | "Add a dated ## Amendment to ADR-0007" | appended only; header names both ports, body pinned, incubator data.sql fails freshness until regenerated | ADR-0007 :429-435, last amendment | — |
| 6.6 | dsp-repository | areas/access/server/CLAUDE.md | — | "Update areas/access/server/CLAUDE.md" | LiveDepositedFiles/media-store in boundary; image names media/data, MEDIA_DATA_DIR, just build-docker-dpe staging, widened filters | server/CLAUDE.md:33-37, :48-67 | — |
| 6.7 | dsp-repository | areas/access/cpe/projects/incunabula/{PROVENANCE,CURATION,DATA_ISSUES}.md, assets/README.md | — | "Repoint path references in Incunabula's …" | no stale config.kdl/narrative/0803-curation.csv paths | rg output | — |
| 6.8 | dsp-repository | areas/access/cpe/store-build/README.md | 6.2 | "Repoint path references in store-build/README.md … and build's signature in cpe/CONTEXT.md:142" | build signature with &CuratedValues; caller flow through both ports; media paths; renamed pin test | README :7-29, :79, :95-96, :100, :210 | — |
| 6.9 | dsp-repository | areas/access/media/data/0803/config.kdl | — | "Reword the comments in the moved config.kdl that name DESIGN.md" | every DESIGN.md names the incubator's file; store pins unchanged | config.kdl :6, :12, :60… | — |
| 6.10 | dsp-repository | CONVENTIONS.md, docs/src/git-conventions.md | — | "Add media-store to the scope tables" | both lists name media-store after sync-store | CONVENTIONS.md:54; git-conventions.md:87 | — |
| 6.11 | dsp-repository | docs/src/repo_structure.md | — | "Update the tree in repo_structure.md", "Update the crate table in repo_structure.md" | tree + bounded-context list + crate rows current; media-store row | repo_structure.md:7, :26-37, :80-93 | — |
| 6.12 | dsp-repository | ARCH-MAP.md | 6.1-6.11 | "Update ARCH-MAP.md" | media active with fingerprint; cpe/sync/datei/server/overview refreshed; fingerprints recomputed on the committed tree | ARCH-MAP :9-34, :137-196, :830-1226; .claude/tmp/p6/fp.sh | — |
| 6.13 | dsp-repository | — (verification, no commit) | 6.1-6.12 | the rg check, "just check", "just test" | all pass | — | — |
| 7.1 | dsp-repository | ARCH-MAP.md | — | — (fingerprint refresh before the squash) | every Access-Area fingerprint matches `.claude/tmp/p6/fp.sh` on a clean tree | ARCH-MAP cpe/sync/media `Fingerprint` lines | — |
| 7.2 | dsp-repository | — (history only) | 7.1 | "Squash the branch to one commit", "Run `just commit-lint`" | rebased onto 6b1255fc, one commit above it, tree equal to the rebased head; commit-lint, check, test pass | brief notes steps 2-4 | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | done | 1a84bf39 | `is_shortcode`/`is_file_name` in `cpe-ports/src/names.rs`, re-exported; refusal test canaried by dropping the leading-`.` check | — |
| 1.2 | dsp-repository | done | 9dc4e81c | `DepositedFiles` + hand-written `FilesError` in `lib.rs`, `FakeDepositedFiles` in `fake.rs` (name check before lookup, `requests()` behind a `Mutex`, no `Clone`), crate doc names both ports; refused-name test canaried | — |
| 1.3 | dsp-repository | done | — (verification) | `cargo clippy -p cpe-ports --all-targets -D warnings`, `just check`, `just test` all exit 0 at 9dc4e81c | — |
| 1.4 | dsp-repository | done | 8d32d492 | Review fix round 1 (`fixup!` of 9dc4e81c): trait states a refused shortcode is `UnknownProject` (checked before the name) and the fake applies `is_shortcode`; `FilesError` Display quotes caller strings with `{:?}` and says "deposited files", not "media"; `is_shortcode` doc names why it is stricter than `shared_metadata::is_valid_shortcode`; `is_file_name` doc names the symlink dependency; `with_entry` one `match` | — |
| 1.5 | dsp-repository | done | 51edb03e | Review fix round 2 (`fixup!` of 9dc4e81c, doc only): `is_shortcode` no longer claims to be how DSP writes every shortcode and names `0801a`–`0801e` as unservable through the port; `FilesError` variant docs name the refusals that map to them. `just check`/`just test` exit 0 | — |
| 2.0 | dsp-repository | done | 9c3a71a0 | `fixup!` of 1a84bf39 (doc only): `is_shortcode` says `0801a`–`0801e` are refused on purpose (metadata-only split of BEOL, one DSP project `0801`, no bucket prefix); `cargo test -p cpe-ports` green | — |
| 2.1 | dsp-repository | done | 9be2540b | `media-store` crate (`cpe-ports`, `tracing` for `#[instrument]` as `sync-store`; `tempfile` dev) and `LiveDepositedFiles`: refusal order shortcode → name before any fs call; `symlink_metadata` per component via a private `Entry` classifier; `NotFound`/`NotADirectory` = absent, other kinds `Unavailable`; a directory or non-regular file under the name `Unavailable`; `Unavailable` source is an `io::Error` naming the path. 13 tests in `files_tests.rs`; canaried (follow symlinks → 3 fail; collapse errors → permission test fails) | — |
| 2.2 | dsp-repository | done | 25e5382d | `check-cpe-seam.sh` rule 4 covers `areas/access/media/*` (one `feed_names` set = sync + media names, for the CPE-manifest check and the root renames); absence error, readable-name check, header/rule-1/`Forbidden names`/failure text, success line counts sync and media crates (`count_lines` helper). Tests: `media-store` in `base_repo`, counts 11 → 12, ten media cases (rule 1 ×2, rule 4 ×6 incl. root rename, dropped path, unreadable name); canary (media names dropped) fails 10. Also updated the restatements in `cpe/CONTEXT.md` (seam section) and `docs/src/workflows.md:29` | — |
| 2.3 | dsp-repository | done | — (verification) | `cargo clippy -p media-store --all-targets -D warnings`, `just check`, `just test` exit 0 at 25e5382d. `just commit-lint` accepts the `media-store` scope; it fails only on the `fixup!` subjects and the commit count, both settled by Phase 7's squash | — |
| 2.4 | dsp-repository | done | e26799cb, 881921bf | Review fix round 1. `fixup!` of 9be2540b: walk = name's directories then last segment (`split('/').next_back()`), `Entry` is `Directory`/`File`/`Neither`; test that a refused shortcode wins over a refused name; crate doc states the port's rule (serves what an anonymous consumer may see) instead of "rights check goes here"; symlink canary re-run, 3 fail. `fixup!` of 25e5382d: rule-4 restatements in `justfile:91` help text, the test's rule-4 header and `ARCH-MAP.md` cpe seam paragraph; rule-1 media case asserts `(rule 1`; `cpe/CONTEXT.md` line wrapped. `just check`/`just test` exit 0 (seam tests 222/0) | — |
| 3.1 | dsp-repository | done | 3920fa88 | `incunabula_store_text()` helper; header-stripped pin `INCUNABULA_STORE_BODY_BYTES` = 5_631_489, `INCUNABULA_STORE_BODY_SHA256` = dfd33273…25af, computed on the unchanged code (fails with placeholder values first) | — |
| 3.2 | dsp-repository | done | 98c8da54 | `cpe_core::validate::named_narratives(src) -> NamedNarratives { files, languages }` over the raw parse (page-body `narrative` + `narrative-for`, deduplicated, unchecked names) and `candidates()` (base, then `variant_filename` per language); test canaried by dropping the `narrative-for` branch | — |
| 3.3 | dsp-repository | done | bab893b7 | `Remodel::reads_curation()` = annotations, a hook key, or any class's `curated_keys()`; tests over four one-line configs and the THINGS fixture (`include_str!` from cpe-server's fixtures); canaried by dropping the hook branch | — |
| 3.4 | dsp-repository | done | 895fc32a | `ValidationContext::narrative_files_complete` (false in `for_tests` and all 22 literals); `check_narrative_file` checks existence when complete or non-empty; message/doc/`config-language.md` say "the project's narrative files"; test canaried by dropping the flag | — |
| 3.5 | dsp-repository | done | 87571298 | `cpe_store_build::{CuratedValues, CurationFault, InvalidCuration}` in `src/curation_file.rs` (module named so it does not shadow `generic::curation`), tests in `curation_file_tests.rs` (sync's, plus one through `parse(&snapshot)`; canaried); `build(snapshot, &curated, mapping, hook)`; media `0803/curation.csv` + `.gitattributes`; `CURATION` static, wrapper and `support::build_with` via `from_unchecked`; activation transitional `from_unchecked(snapshot.curation)` until 3.9 | — |
| 3.6 | dsp-repository | done | 1ce6b808 | header: `Source: the archive projection's snapshot of project S.` / `From the bytes port: the remodel block of S/config.kdl[ and S/curation.csv].` (curation when `reads_curation`) / `Not from a port: the project's hook.` (hook only, so Incunabula's header gains a line); sql.rs + hookless pins updated, new curation-line test; whole pin 5_631_888 / 36da3224…0c86, test renamed `incunabulas_whole_store_text_is_pinned`; body pin unchanged | — |
| 3.7 | dsp-repository | done | d38d032a | `serve::data_dir(Option<String>, &str)` (empty = unset) for `SYNC_DATA_DIR`; test canaried by dropping the empty filter. `MEDIA_DATA_DIR` half of the "Add `data_dir`" checkbox lands with 3.9 | — |
| 3.8 | dsp-repository | done | bffa9c3d | preparatory refactor: `Sources { config, config_file, narrative, narrative_complete, store_sql }`; `load_all(config, discovered, sources)`; both paths read config + narrative from the folder via `folder_sources`; no behaviour change, cpe-server/access-server tests green | — |
| 3.9 | dsp-repository | done | adcd9025 | switch: `discover_for_activation` (SHORTCODE, `Discovery { projects, skipped }`, shared-shortcode refusals) + legacy `discover_sql_folders`; `activation::from_ports` → `port_sources` (config → shortcode check → `named_narratives` candidates → `curation.csv` + `reads_curation` → snapshot → `CuratedValues::parse` → build), complete inventory; `Cpe::activate(projection, files)`; data `git mv` + `SHORTCODE`; access-server `MEDIA_DATA_DIR`/`DEFAULT_MEDIA_DATA_DIR`/warning/`LiveDepositedFiles`; store-build tests on media KDL, hook-table resolver; traversal tests on SHORTCODE/PROVENANCE.md. Canaries: path filter, completeness flag, `reads_curation`, shared-shortcode refusal each fail a test. Discovery fixtures are tempdirs (a symlinked SHORTCODE cannot be committed portably); `from_sql_folders` fixtures untouched. `just check`/`just test` exit 0; body pin unchanged | — |
| 3.10 | dsp-repository | done | — (verification) | at adcd9025: `cargo test -p cpe-server incunabula` 5/5, body pin constants unchanged since 3920fa88; `just check` and `just test` exit 0. `cargo clippy --workspace --all-targets -D warnings` fails only in `mosaic-playground` (`items_after_test_module`, src/app.rs:188) and `dpe-api-oai` (dead `PREVIEW_ORIGIN`/`ark_paths` in a test, metadata/corpus.rs:586,621), both untouched since the branch base; the rest of the workspace is clean (`--exclude` both). That checkbox stays unticked | — |
| 3.11 | dsp-repository | done | 1e1604f7, 95968abe, 6e8d4322 | Review fix round 1 (`fixup!` commits). Verified and fixed: shared-shortcode refusal pass grouped once in a `BTreeMap` (simplicity W); narrative candidates deduped by names asked, not found (simplicity/rust S); `config-language.md` "bundle" wording (consistency S); `main.rs` doc names `STORE`'s source (consistency S). cpe-server/store-build tests green | — |
| 4.1 | dsp-repository | done | f9c6661f | `support::Fixture { facts, curation }` is the argument of the `build` wrapper and `build_with`; `with_curation`/`with_curation_in`/`recurated` act on its values; `main.rs` `COMMITTED` = `SNAPSHOT` + `CURATION`'s values, used by `without()`, `file_of`, the reordered build and `hooks.rs`' three committed builds; `test_build_reads_every_declared_property` and the annotation sibling assert `build(&COMMITTED) == STORE` first (canary: empty `COMMITTED` curation fails 6 tests). Refusal case for `ContractViolation` is now a dangling link (a dangling curated value no longer reaches the contract). Facts literals keep `curation: Vec::new()` until 4.3 | — |
| 4.2a | dsp-repository | done | 0fce5aba | not in the queue (added in flight): the sixteen curation census tests of `sync-store/tests/committed_0803.rs` (347 values, 13 key/lang counts, per-key facts) ported to `cpe-store-build/tests/it/curation_0803.rs` over `CURATION`/`SNAPSHOT`, so removing sync's reader in 4.2 loses no fact of the committed file; the re-read test became "read sorted, without repeats" | — |
| 4.2 | dsp-repository | done | be96e596 | `sync-store` reads `.nq` only: `curation` module + tests, `read_curation`, `CurationFault`/`InvalidCuration`/`SnapshotError::Curation`, the curation cases of `snapshot_tests.rs` and `tests/committed_0803.rs`, and `test_source_outside_tests_names_no_committed_curation_key` (ADR-0010's guard; nothing in sync reads curation) removed; crate/adapter/error docs name the snapshot only; `git rm data/0803-curation.csv` + `.gitattributes` rule; curation section moved to new `media/data/PROVENANCE`, sync's `:26-28` reworded; `sync/CONTEXT.md` drops **Committed curation**. `ark_tests.rs`'s `curation: vec![]` goes with the field in 4.3, so that checkbox is ticked there | — |
| 4.3 | dsp-repository | done | d2dda992 | `ProjectSnapshot.curation`, `cpe_ports::CuratedValue`, the three curation `Violation`s + `curation_name` + `Display` arms + tests, the fake's curated values and every curation sentence of `cpe-ports`' docs removed; `CuratedValue` (pub, re-exported) and `is_curation_name` (`pub(crate)`) now in `cpe_store_build::curation_file`, with their two tests moved to `curation_file_tests.rs`; `contract_error` arms, all `curation:` literals (store-build src/tests, cpe-server, access-server, sync-store mapping + ark_tests) and the `tests/it` imports follow. Workspace tests compile; affected crates' clippy `--all-targets -D warnings` clean and tests green | — |
| 4.4 | dsp-repository | done | 605f6cde | `media/data/PROVENANCE` entry for `0803/config.kdl` + `0803/narrative/`: hand-authored in dsp-incubator `cpe/projects/incunabula` at d68f214b, brought in by DEV-7465 (b20d49dc), moved here by DEV-7511/DEV-7512; change in place, incubator follows on re-vendor; remodel change moves the store pins, a named-but-absent narrative leaves the project not ready | — |
| 4.6 | dsp-repository | done | b03b55df | Review fix round 1 (`fixup!` of d2dda992): public `CuratedValue::key` doc linked to `pub(crate)` `is_curation_name`; `RUSTDOCFLAGS=-D warnings cargo doc -p cpe-store-build` failed (`private_intra_doc_links`), now passes with the rule inline | — |
| 4.5 | dsp-repository | done | — (verification) | at 605f6cde: the plan's `rg -l` prints nothing; `cargo test -p cpe-server incunabula` 5/5, `INCUNABULA_STORE_*` untouched since 70237734; `just check` and `just test` exit 0 | — |
| 5.1 | dsp-repository | done | 1b27b799 | `just build-docker-dpe` stages `cpe/projects` (`mkdir -p "$stage/cpe"`, `cp -r areas/access/cpe/projects`), as `build-dpe` does; built in 5.4 | — |
| 5.2 | dsp-repository | done | 22d9b065 | `staging/media/data` in `build-dpe` and `just build-docker-dpe`; Dockerfile `COPY media/data`, `ENV MEDIA_DATA_DIR`, header (sync snapshots only; cpe/projects SHORTCODE, assets, documents; media deposited files); `.dockerignore` entry; dpe/operations.md data bullet, `MEDIA_DATA_DIR` row, `SYNC_DATA_DIR`/`CPE_PROJECTS_DIR` rows, preview note; cpe/operations.md "from what", five variables, media failure row plus two `SHORTCODE` rows (read from `activation.rs:55-63`, `discovery.rs:300`), measurements note, data sizes on 2026-10-21 (file-size sums). Image built; canary: `-e MEDIA_DATA_DIR=/nonexistent` answers 503 with the warning | — |
| 5.3 | dsp-repository | done | 55095cb5 | `areas/access/{cpe,sync,media}/**` in the preview and Scout filters, `media/**` in both a11y triggers (+ comment); `deployment.md` :20/:114/:116 and the `build-dpe` row (:79, names media's data), `repo_structure.md:117`, `security.md:26` (also gains `server/**`, which the scan already had) | — |
| 5.4 | dsp-repository | done | — (verification) | tree of 55095cb5: `just build-docker-dpe` builds `dpe`; `docker export` lists both media paths; the container on 18080 serves `/cpe/incunabula/` 200 with the title (canary `MEDIA_DATA_DIR=/nonexistent` → 503); `just check` and `just test` exit 0 (94 `test result: ok`, no failure). adbe4b75 after it is docs only | — |
| 5.5 | dsp-repository | done | adbe4b75 | Review fix round 1 (`fixup!` of 22d9b065, docs only): `cpe/operations.md` file counts 20/8 → 19/7 (`find -type f`; consistency, verified), measurements note without the move narration, size paragraph without the file list (simplicity); `MEDIA_DATA_DIR` row states the missing-directory case once (simplicity) | — |
| 6.1 | dsp-repository | done | 93206849 | `areas/access/media/CONTEXT.md`: **Stand-in directory**, **`LiveDepositedFiles`** (refusals read from `media-store/src/lib.rs`), **Anonymous consumer**; "What `media` does not do yet" (request-time routes and CPE's `assets/` interim, render/derive, DaTEI's port, the bucket's adapter); boundary note (seam rule 4 checked, the rest review) | — |
| 6.2 | dsp-repository | done | dc9db58a | `cpe/CONTEXT.md`: intro (two ports, Incunabula's files in `media/data/0803/`); Project snapshot without curation; **Curation** (parse at activation, absent file vs `reads_curation`); **Bytes port** built (`DepositedFiles`, adapters, errors, stricter `is_file_name` → `NarrativeFileNotFound`, the one-line `0801a`–`0801e` reason); Project id names `SHORTCODE`; store builder signature with `&CuratedValues` (the `:142` half of the store-build README checkbox) and its tests' media reads; host over both adapters; `CpeConfig` names no data dir; **Activation** and **Unavailable project** as `port_sources` does it; `from_sql_folders` legacy; project folders + `assets/` interim; "What the port serves" without the curation row | — |
| 6.3 | dsp-repository | done | 199ed798 | `areas/access/CONTEXT.md`: `sync` serves no curation; `media` exists at its minimum (Vitrinli, routes, DaTEI's port still designed); CPE's two ports and both adapters. `datei/CONTEXT.md`: the stand-in directory exists for CPE; DaTEI's adapter waits for its port (decision #3) | — |
| 6.4 | dsp-repository | done | c65487cf | root `CONTEXT.md` Access Area line: `media` at its minimum, `sync` without curation, the rest of `media` planned, `media/CONTEXT.md` in the index; `vitrinli/CONTEXT.md` Access bucket names the stand-in directory | — |
| 6.5 | dsp-repository | done | fb405a5d | ADR-0007 gains `## Amendment (2026-10-21, DEV-7511, DEV-7512)` appended after the last one, existing text untouched (`git diff` shows additions only): header names both ports, text below the header pinned unchanged by `incunabulas_store_text_below_its_header_is_the_prototypes`, whole text re-pinned; the incubator's `fresh.rs` fails after its next re-vendor until `data.sql` is regenerated (worded per the intake note, not "heals by itself") | — |
| 6.6 | dsp-repository | done | 26f3a871 | `server/CLAUDE.md`: wires both adapters; image sets `MEDIA_DATA_DIR`; activation over `LiveDepositedFiles` on `MEDIA_DATA_DIR`, empty = unset, missing dir warned; image section names the three copies and `just build-docker-dpe` as a fourth staging place; all three path filters carry cpe/sync/media; dependency boundary names `media-store`/`LiveDepositedFiles` | — |
| 6.7 | dsp-repository | done | d781dfff | Incunabula `PROVENANCE.md` (intro and "The port and the pin": builder takes snapshot + curated values, curation row out of the incubator's `sync/data` table, re-sourced on its next re-vendor; `:33`, `:49`, `:55`, `:128`, `:155`, `:194`), `CURATION.md:110,126`, `DATA_ISSUES.md:249`, `assets/README.md:6,35` point at `areas/access/media/data/0803/` | — |
| 6.8 | dsp-repository | done | fdee2082 | `store-build/README.md`: intro names curated values; `build` with `&CuratedValues` and the caller flow through both ports; pin paragraph renamed to the two current tests (it named the pre-Phase-3 test name) and narrowed to the text below the header; tests list `curation_0803.rs` and the media reads; "Adding a project" names both ports' directories and `SHORTCODE`; `:210` curation path. `cpe/CONTEXT.md`'s signature landed in 6.2 | — |
| 6.9 | dsp-repository | done | 4a2d60f0 | `media/data/0803/config.kdl`: the first mention is `dsp-incubator cpe/projects/incunabula/DESIGN.md` (not in this repository), all 18 later ones "the incubator's DESIGN.md"; comments only, `cargo test -p cpe-server incunabula` 5/5 with both pins unchanged | — |
| 6.10 | dsp-repository | done | 20708397 | `media-store` after `sync-store` in `CONVENTIONS.md`'s crate-scope row and `docs/src/git-conventions.md`'s crate scopes (`just commit-lint` already accepted it, Phase 2) | — |
| 6.11 | dsp-repository | done | ea73f366 | `docs/src/repo_structure.md`: `media/CONTEXT.md` in the context list; tree `cpe/ports` (two ports), `cpe/projects` (SHORTCODE, assets, documents), `sync` without curation, new `media/` (store, data); crate rows `access-server`, `cpe-ports`, `cpe-store-build`, `cpe-server`, `sync-store`, new `media-store` (deps read from the manifests) | — |
| 6.12 | dsp-repository | done | a1f489d6 | `ARCH-MAP.md` by hand (script `.claude/tmp/p6/archmap.py`): overview (media at its minimum, four planned components, media's CONTEXT.md), `access/server` (both adapters, `MEDIA_DATA_DIR`, `data_dir`, image's three copies + `just build-docker-dpe`), `cpe` (both ports built, key entities `DepositedFiles`/`named_narratives`/`reads_curation`/`CuratedValues`, header-and-body pin rule, SHORTCODE/shared-shortcode/curation activation rules, adapters apply the name rules, test file reads, durable state), `datei` (adapter waits for its port), `sync` (no curation anywhere; the removed ADR-0010 guard test dropped from its rules), `media` active (paths, purpose, key entities, interface, kit, deps, used by, boundary rules with enforcement, durable state). Phase 2's deferral (`:945-946`, `:1012-1013`, `:24`, `:1181-1184`) is covered. Fingerprints recomputed with the map-format recipe (`.claude/tmp/p6/fp.sh`) on the committed tree: dpe `d5ce25de1ac8` (only `areas/access/CONTEXT.md` changed; the entry names none of it), server `ec2b5f259efa`, cpe `c67e66bd01d4`, sync `04c7968de2bd`, media `11feae9d5097` | — |
| 6.13 | dsp-repository | done | — (verification) | at a1f489d6: the plan's `rg` prints nothing; `just check` exit 0 (run before each of the 12 commits); `just test` exit 0 (94 `test result: ok`, no failure) | — |
| 6.14 | dsp-repository | done | 8291401b, e55e8313, a9598f85, 4fdb17f8, 0b5c6d17, c9a4fc94, e785246a, 664a9c60 | Review fix round 1 (`fixup!` commits). Verified: a folder with no `SHORTCODE` is not logged (consistency+rust; `activation.rs:54-63`); `server/CLAUDE.md` heading counted three places (consistency); `repo_structure.md:7` "Eleven ADRs"/ADR-0010 "proposed" (ivan, dune DUNE-003; ADR-0010 frontmatter `superseded by ADR-0012`); ARCH-MAP cpe `:1041` "no asset's bytes are read at activation" contradicted the `assets/` interim (ivan); ADR-0007's 2026-10-13 "until then" left open, and `SHORTCODE` discovery unrecorded in an ADR (dune DUNE-001, ivan) → closing bullet in this phase's own 2026-10-21 amendment (existing text untouched, title widened); bytes-port name rules labelled static-analysis for any adapter (dune DUNE-002) → "review for a new one"; vitrinli entry "areas/access/media, both planned" (consistency); `bacon.toml` serve job did not watch `areas/access/media` (consistency; `fixup!` of 9be2540b); simplicity: media/CONTEXT intro repeated the not-yet section, config.kdl header parenthesis hard to parse. Fingerprints recomputed: server `3d97e3048a9e`, cpe `12390ac23a99`, media `5f99329f6370`. `just check`, `just test` exit 0 | — |
| 6.15 | dsp-repository | done | 0d1f0829, e4141b2d, bdd9a1db | Review fix round 2 (re-review of round 1 by consistency, simplicity, dune; `fixup!` commits): `repo_structure.md:7` "Five concern how the Access Area grows" now lists six (all three reviewers) → "Six"; "All but ADR-0010 are accepted" → "ADR-0010 is superseded; the others are accepted" (dune); `media/CONTEXT.md` intro and ARCH-MAP vitrinli "Used by" rewrapped; media fingerprint `83a21ae670cc`. `just check` exit 0; prose only, no re-review (cap of 2 rounds reached) | — |
| F.1 | dsp-repository | done | edc56fd5 | Final review F1: `cpe-ports` `contract.rs` `is_plain_data_ark` calls `crate::is_shortcode` instead of re-inlining it; `sync-store` `ark.rs` `data_ark` keeps its case-blind check, with a two-line comment naming `cpe_ports::is_shortcode` as the rule its upper-cased output meets. `cpe-core/src/validate.rs`'s copy left (seam rule) | — |
| F.2 | dsp-repository | done | 3273368e | Final review F2 (`fixup!` of 93206849): `media/CONTEXT.md` closing paragraph: a new consumer declares its own port in its own ports crate, `media-store` implements it beside `LiveDepositedFiles`; `is_shortcode`/`is_file_name` stay CPE's until a second consumer, moving them is an ADR or amendment | — |
| F.3 | dsp-repository | done | 217c30a2 | Final review F3 (`fixup!` of adcd9025): `activation.rs` `failing_projects` gains `curation-outage` (`0018`, `with_unavailable_file("0018", "curation.csv")`, a config that does not read curation), so both the cause test and the 503 test cover it; canary: without the fault both tests fail | — |
| F.4 | dsp-repository | done | 37b6fd3e | Final review F4 (`fixup!` of 55095cb5): `.github/actions/build-dpe/**` in the `cloud-run-dpe-pull-request.yml` and `scout-dpe.yml` filters and in `deployment.md`'s preview paragraph. Not `a11y-dpe.yml`: it builds with `cargo build` directly and does not use the action | — |
| F.5 | dsp-repository | done | b50c7161 | Final review F5 doc trims: `curation_file.rs` `text` doc; `port_sources` doc down to the complete-inventory point; `operations.md` measurement dated (curation then in `SYNC_DATA_DIR`, since moved to `MEDIA_DATA_DIR`); `FilesError::Unavailable` source may carry an absolute path, logged never rendered. After the last fix: `just check` exit 0, `just test` exit 0 (94 `test result: ok`) | — |
| 7.1 | dsp-repository | done | a0b9c0e8 (squashed into b0848ac0) | ARCH-MAP fingerprints recomputed with `fp.sh` on a clean tree: cpe `12390ac23a99` → `48f80caf7cdc`, sync `04c7968de2bd` → `919317b431f9`, media `83a21ae670cc` → `13c4559ee397`; dpe and access/server unchanged; datei/vitrinli stay `none`. Unchanged by the rebase (main's diff touches only `areas/deposit/editor` and `docs/src/editor`) | — |
| 7.2 | dsp-repository | done | b0848ac0 | `git rebase --onto 6b1255fc e528da68` (68 commits, no conflicts), then `git reset --soft 6b1255fc` + one commit. Squashed tree `56109c18` equals the rebased head's tree. `git diff 8a7dd1d6 HEAD --stat`: ARCH-MAP.md (3 fingerprints) plus main's `areas/deposit/editor/collector/src/{forge,run}.rs` and `docs/src/editor/collection.md`, the same stat as `e528da68..6b1255fc`. `just commit-lint 6b1255fc` passes (1 commit); `just check` exit 0; `just test` exit 0. Phase review = this tree-equality check, no new code | — |

## Deferrals

- Intake (2026-10-21): `eng:test-browser` (eng.yaml, `areas/*/*/web/**`) skipped by the owner: the plan touches only
  `cpe/web/tests/validation.rs` there; `INCUNABULA_PAGES` and Phase 5's docker curl check cover the served pages.

- Phase 2 review (scope; DUNE-001, Warning): `media`'s own boundary (ARCH-MAP: depends on `cpe-ports`, later
  `datei/ports`, never `sync-store` or a sibling's crates) is not enforced: `media-store` could add `sync-store` or
  `cpe-server` and pass every gate. A new gate rule (or `check-capability-deps.sh` for the Access Area) is outside
  this plan's checklist; for the session's final review.
- Phase 2 re-review (consistency, to Phase 6's ARCH-MAP checkbox, which already covers the `cpe` and `media`
  entries): `ARCH-MAP.md:945-946` ("a dev-dependency on `sync-store` excepted") and `:1012-1013` name only
  `sync-store` beside rule 4, and `:24`/`:1181-1184` still say `areas/access/media` "holds nothing yet". Add
  `media-store` there when the entries are refreshed.
- Phase 3 close-out (session, out of scope): `cargo clippy --workspace --all-targets -- -D warnings` fails only in
  `mosaic-playground` (`src/app.rs:188`) and `dpe-api-oai` (`metadata/corpus.rs:586,621`); `git diff
  e528da68..HEAD -- modules/mosaic areas/access/dpe` is empty, so the lints predate this branch. Every crate the
  branch touches is clean; the plan's checkbox is split accordingly. For a separate fix.

- Phase 5 review (scope; dune DUNE-001 Warning, ivan Warning): the image's "three places that have to agree"
  (staging in `build-dpe` and `just build-docker-dpe`, `COPY`, `ENV`) are kept by hand; a missing `cp` or `COPY`
  fails the build, but a missing or wrong `ENV` builds an image whose CPE answers 503 behind a start-up warning.
  A gate (a script in `just check` comparing the Dockerfile's `COPY`/`*_DIR` `ENV` with both staging blocks, or one
  shared staging script) or a CI smoke request to `/cpe/incunabula/` on the built image is a new check outside this
  plan's checklist; for the session's final review. Same shape for the three DPE path filters (DUNE-003, Suggestion).
- Phase 6 review (dune DUNE-001, Warning; not fixed here): ADR-0010 (`:10`, "retired under DEV-7511 and is
  provisional until then") and ADR-0012 (`:33`, `:90-107`, "is retired under DEV-7511") still speak of DEV-7511 in
  the future tense. ADR-0007 is closed by its 2026-10-21 amendment; whether ADR-0010 and ADR-0012 (the latter from
  PR #494) get a dated "done" note is the session's call, since ADR text is the owner's.

## Deviations

- Proposed (Phase 1 re-review, consistency; not committed): the decided `is_shortcode` (four upper-case hex digits)
  refuses the five-character shortcodes `0801a`–`0801e` that root `CONTEXT.md:35` and
  `docs/src/editor/authentication.md:163` document, so the bytes port can never serve such a project.
  Committed behaviour is unchanged; only the doc now says so (51edb03e). Owner to decide whether the rule should
  admit them (e.g. `[0-9A-F]{4}` plus an optional lower-case letter) before Phase 2's adapter and Phase 3's
  discovery rely on it. No CPE project uses one today.
  **Owner's answer (2026-10-21): keep** four upper-case hex digits. `0801a`–`0801e` are a metadata-only split of
  BEOL, one DSP project (`0801`), so the archive never publishes an Access-bucket prefix under them. The `is_shortcode`
  doc and `cpe/CONTEXT.md` say why in one line (Phase 2 for the doc, Phase 6 for `CONTEXT.md`).

## Side findings

- Intake (2026-10-21): the handoff's unverified assumption holds. dsp-incubator's
  `cpe/store-build/tests/it/fresh.rs` fails when the committed `data.sql` is not the builder's output, byte for byte,
  and its re-vendor procedure regenerates `data.sql` with `just cpe store-build incunabula --inplace`
  (`cpe/projects/incunabula/PROVENANCE.md:5-6, 57-58`).
- Phase 1 review (suggestion, dune/ivan): the bytes port has no shared adapter-conformance check like
  `contract::violations`; the refusal rules are pinned per adapter (fake here, `media-store` in Phase 2's tests). A
  `cpe_ports` helper taking `&dyn DepositedFiles` and asserting the refusals would make it one call per adapter.
- Phase 1 review (suggestion, security): `is_file_name` has no length or depth bound; an over-long name reaches the
  OS as `ENAMETOOLONG`, which Phase 2's adapter maps to `Unavailable` (any kind but `NotFound`/`NotADirectory`).
  Windows reserved device names (`CON`, `NUL`) pass, refuted as a finding: the adapter runs only on Linux/macOS.
- Phase 1 review (suggestion, consistency): `contract.rs:393` hand-rolls the same four-upper-hex check for an ARK's
  shortcode; it could call `is_shortcode`. Left as is (outside this phase's checklist).
- Phase 2 review (refuted or decided, not fixed): a name over `NAME_MAX`/`PATH_MAX` and a directory under the file's
  name are `Unavailable`, and the read has no size cap (security, rust). All three are decided (plan: "anything else
  is `Unavailable`", "`IsADirectory` is `Unavailable`", "no size cap yet"); the "anonymous caller forces a 5xx"
  premise does not hold, since the port's only caller is CPE's activation over names from the KDL, and `media` has
  no request-time route. Revisit with the request-time routes or the real bucket's adapter.
- Phase 2 review (suggestions, not taken): span fields of `#[instrument]` carry the caller's strings (as
  `sync-store`'s do); `count_lines` prints 1 for an empty set (unreachable behind the absence checks); media twins
  of the sync exemption-form cases (the matcher is shared); move the adapter to `src/files.rs` (DUNE-003); a
  shared `DepositedFiles` conformance helper (Phase 1 side finding, still open).
- Phase 2 note: match the fake's refusal order in `LiveDepositedFiles`: `is_shortcode` → `UnknownProject`, then
  `is_file_name` → `UnknownFile`, both before any `symlink_metadata` (trait doc on `DepositedFiles::file`).
- Phase 3 (gate, not fixed): `cargo clippy --workspace --all-targets -- -D warnings` fails on code this branch
  never touched: `modules/mosaic/playground/src/app.rs:188` (`items_after_test_module`) and
  `areas/access/dpe/api-oai/src/metadata/corpus.rs:586,621` (dead `PREVIEW_ORIGIN`, `ark_paths` under
  `cfg(test)`). `just check` lints without `--all-targets`, so CI does not see them. The Phase 3 clippy
  checkbox is left unticked for the session.
- Phase 3 note: the activation discovery fixtures are built per test in tempdirs (`discovery.rs` tests), as
  the existing symlink cases are; a symlinked `SHORTCODE` cannot be committed portably. The
  `tests/fixtures/discovery/*` fixtures of `from_sql_folders` are untouched.
- Phase 3 note: the copied curation reader is `cpe_store_build::curation_file` (tests `curation_file_tests.rs`),
  not `curation`/`curation_tests.rs` as the plan names it: a root `curation` module would shadow the
  `generic::curation` overlay that `lib.rs` imports by that name.
- Phase 3 review (refuted): DUNE-001, Incunabula's `PROVENANCE.md:33,55` naming `projects/incunabula/config.kdl`,
  is Phase 6's checkbox "Repoint path references in Incunabula's `PROVENANCE.md` …". Ivan's Warning, a malformed
  `SHORTCODE` answering 404 and not 503, is the plan's decided behaviour ("a malformed one is skipped with a
  warning, as a folder whose name fails the id rule is today").
- Phase 3 review (suggestions, not taken): split `DiscoveredProject` into an activation type (shortcode,
  refusal not optional) and a legacy folder type with the file readers, so `port_sources` loses its unreachable
  no-shortcode branch and cannot call `narrative_contents` (DUNE-002, simplicity; natural at DEV-7498); gate
  `CuratedValues::from_unchecked` behind a `test-support` feature (DUNE-003, rust; owner chose `#[doc(hidden)]`);
  split `port_sources` into narrative and curation helpers; drop the whole-text pin in favour of the body pin
  and the `sql.rs` header tests; move `the_things_fixture_reads_no_curation` out of `cpe-core` (it
  `include_str!`s a `cpe-server` fixture); a debug log of absent narrative variants (ivan); `SHORTCODE` read
  with `O_NOFOLLOW` on one handle (security, TOCTOU by an operator only); the `serve.rs` traversal test's
  `0001` marker could match an unrelated 404 body; `diagnostic.rs:103`, `ir.rs:1195`,
  `components/src/component/context.rs:168` still say "`narrative/` directory" (cosmetic, the port name keeps
  `narrative/`).
- Phase 3 review note for Phase 4 (consistency): `committed_0803.rs:521,548,556,586-594` build through the
  `build(snapshot)` wrapper from `snapshot.curation` and compare with `STORE`, built from `CURATION`; equal only
  while sync's and media's CSVs agree. Phase 4's `without()`/reordered-build checkboxes already cover the move.
- Phase 3 review (dune, not a finding): until Phase 5 stages `media/data` in the image, an image built from this
  branch answers 503 on Incunabula (`DEFAULT_MEDIA_DATA_DIR` is checkout-relative and the Dockerfile sets no
  `MEDIA_DATA_DIR`). Phases 3 and 5 ship in the same PR.
- Phase 3 re-review of round 1 (consistency, simplicity, dune): clean. The `None` -> `""` key in the shared-
  shortcode grouping is unreachable (see below)
- Phase 4 review (refuted by measurement): ivan's "no committed test reads the real `0803.nq` against the real
  `media/data/0803/curation.csv`": `tests/it/main.rs`'s `CURATION` does exactly that and panics on a fault, and
  sixteen `curation_0803.rs` tests plus every `COMMITTED` build read it. Rust's "a dangling curated value via
  `from_unchecked` builds unrefused": `parse` refuses it (`UnknownResource` test in `curation_file_tests.rs`) and
  activation is the only production constructor (security confirmed); `from_unchecked` stays `#[doc(hidden)]` by
  the owner's decision. Simplicity's "the `build(&COMMITTED) == STORE` guard is pasted twice": each test must hold
  it itself, so it cannot pass vacuously when run alone; kept. Lines over 180 characters in `sync/CONTEXT.md`: none
  (`awk 'length>180'` empty).
- Phase 4 review (suggestions, not taken): gate `from_unchecked` behind a `test-support` feature or a CI grep
  (security, dune DUNE-001; owner chose `#[doc(hidden)]`, already in Phase 3's list); `COMMITTED`'s doc as a
  constraint ("derive committed-data builds from this, not `SNAPSHOT`") and trimming `build`'s third sentence;
  `rebuild.rs` doc "no adapter and no file" vs a `Vec` of curation; the ticket trail in `media/data/PROVENANCE`
  (the plan asks for the PR there); a `Fixture::with_facts` helper; a CPE-side guard that no generic source names a
  curated key (the dropped ADR-0010 sync guard; `hygiene.rs` covers project nouns, not keys); the `.gitattributes`
  glob covers only files named `curation.csv` (that is the port's only curation name).
- (earlier note) the `None` -> `""` key in the shared-shortcode grouping is unreachable (`discover_for_activation` builds every project with `Some`); a `let Some`
  guard would make that loud (suggestion, not taken). `just check`/`just test` exit 0 at 6e8d4322.
- Phase 5 verification (5.4): `just build-docker-dpe` (host arch, arm64) built `dpe`; `docker export` lists
  `app/media/data/0803/config.kdl` and `curation.csv` (and 16 narrative files); the image's `ENV` has
  `MEDIA_DATA_DIR=/app/media/data`; on port 18080 `/cpe/incunabula/` answered 200 with one title match and
  `served=1 unavailable=0`. Canary: with `-e MEDIA_DATA_DIR=/nonexistent` it answers 503 with the start-up warning.
  Containers removed, `dpe-media` stopped. Scripts in `.claude/tmp/p5/`.
- Phase 5 review (refuted or decided, not fixed): ivan's "record the end state of the cpe/projects + media/data
  split" is Phase 6's `cpe/CONTEXT.md` checkbox and decision #4; DUNE-004 (`.dockerignore` comment misleading): the
  comment says no image built from the repo-root context reads the data, which holds; consistency's "a file a project
  needs hides that an absent `curation.csv` is fine without a curation key": the wording is correct; simplicity's "the
  image contents are stated in three places" (data bullet, "from what", env table): each is a plan checkbox and an
  existing structure.
- Phase 5 review (suggestions, not taken): add `.github/actions/build-dpe/**` (and `justfile`) to the preview and
  Scout filters, so a staging break shows before main (devops; predates this branch); a `head.repo` guard on the
  preview's `cleanup-preview` job and SHA-pinned actions (security); an allowlist for what `build-dpe` copies from
  `media/data` (security); a test that no CPE response carries the curation CSV's `#note` column (security; the notes
  are not read into the store); one canonical list of the DPE path filters, linked from the four docs (simplicity,
  devops, ivan); the `MEDIA_DATA_DIR` row's caveat as a note under the table; `Empty is treated as unset` on the
  `SYNC_DATA_DIR` row is outside media (simplicity; it documents Phase 3's `data_dir`); a pointer to the command
  behind the data sizes (dune DUNE-005/006).
- Phase 5 re-review of round 1 (consistency, simplicity, dune): clean; suggestions only (the start-up warning is
  logged only when CPE is mounted; "then" in the measurements note).
- Phase 6 review (refuted or decided, not fixed): simplicity's "the same fact in five or six places" (assets interim,
  DaTEI waits, activation steps, the pins, empty data-dir variable): each file is a plan checkbox for its own reader
  (CONTEXT, ARCH-MAP entry, README, crate CLAUDE.md), and each copy was checked against the code; the ADR
  amendment's "a pinned falsehood is worse than a re-pin" is the decision's rationale, which belongs in an ADR;
  the `openat2`/`cap-std` line in `media/CONTEXT.md` constrains the adapter that replaces this one; defining
  "DESIGN.md" once in `config.kdl` instead of rewording each mention: the checkbox asks for each mention.
- Phase 6 review (suggestions, not taken): add `store-build/README.md` "Adding a project" to the media entry's kit
  (DUNE-004); `docs/src/cpe/operations.md:58` dated measurement "then all in `SYNC_DATA_DIR`" (DUNE-005, a dated
  record); a rule that nothing non-public is committed to `media/data` (ivan; `media/CONTEXT.md` says the stand-in
  holds public files only); a gate for the image's staging places (already deferred, Phase 5); ARCH-MAP lines over
  180 characters (the file's existing style; the diff extended two, `:24` and `:957`); a forward pointer at
  ADR-0007 `:434` to the new amendment (existing text stays unchanged by decision #6).
- Phase 7: after the rebase onto 6b1255fc, ARCH-MAP's `areas/deposit/editor` fingerprint (`3ab6a7e4f739`) no longer matches
  the tree (`755ca5a9e411`): main's collector change (`areas/deposit/editor/collector/src/{forge,run}.rs`) did not refresh it.
  Not this PR's component and outside Phase 7's one content change; left for a `/dune:dune-map` refresh.

## Closeout

- root_cause: CPE's build-time inputs followed the superseded ADR-0010. `config.kdl` and `narrative/` lived in CPE's
  project folder and the curation travelled inside `sync`'s snapshot, so `sync` served a non-fact, a project folder
  held data, and the Access bucket (the archive's second channel) had no place to enter the Access Area.
- investigation: the plan was settled before execution (four plan-review cycles); execution changed no decision.
  The one owner question mid-run was the shortcode rule refusing `0801a`–`0801e` (kept: a metadata-only split of
  BEOL, no bucket prefix). The large phase (3) held because a header-stripped pin of Incunabula's store text was
  written against the unchanged code first and never moved. Curation moved in two steps (copy in Phase 3, retire in
  Phase 4) so every phase compiled; a canary emptying the committed curation made 6 tests fail, proving no test lost
  it. The final review found no Critical; its fixes were a second copy of the shortcode rule in `contract.rs`, an
  ownership note for a second `media` consumer, a missing unreadable-`curation.csv` test and a path-filter gap.
- solution: `cpe-ports` gains the bytes port `DepositedFiles` (+ `FakeDepositedFiles`, `is_shortcode`,
  `is_file_name`); `areas/access/media` arrives with `media-store`'s `LiveDepositedFiles` over `MEDIA_DATA_DIR`
  (per-component symlink refusal, errors by kind); Incunabula's KDL, narrative and curation move to
  `areas/access/media/data/0803/` with a `SHORTCODE` file in the project folder; CPE discovers by `SHORTCODE` and
  activates from both ports, fetching only the narrative files the KDL names; the curation reader moves to
  `cpe-store-build` (`CuratedValues`), and `ProjectSnapshot.curation`, the contract's curation rules and `sync`'s
  reader and CSV are retired; the store header names both ports; the image ships `media/data`, `just
  build-docker-dpe` is fixed, and PR previews and Scout run on CPE, `sync` and `media` changes; CONTEXT files,
  ARCH-MAP, repo docs and an ADR-0007 amendment follow.
- prevention: the header-stripped store pin; `check-cpe-seam.sh`'s `media` edge with tests; adapter refusal tests
  for every path-building input; not-ready (503) tests for each failure mode; open follow-ups in `## Deferrals`
  (media-store's own dependency boundary, an image staging/COPY/ENV check, ADR-0010/0012 tense,
  workspace-wide clippy in untouched crates).
