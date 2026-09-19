# Internal architecture: layers, test seam, crate layout

`dsp-cli` is organised as a five-layer architecture inside a single crate that exposes both a library (`src/lib.rs`) and a binary (`src/main.rs`).
Action functions depend on traits (`DspClient`, `Renderer`) rather than concrete implementations, so neither HTTP nor output format is hard-coded into business logic.
The primary testing seam is the client trait, with sparse `wiremock`-style integration tests covering serialization.

## Layers

```
1. CLI parsing (clap derive)            — pure: produces typed arg structs
2. Action layer (per noun-group)        — orchestrates client + renderer
3a. DSP client trait + HTTP impl        — talks to DSP-API
3b. Renderer trait + format impls       — prose, json, csv, tsv, lines
4. Domain models                        — Project, DataModel, ResourceType, Field, Value, ValueType
5. Config resolution                    — flag → env var → .env → cached token (dsp-cli/ADR-0007)
```

The action layer is the keystone. Each action function signature is roughly:

```rust
pub fn <verb>(
    args: &<Verb><Noun>Args,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &dyn Renderer,
) -> Result<()>
```

This forces two properties:

- **HTTP-free action tests.** Inject a `MockDspClient` and a capturing renderer; assert against the captured calls. Fast and deterministic.
- **Format-free action logic.** Adding a new output format is a single `Renderer` impl; no action code changes.

## Test seam

