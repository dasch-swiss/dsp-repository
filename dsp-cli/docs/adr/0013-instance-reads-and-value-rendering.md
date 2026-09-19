# Instance reads and value rendering

`dsp-cli` reads data **instances** — Resources and their Values — via the
`dsp vre resource` noun-group. The list and metadata-envelope commands shipped in
Phases 8a (`resource list`) and 8b (`resource describe`); this ADR records the
decisions behind the **full value rendering** added in Phase 8c
(`resource describe --values`) and consolidates the instance-read decisions that
8a/8b settled empirically but never wrote down (response schema, addressing,
pagination). It is the design record for the instance-read surface as a whole.

The hard problem 8c solves is **heterogeneous value rendering**: a Resource holds
Values of many different value-types (text, date, link, file, …), each carried by a
different DSP-API JSON-LD shape, and each wanting a different human rendering. This
ADR fixes the rendering matrix, the field-identity model, and the command surface so
the output is a stable public contract.

## Scope

Phase 8c covers, behind a `--values` flag:

- Parsing a Resource's fields and their Values from the complex-schema response.
- Rendering each Value per the value-type matrix below, in **prose** and **json**.
- Resolving human **field labels** (one ontology fetch per referenced data-model)
  and **list-node labels** (one fetch per referenced list node), with graceful
  degradation to local names / IRIs on failure.

Explicitly **deferred** (recorded below, not built in 8c):

- **Tabular value rendering** (`lines` / `csv` / `tsv`) — deferred out of 8c
  scope pending a shape decision. **Decided and shipped in Phase 8.5 item 3**
  (option 1, long-format value rows) — see "Output shape" → "tabular", below.
- **ARK-based addressing** — internal IRI only (Beyond v1; dsp-api has no ARK→IRI
  endpoint).
- **Instance search** (full-text / by-label / Gravsearch) — Beyond v1.

## Response schema: complex everywhere (settled in 8a/8b)

dsp-cli uses the DSP-API **complex** schema for *all* resource reads. This was
settled empirically with the user (plan 022 D4 for `list`, plan 023 D4 for
`describe`) and is **not re-opened here**. The simple schema is not used anywhere.

Rationale, recorded for completeness:

- Simple drops all resource audit/admin metadata (`creationDate`,
  `lastModificationDate`, `attachedToProject`, `attachedToUser`, `hasPermissions`,
  `userHasPermission`) — the 8b envelope needs these.
- Complex is a strict superset and uniquely embeds **link-value targets**
  (`@id` + `@type` + `rdfs:label`), so rendering a link as `→ <target label>` is
  free — simple would force a second fetch per link.
- Complex carries file-value dimensions / IIIF metadata.
- The only cost is wire size / overfetch, explicitly accepted as a non-concern for
  this CLI.

The `schema=complex` query parameter is a constant confined to `src/client/`; it is
**never** a trait parameter and **never** surfaces as a user-facing flag (that would
leak DSP-API/JSON-LD jargon — ADR-0001).

**"Simplifying ourselves" is the value-rendering matrix.** Choosing complex means
each matrix arm reads its datum out of a complex value object (a one-key lookup for
scalars; `valueAsString` / `textValueAsXml` for text; structured calendar fields for
dates; the embedded target for links). This is *less* work than simple would have
been for links, and the only place dsp-cli does per-value-type interpretation.

## Addressing: internal IRI only (settled in 8b)

`resource describe --resource <iri>` takes the resource's internal IRI. ARK
addressing is unsupported in v1 (dsp-api has no ARK→IRI lookup; resolution is an
external service). `-p/--project` is an optional cross-project guard, not a lookup
key (plan 023 D2).

## Pagination (settled in 8a)

`resource list` paginates with `--page` (0-based, one page) and `--all`
(auto-paginate until `knora-api:mayHaveMoreResults` is false), surfaced as the
`_meta` keys `page` / `pages_fetched` / `may_have_more_results` (ADR-0003
amendment). `describe` returns a single Resource and does not paginate. No
permission-aware total-count is available for `list`: both count routes — the v2
`/v2/resources/info` and the richer v3 `GET /v3/projects/{projectIri}/resourcesPerOntology`
(per-class `itemCount`, grouped by ontology) — are unfiltered (they count resources
the caller may not be permitted to see), so `resource list` itself carries no total. **(Amended 2026-07-20, plan 030.)**
Instance counts per resource-type instead shipped as `--count` flags on
`resource-type list`/`describe` — schema-side commands, not `resource list` —
using the v3 route directly. The counts share the same non-permission-filtered,
non-deleted-only contract described above; the disclosure is carried by a
dedicated `MetaContext.count_caveat` field (distinct from this ADR's
`filter_warning`, which is instance-side only). See `docs/BACKLOG.md` (Resolved)
and `CHANGELOG.md`.

