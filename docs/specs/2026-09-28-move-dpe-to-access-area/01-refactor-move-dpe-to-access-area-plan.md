---
title: "refactor: Move DPE from modules/dpe to areas/access/dpe"
type: refactor
date: 2026-09-28
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7396
linear_project: CPE establish production path
---

# refactor: Move DPE from modules/dpe to areas/access/dpe

## Overview

This is the Access Area half of ADR-0002: `modules/dpe/**` becomes `areas/access/dpe/**`, with crate names unchanged
(`dpe-core`, `dpe-api-oai`, `dpe-web`, `dpe-server`). DPE's `CONTEXT.md` moves up to `areas/access/CONTEXT.md`. Everything
that names the old path follows in the same commit. There is no behaviour change.

The editor's move (54480c61, DEV-7373) is the template. This plan copies its shape and adds the DPE-specific surfaces.

## Problem Statement / Motivation

ADR-0002 (accepted) removes `modules/`. The editor has moved and DPE is the remaining application. The move is also a
prerequisite of ADR-0007 (proposed): the Access Area composition root `areas/access/server` (DEV-7397) composes area crates,
not `modules/` crates. DEV-7397 is blocked on this ticket.

## Proposed Solution

One `git mv modules/dpe areas/access/dpe`, then repoint every reference in the same commit. Three kinds of reference change:

1. **Relative paths out of DPE gain one `../`.** These are the Cargo path dependencies, the `include_str!` literals, the
   Tailwind entry's `@import`/`@source`, the Playwright root and the fuzz crate's path dependencies. `../../mosaic/...`
   becomes `../../../../modules/mosaic/...` (up to the repo root, then into `modules/`), because Mosaic has not moved yet.
2. **Repo-root-relative strings `modules/dpe` become `areas/access/dpe`.** These are the workspace members, runtime defaults
   (`DPE_DATA_DIR`, `DPE_PUBLIC_DIR`), the justfile, CI, ignore files, dependabot, eng.yaml, bacon, docs and comments.
3. **The editor's reach into DPE's data** changes one line per toolchain, which the editor move set up
   (`editor_core::DPE_DATA_DIR`, `DPE_DATA_DIR` in the justfile, the editor Playwright constant, `build-editor`).

It lands as one commit, because the tree does not build between the move and the repointing:
`refactor(dpe-core,dpe-api-oai,dpe-web,dpe-server): move DPE to areas/access/dpe`.

## Technical Considerations

- **CONTEXT.md placement (decided 2026-09-28):** `modules/dpe/CONTEXT.md` → `areas/access/CONTEXT.md`, as the ticket, ADR-0002's
  Consequences and ADR-0007 say. This departs from the editor, which kept `areas/deposit/editor/CONTEXT.md`. The file already
  calls itself the Access Area, and CPE will add terms to it, so putting it at the area level now saves a second move. The
  ADR-0002 amendment records the asymmetry. `CLAUDE.md`, `README.md` and `Dockerfile` stay with the capability at
  `areas/access/dpe/`, as the editor's did. The file's only relative link (`../../CONTEXT.md`) keeps its depth, so it still
  resolves.
- **Silent failures.** Most references are plain strings that no build graph checks:
  - **CI path filters:** a stale `paths:` entry just stops triggering the job.
  - **Tailwind `@source`:** a stale glob yields a smaller `app.css` with no error (learning
    `dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`).
  - **Runtime defaults:** `dpe-core::utils` and `dpe-server::config` default to `modules/dpe/server/data` and
    `modules/dpe/public`. A stale default fails only at startup, or not at all when the env var is set.

  The CSS byte-identity check and the grep gate in Implementation Approach cover these.
- **Gates.** After the move, `modules/` holds only Mosaic, so the `modules/*` globs in `check-shared-paths.sh`,
  `check-datastar-delimiters.sh`, `verify-checksums.sh` and `eng.yaml` still match something and do not trip their
  absence checks. The `areas/*/*/…` globs added by the editor move pick DPE up with no script change. Only the test fixtures
  that name `modules/dpe` change.
- **`CARGO_MANIFEST_DIR`-relative paths inside DPE need no change.** Every such path (`core/src/project.rs`,
  `server/src/router.rs`, `api-oai/src/metadata/corpus.rs:20` and others) resolves within DPE's own tree (`/../server/data`,
  `/data`, `/../public`), so the paths move with it.
- **The fuzz crate** (`server/fuzz`) is its own workspace (`[workspace]` in its `Cargo.toml`), so `cargo check` of the root
  workspace does not see a broken path there. It needs its own resolution check.
- **Insta snapshots** carry a `source: modules/dpe/...` header. Insta compares content only, so a stale header would not fail,
  but it would be a stale path. Rewrite the three headers.
