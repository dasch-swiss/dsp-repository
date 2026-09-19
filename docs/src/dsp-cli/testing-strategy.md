# dsp-cli Testing Strategy

`dsp-cli`'s test suite has five layers, each addressing a distinct class of failure. The primary
seam is the `DspClient` trait (see [Architecture](./architecture.md)); the dominant test mode is
action-level tests with mocks and snapshotted output. Live tests against a real DSP environment are
opt-in and not part of the default suite; a dedicated CI job now runs them against a pinned,
containerized stack (see [Drift detection in CI](#drift-detection-in-ci) below).

## Layers

1. **Unit tests** — inline `#[cfg(test)]` modules for pure functions (parsing helpers, filter
   logic, value-type mapping, config resolution).
2. **Action-level tests** — mock the `DspClient` trait and a capturing renderer, asserting against
   the captured calls or snapshotted output. The bulk of the suite. Fast, deterministic, easy to
   write. Mocks are local to each action file rather than shared — see `dsp-cli/CLAUDE.md`'s
   non-obvious constraints for why.
3. **Snapshot tests** — via the `insta` crate, in `tests/`. One snapshot per (noun, verb, format)
   cell with meaningful output. Captures prose / json / csv / tsv / lines exactly. The default way
   to verify output shape; run `just dsp-cli-snap-review` (`cargo insta review`) to accept or
   reject a diff.
4. **Wiremock client tests** — small, focused on HTTP/serialization correctness: request shape,
   header propagation, response deserialization, retry/error paths. Fixtures are hybrid:
   hand-written for the "what we care about" happy paths, recorded from real DSP-API responses for
   regression cases where dsp-cli's view was wrong.
5. **Live tests** — `tests/live_*.rs`, behind the `live` cargo feature; not in the default suite.
   Hit a DSP environment: a developer-selected one locally (via env vars), or the drift job's pinned
   stack in CI. Smoke-check end-to-end integration before significant changes.

`just test` runs layers 1-4 (the cheap suite CI runs). `just dsp-cli-test-live` runs layer 5.

## Live tests are always `#[ignore]`d

Every `#[test]` in `dsp-cli/tests/live_*.rs` also carries `#[ignore]`, so a plain
`cargo test`/`cargo nextest run` never attempts a live call — the tests need a reachable DSP stack,
and without `#[ignore]` they would otherwise run and pass vacuously via their own early-return skip
under CI. `.github/scripts/check-live-tests-ignored.sh`, run by `just check`, enforces this: it
scans every `dsp-cli/tests/live_*.rs` file and fails if any `#[test]` lacks a matching `#[ignore]`.
Run live tests explicitly with `just dsp-cli-test-live`, which passes `--features live` and
`--ignored` (or the equivalent) so the ignored tests actually execute.

## Live-test environment variables

Live tests skip automatically when a required variable is unset — they never fail for missing
config, only report why they were skipped. Each live-test file documents its own required
variables; the common ones are:

| Variable | Required by | Meaning |
|---|---|---|
| `DSP_TEST_SERVER` | all live tests | URL or shortcut of the DSP server to run against (e.g. `dev`, `https://api.dev.dasch.swiss`). |
| `DSP_TEST_PROJECT` | project-dump and resource-list/describe live tests | Shortcode, shortname, or IRI of a small, disposable test project on `DSP_TEST_SERVER`. |
| `DSP_TEST_CLASS_IRI` | resource-list and resource-describe live tests | Full class IRI to list/describe resources for (used to derive a resource IRI to feed into describe). |
| `DSP_TOKEN` | tests that need auth | A pre-issued system-administrator JWT. When set, login is skipped entirely. Preferred for CI or any environment with a long-lived admin token. |
| `DSP_TEST_USER` | tests that need auth | Email of a system-administrator account (alternative to `DSP_TOKEN`). Used together with `DSP_TEST_PASSWORD` to call `dsp auth login` first and obtain a token. |
| `DSP_TEST_PASSWORD` | tests that need auth | Password for `DSP_TEST_USER`. Local/dev/test only — never a production password (same policy as `DSP_PASSWORD` below). |
| `DSP_TEST_NON_ADMIN_TOKEN` | the sparql-query non-admin-token live test | A non-SystemAdmin bearer token, used to verify that the SPARQL passthrough maps a non-admin caller to a 403 auth-required error. |
| `DSP_TEST_VOCAB_PROJECT` | `live_vocabulary.rs` | Shortcode or IRI of the project whose controlled vocabularies are inspected. Set to `0001` in the drift job. |

If `DSP_TOKEN` is set it takes precedence over the `DSP_TEST_USER`/`DSP_TEST_PASSWORD` pair. If
neither a token nor user+password is available, the live test skips with an `eprintln!` note
explaining what is missing. The token is never logged.

### Strict mode (`DSP_LIVE_STRICT`)

By default, a live test missing a required environment variable skips quietly (an `eprintln!` and
an early return) rather than failing — this keeps `just test` and any offline run from breaking
when live-test configuration simply isn't present. Setting `DSP_LIVE_STRICT=1` turns that into a
loud failure instead: `require_env` (`tests/common/mod.rs`) panics naming the missing variable
rather than returning `None`. Use it when you specifically intend to run the live suite and want a
misconfigured environment to fail fast instead of silently skipping every test. `optional_env`
(also in `tests/common/mod.rs`) is unaffected by strict mode — an optional variable stays optional
either way.

## Drift detection in CI

`.github/workflows/dsp-cli-drift.yml` runs the layer-5 live suite in CI, strict
(`DSP_LIVE_STRICT=1`), against a pinned, containerized dsp-api stack rather than a
developer-selected environment. This is the check for a class of drift an OpenAPI diff can't see:
dsp-cli deserializes DSP-API's JSON-LD response bodies, and a renamed or re-shaped key inside a
body is invisible to a schema diff of the endpoint surface. Running the real client against a real
server is what catches it.

The default suite (`just test`, layers 1-4) is unaffected — live tests stay `#[ignore]`d there
regardless of what runs in CI.

### Running the stack locally

The stack lives in `dsp-cli/ci/stack/` (`db` = Apache Jena Fuseki, `api` = knora-api; no sipi, no
ingest). Three just recipes drive it:

- `just dsp-cli-stack-up` — start the stack; this already loads fixtures as part of bringing it up.
- `just dsp-cli-stack-fixtures` — reload fixtures into an already-running stack.
- `just dsp-cli-stack-down` — tear it down.

Run `just dsp-cli-test-live` against it the same way you would against any other environment (see
the environment variables above).

### Bumping the pins

`dsp-cli/ci/stack/stack.env` holds two independent pins, `API` and `DB`, and both are this
repository's own choice, not a dsp-api release signal — neither is kept in lockstep with dsp-api's
own version. `API` sets the `knora-api` image tag and also the `dasch-swiss/dsp-api` tag
`load-fixtures.sh` checks the fixtures and ontologies out from; `DB` sets only the Fuseki image. To
bump either: edit `stack.env`, re-run the live suite locally against the new versions, and commit.

## General configuration variables

These are not live-test-only; they configure any `dsp` invocation and are also useful when
preparing a live-test environment. Copy them into a `.env` file in the working directory (gitignored,
never commit secrets — see [dsp-cli/ADR-0007](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0007-auth-and-environments.md)):

| Variable | Meaning |
|---|---|
| `DSP_SERVER` | Default server target (built-in shortcut or full URL). |
| `DSP_USER` | Default user (email, username, or IRI) for `dsp auth login`, so it need not be retyped. The identifier type is auto-detected from the value. |
| `DSP_PASSWORD` | Password for non-interactive `dsp auth login`. **Local/dev/test setups only — never a production password.** This is a durable plaintext master credential on disk; for real environments prefer `DSP_TOKEN` (scoped and expiring) over `DSP_PASSWORD`. |
| `DSP_TOKEN` | Pre-provisioned auth token. Overrides the cached token from `dsp auth login`. Useful in CI and pre-provisioned agent environments; `dsp auth status` reports when it is in effect and shows its expiry, if readable. |
| `RUST_LOG` | Logging override. Bypasses the `-v` flag. See [dsp-cli/ADR-0012](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0012-diagnostics.md). |
| `DSP_NO_UPDATE_CHECK` | Disables the interactive update-check advisory (a crates.io version check that only fires on prose-format, interactive-TTY runs). Irrelevant to agents/scripts. |

## Coverage expectations

- Every `pub fn` in `src/actions/` has an action-level test.
- Every (noun, verb, format) cell that produces meaningful output has a snapshot. Three commands
  are exempt because they have no `Renderer` and no output format to begin with: `dsp docs`,
  `dsp auth token`, and `dsp vre sparql query` — CLI-help snapshots are **not** exempt from this,
  since `--help` text is part of the contract regardless of a leaf's output shape.
- Every wire-format change (request body, response deserialisation) has at least one wiremock
  test.
- Live tests cover the read paths against a real DSP environment (smoke-only).

## See also

- [Architecture](./architecture.md) — layers and the `DspClient`/`Renderer` test seam.
- [dsp-cli/ADR-0009](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0009-testing-strategy.md) — full rationale and rejected alternatives.
- [dsp-cli/ADR-0008](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/0008-internal-architecture.md) — the test seam (`DspClient` trait + `Renderer` trait).
