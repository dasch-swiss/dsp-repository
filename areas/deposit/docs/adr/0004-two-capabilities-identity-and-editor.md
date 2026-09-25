---
status: accepted
date: 2026-09-25
---

# The Deposit Area is two capabilities, identity and editor

The Deposit Area's modulith (ADR-0003) is composed of two capabilities. **identity** owns accounts, sessions, one-time login codes, the mail that carries them and the depositor list. **editor** owns the draft, the submission, the review, entity proposals, approved records and their collection, as one lifecycle aggregate. The direction was decided on 2026-09-24 and the shape below on 2026-09-25 (DEV-7374). The capability keeps the name `editor` (ADR-0002); "deposition" is the platform's word for the act of depositing and names no crate, directory or capability (`areas/deposit/editor/CONTEXT.md`, Flagged ambiguities). Two rather than more: review, proposals and collection stay inside editor because approve is one transaction across `submissions`, `review_rounds`, `approved_records` and `entity_proposals`, and splitting any of them off would trade that atomic transition for the eventual consistency ADR-0003 prescribes between capabilities, for an extraction nobody plans.

The code does not have this shape yet. `docs/src/editor/architecture.md` describes the code as it is and closes with its divergence from this record; DEV-7375 closes the entries.

## The shape

- `areas/deposit/server` — `deposit-server`, the area's composition root (ADR-0002): configuration, observability, opening the two databases, constructing every adapter and injecting it, assembling the router. No handler, no view, no SQL.
- `areas/deposit/shell` — `deposit-shell`, the area's document shell: `page`, the header with the viewer's name and the sign-out form, the footer, the 403 and 404 pages. A `fn -> Markup` library depending on `mosaic-tiles` and `shared-*` only, never on a capability crate. The routes its navigation links (`/projects`, `/review`, `/depositors`, `/logout`) are string literals: a coupling to both capabilities' route tables that no compiler checks.
- `areas/deposit/identity/{core,store,web}` — `identity-core` (User, Role, Session, LoginCode, the login-flow rules and send caps, its repository ports, the `Mailer` trait, cookie parsing and the session secret); `identity-store` (SQLite for `users`, `user_shortcodes`, `sessions`, `login_codes`, `mail_sends`; the SMTP and console mailers; the `Live*` adapters implementing `editor-ports`); `identity-web` (the login and sign-out pages, the depositor administration, their handlers and routes, its own `Authenticated` and `Rdu` extractors over its own session module). `store` holds identity's infrastructure adapters, SQLite and SMTP alike, behind traits `identity-core` declares: this area's reading of ADR-0003's "store".
- `areas/deposit/editor/{core,store,web,ports,collector}` — `editor-core` (unchanged); `editor-store` (SQLite for `drafts`, `submissions`, `review_rounds`, `approved_records`, `entity_proposals`, today `editor-server/src/db/`); `editor-web` (the views it has today plus the handlers and routes that today live in `editor-server`: sections, review, entities, projects, collection, `/api/v1`; its own `Authenticated(Principal)` and `Rdu` extractors over `editor-ports`); `editor-ports` (below); `editor-collector` (unchanged). There is no `identity-ports`: nothing identity does needs anything from editor.

## What crosses the boundary

- `editor-ports` declares a `Principal` (account id, display name, role, the folded shortcodes the account may reach) and two ports: an authenticator, taking the request's `Cookie` header and the current instant and answering `Option<Principal>`, and a user directory, taking account ids and answering display names. `editor-web` consumes both: the authenticator in its extractors, the directory in the review queue and the concurrent-save notice.
- `identity-store` implements them as `LiveAuthenticator` and `LiveUserDirectory` over its own tables. The session's idle-timeout touch on `GET` stays inside `LiveAuthenticator`: it is identity's write on identity's table.
- Each capability's web crate owns its extractors. `identity-web`'s read its session module directly; `editor-web`'s call the port. There is no shared extractor and no authentication middleware (ADR-0003, amendment of 2026-09-25).
- Every account reference editor stores (`drafts.updated_by`, `submissions.submitted_by` and `reviewed_by`, `review_rounds.actor`, `approved_records.approved_by`, `entity_proposals.proposed_by` and `decided_by`) is an opaque `TEXT` id with no `REFERENCES` clause. A removed account is what the directory answering nothing renders as; `review_rounds.actor` keeps its value for the audit trail.

## Two databases