- **release-please:** `.github/release-please/config.json` and `manifest.json` carry no `modules/dpe` path (checked 2026-09-28).
  Nothing changes there.
- **Mosaic stays at `modules/mosaic`.** Its move to the root is separate ADR-0002 work. `docs/src/repo_structure.md` already
  lists Mosaic at `mosaic/tiles`. That is a pre-existing inconsistency, out of scope here and left as is.
- **The ticket's `CARGO_MANIFEST_DIR/../../dpe/server/data` item is stale.** 54480c61 already folded those spellings into
  `editor_core::DPE_DATA_DIR` / `checkout_dpe_data_dir()` (`areas/deposit/editor/core/src/repo_paths.rs`), so only the constant
  changes.
- **The done-criterion is scoped (decided 2026-09-28):** no `modules/dpe` path may remain outside dated `docs/specs/20*`
  folders, `CHANGELOG.md` (generated history) and the historical text of ADRs. The new ADR-0002 amendment names the old path
  on purpose. `docs/specs/README.md` is a live convention file and gets updated.

## Implementation Approach

### Baseline before the move

- [x] Run `just css` on the unmodified tree and keep the SHA-256 of `modules/dpe/public/assets/app.css` in `.claude/tmp/`
      (the Success Metrics baseline).

### Move

- [x] `git mv modules/dpe areas/access/dpe`
- [x] `git mv areas/access/dpe/CONTEXT.md areas/access/CONTEXT.md`
- [x] Remove the now-empty `modules/dpe` directory, including any untracked build output such as `node_modules` or
      `public/assets/app*.css`, so a later glob cannot see a ghost `modules/dpe`

### Relative paths out of DPE (one extra `../`)

- [x] `areas/access/dpe/core/Cargo.toml`: `shared-metadata` path
- [x] `areas/access/dpe/api-oai/Cargo.toml`: `shared-fair`, `shared-metadata` paths
- [x] `areas/access/dpe/web/Cargo.toml`: `shared-metadata` path; `mosaic-tiles` → `../../../../modules/mosaic/tiles`
- [x] `areas/access/dpe/server/Cargo.toml`: `shared-fair`, `shared-metadata`, `shared-telemetry` paths; `mosaic-tiles` →
      `../../../../modules/mosaic/tiles`
- [x] `areas/access/dpe/server/fuzz/Cargo.toml`: `shared/metadata` and `shared/telemetry` paths (`../../core` is internal and
      stays)
- [x] `include_str!` in `api-oai/src/handlers/get_record.rs`, `api-oai/src/handlers/test_utils.rs` and
      `api-oai/src/metadata/corpus.rs`
- [x] `areas/access/dpe/style/main.css`: the two `@import` lines and the `@source` glob → `../../../../modules/mosaic/...`
- [x] `areas/access/dpe/web-e2e-tests/playwright.config.ts`: both `"../../.."` repo-root spellings → `"../../../.."`
- [x] Look for any relative path out of DPE not listed above, with `git grep -nE '\.\./\.\./' -- areas/access/dpe`, and fix
      each one it turns up

### Repo-root-relative `modules/dpe` → `areas/access/dpe`

Inside DPE:

- [x] `areas/access/dpe/core/src/utils.rs`: the `DPE_DATA_DIR` and `DPE_PUBLIC_DIR` fallbacks
- [x] `areas/access/dpe/server/src/config.rs`: doc comments, `Default` values and the two test assertions
- [x] The three `source:` headers in `areas/access/dpe/server/src/snapshots/*.snap`
- [x] Comments in `server/src/validate.rs`, `api-oai/src/metadata/corpus.rs`,
      `api-oai/src/handlers/testdata/schemas/download-schemas.sh` and `web-e2e-tests/playwright.config.ts`
- [x] `areas/access/dpe/CLAUDE.md`: its path mentions, plus a pointer to the area vocabulary at `../CONTEXT.md`
- [x] `areas/access/dpe/README.md`: path mentions
- [x] `areas/access/CONTEXT.md`: path mentions (the data corpus location)

The editor (one line per toolchain, plus comments):

- [x] `areas/deposit/editor/core/src/repo_paths.rs`: `DPE_DATA_DIR` and its module doc
- [x] `areas/deposit/editor/web-e2e-tests/playwright.config.ts`: `DPE_DATA_DIR` constant and the projects comment
- [x] `areas/deposit/editor/server/src/config.rs`: comment
- [x] `areas/deposit/editor/web/src/pages/section.rs`: two comments pointing at DPE's `CLAUDE.md`
- [x] `areas/deposit/editor/CLAUDE.md`: pointer to DPE's `CLAUDE.md`
- [x] `areas/deposit/editor/public/vendor/README.md`: DPE vendor path
- [x] `.github/actions/build-editor/action.yml`: the data copy line

