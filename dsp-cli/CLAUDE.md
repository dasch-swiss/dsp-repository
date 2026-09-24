# CLAUDE.md

dsp-cli-specific guidance; project-wide guidance is in the top-level `CLAUDE.md`.

## Project Overview

`dsp-cli` is an AI-agent-friendly command-line interface for the DaSCH Service Platform (DSP). It
abstracts DSP-API's verbose RDF/JSON-LD surface behind the vocabulary a researcher actually uses —
**data-model**, **resource-type**, **field**, **value** — instead of DSP-API's own `ontology` /
`class` / `property` / `Value` subclass terms. The binary is named `dsp`; the crate is `dsp-cli`. It
is a root peer of the areas: it depends on no workspace crate and reaches DSP-API purely over HTTP.
v1 covers the VRE only, and is read-only (no write operations). See [`idea.md`](./idea.md) for the
vision.

## Build & Test Commands

All recipes go through `just`, run from the repository root:

```bash
just dsp-cli-run <args>       # cargo run --bin dsp -- <args>
just dsp-cli-test-live        # live tests (dsp-cli/ADR-0009 layer 5; needs DSP_TEST_SERVER etc.)
just dsp-cli-snap-review      # cargo insta review
just dsp-cli-stack-up         # start the pinned Fuseki + knora-api stack and load fixtures
just dsp-cli-stack-fixtures   # reload fixtures into an already-running stack
just dsp-cli-stack-down       # tear the stack down, volumes included
just check                    # workspace-wide: fmt/clippy/lint gates, including dsp-cli
just test                     # workspace-wide: the cheap test suite (dsp-cli/ADR-0009 layers 1-4)
```

The stack recipes need Docker; they wrap `dsp-cli/ci/stack/`, which the `dsp-cli-drift` workflow
uses to run the live tests in CI.

Single-test recipe: `cargo test -p dsp-cli <substring>` from the repository root. Live tests
require `--features live` and are `#[ignore]`d — see
[Testing Strategy](../docs/src/dsp-cli/testing-strategy.md).

## Architecture (1 paragraph)

Five layers (per [dsp-cli/ADR-0008](./docs/adr/0008-internal-architecture.md)): clap parser →
action layer → (`DspClient` trait + HTTP impl) and (`Renderer` trait + format impls) → domain
models → config resolution. Action functions take `&dyn DspClient` and `&mut dyn Renderer` so tests
inject mocks. The directory shape under `src/` mirrors this. Renderer methods are explicit per
(noun-group, shape); prose is irreducibly per-noun. See
[Architecture](../docs/src/dsp-cli/architecture.md) for the full crate layout and test-seam
rationale.

## Non-obvious constraints

These are easy to miss and load-bearing. Reviewers should call them out.

- **Vocabulary divergence is intentional.** The CLI says `data-model` / `resource-type` / `field` /
  `value-type` where DSP-API says `ontology` / `class` / `property` / `Value` subclass. Translation
  happens **once**, at the client/deserialisation boundary (`src/client/`). Everything above the
  client layer uses dsp-cli vocabulary; everything `OntologyDto`-shaped stays inside the client.
  This includes the word "export" (which DSP-API uses for what dsp-cli calls a "dump") — it must not
  appear in help text, error messages, type names, or module names outside `src/client/`. See
  [dsp-cli/ADR-0001](./docs/adr/0001-vocabulary-divergence.md) and the full glossary in
  [`CONTEXT.md`](./CONTEXT.md) — read that first before naming or renaming anything domain-facing.
- **Unit-test mocks are always LOCAL to the action file, deliberately duplicated rather than
  shared.** `tests/support/mod.rs` is a separate integration-test crate and is NOT reachable from
  `#[cfg(test)] mod tests` blocks inside `src/` — a plain Rust visibility boundary, not a style
  choice. Each action file that needs a mock client therefore defines its own `MockDspClient`
  inline; see `src/actions/auth/login.rs` for the canonical pattern. A plan that says "uses the
  shared `MockDspClient` for unit tests" is wrong: only integration tests (under `tests/`) can reach
  `tests/support/mod.rs`.
- **Extending a trait's method set has more touch-points than you expect.** Adding a method to
  `DspClient` or `Renderer` breaks every impl — including the local `MockDspClient` impls described
  above. Before drafting any step that extends a trait, run `grep -rn "impl DspClient for" src/
  tests/` (and likewise for `Renderer`) and list every path as a required touch-point.
