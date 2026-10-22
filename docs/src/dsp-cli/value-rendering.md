# dsp-cli Value Rendering

How `dsp vre resource describe --values` turns the Values of a Resource into output. The decisions
behind it (complex schema, opt-in `--values`, field labels, output shapes, sanitisation) are in
dsp-cli/ADR-0013; this page holds the per-type rules, which are the specification of
`parse_value` in `dsp-cli/src/client/http.rs` and of the shared `render_value_content` helper in
`dsp-cli/src/render/value.rs`.

## Where each value comes from

Each Value's type is detected from the value object's `@type`. The value-type token (the
`value_type` in output) reuses the vocabulary in `dsp-cli/CONTEXT.md`: `text`, `integer`,
`decimal`, `boolean`, `date`, `time`, `uri`, `color`, `geoname`, `vocabulary-item`, `link`,
`still-image`, `moving-image`, `audio`, `document`, `archive`. The DSP-API class names (`TextValue`,
`LinkValue`, …) never leave `src/client/`.

Every datum key in the table below is a `knora-api:`-prefixed CURIE in the JSON-LD value object
(for example `knora-api:intValueAsInt`, `knora-api:linkValueHasTarget`). The JSON-LD `@context`
applies at the document level, not inside nested value objects, so the keys arrive as CURIE strings
and a lookup must use the full prefixed key. The prefix is omitted from the table cells for brevity.

## What counts as a field

Not every key on the resource is a user field. A key in the flattened `extra` map is a field if and
only if its value is a value object (or an array of value objects) whose `@type` is a knora-api
value class (a `knora-api:*Value`: `TextValue`, `IntValue`, `LinkValue`, `StillImageFileValue`, …).
Requiring a value-class `@type`, not merely any `@type`, excludes envelope-metadata objects that
carry an `xsd:` `@type`, notably `knora-api:versionArkUrl` (`{"@value":…,"@type":"xsd:anyURI"}`),
which would otherwise leak as a spurious `raw` field. Scalar system keys (`knora-api:isDeleted`) and
`@`-keys are skipped, since they have no value-class `@type`.

An explicit denylist skips value objects that are `*Value`-shaped but are not the resource's own
user fields:

- `knora-api:hasIncomingLinkValue` (reverse links);
- `knora-api:hasStandoffLinkToValue` and `knora-api:hasStandoffLinkValue` (standoff-internal links,
  both spellings);
- any value whose `@type` is `knora-api:DeletedValue`.

The envelope metadata captured as named DTO fields (`arkUrl`, `creationDate`,
`lastModificationDate`, `attachedToProject`, `attachedToUser`, `hasPermissions`,
`userHasPermission`, and `@context`) never reaches `extra`. Among `knora-api:`-prefixed keys, only
file values are genuine fields, and they pass the value-class test (`*FileValue`).

## Value-type rendering matrix

| value-type | DSP-API `@type` | Datum key(s) (complex schema) | dsp-cli rendering |
|---|---|---|---|
| `text` | `TextValue` | presence-based: `textValueAsXml` if present (formatted), stripped; else `valueAsString` (plain) | plain text; standoff XML stripped via `html_to_text` (lossy for custom standoff tags, which are dropped, not rendered) |
| `integer` | `IntValue` | `intValueAsInt` (JSON number) | the integer |
| `decimal` | `DecimalValue` | `decimalValueAsDecimal.@value` (string) | the decimal string (precision preserved) |
| `boolean` | `BooleanValue` | `booleanValueAsBoolean` | `true` / `false` |
| `date` | `DateValue` | `dateValueHasCalendar`, `dateValueHasStart{Year,Month,Day,Era}`, `dateValueHasEnd{…}` | calendar-aware human date; a range collapses to a single point when start equals end (below) |
| `time` | `TimeValue` | `timeValueAsTimeStamp.@value` | the timestamp string |
| `uri` | `UriValue` | `uriValueAsUri.@value` | the URI |
| `color` | `ColorValue` | `colorValueAsColor` (string) | the hex string |
| `geoname` | `GeonameValue` | `geonameValueAsGeonameCode` (string) | the geonames code |
| `vocabulary-item` | `ListValue` | `listValueAsListNode.@id` | resolved node label (via `/v2/node`); degrades to the node IRI |
| `link` | `LinkValue` | `linkValueHasTarget` (embedded) or `linkValueHasTargetIri.@id` | `<target label> [<target iri>]`; degrades to `<target iri>` alone (no brackets) when no label is available |
| `still-image` | `StillImageFileValue` (and the `StillImageExternalFileValue` and `StillImageVectorFileValue` siblings) | `fileValueHasFilename`, `fileValueAsUrl.@value`, `stillImageFileValueHasDimX/Y` | `<filename> (<W>×<H>) <url>` |
| `moving-image` | `MovingImageFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `audio` | `AudioFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `document` | `DocumentFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| `archive` | `ArchiveFileValue` | `fileValueHasFilename`, `fileValueAsUrl.@value` | `<filename> <url>` |
| long tail | `IntervalValue`, `GeomValue`, any unrecognised `*Value` | none | `raw` fallback (below) |

In the tabular formats a link renders with the `→` glyph (`→ <target label> [<target iri>]`), the
same glyph prose uses.

## Dates

A date is built from the structured calendar fields, not from `valueAsString`, which is not always
present and is machine-oriented. The format is `<start> – <end> (<Calendar>)`, collapsing to
`<point> (<Calendar>)` when start equals end (a field-by-field equality on the date point). A
point's precision follows the fields present: year, year-month, or year-month-day. The era (`CE` or
`BCE`) is appended when present. Examples: a single Gregorian year renders as `1489 CE (GREGORIAN)`,
a full Julian day as `1456-03-14 CE (JULIAN)`, and a range as `1489 CE – 1490 CE (GREGORIAN)`. The
exact glyphs are locked by snapshots; the shape is fixed here.

## The `raw` fallback

The long tail (interval, geometry, and any value-type DSP-API adds later) renders generically
instead of failing, so a read never breaks on an unknown type. For such a value, `value_type` is the
lowercased local name of the `@type` (for example `interval`). The datum (`raw.text`) is
`valueAsString` if present, else the value object's non-metadata literals joined, else a compact
JSON of the value object minus the standard metadata keys. It is never a hard error. This mirrors
the rule in `CONTEXT.md` that value-types outside the canonical set degrade to their lowercased
local name.

## Text values

Formatted and plain text are told apart by presence (`textValueAsXml` present means formatted,
otherwise `valueAsString`). Detection deliberately does not use the `knora-api:hasTextValueType`
discriminant: its server-side implementation in dsp-api is incompletely finished and may be absent
or unreliable on real or older resources. If presence-based detection proves insufficient on real
data, the `raw` fallback still keeps the read from breaking. Live tests cover
unformatted and formatted (standoff) text (`dsp-cli/tests/live_resource_describe.rs`).

## File values

The whole `*FileValue` family is matched by the leading kind of the `@type`: `StillImage*` is
`still-image` (the plain, `External` and `Vector` variants), `MovingImage*` is `moving-image`,
`Audio*` is `audio`, `Document*` is `document`, `Archive*` is `archive`, and `Text*FileValue` is
`document` (filename and URL). All share `fileValueHasFilename` and `fileValueAsUrl`; only
`still-image` adds dimensions. An unrecognised `*FileValue` degrades to `raw`, never to a hard
error.
