# Concepts — the vocabulary dsp-cli speaks

This topic defines the words `dsp-cli` uses. They are chosen to match how
researchers describe their work, not the RDF/OWL terms the underlying API
exposes. If a command's output uses a word you don't recognise, it is defined
here.

## The containment chain

Everything in the VRE nests in one chain:

```
Project ─owns→ Data-model ─defines→ Resource-type ─defines→ Field ─holds→ Value
```

- **Project** — the top-level unit of organisation. Every piece of research data
  belongs to exactly one project. A project has three interchangeable identifiers
  (shortcode, shortname, IRI — see `dsp docs identifiers`) and is either `active`
  or `inactive`.
- **Data-model** — a project-specific schema that defines the resource-types and
  fields a project tracks. A project may have several. (DSP-API calls this an
  *ontology*.)
- **Resource-type** — a kind of thing the project records: "Letter", "Manuscript",
  "Composer". (DSP-API calls this a *class*.)
- **Field** — a slot on a resource-type that can hold values: a `title` field, a
  `sender` field. (DSP-API calls this a *property*.)
- **Value** — the actual data in a field on one specific resource: `"Hamlet"` in a
  `title` field. (DSP-API calls this a *Value*, of some subclass.) Pass `--values` to
  `dsp vre resource describe` to include the resource's field values in the output:
  one value per field line in prose, a `"values"` array in json, and one row per
  value in tabular formats (`csv`/`tsv`/`lines`), with a compact default column set
  (see `dsp docs output`). Resolving field and vocabulary-item labels may require
  additional server requests, which are deduplicated and graceful — a label that
  cannot be fetched falls back to the local field name or node IRI without aborting
  the describe.
- **Resource** — one instance of a resource-type: a specific letter record.
  Resources are instance-side data; `dsp vre resource list` lists resource instances
  within a project, and `dsp vre resource describe` fetches one resource's metadata
  envelope (label, type, IRI, ARK, dates, project, owner, visibility, and your access).

## Two more properties of a field

- **Value-type** — the kind of data a field accepts: `text`, `integer`, `decimal`,
  `boolean`, `date`, `time`, `uri`, `color`, `geoname`, `vocabulary-item`, `link` (to
  another resource), `still-image`, `moving-image`, `audio`, `document`, `archive`.
- **Cardinality** — how many values a field may hold, in dsp-tools' notation:
  `1` (exactly one), `0-1` (optional), `0-n` (any number), `1-n` (at least one).

`dsp vre resource-type describe` shows every field of a resource-type with its
value-type, cardinality, and label.

## Built-in vs project-defined

A **built-in** is a resource-type, field, or data-model that *every* DSP project
inherits from the platform itself — not from the project's own schema. The three
built-in data-models are `knora-api`, `standoff`, and `salsah-gui`. List commands
**hide built-ins by default**; pass `--include-builtins` to reveal them.

## Representation

A resource-type is a **representation** when it carries a binary asset, of kind
`still-image`, `moving-image`, `audio`, `document`, `archive`, or `text`. Most
resource-types are not representations. `dsp vre resource-type describe` surfaces a
resource-type's representation kind when it has one.

## How the resource-types of a data-model relate

Beyond the field list, resource-types relate to one another in two ways, both
surfaced by `dsp vre data-model structure`:

- a **link relation** — a link field on one resource-type points at another
  (e.g. a `sender` field on `Letter` points at `Person`);
- an **inheritance relation** — one resource-type extends another as a superclass.

## Resource permission facets

`dsp vre resource describe` surfaces two translated permission facets. These are
derived at the client boundary from the server's raw permission data; the raw
codes and group names never appear in dsp-cli output.

- **Visibility** — who can see the resource:
  - `public` — anonymous users can view it.
  - `public (restricted view)` — anonymous users can access it in reduced form
    (e.g. a low-resolution image) but not the full representation.
  - `logged-in users` — any authenticated user can see it.
  - `project members only` — only members of the owning project can see it.

- **Your access** — what the calling user can do with the resource:
  - `restricted view` — view in reduced form only.
  - `view` — read full content.
  - `edit` — modify values.
  - `delete` — delete the resource.
  - `manage` — full administrative control.

When a facet cannot be determined (absent or unparseable server data), it is
omitted from the output entirely.

## The DSP-API vocabulary mapping

`dsp-cli` translates DSP-API's RDF terms once, at the API boundary, and uses the
researcher-facing word everywhere above it. If you read DSP-API responses or
DSP-APP directly, this is the dictionary:

| dsp-cli says | DSP-API says |
|---|---|
| data-model | ontology |
| resource-type | class (resource class) |
| field | property |
| value-type | the subclass of `Value` |
| vocabulary | list |
| dump | export |

You should not need the right-hand column to use `dsp-cli`; it's here only to
bridge to the other DSP surfaces.

**One deliberate exception**: `dsp vre sparql query` does not translate at
all — SPARQL is its own W3C vocabulary, invoked directly, not DSP-API jargon
for a dsp-cli concept. See `dsp docs sparql`.

See also: `dsp docs identifiers`, `dsp docs workflows`, `dsp docs sparql`.
