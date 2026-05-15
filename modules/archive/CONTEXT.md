# Archive

The ubiquitous language of the Archive context. Defines what DSP preserves, in what shape, and with what guarantees of readability decades from now. The bit-level long-term preservation contract lives here. Distinct from the DPE / OAI-PMH publication layer, which projects the Archive into a presentation format.

Full design narrative: [`dao-discovery.md`](./dao-discovery.md). Standards extracts: [`standards/`](./standards/).

## Language

This glossary covers terms meaningful to domain experts (archivists, OAIS practitioners, DaSCH curators). Implementation-only terms are not listed here; see the discovery doc for the full vocabulary.

### DAO write-side classes (canonical truth in the Archive)

**IntellectualEntity (IE)**
The PREMIS-aligned term for "a coherent set of content reasonably described as a unit." Has a stable persistent-identity HTTPS URI (`https://archive.dasch.swiss/ie/{uuid}`) and one ARK for long-term citation. Versions over time at deliberate publication events. Replaces the earlier working term **Resource** (which clashed with `knora-base:Resource`).
_Avoid_: Record, Object, Item, Resource.

**Representation**
PREMIS-aligned. The preservation-grade bundle: a set of one or more **Preservation Files** plus Representation-level metadata (license, authorship, copyright, technical metadata). Has a stable persistent-identity URI (`https://archive.dasch.swiss/rep/{uuid}`) and one ARK. Versions over time. Many-to-many with IEs across Versions. Replaces the earlier working term **Asset** (which had a different VRE meaning); the older informal label **Archival Master** is also retired in favour of the three-tier role vocabulary (see Preservation chain roles below).
_Avoid_: File, Blob, Attachment, FileValue, Asset, Archival Master, Archival Information Package (AIP belongs in OAIS-vocabulary discussions only; the Representation is *what is preserved*, not the OAIS package shape).

**Project**
A research project archived in DSP, identified by a 4-character shortcode (e.g. `0803`) and addressed by a project-level ARK. Carries project-level metadata (title, funding, attributions, legal info, temporal coverage, disciplines). Producer/owner context for IEs and Representations.
_Avoid_: Dataset, collection.

**Agent**
Person, organization, or software acting as creator, contributor, depositor, maintainer, or preservation-action runner. May carry external identifiers (ORCID for people, ROR for organizations). Aligned with `PREMIS DD §1.3` (Agent entity).

**Deposition**
**Producer-induced** unit of ingest. Groups events from one producer-side submission. Gated by a `DepositAgreement`. The OAIS Submission Information Package (SIP) is the *wire format*; `dao:Deposition` is the *durable record* of which SIP arrived, was validated, and was committed.
_Avoid_: Submission (overloaded with the wire-format sense), Ingest (which is the OAIS functional entity, not a thing-being-ingested), Batch.

**DepositAgreement**
Producer/Archive contract. Carries producer identity, designated community, accepted formats, retention terms, embargo/access defaults, submission frequency. May store a link to where the executed agreement document lives. Loosely corresponds to OAIS *Submission Agreement* (`OAIS §6.1`).

**PreservationAction**
**Archive-induced** unit of change, distinct from `Deposition`. Gated by internal preservation policy rather than a `DepositAgreement`. Groups events resulting from archive-initiated activity: format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction. Aligned with `PREMIS DD §1.4` (Event entity); `OAIS §4.1.3 / §5.1` (Preservation Planning).
_Avoid_: PreservationEvent (collides with the lower-case `dao:Event`), Maintenance, Curation (too vague).

**Event**
The event-sourcing primitive. The event log is the source of truth on the write side. Subclasses for the working vocabulary: `IntellectualEntityVersionPublished`, `RepresentationVersionCreated`, `DepositionAccepted`, `PreservationActionExecuted`, `FixityChecked`, `FormatMigrated`, `AccessRuleChanged`, `Tombstoned`. Events are immutable facts; they are emitted only after a Command has been validated. (Conceptually parallels `PREMIS DD §1.4` Event, though `dao:Event` is broader: it covers identity-creation and projection-relevant events, not only preservation events.)

