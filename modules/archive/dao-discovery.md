# DaSCH Archival Ontology (DAO) — Working Decision Log

*Working document, updated after each interview decision. Each section records what we decided and why; decisions can be revised but the revision is logged, not silently overwritten.*

**Status:** in-progress design conversation
**Last updated:** 2026-05-16 — Q15 RESOLVED by decision 43. DaSCH commits to **uniform CTS Levels A + C + D across all projects, all Depositions, all preserved content** — institutional invariant, no per-Agreement / per-Deposition / per-IE variation. Consequences: `dao:DepositAgreement` does NOT carry a preservation-level property; no `premis:preservationLevel` overrides in event payloads; no `CurationLevelChanged` event. §1a gained a preservation-commitment paragraph; §8.1 evidence index + coverage-check updated; Q15 in §7 marked RESOLVED. Prior 2026-05-16 entry: Curation & Preservation Levels Position Paper added to `standards/` (CCSDS position paper, Z / D / C / A taxonomy; CTS A required for in-scope). Prior 2026-05-16 entry: CTS extension. Added CoreTrustSeal Requirements 2026-2028 v01.00 (+ Extended Guidance + Glossary) to `standards/`; §8.1 Evidence index CTS column **remapped from working-memory 2023-2025 numbering to actual 2026-2028 numbering** (key shifts: storage R9→R14, preservation plan R10→R09, technical quality R11→R10, workflows R12→R11, identification R13→R12, reuse R14→R13); coverage-check table gained two rows (Legal & Ethical R04; Reuse-side Designated-Community engagement R13); §9.6 fifth-priority struck through as DONE. Prior 2026-05-16 entries: (i) evidence-index pass — §8.1 created mapping 20 decisions to three-tier citations + coverage-check table; (ii) certification-pyramid framing — nestor added, decision 42 records the pyramid, geographic-redundancy commitment made architecturally explicit in §3.7 Phase-2 paragraph; (iii) ISO 16363 + ISO 16919 added to `standards/`, decision 40 citations corrected to actual CCSDS 652.0-M-2 section numbers. Prior session entry (2026-05-15): sizing pass + §3.6 / §3.7 prose rewrite (decisions 40, 41; decisions 22 and 25 amended; numerical baseline re-locked at 100 deposits/year, 200K events/deposit, ~220M events/year at year 20 fixity-dominated, ~1 PB at year 20; storage prose aligned with decisions 36-39 two-substrate model). Prior session entry (2026-05-14/15): decisions 29-39 + decision 12 URN amendment.

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

**CoreTrustSeal is the certification target.** Several DAO design choices (immutable event log, fixity events, format migration provenance, no in-place migration) directly produce CoreTrustSeal evidence. Where this linkage applies, it is noted. See `standards/CoreTrustSeal-Requirements-2026-2028_v01.00.md` for the substantive requirements; decision 42 records the certification-pyramid framing (CTS → nestor → ISO 16363).

**Preservation-level commitment: uniform A + C + D across all projects** (decision 43). DaSCH's mandate commits the Archive to all three CTS Curation & Preservation Levels — **D** Deposit Compliance + **C** Initial Curation + **A** Active Preservation (cumulative, per the *CTS Curation & Preservation Levels Position Paper v3.0*, 2024) — for **every project, every Deposition, every IE / Representation / File**. No per-Agreement, per-Deposition, or per-IE variation. **Consequence for DAO**: the level is not a tunable; it is institutional invariant. `dao:DepositAgreement` does not carry a level property; events do not carry `premis:preservationLevel` overrides; no `CurationLevelChanged` event exists. If DaSCH ever introduces a tiered offering (e.g., Z-level publication-only for non-SNSF projects), decision 43 must be revisited.

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

Bitstreams (the actual file bytes of a Representation — the **Preservation Files** per the three-tier role vocabulary, decision 30) are stored **inside their Representation's OCFL Object**, in its content area. Events carry **references** to bitstreams: a content hash (multihash) plus file-level technical metadata. A **bytes-index** in `cache/redb` (decision 38) provides reverse lookup `multihash → (OCFL Object, content-path)` so the Binary retrieval API (`GET /bitstreams/{multihash}`, §3.7.1) can serve bytes without scanning the storage tree.

The earlier framing of bytes as living "outside the event log, in a content-addressable store" remains true at the level of *abstraction* the event payload sees — the event carries a hash, the content layer resolves it — but the *concrete substrate* is no longer a separate store: per decision 36, bytes live in the entity-storage substrate inside the Rep's OCFL Object, while events live in the event-log substrate. Two physical substrates, one logical content-addressed contract.

**Deviation from PREMIS, documented:** PREMIS 3 models `Representation`, `File`, and `Bitstream` as three distinct `Object` subclasses (`PREMIS DD §1.2.2`, `§1.2.3`, `§1.2.4`), each with its own identifier and metadata. DAO **collapses `File` and `Bitstream` into facts inside event payloads plus content-addressed bytes**. Reasoning:
1. **Event sourcing makes `File`-as-class redundant.** A file is what a `RepresentationVersionCreated` event creates; its existence and metadata are recorded in the event payload. A separate `dao:File` class would duplicate event-payload data on the write side.
2. **Content addressing makes `Bitstream` identity = hash.** PREMIS's `Bitstream` exists partly to identify byte-level units below the file level (e.g. one TIFF strip). DAO does not currently need sub-file granularity; if it ever does, hash-addressed bitstreams can be added without changing the `Representation` class.
3. **Reversibility.** If a future need emerges, the deviation can be reversed: each file-fact in an event payload could be projected into a read-side `File` node with `objectIdentifier = sha256:<hash>`, mechanically.

Per decision 27, this deviation is acceptable because it has documented reasoning and is reversible.

**Hash format and algorithm:** Multihash format (algorithm-flexible self-describing hash). **SHA-256 is the default today.** Multihash allows future migration to stronger algorithms — re-hashing existing bitstreams under a new algorithm produces additional `FixityChecked` events that record the new hash alongside the old.

**Event-payload reference shape:** events carry hash only (with technical metadata). The content store is responsible for "given a hash, return the bytes." The event payload does not carry storage URIs or paths — those are an internal concern of the content store, free to evolve without affecting events.

**File-level technical metadata in event payloads:** size in bytes, MIME type, PRONOM PUID (format identification), encoding, dimensions for images, duration for audio/video, etc. PREMIS-style technical metadata is **small**, carried in the event payload alongside the file's hash. Per-File information is **also rendered as blank-node-structured RDF on the Representation in OCFL** (decisions 33, 34) — see `rep.nt` inside the Representation's OCFL Object. There is no separate `dao:File` class; Files are described, not modelled as first-class entities.

**On-disk RDF serialisation — two formats, two substrates** (decisions 34 + 37):

- **Entity-state RDF inside entity-storage-root OCFL Objects: N-Triples** (decision 34). The `rep.nt`, `ie.nt`, `project.nt`, etc. files inside each entity Object's content area are N-Triples. Line-oriented, prefix-free, deterministic byte representation — critical for OCFL fixity hashes over RDF content.
- **Events inside event-log-storage-root (and on the SSE wire): NDJSON-of-JSON-LD** (decision 37). Each event is one self-contained JSON-LD document per line, with explicit `@id` / `@type` / `stream_id` / `stream_version` / `global_offset` / `timestamp` / `event_schema_version` / `crc32` plus event-type-specific payload. JSON-LD is RDF-equivalent (it parses into the same triples), but the on-disk format is line-oriented JSON-LD rather than N-Triples because events benefit from per-line corruption detection (CRC32), schema-version tagging, and ordering-friendly streaming.

The **Commands API** accepts JSON-LD or Turtle for content negotiation; normalisation happens at the API boundary on write (to N-Triples for entity-state inside OCFL; to NDJSON-JSON-LD for events). The **Events SSE feed** emits NDJSON-of-JSON-LD on the wire — one event per SSE `data:` line.

Example shape of a `RepresentationVersionCreated` event line in the NDJSON-of-JSON-LD log (decision 37). Whitespace added for readability; on-disk the line is compact:

```jsonc
{
  "@id": "urn:dsp:event:01958ab2-...",             // event identity (UUIDv7)
  "@type": "RepresentationVersionCreated",
  "stream_id": "urn:dsp:rep:01958a4f-...",         // persistent identity of the Rep (decision 12 URN)
  "stream_version": 2,                             // per-Rep monotonic counter
  "global_offset": 1234567,                        // monotonic across all events
  "timestamp": "2026-04-15T09:42:18.331Z",
  "event_schema_version": 1,
  "files": [
    {
      "filename": "scan-001.tif",
      "hash": "sha256:abc123...",                  // multihash; identifies the bitstream
      "sizeBytes": 145000000,
      "mimeType": "image/tiff",
      "pronomId": "fmt/353"
    },
    {
      "filename": "scan-001.xmp",
      "hash": "sha256:def456...",
      "sizeBytes": 8192,
      "mimeType": "application/rdf+xml"
    }
  ],
  "license": "spdx:CC-BY-4.0",                     // SPDX URI per §4.4
  "copyrightHolder": "...",
  "authorship": "...",
  "crc32": "..."                                   // per-line corruption detection (decision 37)
}
```

The bytes for the listed files live inside the Rep Object's OCFL content area at `entity-storage-root/{hashed-path}/urn:dsp:rep:{uuid}/v{n}/content/{filename}`. The bytes-index in `cache/redb` provides the multihash → location lookup that the Binary retrieval API uses; subscribers and Producers never see the path.

### 3.7 Storage architecture (right-sized for DaSCH)

**Working assumption:** self-built Archive Component (per decision 17, revised). DaSCH owns the storage implementation directly; no anti-corruption layer needed; DAO is the storage model.

**Scale check (re-baselined 2026-05-15, sizing pass).** DaSCH targets **100 projects/year as a hard cap**, reached over ~2 years from 12-15/year today. The ~50 existing VRE projects migrate into the Archive one-by-one alongside net-new projects; iterative-via-refunding (e.g. CAS / shortcode 0812) is counted as a *new* project for each refunded round, not as iterative-within-project. Deposit pattern is **one-shot**: ~100 deposits/year, **~200K events per deposit** (≈ 100K IE + 100K Rep + 1 `DepositionAccepted`; IE-to-Rep ratio ~1:1; ~1.5 Preservation Files per Rep on average per decision 33; avg Rep ~5 MB). **Ingest steady-state: 20M events/year, ~50 TB/year storage.**

**Fixity dominates from year 5 onward.** Under decision 40 (per-OCFL-Object granularity, continuous-when-idle + ≥1 sweep/Object/year), total event volume tracks accumulated Rep count rather than ingest flow:

| Year | Cumulative Reps (= OCFL Objects) | Annual ingest events | Annual fixity events | **Total events / year** | Steady-state rate |
|---|---|---|---|---|---|
| 1 | 10M | 20M | 10M | **30M** | ~1 event/s |
| 5 | 50M | 20M | 50M | **70M** | ~2.2 events/s |
| 10 | 100M | 20M | 100M | **120M** | ~3.8 events/s |
| 20 | 200M | 20M | 200M | **220M** | ~7 events/s |

Cumulative event count at year 20 ≈ **2.4 B events** (mostly fixity). At ~1 KB/event in NDJSON-JSON-LD that's ~2.4 TB of event log uncompressed; ZFS compression typically 0.5-1 TB. Cumulative bytes storage at year 20 ≈ **1 PB** (PB-scale at the 20-year horizon, not TB-scale as the prior amendment claimed).

Still small enough for a single-machine architecture; heavy event-store / message-broker technology (Kafka, EventStoreDB) remains operational overkill at this volume. Cold-replay-from-genesis at year 20 is **the first number that genuinely presses the ship-and-measure principle** (§9.5): 2.4 B events at conservative 10K events/s SSE throughput = ~67 hours; at 50K events/s = ~13 hours. This is the bound that may justify the deferred bulk-replay optimisation by year 10-15.

**Architecture: two ZFS-backed OCFL storage substrates plus a rebuildable cache** (decision 36).

```
/archive-root/
├── event-log-active/                              # ACTIVE EVENT LOG (outside OCFL)
│   └── segment-{seq}.ndjson                       # unsealed; appended-to; fsynced per event during deposit bursts
│
├── event-log-storage-root/                        # OCFL STORAGE ROOT #1 — sealed event segments
│   ├── 0=ocfl_1.1                                 # namaste (OCFL conformance declaration)
│   ├── ocfl_layout.json                           # storage-layout extension
│   └── {hashed-path}/urn:dsp:event-segment:{period}/
│       ├── 0=ocfl_object_1.1
│       ├── inventory.json
│       └── v1/content/
│           ├── segment-{seq}.ndjson               # sealed segment (NDJSON-of-JSON-LD; decision 37)
│           └── segment-{seq}.sha256               # per-segment fixity manifest (sha256sum -c compatible)
│
├── entity-storage-root/                           # OCFL STORAGE ROOT #2 — entity state + Preservation File bytes
│   ├── 0=ocfl_1.1
│   ├── ocfl_layout.json
│   └── {hashed-path}/urn:dsp:rep:{uuid}/          # one OCFL Object per DAO entity URN (decision 36)
│       ├── 0=ocfl_object_1.1
│       ├── inventory.json
│       └── v{n}/content/
│           ├── rep.nt                             # entity-state RDF (N-Triples; decision 34)
│           ├── scan-001.tif                       # Preservation File bytes
│           ├── scan-001.tif.sha256                # per-File fixity sidecar (decision 39)
│           ├── scan-001.xmp
│           └── scan-001.xmp.sha256
│       # (analogous Object trees under urn:dsp:ie:{uuid}, urn:dsp:project:{uuid},
│       #  urn:dsp:agent:{uuid}, urn:dsp:agreement:{uuid}, plus commit-aggregate
│       #  Objects urn:dsp:deposition:{uuid} and urn:dsp:preservation-action:{uuid}
│       #  which hold audit records only — no Preservation Files)
│
└── cache/                                         # REBUILDABLE FROM THE TWO SUBSTRATES
    └── archive.redb                               # Redb (decision 38): read-side projection,
                                                    # event-log index, bytes-index. All three rebuildable.
```

