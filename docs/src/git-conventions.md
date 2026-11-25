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
- **Curate before merging (mandatory)**: before a PR is merged, rewrite the branch (interactive rebase) so its commits read well on main. Fold fixups into the commit they belong to, reword messages, reorder as needed.
- **One commit per scope per slice**: a commit is one slice of work (a phase of a plan, or a feature within it) inside one component of `ARCH-MAP.md`, whose scope it takes. A PR holds as many such commits as the work has slices. See [Commit Organization](#commit-organization).
- **At most 100 commits per PR**: GitHub's "Rebase and merge" is [limited to 100 commits](https://docs.github.com/en/repositories/creating-and-managing-repositories/repository-limits). Above that it refuses with "This branch cannot be rebased due to conflicts" although nothing conflicts. Larger work is cut into a stack at phase boundaries. CI enforces the limit; see [Enforcement](#enforcement).
- **Comments cleaned before the merge**: in the same curation pass, every comment, doc comment and command help string the branch adds or touches is held to `CONVENTIONS.md` → *Comments*. It describes the current state, carries no journey, and cites only stable sources (an ADR, a `DEV-` id, a `docs/` page), never a plan or a spec, a plan phase or a plan decision number. `eng:review:reference-reviewer`, bound in `eng.yaml`, checks each citation against its source during review. Its findings are folded into the commits they belong to, never fixed in a trailing commit.

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

    feat(dpe)!: remove the deprecated OAI verb alias

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

The release-please config sentence-cases subjects in the changelog, so write the subject in normal prose case (`feat(dpe): add project filter`, not `feat(dpe): Add ...`).

### Removed types

`revert`, `style`, and `ci` are **not** valid types — the commit-lint gate rejects them, and release-please hides them if one ever slips onto `main`.

- **CI/build automation** → `chore(ci): ...` (lands in Miscellaneous Chores). A dedicated Continuous Integration changelog section is noise; it belongs under maintenance. Note that `ci` remains a valid *scope*, just not a type.
- **Formatting-only changes** → fold into the functional commit, or `chore(<scope>): ...`. `rustfmt` and `maudfmt` are enforced gates, so a standalone formatting commit should be rare.
- **Reverts** → `fix(<scope>): revert ...` or `chore(<scope>): revert ...` depending on whether it corrects released behavior. (release-please's built-in revert auto-negation no longer applies; reverts are rare and handled by hand.)

### What `fix:` means

A `fix:` corrects behavior that exists on `main` — a bug a deployer could hit today, or that a released version shipped. It earns a "Bug Fixes" changelog line precisely because deployers need to know. The same change filed as `chore:` or `refactor:` still appears in the release notes, but under a section nobody scanning for a regression will read.

A bug you introduced earlier in the **same branch** is not a `fix:`. Fold it into the commit that introduced it (`git commit --fixup=<sha>`, then `git rebase --autosquash`) so it never lands on `main` and never generates a changelog entry for a bug nobody ever saw.

Corollary: if you discover a genuine pre-existing `main` bug while doing unrelated work, give it its **own** `fix:` commit — don't bury it inside the `feat:`/`refactor:` you happened to be writing.

## Scopes

**A scope is mandatory.** Every commit is `type(scope): subject`, so every release-notes line reads `concern: subject`. There is no scopeless form.

The scope is the **component the change serves**, as [`ARCH-MAP.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/ARCH-MAP.md) lists it under *Components* — not the crate, and not the directory the edited files happen to sit in. A component's crates share its scope: a change to `cpe-ports`, `cpe-core` and `cpe-web` is `cpe`. `ARCH-MAP.md` is the one list; the scope is the component's directory name, prefixed where two would collide:

| Component | Scope |
|-----------|-------|
| `areas/<area>/<capability>` | `<capability>`: `dpe`, `cpe`, `datei`, `sync`, `media`, `editor`, `identity`, `vre-bridge` |
| `areas/access/server`, `areas/deposit/server` | `access-server`, `deposit-server` |
| `areas/deposit/shell` | `deposit-shell` |
| `areas/archive` | `archive` |
| `shared/<role>` | `shared-<role>`: `shared-fair`, `shared-names`, … |
| `modules/mosaic` | `mosaic` |
| `dsp-cli`, `vitrinli`, `chischtli` | `dsp-cli`, `vitrinli`, `chischtli` |

**Cross-cutting scopes** (changes not tied to one component):

- `dpe-data` — project metadata files under `areas/access/dpe/server/data/`
- `ci` — workflows, the justfile, CI scripts
- `deps` — dependency bumps, Dependabot, base-image updates
- `docs` — repo-level and process documentation with no single component owner

Rules:

- Lowercase, kebab-case.
- A commit that spans several components may list them comma-separated: `feat(cpe,sync): ...`. It is rare: a `shared-*` piece a slice adds lands in its own commit ahead of the components that use it.
- **Concern over location.** Scope by the responsibility a change serves, not the enclosing directory. Documentation *about* a component takes that component's scope (`docs(dpe): ...`); only repo-level docs take `docs`.
- A test *about* a specific concern takes that concern's scope (`test(dpe): ...`) — the `test` type already says it is a test.
- **No catch-all.** There is no `repo`/`all` scope. A new scope is a new component, registered in `ARCH-MAP.md` (`/dune:dune-map`), not invented in a commit.
- **Keep `dsp-cli` commits under `dsp-cli/**` where possible.** release-please attributes a commit to a package by which paths it touches, not by its commit scope — a `dsp-cli`-scoped commit that also edits a root file (e.g. `MODULE.bazel`) lands in the root release too. See [Release Please](./deployment.md#dsp-cli) for the double-bump rule this causes.

The gate enforces only that a scope is *present* — it does not restrict which one, so adding a component needs no config change. Choosing the right component stays the author's job.

## Enforcement

Commit messages are gated in CI by [`commitlint-rs`](https://github.com/KeisukeYamashita/commitlint-rs) via the `gate` job in [`commit-hygiene.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/workflows/commit-hygiene.yml) (a composite action at `.github/actions/commit-lint`). The rules live in [`.commitlintrc.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.commitlintrc.yml): the `type` allowlist above is mandatory, `scope-empty` makes the scope mandatory, and the subject must be non-empty.

The same job enforces the **100-commit limit** per PR, GitHub's rebase-merge limit, so the gate fails with a clear message before the merge button fails with a misleading one.

The same job also **rejects merge commits on the branch**. The limit counts real commits, so a branch whose only fault is a merge is told about the merge rather than blamed for having too many commits.

**Update a stale branch by rebasing, never by merging `main` into it:**

    git fetch origin && git rebase origin/main && git push --force-with-lease

In the GitHub UI, use the chevron next to **Update branch** and pick **Update with rebase**. The plain button creates a merge commit, and GitHub offers no repository setting that changes that default or removes the option, so this gate is the only thing that catches it.

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

Shape the history for the agent that reads it later. An agent asks two questions of it: "why is this line the way it is?" (`git blame`, then the commit body) and "how did this component get here?" (`git log -- <path>`). Both are answered best by **one commit per scope per slice**:

- **Too fine** (a commit per screen, per fix, per review round) makes `git log -- <path>` a list of steps the agent must replay to learn the current state, and each body explains one step rather than the design.
- **Too coarse** (a multi-part PR as one commit) makes `git blame` land every line on one message that cannot explain any of them.

A slice is the unit the plan delivered: a phase, or a feature within it. Inside a slice, split by [component](#scopes), so each commit stays within one component's paths and its scope names that component. A `shared-*` piece a slice adds lands in its own commit ahead of the commits that use it, so no commit uses something that lands after it. Each commit builds and passes `just check` on its own. Small work that is one slice in one scope is one commit.

### The body is written for an agent

The subject says what changed; the body carries what the diff cannot:

- the decisions taken and the alternatives rejected, with the reason;
- the invariants the code relies on and where they are enforced;
- the gotchas a later change would trip over;
- the stable sources behind them (an ADR, a `DEV-` id, a `docs/` page).

It describes the end state of the slice, never the journey to it: no "first tried", no review-round history, no behaviour that existed only on the branch. A long body is fine; an agent reads all of it. A subject that says everything needs no body.

### Rules

1. Each `feat:` or `fix:` commit = one changelog entry in the sections developers deploying the DPE read first.
2. Fold trial-and-error and fixups into the commit they belong to, so the changelog carries no entry for work that never reached `main`.
3. Ask: "would a developer deploying this care about this change?" If yes → `feat:` or `fix:`. If no → an internal-work prefix.
4. Debugging journeys (trial-and-error, reverts of in-branch mistakes, iterative fixes) belong in the PR description, not the commit history. See [What `fix:` means](#what-fix-means).

### One PR or a stack

A plan whose curated commits fit under 100 ships as one PR. The commits carry the structure that separate PRs would otherwise carry, grouped per phase in history order, and the PR description has a **Review guide** with one section per phase. Take a separate PR or a stack only when the first rule that matches says so:

1. **The parts would need different Motivation sections** (see the PR template) → **separate PRs**, each from `main`.
2. **The part B builds on is already an open PR, or is ready for review while B is still unfinished** → **stack** with [`gh stack`](https://docs.github.com/en/pull-requests/how-tos/stacked-pull-requests) (`gh extension install github/gh-stack`): `gh stack init <A's branch>` adopts A locally, `gh stack add <B's branch>` puts B on top, and `gh stack submit` pushes both and creates the stack on GitHub.
3. **The work is over 100 commits** → **stack**, cut at phase boundaries: each layer a group of whole phases that builds and passes `just check` at its head. Never a cut by count in the middle of a phase.

Each layer of a stack is its own PR, follows every rule on this page, and is curated before it merges. The stack merges bottom-up with `gh stack merge --rebase`. A layer with a PR above it is never merged with `--delete-branch`: that closes the PR above. In a stack, the commit-hygiene gate counts commits from each PR's own base, so the 100-commit limit applies per PR.

A diff above 300 files or 20,000 lines does not render in full on GitHub, so such a PR is reviewed from its checkout through the Review guide.

### History lives in git, documents hold the current state

Documents in the repository — `docs/`, ADRs, `ARCH-MAP.md`, `CONTEXT.md`, `CLAUDE.md` files,
code comments — state what the code is and does now, or, for an ADR, the target it is being
moved to. They never narrate how they got there: no "previously X, now Y", no dated amendment,
no "superseded by", no status that goes stale once a PR merges. Write a document as though the
PR that introduces it is already merged. What is still to come — a decided target, an open
question, a deferred option — is current information and stays, marked as such and linked to
its Linear issue where one exists (`docs/adr/CLAUDE.md` → *What is still to come*).

The history of a document is the git history of its file. When a commit changes what a document
says — above all when it changes a decision in an ADR — the commit body states what the
document said before and why it changed. That body is the record of the decision's evolution;
`git log -p -- <file>` reads it back. A commit that only says "update ADR-0003" loses the one
thing the rewritten document no longer carries.

`docs/specs/` is the exception: PRDs, plans and journals are point-in-time records of what was
intended when they were written, and are not rewritten to follow the code.

### Where context lives

| Layer | Audience | Content |
|-------|----------|---------|
| Commit messages | Release notes readers, future agents and developers | Every change; `feat:`/`fix:` carry the user-visible ones; the body carries the decisions, invariants and gotchas of the slice, and what changed in a document or decision, and why |
| PR description | Reviewers + future developers | Full context including challenges |
| Documents and ADRs | Everyone | The current state, or the target state with a Linear id; never the journey |
| Learnings docs | Future Claude + engineers | Structured, searchable knowledge |
| Code comments | Code readers | "Why not the obvious approach" |

## Before a merge

A PR or a stack merges only when every line holds for its current head. Say in its Test Plan how each was met, or why it does not apply; a skipped check is stated, never implied.

1. **It stands on its own.** Nothing in it waits on later work.
2. **The history is curated,** comments are cleaned, and `just commit-lint` passes.
3. **CI is green,** on every layer of a stack, at the head being merged.
4. **The whole diff has had a review:** the reviewers `eng.yaml` resolves for its files, and no verified Critical or Warning finding left.
5. **A change someone can see has been seen.** When a page, a message or any rendered text changes or becomes newly reachable: `eng:test-browser` on the running app, and `eng:review:design-reviewer` and `eng:review:accessibility-reviewer` on the result.
6. **Nothing is left only in text.** Every follow-up has a Linear issue, and every learning is in `docs/learnings/` in the same PR.

## Pull Requests

### PR Description

The repository ships a [`.github/PULL_REQUEST_TEMPLATE.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/PULL_REQUEST_TEMPLATE.md) that pre-populates the expected structure (Motivation, Summary, Key Changes, Review guide, Challenges and Decisions, Gotchas, Learnings, Test Plan) when you open a PR. Fill it in rather than starting from scratch.

Use `Part of LINEAR-ID` instead of `Fixes LINEAR-ID` when the PR advances an umbrella issue it does not close.

### Why This Format Matters

The "Challenges and Decisions" section captures the debugging journey that the curated commit history deliberately omits. Well-structured challenges become high-quality learnings automatically.

### PR Creation Process

1. Create as draft: `gh pr create --draft`.
2. Assign to the requesting developer: `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`.
3. If the PR has several phases, fill in the Review guide with one section per phase.

### What Goes Where

| Information | Put it in... |
|-------------|-------------|
| New feature / breaking change | Commit message (`feat:` / `feat!:`) |
| Bug fix | Commit message (`fix:`) |
| Build/CI/refactor details | Commit message (`build:` / `chore(ci):` / `refactor:`) |
| Why the work was needed | PR Motivation section |
| What a reviewer should look at, per phase | PR Review guide section |
| What was tried and failed | PR Challenges section |
| Architecture decisions + rationale | PR Challenges section; a hard-to-reverse one also as an ADR |
| How a decision or document changed, and why | Commit body (the document itself is rewritten in place) |
| Things to watch out for | PR Gotchas section |
| Structured, searchable knowledge | Learnings doc (`docs/learnings/`) |
