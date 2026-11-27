# Execution Journal Archive: 01-refactor-harden-cpe-engine-plan

## Phase 1

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | areas/access/cpe/server/src/{lib,activation,discovery}.rs | — | "loader compiles under cfg(test) only", "dead helpers gated", "tests still pass", "doc comments say what they are for" | non-test build has no `from_sql_folders`/`discover_sql_folders` and no dead-code warning; cpe-server tests pass unchanged | lib.rs § Cpe::from_sql_folders; activation.rs § from_sql_folders; discovery.rs module doc, DiscoveredProject impl, discover_sql_folders | — |
| 1.2 | dsp-repository | areas/access/cpe/CONTEXT.md, ARCH-MAP.md | 1.1 | "CONTEXT.md describes the loader as test support", "ARCH-MAP.md does the same" | no "ends with DEV-7498"/legacy wording for the loader; described as test support | CONTEXT.md:300-303; ARCH-MAP.md:1350,1456-1458,1500 | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | bfebb31f | loader and its dead discovery helpers gated `#[cfg(test)]`; thiserror moved to test deps | none |
| 1.2 | dsp-repository | complete | 995e3fc9 | CONTEXT.md and ARCH-MAP.md describe the loader as test support; ARCH-MAP enforcement review -> structure | none |
| 1.3 | dsp-repository | complete | 19a353e3 | review fix round 1: `just check` rustfmt rewrap of the discovery module doc (fixup of bfebb31f) | none |

## Phase 2

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 2.1 | dsp-repository | areas/access/cpe/server/src/{activation,router,follow,unavailable}.rs | — | Phase 2 tests 1-10, code items (unavailable list, hidden_ids, log, info level, comments) | targeted bazel tests green; unknown → site 404, failed → 503 | plan § Phase 2 (Proposed Solution + Implementation Phases) | — |
| 2.2 | dsp-repository | docs/adr/0007-*.md, ARCH-MAP.md, areas/access/CONTEXT.md, areas/access/cpe/CONTEXT.md, docs/src/cpe/operations.md | 2.1 | Phase 2 doc items (ADR-0007, ARCH-MAP, CONTEXTs, operations.md status table + Releasing section, Hidden project entry) | docs state 404 for unknown, 503 for failed; ADR current-only | plan § Phase 2 | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 2.1 | dsp-repository | done | c43b4fa7 | unknown project answers CPE's 404 (not in unavailable list), hidden_ids from served+failed, info log with shortcode; 10 tests incl. broken token/missing generation/empty location; comments in activation/router/unavailable/lib | — |
| 2.2 | dsp-repository | done | 1eacfec3 | ADR-0007 rewritten in place (not ready 503 vs not there 404); ARCH-MAP prose, both CONTEXTs (unknown project, released, Hidden project), CPE/DPE operations, repo_structure; Releasing section | — |
| 2.3 | dsp-repository | done | 509b0683 | review fix round 1 (fixup of c43b4fa7): module docs of router/activation state the 404; hidden_ids matches Loaded or Failed positively | — |
| 2.4 | dsp-repository | done | d5fb0521 | review fix round 1 (fixup of 1eacfec3): DPE operations row no longer says a missing token answers 503 (DUNE-002); ADR uses defined terms, not "unreleased" (DUNE-003); cpe CONTEXT _Avoid_ reworded; duplicate token-removal sentence dropped | — |

