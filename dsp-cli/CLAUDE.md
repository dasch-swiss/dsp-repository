# CLAUDE.md

Orientation for AI agents (and humans) working on `dsp-cli`.

## What this is

`dsp-cli` is an AI-agent-friendly command-line interface for the DaSCH Service Platform (DSP).
It abstracts DSP-API's verbose RDF surface using domain-expert vocabulary. The binary is named `dsp`. The crate is `dsp-cli`. See [`idea.md`](./idea.md) for the vision.

## Current state

Personal exploratory greenfield project. **Working command surface**: the design is complete (12 ADRs, glossary, contributor docs, project plan).
Phase 1 is complete: clap subcommand tree, `Diagnostic` error type, snapshot tests, `Config::resolve` with `dotenvy` wiring, `MockDspClient` test seam,
five renderer stubs (`ProseRenderer`, `JsonRenderer`, `LinesRenderer`, `CsvRenderer`, `TsvRenderer`), and `--format` / `-j` / `-l` flags on the six vre leaf commands.
Phase 2 is complete: `auth.toml` cache extended with user + expiry fields, three `dsp auth` user commands (`login`, `status`, `logout`) shipped,
first reqwest-blocking HTTP client and `wiremock` test pattern landed, and `DSP_TOKEN` env override wired (overrides cached token;
`dsp auth status` reports when active and shows token expiry when readable).
A `dsp auth` amendment added a fourth command, `set-token` — caches a pre-issued JWT read from stdin after verifying it against the server (`GET /v2/authentication`);
fills the gap where `DSP_TOKEN` overrides but never persists a token.
A second `dsp auth` amendment (plan 026) added a fifth command,
`token` — the read-out counterpart to `set-token`: it prints the resolved bearer token (env or cache) verbatim to stdout for piping,
makes no server round-trip, and exits `3` (auth-required) when no token is cached for the server or a local, advisory check of the JWT `exp` claim finds it expired.
Phase 3 is complete (including Amendment 1): `dsp vre project dump` is the first command that does real work — it adds bearer auth (`Authorization: Bearer` header,
the first real auth use in a data command), multi-step trigger→poll→download orchestration with capped exponential backoff,
the `DumpTask`/`DumpOutcome` domain model and `ProgressReporter` trait, atomic binary streaming to disk,
progress reporting to stderr (human prose or NDJSON in json mode), and the live-test pattern (`tests/live_project_dump.rs` behind `--features live`).
Amendment 1 extended the dump command with existing-dump handling: re-running adopts an existing dump (idempotent by default),
`--replace` discards and recreates, `--delete` removes without downloading.
A cross-project guard (plan 008) has since been added: the DSP-API holds one dump server-wide,
so the dump command now detects when the slot is held by a different project and refuses;
`--replace --discard-other-project` explicitly discards that project's dump to make room, while `--delete` and the default mode never touch another project's dump.
Phase 4 is complete: `dsp vre project list` is implemented — the first read command,
establishing the per-noun `Renderer::projects` + `ProjectListView` view-struct pattern, the `(noun, format)` snapshot baseline,
the boundary translation for a list endpoint (DSP-API `/admin/projects` → dsp-cli `Project` model),
and ADR-0007 auth-state disclosure across all five formats (prose footer / `_meta.auth`, stderr line for lines/csv/tsv).
Phase 5 is complete: `dsp vre project describe` is implemented (first describe/single-object read command, `ProjectDetail`,
ADR-0003 single-object envelope). `dsp vre data-model list` has since shipped — establishing the `/v2/ontologies/metadata` JSON-LD read pattern,
the `DataModelListView` view-struct, `Renderer::data_models`,
and the first exposure of the built-in vs project-defined distinction (`--include-builtins` adds `knora-api`/`standoff`/`salsah-gui`).
`dsp vre data-model describe` has now also shipped — the first command to read the richer `/v2/ontologies/allentities`
JSON-LD endpoint and the first to surface **resource-types**: it summarises a single data-model's child resource-types (count +
aligned `name  label` list in prose) via the new `DataModelDetail` + `ResourceTypeSummary` models and `Renderer::data_model_describe`,
with the resource-class filter (`knora-api:isResourceClass`) and CURIE→IRI expansion (via the response `@context`) confined to the client boundary.
`dsp vre resource-type list` has now also shipped — the first command to surface a top-level resource-type list: it lists the resource-types defined in one data model,
with the new `ResourceType` list-projection model (name/iri/label/is_builtin) alongside the existing `ResourceTypeSummary` describe-child;
`-p`/`--project`, `--data-model`, `--filter`,
and `--include-builtins` flags (appending the 4 user-instantiable platform built-ins: `Region`, `AudioSegment`, `VideoSegment`, `LinkObj`;
`knora-base:Annotation` excluded as deprecated);
and the `builtin_resource_types()` helper parallel to `builtin_data_models()`. `dsp vre resource-type describe` has now also shipped — the
v1 leaf: full field list (name/value-type/cardinality/label) via the `ResourceTypeDetail` + `Field` + `Cardinality` + `Representation` models;
representation kind detection from the flattened file-value restriction; `Extends:` (project superclass);
cross-data-model field resolution with `[from <dm>]` tags (sibling-ontology fetch); `--include-builtins`; `-p` short for `--project`;
live-verified against `incunabula:Page` and `beol:manuscript`. `dsp vre data-model structure` has now also shipped — the final Phase 5 read command,
completing the v1 data-model command suite: it surfaces the link relations (link fields between resource-types, labelled with the field name;
cross-model targets tagged `[to <dm>]`) and `inherits` relations (superclass edges) within a data-model in a flat, edge-centric view; five output formats;
`--include-builtins` to reveal system superclasses and built-in link fields; single `/v2/ontologies/allentities` fetch;
v1 documented limitation (link fields defined in a sibling data-model and only reused here are omitted).
Phase 6 is complete (plan 018): `dsp docs [topic]` ships embedded end-user documentation — `dsp docs` lists topics,
`dsp docs <topic>` prints raw markdown to stdout, `--pager` pages through `$PAGER`, and an unknown topic exits `1` with a Levenshtein "did you mean" suggestion.
Nine topics are embedded at compile time via `include_str!` (a `const TOPICS` table in `src/actions/docs.rs`): `dsp-cli`, `dsp`, `concepts`, `identifiers`,
`connecting`, `output`, `workflows`, `errors`, `dsp-tools` — expanding ADR-0010's original six (added the operational `workflows`/`identifiers`/`errors`;
amended ADR-0010 in the same change). `dsp docs` is the one action with **neither a `DspClient` nor a `Renderer`** — its output is format-agnostic markdown,
so it has no `--format` flag;
it writes to an injected writer via a `run`/`run_impl` seam.
Topic bodies are not snapshot-tested (documentation prose; a smoke test checks each is non-empty and starts with `# ` — plan 018 D5).
Phase 6.5 is complete (plan 019): internal cleanups — shared ADR-0007 disclosure helpers in `src/render/table.rs` and a `local_name` helper in `src/client/http.rs`.
Phase 6.6 is complete (plan 020): cross-cutting output-format flags — `--columns` selects **and reorders** tabular output columns (csv/tsv/lines;
duplicates rejected as a usage error; valid names listed per command via `after_help`, drift-guarded against the render-layer column consts);
`--no-header`/`--header-only` control the csv/tsv header row (headers-on default unchanged);
and `dsp docs -j` emits a machine-readable JSON topic index (`{"_meta": {}, "data": [{name, summary}]}` — the one command whose `_meta` is empty; topic bodies stay raw markdown;
plan-018 D1's no-Renderer design intact).
All tabular renderer methods now route through a shared `render_table` engine (`TableSpec`/`TableOptions`/`HeaderMode`/`QuoteMode` in `src/render/table.rs`);
lines output replaces ASCII control characters in values with spaces (`lines_field`); the one engine bypass is `auth_logout` lines' decorated default.
ADR-0003 amended accordingly (`--fields` renamed, scope, header flags, lines sanitisation, TSV escaping cell, empty-`_meta` carve-out).
Phase 7 is complete (pre-0.1.0 hardening): `just install` end-to-end,
`_meta.auth` vocabulary harmonised across all commands (shared `read_auth_state` in `src/actions/auth_state.rs`), SKILL.md verified against the command surface;
prose server-text sanitisation was descoped (plan 021).
Phase 8a is complete (plan 022): `dsp vre resource list` is the **first instance-side read command**
— a new `vre resource` noun-group that lists data **instances** of a resource-type within a project.
It introduces **pagination** (`--page` 0-based / `--all` auto-paginate;
the `ResourceListPagination` enum carries `SinglePage`/`AllPages`,
surfaced as the new `_meta` keys `page` / `pages_fetched` / `may_have_more_results` — ADR-0003 amended) and the **first real use of
`MetaContext.filter_warning`** (ADR-0007 silent-filter disclosure for instance-side reads: anonymous → "login to see private resources";
authenticated → "results limited to your permissions";
appended in `render_table_disclosure`/`render_prose_footer` and the json `_meta.note`). `--resource-type` accepts a name or full class IRI;
a bare name is resolved by scanning the project's data-models (optional `--data-model` narrows/disambiguates; cross-data-model ambiguity is a usage error).
New `ResourceSummary`/`ResourcePage` models (`src/model/resource.rs`), `DspClient::list_resources` (`GET /v2/resources?resourceClass=…&page=…&schema=complex`,
`x-knora-accept-project` header), `Renderer::resources` + `ResourceListView`. `--order-by` deferred to a follow-up (BACKLOG).
The complex schema is used for all resource reads (live-verified 2026-06-17, plan 022 D4); the simple schema is not used.
Phase 8b is complete (plan 023): `dsp vre resource describe` — the **second instance-side read command** — fetches a single resource's metadata envelope (label,
resource-type, IRI, ARK URL, creation/last-modification dates, attached project, owner, and two **translated** permission facets).
New `ResourceDetail`, `ResourceVisibility`, `ResourceAccess` models; `DspClient::describe_resource` (`GET /v2/resources/<iri>?schema=complex`); `Renderer::resource_describe`;
five output formats; cross-project guard (`-p` triggers a mismatch error when the resource's `attached_project` differs from the supplied project);
ACL→visibility derivation and code→access derivation at the client boundary (ADR-0001); ADR-0007 `filter_warning` disclosure.
Phase 8c is complete (plan 024): `dsp vre resource describe --values` — full **value rendering** behind a default-off flag.
New `FieldValues`/`ValueContent`/`DateValue`/`FileValue` models; `with_values: bool` added to `DspClient::describe_resource`;
HTTP parsing of the complex-schema value objects with per-type dispatch (text/integer/decimal/boolean/date/time/uri/color/geoname/vocabulary-item/link/file + `raw` fallback);
field labels resolved on demand from project-ontology allentities (deduplicated per ontology, built-in knora-api fields degrade to local name);
list-node labels resolved from `GET /v2/node/…` (deduplicated, graceful); prose renderer strips control characters from every server-supplied scalar (ADR-0007 security surface;
D7 of plan 024); json keeps raw server values verbatim;
tabular formats rendered metadata-only with a stderr note at the time — since superseded (see Phase 8.5 item 3,
below). `html_to_text`/`strip_control_chars` relocated from `src/render/html.rs` to a new `src/util/text.rs` (layer-neutral, no cross-layer dependency violation;
`src/render/html.rs` removed).
ADR-0013 authored (instance reads and value rendering).
Phase 8.5 is in progress (pre-0.1.0 backlog hardening): `dsp vre resource list --order-by <field>` has shipped (plan 025) — its first item — resolving the plan-022 D2 deferral.
It sorts results server-side ascending by a resource-type field: a bare field name is resolved to its field IRI via `describe_resource_type` (action-layer,
mirroring `resolve_resource_type_iri`,
which now returns a `ResourceTypeRef { iri, name, data_model_iri }` struct),
and a full field IRI (`://`) bypasses the lookup. `DspClient::list_resources` gained an `order_by: Option<&str>` param (the resolved IRI;
the HTTP impl appends `orderByProperty` via a chained additive `reqwest` `.query()`).
Ascending-only (server constraint; no `--desc`), single-field, no `_meta` echo. Unknown field names and underivable data-models map to `Diagnostic::Usage`.
Its third item (plan 027) has also shipped: `dsp vre resource describe --values` tabular rendering — `lines`/`csv`/`tsv` now render **one row per
value** (superseding the metadata-only-plus-stderr-note behaviour above) via a new shared `render_value_content` helper (`src/render/value.rs`,
reused by prose);
full column set `label, iri, field, field_label, value_type, value`, compact default `field, field_label, value_type, value` (resource `label`/`iri` opt-in via `--columns`,
since they repeat identically on every row of a single-resource describe);
ADR-0013 amended.
`dsp vre resource-type list`/`describe` gained a `--count` flag (plan 030, released 0.1.2) — one extra v3-route call
(`resourcesPerOntology`) surfacing per-resource-type instance counts (non-deleted, NOT permission-filtered — disclosed via
a new `MetaContext.count_caveat`, distinct from ADR-0007's `filter_warning`); ADR-0013 amended.
Released as 0.1.3: an interactive update check (plan 031) — a gated, rate-limited crates.io version check that prints a stderr-only advisory on prose-format, interactive-TTY runs and is opt-out via `DSP_NO_UPDATE_CHECK`; owns the new ADR-0015.
The last remaining Phase 1 item has since landed (plan 032): the binary-level error handler in `main.rs` now routes every top-level error through the
selected output format's `Renderer::diagnostic` — `-j` emits the JSON error envelope to stdout instead of a prose line to stderr; other formats unchanged;
no `anyhow` introduced. Phase 1 is now fully complete.
Phase 10.5 (plan 035) has since shipped `dsp vre sparql query` — a raw SPARQL 1.1 passthrough to the
server's underlying triplestore (`SystemAdmin`-only, off by default per deployment), the first
command that deliberately does **not** abstract DSP-API: no `Renderer`, no `--format`, the store's
own status/media-type/bytes relayed verbatim to stdout. See
[`docs/adr/0016-sparql-passthrough.md`](./docs/adr/0016-sparql-passthrough.md).
See [`docs/PROJECT_PLAN.md`](./docs/PROJECT_PLAN.md).

## Build & Test Commands

All recipes go through `just`. **The incubator root justfile exposes this
crate as a `just` module, so from the repo root every recipe must be
prefixed with `dsp-cli`** (e.g. `just dsp-cli ci`). The bare forms below
only work if you first `cd dsp-cli`. Prefer the namespaced form in any
example you hand the user — see the big note in the repo-root `CLAUDE.md`.

```bash
just dsp-cli                 # list recipes
just dsp-cli dev <args>      # cargo run --bin dsp -- <args>
just dsp-cli test            # cheap test suite (ADR-0009 layers 1–4)
just dsp-cli test-live       # live tests (ADR-0009 layer 5; needs DSP_TEST_SERVER etc.)
just dsp-cli lint            # cargo clippy --all-targets -- -D warnings
just dsp-cli fmt-check        # cargo fmt --all -- --check
just dsp-cli fmt             # cargo fmt --all
just dsp-cli build           # cargo build --release --bin dsp
just dsp-cli snap-review     # cargo insta review
just dsp-cli ci              # fmt-check + lint + test (what CI runs)
```

Single-test recipe: `cargo test <substring>` runs only matching tests. Live tests require `--features live`.

**Nested Claude Code worktree caveat:** `just dsp-cli <recipe>` from the incubator repo root runs the recipe against the **main checkout** (`dsp-cli/`), not the worktree.
A cached build artifact can make it appear to succeed while compile errors in the worktree go undetected.
Inside a nested worktree, run `cargo` commands directly (e.g. `cargo build`,
`cargo test --all-targets`) from the worktree's `dsp-cli/` directory rather than relying on `just dsp-cli` from root.

## Architecture (1-paragraph)

Five layers (per [ADR-0008](./docs/adr/0008-internal-architecture.md)): clap parser → action layer →
(`DspClient` trait + HTTP impl) and (`Renderer` trait + format impls) → domain models → config resolution.
The action functions take `&dyn DspClient` and `&mut dyn Renderer` so tests inject mocks.
The directory shape under `src/` mirrors this. Renderer methods are explicit per (noun-group, shape); prose is irreducibly per-noun.

## Non-obvious constraints

These are easy to miss and load-bearing. Reviewers should call them out.

- **Vocabulary divergence is intentional.** The CLI says `data-model` / `resource-type` /
`field` / `value-type` where DSP-API says `ontology` / `class` / `property` / `Value subclass`.
Translation happens **once**, at the client/deserialisation boundary (`src/client/`).
Everything above the client layer uses dsp-cli vocabulary; everything `OntologyDto`-shaped stays inside the client.
This includes the word "export" (which DSP-API uses for what dsp-cli calls a "dump") — it must not appear in help text,
error messages, type names, or module names outside `src/client/`.
  See [ADR-0001](./docs/adr/0001-vocabulary-divergence.md) and [`docs/dev/domain-language.md`](./docs/dev/domain-language.md).
- **Extending a trait's method set has more touch-points than you expect.** Adding a method to `DspClient` or `Renderer` breaks every impl
— including local `MockDspClient` impls inside `#[cfg(test)] mod tests` blocks in individual action files (e.g. `src/actions/auth/login.rs`).
Before drafting any step that extends a trait, run `grep -rn "impl DspClient for" src/ tests/` (and likewise for `Renderer`) and list every path as a required touch-point.
See [`docs/dev/coding-conventions.md`](./docs/dev/coding-conventions.md).
- **Unit-test mocks are always LOCAL to the action file.** `tests/support/mod.rs` is a separate
integration-test crate and is NOT reachable from `#[cfg(test)] mod tests` blocks inside `src/`.
  Each action file that needs a mock client defines its own `MockDspClient` inline — see `src/actions/auth/login.rs` for the canonical pattern.
  A plan that says "uses the shared MockDspClient for unit tests" is wrong; only integration tests (under `tests/`) can reach `tests/support/mod.rs`.
- **No implicit session state, no default server, no positional identifiers.** Every command takes its identifiers as flags and fails fast if no server is specified.
  See [ADR-0002](./docs/adr/0002-command-shape.md), [ADR-0003](./docs/adr/0003-chaining-and-output.md), [ADR-0007](./docs/adr/0007-auth-and-environments.md).
  The one carve-out is `.env` loading from CWD, which is "visible filesystem state", not in-process session state.
- **dsp-cli does not depend on dsp-tools** and never offers file-roundtripping commands.
  The boundary is interaction mode, not data touched. See [ADR-0004](./docs/adr/0004-dsp-tools-boundary.md).
- **Exit codes are stable.** `0` success, `1` runtime, `2` usage (matches clap default), `3` auth-required.
  The `error.kind` field (top-level sibling of `_meta`, per ADR-0003's envelope `{"_meta":…,"error":{"kind":…}}`) is the public contract for differentiating error kinds.
  See [ADR-0012](./docs/adr/0012-diagnostics.md).
- **Stdout is data; stderr is everything else** (errors, logs, meta lines for non-prose formats).
  The JSON format is the only exception — error envelopes go to stdout to keep a single parser. See [ADR-0012](./docs/adr/0012-diagnostics.md).
- **Output format is part of the public API** once shipped. Prose snapshot tests via `insta` are the regression detector.
  Every (noun, verb, format) cell gets a snapshot. See [ADR-0009](./docs/adr/0009-testing-strategy.md).
- **No `unwrap()` / `expect()` in non-test code.** Library code returns `Result<T, Diagnostic>`. `anyhow` is only allowed in `main.rs`.
  See [coding-conventions](./docs/dev/coding-conventions.md).
- **Pre-1.0 versioning** — `0.x.y`. Breakage is allowed but every user-visible change adds a `[Unreleased]` entry in [`CHANGELOG.md`](./CHANGELOG.md).

## Documentation index

- [`idea.md`](./idea.md) — vision and design principles.
- [`CONTEXT.md`](./CONTEXT.md) — canonical domain glossary. Read first.
- [`docs/PROJECT_PLAN.md`](./docs/PROJECT_PLAN.md) — implementation plan with per-command checklist.
- [`docs/BACKLOG.md`](./docs/BACKLOG.md) — holding pen for surfaced ideas; triaged before each new task.
- [`CHANGELOG.md`](./CHANGELOG.md) — Keep a Changelog format; `[Unreleased]` is the working entry.
- [`docs/adr/`](./docs/adr/) — Architecture Decision Records. The "why" behind every load-bearing decision. Read the ADR before contradicting it.
- [`docs/dev/`](./docs/dev/) — contributor docs:
  - [`domain-language.md`](./docs/dev/domain-language.md) — ubiquitous-language practices.
  - [`coding-conventions.md`](./docs/dev/coding-conventions.md) — module layout, error handling, dependencies, style.
  - [`testing-strategy.md`](./docs/dev/testing-strategy.md) — recipes; ADR-0009 is canonical.
  - [`review-guidelines.md`](./docs/dev/review-guidelines.md) — definition of done; PR checklist.
  - [`git-workflow.md`](./docs/dev/git-workflow.md) — branching, conventional commits, draft PRs.
  - [`ai-agent-setup.md`](./docs/dev/ai-agent-setup.md) — agent harness; deliberately minimal during the personal phase.
- [`docs/topics/`](./docs/topics/) — end-user documentation embedded in the binary at compile time, surfaced via `dsp docs <topic>`.
  Will be authored in Phase 5. See [ADR-0010](./docs/adr/0010-embedded-documentation.md).
- **Agent skill** — the Claude Code skill for agents using `dsp` is **not** in this repo. It lives in [`dasch-claude-plugins`](https://github.com/dasch-swiss/dasch-claude-plugins) as `misc:dsp-cli` (plugin marketplace); removed here 2026-07-24 (see [ADR-0011](./docs/adr/0011-distribution-and-discoverability.md)).
- `.claude/conventions/` — project-specific reviewer criteria. Added lazily, as concrete reviewer workflows demand them.
  Currently absent by design (see [`docs/dev/ai-agent-setup.md`](./docs/dev/ai-agent-setup.md)).

## When in doubt

- A change that contradicts an ADR is a red flag. Either the change is wrong, or the ADR needs amendment in the same PR.
- A new domain noun or verb that isn't in `CONTEXT.md` is a red flag. Pause, define it, add it, then continue.
- Output shape changed? Run `cargo insta review` and accept the diff in the PR.
- Errors must carry a stable `kind` and a helpful message. Stringly-typed errors are not acceptable.

## Git workflow notes (personal-phase)

- Never commit directly to `main`. Feature branches always.
- Draft PRs, author assigned to themselves.
- Conventional Commits. CHANGELOG `[Unreleased]` updated for user-visible changes.
- See [`docs/dev/git-workflow.md`](./docs/dev/git-workflow.md) for stacking conventions and merge style.