### Versioning vocabulary

**IE Version / Representation Version**
Versions are **read-side projections**, not DAO write-side classes. A specific Version is "the n-th `IntellectualEntityVersionPublished` event for this IE." The read store materializes Version nodes from the event log. Cited as `.../v{n}` (e.g. `.../ie/{uuid}/v3`).
_Avoid_: treating these as first-class DAO classes — they are deliberately not.

### Preservation chain roles (shared vocabulary across contexts)

Three role-based labels for artifacts along the preservation-to-delivery chain. Roles are distinguished by **purpose** in the preservation chain, not by format, location, or source provenance. Originated in SIPI's IIIF Server vocabulary and adopted here as cross-context Published Language. **Replaces the earlier two-tier "Archival Master / Service Master" framing** (both terms retired).

**Preservation File**
Long-term bit-level preservation. The file bytes inside a `dao:Representation` — a Representation may contain multiple Preservation Files (e.g. a TIFF plus an XMP sidecar). Authoritative; WORM. Content-addressed by hash (`dao-discovery.md §3.6`).
_Avoid_: "Archival Master" (retired); confusing with `dao:Representation` itself — the Representation is the *bundle* of Preservation Files plus Representation-level metadata, not a single file.
_Owned by_: the Archive context.

**Service File**
Mezzanine baseline derived from Preservation File(s) under a derivation rule. Sized and shaped for downstream delivery contexts (e.g. pyramidal TIFF for IIIF). Regenerable from the Preservation File + derivation rule; carries no preservation commitment; no ARK; no Version.
_Avoid_: "Service Master" (retired — "Master" implied an authority Service Files do not have).
_Owned by_: the Access Area context.

**Access File**
End-user delivery payload, generated on demand from a Service File plus request parameters (IIIF region/size/rotation/quality/format; downloaded bytes; rendered page; etc.). Ephemeral; not stored, not preserved.
_Owned by_: the delivery context that serves the request (IIIF Server, asset server, DPE, …).

### Identifier vocabulary

**Internal IRI**
A DaSCH-controlled HTTPS URI in the Archive namespace (`https://archive.dasch.swiss/...`). Stable within a system; **not** promised across system migrations. Two forms: persistent-identity (`.../ie/{uuid}`, no version suffix) on the write side, and Version-suffixed (`.../ie/{uuid}/v{n}`) as a read-store contract.

**ARK**
The single long-term-stable public identifier. Issued by the **ARK Resolver**, which is its own bounded context with its own event store. Minted per persistent-identity entity (IE, Representation, Project), not per Version — a Version is denoted by an ARK suffix.
_Avoid_: DOI (DaSCH does not currently mint DOIs; if added, they would be additional, not a substitute), Handle, PURL, "permalink."

### Internal structure of the Archive context

The Archive is one bounded context but is deployed as two services. Both speak DAO directly; no anti-corruption layer between them.

**Ingest Area**
Producer-facing deployment. Receives SIPs uploaded from RDU-Tooling, runs validation (SHACL against DAO; `DepositAgreement` enforcement; format and fixity checks). Async by design — large Depositions can take "as long as they take." Only on validation success does it trigger the `DepositionAccepted` event in the rest of the Archive. Separated for bandwidth, failure isolation, and producer-facing operational independence. See `dao-discovery.md §1a`.
_Avoid_: confusing with OAIS *Ingest* (the functional entity, broader concept) or with `dao:Deposition` (the durable record of a successfully committed Submission); the Ingest Area is the *service that runs the gate*, not the gate's output.

### Boundary commitment

**OCFL is internal to the Archive context.** Access Area subscribers, the ARK Resolver, and any other subscriber reach the Archive **only** through its three public APIs: the Commands API (write side), the Events SSE feed (read side, full firehose), and the Binary retrieval API (`GET /bitstreams/{multihash}`). No external context — and no other deployment — touches the OCFL store directly. The event-index, SQLite cache, and Object layout under `ocfl-storage-root/` are implementation details of the Archive, not part of any cross-context contract. See `dao-discovery.md §3.7.1` and decisions 31 / 32.

