# Producer Deposit Manual

**Status: design in progress.** This document specifies what a Producer must submit to the Archive and how. It is the Archive's **deposit contract** — the Published Language a Producer satisfies. The Archive defines it; Producers implement it. RDU-Tooling is the first implementer (VRE → Archive); direct, non-VRE producers implement the same contract.

**Documentation map** — the four-layer model maps onto the OAIS information packages:

| Layer | Package | Home |
|---|---|---|
| 1 | SIP (Producer input) | **this manual** |
| 2–3 | AIP (model + storage) | [`archiving-manual.md`](./archiving-manual.md) |
| 4 | DIP (Consumer output) | [`consumer-manual.md`](./consumer-manual.md) |

Rationale and the decisions behind every rule here live in [`dao-discovery.md`](./dao-discovery.md) (decision numbers are cited inline). Terminology is in [`CONTEXT.md`](./CONTEXT.md). When this manual and `dao-discovery.md` disagree, `dao-discovery.md` is authoritative until this manual is reconciled.

---

## 1. Scope and the deposit contract

A Producer submits a **SIP** (Submission Information Package). The Archive does not transform it; the Archive **validates** it and, on success, **emits events** (`OAIS §2.2`; decisions 47, 50). A Producer never emits events.

Flow (decision 47):

```
Producer ──SIP──▶ Ingest Service ──(validation passes)──▶ Archive Storage ──emits──▶ events
  (untrusted)        (sole gate)                              (sealed core)        (ResourcePublished, …)
```

The contract has two halves:

1. **Shape.** The descriptive metadata in the SIP must already be in **DAO-shape** — the Producer performs all transformation (§3, §5, §6, §7) before submission. The Archive is **Producer-agnostic**: no Knora/VRE-specific logic exists in the Archive core (decision 50). DAO + its SHACL profile is the single authoritative target; a SIP that does not validate against DAO SHACL is rejected at the gate.
2. **Packaging.** The SIP is carried over gRPC as a thin protobuf envelope plus opaque RDF and chunked bytes (§2).

What the Ingest Service checks (decision 47):

- DAO SHACL validation — always.
- `DepositAgreement` check — always.
- Format identification + ClamAV virus scan — on bitstream-bearing SIPs only.

SIP kinds (decision 47): **content-bearing** (Resources + Representations + bitstreams), **metadata-only**, or **mixed**.

**Selection is the Producer's.** The Producer places in the SIP only what is to be archived. Appraisal and selection — what is worth preserving, which derived or working resources to exclude — are a Producer/RDU concern, out of scope for the Archive. The transformation rules in this manual (§3, §5–§8) apply to whatever the SIP contains; they do not judge what belongs in it.

## 2. The SIP wire format (gRPC)

The transport is gRPC (protobuf + HTTP/2 streaming, decision 47). The protobuf message frames **only the package** — it does **not** model DAO classes (decision 50). The DAO-shaped metadata travels as an **opaque RDF payload (N-Triples, decision 34)**; Preservation File bytes travel as **chunked frames**.

Illustrative envelope shape (the authoritative schema is the shared protobuf crate, TBD — Q22 detail):

```proto
// Illustrative only — not the authoritative schema.
message DepositSubmission {
  string deposit_id          = 1;  // producer-assigned correlation id
  string agreement_ref       = 2;  // urn:dsp:agreement:{uuid}
  string project_ref         = 3;  // urn:dsp:project:{uuid}
  repeated ManifestEntry manifest = 4;  // one per Resource / Representation / File
  bytes  rdf_payload         = 5;  // opaque DAO-shaped N-Triples (the descriptive metadata)
  // Preservation File bytes are streamed separately as FileChunk frames,
  // correlated to manifest entries by multihash.
}

message ManifestEntry {
  string iri        = 1;  // internal IRI (urn:dsp:resource: / urn:dsp:rep: / filename within a Rep)
  string multihash  = 2;  // content address for File entries
  uint64 size_bytes = 3;
  string media_type = 4;  // MIME / PRONOM format id
  repeated VersionPin version_pins = 5;  // needs-versions case (decision 49); empty otherwise
}

message FileChunk {
  string multihash = 1;  // which file these bytes belong to
  uint64 offset    = 2;
  bytes  data      = 3;
}
```

