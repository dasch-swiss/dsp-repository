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
- **Archive** (OAIS Archive Component) — ingest, validation, audit, holds **Archival Masters**. **Working assumption: self-built (per decision 17, revised).** Storage architecture: OCFL on filesystem as source of truth; SQLite as read-side cache (§3.7).
- **Access Area** — holds **Service Masters** (access copies optimized per consumer). Single source for all read paths. Push-from-Archive vs. pull-from-Archive is undecided.
- **Read paths**: DPE (default Discovery and Presentation), CPE (Configurable Presentation, project-specific), asset server (downloads), IIIF server (image tile serving and Manifests).

Two committed architectural patterns shape DAO's design space:

- **Domain Driven Design** — bounded contexts, ubiquitous language, aggregates. **DAO is the ubiquitous language of the Archive context.** The Producer/Ingest context (RDU-Tooling, VRE export) and the Access context (Access Area, DPE/CPE/asset/IIIF) have their own internal vocabularies. **DAO terms appear at boundaries** (submissions arriving at the Archive, projection events leaving the Archive) — the lingua franca of context seams, not the universal language. (Anti-corruption layer not required under the self-built working assumption.)
- **Event Sourcing** — events are the source of truth in the Archive; Service Masters and access projections are derived (replayable) views.

**Project metadata format is decided**: RDF/Turtle with SHACL validation. DAO is one such ontology (the canonical archival one). The Archive treats project-level supplementary metadata as black-box storage.

**CoreTrustSeal is the certification target.** Several DAO design choices (immutable event log, fixity events, format migration provenance, no in-place migration) directly produce CoreTrustSeal evidence. Where this linkage applies, it is noted.

### Scope clarification: DAO governs the Archive, not the Access Area

DAO is the schema of what is **preserved in the Archive** (Archival Masters and their descriptive/preservation metadata). It is **not** the schema of Service Masters in the Access Area, nor of presentation views in DPE/CPE.

- **Archival Masters** (Representations in DAO terms) conform to DAO. Versioned. WORM. Source of truth.
- **Service Masters** are derived projections of Representations, shaped for specific consumers (pyramidal TIFF for IIIF, search indexes for DPE, denormalized RDF for SPARQL endpoints, etc.). They are **regenerable from Representations + transformation rule**. They do not carry their own versioning identity in DAO.
- **DPE/CPE views** are presentation, not preservation. Project-specific presentation customizations live outside DAO (CPE configuration, not archival content).

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
- **Representation** (Rep) — the **Archival Master**: a preservation-grade set of files (PREMIS allows multi-file Representations) plus Representation-level metadata (license, authorship, copyright, technical metadata such as filename, MIME type, originalFilename, originalMimeType). Has a stable internal IRI. Versions over time. Many-to-many with IntellectualEntities across Versions. Service Masters are derived from Representations and are not themselves Representations in DAO terms. Replaces what we initially called "Asset" (which had a different meaning in the VRE).
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

### 3.3 Service Masters are not versioned

Service Masters in the Access Area are **derived projections of Representation Versions**, not first-class versioned entities. When a Representation Version is created (i.e., on `RepresentationVersionCreated` event consumption), its Service Masters are (re-)derived. When derivation rules change (e.g., DaSCH adopts a new IIIF profile), Service Masters are re-derived from the unchanged events. Service Masters carry no ARK and no version number of their own — their identity is "the current derivation of Representation X v_n under derivation rule Y."

This aligns with Event Sourcing: Service Masters are projections, regenerable by replay.

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

**Scale check.** DaSCH targets 50-100 projects/year. Estimated tens of thousands of events per year, TB-scale content storage. This is small data — a single-machine architecture is right-sized. Heavy event-store / message-broker technology (Kafka, EventStoreDB) is operational overkill at this volume.

**Architecture: OCFL on filesystem as source of truth, SQLite as rebuildable read-side cache.**

