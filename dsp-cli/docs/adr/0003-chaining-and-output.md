# Chaining model and output formats

`dsp-cli` is designed for **agent-mediated chaining**: each command stands alone, takes explicit flags,
and emits output meant to be read either by an AI agent constructing the next call or by a shell using standard Unix tooling.
The CLI itself has no piping-aware behaviour — no implicit stdin reading, no session state — because the agent is already doing the pipe's job.
Five output formats span the prose / Unix-friendly / structured-interchange spectrum.

## Considered alternatives

- **P1 — Pipes as first-class.** Implicit stdin reading, default parseable output. Rejected — conflicts with prose-by-default;
  "magic" stdin behaviour is surprising in a flag-based CLI; the agent is already pipe-equivalent.
- **P3 — Implicit session state.** Persistent context (env var, state file, REPL) so subsequent commands inherit project/data-model identifiers.
  Rejected — hidden state breaks reproducibility: an agent replaying a transcript should get identical results regardless of prior commands.
  Can be added later as a thin shell wrapper (or a separate context-aware tool) without baking statefulness into the CLI core.
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

The format is selected via `--format=<value>` (canonical) or the short aliases `-j` / `-l`. Adding more formats later (e.g. yaml) extends the enum without proliferating flags.

`--columns=X,Y,Z` selects and reorders output columns for `csv`, `tsv`, and `lines` output; the valid column names per noun-group are documented in each command's `--help`.
*(The flag was originally envisioned as `--fields`; see the Amendment below for the rename rationale and the full contract.)*

### JSON envelope (canonical)

Every `json` response is a single object on stdout with `_meta` first, plus **exactly one** of `data` (success) or `error` (failure):

```json
{"_meta": {"server": "…", "auth": "…", "exit_code": 0}, "data": { … }}
{"_meta": {"server": "…", "auth": "…", "exit_code": 3}, "error": {"kind": "…", "message": "…"}}
```

- `data` is an object for single-result commands and an **array** for list commands, so the envelope is uniform across the whole CLI.
  A consumer always parses one object: if `.error` is present the command failed, otherwise read `.data`.
