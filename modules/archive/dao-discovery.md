# DaSCH Archival Ontology (DAO) — Working Decision Log

*Working document, updated after each interview decision. Each section records what we decided and why; decisions can be revised but the revision is logged, not silently overwritten.*

**Status:** in-progress design conversation
**Last updated:** 2026-05-21 — **Q20 RESOLVED by decision 47.** Producer-facing surface unification: **Ingest Service is the sole producer-facing surface of the Archive**; all Producer submissions (RDU-Tooling, self-service preservation frontend per Q21, future tools) go through Ingest as SIPs over gRPC; SIPs may be content-bearing, metadata-only, or mixed; Ingest runs SHACL + ClamAV (on bitstream-bearing SIPs) + format ID + `DepositAgreement` enforcement; on success commits via Archive Storage's internal `CommandAPI`. **Wire transport is gRPC across all surfaces** — replaces decision 31's SSE on the Events feed with gRPC server-streaming `EventStream`; replaces decision 31's HTTP Commands/Binary with gRPC unary + streaming. **`CommandAPI` moves to internal-only**, accepting mTLS-authenticated principals (Ingest Service; DaSCH-internal preservation admin tooling). **Archive Storage self-defends** by re-validating every command regardless of source. **No SIP-as-submitted preservation** — SIPs held on Ingest for an operational backup window only (OAIS does not require SIP preservation; decision 24 events-with-full-snapshots covers replay). Decision 47 amends decision 31's transport story (HTTP → gRPC; SSE → gRPC server-streaming; CommandAPI: public → internal-only). The decision-31 *vocabulary* of three public APIs (Commands, Events, Binary) survives as conceptual framing; the wire and the publicness changed. **Q21 parked**: self-service preservation frontend (consolidates Metadata Editor and SIP submission GUI; bounded-context status open; phase-1 read path direct gRPC `QueryAPI` → phase-2 EventStream projection for offline-edit; replaces JSON-in-Docker pattern for DPE project metadata). **Q22 parked**: SIP wire-format sub-questions (gRPC envelope shape leaning toward opaque RDF payload + bitstream chunks; external-Producer translation gateway via BagIt deferred until external access is real; mixed-content SIPs handled uniformly by Ingest). CONTEXT-MAP.md and CONTEXT.md updated inline; "Ingest Area" → "Ingest Service" rename. §8.1 evidence index extended with decision-47 row (CTS R02 + R08 + R11 + R14; nestor §3.1 + §6.1 + §13; ISO 16363 §3.5.1 + §4.1.1 + §4.1.1.3). **No design thread in flight at end of session**; §9.6 names the next thread explicitly: **DAO-shape from VRE export (Q10 + Q16 combined)**, anchored against a real VRE project export — Ivan flagged this at session close. Prior 2026-05-18: **Q12 fully RESOLVED.** Q12.1 (γ delivery-layer enforcement, 2026-05-15); Q12.2.1 (decision 44 — explicit per-Version `dao:accessRights`, no propagation in DAO; composition is read-side γ-projection concern); Q12.2.2 (decision 45 — `dao:AccessPolicy` as first-class entity with opaque policy content); **Q12.2.3 (decision 46 — DAO event log carries policy lifecycle via `AccessPolicyCreated` / `AccessPolicyRetired` / `AccessRuleChanged`; γ-per-access logs are operational telemetry, not preservation evidence; analogous to decision 41's Grafana-vs-event-log split)**. Downstream open: Q19 (policy ontology selection for `dao:AccessPolicy.policyContent`, parked, research-needed). §8.1 evidence index, §7 Q12 entry, §9.4 priority list, §9.6 settled-architecture table, §9.6 top-priorities section, §9.2 session log all updated. **No design thread in flight at end of session**; §9.6 lays out seven candidate next moves with a recommended ordering (Q18 next + parallel research dispatches for Q16 / Q17 / Q19). Earlier 2026-05-18: **Q12.2 sub-question 2 RESOLVED by decision 45** (`dao:AccessPolicy` as a first-class DAO entity with opaque RDF policy content; new top-level class `dao:AccessPolicy`; new URN type `urn:dsp:access-policy:{uuid}`; new events `AccessPolicyCreated` / `AccessPolicyRetired`; new property `dao:hasAccessPolicy` on Resource / Rep Versions for the COAR `restricted_access` case). Pattern parallels `dao:DepositAgreement` (decision 18) and Q16 option (a) for project ontologies — now a recurring DAO idiom of "typed reference to a preserved artefact whose internal schema is opaque to DAO." §5.1 event vocabulary, §8.1 evidence index, CONTEXT.md, UBIQUITOUS_LANGUAGE.md updated. **Q19 parked**: policy ontology selection for `dao:AccessPolicy.policyContent` — five candidates (ODRL, PREMIS rights, XACML, DC terms, DaSCH-custom); research dispatch needed. **Earlier 2026-05-18: Q12.2 sub-question 1 RESOLVED by decision 44** (explicit per-Version `dao:accessRights` on `dao:Resource` and `dao:Representation`; **no propagation, no inheritance, no composition in DAO**; composition rules — most-restrictive-wins, parent/child propagation, the contemplated "two access spaces" partitioning — are read-side γ-projection concerns). Rationale (Ivan, four reasons): DAO is the write-side schema and computation belongs to projections; future Producers other than VRE may not produce `kb:isPartOf` hierarchies; future multi-parent / deep-chain models break propagation semantically; γ-projection design needs full evolutionary freedom (e.g., the open-vs-authorized two-space partitioning). Two-level positional scheme (Resource + Representation); File-level deferred as additive (decision 33 makes it additive without restructuring). §8.1 evidence index updated (CTS R13; nestor §11.2 / §6.3; ISO 16363 §4.5 / §3.5.1). **Q18 parked**: Compound Resources and the `kb:isPartOf` + `kb:seqnum` pair in DAO. The pairing is tacit in `dsp-api` code, not declared in `knora-base` ontology. DAO should make the structural commitment explicit. Five sub-dimensions parked. **Load-bearing for Q12.2's γ-projection**: decision 44 made composition a read-side concern, but the projection needs queryable `isPartOf` relationships to *do* propagation. Earlier 2026-05-18 four non-blocking changes ahead of resuming Q12.2. **(1) Q16 parked in §7**: project-authored ontologies and directly-used `knora-base` classes/properties may need to be preserved as interpretation-essential metadata ("the metadata of the data" — CTS R10 / nestor §10.3 / ISO 16363 §4.2.4). Reopens part of §4.1 / §4.3 / decision 6 (amended) / decision 10; forward-references added inline. **(2) Decision 9 amended**: `dao:IntellectualEntity` renamed to **`dao:Resource`** to make explicit that DAO is conceived as a simplification of `knora-base`. Event names lose the redundant `Version` infix: `IntellectualEntityVersionPublished` → `ResourcePublished`, `RepresentationVersionCreated` → `RepresentationCreated`. URN prefix `urn:dsp:ie:{uuid}` → `urn:dsp:resource:{uuid}`. Read-side URL path `/ie/{uuid}/v{n}` → `/resource/{uuid}/v{n}`. Abbreviation "IE" retired. Collision with `knora-base:Resource` managed by namespace-prefix prose discipline. Decision 9 amendment is a documented deviation from decision 27's verbatim-PREMIS-naming default. CONTEXT.md and UBIQUITOUS_LANGUAGE.md updated in lockstep. **(3) §2.2 Internal identifiers rewrite**: a pre-existing inconsistency surfaced during the rename pass — §2.2 still described HTTPS-form (`https://archive.dasch.swiss/{type}/{uuid}`) as the internal IRI, contradicting the 2026-05-15 decision-12 amendment to URN form. §2.2 rewritten to make URN canonical, with the full URN type list enumerated, the read-side reframed as per-Access-Area-subdomain URLs (DPE / IIIF / SIPI / etc., not a single archive-namespace HTTPS URL), and the "why URN over HTTPS" rationale spelled out. §2.7 stability layers and §3.4 worked examples updated to match. **(4) Q17 parked in §7**: ARK Resolver behaviour model — pre-publication reservation, explicit URN ↔ ARK binding events, content-negotiation-driven resolution from URN to subdomain-specific URLs, and move-out-of-DaSCH custody transfer (new `CustodyTransferred` event on the Archive side). Subsumes existing Q11. Research-needed workstream; refines decision 13 + §2.3 – §2.7. Must land before CTS application (R03 + R12). Prior 2026-05-16 entry: Q15 RESOLVED by decision 43. DaSCH commits to **uniform CTS Levels A + C + D across all projects, all Depositions, all preserved content** — institutional invariant, no per-Agreement / per-Deposition / per-Resource variation. Consequences: `dao:DepositAgreement` does NOT carry a preservation-level property; no `premis:preservationLevel` overrides in event payloads; no `CurationLevelChanged` event. §1a gained a preservation-commitment paragraph; §8.1 evidence index + coverage-check updated; Q15 in §7 marked RESOLVED. Prior 2026-05-16 entry: Curation & Preservation Levels Position Paper added to `standards/` (CCSDS position paper, Z / D / C / A taxonomy; CTS A required for in-scope). Prior 2026-05-16 entry: CTS extension. Added CoreTrustSeal Requirements 2026-2028 v01.00 (+ Extended Guidance + Glossary) to `standards/`; §8.1 Evidence index CTS column **remapped from working-memory 2023-2025 numbering to actual 2026-2028 numbering** (key shifts: storage R9→R14, preservation plan R10→R09, technical quality R11→R10, workflows R12→R11, identification R13→R12, reuse R14→R13); coverage-check table gained two rows (Legal & Ethical R04; Reuse-side Designated-Community engagement R13); §9.6 fifth-priority struck through as DONE. Prior 2026-05-16 entries: (i) evidence-index pass — §8.1 created mapping 20 decisions to three-tier citations + coverage-check table; (ii) certification-pyramid framing — nestor added, decision 42 records the pyramid, geographic-redundancy commitment made architecturally explicit in §3.7 Phase-2 paragraph; (iii) ISO 16363 + ISO 16919 added to `standards/`, decision 40 citations corrected to actual CCSDS 652.0-M-2 section numbers. Prior session entry (2026-05-15): sizing pass + §3.6 / §3.7 prose rewrite (decisions 40, 41; decisions 22 and 25 amended; numerical baseline re-locked at 100 deposits/year, 200K events/deposit, ~220M events/year at year 20 fixity-dominated, ~1 PB at year 20; storage prose aligned with decisions 36-39 two-substrate model). Prior session entry (2026-05-14/15): decisions 29-39 + decision 12 URN amendment.

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
- **Archive** (OAIS Archive Component; covers OAIS Ingest, Archival Storage, and the supporting functional entities Preservation Planning, Data Management, Administration) — ingest, validation, audit; holds the preservation-grade record (`dao:Representation`s, each containing one or more **Preservation Files**). **Working assumption: self-built (per decision 17, revised).** Storage architecture: OCFL on filesystem as source of truth; Redb as read-side cache (decision 38; §3.7). Deployed as two services within the same bounded context: an **Ingest Service** (sole producer-facing surface, per decision 47; async SIP submission over gRPC + validation gate — SHACL against DAO, `DepositAgreement` enforcement, ClamAV + format ID on bitstream-bearing SIPs; on success commits via Archive Storage's internal `CommandAPI`, which emits `DepositionAccepted` — the OAIS *Ingest* entity) and **Archive Storage** (the sealed core: event log, OCFL, internal gRPC `CommandAPI` / `QueryAPI` / `EventStream`; accepts mTLS-authenticated principals only — the OAIS *Archival Storage* entity). Split is driven by **trust-boundary (security)** plus bandwidth, failure isolation, and "as long as it takes" processing for large Depositions; both speak DAO directly. Working name "Ingest Area" retired (decision 47).
- **Access Area** (OAIS *Access* entity) — produces DIPs for Consumers. Materializes **Service Files** (and Service Projections for non-file consumers) per subdomain; serves **Access Files** on demand. Subscribes to Archive events; push-from-Archive (resolved 2026-05-13, §9.3). Subdomains: **IIIF** (SIPI), **HTML / Web Discovery** (DPE), **Custom Presentation** (CPE), **Asset / Download**, **SPARQL**. Each subdomain corresponds to one OAIS DIP shape.

Two committed architectural patterns shape DAO's design space:

- **Domain Driven Design** — bounded contexts, ubiquitous language, aggregates. **DAO is the ubiquitous language of the Archive context.** The Producer-side contexts (RDU-Tooling, VRE) and the Access Area context (with its subdomains listed above) have their own internal vocabularies. **DAO terms appear at boundaries** (submissions arriving at the Archive, projection events leaving the Archive) — the lingua franca of context seams, not the universal language. (Anti-corruption layer not required under the self-built working assumption.) **Subdomain ≠ bounded context**: a subdomain is a problem-side slice; a bounded context is a solution-side language boundary. The Access Area's subdomains are implemented in separate codebases (`modules/dpe/`, future SIPI, etc.) but share the Access Area's parent ubiquitous language plus subdomain-specific extensions.
- **Event Sourcing** — events are the source of truth in the Archive; **Service Files and Service Projections** in the Access Area are derived (replayable) views.

**Project metadata format is decided**: RDF/Turtle with SHACL validation. DAO is one such ontology (the canonical archival one). The Archive treats project-level supplementary metadata as black-box storage.

**CoreTrustSeal is the certification target.** Several DAO design choices (immutable event log, fixity events, format migration provenance, no in-place migration) directly produce CoreTrustSeal evidence. Where this linkage applies, it is noted. See `standards/CoreTrustSeal-Requirements-2026-2028_v01.00.md` for the substantive requirements; decision 42 records the certification-pyramid framing (CTS → nestor → ISO 16363).

**Preservation-level commitment: uniform A + C + D across all projects** (decision 43). DaSCH's mandate commits the Archive to all three CTS Curation & Preservation Levels — **D** Deposit Compliance + **C** Initial Curation + **A** Active Preservation (cumulative, per the *CTS Curation & Preservation Levels Position Paper v3.0*, 2024) — for **every project, every Deposition, every Resource / Representation / File**. No per-Agreement, per-Deposition, or per-Resource variation. **Consequence for DAO**: the level is not a tunable; it is institutional invariant. `dao:DepositAgreement` does not carry a level property; events do not carry `premis:preservationLevel` overrides; no `CurationLevelChanged` event exists. If DaSCH ever introduces a tiered offering (e.g., Z-level publication-only for non-SNSF projects), decision 43 must be revisited.

### Scope clarification: DAO governs the Archive, not the Access Area

DAO is the schema of what is **preserved in the Archive** (`dao:Representation`s, the **Preservation Files** they contain, and their descriptive/preservation metadata). It is **not** the schema of **Service Files / Service Projections** in the Access Area, nor of presentation views produced by Access Area subdomains (HTML/Web Discovery, Custom Presentation, IIIF Manifests, etc.).

- **`dao:Representation`s** (containing one or more Preservation Files) conform to DAO. Versioned. WORM. Source of truth.
- **Service Files / Service Projections** are derived projections of Representation Versions, shaped for specific Access Area subdomains (pyramidal TIFF for IIIF, search indexes for HTML/Web Discovery, denormalised RDF for SPARQL, etc.). They are **regenerable from Preservation Files + derivation rule**. They do not carry their own versioning identity in DAO.
- **Presentation views** produced by HTML/Web Discovery (DPE), Custom Presentation (CPE), and other Access Area subdomains are presentation, not preservation. Subdomain-specific customisations live outside DAO.

---

## 1b. Commands vs. Events (CQRS/ES) — and the write/read separation

The Repository follows a **Command/Event (CQRS-ES)** pattern. The distinction is foundational and affects every other decision in this document.

- **Command** = an *intent* to change state. Issued by a producer (RDU-Tooling, ARK Resolver client, an internal preservation-action runner, etc.). Carries everything needed to attempt the change. Can be **rejected**. Examples: `SubmitDeposition`, `MintArk`, `ReserveArk`, `BindArk`, `MigrateRepresentationFormat`.
- **Event** = a *fact* that something has happened. Emitted by the Archive (or by other event-sourced services like the ARK Resolver) **only after** a Command has been validated and accepted. Immutable. Examples: `ResourcePublished`, `RepresentationCreated`, `ArkMinted`, `ArkBound`, `ArkReserved`, `FormatMigrated`.
- **Validation happens at command time.** Collision detection, format checks, schema validation, authorization checks, and any other "this is allowed" logic runs against the Command. The event log thus contains only valid history; Events are never rejected.

### Write side vs. read side

CQRS separates the canonical record (write side) from materialized views for querying (read side):

- **Write side (the Archive's source of truth):** the **event log**, plus the persistent-identity nodes that events refer to (`dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Deposition`, `dao:DepositAgreement`). The write side does **not** materialize Versions as queryable entities. Versions exist as facts encoded in events.
- **Read side (one or more projections / read stores):** materializes whatever views are useful for queries and presentation. One key projection materializes **Version nodes** for each Resource and Representation by folding their publish events. The read store carries derived properties such as `versionNumber`, `publishedAt`, snapshot of metadata, list of pinned Representation Versions, `isCurrentVersion`, etc.

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

- **Resource** — described content with descriptive identity. RDF data conforming to DAO. Has a stable internal IRI. Versions over time. Conceptually `premis:IntellectualEntity` ("a coherent set of content reasonably described as a unit"). **DAO is framed as a simplification of `knora-base`**, and `dao:Resource` is the simplified counterpart of `knora-base:Resource` (the VRE runtime concerns — `kb:isDeleted`, `kb:hasPermissions`, `kb:attachedToUser`, lifecycle dates — are stripped at ingest; the property/value assertions and any directly-used `kb:` properties survive, pending Q16 in §7). The earlier `dao:IntellectualEntity` framing (decision 9, 2026-05-12) is retired by decision 9 amendment (2026-05-18); prose discipline is to always namespace-prefix in writing (`dao:Resource` vs `knora-base:Resource`).
- **Representation** (Rep) — the preservation-grade bundle: a set of one or more **Preservation Files** (PREMIS allows multi-file Representations) plus Representation-level metadata (license, authorship, copyright, technical metadata such as filename, MIME type, originalFilename, originalMimeType). Has a stable internal IRI. Versions over time. Many-to-many with Resources across Versions. **Service Files** (Access Area mezzanines) are derived from Preservation Files and are not themselves Representations in DAO terms. Replaces what we initially called "Asset" (which had a different meaning in the VRE); the informal label "Archival Master" is retired in favour of the three-tier role vocabulary (decision 30).
- **Ontology** — schema. Single archival ontology only (see §4).

### 2.2 Internal identifiers

Two layers of internal identifier — the write-side URN inside DAO, and the read-side HTTPS URL served by Access Area subdomains.

**Write-side (DAO graph IRIs) — URN form, not dereferenceable.**

- Persistent-identity entities (`dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:DepositAgreement`, `dao:Deposition`, `dao:PreservationAction`) carry **URN IRIs** in the DaSCH-controlled `urn:dsp:` scheme: `urn:dsp:{type}:{uuid}`. Examples: `urn:dsp:resource:01234567-89ab-...`, `urn:dsp:rep:...`, `urn:dsp:project:...`. Event-log segments are similarly identified as `urn:dsp:event-segment:{period}`.
- URNs are **not dereferenceable** — they exist purely for cross-event references inside the system. No HTTP resolution is implied or supported. URNs never leave the Archive boundary; nothing outside the Archive context should construct or consume them.
- Identifiers use **UUIDv7** — distributed-allocation safe, time-ordered (sortable, indexable), 128 bits, opaque to humans.
- The persistent-identity URN (without any Version suffix) lives in DAO. It is the URI that events refer to.
- Event identity: each event carries a JSON-LD `@id` plus stream-position fields (`stream_id`, `stream_version`, `global_offset`) per decision 37. See §5 and §3.6 for event identity in detail.

**Read-side (Access Area projection URLs) — HTTPS, dereferenceable.**

- Access Area subdomains serve content via **HTTPS URLs** scoped to each subdomain's deployment, with the Version suffix appended where applicable. Examples: `https://dpe.dasch.swiss/resource/{uuid}/v{n}` (HTML/Web Discovery, owned by DPE), `https://iiif.dasch.swiss/{shortcode}/{rep-uuid}/manifest.json` (IIIF subdomain, owned by SIPI), and analogous URLs for asset/SPARQL/custom-presentation subdomains.
- These URLs are **part of each read store's contract**, not DAO IRIs. They are dereferenceable so Linked Data conventions work; the URL → content binding is honoured as long as the event log is replayable. If a read store is rebuilt, replay regenerates the same URLs.

**Why URN over HTTPS for internal identity.** Fedora 6 sets the precedent (`info:fedora/...` URNs). HTTP-resolvability is not needed internally; tying internal identity to a domain (`archive.dasch.swiss`) makes the data brittle to rebrands and infrastructure changes. URN is the standard scheme for non-dereferenceable persistent identifiers (RFC 8141). Decision 12 amendment (2026-05-15) made this canonical; the earlier `https://archive.dasch.swiss/{type}/{uuid}` HTTPS form is retired.

**Long-term stability commitment.** URN internal IRIs and read-side HTTPS URLs are not promised to outlive a system migration. Only **ARKs** are. See §2.3.

### 2.3 ARKs (separate identifier system, separate bounded context)

**Only ARKs are promised to be stable long-term.** Internal Archive IRIs, read-side URLs, and DPE URLs are operational identifiers; they may change across system migrations. ARKs are the long-term commitment.

**Per-entity ARK minting.** An ARK is minted **once per archival entity that needs public citation** — Resource, Representation, Project, and any future class that warrants its own citable identity. **Versions are not separately ARK'd.** A specific Version is denoted by appending `/v{n}` (or, for VRE-era ARKs, the timestamp suffix) to the entity's ARK string. The ARK string for the entity remains stable across all its Versions; the suffix selects the Version.

**ARKs vs. internal identifiers.** ARK target bindings are mutable: when DaSCH migrates infrastructure, the Resolver re-points ARKs to new internal targets and citations keep working. Archive internal IRIs, by contrast, are stable within a system but not across system migrations.

**ARK Resolver as a separate bounded context:**

- The ARK Resolver is its own service with its own ubiquitous language (ARK, NAAN, suffix, target binding, …) and its own event store.
- The Archive emits domain Events (`ResourcePublished` etc.) that the ARK Resolver consumes (via a subscription / projection) to mint ARKs for newly-introduced persistent identities and to maintain bindings.
- The ARK Resolver answers `GET https://ark.dasch.swiss/ark:/.../resource-id[.version-suffix]` by looking up the ARK's binding to the target service (DPE or other), passing through the Version suffix.
- DPE receives a request like "show Resource {uuid} at v5" (or "at timestamp T" for VRE-era ARKs) and serves it from the read store, which materializes the appropriate Version node from the event log.

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
| **(c) Auto-mint** | A new Resource/Rep is published without a preferred ARK | `2` | ARK Resolver mints fresh on consuming the publish event |

### 2.5 ARK ↔ Archive binding model

- The ARK Resolver's event vocabulary includes (at minimum): `ArkReserved`, `ArkMinted`, `ArkBound`, `ArkRebound`, `ArkRevoked`.
- ARK targets point to **persistent-identity** internal Archive IRIs (e.g., the Resource's URI without Version suffix). The Version suffix from the ARK string is passed through to the target service as a request parameter.
- An ARK can be rebound (e.g., when migrating a VRE-era ARK from pointing-at-VRE to pointing-at-DPE). Rebinding emits an `ArkRebound` event; old citations continue to work because the ARK string is unchanged.

### 2.6 VRE-era ARK migration

- All VRE-era ARKs and their timestamps are knowable up-front by dumping VRE data. Migration is a **one-time bulk registration** into the ARK Resolver, not on-the-fly translation. The mapping is precomputed before any binding events are emitted.
- VRE Resource ARKs map to DAO Resource persistent-identity IRIs. The Version suffix (`/v{n}` for new ARKs; timestamp suffix for VRE-era) selects the Version on the read side.
- VRE Value ARKs (which point to specific values inside a Resource) map to DPE deep links of the form `https://dpe.dasch.swiss/resource/{uuid}/v{n}#value-X`. The ARK Resolver redirects to the deep link; DPE renders the Resource Version with the relevant value highlighted. **No new DAO concept is needed for Value ARKs** — they are a DPE/Resolver concern. **Going forward, no new Value-level ARKs are minted by the Repository.**

### 2.7 Persistence and versioning

**Identity persists across versions.** A Resource (and a Representation) has a stable persistent-identity URI on the write side and a single ARK on the long-term-citation side. Versions are denoted by suffix; they are not separately ARK'd.

**Stability layers (strongest first):**

1. **ARK** — promised to remain resolvable across all future system migrations. The single long-term commitment.
2. **Persistent-identity internal URN IRI** (`urn:dsp:resource:{uuid}`, `urn:dsp:rep:{uuid}`, etc., per §2.2 + decision 12 amendment) — stable within a system; not promised across system migrations. Not dereferenceable; never leaves the Archive boundary.
3. **Read-side projection URL** (e.g., `https://dpe.dasch.swiss/resource/{uuid}/v{n}` for DPE, analogous URLs per Access Area subdomain) — operational read-store contract; honoured as long as the event log is replayable.

**ARK resolution behavior:**
- ARK with no Version/timestamp suffix → Resolver forwards to DPE for "current Version of Resource {uuid}". DPE consults the read store to determine current Version.
- ARK with Version/timestamp suffix → Resolver forwards to DPE with the suffix; DPE serves the matching Version from the read store.

**Resource ↔ Representation linkage:** many-to-many is the desired archival reality, even though the current VRE forces 1:1 by re-uploading.

---

## 3. Versioning rules

### 3.1 What triggers a new Version

| Entity | New Version when... |
|---|---|
| **Representation** | Bitstream bytes change, **OR** Representation metadata changes (license, authorship, technical metadata, etc.) |
| **Resource** | Researcher publishes/archives — at deliberate publication events, **not** on every VRE edit |

**Important consequence:** the Repository's notion of "version" is coarser than the VRE's edit history. A Resource Version is a deliberate, named, citable snapshot. The mapping from VRE value-level edits to deliberate Resource publications is a **Producer / RDU-Tooling decision, not an Archive behaviour** (decision 49): two project types — *needs-versions* (reconstruct history into N successive Versions) and *no-versions* (current state only) — are both expressed Producer-side in the submitted SIP; the Archive emits the resulting `ResourcePublished` events only after Ingest validation (a Producer never emits events).

### 3.2 Resource ↔ Representation version pinning

A Resource Version pins **specific Representation Versions** (model (a) from the interview). Resource v3 forever references the exact Representation Versions it was published with. Citations to a specific Resource Version are stable across time.

**Preservation commitment:** any Representation Version once referenced by a published Resource Version can never be deleted. This is a load-bearing archival commitment.

The pinning is expressed in the **publish event payload**: an `ResourcePublished` event carries references to specific predecessor `RepresentationCreated` events (or to "Representation X as of event E"). The pin is a fact in the event log. The read side materializes this as queryable `:references` triples on Version nodes for SPARQL convenience.

### 3.3 Service Files and Service Projections are not versioned

Service Files (and other Service-tier projections, e.g. search indexes, SPARQL graph denormalisations) in the Access Area are **derived projections of Representation Versions**, not first-class versioned entities. When a Representation Version is created (i.e., on `RepresentationCreated` event consumption), its Service-tier projections are (re-)derived. When derivation rules change (e.g., DaSCH adopts a new IIIF profile), they are re-derived from the unchanged events. Service Files carry no ARK and no version number of their own — their identity is "the current derivation of Representation X v_n under derivation rule Y."

This aligns with Event Sourcing: Service-tier projections are derived, regenerable by replay. (Previous wording used "Service Master" — retired in favour of the three-tier role vocabulary; see `CONTEXT.md` → Preservation chain roles.)

### 3.4 Versioning lives on the read side, not in DAO

In CQRS-ES, "Version 5 of Resource X" is **a fact derived from the event log**, not a write-side entity. DAO models the write side: Resources and Representations have persistent identity; their Versions are encoded as the sequence of `ResourcePublished` and `RepresentationCreated` events that refer to them.

The **read side** materializes Version nodes from the event log. Each Version node carries derived properties:

- `versionNumber` (e.g., 3) — derived from "this is the 3rd publish event for this Resource."
- `publishedAt` — copied from the event's timestamp.
- `isCurrentVersion` (boolean) — derived from "no later non-Tombstoned publish event exists for this Resource."
- A snapshot of descriptive metadata — copied from the event's payload.
- `references` to specific Representation Version nodes — derived from the pinning recorded in the event payload.

**The read store guarantees citable URLs.** Each Access Area subdomain's URL pattern (e.g., `https://dpe.dasch.swiss/resource/{uuid}/v{n}` for DPE) is part of that subdomain's read-store contract, honoured as long as the event log is replayable. If a read store is lost or rebuilt, replay regenerates the same URLs and the same Version content.

Concrete shape on the **write side** (DAO) for a Resource that has been published 3 times:

```
dao:Resource (urn:dsp:resource:{uuid})
  rdf:type → dao:Resource
  [no Version pointers; Versions are not write-side entities]

Event log contains (referring to this Resource):
  Event #1  ResourcePublished  resource=urn:dsp:resource:{uuid}  payload={...}  publishedAt=2024-01-...
  Event #2  ResourcePublished  resource=urn:dsp:resource:{uuid}  payload={...}  publishedAt=2024-08-...
  Event #3  ResourcePublished  resource=urn:dsp:resource:{uuid}  payload={...}  publishedAt=2025-03-...
```

Concrete shape on the **read side** (one possible projection schema; not part of DAO; URLs shown for DPE — analogous patterns apply per Access Area subdomain):

```
read:Resource (https://dpe.dasch.swiss/resource/{uuid})
  read:hasVersion → ResourceVersion node (.../v1)
  read:hasVersion → ResourceVersion node (.../v2)
  read:hasVersion → ResourceVersion node (.../v3)
  read:hasCurrentVersion → ResourceVersion node (.../v3)

ResourceVersion node (https://dpe.dasch.swiss/resource/{uuid}/v3)
  read:versionNumber → 3
  read:publishedAt → 2025-03-...
  read:resultsFrom → Event #3
  read:references → RepresentationVersion node (.../rep/{uuid}/v2)
  [...all the descriptive properties as of v3...]
```

The read-side schema uses its own namespace (illustrated as `read:` above) so it is not confused with DAO. Different read stores may use different schemas; DAO is unaffected.

**Sub-decisions:**

- **Version numbers are monotonically increasing integers** (1, 2, 3, …). Not semver. One number per publish event. Citation needs to be unambiguous; semver implies authorial judgement that doesn't reliably exist for archival data.
- **`isCurrentVersion` is materialized in the read store.** Cheap, one-hop for queries; handles Tombstoning correctly (the current Version may not be the max version number).
- **Tombstoning behavior** is parked for later (open Q9). Likely a Tombstoned event affects how the read side computes `isCurrentVersion`.

### 3.5 Why no Version classes in DAO

This is a deliberate CQRS-ES choice. Three reasons:

1. **Source-of-truth purity.** DAO models what is canonically preserved. The events are canonical; Versions are derived facts. Putting derived facts in the canonical model invites them to drift from the events.
2. **Read-side flexibility.** Different consumers may want different Version-shaped projections (e.g., a search index that flattens Versions into one row per Resource; a graph view that materializes Version nodes; a citation service that resolves Version URLs). DAO not committing to one Version schema lets each reader project as needed.
3. **Replay correctness.** If the read store is rebuilt from the event log, the Version graph is regenerated. Nothing about the canonical archive depends on the read-side Version representation surviving.

The events are the source of truth. The read store materializes whatever is useful. Citation URLs are a contract of the read store, backed by the durability of the event log.

### 3.6 Bitstream storage

Bitstreams (the actual file bytes of a Representation — the **Preservation Files** per the three-tier role vocabulary, decision 30) are stored **inside their Representation's OCFL Object**, in its content area. Events carry **references** to bitstreams: a content hash (multihash) plus file-level technical metadata. A **bytes-index** in `cache/redb` (decision 38) provides reverse lookup `multihash → (OCFL Object, content-path)` so the Binary retrieval API (`GET /bitstreams/{multihash}`, §3.7.1) can serve bytes without scanning the storage tree.

The earlier framing of bytes as living "outside the event log, in a content-addressable store" remains true at the level of *abstraction* the event payload sees — the event carries a hash, the content layer resolves it — but the *concrete substrate* is no longer a separate store: per decision 36, bytes live in the entity-storage substrate inside the Rep's OCFL Object, while events live in the event-log substrate. Two physical substrates, one logical content-addressed contract.

**Deviation from PREMIS, documented:** PREMIS 3 models `Representation`, `File`, and `Bitstream` as three distinct `Object` subclasses (`PREMIS DD §1.2.2`, `§1.2.3`, `§1.2.4`), each with its own identifier and metadata. DAO **collapses `File` and `Bitstream` into facts inside event payloads plus content-addressed bytes**. Reasoning:
1. **Event sourcing makes `File`-as-class redundant.** A file is what a `RepresentationCreated` event creates; its existence and metadata are recorded in the event payload. A separate `dao:File` class would duplicate event-payload data on the write side.
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

Example shape of a `RepresentationCreated` event line in the NDJSON-of-JSON-LD log (decision 37). Whitespace added for readability; on-disk the line is compact:

```jsonc
{
  "@id": "urn:dsp:event:01958ab2-...",             // event identity (UUIDv7)
  "@type": "RepresentationCreated",
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

**Scale check (re-baselined 2026-05-15, sizing pass).** DaSCH targets **100 projects/year as a hard cap**, reached over ~2 years from 12-15/year today. The ~50 existing VRE projects migrate into the Archive one-by-one alongside net-new projects; iterative-via-refunding (e.g. CAS / shortcode 0812) is counted as a *new* project for each refunded round, not as iterative-within-project. Deposit pattern is **one-shot**: ~100 deposits/year, **~200K events per deposit** (≈ 100K Resource + 100K Rep + 1 `DepositionAccepted`; Resource-to-Rep ratio ~1:1; ~1.5 Preservation Files per Rep on average per decision 33; avg Rep ~5 MB). **Ingest steady-state: 20M events/year, ~50 TB/year storage.**

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
│       # (analogous Object trees under urn:dsp:resource:{uuid}, urn:dsp:project:{uuid},
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

**Entity Objects version on curatorially-meaningful state changes.** A `urn:dsp:resource:{uuid}` Object adds a new OCFL version whenever an `ResourcePublished` event commits; a `urn:dsp:rep:{uuid}` Object adds a new OCFL version on every `RepresentationCreated` (bytes + updated `rep.nt`); a `urn:dsp:project:{uuid}` Object on Project metadata changes; and so on. Commit-aggregate Objects (`urn:dsp:deposition:{uuid}`, `urn:dsp:preservation-action:{uuid}`) are written once on commit and never re-versioned — they hold the audit record of *that the commit happened*, not ongoing state. Each entity OCFL Object version is the Archive's own snapshot of that aggregate at that point, distinct from subscriber-side α snapshots in the Access Area (§3.7.2).

**Redb cache (`cache/archive.redb`)** holds three indexed workloads (decision 38), all fully rebuildable from the two substrates:

- **Read-side projection cache** — materialised Version nodes, Resource-in-Project lookups, current-Version-of-Resource, "all events for Rep Z in chronological order", etc. Backs queries that should not scan OCFL.
- **Event-log index** — `(stream_id, stream_version) → (segment_id, byte_offset)` and `global_offset → (segment_id, byte_offset)`. Lets the SSE handler stream events in chronological order without scanning segment files; lets subscribers resume from any `Last-Event-ID`.
- **Bytes-index** — `multihash → (containing-OCFL-Object, content-path)`. Backs the Binary retrieval API (`GET /bitstreams/{multihash}`, §3.7.1) without scanning the entity-storage tree.

Losing `cache/archive.redb` is recoverable, just slow: scan sealed segments to rebuild the event-log-index; scan entity Object inventories to rebuild the bytes-index; replay events to rebuild the read-side projection cache. Redb is convenience and latency optimisation, not a source of truth.

**No separate event-store technology.** The event log is files inside OCFL Objects; the SSE handler is a thin layer over the event-log-index in Redb plus segment reads. No Kafka, no EventStoreDB, no Postgres, no separate message broker. The ARK Resolver (separate bounded context) consumes its own subset of events via the public SSE feed (§3.7.1).

**Write path (for a Deposition).**

1. SIP arrives at the **Ingest Service** over gRPC (decision 47). Ingest is the sole producer-facing surface; it holds the SIP in an operational spool.
2. Validation runs at Ingest: SHACL against DAO; `DepositAgreement` enforcement; format ID + ClamAV on any bitstream-bearing SIP; fixity; authorisation.
3. On Ingest validation success, Ingest issues an internal `SubmitDeposition` command over gRPC to **Archive Storage's internal `CommandAPI`** (mTLS-authenticated; Ingest is one of two authorised principals). Archive Storage **re-validates** every command regardless of source (defense-in-depth per decision 47). On re-validation success:
   1. **Write-ahead the events** to the active event-log segment (NDJSON-JSON-LD lines with CRC32 per decision 37). fsync per event during the burst.
   2. **Materialise entity Object versions** in `entity-storage-root/`: for each Resource in the Deposition, add a new OCFL version of `urn:dsp:resource:{uuid}`; for each Rep, add a new OCFL version of `urn:dsp:rep:{uuid}` containing the Preservation File bytes plus updated `rep.nt`. Write the commit-aggregate audit Object `urn:dsp:deposition:{uuid}` once.
   3. **Update Redb** transactionally: append to event-log-index, append to bytes-index for each new file, update the read-side projection.
4. Acknowledge success to Ingest (the Command issuer); Ingest acknowledges to the Producer over the originating gRPC stream.

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

**Note on Archive-side snapshots that *do* exist.** Each entity OCFL Object version in `entity-storage-root/` *is* the Archive's own snapshot of that aggregate's state at that point — `urn:dsp:resource:{uuid}/v3` is the Resource's state after the third publish event, materialised in N-Triples (decision 36). These are **preservation snapshots** of the canonical entity state, not subscriber-projection snapshots. Subscriber-side α snapshots derive *projected* state shaped for the Subscriber's consumer (e.g., a SPARQL graph, a search index, an IIIF manifest cache) and are not interchangeable with the entity OCFL versions. The two snapshot kinds coexist: entity Object versions serve preservation and direct-state replay; α snapshots serve fast Subscriber restart.

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

**DAO is conceptually built on PREMIS 3.** Its core write-side classes are aligned with PREMIS entities — `dao:Resource` ↔ `premis:IntellectualEntity` (`PREMIS DD §1.1`); `dao:Representation` ↔ `premis:Representation` (`PREMIS DD §1.2.2`); `dao:Agent` ↔ `premis:Agent` (`PREMIS DD §1.3`); `dao:Event` and its subclasses ↔ `premis:Event` (`PREMIS DD §1.4`); `dao:DepositAgreement` corresponds loosely to OAIS *Submission Agreement* (`OAIS §6.1`). DAO uses the `dao:` namespace rather than `premis:` IRIs because it adds DaSCH-specific structure (identifier model, ARK linkage, CQRS-aligned event subclasses, projection-vs-write-side split) — but every DAO class should be reducible to a PREMIS/OAIS concept, and any new class that cannot must be flagged as a deviation per decision 27. The PREMIS class is `premis:IntellectualEntity` (the standard's verbatim name); DAO's `dao:Resource` is the same concept renamed to make explicit that DAO is conceived as a simplification of `knora-base` (decision 9 amended, 2026-05-18).

There is no "preserve the project ontology as a separate first-class artifact in the archive." Project subclasses (`project-xyz:Book` subclassing `knora-base:Resource`) serve VRE-only purposes (namespacing, display labels) and are normalized away during ingest. The VRE's class hierarchy is an implementation detail; it carries no archival information. **What is preserved is the property/value assertions the project made on the Resource — those are the archival content.** The originating project class IRI is not stored; the assertions made on instances of that class are stored in full.

**Open question (Q16, §7).** This position is under review. The asymmetry "property IRIs survive but the ontologies defining them do not" creates an interpretability gap for the Designated Community (CTS R10, nestor §10.3, ISO 16363 §4.2.4). Project-authored ontologies and any `knora-base` classes/properties used by a project directly may need to be preserved as the "metadata of the data" — i.e. the schema that makes the preserved property/value assertions interpretable. See Q16 in §7 for options under evaluation.

### 4.2 External ontology references survive

A Resource in DAO may carry assertions linking it to external ontologies (CIDOC CRM, FRBRoo, FOAF, Bibframe, etc.). RDF multi-typing handles this natively: a Resource can be `dao:Resource` *and* `crm:E22_Human-Made_Object` simultaneously.

The archive does **not** validate or reason over external types. It preserves them as historical assertions made by the project. External ontology maintenance is the external community's responsibility, not DaSCH's.

### 4.3 What gets dropped or reshaped during ingest

Ingest performs three jobs simultaneously, all lossless in archival terms under the current §4.1 position (see Q16 in §7 — that position is under review):

1. **Structural normalization** — flatten project-specific subclass hierarchies; preserve only the property/value assertions made on each instance (the originating class IRI is not preserved per §4.1; Q16 may reopen this).
2. **Administrative pruning** — drop runtime VRE concerns (Knora permissions bookkeeping, internal state).
3. **Vocabulary substitution** — replace internal Knora vocabularies with standards-based ones.

**Locus (clarified 2026-06-01, decision 50):** these three jobs run **Producer-side** — RDU-Tooling for VRE data, or the producer directly — *before* submission; they are what produces the DAO-shaped SIP. The Archive's **Ingest Service does not transform**: it validates the already-DAO-shaped SIP against DAO SHACL and stays Producer-agnostic (decision 47). "Ingest" in this section means producer-side ingest *preparation*, not the Archive's Ingest Service. The authoritative producer-side specification is [`producer-deposit-manual.md`](./producer-deposit-manual.md).

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

**SHACL profile consequence:** DAO's SHACL shapes describe DAO's own structural invariants (a Resource has an internal IRI, a Representation has at least one file, a Version has a publish event). They do **not** carry project-specific cardinality. The archive is structurally validated; it is not semantically validated against project intent.

**DPE consequence:** the presentation layer is permissive about what's present. It renders whatever properties exist on a Resource Version without complaining about missing ones. Partial information is the norm at scale and over time.

---

## 5. Event sourcing and storage

The archival format is **event-sourced** and **WORM-compatible**. The canonical truth is an append-only log of events. Current state is a fold over event history. Versions are derived projections, not mutable records.

### 5.1 Event vocabulary (working set)

Events represent meaningful archival actions. Tight, fixed vocabulary:

- **`ResourcePublished`** — a Resource Version is created and frozen. Carries full Resource metadata snapshot and the list of pinned Representation Versions.
- **`RepresentationCreated`** — a Representation Version is created (bytes or Representation metadata changed). Carries file hashes + Representation metadata snapshot.
- **`DepositionAccepted`** — a Deposition was validated and committed. Groups events from one ingest.
- **`FixityChecked`** — integrity verification of an **OCFL Object** against its inventory (= a `dao:Representation`'s storage container). Granularity per Object, not per File (decision 40). Outcome ∈ {`pass`, `fail`, `missing`}. **Variant payload by outcome**: `pass` = minimal attestation (~250 bytes — Object IRI, version, timestamp, outcome); `fail`/`missing` = per-File forensic detail (~2-5 KB — filename, expected hash, recomputed hash, per-File result, checker version). Both shapes belong in the WORM event log; routine pass-records are audit-grade attestation, fail/missing records are forensic evidence. Cadence: continuous-when-idle + ≥1 sweep/Object/year. Failures and missings trigger preservation-action workflows (parked, Q14). See §3.8.
- **`FormatMigrated`** — preservation action transforming a Representation's files (or, through DAO migration, a Resource) to a new format/shape; produces a new Version with provenance link to predecessor. **Emitted from a `dao:PreservationAction` aggregate, not a `dao:Deposition`** (see §6.1 and decision 26). Aligned with `PREMIS DD §1.4` (Event entity) and `OAIS §5.1.1` (preservation strategies).
- **`AccessRuleChanged`** — embargo lifted, license updated, `dao:hasAccessPolicy` retargeted, etc. Itself produces a new Version of the affected Resource or Representation per §3.1. Fires on the Resource/Rep entity; the policy entity has its own lifecycle events (see below).
- **`AccessPolicyCreated`** — a new `dao:AccessPolicy` entity comes into existence (decision 45). Fires on the policy entity (`urn:dsp:access-policy:{uuid}`). Carries the policy content snapshot (the opaque RDF blob — ontology TBD per Q19).
- **`AccessPolicyRetired`** — a `dao:AccessPolicy` entity is taken out of active use. Existing references from Resource/Rep Versions continue to resolve historically (WORM). New Resource/Rep Versions cannot adopt a retired policy. Aligned with `PREMIS DD §1.4` Event entity.
- **`Tombstoned`** — **logical retraction.** A specific Resource Version or Representation Version is marked as no longer disseminable. **Bytes and metadata remain in OCFL.** The read-side projection hides the Version from dissemination paths (DPE, IIIF, asset server, SPARQL) and ARK resolution returns a tombstone landing page that names the retraction and its provenance instead of the content. WORM is preserved. Curatorial/preservation-policy decision. Aligned with `PREMIS DD §1.4` (Event entity, e.g. types `deaccession` / `deactivation`) and `OAIS §3.3.5` (deactivation of AIPs is permitted; deletion is not in normal operation).
- **`Redacted`** — **surgical content-level erasure**, producing a new Version of the affected Resource or Representation with the offending data removed. The pre-redaction Version is `Tombstoned` *and* its bytes/metadata in OCFL are over-written or zeroed — this is the **single sanctioned exception to OCFL immutability** (see §3.7). The `Redacted` event records *that* a redaction happened, *who* authorized it, and *under what legal basis* (e.g. GDPR Art. 17, court order); it does **not** record *what* was removed. ARK resolution continues to work; it resolves to the post-redaction current Version. Legal/compliance decision, requires named authorization. Aligned with `PREMIS DD §1.4` via a custom Event subtype.
- **`PreservationActionExecuted`** — wrapper event emitted when a DaSCH-internal preservation action commits. Groups the resulting per-entity events (`FormatMigrated`, `RepresentationCreated`, `ResourcePublished` from system-ontology migration, `Tombstoned`/`Redacted` if the action is GDPR-driven, etc.) for audit and provenance. The producer-side analogue is `DepositionAccepted`.

**Outright deletion** (true erasure of an entire Version with no redacted successor) is **not modeled as a DAO event**. It is a board-level exception recorded in a separate governance log outside the archive. Treating it as a normal event would normalize what must be exceptional.

### 5.2 Events carry snapshots, not deltas

`ResourcePublished` event #5 contains the full Resource metadata as of v5, not a diff. Readers reconstruct any Version by finding the latest publish event with version ≤ N. No log replay required. Critical for WORM.

### 5.3 No in-place migration

When a system ontology change requires updating archived content (e.g., a new mandatory property), the migration is performed by emitting **new Version events** that produce new Versions of affected Representations/Resources. Old Versions remain intact. Citations to old Versions resolve to pre-migration state.

---

## 6. DAO top-level classes

**Working assumption on Archive Component**: **self-built**. DaSCH owns the Archive implementation directly; DAO is the storage model. No anti-corruption layer required. Storage layer is OCFL on filesystem with SQLite read-side cache (per §3.7).

DAO models the **write side**: persistent identities and events. Version nodes belong to the read-side projection schema (see §3.4) and are not DAO classes.

### 6.1 Class list (write side only)

| Class | Purpose |
|---|---|
| `dao:Resource` | Persistent identity of a Resource. The URI events refer to. No Version-related properties on the write side. |
| `dao:Representation` | Persistent identity of a Representation (the preservation-grade bundle containing Preservation Files). The URI events refer to. |
| `dao:Project` | Producer/owner context. Identity persists; metadata changes are recorded as events. Not Version-modelled even on the read side (no use case for cited historical Project state). |
| `dao:Agent` | Person, organization, or software acting as creator/contributor/maintainer. May carry external identifiers (ORCID, ROR). |
| `dao:Event` | The event-sourcing primitive. Subclasses for the working vocabulary in §5.1. The event log is the source of truth. |
| `dao:Deposition` | **Producer-induced** unit of ingest, grouping events from one producer-side source at one time. Gated by `dao:DepositAgreement`. Detailed shape parked (open Q3). |
| `dao:DepositAgreement` | Producer/Archive contract. Carries producer identity, designated community, accepted formats, retention terms, embargo/access defaults, frequency of submission. May store a link to where the agreement document lives. |
| `dao:PreservationAction` | **Archive-induced** unit of change. Distinct from `dao:Deposition` (decision 26). Gated by internal preservation policy rather than a `DepositAgreement`. Groups events that result from archive-initiated activity: format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction. Aligned with `PREMIS DD §1.4` (Event entity) and `OAIS §4.1.3 / §5.1` (Preservation Planning). |
| `dao:AccessPolicy` | First-class entity carrying an opaque RDF policy blob, referenced from Resource/Rep Versions via `dao:hasAccessPolicy` for the COAR `restricted_access` case (decision 45, 2026-05-18). Multiple Resources/Reps can reference the same policy (project-level patterns are common). Internal policy ontology — what fills `dao:AccessPolicy.policyContent` — is deferred to Q19 (research candidates: ODRL, PREMIS rights, XACML, DC terms, DaSCH-custom). Lifecycle events `AccessPolicyCreated` / `AccessPolicyRetired` in §5.1. |

### 6.2 Not in DAO

- **`dao:ResourceVersion`, `dao:RepresentationVersion`** — read-side projection nodes, not DAO classes. The read store materializes these from events; their schema lives outside DAO and may evolve independently.
- **Relationships between Resources** (cites, isPartOf, derivedFrom): plain RDF properties between Resources. Not reified as a `dao:Relation` class. The `kb:isPartOf` + `kb:seqnum` pair specifically is parked as Q18 (currently tacit knowledge in `dsp-api` code; DAO should make the structural commitment explicit via SHACL).
- **Files within a Representation**: per-File information lives as blank-node-structured properties on `dao:Representation` via `dao:hasFile`, navigable from the Representation's RDF in its OCFL Object (`rep.nt`). **Not a separate DAO class** (decision 33). Per-File information is addressed by `dao:filename` within the parent Representation Version's context. For event references to a specific File: use the (Representation Version IRI, filename) tuple. Per-file Bitstream-level sub-addressing remains parked.
- **Composition rules for `dao:accessRights`** (most-restrictive-wins, parent/child propagation, the "two access spaces" partitioning) — decision 44 made these read-side γ-projection concerns. DAO stores explicit per-Version facts; γ computes.
- **`dao:Place`, `dao:Concept`, and others**: may emerge when working through project-level metadata. Likely external IRIs (GeoNames for places, Getty AAT for concepts) rather than DAO classes. Parked.

---

## 7. Open questions parked for later

These came up but were deferred. New items added after folding in updated architectural context.

**From the original conversation:**

1. **Is `ResourcePublished` the only path to a new Resource Version?** Format migration of a Representation *referenced by* a Resource does not create a new Resource Version (the existing Resource Version still pins the old Representation Version). But system-ontology-driven Resource migration does. The full path inventory is not yet complete.
2. **Event ordering**: per-entity vs. global total order across the whole Repository. Per-entity is the event-sourcing convention and likely sufficient.
3. **Deposition boundary**: project-scoped incremental batch (recommended (b)) vs. cross-project (c). The separation question is **resolved** by decision 26 (`dao:PreservationAction` is distinct from `dao:Deposition`). What remains open is the granularity of a Deposition itself: one per project per submission event, vs. one spanning multiple projects from the same producer, vs. one open transactional boundary across an arbitrary session.

**From folding in updated architectural context:**

4. **Access Area manifestation: push vs. pull.** Push (Archive emits events, Access Area projects) fits Event Sourcing naturally and was the implicit model for our event vocabulary. Pull (Access Area queries Archive on demand) is simpler operationally but harder to reason about for projections.
5. **Access Area events in DAO scope?** If push: do Access Area events (`ServiceFileDerived`, `ServiceFileInvalidated`, `ProjectionRebuilt`) belong in DAO's event vocabulary, or in a separate Access-context vocabulary? **Resolved 2026-05-15**: separate, because Service Files / Service Projections are not in DAO's archival scope (decision 32). Each subscriber subdomain owns its own internal event vocabulary if it event-sources internally; these events live in an `access:` namespace and are not part of Archive's published SSE feed.
6. **DPE/CPE presentation hints in DAO?** Should DAO carry presentation hints (e.g., "render this Resource as a recipe card", "highlight property X")? Lean: **no** — presentation is not preservation. CPE configuration lives outside the archive.
7. **IIIF Manifests in DAO?** IIIF has its own information model (Manifest, Canvas, Annotation). Lean: DAO stores enough technical metadata on Representations that the IIIF server can compute Manifests at request time. Manifests are not stored in the archive.
8. **CoreTrustSeal evidence linkage.** Several DAO choices (immutable event log, fixity events, format migration provenance, no in-place migration) double as CoreTrustSeal audit evidence. Should be cross-referenced explicitly per requirement.
9. ~~**Tombstoning detailed shape.**~~ **Resolved by decision 28.** Tombstoning is logical-retraction-only; bytes and metadata remain in OCFL; read-side hides the Version. A separate `Redacted` event handles GDPR-driven content erasure. Outright deletion is not a DAO event. See §5.1 and §3.7. *Remaining sub-detail:* exact shape of the tombstone landing page returned by DPE on ARK resolution — DPE concern, not DAO.
10. **Representation property list.** The full property list for `dao:Representation` and `RepresentationCreated` event payloads (drawing on Knora `:FileValue` properties: `internalFilename`, `internalMimeType`, `originalFilename`, `originalMimeType`, `hasCopyrightHolder`, `hasAuthorship`, `hasLicense`, etc., reshaped to use external standards per §4.4) is TBD.
11. **ARK reservation and expiry policy.** Reservations made before publication can be reclaimed (bound to a real target). Whether reservations expire if never claimed, and after how long, is TBD. **Subsumed by Q17** (2026-05-18) as one sub-question of the broader ARK Resolver behaviour-model workstream.
12. ~~**`dao:AccessRights` for restricted access.**~~ **RESOLVED 2026-05-18 across four decisions.** Q12.1 resolved 2026-05-15 (γ delivery-layer enforcement). Q12.2 sub-question 1 resolved 2026-05-18 by **decision 44** (explicit per-Version `dao:accessRights` on Resource and Representation; no propagation in DAO; composition is read-side projection concern). Q12.2 sub-question 2 resolved 2026-05-18 by **decision 45** (`dao:AccessPolicy` as first-class entity with opaque policy content). Q12.2 sub-question 3 resolved 2026-05-18 by **decision 46** (DAO carries policy lifecycle events; γ-per-access logs are operational telemetry, not preservation evidence). **Downstream open**: Q19 (policy ontology selection for `dao:AccessPolicy.policyContent`, parked 2026-05-18, research-needed).
13. **Project metadata as ontology.** Project-level metadata is itself a complete ontology (currently the basis for DPE's data model). Working through this may surface `dao:Place`, `dao:Concept`, and other secondary classes that haven't yet been needed. Not urgent for DAO core but a known workstream.
14. **Preservation-action workflows (response and format-migration policy).** Partially engaged. **Fixity *cadence* and *granularity* settled (decision 40)**: per OCFL Object, continuous-when-idle + ≥1 sweep/Object/year, variant payload by outcome. **Still parked:** (i) response workflow when `FixityChecked` returns `fail` or `missing` — alerting, triage, recovery from secondary copies, who-decides-what; (ii) format-migration triggering policy — when does DaSCH initiate a `FormatMigrated` action, against which format-risk signals (PRONOM advisories, vendor obsolescence notices, etc.), with what stakeholder review; (iii) system-ontology-migration policy. These are orchestration concerns the Archive emits events for, not DAO classes. The events themselves belong to `dao:PreservationAction` (decision 26).

15. ~~**Curation-level commitment per `dao:DepositAgreement`.**~~ **RESOLVED 2026-05-16 by decision 43.** DaSCH commits to uniform A + C + D across all projects, all Depositions, all preserved content (per the CTS Curation & Preservation Levels Position Paper v3.0). No per-Agreement / per-Deposition / per-Resource variation. DAO does not model preservation-level as a tunable — the commitment is institutional invariant. The sub-questions about granularity, override events, and PreservationAction validity are moot. See decision 43 and §1a preservation-commitment paragraph.

16. **Project ontologies as preservation-essential metadata.** Project-authored ontologies (classes and properties a project defines to describe its data), together with any `knora-base` classes/properties used by a project *directly* — i.e. without subclassing or subpropertying — are themselves preservation-essential: they are "the metadata of the data". Without them, the property/value assertions preserved on `dao:Resource` instances are uninterpretable to the Designated Community. **Reopens part of §4.1, §4.3, decision 6 (amended), and decision 10**, which currently take the position that project subclasses "serve VRE-only purposes and are normalized away during ingest." Property IRIs already survive ingest per §4.1, but the ontology files defining them do not — that asymmetry is the substantive question. Options to evaluate: (a) preserve project ontologies as first-class archival artifacts attached to `dao:Project` (e.g. OCFL-packaged TTL with a `dao:hasOntology` link from `dao:Project`); (b) preserve a snapshot of the `knora-base` ontology used at each Deposition's submission time, since directly-used `knora-base` terms are interpretation-anchors equally with project-defined terms (`ontologies/knora/knora-ontologies/` is already checked into the repo, anticipating this); (c) treat them as external-ontology references per §4.2 (project assertions survive as historical assertions; the ontology files themselves are preserved out-of-band in a project-scoped equivalent of `standards/`). Audit anchor: CTS R10 (data interpretable by the Designated Community), nestor §10.3, ISO 16363 §4.2.4. **Distinct from item 13** above (which concerns project *administrative* metadata as ontology — title, funding, attributions); this item concerns the project's *domain* ontology (the schema describing the archived content). May surface a new top-level DAO class `dao:Ontology` (or similar) for the archival artifact wrapping a project's ontology files; defer naming until the option is chosen.

17. **ARK Resolver behaviour model and full URN ↔ ARK lifecycle.** Decision 13 settled the ARK strategy at a coarse level (three use cases: migration / reservation / auto-mint) and §2.3 – §2.7 sketched the ARK Resolver as a separate bounded context with its own event store. This item parks the **substantive design and research workstream** needed to finalize ARK behaviour before implementation. Open dimensions:

    (a) **Pre-publication reservation flow.** A researcher needs an ARK to cite in a paper *months before* the underlying data is published (peer review runs on long horizons; the ARK string must be quotable today and resolve tomorrow). Reserved ARKs should pre-exist in the Resolver, return a "reserved — data pending" response while unbound, and become live resolutions once a `BindArk` (or analogous) event fires. Event vocabulary placeholders in decision 13 / §2.5: `ArkReserved`, `ArkMinted`, `ArkBound`, `ArkRebound`, `ArkRevoked`. Working term `ArkCoined` (Ivan's framing) may be the right name for one of these acts (reservation? initial-publication-mint?); research the ARK community's preferred vocabulary before locking. **Subsumes Q11** (reservation expiry policy).

    (b) **Explicit URN ↔ ARK binding events.** The Archive emits `ResourcePublished` etc. when persistent-identity URNs come into existence; the ARK Resolver consumes those events and mints/binds ARKs. The forward direction (ARK → target) is the Resolver's internal concern. The **explicit URN ↔ ARK binding fact** needs to be first-class in the Resolver's event store, replayable, and citable in audits. Whether the Archive emits a URN-side mirror event ("this URN now has ARK X bound to it") or treats ARK assignment as Resolver-internal is open. CTS R12 / nestor §12.1 / ISO 16363 §4.2.4 want PID assignment to be auditable end-to-end.

    (c) **Content-negotiation-driven resolution.** A single URN (`urn:dsp:resource:{uuid}`) may resolve to multiple Access Area subdomain URLs depending on the requested representation: HTML (default) → DPE (e.g. `https://repository.dasch.swiss/dpe/projects/{shortcode}/resources/{uuid}`); IIIF Manifest → SIPI (`https://iiif.dasch.swiss/{shortcode}/{rep-uuid}/manifest.json`); raw bytes → asset server; SPARQL results → endpoint; JSON-LD → a structured-data endpoint. The ARK Resolver maintains an internal translation table from `(urn, accept-header / suffix-hint)` to target URL. The mechanism, the table's schema, who owns its updates, and how it stays in sync with Access Area deployment topology are all open. **Research needed**: ARK community practice on content negotiation (NMA conventions; the `?info` / `??` inflection patterns); DOI's CrossRef / DataCite content-negotiation patterns; Handle's `type=` parameter convention; how Internet Archive, BnF, and other large ARK-using institutions handle multi-target resolution.

    (d) **Move-out-of-DaSCH / custody transfer.** When a project's data is transferred to another repository (DaSCH winds down a project; the depositing institution takes custody back; the data moves to a peer trustworthy repository) the ARK must continue to resolve — to the new custodian's URL. This is OAIS *AIP transfer / succession* territory. Requires a new event class on the **Archive side** (working name `CustodyTransferred` — research the right term) that records: which entities (Resources, Representations, Project) are leaving; the new custodian's identity (org + endpoint); the new resolution target URL pattern; the legal handoff basis (succession agreement, retention transfer, etc.). The Resolver consumes this and emits `ArkRebound` for each affected ARK. **Extends decision 5** (event sourcing / WORM): the entities physically leave the Archive but the *historical record* of their custody, plus the forwarding pointer, must remain.

    (e) **Research scope before locking the design.** Dispatch a research thread covering: (i) the ARK specification + NMA conventions (current spec maintained by California Digital Library; `arks.org`); (ii) content-negotiation patterns in PID resolvers (DOI CrossRef/DataCite, Handle System, w3id, PURL); (iii) custody-transfer patterns in trustworthy repositories (CLOCKSS / Portico models for journals; nestor §4.6 succession examples; CoreTrustSeal R03 evidence examples); (iv) ARK lifecycle vocabularies (reservation states, deactivation, redirection, tombstoning vs. `410 Gone`); (v) how peer institutions (DARIAH partners, Swiss SNSF-funded repositories) currently handle cross-repository PID continuity.

    **Affects:** §2.3 – §2.7 (ARK content will need refinement after Q17 resolves); decision 13 (likely amendment); §5.1 event vocabulary (new `CustodyTransferred`-class event on the Archive side); CONTEXT.md ARK entry.

    **Audit anchors:** CTS R03 (succession / continuity-of-access), R12 (PIDs); nestor §4.6 (succession), §12.1 (identifier system); ISO 16363 §3.1.2.1 (succession plan), §4.2.4 (unique persistent identifiers). **Must land before CTS application** — R03 and R12 are core CTS requirements that the design must demonstrably support.

18. **Compound Resources and the `kb:isPartOf` + `kb:seqnum` pair in DAO.** Knora-base provides `kb:isPartOf` (subproperty of `kb:hasLinkTo`; both subject and object constrained to `kb:Resource`; `knora-base.ttl:609`) plus `kb:seqnum` (object `kb:IntValue`; *"the position of a resource within a compound object — typically the order of pages within a book or similar"*; `knora-base.ttl:679`). The pairing requirement — *whenever `isPartOf` is asserted, `seqnum` must also be asserted on the same child Resource* — is **tacit knowledge enforced in `dsp-api` code, not declared in the `knora-base` ontology**. DAO should make this structural commitment explicit (via SHACL shape on `dao:Resource`: if `kb:isPartOf` is asserted then `kb:seqnum` is required, with consistent typing). Open dimensions:

    (a) **Property survival across the kb → DAO substitution boundary.** Does DAO keep the `kb:` prefix for `isPartOf` / `seqnum` / `isPartOfValue` (treating them as preserved knora-base terms per §4.2 / Q16 simplification-of-knora-base framing), or does it introduce `dao:isPartOf` / `dao:seqnum` aliases? The Q9-amendment framing ("DAO is a simplification of `knora-base`") suggests **keeping the `kb:` prefix verbatim** — minimal renaming, the source ontology stays recognisable in the archived RDF. Confirm.

    (b) **Compound semantics in DAO.** What counts as a "compound Resource"? Is it a structural pattern (parent + N children-via-`isPartOf` + `seqnum` on each child), a class (a `dao:CompoundResource` subclass of `dao:Resource`?), or just a graph shape recognised by SHACL? Working position: **graph shape, not class.** Aligns with §6.2 "relationships between Resources are plain RDF properties, not reified as classes."

    (c) **Reification companion (`kb:isPartOfValue`).** Knora's permission system uses the `LinkValue` reification pattern (every direct link has a companion `*Value` resource). DAO has no `kb:hasPermissions`-style runtime access machinery (stripped at ingest per §4.3). Does the `isPartOfValue` reification carry archival information, or is it pure VRE-permissions infrastructure that should be stripped along with `kb:hasPermissions`? Likely answer: **strip** — the direct `kb:isPartOf` property carries the archival fact; the `LinkValue` exists for VRE's permission bookkeeping which DAO doesn't reproduce.

    (d) **Related compound pattern — `kb:isRegionOf`.** Image annotations (a Region of an image Representation) use `kb:isRegionOf` (subject `kb:Region`, object `kb:Representation`). This is a different compound shape (Resource-points-at-Representation, not Resource-points-at-Resource). Whether `dao:Region` is preserved and how it relates to DAO's preservation-vs-presentation split (regions feel more like Access Area presentation overlays than preservation-grade content) is open. Defer naming until use cases surface.

    (e) **Connects to access-rights (Q12.2.1 resolution, 2026-05-18).** DAO doesn't propagate access rules across `isPartOf`; every Resource Version carries explicit `dao:accessRights`. **But the structural relationship itself must be navigable** so read-side projections can implement propagation, faceted access decisions, and the "two access spaces" partitioning Ivan is contemplating. Q18 is therefore *load-bearing* for Q12.2's eventual γ-projection design — the projection can only propagate parent rules if `isPartOf` is queryable in the read store, which means the relationship must be preserved (with `seqnum`) at the write side.

    **Affects:** §4.2 (external-ontology survival policy — `kb:isPartOf` / `kb:seqnum` as preserved kb terms); §4.3 (vocabulary substitution — what's stripped vs. preserved); §6 (DAO top-level classes — does `dao:Region` enter?); SHACL shapes (pair-requirement enforcement); CONTEXT.md (compound-Resource pattern should appear in the glossary).

    **Audit anchors:** CTS R10 (technical quality / interpretable for Designated Community); nestor §10 (technical quality); ISO 16363 §4.2.4 (sufficient information for the Designated Community to identify and understand). The DSP-API "tacit knowledge" point is itself an R10 risk — preservation-essential structural commitments should live in declarative artefacts (ontology + SHACL), not in implementation code that may not survive a system migration.

19. **Policy ontology selection for `dao:AccessPolicy.policyContent`** — research-needed (parked 2026-05-18). Decision 45 made `dao:AccessPolicy.policyContent` an opaque RDF blob; the choice of policy ontology to fill that slot is a separate sub-decision. Candidates to evaluate:

    (a) **ODRL** (Open Digital Rights Language) — W3C Recommendation 2.2 (Feb 2018), maintained by the W3C ODRL Community Group. RDF-native, JSON-LD serialization. Information model: Policy → Permission/Prohibition/Duty → Action / Asset / Party / Constraint. Widely used by media organizations, national libraries, EU data portals. The obvious external candidate; aligns with DAO's RDF-native stance.

    (b) **PREMIS rights extension** — `premis:rightsStatement` + `premis:rightsGranted`. Preservation-domain native; aligns with decision 6 amended (PREMIS-conceptual). Less expressive than ODRL but closer to the existing DAO ↔ PREMIS conceptual mapping. May be the right minimum-viable choice if DaSCH's policy patterns are simple.

    (c) **XACML** (eXtensible Access Control Markup Language, OASIS) — XML-based, enterprise/access-control focused. RDF mapping is awkward. Rich expressiveness for fine-grained authorization. Probably overkill for DaSCH's volume and complexity.

    (d) **DC terms** — `dcterms:rights`, `dcterms:accessRights`, etc. Simple; works for natural-language-plus-light-structure cases. Insufficient on its own for `restricted_access` rules with structured group/party logic.

    (e) **DaSCH-custom vocabulary** — bespoke RDF terms fitted to DaSCH's actual policy patterns (likely simple: "authorized agents are [list of ARK URNs]" or "authorized group is X"). Minimal but lock-in; should only be chosen if research shows external candidates are genuinely ill-fitting.

    **Research scope before commitment.** Walk DaSCH's actual current and anticipated restricted-access cases — how complex are the rules in practice? Compare the candidates against those cases. Check ARK-community / CTS-certified-repository practice (e.g., which policy ontology does Zenodo use? Dryad? DataCite-affiliated repositories?). Look at the Swiss data-protection landscape for any FADP-derived requirements that constrain the choice.

    **Likely outcome (without prejudging the research):** **(a) ODRL** for external alignment, or **(b) PREMIS rights** as a fallback if ODRL is overkill for DaSCH's actual patterns. The two compose — using ODRL for rich rules where they exist and PREMIS rights for the simpler cases is also viable.

    **Affects:** decision 45 (the `dao:AccessPolicy.policyContent` slot will get a concrete schema once Q19 resolves); §4.4 vocabulary substitution policy (policy ontology becomes a third substituted standard alongside COAR Access Rights, when picked); §5.1 event vocabulary (new policy-content shape may add or refine event payloads).

    **Audit anchors:** CTS R13 (reuse conditions documented + machine-readable); nestor §11.2; ISO 16363 §4.5, §3.5.1. **Non-blocking on Q12.2 sub-question 3** (audit-trail mechanism); should land before policy implementation begins.

20. ~~**Producer-facing surface unification (Ingest as universal SIP gate; gRPC as Producer-Archive interface).**~~ **RESOLVED 2026-05-21 by decision 47.** Producer-side architecture sharpened from decision 31's "two deployments of the Archive" framing to a stronger model: **Ingest Service is the sole producer-facing surface** for the Archive, accepting *all* Producer submissions (RDU-Tooling bulk Depositions, Metadata Editor project-metadata edits, future Producer-side tools) via a single Producer-Archive interface. Everything Producer-side is a SIP — content-bearing (Resources + Representations + bitstreams), metadata-only (project metadata edits, no bitstreams), or mixed. The Ingest Service runs SIP-shape-appropriate validation (SHACL always; format ID + ClamAV on bitstream-bearing SIPs; `DepositAgreement` enforcement always) and on success commits via the Archive Storage's internal `CommandAPI`. **Wire transport is gRPC** (protobuf schemas + HTTP/2 streaming); commands and queries are unary RPCs, EventStream is server-streaming, bitstream upload/retrieval are streaming. **CommandAPI moves to internal-only** — accepting authenticated principals (Ingest Service; DaSCH-internal preservation admin tooling) via mTLS; no longer Producer-facing. **Archive Storage self-defends** with re-validation of every command regardless of source (defense-in-depth; edge validation is pre-validation for fast failure). **No SIP-as-submitted preservation**: SIPs live on Ingest for a backup window only, not in the WORM event log — OAIS does not require SIP preservation (SIPs are transformed into AIPs; the AIPs are what's preserved); the Producer-Archive interface is documented via the protobuf schema (CTS R02 / R08 evidence). See decision 47.

21. **Self-service preservation frontend (Metadata Editor + SIP submission GUI consolidation).** A separately-deployed Rust SSR service for RDU staff + external users to **create / edit project-level metadata and submit Depositions through a browser**. Likely consolidates with a future "SIP submission GUI" into one frontend: **the self-service preservation frontend**. Key open dimensions:

    (a) **Bounded-context status.** Likely a **new bounded context** distinct from the Archive — it has its own ubiquitous language (drafts, edits, published, save, publish), its own authentication of external users, its own UX state. Not yet confirmed; the call depends on whether the language is genuinely distinct or whether it can stay inside Archive's ubiquitous language. To be settled in Q21's design session.

    (b) **Write path.** Goes through the **Ingest Service** (decision 47) as a SIP-submitting Producer; the Metadata Editor packages each save as a SIP (mostly small, metadata-only; sometimes carrying bitstreams when project pages embed banner images / attached documents — at which point the SIP becomes mixed and Ingest's ClamAV applies). RDU-Tooling and the self-service frontend submit through the *same* gate; the asymmetry is purely in what content their SIPs carry, not in how they reach Ingest.

    (c) **Read path — phased evolution.** **Phase 1 (current pragmatic choice): (a) read directly from Archive Storage's gRPC `QueryAPI`** — sees durable archival state; no local projection to maintain; simplest to build. **Phase 2 (future, when offline-edit capability is wanted): (b) materialise a local projection by subscribing to the EventStream** — gives the Metadata Editor its own read-side that can include unpublished drafts and operate while the Archive is unreachable, with publishing deferred until Archive is back up. (b) makes the Metadata Editor a Producer-side CQRS read-model + draft-state store — meaningful complexity bump, justified only if offline-edit becomes a real requirement.

    (d) **Authentication of external users.** External users (project leads, researchers) authenticate to the Metadata Editor; the Metadata Editor authenticates to Ingest as a principal (mTLS) and carries the originating user identity in submission metadata for audit-trail purposes. The Archive treats Metadata-Editor-submitted Depositions as it treats RDU-Tooling-submitted Depositions: both arrive via Ingest with a documented `DepositAgreement`; the difference is who the originating Producer-side `dao:Agent` is.

    (e) **Replaces the JSON-in-Docker pattern for DPE project metadata.** DPE today serves project-level metadata from JSON files baked into its Docker image (`modules/dpe/server/data/`); changing metadata requires rebuilding the container and redeploying. Under Q21 (combined with decision 47), DPE becomes a true Access Area subscriber per decision 32: project metadata is published into the Archive (as `dao:Project` events emitted from a metadata-edit Deposition), DPE materialises a projection from the EventStream, and DPE serves the projected view. Removes the rebuild-and-redeploy step entirely.

    **Affects:** CONTEXT-MAP.md (add the self-service preservation frontend as a new component, possibly a new bounded context); `modules/dpe/server/data/` (eventually retired in favour of projection-from-EventStream); §5.1 event vocabulary (project-metadata-edit events — likely `ProjectMetadataUpdated` or modelled as the existing `dao:Project`-targeted events emitted from a metadata-only Deposition; to be settled in Q21 design session).

    **Audit anchors:** CTS R13 (reuse conditions / metadata machine-readability — extends to project-level metadata); nestor §6.1 (Ingest), §11 (Access); ISO 16363 §4.1.1 (Ingest workflows), §3.5.1 (Producer-Archive agreement coverage for the new Producer class — external users).

    **Status:** parked; expected to become an active design thread once the DAO-shape session (Q10 + Q16 combined, anchored against a VRE export) lands a concrete write-side shape that the editor would manipulate.

22. **SIP wire-format and Producer-Archive interface — gRPC adopted; BagIt deferred as external-producer drop-in.** Decision 47 commits to **gRPC as the Producer-Archive interface** for the closed ecosystem (DaSCH-controlled Producers: RDU-Tooling, self-service preservation frontend). Open sub-dimensions parked here:

    (a) **gRPC protobuf message shape — `SubmitSip(stream SipChunk) returns (SipReceipt)` and sibling RPCs.** The exact protobuf field set (envelope metadata; chunk framing; content payload separation; receipt structure) is implementation-side concern but should be documented as the Producer-Archive interface spec (CTS R02 / R08 evidence). Open question: does the protobuf carry strongly-typed DAO classes as nested messages (schema duality between proto and SHACL), or carry an opaque serialised-RDF payload (Turtle / N-Triples) plus bitstream chunks (SHACL-only validation, no proto-side DAO schema)? **Leaning option B** — N-Triples-at-rest (decision 34) is preserved more cleanly when wire-side does not duplicate the DAO schema; protobuf describes only the *envelope* (a SIP submission, with metadata header + opaque RDF blob + bitstream chunks). Avoids schema duality.

    (b) **External-Producer translation gateway — drop-in BagIt when needed, not native.** If DaSCH ever opens Producer access beyond DaSCH-controlled tools (third-party researcher tools, peer-archive batch ingest, etc.), **the external format translation lives in RDU-Tooling or a sibling translation-gateway component, not in Ingest**. Ingest remains gRPC-only with one well-specified input grammar. The translation gateway accepts BagIt / RO-Crate / E-ARK CSIP / etc., reshapes into gRPC SIP submissions, and forwards to Ingest. Architectural benefits: Ingest's attack surface stays single-shape; format-fragmentation complexity sits in a non-trust-critical component where it can evolve freely. **Working preference for the eventual external format: BagIt** (lowest tooling burden; most archivist-recognised; widest CTS evidence story). RO-Crate is the second candidate if RDF-native packaging becomes important for FAIR-alignment claims. Research dispatch deferred until external-Producer access is actually scoped (no current driver).

    (c) **Mixed-content SIPs from the self-service preservation frontend.** A SIP from the Metadata Editor may carry: pure project-metadata triples (small, RDF-only); attached bitstreams (banner images, project-attached PDFs — bitstream chunks, ClamAV gates apply); or content-bearing Resources + Representations identical in shape to an RDU-Tooling SIP. The gRPC envelope handles all three uniformly (the SIP is a header + RDF blob + zero-or-more bitstream chunks); Ingest's validator suite runs the bitstream-bearing branches conditionally on chunk presence. No format-level distinction between "metadata edit" and "Deposition" — both are SIPs.

    (d) **Closure on SIP preservation question.** Decision 47 settles: **no SIP-as-submitted preservation in the WORM event log**. SIPs are held on Ingest for an operational backup window (Ingest-local retention + offsite backup of Ingest's spool directory) only. Rationale: OAIS does not require SIP preservation (SIPs → AIPs; AIPs are what's preserved); per-event payloads in the event log carry the full snapshot per decision 24 — sufficient for replay and audit; the Producer-Archive interface (the protobuf schema itself) is documented as the contract, satisfying CTS R02 / R08 without needing the bytes-as-submitted. If a future audit ever needs replay-from-SIP, the events-with-snapshots support replay-from-events.

    **Affects:** Producer-Archive interface protobuf spec (to be authored separately; lives in the implementation repo when work begins); CONTEXT-MAP.md integration row (SIP transport now gRPC, not HTTP); decision 31 amended by decision 47 (CommandAPI internal-only, transport gRPC for all surfaces); §5.1 event vocabulary (unchanged — same events whether the originating SIP comes from RDU-Tooling or the Metadata Editor).

    **Audit anchors:** CTS R02 (Producer-Archive agreement documentation — the protobuf schema is the machine-readable component), R08 (deposit specifications documented); nestor §6.1 (Ingest, documented Producer-Archive interface); ISO 16363 §3.5.1 (deposit specifications), §4.1.1 (workflow that ensures submissions conform to specifications).

    **Status:** sub-questions parked alongside decision 47's closure. Sub-question (a) is design grunt work; (b) defers until external-Producer access is real; (c) is operational implementation; (d) is closed.

23. **`dao:Ontology` versioning.** The VRE does **not** version ontologies — a VRE-sourced `dao:Resource` conforms to the project's single, current ontology, so its conformance reference is trivially "the latest" (one version). **Producers other than the VRE may version their ontologies**, so the model must allow it: a `dao:Resource` then references a **specific `dao:Ontology` version** (pinned, not "latest") and is **validated against that version**. Open dimensions:

    (a) **Entity shape.** Is `dao:Ontology` a versioned entity with its own version history (materialised like Resource/Representation Versions), or are ontology versions distinct preserved artifacts that a Resource pins? The latter mirrors the **Resource ↔ Representation version-pinning** pattern (§3.2: a Resource Version pins specific Representation Versions; here a `dao:Resource` Version pins a specific `dao:Ontology` version — stable conformance across time).

    (b) **VRE degenerate case.** For VRE data there is exactly one ontology version, so the pin is trivially the only version; nothing changes for VRE-sourced Resources beyond making the reference explicit.

    (c) **Version-aware validation.** Decision 54 Tier-2 validation must validate a Resource against the **pinned** ontology version's shapes, not the current one. The stored/dissemination SHACL profile and the fat-event snapshot (decision 52) must carry the ontology-version reference.

    (d) **Custody / migration.** A pinned ontology version can never be dropped while a Resource references it (parallels the §3.2 "pinned Representation Versions never deleted" commitment).

    **Couples to Q16** (ontology preservation; `dao:hasOntology` on `dao:Project` — versioning sits on the ontology artifact) and **decision 50** (Producer-agnostic; non-VRE Producers drive this requirement). **Affects:** §3.2 (version-pinning extended to ontologies), §6 (a `dao:Ontology` class + its versioning), decision 54 (version-aware validation), the manuals' SHACL profiles. **Audit anchor:** CTS R10 / nestor §10.3 / ISO 16363 §4.2.4 (the correct schema must travel with the data it interprets). **Status:** parked (backlog, added 2026-06-02).

---

## 8. Decision log

The numbering is the order decisions were made, not document order.

| # | Topic | Decision | Date |
|---|---|---|---|
| 1 | Source of discomfort with RiC-O | Conceptual mismatch: research data ≠ records-management; long-lived Resources/Representations need first-class versioning | — |
| 2 | Unit of identity across versions | Both Resources and Representations have independent identity and lifecycles (model (c)); each has its own internal IRI and zero-or-more ARKs; many-to-many between them | — |
| 3 | New-Version triggers | Representation: byte change OR metadata change. Resource: at publication events, not on every VRE edit | — |
| 4 | Resource → Representation linkage | Resource Versions pin specific Representation Versions; preservation commitment that pinned Representation Versions are never deleted | — |
| 5 | Event sourcing | Yes; WORM-compatible; events carry snapshots not deltas; no in-place migration | — |
| 6 | Single archival ontology | DAO is the only ontology archived content directly conforms to; project ontologies normalized away; external ontologies (CRM etc.) survive via multi-typing; cardinality not archived. **Amended 2026-05-12 per decision 27:** DAO is conceptually built on PREMIS 3 — its core write-side classes (`Resource`, `Representation`, `Agent`, `Event`) map directly to PREMIS entities; the `dao:` namespace exists to add DaSCH-specific structure, not to replace PREMIS. Every DAO class should be reducible to a PREMIS or OAIS concept, with documented deviations. **Under review (Q16, §7, 2026-05-18):** the "project ontologies normalized away" clause is reopened — project-authored ontologies and directly-used `knora-base` terms may need to be preserved as interpretation-essential metadata for the Designated Community. | — / amended 2026-05-12 / under review 2026-05-18 |
| 7 | DAO top-level classes | Settled. List in §6. Build-vs-buy does not gate DAO class shape | — |
| 8 | Bounded contexts handling of DAO terms | DAO is the Archive context's ubiquitous language. Producer and Access contexts have their own internal vocabularies. DAO terms appear at boundaries (lingua franca of seams), not as universal language | — |
| 9 | Naming the two core entities | Originally (2026-05-12): `dao:IntellectualEntity` (IE) and `dao:Representation` (Rep), adopting PREMIS terms verbatim, with `Resource` explicitly avoided due to collision with `knora-base:Resource`. **Amended 2026-05-18:** the IE class is renamed to **`dao:Resource`** to make explicit that DAO is conceived as a simplification of `knora-base`. The verbatim-PREMIS-naming rule is downgraded to "conceptually reducible to PREMIS" (decision 6 amended still applies): `dao:Resource` is conceptually `premis:IntellectualEntity`. Event names `IntellectualEntityVersionPublished` → `ResourcePublished` and `RepresentationVersionCreated` → `RepresentationCreated` (the `Version` infix dropped: a Version is always created as a consequence of a state-committing event — naming the event after that consequence leaked OCFL/read-side mechanics into the domain vocabulary). URN prefix `urn:dsp:ie:{uuid}` → `urn:dsp:resource:{uuid}`. Read-side URL path `/ie/{uuid}/v{n}` → `/resource/{uuid}/v{n}`. Abbreviation "IE" is retired. Collision with `knora-base:Resource` is managed by prose discipline: always namespace-prefix in writing (`dao:Resource` vs `kb:Resource`). **Deviation from decision 27 "cite or deviate" PREMIS-verbatim default**, documented: the simplification-of-knora-base framing is more honest to how DaSCH thinks about the model than PREMIS-verbatim naming, and the `Resource` name signals that relationship to engineers and curators. `dao:Representation` is unchanged (PREMIS-aligned *and* knora-base-aligned). | — / amended 2026-05-18 |
| 10 | Originating class IRI not preserved | The VRE's class hierarchy is implementation detail. What is preserved is the property/value assertions made by the project on the Resource. **Under review (Q16, §7, 2026-05-18):** the "class IRI not preserved" position is reopened — if project ontologies are preserved as interpretation-essential metadata, the originating class IRI may need to survive as a pointer into the preserved ontology. | — / under review 2026-05-18 |
| 11 | Version model | Versions are encoded in the event log on the write side (a Version exists as the n-th publish event for a Resource). On the read side, materialized Version nodes are projected from events. Version numbers are monotonic integers. `isCurrentVersion` materialized in read store | — |
| 12 | Internal identifiers | UUIDv7-based identifiers in DaSCH-controlled namespace. **Amended 2026-05-15:** internal persistent-identity IRIs use the **URN scheme** `urn:dsp:{type}:{uuid}` (e.g., `urn:dsp:resource:01234567-89ab-...`, `urn:dsp:rep:...`, `urn:dsp:project:...`, `urn:dsp:agent:...`, `urn:dsp:agreement:...`, `urn:dsp:deposition:...`, `urn:dsp:preservation-action:...`, `urn:dsp:event-segment:{period}`). Internal IDs are **not dereferenceable**; they exist purely for cross-event references inside the system. The earlier `https://archive.dasch.swiss/{type}/{uuid}` HTTPS form is retired (rhetorically implied HTTP-resolvability we don't need internally; tied to a domain that may change across rebrands or migrations; Fedora 6 sets the precedent with `info:fedora/...` URNs). **Read-side URLs** served by Access Area subdomains for browser/client access (e.g., `https://dpe.dasch.swiss/resource/{uuid}/v{n}`, `https://iiif.dasch.swiss/{shortcode}/{rep-uuid}/manifest.json`) remain HTTPS URLs and are dereferenceable. **ARKs** are NOT internal identifiers; they are public lookup-and-forwarding handles managed by a separate ARK Resolver bounded context. **Stability layers** (strongest first): ARK → URN internal IRI (stable within a system; not promised across system migrations) → Read-side URL (read-store contract; honoured as long as the event log is replayable). | — / amended 2026-05-15 |
| 13 | ARK strategy | ARK Resolver is its own bounded context with its own event store. ARKs may be (a) supplied by RDU-Tooling for migration of VRE-era ARKs (DSP-version `1`); (b) supplied by RDU-Tooling for reservation before publication (DSP-version `2`); (c) auto-minted by Resolver on publish events (DSP-version `2`). VRE-era ARKs migrated as one-time bulk registration. Value-level ARKs map to DPE deep links; no new Value ARKs minted by Repository | — |
| 14 | Commands vs. Events (CQRS-ES) | Commands are intents that may be rejected; Events are facts emitted only after successful Command validation. Validation (incl. collision detection, uniqueness checks) happens at Command time. DAO models Events and persistent-identity entities; Commands are transient and not preserved | — |
| 15 | Top-level DAO class list (write side only) | `dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event` (with subclasses), `dao:Deposition`, `dao:DepositAgreement`. (`dao:PreservationAction` added later as decision 26.) Version classes (`dao:ResourceVersion`, `dao:RepresentationVersion`) explicitly **NOT** in DAO — they are read-side projections. Relationships are properties, not reified. `dao:AccessRights`, `dao:Place`, `dao:Concept` parked pending project metadata work | — |
| 16 | Events vs. Version nodes (CQRS separation) | Events are the source of truth (write side, in DAO). Version nodes are materialized projections (read side, NOT in DAO). The read store guarantees Version-suffixed URL contracts as long as event log is replayable. Version node properties are derived from events; the projection is regenerable by replay | — |
| 17 | Archive Component build-vs-buy posture | **Working assumption: self-built.** DaSCH owns the Archive implementation; DAO is the storage model directly; no anti-corruption layer required. (Earlier working assumption was commercial/Docuteam; revised after right-sizing analysis showed DaSCH's scale and operational simplicity arguments favor self-built with OCFL-on-filesystem.) | — |
| 18 | DepositAgreement and AccessRights renaming | Submission Agreement → `dao:DepositAgreement` (DaSCH internal naming; may be a link to the document store). AccessRule → `dao:AccessRights`. The straightforward COAR cases (open, embargoed, metadata-only) are simple properties; `restricted_access` is non-trivial and parked as open Q12 | — |
| 19 | ARK is the single long-term stability commitment | ARKs are minted **per persistent-identity entity** (Resource, Representation, Project, etc.), not per Version. A specific Version is denoted by suffix (`/v{n}` or VRE-era timestamp). ARK string remains stable; suffix selects the Version. DPE handles Version display by querying the read store. Internal IRIs and read-side URLs are not promised to outlive a system migration; only ARKs are | — |
| 20 | Bitstream storage | Bitstreams stored outside event log in a content-addressable store. Events carry hash references (multihash format, SHA-256 default) plus PREMIS-style file-level technical metadata in payload. Storage URIs are not in event payloads; the content store owns "given a hash, return bytes". **Deviation from PREMIS, documented (per decision 27):** PREMIS models `Representation → File → Bitstream` as three `Object` subclasses; DAO collapses `File` and `Bitstream` into facts in event payloads + content-addressed bytes. Reasoning: event sourcing makes `File`-as-class redundant; content addressing makes `Bitstream` identity = hash. Reversible — file-facts can be projected into read-side `File` nodes if needed. **Amended 2026-05-15 (decision 33):** the deviation is preserved at the class level (no `dao:File`, no `dao:Bitstream` class). What is clarified post-Fedora-research: per-File information that was previously described as "living only in event payloads" is also rendered as blank-node-structured RDF on the Representation in OCFL (`rep.nt`, or optional `<file>~desc.nt` sidecars), so File information is navigable from OCFL alone without consulting the event log. The bytes themselves remain content-addressed (multihash); `dao:Bitstream` as a separate class is still not needed. See §3.6 | — / deviation logged 2026-05-12 / amended 2026-05-15 |
| 21 | Storage architecture | OCFL on filesystem as source of truth for everything: bitstreams, RDF metadata, event-log artifacts. SQLite as rebuildable read-side cache. No separate event-store technology. Write path: validate command → write event to OCFL → fsync → update SQLite. Recovery: scan OCFL on startup for events not yet in SQLite, replay | — |
| 22 | Right-sized for DaSCH scale | Architecture deliberately right-sized for 50-100 projects/year. Single-machine, two storage things (filesystem + SQLite file). No infrastructure to scale or operate. Replaceable with heavier infrastructure later if scale demands change, without changing OCFL or events. **Amended 2026-05-15:** baseline re-calibrated from "~tens of events/week" to **~200K events/week steady-state, deposit-burst-driven, with single Depositions reaching ~500K events at the extreme** (per decision 31). Still right-sized for single-machine, but the "trivial polling cadence" claim has been removed — push via SSE has replaced polling (decision 31). **Further re-baselined 2026-05-15 (sizing pass):** the earlier amendment misread deposit volume as a weekly figure. Correct values are **~100 deposits/year (one-shot pattern; the 50-project VRE backlog migrates into the same 100/year flow over ~2 years; iterative-via-refunding counted as new projects), ~200K events per deposit, 20M events/year ingest steady-state, ~50 TB/year storage.** Fixity dominates from year 5 onward under decision 40 (per-Object cadence): year-10 ~120M events/year, year-20 ~220M events/year, cumulative ~2.4 B events at year 20, cumulative storage ~1 PB (**PB-scale at the 20-year horizon**). Single-machine architecture still holds; cold-replay-from-genesis at year 20 is the first number that genuinely presses the ship-and-measure principle (§9.5) toward the deferred bulk-replay optimisation. SQLite reference in the original decision text superseded by decision 38 (Redb). | — / amended 2026-05-15 / further amended 2026-05-15 |
| 23 | Event log granularity | **Amended 2026-05-15:** the original weekly time-bucket granularity (`events/{yyyy-Www}/`) has been replaced by **per-aggregate OCFL Objects** (`depositions/{uuid}/` and `preservation-actions/{uuid}/`), each bundling its aggregate's events together with the aggregate's descriptive metadata. A complementary append-only chronological event-index (`cache/event-index/{yyyy-mm}.ndjson`) provides reverse-lookup for SSE replay. Reason for the change: at re-baselined scale (~200K events/week with deposit bursts up to ~500K), weekly time-buckets became unbounded and filesystem-painful; per-aggregate Objects are causally meaningful (decision 26) and naturally bounded by the aggregate. See §3.7. | — / amended 2026-05-15 |
| 24 | Events carry full snapshots | Even though OCFL stores per-version Resource/Representation state, events carry full metadata snapshots (not just references). Redundancy is intentional: preserves replay-from-events-alone capability for disaster recovery | — |
| 25 | Fixity event vocabulary | One `FixityChecked` event with `outcome` ∈ {`pass`, `fail`, `missing`}. Failures and missings trigger preservation-action workflows (workflows are policy, parked separately). **Refined 2026-05-15 by decision 40**: granularity per OCFL Object (not per File); variant payload by outcome (pass = ~250 byte attestation; fail/missing = ~2-5 KB forensic detail); cadence continuous-when-idle + ≥1 sweep/Object/year; ISO 16363 §4.4.2 / §5.1.2 alignment. | — / refined 2026-05-15 |
| 26 | `dao:PreservationAction` as a first-class concept | DaSCH-internal preservation actions (format migration, system-ontology migration, fixity-driven re-encoding, bulk metadata correction) are modeled as `dao:PreservationAction`, **separate from `dao:Deposition`**. Rationale: different actor (Archive-as-system-agent vs. Producer); different authorization regime (internal preservation policy vs. `DepositAgreement`); CoreTrustSeal audit-trail separation of producer-induced vs. archive-induced changes (CTS R09/R12); cleaner event vocabulary (`PreservationActionExecuted` parallels `DepositionAccepted`). Aligned with `PREMIS DD §1.4` (Event entity); `OAIS §4.1.3 / §5.1` (Preservation Planning) | 2026-05-11 |
| 27 | OAIS + PREMIS alignment as default | Every DAO design choice is aligned with OAIS (CCSDS 650.0-M-3, Dec 2024) and PREMIS 3.0, **or** carries a documented deviation with reason in the decision log or an ADR. Standards extracts checked in under [`standards/`](./standards/) with citation conventions (`OAIS §<n.n.n>`, `PREMIS DD §<n.n>`). The alignment is **conceptual**: DAO uses its own URIs in the `dao:` namespace and may diverge in property names and structure where DaSCH-specific needs justify it, but the underlying concepts must be traceable back to OAIS/PREMIS or explicitly justified | 2026-05-11 |
| 28 | Tombstoning vs. redaction | Two distinct events. `Tombstoned` = **logical retraction** (bytes/metadata preserved in OCFL; read-side hides the Version; ARK returns tombstone landing page). `Redacted` = **surgical content-level erasure** producing a new redacted Version of the Resource/Representation; the pre-redaction Version is `Tombstoned` and its OCFL bytes are over-written/zeroed — the **single sanctioned exception to OCFL immutability**. `Redacted` records who authorized the redaction and under what legal basis (e.g. GDPR Art. 17), not what was removed. Outright deletion is **not** a DAO event — it is a board-level exception in a separate governance log. Aligned with `PREMIS DD §1.4` Event entity (custom `Redaction` subtype) and `OAIS §3.3.5` (deactivation permitted; deletion not in normal operation). See §5.1 and §3.7 | 2026-05-12 |
| 29 | Domain framing | The whole `dsp-repository` codebase implements **one domain**: Trusted Repository (OAIS-based). Decomposes into subdomains: Ingest, Archival Storage, Preservation Planning, Data Management, Administration, Access (all OAIS functional entities); Identification (DaSCH-specific, long-term citation via ARK); Producer-side preparation (DaSCH-specific). VRE is external to the domain (a Producer in OAIS terms). See `CONTEXT-MAP.md` → Domain | 2026-05-15 |
| 30 | Three-tier preservation chain role vocabulary | The two-tier "Archival Master / Service Master" framing is **retired**. Replaced by a three-tier role taxonomy distinguished by **purpose in the preservation chain**, not by format, location, or source provenance: **Preservation File** (long-term bit-level preservation; lives inside `dao:Representation`; owned by Archive context); **Service File** (mezzanine derivation under a derivation rule; owned by Access Area context); **Access File** (end-user delivery payload generated on demand; owned by the Access Area subdomain that serves the request). Originated in SIPI's IIIF Server vocabulary; promoted to cross-context Published Language. See `CONTEXT.md` → Preservation chain roles | 2026-05-15 |
| 31 | Archive deployment topology and public interfaces | The Archive bounded context is one logical unit deployed as **two services**: an **Ingest Area** (producer-facing async upload + SHACL/`DepositAgreement` validation gate; emits `DepositionAccepted` only on validation success; OAIS *Ingest* entity) and the rest of the Archive (event log, OCFL, public APIs; OAIS *Archival Storage* entity, with future Preservation Planning / Data Management / Administration entities). Both speak DAO directly; no anti-corruption layer between them. The Archive exposes **three public APIs**: Commands (HTTP), Events SSE (`text/event-stream`, resumable via `Last-Event-ID`, **full firehose** with no server-side filtering — subscribers filter client-side), Binary retrieval (`GET /bitstreams/{multihash}`, HTTP 206 Range supported). **OCFL is exclusive to the Archive boundary**; no other context reaches into the OCFL store. Deposition size is **producer-set** (Path A); realistic upper bound ~500K events for an extreme single submission; per-Deposition OCFL Objects are the proposed granularity. See §1a, §9.3. **Amended 2026-05-21 by decision 47**: (i) "Ingest Area" renamed to **Ingest Service** and reframed as the *sole producer-facing surface* (not just a deployment); (ii) wire transport is **gRPC** across all surfaces (HTTP/JSON Commands → gRPC unary `CommandAPI`; SSE Events feed → gRPC server-streaming `EventStream`; HTTP Binary retrieval → gRPC streaming bitstream retrieval); (iii) `CommandAPI` is **internal-only**, accepting mTLS-authenticated principals (Ingest Service; preservation admin tooling) — no longer Producer-facing. The three-API *vocabulary* (Commands, Events, Binary) survives as conceptual framing; the wire and the publicness changed. | 2026-05-15 / amended 2026-05-21 |
| 32 | Access Area as federated subscribers; cold-replay strategy | Access Area is **one bounded context with N independent subscriber services** (one per DIP-shape subdomain: IIIF, HTML/Web Discovery, Custom Presentation, Asset/Download, SPARQL). Each subscriber maintains its own SSE cursor against Archive, its own storage tuned to its consumer's pattern, its own derivation logic. **Cold-replay strategy by use case**: (α) subscriber-side snapshots for routine restarts; (γ) subscriber-to-subscriber replication for spinning up duplicates of an existing subscriber kind; (δ) full SSE replay from genesis for the rare deploy of a brand-new subscriber kind. Archive serves historical events via the same SSE endpoint as live tail; a bulk-replay optimisation is **deferred** until measurement shows it is needed (see §9.5: ship and measure before optimising) | 2026-05-15 |
| 33 | OCFL granularity — Representation as Archival Group; no `dao:File` class | Each `dao:Representation` is stored as one OCFL Object (Fedora 6's "Archival Group" pattern); Files live as content within (**not** as separate OCFL Objects). Avoids the object-proliferation pain documented in Fedora 6 + Hyrax migrations (10+ OCFL Objects per intellectual work; the Samvera community's pivot from ActiveFedora-on-LDP to Valkyrie in Hyrax 5 is the strongest signal that the "every resource = its own Object" pattern was wrong). `dao:File` is **not** a write-side DAO class. Per-File information is rendered as blank-node-structured properties on the Representation's RDF via `dao:hasFile`, addressed by `dao:filename` within the parent Representation Version's context. For event references to a specific File: use the (Representation Version IRI, filename) tuple. Default on-disk layout: per-File information inline in the Representation OCFL Object's `rep.nt`; Fedora-style `<file>~desc.nt` sidecar pattern is an optional convention deferred until operational need surfaces. Amends decision 20 (deviation narrowed: `dao:File` *not* reintroduced as a class; storage rendering clarified). | 2026-05-15 |
| 34 | RDF serialisation at rest — N-Triples | DAO RDF metadata stored as **N-Triples** inside OCFL Objects (matching Fedora 6's canonical at-rest format). Turtle / JSON-LD accepted on the Commands and Events APIs for content negotiation; normalised to N-Triples on write at the API boundary. Rationale: line-oriented, prefix-free, deterministic byte representation. Critical for OCFL fixity hashes over RDF content; identical triples produce identical bytes regardless of who serialised them. Diff-friendly across OCFL versions; migration-safe. Turtle's prefix table is global state that can change serialisation without changing semantics; bad inside an immutable OCFL version. | 2026-05-15 |
| 35 | Explicit non-adoption of LDP and PCDM as modelling primitives | Following the Samvera community's documented pivot from ActiveFedora-on-LDP to Valkyrie (Hyrax 5, 2024), DAO **does not adopt** LDP containers (DirectContainer, IndirectContainer, BasicContainer) or PCDM (Collection / Object / File / Hash) as modelling primitives. The Hyrax/Samvera experience documents real pain: LDP's one-parent containment constraint, tombstoned URIs blocking ID reuse, chatty per-resource API, and PCDM's conflation of intellectual structure with storage structure. OAIS + PREMIS + DAO already cover containment (Resource → Representation → Files-within-Rep) and provenance (PreservationAction) with cleaner semantics. The single LDP idea that survives at the storage layer is the Fedora 6 sidecar pair pattern (`<file>` + `<file>~fcr-desc.nt` within one OCFL Object) — available as an optional on-disk convention but not required. The DAO model stays storage-agnostic, in line with Valkyrie's lesson. | 2026-05-15 |
| 36 | Two-substrate storage architecture: append-only event log + OCFL entity storage | The Archive uses **two physical storage substrates**, both ZFS-backed. **Event log substrate** (source of truth for events): append-only NDJSON-of-JSON-LD log, organised as sealed segment files; segments seal on a hybrid trigger (monthly OR ~100 MB, whichever first); active (unsealed) segments live in `event-log-active/` outside OCFL; each sealed segment becomes one OCFL Object in a dedicated **event-log-storage-root** (OCFL storage root #1, proper OCFL with namaste + layout + extensions). **Entity storage substrate** (source of truth for entity state and Preservation File bytes): a separate **entity-storage-root** (OCFL storage root #2) containing one OCFL Object per DAO state-aggregate entity (`urn:dsp:resource:{uuid}`, `urn:dsp:rep:{uuid}`, `urn:dsp:project:{uuid}`, `urn:dsp:agent:{uuid}`, `urn:dsp:agreement:{uuid}`) and one OCFL Object per commit aggregate (`urn:dsp:deposition:{uuid}`, `urn:dsp:preservation-action:{uuid}`, holding audit records). Each entity Object versions on curatorially-meaningful state changes; Preservation File bytes live inside the relevant Representation Object's content area. The two substrates are co-written atomically on commit (write-ahead event, then OCFL state, with reconciliation on crash recovery). Replaces the earlier "single OCFL store with commit-aggregate Objects bundling events + bytes" proposal (which had no prior art and was flagged by research as misaligned with both OCFL conventions and CQRS-ES practice). Grounded in: Fedora 6's OCFL-as-source-of-truth pattern; Datomic / EventStoreDB / Kafka separation of event log from large blobs; standard CQRS-ES stream-per-aggregate addressing. **Amends decisions 21, 31, 33** (storage architecture revised; per-aggregate granularity changes; OCFL no longer bundles events with bytes). | 2026-05-15 |
| 37 | Event-log on-disk format | Active log: **NDJSON of JSON-LD events**, one event per line. Each line is a self-contained JSON-LD document with explicit fields: `@id`, `@type`, `global_offset`, `stream_id`, `stream_version`, `timestamp`, `event_schema_version`, plus event-type-specific payload. Each line ends with a `crc32` field over the line's content for per-event corruption detection. **Per-segment fixity**: SHA-256 manifest in standard Unix `[hash]  [filename]` format (compatible with `sha256sum -c`), file extension matches hash type (`.sha256`). **Durability**: fsync per event during deposit bursts (latency is async by design per decision 31); batched fsync (every N events or M ms) acceptable outside bursts; atomic appends with truncation to last valid CRC32 on crash recovery. **Schema evolution**: `event_schema_version` field per event; readers maintain `(event_type, schema_version) → reader_function` mapping for forward compatibility. **Narrows decision 34**: N-Triples is the at-rest format for **entity state RDF in OCFL Objects**; NDJSON-of-JSON-LD is the at-rest format for **events in the event log** (and the wire format on the SSE feed). Both serialise the same RDF data model; choice is operational. | 2026-05-15 |
| 38 | Cache DB technology — Redb | The rebuildable cache (`cache/`) uses **Redb** (pure-Rust embedded B-tree KV store with ACID + MVCC) for three indexing workloads: (a) read-side projection cache of entity state for queries; (b) event-log index (`(stream_id, stream_version) → segment_id + byte_offset`, `global_offset → segment_id + byte_offset`); (c) bytes index (`multihash → containing-OCFL-Object + content-path`). Rationale: **pure Rust** (no C FFI dependency); **B-tree** fits our workload (~10M events/year peak; point-lookups dominate; range scans bounded; bursty but not high-throughput writes; predictable tail latency); **ACID + MVCC** for atomic projection updates during multi-event commits; **typed tables** catch a class of bugs at compile time; **no SQL needed** (query patterns are all known in advance). Schemas designed technology-neutrally for a future engine swap if needed (everything in `cache/` is fully rebuildable from the two substrates). Considered and rejected: **SQLite** (C FFI; pure-Rust preference outweighs SQL convenience at our query-pattern complexity); **TursoDB / Limbo** (too young for a multi-decade preservation system; revisit if mature in 5 years); **Fjall** (LSM over-engineered for our write rate; B-tree more predictable); **RocksDB** (heavy C++ dependency). | 2026-05-15 |
| 39 | Fixity baseline; non-repudiation deferred | **Adopted baseline**: SHA-256 sidecar fixity files in standard Unix `[hash]  [filename]` format (compatible with `sha256sum -c`); file extension matches hash algorithm (`.sha256`). Per-event CRC32 inside event-log segment NDJSON lines for finer-grained corruption detection. **Explicit threat model**: the sidecar fixity baseline defends against accidental disk corruption, bit rot, and unauthorised viewers without write access. It does **not** defend against an attacker with write access to the underlying storage (internal admin, compromised process), who can modify both the data file and its sidecar. **Non-repudiation deferred** to the CoreTrustSeal evidence pass (Q8 in §9.4) with two concrete approaches identified: **(a) HMAC + HSM** — write-time authentication using a key held in a non-extractable Hardware Security Module (commercial HSM, or YubiKey / Nitrokey / GnuPG smartcard for cost-conscious setups); **(b) Merkle Tree log structure with external root publication** — events form an append-only Merkle tree; root hashes published externally on a regular cadence (RFC 3161 TSAs, OpenTimestamps, peer-institution mirrors); tampering invalidates the tree from the tampered event onward, with externally-published roots providing cryptographic proof of what the tree looked like at publication time. Both approaches compose. Implementation deferred per the **ship and measure** principle (§9.5); the architecture admits both additions as additive layers. | 2026-05-15 |
| 40 | Fixity policy: per-Object granularity, continuous cadence, variant payload | **Granularity**: one `FixityChecked` event per OCFL Object (= per `dao:Representation`) validate, not per File. Matches `ocfl validate` as the operational unit; aligns with PREMIS Event semantics (Events bind to PREMIS Objects, and `dao:Representation` is the Object-level entity per decision 33). **Cadence**: continuous fixity sweep running in idle I/O windows, with a hard commitment of **at least one sweep per OCFL Object per year**. Aligns with `ISO 16363 §4.4.1.2` (*"The repository shall actively monitor the integrity of AIPs."*) and `ISO 16363 §5.1.1.3` (*"The repository shall have effective mechanisms to detect bit corruption or loss."*). Fail/missing reporting workflow aligned with `ISO 16363 §5.1.1.3.1` (*"The repository shall record and report to its administration all incidents of data corruption or loss..."*). The continuous-when-idle + ≥1/year cadence exceeds these requirements' documented-policy-consistently-executed bar. Produces CoreTrustSeal R10 / R11 evidence. **Variant payload by outcome**: `pass` = minimal attestation (~250 bytes — Object IRI, version, timestamp, outcome); `fail`/`missing` = per-File forensic detail (~2-5 KB — filename, expected hash, recomputed hash, per-File result, checker version). Both shapes belong in the WORM event log: routine pass-records are audit-grade attestation that monitoring happened on that date; fail/missing records are forensic evidence (and satisfy §5.1.1.3.1 reporting). **Operational consequence**: at year-20 scale (200M Reps) fixity events total ~52 GB/year (vs. ~1 TB/year if uniform forensic-detail were carried). Operational metrics (sweep progress, throughput, queue depth) live in Grafana per decision 41 — operational, not archival. **Refines decision 25** (which only specified the outcome enum); aligned with decision 39's per-event CRC32 baseline. | 2026-05-15 |
| 41 | Grafana `/metrics` endpoint as operational surface; not an Archive public API | The Archive exposes a `/metrics` endpoint scraped by Grafana for operational observability — event rate, fixity sweep progress, queue depth, OCFL Object counts, disk usage, write latency, SSE subscriber lag. This is **operational telemetry, not archival evidence**: Grafana retention is finite, the endpoint can be replaced or reset without affecting the preservation commitment, and no Producer or Subscriber should depend on it as a contract. The three public APIs (Commands, Events SSE, Binary retrieval — decision 31) remain the bounded-context boundary; `/metrics` is a fourth surface internal to operations. **Audit-grade evidence of monitoring activity lives in the event log** (e.g., routine `FixityChecked` pass-events per decision 40), not in Grafana. The two layers are deliberately separate: the event log is durable and tied to the preservation commitment; Grafana is operationally convenient and disposable. **Clarifies decision 31** (which named three public APIs without addressing operational telemetry). | 2026-05-15 |
| 42 | Certification pyramid (CTS → nestor → ISO 16363); preserve-optionality principle | DaSCH targets the trustworthy-repository certification pyramid in difficulty order: **tier 1 CoreTrustSeal** (community self-assessment + peer review; entry level; first target), **tier 2 nestor Seal** (German Kriterienkatalog v2 2008; documented self-assessment + nestor peer review; middle), **tier 3 ISO 16363** (formal external audit by ISO-16919-conforming body; top tier). The substantive technical requirements of tier 3 dominate the others; **design choices are made against tier 3 to avoid foreclosing future certification.** Working principle: where a tier-2 or tier-3 requirement implies a technical capability we cannot implement immediately, the architecture remains *open* to adding that capability as an additive layer — implementation is deferred, optionality is not. **Concrete commitments under this principle**: (i) geographic disaster-recovery backup is architecturally committed but deployment-deferred — both OCFL roots and the active-log directory are designed to replicate via filesystem-level mirroring (`zfs send` to a distant ZFS pool, or `rsync` with content-addressed verification); Redb is rebuildable on the remote and need not be replicated. Required by `nestor §14` (institutional-main-building-disaster must not destroy objects) and implied by `ISO 16363 §5.1.2` + `§5.2.1` (number/coordination/location of copies + risk analysis of insufficient distancing). (ii) HMAC + HSM event authentication and Merkle-tree log with external root publication remain architecturally additive (decision 39 names them). (iii) Digital signatures on DIPs (`nestor §7.3`) are an Access Area concern; subdomain-level signing is additive. (iv) Significant Properties (`nestor §9.2`) can be added later as Rep properties or a new event type; no current architectural lever blocks them. (v) On-demand BagIt / RO-Crate AIP serialisation can mitigate the constituted-AIP deviation (decision 27 / §6.2) if auditors push back — implementation only if asked. **Items that are organizational, not architectural** — Designated Community spec, Producer-Archive agreement detail, financial sustainability evidence, succession planning, security risk analysis — fall outside DAO's scope but are flagged here so the architecture-vs-organisation seam is explicit. | 2026-05-16 |
| 43 | DaSCH preservation-level commitment: uniform A + C + D across all projects | **DaSCH's mandate commits the Archive to CTS Levels A + C + D uniformly** (per the *CTS Curation & Preservation Levels Position Paper v3.0*, 2024 — **D** Deposit Compliance + **C** Initial Curation + **A** Active Preservation; levels are cumulative). The commitment applies to **every project, every Deposition, every preserved Resource / Representation / File**. No per-DepositAgreement, per-Deposition, or per-Resource variation. **Consequences for DAO**: (i) `dao:DepositAgreement` (decision 18) does **NOT** carry a preservation-level property — the level is institutional invariant, not contractual variable. (ii) Event payloads do **NOT** carry `premis:preservationLevel` overrides — there is nothing to override. (iii) No `CurationLevelChanged` event in the vocabulary — there is no level transition to record. (iv) The Position Paper's Z / D / C / A taxonomy is documented as a reference vocabulary in `standards/` but not modelled as a tunable in DAO. **Rationale**: institutional simplicity (a single uniform commitment is easier to audit, operate, and communicate than per-content variation); fits DaSCH's funding model (SNSF-funded mandate applies uniformly to projects in scope); aligns with DaSCH's value proposition (expert curation for all preserved content). Auditors get a single, simple commitment claim that maps directly to CTS R08 / R09 / R10 ("curation levels defined during appraisal" / "responsibility for preservation defined" / "variations for different curation-levels" — DaSCH's response: "no variations; uniform A+C+D"). **Resolves Q15.** **If future DaSCH offerings introduce true tiered service** (e.g., a self-service Z-level publication-only offering for non-SNSF projects), this decision must be revisited; until then, the architecture is deliberately not designed for per-content variation. | 2026-05-16 |
| 44 | Access rights explicit per Resource Version + per Representation Version; no propagation in DAO | Every `dao:Resource` Version and every `dao:Representation` Version carries an explicit `dao:accessRights` property in its event-payload snapshot (consistent with decision 24 — events carry full snapshots, not deltas). **DAO does NOT propagate access rules** across `kb:isPartOf` hierarchies, Resource → Representation linkages, or any other relationship. **No application logic, no inheritance, no composition in DAO.** Composition rules (most-restrictive-wins, parent/child propagation, the contemplated "two access spaces" partitioning — open vs. authorized-restricted) are read-side projection concerns implemented at the γ delivery layer (Q12.1, 2026-05-15). Rationale: (i) DAO is the write-side schema; computation belongs to read-side projections (§1b / §3.4 precedent for Version nodes). (ii) Future Producers other than VRE may not produce `kb:isPartOf` hierarchies at all — DAO shouldn't presuppose VRE's compound-Resource pattern. (iii) Future multi-parent / deep-chain models (`kb:Representation` referenced by multiple `kb:Resource`s; `Res → Res → Res → Rep`) make any propagation rule semantically ambiguous; explicit per-entity rules sidestep this entirely. (iv) The γ-projection design has full flexibility to evolve (e.g., the open-vs-authorized two-space partitioning) without requiring DAO changes — the projection groups explicit DAO facts however institutional policy decides. **Two-level positional scheme** in DAO (Resource + Representation); File-level rules deferred as additive (decision 33 makes per-File information blank-node-structured via `dao:hasFile` — adding `dao:accessRights` to those blank nodes is additive when needed). **Resolves Q12.2 sub-question 1.** Remaining sub-questions of Q12.2: sub-q-2 (structure of `restricted_access` — simple property vs. `dao:AccessPolicy` class), sub-q-3 (audit-trail mechanism for delivery-layer decisions). | 2026-05-18 |
| 45 | Access policy structure: `dao:AccessPolicy` as first-class entity with opaque policy content | The COAR `restricted_access` case (§4.4) is handled by linking from a Resource Version / Representation Version to a first-class **`dao:AccessPolicy`** entity. The simple cases (`open_access`, `embargoed`, `metadata_only`) remain simple property values; only `restricted_access` requires the policy link. **Schema additions**: (i) new top-level class `dao:AccessPolicy` (grows decision 15 class list to 9 — `dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event`, `dao:Deposition`, `dao:DepositAgreement`, `dao:PreservationAction`, `dao:AccessPolicy`); (ii) new URN type `urn:dsp:access-policy:{uuid}` (extends decision 12 amendment's enumeration); (iii) new event types in §5.1: `AccessPolicyCreated` (a new policy entity comes into existence) and `AccessPolicyRetired` (a policy entity is taken out of use; existing references continue to resolve historically — WORM); (iv) new property `dao:hasAccessPolicy` on Resource / Representation Versions linking to the policy entity. **The `dao:AccessPolicy.policyContent` is an opaque RDF blob** — DAO carries the typed reference; the policy ontology choice (ODRL / PREMIS rights / XACML / DC terms / DaSCH-custom) is deferred to Q19 as a research-needed sub-decision. **Pattern reuse**: parallels `dao:DepositAgreement` (decision 18 — first-class entity, may store a link to executed agreement) and Q16 option (a) for project ontologies (`dao:hasOntology` linking to OCFL-packaged TTL). This is now a recurring DAO idiom: "DAO has a typed reference to a preserved artefact; the artefact's internal schema is opaque to DAO." **Aligned with decision 44**: DAO stores explicit facts; γ-projection interprets the policy content. **Multiple Resources/Reps can reference the same `dao:AccessPolicy`** — project-level policies are common (one rule for all of project X's restricted content). **Policy lifecycle**: policy creation emits `AccessPolicyCreated`; policy retirement emits `AccessPolicyRetired` (the policy entity Version remains in OCFL, historical references continue to resolve). **Resource/Rep access-rule changes** emit `AccessRuleChanged` (existing event, §5.1) on the affected Resource/Rep — this produces a new Resource/Rep Version with the updated `dao:accessRights` value and/or `dao:hasAccessPolicy` link. **Amends decision 15** (class list grows) and **decision 12 amendment** (URN-type enumeration grows). **Resolves Q12.2 sub-question 2.** Remaining sub-questions of Q12.2: sub-q-3 (audit-trail mechanism for delivery-layer decisions). | 2026-05-18 |
| 46 | Access-decision audit-trail boundary: DAO carries policy lifecycle; γ-per-access logs are operational telemetry | The Archive's WORM event log carries **policy lifecycle**, not **per-access decisions**. The audit-grade trail of access conditions is composed of three event types already in §5.1: `AccessPolicyCreated` (a new policy entity comes into existence — content snapshot per decision 24); `AccessPolicyRetired` (a policy is taken out of active use; historical references continue to resolve — WORM); `AccessRuleChanged` (a Resource/Rep entity's access rule changes — new COAR value and/or new `dao:hasAccessPolicy` link, producing a new Version per §3.1). Together these answer the audit-grade question *what access conditions applied when*. **γ-per-access logs** (who tried to access what when, was it granted/denied) are **operational telemetry** with finite retention, analogous to decision 41's Grafana-metrics framing. They are not pulled into the Archive event log. **Rationale**: (i) audit-grade evidence asks *what was the policy state at time T*, not *was every individual access attempt logged* — CTS R13 / nestor §11.2 / ISO 16363 §4.5 ask for documented machine-readable access conditions per AIP, which the policy lifecycle satisfies. (ii) γ-per-access event volume would dwarf preservation events by orders of magnitude — every page view, every IIIF tile request, every SPARQL query would be a candidate event; this is operational, not archival. (iii) Consistent with decision 41's Grafana-vs-event-log split — operational telemetry and audit-grade preservation evidence are deliberately separate planes. **Organisational alternative for γ-side logs**: DaSCH may choose to retain γ access logs in an organisational evidence archive (for breach forensics, FADP / data-protection compliance, etc.); that retention is operational and lives outside the Archive's preservation commitment. **Resolves Q12.2 sub-question 3. Resolves Q12 in its entirety.** Q12 sub-resolutions: Q12.1 (2026-05-15, γ delivery-layer enforcement); Q12.2.1 (decision 44, explicit per-Version `dao:accessRights`; no propagation in DAO); Q12.2.2 (decision 45, `dao:AccessPolicy` as first-class entity); Q12.2.3 (this decision). Downstream open: Q19 (policy ontology selection for `dao:AccessPolicy.policyContent`, parked 2026-05-18). | 2026-05-18 |
| 47 | Producer-facing surface unification: Ingest Service as sole gate; gRPC as Producer-Archive interface; CommandAPI internal-only; no SIP preservation | **Producer-facing architecture sharpened.** Decision 31 named the Archive as a single bounded context deployed as two services (Ingest Area + Archival Storage) and listed three public APIs (Commands, Events SSE, Binary retrieval) without locking the transport. This decision tightens the producer-facing surface and the wire-level commitments. **(1) Ingest Service is the sole producer-facing surface.** All Producer submissions — RDU-Tooling bulk Depositions, self-service preservation frontend metadata-edits (Q21), future Producer-side tools — go through Ingest. Everything Producer-side is a SIP: content-bearing (Resources + Representations + bitstreams), metadata-only, or mixed. Ingest runs SIP-shape-appropriate validation (SHACL always; format ID + ClamAV on bitstream-bearing SIPs; `DepositAgreement` always); on success commits via Archive Storage's internal `CommandAPI`. **(2) Wire transport is gRPC** (protobuf + HTTP/2 streaming) across all surfaces. Producer → Ingest: streaming SIP upload + unary commands. Ingest → Archive Storage: unary `CommandAPI` calls + unary `QueryAPI` calls + bitstream streaming for retrieval. Archive Storage → Subscribers: server-streaming `EventStream` (replaces decision 31's SSE on the Events feed). Rationale: strict-typing-prevents-drift via shared protobuf crate compiled into every service in the Rust monorepo; streaming + backpressure built-in for bulk binary; observability via OpenTelemetry + structured logging (auditors do not inspect wire traffic; the audit surface is the event log + documented Producer-Archive interface). **(3) CommandAPI is internal-only.** No longer Producer-facing; accepting authenticated principals (Ingest Service; DaSCH-internal preservation admin tooling) via mTLS. Preservation admin tooling (ARK reservation outside the Resolver, format-migration triggering, GDPR redaction, etc.) gets its own role on the CommandAPI; VPN-scoped at the network layer (operational concern, "we are not a bank" — proportionate to DaSCH's trust profile). **(4) Archive Storage self-defends** by re-validating every command regardless of source. Ingest's edge validation is pre-validation for fast failure; the Archive's re-validation is defense-in-depth (cheap relative to OCFL writes). ClamAV is the one validator that runs *only* at Ingest (bytes in the WORM substrate are not re-scanned). **(5) No SIP-as-submitted preservation in the WORM event log.** SIPs live on Ingest for an operational backup window (Ingest-local spool + offsite backup); when the window closes, they are discarded. OAIS does not require SIP preservation (SIPs → AIPs; AIPs are what's preserved); decision 24 guarantees events carry full snapshots, so replay-from-events covers replay-from-SIP. The Producer-Archive interface is documented as the protobuf schema (CTS R02 / R08 evidence). **Trust topology.** Producer side (untrusted): RDU-Tooling, self-service preservation frontend → gRPC SIP submission. Edge gate: Ingest Service (sole producer-facing surface; AV + format + SHACL). Archive core (sealed): Archive Storage, accepting commands only from authenticated edge services. Subscriber side: gRPC EventStream → DPE, CPE, ARK Resolver, future subscribers. **Amends decision 31** (transports: HTTP → gRPC; CommandAPI: public → internal-only; SSE → gRPC server-streaming). **Subsumes Q20.** Downstream: Q21 (self-service preservation frontend design) and Q22 (gRPC envelope shape sub-questions, external-Producer translation gateway, mixed-content SIPs from Metadata Editor). | 2026-05-21 |
| 48 | Value representation: flattened simplified-schema payload (read-only RDF value node) | A preserved property/value is **not** a `dao:Resource` and **not** Knora's full value reification. It is a **flattened RDF payload node** following DSP-API's **ApiV2Simple** mapping (the simple-schema lexical/datatype rules: `xsd` literals for Text/Int/Bool/Decimal/Uri/Time; self-contained custom datatypes for Date / Color / Geom / Interval / Geoname / ListNode — e.g. `Date` lexical `"JULIAN:1492 CE"`), retained as a **thin read-only node carrying `valueHasUUID`** (stable sub-resource citation target + standoff anchor). The rest of Knora's value-level machinery (`hasPermissions`, `attachedToUser`, `valueHasOrder`, `isDeleted`, `previousValue`) is pruned as VRE working state (§4.3). **Reuses DSP-API's existing, maintained transformation rules** (`ValueContentV2.toJsonLDValue(ApiV2Simple)`, `OntologyConstants.KnoraApiV2Simple`, `KnoraBaseToApiV2SimpleTransformationRules`) rather than reinventing flattening — the full mapping is what appears in a raw VRE export; the simple mapping is the flatten. **Archival overrides** where the retrieval-optimised simple schema is lossy for *preservation* (a retrieval API ≠ an archive): (i) **standoff markup is preserved as XML inside the value blank node** — plus the XSLT as a second value for custom mappings (decision 51); (ii) **list values are kept as the list-node IRI**, resolved against the project's closed vocabulary, which is preserved as part of the project ontology (decision 51, Q16); (iii) **file values are NOT flattened to a single URL** — they become `dao:Representation` with full file metadata (decisions 20 / 33). Cite: `OAIS §4.2` (Content Information + Representation Information must remain interpretable to the Designated Community) — the simple-schema lexical forms preserve the human-meaningful Representation Information; only the recomputable JDN is dropped. **Sharpens §4.1 / §4.3** (the "preserve the property/value assertions" clause now has a concrete shape) without reopening them. Resolves the value-granularity branch of Q10. | 2026-06-01 |
| 49 | Versioning strategy is a Producer / RDU-Tooling concern; Archive is version-strategy-agnostic | Knora versions at the **value** level (per-property edits via `previousValue` chains, ordered by `valueCreationDate`); DAO versions at the **Resource** level (`ResourcePublished` events → read-side Versions, §3.4). The version *structure* is **decided Producer-side by RDU-Tooling** and expressed in the **SIP it submits**; the Archive **emits** the resulting `ResourcePublished` events only after the Ingest Service validates the SIP (decision 47 trust topology — a Producer never emits events). The Archive is agnostic to how many Versions a resource gets. This **confirms §3.1** (a Version = a deliberate publication event) by naming RDU-Tooling as the agent of "deliberate publication"; it does **not** amend §3.1. **Two project types, both expressed Producer-side in the SIP**: **(1) needs-versions** — RDU-Tooling walks `previousValue` chains and reconstructs as-of states by `valueCreationDate`, expressing the N successive Resource Versions in the SIP (each pinned to a reconstructed-boundary timestamp); the Archive emits N chronological `ResourcePublished` events on ingest. **(2) no-versions** — RDU-Tooling follows each property → current value, ignores the chains, and submits a SIP expressing a single state; the Archive emits one `ResourcePublished`. **DAO consequence: none.** No versioning property on `dao:DepositAgreement` (the strategy is inferable from the event count; don't model what the institution doesn't commit to, §9.5). The strategy and its reconstruction algorithm are specified in the **RDU-Tooling deposit-preparation manual** (new deliverable), not in DAO. Empirical note (2026-06-01): real exports carry sparse, mostly-incidental value history (incunabula 3 chains, hdm 0, solec 47; 0 hard-deletes), so the needs-versions case is the exception, not the rule. | 2026-06-01 |
| 50 | SIP shape: Producer transforms to DAO-shape; opaque-RDF-in-a-thin-protobuf-envelope | Resolves Q22(a); clarifies §4.3. **(1) Transformation is Producer-side.** Every Producer (RDU-Tooling for VRE data; direct producers otherwise) transforms its source into **DAO-shape** (the flattened value representation of decision 48; the structural normalization, administrative pruning, and vocabulary substitution of §4.3) **before** submission. The Archive's Ingest Service **validates** the SIP against DAO SHACL (plus format-ID + ClamAV on bitstream-bearing SIPs, decision 47) and performs **no model transformation** — it is **Producer-agnostic**; Knora/VRE-specific mapping never enters the Archive core. DAO + its SHACL profile is the single authoritative target; non-conforming SIPs are rejected at the gate. **Re-attributes §4.3**: the three ingest jobs are **Producer responsibilities specified in the `producer-deposit-manual`**, not behaviours of the Archive's Ingest Service. **(2) Wire shape = thin protobuf envelope + opaque RDF + chunked binary.** The protobuf message frames only the *package*: deposit id, `DepositAgreement` reference, `dao:Project` reference, and a **manifest** (per Resource / Representation / File: internal IRI, content multihash, byte size, MIME / format id, and — for the needs-versions case, decision 49 — the ordered Version pins). DAO-shaped descriptive metadata rides as an **opaque RDF payload (N-Triples, decision 34)**; Preservation File bytes stream as **chunked frames**. Protobuf does **not** model DAO classes — avoids schema duality (DAO living in both RDF/SHACL and protobuf) and the graph→tree impedance of RDF multi-typing (§4.2), blank-node value/file payloads (decisions 48 / 33), and external-ontology references. **Rejected**: (1A) Archive transforms raw VRE RDF — breaks Producer-agnosticism, needs a per-producer transformer in the core; (2A) nested DAO classes in protobuf — schema duality + impedance. Cite: `OAIS §2.2` (SIP is what the Producer submits; the Archive forms the AIP) and decision 47 (Ingest validates, Archive emits). Q22(b) external-Producer translation gateway (BagIt) remains deferred until external access is scoped. | 2026-06-01 |
| 51 | Standoff and list-value preservation shapes (refines decision 48 overrides) | Sharpens the two archival overrides decision 48 left open. **(1) Lists.** A list value is archived inside the Resource as the **list-node IRI/ID** (not the label), resolved against the preserved project vocabulary. **Lists are a closed, project-controlled vocabulary and are preserved as part of the project ontology** (Q16); the list definitions are **not** denormalised into the value. Note from real data: in a VRE export the list *definitions* live in the **data graph** (`data.nq` / `admin.nq`), not the ontology file — the Producer lifts them into the preserved project vocabulary so the archived IRIs resolve. Supersedes decision 48 override (ii)'s "IRI + full label path" wording (the label path is redundant once the vocabulary is preserved). **(2) Standard-mapped standoff** (text using the default DSP standoff↔XML mapping): preserved as **one XML value inside the value blank node**, produced by dsp-api's existing standoff→XML serialisation (`textValueAsXml`), with a **whitespace-normalisation fix** applied (logic exists in dsp-api). **(3) Custom-mapped standoff** (text using a project-custom mapping): preserved as **two values inside the value blank node — the XML and its XSLT** — because the custom markup is only interpretable together with the transform that renders it; the XSLT is the Representation Information (`OAIS §4.2`) for the custom XML. **Inlined deliberately (size + sunset rationale, 2026-06-01):** the XSLT files are tiny (≤~5 KB each; 9 distinct mappings in beol, ~30 KB total; ~46 MB even if duplicated across beol's ~13k custom-mapped values) and the custom-mapping feature is **being removed from the VRE after these projects are archived**. So the XSLT is kept **per value** rather than preserved once at project level — we deliberately do **not** build throwaway project-level mapping/XSLT infrastructure for a dying feature (YAGNI). This is the *opposite* call from lists (item 1, preserved once at project level); the asymmetry is intentional — lists are a permanent feature (worth the clean model), custom standoff is deprecated (inline and move on). The XSLT renders the stored XML→HTML; the **mapping** is a deposit-time tool (it produced the XML from standoff tags via `textValueAsXml`) and is **not** preserved. Standard-mapped values carry XML (+ whitespace fix) referencing the system `StandardMapping` (whether to snapshot that system artifact is Q16). Resolves decision 48 override (i). Empirical basis: project 0801/beol (downloaded 2026-06-01). | 2026-06-01 |
| 52 | Fat events; OCFL collapses to bitstreams + open-format event log; embedded store (not off-the-shelf) | Resolves the fat-vs-thin fork. **Events are FAT and canonical** (confirms decision 24, §5): every state-committing event (`ResourcePublished`, `RepresentationCreated`, …) carries the full DAO metadata snapshot + bitstream byte-references (hashes). Entity state (`dao:Resource` / `dao:Representation` RDF) is a **projection of the event log, never separately stored** — removing the descriptive-metadata duplication decisions 24 + 36 created. **OCFL's role collapses to two things:** (1) **Preservation File bitstreams** (content-addressed); (2) the **event log in open, preservation-grade form** (NDJSON-JSON-LD, decision 37) as the durable canonical copy. **decision 36's entity-storage substrate is dropped** (per-entity versioned OCFL state objects + atomic co-write + crash reconciliation — the heavy, novel part). **Store is embedded, not off-the-shelf:** append NDJSON segments + index/project in Redb (37/38) + stream via gRPC `EventStream` (31/32). Off-the-shelf event stores (EventStoreDB/Kafka/Pulsar) **rejected for now** — multi-decade operational dependency for features not yet needed (cuts against decision 22 minimalism), and their native format is never preservation-grade (the preservation copy is always the OCFL open-format log; any operational store would be a rebuildable serving layer). Ship-and-measure (§9.5). **DIP consequence:** the dissemination package is the emitted `EventStream` (fat metadata events) + Binary API (bytes) — **not a reshaped DIP**. **Amends:** 24 (fat confirmed; "redundant with OCFL" dropped — no entity-state OCFL to be redundant with), 33 (Representation OCFL Object holds only its *bytes*; per-File info rides in events), 34 (N-Triples-at-rest applies to event-log dumps + bitstream sidecars, not entity objects), 36 (single substrate: event log + bitstreams), 40 (fixity granularity → per event-log segment + per bitstream object). **Keeps:** 37, 38. Aligns with decision 36's own research (§9.4 item 14: dedicated event stores separate events from large blobs) — fat-events-plus-OCFL-for-blobs completes that separation. §3.7 rewrite + inline amendment notes on 24/33/34/36/40 land in `archiving-manual.md` (layer 3). | 2026-06-02 |
| 53 | AIP is virtual/on-demand; events embed fine-grained checksums; PDI-completeness invariant | Builds on decision 52 (fat events). **No AIP is stored as a package.** The preserved substrate is the event log + content-addressed bitstreams; an AIP is a *view* materialised on demand. **AIP unit = a Resource** (OAIS **AIU**): its event stream (its `ResourcePublished` versions + `AccessRuleChanged` + the events of the Representation Versions it pins) ∪ the referenced Preservation File bitstreams. **Project / Deposition = AIC** (Archival Information Collection). **Events embed fine-grained fixity:** every referenced bitstream by **multihash** (per-file content checksum — decision 20) *and* a **per-event payload checksum** (SHA-256, consistent with decision 39) over the canonical RDF snapshot — so each AIU is independently verifiable and an on-demand AIP carries a complete fixity manifest. (Complements decision 37's per-line CRC32 + per-segment SHA-256.) **PDI-completeness invariant:** events must carry sufficient Preservation Description Information per AIU — Provenance (event history), Fixity (embedded checksums + `FixityChecked`), Reference (URN/ARK), Context (relationships), Access Rights (`dao:accessRights`/`dao:hasAccessPolicy`) — that a complete, verifiable AIP can be reconstituted even though none is stored. **On-demand serialisation** to BagIt / RO-Crate for export, audit, succession, display (decision 42). A future **Archive-side GUI** displays AIPs on demand (a consumer of read-model A, decision 54). **Documented deviation (decision 27):** OAIS requires the *information* preserved, not a physically constituted AIP; we preserve events + bytes and constitute the package on demand. Resolves topic (1) of the 2026-06-02 storage discussion. | 2026-06-02 |
| 54 | Command validation: two-tier (stateless Ingest / stateful Archive); one Redb, two read models | Operationalises decision 14 (commands are rejectable intents; events are post-validation facts) for the fat-events architecture. **Tier 1 — Ingest Service (stateless, Producer-facing):** SHACL *submission* profile + format-ID + ClamAV (decisions 47/50); self-contained, needs no archive state. **Tier 2 — Archive command handler (stateful, authoritative):** referential integrity (do referenced `dao:Resource`/`dao:Representation`/`dao:AccessPolicy` exist? is the pinned Representation Version real? policy not retired — decision 45?), domain invariants (no publish onto a `Tombstoned` entity, etc.), optimistic concurrency (`expected_stream_version`, decision 37) — validated against a read-model. **Only Tier 2 emits events.** **Validation read-store = Redb (decision 38):** ONE Redb instance, **two read models** — **(A) command-validation + Archive GUI** (the authoritative write-side consistency view; also feeds the on-demand AIP-display GUI of decision 53); **(B) access-area queries** (serves the synchronous `QueryAPI`, decision 47). Both rebuildable from the event log. Read-model B **complements, not replaces,** decision 32 (Access Area subscribers still build their own stores from the async `EventStream`; B serves synchronous point queries). **All mutations are commands to the internal `CommandAPI`** (decision 47): **Producer path** = Ingest → commands; **Preservation Management actions** (format/ontology migration, Tombstone, Redact, bulk correction — decisions 26/28) = admin tooling → commands *directly*, skipping Tier 1 (not SIPs) but still passing Tier 2. Both converge on the single stateful gate. **Refines decision 47** (the Archive's "self-defending re-validation" is this deep referential/invariant/concurrency check against read-model A, not just re-running Ingest's SHACL). Resolves topic (2) of the 2026-06-02 storage discussion. | 2026-06-02 |
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
| 43 | DaSCH uniform A+C+D preservation commitment (no per-project / per-Deposition / per-Resource variation) | R08, R09, R10 + Position Paper v3.0 | §1.2, §6, §7, §8 | §3.3.1, §4.3, §4.4.1.1 |
| 44 | Access rights explicit per Resource Version + per Representation Version; no propagation in DAO (composition is read-side γ-projection concern) | R13 (reuse conditions documented + machine-readable per entity) | §11.2 (access conditions documented), §6.3 (access integrity) | §4.5 (access management — repository shall associate each AIP with access management conditions); §3.5.1 (rights, terms and conditions for use) |
| 45 | `dao:AccessPolicy` as first-class entity with opaque policy content (resolves COAR `restricted_access` case) | R13, R09 (access conditions documented + machine-readable; preservation-plan responsibility for rights) | §11.2 (access conditions documented), §10.4 (preservation actions on rights changes) | §4.5 (access management), §3.5.1 (rights, terms and conditions) |
| 46 | Access-decision audit-trail boundary: DAO carries policy lifecycle (policy / rule lifecycle events are audit-grade); γ-per-access logs are operational telemetry (not pulled into the WORM event log) | R13 (machine-readable access conditions per AIP — the policy lifecycle answers "what conditions when"), R07 (provenance of access-policy changes) | §11.2 (access conditions documented), §6.3 (access integrity audit-grade trail) | §4.5 (access management), §3.5.1.1 (rights and terms documented per AIP) |
| 47 | Producer-facing surface unification (Ingest Service as sole producer-facing surface; gRPC as Producer-Archive interface; CommandAPI internal-only; SHACL + ClamAV + format ID validation gate; no SIP preservation) | R02 (Producer-Archive agreement; protobuf schema is the machine-readable interface spec), R08 (deposit specifications documented), R11 (workflows that prevent unwanted modifications — the validation gate), R14 (storage integrity — Archive self-defending re-validation) | §3.1 (Producer-Archive interface), §6.1 (Ingest workflow), §13 (technical infrastructure — single hardened producer-facing surface) | §3.5.1 (Producer-Archive agreement), §4.1.1 (Ingest workflow ensures submissions conform), §4.1.1.3 (verification of completeness and correctness on ingest) |

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

- **DAO classes (write side)**: `dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event`, `dao:Deposition`, `dao:DepositAgreement`, `dao:PreservationAction`. Listed in §6.
- **Read-side terms (NOT in DAO)**: Resource Version, Representation Version, `versionNumber`, `isCurrentVersion`. Materialized by the read store; schema is the read store's concern.
- **Knora-base, project ontology** — VRE concerns, named here only to describe what gets transformed away during ingest. The question of whether project ontologies are themselves preservation-essential is parked as Q16 in §7.
- **DAO** — DaSCH Archival Ontology, OWL + SHACL. Conceived as a simplification of `knora-base` (decision 9 amended, 2026-05-18). Built on PREMIS (decision 6 amended); every DAO class is conceptually reducible to a PREMIS or OAIS concept (decision 27).
- **Naming history.** The working term **Asset** (VRE-side meaning) was superseded by **Representation** to align with PREMIS. The DaSCH-side term for the IE-tier entity was originally **Resource** (in early discussion), then **IntellectualEntity** (decision 9, 2026-05-12, adopting PREMIS verbatim to avoid `knora-base:Resource` collision), then **Resource** again (decision 9 amended, 2026-05-18, to make the simplification-of-knora-base framing explicit; collision managed by namespace-prefix prose discipline). The informal label **Archival Master / Service Master** is retired in favour of the three-tier role vocabulary (decision 30): **Preservation File / Service File / Access File**.

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
- Q12.2 access-rights composition at Resource / Rep / File levels — initially proposed first-class `dao:File` class; replaced by blank-node `dao:hasFile` properties on Representation (decision 33); then paused entirely as the File-identity question opened the deeper storage-architecture rewrite. **Resume here** in the next session (see §9.6 below).

#### Session of 2026-05-15 (sizing pass)

Continuation of the 2026-05-14/15 session, narrowly focused on locking the numerical baseline before resuming Q12.2 on a stable foundation. Two new decisions (40, 41); amendments to decisions 22 and 25.

**Numerical baseline re-locked.** The earlier 2026-05-15 amendment of decision 22 mis-read deposit volume as a weekly figure. Corrected values:

- **Project flow**: 100 projects/year as a hard cap, reached over ~2 years from 12-15/year today. The ~50 existing VRE projects migrate into the Archive one-by-one alongside net-new projects; iterative-via-refunding (CAS / 0812 as the worked example) counted as a new project per refunded round, not as iterative-within-project.
- **Deposit pattern**: one-shot; ~100 deposits/year; ~200K events per deposit (≈ 100K Resource + 100K Rep + 1 `DepositionAccepted`); ~1.5 Preservation Files per Rep on average; avg Rep ~5 MB; ~500 GB per project.
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

The CTS 2026-2028 Requirements catalog references "curation levels" in R08, R10, R11 but defers the taxonomy to the applicant — this paper supplies the recommended one. **Architectural implication for DAO**: should `dao:DepositAgreement` (decision 18) carry an explicit curation-level property naming the level committed? Likely yes, but the question is parked as Q15 (just added) rather than decided unilaterally — it touches per-Deposition vs. per-Resource granularity and the relationship to `dao:PreservationAction` types.

**DaSCH-specific relevance.** The three project-type tiers (permanent infrastructure / full-lifecycle / publication-only) map onto curation-levels:

- **Permanent infrastructure**: Level A (full active preservation; DaSCH's strongest commitment).
- **Full-lifecycle**: likely Level A as well (active during the project; potentially downgrading after publication).
- **Publication-only**: could be Level C or A depending on the agreement.

Surfacing this in DAO would make the commitment auditable per Deposition and would distinguish DaSCH's tiered service offering from a flat "everything is preserved equally" claim.

**Doc updates done in this session.**

- `standards/README.md` — added Position Paper row with citation prefix `CTS Levels: <Z|D|C|A>`.
- `§7` parked questions — new Q15 added: "Curation-level commitment per `dao:DepositAgreement`" with sub-questions on per-DepositAgreement vs. per-Deposition vs. per-Resource granularity, level-change event semantics, and PreservationAction-type constraints.
- `§8.1` Evidence index — decision-18 row annotated with the Q15 parked question reference.
- `§8.1` coverage-check table — new row added: "Curation-level commitment per Deposition / DepositAgreement" citing CTS R08/R09/R10 + Position Paper, nestor §1.2 / §9.2, ISO 16363 §3.3.1.
- Doc header refreshed.

**Q15 is non-blocking on Q12.2** but should be resolved before tier-1 CTS application begins (the level is a CTS-application-required declaration; cannot be deferred past the application moment).

**Q12.2 remains the top priority** for the next design session.

#### Session of 2026-05-16 (Q15 resolved — uniform A+C+D)

Ivan's response to the Q15 proposal cut through the per-DepositAgreement / per-Deposition / per-Resource complexity I'd built up: **"At DaSCH we are currently building the archive to enable A for all projects. Additionally, we do C and D for all projects. We don't distinguish the levels per project. A+C+D is our mandate."**

**Q15 RESOLVED by decision 43**: DaSCH commits to **uniform A + C + D across all projects, all Depositions, all preserved content**. No per-Agreement / per-Deposition / per-Resource variation. Consequences:

- `dao:DepositAgreement` does **NOT** gain a preservation-level property (the variation I'd proposed was answering a question DaSCH doesn't actually have).
- No `premis:preservationLevel` overrides in event payloads.
- No `CurationLevelChanged` event in the vocabulary.
- The Position Paper's Z / D / C / A taxonomy stays as reference vocabulary in `standards/`, not modelled as a tunable.
- The institutional invariant is documented in §1a "Preservation-level commitment" paragraph.

**Architectural lesson worth flagging.** I'd jumped to per-Deposition/per-Resource granularity because the Position Paper *allows* it and PREMIS *supports* it natively. But "what the standard supports" is not the same as "what the institution commits to." DaSCH's mandate is uniform; modelling per-content variation would have been over-engineering for a non-existent use case. The decision-27 "cite or deviate" pattern works in the other direction too: the standard supports a tunable, DaSCH commits to a constant — *document the constant*, don't model the tunable.

**Doc updates done in this session.**

- **New decision 43** in the log: uniform A+C+D commitment with explicit consequences and rationale; flags that future tiered offerings would require revisiting.
- **§7 Q15** marked RESOLVED with pointer to decision 43.
- **§1a** gained a "Preservation-level commitment: uniform A + C + D across all projects" paragraph, parallel to the CoreTrustSeal-target paragraph.
- **§8.1 evidence index** — decision-18 row clarified (no level property; cites decision 43); decision-43 row added (CTS R08/R09/R10, nestor §1.2/§6/§7/§8, ISO 16363 §3.3.1/§4.3/§4.4.1.1).
- **§8.1 coverage-check table** — "curation-level commitment" row simplified: now points at decision 43 architecturally + flags that the *public mandate statement* itself is organisational (needs to appear in CTS application).
- Doc header refreshed.

**Net effect**: Q15 was the lightest of the remaining design threads; resolving it leaves only Q12.2 (top priority) and the Phase-2 geographic-replication operational plan (organisational/operational) outstanding.

**Q12.2 is now the only remaining design thread.**

#### Session of 2026-05-18 (Q16 parked, decision 9 amended — rename to `dao:Resource`)

Two non-blocking changes ahead of resuming Q12.2. Both came out of a "step back and align" framing question Ivan opened at the start of the session.

**Framing alignment confirmed.** The three-layer mental model was made explicit and confirmed: **OCFL is the packaging** (bit-level container, content-agnostic); **DAO is the schema + ubiquitous language** (defines `dao:Resource`, `dao:Representation`, `dao:Event`, etc., and the relationships); **the instance RDF inside each entity OCFL Object is the archived content** (property/value assertions a project made on each Resource). OCFL knows nothing semantically about DAO; that's the source of OCFL's preservation value. The same layering applies on both substrates (event-log substrate and entity substrate, per decision 36).

**Q16 parked in §7: project ontologies as preservation-essential metadata.** While walking the layering, Ivan flagged that the §4.1 / decision 10 position ("project subclasses serve VRE-only purposes and are normalized away") is incomplete. Project-authored ontologies — *and* any `knora-base` classes/properties that a project used directly without subclassing — are themselves preservation-essential, because they are "the metadata of the data": the schema that lets a future Designated Community interpret the preserved property/value triples. Without them, the asymmetry "property IRIs survive but the defining ontology does not" creates an interpretability gap (CTS R10 / nestor §10.3 / ISO 16363 §4.2.4). Three options are listed in Q16: (a) `dao:hasOntology` from `dao:Project` to OCFL-packaged ontology files; (b) per-Deposition `knora-base` snapshot; (c) treat as external-ontology references per §4.2 with the files preserved out-of-band. Forward-references added inline at §4.1, §4.3, decision 6 (amended), decision 10. Distinct from §7 item 13 (project *administrative* metadata as ontology).

**Decision 9 amended — `dao:IntellectualEntity` → `dao:Resource`.** Ivan's framing: "DAO should be a simplification of `knora-base`; not using `Resource` is introducing a new name without benefit, since `dao:Resource` will be a simplified `knora-base:Resource`." This reverses the original 2026-05-12 choice that picked `IntellectualEntity` precisely to avoid the `knora-base:Resource` collision and align with PREMIS verbatim. The new framing is more honest to how DaSCH thinks about the model: the input is `knora-base`, the output is a simplified subset for archival. PREMIS alignment survives in conceptual form (`dao:Resource` ↔ `premis:IntellectualEntity`, decision 6 amended still applies); only the *name* changes. Collision is managed by namespace-prefix prose discipline (always `dao:Resource` vs `kb:Resource`, never bare "Resource"). Recorded as a documented deviation from decision 27's verbatim-PREMIS default. Decision 9 carries the full amendment text.

**Full rename scope (decision 9 amendment):**

| Variable | Was | Now |
|---|---|---|
| Class name | `dao:IntellectualEntity` | `dao:Resource` |
| Event 1 | `IntellectualEntityVersionPublished` | `ResourcePublished` |
| Event 2 | `RepresentationVersionCreated` | `RepresentationCreated` |
| URN prefix | `urn:dsp:ie:{uuid}` | `urn:dsp:resource:{uuid}` |
| Read-side URL | `/ie/{uuid}/v{n}` | `/resource/{uuid}/v{n}` |
| Abbreviation | "IE" | dropped; write "Resource" |

**Event names dropped the redundant `Version` infix.** A new Version is always created as a consequence of any state-committing event in DAO. Naming the event after the version-creation leaked OCFL/read-side mechanics into the domain vocabulary. The event names now describe *what happened in the domain* (a publication act; a Representation state commit), not the read-side projection consequence (a Version node materialised).

For `RepresentationVersionCreated` → `RepresentationCreated`, the rename keeps the event-sourcing convention "the event names the type of fact, not its temporal position." Initial creation and subsequent state commits both fire `RepresentationCreated`. The alternative would have been splitting into `RepresentationCreated` + `RepresentationUpdated`; rejected as not pulling its weight.

**Doc updates done in this session.**

- `dao-discovery.md`: 80+ rename hits applied; decision 9 amendment text added; appendix "naming history" entry rewritten; §4.1 / §4.3 / decision 6 (amended) / decision 10 gained forward-references to Q16; doc header refreshed; this §9.2 entry added.
- `modules/archive/CONTEXT.md`: Resource entry rewritten with simplification-of-knora-base framing; Representation entry updated (`urn:dsp:rep:` form); Event entry's vocabulary list updated; Resource Version / Representation Version entry updated; Internal IRI entry updated; ARK entry updated; relationships table updated; ambiguities and dialogue updated.
- `UBIQUITOUS_LANGUAGE.md`: Resource entry rewritten; Resource Version entry updated; Internal IRI entry updated; relationships updated; ambiguities updated; example dialogue updated.

**Also folded into this session — §2.2 rewrite to match decision 12 amendment.** A pre-existing inconsistency surfaced during the rename pass: §2.2 still described the pre-URN-amendment HTTPS-form (`https://archive.dasch.swiss/{type}/{uuid}`) as the write-side internal IRI, contradicting the 2026-05-15 decision-12 amendment that moved internal identifiers to the URN scheme `urn:dsp:{type}:{uuid}`. §2.2 was rewritten:

- **Write-side URN form** made canonical and the example list enumerated (`urn:dsp:resource:`, `urn:dsp:rep:`, `urn:dsp:project:`, `urn:dsp:agent:`, `urn:dsp:agreement:`, `urn:dsp:deposition:`, `urn:dsp:preservation-action:`, `urn:dsp:event-segment:{period}`).
- **Read-side URL** section reframed: URLs are *per Access Area subdomain*, not a single Archive-namespace URL; examples updated to `https://dpe.dasch.swiss/resource/{uuid}/v{n}` (DPE), `https://iiif.dasch.swiss/{shortcode}/{rep-uuid}/manifest.json` (SIPI), and analogous patterns for other subdomains.
- **Event identity** clarified: each event has a JSON-LD `@id` plus stream-position fields per decision 37; pointer to §5 / §3.6 for detail.
- **Why URN over HTTPS** paragraph added: Fedora 6 precedent, RFC 8141, no implied HTTP resolvability, brittleness of domain-binding.
- **§2.7 stability layers** updated: layer 2 is "Persistent-identity internal URN IRI" (was the HTTPS form); layer 3 is "Read-side projection URL" with DPE / per-subdomain example (was a single archive-namespace HTTPS URL).
- **§3.4 examples** (the per-Resource walk-through with v1/v2/v3) updated: write-side example shows `urn:dsp:resource:{uuid}`; read-side example shows `https://dpe.dasch.swiss/resource/{uuid}/...`.

**Not done in this session** (deliberate):

- `CONTEXT-MAP.md` at the repo root was not checked in this pass; spot-check on resume.
- `dpe-core` / DPE-side code references to `ie`/`IntellectualEntity` are out of scope for this discovery doc — handled when DPE is re-aligned with the Archive's published vocabulary.

**Q17 parked at end of session — ARK Resolver behaviour model.** Ivan flagged a third parking-lot item: ARKs as a substantive workstream. The current §2.3 – §2.7 + decision 13 settle the *strategy* (separate bounded context, three use cases, per-entity minting) but not the *behaviour*: pre-publication reservation flow with paper-citation-before-data semantics (working term `ArkCoined`); explicit URN ↔ ARK binding events; content-negotiation-driven resolution from a single URN to multiple Access Area subdomain URLs (HTML → DPE, IIIF → SIPI, raw bytes → asset server, etc.); and *move-out-of-DaSCH custody transfer* — when data leaves DaSCH for another repository, the ARK must continue resolving to the new custodian. The latter requires a new event class on the Archive side (working name `CustodyTransferred`). Q17 carries five sub-dimensions and is **research-needed before commitment** in the spirit of §9.5 ("research before architectural commitment"). Q17 subsumes existing Q11 (reservation expiry). Affects §2.3 – §2.7, decision 13, §5.1 event vocabulary, and CONTEXT.md ARK entry — none touched in this session beyond cross-references.

**Q12.2 remains the next thing.** All three changes here were non-blocking. The "Q12.2 is now the only remaining design thread" note from the prior session still holds — although Q16 and Q17 are *important* (both must land before CTS application), they're not foundationally entangled with Q12.2's access-rights work. The four §9.6 foundations (architecture stable, standards available, evidence index, methodology) all carry forward intact.

#### Session of 2026-05-18 (Q12.2 sub-question 1 resolved — decision 44; Q18 parked)

Resumed Q12.2 after the morning's parking-lot work (Q16 / Q17 / decision 9 amendment / §2.2 rewrite). Two outcomes: decision 44 (Q12.2 sub-question 1), plus Q18 parked as a follow-on from the resolution discussion.

**Q12.2.1 resolved by decision 44 — explicit per-Version `dao:accessRights`, no propagation in DAO.** Started by recommending a two-level (Resource + Rep) vertical scheme with propagating composition across `kb:isPartOf`. Ivan reframed: *every `dao:Resource` Version and every `dao:Representation` Version needs explicit `dao:accessRights`* — DAO must not encode any application/inheritance logic.

His four reasons:

1. **DAO is the write-side schema; computation belongs to read-side projections.** Consistent with §1b's CQRS framing and §3.4's precedent for Version nodes (which are projections, not DAO classes).
2. **Future flexibility.** Producers other than VRE may not produce compound `kb:isPartOf` hierarchies at all — DAO shouldn't presuppose VRE's current modeling pattern.
3. **Future multi-parent / deep-chain models break propagation semantically.** When `kb:Representation` can be referenced by multiple `kb:Resource`s, "which parent's rule wins?" becomes ill-defined. Same for `Res → Res → Res → Rep`. Explicit per-entity rules sidestep this.
4. **Read-side projection has full flexibility to evolve without DAO changes.** Ivan is contemplating a "two access spaces" partitioning at the γ layer (open-access space vs. authorized-restricted space). DAO carrying explicit per-entity facts lets γ group them however institutional policy decides; γ can change without touching DAO.

Recorded as **decision 44** (chronologically after decision 43). The two-level *positional* scheme (Resource + Representation, with File-level deferred as additive per decision 33) survives, but the *composition* dimension that I had stitched onto it is rejected — composition isn't in DAO at all.

**Audit anchors** (§8.1 evidence index updated): CTS R13 (reuse conditions documented + machine-readable per entity); nestor §11.2 + §6.3; ISO 16363 §4.5 (access management — repository shall associate each AIP with access management conditions); §3.5.1 (rights, terms and conditions for use).

**Methodology lesson worth flagging.** I'd recommended (ii) propagating composition because curators-think-at-the-work-level. Ivan's reframe revealed I was conflating *what curators think about* with *what the schema should model*. Curators think about works; γ-layer projections compute work-level views; DAO stores explicit facts. The work-level abstraction is a *projection* over explicit facts, not a property of the schema. This is the same lesson as decision 43 / Q15 ("don't model what the institution doesn't commit to" — §9.5): standards/conventions support a surface area, but the institution's commitment is the sub-region to model. Here the inverse: the *interface a user wants* is not the same as the *schema the system stores*. The projection bridges them. Don't fold projection logic into the write-side schema.

**Q18 parked — Compound Resources and the `kb:isPartOf` + `kb:seqnum` pair in DAO.** Came out of the Q12.2 discussion. Ivan noted that the `isPartOf` + `seqnum` pairing — required whenever a Resource is part of a compound — is *tacit knowledge enforced in `dsp-api` code, not declared in the `knora-base` ontology*. DAO should make this structural commitment explicit. Looked up the canonical knora-base property names: `kb:isPartOf` (`knora-base.ttl:609`, subproperty of `kb:hasLinkTo`, subject + object both `kb:Resource`); `kb:isPartOfValue` (reification companion); `kb:seqnum` (`knora-base.ttl:679`, `kb:IntValue` object, comment: "position of a resource within a compound object — typically the order of pages within a book or similar"). Five sub-dimensions parked: property survival across kb→DAO substitution, compound semantics (graph shape vs. class — working position: graph shape, not class), reification handling (strip or preserve `*Value` companions), related `kb:isRegionOf` pattern, and the load-bearing connection to Q12.2's γ-projection design. The last point is important: **decision 44 made composition a γ-projection concern, but the projection needs queryable `isPartOf` relationships in order to *do* propagation.** So Q18 is non-blocking on Q12.2's remaining sub-questions but **must land before γ-projection implementation**.

**Audit anchors:** CTS R10 (technical quality / interpretability); nestor §10; ISO 16363 §4.2.4. The DSP-API-tacit-knowledge point is itself an R10 risk — preservation-essential structural commitments should live in declarative artefacts (ontology + SHACL), not in implementation code.

**Doc updates done in this session.**

- **New decision 44** in the log (chronologically after decision 43): explicit per-Version `dao:accessRights`; no propagation in DAO; composition is γ-projection.
- **§8.1 Evidence index** — decision-44 row added (CTS R13; nestor §11.2 + §6.3; ISO 16363 §4.5 + §3.5.1).
- **§7 item 12 (Q12)** — sub-question 1 marked RESOLVED with pointer to decision 44; remaining sub-questions 2 and 3 noted.
- **§7 item 18 (Q18)** — new parking-lot entry with the five sub-dimensions + knora-base property citations.
- **§9.4 priority list** — Q12 status updated; new item 7c (Q18); Q11's "subsumed by Q17" cross-reference preserved.
- **§9.6 settled-architecture table** — new "Access rights model" row pointing at decision 44 + Q12.1.
- **§9.6 top-priority section** — sub-question 1 marked RESOLVED; sub-question 2 promoted to NEXT with a brief design-tilt note ("decision-44 framing tilts toward simple-property + external-policy-document, but audit/replay requirements may demand more structure").
- **§9.6 outstanding-but-non-blocking** — Q18 paragraph added.
- **§9.6 done log** — three new struck-through items (Q12.2.1, Q18, decision 44).
- **Header `Last updated`** — refreshed.

**Not done in this session** (deliberate):

- **§5.1 event-vocabulary description** — the events `ResourcePublished` and `RepresentationCreated` already carry full snapshots per decision 24; decision 44 only adds `dao:accessRights` to the snapshot's required fields. The explicit "required fields" list is parked under Q10 (Representation property list); resolving Q10 will pick this up.
- **CONTEXT.md** — the *Avoid* column on `Resource` / `Representation` doesn't yet mention "DAO does not propagate access rules" because that's a behaviour assertion, not a naming convention. The glossary is fine as-is; the behaviour is documented in decision 44 and §7 Q12.
- **CONTEXT-MAP.md** — still not checked this session; defer to a future cleanup.

**Q12.2 sub-question 2 is the next thing.** Sub-question 1 is settled. Sub-question 2 asks: how is `restricted_access` *shaped* in DAO — simple property pointing at an external policy document, or a structured `dao:AccessPolicy` class with rule fields γ can interpret directly?

#### Session of 2026-05-18 (Q12.2 sub-question 2 resolved — decision 45; Q19 parked)

Continuing from Q12.2.1 resolution. Three options framed for sub-question 2: (A) simple property + external URI; (B) structured `dao:AccessPolicy` class with rule fields in DAO; (C) hybrid — first-class `dao:AccessPolicy` entity with opaque RDF policy content.

**Resolved option C** (recorded as decision 45). Recommendation reasoning carried into the decision: aligns with decision 44 (DAO stores facts; γ interprets); parallels existing `dao:DepositAgreement` (decision 18) and Q16 option (a) for project ontologies — now a recurring DAO idiom of "typed reference to a preserved artefact whose internal schema is opaque to DAO"; keeps preservation under DaSCH's control (vs. option A's external-URI fragility); doesn't force a policy-ontology choice today (vs. option B locking DAO into a policy expressiveness model); first-class entity allows project-level policy reuse across many Resources/Reps without redundant copies.

**Schema additions baked in by decision 45.**

- New top-level class **`dao:AccessPolicy`** (grows decision 15's class list to 9).
- New URN type **`urn:dsp:access-policy:{uuid}`** (extends decision 12 amendment's enumeration).
- New event types **`AccessPolicyCreated`** and **`AccessPolicyRetired`** in §5.1 — fire on the policy entity, not on the consuming Resource/Rep. Named per decision 9 amendment's "no `Version` infix" rule.
- New property **`dao:hasAccessPolicy`** on Resource / Representation Versions linking to the policy entity. Only present when `dao:accessRights = "restricted_access"`.
- Existing **`AccessRuleChanged`** continues to fire on Resource/Rep entities when their access rule changes (new COAR value, new policy reference, embargo end date changed).

**Q19 parked — policy ontology selection.** The opaque `dao:AccessPolicy.policyContent` slot needs a concrete RDF schema. Five candidates listed in §7 item 19 with prima-facie analysis: **ODRL** (W3C RDF Recommendation 2.2, Feb 2018 — Policy → Permission/Prohibition/Duty → Action/Asset/Party/Constraint; the obvious external candidate); **PREMIS rights extension** (preservation-domain native; minimum-viable for simple cases; aligns with decision 6 amended); **XACML** (OASIS XML; rich but RDF-awkward); **DC terms** (insufficient for structured rules on its own); **DaSCH-custom** (lock-in escape hatch). Likely outcome ODRL or PREMIS rights, possibly composed. **Research dispatch needed** before commitment, in the spirit of §9.5 "research before architectural commitment." Audit anchors: CTS R13; nestor §11.2; ISO 16363 §4.5 / §3.5.1.

**ODRL primer (recorded here for future-session orientation).** Open Digital Rights Language, W3C Recommendation 2.2 (Feb 2018), maintained by the W3C ODRL Community Group. RDF-native, JSON-LD and RDF/XML serializations. Core information model: `Policy → Permission | Prohibition | Duty → Action / Asset / Party / Constraint`. Adopters include large media organizations, national libraries, EU data portals. RDF-native stance aligns with DAO's existing serialization choices (decisions 34 + 37).

**Doc updates done in this session.**

- **New decision 45** in the log (chronologically after decision 44): `dao:AccessPolicy` as first-class entity with opaque policy content. Notes amendments to decisions 15 (class list) and 12 amendment (URN type list).
- **§5.1 Event vocabulary** — `AccessRuleChanged` description expanded; new entries for `AccessPolicyCreated` and `AccessPolicyRetired`.
- **§8.1 Evidence index** — decision-45 row added (CTS R13 + R09; nestor §11.2 + §10.4; ISO 16363 §4.5 + §3.5.1).
- **§7 item 12 (Q12)** — sub-question 2 marked RESOLVED with pointer to decision 45.
- **§7 item 19 (Q19)** — new parking-lot entry with five candidate policy ontologies + research scope.
- **§9.4 priority list** — Q12 status updated; new item 7d (Q19).
- **§9.6 settled-architecture table** — new "Access policy structure" row pointing at decision 45.
- **§9.6 top-priority section** — sub-question 2 marked RESOLVED; sub-question 3 promoted to NEXT.
- **§9.6 outstanding-but-non-blocking** — Q19 paragraph added.
- **§9.6 done log** — two new struck-through items (Q12.2.2, Q19).
- **CONTEXT.md** — new `AccessPolicy` glossary entry; Event entry's vocabulary list updated; Internal IRI entry's URN-type enumeration extended.
- **UBIQUITOUS_LANGUAGE.md** — new `AccessPolicy` row; Internal IRI entry updated.
- **Header `Last updated`** refreshed.

**Q12.2 sub-question 3 is the next thing.** Sub-questions 1 and 2 are settled. Sub-question 3 asks: where does the audit trail of "who accessed what when" live? Decision-44's framing (DAO stores facts; γ computes) and decision 41 (Grafana = operational, event log = audit-grade) together suggest γ-per-access logs are operational and stay out of the Archive's event log — but the *policy lifecycle* (creation, retirement, change) lives in DAO via `AccessPolicyCreated` / `AccessPolicyRetired` / `AccessRuleChanged`. Worth confirming and then we close Q12.2.

#### Session of 2026-05-18 (Q12.2 sub-question 3 resolved — decision 46; Q12 closes)

**Q12.2.3 resolved by decision 46 — clean audit-trail boundary.** The composition built up across decisions 44 + 45 framed sub-question 3 cleanly: DAO stores explicit facts (decision 44); γ-projection interprets (decisions 44 + 45); Grafana-vs-event-log split (decision 41) treats operational telemetry separately from audit-grade preservation evidence. The audit-trail question reduces to *what's audit-grade* vs. *what's operational*. Decision 46 codifies:

- **Audit-grade (in DAO event log)**: policy lifecycle. Three event types do the work: `AccessPolicyCreated` (a new policy entity comes into existence with its content snapshot per decision 24); `AccessPolicyRetired` (a policy is taken out of active use; WORM means historical references continue to resolve); `AccessRuleChanged` (a Resource/Rep entity's access rule changes — new COAR value and/or new `dao:hasAccessPolicy` link, producing a new Resource/Rep Version per §3.1).
- **Operational (not in DAO event log)**: γ-per-access logs. Every page view, every IIIF tile request, every SPARQL query — if pulled into the WORM event log these would dwarf preservation events by orders of magnitude. They are γ-side operational telemetry analogous to Grafana metrics per decision 41. Finite retention; not preservation evidence. DaSCH may retain them in an organisational evidence archive (for breach forensics, FADP / data-protection compliance), but that retention is *organisational*, not part of the Archive's preservation commitment.

**The boundary maps directly to certification framings.** CTS R13 / nestor §11.2 / ISO 16363 §4.5 ask for documented machine-readable access conditions *per AIP* — which the DAO policy lifecycle satisfies. None of these requirements demand that every individual access attempt be preservation-grade evidence. The auditor question is "what did the policy say at time T?", not "did we log every access on day T?". Decision 46 makes that explicit so future audits can cite the policy-lifecycle events directly without questions about whether per-access logs should have been preserved.

**Q12 closes in its entirety.** Four sub-resolutions over the four-day arc:

| When | Sub-question | Decision | Outcome |
|---|---|---|---|
| 2026-05-15 | Q12.1 enforcement locus | (resolved before today's session) | γ delivery-layer (Access Area subscribers) |
| 2026-05-18 | Q12.2.1 composition | Decision 44 | Explicit per-Version `dao:accessRights`; no propagation in DAO |
| 2026-05-18 | Q12.2.2 `restricted_access` structure | Decision 45 | `dao:AccessPolicy` as first-class entity with opaque RDF policy content |
| 2026-05-18 | Q12.2.3 audit-trail mechanism | Decision 46 | DAO event log carries policy lifecycle; γ-per-access logs are operational telemetry |

**Downstream open from Q12 closure**: only **Q19** (policy ontology selection for `dao:AccessPolicy.policyContent`, parked 2026-05-18, research-needed).

**Doc updates done in this session.**

- **New decision 46** in the log (chronologically after decision 45).
- **§8.1 Evidence index** — decision-46 row added (CTS R13 + R07 for provenance of access-policy changes; nestor §11.2 + §6.3; ISO 16363 §4.5 + §3.5.1.1).
- **§7 item 12 (Q12)** — entire entry struck through, marked RESOLVED 2026-05-18 with pointers to all four sub-resolutions and the downstream Q19 thread.
- **§9.4 priority list** — Q12 status updated to RESOLVED.
- **§9.6 settled-architecture table** — new "Access-decision audit-trail boundary" row pointing at decision 46.
- **§9.6 top-priority section** — restructured from "Q12.2 in progress" to "Q12 fully RESOLVED" with a sub-resolution table; new "Top priorities for next session — choose one" section listing the seven candidate next moves with a recommended ordering (Q18 next + parallel research dispatches for Q16 / Q17 / Q19).
- **§9.6 done log** — two new struck-through items (Q12.2.3 + Q12-fully-closed).
- **Header `Last updated`** refreshed.

**No "next thing" pre-selected.** With Q12 closed, no design thread is in flight. The §9.6 "Top priorities for next session — choose one" table lays out the seven candidates by kind (design / research-dispatch / partial / long-horizon) with a recommended ordering. Up to the next session to choose.

#### Session of 2026-05-21 (Producer-facing surface unification — Q20 resolved by decision 47; Q21 + Q22 parked; new component "self-service preservation frontend")

**Conversation opened as a pre-discussion on gRPC for Ingest.** Ivan brought a working idea — "use gRPC for talking to the Ingest Service, since we need to upload a bunch of data and want efficient + strongly-typed messaging." Framed as exploratory: get clarity on the lay of the land before committing.

**The session walked from gRPC-for-bitstream-upload to a full producer-facing-architecture re-framing.** Five conversation turns moved the picture progressively:

1. **gRPC scope clarification.** Initial split: (i) Commands API, (ii) bitstream upload, (iii) control/orchestration. Bitstream upload via gRPC streaming = clean fit; the others are the consequential decisions. Closed ecosystem (only RDU-Tooling submits) narrowed the audit-transparency-vs-strict-typing trade-off.
2. **Security-driven Ingest separation.** Ivan introduced the security argument: SIPs may carry malicious content (ClamAV concern); Ingest must be separated from Archive so a compromised Ingest cannot corrupt the WORM substrate. Three threat models surfaced (malicious bitstream content, compromised Ingest process, network-level attacker); (2) and (3) drove the architecture. **Ivan's framing**: "Maybe not even the RDU-Tooling, if we have the Ingest Service separate" → option (b) confirmed: Ingest Service as a dedicated component in front of the Archive.
3. **Metadata Editor surfaced as new component.** Ivan introduced the self-service preservation use case: external + RDU users editing project-level metadata (currently baked into DPE's Docker image as JSON files). Talks directly to Archive in initial framing; later in conversation, after the "Metadata Editor will allow bitstreams" admission, **converges on Metadata Editor going through Ingest** (same AV gate, same SHACL validation). Generalises to: *everything Producer-side is a SIP*; Ingest is the universal SIP gate.
4. **Auditability concession.** Ivan pushed back on adviser's "wire-level transparency is audit-evidence" framing — *correctly*. CTS / nestor / ISO 16363 do not audit network traffic; the audit surface is the event log + documented Producer-Archive interface. OpenTelemetry + structured logging covers operational observability. Adviser conceded. Strict-typing-prevents-drift (via shared protobuf crate in the Rust monorepo) becomes the dominant gRPC justification.
5. **Closure on shape.** Convergent picture: Ingest = sole producer-facing surface. gRPC = wire transport across all surfaces (Producer → Ingest streaming SIP submission; Ingest → Archive Storage internal `CommandAPI`; Archive Storage → subscribers `EventStream` server-streaming — replaces SSE). CommandAPI internal-only with mTLS-distinguished roles (Ingest, preservation admin tooling). Archive self-defends. No SIP-as-submitted preservation in WORM. Self-service preservation frontend likely consolidates Metadata Editor + future SIP submission GUI into one external-facing component.

**SIP wire-format and transport disentangled.** Ivan's mid-conversation thought "could a SIP be a gRPC message?" exposed a conflation. Adviser disentangled: SIP-format and SIP-transport are separate concerns. For closed ecosystem (only RDU-Tooling + self-service preservation frontend), there is no separate on-disk SIP format — the gRPC protobuf message *is* the Producer-Archive interface, documented as schema (CTS R02 / R08 evidence). For future external Producers, BagIt is the working preference for a drop-in format, but external-format translation belongs in RDU-Tooling or a sibling translation gateway, not in Ingest (Ingest stays single-shape). Ivan endorsed.

**No SIP preservation.** Ivan settled the audit-trail sub-question explicitly: "No SIP preservation. We can leave them on Ingest for some time and backup them, but that should be enough." OAIS does not require SIP preservation; decision 24 (events carry full snapshots) covers replay; the protobuf schema documents the Producer-Archive interface.

**Three things newly parked or resolved.**

- **Q20 — Producer-facing surface unification.** RESOLVED by decision 47. Ingest Service as sole producer-facing surface; gRPC as Producer-Archive interface; CommandAPI internal-only; Archive Storage self-defending; no SIP preservation.
- **Q21 — Self-service preservation frontend** (Metadata Editor + SIP submission GUI consolidation). Parked in §7. Open sub-dimensions: bounded-context status; phase-1 read path (a) from Archive Storage `QueryAPI` → phase-2 (b) projection-from-EventStream evolution for offline-edit capability; authentication of external users; replaces the JSON-in-Docker pattern for DPE project metadata.
- **Q22 — SIP wire-format and Producer-Archive interface sub-questions.** Parked in §7. gRPC envelope shape (leaning: opaque RDF payload + bitstream chunks, not nested DAO classes — preserves N-Triples-at-rest and avoids schema duality); external-Producer translation gateway (BagIt drop-in, deferred until external access is real); mixed-content SIPs from Metadata Editor (RDU-Tooling SIPs and Metadata Editor SIPs use the same gRPC envelope; Ingest's validator suite runs bitstream branches conditionally).

**Substantive amendment to decision 31.** Decision 47 amends decision 31's transport story (HTTP → gRPC across all surfaces; SSE → gRPC server-streaming for EventStream; CommandAPI public → internal-only). The decision-31 *vocabulary* of three public APIs (Commands, Events, Binary) survives as the conceptual framing; what changed is the wire and the publicness.

**Naming.** "Ingest Area" retired in favour of "Ingest Service." The former framed Ingest as a deployment; the latter frames it as a producer-facing component with a defined responsibility. Updated in CONTEXT-MAP.md and CONTEXT.md inline.

**Open thread Ivan wants for the next session: DAO-shape from VRE export.** Concrete consolidation of Q10 + Q16, anchored against an actual VRE project export. Goal: nail down `dao:Resource`, `dao:Representation`, and a newly-named `dao:Ontology` (Q16's preservation-essential project ontologies) by walking real data. Pre-requisites for that session: a representative VRE project export, the project's ontology files, the relevant `knora-base.ttl` excerpts, PREMIS DD §1.2 (already in `standards/`), and the current §4 / §6 / decision-9-amendment framings.

**Doc updates done in this session.**

- **New decision 47** in the log (chronologically after decision 46).
- **§8.1 Evidence index** — decision-47 row added (CTS R02 + R08 + R11 + R14; nestor §3.1 + §6.1 + §13; ISO 16363 §3.5.1 + §4.1.1 + §4.1.1.3).
- **§7 items 20 / 21 / 22** — Q20 marked RESOLVED with pointer to decision 47; Q21 new parking entry (self-service preservation frontend; five sub-dimensions); Q22 new parking entry (gRPC envelope shape, external-Producer translation gateway, mixed-content SIPs, closed SIP-preservation question).
- **CONTEXT-MAP.md** — Archive description updated with Ingest Service / Archive Storage two-deployment framing; new "Self-service preservation frontend" row in the bounded contexts table; "Ingest Area" → "Ingest Service" rename in the Archive subdomains table; integration patterns table rewritten (gRPC SIP submission + internal CommandAPI calls + gRPC EventStream); topology diagram rewritten to show producer-side / DaSCH-edge / DaSCH-core / subscriber-side tiers; new boundary commitment ("Ingest Service is the sole producer-facing surface of the Archive").
- **CONTEXT.md** — "Ingest Area" → "Ingest Service" rename with extended description (sole producer-facing surface; no event-log / OCFL write path; SIPs retained for backup window only); new "Archive Storage" sub-entry describing the sealed core and mTLS-authenticated principals; boundary commitment expanded with the CommandAPI-internal-only paragraph.
- **§9.6 settled-architecture table** — new "Producer-facing surface" row pointing at decision 47.
- **§9.6 top-priority section** — Q22-flavoured sub-thread closed; new top-priority recommendation: DAO-shape from VRE export (Q10 + Q16 combined).
- **§9.6 done log** — three new struck-through items (Q20 resolved by decision 47; Q21 parked; Q22 parked).
- **Header `Last updated`** refreshed.

**No design thread in flight at end of session.** Ivan signalled clean close: "Otherwise, you can do the write-up now." The next session's entry point is set: DAO-shape from VRE export (Q10 + Q16 combined). Q21 and Q22 are research/design dispatches available whenever they become active. Q17 (ARK Resolver) and Q19 (policy ontology) remain parked research-needed threads from the previous session.

#### Session of 2026-06-01 / 2026-06-02 (DAO-shape from real VRE exports — decisions 48-54; SIP/AIP/DIP four-layer split + three manuals; fat-events storage pivot)

The recommended Q10+Q16 thread, run against **real stage exports** (`dsp-cli` against `api.stage.dasch.swiss`): **0803 incunabula** (compound books/pages), **081c hdm** (performance DB + `Cache*` views), **0868 solec** (Solar Eclipses, science data), **0801 beol** (Bernoulli-Euler, custom-standoff letters). Exports + sample images/XSLT live in `.claude/tmp/exports/` (gitignored — **ephemeral**; re-fetch recipe in §9.6).

- **Decision 48** — value flattening: a value is a flattened read-only RDF node following DSP-API's **ApiV2Simple** mapping (reuse `ValueContentV2.toJsonLDValue(ApiV2Simple)`, `OntologyConstants.KnoraApiV2Simple`, `KnoraBaseToApiV2SimpleTransformationRules`), carrying `valueHasUUID`; archival overrides where the retrieval schema is lossy (standoff, lists, files). Resolves Q10's value-granularity branch.
- **Decision 49** — versioning strategy is a **Producer/RDU-Tooling** concern; the Archive is version-strategy-agnostic (confirms §3.1).
- **Decision 50** — SIP shape: **Producer transforms to DAO-shape** (Archive Producer-agnostic; re-attributes §4.3's transformation jobs to the Producer side); wire = **opaque RDF (N-Triples) + chunked bitstreams in a thin protobuf envelope** (no DAO classes in protobuf). Resolves **Q22(a)**.
- **Decision 51** — lists + standoff. Lists → node-IRI, preserved **once** as project vocabulary (part of the ontology, Q16). Standard standoff → XML (+whitespace fix). Custom standoff → XML + XSLT **inline per value** (deliberate: XSLT ≤5 KB and the feature is being sunset; opposite call from lists, by design).
- **Decision 52** — **fat events; storage collapse.** Events are fat + canonical; entity state is a projection, never stored. OCFL drops to **bitstreams + the open-format event log**; decision 36's entity substrate dropped; embedded append-log + Redb (not off-the-shelf). Amends 24/33/34/36/40. DIP = emitted `EventStream`, not reshaped.
- **Decision 53** — AIP is **virtual/on-demand** (AIU = a Resource's stream + pinned Rep streams + bitstreams; AIC = Project/Deposition); events embed per-bitstream multihash + per-event payload SHA-256; PDI-completeness invariant; on-demand BagIt/RO-Crate; future Archive GUI.
- **Decision 54** — command validation: two-tier (stateless Ingest shape / stateful Archive referential+invariant+concurrency against the read-model); one Redb, two read models (A command-validation+GUI, B `QueryAPI`); all mutations are commands to the internal `CommandAPI`; preservation actions hit Tier 2 directly.
- **New deliverables — the SIP/AIP/DIP four-layer split + three manuals** (`dao-discovery.md` is now purely rationale + decision log): [`producer-deposit-manual.md`](./producer-deposit-manual.md) (SIP, layer 1; §1-4 written, §5-9 stubbed), [`archiving-manual.md`](./archiving-manual.md) (AIP + storage, layers 2-3; supersedes §3.7), [`consumer-manual.md`](./consumer-manual.md) (DIP, layer 4). One DAO vocabulary, three SHACL profiles (submission/stored/dissemination) + an OCFL serialization layer.
- **Knora system ontologies** present at `ontologies/knora/` (`knora-base.ttl` etc. + SHACL).
- **In flight at pause: Q18** (compound resources) — first sub-question posed (see §9.6 *In-flight thread*).

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
5. **Scale recalibration:** ~2 deposits/week, but a single deposit may carry tens of thousands of Resources/Representations. Working estimate: **~200,000 events/week ≈ 2.3 events/s steady-state**, deposit-burst-driven. This invalidates the "tens of events/week" framing in §3.7 and decisions 22 / 23.

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
3. ~~**Q12 `dao:AccessRights` for restricted access**~~ — **RESOLVED 2026-05-18.** Q12.1 (γ enforcement, 2026-05-15); Q12.2.1 (decision 44 — explicit per-Version, no propagation in DAO); Q12.2.2 (decision 45 — `dao:AccessPolicy` first-class entity); Q12.2.3 (decision 46 — DAO carries policy lifecycle; γ-per-access logs are operational). Downstream: Q19 (policy ontology, parked, research-needed).
4. **Q10 — split.** **Value-granularity branch RESOLVED 2026-06-01** (decisions 48 + 51: ApiV2Simple flatten + `valueHasUUID`, with archival overrides for lists/standoff/files). **Representation property-list branch still OPEN**: the `dao:Representation` blank-node file shape — which `kb:*FileValue` properties survive (filename, format/MIME, dimX/Y, checksums, IIIF base, page count) and the per-File blank-node structure (decision 33). The ApiV2Simple `File`-as-URL collapse is explicitly **not** used (decision 48 override 3). Now lives as `producer-deposit-manual.md` §8 + `archiving-manual.md`. Real file-value examples in hand (incunabula `kb:StillImageFileValue`; beol XSLT `TextFileValue`).
5. ~~**Q11 ARK reservation expiry policy**~~ — **SUBSUMED by Q17** (2026-05-18). Now one sub-question of the broader ARK Resolver behaviour-model workstream.
6. **Q14 Preservation-action workflows (policy)** — **PARTIALLY ENGAGED 2026-05-15** (sizing pass). `dao:PreservationAction` class is settled (decision 26). **Fixity *cadence* and *granularity* settled (decision 40)**: per OCFL Object, continuous-when-idle + ≥1 sweep/Object/year, variant payload by outcome. **Still open**: (i) response workflow when `FixityChecked` returns `fail` / `missing` — alerting, triage, recovery from secondary copies, who-decides-what; (ii) format-migration triggering policy — when does DaSCH initiate a `FormatMigrated` action, against which format-risk signals (PRONOM advisories, vendor obsolescence notices), with what stakeholder review; (iii) system-ontology-migration policy.
7. **Q13 Project metadata as ontology** — OPEN. Large workstream; may surface `dao:Place`, `dao:Concept`, etc. Defer until the above are answered unless a near-term project demands it. **Distinct from Q16** (which is about preserving project *domain* ontologies for Designated-Community interpretability).
7a. **Q16 Project ontologies as preservation-essential metadata** — OPEN (parked 2026-05-18). Reopens part of §4.1 / §4.3 / decision 6 (amended) / decision 10: project-authored ontologies and directly-used `knora-base` classes/properties may need to be preserved as the "metadata of the data" so the Designated Community can interpret the preserved property/value triples (CTS R10 / nestor §10.3 / ISO 16363 §4.2.4). Three options on the table; non-blocking on Q12.2 but **must land before CTS application** (R10 is hard to evidence after the fact). See §7 item 16. **Engaged 2026-06-01:** decision 51 commits that **project lists are preserved as part of the project vocabulary/ontology** (a first concrete piece of Q16); the four real project ontologies (incunabula/hdm/solec/beol) are in hand for the full design. Now **coupled to Q18** (whether project property/subproperty IRIs survive). Prima-facie target: `dao:hasOntology` on `dao:Project` (option a).

7b. **Q17 ARK Resolver behaviour model and full URN ↔ ARK lifecycle** — OPEN, research-needed (parked 2026-05-18). Refines decision 13 / §2.3 – §2.7. Five sub-dimensions: pre-publication reservation flow, explicit URN ↔ ARK binding events, content-negotiation-driven resolution, move-out-of-DaSCH custody transfer (new Archive-side event class), and a research thread on ARK community vocabulary and patterns. Subsumes Q11. Non-blocking on Q12.2 but **must land before CTS application** (R03 succession + R12 PIDs are core). Likely needs a Fedora 6 / OCFL-as-event-store-style research dispatch before the design commits. See §7 item 17.

7c. **Q18 Compound Resources and the `kb:isPartOf` + `kb:seqnum` pair in DAO** — OPEN (parked 2026-05-18). DAO should make the structural commitment explicit (currently tacit in `dsp-api` code). Knora-base properties confirmed: `kb:isPartOf` (`knora-base.ttl:609`), `kb:isPartOfValue` (reification companion), `kb:seqnum` (`knora-base.ttl:679`). Five sub-dimensions: property survival across kb→DAO substitution, compound semantics (graph shape vs. class), reification handling, related `kb:isRegionOf` pattern, and the load-bearing connection to Q12.2's γ-projection design (decision 44 made composition a read-side concern — but the projection needs queryable `isPartOf` relationships to *do* the propagation). Non-blocking on Q12.2 sub-question 3 but should land before γ-projection implementation. See §7 item 18. **IN FLIGHT 2026-06-02:** confirmed Resource↔Resource (on `dao:Resource`, not `dao:Representation`). Real data (incunabula): `Page incunabula:isPartOfBook Book` (subproperty of `kb:isPartOf`) + reified `isPartOfBookValue`; `incunabula:hasSeqnum` (subproperty of `kb:seqnum`). **First sub-question posed (awaiting Ivan):** canonical `dao:isPartOf` + `dao:seqnum` with a SHACL co-occurrence shape; the Producer maps project subproperties at deposit; the reified LinkValue companion is dropped (decision 48); project-IRI survival deferred to Q16.

7d. **Q19 Policy ontology selection for `dao:AccessPolicy.policyContent`** — OPEN, research-needed (parked 2026-05-18). Decision 45 left `policyContent` as an opaque RDF blob; the policy ontology choice is a separate sub-decision. Five candidates: **(a) ODRL** (W3C RDF-native; the obvious external candidate), **(b) PREMIS rights extension** (preservation-domain native; minimum-viable for simple cases), **(c) XACML** (OASIS XML; rich but RDF-awkward), **(d) DC terms** (simple but insufficient for structured rules), **(e) DaSCH-custom** (lock-in but minimal). Research scope: walk DaSCH's actual restricted-access cases, compare candidates, check ARK-community/CTS-certified-repository practice, look at Swiss FADP implications. Likely outcome: ODRL or PREMIS rights, possibly composed. Non-blocking on Q12.2 sub-question 3 but should land before policy implementation. See §7 item 19.
7e. **Q23 `dao:Ontology` versioning** — OPEN (backlog, added 2026-06-02). The VRE doesn't version ontologies (a VRE `dao:Resource` conforms to the single "latest" project ontology); non-VRE Producers may, so a `dao:Resource` must be able to **reference and validate against a specific `dao:Ontology` version** — version-pinning that mirrors §3.2 (Resource↔Representation), with decision-54 validation becoming version-aware. Couples to Q16 and decision 50. See §7 item 23.
8. **Q1 Path inventory for new Resource Version** — OPEN. Clarifying detail; non-blocking.
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
- No ADRs have been written yet. The strongest ADR candidates, in approximate priority order, are: **decision 17** (build-vs-buy: self-built Archive); **decision 26** (`dao:PreservationAction` as first-class); **decision 29** (Trusted Repository domain framing); **decision 30** (three-tier role vocabulary); **decision 36** (two-substrate storage architecture); **decision 38** (Redb for cache); **decision 39** (fixity baseline + deferred non-repudiation threat model); **decision 48** (value flattening = ApiV2Simple reuse); **decision 50** (Producer transforms to DAO-shape; Archive Producer-agnostic); **decision 52** (fat events + storage collapse); **decision 54** (two-tier command validation). These should be written before implementation begins.
- **Ship and measure before optimising.** Decisions that affect operational performance (event volumes, replay strategies, storage size, derivation costs) ship in their simplest form first, run against real load, and are then optimised based on observed behaviour. The architecture is designed to allow this — full SSE firehose can become filtered, single-segment-per-month can be sub-bucketed, derivation can move from eager to lazy, mmap can be added later, HMAC+HSM and Merkle-Tree non-repudiation are additive layers. None of those refinements is pre-built. If a question of the form "should we optimise X now?" comes up, the default answer is *not until we have measured X under real load*.
- **Research before architectural commitment.** Two research-agent dispatches in the 2026-05-14/15 session changed substantive design decisions (Fedora 6 + LDP research → decisions 33-35; OCFL-as-event-store research → decisions 36-39, second-pivot to two-substrate). When the architecture is in greenfield territory (i.e. no obvious prior art for the exact combination being attempted), do this round of research *before* recording the decision. The cost is one agent dispatch; the benefit is not accumulating year-3 operational debt from a design that nobody else has tried.
- **Don't model what the institution doesn't commit to.** If a standard offers a tunable (e.g., the CTS Curation & Preservation Levels Position Paper's Z / D / C / A per-Resource-or-per-Deposition assignment) and the institution commits to a constant (DaSCH's uniform A + C + D), document the constant — don't model the tunable. Q15 / decision 43 is the worked example: PREMIS supports `preservationLevel` per Resource / Rep / File, and the Position Paper allows per-content variation; modelling that would have been over-engineering for a non-existent use case. **Standards support a surface area; institutions commit to a sub-region of it. Model the commitment, not the support surface.** Ask "what does DaSCH commit to?" *before* "what does the standard allow?".
- **Don't fold projection logic into the write-side schema.** Decision 44 (Q12.2.1) is the worked example: I'd recommended propagating access-rights across `kb:isPartOf` because curators think at the work-level. Ivan's reframe revealed I was conflating *what users think about* with *what the schema should model*. The work-level abstraction is a *projection* over explicit per-entity facts, not a property of the write-side schema. The projection bridges user expectations and storage; the schema stores explicit facts. **DAO models the write side (§1b); behaviour belongs to read-side projections.** When a "should the schema do X automatically?" question arises, the first check is "can a projection do X instead?" — if yes, the projection is the right layer. Sharpens §1b into an actionable test.
- **SIP ≠ AIP ≠ DIP; one vocabulary, three SHACL profiles.** The information packages are distinct *validity contexts* over a single DAO vocabulary (submission / stored / dissemination), plus an OCFL serialization layer — not three ontologies (which would reintroduce translation/duality). Four-layer model: SIP (`producer-deposit-manual.md`), AIP model + storage (`archiving-manual.md`), DIP (`consumer-manual.md`); rationale + decisions stay here. Decisions 50/52/53/54 (2026-06-02).
- **The Producer transforms to DAO-shape; the Archive is Producer-agnostic.** All source-specific mapping (Knora flatten, normalization, vocabulary substitution) happens Producer-side before submission; the Archive only validates against DAO SHACL and never carries VRE-specific logic. A non-VRE producer conforms to the same DAO target. Decision 50; re-attributes §4.3.
- **Fat events; OCFL for blobs + the event log only.** When events carry the full snapshot (canonical, replayable), a separate materialized entity store is redundant — OCFL holds bitstreams + the open-format event log, nothing else. Don't store the same content twice. Decision 52 (the un-redundancy of decisions 24 + 36).
- **Don't build infrastructure for a feature being sunset.** Decision 51 (custom standoff): inline the tiny XSLT per value rather than build project-level mapping/XSLT machinery for a feature being removed — the *opposite* call from lists (a permanent feature, preserved once at project level), by design. **Feature longevity, not just DRY, decides the storage model.**
- **Validate the model against real data before committing the shape.** This session pulled real stage exports; they sharpened or corrected the design at several points (the `DateValue`/standoff/list/file losses in ApiV2Simple → archival overrides; the XSLT-sharing duplication; the kb-subproperty pattern for `isPartOf`/`seqnum`). Walk a representative export before fixing the shape.

### 9.6 Where we left off — next-session entry points

**Last paused: 2026-06-02, mid-Q18.** The recommended Q10+Q16 DAO-shape thread ran against real stage exports and produced **decisions 48-54**, the **SIP/AIP/DIP four-layer split**, and **three manuals**. **In flight: Q18** (compound resources) — see *In-flight thread (Q18)* below.

#### Current state (2026-06-02)

This session resolved the **value model** and pivoted the **storage architecture**:

- A value is a **flattened read-only RDF node** (ApiV2Simple + `valueHasUUID`; archival overrides for lists/standoff/files) — decisions 48/51.
- **Versioning is a Producer/RDU-Tooling concern**; the Archive is agnostic — decision 49 (confirms §3.1).
- The **SIP is Producer-transformed DAO-shape**, wired as opaque RDF + chunked bitstreams in a thin protobuf envelope — decision 50 (resolves Q22a; Archive is Producer-agnostic; re-attributes §4.3).
- **Events are fat and canonical; OCFL collapses to bitstreams + the open-format event log; decision 36's entity substrate is dropped** — decision 52 (embedded append-log + Redb, not off-the-shelf).
- The **AIP is virtual/on-demand** with PDI-complete fat events embedding fixity — decision 53.
- **Commands are validated two-tier** against a Redb read-model (one instance, two read models) — decision 54.

The work is organised as a **four-layer model** (SIP / AIP-model / OCFL-serialization / DIP) documented in three manuals; `dao-discovery.md` is now purely **rationale + decision log**. Earlier parking items remain: **Q16 now engaged** (lists preserved as vocabulary, decision 51), Q17, Q19, Q21, Q22(b); **Q22(a) resolved** (decision 50); Q12 closed (2026-05-18).

Architecture is stable on:

| Concern | Settled by |
|---|---|
| Domain framing (Trusted Repository; bounded contexts) | Decision 29, `CONTEXT-MAP.md` |
| Numerical baseline (100 deposits/yr, 200K events/deposit, 20M events/yr ingest steady-state, ~220M events/yr fixity-dominated at year 20, ~1 PB at year 20) | Decision 22 amended |
| **Storage (fat events; OCFL = bitstreams + open-format event log; entity substrate dropped; embedded append-log + Redb, not off-the-shelf)** | **Decision 52** (amends 21/33/34/36/40); 37, 38 |
| **Value model (flattened ApiV2Simple payload + `valueHasUUID`; lists as node-IRI preserved as project vocabulary; standoff as XML, +XSLT inline for custom mappings)** | **Decisions 48, 51** |
| **SIP shape (Producer transforms to DAO-shape; Archive Producer-agnostic; opaque RDF + chunked bitstreams in a thin protobuf envelope)** | **Decision 50** (resolves Q22a; re-attributes §4.3) |
| **Versioning locus (Producer/RDU-Tooling concern; Archive version-strategy-agnostic)** | **Decision 49** (confirms §3.1) |
| **AIP (virtual/on-demand; AIU = Resource stream + pinned Rep streams + bitstreams, AIC = Project/Deposition; fat events embed fixity; PDI-complete)** | **Decision 53** |
| **Command validation (two-tier: stateless Ingest shape / stateful Archive referential+invariant+concurrency; one Redb, two read models)** | **Decision 54** (refines 47) |
| **Four-layer model + manuals (SIP → `producer-deposit-manual`, AIP → `archiving-manual`, DIP → `consumer-manual`; one DAO vocabulary, three SHACL profiles + OCFL serialization)** | **Decisions 50/52/53/54** |
| Identifiers (URN internal `urn:dsp:{type}:{uuid}`, `urn:dsp:resource:` for Resources; ARK long-term public) | Decision 12 + amendment, decision 9 + amendment, decision 13 |
| Core class names (`dao:Resource` as simplification of `knora-base:Resource`; `dao:Representation`) | Decision 9 + amendment (2026-05-18) |
| Event vocabulary (10 event types; `Version` infix dropped from `ResourcePublished` / `RepresentationCreated`) | §5.1, decision 9 amendment |
| Fixity policy (per-OCFL-Object, continuous-when-idle + ≥1/year, variant payload by outcome) | Decision 40 |
| Producer-facing surface (Ingest Service as sole gate; gRPC as Producer-Archive interface; CommandAPI internal-only; Archive Storage self-defending; no SIP preservation) | Decision 47 (2026-05-21); amends decision 31; resolves Q20 |
| Public APIs / wire transport (gRPC across all surfaces — Producer → Ingest streaming; Ingest → Archive Storage internal `CommandAPI`; Archive Storage → subscribers `EventStream` server-streaming; bitstream retrieval streaming; `/metrics` as operational fourth surface) | Decisions 31, 41, 47 |
| Certification pyramid (CTS → nestor → ISO 16363; preserve-optionality; geographic-redundancy architecturally committed, deployment-deferred) | Decision 42, §3.7 Phase-2 paragraph |
| Preservation-level commitment (**uniform A + C + D**, no per-content variation) | Decision 43, §1a |
| Access rights model (explicit per Resource Version + per Representation Version; no propagation in DAO; composition is γ-projection concern) | Decision 44 (2026-05-18); Q12.1 (γ enforcement, 2026-05-15); resolves Q12.2 sub-question 1 |
| Access policy structure (`dao:AccessPolicy` as first-class entity with opaque RDF policy content; new class + URN + events; policy ontology TBD per Q19) | Decision 45 (2026-05-18); resolves Q12.2 sub-question 2 |
| Access-decision audit-trail boundary (DAO event log carries policy lifecycle; γ-per-access logs are operational telemetry, not preservation evidence) | Decision 46 (2026-05-18); resolves Q12.2 sub-question 3; **closes Q12 in its entirety** |

Standards available in `standards/` (all extracted as `.md` for grep-ability):

- OAIS / ISO 14721 (CCSDS 650.0-M-3, Dec 2024)
- PREMIS DD v3.0 + PREMIS OWL Guidelines
- **CTS 2026-2028** — Requirements + Extended Guidance + Glossary + Curation Levels Position Paper v3.0
- **nestor Kriterienkatalog v2** (2008)
- **ISO 16363** (CCSDS 652.0-M-2, Dec 2024)
- **ISO 16919** (CCSDS 652.1-M-3, Dec 2024)

The living layer specs (read these for the current shape; `dao-discovery.md` is the rationale):

- [`producer-deposit-manual.md`](./producer-deposit-manual.md) — SIP (layer 1; §1-4 written, §5-9 stubbed)
- [`archiving-manual.md`](./archiving-manual.md) — AIP model + storage (layers 2-3; supersedes §3.6/§3.7)
- [`consumer-manual.md`](./consumer-manual.md) — DIP (layer 4)

Compact summaries (use these to orient quickly):

- `§8.1` Evidence index — evidence-bearing decisions × 3 certification tiers + coverage-check table
- `CONTEXT.md` — Archive context glossary
- `UBIQUITOUS_LANGUAGE.md` (repo root) — cross-context glossary
- `CONTEXT-MAP.md` (repo root) — bounded-context map

#### In-flight thread (Q18) — compound resources

Confirmed Resource↔Resource on `dao:Resource` (not `dao:Representation`). Real data (incunabula): `Page incunabula:isPartOfBook Book` (subproperty of `kb:isPartOf`) + reified `isPartOfBookValue`; `incunabula:hasSeqnum` (subproperty of `kb:seqnum`).

**First sub-question posed, awaiting Ivan's accept/redirect:** does DAO assert a **canonical `dao:isPartOf` + `dao:seqnum`** pair, with a SHACL shape requiring their co-occurrence on a compound *member*, onto which the Producer maps project subproperties at deposit? **Recommended yes** — `dao:isPartOf` (Resource→Resource, plain property per §6.2) + `dao:seqnum` (xsd:integer); Producer maps `incunabula:isPartOfBook → dao:isPartOf`, `incunabula:hasSeqnum → dao:seqnum`; reified `isPartOfBookValue` companion dropped (decision 48); the γ-projection (decision 44) gets uniform queryable structure. **Whether the project subproperty IRI also survives is the Q16 question** (coupled, flagged). Open follow-on sub-dimensions: `kb:isRegionOf` pattern, compound semantics (graph shape vs. class), multi-parent cases.

#### Real VRE exports (ephemeral — re-fetch each session)

The exports under `.claude/tmp/exports/` are **gitignored and will not survive**. Re-fetch (sysadmin `DSP_TOKEN` harvested from the DSP web app; binary at `~/.cargo/bin/dsp`):

```
DSP_TOKEN=<sysadmin-jwt> ~/.cargo/bin/dsp vre project dump --project <0803|081c|0868|0801> \
  --skip-assets --server stage --output ./<code>.zip --force --replace
```

Projects: **0803 incunabula** (compound books/pages, still-image reps), **081c hdm** (Person/Work/Performance + `Cache*` views), **0868 solec** (Solar Eclipses, 11 classes, science data), **0801 beol** (Bernoulli-Euler letters; 9 custom standoff mappings + XSLT). `--skip-assets` keeps it to RDF (`data/rdf/*.nq`). For sample bitstreams/XSLT use API v2 (`GET /v2/resources/{url-enc-IRI}` → `fileValueAsUrl` → fetch), **not** full asset dumps (slow + fragile). The dump endpoint is not project-scoped — dump one at a time with `--replace`. Knora system ontologies are checked in at `ontologies/knora/`.

#### How to resume

1. Read **§9.1** (methodology).
2. Read this **§9.6** section — the *Current state* summary above and the *Top priorities for next session* picker below.
3. Skim **§8.1 Evidence Index** for the compact decision overview.
4. **Resume the in-flight Q18 thread** (above) — answer the canonical `dao:isPartOf`/`dao:seqnum` sub-question, then finish Q18.
5. Then drain the manual stubs (Q10 Representation branch + Q16 + normalization). If picking a research-needed item (Q17 / Q19 / Q22b), dispatch the research before architectural commitment per §9.5.

#### Top priorities for next session — choose one

**Q18 is in flight** (see *In-flight thread (Q18)* above) — finish it first. After that, the real exports make the manual stubs concrete. Listed by urgency:

| Kind | Item | Status | Rationale |
|---|---|---|---|
| **In flight ★ finish first** | **Q18 — canonical `dao:isPartOf` + `dao:seqnum`** | Paused mid-flight | First sub-question posed; canonical-property recommendation on the table. Load-bearing for the γ-projection (decision 44); affects DAO SHACL. |
| **Design (real data in hand)** | **Q10 Representation branch + manuals §8** | OPEN | The `dao:Representation` blank-node file shape — which `kb:*FileValue` props survive (decision 33). Brings DAO to operational completeness. incunabula `StillImageFileValue` + beol XSLT `TextFileValue` in hand. |
| **Design (engaged)** | **Q16 — ontology preservation + `producer-deposit-manual` §7** | Engaged | Lists committed as vocabulary (decision 51); settle `dao:hasOntology` on `dao:Project`, which `kb:*`/project terms survive (couples to Q18). Four real ontologies in hand. Must land before CTS R10. |
| **Design (normalization)** | **`producer-deposit-manual` §5 — structural normalization / admin pruning** | OPEN | The §4.3 jobs, Producer-side (decision 50). Detail against the exports. |
| **Bookkeeping** | inline "amended-by-52" notes on decisions 24/33/34/36/40; DAO ontology + SHACL profiles; on-demand AIP serializer; `QueryAPI` surface | Deferred | Cleanup the storage pivot left; noted in `archiving-manual.md`. |
| **Research dispatch** | Q17 (ARK Resolver), Q19 (policy ontology) | Parked, research-needed | Must land before CTS application (R03/R12 PIDs; R13 policy). |
| **Recently parked / long-horizon** | Q21 (self-service preservation frontend), Q14 (preservation-action workflows), Q13 (project metadata as ontology) | Parked / partial | Q21 expected to activate once the DAO-shape lands. |

**Recommended next move:** answer the Q18 canonical-property sub-question and finish Q18; then take the **Q10 Representation branch + Q16** together against the exports already pulled — fixing the `dao:Representation` file shape (decision 33) and the `dao:Ontology` shape (`dao:hasOntology` on `dao:Project`), which fills `producer-deposit-manual` §6/§7/§8 and closes the DAO-shape.

#### Parked items (non-blocking on next session's choice; details in §7)

Each of these has a "must land before X" constraint that determines its eventual urgency, but none blocks any of the top-priority picks above.

**Q16 — project ontologies as preservation-essential metadata.** Parked 2026-05-18 (§7 item 16). Reopens part of §4.1 / §4.3 / decision 6 (amended) / decision 10: project-authored ontologies and directly-used `knora-base` classes/properties may need to be preserved as the "metadata of the data" so the Designated Community can interpret preserved property/value triples (CTS R10 / nestor §10.3 / ISO 16363 §4.2.4). Three options on the table: (a) `dao:hasOntology` on `dao:Project`; (b) per-Deposition `knora-base` snapshot; (c) external-ontology references per §4.2. **Must be settled before CTS application** (R10 is hard to evidence after the fact).

**Q17 — ARK Resolver behaviour model and full URN ↔ ARK lifecycle.** Parked 2026-05-18 (§7 item 17). Refines decision 13 / §2.3 – §2.7. Five dimensions: pre-publication reservation (paper-citation-before-data scenario; `ArkCoined`/`ArkBound` lifecycle); explicit URN ↔ ARK binding events as first-class facts in the Resolver's event store; content-negotiation-driven resolution from a single URN to subdomain-specific URLs (HTML → DPE, IIIF → SIPI, etc.); move-out-of-DaSCH custody transfer requiring a new `CustodyTransferred`-class event on the Archive side; and a research thread on ARK community vocabulary and patterns (`arks.org`, DataCite/CrossRef content-negotiation, Handle, nestor §4.6 succession patterns). Subsumes Q11. **Must be settled before CTS application** (R03 succession + R12 PIDs). Research dispatch needed.

**Q18 — Compound Resources and the `kb:isPartOf` + `kb:seqnum` pair in DAO.** Parked 2026-05-18 (§7 item 18). DAO should make the `isPartOf` + `seqnum` pairing explicit via SHACL (currently tacit in `dsp-api` code). Knora-base properties confirmed: `kb:isPartOf` (`knora-base.ttl:609`, subproperty of `kb:hasLinkTo`, subject + object both `kb:Resource`), `kb:isPartOfValue` (reification companion), `kb:seqnum` (`knora-base.ttl:679`, `kb:IntValue` object). Five sub-dimensions captured in §7 item 18. **Should land before γ-projection implementation** — decision 44 made composition a read-side concern, but the read-side projection needs queryable `isPartOf` relationships to *do* propagation.

**Q19 — Policy ontology selection for `dao:AccessPolicy.policyContent`.** Parked 2026-05-18 (§7 item 19). Decision 45 made the policy content an opaque RDF blob; the policy ontology choice is a separate research-needed sub-decision. Five candidates: ODRL (W3C RDF-native; obvious external candidate), PREMIS rights extension (preservation-domain native; minimum-viable), XACML (rich but RDF-awkward), DC terms (simple but insufficient), DaSCH-custom (lock-in but minimal). Likely outcome ODRL or PREMIS rights, possibly composed. **Should land before policy implementation.** Research dispatch needed.

**Q21 — Self-service preservation frontend (Metadata Editor + SIP submission GUI consolidation).** Parked 2026-05-21 (§7 item 21). Separately-deployed Rust SSR service for RDU staff + external users to create / edit project-level metadata and submit Depositions through a browser. Likely consolidates the Metadata Editor and a future SIP submission GUI into one frontend. Five open sub-dimensions: (a) bounded-context status (likely new BC, distinct ubiquitous language around drafts/edits/published); (b) write path via Ingest Service per decision 47 (mixed-content SIPs allowed — small metadata-edits and bitstream-bearing variants both go through the same gate); (c) read path phased — phase 1 direct gRPC `QueryAPI`, phase 2 EventStream projection for offline-edit capability; (d) external-user authentication; (e) replaces the JSON-in-Docker pattern for DPE project metadata (DPE becomes a true Access Area subscriber materialising project metadata from EventStream). **Expected to become active after the DAO-shape session lands.**

**Q22 — SIP wire-format and Producer-Archive interface sub-questions.** Parked 2026-05-21 (§7 item 22). Decision 47 committed to gRPC as the Producer-Archive interface for the closed ecosystem. Four sub-questions: (a) protobuf envelope shape — leaning toward opaque RDF payload + bitstream chunks (preserves N-Triples-at-rest; avoids schema duality) rather than nested DAO classes in protobuf; (b) external-Producer translation gateway — BagIt as drop-in working preference if/when external access is opened, but translation lives in RDU-Tooling or a sibling component, never in Ingest; (c) mixed-content SIPs from the self-service preservation frontend (RDU-Tooling SIPs and Metadata Editor SIPs use the same envelope, conditional bitstream validators); (d) closed — SIPs are not preserved (Ingest backup window only). Research dispatch deferred for (b) until external-Producer access is real.

#### Q12 fully RESOLVED 2026-05-18 (historical record — four decisions over the four-day arc)

Kept here as the most recent design closure; future resumers can skip if Q12 isn't being revisited. The `dao:AccessRights` work that opened on 2026-05-13 closes here.

| Sub-question | Resolved by | Outcome |
|---|---|---|
| Q12.1 — enforcement locus | 2026-05-15 | γ delivery-layer (Access Area subscribers); the Archive stores policy, γ decides |
| Q12.2.1 — composition / levels | Decision 44 (2026-05-18) | Two-level positional scheme (Resource + Representation); each Version carries explicit `dao:accessRights`; **no propagation, no inheritance, no composition in DAO** — composition rules are read-side γ-projection concerns |
| Q12.2.2 — `restricted_access` structure | Decision 45 (2026-05-18) | `dao:AccessPolicy` as first-class entity with opaque RDF `policyContent`; new URN type, new events, new `dao:hasAccessPolicy` property |
| Q12.2.3 — audit-trail mechanism | Decision 46 (2026-05-18) | DAO event log carries **policy lifecycle** (`AccessPolicyCreated` / `AccessPolicyRetired` / `AccessRuleChanged`); γ-per-access logs are **operational telemetry** (per decision 41 Grafana-vs-event-log framing), not preservation evidence |

Downstream open: Q19 (policy ontology, parked, research-needed). See *Parked items* above.

**Phase-2 geographic-replication implementation plan.** Decision 42 + §3.7 Phase-2 paragraph commit architecturally to a distant DR copy via filesystem mirroring (`zfs send` to a remote ZFS pool, or `rsync` with content-addressed verification). Deployment is deferred. Concrete planning needed: remote site selection, bandwidth for initial seed, `zfs send` cadence (per-Deposition / hourly / daily), failover RTO/RPO targets, remote fixity verification, secondary-site fixity-checker policy, integration with broader DaSCH operational landscape. Organisational + operational, not architectural. **Should land before tier-2 (nestor) certification attempt.**

#### Methodology reminders for the resuming session

- One question at a time (§9.1).
- Cite or deviate (decision 27).
- Update `CONTEXT.md` inline as terms resolve (§9.1).
- Don't reopen settled decisions without flagging that you're doing so (§9.1).
- **Ship and measure** before optimising (§9.5).
- **Research before architectural commitment** in greenfield territory (§9.5).
- **Don't model what the institution doesn't commit to** (§9.5; Q15 / decision 43 is the worked example).
- **Don't fold projection logic into the write-side schema** (§9.5; Q12.2.1 / decision 44 is the worked example).

#### Done log (struck-through priorities from past sessions — skip on resumption; kept for traceability only)

- ~~§3.6 / §3.7 narrative prose rewrite~~ — DONE 2026-05-15 (matches decisions 36-38 two-substrate model).
- ~~`CONTEXT.md` / `UBIQUITOUS_LANGUAGE.md` URN touch-ups~~ — DONE (already aligned).
- ~~ISO 16363 + 16919 added to `standards/`~~ — DONE 2026-05-16.
- ~~Certification-pyramid evidence-linkage pass~~ — DONE 2026-05-16 (§8.1 Evidence index).
- ~~CTS Requirements + Extended Guidance + Glossary added to `standards/`~~ — DONE 2026-05-16.
- ~~CTS Curation & Preservation Levels Position Paper added; Q15 raised~~ — DONE 2026-05-16.
- ~~Q15 (curation-level commitment) resolved~~ — DONE 2026-05-16 by decision 43 (uniform A+C+D).
- ~~Class rename `dao:IntellectualEntity` → `dao:Resource` + event-name `Version`-infix drop~~ — DONE 2026-05-18 by decision 9 amendment.
- ~~Project ontologies parked as Q16 (§7)~~ — DONE 2026-05-18.
- ~~§2.2 Internal identifiers rewrite — align with decision 12 amendment (URN canonical, HTTPS retired)~~ — DONE 2026-05-18 (had been a pre-existing inconsistency since 2026-05-15).
- ~~Q17 (ARK Resolver behaviour model) parked in §7; subsumes Q11~~ — DONE 2026-05-18.
- ~~Q12.2 sub-question 1 resolved — decision 44 (explicit per-Version `dao:accessRights`; no propagation in DAO)~~ — DONE 2026-05-18.
- ~~Q18 (Compound Resources + `kb:isPartOf` / `kb:seqnum` pair in DAO) parked in §7~~ — DONE 2026-05-18.
- ~~Q12.2 sub-question 2 resolved — decision 45 (`dao:AccessPolicy` as first-class entity with opaque policy content)~~ — DONE 2026-05-18.
- ~~Q19 (Policy ontology selection for `dao:AccessPolicy.policyContent`) parked in §7~~ — DONE 2026-05-18.
- ~~Q12.2 sub-question 3 resolved — decision 46 (DAO carries policy lifecycle; γ-per-access logs are operational, not preservation evidence)~~ — DONE 2026-05-18.
- ~~Q12 fully closed (all four sub-resolutions: Q12.1, decision 44, decision 45, decision 46)~~ — DONE 2026-05-18.
- ~~Q20 (Producer-facing surface unification) resolved by decision 47~~ — DONE 2026-05-21 (Ingest Service as sole producer-facing surface; gRPC as Producer-Archive interface; CommandAPI internal-only; Archive Storage self-defending; no SIP preservation; amends decision 31).
- ~~Q21 (Self-service preservation frontend — Metadata Editor + SIP submission GUI consolidation) parked in §7~~ — DONE 2026-05-21.
- ~~Q22 (SIP wire-format sub-questions: gRPC envelope shape, external-Producer translation gateway via BagIt, mixed-content SIPs) parked in §7~~ — DONE 2026-05-21.
- ~~CONTEXT-MAP.md updated — Ingest Service as sole producer-facing surface; self-service preservation frontend as new BC; integration patterns + topology rewritten for gRPC; "Ingest Area" → "Ingest Service" rename~~ — DONE 2026-05-21.
- ~~CONTEXT.md updated — Ingest Service + Archive Storage internal-structure paragraphs rewritten; new boundary commitment for CommandAPI-internal-only~~ — DONE 2026-05-21.
- ~~Q10 value-granularity branch resolved — decision 48 (ApiV2Simple flatten + `valueHasUUID`)~~ — DONE 2026-06-01.
- ~~Versioning locus — decision 49 (Producer/RDU-Tooling concern; Archive agnostic)~~ — DONE 2026-06-01.
- ~~SIP shape — decision 50 (Producer transforms to DAO-shape; opaque RDF + thin protobuf envelope); resolves Q22(a)~~ — DONE 2026-06-01.
- ~~Lists + standoff shapes — decision 51 (lists as project vocabulary; standoff XML, custom +XSLT inline)~~ — DONE 2026-06-01/02.
- ~~Fat events + storage collapse — decision 52 (OCFL = bitstreams + open-format event log; entity substrate dropped; embedded store)~~ — DONE 2026-06-02; amends 24/33/34/36/40.
- ~~AIP definition — decision 53 (virtual/on-demand; events embed fixity; PDI-completeness)~~ — DONE 2026-06-02.
- ~~Command validation — decision 54 (two-tier; one Redb, two read models)~~ — DONE 2026-06-02.
- ~~Three manuals created — `producer-deposit-manual.md`, `archiving-manual.md`, `consumer-manual.md` (SIP/AIP/DIP four-layer split)~~ — DONE 2026-06-02.
- ~~Real stage exports pulled (0803/081c/0868/0801) + sample image/XSLT via API v2~~ — DONE 2026-06-01/02 (ephemeral; re-fetch recipe in §9.6).

