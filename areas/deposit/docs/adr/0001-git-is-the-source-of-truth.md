---
status: accepted
date: 2026-09-25
---

# Git is the source of truth, and the editor's database is working state

The published project files committed to this repository under DPE's data directory are the source of truth for every project's metadata. The editor never writes them: an approved record leaves the editor only as a pull request against this repository, one per project, opened by `editor-collector` from the `editor-collect/<shortcode>` branch, and a change is published by the merge plus a release, never by the editor (`docs/src/editor/collection.md`). The editor's SQLite database holds working state only: drafts, submissions, review rounds, approved records not yet collected, the depositor table. Nothing in it is irreplaceable. Decided when the editor was scaffolded (2026-08-17, c0b7acd8) and stated on the opening paragraph of `docs/src/editor/architecture.md` since; recorded here retroactively (DEV-7374).

Git was chosen to keep the first iteration's infrastructure requirements simple, at a time when much on the infrastructure side was still open; it was never a store-of-record design. What the pages state follows from it: the editor is a producer-side surface and not a store of record; the pull request plus a human merge is the gate between the editor's output and anything served, and the repository's own checks (`canonical_round_trip`, `every_committed_temporal_coverage_resolves`, `the_corpus_is_the_whole_published_set`) run on it because it is opened with `secrets.GH_TOKEN` (`ARCH-MAP.md`, `areas/access/dpe`, Durable state); and the database can therefore be backed up optionally and run with `synchronous=NORMAL` (`docs/src/editor/operations.md`, Backups).

Stated so a violation is describable: no crate under `areas/deposit/` writes a file under DPE's data directory at runtime; `editor-collector` writes only a disposable checkout on an `editor-collect/<shortcode>` branch, never in place, and refuses to force-push over a tip it did not write; the editor holds no GitHub credential and makes no outbound GitHub call; nothing served is read from the editor's database.

## Considered Options

- **Git as the source of truth, reached through one pull request per project (chosen).**
- **Writing the files in place, a commit straight to `main`** — rejected: the pull request plus a human merge is the gate (`ARCH-MAP.md`).
- **The editor's database as the store of record** — deferred, not rejected: the infrastructure it needs was not settled when the first iteration was scoped; it is the next iteration's design (Consequences).

## Consequences

- Online is derived from the published set rather than stored (`areas/deposit/ADR-0003`), because the merge happens outside the editor.
- A total loss of the volume costs only re-creatable state (`docs/src/editor/operations.md`, Backups).
- The next iteration of the editor, no longer blocked on infrastructure, replaces git with the editor's own database as the store of record. Its architecture is not decided and gets its own record, which supersedes this one and settles how that store relates to the archive, which root `CONTEXT.md` (Relationships) names as git's designed replacement.

Enforced by: none mechanical (**review**); the collector's branch and skip rules are stated in `docs/src/editor/collection.md`.