**Two source-of-truth substrates, one rebuildable cache.** Per decision 36 the Archive deliberately separates the **event log** (NDJSON-JSON-LD segments wrapped as OCFL Objects in `event-log-storage-root/`) from **entity state and Preservation File bytes** (one OCFL Object per state-aggregate or commit-aggregate entity in `entity-storage-root/`). Both are ZFS-backed, both are proper OCFL with namaste declarations and storage-layout extensions, both are self-describing and self-verifying. Inspired by Fedora 6's OCFL-as-source-of-truth principle, with the event-log / entity-state separation that Datomic, EventStoreDB, and Kafka-with-tiered-storage conventions universally adopt for CQRS-ES at non-trivial scale (decision 36 rationale; the earlier single-OCFL-store framing had no documented prior art and is retired).

**Event-log segments.** Active segments live in `event-log-active/` outside OCFL, appended-to in real time. A segment **seals** on a hybrid trigger: monthly OR ~100 MB, whichever first (decision 37). On seal, the segment is moved into `event-log-storage-root/` as a new OCFL Object's content, accompanied by its SHA-256 sidecar manifest (decisions 37 + 39); the active segment counter rolls over. This is how the entire event history becomes hash-verified and OCFL-portable while still permitting low-latency fsync-per-event during deposit bursts.

**Entity Objects version on curatorially-meaningful state changes.** A `urn:dsp:ie:{uuid}` Object adds a new OCFL version whenever an `IntellectualEntityVersionPublished` event commits; a `urn:dsp:rep:{uuid}` Object adds a new OCFL version on every `RepresentationVersionCreated` (bytes + updated `rep.nt`); a `urn:dsp:project:{uuid}` Object on Project metadata changes; and so on. Commit-aggregate Objects (`urn:dsp:deposition:{uuid}`, `urn:dsp:preservation-action:{uuid}`) are written once on commit and never re-versioned — they hold the audit record of *that the commit happened*, not ongoing state. Each entity OCFL Object version is the Archive's own snapshot of that aggregate at that point, distinct from subscriber-side α snapshots in the Access Area (§3.7.2).

**Redb cache (`cache/archive.redb`)** holds three indexed workloads (decision 38), all fully rebuildable from the two substrates:

- **Read-side projection cache** — materialised Version nodes, IE-in-Project lookups, current-Version-of-IE, "all events for Rep Z in chronological order", etc. Backs queries that should not scan OCFL.
- **Event-log index** — `(stream_id, stream_version) → (segment_id, byte_offset)` and `global_offset → (segment_id, byte_offset)`. Lets the SSE handler stream events in chronological order without scanning segment files; lets subscribers resume from any `Last-Event-ID`.
- **Bytes-index** — `multihash → (containing-OCFL-Object, content-path)`. Backs the Binary retrieval API (`GET /bitstreams/{multihash}`, §3.7.1) without scanning the entity-storage tree.

Losing `cache/archive.redb` is recoverable, just slow: scan sealed segments to rebuild the event-log-index; scan entity Object inventories to rebuild the bytes-index; replay events to rebuild the read-side projection cache. Redb is convenience and latency optimisation, not a source of truth.

**No separate event-store technology.** The event log is files inside OCFL Objects; the SSE handler is a thin layer over the event-log-index in Redb plus segment reads. No Kafka, no EventStoreDB, no Postgres, no separate message broker. The ARK Resolver (separate bounded context) consumes its own subset of events via the public SSE feed (§3.7.1).

**Write path (for a Deposition).**

1. Command (`SubmitDeposition`) arrives at the Archive (Ingest Area).
2. Validation runs: SHACL against DAO, `DepositAgreement` enforcement, fixity, authorisation.
3. On validation success:
   1. **Write-ahead the events** to the active event-log segment (NDJSON-JSON-LD lines with CRC32 per decision 37). fsync per event during the burst.
   2. **Materialise entity Object versions** in `entity-storage-root/`: for each IE in the Deposition, add a new OCFL version of `urn:dsp:ie:{uuid}`; for each Rep, add a new OCFL version of `urn:dsp:rep:{uuid}` containing the Preservation File bytes plus updated `rep.nt`. Write the commit-aggregate audit Object `urn:dsp:deposition:{uuid}` once.
   3. **Update Redb** transactionally: append to event-log-index, append to bytes-index for each new file, update the read-side projection.
4. Acknowledge success to the Command issuer.

If the active segment hits its hybrid roll-over (monthly or ~100 MB), seal it into `event-log-storage-root/` as an OCFL Object and start a fresh active segment.

**Write path for a PreservationAction** is analogous: events written to the active segment; affected entity Objects gain new OCFL versions for any state they change (e.g. a new `urn:dsp:rep:{uuid}` version for a `FormatMigrated` action); commit-aggregate audit Object `urn:dsp:preservation-action:{uuid}` written once.

**Recovery.** OCFL is the durable source of truth across both substrates; Redb is a rebuildable cache. On a clean restart, Redb is consistent and no work is needed. On crash recovery:

- If the active event-log segment has a partial trailing line (CRC32 fails on the last record), truncate to the last valid line and continue (decision 37).
- If Redb's last-applied `global_offset` lags the latest segment, replay forward from that offset to bring Redb consistent.
- If Redb is lost entirely, rebuild from scratch: scan sealed + active segments to reconstruct the event-log-index; scan entity-storage Object inventories to reconstruct the bytes-index; replay events to rebuild the projection cache. Bounded by accumulated event and Object count (§3.7 scale table): at year 10 (~100M Reps, ~700M cumulative events) this is hours, not days. At year 20 it pushes toward day-scale — same ship-and-measure pressure as cold-replay-from-genesis (§3.7.2 δ).

