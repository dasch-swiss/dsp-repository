# Instance reads use the complex schema and render values only on request, as a stable output contract

`dsp-cli` reads data **instances**, Resources and their Values, through the `dsp vre resource` noun-group. A Resource holds Values of many value-types (text, date, link, file, …), each carried by a different DSP-API JSON-LD shape and each wanting a different human rendering. This record fixes the response schema, the addressing and pagination model, the command surface for values, the field-identity model and the output shapes, which are a stable public contract. The per-type rendering rules are in `docs/src/dsp-cli/value-rendering.md`.

## Response schema: complex everywhere

`dsp-cli` uses the DSP-API **complex** schema for all resource reads. The simple schema is not used anywhere. The reasons:

- Simple drops all resource audit and admin metadata (`creationDate`, `lastModificationDate`, `attachedToProject`, `attachedToUser`, `hasPermissions`, `userHasPermission`), which the metadata envelope needs.
- Complex is a strict superset and uniquely embeds **link-value targets** (`@id`, `@type` and `rdfs:label`), so rendering a link as `→ <target label>` costs nothing. Simple would force a second fetch per link.
- Complex carries file-value dimensions and IIIF metadata.
- The only cost is wire size, accepted as a non-concern for this CLI.

The `schema=complex` query parameter is a constant confined to `src/client/`. It is never a trait parameter and never a user-facing flag, because that would leak DSP-API and JSON-LD jargon (dsp-cli/ADR-0001). Choosing complex means each value-type reads its datum out of a complex value object; that is the only place dsp-cli interprets values per type.

## Addressing: internal IRI only

`resource describe --resource <iri>` takes the resource's internal IRI. `-p/--project` is an optional cross-project guard, not a lookup key. ARK addressing is unsupported: dsp-api has no ARK-to-IRI lookup, and resolution is an external service.

## Pagination and counts

`resource list` paginates with `--page` (0-based, one page) or `--all` (auto-paginate until `knora-api:mayHaveMoreResults` is false), surfaced as the `_meta` keys `page`, `pages_fetched` and `may_have_more_results` (dsp-cli/ADR-0003). `describe` returns a single Resource and does not paginate.

`resource list` carries no total. No permission-aware total count is available: both count routes, the v2 `/v2/resources/info` and the v3 `GET /v3/projects/{projectIri}/resourcesPerOntology` (per-class `itemCount`), are unfiltered and count resources the caller may not be permitted to see. Instance counts per resource-type are `--count` flags on the schema-side commands `resource-type list` and `describe`, using the v3 route. They are not permission-filtered and count non-deleted resources only. The disclosure is carried by `MetaContext.count_caveat`, distinct from `filter_warning`, which is instance-side only.

## The `--values` flag

`resource describe` is **compact by default**: without a flag it renders only the metadata envelope. `--values` opts into the full field and value list.

- **Name:** `--values`, a boolean, default off. It was chosen over a `--detail basic|full` enum: the CLI has no other detail-level concept, and `--values` names exactly what it adds. The name is part of the public contract.
- **Default off:** the envelope is small and answers "what is this resource?", while the full value dump can be large (the sampled `incunabula:Page` is about 13 KB of complex JSON-LD). An agent that spends context wants the envelope by default and opts into the dump deliberately.
- **No schema jargon:** the flag and all its help text are framed around what the user gets (values), never around the DSP-API schema names ("simple", "complex") or RDF vocabulary (dsp-cli/ADR-0001).

## Field identity and humanization

A Resource's complex-schema response keys its fields by property CURIE (`incunabula:hasPagenum`) and carries the Values, but not the field's human label or, for lists, the referenced node's label. `--values` **humanizes** these through extra fetches rather than showing raw identifiers. This does not contradict the complex-schema reasoning above: that was about link-target labels, which complex gives free. Field labels and list-node labels are in neither schema, so resolving them is a separate, opt-in cost gated behind `--values`.

