# Diagnostics: errors, exit codes, and logging

`dsp-cli` partitions "anything that is not the requested data" into two channels: errors (the command failed, exit non-zero) and logs (the command is reporting on its work,
exit code unaffected).
Both go to **stderr**; stdout is reserved for the data the caller asked for.
Exit codes are a small fixed set with stable meanings, so agents can branch on outcome without parsing prose.
Structured output formats (`json`) carry errors in the same envelope as data, so a single parser handles both.

## Exit codes

| Code | Meaning | Examples |
|---|---|---|
| `0` | Success | Command ran, data emitted. |
| `1` | Runtime error | HTTP failure, network unreachable, server 5xx, malformed response, deserialization mismatch, I/O. |
| `2` | Usage error | Bad flag, missing required argument, unknown noun-group, unparseable identifier. This matches `clap`'s default for arg-parse failures, so it stays consistent across hand-written and clap-emitted errors. |
| `3` | Authentication required | Endpoint demands auth and none was provided, or the cached token was rejected (401/403). Distinct from `1` so an agent can decide whether to attempt `dsp auth login` before reporting failure. |

No other exit codes in v1. If a new category appears, it gets a code and an ADR amendment — silent expansion would break callers.

## Stdout vs stderr discipline

- **stdout** is reserved for the requested data: prose, `json`, `lines`, `csv`, `tsv`.
- **stderr** carries: human-readable error messages (on failure), log output (always, when above the configured level),
  and the meta line for `lines` / `csv` / `tsv` output (per dsp-cli/ADR-0007).
- A successful command writes nothing to stderr by default. A failed command writes nothing to stdout in the prose / line-based formats;
  structured formats are the exception (see below).
- Pipes only see the data: `dsp ... -l | xargs ...` works without filtering meta lines or error noise.

## Error shape per format

- **`prose`** (default) — human-readable error message to stderr, optional second line with a remediation hint:

  ```
  Error: data model 'biblio-onto' not found in project 'incunabula'
  Hint: list available data models with `dsp vre data-model list --project incunabula --server <s>`.
  ```

  ANSI colour is applied when stderr is a TTY, suppressed otherwise.

- **`json`** — same `_meta` envelope as success, with an `error` object replacing `data`:

  ```json
  {
    "_meta": { "auth": "...", "server": "...", "exit_code": 3 },
    "error": {
      "kind": "auth_required",
      "message": "endpoint requires authentication",
      "hint": "run `dsp auth login --server test` first"
    }
  }
  ```

  Emitted to **stdout** (same channel as the successful structured response, so a JSON-consuming caller has one parser, one stream).
  The `kind` field is a stable enum (see below); `message` and `hint` are not stable.

- **`lines` / `csv` / `tsv`** — empty data on stdout (no headers emitted), human-readable error on stderr identical to prose.
  These formats are for shell pipelines that already key on exit code; the structured-error envelope is overkill.

## Error kind enum (stable)

The `kind` field in JSON errors is the stable interface:

- `usage` — bad flag, missing argument, unknown identifier syntax.
- `auth_required` — the server demanded auth and none was supplied or the token was rejected.
- `not_found` — the named project / data model / resource type does not exist.
- `server_error` — the server responded with a 5xx or an unparseable response.
- `network` — could not reach the server.
- `conflict` — a server-side resource is busy or already exists — e.g. a dump is already in progress.
- `io` — a local filesystem operation the user requested failed — e.g. writing the dump file.
- `internal` — unexpected state in dsp-cli itself; should be reported as a bug.

Both `conflict` and `io` map to exit code `1` (Runtime). The four exit codes are unchanged;
this is an additive change — existing consumers that ignore unknown kinds are unaffected.

Adding a new kind is additive (existing consumers ignore unknown kinds); renaming is a breaking change (CHANGELOG entry; affects 0.x bumps).

## Logging

`dsp-cli` uses the `tracing` crate with `tracing-subscriber` for output.
Logging is for diagnostics during a command's execution — *not* for the command's primary output, which is the data.

- **Default level**: `WARN`. A successful command produces no log output.
- **`-v` / `--verbose`** raises the level: `-v` = `INFO`, `-vv` = `DEBUG`, `-vvv` = `TRACE`. A single repeated flag, not separate flags per level.
- **`RUST_LOG`** environment variable overrides the flag entirely if set. This is the escape hatch for targeted debugging (`RUST_LOG=dsp_cli::client=trace`).
- All log output goes to **stderr**, formatted as compact human-readable text with a level prefix.
  No JSON-structured logs in v1; if a use case for log aggregation emerges, that's a future ADR.
