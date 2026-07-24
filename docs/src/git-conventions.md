# Git Conventions

The single source for how git is handled in this repository: the branch and
merge workflow, commit messages, commit organization, and pull requests. If
you read one file about git here, read this one — nothing else duplicates it.

## Git Workflow

We use a **rebase workflow**. All changes are made on a branch, then rebased onto `main` before being merged. This keeps a clean, linear commit history.

The goal is a **meaningful history on `main`**: every commit on main should be a deliberate, self-contained unit of change. Working commits ("WIP", "fix typo", "address review feedback") do not belong on main.

- **Rebase-merge**: PRs are integrated using rebase-merge (not squash or merge commits). Every commit on the branch lands on main verbatim — so the branch history _is_ the main history. There is no squash-on-merge safety net; whatever you leave on the branch is what ships.
- **Clean up before merging (mandatory)**: before a PR is merged, rewrite the branch (interactive rebase) so its commits read well on main. Squash working commits, reword messages, reorder as needed.
- **Default to a single commit**: almost always, a PR should end up as **one** clean commit. Split into multiple commits only when the work genuinely represents several independent, self-contained changes that each deserve their own line in the history — and each must stand on its own. When in doubt, squash to one.

## Commit Message Schema

Follow [Conventional Commits](https://www.conventionalcommits.org/). These prefixes drive [release-please](./deployment.md) to determine the SemVer bump and generate the changelog — **using the correct prefix is required, not optional**.

    type(scope): subject
    body

### Types

| Prefix | Meaning | Changelog | Version bump |
|--------|---------|-----------|--------------|
| `feat:` | New user-visible functionality | Features | minor |
| `fix:` | Bug fix | Bug Fixes | patch |
| `perf:` | Performance improvement | Performance | patch |
| `revert:` | Revert a previous commit | Reverts | patch |
| `refactor:` | Code restructuring | hidden | none |
| `test:` | Tests | hidden | none |
| `ci:` | CI/CD | hidden | none |
| `docs:` | Documentation | hidden | none |
| `build:` | Build system, deps | hidden | none |
| `style:` | Formatting | hidden | none |
| `chore:` | Maintenance | hidden | none |

Breaking changes take a `!` suffix (or a `BREAKING CHANGE:` footer) and bump the **major** version: `feat!: remove the deprecated cache endpoint`.

### Scopes

Scopes match crate names: `dpe-server`, `dpe-core`, `dpe-web`, `dpe-api-oai`, `dpe-telemetry`, `mosaic-tiles`, `mosaic-playground`. The scope is optional — omit it for genuinely repo-wide changes.

## Commit Organization

Group commits by user-visible impact, not by implementation journey. Start from the assumption that the whole PR is **one commit**, and only split it up if the work genuinely divides into multiple self-contained changes.

1. Each `feat:` or `fix:` commit = one changelog entry visible to deployers.
2. Internal work (`build:`, `ci:`, `refactor:`, `docs:`, `chore:`, `test:`) is hidden from the changelog — squash aggressively.
3. Ask: "would a developer deploying this care?" If yes → `feat:` or `fix:`. If no → hidden type.
4. Debugging journeys (trial-and-error, reverts of in-branch mistakes, iterative fixes) belong in the PR description, not the commit history.

## Pull Requests

### PR Description

The repository ships a [`.github/PULL_REQUEST_TEMPLATE.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/PULL_REQUEST_TEMPLATE.md) that pre-populates the expected structure (Motivation, Summary, Key Changes, Challenges and Decisions, Gotchas, Test Plan, Commit hygiene) when you open a PR. Fill it in rather than starting from scratch.

CI caps a PR at five commits (see [Commit Organization](#commit-organization)). If the work is genuinely several independent, self-contained changes, tick the `allow-many-commits` checkbox in the PR description to lift the cap — an unticked box does not count.

Use `Part of LINEAR-ID` instead of `Fixes LINEAR-ID` when the PR advances an umbrella issue it does not close.

### Why This Format Matters

The "Challenges and Decisions" section captures the debugging journey that would otherwise be lost when commits are squashed. Well-structured challenges become high-quality learnings automatically.

### PR Creation Process

1. Create as draft: `gh pr create --draft`.
2. Assign to the requesting developer: `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`.
3. If the PR keeps multiple commits, add a "Review Notes" section pointing out that the commits should be reviewed separately.

### What Goes Where

| Information | Put it in... |
|-------------|-------------|
| New feature / breaking change | Commit message (`feat:` / `feat!:`) |
| Bug fix | Commit message (`fix:`) |
| Build/CI/refactor details | Commit message (hidden type) |
| Why the work was needed | PR Motivation section |
| What was tried and failed | PR Challenges section |
| Architecture decisions + rationale | PR Challenges section |
| Things to watch out for | PR Gotchas section |
| Structured, searchable knowledge | Learnings doc (dasch-specs) |