Why opaque RDF rather than modelled protobuf (decision 50): modelling DAO in protobuf would create **schema duality** (DAO maintained in both RDF/SHACL and protobuf) and a graph→tree impedance mismatch — RDF multi-typing (§4.2), the blank-node value and file payloads (decisions 48, 33), and external-ontology references do not map cleanly onto protobuf's closed tree. Keeping RDF opaque on the wire keeps RDF/SHACL the single source of truth.

## 3. Value flattening

**Rule (decision 48): a property/value is a flattened, read-only RDF payload node, not a `dao:Resource` and not Knora's full value reification.** The Producer applies DSP-API's **ApiV2Simple** mapping (the simple-schema lexical/datatype rules) and keeps a thin value node carrying `valueHasUUID` (sub-resource citation target + standoff anchor). The reference implementation already exists in dsp-api — reuse it, do not reinvent it:

- `ValueContentV2.toJsonLDValue(ApiV2Simple, …)` — per-type rendering
- `OntologyConstants.KnoraApiV2Simple` — the custom datatypes
- `KnoraBaseToApiV2SimpleTransformationRules` — the removal list

Per-type mapping (the "full" reified form on the left is what appears in a raw VRE export; the DAO-shape on the right is what the SIP carries):

| Knora value | DAO-shape payload | Notes |
|---|---|---|
| Text (no standoff) | `xsd:string` | — |
| Int / Bool / Decimal / Uri / Time | `xsd:integer` / `boolean` / `decimal` / `anyURI` / `dateTimeStamp` | — |
| Date | `knora-api-simple:Date` lexical, e.g. `"JULIAN:1492 CE"` | calendar + era + Y/M/D; precision implicit in granularity; recomputable JDN dropped |
| Color / Geom / Interval / Geoname | self-contained lexical datatype | — |
| Link | target IRI | the link target is the preserved fact |
| List | **list-node IRI/ID** → preserved project vocabulary | **archival override** — not the leaf label; the vocabulary is preserved as ontology (§7) |
| Text with standoff (standard mapping) | **XML in the value blank node** (+ whitespace fix) | **archival override** — dsp-api `textValueAsXml` |
| Text with standoff (custom mapping) | **XML + XSLT, two values in the blank node** | **archival override** — XSLT is the Representation Information for the custom XML |
| File | becomes a `dao:Representation` (§8) | **not** flattened to a URL |

Pruned as VRE working state (§4.3, administrative pruning): `hasPermissions`, `attachedToUser`, `valueHasOrder`, `isDeleted`, `previousValue`.

**Archival overrides** — the ApiV2Simple schema is optimised for lightweight *retrieval* and discards information an *archive* must keep (decisions 48, 51):

1. **Standoff is preserved as XML inside the value blank node** (decision 51): **standard mapping** → one XML value, produced by dsp-api's `textValueAsXml` serialisation, with a whitespace-normalisation fix (logic exists in dsp-api); **custom mapping** → two values, the XML *and* its XSLT (the XSLT renders the XML → HTML and is the Representation Information for the custom XML). The XSLT is **inlined per value** (it is tiny, ≤~5 KB, and the custom-mapping feature is being sunset — decision 51), unlike lists (item 2) which are preserved once. Anchor to the value via `valueHasUUID`. The standoff *mapping* is a deposit-time tool (it makes the XML) and is not itself preserved.
2. **List values are kept as the list-node IRI** (decision 51), resolved against the project's **closed, controlled vocabulary**, which is preserved as part of the project ontology (§7). The label is not denormalised into the value. Note: in a VRE export the list *definitions* live in the data graph (`data.nq` / `admin.nq`), not the ontology file — the Producer lifts them into the preserved vocabulary.
3. **File values are not flattened to a URL** — they become `dao:Representation` with full file metadata (§8; decisions 20, 33).