## The `--values` flag

`resource describe` is **compact by default**: with no flag it renders only the
metadata envelope (8b behaviour). `--values` opts into the full field/value list.

- **Flag name:** `--values` — a boolean, default off. Chosen over a
  `--detail basic|full` enum: the CLI has no other "detail level" concept, and
  `--values` names exactly what it adds ("show me the values"). The name is the
  frozen public contract.
- **Default off — rationale:** the metadata envelope is small and answers "what is
  this resource?"; the full value dump can be large (the sampled `incunabula:Page`
  is ~13 KB of complex JSON-LD). An agent burning context wants the envelope by
  default and opts into the dump deliberately. Default-off also preserves 8b's
  output byte-for-byte for callers that don't pass the flag.
- **No schema/detail jargon.** Per ADR-0001 the flag and all help text are framed
  around *what the user gets* (values), never the DSP-API schema names
  ("simple" / "complex") or RDF vocabulary.

## Field identity and humanization

A Resource's complex-schema response keys its fields by property CURIE
(`incunabula:hasPagenum`) and carries the Values, but **not** the field's human
label or (for list/link) the referenced node's label. The user chose to
**humanize** these via extra fetches rather than show raw identifiers. This does not
contradict the "no N+1" reasoning behind the complex-schema choice: that reasoning
was specifically about *link-target* labels (which complex gives free). Field labels
and list-node labels are available in *neither* schema, so resolving them is a new,
separate, **opt-in** cost gated behind `--values`.

| Identifier | Source | Resolution | Cost |
|---|---|---|---|
| **Field name** | property CURIE on the resource | strip prefix → local name; strip the `Value` suffix on link properties (`isPartOfBookValue` → `isPartOfBook`) | none |
| **Field label** | not in the response | fetch the defining data-model's ontology (`/v2/ontologies/allentities`), map property IRI → `rdfs:label` | 1 fetch per **distinct** data-model referenced (usually 1) |
| **Link target label** | embedded in the complex link value (`linkValueHasTarget.rdfs:label`) | read it directly | none (free) |
| **List-node label** | not in the value (only `listValueAsListNode.@id`) | fetch `/v2/node/<node-iri>`, read `rdfs:label` | 1 fetch per **distinct** list node referenced |

**Built-in (knora-api) fields** — e.g. `knora-api:hasStillImageFileValue` — do
**not** trigger an ontology fetch (the knora-api ontology is large and its
allentities response heavy). In v1 they degrade directly to their local name
(`hasStillImageFileValue`); no built-in label map is maintained. Only **project**
ontologies are fetched for labels. (A small hardcoded label map for the handful of
common built-in file fields may be added later if the local names read poorly — out
of scope for v1.)

With `--values` on, a describe therefore costs `1 (resource) + N_project_ontologies
+ N_listnodes` HTTP calls. Fetches are **deduplicated** (each ontology / list node
fetched at most once) and **non-fatal**: any label fetch that fails (network,
permission, missing) degrades to the local name (field) or the node IRI (list),
never failing the describe. The field-name local-name derivation (prefix strip,
`Value`-suffix strip) and all label mapping happen **at the client boundary**
(`src/client/`), per ADR-0001 — no DSP-API/JSON-LD vocabulary reaches the model,
renderer, or help layers.

The `Value`-suffix convention is load-bearing: dsp-api appends `Value` to link
properties when serialising the complex schema (verified in
`OntologyTransformer`/`ResourcesRepoLive`), so a link field `foo:isPartOfBook`
appears under the key `foo:isPartOfBookValue`. Stripping the suffix recovers the
field name a user would recognise.

## Value-type rendering matrix

Each Value's type is detected from the value object's `@type`. The dsp-cli
value-type **token** (the `value_type` reported in output) reuses the canonical
vocabulary already in CONTEXT.md (`text`, `integer`, `decimal`, `boolean`, `date`,
`time`, `uri`, `color`, `geoname`, `vocabulary-item`, `link`, `still-image`,
`moving-image`, `audio`, `document`, `archive`). The DSP-API class names
(`TextValue`, `LinkValue`, …) never leave `src/client/`.

