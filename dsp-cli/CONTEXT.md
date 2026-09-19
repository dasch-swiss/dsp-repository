# dsp-cli

A CLI for interacting with the DaSCH Service Platform (DSP), designed for AI-agent ergonomics while remaining human-readable. See `idea.md` for the high-level vision.
v1 covers the VRE only.

## Language

**Project**:
The top-level unit of organisation in the VRE. Every piece of research data belongs to exactly one project.
Identified by **three interchangeable identifiers**: a shortname (e.g. `incunabula`), a 4-character hex shortcode (e.g. `0803`), and an IRI.
**Wherever a command references a project — the `--project` flag — all three are accepted, never a subset.**
The HTTP client's `resolve_project` classifies the supplied value (4 hex digits → shortcode, IRI-shaped → IRI, otherwise shortname)
and resolves it against the matching `/admin/projects/{shortcode|shortname|iri}/…` endpoint.
Help text, docs, and the glossary must always list all three; "shortname or IRI" (dropping shortcode) is a recurring drift to guard against.

**Project status** (`active` / `inactive`):
A project is either **active** or **inactive** on the server. dsp-cli uses these words as the canonical display strings;
DSP-API represents the same concept as a boolean `status` field (`true` = active).
The wire boolean is translated to the dsp-cli enum at the HTTP client boundary and never appears above it. See `model::ProjectStatus`.

**Data-model count**:
Each project carries a count of its data-models (the number of ontologies in DSP-API terms). dsp-cli surfaces this as the `data_models` field on `Project`.
It is derived from the `ontologies` list in the DSP-API response (boundary translation: `ontologies.len()` → `data_models`) and shown in `dsp vre project list` output.
It is a count only — the actual data-model definitions are fetched separately via `dsp vre data-model list`.

**Project detail** (`ProjectDetail`):
The rich single-project view returned by `dsp vre project describe`.
Carries identity (IRI, shortcode, shortname), status, description, keywords, and a data-models summary (count + names).
Contrast **Project**, the lean `list` index projection which holds only identity, status, and a bare data-model count. See `model::ProjectDetail`.

**Project description** (`ProjectDescription`):
One language-tagged description value attached to a project: a `value` string (may contain HTML markup as returned by the server)
and an optional BCP-47 `language` tag (e.g. `"en"`).
The DSP-API represents these as `[{value, language}]` in the project object. See `model::ProjectDescription`.

