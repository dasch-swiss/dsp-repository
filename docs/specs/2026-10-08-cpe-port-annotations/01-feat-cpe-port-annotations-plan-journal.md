---
plan: docs/specs/2026-10-08-cpe-port-annotations/01-feat-cpe-port-annotations-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7486
base_commit: a7e8df14
branch: worktree-DEV-7486
started: 2026-10-08
problem: >
  Incunabula's Slice 6 (reader overlay, annotation register, reuse links, Region entries in
  "Show in dataset") needs the archive's annotations, and DEV-7402 builds Incunabula's store from
  cpe-ports. The port drops every oa:Annotation subject and every link to one, and has no geometry
  or color value kind, so the 77 kb:Region and 40 kb:LinkObj of the committed 0803 projection are
  not served.
status: complete
---

# Execution Journal: 01-feat-cpe-port-annotations-plan

## Repos

| repo           | base_commit | branch            | merge_strategy | status      | pr |
|----------------|-------------|-------------------|----------------|-------------|----|
| dsp-repository | a7e8df14    | worktree-DEV-7486 | rebase         | shipped     | https://github.com/dasch-swiss/dsp-repository/pull/464 |
| dsp-incubator  | —           | worktree-DEV-7486 | squash         | shipped     | https://github.com/dasch-swiss/dsp-incubator/pull/479 |

## Phases

| phase | status      | phase_base              | review_fix_rounds |
|-------|-------------|-------------------------|-------------------|
| 1     | reviewed    | dsp-repository@d8f0a72e | 1                 |
| 2     | reviewed    | dsp-repository@d8f0a72e | 1                 |
| 3     | complete    | dsp-incubator           | 0                 |

## Chunk queue

