---
plan: docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/cpe-merge--port
base_commit: 821254df02a557d11f8453146a869e6897de545b
branch: worktree-cpe-merge--port
started: 2026-09-29
problem: >
  ADR-0007 makes CPE's store a disposable read model rebuilt only from what `sync` serves through a
  port CPE declares, and requires that port to speak the archive's shape rather than CPE's presentation
  model. DEV-7399 (minimal `sync`) and DEV-7400 (bring CPE in) are blocked until the port exists: the
  trait, its archive-shaped DTOs, an in-memory fake for CPE's tests, and a contract check every adapter's
  output must satisfy.
symptoms:
status: complete
---

# Execution Journal: 01-feat-cpe-archive-projection-port-plan

## Chunk queue

Repo policy lands the whole PR as the single commit `feat(cpe-ports): declare CPE's port onto the archive
projection`; chunk 1 creates it and chunk 2 is amended into it.

| id | files | depends_on | checkboxes | acceptance | context | replaces |
|----|-------|------------|------------|------------|---------|----------|
| 1 | Cargo.toml, Cargo.lock, areas/access/cpe/ports/{Cargo.toml,src/lib.rs,src/snapshot.rs,src/fake.rs,src/contract.rs} | — | "Crate scaffold" (2), "Tests first" (13), "Implementation" (6) | `cargo test -p cpe-ports` green, `cargo clippy -p cpe-ports --all-targets -- -D warnings` clean, no `[dependencies]` | plan § Proposed Solution; precedent areas/deposit/editor/core/src/repository.rs:1-60 | — |
| 2 | areas/access/cpe/CONTEXT.md, areas/access/CONTEXT.md, CONTEXT.md, docs/src/repo_structure.md, CONVENTIONS.md, ARCH-MAP.md | 1 | "Documentation" (9) | every doc names `cpe-ports`; ARCH-MAP cpe entry corrected in the three places | plan § In and out; ARCH-MAP.md:537 § areas/access/cpe | — |
| 3 | — | 1, 2 | "Verification" (3) | `just check`, `just test` pass; commit-lint reports only the expected 2-commit count | plan § Verification | — |

## Chunks

| id | status | commit(s) | summary | blocker |
|----|--------|-----------|---------|---------|
| 1 | complete | a455af0d | `cpe-ports` crate: trait, DTOs, fake, `contract::violations` (Tarjan SCC for cycles), 24 tests, mutation-checked | none |
| 2 | complete | 3420c69d | docs amended into the feat commit: cpe/CONTEXT.md seed, both CONTEXT indexes, repo_structure (table + tree), CONVENTIONS and git-conventions scopes, ARCH-MAP cpe entry and overview edited by hand | none (the `dune:dune-map` run is left to the session: it must confirm with the user before overwriting) |
| 3 | complete | 3420c69d | `just check` and `just test` green; `just commit-lint` passes messages and fails only the commit count (2 > 1, plan commit f4a17991 present), expected until the ship-time history tidy | commit count resolved at ship |
| R1 | complete | fc3300bb (amends 3420c69d) | final-review fix round 1 of 2, 16 verified findings: `violations` sorts by derived `Ord` (rank/first_iri/referrer iri removed); Tarjan frame loop without `expect`; SCC-once doc on both cycle variants; sibling nodes deduped so a repeated node is only `DuplicateListNode`; 7 new tests (cycle entered above smallest IRI for both graphs, cycle with tail, two disjoint cycles, 100_000-deep chain, repeated child node, fake last-write-wins reversed), duplicate tests differ by label/name, `prop()` used throughout; `#[must_use]` on `unavailable`/`with_project`/`with_unavailable`; `SimulatedOutage` replaced by `io::Error::other`; ARCH-MAP count/status/boundary levels/`sync` notes/Paths remark/date, cpe CONTEXT `_Avoid_` scoped, decisions.md "(proposed)" dropped. `cargo test -p cpe-ports` (31), clippy, `just check`, `just test` green; dedup canary fails without the fix | none |

## Deferrals

- `dune:dune-map` checkbox: left for the session. The ARCH-MAP `areas/access/cpe` entry was edited by hand and stays `status: planned` with `Fingerprint: none`; dune-map decides whether to flip it to active and records the fingerprint.
- Acceptance criterion "No DTO names a CPE presentation concept": left for the session's final review. A grep of `snapshot.rs`/`lib.rs` finds only knora-base's `Representation` in a doc comment on `Resource::file`.

## Side findings

- `Violation` derives `PartialOrd, Ord` so the sort in `violations` can break ties. The documented key (variant, then first IRI) is computed separately, because the derived order of `ListNodeReferrer` would put every value referrer before every parent referrer.
- The repo also lists crate scopes in `docs/src/git-conventions.md`, and `docs/src/repo_structure.md` has a directory tree. The plan named neither; both were updated alongside the named docs.
- `commit-lint`'s count check fails by design while the plan commit sits on the branch.
- Fix round R1 supersedes the first side finding: `violations` now sorts by the derived `Ord`, so a `DanglingListNode` with a value referrer sorts before one with a parent referrer regardless of IRI; the `Violation` doc states the `Ord` order.
- Not fixed, outside the verified list: a resource or list node repeated with a dangling link, parent or list value reports the same violation once per copy (identical entries in the result). A `found.dedup()` after the sort would fold them.
- Finding 7's "prayer/sermon ordering" comment on `valid_snapshot()` was not in the file at R1; only the `prop()` consistency part applied.
## Closeout

- root_cause: CPE had no declared seam onto the archive projection; the prototype's remodel ran dump → CSV → `data.sql`, and the CSV layer already dropped facts such as value order.
- investigation: knora-base, not the API rendering, fixed the DTO shape: dates as JDN plus precision per bound, `isPartOf`/`seqnum` as separate resource-level facts with subproperties, at most one file value per Representation, optional list-node positions (roots have none), and superseded values that are not flagged `isDeleted`.
- solution: `cpe-ports` (std only): `ArchiveProjection`, archive-shaped DTOs, `FakeArchiveProjection`, and `contract::violations` with Tarjan-based cycle detection and `Ord`-sorted output; a seed `areas/access/cpe/CONTEXT.md` with the in/out table; ARCH-MAP, CONTEXT, scope and repo-structure docs updated.
- prevention: 31 crate tests using non-sorted fixtures, including cycle tests whose entry node is not the minimum, a 100k-node chain, and last-write-wins in the fake. The std-only rule for ports crates stays with review until Bazel `visibility` (ADR-0003); a script was considered and declined.