**Key-prefix note (load-bearing):** every key in the table below appears in the
JSON-LD value object as a **`knora-api:`-prefixed CURIE** (e.g.
`knora-api:intValueAsInt`, `knora-api:linkValueHasTarget`). The JSON-LD `@context`
applies at the document level, not inside nested value objects, so the keys arrive
verbatim as CURIE strings — a lookup must use the full prefixed key. The prefix is
omitted from the table cells only for brevity.

**What counts as a field (extraction rule).** Not every key on the resource is a
user field. A key in the flattened `extra` map is treated as a field **iff** its
value is a value object (or array of value objects) whose `@type` is a **knora-api
value class** (a `knora-api:*Value` — `TextValue`, `IntValue`, `LinkValue`,
`StillImageFileValue`, …). Requiring a *value-class* `@type` (not merely *any*
`@type`) is load-bearing: it excludes envelope-metadata objects that carry an
`xsd:` `@type` rather than a value-class one — notably `knora-api:versionArkUrl`
(`{"@value":…,"@type":"xsd:anyURI"}`), which is **not** a named DTO field and would
otherwise leak as a spurious `raw` field. Scalar system keys
(`knora-api:isDeleted`) and `@`-keys are skipped automatically (no value-class
`@type`). In addition, an explicit denylist skips value objects whose `@type` *is*
`*Value`-shaped but are not the resource's own user fields: keyed by
`knora-api:hasIncomingLinkValue` (reverse links) and
`knora-api:hasStandoffLinkToValue` / `knora-api:hasStandoffLinkValue`
(standoff-internal links, both spellings), and any value whose
`@type` is `knora-api:DeletedValue` (a deleted value — present in `knora-base` but
not a user field; would otherwise pass the suffix test). The
envelope metadata captured as **named DTO fields** (`arkUrl`, `creationDate`,
`lastModificationDate`, `attachedToProject`, `attachedToUser`, `hasPermissions`,
`userHasPermission`, plus the new `@context`) never reach `extra` at all; only file
values among `knora-api:`-prefixed keys are genuine fields, and they pass the
value-class test (`*FileValue`).

| value-type | DSP-API `@type` | Datum key(s) (complex schema) | dsp-cli rendering |
|---|---|---|---|
| `text` | `TextValue` | **presence-based**: `textValueAsXml` if present (formatted) → strip; else `valueAsString` (plain) | plain text; standoff XML stripped via `html_to_text` (lossy for custom standoff tags — they are dropped, not rendered) |
| `integer` | `IntValue` | `intValueAsInt` (JSON number) | the integer |
| `decimal` | `DecimalValue` | `decimalValueAsDecimal.@value` (string) | the decimal string (precision preserved) |
| `boolean` | `BooleanValue` | `booleanValueAsBoolean` | `true` / `false` |
| `date` | `DateValue` | `dateValueHasCalendar`, `dateValueHasStart{Year,Month,Day,Era}`, `dateValueHasEnd{…}` | calendar-aware human date; range collapsed to a single point when start == end (see below) |
| `time` | `TimeValue` | `timeValueAsTimeStamp.@value` | the timestamp string |
| `uri` | `UriValue` | `uriValueAsUri.@value` | the URI |
| `color` | `ColorValue` | `colorValueAsColor` (string) | the hex string |
| `geoname` | `GeonameValue` | `geonameValueAsGeonameCode` (string) | the geonames code |
| `vocabulary-item` | `ListValue` | `listValueAsListNode.@id` | resolved node label (via `/v2/node`); degrades to the node IRI |
| `link` | `LinkValue` | `linkValueHasTarget` (embedded) **or** `linkValueHasTargetIri.@id` | `<target label> [<target iri>]`; degrades to `<target iri>` alone (no brackets) when no label is available |
| `still-image` | `StillImageFileValue` (and the `StillImageExternalFileValue` / `StillImageVectorFileValue` siblings) | `fileValueHasFilename`, `fileValueAsUrl.@value`, `stillImageFileValueHasDimX/Y` | `<filename> (<W>×<H>) <url>` |
| `moving-image` | `MovingImageFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `audio` | `AudioFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `document` | `DocumentFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `archive` | `ArchiveFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| *long tail* | `IntervalValue`, `GeomValue`, any unrecognised `*Value` | — | **`raw` fallback**: `value_type` is the lowercased local name of the `@type` (e.g. `interval`); the datum (`raw.text`) is `valueAsString` if present, else the value object's non-metadata literal(s) joined, else a compact JSON of the value object minus the standard metadata keys; never a hard error |

