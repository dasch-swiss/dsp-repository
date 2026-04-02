# Workflows and Conventions

## Entry Points

The first entry point of this repository is the README file, which should give anyone an indication of where to find any information they need.

For any interaction or coding-related workflow, the justfile is the primary source of truth. Run `just` without arguments to see all available commands with descriptions.

## Key Development Commands

| Command | Description |
|---------|-------------|
| `just check` | Run formatting and linting checks |
| `just build` | Build all targets |
| `just test` | Run all tests |
| `just fmt` | Format all Rust code (cargo fmt + leptosfmt) |
| `just run` | Run server (release mode) |
| `just watch` | Watch for changes and run tests |
| `just watch-dpe` | Run DPE with hot reload |
| `just watch-mosaic-playground` | Run Mosaic playground with hot reload |
| `just install-requirements` | Install all development dependencies |
| `just install-e2e-requirements` | Install Playwright browsers for E2E tests |
| `just docs-serve` | Serve documentation locally at localhost:3000 |
| `just validate-data` | Validate all data files in the default data directory |

## Git Workflow

We use a **rebase workflow**. All changes are made on a branch, then rebased onto main before being merged. This keeps a clean, linear commit history.

- **Rebase-merge**: PRs are integrated using rebase-merge (not squash or merge commits). Every commit on a branch becomes a commit on main.
- **Clean commit history**: Before merging, clean up the branch so that each commit represents one logical unit of change. Squash fixups, reword messages, and reorder commits so the history reads well on main.

## Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/). Scopes match crate names: `dpe-server`, `dpe-core`, `dpe-web`, `dpe-api-oai`, `mosaic-tiles`, `mosaic-playground`, `mosaic-playground-macro`.

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

### Commit Organization

Group commits by user-visible impact, not by implementation journey.

1. Each `feat:` or `fix:` commit = one changelog entry visible to deployers
2. Internal work (`build:`, `ci:`, `refactor:`, `docs:`, `chore:`, `test:`) is hidden from changelog — squash aggressively
3. Ask: "would a developer deploying this care?" If yes → `feat:` or `fix:`. If no → hidden type.
4. Debugging journeys (trial-and-error, reverts, iterative fixes) belong in the PR description, not the commit history

## Pull Request Workflow

### PR Template

```
Fixes LINEAR-ID, LINEAR-ID, ...

## Motivation
Why this work was needed. What problem it solves for users.

## Summary
1-3 bullet points of user-visible changes.

## Key Changes
### [Topic]
- change details

## Challenges and Decisions
What was tried, what failed, and key architecture decisions.
Structure as sub-sections when multiple challenges exist:

### [Challenge title]
**Problem:** description of the issue encountered
**Tried:** approaches that didn't work and why
**Solution:** what worked and why it's the right approach

## Gotchas
Things future developers should know. Each gotcha should be
actionable — not just "this is hard" but "do X instead of Y".

## Test Plan
- [ ] verification steps
```

### Why This Format Matters

The "Challenges and Decisions" section captures the debugging journey that would otherwise be lost when commits are squashed. Well-structured challenges become high-quality learnings automatically.

### PR Creation Process

1. Create as draft: `gh pr create --draft`
2. Assign to the requesting developer: `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`
3. Include a "Review Notes" section mentioning that separate commits should be checked for easier review

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

## Release Workflow

Releases are automated via [Release Please](https://github.com/googleapis/release-please). On every push to `main`, Release Please reads conventional commit messages and either creates or updates a release PR. Merging the release PR creates a GitHub Release with auto-generated release notes.

## Code Review

See [Review Guidelines](./fundamentals/review-guidelines.md) for the review checklist.

## CI/CD

GitHub Actions workflows run automatically on pushes and pull requests. See [Release, Deployment and Versioning](./deployment.md) for details on the CI/CD pipelines.