- **Primary: T2 — mock at the `DspClient` trait.** Action-level tests construct a `MockDspClient` with canned responses and assert against the renderer's output
  (or against the renderer's captured calls).
  These tests are the vast majority of the test suite, run in milliseconds, and stay focused on business logic.
- **Secondary: T1 — wiremock integration tests.** A small dedicated set of tests covers HTTP-level concerns: request shape, header propagation,
  deserialization of real DSP-API response payloads, retry/error behaviour.
  Slower; few in number; not where verb logic gets validated.

## Crate layout (L2)

```
dsp-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # binary entry: parse → dispatch
│   ├── lib.rs               # library re-exports
│   ├── cli/                 # clap derive structs (layer 1)
│   ├── actions/             # layer 2
│   │   └── vre/
│   │       ├── project.rs
│   │       ├── data_model.rs
│   │       └── resource_type.rs
│   ├── client/              # layer 3a — trait + HTTP impl
│   ├── render/              # layer 3b — Renderer trait + format impls
│   ├── model/               # layer 4 — domain types
│   ├── util/                # layer-neutral helpers (imports from no other layer; all layers may import it)
│   │   └── text.rs          # html_to_text, strip_control_chars (dependency-free text processing)
│   └── config/              # layer 5 — config stack
└── tests/                   # wiremock integration tests + cli-level assert_cmd tests
```

> **Amendment (2026-06-17, plan 024 / Phase 8c):** `src/util/` was added as a layer-neutral home for dependency-free helpers.
> It imports from no other crate-internal layer; any layer may import it freely.
> The immediate resident is `src/util/text.rs` (`html_to_text`, `strip_control_chars`), relocated from `src/render/html.rs` to eliminate the client→render layering
> violation that arose when value parsing (in `src/client/`) needed `html_to_text`.
> `src/render/html.rs` was removed entirely (no re-export shim — a shim would leave the violation reachable).
> The cross-layer dependency rule is therefore: **client and render are sibling layers; neither may import the other; both may import util.**

Single crate keeps build and test cycles simple. The library/binary split lets `cargo test` exercise everything except the very thin `main.rs`.
The directory shape mirrors the noun-group hierarchy in dsp-cli/ADR-0002, so navigating "where does `dsp vre data-model describe` live" is mechanical: `src/actions/vre/data_model.rs`.

## Renderer trait granularity

Explicit per-(noun, shape) methods, not a generic `render<T: Renderable>`:

```rust
trait Renderer {
    fn projects(&mut self, items: &[Project], meta: &MetaContext) -> Result<()>;
    fn project_detail(&mut self, item: &Project, data_models: &[DataModelSummary], meta: &MetaContext) -> Result<()>;
    fn data_models(&mut self, items: &[DataModel], meta: &MetaContext) -> Result<()>;
    fn data_model_detail(&mut self, item: &DataModel, resource_types: &[ResourceTypeSummary], meta: &MetaContext) -> Result<()>;
    fn resource_types(&mut self, items: &[ResourceType], meta: &MetaContext) -> Result<()>;
    fn resource_type_detail(&mut self, item: &ResourceType, meta: &MetaContext) -> Result<()>;
    // ...
}
```

> **Implementation note (Phase 4):** the real `Renderer::projects` signature takes `view: &ProjectListView` (an owned per-noun view struct carrying the
> post-filter items, the pre-filter total, and the filter string) rather than the `items: &[Project]` shown above.
> The sketch is illustrative — the view-struct pattern lets prose render a "(m of n matching …)" count line without the action duplicating that logic.
> Future list methods will follow the same `<Noun>ListView` pattern.
> The `_meta` argument stays separate as `meta: &MetaContext` per the sketch.

Prose rendering is irreducibly per-noun (each entity has its own natural-language phrasing).
Generic rendering via a trait object would force every format to read the same data through the same lens, which is exactly wrong for prose.
The slight cost (more trait methods as the surface grows) is bounded by the fact that the noun-group set is small and grows slowly.

## Considered alternatives

- **L1 (binary-only single crate).** Rejected — actions only testable via end-to-end `assert_cmd`, which is slow and friction-heavy for the dominant test mode.
- **L3 (Cargo workspace).** Rejected for now — adds ceremony without benefit at personal-project scale.
  The migration to L3 is mechanical when dsp-cli moves into the dsp-repository monorepo (dsp-cli/ADR-0005): split modules into crates, declare a workspace.
- **T1-only test seam (wiremock everywhere).** Rejected — action-level tests become 10–100× slower; edge cases (error paths, partial data) harder to construct.
- **Generic `render<T>` trait.** Rejected — wrong shape for prose.
- **Action functions that own HTTP directly** (no client trait). Rejected — eliminates the fast test seam.

## Consequences

- `DspClient` is a trait from day one. The trait's surface co-evolves with the action layer: every new verb that needs a new endpoint adds a method to the trait.
- `Renderer` is a trait with explicit per-noun methods. New noun-groups add new methods; each format impl gets a new method.
- The `MetaContext` struct (server label, auth state, optional filter warning) threads through every renderer call.
  This is the implementation site of dsp-cli/ADR-0007's auth-state disclosure requirement.
- Async/tokio is not required. `reqwest::blocking` or `ureq` works fine for sequential CLI calls.
  (Specific HTTP client choice deferred to implementation; both options preserve this architecture.)
- Sync execution + trait-object dispatch keeps compile times reasonable and stack traces readable.
- When the migration to dsp-repository happens, the natural split is: `dsp-client` (model + client trait + HTTP impl),
  `dsp-render` (model is shared via dsp-client, renderer trait + impls live here), `dsp-cli` (clap + actions + main).
  The current layout makes that split a directory-rename operation. The move is now a recorded decision — see dsp-cli/ADR-0014 (PROJECT_PLAN Phase 11).

## Amendment (2026-08-07, plan 035)

### Layer 4 vs layer 3a: translated domain types vs transport-shaped relay types

This ADR's layer-4 contract (`src/model/`) has always held translated domain types only, but never
said so in words. `dsp vre sparql query` ([dsp-cli/ADR-0016](0016-sparql-passthrough.md)) is the first method
to return something else — `SparqlResponse` (an HTTP status, a MIME string, and raw bytes) — and it
lives in `src/client/sparql.rs`, layer 3a, not `src/model/`. Stated explicitly so the next
`src/client/`-vs-`src/model/` call has a rule to apply rather than having to re-derive it from a plan:

> `src/model/` (layer 4) holds translated domain types only. Transport-shaped relay types — where the
> shape is chosen by the server or the store, not by dsp-cli's translation — live in `src/client/`
> (layer 3a).
