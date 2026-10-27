---
title: "refactor: write Datastar actions through a typed builder and hold attribute names to a whitelist grammar"
type: refactor
date: 2026-10-27
author: "Marcin Procyk"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7369
---

# refactor: write Datastar actions through a typed builder and hold attribute names to a whitelist grammar

## Overview

Datastar action expressions (`@get(…)`, `@post(…)`) in DPE, CPE and the editor move from hand-written strings to a typed builder in a new `shared-datastar` crate. Attribute names stay Maud literals, because Maud cannot take anything else, and `check-datastar-delimiters.sh` is replaced by `check-datastar-attributes.sh`: a whitelist grammar every Datastar-shaped name must parse under, plus a guard that no raw action expression comes back.

## Order of work

Two PRs under DEV-7369:

- **The refactor PR: [#545](https://github.com/dasch-swiss/dsp-repository/pull/545), branch `refactor/dev-7369-typed-datastar-actions`, from `origin/main`.** This plan is its first commit; `eng:working` (`/eng:workflows:work <this plan>`) runs Phases 1-4 on it. The crate, the three migrations and the gate. Rendered HTML does not change for today's inputs.
- **The fix PR: branch `fix/dev-7369-cpe-boolean-data-on`, from `origin/main`, one commit.** Done directly from *Prerequisite: the fix PR* below, not by `eng:working`, which runs every phase on the branch the plan lives on. CPE's boolean display renders `span.v-bool data-on="true|false"` (`areas/access/cpe/components/src/component/resource_detail.rs`, the `PropertyValue::Boolean` arm), a styling hook named like the Datastar `on` plugin. The vendored Datastar 1.0.2 registers `on` with `requirement:"must"` and throws `KeyRequired` for a key-less `data-on`, so every boolean value on a CPE page raises a Datastar error. The fix renames the hook to `data-bool`.

Sequence:

1. Open the fix PR at any point before Phase 4; it needs nothing from the refactor.
2. `eng:working` runs Phases 1-3 on #545; none of them depends on the fix.
3. **Gate H1:** the owner merges the fix PR.
4. Phase 4 rebases #545 onto `origin/main`, which brings the fix in, then lands the whitelist gate, which rejects the attribute the fix removes.
5. **H2:** the owner merges #545.

The fix ships on its own because it is a user-visible defect that should not wait for a refactor's review, and because its motivation differs from the refactor's (`docs/src/git-conventions.md` → *One PR, several commits, or a stack*, rule 1).

## Prerequisite: the fix PR

Done directly (it is one commit), before Phase 4 starts:

1. `git fetch origin && git checkout -b fix/dev-7369-cpe-boolean-data-on origin/main` (a separate worktree if #545's branch is checked out).
2. In `areas/access/cpe/components/src/component/resource_detail.rs` (`PropertyValue::Boolean` arm), rename the hook on `span.v-bool` from `data-on` to `data-bool`, keeping the `"true"`/`"false"` values; rename the local `data_on` and rewrite the comment above the `html!` that names `data-on`.
3. Update the rustdoc that describes the hook: the module doc line `Boolean → span.v-bool[data-on]` and the `Boolean` bullet above the value renderer.
4. In `areas/access/cpe/components/style/record.css`, change both `.v-bool[data-on="false"]` selectors to `data-bool`.
5. Update the boolean tests in `resource_detail.rs` to assert `data-bool="true"`/`"false"`, and add `!out.contains("data-on=")` to both (fails against the unfixed code).
6. `just check && just test` passes, with no snapshot changed (no `.snap` contains `v-bool`).
7. Review with `eng:review:maud-datastar-reviewer` and `eng:review:accessibility-reviewer`; amend verified findings.
8. Commit `fix(cpe-components): stop a boolean value's styling hook from reading as a key-less data-on (DEV-7369)`, push, open the PR (`Related to DEV-7369`; Motivation: the `KeyRequired` error).
9. Optional: `eng:test-browser` on a CPE record with a boolean value: no `KeyRequired` in the console, and the false value keeps its faint styling.

## Problem Statement / Motivation

Datastar attributes are plain strings in Maud markup. A wrong spelling renders fine and leaves the control inert; the only guard is a `git grep` blacklist that grows one regex per mistake already shipped (the pre-RC.6 `-` delimiter, then `retry: 'never'` without `retryMaxCount: 0` from DEV-7366). Any other misspelled modifier, key-less plugin or malformed action passes CI. ADR-0004 names that gate plus the editor's no-JS E2E pass as the enforcement; DPE and CPE have the gate alone. Review on #429 asked for a type-safe DSL so an unwanted attribute cannot be written.

What the repository allows (verified 2026-10-27 against `origin/main` `c25ca4e4`):

- **Maud 0.27.0 (latest release) takes only literal tokens as attribute names.** `HtmlNameFragment` in `maud_macros-0.27.0/src/ast.rs` is `Ident | LitInt | LitStr | Empty`, joined by `-` or `:`; no splice, no macro call. A DSL cannot produce the name, so DEV-7369's first acceptance bullet ("no raw `data-on:` string remains") is unreachable; the whitelist gate replaces it.
- **`datastar` 0.3.1 (pinned) and 0.4.1 (latest) are server-side SSE only** (`axum`, `rocket`, `warp`, `consts`, `execute_script`, `patch_elements`, `patch_signals`). Nothing on the markup side to reuse.
- **Maud splices any `Display` type** (`maud-0.27.0/src/lib.rs:463`), and escapes literal values and splices alike, with the same set: `& < > "` (`maud-0.27.0/src/escape.rs`). A builder emitting raw text through `Display` renders byte-identically to today's literals and `format!` splices.
- **17 production action lines**: `cpe-components` 11, `editor-web` 4, `dpe-web` 2 (site map under Phases 1-3; counted with gate rule 2's region rule below).

## Proposed Solution

### `shared/datastar` (package `shared-datastar`)

Depends on nothing but `std` (`maud` as a dev-dependency only). Two builder types, one per method, so combinations nobody uses cannot be written. Each implements `Display`; neither implements `maud::Render`, and neither pre-escapes HTML, because Maud escapes the splice.

```rust
// shape, not final code
pub struct Get { path: String, tail_js: Option<String>, retry_never: bool }
pub struct Post { path: String, submitter_first: bool, form: bool }

pub fn get(path: impl Into<String>) -> Get;
pub fn post(path: impl Into<String>) -> Post;

impl Get {
    /// `'path' + <expr>`: the path is a literal, the tail a JS expression.
    pub fn plus_js(self, expr: impl Into<String>) -> Self;
    /// `{retry: 'never', retryMaxCount: 0}`: the two keys cannot be split (DEV-7366).
    pub fn retry_never(self) -> Self;
}
impl Post {
    /// `evt.submitter?.formAction || 'path'` (see the editor's `formaction` pitfall).
    pub fn submitter_first(self) -> Self;
    /// `{contentType: 'form'}`
    pub fn form(self) -> Self;
}
```

Output, character for character, is today's:

| Builder | Renders |
|---|---|
| `get("/x")` | `@get('/x')` |
| `get("/x").retry_never()` | `@get('/x', {retry: 'never', retryMaxCount: 0})` |
| `get("/x").plus_js("encodeURIComponent(evt.target.value)")` | `@get('/x' + encodeURIComponent(evt.target.value))` |
| `post("/x").form()` | `@post('/x', {contentType: 'form'})` |
| `post("/x").submitter_first().form()` | `@post(evt.submitter?.formAction \|\| '/x', {contentType: 'form'})` |

The path is percent-encoded for `'`, `` ` `` and `\` (the rule of `cpe_core::util::encode_get_arg`, which moves here). For CPE this is today's behaviour (`%27` contains no `'`, so nothing double-encodes). For DPE and the editor it is new: their `shortcode`/`value`/action paths are spliced unencoded today. No current input contains any of the three characters, so rendered HTML is unchanged; the encoding is a deliberate hardening for inputs that would.

Not typed: signal and JS expressions (`$_expanded = !$_expanded`, `data-signals` objects, handler bodies, the JS prefix of a compound statement). Typing them is a JS DSL and out of scope.

```rust
// call sites
a href=(href) data-on:click__prevent=(datastar::get(tab_url).retry_never()) { … }
form data-on:submit=(datastar::post(action).submitter_first().form()) { … }
// a compound statement keeps its JS prefix; only the action is typed
data-on:click__prevent=(format!("document.getElementById('q').value = ''; {}", datastar::get(&frag)))
```

### `check-datastar-attributes.sh`

Replaces `check-datastar-delimiters.sh`. Same pathspecs (`modules/*/*/src/*.rs`, `areas/*/*/src/*.rs`, git pathspecs so `*` crosses `/`) and the same absence check (no file matched is an error).

**Production region** of a file: every line before the first column-0 `#[cfg(test)]` whose next non-attribute line starts with `mod `. A `#[cfg(test)]` on a `use`, `fn` or indented item does not end the region (`areas/access/cpe/web/src/project.rs:4` is one). Skipped: files named `*tests.rs`, `*test_support.rs`, `*test_utils.rs`; lines whose first non-blank characters are `//` (so `///` and `//!` too).

**Rule 1, names.** A token is Datastar-shaped when it is `data-<plugin>` followed by `:`, `_`, `-`, `"`, `=`, whitespace or the end of the token, with `<plugin>` one of the 13 the vendored 1.0.2 bundle registers (`on bind attr class signals indicator show text style effect init ref computed`), or when it is any `data-<word>:`. Every Datastar-shaped token must match the grammar below, which admits exactly what the code uses today and grows only with the change that first needs more:

- `data-on:<event>(__<modifier>)*`, event `[a-z][a-z0-9-]*`, key required.
- `data-(attr|class|signals|bind|indicator):<key>`, key `[A-Za-z_][A-Za-z0-9_-]*` (covers `_tab_loading`, `aria-expanded`), or key-less `data-(attr|class|signals|bind|indicator|show|text|effect|init)`.
- Modifiers, on `data-on` only, chainable: `prevent`, `window`, `outside`, `debounce.<n>(ms|s)`, `throttle.<n>(ms|s)` with optional `.trailing`.
- Anything else fails: the hyphen forms (`data-on-click`, `data-attr-hidden`), a key-less `data-on`, an unknown modifier, `data-style`/`data-ref`/`data-computed` (unused today), and a custom attribute that starts with a plugin name and a hyphen (`data-text-size`). Rename such a custom attribute.

**Rule 2, actions.** No `@(get|post|put|patch|delete)(` in a production region. Actions come from `shared-datastar`, which sits outside the pathspecs.

The retry check goes: rule 2 plus `Get::retry_never` make the DEV-7366 mistake unrepresentable.

**Residual**, stated in ADR-0004: a misspelled key-less plugin (`data-sigals=`) looks like a custom `data-*` attribute and passes; so do the hyphen-named plugins the bundle also ships (`ignore-morph`, `json-signals`, `preserve-attr`); an action built through `PreEscaped` or a computed method name escapes rule 2; JS inside values is unchecked. The no-JS E2E passes remain the backstop.

## Alternative Approaches Considered

- **Typed names via a proc macro wrapping `html!`.** Rejected: breaks `maudfmt` (the `just check` format gate) and IDE support for every template, for a guarantee the whitelist gives at CI time.
- **Typed actions only, blacklist gate kept.** Rejected: a misspelled name or modifier still passes, which is the reviewer's point.
- **Validator over rendered HTML in tests.** Rejected as the primary guard: it sees only what tests render, and nothing is typed at write time.
- **Rewriting keyed attributes to key-less object forms** to shrink the name surface. Rejected: changes rendered HTML, and `data-on` has no key-less form.
- **The fix as a phase of this plan, or a stack under #545** (`gh stack`). Rejected: `eng:working` runs every phase on the branch the plan lives on and ships one PR per repository, so a fix phase would land in #545; a separate one-commit PR merged before Phase 4 keeps both PRs clean, at the cost of one rebase in Phase 4.

## Technical Considerations

- **Placement.** Three apps in two areas use it, and `check-cpe-seam.sh` forbids CPE from depending on Mosaic, so `shared/` is the only home (`docs/src/repo_structure.md` → Shared Crates). `check-capability-deps.sh` allows `shared/*` from every role; `check-shared-paths.sh` forbids only paths into `areas/`/`modules/`, so route literals in the crate's tests are fine. `bacon.toml` watches `shared`, and every path-filtered workflow lists `shared/**`.
- **Byte-identical HTML** is the refactor PR's contract for today's inputs. The snapshot suites (`insta`) and the `out.contains(…)` assertions are the oracle: a snapshot mismatch fails `just test`, and a migration that needs a snapshot update is wrong, not the snapshot.
- **ADR rules** (`docs/adr/CLAUDE.md`): ADR-0004 and ADR-0007 are rewritten in place to the current decision; the commit body records what they said before and why.
- **`ARCH-MAP.md`** is owned by `dune:dune-map`; each step that uses it names the expected result.
- **Commits** (`CLAUDE.md`, `docs/src/git-conventions.md`): `type(scope): subject`. #545 carries several self-contained commits (the plan, then one per phase), so its body ticks `allow-many-commits` and has Review Notes; the fix PR is one commit. Review findings are amended into the phase's commit; no `fixup!` is left on a pushed branch.

## Implementation Phases

Verification in every phase: `just check && just test` (`eng.yaml` `commands.gate`). Reviewers for the phase reviews, beyond the three `eng:working` always runs: `eng:review:maud-datastar-reviewer`, `eng:review:security-reviewer`; `eng:review:devops-reviewer` for Phase 4.

#### Phase 1: shared-datastar, and DPE's actions through it

- [x] Create `shared/datastar` (package `shared-datastar`, no dependencies, `maud` as dev-dependency) with `get`/`post`, `Get`/`Post` and `Display` as under Proposed Solution, and add it to the root `Cargo.toml` workspace members
- [x] Unit tests in `shared-datastar` pinning each row of the output table, and the percent-encoding of `'`, `` ` `` and `\` in a path
- [x] A test in `shared-datastar` rendering `get("/x?a=1&b=2")` through `maud::html!` as an attribute value and asserting `&amp;` (guards against a future `Render` impl or pre-escaping)
- [x] Migrate `areas/access/dpe/web/src/pages/project/components/project_details_tabs/mod.rs` (the tab link's `format!` inside `({ … })`) to `datastar::get(format!(…)).retry_never()`
- [x] Migrate `areas/access/dpe/web/src/pages/projects/components/project_search_input.rs` (`"data-on:input__debounce.300ms"`, a literal value) to `datastar::get("/dpe/projects/search")`, keeping the quoted attribute name
- [x] Add `shared-datastar` to `dpe-web`'s dependencies; commit the `Cargo.lock` change
- [x] Add a `shared-datastar` row to the crate table and the `shared/` entry of the tree in `docs/src/repo_structure.md`
- [x] Add `shared-datastar` to `shared/README.md` (tree and a section like the other crates')
- [x] Add `shared-datastar` where `CONTEXT.md` lists the shared crates
- [x] Add `shared-datastar` to the crate scopes in `docs/src/git-conventions.md` and `CONVENTIONS.md`
- [x] Add `shared/datastar/**` → `eng:review:security-reviewer` to `eng.yaml` (the crate encodes URLs into a JS string context)
- [x] With `dune:dune-map`, register `shared-datastar` in `ARCH-MAP.md`: a `shared/datastar` section with Paths and Local-context kit like the other shared crates, and `shared-datastar` in DPE's Depends-on; check the diff shows exactly that
- [x] Add the `shared-datastar` edge to `docs/src/dpe/project_structure.md` (dependency graph and the `dpe-web` entry)
- [x] In `docs/src/dpe/architecture.md` → Datastar Attribute Conventions, state the retry rule as `Get::retry_never()` instead of a string to remember
- [x] In `areas/access/dpe/CLAUDE.md` → Maud + Datastar attribute syntax, add that actions are built with `shared_datastar`, never written as strings
- [x] Run `just check && just test`; passes, no snapshot changed
- [x] Commit `refactor(shared-datastar,dpe-web): build DPE's Datastar actions with a typed builder (DEV-7369)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: the editor's actions through shared-datastar

- [x] Migrate the submit action in `areas/deposit/editor/web/src/pages/section.rs` (the three-part Maud concatenation on `data-on:submit`) to `datastar::post(action).submitter_first().form()`
- [x] Migrate the autosave action in `section.rs` to `datastar::post(action).form()` (no `submitter_first`: today's string has none); change its `.then(|| format!(…))` to yield `Option<Post>` and splice it as `"data-on:change__debounce.1s"=[autosave]`
- [x] Migrate `areas/deposit/editor/web/src/pages/review.rs` (`data-on:submit` concatenation) to `datastar::post(action).form()`
- [x] Migrate `areas/deposit/editor/web/src/entity/mod.rs` (`data-on:submit` concatenation) to `datastar::post(action).submitter_first().form()`
- [x] Add `shared-datastar` to `editor-web`'s dependencies; commit the `Cargo.lock` change
- [x] Add `shared-datastar` to editor-web's Depends-on in `ARCH-MAP.md`
- [x] In `areas/deposit/editor/CLAUDE.md`, point the "row control is a submit button carrying `formaction`" pitfall at `Post::submitter_first()` and add the builder rule to "Datastar attribute syntax"
- [x] Add the builder rule to `docs/src/editor/rendering.md` where it states the Datastar attribute syntax
- [x] Run `just check && just test`; passes, no snapshot changed
- [x] Commit `refactor(editor-web): build the editor's Datastar actions with shared-datastar (DEV-7369)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: CPE's actions through shared-datastar

- [x] Migrate `areas/access/cpe/components/src/component/search_bar.rs`: the input action to `datastar::get(prefix).plus_js("encodeURIComponent(evt.target.value)")`, the clear action to its JS prefix plus `datastar::get(prefix)`
- [x] Migrate `compound_viewer.rs`: `picker_click_expr` takes the raw path and ends in `datastar::get(path).plus_js("a.search")`; `compound_fragment_get` returns `Get`, and its `Option<String>` consumers take `Option<Get>`
- [x] Migrate `resource_list.rs`: the sort action becomes `datastar::get(format!("{prefix}sort=")).plus_js("evt.submitter.value")` inside the existing expression, the `{preventScroll: true}` part staying in the JS prefix; rewrite the stale comment about `&&` above it
- [x] Migrate the six actions in `facet_panel.rs` to `datastar::get(frag)`, keeping each compound statement's JS prefix (the `None`/`Some` arms included)
- [x] Drop the `encode_get_arg` calls that fed these actions, delete `encode_get_arg` and its unit tests from `areas/access/cpe/core/src/util.rs`, and keep the component-level encoding tests in `compound_viewer.rs` (rewrite their failure message that names `encode_get_arg`)
- [x] Rewrite every doc comment that names `encode_get_arg` (`util.rs`, `media.rs`, `facet_panel.rs`, `search_bar.rs`, `resource_list.rs`, `compound_viewer.rs`): callers now pass the raw path; `git grep encode_get_arg` returns only files under `docs/specs/`
- [x] Add `shared-datastar` to `cpe-components`' dependencies; commit the `Cargo.lock` change
- [x] Add `shared-datastar` to the `cpe-components` row of `docs/src/repo_structure.md` (its dependency list is exhaustive) and to cpe-components' Depends-on in `ARCH-MAP.md`
- [x] Rewrite ADR-0007's clause "`cpe-components` depends on CPE's crates and on no web framework or database crate" to admit the `shared-*` crates, as its `cpe-image-viewer` clause does; the commit body records the old wording
- [x] Run `just check && just test`; passes, no snapshot changed
- [x] Commit `refactor(cpe-components): build CPE's Datastar actions with shared-datastar (DEV-7369)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: the whitelist gate replaces the delimiter blacklist

**Gate: H1**: resolve before starting this phase.

- [ ] `git fetch origin && git rebase origin/main`, bringing in the fix PR; `resource_detail.rs` renders `data-bool`, and `just check && just test` passes before any gate change
- [x] `git mv` `.github/scripts/check-datastar-delimiters.sh` to `check-datastar-attributes.sh` and rewrite it to the production region, rule 1 and rule 2 under Proposed Solution, without the retry check; its failure messages cite `areas/deposit/editor/CLAUDE.md` → Common Pitfalls → Datastar attribute syntax and `docs/src/dpe/architecture.md` → Datastar Attribute Conventions
- [x] `git mv` the test to `check-datastar-attributes.test.sh` and rewrite its cases: clean tree passes; each hyphen form fails; a key-less `data-on` fails; an unknown modifier fails; `data-onn:click` fails; a chained `__window__throttle.200ms.trailing` passes; `data-indicator:_tab_loading` and `data-attr:aria-expanded` pass; a raw `@get(` and `@post(` fail; the same strings inside a `#[cfg(test)] mod`, a `*tests.rs` / `*test_support.rs` file, or a `//`, `///` or `//!` line pass; `#[cfg(test)] use` at the top of a file followed by a violating template fails; custom attributes (`data-only`, `data-frame`, `data-testid`, `data-bool`) pass; nested and area-level paths are scanned; an empty tree is an error
- [x] Rename the `justfile` recipe to `check-datastar-attributes` and update the `check` prerequisites
- [x] Update the comment in `.github/scripts/check-views-no-axum.test.sh` that names `check-datastar-delimiters.test.sh`
- [x] Rewrite ADR-0004's Consequences and *Enforced by* to the current enforcement (typed actions, whitelist names, the residual under Proposed Solution, the no-JS E2E passes); the commit body records what they said before and why
- [x] Update the gate name in `docs/adr/0002-areas-at-the-repository-root.md` (the list of gates that glob application directories)
- [x] Update the gate name in `ARCH-MAP.md` (the Hypermedia enforcement line and the CI gate list)
- [x] `git grep -l check-datastar-delimiters` lists only files under `docs/specs/`
- [x] Run `just check && just test`; passes (the new test runs through `just test-scripts`)
- [x] Commit `chore(ci): hold Datastar attribute names to a whitelist grammar and keep actions typed (DEV-7369)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge the fix PR (`fix/dev-7369-cpe-boolean-data-on`, see *Prerequisite: the fix PR*) | Marcin | before Phase 4 (Why mid-plan: only Phase 4's gate needs the fix, which rejects the attribute the fix removes; making it `before start` would hold Phases 1-3 on an unrelated review) | Merging is the owner's decision |
| H2 | Merge #545 (`refactor/dev-7369-typed-datastar-actions`) | Marcin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] CPE renders no key-less `data-on`; the boolean false styling is unchanged (the fix PR)
- [x] Every `@get`/`@post` in dpe, cpe and editor markup comes from `shared_datastar`: gate rule 2 passes, and `git grep -n -E "@(get|post)\(" -- 'areas/*.rs'` hits only comment lines and test code
- [x] Datastar attribute names are Maud literals that `check-datastar-attributes.sh` holds to the whitelist grammar; `check-datastar-delimiters.sh`, its test and its recipe are gone
- [x] Rendered HTML unchanged by #545: `just test` passes with no snapshot updated
- [x] ADR-0004, ADR-0007, ADR-0002, `ARCH-MAP.md`, `docs/src/repo_structure.md`, `shared/README.md`, `CONTEXT.md`, `docs/src/git-conventions.md`, `CONVENTIONS.md`, `eng.yaml`, `docs/src/dpe/architecture.md`, `docs/src/dpe/project_structure.md`, `docs/src/editor/rendering.md` and the DPE and editor `CLAUDE.md` files describe the new crate and enforcement
- [x] No existing spec under `docs/specs/` is rewritten

Deviation from DEV-7369's acceptance, decided during planning: "no raw `data-on:` / `data-bind:` … string remains" is replaced by the whitelist gate, because Maud 0.27 cannot take a computed attribute name.

## Dependencies & Risks

- Phase 4 depends on the fix PR being merged (H1); Phases 1-3 do not.
- **Byte drift in a migrated value** (a space in an options object, quote style): the builder's unit tests pin every shape; snapshots and the `contains` assertions catch drift at the covered sites.
- **Gate false positives** on prose, test code or custom attributes: the region rule, the comment skip and the delimiter-after-plugin rule exist for this; the Phase 5 test cases pin them.
- **Whitelist too tight for the next feature**: by design. A new modifier or plugin form is a one-line grammar edit in the change that first uses it.

## Success Metrics

- Baseline (`origin/main` `c25ca4e4`): 17 raw action lines in production regions; a two-pattern blacklist gate. After #545: 0, as gate rule 2 reports in `just check`; every Datastar-shaped name parsed by the grammar.
- `just test` on #545 passes with zero snapshot changes.

## References

- Issue: DEV-7369; origin DEV-7366 / #429 review
- `maud_macros-0.27.0/src/ast.rs` (`HtmlName`, `HtmlNameFragment`); `maud-0.27.0/src/lib.rs:463`; `maud-0.27.0/src/escape.rs`
- Vendored Datastar 1.0.2: `areas/{access/cpe,access/dpe,deposit/editor}/public/vendor/datastar.js` (`on` plugin `requirement:"must"` → `KeyRequired`)
- `eng:working` (eng 4.5.0) *Phases and repositories*: one branch per repository, and a merged PR's remaining phases start on a fresh branch
- `docs/adr/0004-hypermedia-frontends.md`, `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`, `docs/adr/CLAUDE.md`, `docs/src/repo_structure.md` → Shared Crates
- Prior context: `docs/specs/2026-10-11-move-cpe-components/01-feat-move-cpe-components-plan-journal.md` (deferred widening of the gate)