Build and tooling:

- [x] Root `Cargo.toml`: the four workspace members and the Dockerfile comment
- [x] `justfile`: `DPE_DATA_DIR`, `install-e2e-requirements`, `validate-data`, `refresh-datacite-schema`, `fetch-records`,
      `css`, `css-release`, `dev`, `dev-otel`, `build-docker-dpe`, `test-a11y-dpe` and `lint-e2e` (every line from
      `git grep -n modules/dpe justfile`)
- [x] `justfile`: the `verify-checksums` comment (line 58, `modules/*/public/vendor`), which gains
      `areas/*/*/public/vendor` to match the script
- [x] `bacon.toml`: the `serve` watch list and its comment
- [x] `eng.yaml`: the DPE override → `when: "areas/access/dpe/**"`, `conventions: [areas/access/dpe/CLAUDE.md]`
- [x] `.gitignore`: the CSS output, record dumps and comment lines
- [x] `.dockerignore`: the e2e exclusion and the Dockerfile re-include
- [x] `.gitattributes`: comment
- [x] `scripts/build-temporal-coverage-enrichment.py`: `DEFAULT_DATA_DIR` and two comments

CI:

- [x] `.github/actions/build-dpe/action.yml`: the comment and the three `cp` lines
- [x] `.github/workflows/a11y-dpe.yml`: both `paths:` filters, the two `cd` lines, the comment and the report `path:`
- [x] `.github/workflows/check.yml`: the Biome `cd`
- [x] `.github/workflows/cloud-run-dpe-pull-request.yml`: the `paths:` filter
- [x] `.github/workflows/fuzz.yml`: the `working-directory` and the three corpus/artifact paths
- [x] `.github/workflows/scout-dpe.yml`: the `paths:` filter
- [x] `.github/dependabot.yml`: the cargo/docker directory and the npm e2e directory
- [x] `.github/scripts/check-shared-paths.test.sh`: move the `make_repo` baseline (`modules/dpe/server/data`, lines 41–47)
      and its comment to `areas/access/dpe`, so the fixture keeps the real repo's shape
- [x] `.github/scripts/check-shared-paths.test.sh`: move the two violation fixtures (lines 82–90) to `areas/access/dpe`, and
      give the case that exercises the `modules/` root (line 84) a neutral module name, so both roots stay covered.
      Case 5 already uses `modules/mosaic` and stays as it is
- [x] `.github/scripts/check-datastar-delimiters.test.sh`: the loop's `modules/dpe/web` → `areas/access/dpe/web`

Other crates and skills:

- [x] `modules/mosaic/tiles/src/components/theme_provider/tokens.css`: comment
- [x] `.claude/skills/add-mosaic-component/SKILL.md`: DPE's Tailwind entry path
- [x] `shared/README.md`: path mentions

Docs:

- [x] Root `README.md`, `CONTEXT.md` (the Access Area entry and the Deposit→Access relationship), `CONVENTIONS.md` and
      `REVIEW.md`
- [x] `docs/src/repo_structure.md`: the DPE crate table rows, the CI path filter and directory-scoped-instructions bullets,
      the content rule and the API list
- [x] `docs/src/dpe/project_structure.md`: the tree root
- [x] `docs/src/dpe/fair-principles.md`: every source path
- [x] `docs/src/dpe/architecture.md`, `metadata-model.md`, `oai-pmh.md`, `operations.md` (the env var default table) and
      `testing-strategy.md`
- [x] `docs/src/deployment.md`, `docs/src/security.md`, `docs/src/git-conventions.md` (the `dpe-data` scope) and
      `docs/src/editor/operations.md`
- [x] `docs/specs/README.md`: the LFS-scope note
- [x] `ARCH-MAP.md`: run `dune:dune-map` to regenerate the DPE section (`### areas/access/dpe`, paths glob, local-context
      kit) and every cross-reference in other components' entries; confirm the `CONTEXT.md` index line names
      `areas/access/CONTEXT.md`
- [x] `docs/adr/0002-areas-at-the-repository-root.md`: in the Decision text (line 24), change "today `modules/dpe/CONTEXT.md`"
      to `areas/access/CONTEXT.md`
- [x] `docs/adr/0002-areas-at-the-repository-root.md`: add an `## Amendment (2026-09-28): the Access Area half is done`
      section, covering DPE at `areas/access/dpe/` (DEV-7396); `CONTEXT.md` at the area level and why this differs from the
      editor; the editor's per-toolchain constants each changing one line; and Mosaic's move as the one still open
