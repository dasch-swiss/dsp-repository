---
status: proposed
date: 2026-09-25
---

# Git is the source of truth, and the editor's database is working state

The published project files committed to this repository under DPE's data directory are the source of truth for every project's metadata. The editor never writes them: an approved record leaves the editor only as a pull request against this repository, one per project, opened by `editor-collector` from the `editor-collect/<shortcode>` branch, and a change is published by the merge plus a release, never by the editor (`docs/src/editor/collection.md`). The editor's SQLite database holds working state only: drafts, submissions, review rounds, approved records not yet collected, the depositor table. Nothing in it is irreplaceable. Decided when the editor was scaffolded (2026-08-17, c0b7acd8) and stated on `docs/src/editor/architecture.md:3` since; recorded here retroactively (DEV-7374).

The rationale as the pages state it, to be confirmed by Balduin Landolt in review: git is the source of truth today because the archive is designed to replace it (root `CONTEXT.md`, Relationships), so the editor is a producer-side surface and not a store of record; the pull request plus a human merge is the gate between the editor's output and anything served, and the repository's own checks (`canonical_round_trip`, `every_committed_temporal_coverage_resolves`, `the_corpus_is_the_whole_published_set`) run on it because it is opened with `secrets.GH_TOKEN` (`ARCH-MAP.md`, `modules/dpe`, Durable state); and the database can therefore be backed up optionally and run with `synchronous=NORMAL` (`docs/src/editor/operations.md:241`). Why git rather than the editor's own database as the store of record: rationale not recorded on any page.

Stated so a violation is describable: no crate under `areas/deposit/` writes a file under DPE's data directory at runtime; `editor-collector` writes only a disposable checkout on an `editor-collect/<shortcode>` branch, never in place, and refuses to force-push over a tip it did not write; the editor holds no GitHub credential and makes no outbound GitHub call; nothing served is read from the editor's database.

## Considered Options

- **Git as the source of truth, reached through one pull request per project (chosen).**
- **Writing the files in place, a commit straight to `main`** — rejected: the pull request plus a human merge is the gate (`ARCH-MAP.md`).
- **The editor's database as the store of record** — not recorded as considered on any page; to be confirmed.

## Consequences

- Online is derived from the published set rather than stored (`areas/deposit/ADR-0003`), because the merge happens outside the editor.
- A total loss of the volume costs only re-creatable state (`docs/src/editor/operations.md:241`).
- The archive replaces git as the store of record in the target design (ADR-0002); this record is superseded when it does.

Enforced by: none mechanical (**review**); the collector's branch and skip rules are stated in `docs/src/editor/collection.md`.
