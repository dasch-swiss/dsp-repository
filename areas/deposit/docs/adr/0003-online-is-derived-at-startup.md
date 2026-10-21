---
status: accepted
date: 2026-09-25
---

# Online is derived at startup from the baked-in published set, never stored

Of the five depositor-facing project states, only three are stored. Online is derived by comparing each local record with the published set baked into the editor image (`EDITOR_DATA_DIR`), once, at startup, in `editor-server`'s `reconcile` module, through `editor_core::review::diff`, the same comparison the review surface renders from. The pass writes two things and nothing else: it deletes an approved record whose data the published set now carries (REQ-2.4), and it stamps `entity_proposals.retired_at` on an accepted new-entity proposal whose entity has appeared in the published set. Online is a resting state, not a transition. Decided on 2026-09-11 (8c9bde61); `docs/src/editor/status.md`; recorded here retroactively (DEV-7374).

Online is derived because nothing in the editor learns that a change has shipped except by looking at the data the deployment carries. Once is enough: the published set is baked into the image and cannot change while the process runs, so the moment a deployment carrying an approved change starts is the moment that change is Online (`docs/src/editor/status.md`, opening section). And it must be a resting state: REQ-2.4 discards the local record and a review round does not keep the approved payload, so after the discard nothing anywhere records that a change shipped; were Online only the moment of the transition, no depositor would ever see it (`status.md`, "Online is a resting state").

Stated so a violation is describable: no column stores Online; nothing polls the repository or receives a webhook; the comparison is `editor_core::review::diff` and no second implementation; the startup pass deletes approved records and retires proposals and writes nothing else; a failure of the pass is not fatal (`status.md`, "What the pass writes").

## Considered Options

- **Derive at startup from the baked-in set (chosen).**
- **Store Online at the moment of a transition** — rejected: after REQ-2.4's discard nothing records that a change shipped, so a stored transition would never be seen (`status.md`, "Online is a resting state").
- **Poll the repository or take a webhook at runtime** — not recorded as considered on any page.

## Consequences

- A change is Online at the next deployment, never before; `/states` shows the expected wait (REQ-2.6).
- A record whose pull request merged with reviewer edits reads Waiting for release indefinitely, and RDU resolves it by discarding it from `/collection` (`docs/src/editor/collection.md`, "What RDU sees in the editor").
- The published set stays the editor's, loaded once at the composition root and injected, not behind a port (`docs/src/editor/project-representation.md`, "The published set"; `areas/deposit/ADR-0004`).

Enforced by: the `reconcile` tests in `editor-server` pin the discard, the dropped-upstream case and the retirement (**static-analysis**); the single-comparison rule by **review**.

Note (2026-10-14): `editor-server` is now `deposit-server` (`areas/deposit/server`), and the `reconcile` module and its tests are in `editor-web` (`areas/deposit/editor/web/src/reconcile.rs`).
