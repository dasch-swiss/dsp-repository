# FAIR Principles

What DPE does about each of the fifteen FAIR principles, where in the code it
happens, and what is not satisfied.

This page is organised by the principles, not by an assessor's metrics.
[Machine-Readable Metadata](./machine-readable-metadata.md) records what is
emitted and what F-UJI scored for it; those scores appear below only as
evidence. **A metric code is never the reason a principle is called satisfied.**
The principles are stable and published; metric numbering is one tool's, and
changes when that tool does.

No score is claimed anywhere on this page.

## Scope: three different things

FAIR principles are scoped `(meta)data`. DPE publishes metadata, so the three
things below have three different answers and every section keeps them apart.

- **Project metadata** — the description of one project's data holdings, served
  at `/dpe/projects/{shortcode}`, the page the project's ARK resolves to. This
  is what DPE publishes, and what a claim below is about unless it says
  otherwise.
- **Record-level pointers** — a project's records appear in its metadata as
  `hasPart` entries carrying record ARKs, and the files of fully open records as
  `distribution` entries carrying dsp-ingest URLs. Both are bounded prefixes of
  the record list. Neither is a record landing page.
- **The research data itself** — the bitstreams, and everything the VRE holds.
  DPE does not publish it. It serves metadata only, does not serve a file's
  bytes and does not redirect to them
  ([OAI-PMH](./oai-pmh.md#file-metadata-endpoint)); access to the data is
  governed outside this repository.

**Record landing pages do not exist.** `RecordGraph` is in
`shared/fair/src/graph.rs`, Record is a contract term in the root
`CONTEXT.md`, and both record writers (`shared/fair/src/record_datacite.rs`,
`shared/fair/src/record_dublin_core.rs`) feed OAI-PMH. But
`modules/dpe/server/src/router.rs` registers `/dpe/projects/{id}` and
`/dpe/records/{shortcode}/{record_id}/file`, and no page route for a record. A
record ARK therefore resolves to something outside DPE; what, is not settled in
this repository. This is a boundary of what has been built, stated so that no
section below reads as though records had pages.

**What this page does not assess.** The FAIR principles describe `(meta)data`,
not the organisation holding them. Repository trustworthiness — OAIS conformance
(root `CONTEXT.md`), CoreTrustSeal or any other certification — is a separate
question with its own criteria, and nothing here is evidence about it either
way.

## Three kinds of residual

The sections below separate what is not satisfied into three kinds, because
they are fixed by three different people.

- **Unbuilt** — engineering work nobody has done. A code change fixes it.
- **Data quality** — the code is correct and the corpus does not carry the
  value. The export, or the depositor, fixes it.
- **Institutional** — needs a policy, a registration or an organisational
  decision. No code change fixes it.

The [residuals ledger](./machine-readable-metadata.md#known-residuals) already
holds most of the institutional items in their honest form; this page points at
them rather than restating them.

**A decision already taken is not a residual of any kind.** Where one bounds
what is reachable, the section says so in its own words and keeps it out of the
list — the identifier scheme in F1, what that costs in F4, and DPE being
unauthenticated by design in A1.2.

## Findable

### F1

> (meta)data are assigned a globally unique and persistent identifier

**Project metadata: yes.** Every project has an ARK. `canonical_ark` in
`shared/fair/src/project_graph.rs` takes the recorded `pid` when it is neither a
Placeholder nor empty, and otherwise builds one from the shortcode. That one
value is the schema.org `@id` and the first `identifier` entry (`render` in
`shared/fair/src/schema_org.rs`), the DataCite `identifier` with
`identifierType="ARK"`, the `DC.identifier` meta tag, and the single `cite-as`
link (`project_to_link_set` in `shared/fair/src/signposting.rs`). Records carry
ARKs of their own (`RecordGraph::ark`, `shared/fair/src/graph.rs`).

The ARK path is never rewritten. `modules/dpe/core/src/ark.rs` normalises only
the *host*, and only on a deployment that publishes ARKs naming itself.

**The data: no.** A bitstream has no identifier of its own. It is named by a
dsp-ingest URL, in `distribution.contentUrl` and as `downloadUrl` from the
file-metadata endpoint. A URL is a location, and that location has a retirement
date ([Machine-Readable Metadata](./machine-readable-metadata.md#record-files)).

**A settled non-goal, not a residual.** The scheme is ARK throughout, and DaSCH
mints no DOIs. The reason is granularity and cost: every resource carries an
ARK, so DOI parity would mean hundreds of thousands of DOIs, which is
prohibitively expensive. That is a decision taken rather than a task
outstanding, and it is why no section of this page treats a DOI as something
missing. F4 records where it bites.

**Not satisfied**

- Bitstreams have no persistent identifier. *Unbuilt.*
- An ARK is persistent because DaSCH goes on resolving it. That is an
  organisational commitment, and nothing in this code makes it one.
  *Institutional.*

### F2

> data are described with rich metadata (defined by R1 below)

**What DPE does.** A project file has 37 members (root `CONTEXT.md`).
`ProjectGraph::build` in `shared/fair/src/project_graph.rs` resolves them into
one graph: titles and alternative names, description and abstract, keywords,
disciplines, creators and contributors with their ORCID and GND identifiers and
resolved affiliations, start and end dates, temporal coverage resolved to W3CDTF
ranges, spatial coverage, data language, licences, access rights, funding,
publications, the project website, a how-to-cite string, status, and the part
list. Every representation is a projection of that one graph.

**Where.** `ProjectGraph::build`; `project_to_schema_org`
(`shared/fair/src/schema_org.rs`); `project_to_datacite` and
`project_to_datacite_json`; `project_to_dublin_core` and
`project_to_dublin_core_meta`.

**Not satisfied**

- **Eight recorded fields reach no machine-readable representation.**
  `ProjectRaw` (`shared/metadata/src/project.rs`) carries `provenance`,
  `data_management_plan`, `contact_point`, `type_of_data`, `short_description`,
  `documentation_material`, `additional_material` and `image_credit`; none of
  them appears anywhere in `shared/fair/src` outside its test fixtures, and
  `ProjectGraph` has no field for any of them. The human page renders several —
  `provenance_card` in
  `modules/dpe/web/src/pages/project/components/project_details_tabs/dataset_overview_section/mod.rs`,
  the DMP and contact point in
  `modules/dpe/web/src/pages/project/components/project_sidebar/mod.rs`. A
  machine therefore sees less than a person does. `clusters` and `collections`
  are absent the same way; they are I3's point rather than this one.
  `type_of_data` is the consequential one: the project's DataCite
  `resourceType` is the constant `"Research Project"` and
  `resourceTypeGeneral` the constant `"Project"` (`shared/fair/src/datacite.rs`),
  so the recorded kind of the data never reaches a representation. *Unbuilt.*
- The DaSCH metadata schema records no variable or observation-type element, so
  nothing maps to `variableMeasured`. *Data quality*, at the schema level.
  (Evidence: `R1-01MD-4` in the residuals ledger.)
- How rich the metadata is varies per project. A field the corpus records as a
  Placeholder yields no key at all (`helpers::real`), which is truthful and also
  means F2 is answered differently for different projects.

### F3

> metadata clearly and explicitly include the identifier of the data it describes

**The project's own identifier: yes.** The ARK is both `@id` and the first
`identifier` entry, and `url` carries the landing page beside it (`render` in
`shared/fair/src/schema_org.rs`).

**The identifiers of its records: partly.** `part_nodes` emits one `Dataset`
node per record with the record's ARK as `@id`, but only over a prefix: 100
entries in the embedded block (`HAS_PART_CAP` in
`modules/dpe/server/src/metadata.rs`) and as many as fit 4,000,000 bytes in
`/metadata.jsonld` (`JSON_LD_BYTE_BUDGET`). The complete list is the OAI set
`project:{shortcode}`, reachable from the document in two hops over links it
already carries.

**Pointers to the bitstreams: a location, not an identifier.**
`distribution_nodes` emits one `DataDownload` per file of a listed record, with
`contentUrl`, and only for a record the corpus records as `Full Open Access`
(`publishable_file` in `shared/fair/src/graph.rs`). `hasPart` and `distribution`
take the same prefix in the same order, so the files described are the files of
the records listed.

*Evidence, not the claim:* `F3-01M` moved from 0/1 to 1/1 for the two projects
whose records carry files once `distribution` was emitted; see the
[assessment results](./machine-readable-metadata.md#assessment-results).

**Not satisfied**

- What the metadata includes for a bitstream is a retrieval URL, not an
  identifier, and that URL names a host with a retirement date. *Unbuilt* (a
  DPE-owned stable download URL is considered and rejected in
  [Machine-Readable Metadata](./machine-readable-metadata.md#record-files); it
  is an ADR question).
- Files of records the corpus does not record as fully open are not pointed at
  at all. That is ADR-0005's decision rather than a defect, and it does mean F3
  is unanswered for those records.

### F4

> (meta)data are registered or indexed in a searchable resource

**What DPE does.** Two routes, neither of which needs a DOI.

- **Harvesting.** DPE is an OAI-PMH 2.0 data provider at `/dpe/oai`
  (`modules/dpe/api-oai/`): all six verbs, `oai_dc` and `oai_datacite`, and
  selective harvesting over the sets `project:{shortcode}` and `cluster:{id}`.
  Any aggregator can take the whole corpus. F-UJI's `F4-01M-1` passes on this
  alone.
- **Search-engine indexing.** The embedded schema.org `Dataset` block is the
  form Google Dataset Search ingests, and at 3.7% of Googlebot's 2 MB crawl
  budget for the largest committed project it has room ([Machine-Readable
  Metadata](./machine-readable-metadata.md#who-reads-which-document-and-what-each-one-can-take)).
  The project graph also names the catalogue it belongs to, as
  `includedInDataCatalog`.

**Where.** `modules/dpe/server/src/router.rs` (`/dpe/oai`),
`modules/dpe/api-oai/src/handlers/`, [OAI-PMH](./oai-pmh.md); `render` in
`shared/fair/src/schema_org.rs` for `includedInDataCatalog`.

**A permanent limit, from the identifier scheme.** DaSCH mints no DOIs, for the
reason F1 records, and registration in a DOI-based registry therefore does not
happen. `F4-01M-2` asks for registration in a major research-data registry, and
in the pinned F-UJI image DataCite is the only one of the three routes actually
queried — Mendeley
fails DNS resolution inside the container and the Google Search cache database
ships empty, so that sub-test is partly a property of how the container is
provisioned
(`docs/specs/2026-09-15-fair-metadata-exposure/01-feat-fair-metadata-exposure-plan-journal.md`).
DataCite registration is keyed on a DOI prefix. So as F-UJI measures F4 there is
a ceiling, it follows from the identifier decision, and no work on this code
moves it. It is not an open item.

**Not satisfied**

- Being *indexable* is not being *indexed*. DPE serves no sitemap; nothing in
  the tree advertises the catalogue to a crawler, which then has to find every
  project page by following links. *Unbuilt.*
- Not indexed by a search engine. *Institutional.* (Evidence: FAIR Champion's
  *DiscoverableInBing*.)

## Accessible

### A1

> (meta)data are retrievable by their identifier using a standardized communications protocol

**Project metadata: yes.** The ARK resolves by HTTP redirect to the landing
page. In production the resolver is `ark.dasch.swiss`; a deployment that
publishes ARKs naming itself mounts its own
(`modules/dpe/server/src/ark.rs::resolve_project`, registered in `router.rs`
only when `DPE_ARK_RESOLVER_BASE_URL` is set). The page carries its metadata
inline, and two machine-readable representations sit beside it at their own
URLs, reachable directly or by a single `303` when `Accept` prefers one
(`landing_page` in `modules/dpe/server/src/metadata.rs`, over the decision table
in `shared/fair/src/negotiate.rs`). Every answer from that route carries
`Vary: Accept` (`project_page_handler` in `modules/dpe/server/src/main.rs`).
OAI-PMH is a second retrieval protocol over the same metadata.

**The data: not from DPE.** DPE serves metadata only. A consumer reads
`downloadUrl` from `/dpe/records/{shortcode}/{record_id}/file`
(`modules/dpe/server/src/downloads.rs`) and then fetches the bytes from
dsp-ingest itself.

**Not satisfied**

- Nothing in DPE retrieves data by its identifier, because no bitstream has one
  (F1). What it offers is retrieval by location.

### A1.1

> the protocol is open, free, and universally implementable

**What DPE does.** HTTP throughout, and OAI-PMH 2.0 over HTTP. No client
library, no key, no account, no registration: DPE is the public,
unauthenticated, read-only server of the Access Area
(`modules/dpe/CONTEXT.md`). The representation routes and `/dpe/oai` share a
per-IP rate limit (`rate_limited_router` in `modules/dpe/server/src/router.rs`,
`DPE_OAI_RATE_LIMIT_*`), which bounds request rate and gates nothing.

**Not satisfied.** Nothing identified. This is the one section with no residual,
and it is the cheapest of the fifteen to satisfy.

### A1.2

> the protocol allows for an authentication and authorization procedure, where necessary

This section is the one most easily answered with the wrong evidence, so the
wrong evidence is named first.

**What DPE does, and what it does not mean.** DPE publishes access conditions:
`isAccessibleForFree` and `conditionsOfAccess` in the schema.org graph, and
`DC.accessRights` as a COAR access-right URI (`helpers::coar_access_right`). It
also withholds the file pointer of any record the corpus does not record as
`Full Open Access` (`publishable_file` in `shared/fair/src/graph.rs`). **None of
that is an authentication or authorization procedure.** `publishable_file`'s own
documentation says so: it governs what is *published*, not what is reachable —
dsp-ingest serves those URLs to anyone holding one, and DPE's own file-metadata
endpoint returns the same URL for any record with a file.

DPE itself needs no procedure: everything it serves is published metadata.
Authorization, where it is necessary, lives outside this repository — in the
VRE, in dsp-ingest, and in the Deposit Area, whose editor has its own
([Authentication](../editor/authentication.md)). That is the producer side, not
access to published data.

**Not answered by DPE**

- DPE implements no authenticated path to restricted data, and does not mediate
  access to it by any procedure. HTTP would allow one; nothing here uses it.
  That is a boundary rather than a residual — `modules/dpe/CONTEXT.md` defines
  DPE as the public, unauthenticated, read-only server — and whether the Access
  Area should offer one is a question no decision record answers yet.

### A2

> metadata are accessible, even when the data are no longer available

**What DPE does.** Metadata availability does not depend on bitstream
availability. DPE serves the committed corpus under `modules/dpe/server/data/`
rather than reading the archive, and a project whose records carry no file has a
complete landing page all the same — 081C, with 27,026 records and not one file,
is the worked case
([assessment results](./machine-readable-metadata.md#assessment-results)).

That is a property of today's deployment, not a commitment. The rest of this
section is what is missing.

**Not satisfied.** This is the weakest of the fifteen.

- **No tombstone.** A shortcode that names no project gets an always-200
  "Project Not Found" page (`landing_page` in
  `modules/dpe/server/src/metadata.rs`) — not a `410 Gone` carrying the metadata
  of what used to be there. Nothing in the tree distinguishes "never existed"
  from "withdrawn". *Unbuilt.*
- **OAI-PMH says so out loud.** `Identify` advertises
  `<deletedRecord>no</deletedRecord>` (`OaiXmlBuilder::write_identify` in
  `modules/dpe/api-oai/src/xml.rs`), which tells a harvester the repository
  maintains no information about deletions. *Unbuilt.*
- **No metadata persistence policy** to point a `persistencePolicy` link at.
  *Institutional.* (Evidence: FAIR Champion's *MetadataPersistence*.)

## Interoperable

### I1

> (meta)data use a formal, accessible, shared, and broadly applicable language for knowledge representation.

**What DPE does.** JSON-LD, twice: embedded in the page head as
`<script type="application/ld+json">` and standalone at
`/dpe/projects/{shortcode}/metadata.jsonld`. The `@context` is
`https://schema.org` plus a `prov:` binding (`render` in
`shared/fair/src/schema_org.rs`). Beside it, DataCite kernel 4 as JSON and as
XML, and Dublin Core as `<meta>` tags and as `oai_dc`. An assessor reads the
JSON-LD as RDF, which is why several property shapes are node objects rather
than strings ([Machine-Readable
Metadata](./machine-readable-metadata.md#schemaorg-json-ld)).

**Where.** `shared/fair/src/schema_org.rs` (`render`, `script_safe_json`);
`modules/dpe/server/src/metadata.rs` (`render`, `project_json_ld`);
`shared/fair/src/datacite_json.rs`; `shared/fair/src/dublin_core_meta.rs`.

**Not satisfied**

- No Turtle and no RDF/XML. ADR-0005 leaves Turtle open ("if it earns its
  place") and it has not been served. A consumer wanting RDF runs the JSON-LD
  through a processor. *Unbuilt, deliberately deferred.*
- The JSON-LD representation is a bounded prefix and **does not say so**.
  schema.org has no property meaning "this collection is partial"; Hydra's
  `totalItems` was considered and rejected. A consumer reading the document
  alone cannot tell; one that follows its links can recover the complete list.
  The reasoning is in [A byte
  budget](./machine-readable-metadata.md#a-byte-budget).

### I2

> (meta)data use vocabularies that follow FAIR principles

**What DPE does.** Published, versioned, resolvable vocabularies throughout:

| Vocabulary | Used for | Where |
|---|---|---|
| schema.org | the root graph and every node in it | `shared/fair/src/schema_org.rs` |
| Dublin Core elements | `<meta name="DC.*">` and the `oai_dc` record | `shared/fair/src/dublin_core_meta.rs`, `dublin_core.rs` |
| DataCite kernel 4 | `metadata.datacite.json` and `oai_datacite` | `shared/fair/src/datacite.rs`, `datacite_json.rs` |
| PROV-O | `prov:wasAttributedTo` | `PROV_NAMESPACE`, `attributed_to` in `schema_org.rs` |
| SPDX | licence identifiers (`rightsIdentifierScheme="SPDX"`) | `shared/fair/src/datacite.rs`, `helpers::license_identifier_to_label` |
| COAR access rights | `DC.accessRights` | `helpers::coar_access_right` |
| ORCID, GND | agent identifiers | `person_to_agent` in `shared/fair/src/resolve.rs` |
| GND, STW, LCSH, AAT | subject schemes, inferred from an authority URL | `helpers::infer_subject_scheme` |
| ChronOntology, W3CDTF | temporal coverage and its resolved ranges | `shared_metadata::temporal_coverage` |
| FAIR Signposting (level 1) | the typed link set | `shared/fair/src/signposting.rs` |
| OAI-PMH 2.0 | the harvesting protocol | `modules/dpe/api-oai/` |

*Evidence, not the claim:* `I2-01M` moved from 0/1 to 1/1 when PROV was added,
because F-UJI's linked-data registry lists that namespace.

**Not satisfied**

- **Contributor roles are free text.** `CONTRIBUTOR_ROLES` in
  `shared/metadata/src/project.rs` is documented as "an offer, not a
  constraint", and the committed data spells the same role several ways, crams
  several into one entry, and uses prose in place of a role.
  `helpers::map_contributor_type` maps what it recognises onto DataCite's
  vocabulary and falls back to `Other`. *Data quality.*
- **Organizations have no identifier.** `resolve_agent` gives an organization
  `name_identifiers: vec![]`, and an affiliation is emitted as an `Organization`
  name with nothing else (`agent_node` in `schema_org.rs`). No ROR. The
  organization files carry a homepage and a postal address and no persistent
  identifier (`modules/dpe/server/data/organizations/`). *Unbuilt*, and
  *data quality* behind it.
- **Person identifiers are sparse, and two are typed away.** 96 of the 416
  committed person files record `"type": "ORCID"` in `sameAs`. Two more hold an
  `orcid.org` URL typed `"URL"` (`person-146`, `person-246`); `person_to_agent`
  matches on the type string, so those two agents get no `@id`, no `author` link
  and no `prov:wasAttributedTo` entry although the identifier is there. Every
  section below that depends on an agent IRI inherits this. *Data quality.*
- Keywords are free text, and disciplines carry a subject scheme only where the
  corpus records an authority URL.

### I3

> (meta)data include qualified references to other (meta)data

**What DPE does.** This is the best-served of the fifteen.

- A record's DataCite record relates it to its project: `relationType="IsPartOf"`,
  `relatedIdentifierType="ARK"` (`record_to_datacite` in
  `shared/fair/src/record_datacite.rs`).
- The project's schema.org graph carries `hasPart` over record ARKs
  (`part_nodes`), `citation` nodes whose `@id` is a publication's PID when one is
  recorded (`citation_nodes`), `spatialCoverage` `Place` nodes with `sameAs` to
  an authority URL (`spatial_nodes`), agent nodes whose `@id` is the agent's
  ORCID (`agent_node`), `prov:wasAttributedTo` pointing at *those same nodes*
  rather than describing copies (`attributed_to`), `includedInDataCatalog`, and
  `sameAs` to the recorded PID when it differs from the resolved ARK.
- The Signposting set adds `cite-as`, one `describedby` per representation,
  `author` per ORCID, and `license` when exactly one applies; each representation
  answers with `describes` back at the page (`representation_to_link_set`).

**Not satisfied**

- **A project's DataCite record carries no `relatedIdentifiers` at all.**
  `shared/fair/src/datacite.rs` holds the unfilled note: *RelatedIdentifiers —
  should contain parent Project Cluster ARK. TODO: Populate once Project Cluster
  data is available.* So a project's cluster membership, its records and its
  publications are all absent from the DataCite representations, although the
  schema.org graph carries the last two. *Unbuilt.*
- A reference to an agent is qualified only when that agent has an ORCID. One
  without stays a blank node and is left out of `prov:wasAttributedTo`
  entirely — naming some agents does not deny the others, but it does mean the
  reference is unqualified for most of them (see I2).
- Nothing relates one project to another, although Clusters and Collections
  exist and are served as OAI sets. *Unbuilt.*

## Reusable

### R1

> meta(data) are richly described with a plurality of accurate and relevant attributes

(Quoted as published; the paper's Box 2 reads `meta(data)` here.)

**What DPE does.** The attribute list is F2's. What R1 adds is *accurate*, and
three rules carry that:

- **Nothing is invented for a score** (ADR-0005). No representation asserts a
  fact the corpus does not record.
- **A Placeholder yields no key.** `helpers::real` is the test every writer that
  must not assert a falsehood applies to a corpus string; `MISSING` never
  reaches a `<meta>` tag or a JSON-LD value.
- **One resolved graph feeds every representation**, so two of them cannot
  disagree — asserted for every committed project by
  `every_representation_of_a_committed_object_agrees_with_the_others` in
  `dpe-api-oai`.

**Not satisfied**

- The eight unemitted fields of F2. *Unbuilt.*
- No variable or observation-type element exists to describe. *Data quality*, at
  the schema level.
- **Two mandatory-field fallbacks are not facts, and a consumer cannot tell.**
  DataCite makes `publicationYear` and at least one creator mandatory, so the
  representations that rule governs fall back to the year `2015`
  (`FALLBACK_PUBLICATION_YEAR` in `shared/fair/src/graph.rs`) and to an
  organizational `DaSCH` (`creators_with_fallback`). Both are reached only
  through those named helpers, and schema.org's optional `datePublished`
  deliberately omits the key instead — but a consumer reading a DataCite record
  for a project that records no usable date reads `2015` as though it were
  recorded. *Data quality*: the fallback fires only where the corpus records no
  usable value, and DataCite's mandatory field leaves the writer no way to mark
  one as supplied.

### R1.1

> (meta)data are released with a clear and accessible data usage license

**The data: yes, where the corpus records a licence.** `license` as a node
object `{"@id": <SPDX URI>}` in the schema.org graph — a node and not a string,
because schema.org's remote context does not coerce `license` to `@id` and a
consumer asking for a licence *resource* would find none. Beside it: `DC.rights`,
DataCite `rightsList` with `rightsIdentifierScheme="SPDX"`, a Signposting
`license` link, and a `license` on each `DataDownload` carrying **the record's
own** licence, which across the corpus is not always the project's.

**The metadata: no.** Nothing in this tree licenses the metadata records
themselves. `Identify` emits `repositoryName`, `baseURL`, `protocolVersion`,
`adminEmail`, `earliestDatestamp`, `deletedRecord` and `granularity`, and no
rights block (`OaiXmlBuilder::write_identify`). No landing page says anything
about reuse of the metadata. A harvester taking `oai_dc` records away has no
licence for them.

**Not satisfied**

- **The metadata has no licence.** This needs a decision first and one field
  after. *Institutional.*
- A project that records no licence emits no `license` key — truthful, and R1.1
  is then unanswered for that project. *Data quality.*
- A project licensed several ways gets no Signposting `license` link at all,
  because the profile allows at most one and has no way to say "these apply
  together" (`project_to_link_set`). The JSON-LD still lists them.

### R1.2

> (meta)data are associated with detailed provenance

**What DPE does.** `prov:wasAttributedTo` over DaSCH and every credited creator
that has an ORCID, with `prov:` bound in the `@context` (`PROV_NAMESPACE` and
`attributed_to` in `shared/fair/src/schema_org.rs`). Nothing new is asserted:
`creator` and `publisher` already name those agents, and PROV-O restates the
relation in the vocabulary a consumer asking about provenance reads. Beside it:
`producer` as a `ResearchProject` with start and end dates and members
(`producer_node`), `funder` and `funding` as `MonetaryGrant` nodes,
`datePublished` where a year is recorded, and DataCite's `Created`, `Issued` and
`Collected` dates.

*Evidence, not the claim:* `R1.2-01M` moved from 1/2 to 2/2 on the run that
added PROV.

**Not satisfied**

- **The project's recorded provenance statement is not emitted anywhere
  machine-readable.** `ProjectRaw.provenance` is free text, the human page
  renders it as a card (`provenance_card`), and no `shared-fair` writer reads
  it. The one field named for this principle is the one the machine-readable
  metadata leaves out. *Unbuilt.*
- **Nothing records the provenance of the metadata.** `ProjectRaw` carries no
  created or modified date (`shared/metadata/src/project.rs`), so who curated a
  project's description, when, and through what revision is not recorded and
  cannot be emitted. Records do carry `date_created` and `date_modified`
  (`RecordGraph`); projects do not. *Unbuilt*, and a contract change before it is
  a writer change.
- `prov:generatedAtTime` is deliberately not emitted: the corpus records a
  publication *year*, not a generation time, so the statement would be
  approximately true at best
  ([Provenance](./machine-readable-metadata.md#provenance)).
- A creator without an ORCID is not attributed, because there is no node to
  point at (see I2 for how many that is).

### R1.3

> (meta)data meet domain-relevant community standards

**What DPE does.** The general research-data standards are met and checked:

- **DataCite kernel 4**, `schemaVersion` 4.6, `datacentreSymbol` `DASCH.DSP`.
  The JSON is validated against DataCite's own JSON schema and the XML against
  `datacite-kernel-4.xsd` for every committed project, by the corpus-wide test
  in `dpe-api-oai`.
- **OAI-PMH 2.0**, validated against `OAI-PMH.xsd`.
- **Dublin Core**, **schema.org `Dataset`** (the shape Google Dataset Search
  reads), **FAIR Signposting level 1**, **COAR access rights**, **SPDX**,
  **W3CDTF**.

**Not satisfied**

- **No humanities-domain standard is served.** No representation offers TEI,
  CIDOC-CRM, EAD or CMDI. The corpus records `XML (TEI)` as a `typeOfData` value
  (`general_data_type` in `shared/fair/src/graph.rs`), so content in one of
  those standards is described by metadata that does not speak it. *Unbuilt.*
- **File formats are described only where the export records a MIME type.**
  `encodingFormat` is omitted rather than guessed. All 4,062 files of project
  0803 carry no `mimeType`, and dsp-ingest serves them as
  `application/octet-stream`, so neither end supplies it. *Data quality*, at
  both ends. (Evidence: `R1.3-02D`.)

## Sources

The principle wording quoted above is Box 2 of:

> Wilkinson, M. D. *et al.* "The FAIR Guiding Principles for scientific data
> management and stewardship". *Scientific Data* **3**, 160018 (2016).
> <https://doi.org/10.1038/sdata.2016.18>

GO FAIR restates the same fifteen principles with a commentary on each, at
<https://www.go-fair.org/fair-principles/>.

The assessors named in the evidence notes are separate from the principles.
F-UJI's metrics are cited in its own result payload as
<https://doi.org/10.5281/zenodo.6461229>; FAIR Champion's indicators are defined
in the [FAIR Maturity Indicators
index](https://fairmetrics.github.io/Metrics/landingpages/general/index.html).
What each of them scored, when, and against what, is in
[Machine-Readable Metadata](./machine-readable-metadata.md#assessment-results).