Each capability opens its own SQLite file under `EDITOR_DB_DIR`, `identity.sqlite` and `editor.sqlite`, with its own migration series under its own `PRAGMA user_version` and its own single-writer pool; the in-memory variant is two named shared-cache databases. A store crate opens only its own file, so ADR-0003's "no cross-capability SQL" holds by structure rather than by review, and approve stays one transaction because every table it touches is editor's. Nothing irreplaceable is in either file (`docs/src/editor/operations.md`, Backups; the first retroactive record of this series, git as the source of truth, carries the decision).

## Considered Options

- **Two capabilities, identity and editor (chosen).**
- **One capability, the editor as it is** — rejected: accounts and sessions are the first thing the next capabilities (`media`, the data-model creator, data creation) need, and inside the editor they could only be reached through the editor's crates, the sibling import ADR-0003 forbids.
- **Three or more: review, entity proposals or collection as capabilities of their own** — rejected: approve deletes the submission, writes the round, applies the proposal decisions and writes the approved record in one transaction; as separate capabilities each step would be its own transaction, consistent eventually, for an extraction nobody plans.
- **`deposition` as the capability's name** — rejected: ADR-0002 confirmed `editor`, DEV-7373 moved the crates under that name, and `deposit`, `deposition` and `depositor` on adjacent doors say less than one of them.
- **One area-level `deposit-auth` crate holding one extractor for both web crates** — rejected: an extractor needs the session lookup, so the crate would depend on `identity-store` and every consumer on it transitively, the edge ADR-0003 forbids; behind a port it is the chosen shape with one crate more.
- **Authentication as a middleware in `deposit-server` putting a principal into request extensions** — rejected: the composition root would hold adapter logic, and the structure-level rule "a public handler is public in its signature" (`ARCH-MAP.md`) would go.
- **The shell in `deposit-server`, as DPE keeps it** — rejected: the handlers live in the capabilities' web crates and have to produce responses, so either each declares a shell port the root implements (view code at the composition root) or a root-level layer rewrites responses.
- **One shell per capability** — rejected: two copies of a header whose navigation spans both, drifting with no test.
- **A generic app-shell tile in `mosaic-tiles`** — not chosen now: it would cover DPE too and widens this decision into a Mosaic and DPE change; `deposit-shell` can be replaced by it later without touching a port.
- **One database file with the `REFERENCES` clauses dropped** — rejected: one `user_version` is one migration series that no capability owns, and any store crate could still read the other's tables.
- **Keeping the foreign keys until Bazel** — rejected: the constraint is the cross-capability join ADR-0003 forbids, and dropping it is a schema edit that costs nothing before the first deployment.
- **The SMTP transport in `identity-web` or in `deposit-server`** — rejected: a `lettre` dependency in a views-and-routes crate, or adapter logic at the composition root.

## Consequences

- `editor-server` disappears: `serve`, `main`, `cli`, `config` and `observability` move to `deposit-server`; `db/` to `editor-store` and `identity-store`; the handlers to `editor-web` and `identity-web`; `auth/` to `identity-core` and `identity-web`; `shell.rs` to `deposit-shell`. Configuration is loaded once at the root, and each capability receives the part it declares.
- `docs/src/editor/architecture.md`, `ARCH-MAP.md` and the `CONTEXT.md` files describe the code as it is; this record is the only place the target lives. The page's "Divergence from the target architecture" section lists, per entry, what differs, the clause here it violates and where in the code; each DEV-7375 step removes the entries it closes.
- `identity` gets its own `CONTEXT.md` (the five terms under "People and access" in `areas/deposit/editor/CONTEXT.md`) in the refactor step that creates `identity-core`, registered in the root index then (ADR-0003, Consequences).
- `EDITOR_DB_DIR` holds two files and the backup note in `docs/src/editor/operations.md` names both. The collector and the `/api/v1` routes are editor's and authenticate by bearer token as today, never by session.
- ADR-0003's amendment of 2026-09-25 records the three general rules this record applies: a consumer's authentication as a port with an extractor per web crate, one shell crate per area, and the crate-graph gate. The link is written from both ends (ADR-0006).

Enforced by: the two database files, each opened by one store crate (**structure**); `.github/scripts/check-capability-deps.sh` over `cargo metadata`, run by `just check`, once it lands with the crates (**static-analysis**), Bazel `visibility` after ADR-0001 (**structure**). Until the refactor lands: none (**docs-only**), with the divergence section of `docs/src/editor/architecture.md` as the record of what is still unenforced.