**Sanctioned exception to OCFL immutability — Redaction.** A `Redacted` event (§5.1) is the **only** operation that may mutate or zero existing bytes in OCFL. The mutation is applied to **both substrates**: in `entity-storage-root/`, the affected Rep Object gets a new OCFL version with the offending bytes removed (never as in-place overwrite of an earlier version-directory's contents); in the event log, the `Redacted` event records *that* the redaction happened, *who* authorised it, and *under what legal basis* (e.g. GDPR Art. 17, court order) — it does **not** record *what* was removed. The pre-redaction Version is `Tombstoned` in the same commit. Every other operation in §5.1 is additive only. This exception exists because GDPR Art. 17 and equivalent legal obligations can compel erasure that immutable preservation cannot otherwise satisfy; it is bounded narrowly to redaction events and never extended to routine operation.

**What this gets DaSCH.**

- **Operational simplicity.** Three storage things on disk (active log directory + two OCFL roots + Redb cache file), all on one ZFS-backed filesystem. Backups are filesystem snapshots; replication is `zfs send` or filesystem-level mirroring.
- **Preservation purity.** Both substrates are fully self-describing and self-verifying OCFL. A future archivist with only the OCFL spec, an NDJSON parser, and the DAO ontology extracts can recover everything — no DaSCH software dependency for long-term reading.
- **CQRS-ES discipline at the storage layer.** Events and entity state are physically separate substrates; neither can drift into the other's role. Aligns with how Datomic / EventStoreDB / Kafka separate events from large blobs (decision 36 rationale).
- **No infrastructure to scale or operate.** No Postgres, no Elasticsearch, no Kafka, no S3, no message broker. Redb (pure-Rust embedded ACID + MVCC; decision 38) replaces the prior SQLite reference; the decision-22 "two storage things" framing now reads as "two OCFL roots + one Redb cache file + one active-log directory."
- **Deferred decisions remain open.** If scale demands change, Redb can be replaced by Postgres on the read side without changing OCFL or events. If geographic replication is required, both OCFL roots replicate as filesystem-level mirroring; the event-log root's append-only nature is replication-friendly. The architecture starts small and stays simple.

**Phase-2 geographic replication (architecturally committed; deployment-deferred — per decision 42).** A geographically-distant disaster-recovery copy is required by `nestor §14` (a fire in the operating institution's main building must not destroy objects) and is implied by `ISO 16363 §5.1.2` + `§5.2.1` (manage number/coordination/location of copies + risk analysis of insufficient distancing such that all copies could be hit by the same natural disaster). The two-substrate architecture above is designed for this:

- **Both OCFL roots** (`event-log-storage-root/` and `entity-storage-root/`) replicate to a remote ZFS pool via `zfs send` (preferred — atomic snapshots, fixity preserved) or `rsync` with content-addressed verification (fallback for non-ZFS targets). Both roots are append-mostly (entity Objects gain versions; event-log Objects are sealed-once); replication is efficient.
- **The active event-log directory** (`event-log-active/`) is the only mid-write substrate. Replication tolerates partial-trailing-line truncation per decision 37's CRC32 recovery rule, so the remote can safely synchronise even while a deposit burst is in flight.
- **`cache/archive.redb`** is *not* replicated — it is fully rebuildable from the two OCFL substrates (decision 36 + 38). The remote starts with an empty Redb and rebuilds on first activation, or eagerly if low-RTO failover is required.
- **Multi-copy synchronization** (`ISO 16363 §5.1.2.1`) is satisfied by the replication tooling's own atomic-snapshot semantics; the Archive itself does not need to know about the remote copy.

Implementation is deferred to a Phase-2 milestone — operational, not architectural; falls within the ship-and-measure principle (§9.5). The commitment recorded here is that the architecture does not foreclose this option.

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
- **Lookup mechanism:** internally the API consults the **bytes-index** in `cache/archive.redb` (decision 38) to resolve `multihash → (containing OCFL Object in entity-storage-root, content-path)`, then streams from disk. Subscribers and Producers never see the storage path; the bytes-index is rebuildable from entity-Object inventories if `cache/archive.redb` is lost.
- **Range requests:** HTTP 206 Partial Content supported, for clients that want to stream large files.
- **Caching:** content is immutable (the multihash binds bytes to identity), so the response carries long `Cache-Control` lifetimes and an `ETag` of the multihash itself. Access Area subscribers cache aggressively at derivation time.
- **Auth:** mTLS or bearer-token; same auth as the SSE feed. (Access decisions are made by the Access Area subdomain at user-request time, not at this layer — the Archive only authenticates that the caller is an authorised subscriber.)
- **Errors:** 404 if the multihash is unknown; 410 Gone if the bytes were intentionally redacted (the `Redacted` event in the relevant Representation history explains why).

#### Operational metrics endpoint (not part of the bounded-context contract)

The Archive also exposes a **`/metrics` endpoint** scraped by Grafana for operational observability — event rate, fixity sweep progress, queue depth, OCFL Object counts, disk usage, write latency, SSE subscriber lag. Per decision 41 this is **operational telemetry, not archival evidence**: Grafana retention is finite, the endpoint can be replaced or reset without affecting the preservation commitment, and no Producer or Subscriber should depend on it as a contract. The three public APIs above (Commands, Events SSE, Binary retrieval — decision 31) remain the bounded-context boundary; `/metrics` is a fourth surface internal to operations.

Audit-grade evidence of monitoring activity lives in the event log (e.g., routine `FixityChecked` pass-events per decision 40 and §3.8), not in Grafana. The two layers are deliberately separate: the event log is durable and tied to the preservation commitment; Grafana is operationally convenient and disposable.

### 3.7.2 Subscriber bootstrap (cold replay)

Three bootstrap strategies are supported, by use case (decision 32):

**(α) Subscriber-side snapshots — for routine restarts.** Each subscriber periodically writes its derived state to a snapshot, with the `event_id` of the last applied event embedded. Snapshot cadence is the subscriber's choice (every N events, every M minutes, or after every Deposition commit). On restart, the subscriber:

1. Restores its state from the latest snapshot.
2. Opens the Archive SSE with `Last-Event-ID = snapshot.last_event_id`.
3. Receives all events since the snapshot, then transitions to live tail.

**(γ) Subscriber-to-subscriber replication — for spinning up duplicates.** When a second instance of an existing subscriber kind is brought up (HA, scaling, geographical replication), it copies state from an existing peer rather than replaying from Archive. The existing peer exposes an internal "snapshot export" endpoint. After the copy, the new instance tails Archive's SSE from `Last-Event-ID = peer.last_event_id`. Only works between subscribers running the *same* derivation logic version.

**(δ) Full SSE replay from genesis — for brand-new subscriber kinds.** When deploying a subscriber kind that has never run before (e.g. a new SPARQL projector with new derivation logic), the subscriber opens the SSE with `Last-Event-ID = 0` and consumes the entire historical event stream. Slow but bounded; expected to be a rare one-time event. A bulk-replay mode (returning events in batches rather than as a paced SSE stream) is a deferred optimisation per §9.5 — added only when measurement under real load shows that paced SSE replay is intolerable.

**Snapshot durability and recovery for (α).** Snapshots are the subscriber's own responsibility, not Archive's. Lost snapshots fall back to (δ). Subscribers should keep at least two recent snapshots to survive corruption of the most recent one.

**No Archive-side snapshot mechanism for Subscribers.** The Archive does not maintain or serve subscriber-shaped snapshots — that would require the Archive to know each subscriber's derivation logic, which it must not.

**Note on Archive-side snapshots that *do* exist.** Each entity OCFL Object version in `entity-storage-root/` *is* the Archive's own snapshot of that aggregate's state at that point — `urn:dsp:ie:{uuid}/v3` is the IE's state after the third publish event, materialised in N-Triples (decision 36). These are **preservation snapshots** of the canonical entity state, not subscriber-projection snapshots. Subscriber-side α snapshots derive *projected* state shaped for the Subscriber's consumer (e.g., a SPARQL graph, a search index, an IIIF manifest cache) and are not interchangeable with the entity OCFL versions. The two snapshot kinds coexist: entity Object versions serve preservation and direct-state replay; α snapshots serve fast Subscriber restart.

### 3.8 Fixity checks

A `FixityChecked` event records the result of verifying an **OCFL Object** (= a `dao:Representation`'s storage container) against its OCFL inventory manifest. Granularity is **per Object, not per File** (decision 40): one event per Rep Object validate, with per-File outcomes inside the payload for fail/missing cases. This matches `ocfl validate` as the operational unit and aligns with PREMIS Event semantics (Events bind to PREMIS Objects, and `dao:Representation` is the Object-level entity per decision 33).

**Cadence (decision 40):** continuous fixity sweep running in idle I/O windows, with a hard commitment of **at least one sweep per OCFL Object per year**. Aligns with `ISO 16363 §4.4.1.2` (*"The repository shall actively monitor the integrity of AIPs."*) and `ISO 16363 §5.1.1.3` (*"The repository shall have effective mechanisms to detect bit corruption or loss."*). Fail/missing reporting workflow aligned with `ISO 16363 §5.1.1.3.1` (*"The repository shall record and report to its administration all incidents of data corruption or loss..."*). The continuous-when-idle + ≥1/year cadence exceeds the standard's documented-policy-consistently-executed bar. Produces CoreTrustSeal R10 / R11 evidence.

**Outcomes** ∈ {`pass`, `fail`, `missing`}:

- **`pass`** — Object validates against its OCFL inventory. Event payload is minimal (~250 bytes uncompressed): Object IRI, version, timestamp, `outcome: pass`. The event itself is the audit-grade attestation that the check happened on that date. Routine, expected, the overwhelming majority.
- **`fail`** — one or more Files in the Object hash to a different value than the inventory. **Preservation incident.** Event payload carries per-File outcomes (filename, expected hash, recomputed hash, per-File result) plus checker version (~2-5 KB).
- **`missing`** — one or more Files are not retrievable. Same incident response as `fail`; same payload shape.

**Variant payload by outcome (decision 40).** Pass-records are *attestation*; fail/missing records are *forensic evidence*. Different artefacts, different durability needs — but both belong in the WORM event log so a future auditor can prove monitoring activity over the entire history without depending on operational systems. Operational metrics (sweep progress, throughput, queue depth) live in Grafana and are operational, not archival — see §3.7.1 and decision 41.

**Storage consequence.** At year 20 (200M Reps), the (a)/(c) variant split keeps total fixity-event bytes to ~52 GB/year — roughly 8× smaller than uniform per-File events would produce. Fixity events comfortably fit into decision-36 monthly segments without forcing the size trigger.

The event itself is the durable record of the check; the workflow that responds to `fail` and `missing` outcomes is a separate concern (parked, Q14).

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
- **`FixityChecked`** — integrity verification of an **OCFL Object** against its inventory (= a `dao:Representation`'s storage container). Granularity per Object, not per File (decision 40). Outcome ∈ {`pass`, `fail`, `missing`}. **Variant payload by outcome**: `pass` = minimal attestation (~250 bytes — Object IRI, version, timestamp, outcome); `fail`/`missing` = per-File forensic detail (~2-5 KB — filename, expected hash, recomputed hash, per-File result, checker version). Both shapes belong in the WORM event log; routine pass-records are audit-grade attestation, fail/missing records are forensic evidence. Cadence: continuous-when-idle + ≥1 sweep/Object/year. Failures and missings trigger preservation-action workflows (parked, Q14). See §3.8.
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
- **Files within a Representation**: per-File information lives as blank-node-structured properties on `dao:Representation` via `dao:hasFile`, navigable from the Representation's RDF in its OCFL Object (`rep.nt`). **Not a separate DAO class** (decision 33). Per-File information is addressed by `dao:filename` within the parent Representation Version's context. For event references to a specific File: use the (Representation Version IRI, filename) tuple. Per-file Bitstream-level sub-addressing remains parked.
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
14. **Preservation-action workflows (response and format-migration policy).** Partially engaged. **Fixity *cadence* and *granularity* settled (decision 40)**: per OCFL Object, continuous-when-idle + ≥1 sweep/Object/year, variant payload by outcome. **Still parked:** (i) response workflow when `FixityChecked` returns `fail` or `missing` — alerting, triage, recovery from secondary copies, who-decides-what; (ii) format-migration triggering policy — when does DaSCH initiate a `FormatMigrated` action, against which format-risk signals (PRONOM advisories, vendor obsolescence notices, etc.), with what stakeholder review; (iii) system-ontology-migration policy. These are orchestration concerns the Archive emits events for, not DAO classes. The events themselves belong to `dao:PreservationAction` (decision 26).

15. ~~**Curation-level commitment per `dao:DepositAgreement`.**~~ **RESOLVED 2026-05-16 by decision 43.** DaSCH commits to uniform A + C + D across all projects, all Depositions, all preserved content (per the CTS Curation & Preservation Levels Position Paper v3.0). No per-Agreement / per-Deposition / per-IE variation. DAO does not model preservation-level as a tunable — the commitment is institutional invariant. The sub-questions about granularity, override events, and PreservationAction validity are moot. See decision 43 and §1a preservation-commitment paragraph.

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
| 12 | Internal identifiers | UUIDv7-based identifiers in DaSCH-controlled namespace. **Amended 2026-05-15:** internal persistent-identity IRIs use the **URN scheme** `urn:dsp:{type}:{uuid}` (e.g., `urn:dsp:ie:01234567-89ab-...`, `urn:dsp:rep:...`, `urn:dsp:project:...`, `urn:dsp:agent:...`, `urn:dsp:agreement:...`, `urn:dsp:deposition:...`, `urn:dsp:preservation-action:...`, `urn:dsp:event-segment:{period}`). Internal IDs are **not dereferenceable**; they exist purely for cross-event references inside the system. The earlier `https://archive.dasch.swiss/{type}/{uuid}` HTTPS form is retired (rhetorically implied HTTP-resolvability we don't need internally; tied to a domain that may change across rebrands or migrations; Fedora 6 sets the precedent with `info:fedora/...` URNs). **Read-side URLs** served by Access Area subdomains for browser/client access (e.g., `https://dpe.dasch.swiss/ie/{uuid}/v{n}`, `https://iiif.dasch.swiss/{shortcode}/{rep-uuid}/manifest.json`) remain HTTPS URLs and are dereferenceable. **ARKs** are NOT internal identifiers; they are public lookup-and-forwarding handles managed by a separate ARK Resolver bounded context. **Stability layers** (strongest first): ARK → URN internal IRI (stable within a system; not promised across system migrations) → Read-side URL (read-store contract; honoured as long as the event log is replayable). | — / amended 2026-05-15 |
| 13 | ARK strategy | ARK Resolver is its own bounded context with its own event store. ARKs may be (a) supplied by RDU-Tooling for migration of VRE-era ARKs (DSP-version `1`); (b) supplied by RDU-Tooling for reservation before publication (DSP-version `2`); (c) auto-minted by Resolver on publish events (DSP-version `2`). VRE-era ARKs migrated as one-time bulk registration. Value-level ARKs map to DPE deep links; no new Value ARKs minted by Repository | — |
| 14 | Commands vs. Events (CQRS-ES) | Commands are intents that may be rejected; Events are facts emitted only after successful Command validation. Validation (incl. collision detection, uniqueness checks) happens at Command time. DAO models Events and persistent-identity entities; Commands are transient and not preserved | — |
| 15 | Top-level DAO class list (write side only) | `dao:IntellectualEntity`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event` (with subclasses), `dao:Deposition`, `dao:DepositAgreement`. (`dao:PreservationAction` added later as decision 26.) Version classes (`dao:IntellectualEntityVersion`, `dao:RepresentationVersion`) explicitly **NOT** in DAO — they are read-side projections. Relationships are properties, not reified. `dao:AccessRights`, `dao:Place`, `dao:Concept` parked pending project metadata work | — |
| 16 | Events vs. Version nodes (CQRS separation) | Events are the source of truth (write side, in DAO). Version nodes are materialized projections (read side, NOT in DAO). The read store guarantees Version-suffixed URL contracts as long as event log is replayable. Version node properties are derived from events; the projection is regenerable by replay | — |
| 17 | Archive Component build-vs-buy posture | **Working assumption: self-built.** DaSCH owns the Archive implementation; DAO is the storage model directly; no anti-corruption layer required. (Earlier working assumption was commercial/Docuteam; revised after right-sizing analysis showed DaSCH's scale and operational simplicity arguments favor self-built with OCFL-on-filesystem.) | — |
| 18 | DepositAgreement and AccessRights renaming | Submission Agreement → `dao:DepositAgreement` (DaSCH internal naming; may be a link to the document store). AccessRule → `dao:AccessRights`. The straightforward COAR cases (open, embargoed, metadata-only) are simple properties; `restricted_access` is non-trivial and parked as open Q12 | — |
| 19 | ARK is the single long-term stability commitment | ARKs are minted **per persistent-identity entity** (IE, Representation, Project, etc.), not per Version. A specific Version is denoted by suffix (`/v{n}` or VRE-era timestamp). ARK string remains stable; suffix selects the Version. DPE handles Version display by querying the read store. Internal IRIs and read-side URLs are not promised to outlive a system migration; only ARKs are | — |
| 20 | Bitstream storage | Bitstreams stored outside event log in a content-addressable store. Events carry hash references (multihash format, SHA-256 default) plus PREMIS-style file-level technical metadata in payload. Storage URIs are not in event payloads; the content store owns "given a hash, return bytes". **Deviation from PREMIS, documented (per decision 27):** PREMIS models `Representation → File → Bitstream` as three `Object` subclasses; DAO collapses `File` and `Bitstream` into facts in event payloads + content-addressed bytes. Reasoning: event sourcing makes `File`-as-class redundant; content addressing makes `Bitstream` identity = hash. Reversible — file-facts can be projected into read-side `File` nodes if needed. **Amended 2026-05-15 (decision 33):** the deviation is preserved at the class level (no `dao:File`, no `dao:Bitstream` class). What is clarified post-Fedora-research: per-File information that was previously described as "living only in event payloads" is also rendered as blank-node-structured RDF on the Representation in OCFL (`rep.nt`, or optional `<file>~desc.nt` sidecars), so File information is navigable from OCFL alone without consulting the event log. The bytes themselves remain content-addressed (multihash); `dao:Bitstream` as a separate class is still not needed. See §3.6 | — / deviation logged 2026-05-12 / amended 2026-05-15 |
| 21 | Storage architecture | OCFL on filesystem as source of truth for everything: bitstreams, RDF metadata, event-log artifacts. SQLite as rebuildable read-side cache. No separate event-store technology. Write path: validate command → write event to OCFL → fsync → update SQLite. Recovery: scan OCFL on startup for events not yet in SQLite, replay | — |
| 22 | Right-sized for DaSCH scale | Architecture deliberately right-sized for 50-100 projects/year. Single-machine, two storage things (filesystem + SQLite file). No infrastructure to scale or operate. Replaceable with heavier infrastructure later if scale demands change, without changing OCFL or events. **Amended 2026-05-15:** baseline re-calibrated from "~tens of events/week" to **~200K events/week steady-state, deposit-burst-driven, with single Depositions reaching ~500K events at the extreme** (per decision 31). Still right-sized for single-machine, but the "trivial polling cadence" claim has been removed — push via SSE has replaced polling (decision 31). **Further re-baselined 2026-05-15 (sizing pass):** the earlier amendment misread deposit volume as a weekly figure. Correct values are **~100 deposits/year (one-shot pattern; the 50-project VRE backlog migrates into the same 100/year flow over ~2 years; iterative-via-refunding counted as new projects), ~200K events per deposit, 20M events/year ingest steady-state, ~50 TB/year storage.** Fixity dominates from year 5 onward under decision 40 (per-Object cadence): year-10 ~120M events/year, year-20 ~220M events/year, cumulative ~2.4 B events at year 20, cumulative storage ~1 PB (**PB-scale at the 20-year horizon**). Single-machine architecture still holds; cold-replay-from-genesis at year 20 is the first number that genuinely presses the ship-and-measure principle (§9.5) toward the deferred bulk-replay optimisation. SQLite reference in the original decision text superseded by decision 38 (Redb). | — / amended 2026-05-15 / further amended 2026-05-15 |
| 23 | Event log granularity | **Amended 2026-05-15:** the original weekly time-bucket granularity (`events/{yyyy-Www}/`) has been replaced by **per-aggregate OCFL Objects** (`depositions/{uuid}/` and `preservation-actions/{uuid}/`), each bundling its aggregate's events together with the aggregate's descriptive metadata. A complementary append-only chronological event-index (`cache/event-index/{yyyy-mm}.ndjson`) provides reverse-lookup for SSE replay. Reason for the change: at re-baselined scale (~200K events/week with deposit bursts up to ~500K), weekly time-buckets became unbounded and filesystem-painful; per-aggregate Objects are causally meaningful (decision 26) and naturally bounded by the aggregate. See §3.7. | — / amended 2026-05-15 |
| 24 | Events carry full snapshots | Even though OCFL stores per-version IE/Representation state, events carry full metadata snapshots (not just references). Redundancy is intentional: preserves replay-from-events-alone capability for disaster recovery | — |
| 25 | Fixity event vocabulary | One `FixityChecked` event with `outcome` ∈ {`pass`, `fail`, `missing`}. Failures and missings trigger preservation-action workflows (workflows are policy, parked separately). **Refined 2026-05-15 by decision 40**: granularity per OCFL Object (not per File); variant payload by outcome (pass = ~250 byte attestation; fail/missing = ~2-5 KB forensic detail); cadence continuous-when-idle + ≥1 sweep/Object/year; ISO 16363 §4.4.2 / §5.1.2 alignment. | — / refined 2026-05-15 |
| 26 | `dao:PreservationAction` as a first-class concept | DaSCH-internal preservation actions (format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction) are modeled as `dao:PreservationAction`, **separate from `dao:Deposition`**. Rationale: different actor (Archive-as-system-agent vs. Producer); different authorization regime (internal preservation policy vs. `DepositAgreement`); CoreTrustSeal audit-trail separation of producer-induced vs. archive-induced changes (CTS R09/R12); cleaner event vocabulary (`PreservationActionExecuted` parallels `DepositionAccepted`). Aligned with `PREMIS DD §1.4` (Event entity); `OAIS §4.1.3 / §5.1` (Preservation Planning) | 2026-05-11 |
| 27 | OAIS + PREMIS alignment as default | Every DAO design choice is aligned with OAIS (CCSDS 650.0-M-3, Dec 2024) and PREMIS 3.0, **or** carries a documented deviation with reason in the decision log or an ADR. Standards extracts checked in under [`standards/`](./standards/) with citation conventions (`OAIS §<n.n.n>`, `PREMIS DD §<n.n>`). The alignment is **conceptual**: DAO uses its own URIs in the `dao:` namespace and may diverge in property names and structure where DaSCH-specific needs justify it, but the underlying concepts must be traceable back to OAIS/PREMIS or explicitly justified | 2026-05-11 |
| 28 | Tombstoning vs. redaction | Two distinct events. `Tombstoned` = **logical retraction** (bytes/metadata preserved in OCFL; read-side hides the Version; ARK returns tombstone landing page). `Redacted` = **surgical content-level erasure** producing a new redacted Version of the IE/Representation; the pre-redaction Version is `Tombstoned` and its OCFL bytes are over-written/zeroed — the **single sanctioned exception to OCFL immutability**. `Redacted` records who authorized the redaction and under what legal basis (e.g. GDPR Art. 17), not what was removed. Outright deletion is **not** a DAO event — it is a board-level exception in a separate governance log. Aligned with `PREMIS DD §1.4` Event entity (custom `Redaction` subtype) and `OAIS §3.3.5` (deactivation permitted; deletion not in normal operation). See §5.1 and §3.7 | 2026-05-12 |
| 29 | Domain framing | The whole `dsp-repository` codebase implements **one domain**: Trusted Repository (OAIS-based). Decomposes into subdomains: Ingest, Archival Storage, Preservation Planning, Data Management, Administration, Access (all OAIS functional entities); Identification (DaSCH-specific, long-term citation via ARK); Producer-side preparation (DaSCH-specific). VRE is external to the domain (a Producer in OAIS terms). See `CONTEXT-MAP.md` → Domain | 2026-05-15 |
| 30 | Three-tier preservation chain role vocabulary | The two-tier "Archival Master / Service Master" framing is **retired**. Replaced by a three-tier role taxonomy distinguished by **purpose in the preservation chain**, not by format, location, or source provenance: **Preservation File** (long-term bit-level preservation; lives inside `dao:Representation`; owned by Archive context); **Service File** (mezzanine derivation under a derivation rule; owned by Access Area context); **Access File** (end-user delivery payload generated on demand; owned by the Access Area subdomain that serves the request). Originated in SIPI's IIIF Server vocabulary; promoted to cross-context Published Language. See `CONTEXT.md` → Preservation chain roles | 2026-05-15 |
| 31 | Archive deployment topology and public interfaces | The Archive bounded context is one logical unit deployed as **two services**: an **Ingest Area** (producer-facing async upload + SHACL/`DepositAgreement` validation gate; emits `DepositionAccepted` only on validation success; OAIS *Ingest* entity) and the rest of the Archive (event log, OCFL, public APIs; OAIS *Archival Storage* entity, with future Preservation Planning / Data Management / Administration entities). Both speak DAO directly; no anti-corruption layer between them. The Archive exposes **three public APIs**: Commands (HTTP), Events SSE (`text/event-stream`, resumable via `Last-Event-ID`, **full firehose** with no server-side filtering — subscribers filter client-side), Binary retrieval (`GET /bitstreams/{multihash}`, HTTP 206 Range supported). **OCFL is exclusive to the Archive boundary**; no other context reaches into the OCFL store. Deposition size is **producer-set** (Path A); realistic upper bound ~500K events for an extreme single submission; per-Deposition OCFL Objects are the proposed granularity. See §1a, §9.3 | 2026-05-15 |
| 32 | Access Area as federated subscribers; cold-replay strategy | Access Area is **one bounded context with N independent subscriber services** (one per DIP-shape subdomain: IIIF, HTML/Web Discovery, Custom Presentation, Asset/Download, SPARQL). Each subscriber maintains its own SSE cursor against Archive, its own storage tuned to its consumer's pattern, its own derivation logic. **Cold-replay strategy by use case**: (α) subscriber-side snapshots for routine restarts; (γ) subscriber-to-subscriber replication for spinning up duplicates of an existing subscriber kind; (δ) full SSE replay from genesis for the rare deploy of a brand-new subscriber kind. Archive serves historical events via the same SSE endpoint as live tail; a bulk-replay optimisation is **deferred** until measurement shows it is needed (see §9.5: ship and measure before optimising) | 2026-05-15 |
| 33 | OCFL granularity — Representation as Archival Group; no `dao:File` class | Each `dao:Representation` is stored as one OCFL Object (Fedora 6's "Archival Group" pattern); Files live as content within (**not** as separate OCFL Objects). Avoids the object-proliferation pain documented in Fedora 6 + Hyrax migrations (10+ OCFL Objects per intellectual work; the Samvera community's pivot from ActiveFedora-on-LDP to Valkyrie in Hyrax 5 is the strongest signal that the "every resource = its own Object" pattern was wrong). `dao:File` is **not** a write-side DAO class. Per-File information is rendered as blank-node-structured properties on the Representation's RDF via `dao:hasFile`, addressed by `dao:filename` within the parent Representation Version's context. For event references to a specific File: use the (Representation Version IRI, filename) tuple. Default on-disk layout: per-File information inline in the Representation OCFL Object's `rep.nt`; Fedora-style `<file>~desc.nt` sidecar pattern is an optional convention deferred until operational need surfaces. Amends decision 20 (deviation narrowed: `dao:File` *not* reintroduced as a class; storage rendering clarified). | 2026-05-15 |
| 34 | RDF serialisation at rest — N-Triples | DAO RDF metadata stored as **N-Triples** inside OCFL Objects (matching Fedora 6's canonical at-rest format). Turtle / JSON-LD accepted on the Commands and Events APIs for content negotiation; normalised to N-Triples on write at the API boundary. Rationale: line-oriented, prefix-free, deterministic byte representation. Critical for OCFL fixity hashes over RDF content; identical triples produce identical bytes regardless of who serialised them. Diff-friendly across OCFL versions; migration-safe. Turtle's prefix table is global state that can change serialisation without changing semantics; bad inside an immutable OCFL version. | 2026-05-15 |
| 35 | Explicit non-adoption of LDP and PCDM as modelling primitives | Following the Samvera community's documented pivot from ActiveFedora-on-LDP to Valkyrie (Hyrax 5, 2024), DAO **does not adopt** LDP containers (DirectContainer, IndirectContainer, BasicContainer) or PCDM (Collection / Object / File / Hash) as modelling primitives. The Hyrax/Samvera experience documents real pain: LDP's one-parent containment constraint, tombstoned URIs blocking ID reuse, chatty per-resource API, and PCDM's conflation of intellectual structure with storage structure. OAIS + PREMIS + DAO already cover containment (IE → Representation → Files-within-Rep) and provenance (PreservationAction) with cleaner semantics. The single LDP idea that survives at the storage layer is the Fedora 6 sidecar pair pattern (`<file>` + `<file>~fcr-desc.nt` within one OCFL Object) — available as an optional on-disk convention but not required. The DAO model stays storage-agnostic, in line with Valkyrie's lesson. | 2026-05-15 |
| 36 | Two-substrate storage architecture: append-only event log + OCFL entity storage | The Archive uses **two physical storage substrates**, both ZFS-backed. **Event log substrate** (source of truth for events): append-only NDJSON-of-JSON-LD log, organised as sealed segment files; segments seal on a hybrid trigger (monthly OR ~100 MB, whichever first); active (unsealed) segments live in `event-log-active/` outside OCFL; each sealed segment becomes one OCFL Object in a dedicated **event-log-storage-root** (OCFL storage root #1, proper OCFL with namaste + layout + extensions). **Entity storage substrate** (source of truth for entity state and Preservation File bytes): a separate **entity-storage-root** (OCFL storage root #2) containing one OCFL Object per DAO state-aggregate entity (`urn:dsp:ie:{uuid}`, `urn:dsp:rep:{uuid}`, `urn:dsp:project:{uuid}`, `urn:dsp:agent:{uuid}`, `urn:dsp:agreement:{uuid}`) and one OCFL Object per commit aggregate (`urn:dsp:deposition:{uuid}`, `urn:dsp:preservation-action:{uuid}`, holding audit records). Each entity Object versions on curatorially-meaningful state changes; Preservation File bytes live inside the relevant Representation Object's content area. The two substrates are co-written atomically on commit (write-ahead event, then OCFL state, with reconciliation on crash recovery). Replaces the earlier "single OCFL store with commit-aggregate Objects bundling events + bytes" proposal (which had no prior art and was flagged by research as misaligned with both OCFL conventions and CQRS-ES practice). Grounded in: Fedora 6's OCFL-as-source-of-truth pattern; Datomic / EventStoreDB / Kafka separation of event log from large blobs; standard CQRS-ES stream-per-aggregate addressing. **Amends decisions 21, 31, 33** (storage architecture revised; per-aggregate granularity changes; OCFL no longer bundles events with bytes). | 2026-05-15 |
| 37 | Event-log on-disk format | Active log: **NDJSON of JSON-LD events**, one event per line. Each line is a self-contained JSON-LD document with explicit fields: `@id`, `@type`, `global_offset`, `stream_id`, `stream_version`, `timestamp`, `event_schema_version`, plus event-type-specific payload. Each line ends with a `crc32` field over the line's content for per-event corruption detection. **Per-segment fixity**: SHA-256 manifest in standard Unix `[hash]  [filename]` format (compatible with `sha256sum -c`), file extension matches hash type (`.sha256`). **Durability**: fsync per event during deposit bursts (latency is async by design per decision 31); batched fsync (every N events or M ms) acceptable outside bursts; atomic appends with truncation to last valid CRC32 on crash recovery. **Schema evolution**: `event_schema_version` field per event; readers maintain `(event_type, schema_version) → reader_function` mapping for forward compatibility. **Narrows decision 34**: N-Triples is the at-rest format for **entity state RDF in OCFL Objects**; NDJSON-of-JSON-LD is the at-rest format for **events in the event log** (and the wire format on the SSE feed). Both serialise the same RDF data model; choice is operational. | 2026-05-15 |
| 38 | Cache DB technology — Redb | The rebuildable cache (`cache/`) uses **Redb** (pure-Rust embedded B-tree KV store with ACID + MVCC) for three indexing workloads: (a) read-side projection cache of entity state for queries; (b) event-log index (`(stream_id, stream_version) → segment_id + byte_offset`, `global_offset → segment_id + byte_offset`); (c) bytes index (`multihash → containing-OCFL-Object + content-path`). Rationale: **pure Rust** (no C FFI dependency); **B-tree** fits our workload (~10M events/year peak; point-lookups dominate; range scans bounded; bursty but not high-throughput writes; predictable tail latency); **ACID + MVCC** for atomic projection updates during multi-event commits; **typed tables** catch a class of bugs at compile time; **no SQL needed** (query patterns are all known in advance). Schemas designed technology-neutrally for a future engine swap if needed (everything in `cache/` is fully rebuildable from the two substrates). Considered and rejected: **SQLite** (C FFI; pure-Rust preference outweighs SQL convenience at our query-pattern complexity); **TursoDB / Limbo** (too young for a multi-decade preservation system; revisit if mature in 5 years); **Fjall** (LSM over-engineered for our write rate; B-tree more predictable); **RocksDB** (heavy C++ dependency). | 2026-05-15 |
| 39 | Fixity baseline; non-repudiation deferred | **Adopted baseline**: SHA-256 sidecar fixity files in standard Unix `[hash]  [filename]` format (compatible with `sha256sum -c`); file extension matches hash algorithm (`.sha256`). Per-event CRC32 inside event-log segment NDJSON lines for finer-grained corruption detection. **Explicit threat model**: the sidecar fixity baseline defends against accidental disk corruption, bit rot, and unauthorised viewers without write access. It does **not** defend against an attacker with write access to the underlying storage (internal admin, compromised process), who can modify both the data file and its sidecar. **Non-repudiation deferred** to the CoreTrustSeal evidence pass (Q8 in §9.4) with two concrete approaches identified: **(a) HMAC + HSM** — write-time authentication using a key held in a non-extractable Hardware Security Module (commercial HSM, or YubiKey / Nitrokey / GnuPG smartcard for cost-conscious setups); **(b) Merkle Tree log structure with external root publication** — events form an append-only Merkle tree; root hashes published externally on a regular cadence (RFC 3161 TSAs, OpenTimestamps, peer-institution mirrors); tampering invalidates the tree from the tampered event onward, with externally-published roots providing cryptographic proof of what the tree looked like at publication time. Both approaches compose. Implementation deferred per the **ship and measure** principle (§9.5); the architecture admits both additions as additive layers. | 2026-05-15 |
| 40 | Fixity policy: per-Object granularity, continuous cadence, variant payload | **Granularity**: one `FixityChecked` event per OCFL Object (= per `dao:Representation`) validate, not per File. Matches `ocfl validate` as the operational unit; aligns with PREMIS Event semantics (Events bind to PREMIS Objects, and `dao:Representation` is the Object-level entity per decision 33). **Cadence**: continuous fixity sweep running in idle I/O windows, with a hard commitment of **at least one sweep per OCFL Object per year**. Aligns with `ISO 16363 §4.4.1.2` (*"The repository shall actively monitor the integrity of AIPs."*) and `ISO 16363 §5.1.1.3` (*"The repository shall have effective mechanisms to detect bit corruption or loss."*). Fail/missing reporting workflow aligned with `ISO 16363 §5.1.1.3.1` (*"The repository shall record and report to its administration all incidents of data corruption or loss..."*). The continuous-when-idle + ≥1/year cadence exceeds these requirements' documented-policy-consistently-executed bar. Produces CoreTrustSeal R10 / R11 evidence. **Variant payload by outcome**: `pass` = minimal attestation (~250 bytes — Object IRI, version, timestamp, outcome); `fail`/`missing` = per-File forensic detail (~2-5 KB — filename, expected hash, recomputed hash, per-File result, checker version). Both shapes belong in the WORM event log: routine pass-records are audit-grade attestation that monitoring happened on that date; fail/missing records are forensic evidence (and satisfy §5.1.1.3.1 reporting). **Operational consequence**: at year-20 scale (200M Reps) fixity events total ~52 GB/year (vs. ~1 TB/year if uniform forensic-detail were carried). Operational metrics (sweep progress, throughput, queue depth) live in Grafana per decision 41 — operational, not archival. **Refines decision 25** (which only specified the outcome enum); aligned with decision 39's per-event CRC32 baseline. | 2026-05-15 |
| 41 | Grafana `/metrics` endpoint as operational surface; not an Archive public API | The Archive exposes a `/metrics` endpoint scraped by Grafana for operational observability — event rate, fixity sweep progress, queue depth, OCFL Object counts, disk usage, write latency, SSE subscriber lag. This is **operational telemetry, not archival evidence**: Grafana retention is finite, the endpoint can be replaced or reset without affecting the preservation commitment, and no Producer or Subscriber should depend on it as a contract. The three public APIs (Commands, Events SSE, Binary retrieval — decision 31) remain the bounded-context boundary; `/metrics` is a fourth surface internal to operations. **Audit-grade evidence of monitoring activity lives in the event log** (e.g., routine `FixityChecked` pass-events per decision 40), not in Grafana. The two layers are deliberately separate: the event log is durable and tied to the preservation commitment; Grafana is operationally convenient and disposable. **Clarifies decision 31** (which named three public APIs without addressing operational telemetry). | 2026-05-15 |
| 42 | Certification pyramid (CTS → nestor → ISO 16363); preserve-optionality principle | DaSCH targets the trustworthy-repository certification pyramid in difficulty order: **tier 1 CoreTrustSeal** (community self-assessment + peer review; entry level; first target), **tier 2 nestor Seal** (German Kriterienkatalog v2 2008; documented self-assessment + nestor peer review; middle), **tier 3 ISO 16363** (formal external audit by ISO-16919-conforming body; top tier). The substantive technical requirements of tier 3 dominate the others; **design choices are made against tier 3 to avoid foreclosing future certification.** Working principle: where a tier-2 or tier-3 requirement implies a technical capability we cannot implement immediately, the architecture remains *open* to adding that capability as an additive layer — implementation is deferred, optionality is not. **Concrete commitments under this principle**: (i) geographic disaster-recovery backup is architecturally committed but deployment-deferred — both OCFL roots and the active-log directory are designed to replicate via filesystem-level mirroring (`zfs send` to a distant ZFS pool, or `rsync` with content-addressed verification); Redb is rebuildable on the remote and need not be replicated. Required by `nestor §14` (institutional-main-building-disaster must not destroy objects) and implied by `ISO 16363 §5.1.2` + `§5.2.1` (number/coordination/location of copies + risk analysis of insufficient distancing). (ii) HMAC + HSM event authentication and Merkle-tree log with external root publication remain architecturally additive (decision 39 names them). (iii) Digital signatures on DIPs (`nestor §7.3`) are an Access Area concern; subdomain-level signing is additive. (iv) Significant Properties (`nestor §9.2`) can be added later as Rep properties or a new event type; no current architectural lever blocks them. (v) On-demand BagIt / RO-Crate AIP serialisation can mitigate the constituted-AIP deviation (decision 27 / §6.2) if auditors push back — implementation only if asked. **Items that are organizational, not architectural** — Designated Community spec, Producer-Archive agreement detail, financial sustainability evidence, succession planning, security risk analysis — fall outside DAO's scope but are flagged here so the architecture-vs-organisation seam is explicit. | 2026-05-16 |
| 43 | DaSCH preservation-level commitment: uniform A + C + D across all projects | **DaSCH's mandate commits the Archive to CTS Levels A + C + D uniformly** (per the *CTS Curation & Preservation Levels Position Paper v3.0*, 2024 — **D** Deposit Compliance + **C** Initial Curation + **A** Active Preservation; levels are cumulative). The commitment applies to **every project, every Deposition, every preserved IE / Representation / File**. No per-DepositAgreement, per-Deposition, or per-IE variation. **Consequences for DAO**: (i) `dao:DepositAgreement` (decision 18) does **NOT** carry a preservation-level property — the level is institutional invariant, not contractual variable. (ii) Event payloads do **NOT** carry `premis:preservationLevel` overrides — there is nothing to override. (iii) No `CurationLevelChanged` event in the vocabulary — there is no level transition to record. (iv) The Position Paper's Z / D / C / A taxonomy is documented as a reference vocabulary in `standards/` but not modelled as a tunable in DAO. **Rationale**: institutional simplicity (a single uniform commitment is easier to audit, operate, and communicate than per-content variation); fits DaSCH's funding model (SNSF-funded mandate applies uniformly to projects in scope); aligns with DaSCH's value proposition (expert curation for all preserved content). Auditors get a single, simple commitment claim that maps directly to CTS R08 / R09 / R10 ("curation levels defined during appraisal" / "responsibility for preservation defined" / "variations for different curation-levels" — DaSCH's response: "no variations; uniform A+C+D"). **Resolves Q15.** **If future DaSCH offerings introduce true tiered service** (e.g., a self-service Z-level publication-only offering for non-SNSF projects), this decision must be revisited; until then, the architecture is deliberately not designed for per-content variation. | 2026-05-16 |
| — | DAO scope (folded in) | DAO governs the Archive (`dao:Representation`s containing Preservation Files), not the Access Area (Service Files and Service Projections are derived) or Access Area subdomain presentation views (DPE/CPE/IIIF, etc., are presentation, not preservation) | folded in (terminology updated 2026-05-15) |
| — | Service-tier versioning (folded in) | Service Files / Service Projections are derived projections, not versioned entities; regenerable by replay from Representation + derivation rule | folded in (terminology updated 2026-05-15) |
| — | Representation = preservation-grade bundle (folded in) | The `dao:Representation` class refers to the preservation-grade bundle (one or more Preservation Files plus Representation-level metadata); Service-tier projections are derivatives outside DAO's identity model | folded in (terminology updated 2026-05-15) |

### 8.1 Evidence index (per certification tier)

Maps decisions that produce **audit-grade evidence** to citations across the three certification tiers framed in decision 42:

- **CTS** — CoreTrustSeal **Requirements 2026-2028 v01.00** (R01-R16). The catalog is refreshed every 3 years; this is the current version (the prior 2023-2025 catalog used a different ordering of R-numbers).
- **nestor** — Kriterienkatalog vertrauenswürdige digitale Langzeitarchive v2 (2008).
- **ISO 16363** — CCSDS 652.0-M-2 (Dec 2024).

Decisions not listed are internal-design choices, vocabulary cleanups, or naming refinements that do not directly produce certification evidence — they may still appear in audits as supporting context.

| # | Topic | CTS | nestor | ISO 16363 |
|---|---|---|---|---|
| 5 | Event sourcing / WORM event log | R07 | §7.2 | §4.4.2 |
| 13 | ARK strategy (persistent identifiers) | R12 | §12.1 | §4.2.4 |
| 14 | Commands vs Events (CQRS-ES validation discipline) | R07, R11 | §6.1, §7.1 | §4.1.1, §4.4.2 |
| 17 | Self-built Archive Component | R05, R15 | §4, §13 | §3.2, §5.1 |
| 18 | `dao:DepositAgreement` (Producer-Archive contract); preservation-level NOT a property (uniform A+C+D per decision 43) | R02, R08 | §3.1 | §3.5.1, §3.5.1.1 |
| 20 | Bitstream storage / content addressing (PREMIS deviation, documented) | R14 | §6.2 | §4.4.1 |
| 21 | OCFL on filesystem as source of truth | R14, R15 | §6.2, §13.1 | §4.4.1, §5.1.1.5 |
| 24 | Events carry full snapshots (replay capability) | R07 | §7.2 | §4.4.2 |
| 26 | `dao:PreservationAction` first-class | R09, R11 | §8, §10.4 | §4.3, §4.4.2.1 |
| 27 | OAIS + PREMIS alignment as default | R10 (technical standards compliance) / cross-cutting | (cross-cutting; nestor references OAIS throughout) | (built on OAIS) |
| 28 | Tombstoned vs Redacted (logical retraction + GDPR-driven content erasure) | R07, R09 (tombstone records + deletion impact on PIDs) | §7.2 | §4.5, OAIS §3.3.5 |
| 31 | Archive deployment + 3 public APIs (Commands, SSE, Binary retrieval) | R11, R14 | §6.1, §6.3, §13.1 | §4.1, §4.6 |
| 32 | Access Area as N federated subscribers | R11, R13 | §11 | §4.6 |
| 33 | Representation as OCFL Object; no `dao:File` class | R14 | §6.2, §10.1 | §4.2, §4.4.1 |
| 34 | N-Triples at rest (readability without DaSCH software) | R13, R14 | §10.3 | §4.4.1 |
| 36 | Two-substrate storage (event log + entity state) | R07, R14 | §6.2, §13 | §4.4.1, §4.4.1.2 (fixity-separation principle), §5.1 |
| 37 | Event-log NDJSON-of-JSON-LD + per-event CRC32 | R07, R14 | §6.2 | §5.1.1.3 |
| 39 | Fixity baseline (SHA-256 sidecars + CRC32) + deferred non-repudiation | R07, R14 | §6.2, §7.2 | §5.1.1.3, §5.1.1.3.1 |
| 40 | Fixity policy (per-Object granularity, continuous, variant payload) | R09, R14 | §6, §6.2 | §4.4.1.2, §5.1.1.3, §5.1.1.3.1 |
| 42 | Certification pyramid; preserve-optionality; geographic-redundancy commitment | (meta) + R03 + R14 (multiple-copy strategy) + R16 | (meta) + §14 (geographic redundancy) | (meta) + §5.1.2, §5.2.1 |
| 43 | DaSCH uniform A+C+D preservation commitment (no per-project / per-Deposition / per-IE variation) | R08, R09, R10 + Position Paper v3.0 | §1.2, §6, §7, §8 | §3.3.1, §4.3, §4.4.1.1 |

**Reading guide.**

- A `nestor §6` citation without sub-section indicates evidence applies across all three sub-points (§6.1 Ingest, §6.2 Storage, §6.3 Access integrity). Where the design's load-bearing impact is specific to one phase, the sub-section is given.
- `ISO 16363 §4.4.1.2` (active integrity monitoring) is the most-cited single section — most decisions in some way support it. `§5.1.1.3` (corruption-detection mechanisms) is its peer at the infrastructure layer.
- `(meta)` indicates a framing decision that subsumes others rather than supplying a single-tier-specific evidence claim.
- A decision can appear in the audit for an evidence claim it does *not* support directly — e.g., decision 17 (self-built) supports CTS R5 (org infrastructure) not by being a technical artefact but by being an organisational choice that *enables* DaSCH staff to demonstrate competence. These second-order claims are not enumerated here.

**Coverage check — what's NOT covered by current decisions** and falls outside DAO's architectural scope, but will be required at audit time as organisational / operational evidence:

| Audit-time requirement | Tier citations | Owner |
|---|---|---|
| Mission statement + Designated Community spec | CTS R01; nestor §1, §1.3; ISO 16363 §3.1.1, §3.3.1 | DaSCH leadership |
| Succession plan / continuity-of-access / business continuity | CTS R03; nestor §4.6; ISO 16363 §3.1.2.1 | DaSCH leadership + SNSF |
| Legal & ethical compliance (incl. data protection) | CTS R04; nestor §3; ISO 16363 §3.5 | DaSCH legal/compliance |
| Governance & financial sustainability evidence | CTS R05; nestor §4.1, §4.3; ISO 16363 §3.1, §3.4 | DaSCH co-direction |
| Staff competence + professional development | CTS R06; nestor §4.2; ISO 16363 §3.2.1.3 | DaSCH leadership |
| Quality assurance during curation | CTS R10; nestor §5; ISO 16363 §4.2 | DaSCH RDU |
| Periodic internal audit | (covered indirectly via R05); nestor §5; ISO 16363 §3.3.6 | DaSCH governance |
| Format-risk monitoring / technology watch (informs Q14 format-migration policy) | CTS R09; nestor §10.4; ISO 16363 §5.1.1.1 | DaSCH engineering |
| Reuse-side Designated-Community engagement | CTS R13; nestor §11.2; ISO 16363 §4.5 | DaSCH RDU + Access Area subdomain owners |
| Security risk analysis (incl. geographic-disaster planning) | CTS R16; nestor §14; ISO 16363 §5.2.1 | DaSCH infrastructure + governance |
| Phase-2 geographic-replication operational plan | CTS R03 + R14 + R16; nestor §14; ISO 16363 §5.1.2 / §5.2.1 | DaSCH infrastructure (see §9.6 fourth priority) |
| Producer-Archive agreement detail (formal contract language) | CTS R02, R08; nestor §3.1; ISO 16363 §3.5.1.1 | DaSCH + Producer (decision 18 has the class; the contract text is org work) |
| Preservation-level commitment statement (uniform A+C+D mandate) | CTS R08, R09, R10 + Position Paper v3.0; nestor §1.2; ISO 16363 §3.3.1 | DaSCH leadership (decision 43 records the commitment architecturally; the *public mandate statement* itself is organisational — needs to appear in CTS application alongside mission statement) |

This index is the **starting point for the per-decision evidence map** in §9.4 priority 11. Further refinement (e.g., explicit cross-mapping to nestor's organisational criteria where the design itself doesn't supply technical evidence but the org-side documentation will, enumeration of second-order evidence claims, cross-reading with the CTS Extended Guidance document) is deferred until certification preparation begins.

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

### 9.2 Session log

#### Session of 2026-05-11 / 2026-05-12 / 2026-05-13

- Decision 26 — `dao:PreservationAction` is a first-class concept, distinct from `dao:Deposition`.
- Decision 27 — OAIS + PREMIS alignment as default; deviations require documented reason.
- Decision 28 — `Tombstoned` (logical retraction) vs. `Redacted` (surgical content erasure) as two events; outright deletion explicitly *not* a DAO event.
- Decision 6 amended — DAO is conceptually built on PREMIS; the `dao:` namespace adds DaSCH-specific structure rather than replacing PREMIS.
- Decision 20 amended — PREMIS's `Representation → File → Bitstream` hierarchy is *deliberately collapsed* in DAO; deviation documented in §3.6.
- Standards extracts added under `standards/` (OAIS v3, PREMIS DD v3.0, PREMIS OWL Guidelines).
- `CONTEXT.md` rewritten — current entity glossary, identifier vocabulary, OAIS-vocabulary disambiguation.
- §7 → §8 renumbering fixed (the prior duplicate `## 7` is gone).
- Q9 in parked-questions list marked resolved; Q3 and Q14 marked partially resolved.

#### Session of 2026-05-14 / 2026-05-15

Major architectural session. Substantial doc restructuring; four research-driven pivots; eleven new or amended decisions.

**Domain framing and bounded contexts.**
- Decision 29 — domain framing: the whole `dsp-repository` codebase implements **one domain (Trusted Repository, OAIS-based)**; subdomains map to OAIS functional entities + DaSCH-specific concerns (Identification, Producer-side preparation); VRE is external.
- Created `CONTEXT-MAP.md` at the repo root capturing the OAIS-anchored bounded-context split.
- Renamed `modules/archival-format/` → `modules/archive/`; retitled `CONTEXT.md` to "Archive" (the bounded-context name rather than the artifact name).
- Decision 30 — three-tier preservation chain role vocabulary: **Preservation File / Service File / Access File** replaces the retired "Archival Master / Service Master" framing. Originated in SIPI's IIIF Server vocabulary; promoted to cross-context Published Language. Owned by Archive / Access Area / Access Area subdomain respectively.

**Archive deployment topology and APIs.**
- Decision 31 — Archive deployed as two services within one bounded context: **Ingest Area** (producer-facing async upload + DAO/SHACL validation gate) + the rest of the Archive (event log + APIs). Three public APIs: Commands (HTTP), Events SSE (full firehose, no server-side filtering), Binary retrieval (`GET /bitstreams/{multihash}`). OCFL exclusive to the Archive boundary. Path A (producer-set Deposition size, no ceiling; ~500K events worst case per Deposition).

**Access Area architecture.**
- Decision 32 — Access Area is **one bounded context with N federated subscriber services** (one per DIP-shape subdomain: IIIF, HTML/Web Discovery, Custom Presentation, Asset/Download, SPARQL). Each subscriber owns its SSE cursor, storage, derivation logic. Cold-replay strategy by use case (α subscriber-side snapshots / γ peer replication / δ full replay).

**Storage architecture — first pivot from Fedora 6 + LDP research.**
- Decision 33 — Representation as Archival Group (Fedora 6 pattern). **`dao:File` is NOT a write-side class**; per-File information rendered as blank-node-structured properties on the Representation's RDF via `dao:hasFile`. Avoids Fedora 6 + Hyrax migration's documented object-proliferation pain.
- Decision 34 — **N-Triples at rest** for entity-state RDF in OCFL (Fedora 6's canonical format). Turtle / JSON-LD accepted on APIs for content negotiation; normalised to N-Triples on write.
- Decision 35 — Explicit **non-adoption of LDP and PCDM** as modelling primitives. Samvera community's pivot from ActiveFedora-on-LDP to Valkyrie (Hyrax 5, 2024) is the signal. Single LDP idea retained at storage layer is the sidecar pair pattern (`<file>` + `<file>~desc.nt`).

**Storage architecture — second pivot from "OCFL-as-event-store" research.**
- Decision 36 — **Two-substrate storage architecture**: append-only event log (NDJSON-of-JSON-LD, sealed segments wrapped as proper OCFL Objects in a dedicated `event-log-storage-root`) **separate from** OCFL entity storage (`entity-storage-root` with one OCFL Object per state-aggregate + per commit-aggregate entity; Preservation File bytes inside Representation Objects). Replaces the earlier "single OCFL store bundling events + bytes" proposal (which had no prior art and was misaligned with both OCFL conventions and CQRS-ES practice; the prior framing also conflated commit-aggregates with domain-aggregates — Vernon / Young's stream-per-aggregate convention treats long-lived domain entities as aggregate roots, not commits). **Amends decisions 21, 31, 33** (storage architecture revised; per-aggregate granularity changes; OCFL no longer bundles events with bytes).
- Decision 37 — Event-log on-disk format: NDJSON-of-JSON-LD lines with `crc32` per event; per-segment SHA-256 manifest in standard Unix `[hash]  [filename]` format. Hybrid rollover trigger (monthly OR ~100 MB, whichever first). **Narrows decision 34**: N-Triples for entity-state RDF in OCFL; NDJSON-of-JSON-LD for events in the event log and on the SSE wire.
- Decision 38 — Cache DB technology: **Redb** (pure-Rust embedded B-tree KV store with ACID + MVCC). Considered and rejected: SQLite (C FFI), TursoDB / Limbo (too young for multi-decade preservation), Fjall (LSM over-engineered for our write rate), RocksDB (heavy C++ dep).
- Decision 39 — Fixity baseline: SHA-256 sidecar in standard Unix format + per-event CRC32. **Explicit threat model**: defends against accidental corruption, *not* against attackers with write access. **Non-repudiation deferred**: HMAC + HSM (for write-time authentication) and Merkle Tree log structure with external root publication (for ongoing tamper-evidence, Certificate-Transparency-style) identified as the two concrete approaches. Both compose; implementation deferred per ship-and-measure.

**Internal identifiers.**
- Decision 12 amended — internal persistent-identity IRIs use the **URN scheme** `urn:dsp:{type}:{uuid}` (not dereferenceable; never leave the system). Read-side URLs (HTTPS) remain dereferenceable for browser/client access. ARKs are the only long-term-stable public identifiers. Fedora 6 precedent (`info:fedora/...`).

**Process and methodology.**
- Added **"Ship and measure before optimising"** as the fourth process note in §9.5.
- New artifact: `UBIQUITOUS_LANGUAGE.md` at the repo root (consolidated cross-context glossary; produced by the `my-eng:ubiquitous-language` skill).
- `CLAUDE.md` updated with a "Domain Model" section pointing at CONTEXT-MAP / per-context CONTEXT.md / UBIQUITOUS_LANGUAGE / per-context design-narrative docs.
- Two research agents dispatched:
  - **Fedora 6 + LDP research** — informed decisions 33 / 34 / 35.
  - **OCFL-as-event-store prior art research** (Datomic / EventStoreDB / Kafka tiered storage / Pulsar / SQLite-as-event-store; Fedora 6 OCFL adoption; CQRS-ES patterns from Vernon / Young) — informed decisions 36 / 37 / 38 / 39 and the pivot to two-substrate architecture.

**Q12 access-rights work paused mid-flight.**
- Q12.1 resolved 2026-05-15: enforcement at delivery layer (γ).
- Q12.2 access-rights composition at IE / Rep / File levels — initially proposed first-class `dao:File` class; replaced by blank-node `dao:hasFile` properties on Representation (decision 33); then paused entirely as the File-identity question opened the deeper storage-architecture rewrite. **Resume here** in the next session (see §9.6 below).

#### Session of 2026-05-15 (sizing pass)

Continuation of the 2026-05-14/15 session, narrowly focused on locking the numerical baseline before resuming Q12.2 on a stable foundation. Two new decisions (40, 41); amendments to decisions 22 and 25.

**Numerical baseline re-locked.** The earlier 2026-05-15 amendment of decision 22 mis-read deposit volume as a weekly figure. Corrected values:

- **Project flow**: 100 projects/year as a hard cap, reached over ~2 years from 12-15/year today. The ~50 existing VRE projects migrate into the Archive one-by-one alongside net-new projects; iterative-via-refunding (CAS / 0812 as the worked example) counted as a new project per refunded round, not as iterative-within-project.
- **Deposit pattern**: one-shot; ~100 deposits/year; ~200K events per deposit (≈ 100K IE + 100K Rep + 1 `DepositionAccepted`); ~1.5 Preservation Files per Rep on average; avg Rep ~5 MB; ~500 GB per project.
- **Ingest steady-state**: 20M events/year, ~50 TB/year storage.
- **Fixity dominates from year 5 onward** under decision 40 (per-Object, continuous + ≥1/year): year-10 total events ~120M/year, year-20 ~220M/year, cumulative events at year 20 ~2.4 B, cumulative storage ~1 PB.
- **Cold-replay-from-genesis at year 20**: ~13-67 hours depending on SSE throughput. First number that genuinely presses the ship-and-measure principle (§9.5) toward the deferred bulk-replay optimisation.

**Fixity policy settled (decision 40).** Per-OCFL-Object granularity (not per File) — matches `ocfl validate` as operational unit, aligns with PREMIS Events binding to PREMIS Objects. Cadence: continuous fixity in idle I/O windows + a hard ≥1 sweep/Object/year commitment. ISO 16363 §4.4.1.2 (active integrity monitoring) / §5.1.1.3 (corruption-detection mechanisms) / §5.1.1.3.1 (incident reporting) alignment; CoreTrustSeal R10 / R11 evidence-ready. Variant payload by outcome: pass = ~250 byte attestation; fail/missing = ~2-5 KB forensic detail. Refines decision 25.

**Grafana boundary made explicit (decision 41).** `/metrics` endpoint is an operational fourth surface — not part of the three-public-APIs bounded-context contract from decision 31. Audit-grade evidence of monitoring lives in the event log; operational metrics live in Grafana with finite retention.

**Q14 (preservation-action workflows) partially engaged.** Fixity *cadence* and *granularity* are now settled (decision 40); the response workflow for `fail`/`missing` and the format-migration triggering policy remain parked.

**ISO 16363 missing from `standards/`.** The decision-27 "cite or deviate" pattern is supposed to be grep-able. ISO 16363 (commercial PDF) needs purchase by DaSCH and addition to `standards/` to make the citations real; until then, citations to ISO 16363 are from working memory of the standard rather than from a checked-in extract.

**Doc updates done in this session.** §3.7 scale prose (with the year-1/5/10/20 event-volume table); §3.7.1 Grafana endpoint subsection; §3.8 fixity full rewrite (per-Object granularity, cadence, variant payload, ISO 16363 citations); §5.1 `FixityChecked` vocabulary entry; §7 Q14 status update; decision 22 further-amendment; decision 25 refinement note; new decisions 40 and 41; this §9.2 entry; §9.4 and §9.6 status updates.

**Continuation — §3.6 / §3.7 prose rewrite (same-day, same conversation).** Immediately after the sizing-pass edits above, the §9.6 second-priority "storage prose rewrite" was tackled in the same session:

- **§3.6 Bitstream storage** — para 1 rewritten to describe Preservation File bytes living inside the Rep OCFL Object's content area, with the bytes-index in `cache/archive.redb` providing reverse lookup; HTTPS URI in the event payload example replaced with the URN scheme (decision 12 amendment); event payload reshaped to match the decision-37 NDJSON-JSON-LD structure (`@id` / `@type` / `stream_id` / `stream_version` / `global_offset` / `timestamp` / `event_schema_version` / `crc32`); on-disk RDF serialisation paragraph clarified to document the two-format split (N-Triples for entity-state RDF inside OCFL; NDJSON-JSON-LD for events per decision 37); final sentence updated.
- **§3.7 Storage architecture** — full replacement from the "Architecture: OCFL on filesystem..." line through the closing "what this gets DaSCH" bullets. New directory tree showing `event-log-active/` + `event-log-storage-root/` (sealed segments wrapped as OCFL Objects) + `entity-storage-root/` (one OCFL Object per DAO entity URN, holding entity-state RDF and Preservation File bytes) + `cache/archive.redb`. New prose: two-substrate framing, event-log sealing on hybrid trigger (monthly OR ~100 MB), entity Objects versioning on curatorially-meaningful state changes, Redb's three indexed workloads (read-projection / event-log-index / bytes-index), write path co-writing both substrates with Redb update transactional, recovery from CRC32 truncation / Redb-lag / total-Redb-loss, Redaction exception applied to both substrates, refreshed "what this gets DaSCH" framing including the CQRS-ES-discipline argument.
- **§3.7.1 Binary retrieval API** — gained a `Lookup mechanism` bullet describing the bytes-index path.
- **§3.7.2 Subscriber bootstrap** — gained an "Archive-side snapshots that *do* exist" note disambiguating entity-Object versions (preservation snapshots) from subscriber-side α snapshots (projection snapshots).

**Doc cleanup**: §9.4 item 15 (the "§3.6 / §3.7 narrative prose rewrite") struck through as DONE; §9.6 second priority struck through; remaining outstanding-but-non-blocking work is the ISO 16363 `standards/` extract purchase.

**Next session entry points** — Q12.2 access-rights composition remains the top priority, now on fully stable foundations (numerical baseline + fixity policy + two-substrate storage prose all settled).

#### Session of 2026-05-16 (standards extension)

Two CCSDS December 2024 PDFs added to `standards/` by Ivan and extracted to layout-preserved `.md` via `pdftotext -layout`:

- **ISO 16363** — CCSDS 652.0-M-2 (Dec 2024), *Audit and Certification of Trustworthy Digital Repositories*. Structure: §1 Introduction; §2 Overview of audit/certification criteria; §3 Organizational Infrastructure (governance, staffing, procedural accountability, financial sustainability, contracts); §4 Digital Object Management (ingest, AIP creation, preservation planning, AIP preservation, information management, access management); §5 Infrastructure and Security Risk Management. The substantive standard the Archive must align with.
- **ISO 16919** — CCSDS 652.1-M-3 (Dec 2024), *Requirements for Bodies Providing Audit and Certification of Candidate Trustworthy Digital Repositories*. Structure: §1 Introduction; §2 Overview; §3 Reserved; §4 Principles; §5 General Requirements (legal, impartiality, liability); §6 Structural Requirements; §7 Resource Requirements (competence of personnel); §8 Information Requirements; §9 Process Requirements; §10 Management System Requirements. The auditor-side companion — what an audit body must do.

**Decision 40 citation correction.** Decision 40 originally cited `ISO 16363 §4.4.2` and `§5.1.2` from working memory of the standard. The actual CCSDS 652.0-M-2 (Dec 2024) sections are:
- `§4.4.1.2` — *"The repository shall actively monitor the integrity of AIPs."* (replaces the §4.4.2 citation)
- `§5.1.1.3` — *"The repository shall have effective mechanisms to detect bit corruption or loss."* (replaces the §5.1.2 citation)
- `§5.1.1.3.1` — *"The repository shall record and report to its administration all incidents of data corruption or loss..."* (new — covers the fail/missing reporting workflow)

Citations updated in decision 40, §3.8 fixity prose, and the §9.2 sizing-pass entry above.

**`standards/README.md` updated** to register both new standards with `ISO 16363 §<n.n.n>` and `ISO 16919 §<n.n.n>` citation prefixes; OAIS file reference updated to its renamed path (`ISO 14721 - oais-v3.md`); a note added clarifying ISO-vs-CCSDS numbering (the ISO numbers are stable identifiers but the checked-in content is the Dec 2024 CCSDS revisions, which are what ISO publishes).

**§9.4 priority 11 updated** to PARTIALLY ENGAGED with the new standards in scope. **§9.6 "outstanding ISO 16363 purchase" task** marked DONE. A new third-priority outstanding task remains (the systematic Q8 evidence-linkage pass), but it's non-blocking on Q12.2.

**Q12.2 still the top priority** for the next design session — now with full ISO 16363 + 16919 grounding available for evidence questions.

#### Session of 2026-05-16 (certification-pyramid framing — continuation)

Ivan added the **nestor Kriterienkatalog v2 (2008)** to `standards/` (PDF + extracted `.md` via `pdftotext -layout`), framing it as the middle tier of the trustworthy-repository certification pyramid (CTS → nestor → ISO 16363, in difficulty order). Ivan's working principle for this session: *"know what we need technically, and if we can't implement upright, not make it impossible to do later."*

**Technical-requirements audit done.** Three buckets evaluated:

- **(A) Already designed-for**: fixity / active integrity monitoring (decision 40 covers `ISO 16363 §4.4.1.2 / §5.1.1.3 / §5.1.1.3.1` and `nestor §6`); WORM event log + gapless audit trail (decisions 5, 26, 28 cover `nestor §7.2`); Producer-Archive agreement (decision 18 covers `ISO 16363 §3.5` / `nestor §3`); OCFL storage spec (decisions 21, 33, 36 cover `ISO 16363 §4.4.1` / `nestor §6.2`); fixity separated from AIPs by being in a different OCFL substrate (decision 36 satisfies `ISO 16363 §4.4.1.2` discussion's separation principle); self-describing recovery without DaSCH software (decision 36's Fedora-6 principle satisfies `nestor §4.6` succession).
- **(B) Deferred but architecturally additive**: HMAC + HSM event authentication (decision 39); Merkle-tree log + external root publication (decision 39); digital signatures on DIPs (`nestor §7.3`, Access Area concern); format-migration orchestration (Q14 parked); Significant Properties (`nestor §9.2`, no current need); Designated Community spec (organisational); restricted-access details (Q12.2 paused); on-demand BagIt/RO-Crate AIP serialisation (mitigates constituted-AIP deviation if asked).
- **(C) Foreclosed by current design**: **none identified.** Architecture is well-positioned for all three certification tiers without redesign.

**Critical finding made explicit: geographic redundancy.** `nestor §14` is the sharpest formulation ("a fire in the operating institution's main building must not destroy objects — a geographically-distant backup must take over"); `ISO 16363 §5.1.2` + `§5.2.1` are softer but equivalent. The current single-machine architecture **does not foreclose** this — both OCFL roots and the active-log directory are designed to replicate via filesystem-level mirroring — but the commitment was only a one-liner in decision 22. Made explicit in:

- **New decision 42**: certification-pyramid framing (CTS → nestor → ISO 16363) + preserve-optionality working principle; geographic backup is architecturally committed and deployment-deferred; HMAC+HSM, Merkle-tree log, DIP digital signatures, Significant Properties, BagIt/RO-Crate AIP serialisation listed as architecturally additive.
- **New §3.7 Phase-2 paragraph**: concrete replication plan — `zfs send` to a distant ZFS pool (or `rsync` with content-addressed verification); both OCFL roots replicate; active-log directory tolerates partial-trailing-line truncation per decision 37; Redb is rebuildable on the remote and need not be replicated; multi-copy synchronization (`ISO 16363 §5.1.2.1`) satisfied by replication tooling's atomic-snapshot semantics.

**Standards extension.** `standards/README.md` updated with the nestor entry and the new certification-pyramid table; "preserve optionality" stated as a working principle in the README alongside decision 27's "cite or deviate."

**Doc-status updates.**
- `§9.4` priority 11 reframed from "CoreTrustSeal evidence linkage" to "certification-pyramid evidence linkage" across all three tiers; current decisions referenced.
- `§9.6` gained a fourth-priority item (Phase-2 geographic-replication operational plan; needs to land before tier-2/nestor certification attempt). Third priority (per-decision evidence map across CTS / nestor / ISO 16363) carried forward.
- Doc header refreshed.

**Q12.2 remains the top priority** for the next design session; certification-pyramid evidence-linkage and geographic-replication operational plan are mechanical/operational and non-blocking.

#### Session of 2026-05-16 (evidence-index pass — continuation)

Continuation of the certification-pyramid framing session. The §9.6 third-priority work (per-decision evidence map) was tackled directly:

- **§8.1 Evidence index added** at the end of the decision log. Maps ~20 evidence-producing decisions to citations across three tiers (CTS R-number / nestor § / ISO 16363 §). Covers decisions 5, 13, 14, 17, 18, 20, 21, 24, 26, 27, 28, 31, 32, 33, 34, 36, 37, 39, 40, 42. Decisions not listed are internal-design choices or vocabulary cleanups that don't directly produce certification evidence.
- **Coverage-check table** in §8.1 identifies what falls *outside* DAO's architectural scope but will be required at audit time: mission statement + Designated Community, succession plan, financial sustainability evidence, security risk analysis, staff competence, periodic internal audit, format-risk monitoring, Phase-2 geographic-replication operational plan, Producer-Archive agreement contract language. Owners enumerated (DaSCH leadership / governance / infrastructure / Producer).
- **Honest caveat**: CTS R-numbers in §8.1 are working-memory of the 2023-2025 catalog; **the CTS Requirements document is not yet in `standards/`**. New §9.6 fifth-priority item added to fix this (free download from coretrustseal.org).
- **Reading guide** in §8.1 explains the citation conventions (nestor §-without-subsection = applies across all sub-points; `(meta)` for framing decisions; second-order claims not enumerated).

**Doc-status updates.**
- `§9.4` priority 11 reframed from PARTIALLY ENGAGED to MOSTLY DONE; remaining work is CTS-catalog verification and second-order claim enumeration (both during certification prep).
- `§9.6` third priority struck through as substantially done with the CTS-verification caveat carried forward to the new fifth-priority.
- New `§9.6` fifth-priority: add CTS Requirements catalog to `standards/`.

**Q12.2 still the top priority** for the next design session. All certification-pyramid evidence work has now been pulled forward to the point where Q12.2 can proceed without any preceding mechanical doc work blocking it.

#### Session of 2026-05-16 (CTS extension — continuation)

Ivan added three CoreTrustSeal documents to `standards/`: **Requirements 2026-2028 v01.00**, **Extended Guidance**, and **Glossary**. Extracted to `.md` via `pdftotext -layout`. This is the **current 2026-2028 catalog** — refreshed every 3 years; supersedes the 2023-2025 numbering that the §8.1 Evidence index had been using from working memory.

**Catalog version shift is material.** The 2026-2028 CTS reorders requirements vs. 2023-2025; key shifts that affected the §8.1 remap:

| Topic | 2023-2025 (working memory) | 2026-2028 (actual) |
|---|---|---|
| Storage procedures / fixity | R9 | **R14** (Storage & Integrity) |
| Preservation plan | R10 | **R09** |
| Technical quality | R11 | **R10** (Quality Assurance) |
| Workflows | R12 | **R11** |
| Discovery & identification | R13 | **R12** |
| Reuse | R14 | **R13** |
| Continuity of service (NEW concept) | (was implicit in R3 / R5) | **R03** — now explicit |
| Mission / scope | R1 | **R01** (with leading zero) |
| Security | R16 | **R16** (unchanged) |

The renumbering means **every CTS citation in §8.1** had to be remapped against the actual 2026-2028 catalog text. Done.

**Doc updates done in this session.**

- `standards/README.md` — added three CTS entries (Requirements + Extended Guidance + Glossary) with `CTS R<nn>` / `CTS Guidance R<nn>` / `CTS Glossary: <term>` citation prefixes; certification-pyramid table now notes the 2026-2028 catalog version and the 3-year refresh cadence.
- `§8.1` Evidence index — all 20 evidence-row CTS citations remapped to 2026-2028 (R01-R16 with leading zeros). Highlights: decision 18 (DepositAgreement) now cites R02 + R08 (rights + deposit); decision 26 (PreservationAction) now R09 + R11; decision 28 (Tombstoned/Redacted) gained R09 because the 2026-2028 R09 explicitly mentions tombstone records and deletion-impact on PIDs; decision 42 (certification pyramid) now cites R03 + R14 + R16 explicitly for the multi-copy commitment.
- `§8.1` coverage-check table — also remapped; gained two new rows for **Legal & Ethical** (CTS R04) and **Reuse-side Designated-Community engagement** (CTS R13) that the prior pass missed; periodic-internal-audit row noted as covered indirectly by R05 governance.
- The "working memory" caveat in §8.1 intro removed.
- `§9.4` priority 11 caveat about CTS-not-in-standards removed.
- `§9.6` fifth priority struck through as DONE.
- Doc header refreshed.

**No remaining outstanding doc work tied to certification-pyramid evidence linkage** beyond what's deferred to certification-preparation time (Extended Guidance cross-read; ISO 16919 auditor-perspective read; second-order claim enumeration). All three are explicitly scoped as "do this when actually applying for certification" rather than "do this now."

**Q12.2 remains the top priority** for the next design session; only the Phase-2 geographic-replication operational plan (fourth priority) remains outstanding, and it's organisational/operational, not architectural.

#### Session of 2026-05-16 (Curation & Preservation Levels Position Paper)

Ivan added the **CTS Curation & Preservation Levels Position Paper v3.0 (2024)** to `standards/`. CCSDS-format position paper, 212 lines after extraction. Establishes the **Z / D / C / A** taxonomy:

- **Z** — Unattended deposit-storage-access (out of CTS scope).
- **D** — Deposit Compliance (checks but no curation; not in CTS scope alone).
- **C** — Initial Curation (format/metadata work; not in CTS scope alone).
- **A** — Active Preservation (long-term commitment to understandability + rendering for Designated Community; **required for CTS scope**).

The CTS 2026-2028 Requirements catalog references "curation levels" in R08, R10, R11 but defers the taxonomy to the applicant — this paper supplies the recommended one. **Architectural implication for DAO**: should `dao:DepositAgreement` (decision 18) carry an explicit curation-level property naming the level committed? Likely yes, but the question is parked as Q15 (just added) rather than decided unilaterally — it touches per-Deposition vs. per-IE granularity and the relationship to `dao:PreservationAction` types.

**DaSCH-specific relevance.** The three project-type tiers (permanent infrastructure / full-lifecycle / publication-only) map onto curation-levels:

- **Permanent infrastructure**: Level A (full active preservation; DaSCH's strongest commitment).
- **Full-lifecycle**: likely Level A as well (active during the project; potentially downgrading after publication).
- **Publication-only**: could be Level C or A depending on the agreement.

Surfacing this in DAO would make the commitment auditable per Deposition and would distinguish DaSCH's tiered service offering from a flat "everything is preserved equally" claim.

**Doc updates done in this session.**

- `standards/README.md` — added Position Paper row with citation prefix `CTS Levels: <Z|D|C|A>`.
- `§7` parked questions — new Q15 added: "Curation-level commitment per `dao:DepositAgreement`" with sub-questions on per-DepositAgreement vs. per-Deposition vs. per-IE granularity, level-change event semantics, and PreservationAction-type constraints.
- `§8.1` Evidence index — decision-18 row annotated with the Q15 parked question reference.
- `§8.1` coverage-check table — new row added: "Curation-level commitment per Deposition / DepositAgreement" citing CTS R08/R09/R10 + Position Paper, nestor §1.2 / §9.2, ISO 16363 §3.3.1.
- Doc header refreshed.

**Q15 is non-blocking on Q12.2** but should be resolved before tier-1 CTS application begins (the level is a CTS-application-required declaration; cannot be deferred past the application moment).

**Q12.2 remains the top priority** for the next design session.

#### Session of 2026-05-16 (Q15 resolved — uniform A+C+D)

Ivan's response to the Q15 proposal cut through the per-DepositAgreement / per-Deposition / per-IE complexity I'd built up: **"At DaSCH we are currently building the archive to enable A for all projects. Additionally, we do C and D for all projects. We don't distinguish the levels per project. A+C+D is our mandate."**

**Q15 RESOLVED by decision 43**: DaSCH commits to **uniform A + C + D across all projects, all Depositions, all preserved content**. No per-Agreement / per-Deposition / per-IE variation. Consequences:

- `dao:DepositAgreement` does **NOT** gain a preservation-level property (the variation I'd proposed was answering a question DaSCH doesn't actually have).
- No `premis:preservationLevel` overrides in event payloads.
- No `CurationLevelChanged` event in the vocabulary.
- The Position Paper's Z / D / C / A taxonomy stays as reference vocabulary in `standards/`, not modelled as a tunable.
- The institutional invariant is documented in §1a "Preservation-level commitment" paragraph.

**Architectural lesson worth flagging.** I'd jumped to per-Deposition/per-IE granularity because the Position Paper *allows* it and PREMIS *supports* it natively. But "what the standard supports" is not the same as "what the institution commits to." DaSCH's mandate is uniform; modelling per-content variation would have been over-engineering for a non-existent use case. The decision-27 "cite or deviate" pattern works in the other direction too: the standard supports a tunable, DaSCH commits to a constant — *document the constant*, don't model the tunable.

**Doc updates done in this session.**

- **New decision 43** in the log: uniform A+C+D commitment with explicit consequences and rationale; flags that future tiered offerings would require revisiting.
- **§7 Q15** marked RESOLVED with pointer to decision 43.
- **§1a** gained a "Preservation-level commitment: uniform A + C + D across all projects" paragraph, parallel to the CoreTrustSeal-target paragraph.
- **§8.1 evidence index** — decision-18 row clarified (no level property; cites decision 43); decision-43 row added (CTS R08/R09/R10, nestor §1.2/§6/§7/§8, ISO 16363 §3.3.1/§4.3/§4.4.1.1).
- **§8.1 coverage-check table** — "curation-level commitment" row simplified: now points at decision 43 architecturally + flags that the *public mandate statement* itself is organisational (needs to appear in CTS application).
- Doc header refreshed.

**Net effect**: Q15 was the lightest of the remaining design threads; resolving it leaves only Q12.2 (top priority) and the Phase-2 geographic-replication operational plan (organisational/operational) outstanding.

**Q12.2 is now the only remaining design thread.**

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

Status legend: **OPEN** (not yet addressed), **PAUSED** (started, paused mid-flight), **RESOLVED** (done), **DONE** (research-thread completed).

1. ~~**Q4 / Q5 push-topology details**~~ **RESOLVED 2026-05-14/15** (§9.3, decisions 31 + 32).
2. ~~**Q3 Deposition granularity**~~ **RESOLVED implicitly 2026-05-14** by decision 31 (Path A: producer-set, no Archive-enforced ceiling; one Deposition = one producer-side submission against one Project at one point in time; multiple Depositions over a project lifecycle = iterative deposit). Formal write-up to be added to the prose narrative; the design itself is settled.
3. **Q12 `dao:AccessRights` for restricted access** — **PAUSED** mid-Q12.2. Q12.1 resolved: enforcement at delivery layer (γ). Q12.2 (composition at IE / Rep / File levels with the per-File representation now decided as blank-node `dao:hasFile` properties, not a class) needs resuming with the storage architecture (decisions 36-39) in place. **Top priority for next session — see §9.6.**
4. **Q10 Representation property list** — OPEN. Concrete grunt work. Walk PREMIS DD §1.2 (Object > File > Bitstream semantic units) and Knora `:FileValue` properties, produce the final canonical list shaped against decision 27 (PREMIS-aligned where possible; external standards per §4.4). Now also needs to specify the blank-node shape inside Representation RDF (per decision 33).
5. **Q11 ARK reservation expiry policy** — OPEN. Operational. Should reservations expire if never claimed; if so, how long; reclamation semantics.
6. **Q14 Preservation-action workflows (policy)** — **PARTIALLY ENGAGED 2026-05-15** (sizing pass). `dao:PreservationAction` class is settled (decision 26). **Fixity *cadence* and *granularity* settled (decision 40)**: per OCFL Object, continuous-when-idle + ≥1 sweep/Object/year, variant payload by outcome. **Still open**: (i) response workflow when `FixityChecked` returns `fail` / `missing` — alerting, triage, recovery from secondary copies, who-decides-what; (ii) format-migration triggering policy — when does DaSCH initiate a `FormatMigrated` action, against which format-risk signals (PRONOM advisories, vendor obsolescence notices), with what stakeholder review; (iii) system-ontology-migration policy.
7. **Q13 Project metadata as ontology** — OPEN. Large workstream; may surface `dao:Place`, `dao:Concept`, etc. Defer until the above are answered unless a near-term project demands it.
8. **Q1 Path inventory for new IE Version** — OPEN. Clarifying detail; non-blocking.
9. **Q2 Event ordering (per-entity vs global)** — OPEN. Per-entity is the working assumption; document explicitly. **Note**: decision 37 introduces both `global_offset` (monotonic across all events) and `stream_version` (per-entity counter), so both orderings are now first-class. The question becomes "what does each guarantee, and what should the SSE feed default to?"
10. **Q6 / Q7 DPE/CPE presentation hints and IIIF Manifests in DAO** — OPEN. Both lean "no"; document the no and move on.
11. **Q8 Certification-pyramid evidence linkage (CTS → nestor → ISO 16363)** — **MOSTLY DONE 2026-05-16**. Decision 42 records the pyramid framing and the preserve-optionality principle; decision 39 names HMAC+HSM and Merkle-tree log structure as the deferred non-repudiation approaches with explicit threat-model boundary; decision 40 cites `ISO 16363 §4.4.1.2 / §5.1.1.3 / §5.1.1.3.1` directly; decision 42 + §3.7 Phase-2 paragraph make the geographic-replication commitment explicit; **§8.1 Evidence index** maps ~20 evidence-producing decisions to CTS R-numbers / nestor § / ISO 16363 § and identifies organisational/operational items outside DAO scope. `standards/` contains all three tiers' source documents: CoreTrustSeal Requirements 2026-2028 v01.00 (+ Extended Guidance + Glossary), nestor Kriterienkatalog v2 (2008), ISO 16363 (CCSDS 652.0-M-2 Dec 2024), and ISO 16919 (CCSDS 652.1-M-3 Dec 2024). **Still pending** for certification-prep time: cross-reading the CTS Extended Guidance against each evidence-index row; enumerating second-order evidence claims (where a decision enables an org-level claim rather than supplying a technical artefact); reading ISO 16919 for the auditor-side perspective on evidence-production at tier 3.
12. ~~**Fedora 6 OCFL patterns (research thread)**~~ **DONE 2026-05-15.** Research informed decisions 33 / 34 / 35. Findings: Fedora 6 maps each LDP resource to one OCFL Object (identifier is the resource URN, hashed via Hashed-N-Tuple Storage Extension); RDF serialised as N-Triples canonically at rest (Turtle / JSON-LD only for content negotiation on the API); binary + description as sidecar pair in same Object (`<file>` + `<file>~fcr-desc.nt`); object-proliferation is the major operational pain (Hyrax migrations report 10+ OCFL Objects per intellectual work); Archival Groups added as Fedora's retrofit for the proliferation problem; database is rebuildable cache, OCFL is source of truth.
13. ~~**Fedora LDP (Linked Data Platform) lessons (research thread)**~~ **DONE 2026-05-15.** Research informed decision 35. Findings: LDP containers (DirectContainer / IndirectContainer / BasicContainer) and PCDM (Collection / Object / File / Hash) caused operational pain: one-parent containment constraint, tombstoned URIs blocking ID reuse, chatty per-resource API, conflation of intellectual structure with storage structure. Samvera community's pivot from ActiveFedora-on-LDP to Valkyrie in Hyrax 5 (2024) is the strongest signal. DaSCH does not adopt LDP/PCDM as modelling primitives; only the LDP-NR + LDP-RS sidecar pair pattern survives at storage layer.
14. **OCFL-as-event-store and CQRS-ES prior art (research thread)** — **DONE 2026-05-15.** Research informed decisions 36 / 37 / 38 / 39. Findings: no documented OCFL-backed event store exists; all OCFL adopters (Fedora 6, Stanford SDR, Harvard DRS, Oxford ORA, Wisconsin, Penn State) are state-sourced. Every dedicated event store (EventStoreDB / Datomic / Kafka with Tiered Storage / Pulsar) separates events from large blobs. Industry CQRS-ES practice (Vernon, Young, EventStoreDB conventions) treats domain entities as aggregate roots, not commits. OCFL anti-patterns flagged: many small files inside Objects degrades performance; ZFS-dedup-vs-OCFL-portability tension; "many small Objects" produces year-3 operational pain (Hyrax evidence). These findings drove the pivot to two-substrate architecture (decision 36).
15. ~~**§3.6 / §3.7 narrative prose rewrite**~~ — **DONE 2026-05-15** (prose-rewrite session). §3.6 now describes Preservation File bytes inside Rep OCFL Objects with the bytes-index lookup; the event payload example uses the URN scheme and decision-37 NDJSON-JSON-LD shape. §3.7 architecture replaced: two-substrate directory tree (event-log-storage-root + entity-storage-root + active log + Redb cache), entity Object versioning on curatorially-meaningful state changes, write/recovery paths for both substrates, refreshed "what this gets DaSCH" framing. §3.7.1 Binary retrieval API gained a bytes-index lookup note. §3.7.2 Subscriber bootstrap gained an entity-Object-snapshot vs. α-snapshot disambiguation note.

### 9.5 Process notes

- Standards PDFs live in [`standards/`](./standards/) as both `.pdf` and `.md` (verbatim text extracts via `pdftotext -layout`). The `.md` extracts are grep-able and exist specifically to make in-doc citation cheap.
- `CONTEXT.md` is the working glossary; treat it as authoritative for terminology that has stabilized. The discovery doc is the working *design narrative*; treat it as authoritative for decisions and their rationale. `UBIQUITOUS_LANGUAGE.md` at the repo root is the consolidated cross-context glossary; it complements `CONTEXT.md` rather than replacing it.
- No ADRs have been written yet. The strongest ADR candidates, in approximate priority order, are: **decision 17** (build-vs-buy: self-built Archive); **decision 26** (`dao:PreservationAction` as first-class); **decision 29** (Trusted Repository domain framing); **decision 30** (three-tier role vocabulary); **decision 36** (two-substrate storage architecture); **decision 38** (Redb for cache); **decision 39** (fixity baseline + deferred non-repudiation threat model). These should be written before implementation begins.
- **Ship and measure before optimising.** Decisions that affect operational performance (event volumes, replay strategies, storage size, derivation costs) ship in their simplest form first, run against real load, and are then optimised based on observed behaviour. The architecture is designed to allow this — full SSE firehose can become filtered, single-segment-per-month can be sub-bucketed, derivation can move from eager to lazy, mmap can be added later, HMAC+HSM and Merkle-Tree non-repudiation are additive layers. None of those refinements is pre-built. If a question of the form "should we optimise X now?" comes up, the default answer is *not until we have measured X under real load*.
- **Research before architectural commitment.** Two research-agent dispatches in the 2026-05-14/15 session changed substantive design decisions (Fedora 6 + LDP research → decisions 33-35; OCFL-as-event-store research → decisions 36-39, second-pivot to two-substrate). When the architecture is in greenfield territory (i.e. no obvious prior art for the exact combination being attempted), do this round of research *before* recording the decision. The cost is one agent dispatch; the benefit is not accumulating year-3 operational debt from a design that nobody else has tried.
- **Don't model what the institution doesn't commit to.** If a standard offers a tunable (e.g., the CTS Curation & Preservation Levels Position Paper's Z / D / C / A per-IE-or-per-Deposition assignment) and the institution commits to a constant (DaSCH's uniform A + C + D), document the constant — don't model the tunable. Q15 / decision 43 is the worked example: PREMIS supports `preservationLevel` per IE / Rep / File, and the Position Paper allows per-content variation; modelling that would have been over-engineering for a non-existent use case. **Standards support a surface area; institutions commit to a sub-region of it. Model the commitment, not the support surface.** Ask "what does DaSCH commit to?" *before* "what does the standard allow?".

### 9.6 Where we left off — next-session entry points

**Last paused: 2026-05-16, end of the standards-extension week. Conversation paused cleanly; nothing in flight.**

#### Current state (2026-05-16)

The week's work cleared every preceding mechanical and foundational thread. **Q12.2 access-rights composition is the only remaining design thread.**

Architecture is stable on:

| Concern | Settled by |
|---|---|
| Domain framing (Trusted Repository; bounded contexts) | Decision 29, `CONTEXT-MAP.md` |
| Numerical baseline (100 deposits/yr, 200K events/deposit, 20M events/yr ingest steady-state, ~220M events/yr fixity-dominated at year 20, ~1 PB at year 20) | Decision 22 amended |
| Storage (two-substrate OCFL: event-log root + entity root; Redb cache) | Decisions 21, 33, 34, 36, 37, 38 |
| Identifiers (URN internal `urn:dsp:{type}:{uuid}`; ARK long-term public) | Decision 12 + amendment, decision 13 |
| Event vocabulary (10 event types) | §5.1 |
| Fixity policy (per-OCFL-Object, continuous-when-idle + ≥1/year, variant payload by outcome) | Decision 40 |
| Public APIs (Commands, Events SSE, Binary retrieval; `/metrics` as operational fourth surface) | Decisions 31, 41 |
| Certification pyramid (CTS → nestor → ISO 16363; preserve-optionality; geographic-redundancy architecturally committed, deployment-deferred) | Decision 42, §3.7 Phase-2 paragraph |
| Preservation-level commitment (**uniform A + C + D**, no per-content variation) | Decision 43, §1a |

Standards available in `standards/` (all extracted as `.md` for grep-ability):

- OAIS / ISO 14721 (CCSDS 650.0-M-3, Dec 2024)
- PREMIS DD v3.0 + PREMIS OWL Guidelines
- **CTS 2026-2028** — Requirements + Extended Guidance + Glossary + Curation Levels Position Paper v3.0
- **nestor Kriterienkatalog v2** (2008)
- **ISO 16363** (CCSDS 652.0-M-2, Dec 2024)
- **ISO 16919** (CCSDS 652.1-M-3, Dec 2024)

Compact summaries (use these to orient quickly):

- `§8.1` Evidence index — 20 decisions × 3 certification tiers + coverage-check table
- `CONTEXT.md` — Archive context glossary
- `UBIQUITOUS_LANGUAGE.md` (repo root) — cross-context glossary
- `CONTEXT-MAP.md` (repo root) — bounded-context map

#### How to resume

1. Read **§9.1** (methodology).
2. Read this **§9.6** section (current state + top priority).
3. Skim **§8.1 Evidence Index** for the compact decision overview (~20 rows).
4. Open **Q12.2** with the three sub-questions below.

#### Top priority — Q12.2 access-rights composition (PAUSED since 2026-05-14/15)

Q12.1 was resolved (γ delivery-layer enforcement). Q12.2 was in flight when we pivoted to the storage research. All foundational dependencies now settled — decisions 33 (no `dao:File` class; per-File info as `dao:hasFile` blank nodes), 36 (two-substrate storage), 40 (fixity policy), 43 (uniform A+C+D commitment). Q12.2 has a stable foundation.

Three sub-questions:

1. **Multi-level composition** — confirm IE / Representation / File-as-blank-node-on-Rep as the three levels at which `dao:AccessRights` applies, with most-restrictive-wins composition and no "exception up the chain".
2. **Structure of `restricted_access`** — simple property value with a URI reference to a separate policy document, or a `dao:AccessPolicy` class with structured rule fields? (COAR's `restricted_access` is the only non-trivial case; `open` / `embargoed` / `metadata_only` work as simple properties.)
3. **Audit trail mechanism** — restricted-access decisions are made at the delivery layer (γ). Where does the audit trail of "who accessed what when" live? Likely answer: delivery-layer logs are operational; the Archive carries the policy and reflects access-policy *changes* as `AccessRuleChanged` events (which already exists in §5.1).

#### Outstanding-but-non-blocking on Q12.2

**Phase-2 geographic-replication implementation plan.** Decision 42 + §3.7 Phase-2 paragraph commit architecturally to a distant DR copy via filesystem mirroring (`zfs send` to a remote ZFS pool, or `rsync` with content-addressed verification). Deployment is deferred. Concrete planning needed: remote site selection, bandwidth for initial seed, `zfs send` cadence (per-Deposition / hourly / daily), failover RTO/RPO targets, remote fixity verification, secondary-site fixity-checker policy, integration with broader DaSCH operational landscape. Organisational + operational, not architectural. **Should land before tier-2 (nestor) certification attempt.**

#### Methodology reminders for the resuming session

- One question at a time (§9.1).
- Cite or deviate (decision 27).
- Update `CONTEXT.md` inline as terms resolve (§9.1).
- Don't reopen settled decisions without flagging that you're doing so (§9.1).
- **Ship and measure** before optimising (§9.5).
- **Research before architectural commitment** in greenfield territory (§9.5).
- **Don't model what the institution doesn't commit to** (§9.5; Q15 / decision 43 is the worked example).

#### Done log (struck-through priorities from past sessions — skip on resumption; kept for traceability only)

- ~~§3.6 / §3.7 narrative prose rewrite~~ — DONE 2026-05-15 (matches decisions 36-38 two-substrate model).
- ~~`CONTEXT.md` / `UBIQUITOUS_LANGUAGE.md` URN touch-ups~~ — DONE (already aligned).
- ~~ISO 16363 + 16919 added to `standards/`~~ — DONE 2026-05-16.
- ~~Certification-pyramid evidence-linkage pass~~ — DONE 2026-05-16 (§8.1 Evidence index).
- ~~CTS Requirements + Extended Guidance + Glossary added to `standards/`~~ — DONE 2026-05-16.
- ~~CTS Curation & Preservation Levels Position Paper added; Q15 raised~~ — DONE 2026-05-16.
- ~~Q15 (curation-level commitment) resolved~~ — DONE 2026-05-16 by decision 43 (uniform A+C+D).