```
/archive-root/
├── ocfl-storage-root/                         # SOURCE OF TRUTH
│   ├── ie/{uuid}/...                          # OCFL Object per IE (RDF metadata, versioned)
│   ├── rep/{uuid}/...                         # OCFL Object per Representation (bitstreams + RDF, versioned)
│   ├── projects/{uuid}/...                    # OCFL Object per Project
│   ├── depositions/{uuid}/...                 # OCFL Object per Deposition
│   └── events/{yyyy-Www}/...                  # OCFL Object per weekly event-log bucket
└── cache/
    └── archive.sqlite                         # rebuildable read-side cache
```

**OCFL on filesystem is the durable canonical state for everything**: bitstreams, RDF metadata, and the event log itself. Self-describing, hash-verified, navigable without DaSCH software (a future archivist with only the OCFL spec can recover everything). Inspired by Fedora 6's principle that the on-disk layout is the source of truth and all databases are derived caches.

**Event-log artifacts go into OCFL too.** Each event is a JSON-LD or Turtle document, batched into **weekly OCFL Objects** (`events/2026-W17/`). At DaSCH's scale (~tens of events per week on a busy week), weekly buckets are a comfortable granularity; can be revisited if write volume changes.

**SQLite cache holds the read-side projection.** Materialized Version nodes, indexed views ("all IEs in Project X", "current Version of IE Y", "all events for Representation Z in chronological order"). One SQLite file per archive instance. Embedded, transactional, no server. Rebuildable from OCFL by replaying events.

**No separate event-store technology.** Events are files in OCFL. Read-side projector reads them and updates SQLite. ARK Resolver (separate bounded context) consumes its own subset of events similarly. Polling cadence at this scale is trivial.

**Write path:**

1. Command arrives at the Archive service.
2. Validation runs (schema, uniqueness, authorization, etc.).
3. Event written to OCFL event-log bucket (append to weekly Object).
4. fsync.
5. SQLite updated transactionally.
6. Acknowledge success to Command issuer.

**Recovery:** if the process crashes between steps 4 and 5 (or SQLite is lost entirely), restart scans OCFL for events with timestamps after SQLite's last-applied event and replays them into SQLite. OCFL is the source of truth; SQLite catches up. Standard pattern.

**Sanctioned exception to OCFL immutability — Redaction.** A `Redacted` event (§5.1) is the **only** operation that may mutate or zero existing bytes in OCFL. The mutation is recorded as a new OCFL Object version with the offending bytes removed, never as in-place overwrite of an earlier OCFL version-directory contents. The `Redacted` event in the event log carries the legal basis and authorizing Agent; the OCFL Object's version-log carries the corresponding pointer. Every other operation in §5.1 is additive only. This exception exists because GDPR Art. 17 and equivalent legal obligations can compel erasure that immutable preservation cannot otherwise satisfy; it is bounded narrowly to redaction events and never extended to routine operation.

**What this gets DaSCH:**