## 4. Versioning strategy

**The Producer decides the version structure and expresses it in the SIP; the Archive emits the events (decision 49).** Knora versions at the value level (`previousValue` chains ordered by `valueCreationDate`); DAO versions at the Resource level (`ResourcePublished` events → read-side Versions, §3.4). A Resource Version is a *deliberate publication* (§3.1) — the Producer is the agent of "deliberate publication."

Two project types, both expressed Producer-side:

- **Type 1 — needs-versions.** Walk each value's `previousValue` chain; the distinct `valueCreationDate` timestamps across all of a resource's values define the version boundaries. Reconstruct the resource state as-of each boundary (per property, the value version with the latest `valueCreationDate` ≤ boundary). Express the N successive Versions in the SIP, each pinned to its boundary timestamp. The Archive emits N chronological `ResourcePublished` events.
- **Type 2 — no-versions.** Follow each property → current value only; ignore the `previousValue` chains. Express a single state. The Archive emits one `ResourcePublished`.

The choice is per-project and Producer-side. The Archive carries no versioning flag (decision 49); the strategy is inferable from the event count.

Reading value history from a VRE export (N-Quads): the resource property edge points to the **current** value; older versions are reachable **only** via the `previousValue` chain and are **not** `isDeleted` (superseded ≠ deleted). Empirically these chains are sparse and mostly incidental (e.g. label/typo corrections), so Type 1 is the exception.

## 5. Structural normalization and administrative pruning

> **TODO.** Producer-side jobs from §4.3 (decision 50 re-attribution): flatten project-specific subclass hierarchies (preserve the property/value assertions, not the class IRI — decision 10, *under review by Q16*); drop runtime VRE concerns (Knora permissions, internal state); substitute internal Knora vocabularies for standards-based ones (§4.4). To be detailed against the three real exports.

## 6. Compound resources (`isPartOf` / `seqnum`)

> **TODO (Q18).** Specify how compound structure survives. Real data: incunabula uses a project-specific subproperty `incunabula:isPartOfBook` (subproperty of `kb:isPartOf`) plus `incunabula:hasSeqnum`, and the reified `isPartOfBookValue` companion. Resolve: property survival across the kb→DAO flatten, the SHACL shape that makes the `isPartOf` + `seqnum` pairing explicit, reification handling, and the link to the γ-projection's queryable structure (decision 44).

## 7. Ontology preservation

> **TODO (Q16).** Whether and how the project-authored ontology and directly-used `knora-base` terms are preserved as interpretability metadata for the Designated Community (CTS R10 / nestor §10.3 / ISO 16363 §4.2.4). Prima-facie shape: `dao:hasOntology` on `dao:Project` referencing an OCFL-packaged ontology artefact. **The preserved vocabulary must include the project's lists** — list nodes are a closed, project-controlled vocabulary and the archived list-value IRIs (§3, decision 51) resolve against them; in a VRE export the list definitions live in the data graph, so the Producer lifts them into the preserved vocabulary. Real ontologies available: `incunabula` (3 classes), `hdm` (9 classes), `SolarEclipses` (11 classes); custom-standoff mapping example in `0801`.

## 8. Representations and file values

> **TODO.** The `dao:Representation` blank-node file shape (decision 33, `dao:hasFile`): which `kb:*FileValue` properties survive (filename, format, dimensions, checksums, IIIF base, page count) and the per-File blank-node structure. The ApiV2Simple `File`-as-URL collapse is explicitly **not** used here (decision 48 override 3).

## 9. Worked examples

> **TODO.** End-to-end SIP examples from the three stage exports — incunabula (0803, compound books/pages), hdm (081C, `Cache*` views + performance data), solec (0868, scientific data). Each: raw VRE fragment → DAO-shape → manifest entry.
