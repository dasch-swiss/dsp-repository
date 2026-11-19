# Execution Journal Archive: 01-feat-hidden-projects-plan

## Phase 1

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | areas/access/cpe/server/src/config.rs | — | "Add `hidden_shortcodes` to `CpeConfig`" | field exists, serde-skipped, empty by default; cpe-server tests pass | config.rs § CpeConfig, Default | — |
| 1.2 | dsp-repository | areas/access/cpe/web/src/page.rs, areas/access/cpe/server/src/router.rs | — | "Add `Indexing`…", "Add the `indexing` parameter to `render_page`…", "Update the existing test callers…", "`document_head` with `Indexing::NoIndex`…" test | meta only for NoIndex; snapshots unchanged | page.rs § document_head, render_page, render_not_found_project, site_page | — |
| 1.3 | dsp-repository | areas/access/cpe/server/src/router.rs | — | "Lift the segment extraction of `fallback_handler` into `router::project_segment`" | behaviour unchanged; router tests pass | router.rs § fallback_handler | — |
| 1.4 | dsp-repository | areas/access/cpe/server/src/{activation,router,request,lib,base_tests,static_files}.rs | 1.1, 1.2, 1.3 | load_all hidden_ids + logs; robots layer + consts; router::build hidden_ids, Indexing per project, fallback set, overview filter; other build callers; lib.rs doc comments; Base-tests fixture + SSE/asset test; activation shared-shortcode test | hidden project's responses carry X-Robots-Tag; overview omits it | activation.rs § load_all, request.rs § report_matched_route, router.rs § build | — |
| 1.5 | dsp-repository | areas/access/server/src/serve.rs | 1.4 | parse_hidden_projects + tests; serve() wiring; things_snapshot/things_files fixtures; the assembled-app tests | serve.rs tests pass; serve() logs set and rejects | serve.rs § serve, tests helpers | — |
| 1.6 | dsp-repository | areas/access/CONTEXT.md, areas/access/datei/CONTEXT.md, areas/access/cpe/CONTEXT.md, docs/src/dpe/operations.md, docs/src/cpe/operations.md, areas/access/server/CLAUDE.md, ARCH-MAP.md | 1.5 | all Documentation checkboxes | docs describe the mechanism and DaTEI's adoption | plan § Vocabulary and documentation | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 93148d14 | `CpeConfig::hidden_shortcodes`, serde-skipped, empty by default | none |
| 1.2 | dsp-repository | complete | 334d64d7 | `cpe_web::page::Indexing`; `document_head` writes the robots meta for `NoIndex`; renderers take it; callers pass `Indexable` | none |
| 1.3 | dsp-repository | complete | e6187772 | `router::project_segment` lifted out of `fallback_handler`, behaviour unchanged | none |
| 1.4 | dsp-repository | complete | af40dac0 | hidden ids in `load_all` + logs; `robots_for_hidden_projects` layer; `build` takes `hidden_ids` (NoIndex pages/404, overview filter); lib.rs docs; base_tests + activation tests | none |
| 1.5 | dsp-repository | complete | 371b588a | `parse_hidden_projects` + `serve()` wiring and logs; fixtures (`things_snapshot(shortcode)`, `0003`, `things_other_and_offline`); parser and assembled-app tests | none |
| 1.6 | dsp-repository | complete | 0257f9b1 | Hidden project term; datei boundary commitment; cpe Overview line; DPE env row; CPE ops sentence, failure row, hiding paragraph; server CLAUDE.md bullet + boundary; ARCH-MAP entries | none |
| 1-review-1 | dsp-repository | complete | 74a32ddc | review fixes: CPE matches hidden shortcodes in any case; warn on non-UTF-8 `ACCESS_HIDDEN_PROJECTS`; meta test on a parameterised page; ARCH-MAP boundary rule | none |
