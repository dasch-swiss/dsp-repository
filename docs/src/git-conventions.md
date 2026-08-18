# Git Conventions

The single source for how git is handled in this repository: the branch and
merge workflow, commit messages, commit organization, and pull requests. If
you read one file about git here, read this one — nothing else duplicates it.

These conventions are shared with [SIPI](https://github.com/dasch-swiss/sipi);
the rules are the same across DaSCH repositories, only the scope vocabulary
differs.

## Git Workflow

We use a **rebase workflow**. All changes are made on a branch, then rebased onto `main` before being merged. This keeps a clean, linear commit history.

The goal is a **meaningful history on `main`**: every commit on main should be a deliberate, self-contained unit of change. Working commits ("WIP", "fix typo", "address review feedback") do not belong on main.

- **Rebase-merge**: PRs are integrated using rebase-merge (not squash or merge commits). Every commit on the branch lands on main verbatim — so the branch history _is_ the main history. There is no squash-on-merge safety net; whatever you leave on the branch is what ships.
- **Clean up before merging (mandatory)**: before a PR is merged, rewrite the branch (interactive rebase) so its commits read well on main. Squash working commits, reword messages, reorder as needed.
- **One commit per PR**: a PR lands as **one** commit by default, and CI enforces it. Split into multiple commits only when the work genuinely represents several independent, self-contained changes that each deserve their own line in the history — and each must stand on its own. See [Commit Organization](#commit-organization) and [Enforcement](#enforcement).

## Commit Message Schema

Follow [Conventional Commits](https://www.conventionalcommits.org/). These prefixes drive [release-please](./deployment.md) to determine the SemVer bump and generate the changelog — **using the correct prefix is required, not optional**.

    type(scope): subject
    body

**Every commit has a type and a scope** — both are mandatory (see [Scopes](#scopes)). This is the whole table; there are exactly eight types:

| Type | Meaning | Changelog section | Version bump |
|------|---------|-------------------|--------------|
| `feat` | New user-visible functionality | Features | minor |
| `fix` | Bug fix (see [What `fix:` means](#what-fix-means)) | Bug Fixes | patch |
| `perf` | Performance improvement | Performance Improvements | patch |
| `refactor` | Code restructuring, no behavior change | Code Refactoring | patch |
| `docs` | Documentation | Documentation | patch |
| `test` | Adding or refactoring tests | Tests | patch |
| `build` | Build system or dependencies | Build System | patch |
| `chore` | Miscellaneous maintenance | Miscellaneous Chores | patch |

Every type is visible in the release notes, in its own section, rendered in the order above. Because the scope is mandatory, every changelog line reads `concern: subject`. This table is the single human source for the type vocabulary; the machine source is [`.github/release-please/config.json`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/release-please/config.json).

Breaking changes take a `!` suffix (or a `BREAKING CHANGE:` footer):

    feat(dpe-api-oai)!: remove the deprecated verb alias

> [!NOTE]
> The "Version bump" column above describes the steady state from 1.0.0 onward.
> While the version is below 1.0.0 (it is currently 0.x), the whole scheme shifts
> down one place, so that the version still says whether a release broke anything:
>
> | Commit | Pre-1.0 bump | From 1.0.0 |
> |--------|--------------|------------|
> | `feat!:` / `BREAKING CHANGE:` | minor | major |
> | `feat:` | patch | minor |
> | everything else | patch | patch |
>
> This is `bump-minor-pre-major` + `bump-patch-for-minor-pre-major` in the
> release-please config, and it matches how Cargo reads 0.x versions: the minor
> is the breaking position. Without the second flag `feat:` and `feat!:` would
> both bump the minor, and the version would carry no signal about breakage.

The release-please config sentence-cases subjects in the changelog, so write the subject in normal prose case (`feat(dpe-web): add project filter`, not `feat(dpe-web): Add ...`).

### Removed types

`revert`, `style`, and `ci` are **not** valid types — the commit-lint gate rejects them, and release-please hides them if one ever slips onto `main`.

- **CI/build automation** → `chore(ci): ...` (lands in Miscellaneous Chores). A dedicated Continuous Integration changelog section is noise; it belongs under maintenance. Note that `ci` remains a valid *scope*, just not a type.
- **Formatting-only changes** → fold into the functional commit, or `chore(<scope>): ...`. `cargo fmt` and `maudfmt` are enforced gates, so a standalone formatting commit should be rare.
- **Reverts** → `fix(<scope>): revert ...` or `chore(<scope>): revert ...` depending on whether it corrects released behavior. (release-please's built-in revert auto-negation no longer applies; reverts are rare and handled by hand.)

### What `fix:` means

A `fix:` corrects behavior that exists on `main` — a bug a deployer could hit today, or that a released version shipped. It earns a "Bug Fixes" changelog line precisely because deployers need to know. The same change filed as `chore:` or `refactor:` still appears in the release notes, but under a section nobody scanning for a regression will read.

A bug you introduced earlier in the **same branch** is not a `fix:`. Fold it into the commit that introduced it (`git commit --fixup=<sha>`, then `git rebase --autosquash`) so it never lands on `main` and never generates a changelog entry for a bug nobody ever saw. With the one-commit default this is the normal case: you are amending, not accumulating.

Corollary: if you discover a genuine pre-existing `main` bug while doing unrelated work, give it its **own** `fix:` commit — don't bury it inside the `feat:`/`refactor:` you happened to be writing.

## Scopes

**A scope is mandatory.** Every commit is `type(scope): subject`, so every release-notes line reads `concern: subject`. There is no scopeless form.

The scope names the **concern the change serves** — the crate or area whose responsibility it belongs to, not the directory the edited files happen to sit in. Use one of these names, lowercase:

- **Crate scopes:** `dpe-core`, `dpe-server`, `dpe-web`, `dpe-api-oai`, `platform-telemetry`, `editor-core`, `editor-web`, `editor-server`, `mosaic-tiles`, `mosaic-playground`
- **Cross-cutting scopes** (changes not tied to one crate):
  - `dpe-data` — project metadata files under `modules/dpe/server/data/`
  - `ci` — workflows, the justfile, CI scripts
  - `deps` — dependency bumps, Dependabot, base-image updates
  - `docs` — repo-level and process documentation with no single crate owner

Rules:

- Lowercase, kebab-case.
- A commit that spans several crates may list them comma-separated: `refactor(dpe-core,dpe-web): ...`.
- **Concern over location.** Scope by the responsibility a change serves, not the enclosing directory. Documentation *about* a crate takes that crate's scope (`docs(dpe-api-oai): ...`); only repo-level docs take `docs`.
- A test *about* a specific concern takes that concern's scope (`test(dpe-core): ...`) — the `test` type already says it is a test.
- **No catch-all.** There is no `repo`/`all` scope. If none of the enumerated scopes genuinely fits, ask the maintainer before inventing one.

The gate enforces only that a scope is *present* — it does not restrict which one, so adding a crate needs no config change. The list above is the advisory vocabulary; keep to it.

## Enforcement

Commit messages are gated in CI by [`commitlint-rs`](https://github.com/KeisukeYamashita/commitlint-rs) via the `gate` job in [`commit-hygiene.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/workflows/commit-hygiene.yml) (a composite action at `.github/actions/commit-lint`). The rules live in [`.commitlintrc.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.commitlintrc.yml): the `type` allowlist above is mandatory, `scope-empty` makes the scope mandatory, and the subject must be non-empty.

The same job enforces the **one-commit-per-PR cap**. To land more than one commit, tick the `allow-many-commits` checkbox in the PR description — an unticked box does not count.

Run the same checks locally before pushing:

    just commit-lint            # checks origin/main..HEAD
    just commit-lint <base-ref> # checks <base-ref>..HEAD

`commitlint-rs` is installed by `just install-requirements`; CI installs it with `cargo install`.

Two rules need no dedicated check:

- `fixup!`/`squash!` commits do not parse as Conventional Commits, so the linter rejects them outright.
- Merge commits cannot reach `main` at all — the `main` ruleset enables `required_linear_history`. The linter skips them, since message rules do not apply to merges.

A second, **advisory** job asks a model whether the history reads well (squash / split / reword). It never blocks: it posts a comment, and any API error is a no-op.

## Commit Organization

### Principle

Start from the assumption that the whole PR is **one commit**. Group commits by user-visible impact, not by implementation journey. Only split the PR when the work genuinely divides into several independent, self-contained changes that each stand on their own.

### Rules

1. Each `feat:` or `fix:` commit = one changelog entry in the sections developers deploying the DPE read first.
2. Internal work (`build:`, `refactor:`, `docs:`, `chore:`, `test:`) lands in its own changelog section further down — squash aggressively so those sections stay readable.
3. Ask: "would a developer deploying this care about this change?" If yes → `feat:` or `fix:`. If no → an internal-work prefix.
4. Debugging journeys (trial-and-error, reverts of in-branch mistakes, iterative fixes) belong in the PR description, not the commit history. See [What `fix:` means](#what-fix-means).

### Where context lives

| Layer | Audience | Content |
|-------|----------|---------|
| Commit messages | Release notes readers | Every change; `feat:`/`fix:` carry the user-visible ones |
| PR description | Reviewers + future developers | Full context including challenges |
| Learnings docs | Future Claude + engineers | Structured, searchable knowledge |
| Code comments | Code readers | "Why not the obvious approach" |

## Pull Requests

### PR Description

The repository ships a [`.github/PULL_REQUEST_TEMPLATE.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/PULL_REQUEST_TEMPLATE.md) that pre-populates the expected structure (Motivation, Summary, Key Changes, Challenges and Decisions, Gotchas, Test Plan, Commit hygiene) when you open a PR. Fill it in rather than starting from scratch.

Use `Part of LINEAR-ID` instead of `Fixes LINEAR-ID` when the PR advances an umbrella issue it does not close.

### Why This Format Matters

The "Challenges and Decisions" section captures the debugging journey that would otherwise be lost when commits are squashed. Well-structured challenges become high-quality learnings automatically.

### PR Creation Process

1. Create as draft: `gh pr create --draft`.
2. Assign to the requesting developer: `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`.
3. If the PR keeps multiple commits, tick `allow-many-commits` and add a "Review Notes" section pointing out that the commits should be reviewed separately.

### What Goes Where

| Information | Put it in... |
|-------------|-------------|
| New feature / breaking change | Commit message (`feat:` / `feat!:`) |
| Bug fix | Commit message (`fix:`) |
| Build/CI/refactor details | Commit message (`build:` / `chore(ci):` / `refactor:`) |
| Why the work was needed | PR Motivation section |
| What was tried and failed | PR Challenges section |
| Architecture decisions + rationale | PR Challenges section |
| Things to watch out for | PR Gotchas section |
| Structured, searchable knowledge | Learnings doc (dasch-specs) |