| Identifier | Source | Resolution | Cost |
|---|---|---|---|
| **Field name** | property CURIE on the resource | strip the prefix to the local name; strip the `Value` suffix on link properties (`isPartOfBookValue` becomes `isPartOfBook`) | none |
| **Field label** | not in the response | fetch the defining data-model's ontology (`/v2/ontologies/allentities`) and map property IRI to `rdfs:label` | 1 fetch per distinct data-model referenced (usually 1) |
| **Link target label** | embedded in the complex link value (`linkValueHasTarget.rdfs:label`) | read it directly | none |
| **List-node label** | not in the value (only `listValueAsListNode.@id`) | fetch `/v2/node/<node-iri>` and read `rdfs:label` | 1 fetch per distinct list node referenced |

- The `Value`-suffix convention is load-bearing: dsp-api appends `Value` to link properties when serialising the complex schema, so a link field `foo:isPartOfBook` appears under `foo:isPartOfBookValue`. Stripping the suffix recovers the field name a user recognises.
- **Built-in (knora-api) fields**, for example `knora-api:hasStillImageFileValue`, trigger no ontology fetch (the knora-api ontology is large) and degrade directly to their local name. Only project ontologies are fetched for labels.
- With `--values`, a describe costs `1 (resource) + N_project_ontologies + N_listnodes` HTTP calls. Fetches are **deduplicated** and **non-fatal**: a label fetch that fails (network, permission, missing) degrades to the local name (field) or the node IRI (list) and never fails the describe.
- The prefix and suffix stripping and all label mapping happen at the client boundary (`src/client/`), so no DSP-API or JSON-LD vocabulary reaches the model, renderer or help layers.

## Value types as output contract

The value-type tokens are `text`, `integer`, `decimal`, `boolean`, `date`, `time`, `uri`, `color`, `geoname`, `vocabulary-item`, `link`, `still-image`, `moving-image`, `audio`, `document` and `archive`. They are the vocabulary of `CONTEXT.md`, and the DSP-API class names never leave `src/client/`. A value-type outside this set renders through the `raw` fallback, with the lowercased local name of its `@type` as token, and never fails the read. The fallback is a feature: it keeps reads working for value-types DSP-API adds later.

Controlled-vocabulary values use the token `vocabulary-item`, the same word `dsp vre vocabulary describe` uses for the nodes of a vocabulary. Two words for one concept in one tool is the inconsistency dsp-cli/ADR-0001 exists to prevent.

The rules for detecting each type, extracting fields, formatting dates, reading text and matching file values are in `docs/src/dsp-cli/value-rendering.md`.

## Output shape

### prose

After the metadata block, a `Values:` section. Each field is a header line `<field label> (<field name>)`; when the label is unresolved the header is `<field name>` alone, without parentheses. The Values follow, one per line and indented, each in its type's rendering (link `→`, file dimensions, …). A field with no readable values is omitted. Fields are in the order the server returns them.

### json

The single-object `data` gains a `values` key **only when `--values` is set**; it is absent otherwise, so the envelope is unchanged for default calls. `values` is an array of field groups `{ "field", "field_label", "values": [ { "value_type", …type-specific keys… } ] }` with deterministic key order (dsp-cli/ADR-0003). `field_label` is `null` when unresolved. JSON keeps full type fidelity: structured per-type keys (a date's calendar, start and end; a file's width, height and URL), not a pre-rendered string.

### tabular

With `--values`, `lines`, `csv` and `tsv` render **one row per Value** (long format), replacing the metadata row; without `--values` the metadata row is the output. The full column set is `label, iri, field, field_label, value_type, value, comment`; the default set (no `--columns`) is `field, field_label, value_type, value`. `label` and `iri` are constant across the value rows of a single-resource describe, so repeating them by default is redundancy and token waste for an agent; they stay available through `--columns label,iri,…` for self-contained, greppable rows.

The shape asymmetry with prose and json is deliberate: those add a values section on top of the metadata envelope, while tabular `--values` shows values only and drops the rich envelope metadata (omit the flag to get it). The tabular `value` cell reuses the prose glyphs (`→ <target label> [<target iri>]` for links, `<filename> (<W>×<H>) <url>` for still images, and so on), so one shared `render_value_content` helper decides how a value becomes a display string. It returns that string raw; sanitisation is left to each caller.

