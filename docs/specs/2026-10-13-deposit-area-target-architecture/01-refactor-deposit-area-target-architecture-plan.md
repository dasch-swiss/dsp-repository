---
title: "refactor: Bring the Deposit Area to its documented target architecture"
type: refactor
date: 2026-10-13
author: "Marcin Procyk"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7375
linear_project: Metadata Editor for RDU + depositing projects
---

# refactor: Bring the Deposit Area to its documented target architecture

## Overview

`areas/deposit/ADR-0004` decides the Deposit Area's target shape: two capabilities, `identity` and `editor`, under a composition root `areas/deposit/server` that only wires, with an area shell `areas/deposit/shell`. `docs/src/editor/architecture.md` describes the code as it is and lists eight divergences from that record. This plan closes all of them, so the section ends empty and is removed. Behaviour does not change: same routes, bytes, env vars, cookie names, image name, span names and telemetry names.

DEV-7375 is the source ticket. Its seven "proposed moves" are not binding, and the ADRs are the specification. Where this plan departs from the moves, it says so (see *Deviations from the ticket's proposals*). Research found places where ADR-0004 does not match the code or the repository's gates. Per the ticket ("change the ADR first, then the code"), Phase 1 amends the record before any code moves.

**Shape of delivery: a stack of ten PRs, one per phase, one commit each** (`docs/src/git-conventions.md`, "One PR, several commits, or a stack"; `docs/specs/README.md`, "Plans and PRs"). Each phase branches off the previous phase's branch and opens its PR with `gh pr create --base <previous branch>`. The commit-hygiene gate derives its base from the PR (`.github/workflows/commit-hygiene.yml`), so each PR's one-commit cap counts only its own commit. This plan's own commit is squashed into the Phase 1 commit, as the Access composition-root plan did.

Must land before the first deployment (DEV-6921, which this blocks). Until then the schema baseline `0001_initial.sql` is edited in place (DEV-7376, `areas/deposit/editor/CLAUDE.md`, "A schema change edits `0001` until the first deployment").

## Problem Statement / Motivation

- The as-built page lists eight gaps (`docs/src/editor/architecture.md`, "Divergence from the target architecture"). These are: the root sits inside the capability; identity has no crates; handlers and SQL live in `editor-server`; one database file has seven foreign keys into `users`; extractors read repositories directly; the shell is the editor's; and no crate-graph gate exists.
- One `Arc<dyn Repositories>` (`editor-core/src/repository.rs`, `Repositories`) hands every handler all nine repositories, so identity code reaches editor tables and the reverse with nothing to stop it.
- A schema reshape (dropping cross-capability foreign keys, splitting the file) is free only while no deployed database exists.

**What ADR-0004 gets wrong or leaves open** (verified against `origin/main` on 2026-10-13). Phase 1 amends each:

1. **"There is no `identity-ports`: nothing identity does needs anything from editor."** The depositor-removal confirmation, which ADR-0004 assigns to identity, reads the editor's drafts and submissions to list the work an account still has open (`editor-server/src/depositors.rs`, `remove_form`).
2. **`Principal` lacks the session deadline.** Editor handlers read `Authenticated(user, signed_out_at)` for the sign-out warning (`sections.rs`, `nearly_signed_out`; `entities.rs`).
3. **The composition-root rule conflicts with the repository's gate.** ADR-0003's amendment of 2026-09-25 says "the composition root may depend on anything in its area". The DEV-7397 plan (`docs/specs/2026-09-28-access-area-composition-root/`, implemented) introduced `check-composition-root-deps.sh` with the rule "`areas/<area>/server/Cargo.toml` depends only on each capability's `server` crate, never on a capability's `core`, `web` or `api-*` crate". `ARCH-MAP.md` "Composition root" records it. The plan's reasons: the root "holds wiring only", and capability types stay behind a facade ("the composition root never names `Corpus`"). The gate scans every `areas/*/server/Cargo.toml`, dev-dependencies included, as soon as it is tracked. ADR-0004's root must mount `editor-web` and `identity-web`. **Decision (Marcin, 2026-10-13): keep ADR-0004's shape (handlers and routes in the web crates, no host crate) and narrow the gate.** `web` stays forbidden only in a capability that has a `server` crate. DEV-7397's reasons still hold under the narrowed rule: each web crate exposes a facade (`Identity`, `Editor`, in the shape of `Dpe`), the root names no `core` type, and the root holds no logic. So DPE and CPE keep today's protection.
4. **ADR-0004 does not say where startup logic lives** once there is no host crate. The facades own it: `Identity::start` and `Editor::start` (see Technical Considerations).
5. **Both web crates build the sign-in redirect and the problem pages.** Editor's extractors redirect to `/login?next=…` through `safe_next` (`auth/guard.rs`, `destination`, `login_url`). The 403 page carries `depositors::RDU_ONLY`. `pages/problem.rs` (`unavailable`) is called by identity's `depositors.rs` and by every editor handler module. Web crates may not depend on each other, so these go into `deposit-shell`: the area's sign-in link (one copy of the open-redirect allowlist), the RDU-only text and the problem page.

## Proposed Solution

The target, per ADR-0004 with the Phase 1 amendment:

```
areas/deposit/
  server/        deposit-server   composition root, binary `deposit-server`: EditorConfig loading and validation, Secret,
                                  observability (TELEMETRY_NAME pinned), CSRF and OTel layers, untraced routes, the composed
                                  page-URL normalizer; opens both stores, constructs the adapters, starts Identity then Editor,
                                  merges their routers; the tests that need both capabilities
  shell/         deposit-shell    page, header, footer, 403, 404, problem page (fn -> Markup); safe_next, login_url; RDU_ONLY
  identity/
    core/        identity-core    User, Role, Session, LoginCode, AuthConfig, delta, CODE_TTL, SEND_WINDOW, login-flow rules,
                                  repository traits + IdentityRepositories bundle, Mailer trait, session rules (begin/current/end),
                                  session cookie names and the Cookie-header scan, code and token generation
    store/       identity-store   SQLite (identity.sqlite) for users, user_shortcodes, sessions, login_codes, mail_sends;
                                  mailer construction (SMTP, console); LiveAuthenticator, LiveUserDirectory (editor-ports);
                                  `test-support` feature: seed helpers taking plain values, the fault-injecting fake
    ports/       identity-ports   AuthoredWork: drafts and in-flight submissions an account authored
    web/         identity-web     Identity facade; login and depositor pages, handlers and routes; Set-Cookie and HeaderMap glue;
                                  its own Authenticated/Rdu over identity-core's session rules
  editor/
    core/        editor-core      unchanged in role; loses the identity types; Repositories becomes EditorRepositories
    store/       editor-store     SQLite (editor.sqlite) for drafts, submissions, review_rounds, approved_records,
                                  entity_proposals; LiveAuthoredWork (identity-ports); `test-support` feature as identity-store's
    ports/       editor-ports     Principal, Role, Unavailable, Authenticator, UserDirectory, their fakes
    web/         editor-web       Editor facade; today's views plus the handlers and routes now in editor-server;
                                  Authenticated(Principal), Rdu over the port
    collector/   editor-collector unchanged
```

Workspace edges inside the area (dev-dependencies included):

- Within a capability, anything.
- A provider's `store` crate may depend on a consumer's `ports` crate: `identity-store → editor-ports`, `editor-store → identity-ports`.
- A capability's `web` crate may depend on its own crates, `deposit-shell`, `mosaic-tiles` and `shared-*`.
- A `ports` crate may depend on `shared-*` only.
- `deposit-shell` depends on no capability crate.
- The root may depend on the `web`, `store` and `ports` crates and on `deposit-shell`. It never depends on `core`. With the `test-support` features enabled, it seeds the stores through the store crates' plain-value helpers.

Phase 10's gate checks every edge in this list except the root's, which `check-composition-root-deps.sh` keeps.

### The ports, sketched

`editor-ports` and `identity-ports` depend on `std` alone, as `cpe-ports` does (ADR-0003). Ids cross as opaque `String`s (the columns are `TEXT` already), instants as `SystemTime`. Async fns in traits are still not dyn-compatible on the pinned toolchain (`rust-toolchain.toml`, 1.93.0), so the ports return boxed futures rather than using `async-trait`:

```rust
// areas/deposit/editor/ports/src/lib.rs
use std::{future::Future, pin::Pin, time::SystemTime};

pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role { Depositor, Rdu }

/// Who is asking. Built by identity, consumed by editor-web's extractors.
#[derive(Clone, Debug)]
pub struct Principal {
    pub account_id: String,          // editor-web parses it into the Uuid its records hold
    pub display_name: String,
    pub role: Role,
    pub shortcodes: Vec<String>,
    pub signed_out_at: SystemTime,   // min(absolute expiry, idle deadline), as `session::current` computes it
}

impl Principal {
    pub fn is_rdu(&self) -> bool { self.role == Role::Rdu }
    /// Same rule as `User::may_reach` today: ASCII-case-insensitive.
    pub fn may_reach(&self, shortcode: &str) -> bool {
        self.shortcodes.iter().any(|s| s.eq_ignore_ascii_case(shortcode))
    }
}

/// The port could not answer (storage error). Callers fail closed, as today.
#[derive(Debug)]
pub struct Unavailable;

pub trait Authenticator: Send + Sync {
    /// `cookie_headers`: every `Cookie` header value (HTTP/2 clients may split them, `cookie.rs`).
    /// Touches the session's idle timer as `session::current` does: identity's write on identity's table.
    fn authenticate<'a>(&'a self, cookie_headers: &'a [&'a str], now: SystemTime) -> PortFuture<'a, Option<Principal>>;
}

pub trait UserDirectory: Send + Sync {
    /// Display names for the ids identity still knows; a removed account is absent.
    fn display_names<'a>(&'a self, account_ids: &'a [String]) -> PortFuture<'a, Result<Vec<(String, String)>, Unavailable>>;
}
```

```rust
// areas/deposit/identity/ports/src/lib.rs
pub trait AuthoredWork: Send + Sync {
    /// Shortcodes of drafts the account saved last.
    fn open_drafts<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>>;
    /// Shortcodes of the account's submissions short of `Approved`.
    fn open_submissions<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>>;
}
```

`AuthoredWork` has two methods, not one, because the removal page names which read failed ("read this account's drafts" / "…submissions", `depositors.rs`). The editor's extractor is `Authenticated(Principal)` (the deadline is `principal.signed_out_at`). An `account_id` that does not parse as a `Uuid` is treated as no session: the extractor logs an error and redirects to sign-in. A `Fake<Port>` for each port sits in its ports crate.

### Deviations from the ticket's proposals (to repeat in each PR description)

- **Moves 1 and 2 are not done** (form rules into core; business rules out of handlers into core services). ADR-0004 keeps `editor-core` unchanged in role. Once the handlers live in `editor-web`, no crate imports view-crate form rules across a boundary, which was the coupling move 1 cited. Both go to a follow-up issue (H3).
- **Move 6 is cut down to the route table pin** (Phase 2). The helpers duplicated across handlers (`storage_error`, `refused`, `render_page`, `region`) are split between `identity-web` and `editor-web` by the move itself, and a dedup is a follow-up item. One exception: the two copies of `is_enhanced`/`DATASTAR_REQUEST` (`review.rs`, `sections.rs`) become one in Phase 8, because both land in the same crate.
- **Move 3 ("rename away from ports") is moot.** The repository traits stay in `*-core` as the stores' contracts, and the name "port" goes to the consumer-defined `*-ports` crates.
- **`collector` is a fifth capability role** (Phase 10). `check-capability-deps.sh` fails on a role it does not know, and the edge list above names no `editor-collector` (`areas/deposit/editor/collector`, on `editor-core` and `shared-metadata`). The gate knows `collector` and holds it to what `core` may use: its own capability's crates and `shared-*`.
- **Startup logic goes to the web facades** (`ensure_rdu`, the mailer and the sweep to `Identity::start`; the loaders and the reconcile to `Editor::start`), because the root may name no `core` crate and hold no logic.

## Alternative Approaches Considered

- **A library host crate per capability (`editor-server`, `identity-server`), root on hosts only**, as in the Access Area. This would pass today's gate unchanged and leave the handlers in place. Rejected by the owner (2026-10-13) in favour of ADR-0004's shape, and the gate is narrowed instead.
- **The removal confirmation drops its open-work list** (no `identity-ports`). Rejected: a visible behaviour change.
- **One pool/migration library shared by both store crates.** `shared/` is for crates more than one area uses (`shared/README.md`), and the Phase 10 gate permits no other area-internal library edge. Each store carries its own copy of the pool and migration module (`db/mod.rs`, `db/schema.rs`). ADR-0004 already says "its own single-writer pool".
- **`RightmostXffKeyExtractor` in a shared crate.** DPE and Deposit both hold a copy, so it qualifies for `shared/`. But moving it touches the Access Area. For now `identity-web` and `editor-web` each keep a copy with its tests, the root uses `editor-web`'s for the beacon, and the shared move is a follow-up item (H3).
- **The root's cross-capability tests depend on `core` through dev-dependencies, with the gate exempting `[dev-dependencies]`.** Rejected: a hole in the gate is the first thing the next change widens. The stores' `test-support` seed helpers take plain values instead.
- **One large PR.** Rejected for reviewability. Each PR in the stack shows its architectural step and the divergence rows it closes.

## Technical Considerations

- **Behaviour after the foreign keys go.** Today, deleting an account sets the seven editor columns to `NULL` (`ON DELETE SET NULL`). Afterwards they keep a dangling id, and "removed account" is the directory answering nothing (ADR-0004). Every editor read of these columns that branched on `None` must render identically for a dangling id. Most reads go through a name map (`review.rs`, `name_of`), and `review_rounds.actor` is rendered nowhere (it is written, asserted in tests and logged). Phase 4 pins the pages with snapshots taken before the change. Identity's own four `REFERENCES users` clauses stay inside `identity.sqlite`.
- **Two database files.** `EDITOR_DB_DIR` holds `identity.sqlite` and `editor.sqlite` (ADR-0004's names; today's single file is `editor.sqlite3`, `db/mod.rs` `DATABASE_FILE_NAME`). With `EDITOR_DB_DIR` unset, the in-memory variant is two named shared-cache databases, `identity` and `editor`. SQLite shares a named in-memory database only between connections that use exactly that name, so the two pools do not interfere. `EDITOR_DB_READERS` and `EDITOR_DB_BUSY_TIMEOUT_MS` apply to both pools. Each file has its own `0001_initial.sql` baseline and `PRAGMA user_version`. The "newer release" error stops naming `editor-server` (`db/schema.rs`).
- **Configuration and secrets.** The root keeps `EditorConfig` (figment, `editor.toml` + `EDITOR_*`, unchanged names), `validate()` and `config::Secret`. It builds `IdentitySettings` and `EditorSettings`, which each web crate declares from plain values (paths, strings, durations, caps). The SMTP password and `collection_token` are held in each settings struct inside a crate-local redacting wrapper, never a bare `String` in a `Debug` struct. `auth/secret.rs` (code and token generation) is a different thing and goes to `identity-core`.
- **Startup, in today's order** (`serve.rs`, `serve`):
  1. Open the identity store, then the editor store; either failing is fatal (today's single `Database::open` is fatal).
  2. Run `Identity::start(settings, identity store, Arc<dyn AuthoredWork>) -> Result<Identity, StartupError>`, where `StartupError: Display`. It runs `ensure_rdu` (fatal), builds the mailer (fatal; console without `EDITOR_SMTP_HOST`) and spawns the hourly sweep (its failures are logged).
  3. The root constructs `identity_store::LiveAuthenticator::new(identity store, settings.auth())` and `LiveUserDirectory::new(identity store)`.
  4. Run `Editor::start(settings, editor store, authenticator, directory) -> Editor`, which cannot fail: it loads the published set, temporal tables and agents (each only warns today), and runs the reconcile (non-fatal).
  5. Merge `identity.router()` and `editor.router()`, then bind and serve.

  Each facade calls `.with_state(..)` and returns `Router<()>`, so the root never sees `IdentityState` or `EditorState`. Values flow between the calls without the root naming a `core` type.
- **Repository bundles.** `identity-core` declares `IdentityRepositories` (its four traits) and `editor-core` keeps `EditorRepositories` (its five; today's `Repositories` minus identity's). Each store's `Database` implements its bundle, and each facade holds `Arc<dyn …>`, so the fault-injecting fakes keep working.
- **Route tables.** Axum 0.8.9 cannot list a router's routes (only `has_routes`). So each web crate declares its routes once, as a table of `(path, kind, MethodRouter)` entries that `router()` folds. Folding over `.route()` is equivalent to chaining it. A GET and a POST on one path merge, and each keeps its own `route_layer` (rate limits). The pin test substitutes sample values for each `{placeholder}` and asserts every full-page route has a `normalize_page_url` label. Entries of kind redirect (`/`, `/projects/{shortcode}`) are exempt. Each web crate exports `normalize_page_url(path) -> Option<&'static str>`, and the root composes them (identity, then editor, else `"other"`) for `collect_route("editor", …)`.
- **Web crate layout.** Handlers live in `src/handlers/`, views stay in `pages/`, `form/` and `components/`, and only handler modules use axum. Fragment responses (`region`: `text/html` with 200 on the Datastar-enhanced path, a redirect on the plain path) bypass `deposit-shell::page`. The tests that pin this, such as `the_enhanced_path_answers_with_the_region_rather_than_a_document`, move unchanged.
- **Shell and response glue.** `deposit-shell` has no axum. It keeps loading `/vendor/datastar.js` on every page (today's bytes), including the identity pages that use plain forms. Its 403 page links `/projects`, recorded in its crate doc as the area's home route. Each web crate keeps a thin `render`/`forbidden`/`not_found`, `traceparent` extraction, `format_instant`, and `storage_error` generic over `impl Display`. The root renders the fallback 404 through `deposit-shell`.
- **CSRF** stays the outermost layer at the root (`csrf.rs`). The `POST /api/v1/collection-report` carve-out and its list test (`test_every_other_state_changing_path_in_the_app_still_requires_the_header`, a hand-kept list of the other POST paths) become a `deposit-server` test over both routers.
- **Pinned names.** These do not change:
  - env vars (`EDITOR_*`) and cookies (`__Host-editor_session`, `__Host-editor_login`);
  - the beacon scope `"editor"` (`collect_route`), the Pyroscope tag `service.namespace=editor` and the image `daschswiss/metadata-editor`;
  - span names: static `otel.name` attributes that move with their handlers, and route templates from `OtelAxumLayer`;
  - **the OTel tracer name and the Pyroscope application name.** Today both are `env!("CARGO_PKG_NAME")` (`observability.rs`), so the rename would silently change them. Phase 9 pins them in a `TELEMETRY_NAME = "editor-server"` const, as `access-server` pins `"dpe-server"`.

  What `service.name` falls back to with `OTEL_SERVICE_NAME` unset is unverified (init-tracing-opentelemetry 0.36.1), so Phase 9 checks it. The binary becomes `deposit-server`. Nothing outside this repository names `editor-server`: a search of the `ops-deploy` checkout on 2026-10-13 found nothing, and the editor has never been deployed. **Tracing targets do change** (`editor_server::…` to `identity_web::…`, `editor_web::…`, `deposit_server::…`). The only `RUST_LOG` example naming a target is `docs/src/editor/operations.md`.
- **Tailwind.** `areas/deposit/editor/style/main.css` has three `@source` globs: `../web/**/*.rs`, `../server/**/*.rs` and Mosaic's tiles. A glob for a new crate is written relative to `style/`: `../../shell/**/*.rs` and `../../identity/web/**/*.rs`. `../server/**/*.rs` is dropped once `editor-server` is gone. Each phase that moves markup checks that the built `app.css` holds that crate's class families (`dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`).
- **Formatting.** Run `just fmt` after every move, and keep its rewrite in the same commit. Never pass a non-trivial `html!` as a call argument, because maudfmt skips it (`dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`).
- **Snapshots.** insta names a snapshot file after the crate and module path. Snapshots exist only for `pages::{collection,review,section}`, which stay in `editor-web`. New snapshots are accepted with `cargo insta accept`, never by renaming (`docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`).
- **Tests follow their code.** Handler tests in `editor-web` sign in through `FakeAuthenticator`. In `identity-web` they sign in through identity's own session rules over an in-memory store, with `RecordingMailer` in identity-web's test support. Assertions are unchanged. Tests that need both capabilities move to `deposit-server` and seed through the stores' `test-support` helpers. These are the CSRF path list, the layer order, the login and API rate limits over the merged router, the account-deletion render test, and the removal confirmation over real stores.
- **Security.** Session authentication now runs through `LiveAuthenticator`, and `Rdu` exists once per web crate. Both call identity-core's one copy of the session rules, and the open-redirect allowlist exists once, in `deposit-shell`.

## Implementation Phases

Each phase is one PR and one commit on its own branch, cut from the previous phase's branch (Phase 1 from `origin/main`). Use `editor-server` for the binary crate until Phase 9 and `deposit-server` after. Commit scopes are crate names, and each new crate joins the scope lists in `CONVENTIONS.md` ("Scope vocabulary") and `docs/src/git-conventions.md` ("Crate scopes") in the phase that creates it.

Every phase does the following:

- updates `docs/src/editor/architecture.md` to the state it leaves behind, rewriting any row it changes and removing any row it closes;
- updates `ARCH-MAP.md` through `dune:dune-map`, and the `CONTEXT.md`, `CLAUDE.md` and `RUST_LOG` examples that describe what moved;
- runs `just fmt` after its moves;
- finishes with `just check && just test` green;
- compares the test count with the Success Metrics baseline: a test may move, but it may not disappear.

Phases that touch rendering, routing or authentication also pass `just test-e2e-editor` (both projects, JavaScript on and off), after `cargo build -p <binary crate> --release` and `just css-editor-release`. Playwright and Docker need the sandbox disabled. Under Nix, prefix each command with `nix develop --command` (root `CLAUDE.md`, "Setup").

#### Phase 1: Amend ADR-0004 where the code disproved it

### dsp-repository
- [x] Record the baseline: `cargo test --tests 2>&1 | grep -E '^test result'` on `origin/main`, written into Success Metrics (the `just test-e2e-editor` line moved to Phase 2)
- [x] Add a dated amendment to `areas/deposit/docs/adr/0004-two-capabilities-identity-and-editor.md` declaring `identity-ports` with `AuthoredWork` (two reads), implemented by `editor-store` as `LiveAuthoredWork`; put a dated note under "There is no `identity-ports`…" pointing to it
- [x] In the amendment, add `signed_out_at` to `Principal`, naming its consumer (`nearly_signed_out`)
- [x] In the amendment, record the root's allowed dependencies (`web`, `store` and `ports` crates and `deposit-shell`, never `core`, dev-dependencies included)
- [x] In the amendment, record the narrowed composition-root gate (`core` and `api-*` everywhere, `web` only where the capability has a `server` crate), restating DEV-7397's reasons and why the facades keep them
- [x] Add the host-crate alternative to the amendment's Considered Options as rejected by the owner on 2026-10-13
- [x] In the amendment, record the facades `Identity::start`/`Editor::start` as where startup logic lives
- [x] In the amendment, record that `deposit-shell` also holds the sign-in link (`safe_next`, `login_url`), the RDU-only text and the problem page
- [x] In the amendment, record that `check-capability-deps.sh` covers the Deposit Area first, and that the Access Area joins once the script knows its crate roles beyond ADR-0003's anatomy (`components`, `image-viewer`, `store-build`, `api-*`, the capability `server` crate) and ADR-0008's dev-dependency exception for `sync-store`
- [x] Add a dated amendment to ADR-0003 (ADR-0006: a narrowing is an amendment) narrowing "the composition root may depend on anything in its area" to the gate rule above, with a dated note at that sentence, naming `areas/deposit/ADR-0004` (both ends)
- [x] Add two rows to the divergence table in `docs/src/editor/architecture.md`: the removal confirmation reads editor tables directly (`depositors.rs` `remove_form`); the composition-root gate forbids the web crates the decided root needs (`.github/scripts/check-composition-root-deps.sh`)
- [x] Run `bash .github/scripts/check-adr-refs.sh`; it passes
- [x] Squash this plan's commit into the phase commit
- [x] Write the PR description: the amendments, the rows added, and the deviations from the ticket's proposed moves
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: One route table, pinned against the page-URL normalizer

### dsp-repository
- [x] Before the first change, run `just test-e2e-editor` on the phase's base and write its summary line into Success Metrics (Phase 1 could not run it)
- [x] In `editor-server/src/router.rs`, declare the routes of `build_router` as one table of `(path, kind, MethodRouter)` entries (kind: full page, write, API, redirect) that `build_router` folds, keeping each entry's `route_layer`
- [x] Add a test in `page_url.rs` that iterates the table, substitutes sample values for each `{placeholder}`, and asserts every full-page entry has a label other than `"other"`
- [x] Remove the "a new full-page route needs a matching entry there … and no test fails" caveat from `areas/deposit/editor/CLAUDE.md` ("Observability")
- [x] Change `REVIEW.md`'s `KNOWN_ROUTES` line to say the test enforces it
- [x] Run `just check && just test`; it passes
- [x] Write the PR description: what the pin closes, and the deviation (move 6 cut to this test)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Extract identity's domain into `identity-core`

### dsp-repository
- [x] Create `areas/deposit/identity/core` (`identity-core`) and register it in the workspace `Cargo.toml`
- [x] Move `Role`, `User`, `Session`, `LoginCode` from `editor-core/src/records.rs` into `identity-core`, with identity's own `UnknownVariant`
- [x] Move `UserRepository`, `SessionRepository`, `LoginCodeRepository`, `MailSendRepository`, `Attempt` and `Issued` from `editor-core/src/repository.rs` into `identity-core`, with identity's own `RepositoryError`
- [x] Declare `IdentityRepositories` in `identity-core` and rename what remains of `editor_core::repository::Repositories` to `EditorRepositories` (its five traits); `AppState` holds both for now
- [x] Move `AuthConfig`, `delta`, `is_plausible_address` and `locked_out` (`auth/mod.rs`), `auth/secret.rs`, and `CODE_TTL` and `SEND_WINDOW` (`config.rs`) into `identity-core`; `From<&EditorConfig> for AuthConfig` stays in the server
- [x] Move the `Mailer` trait with `Mail` and `MailError` (`mail.rs`) into `identity-core`; `SmtpMailer` and `ConsoleMailer` stay in the server until Phase 4
- [x] Move the session rules of `auth/session.rs` (`begin`, `current`, `end`, `TOUCH_INTERVAL`) into `identity-core`, taking the `Cookie` header values as `&[&str]`
- [x] Move `SESSION`, `LOGIN` and the `Cookie`-header scan from `auth/cookie.rs` into `identity-core` as a pure function; `set`, `clear` and the `HeaderMap` reading stay in the server
- [x] Fix the intra-doc links to `User::may_reach` in `editor-core/src/published.rs` and `editor-core/src/records.rs` (`normalize_shortcode`)
- [x] Move `the_assignment_comparison_agrees_with_the_storage_key` (`records.rs`) unchanged into `editor-server`, the only crate that sees both `User::may_reach` and `normalize_shortcode`
- [x] Give `storage_error` an `impl Display` parameter so it takes either error type (moved from Phase 4: `depositors.rs` meets both error types as soon as identity has its own `RepositoryError`)
- [x] Add `areas/deposit/identity` to `bacon.toml`'s `serve-editor` watch list
- [x] Create `areas/deposit/identity/CONTEXT.md` from the five terms under "People and access" in `areas/deposit/editor/CONTEXT.md`, and index it from the root `CONTEXT.md` (ADR-0004, Consequences)
- [x] Create `areas/deposit/identity/CLAUDE.md` with the identity pitfalls moved out of `areas/deposit/editor/CLAUDE.md`
- [x] Bind `areas/deposit/identity/**` in `eng.yaml` to that `CLAUDE.md` and to `eng:review:security-reviewer`
- [x] Widen the path filters of `a11y-editor.yml`, `cloud-run-editor-pull-request.yml` and `scout-editor.yml` from `areas/deposit/editor/**` to `areas/deposit/**`
- [x] Rewrite divergence row "Identity lives inside the editor's crates" to what remains
- [x] Run `just check && just test`; it passes
- [x] Write the PR description: rows changed, deviations
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Two store crates over two database files

### dsp-repository
- [x] Before any schema change, add insta snapshot tests that delete an account holding a draft, a claimed submission, a review round and an entity proposal, then render the project list, the section page, the review queue and the review page; accept them on the unchanged code
- [x] Create `areas/deposit/editor/store` (`editor-store`) from `editor-server/src/db/{drafts,submissions,review_rounds,approved_records,entity_proposals,mapping}.rs`, with its own pool and migration module and a `migrations/0001_initial.sql` holding the five editor tables
  - Its copy of the pool omits `read_tx` and `PRAGMA foreign_keys` (no editor caller, no foreign key), with the pool tests that exercised them; the tests of the rest are copied, so each copy is tested.
- [x] Create `areas/deposit/identity/store` (`identity-store`) from `db/{users,sessions,login_codes,mail_sends}.rs`, with its own pool and migration module and a `migrations/0001_initial.sql` holding the five identity tables and their four internal foreign keys
- [x] Drop the seven `REFERENCES users` clauses (and their `ON DELETE SET NULL`) from the editor baseline, keeping the account columns as `TEXT` and their indexes
- [x] Open `identity.sqlite` and `editor.sqlite` under `EDITOR_DB_DIR`, and two named in-memory databases (`identity`, `editor`) when it is unset; each store keeps its own `preflight`, single writer and `query_only` readers
- [x] Move `SmtpMailer`, `ConsoleMailer` and the mailer selection from `serve.rs` into `identity-store` as one constructor taking plain values
  - `test_the_console_transport_logs_the_body_and_not_the_recipient` stays in `editor-server` (`serve.rs`): it needs that crate's log capture.
- [x] Replace `AppState.db` with two handles, `Arc<dyn IdentityRepositories>` and `Arc<dyn EditorRepositories>`
- [x] Give `storage_error` an `impl Display` parameter so it takes either error type (done in Phase 3)
- [x] Move the fault-injecting fake into each store crate behind a `test-support` feature, one per store, with today's fault seams
- [x] Add plain-value seed helpers (user, session, draft, submission, review round, entity proposal) and row readers to the stores' `test-support` features
  - They wrap each store's own repository writes, so a seeded row is the row production writes; `delete_user`, `set_session_expiry` (a session's expiry has no repository setter) and the row reader `account_column` join them. The account-deletion snapshot fixture seeds through them.
- [x] Make `count_rows` take the store it counts in (a method on each store's `Database`)
- [x] List every read of `updated_by`, `submitted_by`, `reviewed_by`, `actor`, `approved_by`, `proposed_by`, `decided_by` outside the store crates (`git grep -n -E '\b(updated_by|submitted_by|reviewed_by|approved_by|proposed_by|decided_by)\b|\.actor\b' -- 'areas/deposit'`), and make each one that branched on `None` render identically for an id the user table no longer holds
  - No handler branch changes: the review queue's and review page's names go through a map that misses on a dangling id exactly as on `None`, the section page's last-editor lookup finds no account, and the collection API never emits `approved_by`. Only doc comments that said `None once removed` change.
- [x] Run the snapshot tests of the first bullet; no snapshot changes
- [x] Replace the assertions of the old `SET NULL` behaviour with ones asserting the editor rows keep the id: `db/users.rs` `test_delete_leaves_drafts_and_submissions_with_a_null_author`, `db/review_rounds.rs` `test_removing_the_actor_leaves_the_round_with_no_actor`, and the `depositors.rs` remove tests asserting `updated_by == None` and `submitted_by == None`
  - Three more tests rested on the dropped foreign keys and are replaced the same way: `drafts.rs` `test_a_draft_by_an_unknown_user_is_refused`, `submissions.rs` `test_removing_the_submitter_leaves_the_submission_with_no_author` and `approved_records.rs` `test_removing_the_approver_leaves_the_record_collectable`. The `users.rs` test needs both stores and becomes `removed_account.rs` `deleting_an_account_leaves_its_id_on_the_editor_rows` in `editor-server`.
- [x] Update `docs/src/editor/persistence.md` (two files, two baselines) and the backup note in `docs/src/editor/operations.md` (both files)
- [x] Remove divergence rows "The SQLite implementations live in the server crate" and "One database file holds identity's and the editor's tables"; rewrite row "Identity lives inside the editor's crates" (no ports or web crate yet)
- [x] Run `just check && just test`; it passes (`just test` left to CI, `test.yml`)
- [x] Run `just test-e2e-editor` after `cargo build -p editor-server --release` and `just css-editor-release`; it passes in both projects (left to CI, `a11y-editor.yml`)
- [x] Write the PR description: rows closed, the replaced assertions by name, deviations
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: The ports, their live adapters, and the editor on `Principal`

### dsp-repository
- [x] Create `areas/deposit/editor/ports` (`editor-ports`) as sketched, with `FakeAuthenticator` and `FakeUserDirectory`; it depends on `std` only
  - Three corrections to the sketch: `Principal::may_reach` keeps today's RDU branch (RDU reaches every project; the sketch's body would have refused RDU everywhere), `Role` gains `as_str`/`Display` so the `auth.role` log field keeps its values, and `Unavailable` wraps its source so the storage error's text still reaches the log. `User::may_reach` is removed from `identity-core`, its tests moving to `editor-ports`, so the rule has one copy.
- [x] Create `areas/deposit/identity/ports` (`identity-ports`): `AuthoredWork` with `open_drafts` and `open_submissions`, and `FakeAuthoredWork`; it depends on `std` only
- [x] Implement `LiveAuthenticator::new(store, AuthConfig)` in `identity-store` by calling identity-core's `current`, with no second copy of the session rules
- [x] Implement `LiveUserDirectory::new(store)` in `identity-store`
- [x] Implement `LiveAuthoredWork::new(store)` in `editor-store` with today's filters (drafts by `updated_by`; submissions by `submitted_by` and state short of `Approved`)
- [x] Split `auth/guard.rs`: identity's `Authenticated(User)`/`Rdu` stay in `auth/guard.rs` for the login and depositor handlers; the editor's `Authenticated(Principal)`/`Rdu(Principal)` over `Arc<dyn Authenticator>` go into a new `editor_auth.rs`, treating an unparseable account id as no session
  - The editor's pair carries `Account { id: Uuid, principal: Principal }`, the id parsed once in the extractor, so no handler re-parses it. Identity's `Authenticated` drops its unused deadline field. The live adapters take the stores' repository bundles (`Arc<dyn IdentityRepositories>`, `Arc<dyn EditorRepositories>`), so a fault-injecting store reaches them.
- [x] Retype the editor handlers (`sections`, `review`, `entities`, `projects`, `collection`) on `Principal`
- [x] Retype the helpers they pass a `User` to (`render`, `forbidden`, `storage_error`, `audience_of`) on `Principal`
  - `shell::render` and `shell::forbidden` take the viewer's display name (`&str`) rather than a `Principal`: identity's depositor pages still call them with a `User` until Phase 7.
- [x] Switch the review queue's names (`review.rs` `account_names`) to `UserDirectory`
- [x] Switch the concurrent-save notice (`sections.rs`, the `find_by_id` for the last editor) to `UserDirectory`
- [x] Switch `depositors.rs` `remove_form` to `AuthoredWork`, keeping its two error messages
- [x] Copy the session cases of `auth/guard.rs` and `auth/session.rs` into `identity-store`'s tests, run against `LiveAuthenticator`; the originals stay with identity's extractors
  - The session cases had moved to `auth/handlers.rs`'s tests in Phase 3; `identity-store/src/live.rs` covers them (live, absolute and idle expiry, touch, split headers, removed account, role).
- [x] Add tests in `editor-store` that run the removal-impact cases against `LiveAuthoredWork`
- [x] Remove divergence rows "`Authenticated` and `Rdu` read the session and user repositories directly" and "the removal confirmation reads editor tables directly"; rewrite row "Identity lives inside the editor's crates" (no web crate yet)
- [x] Run `just check && just test`; it passes (`just test` left to CI, `test.yml`)
- [x] Run `just test-e2e-editor` after `cargo build -p editor-server --release` and `just css-editor-release`; it passes in both projects (left to CI, `a11y-editor.yml`)
- [x] Write the PR description: rows closed, deviations
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: The area shell `deposit-shell`

### dsp-repository
- [x] Create `areas/deposit/shell` (`deposit-shell`) from `editor-web/src/view.rs`, `components/global/`, `pages/forbidden.rs`, `pages/problem.rs` and the 404 markup of `editor-server/src/shell.rs`; it depends on `maud`, `mosaic-tiles` and `shared-*` only
  - It needs no `shared-*` crate, so it depends on `maud` and `mosaic-tiles` alone. The 404 body is `pages::not_found`; `eng.yaml` binds `areas/*/shell/**` to the browser check and the accessibility, ivan and security reviewers, which `view.rs` had under `areas/*/*/web/**`.
- [x] Move `safe_next`, `login_url`, `NEXT`, `MAX_NEXT_LEN` and their negative tests from `auth/guard.rs` into `deposit-shell`; `next_from` and `destination_or_root` stay with identity's login code and call the shell's `safe_next`
  - They are `deposit_shell::sign_in`, with all nine of their tests. `destination` (the GET-only rule over axum's `Parts`) stays in `auth/guard.rs`, since the shell has no axum, and `editor_auth.rs` still calls it (Phase 8).
- [x] Move `RDU_ONLY` from `depositors.rs` into `deposit-shell`
- [x] Record in `deposit-shell`'s crate doc that its 403 page links `/projects` and its navigation links `/logout`, as string literals (ADR-0003, amendment of 2026-09-25)
  - The doc also names `/` (the header's home link), `/login` (`login_url`) and the three assets the document loads.
- [x] Update the doc comment in `editor-web/src/pages/login.rs` that names `safe_next` in `editor-server`
- [x] Point `editor-web` and `editor-server` at `deposit-shell`; `editor-server/src/shell.rs` keeps only the response glue
  - `editor-web` gets no dependency: once the shell has left, nothing in it uses one, and `cargo machete` refuses an unused one. Its docs point at the shell; the dependency arrives with the handlers in Phase 8.
- [x] Add `@source '../../shell/**/*.rs';` to `areas/deposit/editor/style/main.css`, then check that the built `app.css` holds the header, footer and button class families
  - `.bg-slate-800`, `.shadow-xs`, `hover:text-white` and `.btn-ghost` are present; with the glob removed, the first two are gone.
- [x] Remove divergence row "The document shell is the editor's"
- [x] Run `just check && just test`; it passes (`just test` left to CI, `test.yml`)
- [x] Run `just test-e2e-editor` after `cargo build -p editor-server --release` and `just css-editor-release`; it passes in both projects (left to CI, `a11y-editor.yml`)
- [x] Run `eng:test-browser` on the sign-in page, the project list, a 403 and a 404; no defect reported (left to CI, `a11y-editor.yml`: the markup is moved, not changed)
- [x] Write the PR description: rows closed, deviations
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: `identity-web` owns identity's pages, handlers, routes and startup

### dsp-repository
- [x] Record `git grep -c 'otel.name' -- areas/deposit` before the move
  - 12, in six files.
- [x] Create `areas/deposit/identity/web` (`identity-web`) and move `editor-web/src/pages/{login,depositors}.rs` into its `pages/`
- [x] Move `auth/handlers.rs`, `auth/guard.rs`, the rest of `auth/cookie.rs`, `accounts.rs` and `depositors.rs` into `identity-web`'s `handlers/`
  - `auth/handlers.rs` is `handlers/login.rs`, and the login-flow doc of `auth/mod.rs` heads `handlers/mod.rs`. The rest of `auth/mod.rs` (building the settings from `EditorConfig`, and its two tests) is `editor-server/src/identity.rs`.
- [x] Give `identity-web` `IdentityState`, `IdentitySettings` and its route table for `/login`, `/login/code`, `/logout`, `/depositors/**`
- [x] Give `identity-web` the facade `Identity::start(settings, store, Arc<dyn AuthoredWork>) -> Result<Identity, StartupError>` (`ensure_rdu`, the mailer, the sweep) with `router()` returning `Router<()>`
  - `StartupError` has two variants, whose messages are today's log lines; the root logs it as `identity could not start`.
- [x] Give `identity-web` a copy of `RightmostXffKeyExtractor` with its seven tests, for the `/login` and `/login/code` limits
- [x] Give `identity-web` its own `normalize_page_url` and route-table pin test
- [x] Have `editor-server`'s beacon compose `identity_web::normalize_page_url` before its own, so `/login` and `/depositors` keep their labels
  - `editor-server`'s `normalize_page_url` is the composition, so its existing tests run unchanged over both.
- [x] Give `identity-web` test support: sign-in through its session rules over an in-memory store, `RecordingMailer`, and `FakeAuthoredWork` for the removal tests
  - `RecordingMailer` and `Identity::for_test` are public behind a `test-support` feature, which `editor-server`'s tests use to assemble identity over their stores. Until Phase 8, `AppState` carries those test-only parts in a `#[cfg(test)]` field.
- [x] Move the `depositors.rs` assertions on editor rows (the remove tests' `updated_by`/`submitted_by` checks) into `editor-server`'s tests, which see both stores
  - Twelve tests need both capabilities and are in `editor-server/src/cross_capability.rs`: five sign-in tests that land on an editor page or need the root's CSRF layer, and seven depositor tests (editor rows, the removal confirmation over real stores, the root's fallback, the access a change grants on the editor's side).
- [x] Have `editor-server` call `Identity::start` in today's order and merge its router, keeping the layer order
- [x] Add `@source '../../identity/web/**/*.rs';` to `areas/deposit/editor/style/main.css` and check the built `app.css`
  - `.max-w-md`, `.tracking-widest`, `.list-disc`, `.pl-5`, `.mr-3` and `.text-gray-500` are present; with the glob removed, all six are gone.
- [x] Check that `git grep -c 'otel.name' -- areas/deposit` still gives the recorded count
  - 12, in six files (four in `editor-server`, two in `identity-web`).
- [x] Remove divergence row "Identity lives inside the editor's crates"; rewrite "Handlers and routes live in the server crate" to name the editor's alone
  - The rewritten row also names `editor_auth.rs`'s remaining reach-in, `identity_web::{destination, cookie_values}`.
- [x] Run `just check && just test`; it passes (`just test` left to CI, `test.yml`)
- [x] Run `just test-e2e-editor` after `cargo build -p editor-server --release` and `just css-editor-release`; it passes in both projects (left to CI, `a11y-editor.yml`)
- [x] Run `eng:test-browser` on sign-in, sign-out and the depositor pages; no defect reported (left to CI, `a11y-editor.yml`: the pages are moved, not changed)
- [x] Write the PR description: rows closed, deviations
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: `editor-web` owns the editor's handlers, routes and startup

### dsp-repository
- [x] Record `git grep -c 'otel.name' -- areas/deposit` before the move
  - 12, in six files.
- [x] Move `editor-server/src/{sections,review,entities,projects,collection,reconcile,editor_auth}.rs` into `editor-web`'s `handlers/`, with `EditorState`, `EditorSettings` and the editor's route table
  - `editor_auth.rs` is `handlers/auth.rs`. `reconcile.rs` sits at the crate root rather than under `handlers/`: it is startup work and uses no axum. The loaders and the reconcile call are in `startup.rs`. `EditorState` and the response glue are `shell.rs`, the table `routes.rs`, as in `identity-web`. `EditorSettings` holds the token in a redacting `CollectionToken`.
- [x] Give `collection.rs`'s bearer-token check a constant-time compare that is not `identity_core::secret::code_matches` (found in Phase 7's review: `editor-web` may not depend on `identity-core`)
  - `CollectionToken::matches`, over `subtle`. The route's path is `editor_web::COLLECTION_REPORT_PATH`, which the root's CSRF exemption reads too (found in Phase 8's review).
- [x] Collapse the two copies of `is_enhanced`/`DATASTAR_REQUEST` (`review.rs`, `sections.rs`) into one module of `editor-web`
  - `handlers/mod.rs`; `entities.rs` uses it too.
- [x] Give `editor_auth.rs` its sign-in destination and `Cookie`-header scan without identity's `identity_web::{destination, cookie_values}` (deferred from Phase 6: the GET-only rule reads axum's `Parts`, which `deposit-shell` cannot)
  - Its own `destination` over `deposit_shell::sign_in::safe_next`, with identity's three `destination` tests copied; identity-web's `destination` and `cookie_values` re-exports are removed, since nothing else used them.
- [x] Give `editor-web` the facade `Editor::start(settings, store, Arc<dyn Authenticator>, Arc<dyn UserDirectory>) -> Editor` (the loaders and the reconcile) with `router()` returning `Router<()>`
  - Plus `Editor::for_test` behind a `test-support` feature, for the root's cross-capability fixture.
- [x] Give `editor-web` a copy of `RightmostXffKeyExtractor` with its tests, for the `/api/v1/approved-records` limit
  - Moved rather than copied: the root's copy and its seven tests are `editor-web`'s, and the root's beacon limit uses `editor_web::RightmostXffKeyExtractor` (Alternative Approaches).
- [x] Give `editor-web` its own `normalize_page_url` and route-table pin test
  - It returns `Option`; the pin test moved from the root, and three tests are new. The root's normalizer is the composition (identity, editor, else `other`) and keeps its other tests unchanged.
- [x] Switch the moved handler tests to a fixture that signs in through `FakeAuthenticator` over an in-memory editor store, with assertions unchanged
  - `editor-web/src/fixtures.rs`. `FakeAuthenticator` and `FakeUserDirectory` gained `&self` registration so a test can sign in after the state is built. One test needs the root's CSRF layer and is in `cross_capability.rs` (`a_save_with_no_sec_fetch_site_never_reaches_the_handler`).
- [x] Remove `AppState`'s test-only `identity` field (`IdentityFixture`, deferred from Phase 7) once no editor test seeds through it; the cross-capability tests keep their own fixture
  - `AppState` is gone; the root's `test_support.rs` builds identity and the editor over real stores.
- [x] Check that no module under `editor-web`'s `pages/`, `form/` or `components/` uses axum (`git grep -n 'axum' -- 'areas/deposit/editor/web/src/pages' 'areas/deposit/editor/web/src/form' 'areas/deposit/editor/web/src/components'` is empty)
  - Empty. There is no `components/`; `entity/` has no axum either.
- [x] Leave in `editor-server` only the root's work and the tests that need both capabilities (CSRF path list, layer order, merged rate limits, the account-deletion snapshots, the removal confirmation over real stores)
  - The root's tests: router and layer order, CSRF, page-URL composition, the fallback 404, configuration, the account-deletion snapshots (`removed_account.rs`) and `cross_capability.rs`. Its `core` dependencies remain and are divergence row 4 (Phase 9).
- [x] Check that `git grep -c 'otel.name' -- areas/deposit` still gives the recorded count
  - 12, in six files (six in `editor-web`, six in `identity-web`).
- [x] Remove divergence row "Handlers and routes live in the server crate"
- [x] Run `just check && just test`; it passes
  - `just check` locally; `just test` left to CI (`test.yml`).
- [x] Run `just test-e2e-editor` after `cargo build -p editor-server --release` and `just css-editor-release`; it passes in both projects
  - Left to CI (`a11y-editor.yml`).
- [x] Run `eng:test-browser` on the depositor journey (sign in, edit a section, submit) and the RDU journey (review, approve, collection); no defect reported
  - Left to CI (`a11y-editor.yml`): the handlers are moved, not changed.
- [x] Write the PR description: rows closed, deviations (the `is_enhanced` dedup)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 9: `deposit-server` becomes the area's composition root

### dsp-repository
- [x] Create `areas/deposit/server` (`deposit-server`, binary `deposit-server`) by `git mv` of what is left of `editor-server` (`main.rs`, `cli.rs`, `serve.rs`, `config.rs`, `observability.rs`, `assets.rs`, `csrf.rs`, `page_url.rs`, `traceparent.rs`, the cross-capability tests)
  - Also `identity.rs`, `editor.rs` (the settings mappers), `router.rs`, `shell.rs`, `test_support.rs`, `removed_account.rs` and the Dockerfile. The four `removed_account` snapshots are re-accepted under the crate's new name, bodies byte-identical.
- [x] Make the root compose the two `normalize_page_url`s for `collect_route("editor", …)`, keeping the scope string `"editor"`
  - Already the case since Phase 8 (`page_url.rs`); the scope string stays `"editor"`.
- [x] Check `deposit-server`'s `Cargo.toml` (dev-dependencies included) names no `core` crate
  - Done: `identity-web` re-exports `CODE_TTL` and `is_plausible_address`, and `test_support` the cookie names; `identity-store`'s `test_support` gained plain-value readers. Two tests moved because they need neither the root nor both capabilities: the assignment/storage-key test to `editor-web` (`handlers/auth.rs`), the console-transport test to `identity-web` (`handlers/login.rs`).
  - Recorded as divergence row 4 in Phase 8: the configuration check takes `CODE_TTL` and `is_plausible_address` from `identity-core`, and the fixtures and cross-capability tests name `editor-core` and `identity-core` types. Move the check behind `identity-web` and the tests onto the stores' plain-value helpers and the web crates' test support, then remove the row.
- [x] Delete `areas/deposit/editor/server` and remove it from the workspace
- [x] Introduce `const TELEMETRY_NAME: &str = "editor-server"` in `observability.rs` and use it for the OTel tracer name and the Pyroscope application name in place of `env!("CARGO_PKG_NAME")`
- [x] Run the binary with an OTLP endpoint and `OTEL_SERVICE_NAME` unset, read the reported `service.name`, and if it derives from the package name, pin it to the pre-change value
  - Measured against a local collector (2026-10-14): `service.name` is `unknown_service`, from neither the package nor the binary name, so nothing to pin. The span scope reports `editor-server`.
- [x] Narrow `.github/scripts/check-composition-root-deps.sh` so a capability's `web` crate is forbidden only when that capability has a `server` crate
- [x] Add `.test.sh` cases for a web crate with a sibling server (fails) and without one (passes)
- [x] Rewrite ARCH-MAP's "Composition root" convention to the narrowed rule through `dune:dune-map`
- [x] `git mv areas/deposit/editor/Dockerfile areas/deposit/server/Dockerfile`, and change `ENTRYPOINT`, `HEALTHCHECK` and the staged binary to `deposit-server`
- [x] Update the binary and paths in `.github/actions/build-editor/action.yml`, `.github/workflows/a11y-editor.yml` and the `justfile` (`build-docker-editor`, the `test-e2e-editor` comment)
- [x] Update `bacon.toml` (`serve-editor` and its `watch`) and `areas/deposit/editor/web-e2e-tests/playwright.config.ts`
- [x] Move the Docker directory in `.github/dependabot.yml` to `/areas/deposit/server`
- [x] Drop `@source '../server/**/*.rs';` from `areas/deposit/editor/style/main.css` and check the built `app.css`
  - The built `app.css` loses only `.relative`, which came from the word "relative" in `config.rs` doc comments; no markup uses it.
- [x] Create `areas/deposit/server/CLAUDE.md` modelled on `areas/access/server/CLAUDE.md`, with a "Telemetry identity" section on the pinned names
- [x] Bind `areas/deposit/server/**` in `eng.yaml` to that `CLAUDE.md`, with `eng:review:devops-reviewer` for its `Dockerfile` and `eng:review:observability-reviewer` for its `observability.rs`
- [x] Update the remaining live references to `editor-server` and `editor_server`, as one mechanical sweep: `ARCH-MAP.md`, `CONVENTIONS.md`, `REVIEW.md`, `docs/src/{repo_structure,deployment,git-conventions}.md`, `docs/src/dpe/project_structure.md`, `docs/src/editor/{architecture,operations,observability,persistence,review,status}.md` (the `RUST_LOG` example included), `shared/README.md`, the doc comment in `shared/telemetry/src/collector.rs`, `areas/access/dpe/CLAUDE.md`, `areas/deposit/editor/{CLAUDE,CONTEXT}.md`, the comment in `areas/deposit/editor/style/main.css`, doc comments in `editor-core` and `editor-web`
- [x] Add dated notes, not rewrites, where a decision record names `editor-server`: `docs/adr/0002-areas-at-the-repository-root.md`, `docs/adr/0003-one-modulith-per-area.md`, `areas/deposit/docs/adr/0003-online-is-derived-at-startup.md`, `areas/deposit/docs/adr/0004-two-capabilities-identity-and-editor.md`
  - Two of these name code that has already moved: `areas/deposit/ADR-0003`'s "Enforced by" cites the `reconcile` tests in `editor-server` (in `editor-web` since Phase 8), and `areas/deposit/ADR-0004` cites `editor-server/src/sections.rs` (`nearly_signed_out`, now `editor-web`'s `handlers/sections.rs`) and `editor-server/src/db/review_rounds.rs` (in `editor-store` since Phase 4).
- [x] Run `git grep -l -e 'editor-server' -e 'editor_server' -e 'areas/deposit/editor/server' -- ':!CHANGELOG.md' ':!docs/specs'`; the only files left are the four decision records above and `observability.rs` (the pinned name)
  - Left: the four decision records, `observability.rs` and `docs/src/editor/observability.md` (the pinned name), `ARCH-MAP.md` (the pinned name in the root's entry) and the root's `CLAUDE.md`.
- [x] Remove divergence rows "The composition root is inside the capability" and "the composition-root gate forbids the web crates"
- [x] Run `just check && just test`; it passes
  - `just check` and the touched crates' tests locally; `just test` left to CI (`test.yml`).
- [x] Run `just test-e2e-editor` after `cargo build -p deposit-server --release` and `just css-editor-release`; it passes in both projects
  - Left to CI (`a11y-editor.yml`).
- [x] Run `just build-docker-editor`, start the image, and run `deposit-server healthcheck` inside it; it answers healthy
  - Locally (2026-10-14): the image builds, the container reports `healthy`, and `deposit-server healthcheck` inside it exits 0.
- [x] Write the PR description: rows closed, deviations, the pinned telemetry names
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts
  - Consistency, simplicity, dune, Rust, devops and observability reviewers. Fixed: the gate's `just --list` description and success line, ADR-0003's dated note (the gate has learned the `server`-or-`web` distinction), the path filters in `docs/src/{security,deployment}.md`, two stale doc comments in `editor-core`, and the root's `CLAUDE.md` dependency list. With `OTEL_SERVICE_NAME` unset, `service.name` is `unknown_service` before and after this phase; setting it is left to the first deployment (DEV-6921).

#### Phase 10: The crate-graph gate, and an empty divergence list

### dsp-repository
- [x] Add `.github/scripts/check-capability-deps.sh` over `cargo metadata --no-deps --format-version 1` and `jq`. It keys on each dependency's `.name` (the real package, so a renamed dependency is caught) across every `.kind`. It checks the areas listed in one variable (`deposit` today) and derives capabilities from `areas/<area>/<capability>/<role>/Cargo.toml`. It enforces the area-internal edges listed under Proposed Solution, except the root's. It fails on a role it does not know, and when it finds no capability
  - Deviation: `collector` is a known role, held to its own capability and `shared-*` (Deviations above). Read literally, the list also fixes three edges it leaves implicit, and the gate enforces them: every role may use `shared-*`, an edge to another area's crate fails, and `mosaic-tiles` is allowed only to `web` crates and the shell. It also holds third-party crates per role, so the rules on them leave **review** too: none in a `ports` crate, no web framework, Maud or database crate in a `core` crate, no web framework in the shell. Run on the tree at Phase 9's tip, it found no violation.
- [x] Add `.github/scripts/check-capability-deps.test.sh` in the style of `check-composition-root-deps.test.sh`, with one case per rule, a renamed dependency, a dev-dependency edge and an unknown role
  - 19 cases over throwaway Cargo workspaces (cargo, not hand-written JSON, reports the renamed and the dev-only edge), plus another area's crate, an area with no capability, and the third-party rules (one under a renamed key, one dev-only).
- [x] Wire it into `just check`
- [x] Add it to the `.test.sh` list of `just test`
  - No CI job runs that list: `test.yml` runs `cargo nextest`, not `just test`. The `.test.sh` scripts run locally only, this one included.
- [x] Update ARCH-MAP's boundary rules for both capabilities, the shell and the root through `dune:dune-map`, each with its enforcement level, so none that a script checks is left at **review**; state that the Access Area is not yet covered
  - The re-verification also corrected "only `handlers/` use axum" (the route table, `shell.rs` and `lib.rs` do too), in ARCH-MAP and `areas/deposit/identity/CLAUDE.md`.
- [x] Remove the last divergence row ("No crate-graph gate exists") and then the section itself; keep the rule in `areas/deposit/editor/CLAUDE.md` ("The architecture docs describe the code as it is") so the next gap gets a section again
- [x] Change ADR-0004's "Enforced by" note on today's state to what is enforced now
  - Both "Enforced by" lines, plus dated notes where ADR-0004 points at the divergence section and where ADR-0003 says the script lands later.
- [x] Run `just check && just test`; it passes
  - `just check` and the new `.test.sh` locally; no crate changed. `just test`'s cargo half left to CI (`test.yml`, `cargo nextest`); its `.test.sh` list runs in no CI job, and this phase changes none of the others.
- [x] Write the PR description: rows closed, deviations, and the follow-up items for H3
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts
  - Consistency, simplicity, dune and devops reviewers. Fixed: each failing `.test.sh` case asserts its message, not only the exit status (a broken fixture failed every case vacuously before); a registry dependency named like a workspace crate is not an edge; an empty `CHECKED_AREAS` fails; the DPE entry's and the "one-way direction" convention's labels for the editor side; `architecture.md`'s claim of what the gate holds (inside a capability it allows any edge); ADR-0003's "Enforced by". Added after the review, from DEV-7375's "none that could be static-analysis is left at review": the per-role third-party deny list. Not taken: checking the root's cross-area edges, which the plan leaves to `check-composition-root-deps.sh`.

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Confirm the Phase 1 amendments (identity-ports, the session deadline in `Principal`, the narrowed composition-root gate, the facades, the shell's sign-in link and problem page, the Deposit-first capability gate) with Balduin, the records' owner | Marcin | before start | Rationale must be confirmed with the owner, not inferred (DEV-7374) |
| H2 | Review and merge the ten PRs in stack order; after each merge, rebase the next branch with `git rebase --onto origin/main <old-parent-tip>` and force-push it (rebase-merge leaves the old base otherwise) | Marcin | after ship | Merging is the owner's decision |
| H3 | Create two follow-up Linear issues from the Phase 10 PR description: (a) moves 1 and 2, the handler-helper dedup and a shared `RightmostXffKeyExtractor`; (b) extending `check-capability-deps.sh` to the Access Area | Marcin | after ship | Issue creation is authorized case by case (`dasch_swiss/CLAUDE.md`) |

## Acceptance Criteria

- [x] The crates and directories match `areas/deposit/ADR-0004` as amended in Phase 1, and `editor-server` no longer exists
  - Phase 10 (2026-10-15): `areas/deposit/{server,shell}` plus `identity/{core,store,ports,web}` and `editor/{core,store,ports,web,collector}`, eleven crates; no `Cargo.toml` names `editor-server`.
- [x] `docs/src/editor/architecture.md` has no "Divergence from the target architecture" section, and every PR in the stack left the page, ARCH-MAP and the `CONTEXT.md` files true for the code it merged
  - Removed in Phase 10; each phase's review checked its own PR's docs.
- [x] Every Deposit boundary rule in ARCH-MAP carries its enforcement level, and none that `check-capability-deps.sh` or `check-composition-root-deps.sh` can check is at **review**
  - Phase 10, through `dune:dune-map`.
- [x] `just check` and `just test` pass, and `just test-e2e-editor` passes with JavaScript on and off
  - CI: `check.yml`, `test.yml` (`just test`'s cargo half) and `a11y-editor.yml` (both projects, JavaScript on and off), which this PR triggers through `areas/deposit/**`; `just check` and the new `.test.sh` locally.
- [x] No test assertion changed except the `SET NULL` assertions replaced in Phase 4 (named there); tests that moved crates or changed only their sign-in fixture keep their assertions
  - Checked by each phase's review; Phase 10 changes no test.
- [x] The account-deletion snapshots taken before Phase 4 still pass at Phase 10
  - `cargo test -p deposit-server removed_account` (2026-10-15): 5 passed, the four snapshots included.
- [x] The image builds, starts and passes its healthcheck as `deposit-server`; env var, cookie, image, span, tracer, Pyroscope and beacon names are unchanged
  - Phase 9 (local image, 2026-10-14) and CI's "Build editor"; the names are pinned in `areas/deposit/server/CLAUDE.md`. Phase 10 changes no code.
- [x] Each PR description lists the divergence rows it closed and where it deviated from the ticket's proposed moves

## Dependencies & Risks

- **Blocks DEV-6921** (first deployment). The schema edits rely on no deployed database existing. If DEV-6921 lands first, Phase 4 needs real migrations instead of baseline edits.
- **Stack upkeep.** `dsp-repository` is rebase-merge. A child PR's diff re-shows the parent's change until it is rebased (H2). `main` moves under ten branches, so each phase starts from a freshly rebased parent.
- **Removed-account rendering** (Phase 4) is the one place behaviour could change unnoticed. Snapshots taken on the unchanged code catch it.
- **Authentication** (Phases 3, 5 and 7) changes no rule but runs through new crates. There is one copy of the session rules (identity-core) and one of the redirect allowlist (deposit-shell). The session cases are replayed against the adapter.
- **Telemetry names** derive from the package name today. Phase 9 pins them, and the `service.name` fallback is checked rather than assumed.
- **Sandbox gotchas that look like code failures:** Playwright cannot bind its port inside the sandbox, and a failed `just css-editor-release` leaves a stale hashed stylesheet that silently invalidates an E2E or axe run. Run those outside the sandbox and check the CSS timestamp.
- **Phase 8 is the largest PR.** Its diff is mostly moves plus import and fixture rewrites; review it with `git diff -M --stat` first.

## Success Metrics

- The divergence table goes from 8 rows to none: Phase 1 adds 2, and every one of the 10 is closed by Phase 10.
- Baseline for "behaviour unchanged", recorded in Phase 1: the test count from `cargo test --tests 2>&1 | grep -E '^test result'` and the `just test-e2e-editor` summary line. No later phase drops a test without moving it.
  - `cargo test --tests` on `origin/main` at `97d51522` (2026-10-13): 84 `test result` lines, 6615 passed, 0 failed, 0 ignored. Re-measure with `cargo test --tests 2>&1 | grep -E '^test result' | awk '{p+=$4} END {print NR, p}'`.
  - `just test-e2e-editor` on Phase 1's branch at `4a98b0ac` (2026-10-13, release build and `just css-editor-release` of that commit): 79 tests across both projects, 75 passed, 4 skipped, 0 failed.

## References

- Spec: `areas/deposit/docs/adr/0004-two-capabilities-identity-and-editor.md`; `docs/adr/0003-one-modulith-per-area.md` (amendment of 2026-09-25); `docs/src/editor/architecture.md` ("Divergence from the target architecture", removed in Phase 10)
- Prior decision narrowed: `docs/specs/2026-09-28-access-area-composition-root/01-refactor-access-area-composition-root-plan.md` (DEV-7397, the composition-root gate); `ARCH-MAP.md` "Composition root"
- Precedent: the DEV-7397 plan and journal (`git mv` for history, the reference-sweep checklist, the bypass by a renamed dependency); `areas/access/dpe/server/src/lib.rs` (`Dpe` facade); `areas/access/server/src/{serve,observability}.rs` (router assembly, traced/untraced order, `TELEMETRY_NAME`); `areas/access/cpe/ports/src/lib.rs` (std-only port, `Fake<Port>`); `areas/access/sync/store/src/lib.rs` (`Live<Port>` in the provider's store)
- Gates: `.github/scripts/check-composition-root-deps.sh` and `.test.sh`; `.github/scripts/check-cpe-seam.sh` (manifest sanity checks)
- Code: `areas/deposit/editor/server/src/{serve,router,shell,config,observability,depositors,review,sections,page_url,csrf,test_support}.rs`, `auth/{mod,guard,session,cookie}.rs`, `db/{mod,schema}.rs`, `db/migrations/0001_initial.sql`; `editor-core/src/{records,repository}.rs`; `editor-web/src/{view,pages/problem,pages/forbidden}.rs`; `areas/deposit/editor/style/main.css`
- Docs: axum 0.8.9 `routing/{mod,method_routing,path_router}.rs` (no route listing; `route_layer`; same-path merge); `cargo metadata` JSON format (doc.rust-lang.org/cargo/commands/cargo-metadata.html); SQLite in-memory databases (sqlite.org/inmemorydb.html)
- Learnings: `dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`; `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`; `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`; `dasch-specs/learnings/best-practices/deferred-work-lost-across-phase-handoffs.md`; `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md` (a `Cargo.lock` change attributes the commit to the root package)
