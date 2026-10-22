# Commands are stateless; output is one of five formats in a stable envelope

`dsp-cli` is designed for **agent-mediated chaining**: each command stands alone, takes explicit flags,
and emits output meant to be read either by an AI agent constructing the next call or by a shell using standard Unix tooling.
The CLI itself has no piping-aware behaviour — no implicit stdin reading, no session state — because the agent is already doing the pipe's job.
Five output formats span the prose / Unix-friendly / structured-interchange spectrum.

## Considered Options

- **P1 — Pipes as first-class.** Implicit stdin reading, default parseable output. Rejected — conflicts with prose-by-default;
  "magic" stdin behaviour is surprising in a flag-based CLI; the agent is already pipe-equivalent.
- **P3 — Implicit session state.** Persistent context (env var, state file, REPL) so subsequent commands inherit project/data-model identifiers.
  Rejected inside the CLI — hidden state breaks reproducibility: an agent replaying a transcript should get identical results regardless of prior commands.
  *Deferred* outside it: a thin shell wrapper or a separate context-aware tool, if demand for session-context shortcuts emerges. The core CLI stays stateless.
- **P2 — Agent-mediated, with structured output as the bridge to Unix.** Chosen. Each command is self-contained and replay-safe;
  the structured formats let `jq`, `cut`, `awk`, and `xargs` handle bulk work for shell users.

## Output format catalog

| Format | Short | Header | Escaping | Use case |
|---|---|---|---|---|
| `prose` | — | n/a | n/a | default; rich, contextual, includes next-step hints |
| `lines` | `-l` | no | control chars → space | one entity per line, tab-separated; the dumb-Unix format for `cut -f1` / `awk` / `xargs` |
| `tsv` | — | yes | control chars → space (incl. tab/newline) | formal tab-separated table for spreadsheet/dataframe import |
| `csv` | — | yes | yes (RFC 4180); control chars → space | formal comma-separated table, universal interchange |
| `json` | `-j` | n/a | n/a | nested structure, full type fidelity; for `jq` |

The format is selected via `--format=<value>` (canonical) or the short aliases `-j` / `-l`; `-j` wins over `-l`, which wins over `--format`.
A new format extends the enum without proliferating flags.

Each noun-group specifies which columns it emits in which format, documented in each command's `--help`. This is part of the public CLI contract (stability obligation).

### Columns and headers

- `--columns=X,Y,Z` selects **and reorders** columns for `csv`, `tsv` and `lines`; the user's order wins.
  It is not supported for `json` (consumers have `jq`) or `prose`: either is `Diagnostic::Usage`, exit 2, hint "`--columns` works with csv, tsv, and lines output".
  Duplicates are rejected with `Diagnostic::Usage`, and names are matched case-sensitively against the csv header names.
  The flag is `--columns`, not `--fields`, because `field` is a reserved domain noun (a slot on a resource-type).
- On `lines`, `--columns` selects from the **full column set** (the csv header set), overriding the lean default subset:
  `--columns iri --format lines` yields bare IRIs for piping. Without the flag, `lines` emits its lean subset.
- `--no-header` suppresses the header row of `csv`/`tsv` (enabling `>> all.csv` concatenation). `--header-only` emits only the header row and exits 0;
  with `--columns` it emits the selected header. The two are mutually exclusive (`clap conflicts_with`), and either with `prose`, `lines` or `json`
  is `Diagnostic::Usage`, exit 2. Headers are on by default; there is no `--header` flag. `--header-only` still contacts the server
  before it emits the header; skipping that fetch is *deferred* until a caller needs the header without a server.

### Tabular formats: control characters replaced by spaces

`lines`, `csv` and `tsv` replace every ASCII control character (`char::is_ascii_control()`: the C0 range plus DEL) with a single space before any format-specific quoting,
at the single `QuoteMode::apply` chokepoint, so every column of every tabular command is covered. `csv` then applies RFC-4180 quoting.
This closes a terminal-escape-sequence-injection surface and keeps an embedded tab or newline from breaking one-record-per-line or delimited consumers,
especially since `--columns` makes arbitrary server-controlled columns (`label`, `longname`, …) first-class output.

