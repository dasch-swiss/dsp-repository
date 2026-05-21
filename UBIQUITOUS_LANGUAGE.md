# Ubiquitous Language

Cross-context glossary for `dsp-repository`. The canonical terms used across the bounded contexts that implement the **Trusted Repository (OAIS-based)** domain. For per-context detail (relationships, internal structure, boundary commitments, design narrative) see each context's `CONTEXT.md` and design doc — currently `modules/archive/CONTEXT.md` and `modules/archive/dao-discovery.md`. For the map of how contexts relate, see `CONTEXT-MAP.md` at the repo root.

## Domain framing

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Trusted Repository** | The domain implemented by `dsp-repository`: long-term preservation and trustworthy dissemination of research data, governed by OAIS (CCSDS 650.0-M-3) | Repository, archive system, preservation platform |
| **Subdomain** | A problem-side slice of the domain; corresponds to an OAIS functional entity or a DaSCH-specific concern | — |
| **Bounded context** | A solution-side language boundary; implements one or more subdomains | — |

## OAIS package shapes

Boundary wire-formats, not durable entities. They do not have DAO classes.

| Term | Definition | Aliases to avoid |
|---|---|---|
| **SIP** | Submission Information Package — wire format crossing the Producer → Archive boundary | submission, payload |
| **AIP** | Archival Information Package — what is preserved; constituted from `dao:Representation`s + descriptive metadata + provenance events | archive package |
| **DIP** | Dissemination Information Package — produced by an Access Area subdomain in response to a Consumer request | response, output |

## Bounded contexts

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Archive** | Bounded context holding the preservation-grade record. Covers OAIS Ingest, Archival Storage, and supporting functional entities | — |
| **Access Area** | Bounded context producing DIPs (OAIS Access entity); implemented as N federated subscriber services, one per subdomain | — |
| **ARK Resolver** | Bounded context for persistent identifiers; its own event store | — |
| **RDU-Tooling** | Producer-side data preparation; issues Commands to the Archive | — |
| **VRE** | Virtual Research Environment; active-research data platform; external Producer; out of scope of `dsp-repository` | — |

## Archive context — first-class entities

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Resource** | A coherent set of content reasonably described as a unit; conceptually `premis:IntellectualEntity`. DAO is conceived as a simplification of `knora-base`, and `dao:Resource` corresponds (after ingest normalisation) to a `knora-base:Resource` instance with the VRE runtime concerns stripped. Versions at deliberate publication events. See decision 9 (amended 2026-05-18) | Record, Object, Item, IntellectualEntity (retired DaSCH term — kept only in PREMIS-vocabulary discussions) |
| **Representation** | The preservation-grade bundle: one or more Preservation Files plus Representation-level metadata. Versions over time | Asset, File, Archival Master |
| **Project** | A research project archived in DSP, identified by a 4-character shortcode | Dataset, Collection |
| **Agent** | Person, organisation, or software acting as creator, contributor, depositor, maintainer, preservation-action runner | — |
| **Deposition** | Producer-induced unit of ingest; gated by a DepositAgreement | Submission, Ingest, Batch |
| **DepositAgreement** | Producer ↔ Archive contract: identity, accepted formats, retention terms, embargo/access defaults | Submission Agreement (OAIS term — use the OAIS term only in OAIS-vocabulary discussions) |
| **PreservationAction** | Archive-induced unit of change; gated by internal preservation policy rather than a DepositAgreement | PreservationEvent, Maintenance, Curation |
| **AccessPolicy** | First-class DAO entity carrying an opaque RDF policy blob, referenced from Resource/Rep Versions via `dao:hasAccessPolicy` for the COAR `restricted_access` case (decision 45, 2026-05-18). Has its own URN (`urn:dsp:access-policy:{uuid}`), OCFL Object, and lifecycle events. Internal policy ontology deferred to Q19 | AccessRule (used only for the change-event name); Permission, Authorization (γ-layer concepts, not DAO) |
| **Event** | The event-sourcing primitive; an immutable fact emitted only after a Command has been validated | — |

## Preservation chain roles (shared cross-context vocabulary)

Three role-based labels distinguished by **purpose** in the preservation chain — not by format, location, or source provenance. Originated in SIPI's IIIF Server vocabulary; promoted to cross-context Published Language (decision 30).

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Preservation File** | Long-term bit-level preservation; file bytes inside a `dao:Representation`. Authoritative; WORM. Content-addressed by hash. Owned by Archive context. **Not a separate DAO class** — described as blank-node properties on the Representation (`dao:hasFile`), addressed by `dao:filename` within the parent Rep Version | Archival Master, `dao:File` (no such class) |
| **Service File** | Mezzanine baseline derived from Preservation File(s) under a derivation rule; regenerable; carries no preservation commitment. Owned by Access Area context | Service Master |
| **Access File** | End-user delivery payload generated on demand from a Service File plus request parameters; ephemeral. Owned by the Access Area subdomain that serves the request | — |