- **Operational simplicity.** Two storage things: a filesystem and a SQLite file. Backups are filesystem snapshots.
- **Preservation purity.** OCFL is fully self-describing and self-verifying. No vendor or infrastructure dependency for long-term reading of the archive.
- **No infrastructure to scale or operate.** No Postgres, no Elasticsearch, no Kafka, no S3, no message broker.
- **Aligned with DaSCH's stated values:** architectural simplicity, long-term maintainability, escape from infrastructure-heavy designs.
- **Deferred decisions remain open.** If scale demands change, SQLite can be replaced by Postgres without changing OCFL or events. If geographic replication is required, OCFL replicates as filesystem-level mirroring. The architecture starts small and stays simple.

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
| `dao:Representation` | Persistent identity of a Representation (Archival Master). The URI events refer to. |
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
5. **Access Area events in DAO scope?** If push: do Access Area events (`ServiceMasterDerived`, `ServiceMasterInvalidated`, `ProjectionRebuilt`) belong in DAO's event vocabulary, or in a separate Access-context vocabulary? Lean: separate, because Service Masters are not in DAO's archival scope.
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
| 22 | Right-sized for DaSCH scale | Architecture deliberately right-sized for 50-100 projects/year (~tens of events/week). Single-machine, two storage things (filesystem + SQLite file). No infrastructure to scale or operate. Replaceable with heavier infrastructure later if scale demands change, without changing OCFL or events | — |
| 23 | Event log granularity | Weekly OCFL Object per event-log bucket (`events/{yyyy-Www}/`). Revisable if write volume changes | — |
| 24 | Events carry full snapshots | Even though OCFL stores per-version IE/Representation state, events carry full metadata snapshots (not just references). Redundancy is intentional: preserves replay-from-events-alone capability for disaster recovery | — |
| 25 | Fixity event vocabulary | One `FixityChecked` event with `outcome` ∈ {`pass`, `fail`, `missing`}. Failures and missings trigger preservation-action workflows (workflows are policy, parked separately) | — |
| 26 | `dao:PreservationAction` as a first-class concept | DaSCH-internal preservation actions (format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction) are modeled as `dao:PreservationAction`, **separate from `dao:Deposition`**. Rationale: different actor (Archive-as-system-agent vs. Producer); different authorization regime (internal preservation policy vs. `DepositAgreement`); CoreTrustSeal audit-trail separation of producer-induced vs. archive-induced changes (CTS R09/R12); cleaner event vocabulary (`PreservationActionExecuted` parallels `DepositionAccepted`). Aligned with `PREMIS DD §1.4` (Event entity); `OAIS §4.1.3 / §5.1` (Preservation Planning) | 2026-05-11 |
| 27 | OAIS + PREMIS alignment as default | Every DAO design choice is aligned with OAIS (CCSDS 650.0-M-3, Dec 2024) and PREMIS 3.0, **or** carries a documented deviation with reason in the decision log or an ADR. Standards extracts checked in under [`standards/`](./standards/) with citation conventions (`OAIS §<n.n.n>`, `PREMIS DD §<n.n>`). The alignment is **conceptual**: DAO uses its own URIs in the `dao:` namespace and may diverge in property names and structure where DaSCH-specific needs justify it, but the underlying concepts must be traceable back to OAIS/PREMIS or explicitly justified | 2026-05-11 |
| 28 | Tombstoning vs. redaction | Two distinct events. `Tombstoned` = **logical retraction** (bytes/metadata preserved in OCFL; read-side hides the Version; ARK returns tombstone landing page). `Redacted` = **surgical content-level erasure** producing a new redacted Version of the IE/Representation; the pre-redaction Version is `Tombstoned` and its OCFL bytes are over-written/zeroed — the **single sanctioned exception to OCFL immutability**. `Redacted` records who authorized the redaction and under what legal basis (e.g. GDPR Art. 17), not what was removed. Outright deletion is **not** a DAO event — it is a board-level exception in a separate governance log. Aligned with `PREMIS DD §1.4` Event entity (custom `Redaction` subtype) and `OAIS §3.3.5` (deactivation permitted; deletion not in normal operation). See §5.1 and §3.7 | 2026-05-12 |
| — | DAO scope | DAO governs the Archive (Archival Masters), not the Access Area (Service Masters are derived projections) or DPE/CPE (presentation, not preservation) | folded in |
| — | Service Master versioning | Service Masters are derived projections, not versioned entities; regenerable by replay from Representation + derivation rule | folded in |
| — | Representation = Archival Master | The Representation class refers to the Archival Master; Service Masters are derivatives outside DAO's identity model | folded in |

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

This thread was open when the session was paused. **Inputs from Ivan are confirmed; the doc has NOT yet been updated to reflect them.** Pick up here:

**Confirmed inputs (Ivan, 2026-05-13):**