- The `error` block is defined by [ADR-0012](0012-diagnostics.md); the `error`-bearing form is the documented stdout exception (errors don't split across stdout/stderr).
- `server` appears only in `_meta`, never duplicated inside `data`.
- Key order is deterministic (`serde_json` `preserve_order`); `_meta` is always first. The shape is part of the public CLI contract (stability obligation).

## Consequences

- Every command is reproducible in isolation. Replaying a transcript always behaves the same — critical for AI-agent debuggability.
- Bulk shell work composes with standard Unix tools: `dsp vre … --json | jq …` or `dsp vre … -l | xargs …`. Nothing dsp-cli-specific is required.
- Each noun-group must specify which fields it emits in which format. This is documented in `--help` and is part of the public CLI contract (stability obligation).
- If real demand emerges later for session-context shortcuts, the option remains open as a thin wrapper or new ADR — but the core CLI stays stateless.
- Two explicit "no"s recorded so they don't get re-litigated: no implicit stdin reading, no implicit session state.

## Amendment (2026-06-12, plan 020)

### `--columns` replaces the envisioned `--fields`

The flag was originally named `--fields` in this ADR. The name is changed to `--columns` because `field` is a reserved domain noun in dsp-cli vocabulary
(a slot on a resource-type — see `CONTEXT.md`).
In `resource-type describe` tabular output the *rows* are fields; `--fields=name,label` would read as schema-field selection.
`--columns` names the output concept directly (a named position in tabular output).
The flag was envisioned but never shipped, so there is no compatibility break.

### Projection scope: `csv`, `tsv`, `lines` only

`--columns` applies to `csv`, `tsv`, and `lines` output.
It is **not** supported for `json` (JSON consumers have `jq`; projection there would touch the serde view path and key-order guarantees for marginal gain)
or `prose` (format-agnostic narrative, not column-structured).
Using `--columns` with `prose` or `json` → `Diagnostic::Usage`, exit 2, with the hint: "`--columns` works with csv, tsv, and lines output".

On `lines`, projection selects from the **full column set** (the csv header set), overriding the lean default subset — `--columns iri --format lines` yields bare IRIs for piping.
Without the flag, `lines` keeps its current lean subset byte-identically.

### Projection semantics: select and reorder; duplicates rejected

`--columns` selects **and reorders** columns — user order wins.
**Duplicates are rejected** with `Diagnostic::Usage` (duplicate CSV header cells break downstream parsers; a projection that is dedup-free surprises nobody).
Column name matching is case-sensitive exact match on the csv header names. The valid column set per noun-group is listed in each command's `--help`.

### Header control: `--no-header` and `--header-only`

Two boolean flags control header emission for `csv` and `tsv` output:

- `--no-header` — suppress the header row (enables `>> all.csv` row concatenation across invocations).
- `--header-only` — emit only the header row and no data rows (exit 0). Note: **the server fetch still happens** — short-circuiting the fetch is out of scope for v1.

Headers-on is the unflagged default; there is no redundant `--header` flag. The two flags are mutually exclusive (`clap conflicts_with`).
Either flag with `prose`, `lines`, or `json` → `Diagnostic::Usage`, exit 2 (`lines` has no header concept by design).

When composed with `--columns`, `--header-only` emits the selected header.

### Tabular formats: control characters replaced by spaces

**All three tabular formats — `lines`, `csv`, and `tsv` — replace every ASCII control character**
(Rust's `char::is_ascii_control()`: the C0 range — tab `\t`, newline `\n`, carriage return `\r`, NUL, ESC, … — plus DEL `\x7f`)
**with a single space** before any format-specific quoting.
The shared helper is `crate::util::text::replace_control_chars` (a sibling of `strip_control_chars`, the prose sanitiser that keeps `\n`/`\t`).
This is applied at the single `QuoteMode::apply` chokepoint, so it covers every column of every tabular command:

- `lines` → `replace_control_chars` only.
- `tsv` → `replace_control_chars` only (this also prevents an embedded tab/newline from silently splitting a column — `tsv` was previously identity, a latent integrity bug).
- `csv` → `replace_control_chars`, then RFC-4180 quoting (`csv_field`).

Rationale: closes a terminal-escape-sequence-injection surface uniformly across every human-viewable format,
and prevents embedded `\t`/`\n` from breaking one-record-per-line / delimited consumers —
especially once `--columns` makes arbitrary server-controlled columns (`label`, `longname`, …) first-class tabular output.

The fidelity cost is negligible **on the convention** that tabular columns hold single-line values (short identifiers, single-line labels, dates;
multi-line text such as descriptions is prose-only).
This is a DSP convention, *not* enforced at the client boundary — instance labels in particular are user-entered.
Callers needing byte-for-byte server fidelity use `-j` (json), which is unaffected.

Out of scope (not sanitised, accepted under the personal-CLI / user-controls-the-server threat model — the same residual posture as `csv_field`'s formula-injection note):
`json` (raw values; `serde_json` escapes all C0 incl. ESC as `\uXXXX`, so a raw ESC never reaches the terminal —
a raw DEL `\x7f` may, but DEL alone is not an escape-sequence vector);
**header cells** (compile-time `&'static str` column-name consts, never server-controlled);
and **stderr diagnostic output** (`HumanProgress` server-derived dump IDs / project IRIs in `src/render/progress.rs`,
the auth-state disclosure line, and `Diagnostic` error messages).

### `dsp docs -j` and empty `_meta`

`dsp docs -j` (the no-arg topic index in JSON) is the one command whose `_meta` is **literally the empty object** `{}`.
No auth/server context applies to embedded documentation, and new `_meta` vocabulary (e.g. a version stamp) is deferred until something concrete needs it.
The shape `{"_meta": {}, "data": [{…}]}` is part of the stable contract; `_meta` is still present and still first.

## Amendment (2026-06-16, plan 022)

### Pagination `_meta` keys — stable contract for list commands with pagination

`dsp vre resource list` introduces pagination to the JSON output.
The following `_meta` keys are a stable public contract (snake_case; additive — existing callers that pass no pagination emit neither key):

**Single-page mode** (default, or `--page N` explicit):
- `page` (integer) — the zero-based page number that was fetched.
- `may_have_more_results` (boolean) — whether the server indicated that more pages are available beyond this one.

**All-pages mode** (`--all`):
- `pages_fetched` (integer) — the total number of pages fetched to build the full result.
- `may_have_more_results` (boolean) — always `false` in `--all` mode (the loop exits only once the server reports no more results).

**By-mode asymmetry is intentional.** Code must not assume both `page` and `pages_fetched` are present simultaneously —
exactly one of the two keys will be present, depending on the mode used.
Consumers should branch on the presence of `pages_fetched` to distinguish `--all` output from single-page output.

This amendment is **unconditional** of the simple-vs-complex response schema choice for `resource list` (deferred to ADR-0013, Phase 8c).
The `_meta` pagination shape is stable regardless of which schema the HTTP client uses internally to decode the response.

## Amendment (2026-06-19, Phase 8.5)

### Tabular control-char neutralisation — supersedes the original csv/tsv "fidelity" stance

The original catalog described `tsv` escaping as "no (embedded tabs unescaped; … Phase 7)" and treated `csv`/`tsv` as fidelity-preserving machine formats.
A security review of the prose resource-label hardening (Phase 8.5 #1) surfaced that this left a terminal-escape-sequence-injection surface in `csv`/`tsv` output
(and, for `tsv`, a latent column-corruption bug, since the mode was identity).
This amendment revises the stance: **all three tabular formats (`lines`/`csv`/`tsv`) now replace every ASCII control character with a space**
at the shared `QuoteMode::apply` chokepoint (helper `crate::util::text::replace_control_chars`).
See the revised _Output format catalog_ table and the _Tabular formats: control characters replaced by spaces_ section above for the full contract,
the convention assumption behind the negligible-fidelity-loss claim, the `json`/header-cell carve-outs, and the stderr-diagnostics scope-out.

## Amendment (2026-07-13, plan 026)

### `dsp auth token` — a second no-envelope, no-`--format` raw output shape

`dsp auth token` prints a single raw value — the resolved bearer token — directly to stdout: **no envelope, no `_meta`, and no `--format`/`-j`/`-l`**.
This is not a variant of the `json` envelope with an empty `_meta`
(that carve-out, documented above under _`dsp docs -j` and empty `_meta`_, is specifically about `docs`' JSON *mode* and does not apply here).
Rather, `dsp auth token` has no `--format` flag at all, in the same way `dsp docs <topic>` has none:
both commands hand back one raw value that isn't structured data to begin with (a markdown document; a JWT).
A JWT is not a JSON object, so wrapping it in `{"_meta": …, "data": "<token>"}` would only get in the way of the piping idioms this command exists for
(`export DSP_TOKEN=$(dsp auth token -s dev)`, `curl -H "Authorization: Bearer $(dsp auth token -s dev)"`).

This is the CLI's **second** no-`Renderer` command after `dsp docs <topic>`; [ADR-0010](0010-embedded-documentation.md) is the precedent this amendment follows.
As with `docs`, errors still go through the normal `Diagnostic`/exit-code path (`AuthRequired`, exit `3`, for a missing or locally-expired token) — only the success path is raw.

## Amendment (2026-07-21, plan 032)

### Top-level errors get a reduced `_meta`

Top-level errors — those caught by the binary-level handler in `main.rs`, as opposed to a command's own action-layer error — emit a **reduced** `_meta` compared to a normal command response: `exit_code` is always present, `server` is best-effort (populated when resolvable from the `--server`/`-s` flag at the top level, omitted otherwise), and `auth` is omitted unconditionally (`main` has no auth-cache access at that point). This is a narrower `_meta` shape than every other JSON output cataloged above, scoped specifically to the top-level error path.

## Amendment (2026-08-07, plan 035)

### A rule for the no-`Renderer` carve-out, not just a third example

`dsp vre sparql query` ([ADR-0016](0016-sparql-passthrough.md)) is the **third** command with no
`Renderer`, no `--format`, no `-j`/`-l`, and no `--columns`/`--no-header` — after `dsp docs` (plan
018) and `dsp auth token` (plan 026). It goes further than either precedent: it also relays a
store-chosen `Content-Type` and a store-chosen status, which neither prior carve-out did. Three data
points are now a pattern; a fourth candidate should apply a rule, not eyeball similarity to three
examples:

> No `Renderer` when the response body's shape and encoding are chosen by something other than
> dsp-cli, and re-encoding it through dsp-cli's envelope would corrupt it (csv/tsv/lines) or
> double-encode it (json embedding a JSON document as a string).

`dsp vre sparql query`'s stdout is the store's own bytes, byte-verbatim, in whatever media type it
negotiated (default `application/sparql-results+json`, overridable via `--accept`) — never re-encoded,
never control-character-stripped. Consequently `Cli::output_format()` returns `None` for this leaf,
exactly as it does for `AuthCmd::Token(_)`.