## Versioning (read-side, not DAO classes)

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Resource Version** | Read-side projection: "the n-th `ResourcePublished` event for this Resource." Cited as `.../v{n}` | IE Version (retired 2026-05-18) |
| **Representation Version** | Read-side projection: "the n-th `RepresentationCreated` event for this Representation." Cited as `.../v{n}` | — |

## Identifiers

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Internal IRI** | DaSCH-controlled identifier in URN form `urn:dsp:{type}:{uuid}` (e.g., `urn:dsp:resource:...`, `urn:dsp:rep:...`, `urn:dsp:access-policy:...`). Not dereferenceable; never leaves the system. Stable within a system; **not** promised across system migrations. Earlier HTTPS form (`https://archive.dasch.swiss/{type}/{uuid}`) is retired per decision 12 amendment (2026-05-15) | HTTPS URI form (retired); "internal URL"; `urn:dsp:ie:...` (retired per decision 9 amendment 2026-05-18) |
| **ARK** | The single long-term-stable public identifier; minted per persistent-identity entity, not per Version. Resolved by the ARK Resolver context | DOI, Handle, PURL, permalink |

## Archive internal structure

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Ingest Service** | **Sole producer-facing surface** of the Archive (decision 47). Receives all SIP submissions over gRPC from RDU-Tooling, the self-service preservation frontend, and any future Producer-side tool. Runs SIP-shape-appropriate validation (SHACL always; ClamAV + format ID on bitstream-bearing SIPs; `DepositAgreement` always); on success commits via Archive Storage's internal `CommandAPI`. Holds no event-log or OCFL write path. Earlier name "Ingest Area" retired (it framed Ingest as a deployment rather than a sole producer-facing surface) | OAIS *Ingest* (the broader functional-entity concept), `dao:Deposition` (the durable record of a successful Submission), "Ingest Area" (retired 2026-05-21) |
| **Archive Storage** | Sealed core deployment of the Archive: OCFL store + event log + internal gRPC `CommandAPI` / `QueryAPI` / `EventStream` (decision 47). Accepts mTLS-authenticated principals only (Ingest Service; preservation admin tooling). Self-defends with re-validation of every command regardless of source | "the rest of the Archive" (loose), "Archival Storage" (OAIS functional-entity name; correct usage but more abstract) |
| **Self-service preservation frontend** | Browser-based Rust SSR service for RDU staff + external users to create / edit project-level metadata and submit Depositions. Likely consolidates a "Metadata Editor" and a "SIP submission GUI" into one frontend. Submits SIPs to Ingest Service over gRPC. Bounded-context status open (Q21) | Metadata Editor (one likely component), SIP submission GUI (the other) |

## Access Area subdomains (DIP shapes)

Each subdomain produces one OAIS DIP shape. Each is implemented as an independent subscriber service (decision 32).

| Term | Definition | Aliases to avoid |
|---|---|---|
| **IIIF** | Image dissemination subdomain (DIP shape: IIIF Manifest + image tiles per the IIIF Image/Presentation API). Implemented by SIPI | — |
| **HTML / Web Discovery** | Discovery and presentation subdomain (DIP shape: rendered HTML pages with faceting and navigation). Implemented by DPE | — |
| **Custom Presentation** | Project-specific presentation subdomain (DIP shape: project-tailored HTML/CSS). Implemented by CPE (future) | — |
| **Asset / Download** | File-bytes download subdomain (DIP shape: HTTP byte stream with Range support). Implemented by an asset server (future) | — |
| **SPARQL** | Graph-query subdomain (DIP shape: SPARQL query results over a denormalised projection). Implemented by a SPARQL endpoint (future) | — |

## Subscribers and replay

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Subscriber** | An Access Area subdomain service that subscribes to the Archive's Events SSE feed and projects events into its own storage | — |
| **Last-Event-ID** | The SSE resume cursor a Subscriber owns; bookmarks how far the Subscriber has projected | — |
| **Snapshot** | Subscriber-side serialised state, written periodically, used to restart fast without replaying from genesis (α bootstrap strategy) | — |
| **Cold replay** | Bringing up a new Subscriber that has never run before, by replaying the full event history (δ bootstrap strategy) | — |

## Operational artifacts (Archive internal)

| Term | Definition | Aliases to avoid |
|---|---|---|
| **OCFL Object** | Oxford Common File Layout Object; versioned, self-describing, hash-verified directory. The canonical storage unit inside the Archive | — |
| **Event-index** | Append-only chronological NDJSON in the Archive's cache; provides reverse-lookup from event-id to containing OCFL Object; rebuildable | — |
| **DAO** | The DaSCH Archival Ontology (OWL + SHACL); the Archive context's ubiquitous language as machine-readable schema | — |

