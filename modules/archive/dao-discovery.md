# DaSCH Archival Ontology (DAO) — Working Decision Log

*Working document, updated after each interview decision. Each section records what we decided and why; decisions can be revised but the revision is logged, not silently overwritten.*

**Status:** in-progress design conversation
**Last updated:** decision 28 added (`Tombstoned` and `Redacted` as two distinct events to reconcile GDPR right-to-erasure with WORM); decisions 6 and 20 amended with PREMIS-alignment notes (DAO is conceptually built on PREMIS; the `Representation → File → Bitstream` hierarchy is collapsed deliberately).

**Reference standards.** Every decision below is aligned with OAIS (CCSDS 650.0-M-3) and PREMIS 3.0 unless an explicit deviation is documented. Verbatim text extracts of both standards are checked in alongside this doc — see [`standards/README.md`](./standards/README.md) for citation conventions.

---

## 1. Scope and framing

The vocabulary discussion covers two distinct concerns:

1. **Archival Packaging** — the container format crossing VRE→Repository and inter-component boundaries. OAIS SIP/AIP/DIP, BagIt, METS, RO-Crate live here.
2. **Archival Content Shape** — the model of what is *inside* the package: a cleaned-up, simplified RDF representation of project data, structured so a future reader can reconstruct meaning without semantic loss.

Both are in scope. They are independent — packaging can change without changing content shape, and vice versa.

**Why we're not anchoring on RiC-O:** RiC-O is a records-management ontology (fonds, agents, activities, provenance chains). DaSCH content is research data — long-lived datasets produced by humanities researchers in the VRE over years, with periodic publication. The "record as one-and-done" frame does not match. Versioning of long-lived Resources and Assets must be a first-class property of the archival format, not bolted on.

---

## 1a. Architectural context (relevant to DAO scope)

The Repository topology DAO must fit into:

- **RDU-Tooling** (Producer-side, local) — prepares data into Submission packages.
- **Archive** (OAIS Archive Component; covers OAIS Ingest, Archival Storage, and the supporting functional entities Preservation Planning, Data Management, Administration) — ingest, validation, audit; holds the preservation-grade record (`dao:Representation`s, each containing one or more **Preservation Files**). **Working assumption: self-built (per decision 17, revised).** Storage architecture: OCFL on filesystem as source of truth; SQLite as read-side cache (§3.7). Deployed as two services within the same bounded context: an **Ingest Area** (producer-facing; async upload + validation gate against DAO SHACL and `DepositAgreement`; emits `DepositionAccepted` only on validation success — the OAIS *Ingest* entity) and the rest of the Archive (event log, OCFL, public APIs — the OAIS *Archival Storage* entity). Split is operational (bandwidth, failure isolation, "as long as it takes" processing for large Depositions); both speak DAO directly.
- **Access Area** (OAIS *Access* entity) — produces DIPs for Consumers. Materializes **Service Files** (and Service Projections for non-file consumers) per subdomain; serves **Access Files** on demand. Subscribes to Archive events; push-from-Archive (resolved 2026-05-13, §9.3). Subdomains: **IIIF** (SIPI), **HTML / Web Discovery** (DPE), **Custom Presentation** (CPE), **Asset / Download**, **SPARQL**. Each subdomain corresponds to one OAIS DIP shape.

Two committed architectural patterns shape DAO's design space:

- **Domain Driven Design** — bounded contexts, ubiquitous language, aggregates. **DAO is the ubiquitous language of the Archive context.** The Producer-side contexts (RDU-Tooling, VRE) and the Access Area context (with its subdomains listed above) have their own internal vocabularies. **DAO terms appear at boundaries** (submissions arriving at the Archive, projection events leaving the Archive) — the lingua franca of context seams, not the universal language. (Anti-corruption layer not required under the self-built working assumption.) **Subdomain ≠ bounded context**: a subdomain is a problem-side slice; a bounded context is a solution-side language boundary. The Access Area's subdomains are implemented in separate codebases (`modules/dpe/`, future SIPI, etc.) but share the Access Area's parent ubiquitous language plus subdomain-specific extensions.
- **Event Sourcing** — events are the source of truth in the Archive; **Service Files and Service Projections** in the Access Area are derived (replayable) views.

**Project metadata format is decided**: RDF/Turtle with SHACL validation. DAO is one such ontology (the canonical archival one). The Archive treats project-level supplementary metadata as black-box storage.

**CoreTrustSeal is the certification target.** Several DAO design choices (immutable event log, fixity events, format migration provenance, no in-place migration) directly produce CoreTrustSeal evidence. Where this linkage applies, it is noted.

### Scope clarification: DAO governs the Archive, not the Access Area

DAO is the schema of what is **preserved in the Archive** (`dao:Representation`s, the **Preservation Files** they contain, and their descriptive/preservation metadata). It is **not** the schema of **Service Files / Service Projections** in the Access Area, nor of presentation views produced by Access Area subdomains (HTML/Web Discovery, Custom Presentation, IIIF Manifests, etc.).

- **`dao:Representation`s** (containing one or more Preservation Files) conform to DAO. Versioned. WORM. Source of truth.
- **Service Files / Service Projections** are derived projections of Representation Versions, shaped for specific Access Area subdomains (pyramidal TIFF for IIIF, search indexes for HTML/Web Discovery, denormalised RDF for SPARQL, etc.). They are **regenerable from Preservation Files + derivation rule**. They do not carry their own versioning identity in DAO.
- **Presentation views** produced by HTML/Web Discovery (DPE), Custom Presentation (CPE), and other Access Area subdomains are presentation, not preservation. Subdomain-specific customisations live outside DAO.

---

## 1b. Commands vs. Events (CQRS/ES) — and the write/read separation

The Repository follows a **Command/Event (CQRS-ES)** pattern. The distinction is foundational and affects every other decision in this document.

- **Command** = an *intent* to change state. Issued by a producer (RDU-Tooling, ARK Resolver client, an internal preservation-action runner, etc.). Carries everything needed to attempt the change. Can be **rejected**. Examples: `SubmitDeposition`, `MintArk`, `ReserveArk`, `BindArk`, `MigrateRepresentationFormat`.
- **Event** = a *fact* that something has happened. Emitted by the Archive (or by other event-sourced services like the ARK Resolver) **only after** a Command has been validated and accepted. Immutable. Examples: `IntellectualEntityVersionPublished`, `RepresentationVersionCreated`, `ArkMinted`, `ArkBound`, `ArkReserved`, `FormatMigrated`.
- **Validation happens at command time.** Collision detection, format checks, schema validation, authorization checks, and any other "this is allowed" logic runs against the Command. The event log thus contains only valid history; Events are never rejected.

### Write side vs. read side

CQRS separates the canonical record (write side) from materialized views for querying (read side):