1. **Push, not pull**, for the Archive→Access feed.
2. **Archive and Access Area are separate deployments.**
3. **OCFL is exclusive to the Archive.** No other component reaches into OCFL directly.
4. The Archive exposes three **public APIs**:
   - **Commands API** — for RDU-Tooling and the preservation-action runner (write side).
   - **Events SSE feed** — `text/event-stream`, resumable via `Last-Event-ID`, at-least-once delivery, per-entity ordering, heartbeats for proxy traversal. Subscribers: Access Area, ARK Resolver, future projectors.
   - **Binary-retrieval API** — `GET /bitstreams/{multihash}` returning bytes (HTTP 206 Range supported). Used by Access Area at Service Master derivation time; cached locally so user read paths never round-trip to the Archive.
5. **Scale recalibration:** ~2 deposits/week, but a single deposit may carry tens of thousands of IEs/Representations. Working estimate: **~200,000 events/week ≈ 2.3 events/s steady-state**, deposit-burst-driven. This invalidates the "tens of events/week" framing in §3.7 and decisions 22 / 23.

**Pending question from Ivan (not yet answered):**

> Q1 — What's a plausible worst-case single-deposit size? Tens of thousands is average; does any plausible deposit reach ~100K events (e.g. a 100K-page manuscript corpus with one IE per page)? Or is "tens of thousands" the realistic ceiling?

The answer shapes whether per-Deposition OCFL Objects are bounded enough or whether very large deposits need sub-bucketing.

**Second question held back, ask after Q1 is answered:**

> Q2 — Cold-replay strategy: a new subscriber facing ~10M events of history after a year of operation cannot tolerably project from genesis on every fresh deployment. Confirm the snapshot-plus-tail bootstrap (the Archive periodically writes projection snapshots that subscribers can fetch as a starting point, then tail the SSE feed from that event-id forward), or propose an alternative.

**Doc updates queued (to land after Q1 + Q2 are resolved):**

1. §1a — explicit deployment topology (Archive and Access Area separate; Archive owns OCFL exclusively; Archive exposes the three public APIs above).
2. New §3.7.1 (Public APIs of the Archive) — Commands, Events SSE, Binary retrieval. Auth, versioning, backpressure, error semantics.
3. Revise §3.7 — OCFL Object granularity changes from **weekly time-bucket** to **per-Deposition / per-PreservationAction Object** plus a complementary append-only chronological index keyed by event UUID and timestamp. Rationale: at 200K events/week, weekly OCFL Objects become filesystem-painful; aggregate-keyed Objects are causally meaningful and bounded.
4. Revise decision 22 — re-baseline scale (200K events/week steady, deposit-burst-driven, 10K+ events per deposit common; still right-sized for single-machine but with a much smaller comfort margin than originally written; "trivial polling cadence" claim removed since polling is being replaced by SSE).
5. Revise decision 23 — event-log granularity changes from weekly time-buckets to per-aggregate Objects.
6. New decision 29 — Archive deployment topology and public interfaces (push via SSE; OCFL Archive-exclusive; three public APIs; subscribers cannot reach OCFL directly).
7. New §3.7.2 (Subscriber bootstrap) — snapshot + tail strategy for cold replay; snapshot cadence TBD pending Q2.
8. `CONTEXT.md` — note that Access Area, ARK Resolver, and any other subscriber accesses the Archive only via its three public APIs; OCFL is internal to the Archive boundary.

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

### 9.5 Process notes

- Standards PDFs live in [`standards/`](./standards/) as both `.pdf` and `.md` (verbatim text extracts via `pdftotext -layout`). The `.md` extracts are grep-able and exist specifically to make in-doc citation cheap.
- `CONTEXT.md` is the working glossary; treat it as authoritative for terminology that has stabilized. The discovery doc is the working *design narrative*; treat it as authoritative for decisions and their rationale.
- No ADRs have been written yet. The build-vs-buy revision (decision 17) and the OCFL-on-filesystem choice (decisions 21–23) are the strongest ADR candidates and should be written before implementation begins.