The fidelity cost is negligible on the convention that tabular columns hold single-line values (identifiers, single-line labels, dates; multi-line text is prose-only).
This is a DSP convention, not enforced at the client boundary: instance labels in particular are user-entered. Callers needing byte-for-byte server fidelity use `-j`.

Not sanitised, under the personal-CLI / user-controls-the-server threat model: `json` values (`serde_json` escapes C0 including ESC, so a raw ESC never reaches the terminal;
a raw DEL may, which is not an escape-sequence vector alone), header cells (compile-time `&'static str` constants), and stderr diagnostics
(server-derived dump IDs and project IRIs in progress output, the auth-state line, `Diagnostic` messages).

### JSON envelope

Every `json` response is a single object on stdout with `_meta` first, plus **exactly one** of `data` (success) or `error` (failure):

```json
{"_meta": {"server": "…", "auth": "…", "exit_code": 0}, "data": { … }}
{"_meta": {"server": "…", "auth": "…", "exit_code": 3}, "error": {"kind": "…", "message": "…"}}
```

- `data` is an object for single-result commands and an **array** for list commands, so the envelope is uniform across the CLI.
  A consumer parses one object: if `.error` is present the command failed, otherwise read `.data`.
- The `error` block is defined by [dsp-cli/ADR-0012](0012-diagnostics.md); the `error`-bearing form is the documented stdout exception (errors don't split across stdout/stderr).
- `server` appears only in `_meta`, never inside `data`.
- Key order is deterministic (`serde_json` `preserve_order`); `_meta` is always first. The shape is part of the public CLI contract (stability obligation).
- Top-level errors, caught by the binary-level handler in `main.rs` rather than by a command's own action layer, carry a **reduced** `_meta`:
  `exit_code` always, `server` when resolvable from the `--server`/`-s` flag, and no `auth` (`main` has no auth-cache access there).
- `dsp docs -j` is the one command whose `_meta` is the empty object `{}`: no auth or server context applies to embedded documentation.
  `{"_meta": {}, "data": [{…}]}` is part of the stable contract. A version stamp or other new `_meta` vocabulary there is *deferred* until something concrete needs it.
- List commands with pagination (`dsp vre resource list`) add stable, snake_case `_meta` keys: in single-page mode (default, or `--page N`) `page`
  (zero-based) and `may_have_more_results`; with `--all`, `pages_fetched` and `may_have_more_results` (always `false`).
  Exactly one of `page` and `pages_fetched` is present, so consumers branch on `pages_fetched` to tell `--all` output from single-page output.

### Commands without a `Renderer`

Three commands have no `Renderer`, no `--format`, no `-j`/`-l`, no `--columns` and no `_meta`: `dsp docs <topic>` (a markdown document), `dsp auth token`
(a bearer token; a JWT is not a JSON object, so an envelope would only get in the way of `export DSP_TOKEN=$(dsp auth token -s dev)`),
and `dsp vre sparql query` ([dsp-cli/ADR-0016](0016-sparql-passthrough.md)). Errors still go through the normal `Diagnostic`/exit-code path; only the success path is raw.
`Cli::output_format()` returns `None` for `AuthCmd::Token` and `SparqlCmd::Query`.

A fourth candidate applies this rule, not a resemblance to the three:

> No `Renderer` when the response body's shape and encoding are chosen by something other than dsp-cli, and re-encoding it through dsp-cli's envelope would
> corrupt it (csv/tsv/lines) or double-encode it (json embedding a JSON document as a string).

## Consequences

- Every command is reproducible in isolation. Replaying a transcript always behaves the same — critical for AI-agent debuggability.
- Bulk shell work composes with standard Unix tools: `dsp vre … --json | jq …` or `dsp vre … -l | xargs …`. Nothing dsp-cli-specific is required.
- Two explicit "no"s so they don't get re-litigated: no implicit stdin reading, no implicit session state.

Enforced by: the per-format renders in `dsp-cli/tests/*_snapshots.rs` (including `output_format_flags_snapshots.rs`), the JSON-envelope and
`--columns`/header-flag usage tests in `dsp-cli/tests/cli.rs` and `dsp-cli/tests/docs.rs`, a drift-guard test between the `after_help` column lines and the
column constants, and the `replace_control_chars` unit tests in `dsp-cli/src/util/text.rs` (**static-analysis**); statelessness and the no-stdin rule by
**review**.
