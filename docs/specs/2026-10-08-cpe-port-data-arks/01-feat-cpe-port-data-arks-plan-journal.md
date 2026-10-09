---
plan: docs/specs/2026-10-08-cpe-port-data-arks/01-feat-cpe-port-data-arks-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7487
base_commit: 2b0d86fe
branch: worktree-DEV-7487
started: 2026-10-08
problem: >
  CPE's "Show in dataset" panel and its MalformedDataArk boot check need each resource's plain data
  ARK (4,079 resource and 64 Region ARKs for 0803), and DEV-7402 builds Incunabula's store from
  cpe-ports. The port has no ARK field and 0803.nq carries none; the incubator computes them in
  dao-lift from the resource IRI.
status: complete
---

# Execution Journal: 01-feat-cpe-port-data-arks-plan

## Repos

| repo           | base_commit | branch            | merge_strategy | status      | pr |
|----------------|-------------|-------------------|----------------|-------------|----|
| dsp-repository | 2b0d86fe    | worktree-DEV-7487 | rebase         | shipped     | https://github.com/dasch-swiss/dsp-repository/pull/465 |
| dsp-incubator  | d23b6ad8    | worktree-DEV-7487 | squash         | shipped     | https://github.com/dasch-swiss/dsp-incubator/pull/486 |

## Phases

| phase | status      | phase_base              | review_fix_rounds |
|-------|-------------|-------------------------|-------------------|
| 1     | reviewed    | dsp-repository@93e9c4d6 | 1                 |
| 2     | reviewed    | dsp-repository@93e9c4d6 | 1                 |
| 3     | complete    | dsp-incubator@d23b6ad8  | 0                 |

## Chunk queue

