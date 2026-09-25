---
status: proposed
date: 2026-09-25
---

# The draft is the project's JSON members, not a typed mirror

`editor_core::draft::ProjectDraft` is `#[serde(transparent)]` over the project file's members, a `serde_json` map, not a struct mirroring `ProjectRaw` with 36 `Option` fields. An absent key is a field not yet filled in, any value is kept as typed whether it validates or not, every member the editor does not manage is carried unchanged, and validity is decided once, at `to_raw`, the submission boundary. Recorded on `docs/src/editor/architecture.md:72` on 2026-08-28 (a8e35e2f); recorded here retroactively (DEV-7374).

The rationale as the page states it (`architecture.md:72`), to be confirmed by Balduin Landolt in review: "Three requirements pull that way at once: a draft must hold a field the depositor has not filled in and a value that is present but invalid, it must carry every field the editor does not manage unchanged, and it must survive a field being added to the contract without an editor change."

Stated so a violation is describable: no `editor-*` crate defines a struct whose fields mirror `ProjectRaw`'s members for the draft; `drafts.payload`, `submissions.payload` and `approved_records.payload` hold the serialized members and the persistence layer never interprets them (`architecture.md:63`); the editor's path is `ProjectRaw` → draft → `ProjectRaw`, never through `dpe_core::Project` (`ARCH-MAP.md`).

## Considered Options

- **The project's JSON members (chosen).**
- **A struct of 36 `Option` fields** — rejected: it cannot hold an invalid value as typed, drops members it does not know, and needs an editor change for every member added to the contract.

## Consequences

- Every applier works on `serde_json::Value`; a field's `form::Shape`, not a type, decides how a posted body is read back (`architecture.md:167`).
- The review diff compares top-level members, not registry fields, so a change arriving through a member no applier touches is still shown (`architecture.md:329`).
- Entity proposals cannot live in the payload, since `to_raw` would drop a non-contract member; hence the `entity_proposals` table (`architecture.md:93`).

Enforced by: `editor-core/tests/canonical_round_trip.rs` (load → draft → write byte-identical over the whole published corpus) and `editor-web/tests/untouched_form_round_trip.rs` (**static-analysis**).