## Relationships

- A **Project** contains zero-or-more **IntellectualEntities** and zero-or-more **Representations**.
- An **IntellectualEntity** Version **pins specific Representation Versions** (model (a) — see `dao-discovery.md §3.2`).
- A **Representation** belongs to exactly one **Project** for ownership purposes; it may be **referenced by multiple IE Versions** (the many-to-many is across Versions, not contemporaneous).
- A **Representation Version** once pinned by a published IE Version **can never be deleted** (preservation commitment; `dao-discovery.md §3.2`).
- A **Deposition** is associated with exactly one **Project** and at least one **Agent** (the depositor); it produces one-or-more IE/Representation events.
- A **PreservationAction** is associated with zero-or-one **Project** (cross-project actions are allowed — e.g. fleet-wide format migration); it is initiated by an **Agent** of type "Archive system agent."
- An **ARK** binds to a persistent-identity Internal IRI. Bindings are mutable across system migrations; the ARK string is not.

## Flagged ambiguities

- **"Record"** is used in three different ways across the platform and must be disambiguated.
  - In the Archive context (this document and `dao-discovery.md`), Record is **not** a domain term. The archived units are **IntellectualEntities** and **Representations**.
  - In the DPE / `dpe-core` codebase, `Record` is currently a flat metadata projection used to serve OAI-PMH (`id`, `pid`, `label`, `accessRights`, `legalInfo`, `typeOfData`, …). It conflates IE-like and Representation-like things behind a single shape.
  - In OAI-PMH itself, "record" has a precise external meaning: a metadata blob in a specific metadata format, keyed by an OAI identifier and a datestamp.
  - **Resolution: pending** — DPE may keep a `Record` projection name, but the Archive context must not adopt the term. Cross-context translation happens at the read-side projector that feeds DPE.

- **"Version"** is overloaded.
  - DAO: Version is a read-side projection over events. There is no `dao:Version` class on the write side.
  - PREMIS DD: PREMIS uses `objectIdentifier` plus event chains; PREMIS does not have a single "Version" concept either, and DAO's Version-as-projection is aligned with that.
  - VRE: the VRE has fine-grained edit history that the Repository deliberately does *not* preserve as Versions. Only deliberate publication events become Versions on the read side.

- **"AIP / SIP / DIP"** belong to OAIS vocabulary. They describe **package shapes at boundaries**, not durable entities.
  - At the Producer→Archive boundary a **SIP** arrives; the durable artifact recording its arrival is a `dao:Deposition`.
  - The **AIP** corresponds to "what is preserved" — i.e. the union of `dao:IntellectualEntity` + its pinned `dao:Representation` Versions + descriptive metadata + provenance events. DAO does not have an `AIP` class; the AIP is *constituted* from the write-side entities.
  - **DIP** is constructed by an Access Area subdomain on demand from Service Files and Service Projections. Not a DAO concern.

## Example dialogue

> **Dev:** "When we archive a digitised manuscript, do we store one **IntellectualEntity** with many **Representations**, or many **IntellectualEntities** each with one **Representation**?"
> **Domain expert:** "Depends on the data model the project chose. A page-by-page model yields one IE per page, each pinning one image Representation Version. A book-as-blob model has one IE pinning many Representation Versions."

> **Dev:** "If the same scan is referenced by two **IntellectualEntities**, do we store the bytes twice?"
> **Domain expert:** "No. Bitstreams are content-addressed by hash (`dao-discovery.md §3.6`). Both Representations resolve to the same bytes in the content store. The Representation-level metadata may differ between the two — each Representation is its own descriptive artifact even if it points at the same bytes."

> **Dev:** "Who can trigger a format migration?"
> **Domain expert:** "Only the Archive itself. A format migration is a `dao:PreservationAction`, not a `dao:Deposition` — a producer cannot submit one. The `DepositAgreement` doesn't apply; internal preservation policy does."
