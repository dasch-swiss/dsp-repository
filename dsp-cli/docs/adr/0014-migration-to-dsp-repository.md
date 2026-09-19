# Migration to dsp-repository

`dsp-cli` currently lives in `dasch-swiss/dsp-incubator` as a prototype (it began as
a private `balduinLandolt/dsp-cli` repo — see ADR-0005). Its intended permanent home
is the `dasch-swiss/dsp-repository` Cargo workspace (ADR-0005, ADR-0008). This ADR
records the **decision to migrate there and how** — sequencing, target shape, git
history, the (open) versioning question, and convention harmonization. It does **not** execute the
migration: the mechanical runbook is written when the migration is actually scheduled
(PROJECT_PLAN Phase 11). ADR-0005 and ADR-0008 predicted this move and shaped the code
to make it cheap; this ADR is the concrete record of the *how*.

## Context

- The crate **name + owner** are settled (PROJECT_PLAN, 2026-06-18): the published
  crate is `dsp-cli`, owned via the `dasch-swiss` GitHub team added as a crate owner
  (crates.io has no "organization account" — crates are owned by users and/or GitHub
  teams). `dsp-cli` is available on crates.io; the name was chosen to survive this move.
- ADR-0008 already designed the internal layout so that splitting into workspace
  crates is "a directory-rename operation": the natural split is `dsp-client`
  (model + client trait + HTTP impl), `dsp-render` (renderer trait + impls), and
  `dsp-cli` (clap + actions + main).
- `dsp-repository` is a mature Cargo workspace (members under `modules/`, `resolver`
  `"3"`, shared `[workspace.package]` and `[workspace.dependencies]`) with conventions
  that differ substantially from `dsp-cli`'s current personal-phase setup: a Nix flake
  toolchain, `cargo-deny`, an mdBook docs tree (`docs/src/`) plus top-level
  `CONVENTIONS.md` / `REVIEW.md`, and a `.claude/` agent harness whose plans live in
  `.claude/tmp/` — not `dsp-cli`'s `/dev:*` + `docs/design/plans/` workflow.

## Decision

### Sequencing — publish-first

Publish **0.1.0 from the current setup** (in `dsp-incubator`), *then* migrate.
Rationale: reserving `dsp-cli` for the `dasch-swiss` team closes the name-squatting
window immediately; the migration is a large, multi-part effort that should not gate
the first release; and the rework cost after migrating (bump `repository` metadata,
re-point CI) is cheap and routine pre-1.0. Harmonization happens *inside* the
migration, not as a precondition of the first publish.

### Target shape — workspace member(s) under `modules/`

Land `dsp-cli` as one or more workspace members per ADR-0008's split. Whether to land
as a single crate first and split later, or do the three-crate split (`dsp-client` /
`dsp-render` / `dsp-cli`) in the migration itself, is a **runbook-time call** — the
current layout supports either.

### Git history — incubator-as-archive

**No history import.** `dsp-incubator` remains the full-history archive: real git,
present in every clone, with the complete commit record. The migration lands in
`dsp-repository` as a fresh commit that links back to the incubator.

A branch-and-squash-via-PR import was considered (extract `dsp-cli`'s history with
`git filter-repo`, push it as a branch, squash-merge it — the merged PR's
`refs/pull/<n>/head` would then park the original commits on GitHub even after the
branch is deleted). Rejected: that yields only a **GitHub-side, forge-dependent** copy
(absent from normal clones, reachable only via the PR UI / pull refs), and the
incubator already provides a *better* archive (in-repo, clone-present). Not worth the
`filter-repo` extraction.

### Versioning — open question (leaning independent)

`dsp-repository` pins a single shared `[workspace.package]` version (0.7.1 at time of
writing) that members inherit via `version.workspace = true`. Two options for `dsp-cli`
once it joins the workspace — **not decided here**:

- **Independent version line (preferred lean).** `dsp-cli` keeps its own `version` key,
  decoupled from the workspace version, continuing its `0.x` line uninterrupted. Pro:
  the published version tracks `dsp-cli`'s own changes, not unrelated `dsp-repository`
  releases. Con: `dsp-repository`'s release / versioning tooling may assume every member
  inherits the workspace version — the impact of a member opting out is **not yet
  understood** and must be checked before committing.
