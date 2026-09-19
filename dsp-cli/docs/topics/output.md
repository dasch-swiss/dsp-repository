# Output — formats, channels, and the envelope

`dsp-cli` produces prose by default and structured formats on demand. This topic
explains the philosophy and the contract; per-flag detail stays in `--help`.

## Prose by default

With no format flag, commands print readable prose — aligned, contextual, often
with a next-step hint. This is the right format while exploring, for humans, and
for an agent reasoning about what to do next.

## Structured formats, for piping

One flag switches to a machine format:

| flag | format | use it for |
|---|---|---|
| `-j` / `--format json` | JSON | nested data, programmatic consumption |
| `--csv` | CSV | spreadsheets, tabular import (RFC-4180 quoted) |
| `--tsv` | TSV | tab-separated tabular data |
| `-l` / `--format lines` | lines | the lean, pipe-into-`grep`/`cut` format |

Reach for a structured format the moment you intend to pipe output into another
tool. See `dsp docs workflows` for chaining examples.

**Not every command has a format flag.** `dsp docs`, `dsp auth token`, and `dsp
vre sparql query` have no `--format`/`-j`/`-l` at all — each hands back a value
whose shape isn't dsp-cli's to wrap: a markdown document, a bare JWT, and a
store-authored SPARQL response, respectively. See `dsp docs sparql` for the
SPARQL passthrough command's own output contract.

## stdout is data; stderr is everything else

- **stdout** carries only the data you asked for (prose or structured).
- **stderr** carries logs, progress, and — for `lines`/`csv`/`tsv` — the
  auth-state disclosure line.

A successful command writes nothing to stderr by default, so
`dsp … -l | xargs …` sees only data, no noise.

## The JSON envelope

Every JSON response is a single object with `_meta` first, then exactly one of
`data` (success) or `error` (failure):

```json
{ "_meta": { "server": "…", "auth": "…", "exit_code": 0 }, "data": … }
```

Key order is deterministic, so the envelope is a stable contract. `_meta.auth` is
the structured form of the auth-state disclosure (see `dsp docs connecting`); a
list command's `data` is an array, a describe command's `data` is an object.

## Column projection (`--columns`)

For `csv`, `tsv`, and `lines` output, `--columns=X,Y,Z` selects **and reorders**
the output columns — user order wins. The valid column names for a given command
are listed in its `--help` (under `Columns (--columns):`).

- `--columns` is not supported for `json` (use `jq` instead) or `prose`
  (narrative, not column-structured) — either exits with a usage error.
- Duplicates in the column list are rejected as a usage error.
- On `lines`, `--columns` selects from the **full column set** (the csv header
  set), overriding the lean default subset. Without the flag, `lines` keeps its
  current lean default byte-identically.

## Header control (`--no-header`, `--header-only`)

For `csv` and `tsv` output only:

- `--no-header` — suppress the header row. Useful for appending rows to an
  existing file: `dsp vre project list --format csv --no-header >> all.csv`.
- `--header-only` — emit only the header row and exit 0. Useful to inspect the
  column set. **Note: the server fetch still happens** — short-circuiting the
  fetch is out of scope for v1. Compose with `--columns` to see the selected
  header only.

Headers-on is the unflagged default. Both flags are mutually exclusive. Using
either with `prose`, `lines`, or `json` exits with a usage error.

## Value rendering in `resource describe`

`dsp vre resource describe --values` extends the output with the resource's field
values. The `--values` flag is off by default; without it the output is the
compact metadata envelope only (unchanged from unflagged behaviour).

**Shape differs by format — read this before piping.** In **prose** and **json**,
`--values` *adds* a values section on top of the metadata envelope, so you get
both. In **tabular formats** (`csv`, `tsv`, `lines`), `--values` instead *replaces*
the metadata row with one row per value — the envelope metadata is not shown in
that mode (omit `--values` to get it back).

- **prose** — a `Values:` block follows the metadata header, one field per subsection
  with the field's human label (when resolvable) and its value(s) indented. Each
  server-supplied scalar is sanitised against control characters before printing.
  When a value has a comment (a free-text annotation the server attaches to that
  specific value, e.g. "reading uncertain"), it renders on its own indented line
  right under the value.
- **json** — a `"values"` key is added to the `data` object; absent entirely when
  `--values` is not passed (not `null` — absent). Each value object carries a
  `value_type` key and type-specific payload keys (e.g. `text`, `integer`, or
  `target_iri`/`target_label` for links). A value object also carries a `comment`
  key when the value has one — present only when set; omitted (never `null`)
  otherwise.
- **tabular** (`csv`, `tsv`, `lines`) — one row per value. The default columns (no
  `--columns`) are `field, field_label, value_type, value` — compact, since the
  resource's own `label`/`iri` would otherwise repeat identically on every row.
  Pass `--columns label,iri,field,field_label,value_type,value,comment` (or any
  subset) to include the resource's `label`/`iri` and/or a value's comment
  alongside each row — useful for self-contained/greppable rows when
  concatenating several describes' output. `comment` is opt-in like `label`/`iri`
  (part of the full column set, not the default) since comments are empirically
  rare — a mostly-empty trailing column on every row isn't worth the width by
  default.

Resolving field and vocabulary-item labels may require additional server requests (one per
project ontology, one per distinct list node). These are deduplicated and
non-blocking: any label that cannot be resolved falls back to the local field name
or node IRI without aborting the describe.

## Tabular formats: control characters

All three tabular formats — `lines`, `csv`, and `tsv` — **replace every ASCII
control character** — tab, newline, carriage return, NUL, ESC, and all other C0
controls plus DEL (`\x7f`) — with a single space (for `csv`, before RFC-4180
quoting). This keeps one-record-per-line and delimited consumers safe when
server-controlled text (labels, names) contains embedded control characters, and
keeps escape sequences from reaching the terminal. `json` keeps raw values (its
string encoding escapes control characters), so use `-j` if you need
byte-for-byte server fidelity.

A note on `auth logout` lines output: the default (no `--columns`) emits the
decorated `<server>\twas_cached=true/false` form as a single token per line.
When projection is active (`--columns was_cached`), it emits the plain value
(`true` or `false`).

A note on `dump --delete` lines output: the lines token is `deleted` or
`not-deleted`, while the csv/tsv column is `true`/`false`. These are distinct
representations of the same outcome — the difference is intentional (lines is
human-readable; csv/tsv is machine-parseable).

## `dsp docs -j` — machine-readable topic index

`dsp docs -j` emits the standard JSON envelope but with an **empty `_meta`**:

```json
{"_meta": {}, "data": [{"name": "dsp-cli", "summary": "…"}, …]}
```

No server or auth context applies to embedded documentation, so `_meta` is
literally `{}`. This is the one command whose `_meta` is empty (see
`dsp docs output` → The JSON envelope for the usual `_meta` shape; ADR-0003
documents the empty-`_meta` carve-out). `-j` conflicts with a specific topic
positional and with `--pager` — it exists only for the no-arg index form.
Topic bodies stay raw markdown and are never JSON-wrapped.

## Errors and output

On failure, the data is replaced by error information and the exit code is
non-zero — branch on the **exit code**, which is the reliable contract. Under `-j`,
the structured JSON error envelope (`{ "_meta": …, "error": { "kind": … } }`) is
emitted on stdout; every other format prints `Error: …` to stderr. Full detail and
recovery guidance: `dsp docs errors`.

See also: `dsp docs errors`, `dsp docs workflows`.