| id  | repo           | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|----------------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | areas/access/cpe/ports/src/{snapshot,lib,fake,contract}.rs, areas/access/sync/store/src/{mapping/resources,mapping_tests}.rs | — | "Add ValueKind::Geometry", "Add ValueKind::Color", "Extend the ValueKind doc", "Add Annotation and Motivation", "Add Resource.annotation", "Re-export", four "Set annotation: None" | workspace compiles; existing tests green; no behaviour change | snapshot.rs § Resource, ValueKind; plan § Boundary DTOs | — |
| 1.2 | dsp-repository | areas/access/cpe/ports/src/contract.rs | 1.1 | "Declare Violation::DanglingTarget …", the five contract tests, documented-order cases, the two checks, "Review contract.rs's module doc", "Run just check && just test", "Commit as feat(cpe-ports,sync-store)" | new tests fail without the checks, pass with them; just check && just test green; one feat commit on d8f0a72e | contract.rs § Violation, violations(), tests; plan § Contract | — |
| 2.1 | dsp-repository | areas/access/sync/store/src/{vocab,error,lib,mapping_tests,snapshot_tests}.rs, src/mapping/{mod,resources,values}.rs, tests/committed_0803.rs | 1.2 | vocab, InvalidFact, every mapping/snapshot/committed test, kind() arms, pre-pass, served/type check, part_of arm and doc, motivation/target mapping and refusals, values.rs comment, crate doc, mod.rs doc | tests written first and failing; then green; committed tests pin 4,198 / 77 / 40 / 79 / 117 and targets == link targets | plan § Mapping (sync-store), § Tests | — |
| 2.2 | dsp-repository | areas/access/cpe/CONTEXT.md, areas/access/** wording | 2.1 | CONTEXT.md rows 31/34, archive-shaped paragraph, omission lines, Annotation term, the areas/access grep | no stale "annotations omitted" wording under areas/access | plan § Phase 2 checklist | — |
| 2.3 | dsp-repository | ARCH-MAP.md | 2.2 | the two dune-map refresh checkboxes | cpe and sync entries match the change | ARCH-MAP.md areas/access/{cpe,sync} | — |
| 2.4 | dsp-repository | — | 2.3 | "Run just check && just test", "Amend Phase 2's changes", "Run just commit-lint" | gate green; one feat commit on d8f0a72e | brief resume_state notes 1, 5 | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 30be5e59 (to be amended) | Geometry/Color value kinds, Annotation/Motivation DTOs, Resource.annotation, re-exports; four literals set annotation: None | none |
| 1.2 | dsp-repository | complete | 00aa58ab (1.1 amended in) | DanglingTarget and UntargetedAnnotation invariants with Display arms and checks; contract tests incl. geometry/color MissingValueUuid; valid_snapshot carries a Region; module doc names what stays unchecked | none |
| 1.3 | dsp-repository | complete | 33ac12e1 (amended into the feat commit) | Review fix round 1: contract.rs module doc reflowed; the unchecked target promises (sorted, unique, equal to own link targets) stated as left to the adapter, with the reason, instead of as invisible | none |
| 2.1 | dsp-repository | complete | 2eb9267e (to be squashed into the feat commit) | vocab IRIs and six InvalidFact variants; tests first (mapping, snapshot, committed-0803; 20 failed before the change); kind() Geometry/Color arms; annotation-marker pre-pass; every dao:Resource served; motivation and BTreeSet-sorted targets; part_of annotation arm removed; crate, mod and values docs. The vacuous `xdvQ` negative in the book-title test became its own positive test of Region GOkuI's live comment | none |
| 2.2 | dsp-repository | complete | c24b931e (to be squashed into the feat commit) | CONTEXT.md: geometry/color In (geoname, time, interval stay out); Regions/LinkObjs and their oa: facts In, LinkValue reifications out because DAO strips them; Annotation term; archive-shaped paragraph; refused out-of-file target in both omission spots. Grep of areas/access found no other stale wording | none |
| 2.3 | dsp-repository | complete | 84fb05b9 (to be squashed into the feat commit) | dune-map loaded; update run non-interactively (its overwrite confirmation skipped: no user, the plan checkbox authorizes it) on the two entries only, from the known diff without Explore. cpe stays status: planned, Fingerprint none, Key entities gain Resource/Annotation; sync Purpose and Boundary rules name served annotations and the refused out-of-file target; sync Fingerprint 239f85794ba0, map date 2026-10-08 | none |
| 2.5 | dsp-repository | complete | f6733f79 (to be squashed into the feat commit) | Review fix round 1 (rust Warning): StrayAnnotationFact test on a representation node, which its doc names; fails without the pre-pass | none |
| 2.4 | dsp-repository | complete | cc5df33e (2eb9267e, c24b931e, 84fb05b9, f6733f79 and Phase 1 33ac12e1 squashed onto d8f0a72e) | sync fingerprint re-recorded as e338488b58bc after the review fix touched snapshot_tests.rs; commit body reworded to cover Phase 2; just check and just test green on the squashed tree; just commit-lint passes messages and no-merge, fails only the expected 2-commit count (PR ticks allow-many-commits) | none |
| F1 | dsp-repository | complete | ccb11520 (amended into the feat commit) | Final review (dune W): sync/CONTEXT.md lists Annotation among the inherited port terms; LiveArchiveProjection names the UnknownTarget refusal vs the dropped out-of-file link | none |
| F2 | dsp-repository | complete | ccb11520 (amended) | Final review (rust W): annotation targets collected in a Vec in `Facts` order (Index sorts by (predicate, term_key), named nodes by IRI, and dedups); the redundant BTreeSet and its misattributing comment removed; existing sorted/repeat mapping tests pin it | none |
| F3 | dsp-repository | complete | ccb11520 (amended) | Final review (rust W): lib.rs lists an annotation without a target among the rules the mapping names first | none |
| F4 | dsp-repository | complete | ccb11520 (amended) | Final review (rust S): "not a resource of the file" in error.rs (new variant doc), lib.rs, resources.rs, cpe/CONTEXT.md, ARCH-MAP, sync/CONTEXT.md; new test test_snapshot_annotation_target_untyped_value_node_returns_unavailable, fails (Contract DanglingTarget) without the served check | none |
| F5 | dsp-repository | complete | ccb11520 (amended) | Final review (simplicity W): contract.rs module doc points at Annotation::targets and says it checks only non-empty and resource-of-snapshot; resources.rs comment a pointer; snapshot.rs stays canonical | none |
| F6 | dsp-repository | complete | ccb11520 (amended) | Final review (consistency S): mapping/mod.rs doc in present tense | none |
| F7 | dsp-repository | complete | ccb11520 (amended) | Final review (dune S): Motivation in the ARCH-MAP cpe Key entities; fix diff re-reviewed (consistency, simplicity, dune): no Critical/Warning, two wording suggestions applied; sync fingerprint re-recorded 5bff0a0bb38f, verified equal to a fresh computation over HEAD's index; commit body reworded for the target rule; just check, just test green; commit-lint fails only the expected 2-commit count | none |
| 3.1 | dsp-incubator | complete | squash-merged as 1fa1c440 | Recorded after the fact from dsp-incubator#479, run by another session: pin moved a7e8df14 → 7118cdbf (#464 as merged); no `cpe_ports` use outside cpe/vendor/, so no match or literal changed; Cargo.lock unchanged; `vendor-diff` empty, no ignored file, `just cpe test` and `just cpe ci` pass (its test plan). No Phase review is recorded | none |

## Deferrals

- scope (Phase 1 review, rust + dune Warning): enforce in `contract::violations` that `Annotation.targets` is sorted,
  without repeats, and equal to the resource's own `Link` targets. Not fixed: the plan decides these are the adapter's
  promise (sort/repeats like value order) and that the port cannot tell which link properties name targets (a project
  subclass may carry other links); Phase 2's committed-0803 tests pin the equality. For the session's final review.

## Side findings

- Phase 1 review suggestions, not acted on: per-variant `///` docs on `Motivation` naming the `oa:` IRI (dune, rust);
  `DanglingTarget`'s Display could name `oa:hasTarget` (rust); a target listed twice and dangling reports two identical
  violations (consistency); nothing pins `annotation.is_some()` for every Region/LinkObj until Phase 2's committed test
  (dune DUNE-002, covered by the plan's "117 resources have `annotation: Some`" checkbox).
- `just commit-lint` reports the expected 2-commit count over origin/main; the PR must tick `allow-many-commits`.
- Phase 2 review, not acted on (suggestions or plan-fixed choices): rename `UnservedAnnotation` (e.g. `AnnotationNotAResource`; dune, simplicity) — the plan's message table fixes the name; one `ANNOTATION_PREDICATES` list in `vocab.rs` shared by the marker pre-pass and `annotation()` (dune DUNE-001); move "find annotations by `Resource.annotation`, never by `class`" into the ARCH-MAP cpe **Boundary rules** (dune DUNE-003); `///` docs on the other new `InvalidFact` variants (rust, dune); merge the three per-Region committed loops and drop the repeated count asserts, the 4,198 total and the highlighting mapping test (simplicity Warnings, refuted: each is a test the plan's checklist names, and repetition is the house test style); a `{what}_{condition}` rename of the Region mapping test (consistency).
- Plan deviation: "Extend the superseded-value test (`committed_0803.rs:104`)" landed as a separate test, `test_committed_0803_region_comment_with_superseded_versions_serves_current_value`; the old negative assertion on `xdvQ…` was false once Regions are served (that UUID is GOkuI's live comment).
- Final-review fix round: an `oa:hasTarget` to a node typed `dao:Value` is refused earlier as
  `UnfitSourceProperty { edge: oa:hasTarget }` (values mapping treats it as a value edge), not as `UnknownTarget`;
  the new test therefore uses an untyped value node, the case the refusal exists for. Still Unavailable either way.

## Closeout

- root_cause: The port dropped every `oa:Annotation` subject and every link to one, and had no geometry or color value kind, so the 77 `kb:Region` and 40 `kb:LinkObj` of the committed 0803 projection were not served.
- investigation: DAO marks annotations with `oa:` facts, not by class; LinkValue reifications are stripped by DAO and stay out; an annotation target outside the file is a broken snapshot, unlike an ordinary out-of-file link, which is dropped.
- solution: `cpe-ports` gains `ValueKind::Geometry` and `Color`, `Annotation` and `Motivation`, `Resource.annotation`, and the `DanglingTarget` and `UntargetedAnnotation` violations; `sync-store` serves every `dao:Resource`, maps motivation and targets, and refuses broken annotation facts with their own `InvalidFact`; the incubator vendors the merged commit (dsp-incubator#479).
- prevention: committed-0803 tests pin 4,198 resources, 77 Regions, 40 LinkObjs, 79 LinkObj links and 117 annotations whose targets equal their link targets; each annotation refusal has a single-fault test.