## Relationships

- A **Project** contains zero-or-more **Resources** and zero-or-more **Representations**.
- A **Resource Version** pins specific **Representation Versions** (preservation commitment: a pinned Representation Version cannot be deleted).
- A **Representation** contains one or more **Preservation Files**.
- A **Deposition** belongs to exactly one **Project** and at least one **Agent** (the depositor).
- A **PreservationAction** is initiated by an Archive-system **Agent**; may span zero-or-one **Project** (cross-project actions are allowed).
- An **ARK** binds to a persistent-identity **Internal IRI**; bindings are mutable across system migrations, ARK strings are not.
- A **Service File** is derived from one or more **Preservation Files** under a derivation rule; the rule and the storage belong to the Access Area subdomain that owns the derivation.
- An **Access File** is generated on demand from a **Service File** plus request parameters; not stored.

## Example dialogue

> **Dev:** "A Producer uploads a SIP through RDU-Tooling. When does a Deposition exist?"
> **Domain expert:** "Not until the Ingest Service validates the SIP successfully. Until then it's just a SIP sitting in the Ingest Service being checked. Only when validation passes does the Ingest Service commit a `dao:Deposition` via Archive Storage's internal `CommandAPI`, which emits `DepositionAccepted` followed by the per-entity events for the Resources and Representations the deposit produced."

> **Dev:** "We're adopting a new IIIF profile. Does that bump Representation Versions?"
> **Domain expert:** "No. Service Files are downstream of Representations; they have no versioning identity of their own. The Representation Versions are unchanged. The IIIF Subscriber re-derives its Service Files from the same Preservation Files under the new derivation rule — that's an Access Area concern, not an Archive event."

> **Dev:** "Can SIPI fetch a Preservation File directly from OCFL?"
> **Domain expert:** "No. OCFL is internal to the Archive bounded context. SIPI — as the IIIF subdomain's Subscriber — pulls bytes through the Archive's Binary retrieval API by multihash. That contract is the only way bytes leave the Archive."

> **Dev:** "If a SPARQL Subscriber falls a million events behind, what happens?"
> **Domain expert:** "Its `Last-Event-ID` is stale. When it reconnects to the SSE feed, the Archive serves it the missing events from the event-index in chronological order, then it transitions to the live tail. If it falls catastrophically behind — beyond what its snapshots can recover from — the operator's recourse is the (γ) peer-clone or (δ) cold-replay strategy."

## Flagged ambiguities

- **"Record"** has three distinct meanings across the platform: a flat projection in DPE (`dpe-core`), an OAI-PMH metadata blob (external standard term), and (in the Archive context) **not a domain term at all** — the archived units are **Resources** and **Representations**. The DPE-internal use can stay; cross-context translation happens at the read-side projector that feeds DPE. See `modules/archive/CONTEXT.md` → Flagged ambiguities.
- **"Version"** is overloaded across DAO (read-side projection over deliberate publication events), PREMIS (no single concept; uses `objectIdentifier` + event chains), and VRE (fine-grained edit history that DAO deliberately does *not* preserve as Versions). Only deliberate publication events become Versions on the DAO read side.
- **"AIP / SIP / DIP"** belong to OAIS vocabulary — package shapes at boundaries, not durable entities. DAO does not have AIP / SIP / DIP classes. The AIP is *constituted* from `dao:Representation` + descriptive metadata + provenance events; the SIP arrives at the Ingest Service (decision 47); the DIP is constructed on demand by an Access Area subdomain.
- **"Master"** as a label for preservation-chain artifacts is **retired**. Use the three-tier role vocabulary: Preservation File / Service File / Access File. "Archival Master" and "Service Master" should not appear in new prose. See decision 30 in `modules/archive/dao-discovery.md`.
- **"Ingest Service" vs "Access Area"** parallel linguistically, but they are not architectural peers: Access Area is a separate bounded context with N subscriber services; Ingest Service is a deployment subdomain *inside* the Archive bounded context (the sole producer-facing surface, per decision 47). The naming symmetry is incidental. The earlier label "Ingest Area" is retired 2026-05-21 (decision 47) — it framed Ingest as a deployment rather than a sole producer-facing surface; "Ingest Service" is the canonical term.
- **"Subdomain"** is overloaded: at the top level it refers to OAIS-aligned slices of the Trusted Repository domain (Ingest, Access, …); within a bounded context it can also refer to nested sub-slices (e.g., the Access Area's DIP-shape subdomains). Context disambiguates.