- **Write side (the Archive's source of truth):** the **event log**, plus the persistent-identity nodes that events refer to (`dao:IntellectualEntity`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Deposition`, `dao:DepositAgreement`). The write side does **not** materialize Versions as queryable entities. Versions exist as facts encoded in events.
- **Read side (one or more projections / read stores):** materializes whatever views are useful for queries and presentation. One key projection materializes **Version nodes** for each IE and Representation by folding their publish events. The read store carries derived properties such as `versionNumber`, `publishedAt`, snapshot of metadata, list of pinned Representation Versions, `isCurrentVersion`, etc.

**DAO models the write side.** The read-side projection schema is a separate concern — it can evolve, be rebuilt, or be replaced by replaying events.

**What this means for DAO and other contexts:**

- DAO models **Events and persistent-identity entities**. Commands are transient and not preserved. Version nodes are read-side projections and are not DAO classes.
- The Producer/Ingest bounded context (RDU-Tooling) speaks in **Commands** issued to the Archive. It may also have its own internal Events (e.g., `DepositionDrafted`, `DepositionValidated`) before a Submission is sent to the Archive.
- The ARK Resolver is its own bounded context. It accepts ARK Commands, emits ARK Events, and maintains its own event store.
- "Collisions are detected at Command validation time" is a general principle: any uniqueness or referential constraint is a Command-time check, not an Event-time correction.

---

## 2. First-class entities and identity

### 2.1 Entities

There are **three first-class versioned entities**:

- **IntellectualEntity** (IE) — described content with descriptive identity. RDF data conforming to DAO. Has a stable internal IRI. Versions over time. PREMIS-aligned term: "a coherent set of content reasonably described as a unit." Replaces what we initially called "Resource" (which conflicted with `knora-base:Resource` and other senses).
- **Representation** (Rep) — the preservation-grade bundle: a set of one or more **Preservation Files** (PREMIS allows multi-file Representations) plus Representation-level metadata (license, authorship, copyright, technical metadata such as filename, MIME type, originalFilename, originalMimeType). Has a stable internal IRI. Versions over time. Many-to-many with IntellectualEntities across Versions. **Service Files** (Access Area mezzanines) are derived from Preservation Files and are not themselves Representations in DAO terms. Replaces what we initially called "Asset" (which had a different meaning in the VRE); the informal label "Archival Master" is retired in favour of the three-tier role vocabulary (decision 30).
- **Ontology** — schema. Single archival ontology only (see §4).

### 2.2 Internal identifiers

Two layers of internal identifier:

**Write-side (DAO graph IRIs):**

- `dao:IntellectualEntity` and `dao:Representation` instances have **HTTPS URIs** in a DaSCH-controlled Archive namespace, e.g., `https://archive.dasch.swiss/ie/{uuid}` and `https://archive.dasch.swiss/rep/{uuid}`.
- Identifiers are **UUIDv7** — distributed-allocation safe, time-ordered (sortable, indexable), 128 bits, opaque to humans.
- Events also use UUIDv7: `https://archive.dasch.swiss/event/{uuid}`.
- Persistent-identity URIs (without Version suffix) live in DAO. They are the URIs events refer to.

**Read-side (projection URLs):**

- Versions are addressed as **read-side URLs** with a Version suffix: `https://archive.dasch.swiss/ie/{uuid}/v{n}`. These URLs are **part of the read store's contract**, not DAO IRIs.
- The read store guarantees these URLs resolve as long as the event log is replayable. If the read store is rebuilt after a system migration, the URL pattern is honored.
- DPE serves Version-suffixed URLs by querying the read store's materialized Version nodes.

Internal IRIs and read-side URLs are **dereferenceable HTTP URIs** so Linked Data conventions work; what they serve is decided separately.

**Long-term stability commitment.** Internal IRIs and read-side URLs are not promised to outlive a system migration. Only **ARKs** are. See §2.3.

### 2.3 ARKs (separate identifier system, separate bounded context)

**Only ARKs are promised to be stable long-term.** Internal Archive IRIs, read-side URLs, and DPE URLs are operational identifiers; they may change across system migrations. ARKs are the long-term commitment.

**Per-entity ARK minting.** An ARK is minted **once per archival entity that needs public citation** — IE, Representation, Project, and any future class that warrants its own citable identity. **Versions are not separately ARK'd.** A specific Version is denoted by appending `/v{n}` (or, for VRE-era ARKs, the timestamp suffix) to the entity's ARK string. The ARK string for the entity remains stable across all its Versions; the suffix selects the Version.

**ARKs vs. internal identifiers.** ARK target bindings are mutable: when DaSCH migrates infrastructure, the Resolver re-points ARKs to new internal targets and citations keep working. Archive internal IRIs, by contrast, are stable within a system but not across system migrations.

**ARK Resolver as a separate bounded context:**

- The ARK Resolver is its own service with its own ubiquitous language (ARK, NAAN, suffix, target binding, …) and its own event store.
- The Archive emits domain Events (`IntellectualEntityVersionPublished` etc.) that the ARK Resolver consumes (via a subscription / projection) to mint ARKs for newly-introduced persistent identities and to maintain bindings.
- The ARK Resolver answers `GET https://ark.dasch.swiss/ark:/.../resource-id[.version-suffix]` by looking up the ARK's binding to the target service (DPE or other), passing through the Version suffix.
- DPE receives a request like "show IE {uuid} at v5" (or "at timestamp T" for VRE-era ARKs) and serves it from the read store, which materializes the appropriate Version node from the event log.

**ARK structure (preserved from VRE era for compatibility):**

```
https://ark.dasch.swiss/ark:/72163/1/082B/SQkTPdHdTzq_gqbwj6QR=AR.20210712T074941501291Z
                              |    | |    |                    |  |
                            NAAN   | |    Resource ID          |  Optional Timestamp
                                   | |                         |  (= Version pointer)
                          DSP-internal Project Shortcode    Check digit
                          ARK Version
```

- **NAAN `72163`** — DaSCH's institution identifier. Permanent.
- **DSP-internal ARK Version** — versioning of the *ARK scheme itself*. **`1` = VRE era**; **`2` = Repository / new ARK Resolver era**. Different DSP-versions cannot collide.
- **Project Shortcode** — semantic content identifying the project.
- **Resource ID** — opaque identifier within the project namespace.
- **Check digit** — error detection.
- **Timestamp suffix** — optional; identifies the Version of the Resource that was current at that timestamp.

### 2.4 ARK use cases

ARKs may be **supplied by RDU-Tooling at submission time** (optional) or **auto-minted by the ARK Resolver** for new content. Three use cases:

| Case | When | DSP-version | Source |
|---|---|---|---|
| **(a) Migration** | A VRE-era ARK already exists; ingest preserves it | `1` | RDU-Tooling supplies it in the Submission Command |
| **(b) Reservation** | A researcher needs to cite an ARK in a paper *before* publication | `2` | RDU-Tooling issues a `ReserveArk` Command before any Deposition exists; later, a `BindArk` Command claims the reservation when the Deposition publishes |
| **(c) Auto-mint** | A new IE/Rep is published without a preferred ARK | `2` | ARK Resolver mints fresh on consuming the publish event |

### 2.5 ARK ↔ Archive binding model

- The ARK Resolver's event vocabulary includes (at minimum): `ArkReserved`, `ArkMinted`, `ArkBound`, `ArkRebound`, `ArkRevoked`.
- ARK targets point to **persistent-identity** internal Archive IRIs (e.g., the IE's URI without Version suffix). The Version suffix from the ARK string is passed through to the target service as a request parameter.
- An ARK can be rebound (e.g., when migrating a VRE-era ARK from pointing-at-VRE to pointing-at-DPE). Rebinding emits an `ArkRebound` event; old citations continue to work because the ARK string is unchanged.

### 2.6 VRE-era ARK migration

- All VRE-era ARKs and their timestamps are knowable up-front by dumping VRE data. Migration is a **one-time bulk registration** into the ARK Resolver, not on-the-fly translation. The mapping is precomputed before any binding events are emitted.
- VRE Resource ARKs map to DAO IE persistent-identity IRIs. The Version suffix (`/v{n}` for new ARKs; timestamp suffix for VRE-era) selects the Version on the read side.
- VRE Value ARKs (which point to specific values inside a Resource) map to DPE deep links of the form `https://dpe.dasch.swiss/ie/{uuid}/v{n}#value-X`. The ARK Resolver redirects to the deep link; DPE renders the IE Version with the relevant value highlighted. **No new DAO concept is needed for Value ARKs** — they are a DPE/Resolver concern. **Going forward, no new Value-level ARKs are minted by the Repository.**

### 2.7 Persistence and versioning

**Identity persists across versions.** An IE (and a Representation) has a stable persistent-identity URI on the write side and a single ARK on the long-term-citation side. Versions are denoted by suffix; they are not separately ARK'd.

**Stability layers (strongest first):**

1. **ARK** — promised to remain resolvable across all future system migrations. The single long-term commitment.
2. **Persistent-identity internal IRI** (`https://archive.dasch.swiss/ie/{uuid}`) — stable within a system; may change across system migrations.
3. **Read-side Version URL** (`https://archive.dasch.swiss/ie/{uuid}/v{n}`) — operational read-store URL pattern; honored by the read store as long as the event log is replayable.

**ARK resolution behavior:**
- ARK with no Version/timestamp suffix → Resolver forwards to DPE for "current Version of IE {uuid}". DPE consults the read store to determine current Version.
- ARK with Version/timestamp suffix → Resolver forwards to DPE with the suffix; DPE serves the matching Version from the read store.

**IE ↔ Representation linkage:** many-to-many is the desired archival reality, even though the current VRE forces 1:1 by re-uploading.

---

## 3. Versioning rules

### 3.1 What triggers a new Version

| Entity | New Version when... |
|---|---|
| **Representation** | Bitstream bytes change, **OR** Representation metadata changes (license, authorship, technical metadata, etc.) |
| **IntellectualEntity** | Researcher publishes/archives — at deliberate publication events, **not** on every VRE edit |

**Important consequence:** the Repository's notion of "version" is coarser than the VRE's edit history. A Resource Version is a deliberate, named, citable snapshot.

### 3.2 IE ↔ Representation version pinning

An IE Version pins **specific Representation Versions** (model (a) from the interview). IE v3 forever references the exact Representation Versions it was published with. Citations to a specific IE Version are stable across time.

**Preservation commitment:** any Representation Version once referenced by a published IE Version can never be deleted. This is a load-bearing archival commitment.

The pinning is expressed in the **publish event payload**: an `IntellectualEntityVersionPublished` event carries references to specific predecessor `RepresentationVersionCreated` events (or to "Representation X as of event E"). The pin is a fact in the event log. The read side materializes this as queryable `:references` triples on Version nodes for SPARQL convenience.

### 3.3 Service Files and Service Projections are not versioned

Service Files (and other Service-tier projections, e.g. search indexes, SPARQL graph denormalisations) in the Access Area are **derived projections of Representation Versions**, not first-class versioned entities. When a Representation Version is created (i.e., on `RepresentationVersionCreated` event consumption), its Service-tier projections are (re-)derived. When derivation rules change (e.g., DaSCH adopts a new IIIF profile), they are re-derived from the unchanged events. Service Files carry no ARK and no version number of their own — their identity is "the current derivation of Representation X v_n under derivation rule Y."

This aligns with Event Sourcing: Service-tier projections are derived, regenerable by replay. (Previous wording used "Service Master" — retired in favour of the three-tier role vocabulary; see `CONTEXT.md` → Preservation chain roles.)

### 3.4 Versioning lives on the read side, not in DAO

In CQRS-ES, "Version 5 of IE X" is **a fact derived from the event log**, not a write-side entity. DAO models the write side: IEs and Representations have persistent identity; their Versions are encoded as the sequence of `IntellectualEntityVersionPublished` and `RepresentationVersionCreated` events that refer to them.

The **read side** materializes Version nodes from the event log. Each Version node carries derived properties:

- `versionNumber` (e.g., 3) — derived from "this is the 3rd publish event for this IE."
- `publishedAt` — copied from the event's timestamp.
- `isCurrentVersion` (boolean) — derived from "no later non-Tombstoned publish event exists for this IE."
- A snapshot of descriptive metadata — copied from the event's payload.
- `references` to specific Representation Version nodes — derived from the pinning recorded in the event payload.

**The read store guarantees citable URLs.** The URL pattern `https://archive.dasch.swiss/ie/{uuid}/v{n}` is part of the read store's contract, honored as long as the event log is replayable. If the read store is lost or rebuilt, replay regenerates the same URLs and the same Version content.

Concrete shape on the **write side** (DAO) for an IE that has been published 3 times:

```
dao:IntellectualEntity (https://archive.dasch.swiss/ie/{uuid})
  rdf:type → dao:IntellectualEntity
  [no Version pointers; Versions are not write-side entities]

Event log contains (referring to this IE):
  Event #1  IntellectualEntityVersionPublished  ie={uuid}  payload={...}  publishedAt=2024-01-...
  Event #2  IntellectualEntityVersionPublished  ie={uuid}  payload={...}  publishedAt=2024-08-...
  Event #3  IntellectualEntityVersionPublished  ie={uuid}  payload={...}  publishedAt=2025-03-...
```

Concrete shape on the **read side** (one possible projection schema; not part of DAO):

```
ie:IntellectualEntity (https://archive.dasch.swiss/ie/{uuid})
  ie:hasVersion → IEVersion node (.../v1)
  ie:hasVersion → IEVersion node (.../v2)
  ie:hasVersion → IEVersion node (.../v3)
  ie:hasCurrentVersion → IEVersion node (.../v3)

IEVersion node (https://archive.dasch.swiss/ie/{uuid}/v3)
  ie:versionNumber → 3
  ie:publishedAt → 2025-03-...
  ie:resultsFrom → Event #3
  ie:references → RepresentationVersion node (.../rep/{uuid}/v2)
  [...all the descriptive properties as of v3...]
```

The read-side schema uses its own namespace (illustrated as `ie:` above) so it is not confused with DAO. Different read stores may use different schemas; DAO is unaffected.

**Sub-decisions:**

- **Version numbers are monotonically increasing integers** (1, 2, 3, …). Not semver. One number per publish event. Citation needs to be unambiguous; semver implies authorial judgement that doesn't reliably exist for archival data.
- **`isCurrentVersion` is materialized in the read store.** Cheap, one-hop for queries; handles Tombstoning correctly (the current Version may not be the max version number).
- **Tombstoning behavior** is parked for later (open Q9). Likely a Tombstoned event affects how the read side computes `isCurrentVersion`.

### 3.5 Why no Version classes in DAO

This is a deliberate CQRS-ES choice. Three reasons:

1. **Source-of-truth purity.** DAO models what is canonically preserved. The events are canonical; Versions are derived facts. Putting derived facts in the canonical model invites them to drift from the events.
2. **Read-side flexibility.** Different consumers may want different Version-shaped projections (e.g., a search index that flattens Versions into one row per IE; a graph view that materializes Version nodes; a citation service that resolves Version URLs). DAO not committing to one Version schema lets each reader project as needed.
3. **Replay correctness.** If the read store is rebuilt from the event log, the Version graph is regenerated. Nothing about the canonical archive depends on the read-side Version representation surviving.

The events are the source of truth. The read store materializes whatever is useful. Citation URLs are a contract of the read store, backed by the durability of the event log.

### 3.6 Bitstream storage

Bitstreams (the actual file bytes of a Representation) are stored **outside the event log**, in a content-addressable store. Events carry **references** to bitstreams: a content hash plus optional file-level technical metadata.

**Deviation from PREMIS, documented:** PREMIS 3 models `Representation`, `File`, and `Bitstream` as three distinct `Object` subclasses (`PREMIS DD §1.2.2`, `§1.2.3`, `§1.2.4`), each with its own identifier and metadata. DAO **collapses `File` and `Bitstream` into facts inside event payloads plus content-addressed bytes**. Reasoning:
1. **Event sourcing makes `File`-as-class redundant.** A file is what a `RepresentationVersionCreated` event creates; its existence and metadata are recorded in the event payload. A separate `dao:File` class would duplicate event-payload data on the write side.
2. **Content addressing makes `Bitstream` identity = hash.** PREMIS's `Bitstream` exists partly to identify byte-level units below the file level (e.g. one TIFF strip). DAO does not currently need sub-file granularity; if it ever does, hash-addressed bitstreams can be added without changing the `Representation` class.
3. **Reversibility.** If a future need emerges, the deviation can be reversed: each file-fact in an event payload could be projected into a read-side `File` node with `objectIdentifier = sha256:<hash>`, mechanically.

Per decision 27, this deviation is acceptable because it has documented reasoning and is reversible.

**Hash format and algorithm:** Multihash format (algorithm-flexible self-describing hash). **SHA-256 is the default today.** Multihash allows future migration to stronger algorithms — re-hashing existing bitstreams under a new algorithm produces additional `FixityChecked` events that record the new hash alongside the old.

**Event-payload reference shape:** events carry hash only (with technical metadata). The content store is responsible for "given a hash, return the bytes." The event payload does not carry storage URIs or paths — those are an internal concern of the content store, free to evolve without affecting events.

**File-level technical metadata in event payloads:** size in bytes, MIME type, PRONOM PUID (format identification), encoding, dimensions for images, duration for audio/video, etc. PREMIS-style technical metadata is **small**, carried in the event payload alongside the file's hash.

Example shape of an `RepresentationVersionCreated` event payload:

```
RepresentationVersionCreated:
  rep: https://archive.dasch.swiss/rep/{uuid}     # persistent identity, in DAO
  versionNumber: 2                                # for the read side
  publishedAt: 2026-04-15T...
  files:
    - filename: "scan-001.tif"
      hash: "sha256:abc123..."                    # multihash; identifies the bitstream
      sizeBytes: 145000000
      mimeType: "image/tiff"
      pronomId: "fmt/353"
    - filename: "scan-001.xmp"
      hash: "sha256:def456..."
      sizeBytes: 8192
      mimeType: "application/rdf+xml"
  license: spdx:CC-BY-4.0                         # SPDX URI per §4.4
  copyrightHolder: ...
  authorship: ...
```

The bytes for the listed files live in the content store, addressed by their hashes.

### 3.7 Storage architecture (right-sized for DaSCH)

**Working assumption:** self-built Archive Component (per decision 17, revised). DaSCH owns the storage implementation directly; no anti-corruption layer needed; DAO is the storage model.

**Scale check (re-baselined 2026-05-13/15).** DaSCH targets 50-100 projects/year, ~2 Depositions/week, **~200K events/week** steady-state. Deposit-burst-driven: a single Deposition may carry 10K to ~500K events (per decision 31). Approximately ~10M events/year, TB-scale content storage. Still small enough for a single-machine architecture, provided OCFL Object granularity is chosen carefully (per-aggregate rather than time-bucketed). Heavy event-store / message-broker technology (Kafka, EventStoreDB) remains operational overkill at this volume.

**Architecture: OCFL on filesystem as source of truth, SQLite as rebuildable read-side cache.**

```
/archive-root/
├── ocfl-storage-root/                         # SOURCE OF TRUTH
│   ├── ie/{uuid}/...                          # OCFL Object per IE (RDF metadata, versioned)
│   ├── rep/{uuid}/...                         # OCFL Object per Representation (Preservation Files + RDF, versioned)
│   ├── projects/{uuid}/...                    # OCFL Object per Project
│   ├── depositions/{uuid}/...                 # OCFL Object per Deposition  (producer-induced; bundles the events emitted during ingest)
│   └── preservation-actions/{uuid}/...        # OCFL Object per PreservationAction (archive-induced; bundles the events emitted during the action)
└── cache/
    ├── archive.sqlite                         # rebuildable read-side cache
    └── event-index/                           # append-only chronological event index (rebuildable from OCFL)
        └── {yyyy-mm}.ndjson                   # one line per event: {event_uuid, timestamp, aggregate_kind, aggregate_uuid}
```

**OCFL on filesystem is the durable canonical state for everything**: bitstreams (Preservation Files), RDF metadata, and the event log itself. Self-describing, hash-verified, navigable without DaSCH software (a future archivist with only the OCFL spec can recover everything). Inspired by Fedora 6's principle that the on-disk layout is the source of truth and all databases are derived caches.

**Event-log artifacts are bundled with their causally-meaningful aggregate.** Every event belongs to either a `Deposition` (producer-induced) or a `PreservationAction` (archive-induced); the corresponding OCFL Object holds both the aggregate's descriptive metadata and the events it emitted, as a single coherent unit. **Granularity changed from weekly time-buckets to per-aggregate Objects** in 2026-05-15 (revises decision 23): at 200K events/week with deposit bursts up to 500K, weekly time-bucket Objects became unbounded and filesystem-painful, while per-aggregate Objects are causally meaningful, naturally bounded, and align with PreservationAction-as-aggregate (decision 26).

**Event-index is the chronological reverse-lookup.** A subscriber tailing the Events SSE feed needs chronological access; events themselves live inside per-aggregate Objects. The `event-index/` directory holds an append-only NDJSON per month: one line per event with `{event_uuid, timestamp, aggregate_kind, aggregate_uuid}`. The SSE handler reads the index in order, dereferences each event from its containing OCFL Object, and emits. The index is **rebuildable** by scanning all OCFL Objects (cost: O(events); reasonable as a one-time recovery step).

**SQLite cache holds the read-side projection.** Materialized Version nodes, indexed views ("all IEs in Project X", "current Version of IE Y", "all events for Representation Z in chronological order"). One SQLite file per archive instance. Embedded, transactional, no server. Rebuildable from OCFL by replaying events.

**No separate event-store technology.** Events are files inside per-aggregate OCFL Objects. Read-side projector consumes the event-index and updates SQLite. ARK Resolver (separate bounded context) consumes its own subset of events via the public SSE feed (see §3.7.1).

**Write path (for a Deposition):**

1. Command (e.g. `SubmitDeposition`) arrives at the Archive (Ingest Area).
2. Validation runs (SHACL against DAO, `DepositAgreement` enforcement, fixity, authorisation).
3. On validation success: the Deposition aggregate is committed as a new OCFL Object under `depositions/{uuid}/`, containing the `DepositionAccepted` event and all per-entity events (`IntellectualEntityVersionPublished`, `RepresentationVersionCreated`, etc.) it emitted, plus the Deposition's descriptive metadata.
4. New entries appended to `event-index/{yyyy-mm}.ndjson` for each event in the commit.
5. fsync.
6. SQLite updated transactionally.
7. Acknowledge success to Command issuer.

**Write path for a PreservationAction** is analogous: aggregate Object committed under `preservation-actions/{uuid}/`, event-index extended, SQLite updated.

**Recovery:** if the process crashes between steps 5 and 6 (or SQLite is lost entirely), restart scans the event-index for entries after SQLite's last-applied event and replays them. If the event-index itself is lost, it is rebuilt by walking all OCFL Objects under `depositions/` and `preservation-actions/`. OCFL is the source of truth; SQLite and the event-index are caches.

**Sanctioned exception to OCFL immutability — Redaction.** A `Redacted` event (§5.1) is the **only** operation that may mutate or zero existing bytes in OCFL. The mutation is recorded as a new OCFL Object version with the offending bytes removed, never as in-place overwrite of an earlier OCFL version-directory contents. The `Redacted` event in the event log carries the legal basis and authorizing Agent; the OCFL Object's version-log carries the corresponding pointer. Every other operation in §5.1 is additive only. This exception exists because GDPR Art. 17 and equivalent legal obligations can compel erasure that immutable preservation cannot otherwise satisfy; it is bounded narrowly to redaction events and never extended to routine operation.

**What this gets DaSCH:**

- **Operational simplicity.** Two storage things: a filesystem and a SQLite file. Backups are filesystem snapshots.
- **Preservation purity.** OCFL is fully self-describing and self-verifying. No vendor or infrastructure dependency for long-term reading of the archive.
- **No infrastructure to scale or operate.** No Postgres, no Elasticsearch, no Kafka, no S3, no message broker.
- **Aligned with DaSCH's stated values:** architectural simplicity, long-term maintainability, escape from infrastructure-heavy designs.
- **Deferred decisions remain open.** If scale demands change, SQLite can be replaced by Postgres without changing OCFL or events. If geographic replication is required, OCFL replicates as filesystem-level mirroring. The architecture starts small and stays simple.

### 3.7.1 Public APIs of the Archive

The Archive exposes **three public APIs** at its bounded-context boundary (decision 31). Subscribers and Producers reach the Archive only through these; OCFL is internal and never directly exposed.

#### Commands API

- **Shape:** HTTP. RPC-style or per-command endpoints; exact URL design deferred to implementation. Commands are intents to change state (`SubmitDeposition`, `MintArk` if intra-Archive, `RunFixityCheck`, `MigrateRepresentationFormat`, `Tombstone`, `Redact`, etc.).
- **Synchronicity:** Validation is synchronous; the API returns 4xx immediately on failure. Commit may be asynchronous for large Depositions: the API returns 202 Accepted with an opaque `ticket-id` and a status URL the Producer can poll. Small commands (e.g. `MintArk`) may return 200/201 synchronously when commit is fast.
- **Idempotency:** Commands carry a client-supplied `Idempotency-Key` header; the Archive deduplicates retries within a configurable retention window.
- **Auth:** mTLS or bearer-token; per-Producer credentials tied to a `DepositAgreement`. Anonymous Commands are not accepted.
- **Errors:** structured JSON error responses with stable `error_code` strings (`VALIDATION_FAILED`, `AGREEMENT_VIOLATION`, `IDEMPOTENCY_CONFLICT`, etc.).
- **Versioning:** URL path or `Accept` header carries an API version (`v1`). Backward-incompatible changes bump the version; the Archive may run multiple versions concurrently during transitions.

#### Events SSE feed

- **Shape:** `GET /api/v1/events` with `Accept: text/event-stream`. Standard W3C Server-Sent Events.
- **Resumability:** `Last-Event-ID` request header. If absent, the subscriber receives the live tail starting from "now". If present and well-formed, the subscriber receives all events with `event_id > Last-Event-ID`, in chronological order, then transitions seamlessly to the live tail.
- **From genesis:** `Last-Event-ID: 0` (or the equivalent sentinel) is a valid resume point. The Archive serves the entire historical event stream from the event-index, then catches up to live. No separate "bulk" endpoint in the initial implementation (per the ship-and-measure principle, §9.5).
- **Delivery semantics:** at-least-once; subscribers must be idempotent on event consumption. Per-entity ordering is guaranteed (events for the same aggregate are emitted in commit order); cross-entity total order is not guaranteed.
- **Heartbeats:** SSE comment lines (`: heartbeat`) every ~30s to keep HTTP/proxy connections alive.
- **Filtering:** **none server-side.** Subscribers receive the full firehose and filter client-side (decision 31).
- **Auth:** mTLS or bearer-token. Subscribers are registered (the Archive knows which subscribers exist for observability and rate-limit purposes); subscription itself is open to any registered subscriber.
- **Backpressure:** slow subscribers fall behind on `Last-Event-ID`; on resume, they receive historical events from the index. No backpressure signal from Archive to subscriber. If a subscriber falls catastrophically behind, the operator's recourse is to switch to the (γ) or (δ) bootstrap strategy (§3.7.2).

#### Binary retrieval API

- **Shape:** `GET /api/v1/bitstreams/{multihash}`. Content-addressed; the multihash uniquely identifies the bytes.
- **Range requests:** HTTP 206 Partial Content supported, for clients that want to stream large files.
- **Caching:** content is immutable (the multihash binds bytes to identity), so the response carries long `Cache-Control` lifetimes and an `ETag` of the multihash itself. Access Area subscribers cache aggressively at derivation time.
- **Auth:** mTLS or bearer-token; same auth as the SSE feed. (Access decisions are made by the Access Area subdomain at user-request time, not at this layer — the Archive only authenticates that the caller is an authorised subscriber.)
- **Errors:** 404 if the multihash is unknown; 410 Gone if the bytes were intentionally redacted (the `Redacted` event in the relevant Representation history explains why).

### 3.7.2 Subscriber bootstrap (cold replay)

Three bootstrap strategies are supported, by use case (decision 32):

**(α) Subscriber-side snapshots — for routine restarts.** Each subscriber periodically writes its derived state to a snapshot, with the `event_id` of the last applied event embedded. Snapshot cadence is the subscriber's choice (every N events, every M minutes, or after every Deposition commit). On restart, the subscriber:

1. Restores its state from the latest snapshot.
2. Opens the Archive SSE with `Last-Event-ID = snapshot.last_event_id`.
3. Receives all events since the snapshot, then transitions to live tail.

**(γ) Subscriber-to-subscriber replication — for spinning up duplicates.** When a second instance of an existing subscriber kind is brought up (HA, scaling, geographical replication), it copies state from an existing peer rather than replaying from Archive. The existing peer exposes an internal "snapshot export" endpoint. After the copy, the new instance tails Archive's SSE from `Last-Event-ID = peer.last_event_id`. Only works between subscribers running the *same* derivation logic version.

**(δ) Full SSE replay from genesis — for brand-new subscriber kinds.** When deploying a subscriber kind that has never run before (e.g. a new SPARQL projector with new derivation logic), the subscriber opens the SSE with `Last-Event-ID = 0` and consumes the entire historical event stream. Slow but bounded; expected to be a rare one-time event. A bulk-replay mode (returning events in batches rather than as a paced SSE stream) is a deferred optimisation per §9.5 — added only when measurement under real load shows that paced SSE replay is intolerable.

**Snapshot durability and recovery for (α).** Snapshots are the subscriber's own responsibility, not Archive's. Lost snapshots fall back to (δ). Subscribers should keep at least two recent snapshots to survive corruption of the most recent one.

**No Archive-side snapshot mechanism.** The Archive does not maintain or serve subscriber-shaped snapshots — that would require the Archive to know each subscriber's derivation logic, which it must not.

### 3.8 Fixity checks

A `FixityChecked` event records the result of verifying a bitstream against its hash.

**Outcomes** ∈ {`pass`, `fail`, `missing`}:

- **`pass`** — bytes hash to the expected value. Event records the check happened.
- **`fail`** — bytes hash to a different value. **Preservation incident.** Triggers alert and preservation-action workflow (workflow is policy, parked separately).
- **`missing`** — bytes are not retrievable. Same incident response as `fail`.

The event itself is the durable record of the check; the workflow that responds to `fail` and `missing` outcomes is a separate concern (parked).

---

## 4. Ontology model

### 4.1 The DaSCH Archival Ontology (DAO)

**DAO is the only ontology archived content directly conforms to.** Built fresh as **OWL + SHACL**, designed cleanly from the start (no pre-SHACL OWL misuse).

**DAO is conceptually built on PREMIS 3.** Its core write-side classes are aligned with PREMIS entities — `dao:IntellectualEntity` ↔ `premis:IntellectualEntity` (`PREMIS DD §1.1`); `dao:Representation` ↔ `premis:Representation` (`PREMIS DD §1.2.2`); `dao:Agent` ↔ `premis:Agent` (`PREMIS DD §1.3`); `dao:Event` and its subclasses ↔ `premis:Event` (`PREMIS DD §1.4`); `dao:DepositAgreement` corresponds loosely to OAIS *Submission Agreement* (`OAIS §6.1`). DAO uses the `dao:` namespace rather than `premis:` IRIs because it adds DaSCH-specific structure (identifier model, ARK linkage, CQRS-aligned event subclasses, projection-vs-write-side split) — but every DAO class should be reducible to a PREMIS/OAIS concept, and any new class that cannot must be flagged as a deviation per decision 27.

There is no "preserve the project ontology as a separate first-class artifact in the archive." Project subclasses (`project-xyz:Book` subclassing `knora-base:Resource`) serve VRE-only purposes (namespacing, display labels) and are normalized away during ingest. The VRE's class hierarchy is an implementation detail; it carries no archival information. **What is preserved is the property/value assertions the project made on the IE — those are the archival content.** The originating project class IRI is not stored; the assertions made on instances of that class are stored in full.

### 4.2 External ontology references survive

An IE in DAO may carry assertions linking it to external ontologies (CIDOC CRM, FRBRoo, FOAF, Bibframe, etc.). RDF multi-typing handles this natively: an IE can be `dao:IntellectualEntity` *and* `crm:E22_Human-Made_Object` simultaneously.

The archive does **not** validate or reason over external types. It preserves them as historical assertions made by the project. External ontology maintenance is the external community's responsibility, not DaSCH's.

### 4.3 What gets dropped or reshaped during ingest

Ingest performs three jobs simultaneously, all lossless in archival terms (runtime operational state is shed; historical/interpretive content is preserved):

1. **Structural normalization** — flatten project-specific subclass hierarchies; preserve only the property/value assertions made on each instance (the originating class IRI is not preserved per §4.1).
2. **Administrative pruning** — drop runtime VRE concerns (Knora permissions bookkeeping, internal state).
3. **Vocabulary substitution** — replace internal Knora vocabularies with standards-based ones.

### 4.4 Vocabulary substitution policy

**DAO defers to external standards wherever they exist.** DaSCH-specific vocabulary is the exception, requiring justification.

Confirmed substitutions so far:

| Concern | DAO uses |
|---|---|
| Access rights | **COAR Access Rights** (`open access`, `embargoed access`, `restricted access`, `metadata only access`) |
| (others to be decided as we encounter them) | |

Candidates likely to come up: SPDX (licenses), IANA media types + PRONOM (formats), BCP-47 (language).

### 4.5 Cardinality is not archived

Project ontologies' cardinality constraints (mandatory/optional properties, min/max counts) are **interpretation rules the project applied during data collection**, not properties of the data itself. The archive preserves what is, not what should have been.

**SHACL profile consequence:** DAO's SHACL shapes describe DAO's own structural invariants (an IE has an internal IRI, a Representation has at least one file, a Version has a publish event). They do **not** carry project-specific cardinality. The archive is structurally validated; it is not semantically validated against project intent.

**DPE consequence:** the presentation layer is permissive about what's present. It renders whatever properties exist on an IE Version without complaining about missing ones. Partial information is the norm at scale and over time.

---

## 5. Event sourcing and storage

The archival format is **event-sourced** and **WORM-compatible**. The canonical truth is an append-only log of events. Current state is a fold over event history. Versions are derived projections, not mutable records.

### 5.1 Event vocabulary (working set)

Events represent meaningful archival actions. Tight, fixed vocabulary:

- **`IntellectualEntityVersionPublished`** — an IE Version is created and frozen. Carries full IE metadata snapshot and the list of pinned Representation Versions.
- **`RepresentationVersionCreated`** — a Representation Version is created (bytes or Representation metadata changed). Carries file hashes + Representation metadata snapshot.
- **`DepositionAccepted`** — a Deposition was validated and committed. Groups events from one ingest.
- **`FixityChecked`** — integrity verification of a bitstream against its hash. Outcome ∈ {`pass`, `fail`, `missing`}. Failures and missings trigger preservation-action workflows (parked). See §3.8.
- **`FormatMigrated`** — preservation action transforming a Representation's files (or, through DAO migration, an IE) to a new format/shape; produces a new Version with provenance link to predecessor. **Emitted from a `dao:PreservationAction` aggregate, not a `dao:Deposition`** (see §6.1 and decision 26). Aligned with `PREMIS DD §1.4` (Event entity) and `OAIS §5.1.1` (preservation strategies).
- **`AccessRuleChanged`** — embargo lifted, license updated, etc. Itself produces a new Version per §3.1.
- **`Tombstoned`** — **logical retraction.** A specific IE Version or Representation Version is marked as no longer disseminable. **Bytes and metadata remain in OCFL.** The read-side projection hides the Version from dissemination paths (DPE, IIIF, asset server, SPARQL) and ARK resolution returns a tombstone landing page that names the retraction and its provenance instead of the content. WORM is preserved. Curatorial/preservation-policy decision. Aligned with `PREMIS DD §1.4` (Event entity, e.g. types `deaccession` / `deactivation`) and `OAIS §3.3.5` (deactivation of AIPs is permitted; deletion is not in normal operation).
- **`Redacted`** — **surgical content-level erasure**, producing a new Version of the affected IE or Representation with the offending data removed. The pre-redaction Version is `Tombstoned` *and* its bytes/metadata in OCFL are over-written or zeroed — this is the **single sanctioned exception to OCFL immutability** (see §3.7). The `Redacted` event records *that* a redaction happened, *who* authorized it, and *under what legal basis* (e.g. GDPR Art. 17, court order); it does **not** record *what* was removed. ARK resolution continues to work; it resolves to the post-redaction current Version. Legal/compliance decision, requires named authorization. Aligned with `PREMIS DD §1.4` via a custom Event subtype.
- **`PreservationActionExecuted`** — wrapper event emitted when a DaSCH-internal preservation action commits. Groups the resulting per-entity events (`FormatMigrated`, `RepresentationVersionCreated`, `IntellectualEntityVersionPublished` from system-ontology migration, `Tombstoned`/`Redacted` if the action is GDPR-driven, etc.) for audit and provenance. The producer-side analogue is `DepositionAccepted`.

**Outright deletion** (true erasure of an entire Version with no redacted successor) is **not modeled as a DAO event**. It is a board-level exception recorded in a separate governance log outside the archive. Treating it as a normal event would normalize what must be exceptional.

### 5.2 Events carry snapshots, not deltas

`IntellectualEntityVersionPublished` event #5 contains the full IE metadata as of v5, not a diff. Readers reconstruct any Version by finding the latest publish event with version ≤ N. No log replay required. Critical for WORM.

### 5.3 No in-place migration

When a system ontology change requires updating archived content (e.g., a new mandatory property), the migration is performed by emitting **new Version events** that produce new Versions of affected Representations/IEs. Old Versions remain intact. Citations to old Versions resolve to pre-migration state.

---

## 6. DAO top-level classes

**Working assumption on Archive Component**: **self-built**. DaSCH owns the Archive implementation directly; DAO is the storage model. No anti-corruption layer required. Storage layer is OCFL on filesystem with SQLite read-side cache (per §3.7).

DAO models the **write side**: persistent identities and events. Version nodes belong to the read-side projection schema (see §3.4) and are not DAO classes.

### 6.1 Class list (write side only)

| Class | Purpose |
|---|---|
| `dao:IntellectualEntity` | Persistent identity of an IE. The URI events refer to. No Version-related properties on the write side. |
| `dao:Representation` | Persistent identity of a Representation (the preservation-grade bundle containing Preservation Files). The URI events refer to. |
| `dao:Project` | Producer/owner context. Identity persists; metadata changes are recorded as events. Not Version-modelled even on the read side (no use case for cited historical Project state). |
| `dao:Agent` | Person, organization, or software acting as creator/contributor/maintainer. May carry external identifiers (ORCID, ROR). |
| `dao:Event` | The event-sourcing primitive. Subclasses for the working vocabulary in §5.1. The event log is the source of truth. |
| `dao:Deposition` | **Producer-induced** unit of ingest, grouping events from one producer-side source at one time. Gated by `dao:DepositAgreement`. Detailed shape parked (open Q3). |
| `dao:DepositAgreement` | Producer/Archive contract. Carries producer identity, designated community, accepted formats, retention terms, embargo/access defaults, frequency of submission. May store a link to where the agreement document lives. |
| `dao:PreservationAction` | **Archive-induced** unit of change. Distinct from `dao:Deposition` (decision 26). Gated by internal preservation policy rather than a `DepositAgreement`. Groups events that result from archive-initiated activity: format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction. Aligned with `PREMIS DD §1.4` (Event entity) and `OAIS §4.1.3 / §5.1` (Preservation Planning). |

### 6.2 Not in DAO

- **`dao:IntellectualEntityVersion`, `dao:RepresentationVersion`** — read-side projection nodes, not DAO classes. The read store materializes these from events; their schema lives outside DAO and may evolve independently.
- **Relationships between IEs** (cites, isPartOf, derivedFrom): plain RDF properties between IEs. Not reified as a `dao:Relation` class.
- **Files within a Representation**: properties on the events that record the Representation's content. Per-file Bitstream-level addressing is parked.
- **`dao:AccessRights`**: expressed via the COAR Access Rights vocabulary in event payloads. The straightforward cases (open, embargoed, metadata-only) work as properties; the `restricted_access` case is non-trivial and is parked as open Q12. May or may not become a class depending on how `restricted_access` shapes up.
- **`dao:Place`, `dao:Concept`, and others**: may emerge when working through project-level metadata. Likely external IRIs (GeoNames for places, Getty AAT for concepts) rather than DAO classes. Parked.

---

## 7. Open questions parked for later

These came up but were deferred. New items added after folding in updated architectural context.

**From the original conversation:**

1. **Is `IntellectualEntityVersionPublished` the only path to a new IE Version?** Format migration of a Representation *referenced by* an IE does not create a new IE Version (the existing IE Version still pins the old Representation Version). But system-ontology-driven IE migration does. The full path inventory is not yet complete.
2. **Event ordering**: per-entity vs. global total order across the whole Repository. Per-entity is the event-sourcing convention and likely sufficient.
3. **Deposition boundary**: project-scoped incremental batch (recommended (b)) vs. cross-project (c). The separation question is **resolved** by decision 26 (`dao:PreservationAction` is distinct from `dao:Deposition`). What remains open is the granularity of a Deposition itself: one per project per submission event, vs. one spanning multiple projects from the same producer, vs. one open transactional boundary across an arbitrary session.

**From folding in updated architectural context:**

4. **Access Area manifestation: push vs. pull.** Push (Archive emits events, Access Area projects) fits Event Sourcing naturally and was the implicit model for our event vocabulary. Pull (Access Area queries Archive on demand) is simpler operationally but harder to reason about for projections.
5. **Access Area events in DAO scope?** If push: do Access Area events (`ServiceFileDerived`, `ServiceFileInvalidated`, `ProjectionRebuilt`) belong in DAO's event vocabulary, or in a separate Access-context vocabulary? **Resolved 2026-05-15**: separate, because Service Files / Service Projections are not in DAO's archival scope (decision 32). Each subscriber subdomain owns its own internal event vocabulary if it event-sources internally; these events live in an `access:` namespace and are not part of Archive's published SSE feed.
6. **DPE/CPE presentation hints in DAO?** Should DAO carry presentation hints (e.g., "render this IE as a recipe card", "highlight property X")? Lean: **no** — presentation is not preservation. CPE configuration lives outside the archive.
7. **IIIF Manifests in DAO?** IIIF has its own information model (Manifest, Canvas, Annotation). Lean: DAO stores enough technical metadata on Representations that the IIIF server can compute Manifests at request time. Manifests are not stored in the archive.
8. **CoreTrustSeal evidence linkage.** Several DAO choices (immutable event log, fixity events, format migration provenance, no in-place migration) double as CoreTrustSeal audit evidence. Should be cross-referenced explicitly per requirement.
9. ~~**Tombstoning detailed shape.**~~ **Resolved by decision 28.** Tombstoning is logical-retraction-only; bytes and metadata remain in OCFL; read-side hides the Version. A separate `Redacted` event handles GDPR-driven content erasure. Outright deletion is not a DAO event. See §5.1 and §3.7. *Remaining sub-detail:* exact shape of the tombstone landing page returned by DPE on ARK resolution — DPE concern, not DAO.
10. **Representation property list.** The full property list for `dao:Representation` and `RepresentationVersionCreated` event payloads (drawing on Knora `:FileValue` properties: `internalFilename`, `internalMimeType`, `originalFilename`, `originalMimeType`, `hasCopyrightHolder`, `hasAuthorship`, `hasLicense`, etc., reshaped to use external standards per §4.4) is TBD.
11. **ARK reservation and expiry policy.** Reservations made before publication can be reclaimed (bound to a real target). Whether reservations expire if never claimed, and after how long, is TBD.
12. **`dao:AccessRights` for restricted access.** The COAR Access Rights vocabulary covers open / embargoed / restricted / metadata-only. The straightforward cases (open, embargoed, metadata-only) work as properties in event payloads. The `restricted_access` case is non-trivial — what defines who can access, how is it enforced, who decides, what is the audit trail. May require its own class or sub-model. TBD.
13. **Project metadata as ontology.** Project-level metadata is itself a complete ontology (currently the basis for DPE's data model). Working through this may surface `dao:Place`, `dao:Concept`, and other secondary classes that haven't yet been needed. Not urgent for DAO core but a known workstream.
14. **Preservation-action workflows.** `FixityChecked` failures, format migrations, system-ontology-driven migrations — the workflows that respond to and produce these events are policy concerns, not DAO concerns, but they need to be designed. The events themselves now belong to `dao:PreservationAction` (decision 26); the **policy and orchestration** layer that triggers them remains parked.

---

## 8. Decision log

The numbering is the order decisions were made, not document order.

| # | Topic | Decision | Date |
|---|---|---|---|
| 1 | Source of discomfort with RiC-O | Conceptual mismatch: research data ≠ records-management; long-lived IEs/Representations need first-class versioning | — |
| 2 | Unit of identity across versions | Both IEs and Representations have independent identity and lifecycles (model (c)); each has its own internal IRI and zero-or-more ARKs; many-to-many between them | — |
| 3 | New-Version triggers | Representation: byte change OR metadata change. IE: at publication events, not on every VRE edit | — |
| 4 | IE → Representation linkage | IE Versions pin specific Representation Versions; preservation commitment that pinned Representation Versions are never deleted | — |
| 5 | Event sourcing | Yes; WORM-compatible; events carry snapshots not deltas; no in-place migration | — |
| 6 | Single archival ontology | DAO is the only ontology archived content directly conforms to; project ontologies normalized away; external ontologies (CRM etc.) survive via multi-typing; cardinality not archived. **Amended 2026-05-12 per decision 27:** DAO is conceptually built on PREMIS 3 — its core write-side classes (`IntellectualEntity`, `Representation`, `Agent`, `Event`) map directly to PREMIS entities; the `dao:` namespace exists to add DaSCH-specific structure, not to replace PREMIS. Every DAO class should be reducible to a PREMIS or OAIS concept, with documented deviations | — / amended 2026-05-12 |
| 7 | DAO top-level classes | Settled. List in §6. Build-vs-buy does not gate DAO class shape | — |
| 8 | Bounded contexts handling of DAO terms | DAO is the Archive context's ubiquitous language. Producer and Access contexts have their own internal vocabularies. DAO terms appear at boundaries (lingua franca of seams), not as universal language | — |
| 9 | Naming the two core entities | `dao:IntellectualEntity` (IE) and `dao:Representation` (Rep), adopting PREMIS terms verbatim | — |
| 10 | Originating class IRI not preserved | The VRE's class hierarchy is implementation detail. What is preserved is the property/value assertions made by the project on the IE | — |
| 11 | Version model | Versions are encoded in the event log on the write side (a Version exists as the n-th publish event for an IE). On the read side, materialized Version nodes are projected from events. Version numbers are monotonic integers. `isCurrentVersion` materialized in read store | — |
| 12 | Internal identifiers | UUIDv7-based HTTPS URIs in DaSCH-controlled Archive namespace. Write-side: persistent-identity URIs only (`https://archive.dasch.swiss/ie/{uuid}`). Read-side: Version-suffixed URLs (`.../v{n}`) as a contract of the read store. ARKs are NOT internal identifiers; ARKs are public lookup-and-forwarding handles managed by a separate ARK Resolver bounded context | — |
| 13 | ARK strategy | ARK Resolver is its own bounded context with its own event store. ARKs may be (a) supplied by RDU-Tooling for migration of VRE-era ARKs (DSP-version `1`); (b) supplied by RDU-Tooling for reservation before publication (DSP-version `2`); (c) auto-minted by Resolver on publish events (DSP-version `2`). VRE-era ARKs migrated as one-time bulk registration. Value-level ARKs map to DPE deep links; no new Value ARKs minted by Repository | — |
| 14 | Commands vs. Events (CQRS-ES) | Commands are intents that may be rejected; Events are facts emitted only after successful Command validation. Validation (incl. collision detection, uniqueness checks) happens at Command time. DAO models Events and persistent-identity entities; Commands are transient and not preserved | — |
| 15 | Top-level DAO class list (write side only) | `dao:IntellectualEntity`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event` (with subclasses), `dao:Deposition`, `dao:DepositAgreement`. (`dao:PreservationAction` added later as decision 26.) Version classes (`dao:IntellectualEntityVersion`, `dao:RepresentationVersion`) explicitly **NOT** in DAO — they are read-side projections. Relationships are properties, not reified. `dao:AccessRights`, `dao:Place`, `dao:Concept` parked pending project metadata work | — |
| 16 | Events vs. Version nodes (CQRS separation) | Events are the source of truth (write side, in DAO). Version nodes are materialized projections (read side, NOT in DAO). The read store guarantees Version-suffixed URL contracts as long as event log is replayable. Version node properties are derived from events; the projection is regenerable by replay | — |
| 17 | Archive Component build-vs-buy posture | **Working assumption: self-built.** DaSCH owns the Archive implementation; DAO is the storage model directly; no anti-corruption layer required. (Earlier working assumption was commercial/Docuteam; revised after right-sizing analysis showed DaSCH's scale and operational simplicity arguments favor self-built with OCFL-on-filesystem.) | — |
| 18 | DepositAgreement and AccessRights renaming | Submission Agreement → `dao:DepositAgreement` (DaSCH internal naming; may be a link to the document store). AccessRule → `dao:AccessRights`. The straightforward COAR cases (open, embargoed, metadata-only) are simple properties; `restricted_access` is non-trivial and parked as open Q12 | — |
| 19 | ARK is the single long-term stability commitment | ARKs are minted **per persistent-identity entity** (IE, Representation, Project, etc.), not per Version. A specific Version is denoted by suffix (`/v{n}` or VRE-era timestamp). ARK string remains stable; suffix selects the Version. DPE handles Version display by querying the read store. Internal IRIs and read-side URLs are not promised to outlive a system migration; only ARKs are | — |
| 20 | Bitstream storage | Bitstreams stored outside event log in a content-addressable store. Events carry hash references (multihash format, SHA-256 default) plus PREMIS-style file-level technical metadata in payload. Storage URIs are not in event payloads; the content store owns "given a hash, return bytes". **Deviation from PREMIS, documented (per decision 27):** PREMIS models `Representation → File → Bitstream` as three `Object` subclasses; DAO collapses `File` and `Bitstream` into facts in event payloads + content-addressed bytes. Reasoning: event sourcing makes `File`-as-class redundant; content addressing makes `Bitstream` identity = hash. Reversible — file-facts can be projected into read-side `File` nodes if needed. See §3.6 | — / deviation logged 2026-05-12 |
| 21 | Storage architecture | OCFL on filesystem as source of truth for everything: bitstreams, RDF metadata, event-log artifacts. SQLite as rebuildable read-side cache. No separate event-store technology. Write path: validate command → write event to OCFL → fsync → update SQLite. Recovery: scan OCFL on startup for events not yet in SQLite, replay | — |
| 22 | Right-sized for DaSCH scale | Architecture deliberately right-sized for 50-100 projects/year. Single-machine, two storage things (filesystem + SQLite file). No infrastructure to scale or operate. Replaceable with heavier infrastructure later if scale demands change, without changing OCFL or events. **Amended 2026-05-15:** baseline re-calibrated from "~tens of events/week" to **~200K events/week steady-state, deposit-burst-driven, with single Depositions reaching ~500K events at the extreme** (per decision 31). Still right-sized for single-machine, but the "trivial polling cadence" claim has been removed — push via SSE has replaced polling (decision 31). | — / amended 2026-05-15 |
| 23 | Event log granularity | **Amended 2026-05-15:** the original weekly time-bucket granularity (`events/{yyyy-Www}/`) has been replaced by **per-aggregate OCFL Objects** (`depositions/{uuid}/` and `preservation-actions/{uuid}/`), each bundling its aggregate's events together with the aggregate's descriptive metadata. A complementary append-only chronological event-index (`cache/event-index/{yyyy-mm}.ndjson`) provides reverse-lookup for SSE replay. Reason for the change: at re-baselined scale (~200K events/week with deposit bursts up to ~500K), weekly time-buckets became unbounded and filesystem-painful; per-aggregate Objects are causally meaningful (decision 26) and naturally bounded by the aggregate. See §3.7. | — / amended 2026-05-15 |
| 24 | Events carry full snapshots | Even though OCFL stores per-version IE/Representation state, events carry full metadata snapshots (not just references). Redundancy is intentional: preserves replay-from-events-alone capability for disaster recovery | — |
| 25 | Fixity event vocabulary | One `FixityChecked` event with `outcome` ∈ {`pass`, `fail`, `missing`}. Failures and missings trigger preservation-action workflows (workflows are policy, parked separately) | — |
| 26 | `dao:PreservationAction` as a first-class concept | DaSCH-internal preservation actions (format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction) are modeled as `dao:PreservationAction`, **separate from `dao:Deposition`**. Rationale: different actor (Archive-as-system-agent vs. Producer); different authorization regime (internal preservation policy vs. `DepositAgreement`); CoreTrustSeal audit-trail separation of producer-induced vs. archive-induced changes (CTS R09/R12); cleaner event vocabulary (`PreservationActionExecuted` parallels `DepositionAccepted`). Aligned with `PREMIS DD §1.4` (Event entity); `OAIS §4.1.3 / §5.1` (Preservation Planning) | 2026-05-11 |
| 27 | OAIS + PREMIS alignment as default | Every DAO design choice is aligned with OAIS (CCSDS 650.0-M-3, Dec 2024) and PREMIS 3.0, **or** carries a documented deviation with reason in the decision log or an ADR. Standards extracts checked in under [`standards/`](./standards/) with citation conventions (`OAIS §<n.n.n>`, `PREMIS DD §<n.n>`). The alignment is **conceptual**: DAO uses its own URIs in the `dao:` namespace and may diverge in property names and structure where DaSCH-specific needs justify it, but the underlying concepts must be traceable back to OAIS/PREMIS or explicitly justified | 2026-05-11 |
| 28 | Tombstoning vs. redaction | Two distinct events. `Tombstoned` = **logical retraction** (bytes/metadata preserved in OCFL; read-side hides the Version; ARK returns tombstone landing page). `Redacted` = **surgical content-level erasure** producing a new redacted Version of the IE/Representation; the pre-redaction Version is `Tombstoned` and its OCFL bytes are over-written/zeroed — the **single sanctioned exception to OCFL immutability**. `Redacted` records who authorized the redaction and under what legal basis (e.g. GDPR Art. 17), not what was removed. Outright deletion is **not** a DAO event — it is a board-level exception in a separate governance log. Aligned with `PREMIS DD §1.4` Event entity (custom `Redaction` subtype) and `OAIS §3.3.5` (deactivation permitted; deletion not in normal operation). See §5.1 and §3.7 | 2026-05-12 |
| 29 | Domain framing | The whole `dsp-repository` codebase implements **one domain**: Trusted Repository (OAIS-based). Decomposes into subdomains: Ingest, Archival Storage, Preservation Planning, Data Management, Administration, Access (all OAIS functional entities); Identification (DaSCH-specific, long-term citation via ARK); Producer-side preparation (DaSCH-specific). VRE is external to the domain (a Producer in OAIS terms). See `CONTEXT-MAP.md` → Domain | 2026-05-15 |
| 30 | Three-tier preservation chain role vocabulary | The two-tier "Archival Master / Service Master" framing is **retired**. Replaced by a three-tier role taxonomy distinguished by **purpose in the preservation chain**, not by format, location, or source provenance: **Preservation File** (long-term bit-level preservation; lives inside `dao:Representation`; owned by Archive context); **Service File** (mezzanine derivation under a derivation rule; owned by Access Area context); **Access File** (end-user delivery payload generated on demand; owned by the Access Area subdomain that serves the request). Originated in SIPI's IIIF Server vocabulary; promoted to cross-context Published Language. See `CONTEXT.md` → Preservation chain roles | 2026-05-15 |
| 31 | Archive deployment topology and public interfaces | The Archive bounded context is one logical unit deployed as **two services**: an **Ingest Area** (producer-facing async upload + SHACL/`DepositAgreement` validation gate; emits `DepositionAccepted` only on validation success; OAIS *Ingest* entity) and the rest of the Archive (event log, OCFL, public APIs; OAIS *Archival Storage* entity, with future Preservation Planning / Data Management / Administration entities). Both speak DAO directly; no anti-corruption layer between them. The Archive exposes **three public APIs**: Commands (HTTP), Events SSE (`text/event-stream`, resumable via `Last-Event-ID`, **full firehose** with no server-side filtering — subscribers filter client-side), Binary retrieval (`GET /bitstreams/{multihash}`, HTTP 206 Range supported). **OCFL is exclusive to the Archive boundary**; no other context reaches into the OCFL store. Deposition size is **producer-set** (Path A); realistic upper bound ~500K events for an extreme single submission; per-Deposition OCFL Objects are the proposed granularity. See §1a, §9.3 | 2026-05-15 |
| 32 | Access Area as federated subscribers; cold-replay strategy | Access Area is **one bounded context with N independent subscriber services** (one per DIP-shape subdomain: IIIF, HTML/Web Discovery, Custom Presentation, Asset/Download, SPARQL). Each subscriber maintains its own SSE cursor against Archive, its own storage tuned to its consumer's pattern, its own derivation logic. **Cold-replay strategy by use case**: (α) subscriber-side snapshots for routine restarts; (γ) subscriber-to-subscriber replication for spinning up duplicates of an existing subscriber kind; (δ) full SSE replay from genesis for the rare deploy of a brand-new subscriber kind. Archive serves historical events via the same SSE endpoint as live tail; a bulk-replay optimisation is **deferred** until measurement shows it is needed (see §9.5: ship and measure before optimising) | 2026-05-15 |
| — | DAO scope (folded in) | DAO governs the Archive (`dao:Representation`s containing Preservation Files), not the Access Area (Service Files and Service Projections are derived) or Access Area subdomain presentation views (DPE/CPE/IIIF, etc., are presentation, not preservation) | folded in (terminology updated 2026-05-15) |
| — | Service-tier versioning (folded in) | Service Files / Service Projections are derived projections, not versioned entities; regenerable by replay from Representation + derivation rule | folded in (terminology updated 2026-05-15) |
| — | Representation = preservation-grade bundle (folded in) | The `dao:Representation` class refers to the preservation-grade bundle (one or more Preservation Files plus Representation-level metadata); Service-tier projections are derivatives outside DAO's identity model | folded in (terminology updated 2026-05-15) |

---

## Appendix: terms used in this document, deliberately

- **DAO classes (write side)**: `dao:IntellectualEntity`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event`, `dao:Deposition`, `dao:DepositAgreement`, `dao:PreservationAction`. Listed in §6.
- **Read-side terms (NOT in DAO)**: IE Version, Representation Version, `versionNumber`, `isCurrentVersion`. Materialized by the read store; schema is the read store's concern.
- **Knora-base, project ontology** — VRE concerns, named here only to describe what gets transformed away during ingest.
- **DAO** — DaSCH Archival Ontology, OWL + SHACL.
- The earlier working terms **Resource** and **Asset** have been superseded by **IntellectualEntity** and **Representation** respectively, to avoid collision with `knora-base:Resource` and the VRE's existing use of "Asset", and to align with PREMIS preservation vocabulary.

---

## 9. Continuing this discussion in a future session

This document is the durable state of the design conversation. When resuming, read it top-to-bottom, then check this section for what is in-flight.

### 9.1 Methodology to honor when resuming

- **One question at a time.** Walk the design tree depth-first; resolve one branch before opening the next. Carry a recommended answer with each question so Ivan can react rather than guess what's being asked.
- **Cite or deviate (decision 27).** Every new decision either cites OAIS / PREMIS / a relevant standard, or carries an explicit "deviation, because …" note. The standards extracts under [`standards/`](./standards/) are grep-able; see [`standards/README.md`](./standards/README.md) for citation conventions.
- **Update `CONTEXT.md` inline** as new terms are resolved. Do not let it drift.
- **Don't reopen settled decisions** without flagging that you are doing so. The decision log in §8 is the source of truth on what's settled.

### 9.2 What was resolved in the session of 2026-05-11 / 2026-05-12 / 2026-05-13

- Decision 26 — `dao:PreservationAction` is a first-class concept, distinct from `dao:Deposition`.
- Decision 27 — OAIS + PREMIS alignment as default; deviations require documented reason.
- Decision 28 — `Tombstoned` (logical retraction) vs. `Redacted` (surgical content erasure) as two events; outright deletion explicitly *not* a DAO event.
- Decision 6 amended — DAO is conceptually built on PREMIS; the `dao:` namespace adds DaSCH-specific structure rather than replacing PREMIS.
- Decision 20 amended — PREMIS's `Representation → File → Bitstream` hierarchy is *deliberately collapsed* in DAO; deviation documented in §3.6.
- Standards extracts added under `standards/` (OAIS v3, PREMIS DD v3.0, PREMIS OWL Guidelines).
- `CONTEXT.md` rewritten — current entity glossary, identifier vocabulary, OAIS-vocabulary disambiguation.
- §7 → §8 renumbering fixed (the prior duplicate `## 7` is gone).
- Q9 in parked-questions list marked resolved; Q3 and Q14 marked partially resolved.

### 9.3 In-flight thread — Archive deployment topology and public APIs

This thread was opened in 2026-05-13 and resolved over 2026-05-14 / 2026-05-15. All confirmed inputs and the Q1 / Q2 questions are now answered. The doc has been updated to reflect them in §1a, §3.3, the decision log (decisions 29 / 30 / 31 / 32), and `CONTEXT.md`. What remains is documentation cleanup; the design itself is settled.

**Confirmed inputs (Ivan, 2026-05-13):**

1. **Push, not pull**, for the Archive→Access feed.
2. **Archive and Access Area are separate deployments.**
3. **OCFL is exclusive to the Archive.** No other component reaches into OCFL directly.
4. The Archive exposes three **public APIs**:
   - **Commands API** — for RDU-Tooling and the preservation-action runner (write side).
   - **Events SSE feed** — `text/event-stream`, resumable via `Last-Event-ID`, at-least-once delivery, per-entity ordering, heartbeats for proxy traversal. Subscribers: Access Area (one subscriber per subdomain — resolved 2026-05-15, decision 32), ARK Resolver, future projectors. **Full firehose**: every subscriber receives the full event stream and filters client-side; Archive carries no subscriber-facing filter grammar (resolved 2026-05-15, decision 31).
   - **Binary-retrieval API** — `GET /bitstreams/{multihash}` returning bytes (HTTP 206 Range supported). Used by Access Area subdomains at Service File derivation time; cached locally so user read paths never round-trip to the Archive.
5. **Scale recalibration:** ~2 deposits/week, but a single deposit may carry tens of thousands of IEs/Representations. Working estimate: **~200,000 events/week ≈ 2.3 events/s steady-state**, deposit-burst-driven. This invalidates the "tens of events/week" framing in §3.7 and decisions 22 / 23.

**Q1 — resolved 2026-05-14 (decision 31).** Worst-case single-Deposition size is **producer-set** (Path A): there is no Archive-enforced ceiling. Realistic upper bound is ~500K events for an extreme single submission (e.g. a fully-digitised monastic library). Multiple Depositions over a project's lifecycle correspond to **iterative deposit** as the project keeps working in VRE and periodically submits — not chunking of a single large submission.

**Q2 — resolved 2026-05-15 (decision 32).** Cold-replay is handled by combining three approaches by use case: **(α)** subscriber-side snapshots for routine restarts; **(γ)** subscriber-to-subscriber replication for spinning up duplicates of an existing subscriber kind; **(δ)** full SSE replay from genesis for the rare deploy of a brand-new subscriber kind. Archive serves historical events via the same SSE endpoint; a bulk-replay optimisation is deferred per the **ship and measure** principle (§9.5).

**Doc-cleanup queue — all done 2026-05-15:**

1. ~~§1a — explicit deployment topology~~ **Done.**
2. ~~New §3.7.1 (Public APIs of the Archive)~~ **Done.**
3. ~~Revise §3.7 — OCFL Object granularity from weekly time-bucket to per-aggregate Objects~~ **Done.**
4. ~~Revise decision 22 — re-baseline scale~~ **Done** (amendment note added inline).
5. ~~Revise decision 23 — per-aggregate granularity~~ **Done** (amendment note added inline).
6. ~~New decision for Archive deployment topology~~ **Done** (decisions 31 and 32).
7. ~~New §3.7.2 (Subscriber bootstrap)~~ **Done.**
8. ~~`CONTEXT.md` — boundary commitment note~~ **Done.**

### 9.4 Remaining open questions (priority order suggested for future sessions)

These are the items still parked in §7 that warrant the next several interview turns, ordered by my judgement of strategic priority. **Always cross-check OAIS + PREMIS per decision 27.**

1. **Q4 / Q5 push-topology details** — once Q1 and Q2 above are answered, finalize the SSE event-feed contract and Access-context event-vocabulary scope (Access events live in an `access:` namespace outside DAO).
2. **Q3 Deposition granularity** — partially resolved (separate `PreservationAction` decided). Remaining: one Deposition per project per submission event vs. one spanning multiple projects vs. open transactional session. Likely answer: one Deposition = one producer-side submission against one Project, with explicit `Deposition` aggregate boundary set by the producer (RDU-Tooling).
3. **Q12 `dao:AccessRights` for restricted access** — COAR's straightforward cases work as event-payload properties; `restricted_access` is non-trivial and may force a new DAO class. Likely involves a policy/predicate model: who can access, enforcement point, audit trail.
4. **Q10 Representation property list** — concrete grunt work. Walk PREMIS DD §1.2 (Object > File > Bitstream semantic units) and Knora `:FileValue` properties, produce the final canonical list shaped against decision 27 (PREMIS-aligned where possible; external standards per §4.4).
5. **Q11 ARK reservation expiry policy** — operational. Should reservations expire if never claimed; if so, how long; reclamation semantics.
6. **Q14 Preservation-action workflows (policy)** — `dao:PreservationAction` class is settled (decision 26). What's open: orchestration policy — who triggers, who reviews, how failures escalate.
7. **Q13 Project metadata as ontology** — large workstream; may surface `dao:Place`, `dao:Concept`, etc. Defer until the above are answered unless a near-term project demands it.
8. **Q1 Path inventory for new IE Version** — clarifying detail; non-blocking.
9. **Q2 Event ordering (per-entity vs global)** — per-entity is the working assumption; document explicitly.
10. **Q6 / Q7 DPE/CPE presentation hints and IIIF Manifests in DAO** — both lean "no"; document the no and move on.
11. **Q8 CoreTrustSeal evidence linkage** — annotation pass across the doc once the design stabilizes. Add a `CTS:` callout next to each decision that produces audit evidence.
12. **Fedora 6 OCFL patterns (research thread, not a design question).** Fedora 6 was the first major repository platform to commit to OCFL as the on-disk source of truth, with all databases treated as derived caches. §3.7 already cites this as the inspiration for decision 21. Open: what did Fedora 6 learn about OCFL Object granularity, multi-version management, large-Object handling, read-cache rebuild times, migration paths from Fedora 4/5 Akubra storage, and operational pain points? Useful input for revising decisions 21–23 and for the per-aggregate-Object proposal in §9.3.
13. **Fedora LDP (Linked Data Platform) lessons (research thread, not a design question).** Fedora 4/5 modelled their REST API on W3C LDP: containers, direct/indirect containers, RDF resources, pairtree containment. Open: what worked and what hurt? Specifically: (a) lessons for the Archive's Commands API shape; (b) lessons for how the Access Area exposes its read store to DPE/CPE/IIIF; (c) whether containment-as-LDP would over-constrain DAO's persistent-identity model. Cross-reference with PCDM (Portland Common Data Model), which Fedora-using institutions layered on top of LDP.

### 9.5 Process notes

- Standards PDFs live in [`standards/`](./standards/) as both `.pdf` and `.md` (verbatim text extracts via `pdftotext -layout`). The `.md` extracts are grep-able and exist specifically to make in-doc citation cheap.
- `CONTEXT.md` is the working glossary; treat it as authoritative for terminology that has stabilized. The discovery doc is the working *design narrative*; treat it as authoritative for decisions and their rationale.
- No ADRs have been written yet. The build-vs-buy revision (decision 17) and the OCFL-on-filesystem choice (decisions 21–23) are the strongest ADR candidates and should be written before implementation begins.
- **Ship and measure before optimising.** Decisions that affect operational performance (event volumes, replay strategies, storage size, derivation costs) ship in their simplest form first, run against real load, and are then optimised based on observed behaviour. The architecture is designed to allow this — full SSE firehose can become filtered, per-Deposition OCFL Objects can be sub-bucketed, derivation can move from eager to lazy. None of those refinements is pre-built. If a question of the form "should we optimise X now?" comes up, the default answer is *not until we have measured X under real load*.

