# Testing strategy

`dsp-cli`'s test suite has four layers, each addressing a distinct class of failure. The primary seam is the `DspClient` trait (per ADR-0008);
the dominant test mode is action-level tests with mocks and snapshotted output.
Live tests against a real DSP environment are opt-in and not in CI for v1.

## Layers

1. **Unit tests** — inline `#[cfg(test)]` modules for pure functions (parsing helpers, filter logic, value-type mapping, config resolution).
2. **Action-level tests** — `tests/actions/` using a `MockDspClient` and a capturing renderer. The T2 seam from ADR-0008.
   The bulk of the suite. Fast, deterministic, easy to write.
3. **Snapshot tests** — `tests/snapshots/` via the `insta` crate. One snapshot per (noun, verb, format) cell with meaningful output.
   Captures prose / json / csv / tsv / lines exactly. Default for verifying output shape.
4. **Wiremock client tests** — `tests/client/` (T1 seam from ADR-0008). Small, focused on HTTP/serialization correctness: request shape,
   header propagation, response deserialization, retry/error paths. Fixtures are hybrid: hand-written for the "what we care about" happy paths;
   recorded from real DSP-API responses for regression cases where dsp-cli's view was wrong.
5. **Live tests** — `tests/live/` behind a `live` cargo feature; not in CI. Hit an existing DSP environment (developer-selected via env vars).
   Smoke-check end-to-end integration before significant changes.

## Decisions per axis

### (A1) Prose snapshot testing via `insta`

Prose is the default output format. Without snapshots, it degrades silently: a stray newline, a typo in a template, a misformatted footer all pass everything except a human's eye.
`insta` makes the diff-and-review loop fast (`cargo insta review`). Cost: snapshots must be reviewed on every output-shape change. Acceptable.

### (B3) Hybrid fixtures for wiremock tests

- **Hand-written** for canonical happy paths — what we deserialize *into*.
- **Recorded** from real DSP-API responses for cases where the API surprised us. These become regression tests.

This avoids both extremes: hand-written-only drifts silently from real API shapes; recorded-only requires a live DSP environment for every new test scenario.

### (C2) Opt-in live tests, not in CI

A small set of tests gated behind `--features live`. The developer runs them locally against an existing DSP environment
(e.g. `test.dasch.swiss`) before merging meaningful changes.
CI runs only the offline layers (1–4).

Right trade for a personal exploratory project: drift detection when needed, no commitment to credentialed CI infrastructure or shared-environment uptime.

## Considered alternatives

- **(A2) Structural assertions on prose output.** Rejected — brittle on whitespace; weak on layout regressions; doesn't catch the most common breakage modes.
- **(B1) Hand-written fixtures only.** Rejected — drifts from real API.
- **(B2) Recorded fixtures only.** Rejected — requires a live env for every new test scenario; couples test writing to environment availability.
- **(C1) Live tests in CI, always.** Rejected — flakes; requires CI credentials; couples CI to shared environment uptime;
  the drift-detection value at personal-project scale doesn't justify the infrastructure burden.
- **(C3) No live tests, ever.** Rejected — drift only caught in production, which is the wrong feedback loop.
- **(C4) Locally-spun-up DSP stack as test infrastructure** (using `dsp-tools start-stack` + `dsp-tools create` as subprocess fixtures;
  this is *not* a runtime dependency on dsp-tools and therefore not a violation of ADR-0004). **Deferred, not rejected.**
  The cost (Docker as test dep, 10–30s container startup, coupling to dsp-tools' stack-management stability) is not justified by v1's read-only surface,
  where C2 catches the same drift more cheaply.
  Triggers to reconsider C4 in the future: needing destructive write/delete tests, wanting CI without credentials, or test isolation from a shared environment.

## Consequences

- `insta` becomes a permanent dev-dependency. Every PR that changes user-facing output must accept the snapshot diff. This is exactly the right friction.
- `tests/fixtures/` is committed to the repo. Recorded fixtures are versioned with the code they regress against.
- The `live` cargo feature is the documented way to run integration tests against real DSP. `tests/live/` documents which env vars it expects
  (e.g. `DSP_TEST_SERVER`, `DSP_TEST_USER`).
- Test suite stays cheap to run by default (layers 1–4 are pure-local). CI runs only the cheap suite.
  Coverage of the "real DSP" failure mode is a developer-discipline concern, not a CI gate.
- If at some point dsp-cli grows write operations, **revisit C4** — destructive tests against shared environments are a bad idea,
  and that's the natural moment to invest in stack-managed e2e.
- "Open a can of worms" remains accurate for now; this ADR is the answer to "shouldn't we have spin-up-stack e2e tests?" — yes, eventually,
  but not until the cost is justified by the test scenarios.

## Amendment (2026-08-07, plan 035)

### The no-`Renderer` carve-out has no `(noun, verb, format)` snapshot cells

This ADR states prose snapshots as the unconditional regression detector for every `(noun, verb,
format)` cell. Three commands are exceptions, because they have no format and no `Renderer` to begin
with: `dsp docs`, `dsp auth token`, and now `dsp vre sparql query`
([ADR-0016](0016-sparql-passthrough.md)). Naming the carve-out explicitly, rather than leaving it as
an implicit gap in the "unconditional" language above (the same undocumented-exception problem
ADR-0007's amendment above fixes for auth-state disclosure):

> A command with no `Renderer` and no output format has no `(noun, verb, format)` snapshot cells.

⚠️ The **CLI-help** snapshots (`tests/snapshots/cli__help_*.snap`) are **not** exempt — `--help` text
is dsp-cli's contract regardless of a leaf's output shape, and `dsp vre sparql query`'s help snapshot
(`cli__help_vre_sparql_query.snap`) is tested exactly like every other leaf's. Byte-exactness of a
relayed SPARQL response body is asserted directly in `tests/sparql_query_http.rs` and the action-layer
unit tests, not snapshotted — the body is store-authored and store-versioned, not dsp-cli's contract
to pin.
