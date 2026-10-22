# dsp-cli Architecture

`dsp-cli` is an AI-agent-friendly command-line interface for the DaSCH Service Platform (DSP). It
translates DSP-API's RDF/JSON-LD surface into researcher vocabulary — **data-model**,
**resource-type**, **field**, **value** — instead of DSP-API's own `ontology` / `class` /
`property` / `Value` subclass terms. It is a root peer of the areas: a single crate, depending on
no workspace crate, that reaches DSP-API purely over HTTP.

## Layers

`dsp-cli` is organised as five layers inside a single crate that exposes both a library
(`src/lib.rs`) and a binary (`src/main.rs`). Action functions depend on traits (`DspClient`,
`Renderer`) rather than concrete implementations, so neither HTTP nor output format is hard-coded
into business logic.

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
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic>
```

This forces two properties:

- **HTTP-free action tests.** Inject a `MockDspClient` and a capturing renderer; assert against
  the captured calls. Fast and deterministic.
- **Format-free action logic.** Adding a new output format is a single `Renderer` impl; no action
  code changes.

`src/model/` (layer 4) holds translated domain types only. Transport-shaped relay types — where
the shape is chosen by the server or the store, not by dsp-cli's translation, such as the raw
SPARQL passthrough response — live in `src/client/` (layer 3a) instead.

## Test seam

- **Primary: mock at the `DspClient` trait.** Action-level tests construct a `MockDspClient` with
  canned responses and assert against the renderer's output or captured calls. These tests are the
  vast majority of the suite, run in milliseconds, and stay focused on business logic.
- **Secondary: wiremock integration tests.** A small dedicated set of tests covers HTTP-level
  concerns: request shape, header propagation, deserialization of real DSP-API response payloads,
  retry/error behaviour. Slower; few in number; not where verb logic gets validated.

See [Testing Strategy](./testing-strategy.md) for the full layer breakdown.

## Crate layout

```
dsp-cli/
├── Cargo.toml
├── src/
│   ├── main.rs               # binary entry: parse → dispatch
│   ├── lib.rs                # library re-exports and command dispatch
│   ├── diagnostic.rs         # Diagnostic: error kind, exit category, message (dsp-cli/ADR-0012)
│   ├── cli/                  # clap derive structs (layer 1)
│   ├── actions/              # layer 2
│   │   ├── auth/             # login, logout, set-token, status, token
│   │   ├── docs.rs           # dsp docs (embedded topics)
│   │   └── vre/              # project, data_model, resource_type, resource, vocabulary, sparql
│   ├── client/               # layer 3a — DspClient trait + HTTP impl
│   ├── render/               # layer 3b — Renderer trait + format impls
│   ├── model/                # layer 4 — domain types
│   ├── config/               # layer 5 — config stack
│   ├── update/               # the update check (dsp-cli/ADR-0015)
│   └── util/                 # layer-neutral helpers (imports from no other layer; all layers may import it)
│       └── text.rs           # html_to_text, strip_control_chars (dependency-free text processing)
└── tests/                    # wiremock integration tests + cli-level assert_cmd tests
```

The cross-layer dependency rule: **client and render are sibling layers; neither may import the
other; both may import util.**

Single crate keeps build and test cycles simple. The library/binary split lets `cargo test`
exercise everything except the very thin `main.rs`. The directory shape mirrors the noun-group
hierarchy in dsp-cli/ADR-0002, so navigating "where does `dsp vre data-model describe` live" is
mechanical: `src/actions/vre/data_model.rs`.

## Renderer trait granularity

Explicit per-(noun, shape) methods, not a generic `render<T: Renderable>`:

```rust
trait Renderer {
    fn diagnostic(&mut self, diag: &Diagnostic, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic>;
    fn resource_type_describe(&mut self, detail: &ResourceTypeDetail, meta: &MetaContext) -> Result<(), Diagnostic>;
    // ... one method per (noun, shape), plus the auth and dump outcomes
}
```

List methods take an owned per-noun view struct (e.g. `ProjectListView`) carrying the post-filter
items, the pre-filter total, and the filter string, rather than a bare slice — this lets prose
render a "(m of n matching …)" count line without the action layer duplicating that logic. The
`_meta` argument (`MetaContext`: server label, auth state, and the optional notes `filter_warning`,
`count_caveat` and `count_cost`) threads through
every renderer call; this is the implementation site of dsp-cli/ADR-0007's auth-state disclosure
requirement.

Prose rendering is irreducibly per-noun (each entity has its own natural-language phrasing).
Generic rendering via a trait object would force every format to read the same data through the
same lens, which is exactly wrong for prose. The slight cost (more trait methods as the surface
grows) is bounded by the fact that the noun-group set is small and grows slowly.

## Considered alternatives

- **Binary-only single crate.** Rejected — actions only testable via end-to-end `assert_cmd`,
  which is slow and friction-heavy for the dominant test mode.
- **Several crates.** dsp-cli remains a single crate; a split into `dsp-client`, `dsp-render` and
  `dsp-cli` is a deferred option in dsp-cli/ADR-0008.
- **Wiremock-only test seam.** Rejected — action-level tests become 10-100x slower; edge cases
  (error paths, partial data) harder to construct.
- **Generic `render<T>` trait.** Rejected — wrong shape for prose.
- **Action functions that own HTTP directly** (no client trait). Rejected — eliminates the fast
  test seam.

## Consequences

- `DspClient` is a trait. The trait's surface co-evolves with the action layer: every
  new verb that needs a new endpoint adds a method to the trait.
- `Renderer` is a trait with explicit per-noun methods. New noun-groups add new methods; each
  format impl gets a new method.
- Async/tokio is not required. `reqwest::blocking` works fine for sequential CLI calls. Sync
  execution + trait-object dispatch keeps compile times reasonable and stack traces readable.

See [dsp-cli/ADR-0008](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0008-internal-architecture.md)
for the full rationale and rejected alternatives, and [dsp-cli/ADR-0001](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0001-vocabulary-divergence.md)
for the vocabulary-translation boundary.
