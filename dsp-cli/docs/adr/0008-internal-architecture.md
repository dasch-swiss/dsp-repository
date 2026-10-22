# Internal architecture: one crate of five layers, with actions depending on traits

`dsp-cli` is organised as a single crate that exposes both a library (`src/lib.rs`) and a binary (`src/main.rs`), in five layers.
Action functions depend on traits (`DspClient`, `Renderer`) rather than concrete implementations, so neither HTTP nor output format is hard-coded into business logic.
The primary testing seam is the client trait, with sparse `wiremock` integration tests covering serialization.
The crate layout, the layer diagram and the `Renderer` method list are in [Architecture](../../../docs/src/dsp-cli/architecture.md).

## Layers

1. CLI parsing (clap derive, `src/cli/`) — pure: produces typed arg structs.
2. Action layer (`src/actions/`, per noun-group) — orchestrates client and renderer.
3. 3a. `DspClient` trait and its HTTP implementation (`src/client/`); 3b. `Renderer` trait and its format implementations (`src/render/`).
4. Domain models (`src/model/`).
5. Config resolution (`src/config/`): flag → env var → `.env` → cached token ([dsp-cli/ADR-0007](0007-auth-and-environments.md)).

The directory shape under `src/` mirrors the layers and the noun-group hierarchy of dsp-cli/ADR-0002, so "where does `dsp vre data-model describe` live" is mechanical:
`src/actions/vre/data_model.rs`. The library/binary split lets `cargo test` exercise everything except the very thin `main.rs`.

The action layer is the keystone. Every action function has the uniform shape

```rust
pub fn <verb>(args: &<Verb><Noun>Args, cfg: &Config, client: &dyn DspClient, renderer: &mut dyn Renderer) -> Result<(), Diagnostic>
```

so action tests are HTTP-free (inject a `MockDspClient` and a capturing renderer) and action logic is format-free (a new output format is one `Renderer` impl, no action change).
The shape carries the full context by design, which is why some action signatures allow `clippy::too_many_arguments`; splitting them would break the uniformity.

## Dependency rules

- **Client and render are sibling layers; neither may import the other. Both may import `src/util/`.**
- `src/util/` is the layer-neutral home for dependency-free helpers (text processing such as `html_to_text` and the control-character helpers). It imports from no other layer.
- `src/model/` (layer 4) holds translated domain types only. Transport-shaped relay types — where the shape is chosen by the server or the store, not by dsp-cli's
  translation, such as `SparqlResponse` ([dsp-cli/ADR-0016](0016-sparql-passthrough.md)) — live in `src/client/` (layer 3a).
- Wire DTOs in `src/client/http.rs` declare a field without `#[serde(default)]` when the CLI depends on it, so a server-contract change fails the parse loudly
  (→ `ServerError`) instead of being papered over with a default. When the parse fails because a field was *removed upstream*, the field is removed from the DTO,
  from the domain model and from the rendered output. `#[serde(default)]` is reserved for fields that are genuinely optional in the contract.

## Test seam

- **Primary: mock at the `DspClient` trait.** Action-level tests construct a `MockDspClient` with canned responses and assert against the renderer's output
  (or its captured calls). These are the vast majority of the suite, run in milliseconds, and stay focused on business logic.
- **Secondary: `wiremock` integration tests.** A small dedicated set covers HTTP-level concerns: request shape, header propagation,
  deserialization of real DSP-API payloads, error behaviour. Slower; few in number; not where verb logic gets validated.

## Renderer granularity

`Renderer` has explicit per-(noun, shape) methods, not a generic `render<T: Renderable>`. Prose rendering is irreducibly per-noun (each entity has its own phrasing);
a generic render would force every format to read the data through the same lens, which is wrong for prose. List methods take an owned per-noun
`<Noun>ListView` carrying the post-filter items, the pre-filter total and the filter string, so prose can render a "(m of n matching …)" line without the action duplicating it.
A `MetaContext` (server label, auth state, optional filter warning and count caveats) threads through every renderer call; it is the implementation site of
dsp-cli/ADR-0007's auth-state disclosure. The cost, more trait methods as the surface grows, is bounded because the noun-group set is small and grows slowly.

`DspClient` is a trait with a method per endpoint: a new verb that needs a new endpoint adds a method. A new noun-group adds `Renderer` methods, and each format impl gets each new method.
Execution is synchronous (`reqwest::blocking`); async/tokio is not required for sequential CLI calls, and sync execution with trait-object dispatch keeps compile times
reasonable and stack traces readable.

## Considered Options

- **Binary-only single crate.** Rejected — actions only testable via end-to-end `assert_cmd`, which is slow and friction-heavy for the dominant test mode.
- **Cargo workspace of three crates**: `dsp-client` (model, client trait, HTTP impl), `dsp-render` (renderer trait and format impls, sharing the model through `dsp-client`)
  and `dsp-cli` (clap, actions, main). *Deferred*, not rejected: it adds ceremony without benefit for one consumer, and the layer layout makes it a directory-rename operation.
  It would be reconsidered if another crate in the workspace needs the client or the models.
- **Wiremock-only test seam.** Rejected — action-level tests become 10–100× slower; edge cases (error paths, partial data) harder to construct.
- **Generic `render<T>` trait.** Rejected — wrong shape for prose.
- **Action functions that own HTTP directly** (no client trait). Rejected — eliminates the fast test seam.
- **`#[serde(default)] status: bool` mapping an upstream-removed field to a fixed value.** Rejected: it invents data the server no longer has an opinion about,
  and would silently outlive the day the server reintroduces some other concept under the same key.

Enforced by: the fail-loudly DTO contract by the wiremock suites in `dsp-cli/tests/project_list_http.rs` and `dsp-cli/tests/project_describe_http.rs`
(**static-analysis**); the layers, their import rules and the action shape by **review**; the `DspClient` seam is exercised by every action test.