- **No implicit session state, no default server, no positional identifiers.** Every command takes
  its identifiers as flags and fails fast if no server is specified. See
  [dsp-cli/ADR-0002](./docs/adr/0002-command-shape.md),
  [dsp-cli/ADR-0003](./docs/adr/0003-chaining-and-output.md),
  [dsp-cli/ADR-0007](./docs/adr/0007-auth-and-environments.md). The one carve-out is `.env` loading
  from CWD, which is "visible filesystem state", not in-process session state.
- **dsp-cli does not depend on dsp-tools** and never offers file-roundtripping commands. The
  boundary is interaction mode, not data touched. See
  [dsp-cli/ADR-0004](./docs/adr/0004-dsp-tools-boundary.md).
- **Exit codes are stable.** `0` success, `1` runtime, `2` usage (matches clap default), `3`
  auth-required. The `error.kind` field (top-level sibling of `_meta`, per dsp-cli/ADR-0003's
  envelope `{"_meta":…,"error":{"kind":…}}`) is the public contract for differentiating error
  kinds. See [dsp-cli/ADR-0012](./docs/adr/0012-diagnostics.md).
- **Stdout is data; stderr is everything else** (errors, logs, meta lines for non-prose formats).
  The JSON format is the only exception — error envelopes go to stdout to keep a single parser. See
  [dsp-cli/ADR-0012](./docs/adr/0012-diagnostics.md).
- **Output format is part of the public API** once shipped. Prose snapshot tests via `insta` are
  the regression detector. Every (noun, verb, format) cell gets a snapshot — see
  [Testing Strategy](../docs/src/dsp-cli/testing-strategy.md).
- **No `unwrap()` / `expect()` outside test code.** Library code returns `Result<T, Diagnostic>`.
  `anyhow` is only allowed in `main.rs`.
- **`#[allow(clippy::too_many_arguments)]` on six action functions in `src/actions/vre/project.rs`
  and `src/actions/vre/sparql.rs` is deliberate**, not an oversight to "clean up" — those signatures
  carry the action layer's full context (config, client, renderer, and command-specific args) by
  design, per the layering in dsp-cli/ADR-0008. Splitting them to satisfy the lint would break the
  uniform `fn(args, cfg, client, renderer)` shape the action layer relies on.
- **Pre-1.0 versioning:** `0.x.y`. Breakage is allowed. [`CHANGELOG.md`](./CHANGELOG.md) is generated
  by release-please from commit messages; don't hand-edit it.

## Documentation index

- [`idea.md`](./idea.md) — vision and design principles.
- [`CONTEXT.md`](./CONTEXT.md) — canonical domain glossary. Read first.
- [`CHANGELOG.md`](./CHANGELOG.md): generated by release-please; don't hand-edit it.
- [`docs/adr/`](./docs/adr/) — dsp-cli's own Architecture Decision Records, cited qualified
  (`dsp-cli/ADR-NNNN`) everywhere, including from inside this crate — root ADRs are cited bare. See
  [ADR-0006](../docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md). Read the ADR
  before contradicting it.
- [`docs/topics/`](./docs/topics/) — end-user documentation embedded in the binary at compile time,
  surfaced via `dsp docs <topic>`. See [dsp-cli/ADR-0010](./docs/adr/0010-embedded-documentation.md).
- [Architecture](../docs/src/dsp-cli/architecture.md) — layers, test seam, crate layout.
- [Testing Strategy](../docs/src/dsp-cli/testing-strategy.md) — test layers, live-mode env vars,
  strict mode.
- [Usage](../docs/src/dsp-cli/usage.md) — install, `dsp docs`, server shortcuts, auth.
- **Agent skill** — the Claude Code skill for agents using `dsp` is **not** in this repo. It lives
  in [`dasch-claude-plugins`](https://github.com/dasch-swiss/dasch-claude-plugins) as
  `misc:dsp-cli` (plugin marketplace); see
  [dsp-cli/ADR-0011](./docs/adr/0011-distribution-and-discoverability.md).

## When in doubt

- A change that contradicts an ADR is a red flag. Either the change is wrong, or the ADR needs
  amendment in the same PR.
- A new domain noun or verb that isn't in `CONTEXT.md` is a red flag. Pause, define it, add it,
  then continue.
- Output shape changed? Run `just dsp-cli-snap-review` and accept the diff in the PR.
- Errors must carry a stable `kind` and a helpful message. Stringly-typed errors are not
  acceptable.
