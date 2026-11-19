# Execution Journal Archive: 01-feat-datei-integration-plan

## Phase 0

### Chunk queue
- 0.1 | dsp-incubator | files: datei-engine/src/files/layout.rs, datei-engine/src/rebuild.rs, da-tei/src/files/directory.rs, datei-engine/src/files/memory.rs | depends_on: — | checkboxes: "FileLayout::parse accepts an entry without idno", "Rebuild stamps source_files.content_hash for ODD and CSS from the bytes it read", "write_file_layout keeps writing hashes", "files/layout.rs tests" | acceptance: a layout entry without sha256 parses and round-trips without idno; rebuild stamps ODD/CSS hash from bytes | context: datei-engine/src/files/layout.rs § parse_entry, datei-engine/src/rebuild.rs § load_editions | replaces: —
- 0.2 | dsp-incubator | files: datei-engine/src/files/{stored.rs,manifest.rs,mod.rs} | depends_on: — | checkboxes: "StoredName accepts any relative name…", "The manifest parser reads … refuses another format or version; iiif entries parsed and kept", "Two manifest entries with the same key, or the same stored name…", "Test: StoredName refuses…", "Test: duplicate keys", "Test: duplicate stored names", "Test: unparseable manifest, unknown format or version" | acceptance: StoredName and FileManifest::parse with unit tests | context: plan § Within a project, Phase 0 checklist | replaces: —
- 0.3 | dsp-incubator | files: datei-engine/src/files/*, datei-engine/src/{rebuild.rs,pipeline.rs,odd/parser.rs,core/error.rs,ir/error.rs,ir/view_spec.rs}, datei-engine/clippy.toml, da-tei/src/files/*, da-tei/src/{rebuild.rs,main.rs,upload/create.rs,web/lifecycle.rs,web/handlers.rs}, datei-test-support/src/rebuild_fixture.rs, engine and configurator tests naming the port | depends_on: 0.1, 0.2 | checkboxes: FilePort→FileSource, PortError, ProjectFiles (open, read, read_text, entries, contains, layout), ProjectFilesError, missing manifest, join errors, iiif-for-layout error, advisory hash warning, MemoryFiles, rebuild.rs via ProjectFiles, ODD chain contains, seeding/run_with_files, DirectoryFiles, create/upload staging, load_artefact, print_dropped_artefacts, reload, CLI rebuild, status mapping, test rewrites, fixture, tests: changed bytes warning, manifest missing entry, Unavailable manifest | acceptance: `just da-tei ci` green; no FilePort left | context: datei-engine/src/files/port.rs, da-tei/src/files/directory.rs, web/handlers.rs § open_error_status | replaces: —
- 0.4 | dsp-incubator | files: datei-engine/tests/rebuild_manifest.rs (new) | depends_on: 0.3 | checkboxes: "Test: a manifest maps flat stored names to keys with a space, an umlaut and two files of the same base name…" | acceptance: rebuild through a manifest equals rebuild without | context: datei-test-support/src/rebuild_fixture.rs | replaces: —
- 0.5 | dsp-incubator | files: da-tei/docs/architecture/{file-port.md,file-layout.md,rebuild.md}, da-tei/CLAUDE.md, files/mod.rs doc | depends_on: 0.3 | checkboxes: four Docs items | acceptance: docs describe FileSource, ProjectFiles, manifest, advisory hashes | context: docs/architecture/file-port.md | replaces: —
- 0.6 | dsp-incubator | files: none (verification) | depends_on: 0.3 | checkboxes: "Run just da-tei ci", "Before/after check" | acceptance: ci green; tei-compare reports no difference | context: da-tei/justfile § demo-bibles, repro-bibles | replaces: —

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 0.1 | dsp-incubator | complete | a1f3d709 | layout entries may omit sha256; Rebuild stamps ODD/CSS hash from bytes read | none |
| 0.2 | dsp-incubator | complete | 0234c434 | StoredName and FileManifest::parse with unit tests | none |
| 0.3 | dsp-incubator | complete | fbb75244 | FilePort replaced by FileSource + ProjectFiles (manifest join, advisory hashes); every caller and test moved | none |
| 0.4 | dsp-incubator | complete | 8295d02b | Rebuild through a manifest of flat stored names equals Rebuild by logical path | none |
| 0.5 | dsp-incubator | complete | c86ee3b3 | docs: file-port.md, file-layout.md, rebuild.md, css-layers.md, assets.md, CLAUDE.md | none |
| 0.7 | dsp-incubator | complete | dd4f70de | review fix round 1: ReservedName refusal, DirectoryFiles private, stale hash wording in CLI help/docs, status-mapping and edge tests | none |
| 0.8 | dsp-incubator | complete | 71a77e55 | review fix round 2: fixed-name refusal pinned through a manifest, ReservedName 409 tested, cli.md hash wording | none |
| 0.6 | dsp-incubator | complete | — | verification: before/after Bibles check (base 788e8147 demo-bibles, head repro-bibles) tei-compare: 0 changed, 0 added, 0 removed; just da-tei ci green | none |

## Phase 1

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | docs/adr/0009-datei-joins-the-access-area-as-its-third-capability.md | — | the nine "ADR-0009: …" items; "This plan is committed" (done by 22ecfbec, ticked with 1.1) | ADR-0009 states crates/edges, port, activation, mount, hidden projects, deviations with end conditions, Not-yet-in-code lines per DEV issue; no stale incubator/consequence text | plan § Proposed Solution, Technical Considerations; docs/adr/CLAUDE.md | — |
| 1.2 | dsp-repository | docs/adr/0012-build-time-inputs-come-through-two-channels-one-consumer-each.md | — | the two "ADR-0012: …" items | project-listing sentence with reason; stand-in written by bridge or hand with provenance, Ready token and manifest in its layout | ADR-0012 first and last bullets | — |
| 1.3 | dsp-repository | docs/adr/0003-one-modulith-per-area.md | — | "ADR-0003: DaTEI is no longer only planned…" | DaTEI listed as a capability with a Not-yet-in-code line; stand-in not only committed | ADR-0003 lines 13-14, 188-190 | — |
| 1.4 | dsp-repository | areas/access/CONTEXT.md | — | "areas/access/CONTEXT.md: …" | no "not built here yet"/"vocabulary seed"/"waits to declare its port"; Not yet in code where code comes later | intro paragraph, DaTEI and Hidden project entries | — |
| 1.5 | dsp-repository | areas/access/datei/CONTEXT.md | 1.1 | both "areas/access/datei/CONTEXT.md: …" items | new terms defined; N/N-1, rights per row, "no code here yet", open URL layout replaced by decisions | whole file | — |
| 1.6 | dsp-repository | areas/access/media/CONTEXT.md | 1.2 | "areas/access/media/CONTEXT.md: …" | stand-in on stage written by the bridge; DaTEI adapter and datei_ports named as planned | Stand-in directory entry, "What media does not do yet" | — |
| 1.7 | dsp-repository | docs/src/git-conventions.md, CONVENTIONS.md | — | "git-conventions.md lists the datei-* crate scopes"; "Run just check" | six datei-* scopes in both scope lists | git-conventions.md:87; CONVENTIONS.md scope table | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | done | 88147799 | ADR-0009 rewritten: crates/edges, DateiFiles port, layout/manifest/mapping, background activation, mount, deviations, Not-yet lines (DEV-7549/7574/7935/7938); plan-committed box ticked against 22ecfbec | — |
| 1.2 | dsp-repository | done | 26316c5c | ADR-0012: ports may list projects holding the configuration; media's stand-in written by the VRE bridge or by hand, with provenance, manifest and Ready token; Not-yet line for the bridge (P-DEV-301) | — |
| 1.3 | dsp-repository | done | 7eaec547 | ADR-0003: DaTEI listed as an Access capability with Not-yet lines (DEV-7549 crates, DEV-7574 mount); media's stand-in is MEDIA_DATA_DIR, not only a committed directory | — |
| 1.4 | dsp-repository | done | 12f86cf3 | areas/access/CONTEXT.md: DaTEI's crates, port, activation and mount stated as decided; Not-yet lines for crates, adapter, mount, hidden obligations (DEV-7549, DEV-7574) | — |
| 1.5 | dsp-repository | done | 54ce5c9d | datei/CONTEXT.md: new terms (logical path, stored name, file layout/manifest/mapping, Ready token, DateiFiles, file source, project state); N/N-1, rights per row, no-code and open URL layout replaced by ADR-0009's decisions | — |
| 1.6 | dsp-repository | done | 39a3f864 | media/CONTEXT.md: stand-in written by the VRE bridge as well as by hand, with stored names, manifest, Ready token; DaTEI adapter and datei_ports dependency named as Not yet in code (DEV-7574) | — |
| 1.7 | dsp-repository | done | 43069509 | datei-ports/core/store/store-build/web/server scopes added to git-conventions.md and CONVENTIONS.md; just check green | — |
| 1-review-1 | dsp-repository | done | 46e524da | review fix round 1: ADR-0009 FileSource/ProjectFiles homes, CPE sequencing by access-server, datei-store writes, Not-yet lines for gates and bridge; ADR-0012/0004/0003 pointers; CONTEXT Not-yet per issue, one writer per stand-in dir, stored-name fallback; root and vitrinli CONTEXT stale wording | — |
| 1-review-2 | dsp-repository | done | 37de5d57 | review fix round 2: Ready token checker and open name (DEV-7574), datei-store finishing write named, one writer per stand-in dir in ADR-0012, bridge as planned in root/vitrinli CONTEXT, wording | — |