### Per-value comments

DSP-API allows an optional free-text annotation (`knora-api:valueHasComment`) on any value. A `Value { content, comment: Option<String> }` wrapper carries every value; the comment is read once at the client boundary, so the DSP-API key never reaches the model, renderers or help layers.

- prose and json, the two rich formats, **always** surface a comment when present: prose on an indented line under the value, json as a `comment` key in the value object.
- tabular surfaces it only on request, through `--columns …,comment`; `comment` is in the full column set but not the default set. Comments are empirically rare, so a default column would be near-permanently empty, and nothing is hidden from a caller who wants it.
- json **omits** the `comment` key when absent rather than emitting `null`. `field_label` is `null`-when-none because it is a per-field key that is always conceptually present, whereas a comment is genuinely optional per value.
- A multi-line comment renders across multiple prose lines, as multi-line text values do.

### Server-text sanitisation

`--values` renders a large amount of server-controlled scalar text to prose stdout: `valueAsString`, field labels, list-node labels, link target labels, filenames, comments and the `raw` fallback string. The prose renderer strips ASCII control characters (including ESC, the ANSI-escape vector) from each such scalar with `strip_control_chars` (`src/util/text.rs`), which keeps `\n` and `\t`. Its sibling `replace_control_chars`, which replaces every control character including `\n` and `\t` with a space, backs the tabular formats. This happens at the render layer, not the client boundary, so **json keeps the raw server value verbatim** (dsp-cli/ADR-0003). Standoff text additionally passes through `html_to_text`. `html_to_text` and the sanitisers live in `src/util/`, a layer-neutral module, so the client boundary can use `html_to_text` without a client-to-render dependency (dsp-cli/ADR-0008).

## Considered Options

- **The simple schema**: rejected for the reasons under "Response schema". Reopening the choice means reopening this record.
- **A `--detail basic|full` enum instead of `--values`**: rejected; the CLI has no other detail-level concept.
- **Tabular `--values` as one metadata row plus a single serialised `values` cell**: rejected. The cell is not parseable.
- **No tabular value rendering at all**: rejected in favour of long-format value rows.
- **A `--comment` flag**: rejected. It would be inconsistent with the `--columns` mechanism for optional columns and add CLI surface for what is, in tabular formats, one more column.
- **The token name `list-item`**: rejected. `dsp vre vocabulary` names the same concept, and two words for it is the inconsistency to avoid.
- **A hardcoded label map for the common built-in file fields** (for example `hasStillImageFileValue`): deferred, not rejected. Built-in fields show their local name today; a map is added if those names read poorly.
- **Addressing a resource by ARK**: *deferred*. dsp-api has no ARK-to-IRI lookup; it comes back when one exists.
- **Instance search** (full-text, by label, Gravsearch): *deferred*. It comes back when a workflow needs it.

## Consequences

- The `--values` flag name, the prose `Values:` layout, the json `values` array shape, the value-type tokens and the `raw` fallback are public output contract.
- `describe --values` makes more than one HTTP request, the one read command that fans out to ontology and list-node fetches. This is documented for users (`dsp docs`) and bounded by deduplication and graceful degradation.
- Text detection is presence-based (`textValueAsXml`, else `valueAsString`) because `knora-api:hasTextValueType` is incompletely implemented in dsp-api and may be absent or unreliable. It is validated against real data by live tests, and the `raw` fallback keeps the read working if it proves insufficient.
- Instance-side value models live in `src/model/resource.rs`, distinct from the schema-side `Field` and `Cardinality`, but sharing the `ValueType` token vocabulary.

Enforced by: `dsp-cli/tests/resource_describe_values_http.rs` and `dsp-cli/tests/resource_describe_values_snapshots.rs` (field extraction, the value-type
matrix and `raw` fallback, label resolution and degradation, the prose, json and tabular shapes and comments), the other `resource_*` HTTP and snapshot tests,
and the unit tests of `dsp-cli/src/util/text.rs`, run by `just test` (**static-analysis**); the confinement of DSP-API names and the `schema=complex` constant
to `src/client/`, by **review**.