- **Inherit the workspace version.** `dsp-cli`'s published version would jump to the
  workspace version on the first post-migration release (e.g. `0.3.0 → 0.7.x`). This is
  *allowed* on crates.io (published versions need only increase monotonically), but
  couples `dsp-cli`'s public version to `dsp-repository`'s cadence: every workspace
  release bumps `dsp-cli` even with no `dsp-cli` changes, and the number carries no
  `dsp-cli` semantics.

**Lean: independent.** Deferred until the consequences for `dsp-repository`'s release
tooling are understood — resolve in the migration runbook (Phase 11), or sooner if it
turns out to affect anything before then. The 0.1.0 release from `dsp-incubator` is
unaffected either way (it predates the workspace entirely).

### Convention harmonization — done as part of the migration

Performed in the migration move (not before the 0.1.0 publish):

- **Build / toolchain:** adopt the Nix flake (`flake.nix` / `flake.lock`),
  `cargo-deny` (`deny.toml`), the shared `.rustfmt.toml` + strict clippy; reconcile
  `just` recipes with `dsp-repository`'s (`just fmt` / `just check`). Reconcile the
  Rust **edition** (`dsp-cli` is `2024`; the `dsp-repository` workspace is `2021`).
- **Workspace:** add `dsp-cli`'s member(s) to `[workspace.members]`; route shared deps
  through `[workspace.dependencies]`.
- **Docs:** re-home `docs/adr/`, `docs/dev/`, `PROJECT_PLAN.md`, and `BACKLOG.md` onto
  `dsp-repository`'s `docs/src/` mdBook plus top-level `CONVENTIONS.md` / `REVIEW.md`.
  The **embedded `dsp docs` topics (`docs/topics/`) stay** — they are a binary feature
  (compiled in via `include_str!`, ADR-0010), not contributor docs.
- **Agent harness:** adopt `dsp-repository`'s `.claude/` harness (plans in
  `.claude/tmp/`, `CONVENTIONS.md` / `REVIEW.md`) and retire `dsp-cli`'s `/dev:*` +
  `docs/design/plans/` workflow and bespoke skills.
- **Conventions:** Conventional Commits with **scope = crate name**; the
  `dsp-repository` PR template; test naming `test_{what}_{condition}_{expected}`. The
  observability conventions (tracing/OTel) apply to services, not a short-lived CLI, so
  they remain N/A unless `dsp-cli` ever grows long-running surfaces.

### What does not change

The design decisions in ADRs 0001–0013 (vocabulary divergence, command shape, output
contract, diagnostics, instance reads, …) are about the *tool*, not its hosting. They
survive the move unchanged; only their physical location in the docs tree changes.

## Consequences

- 0.1.0 publishes from `dsp-incubator`; the crate's `repository` metadata initially
  points at `dsp-incubator` and is updated to `dsp-repository` in the migration release
  — a pre-1.0 metadata bump, no breakage.
- The migration is a distinct, post-0.1.0 milestone (PROJECT_PLAN **Phase 11**), with
  its own mechanical runbook authored when it is scheduled.
- After migration, the `dsp-incubator` prototype is archived (`/archive-prototype
  dsp-cli`); it remains the historical archive.
- Whether `dsp-cli` participates in `dsp-repository`'s synchronized workspace
  versioning is an **open question** (lean: independent version line) — see
  "Versioning" above.
- ADR-0005's and ADR-0008's migration notes are elaborated (not contradicted) by this
  ADR; they predicted the move, this records its shape.

## Considered alternatives

- **Migrate-first (publish 0.1.0 only from `dsp-repository`).** Rejected — gates the
  first release on a large migration and delays reserving the crate name (squatting
  window stays open longer).
- **History import via `filter-repo` + branch + squash-PR.** Rejected — see "Git
  history": the incubator is already a superior, clone-present archive.
- **Single crate vs three-crate split at migration time.** Not decided here — deferred
  to the runbook; ADR-0008's layout supports either.
