# Archival Format

The bit-level long-term preservation contract for DSP. Defines what is stored in the archive, in what shape, and with what guarantees of readability decades from now. Distinct from the DPE / OAI-PMH publication layer, which projects the archive into a presentation format.

Full design narrative: [`dao-discovery.md`](./dao-discovery.md). Standards extracts: [`standards/`](./standards/).

## Language

This glossary covers terms meaningful to domain experts (archivists, OAIS practitioners, DaSCH curators). Implementation-only terms are not listed here; see the discovery doc for the full vocabulary.

### DAO write-side classes (canonical truth in the Archive)

**IntellectualEntity (IE)**
The PREMIS-aligned term for "a coherent set of content reasonably described as a unit." Has a stable persistent-identity HTTPS URI (`https://archive.dasch.swiss/ie/{uuid}`) and one ARK for long-term citation. Versions over time at deliberate publication events. Replaces the earlier working term **Resource** (which clashed with `knora-base:Resource`).
_Avoid_: Record, Object, Item, Resource.

**Representation**
PREMIS-aligned. The **Archival Master**: a preservation-grade set of files plus Representation-level metadata (license, authorship, copyright, technical metadata). Has a stable persistent-identity URI (`https://archive.dasch.swiss/rep/{uuid}`) and one ARK. Versions over time. Many-to-many with IEs across Versions. Replaces the earlier working term **Asset** (which had a different VRE meaning).
_Avoid_: File, Blob, Attachment, FileValue, Asset, Archival Information Package (AIP belongs in OAIS-vocabulary discussions only; the Representation is *what is preserved*, not the OAIS package shape).

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

**Archival Master**
The Representation. The preservation-grade Master of record. WORM.

**Service Master**
A *derived projection* of one or more Representations, shaped for a specific consumer (pyramidal TIFF for IIIF, search index for DPE, denormalized RDF for SPARQL). Not in DAO. Regenerable by replay from a Representation Version + derivation rule. Has no ARK and no Version of its own.

### Identifier vocabulary

**Internal IRI**
A DaSCH-controlled HTTPS URI in the Archive namespace (`https://archive.dasch.swiss/...`). Stable within a system; **not** promised across system migrations. Two forms: persistent-identity (`.../ie/{uuid}`, no version suffix) on the write side, and Version-suffixed (`.../ie/{uuid}/v{n}`) as a read-store contract.

**ARK**
The single long-term-stable public identifier. Issued by the **ARK Resolver**, which is its own bounded context with its own event store. Minted per persistent-identity entity (IE, Representation, Project), not per Version — a Version is denoted by an ARK suffix.
_Avoid_: DOI (DaSCH does not currently mint DOIs; if added, they would be additional, not a substitute), Handle, PURL, "permalink."

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
  - In the archival-format context (this document and `dao-discovery.md`), Record is **not** a domain term. The archived units are **IntellectualEntities** and **Representations**.
  - In the DPE / `dpe-core` codebase, `Record` is currently a flat metadata projection used to serve OAI-PMH (`id`, `pid`, `label`, `accessRights`, `legalInfo`, `typeOfData`, …). It conflates IE-like and Representation-like things behind a single shape.
  - In OAI-PMH itself, "record" has a precise external meaning: a metadata blob in a specific metadata format, keyed by an OAI identifier and a datestamp.
  - **Resolution: pending** — DPE may keep a `Record` projection name, but the archival-format context must not adopt the term. Cross-context translation happens at the read-side projector that feeds DPE.

- **"Version"** is overloaded.
  - DAO: Version is a read-side projection over events. There is no `dao:Version` class on the write side.
  - PREMIS DD: PREMIS uses `objectIdentifier` plus event chains; PREMIS does not have a single "Version" concept either, and DAO's Version-as-projection is aligned with that.
  - VRE: the VRE has fine-grained edit history that the Repository deliberately does *not* preserve as Versions. Only deliberate publication events become Versions on the read side.

- **"AIP / SIP / DIP"** belong to OAIS vocabulary. They describe **package shapes at boundaries**, not durable entities.
  - At the Producer→Archive boundary a **SIP** arrives; the durable artifact recording its arrival is a `dao:Deposition`.
  - The **AIP** corresponds to "what is preserved" — i.e. the union of `dao:IntellectualEntity` + its pinned `dao:Representation` Versions + descriptive metadata + provenance events. DAO does not have an `AIP` class; the AIP is *constituted* from the write-side entities.
  - **DIP** is constructed by the Access Area on demand from Service Masters. Not a DAO concern.

## Example dialogue

> **Dev:** "When we archive a digitised manuscript, do we store one **IntellectualEntity** with many **Representations**, or many **IntellectualEntities** each with one **Representation**?"
> **Domain expert:** "Depends on the data model the project chose. A page-by-page model yields one IE per page, each pinning one image Representation Version. A book-as-blob model has one IE pinning many Representation Versions."

> **Dev:** "If the same scan is referenced by two **IntellectualEntities**, do we store the bytes twice?"
> **Domain expert:** "No. Bitstreams are content-addressed by hash (`dao-discovery.md §3.6`). Both Representations resolve to the same bytes in the content store. The Representation-level metadata may differ between the two — each Representation is its own descriptive artifact even if it points at the same bytes."

> **Dev:** "Who can trigger a format migration?"
> **Domain expert:** "Only the Archive itself. A format migration is a `dao:PreservationAction`, not a `dao:Deposition` — a producer cannot submit one. The `DepositAgreement` doesn't apply; internal preservation policy does."
