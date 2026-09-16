# Archive Area (Spycherli)

The OAIS archive of the Trusted Repository, working name **Spycherli** (Swiss German, "little granary"): Ingest, Archival Storage and the supporting functional entities. It holds the preservation-grade record of what DSP preserves, in a shape meant to stay readable decades from now, and it is sealed: no other context reads its storage. No code exists in this repository yet; this file records the vocabulary and the boundary commitments the code will have to honour, so that an agent can understand the seams the Deposit and Access Areas are built against without leaving the repository. Contract terms (Project, Shortcode, Person, Organization) and the file vocabulary (Preservation File, Service File, Original) are defined in the root [`CONTEXT.md`](../CONTEXT.md) `## Shared` and only used here.

## Language

### What is preserved

**Resource**:
A coherent set of content described as one unit — the archive's counterpart of a research project's resource, with the active-research runtime concerns stripped at ingest; identified by an internal IRI and one ARK, versioned at deliberate publication events.
_Avoid_: Record, Object, Item, Intellectual Entity (retired).

**Representation**:
The preservation-grade bundle of one or more Preservation Files plus their metadata (license, authorship, technical metadata); identified by an internal IRI and one ARK, versioned, and referenced by one or more Resource versions.
_Avoid_: Asset, File, Blob, Attachment, Archival Master.

**Application Profile**:
The per-project data artifact that replaces the project's ontology in the archive: one documentation record per observed property, class and value list, keyed on what the data actually uses. Travels in every SIP.
_Avoid_: ontology snapshot, schema export.

**DAO**:
The DaSCH Archival Ontology — the archive's language as a machine-readable schema (OWL + SHACL). It appears at the archive's boundaries (what a SIP must contain, what a data product carries); other areas translate at the seam and never adopt it as their own vocabulary.

### How things change

**Deposition**:
The producer-induced unit of ingest: the durable record that one submission arrived, was validated and was committed; gated by a DepositAgreement.
_Avoid_: Submission (the editor's row and the OAIS wire-format sense), Ingest (the functional entity), Batch.

**DepositAgreement**:
The contract between a producer and the archive: identity, accepted formats, retention terms, embargo and access defaults.
_Avoid_: Submission Agreement (the OAIS term; use only in OAIS discussions).

**PreservationAction**:
The archive-induced unit of change — a format migration, a fixity-driven re-encoding, a bulk metadata correction — gated by internal preservation policy rather than by a DepositAgreement.
_Avoid_: PreservationEvent, Maintenance, Curation.

**AccessPolicy**:
A first-class entity carrying an opaque policy document, referenced from Resource and Representation versions whose access rights are restricted; one policy may cover many versions.
_Avoid_: Permission, Authorization (concepts of the serving layer, not of the archive).

**Event**:
An immutable fact in the archive's log, emitted only after a command was validated; the write side's source of truth.

**Version** (Resource Version, Representation Version):
A read-side projection — "the n-th publication event for this Resource" — cited as `…/v{n}`; not a stored class of its own.
_Avoid_: treating a Version as a first-class archive entity; the active-research platform's fine-grained edit history (not preserved as Versions).

### Identifiers

**Internal IRI**:
A non-dereferenceable, DaSCH-controlled identifier in URN form (`urn:dsp:{type}:{uuid}`), stable within a system but not promised across system migrations.
_Avoid_: internal URL, the retired HTTPS form.

**ARK**:
The single long-term-stable public identifier, minted per persistent-identity entity (Resource, Representation, Project), not per Version; resolved by the ARK resolver and redirected to the Access Area.
_Avoid_: DOI, Handle, PURL, permalink.

### The edge

**Intent protocol**:
The sole producer-facing surface of the archive: a producer registers an **Ingest Intent** with a declared manifest (HTTPS + mTLS), uploads the files to a quarantine bucket under a presigned grant, and completes; the archive verifies the manifest, validates (SHACL against DAO always; virus scan and format identification on bitstream-bearing packages; the DepositAgreement always) and only then commits. Deposit-Area producers are trusted but not privileged: same protocol, same gate, no shortcut.
_Avoid_: upload API, Ingest Service (a retired design in which a separate service ran the gate).

**Ingest Intent**:
One submission session under the intent protocol; its `intent_id` is the ingest-wide idempotency key; terminal states archived, abandoned, expired.
_Avoid_: upload session, transaction.

**SIP**:
The Submission Information Package — the wire format crossing from a producer into the archive; not a stored entity (the Deposition is).

**Data product**:
What the archive publishes outward for the Access Area: immutable snapshots and deltas in object storage, announced by pointer messages on a message bus, written before they are announced. Every read-side service is a disposable projection over them, rebuildable from snapshot plus replay.
_Avoid_: event stream (no fat events cross the boundary), export.

## Relationships

- A **Project** contains zero or more **Resources** and zero or more **Representations**.
- A **Resource** version pins specific **Representation** versions; a pinned Representation version can never be deleted.
- A **Deposition** belongs to exactly one **Project** and at least one depositing agent; it produces one or more Resource and Representation events.
- A **PreservationAction** spans zero or one **Project** and is initiated by the archive itself.
- An **ARK** binds to one persistent-identity **Internal IRI**; the binding may change across migrations, the ARK string never does.
- The archive derives a **Service File** from one or more **Preservation Files** under a derivation rule and publishes it as a data product; the Access Area's `media` capability serves it through Vitrinli, whole or rendered. (The Deposit Area derives its own Service Files from Originals before archiving; those never enter the archive.)

## Boundary commitments

- Preservation storage is exclusive to this area: no other context reaches the sealed store, the log, or Preservation File bytes. Consumers receive data products.
- The intent protocol is the only way in. Isolation from possibly malicious content is by bucket separation (quarantine versus sealed stores) and validation before promotion.
- One read-side integration pattern everywhere: pointers on the bus, immutable payloads in object storage, no direct queries by consumers, and staleness is legitimate — a consumer serves what it has while the archive is unavailable.
- DAO is this area's language; other areas translate at the seam.
- ARKs are the only long-term-stable identifiers; internal IRIs and presentation URLs may change.

## Example dialogue

> **Dev:** "A depositor uploads through the editor. When does a **Deposition** exist?"
> **Domain expert:** "Not until validation passes. The editor registers an **Ingest Intent**, uploads to the quarantine bucket, completes. The verified completion is the durable acknowledgement. Only when the scan and the validation gates pass does the archive commit the Deposition and the per-entity events."

> **Dev:** "Can Vitrinli fetch a **Preservation File** directly from the sealed store?"
> **Domain expert:** "No. The archive's derivation workers produce **Service Files** and publish them as **data products**; the Access Area's `media` capability reads those and serves them through Vitrinli. User read paths never touch Preservation Files."

## Flagged ambiguities

- **"Record"**: not a domain term here; the archived units are **Resources** and **Representations**. Resolution: the archive never adopts the word; see the root `CONTEXT.md`.
- **"Ingest"**: the OAIS functional entity that lives in this area, versus the retired names "Ingest Service" and "Ingest Area". Resolution: "Ingest" alone means the functional entity; the producer side is the **Deposit Area** (ADR-0002).
- **"Master"** (Archival Master, Service Master) is retired in favour of Preservation File and Service File.
- **"AIP / SIP / DIP"** are OAIS package shapes at boundaries, not stored entities; the archive has no classes for them.