| id  | repo           | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|----------------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | areas/access/cpe/ports/src/{snapshot,lib,contract,fake}.rs | — | DataArk, Resource.ark, re-export, MalformedDataArk + Display, four contract tests, documented-order case, resource() helper ark, the check, contract doc review, fake.rs ark | `cargo test -p cpe-ports` green; new contract tests fail without the check | plan § Boundary DTO, § Contract | — |
| 1.2 | dsp-repository | areas/access/sync/store/src/{ark,ark_tests,lib,error,mapping/resources,mapping_tests,snapshot_tests}.rs | 1.1 | ark_tests (vectors, rejected shapes, upper-casing, error kinds, contract shape, zz-book), ark.rs, lib registration + ArkError re-export, NoDataArk, two snapshot tests, resources.rs ark, mapping_tests literals, mapping test, crate doc | `cargo test -p sync-store` green; tests written first | plan § Derivation, § Mapping | — |
| 1.3 | dsp-repository | — | 1.2 | "Run just check && just test", "Commit as feat(cpe-ports,sync-store)" | gate green; one feat commit on 93e9c4d6 | — | — |
| 2.1 | dsp-repository | areas/access/sync/data/{0803-arks.txt,PROVENANCE}, areas/access/sync/store/tests/committed_0803.rs | 1.3 | extraction, PROVENANCE section, three committed tests | 4,143 lines; committed tests green; pin test unchanged | plan § Oracle | — |
| 2.2 | dsp-repository | areas/access/cpe/CONTEXT.md, areas/access/** wording, ARCH-MAP.md wording | 2.1 | CONTEXT.md row 42, Data ARK term, archive-shaped sentence, the grep | no stale "ARKs out" wording | plan § Phase 2 checklist | — |
| 2.3 | dsp-repository | ARCH-MAP.md | 2.2 | the two dune-map refresh checkboxes | cpe and sync entries match the change | ARCH-MAP.md areas/access/{cpe,sync} | — |
| 2.4 | dsp-repository | — | 2.3 | "Run just check && just test", "Amend Phase 2's changes", "Run just commit-lint" | gate green; one feat commit on 93e9c4d6 | — | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | a6ed3408 | DataArk newtype, Resource.ark after iri, re-export; MalformedDataArk after DuplicateResource with a byte-based std-only shape check in the per-resource loop; four contract tests and a documented-order case (5 fail with the check disabled); resource() helper and fake.rs (zz=booko) carry ARKs | none |
| 1.2 | dsp-repository | complete | a6ed3408 | ark.rs ports dao-lift (typed ArkError, shortcode upper-cased like dsp-api); ark_tests carry dsp-api and eleven production vectors, rejections, error kinds, zz-book and a contract round-trip; NoDataArk { reason } wired into resources.rs; two snapshot and one mapping test; crate doc. Literal ARKs cross-checked independently (Xr0eJ, zR8cR, 081C/aM) | none |
| 1.3 | dsp-repository | complete | a6ed3408 | just check and just test green; feat commit on 93e9c4d6 | none |
| 1.4 | dsp-repository | complete | 19c7d474 (amended) | Phase 1 review fixes. Reviewers: rust (no C/W), dune (W: prefix spelled in two crates; W: Phase 2 doc gaps), consistency (W: sync-store docs framed NoDataArk as a FORMAT.md rule), simplicity (W: same prefix; provisional note repeated). Applied: cpe_ports::DATA_ARK_PREFIX used by contract and ark.rs; lib.rs/mod.rs docs; contract module doc says shape only; MalformedDataArk doc without bracketed regexes; ArkError variant docs; round-trip test asserts no violations. Plan: Phase 2 gains sync/CONTEXT.md, the Pid distinction, wider grep, named ARCH-MAP entities. Dropped: trimming the production-vector labels (they mirror dao-lift's provenance), #[source] on reason (neighbours use the same form). dune's operations.md claim refuted: that host rewrite concerns DPE's project ARKs | none |

| 2.1 | dsp-repository | complete | 4676af28 (amended) | data.sql blob verified equal to c2b3e10a (ec082a1c); 0803-arks.txt extracted, 4,143 lines; PROVENANCE section in the `commit <sha>` form (pin test unchanged); three committed tests, the all-ARKs one fails with every line when the comparison is flipped | none |
| 2.2 | dsp-repository | complete | 4676af28 (amended) | cpe/CONTEXT.md: data-ARK In row, old row keeps permissions and creation/deletion metadata out; Data ARK term distinct from Pid; archive-shaped sentence. sync/CONTEXT.md: derived, not read; NoDataArk. Grep of areas/access, docs/src, root CONTEXT.md, ARCH-MAP: no other stale wording | none |
| 2.3 | dsp-repository | complete | 4676af28 (amended) | dune-map run non-interactively on the two entries from the known diff (the plan checkbox authorizes the overwrite): cpe Key entities gain DataArk, DATA_ARK_PREFIX, MalformedDataArk; sync Purpose, Key entities (ArkError, NoDataArk) and Durable state (the oracle). sync Fingerprint d1f6549a2577 over the staged tree; cpe stays planned, none | none |
| 2.4 | dsp-repository | complete | 4676af28 | just check and just test green; Phase 2 amended into the feat commit, message extended; commit-lint passes messages, fails only the count, which over a7e8df14..HEAD includes #464's two commits (the PR ticks allow-many-commits) | none |
| 2.5 | dsp-repository | complete | b867f01b (amended) | Phase 2 review fixes. Reviewers: consistency (W: ARCH-MAP said sync-store reads the oracle), dune (W: sync kit lacked ark.rs; W: Pid/Data ARK linked one way), rust (W: byte slice and panic-at-first-miss in the oracle test), simplicity (W: drop literal count, spot tests, PROVENANCE counts; all rejected: literal counts are a repo learning, the spot tests are plan deliverables, the subset description is provenance). Applied: oracle's single writer named, never generated from ark.rs; sync Paths and kit name the oracle and ark.rs; cpe entry names sync-store's ark.rs; Pid entry excludes the data ARK; sync CONTEXT links the term; the oracle test uses a map, reports unserved resources, strips the check digit safely, asserts sorted-unique, and says why it must not be regenerated (one corrupted line: 1 mismatch). sync Fingerprint ec1861e4ca2c, recomputed over the committed tree | none |

| 3.1 | dsp-incubator | complete | b79be915 (squash-merged as ae2610df) | Pin moved 7118cdbf → f73009e2 (dsp-repository#465 as merged); only the data ARKs, since dsp-incubator#479 had already vendored the annotations. No `cpe_ports` use outside cpe/vendor/, so no match or literal changed; Cargo.lock unchanged. `just cpe test`, `vendor-diff` (empty), the ignored-file check and `just cpe ci` pass. Phase review skipped: the vendored tree is byte-identical to the code reviewed in #465 | none |

## Side findings

- #464 was rebase-merged during the run (7118cdbf on main); the stack base dropped out, #465 was
  retargeted to main, and the branch was rebased onto origin/main with the user's approval
  (d593a5b0, 6681c0bc). #465 was rebase-merged as 1eb0ffac and f73009e2, which resolved H1.
- DEV-7486's own incubator phase was not superseded after all: dsp-incubator#479 ran it before this
  plan's Phase 3. Its plan's note now says so; Phase 3 here vendored only the data ARKs.

## Deferrals

- No eng.yaml `verify` entries; no browser or TUI surface in the diff.

## Closeout

- root_cause: CPE's "Show in dataset" panel and `MalformedDataArk` check need each resource's data ARK; the port had no ARK field, 0803.nq carries none, and only the incubator's `dao-lift` computed them.
- investigation: dsp-api's `resourceIriToArkUrl` (ported by `dao-lift`) fixes the algorithm: base64url check digit over the id, `-` escaped as `=`, shortcode upper-cased. Every one of the incubator's 4,143 committed 0803 ARKs is served.
- solution: `cpe-ports` gains `DataArk`, `DATA_ARK_PREFIX`, `Resource.ark` and a std-only `MalformedDataArk` shape check; `sync-store` derives the ARK from the IRI in `ark.rs` and refuses a snapshot with `NoDataArk`; the incubator vendors the merged commit (dsp-incubator#486).
- prevention: `0803-arks.txt` is a committed oracle from `dao-lift`'s output, never regenerated from `ark.rs`, checked line by line against the served ARKs; `ark_tests.rs` carries dsp-api's and eleven production vectors and one test per `ArkError` kind.