**Date formatting.** Built from the structured calendar fields, not from
`valueAsString` (which is not always present and is machine-ish). Format:
`<start> – <end> (<Calendar>)`, collapsing to `<point> (<Calendar>)` when start ==
end (a field-by-field equality on the `DatePoint`). A point's precision follows the
fields present (year; year-month; year-month-day). Era (`CE`/`BCE`) is appended when
present. Example shapes (exact glyphs locked by snapshot at implementation; this ADR
fixes the shape, not every glyph): a single Gregorian year → `1489 CE (GREGORIAN)`;
a full Julian day → `1456-03-14 CE (JULIAN)`; a range → `1489 CE – 1490 CE
(GREGORIAN)`.

**The `raw` fallback is a feature, not a gap.** The long tail (interval, geometry,
and any value-type DSP-API adds later) renders generically rather than failing —
the read never breaks on an unknown type. This mirrors CONTEXT.md's "value-types
outside this set degrade to their lowercased local name" rule.

**Text value handling — verify against real data first.** Detection of formatted vs
plain text is **presence-based** (`textValueAsXml` present → formatted; else
`valueAsString`) and deliberately does **not** depend on the
`knora-api:hasTextValueType` discriminant: that field's server-side implementation
in dsp-api is **incompletely finished** and may be absent or unreliable on real /
older resources (flagged by the DaSCH maintainer). The implementation must be
validated against a real text-bearing resource on a live server (formatted *and*
unformatted text) before relying on the parse — this is a required live-test case,
not an assumption. If presence-based detection proves insufficient in practice, the
`raw` fallback still guarantees the read does not break.

**File value detection — match the whole `*FileValue` family.** Map by the leading
kind of any `@type` ending in `FileValue`: `StillImage*` → `still-image` (covers the
plain, `External`, and `Vector` variants), `MovingImage*` → `moving-image`,
`Audio*` → `audio`, `Document*` → `document`, `Archive*` → `archive`,
`Text*FileValue` → `document`-style filename/url. All share
`fileValueHasFilename` + `fileValueAsUrl`; only `still-image` adds dimensions. An
unrecognised `*FileValue` degrades to `raw` (never a hard error).

## Output shape

### prose

After the metadata block, a `Values:` section. Each field is a header
line `<field label> (<field name>)`; when the label is unresolved (`None`) the
header is `<field name>` alone (no parentheses). The Value(s) follow, one per line,
indented; typed values show their rendering (link `→`, file dims, etc.). A field
with no readable values is omitted. Fields are ordered as the server returns them.

### json

