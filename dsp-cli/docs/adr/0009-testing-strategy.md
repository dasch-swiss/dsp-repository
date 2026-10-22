# Tests are mock-seam first, snapshot-guarded, and run live against a pinned stack

`dsp-cli`'s test suite has five layers, each addressing a distinct class of failure. The primary seam is the `DspClient` trait (dsp-cli/ADR-0008); the dominant test mode is action-level tests with mocks and snapshotted output. Live tests against a real DSP stack are `#[ignore]`d in the default suite and run strict in a dedicated CI job against a pinned, containerized stack. The live-test environment variables and the drift job are described in `docs/src/dsp-cli/testing-strategy.md`.

## Layers

1. **Unit tests**: inline `#[cfg(test)]` modules for pure functions (parsing helpers, filter logic, value-type mapping, config resolution).
2. **Action-level tests**: a `MockDspClient` and a capturing renderer, per the `DspClient` seam of dsp-cli/ADR-0008. The bulk of the suite: fast, deterministic, easy to write.
3. **Snapshot tests**: the `insta` crate. One snapshot per (noun, verb, format) cell with meaningful output. They capture prose, json, csv, tsv and lines exactly and are the default way to verify output shape.
4. **Wiremock client tests**: small and focused on HTTP and serialization correctness: request shape, header propagation, response deserialization, retry and error paths.
5. **Live tests**: behind the `live` cargo feature, against a DSP stack chosen through environment variables.

Layers 1 to 4 are pure-local and run on every PR. Layer 5 runs in the drift job.

### Where the tests live

Integration tests are flat files directly in `dsp-cli/tests/`, one binary each; there are no per-layer subdirectories.

- **Layers 1 and 2** live in `src/`, as inline `#[cfg(test)]` modules. Action-level tests sit in each `src/actions/**` file, each with its own local
  `MockDspClient`, because `tests/support/` is not reachable from `src/`.
- **Layer 3** is `tests/*_snapshots.rs`, one file per noun and verb (plus `auth_snapshots.rs`, `dump_progress_snapshots.rs` and
  `output_format_flags_snapshots.rs`). `tests/snapshots/` holds only the `insta` `.snap` files.
- **Layer 4** is `tests/*_http.rs`, one file per noun and verb (plus `login_http.rs`, `set_token_http.rs` and `project_lookup_http.rs`), and
  `tests/update_check.rs`. Most fixtures are inline `serde_json::json!` bodies; `tests/fixtures/` holds the few kept as files.
- **Layer 5** is `tests/live_*.rs`, behind the `live` feature and `#[ignore]`d.
- **Binary-level tests** run the built `dsp` through `assert_cmd` in the default suite: `tests/cli.rs` (the help snapshots, usage errors, the JSON error
  envelope), `tests/docs.rs` and `tests/stdout_broken_pipe.rs`.

Two shared modules are included with `mod`: `tests/support/mod.rs` (a `MockDspClient` and the `SharedBuf` output capture, for snapshot and binary-level
tests) and `tests/common/mod.rs` (`require_env` and `optional_env`, for live tests, with the `DSP_LIVE_STRICT` behaviour).

## Decisions

### Prose snapshot testing via `insta`

Prose is the default output format. Without snapshots it degrades silently: a stray newline, a typo in a template or a misformatted footer passes everything except a human's eye. `insta` makes the diff-and-review loop fast (`cargo insta review`). The cost is that every PR that changes user-facing output accepts the snapshot diff, which is the intended friction.

A command with no `Renderer` and no output format has no (noun, verb, format) snapshot cells: `dsp docs`, `dsp auth token` and `dsp vre sparql query`. The `--help` snapshots are not exempt, because help text is dsp-cli's contract regardless of a command's output shape. Byte-exactness of a relayed SPARQL response body is asserted directly in `tests/sparql_query_http.rs` and the action-layer tests, not snapshotted: the body is store-authored and store-versioned, so it is not dsp-cli's contract to pin.

### Hybrid fixtures for wiremock tests

- **Hand-written** fixtures for the canonical happy paths: what dsp-cli deserializes into.
- **Recorded** fixtures from real DSP-API responses for cases where the API surprised us. These become regression tests.

Hand-written fixtures alone drift silently from real API shapes; recorded fixtures alone require a live DSP environment for every new test scenario. `tests/fixtures/` is committed, so recorded fixtures are versioned with the code they regress against.

### Live tests: ignored by default, strict against a pinned stack in CI

- Every live test is `#[ignore]`d, so a plain `cargo test` or `cargo nextest run` never makes a live call. `.github/scripts/check-live-tests-ignored.sh` enforces this and is run by `just check`.
- A developer can run them against any DSP environment (for example `dev`) by setting the environment variables each live-test file documents.
- The drift-detection CI job (`.github/workflows/dsp-cli-drift.yml`) runs the live suite strict (`DSP_LIVE_STRICT=1`) against a pinned, containerized DSP stack (`dsp-cli/ci/stack/`: plain `db` and `api` services, no sipi, no ingest). The pin lives in `dsp-cli/ci/stack/stack.env` and is this repository's own choice, not a dsp-api release signal.
- The same stack is stood up locally by `just dsp-cli-stack-up`, `just dsp-cli-stack-fixtures` and `just dsp-cli-stack-down`.

The "real DSP" failure mode, most notably JSON-LD key drift in DSP-API response bodies that an OpenAPI surface diff cannot see, is therefore caught in CI and does not depend on developer discipline alone.

## Considered Options

- **Structural assertions on prose output**: rejected. They are brittle on whitespace, weak on layout regressions, and miss the most common breakage modes.
- **Hand-written fixtures only**: rejected. They drift from the real API.
- **Recorded fixtures only**: rejected. They require a live environment for every new test scenario and couple test writing to environment availability.
- **Live tests in CI against a shared, always-on environment**: rejected. They flake, need CI credentials and couple CI to that environment's uptime. The pinned stack is disposable, which is why it is not this option.
- **No live tests**: rejected. Drift would only be caught in production, which is the wrong feedback loop.
- **A locally spun-up stack as test infrastructure** (chosen): the drift job's stack. It is not a runtime dependency on dsp-tools and so does not violate dsp-cli/ADR-0004. The cost is Docker as a test dependency and container startup time.

## Consequences

- `insta` is a permanent dev-dependency, and `tests/fixtures/` is part of the source tree.
- The default suite stays cheap to run: layers 1 to 4 need no network and no Docker.
- If dsp-cli grows write operations, destructive live tests run only against the disposable pinned stack, never against a shared environment.

Enforced by: `.github/scripts/check-live-tests-ignored.sh`, run by `just check`, for live tests staying out of the default suite, and `insta`, which fails
`just test` on any unaccepted snapshot diff (**static-analysis**); the drift job in CI; one snapshot per `(noun, verb, format)` cell, and the fixture split,
by **review**.