- [x] `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`: reword the Consequences bullet on `CONTEXT.md`
      (lines 123–124), because the move to `areas/access/CONTEXT.md` has happened; what remains is one file per capability
      once CPE's terms arrive

### Verification

- [x] `git grep -n -I 'modules/dpe' -- ':!docs/specs/20*' ':!CHANGELOG.md' ':!docs/adr'` returns nothing
- [x] `git grep -n -I 'modules/dpe' -- docs/adr` returns only historical text and the new amendment
- [x] `just css`; the SHA-256 of `areas/access/dpe/public/assets/app.css` matches the baseline
- [x] `cargo metadata --format-version 1 --manifest-path areas/access/dpe/server/fuzz/Cargo.toml` resolves without error
- [x] `just validate-data` passes with the new default data directory (no `DPE_DATA_DIR` set)
- [x] `just lint-e2e` passes
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Start DPE with `just run` from the repo root, with no env overrides, and run `eng:test-browser` on the home page, a
      project page and `/dpe/oai?verb=Identify`: styles load, images resolve, no console errors
- [x] Run `eng:reviewing` on the diff with `eng:review:consistency-reviewer`, `eng:review:devops-reviewer`,
      `eng:review:dune-reviewer`, `eng:review:accessibility-reviewer` and `eng:review:ivan-reviewer` (the last two are the
      eng.yaml bindings for `areas/*/*/web/**`); fix verified findings in the commit
- [x] Commit as `refactor(dpe-core,dpe-api-oai,dpe-web,dpe-server): move DPE to areas/access/dpe`, with a body covering the
      same points as 54480c61 (what moved, what followed, the `../` rule, the CONTEXT.md decision); run `just commit-lint`,
      which passes
- [x] After push: `gh pr checks` shows `a11y-dpe`, `scout-dpe` and `cloud-run-dpe-pull-request` triggered and green, and
      the preview comment (`<!-- dpe-preview -->`) links a working deployment
- [x] After push: `gh workflow run fuzz.yml --ref <branch>` (the fuzz job runs nightly and has no path filter), and the run
      builds all targets from `areas/access/dpe/server` (the workflow's matrix holds two of the crate's five targets;
      both built and ran)

## Acceptance Criteria

- [x] All four DPE crates live under `areas/access/dpe/`, with names and behaviour unchanged
- [x] `areas/access/CONTEXT.md` exists and `modules/dpe/` does not
- [x] `just check` and `just test` pass
- [x] DPE's path-filtered workflows (preview, Scout, a11y) trigger on `areas/access/dpe/**`, and the PR preview deploys
- [x] A manually dispatched fuzz run succeeds from the new working directory
- [x] No `modules/dpe` outside dated specs, `CHANGELOG.md` and ADR history
- [x] The DPE stylesheet is byte-identical to the one built before the move
- [x] ADR-0002 carries the dated amendment; `ARCH-MAP.md` is regenerated

## Dependencies & Risks

- **Blocked by DEV-7393 in Linear** (ADR-0007). The move itself follows from ADR-0002, which is accepted. It does not depend
  on ADR-0007 being accepted; only the reworded bullet in ADR-0007 touches it.
- **Blocks DEV-7397** (extract `areas/access/server`).
- **Merge conflicts:** the move touches about 60 files outside DPE, and any open PR under `modules/dpe/**` will need a rebase.
  Git's rename detection carries most content changes across. Check open PRs just before merging.
- **Deploy config outside this repo:** DPE's production image is built from the staging directory `build-dpe` assembles, and
  the image layout (`/app/public`, `/app/server/data`) does not change. No change is expected in `ops-deploy`.
- **`just build-docker-dpe`** runs `docker build -f <Dockerfile> .` from the repo root, while the Dockerfile `COPY`s from a
  staging layout (`dpe-server`, `public`, `data`). It may already be broken on main (not verified). The plan changes only its
  path and does not try to fix it.

## Success Metrics

- Baseline: the SHA-256 of `app.css` built by `just css` before the move. After the move it is identical.
- The grep gate above returns zero hits.
- The PR preview from the new path serves the same pages as the current staging DPE.

## References

- Precedent: 54480c61 `refactor(editor-core,editor-web,editor-server,editor-collector): move the editor to areas/deposit/editor`
- `docs/adr/0002-areas-at-the-repository-root.md` (line 24: the composition root and CONTEXT.md; line 52: the Consequences
  list; 2026-09-24 amendment)
- `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md:110-124`
- `areas/deposit/editor/core/src/repo_paths.rs`: the one editor constant
- `.github/scripts/check-shared-paths.sh:30`: the `for root in modules areas` loop
- Learning: `dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`
- Learning: `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md` (checked: no release-please
  path to update)
- Linear: DEV-7396 (this), DEV-7393 (blocker), DEV-7397 (blocked)