The single-object `data` gains a `values` key **only when `--values`
is set** (absent otherwise — `Option`, so the 8b envelope is unchanged for
default calls). `values` is an array of field groups `{ "field", "field_label",
"values": [ { "value_type", …type-specific keys… } ] }`, deterministic key order
(ADR-0003). JSON keeps full type fidelity — structured per-type keys (e.g. a date's
calendar/start/end, a file's width/height/url), not a pre-rendered string.

**Server-text sanitisation in prose.** `--values` newly renders a large amount
of **server-controlled scalar text** to prose stdout — `valueAsString`, resolved
field labels, list-node labels, link target labels, filenames, and the `raw`
fallback string. The prose renderer **strips ASCII control characters** (including
ESC, the ANSI-escape vector) from every such scalar before printing, via the
`strip_control_chars` helper (`src/util/text.rs`), which keeps `\n`/`\t` (legitimate
in multi-line prose values). Its sibling `replace_control_chars` (which replaces
*every* control character, incl. `\n`/`\t`, with a space) backs the tabular formats
(`lines`/`csv`/`tsv`) — see ADR-0003. This
is done **at the prose render layer, not the client boundary**, so **json keeps the
raw server value verbatim** (ADR-0003 fidelity). This materially narrows the
control-character/terminal-injection surface that `--values` would otherwise widen
well beyond the single resource-label field of 8b (whose prose header / list-column
was itself brought under `strip_control_chars` in Phase 8.5 #1) — it is a deliberate,
scoped re-visit of the Phase 7 prose-sanitisation descope, justified by the new surface.
Standoff text additionally passes through `html_to_text` (which already strips C0
controls); the sanitiser covers the plain-text and label paths that do not.

### tabular

**Decided: ADR-0013 option 1 (long-format value rows), shipped Phase 8.5 item 3.**
With `--values`, `lines`/`csv`/`tsv` render **one row per Value**. Full column
set: `label, iri, field, field_label, value_type, value` — `label`/`iri` are the
leading "key columns" identifying which resource the row belongs to. This
*replaces* the metadata row in `--values` mode; the metadata row remains the
output when `--values` is absent (unchanged 8b behaviour).

**Shape asymmetry with prose/json is deliberate.** prose and json *add* a
values section on top of the metadata envelope; tabular `--values` shows
values *only* and **drops the rich envelope metadata** in that mode (the
envelope is available by omitting the flag). Options 2 ("values-in-a-cell" —
one metadata row plus a single serialised `values` cell, not parseable) and 3
("never" — formalise the 8c metadata-only behaviour permanently) were
considered and rejected in favour of option 1.

**Compact default, resource keys opt-in.** The default view (no `--columns`)
is the subset `field, field_label, value_type, value`. The resource
`label`/`iri` are constant across every value row of a single-resource
describe, so repeating them by default is pure redundancy — and token-waste
for an agent. They stay available via `--columns label,iri,…` for callers who
want self-contained/greppable rows (e.g. concatenating several describes).
This is the existing `default_columns` mechanism (the same one
`RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS` established). `field_label` stays in
the default for parity with the json `values` array, which already carries
`field_label` — no new vocabulary.

**The tabular `value` cell reuses the prose value glyphs**, rather than a
plainer awk-split form: `→ <target label> [<target iri>]` for links,
`<filename> (<W>×<H>) <url>` for still-image files, and so on for every other
value-type — a deliberate consistency/one-renderer choice. Both prose and the
three tabular renderers call the same shared `render_value_content` helper, so
there is exactly one place that decides how a `ValueContent` becomes a display
string; it returns that string **raw** (un-sanitised — see its `SECURITY:`
rustdoc note), leaving sanitisation to each caller (prose: one outer
`strip_control_chars`; tabular: the `render_table` → `QuoteMode::apply` →
`replace_control_chars` chokepoint).

**Date-arm sanitisation note.** Extracting `render_value_content` as that
shared helper means the Date arm — previously rendered in prose via a direct
`format_date_value` call with **no** `strip_control_chars` pass — is now
covered by sanitisation for the first time. Harmless in practice (calendar/era
are closed vocabularies — `GREGORIAN`/`JULIAN`/`CE`/`BCE` — with no control
characters to strip) and a net improvement; recorded here so it isn't a silent
change.

### Per-value comments

**Shipped in Phase 8.5 item 4** (was deferred out of 8c above as an empirically
rare feature; no longer deferred). DSP-API allows an optional free-text
annotation (`knora-api:valueHasComment`) on *any* value — e.g. "reading
uncertain" on a transcribed word.

**Model.** A `Value { content: ValueContent, comment: Option<String> }`
wrapper now carries every value in `FieldValues.values` (`Vec<Value>`,
replacing the bare `Vec<ValueContent>`). `comment` is read once at the client
boundary in `parse_value`, per ADR-0001 — the DSP-API key name
`knora-api:valueHasComment` itself stays confined to `src/client/http.rs` and
never reaches the model, renderers, or help layers.

**Always-rich, opt-in-tabular (the core decision).** prose and json —
the two "rich" formats — **always** surface a value's comment when present:
prose on an indented line under the value; json as a `comment` key inside the
value object. tabular (`lines`/`csv`/`tsv`) only surfaces it when the caller
explicitly asks, via `--columns …,comment` — `comment` is in the **full**
column set but **not** the default set. Rationale: comments are empirically
rare (absent across the sampled DaSCH-project classes and the incunabula
`Page`, 2026-06-18), so a default trailing column would be near-permanently
empty across every tabular row; and nothing is actually hidden from a caller
who wants it — prose and json both surface comments unconditionally, so a
default-off tabular column is not a lossy default, merely a compact one.

A `--comment`/`--comments` flag was considered and rejected: it would be
inconsistent with the existing `--columns` mechanism for optional columns, and
would add CLI surface + scope ambiguity for what is, in the tabular formats,
just one more column.

**json omit-when-absent, not `null`.** A value's `comment` key is present only
when the value has a comment; it is omitted (not emitted as `null`) otherwise.
This keeps every pre-existing json value snapshot byte-identical, matches the
compact-output goal, and mirrors this ADR's existing "the `values` key is
absent, not `null`, when `--values` is off" precedent. Contrast with
`field_label`, which **is** `null`-when-none: `field_label` is a per-field key
that is always conceptually present (every field either resolves a label or
doesn't), whereas a comment is genuinely optional per value — the two cases
differ, and this is not an inconsistency.

**Sanitisation coverage.** prose comments go through `strip_control_chars` —
the same helper, same call-site treatment as every other prose scalar
described above (preserves `\n`/`\t`). tabular comments go through the same
`render_table` → `QuoteMode::apply` → `replace_control_chars` chokepoint as
every other cell (neutralises `\n`/`\t` along with other control characters).
json stays raw (serde escapes C0 automatically), consistent with this ADR's
existing "json keeps the raw server value verbatim" principle.

One consequence worth stating explicitly: comments inherit the same prose
newline treatment as value scalars. Because `strip_control_chars` preserves
`\n`/`\t`, a multi-line server comment renders across multiple (un-indented
continuation) prose lines exactly as multi-line text values already do today;
json and tabular both neutralise `\n`, so the machine-readable surfaces are
unaffected either way. This is not a regression — it already happens for value
scalars — but it is recorded here so it isn't silently undocumented for
comments specifically.

**ADR-0003 needs no amendment.** The json `comment` key is additive within the
existing `values` array shape ADR-0003 already governs; the tabular column set
(full vs default, `--columns` opt-in) is owned by this ADR, not ADR-0003.

## Consequences

- The `--values` flag name, the prose `Values:` layout, the json `values` array
  shape, the value-type tokens, and the `raw` fallback become **public output
  contract** once snapshot baselines are accepted (ADR-0009). User-endorsed at plan
  approval.
- `describe` with `--values` makes more than one HTTP request — the first read
  command to fan out to ontology + list-node fetches. Documented for users
  (`dsp docs`) and bounded by dedup + graceful degradation.
- Instance-side value models (`Value`, value content, field grouping) are added in
  `src/model/resource.rs`, distinct from the schema-side `Field`/`Cardinality`
  (resource-type describe) but sharing the `ValueType` token vocabulary.
- `html_to_text` / `strip_control_chars` move from `src/render/html.rs` to a
  layer-neutral `src/util/` module so the client boundary can reuse `html_to_text`
  for standoff without a client→render dependency (ADR-0008 sibling-layer rule).
  ADR-0008 is amended in this task to record the `src/util/` home for dependency-free
  text helpers.
- The tabular-value follow-up is no longer a gap: it shipped in Phase 8.5 item 3
  (see "Output shape" → "tabular", above).
- Per-value comments (`knora-api:valueHasComment`) are no longer deferred: they
  shipped in Phase 8.5 item 4 (see "Output shape" → "Per-value comments", above).
- Re-opening the schema choice (simple vs complex) requires amending this ADR and
  plans 022/023 — it is settled.

## Amendment (2026-07-30, plan 034)

### `list-item` renamed to `vocabulary-item` — a deliberate break of the public output contract

The value-type token, both in the canonical token list and in the "Value-type rendering matrix"
table above, was renamed by exact identifier from `list-item` to `vocabulary-item`:
`ValueType::ListItem` → `VocabularyItem`, `ValueContent::ListItem` → `VocabularyItem`, and the
wire/output token string itself. It is surfaced as the `value_type` cell/key in `resource describe
--values` and `resource-type describe`, across json/csv/tsv/lines.

This ADR's own "Consequences" section states that "the value-type tokens … become public output
contract once snapshot baselines are accepted" (above). This amendment records that plan 034
knowingly broke that contract.

**Why.** Plan 034 adds a new `dsp vre vocabulary` noun-group (`list` + `describe`) that inspects a
controlled vocabulary's node tree. Left unrenamed, `dsp vre vocabulary describe` would describe a
vocabulary's **nodes** while `resource describe --values` called the exact same kind of value
`list-item` — two words for one concept in one tool. That is precisely the internal inconsistency
this ADR's own vocabulary discipline (and ADR-0001) exists to prevent, so leaving the token
unrenamed once `vocabulary` existed as a noun would have been the greater cost.

**Impact.** Breaking change to already-shipped output. Lands as `0.2.0` (the project's pre-1.0
convention bumps the **second** digit for a breaking change, the third for a feature). See the
`CHANGELOG.md` `[Unreleased]` `### Changed` entry and the `docs/adr/0001-vocabulary-divergence.md`
amendment recording the new `vocabulary` term itself.