## Phase 3

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 3.1 | dsp-repository | areas/access/cpe/server/src/{lib.rs,activation.rs,discovery.rs} | — | Phase 3: cpe-server tests (3), load_all keeps reports, lib.rs defines function and outcome type | cpe-server tests pass; facade test passes | plan Phase 3 "the verb"; activation.rs load_all | — |
| 3.2 | dsp-repository | areas/access/dpe/server/src/{lib.rs,validate.rs}, areas/access/server/src/main.rs | — | Phase 3: dpe_server::validate returns a combinable result | dpe-server and access-server build and test | dpe validate.rs:3 | — |
| 3.3 | dsp-repository | areas/access/server/src/{main.rs,serve.rs,validate.rs,cli.rs} | 3.1, 3.2 | Phase 3: access-server tests (5), shared config function, verb checks DPE then CPE, help text | access-server tests pass | serve.rs:293-302 | — |
| 3.4 | dsp-repository | areas/access/server/CLAUDE.md, docs/src/dpe/operations.md, docs/src/cpe/operations.md, docs/adr/0007-*.md | 3.3 | Phase 3: CLAUDE.md, dpe and cpe operations, ADR-0007 | docs describe the verb | ADR-0007 l.121, l.323 | — |
| 3.5 | dsp-repository | — | 3.4 | Phase 3: just check, just test | both pass | — | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 3.1 | dsp-repository | done | 17fd058e | `cpe_server::validate` with `Validation`/`ProjectReport`/`ProjectOutcome` in lib.rs; `ProjectStatus` carries report and rendered warnings; skipped folders (bad SHORTCODE, bad name) reported failed; `SkipReason` Display | — |
| 3.2 | dsp-repository | done | 54574c31 | `dpe_server::validate(&Path) -> bool`; `main.rs` maps it to the exit code | — |
| 3.3 | dsp-repository | done | 41048873 | `validate.rs` (verb logic, writes CPE's report to a writer), `serve::cpe_setup_from_env` shared by both verbs, main wiring, help text; serve test with the same broken fixture | — |
| 3.4 | dsp-repository | done | 51b74f07 | server CLAUDE.md section and boundary list, DPE/CPE operations, ADR-0007 strict-mode bullet and Not-yet-in-code line, ARCH-MAP prose (key entities, cpe-server items), cpe CONTEXT host-crate paragraph | — |
| 3.6 | dsp-repository | done | 039be881 | review fix (rust W): `validate` uses `load_each`, builds no router; skipped folder name escaped in its report (security W) | — |
| 3.7 | dsp-repository | done | de981e44 | review fix (security W): report escapes control characters in names, shortcodes, report lines; test | — |
| 3.8 | dsp-repository | done | 708d32de | review fix (consistency W x2, dune DUNE-001/002, devops): ARCH-MAP validate caller and kit, workflows.md, CONTEXT and ADR-0007 on refused folders and `served`, image note | — |
| 3.5 | dsp-repository | done | — | `just check` and `just test` pass after the fix round (102 tests) | — |

## Phase 4

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 4.1 | dsp-repository | areas/access/cpe/core/src/validate.rs | — | NamedNarratives default language | `NamedNarratives.default_language` (site default, else first language) and a method listing named files with neither base nor default-language variant; cpe-core test | plan Phase 4 + "Phases 3 and 4", Phase 4 bullet 2; validate.rs check_narrative_file | — |
| 4.2 | dsp-repository | areas/access/cpe/server/src/{lib.rs,activation.rs,inputs.rs}, areas/access/server/src/validate.rs (call site) | 4.1 | data-free tests (config parse, narrative, mapping shortcode, malformed SHORTCODE), inputs-dir-missing / no-project-folder half of the run test, validation function runs the data-free check | `cpe_server::validate(.., inputs)`; per-project inputs outcome; `Validation::passes` fails on a failed inputs check; error for a missing inputs dir and for no project folder | lib.rs `validate`, activation.rs `port_sources` | — |
| 4.3 | dsp-repository | areas/access/server/src/{cli.rs,main.rs,validate.rs}, areas/access/server/CLAUDE.md | 4.2 | note test, must-serve test, no-folder/missing-inputs run test, `--inputs`, `--must-serve` | verb flags; must-serve fails when unknown/failed/absent; note for a project with neither stand-in nor inputs | validate.rs `cpe_report` | — |
| 4.4 | dsp-repository | justfile, .github/workflows/test.yml, docs/adr/0002*, docs/src/cpe/operations.md, docs/src/workflows.md, docs/src/bazel.md, CONVENTIONS.md | 4.3 | recipe, test.yml step, ADR-0002, operations.md, workflows/bazel.md, CONVENTIONS.md | `just validate-cpe` passes with 0803 served; required job runs it after `just test` | justfile `validate-data` | — |
| 4.5 | dsp-repository | — | 4.4 | validate-data on parent, `just validate-cpe`, `just check`, `just test` | gates pass | — | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 4.1 | dsp-repository | done | 1761e66f | NamedNarratives.default_language and `unresolved` (base or default-language variant), cpe-core test | — |
| 4.2 | dsp-repository | done | 6767cb74 | cpe-server data-free check (inputs.rs), `validate(.., inputs)`, `ProjectReport::inputs`/`InputsCheck`, errors for no project folder and a missing inputs dir; `remodel_of` shared with activation | — |
| 4.3 | dsp-repository | done | 4df29e95 | access-server `--inputs`, `--must-serve` (clap, shortcode-checked), must-serve check, inputs lines and note in the report; access-server CLAUDE.md, ARCH-MAP prose | — |
| 4.4 | dsp-repository | done | d229a732 | `just validate-cpe`, test.yml step after `just test`, ADR-0002 names the read, operations.md section, workflows/bazel/deployment/testing-strategy docs, CONVENTIONS.md | — |
| 4.R1 | dsp-repository | done | e5cc322d | review fix round 1: shared `narrative_resolves`, `ProjectReport::failed`, no followed `narrative/` link, must-serve lines skipped after a whole-check error, docs | — |
| 4.R2 | dsp-repository | done | cb4f6fab | review fix round 2: `narrative_resolves` owns the no-default case, one no-follow probe, `narrative` not a directory fails alone, `not_served` guard inside, docs | — |
| 4.5 | dsp-repository | done | — | gates after the fix rounds: `just test` (102 pass), `just check`, `just validate-cpe` (0803 served, inputs checked); `just validate-data` passed on the parent tree before the first change | — |

## Phase 5

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 5.1 | dsp-repository | areas/access/cpe/components/src/lib.rs, areas/access/cpe/components/BUILD.bazel, ARCH-MAP.md | — | Phase 5 items 1-6 | test counts PreEscaped( per file (cut at #[cfg(test)] mod), pins editorial 10, markdown 2, compound_viewer 1, annotation 1; fails on a changed count; sources as data; ARCH-MAP names the test | plan "Phases 5 and 6" para Phase 5; dpe/server/src/metadata.rs this_crate_splices_pre_escaped_markup_exactly_once | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 5.1 | dsp-repository | done | 9319699e | test `pre_escaped_call_sites_match_the_pinned_table` in components/src/lib.rs pins editorial 10, markdown 2, compound_viewer 1, annotation 1 (cut at first `#[cfg(test)]\nmod`); test target gets src as data + relative CARGO_MANIFEST_DIR (absolute sandbox default is gone at run time); ARCH-MAP names it | — |
| 5.R1 | dsp-repository | done | d577b343 | review fix: test comment drops `shared-datastar` (no PreEscaped there; reference-reviewer) and states it does not see code below a file's first test module (rust-reviewer) | — |

## Phase 6

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 6.1 | dsp-repository | areas/access/cpe/web-e2e-tests/{package.json,package-lock.json,tsconfig.json,biome.json,.gitignore,playwright.config.ts} | — | suite files; playwright.config.ts; throws without ACCESS_SERVER_BIN | `npx playwright test --list` throws without the binary; lists projects with it | plan §Phase 6 + editor suite config | — |
| 6.2 | dsp-repository | justfile, .github/workflows/check.yml, .github/workflows/a11y-cpe.yml | 6.1 | test-e2e-cpe; test-a11y-cpe; install/lint-e2e; check.yml lint-e2e; a11y-cpe.yml | recipes run the suite; workflow shaped like a11y-dpe.yml | justfile:645-745, a11y-dpe.yml | — |
| 6.3 | dsp-repository | areas/access/cpe/web-e2e-tests/tests/*.spec.ts, tests/support/* | 6.1, 6.2 | overview, browse, book, strips, narrative, accessibility specs | both projects pass | plan §Phase 6 spec table | — |
| 6.4 | dsp-repository | areas/access/cpe/{components,web,server}/src/** (only if a no-JS defect is small) | 6.3 | no-JS failures fixed or fixme | each no-JS defect fixed or `test.fixme` naming it | owner decision DEV-7510 triage | — |
| 6.5 | dsp-repository | docs/adr/0004-hypermedia-frontends.md, docs/src/dpe/testing-strategy.md, docs/src/deployment.md, docs/src/bazel.md, docs/src/cpe/operations.md, areas/access/server/CLAUDE.md | 6.2 | ADR-0004; testing-strategy; deployment/bazel/operations/CLAUDE.md | docs name the suite, recipes, workflow | plan §Phase 6 last paragraph | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 6.1 | dsp-repository | done | 77a2dc9f | Suite files copied from the editor suite (lock renamed); config: chromium-js/chromium-nojs, webServer.url on /cpe/incunabula/, 120 s timeout, throws without ACCESS_SERVER_BIN | — |
| 6.2 | dsp-repository | done | 2a979b52 | test-e2e-cpe / test-a11y-cpe (no CSS step); install-e2e-requirements, lint-e2e, check.yml lint-e2e gain the CPE dir; a11y-cpe.yml on areas/access/**, shared/**, modules/mosaic/** + Bazel inputs, ubuntu-24.04, local bazel-setup | — |
| 6.3 | dsp-repository | done | b2cac079 | Six specs; fixture aborts other origins; locale en-US and reducedMotion in config; 48 pass, 1 skip (Back/Forward, nojs), 4× repeat green; no no-JS defect found, so 6.4 is void | — |
| 6.5 | dsp-repository | done | c5662970 | ADR-0004 Not-yet-in-code line removed, Enforced by names CPE pass; testing-strategy, cpe/operations, bazel.md, deployment.md, repo_structure.md, access-server CLAUDE.md name suite/recipes/workflow | — |
| 6.4 | dsp-repository | done | — | void: the no-JS pass found no broken path, so nothing to fix or mark fixme | — |
| 6.R1a | dsp-repository | done | 06d6b0f3 | Review fix: reader located with exact name (typescript Critical); axe spec asserts status per page and scans an annotated reader page; workflow pin and view-transition comments corrected | — |
| 6.R1b | dsp-repository | done | 4700e85e | Review fix: .bazelignore gains the suite node_modules; CONVENTIONS, testing-strategy, review-guidelines, CPE CONTEXT.md and ARCH-MAP prose (fingerprint untouched) name the suite | — |
| 6.R2 | dsp-repository | done | d3d44ef4 | Review fix round 2: CONVENTIONS Before Committing names just test-e2e-cpe (dune); CONTEXT.md sentence made descriptive; axe PAGES default status 200 (simplicity); config comment rewrapped | — |
| 6.G | dsp-repository | done | — | Gates at d3d44ef4: just check 0; just test 0; just test-e2e-cpe 49 passed, 1 skipped (Back/Forward in nojs); just test-a11y-cpe 12 passed; commit-lint messages OK, count over 1 as expected before the squash | — |

## Phase 7

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 7.1 | dsp-repository | cpe/server/src/{activation,follow,lib}.rs, cpe/store-build/src/{hook,hooks,lib}.rs, store-build/README.md, ARCH-MAP.md, cpe/CONTEXT.md | — | Phase 7 items 1–13 | patch applied minus sync-store; hooks kept across re-activation; server and store-build tests pass | plan Phase 7 + "Phases 7 to 10" | — |
| 7.2 | dsp-repository | — | 7.1 | Phase 7 `just check`, `just test` | both pass | plan Phase 7 | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 7.1 | dsp-repository | done | 37df3363 | incubator 521b0916 applied without sync-store hunks; `port_sources` hunk and error-text tests fitted to `NotLoaded`; `Activated` keeps hooks, `reactivate` reuses them; `follow_with_hooks`; boundary exception covers both hidden methods; docs | — |
| 7.R0 | dsp-repository | done | a335113f | predecessor's partial fix round (findings lost), kept after judging: CONTEXT facade rule names the hooks exception, store-build module doc and ARCH-MAP say a host may supply rows, `follow_with_hooks` doc names the no-hooks case after a panicked first activation | — |
| 7.R1 | dsp-repository | done | 33c5faea | review fix round 1: `hooks::resolve`, `validate` and ARCH-MAP say they see no host's hook rows; `Hook` doc covers a host's row; garbled `build` doc sentence | — |
| 7.R1f | dsp-repository | done | fbc0d82f | rustfmt rewrap of 7.R1's doc line | — |
| 7.2 | dsp-repository | done | — | `just check` and `just test` pass after the fix round (re-review of 7.R1: no Critical or Warning) | — |

## Phase 8

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 8.1 | dsp-repository | the 26 files of incubator c8bdcfe9 (cpe-core remodel lowering + validate, cpe-store-build builder, fixtures, goldens, docs/src/cpe/config-language.md, cpe-store-build README) | — | Phase 8: c8bdcfe9 applied; config-language.md; validate-cpe; check; test | patch applied 3-way, conflicts resolved keeping both sides, nightly fmt/clippy clean, patch's tests pass, Incunabula snapshots unchanged | plan Phase 8 + Proposed Solution Phases 7-10 table | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 8.1 | dsp-repository | done | 8affbbd0 | c8bdcfe9 applied 3-way, all 26 files clean; fit: two new tests' `include_str!(concat!(env!("CARGO_MANIFEST_DIR"),…))` made relative (Bazel sets a relative CARGO_MANIFEST_DIR); nightly fmt rewrapped one doc comment in diagnostic.rs; validate-cpe, check, test green; no snapshot changed | — |
| 8.R1 | dsp-repository | done | 040ca0bd | review fix (fixup of 8affbbd0): store-build README "Only properties are dropped" and config-language.md "No `annotations`" said an undeclared class is refused; now "neither declares nor leaves out", matching ignore-class | — |

## Phase 9

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 9.1 | dsp-repository | areas/access/cpe/store-build/{README.md,src/draft.rs,src/hook.rs,src/mapping/*.rs,src/projects/incunabula/*.rs,tests/it/hooks.rs}, areas/access/sync/store/src/{lib.rs,snapshot_tests.rs}, docs/src/cpe/config-language.md | — | 9: patch applied; serve/read_snapshot share map_quads; read_snapshot tests rewritten over a reader; other tests pass; doc comment names caller | targeted bazel tests of cpe-store-build and sync-store green | plan Phase 9 + .claude/tmp/p9.patch | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 9.1 | dsp-repository | done | a3d0b3b3 | c7d6c2c8 applied 3-way; rerere resolution of sync-store lib.rs dropped the patch's doc comments on read_snapshot/map_quads, restored (fitted to snapshot.nq, caller named) plus its module-doc sentence; the one test using serving/write_0804 rewritten over a File reader on 0804's generation snapshot vs LiveArchiveProjection::new | — |
| 9.R1 | dsp-repository | done | d3b3933b | review fix round 1 (fixup!): read_snapshot docs narrowed to the rules on a snapshot's content (no token, format-version or file-system checks; completeness is the caller's); README's stale 'cannot provide a string it builds' dropped; SnapshotError::Read doc widened; read_snapshot in ARCH-MAP sync entry (rust, consistency, dune, security) | — |
| 9.R2 | dsp-repository | done | aa51c2a4 | review fix round 2 (fixup!, dune re-review DUNE-001): read_snapshot's `# Errors` names the variants it can return; token and generation errors are the adapter's. Cap reached; re-review of this 3-line doc fix not run, variant names checked against error.rs; just check and just test green after it | — |

## Phase 10

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 10.1 | dsp-repository | the patch's 31 files under areas/access/cpe/{core,ports,store-build}, areas/access/sync/store, docs/src/cpe/config-language.md; sync-store mapping_tests.rs | — | P10 #1, #2, #3, #8 | patch applied with conflicts resolved; stack test cut to rights and classification; new test serves authorship and copyright holder on a file; targeted bazel tests green; Incunabula snapshots unchanged | plan "Phases 7 to 10" row 10; .claude/tmp/p10.patch | — |
| 10.2 | dsp-repository | sync/store/src/vocab.rs, sync/store/src/lib.rs, areas/access/cpe/CONTEXT.md, areas/access/sync/CONTEXT.md, cpe/store-build/README.md | 10.1 | P10 #4, #5, #6, #7 | docs say legal information is read, rights and classification still left out | plan Phase 10 checklist | — |
| 10.3 | dsp-repository | — (gates) | 10.2 | P10 #9, #10, #11 | just validate-cpe, just check, just test pass | — | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 10.1 | dsp-repository | done | 8fa1befb | incubator 60651cdf applied 3-way; one conflict (store-build lib.rs build() doc: kept the stack's wording plus the patch's "member links"); no rerere reuse (other 30 files applied cleanly); stack legal-information test split into rights/classification-unchanged and legal-served-on-file; nightly fmt reformatted 3 patched files; CPE and sync tests green, no snapshot changed | — |
| 10.2 | dsp-repository | done | 98d3df7f | vocab.rs header, sync-store lib.rs omit list, sync CONTEXT.md (LiveArchiveProjection) and CPE port table now say legal information is read, rights/classification left out; README needed no edit (patch applied cleanly over Phases 7-9's rows, both present) | — |
| 10.3 | dsp-repository | done | — | gates on 98d3df7f and again on 5a6087f2: just validate-cpe (0803: no project failed), just check, just test (102/102) pass; no .snap file changed in e871e970..HEAD | — |
| 10.R1 | dsp-repository | done | 5a6087f2 | review fix: CPE port table gives a file's license its own row (not carried, FORMAT.md D14; dune DUNE-002, which named this phase's own wording); legal-served test no longer adds rights/classification (simplicity). Re-review of the 3-line fix done inline | — |

## Phase 11

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 11.1 | dsp-repository | areas/access/cpe/components/src/{component/resource_list.rs,component/dom.rs,component/annotation.rs,component/compound_viewer.rs,lib.rs} | — | Phase 11 items 1-4 | no verbatim copy of the two expressions; `anno_dot()` via `html!`; annotation.rs pin row removed and the test fails with it at 1; no snapshot changes | plan "#### Phase 11" | — |
| 11.G | dsp-repository | — | 11.1 | Phase 11 items 5-6 (`just check`, `just test`) | both pass | plan "#### Phase 11" | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 11.1 | dsp-repository | done | e4151ce4 | order control splices dom.rs's two expressions; `anno_dot()` renders the dot with `html!`; annotation.rs pin row removed (test failed with it at 1, verified); zero snapshot changes | — |
| 11.G | dsp-repository | done | — | `just check` and `just test` pass at e4151ce4 (after the review, which found no Critical or Warning) | — |

## Phase 12

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 12.1 | dsp-repository | areas/access/cpe/web-e2e-tests/tests/book.spec.ts | — | 12: book.spec.ts covers; pass on parent binary | new cases pass on e234699d's binary, both projects | plan Phase 12 §; book.spec.ts | — |
| 12.2 | dsp-repository | components/src/component/compound_viewer.rs (+snapshots), components/src/lib.rs, web/src/page.rs, server/src/router.rs (comment only) | 12.1 | 12: render_cv_root no script; unit tests assert attribute; pin zero; snapshots reviewed; test-e2e-cpe; just check; just test | no `<script>` in the root; data-init carries fix+seed; pin row gone | compound_viewer.rs render_cv_root, stale_url_correction | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 12.1 | dsp-repository | done | 9c163570 | book.spec.ts: reload + Back, stale-address correction, paging after it; 53 passed / 3 skipped on e234699d binary | — |
| 12.2 | dsp-repository | done | d2d2c8bd | render_cv_root: script child → data-init (fix+seed); READER_URL_INIT_SCRIPT → READER_URL_SEED_EXPR; pin row removed; 13 snapshots (script removed, attribute added only); router.rs comment only; e2e 53 passed / 3 skipped after | — |
| 12.R1a | dsp-repository | done | 4b60a717 | review fix (typescript-reviewer): Back after reload never reached popstate; new case checks no reload on in-page Back after a reload, and a stale address keeping `x=$expand@get(1)&y=2`; canary (forced stale record) fails both; e2e 55 passed / 5 skipped | — |
| 12.R1b | dsp-repository | done | 1a8c3b18 | review fix (dune DUNE-001): `@`→`\x40` moved into `cpe_core::util::js_string_escape` (+ unit test); seed doc shortened (simplicity) | — |

## Phase 13

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 13.1 | dsp-repository | areas/access/cpe/web-e2e-tests/tests/book.spec.ts | — | P13 cb1, cb2 | arrival (emphasis, sticky and one-track scroll), picker open/choose/current/mark, arrow keys; pass on b2f691d0's binary | plan "Phase 13" bullet | — |
| 13.2 | dsp-repository | areas/access/cpe/public/cpe.js, areas/access/cpe/public/.biome.json, cpe/server/src/static_files.rs, cpe/web/src/page.rs (+snapshots), justfile, .github/workflows/check.yml | 13.1 | P13 cb3, cb4, cb5, cb10, cb11 | `window.cpe` with four functions; served text/javascript; `defer` before DataStar; biome lints the one file | plan "Phase 13" bullet | — |
| 13.3 | dsp-repository | cpe/components/src/component/annotation.rs, resource_detail.rs (+snapshots), dom.rs comments | 13.2 | P13 cb6, cb8 (part) | arrival attribute calls `cpe.arrival(el, ids)`, assigns both signals; no IIFE | annotation.rs ARRIVAL_INIT_EXPR | — |
| 13.4 | dsp-repository | cpe/components/src/component/compound_viewer.rs (+snapshots) | 13.2 | P13 cb7, cb8 (part), cb12 | toggle, click, keydown call cpe.*; `@get` from shared-datastar | compound_viewer.rs PICKER_TOGGLE_EXPR, picker_click_expr, PAGES_KEYS_EXPR | — |
| 13.5 | dsp-repository | areas/access/cpe/CONTEXT.md | — | P13 cb9 | listed exceptions recorded | plan "listed exceptions" paragraph | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 13.1 | dsp-repository | done | 96e0a760 | book.spec.ts: arrival by fragment (emphasis; sticky and one-track scroll at 1600 and 600 px), picker open/choose/current/mark, arrow keys; pass 3x on b2f691d0's binary; arrival note uncovered (no link object in the fixture) | — |
| 13.2 | dsp-repository | done | 9aa0cf09 | cpe.js (four functions, no dependency) + public/.biome.json; FILES row + content-type tests; document_head loads it deferred before DataStar (17 page snapshots gain the tag only); lint-e2e and check.yml lint it; two doubled-base checks narrowed to "/cpe/cpe/" | — |
| 13.3 | dsp-repository | done | 5ed35360 | ARRIVAL_INIT_EXPR → arrival_init_expr(): calls cpe.arrival(el, {register, entryPrefix, reader, title}) and writes both signals; resource_detail renders it; annotation test + snapshot assert the attribute; cpe-web test pins cpe.js's partner-data reads (note uncovered e2e) | — |
| 13.4 | dsp-repository | done | cb58207e | PICKER_TOGGLE_EXPR, picker_click_expr, PAGES_KEYS_EXPR call cpe.pickerToggle/pickerTarget/pageKey; @get via shared-datastar plus_js("query") behind query !== null; tests + 7 snapshots (attributes only); e2e 68 passed; canary (window.cpe renamed) fails 10 JS cases | — |
| 13.5 | dsp-repository | done | d40da0cb | CONTEXT.md: cpe.js under the static files, **Inline expression** with the listed exceptions and the two Not-yet items; ARCH-MAP CPE prose names cpe.js (fingerprint untouched) | — |
| 13.R1 | dsp-repository | done | 9a2fd64a | review round 1: arrival data-init guarded (`window.cpe?.arrival`, signals only when it returned; a throw stopped the address seed's data-init, canary-verified) + e2e with cpe.js aborted; race-free "does nothing" specs (Next turn as the only _compound request), Shift+arrow case, href-derived mark heading, no-fragment waits for DataStar; reader-id tie test; cpe-web pins the four function names; DOM-contract notes in cpe.js and at the Rust sites; doubled-base checks ignore only `_static/cpe/cpe.js`; record.css / check.yml / CONTEXT.md stale text | — |
| 13.R2 | dsp-repository | done | 17460695 | review round 2 (re-review of 9a2fd64a): cpe.js header lists every class/element/attribute it reads and the `window.cpe?.` rule for data-init; cpe-web test pins the partner-data reads as whole expressions again; picker_click_expr doc names the plain-link fallback; rewrap. Final gate: just check, just test (102), just test-e2e-cpe (71 passed, 15 skipped), biome clean | — |

## Phase 14

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 14.1 | dsp-repository | areas/access/cpe/server/src/{assets.rs,static_files.rs} | — | `static_files.rs` sends `ASSET_CSP` on SVG files | a `_static` SVG carries `ASSET_CSP`, a stylesheet and `cpe.js` do not; `bazel test //areas/access/cpe/server/...` green | plan Phase 14, 'Proposed Solution' para 2 | — |
| 14.2 | dsp-repository | areas/access/server/{BUILD.bazel,src/serve.rs}, docs/src/{cpe,dpe}/operations.md | 14.1 | four Test items, BUILD.bazel lists tower-http, `assemble()` adds the two headers, both operations.md list the headers | DPE page, CPE page, CPE 404, failed 503 carry both headers; `_assets`/`_static` SVG keep ASSET_CSP only; stylesheet/cpe.js one nosniff + page policy; `/healthz` unchanged; `bazel test //areas/access/server/...` green | plan Phase 14, 'Proposed Solution' para 1; serve.rs `assemble` | — |
| 14.G | dsp-repository | — | 14.2 | Run `just test-e2e-cpe`, `just check`, `just test` | all three pass | plan Phase 14 checklist | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 14.1 | dsp-repository | done | b453759e | `static_files::serve` sends `ASSET_CSP` (now `pub(crate)`) on SVGs; a test pins every SVG and no other file | — |
| 14.2 | dsp-repository | done | ca4ee9a3 | `assemble` adds nosniff + `base-uri 'self'; object-src 'none'` with `if_not_present` after the OTel layers; four `assemble` tests (pages, 404/503, SVG policies, one nosniff on static files, `/healthz` without); BUILD.bazel tower-http; both operations.md, access-server CLAUDE.md and ARCH-MAP prose | — |
| 14.R1 | dsp-repository | done | 8452a58a | review fix (dune DUNE-001, Warning): ARCH-MAP's areas/access/server entry names `tower-http` and gains the baseline-headers boundary rule | — |
| 14.G | dsp-repository | done | — | gates on 8452a58a: `just test-e2e-cpe` 71 passed, 15 skipped, exit 0 (run on ca4ee9a3; the fix after it is ARCH-MAP only); `just check` and `just test` exit 0; re-review of 14.R1 (consistency, simplicity, dune) clean | — |

## Phase 15

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 15.1 | dsp-repository | areas/access/cpe/web-e2e-tests/tests/rail-layout.spec.ts, areas/access/cpe/components/style/record.css | — | Phase 15: rail-layout spec (sideways scroll, wide reader child); left edge; fails on parent; record.css changed; no snapshot changes | spec fails on 3a2cd1d0 at the breaking widths, passes after the CSS change; no .snap diff | plan "#### Phase 15"; record.css COMPOUND-VIEWER CONTENTS RAIL | — |
| 15.G | dsp-repository | — | 15.1 | Phase 15: just test-e2e-cpe; just check; just test | all three pass | plan "#### Phase 15" | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 15.1 | dsp-repository | done | 590d0149 | rail-layout.spec.ts (48 tests: 8 widths x sideways scroll, long reader line, intro/rail left edge; both projects) failed 26 on 3a2cd1d0; record.css: `.cv-toc` reader track `minmax(0, 1fr)`, and `.editorial.page:has(+ .compound-viewer.page) > .narr { margin-inline: 0 }`; no snapshot change | — |
| 15.G | dsp-repository | done | — | just test-e2e-cpe (119 passed, 15 pre-existing skips), just check, just test pass; biome and tsc clean on the suite | — |
| 15.R1 | dsp-repository | done | c4154f4c | review round 1 (consistency, dune, a11y, maud-datastar, typescript): margin rule scoped to `:has(+ .compound-viewer.page > .cv-toc) > .narr:not(.narr-wide)` (it had moved breakouts and fired above rail-less viewers); THE COMPOSITION names the exception; spec load guard checks all seven stylesheets answered (canary: a missing sheet fails every test) | — |

## Phase 16

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 16.1 | dsp-repository | docs/src/cpe/architecture.md, docs/src/SUMMARY.md, docs/adr/0007-* | — | architecture.md; SUMMARY (part) | page builds in mdBook; every path/symbol exists; ADR-0007 offline-page ticket reads DEV-8027 | plan "### Phase 16: documentation" | — |
| 16.2 | dsp-repository | docs/src/cpe/component-library.md, docs/src/SUMMARY.md | — | component-library.md; SUMMARY (rest); JS rule (part) | page builds; JS rule and listed exceptions match CONTEXT.md | plan "### Phases 11 to 13" + "### Phase 16" | — |
| 16.3 | dsp-repository | areas/access/cpe/CLAUDE.md, eng.yaml, ARCH-MAP.md | 16.1, 16.2 | CLAUDE.md; eng.yaml override; resolver check; ARCH-MAP; JS rule (rest) | resolver exit 0 lists the convention; recipes/paths exist | plan "### Phase 16" + incubator cpe/CLAUDE.md | — |
| 16.4 | dsp-repository | — | 16.1-16.3 | path check; just check; just test | all three gates green | plan Phase 16 checklist | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 16.1 | dsp-repository | done | 02112acf | docs/src/cpe/architecture.md (crates, ports, activation and statuses, request path), SUMMARY entry; ADR-0007 and CPE CONTEXT.md offline-page ticket DEV-7403 -> DEV-8027 | — |
| 16.2 | dsp-repository | done | 2a07175d | docs/src/cpe/component-library.md (components, stylesheets, layout contract, Mosaic seam, JavaScript rule), SUMMARY lists both pages | — |
| 16.3 | dsp-repository | done | e4363bbe | areas/access/cpe/CLAUDE.md runbook; eng.yaml override for areas/access/cpe/** (resolver run after the edit: exit 0, lists areas/access/cpe/CLAUDE.md; validate ok); ARCH-MAP local-context kit names the three files | — |
| 16.R1 | dsp-repository | done | 7e20db5e | review fixes: `just snap-accept` for snapshots (consistency), BINARY_ONLY only with a hook (dune DUNE-001), unknown 404 vs Unavailable 503 named (dune), #582 cited as what the ExecuteScript waits for, also in CONTEXT.md (reference), rail one column at 860px and below (consistency) | — |
| 16.R2 | dsp-repository | done | c04f9e57 | re-review fixes: runbook gives an unknown project's condition (sync's location holds other projects; else 503) and points to architecture.md's status table (dune); hygiene-list rule left to the store-build README (dune, simplicity); `web/src/page.rs` named by path in all three exception lists (consistency) | — |
| 16.4 | dsp-repository | done | — | every just recipe, path and symbol in the three documents checked against the tree (script over backticked tokens, plus the reviewers); mdbook build ok; just check and just test pass after the fix rounds | — |