- Logging is initialised once, in `main.rs`, before any command logic runs. Library callers can install their own subscriber.

## Considered alternatives

- **Two-code model (`0` / `1`).** Rejected — agents lose the ability to distinguish "I should retry after login" from "the server is unreachable" without parsing prose.
  The cost of three extra codes is negligible; the agent ergonomics win is real.
- **Five-code model with a separate `not_found`.** Rejected — `not_found` is recoverable via the same UX as a `usage` error (list the available identifiers),
  and conflating them with `1`-runtime would lose meaning. Putting it under `1` and surfacing the distinction via the JSON `kind` field is enough.
- **`stderr` for structured errors too.** Rejected — splits the channel an automated caller has to read.
  A JSON consumer would have to merge stdout and stderr to handle both success and failure, defeating the point of the envelope.
- **`anyhow` everywhere with stringified errors.** Rejected — anyhow is great for the binary's top-level error handling, but stringified errors lose the `kind` enum.
  Use `thiserror`-defined error types in library code and convert at the binary boundary.
- **Auto-detect a `--quiet` flag.** Rejected for v1. Default is already terse (no logging output on success). Add `--quiet` if real noise emerges.

## Amendment (2026-07-21, plan 032)

The binary-level error handler in `main.rs` now routes every top-level error through the selected format's `Renderer::diagnostic` — no `anyhow` (D2 of plan 032; the existing `Diagnostic` enum is routed through unchanged, closing out the "anyhow handler" language in the original Phase-1 backlog item). Under `-j` this emits the JSON error envelope to stdout; other formats emit `Error: {diag}` to stderr, matching the per-format `diagnostic()` contract already specified above.

## Consequences

- The `_meta` envelope from dsp-cli/ADR-0007 carries `exit_code` so JSON consumers don't need to read the process exit code (they often can't).
- The `kind` enum is part of the public CLI contract per dsp-cli/ADR-0003. Snapshot tests cover representative `kind` values per format.
- `tracing` and `tracing-subscriber` are dev/runtime dependencies from day one. No commitment to OpenTelemetry or structured log shipping; that would be a separate ADR.
- The four exit codes are documented in `dsp docs output` so agents reading the embedded docs see them alongside the format catalogue.
- Implementation: a single `Diagnostic` (or `DspError`) enum in `src/diagnostic.rs` carries the `kind`, message, and optional hint;
  the renderer trait grows `error_prose` / `error_json` / etc. methods symmetric to the data-rendering ones.

## Amendment (2026-08-07, plan 035)

### Exit semantics when the payload is a store-authored error document

`dsp vre sparql query` ([dsp-cli/ADR-0016](0016-sparql-passthrough.md)) is the first command whose payload
can itself be an error document authored by something other than dsp-cli — the triplestore's own
rejection of a malformed query. stdout stays data-only: on any non-`2xx` relay, nothing is written to
stdout at all. The store's own text goes to **stderr** instead, sanitised (control characters
stripped, `\n`/`\t` kept) and capped at 200 characters, with exit `1`. dsp-api's own typed failures
(distinguished by status, never by content-sniffing) map to the existing `Diagnostic` variants and
exit codes: `401`/`403` → `AuthRequired`/3, `404` → `NotFound`/1, `413` → `Usage`/2 (the one
caller-fault mapping in this command that gets exit `2`, since it is unambiguously the submitted
query text being too big), `415` → `Internal`/1, `500`–`504` → `ServerError`/1.

## Amendment (2026-09-20) — a closed stdout pipe is not an error

`dsp ... | head` used to exit `1` with `Error: internal error: io error: Broken pipe` on stderr,
because the blanket `From<std::io::Error> for Diagnostic` maps every I/O failure to `Internal`. A
reader that stops reading is the normal end of a pipeline, not a runtime error, and the noise broke
the "pipes only see the data" promise above.

A write to stdout that fails with `ErrorKind::BrokenPipe` now ends the process with exit `0` and no
stderr output. Every other write error still becomes `Internal` and exits `1`, and the exit-code
table is otherwise unchanged — this is a narrowing of when `1` is reported, not a new code.

`Diagnostic::from(io::Error)` is deliberately left as it is: it is reached from non-stdout paths
too, such as reading `auth.toml`, where a broken pipe is meaningless. The check sits upstream
instead, in a writer adapter wrapping each stdout handle where it is constructed — the five
renderers' sinks plus the three commands that write to stdout directly (`dsp docs`,
`dsp auth token`, `dsp vre sparql query`). Nothing in the renderer trait or any public signature
changed.