**Data-model summary** (`DataModelSummary`):
A lean reference to a child data-model as surfaced by `project describe`: a short `name` (e.g. `beol`)
and the full ontology `iri` (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2`).
The name is derived from the IRI by the HTTP client layer at the ADR-0001 boundary — it is not a server-supplied field.
`project describe` shows count + names (e.g. `Data-models (4): beol, biblio, leibniz, newton`);
this is distinct from `list`'s bare `data_models` count. See `model::DataModelSummary`.

**Vocabulary**:
A project-scoped, hierarchical controlled vocabulary of named, labelled nodes (e.g. a "Period" vocabulary with nodes `Palaeolithic`, `Mesolithic`, …)
that a field can reference as its allowed values. Surfaced by `dsp vre vocabulary list` (the project's vocabularies)
and `dsp vre vocabulary describe` (one vocabulary's full node tree).
DSP-API calls this a "list"; DSP-APP's UI section is titled "Controlled vocabularies".
_Avoid_: list (DSP-API's own term — ambiguous, and collides with dsp-cli's `list` VERB used across every other noun-group; see `docs/adr/0001-vocabulary-divergence.md`).
See `model::Vocabulary`.

**Vocabulary root**:
The top node of a vocabulary's tree; carries the vocabulary's own identity (`iri`, `name`, `labels`, `comments`) via `model::VocabularyHeader`.
`dsp vre vocabulary list` shows one row per root. DSP-API calls it a root `ListNode` (`isRootNode: true`).

**Vocabulary node**:
One entry inside a vocabulary's tree, below the root (e.g. `Palaeolithic` inside a "Period" vocabulary), surfaced by `dsp vre vocabulary describe`.
DSP-API calls it a `listNode`; its wire `listNodePosition` is translated to dsp-cli's `position` field at the client boundary
(0-based, sibling order) — distinct from the renderer-derived `number` column (1-based dotted outline, e.g. `1.2`, `1.2.1`), which is presentation, not a model field.
See `model::VocabularyNode`.

**Data Model**:
A project-specific schema that defines the resource-types and fields tracked by the project. A project may have multiple data models.
The list projection (`dsp vre data-model list`) exposes: `name` (short label derived from the IRI, e.g. `beol`), `iri` (full ontology IRI),
`label` (human-readable English label from the JSON-LD metadata), and `last-modified` (ISO-8601 timestamp).
Sourced from the DSP-API `/v2/ontologies/metadata` JSON-LD endpoint.
The **detail** projection (`DataModelDetail`, surfaced by `dsp vre data-model describe`) adds a **summary of the data-model's child resource-types**
(count + names + labels) on top of the same identity/label/last-modified fields;
it is sourced from the richer `/v2/ontologies/allentities` JSON-LD endpoint (which returns the full schema) and is where resource-types first surface in the CLI.
See `model::DataModelDetail`.
_Avoid_: ontology (synonym in DSP-API responses, used loosely by DaSCH staff — but `data-model` is the canonical CLI term).

**Resource Type**:
A kind of resource defined inside a data model — for example "Manuscript" or "Composer".
_Avoid_: class, resource class (the DSP-API/DSP-APP terms; we diverge — see `docs/adr/0001-vocabulary-divergence.md`).

**Resource type** (`ResourceType`):
The list-projection of a resource-type as surfaced by `dsp vre resource-type list`: a short `name` (e.g. `letter`), the full `iri`, an optional `label`,
and an `is_builtin` boolean (false for project-defined types, true for the four user-instantiable platform built-ins).
Contrast `ResourceTypeSummary` (the describe-child inside `DataModelDetail`, which carries no `is_builtin` concept — it only ever comes from a project ontology).
The `is_builtin` field also appears as a `(built-in)` marker in prose output and as a column in tabular/JSON output. See `model::ResourceType`.

**Resource-type summary** (`ResourceTypeSummary`):
A lean reference to a child resource-type as surfaced by `dsp vre data-model describe`:
a short `name` (e.g. `letter`, derived from the resource-type's IRI at the ADR-0001 client boundary — not a server-supplied field),
the full `iri` (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2#letter`, expanded from the JSON-LD CURIE via the response's `@context`),
and an optional human `label` (e.g. `Basic Letter`).
`data-model describe` prose shows count + an aligned `name  label` list.
This is the describe-summary projection only — the **full** resource-type model (its fields, value-types, and cardinalities)
is surfaced by the `dsp vre resource-type describe` leaf command (`model::ResourceTypeDetail` / `Field`).
See `model::ResourceTypeSummary`.

**Resource**:
An instance of a resource-type — for example a specific manuscript record. A **Resource** is identified by an IRI (and optionally an ARK).
Resources are listed via `dsp vre resource list` within a project; a single resource's metadata envelope (label, type, IRI, ARK, dates, project, owner, and permission facets)
is fetched via `dsp vre resource describe`.
_Avoid_: resource instance.

**Visibility**:
A translated facet derived from a resource's permission ACL — answers *"who can see this resource?"*
dsp-cli surfaces one of four canonical values: `public` (anonymous access ≥ view),
`public (restricted view)` (anonymous access = restricted view only), `logged-in users` (only authenticated users can see it),
or `project members only` (no world-facing group is granted access).
The translation happens at the client boundary; raw ACL strings and `knora-admin:` group names never appear in output. When the ACL is absent or unparseable the field is omitted.
_Avoid_: permissions, hasPermissions, ACL, knora-admin group names, raw permission codes (RV/V/M/D/CR).

**Access** (your access):
A translated facet derived from the requesting caller's effective permission on a specific resource — answers *"what can I do with this resource?"*
dsp-cli surfaces one of five canonical values: `restricted view`, `view`, `edit`, `delete`, or `manage`.
The translation happens at the client boundary; the raw one-letter DSP-API codes never appear in output. When the field is absent or the code is unrecognised the field is omitted.
_Avoid_: userHasPermission, raw permission codes (RV/V/M/D/CR).

**Column**:
A named position in tabular output (`csv`, `tsv`, `lines`). Selected and reordered via `--columns=X,Y,Z`; the valid column names for a given command are listed in its `--help`.
Distinct from **Field** (see below): in `resource-type describe` tabular output the *rows* are fields and the *columns* are their attributes (name, value-type, cardinality, label).
"Column" is an output concept; "field" is a schema concept.
_Avoid_: field (for output positions), attribute.

**Field**:
A slot defined on a resource-type that can hold one or more values — for example a `title` field on "Manuscript".
Distinct from **Column** (an output position in tabular formats).
See `docs/adr/0003-chaining-and-output.md` for the rename rationale (`--fields` → `--columns`).
_Avoid_: property (the DSP-API/DSP-APP term we diverge from), attribute, slot.

**Value**:
The data held in a field on a specific resource — for example `"Hamlet"` in the `title` field of a manuscript record.
Values are rendered by `dsp vre resource describe --values` —
one value per field line in prose, a `"values"` array in json, and one row per value in tabular formats (`csv`, `tsv`, `lines`),
where the default columns are `field, field_label, value_type, value` (resource `label`/`iri` are available via `--columns` but hidden by default,
since they repeat identically on every row of a single-resource describe).
_Avoid_: data, content.

**Comment**:
An optional free-text annotation on a single *Value* (not the resource or field as a whole) —
for example, "reading uncertain" on a transcribed word.
It is surfaced by `dsp vre resource describe --values`: prose renders it on an indented line under the value;
json carries a `comment` key inside the value object, present only when the value has a comment (omitted, not `null`, otherwise);
tabular formats (`csv`, `tsv`, `lines`) add an opt-in `comment` column to the full column set (not the default), selectable via `--columns`.
Maps to DSP-API `knora-api:valueHasComment` at the client boundary; empirically rare in real data.
_Avoid_: note, annotation, remark.

**Value Type**:
The kind of data a field accepts: `text`, `integer`, `decimal`, `boolean`, `date`, `time`, `uri`, `color`, `geoname`, `vocabulary-item`,
`link` (to another resource), `still-image`, `moving-image`, `audio`, `document`, `archive`.
Value-types outside this set (rare — e.g. geometry or interval values that appear only on built-in types) degrade to their lowercased local name rather than failing the read.
The value-type is surfaced as the `value_type` key per value object in `resource describe --values` json output,
and determines which additional keys are present (e.g. `target_iri`/`target_label` for `link`,
`calendar`/`start`/`end` for `date`, `filename`/`url`/`width`/`height` for file values).
See `model::ValueType`.
_Avoid_: field-type (would conflict with the rejected schema/instance use), data-type.

**Cardinality**:
How many values a field may hold on a resource. dsp-cli uses dsp-tools' notation: `1` (exactly one — required, single), `0-1` (optional, single),
`0-n` (optional, repeatable), `1-n` (required, repeatable).
Translated at the client boundary from the DSP-API OWL restrictions (`owl:cardinality`/`owl:minCardinality`/`owl:maxCardinality`) on a resource-type's properties;
the OWL vocabulary never appears above the client layer. See `model::Cardinality`.

**Representation**:
Whether a resource-type carries a binary asset, and which kind — `still-image`, `moving-image`, `audio`, `document`, `archive`, or `text`.
A resource-type is a representation when it (transitively) subclasses the corresponding DSP-API base class (e.g. `StillImageRepresentation`);
dsp-cli detects this from the resource-type's flattened file-value field and surfaces it on `resource-type describe`.
Most resource-types are not representations. See `model::Representation`.

**Structure** — the overview of how a data-model's resource-types relate to and inherit from one another; surfaced by `dsp vre data-model structure` as a set of **relations**.
_Avoid_: graph (CS-flavoured), schema (overloaded with data-model).

**Relation** — a directed edge between two resource-types within (or out of) a data-model.
Two kinds: a **link relation** (a link field on the source points to the target resource-type; the field name labels the relation)
and an **inheritance relation** (the source extends the target as a superclass).
A relation has a `source`, a `target`, a `kind` (`link` | `inherits`), an optional `field` label (link relations only),
and an optional **target**-data-model tag for cross-data-model targets (`[to <dm>]`).
See `model::Relation`. _Avoid_: edge (CS term), link (ambiguous with link field / link value).

**Built-in**:
A resource-type, field, or data model that every DSP project inherits from the platform itself, not from the project's own schema.
The three built-in data models are `knora-api` (core types and properties), `standoff` (rich-text markup types), and `salsah-gui` (GUI attribute definitions).
`dsp vre data-model list` omits them by default; `--include-builtins` adds them.
The same distinction applies to resource-types and fields: built-ins come from `knora-api` (e.g. `Resource`, `StillImageRepresentation`, `hasComment`);
project-defined ones come from the project's own data models.

The four **user-instantiable built-in resource-types** — i.e. the built-in resource-types that a researcher can create instances of without defining them
in a project data model — are:

| name           | IRI                                                        | label (en)        |
|----------------|------------------------------------------------------------|-------------------|
| `Region`       | `http://api.knora.org/ontology/knora-api/v2#Region`        | Region            |
| `AudioSegment` | `http://api.knora.org/ontology/knora-api/v2#AudioSegment`  | Audio Annotation  |
| `VideoSegment` | `http://api.knora.org/ontology/knora-api/v2#VideoSegment`  | Video Annotation  |
| `LinkObj`      | `http://api.knora.org/ontology/knora-api/v2#LinkObj`       | Link Object       |

`dsp vre resource-type list` omits them by default; `--include-builtins` appends these four.
The `knora-base:Annotation` class (also marked `canBeInstantiated` in the DSP platform source) is **excluded** as deprecated —
in modern DaSCH tooling, "annotation" resources are expressed as `Region`, `AudioSegment`, or `VideoSegment`.
`dsp vre data-model list` is the first command where the data-model built-in distinction surfaces;
`dsp vre resource-type list` is the first command where the resource-type built-in distinction surfaces.
_Avoid_: inherited, system, knora-api (used too broadly).

**Dump**:
A server-produced bagit-zip archive containing a project's **structured data** and, by default, its **binary assets**.
Triggered asynchronously by the V3 export API and downloaded once the server finishes packaging it.
Opaque to dsp-cli — we trigger, poll, and stream the bytes to disk; we do not parse or repackage them.
_Avoid_: export (the DSP-API verb; reserved for dsp-tools' file-driven `export-project-data` workflow — see ADR-0004), backup, archive (too generic).

**Dump slot (server-wide)**:
The DSP-API holds **one dump at a time across the whole server instance** — not one per project. Only a single dump can exist at any moment.
A `POST …/exports` while the slot is occupied returns a `409` conflict identifying the **occupying project** (which may differ from the requested one).
A command for project B must never silently read, write, or destroy project A's dump; `dsp vre project dump` detects the foreign-slot case and refuses,
or — with `--replace --discard-other-project` — explicitly discards the occupying dump after warning the user.
_Avoid_: export slot, per-project quota (it is not per-project).

**Structured data (RDF data)**:
The non-binary content of a project, stored as RDF in the triplestore — its data-models, resources, and values, together with the project's administrative and permission records.
One of the two things a **Dump** contains; the portion `--skip-assets` keeps.
_Avoid_: metadata (see _Flagged ambiguities_ — in DaSCH "metadata" is the Repository/DPE descriptive layer, not the project's research data).

**Binary asset**:
An on-site media file (image, audio, video, document, archive) attached to a resource. The other thing a **Dump** contains; the portion `--skip-assets` omits.
_Avoid_: file, attachment (too loose).

**SPARQL query**:
A query written in SPARQL 1.1, DSP-API's underlying triplestore's own query language, sent raw and unabstracted via `dsp vre sparql query`
(`POST /admin/sparql/query`, `docs/adr/0016-sparql-passthrough.md`). Unlike every other dsp-cli noun, this is not translated at the ADR-0001 boundary:
SPARQL is a W3C standard the user invokes directly, not DSP-API jargon for a domain concept. The response is the store's own document,
byte-verbatim, in a store-negotiated media type — dsp-cli does not interpret it.
_Avoid_ using "SPARQL"/"query" (in this sense) outside `dsp vre sparql query`'s own surface and messages — see **triplestore**, below, for the same scoping.

**Passthrough**:
The design of `dsp vre sparql query`'s endpoint: DSP-API relays the request to the triplestore and the response back, unmodified except for
authentication, a default `Accept` header, and status classification (`docs/adr/0016-sparql-passthrough.md`). Requires a `SystemAdmin` token and is
off by default per deployment (`allow-sparql-passthrough`). "Dumb passthrough, intelligence lives in the driver" is the project brief's framing.
_Avoid_ using "passthrough" for any other command — it names this one specific design, not a general dsp-cli property.

**Triplestore**:
The RDF store underlying a DSP-API server (Fuseki in practice) — DSP-API's storage layer for **Structured data**, above. dsp-cli has no direct
connection to it; every access goes through DSP-API. This word is admissible in user-facing text **only** in `dsp vre sparql query`'s own surface,
docs and messages — the command where the user has explicitly opted into the layer below DSP-API's usual abstraction. Every other command must keep
speaking dsp-cli/DSP-API vocabulary, never "the triplestore".
_Avoid_ using "triplestore" anywhere else in help text, error messages, or docs — say "the server" instead.

## Authentication

**Server**:
The DSP-API endpoint the CLI talks to, identified by a URL or a built-in shortcut (e.g. `prod`, `dev`).

**Auth token**:
The JWT the DSP-API returns from `POST /v2/authentication` on successful login; cached for subsequent authenticated requests.

**Auth cache**:
`~/.config/dsp-cli/auth.toml`, mode `0600`, keyed by server URL; stores the token plus optional `user`, `acquired_at`, and `expires_at` fields.

**Login**:
`dsp auth login` exchanges a user identifier (email address, username, or user IRI) and password for a token via the DSP-API and writes the result to the auth cache.

**Logout**:
`dsp auth logout` removes the cached token for a server; idempotent — a no-op when nothing is cached.

**Set-token**:
Cache a pre-issued JWT (e.g. one harvested from a logged-in DSP web-app session) into the auth cache without a credential exchange.
Unlike *Login* (which posts username+password to obtain a token), `set-token` is handed an existing token on stdin, verifies it against the server, and stores it.
The cached `user` is taken from the token's `sub` claim (a user IRI), not an email/username.

**Token**:
`dsp auth token` prints the resolved bearer token (env `DSP_TOKEN` or the auth cache, same precedence as everywhere else) to stdout for piping —
the read-out counterpart to *Set-token*.
It makes no API call; a local, advisory `exp` check gates exit `3` (no cached/env token, or the token is locally detected as expired).

## Documentation

**Topic**:
A unit of embedded end-user documentation, addressed by a short name (`dsp-cli`, `dsp`, `concepts`, `connecting`, `output`, `dsp-tools`, `workflows`, `identifiers`, `errors`)
and printed by `dsp docs <topic>`.
Topics are conceptual and crosscutting — they explain *what a thing is* and *how to operate the CLI*, complementing `--help` (which explains *what a single command does*).
A topic is deliberately **not** the long form of a `--help` text. Topics are authored as markdown in `docs/topics/` and embedded into the binary at compile time (`include_str!`),
so documentation ships version-synced with the binary and needs no network.
Of the three documentation categories — ADRs (`docs/adr/`), developer docs (`docs/dev/`), and topics (`docs/topics/`) —
only **topics** are surfaced via `dsp docs`; the other two are contributor artefacts.
See [ADR-0010](./docs/adr/0010-embedded-documentation.md).
_Avoid_: page, article, manual, help-topic.

## Relationships

- The server's **Dump slot** is shared across all **Projects** — at most one **Dump** exists server-wide at any time
- A **Project** owns one or more **Data Models**
- A **Data Model** defines zero or more **Resource Types**
- A **Resource Type** defines zero or more **Fields** (with cardinality and a value-type per field)
- A **Resource** is an instance of exactly one **Resource Type** and lives inside exactly one **Project**
- For each **Field** of its **Resource Type**, a **Resource** holds zero or more **Values** (bounded by the field's cardinality)
- Every **Field** has exactly one **Value Type**
- Within a project, a resource-type may **reuse fields defined in a sibling data-model** (another data-model owned by the same project).
  `resource-type describe` resolves and tags these `[from <dm>]` so their source is visible. Cross-**project** field inheritance is not possible in DSP.
- The **relations** of a data-model — its link and inheritance edges between **Resource Types** — are surfaced by `dsp vre data-model structure`.
  Derived from the same `allentities` read as `data-model describe`.

## Example dialogue

> **Researcher:** "I want to model letters between composers."
> **Dev:** "OK — so in your **data model** you'll define a **resource-type** 'Letter' with **fields** like `sender`, `recipient`, `date_sent`, `body`.
> `sender` and `recipient` would be **link** fields pointing to a 'Person' resource-type. `body` would be a **text** field."
> **Researcher:** "And then when I upload Mozart's letter to Haydn?"
> **Dev:** "That creates a **resource** of resource-type 'Letter', with **values** in each field — the `sender` field holds a link-value pointing to the Mozart **resource**,
> the `body` field holds the text-value of the letter."

## Flagged ambiguities

- **"ontology" vs "data-model"** — the DaSCH knowledge hub treats these as related-but-distinct (ontology is broader/academic).
  DSP-API uses "ontology" for what we call "data-model". Resolution: `data-model` is the canonical CLI term;
  `ontology` is a synonym in API contexts and a near-synonym in DaSCH-internal speech.
- **"property" doubly overloaded** — in everyday speech, "property" gets confused with "value" (slot vs. content),
  and RDF-correctly the property *definition* would be a "property class"
  distinct from the property *use* on instances. We sidestep both by using `field` (definition slot) and `value` (data filling it). See ADR-0001.
- **"class" overloaded with OOP** — same family of problem. `resource-type` makes the schema/instance pairing (with `resource`) lexical instead of relying on context.
- **"metadata" — reserved; not the VRE research data.** At DaSCH "metadata" means the project-level *descriptive* metadata (title, PI, funder, keywords)
  the DSP Repository / DPE publishes and OAI-PMH harvests — the legacy `dsp-meta` layer.
  It is **not** the structured RDF data inside the VRE. Never call a project's RDF resources/values "metadata" (the `--skip-assets` dump help used to make exactly this mistake).
  The generic computing sense — JWT-claim metadata, filesystem metadata — is fine in internal code; the prohibition is about VRE research data on the user-facing surface.
