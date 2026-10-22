# Diagnostics: errors, exit codes, and logging

`dsp-cli` partitions "anything that is not the requested data" into two channels: errors (the command failed, exit non-zero) and logs (the command is reporting on its work, exit code unaffected). Both go to **stderr**; stdout is reserved for the data the caller asked for. Exit codes are a small fixed set with stable meanings, so agents can branch on outcome without parsing prose. Structured output formats (`json`) carry errors in the same envelope as data, so a single parser handles both.

## Exit codes

| Code | Meaning | Examples |
|---|---|---|
| `0` | Success | Command ran, data emitted. |
| `1` | Runtime error | HTTP failure, network unreachable, server 5xx, malformed response, deserialization mismatch, I/O. |
| `2` | Usage error | Bad flag, missing required argument, unknown noun-group, unparseable identifier. This matches `clap`'s default for arg-parse failures, so it stays consistent across hand-written and clap-emitted errors. |
| `3` | Authentication required | Endpoint demands auth and none was provided, or the cached token was rejected (401/403). Distinct from `1` so an agent can decide whether to attempt `dsp auth login` before reporting failure. |

There are no other exit codes. A new category gets a new code and a rewrite of this record: silent expansion would break callers.

A write to stdout that fails with `ErrorKind::BrokenPipe` ends the process with exit `0` and no stderr output: a reader that stops reading (`dsp ... | head`) is the normal end of a pipeline. Every other write error becomes `Internal` and exits `1`. The check sits in a writer adapter wrapped around each stdout handle, where the handle is constructed (the renderers' sinks and the commands that write to stdout directly: `dsp docs`, `dsp auth token`, `dsp vre sparql query`). `Diagnostic::from(io::Error)` stays as it is, because it is also reached from non-stdout paths such as reading `auth.toml`, where a broken pipe is meaningless.

## Stdout vs stderr discipline

- **stdout** is reserved for the requested data: prose, `json`, `lines`, `csv`, `tsv`.
- **stderr** carries human-readable error messages (on failure), log output (always, when above the configured level), and the meta line for `lines` / `csv` / `tsv` output (dsp-cli/ADR-0007).
- A successful command writes nothing to stderr by default. A failed command writes nothing to stdout in the prose and line-based formats; structured formats are the exception (below).
- Pipes only see the data: `dsp ... -l | xargs ...` works without filtering meta lines or error noise.

A command whose payload can itself be a store-authored error document (`dsp vre sparql query`, dsp-cli/ADR-0016) keeps stdout data-only: on a non-2xx relay nothing is written to stdout, and the store's text goes to stderr. dsp-cli/ADR-0016 holds that command's status mapping.

## Error shape per format

Every top-level error goes through the selected format's `Renderer::diagnostic`. `anyhow` is not used; library code returns the `thiserror`-defined `Diagnostic`, which carries the `kind`, message and optional hint.

- **`prose`** (default): a human-readable error message to stderr, with an optional second line carrying a remediation hint:

  ```
  Error: data model 'biblio-onto' not found in project 'incunabula'
  Hint: list available data models with `dsp vre data-model list --project incunabula --server <s>`.
  ```

  ANSI colour is applied when stderr is a TTY and suppressed otherwise.

- **`json`**: the same `_meta` envelope as success, with an `error` object replacing `data`:

  ```json
  {
    "_meta": { "auth": "...", "server": "...", "exit_code": 3 },
    "error": {
      "kind": "auth_required",
      "message": "endpoint requires authentication",
      "hint": "run `dsp auth login --server dev` first"
    }
  }
  ```

  It is emitted to **stdout**, the same channel as the successful structured response, so a JSON-consuming caller has one parser and one stream. `_meta.exit_code` means JSON consumers do not need the process exit code, which they often cannot read. The `kind` field is a stable enum (below); `message` and `hint` are not stable.

- **`lines` / `csv` / `tsv`**: empty data on stdout (no headers emitted), and a human-readable error on stderr identical to prose. These formats are for shell pipelines that already key on the exit code; the structured-error envelope is overkill.

## Error kind enum (stable)

The `kind` field in JSON errors is the stable interface:

- `usage`: bad flag, missing argument, unknown identifier syntax.
- `auth_required`: the server demanded auth and none was supplied, or the token was rejected.
- `not_found`: the named project, data-model or resource-type does not exist.
- `server_error`: the server responded with a 5xx or an unparseable response.
- `network`: could not reach the server.
- `conflict`: a server-side resource is busy or already exists, for example a dump already in progress.
- `io`: a local filesystem operation the user requested failed, for example writing the dump file.
- `internal`: unexpected state in dsp-cli itself; to be reported as a bug.

`conflict` and `io` map to exit code `1`. Adding a kind is additive (consumers ignore unknown kinds); renaming one is a breaking change (a `CHANGELOG` entry, and it affects 0.x bumps). The kind enum is part of the public CLI contract (dsp-cli/ADR-0003), and snapshot tests cover representative kinds per format.

## Logging

`dsp-cli` uses the `tracing` crate with `tracing-subscriber`. Logging is for diagnostics during a command's execution, not for its primary output.

- **Default level**: `WARN`. A successful command produces no log output.
- **`-v` / `--verbose`** raises the level: `-v` is `INFO`, `-vv` is `DEBUG`, `-vvv` is `TRACE`. It is a single repeated flag, not separate flags per level.
- **`RUST_LOG`** overrides the flag entirely when set. This is the escape hatch for targeted debugging (`RUST_LOG=dsp_cli::client=trace`).
- All log output goes to **stderr**, as compact human-readable text with a level prefix.
- Logging is initialised once, in `main.rs`, before any command logic runs. Library callers can install their own subscriber.

JSON-structured logs, OpenTelemetry and structured log shipping are not provided. If a use case for log aggregation emerges, it needs its own record. The repository's observability conventions for services do not apply to `dsp-cli`, a short-lived process, unless it ever grows long-running surfaces.

## Considered Options

- **Two-code model (`0` / `1`)**: rejected. Agents lose the ability to distinguish "I should retry after login" from "the server is unreachable" without parsing prose. The cost of two extra codes is negligible.
- **Five-code model with a separate `not_found`**: rejected. `not_found` is recoverable by the same UX as a usage error (list the available identifiers), and merging it with runtime `1` would lose meaning. Putting it under `1` and surfacing the distinction through the JSON `kind` is enough.
- **`stderr` for structured errors too**: rejected. It splits the channel an automated caller has to read, which defeats the point of the envelope.
- **`anyhow` everywhere with stringified errors**: rejected. It loses the `kind` enum. `thiserror` types are used in library code and converted at the binary boundary.
- **A `--quiet` flag**: deferred, not rejected. The default is already terse (no log output on success); it is added if real noise emerges.

Enforced by: the unit tests in `dsp-cli/src/diagnostic.rs` (kind to exit-code mapping), `dsp-cli/tests/cli.rs` (usage errors exit `2`, the JSON error
envelope on stdout), `dsp-cli/tests/stdout_broken_pipe.rs`, `dsp-cli/tests/sparql_query_http.rs` (status mapping), and the per-format `insta` snapshots, run by
`just test` (**static-analysis**); the closed set of exit codes, the stdout and stderr discipline and the logging defaults, by **review**.
