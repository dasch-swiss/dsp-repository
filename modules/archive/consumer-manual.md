# Consumer Manual

**Status: design in progress.** This manual specifies the **DIP** — what the Access Area (and other Consumers) receive from the Archive, and the obligations they carry. The DIP is **DAO-as-events** (decision 52): the Archive emits fat events; there is no separately reshaped dissemination package.

**Documentation map** — the four-layer model maps onto the OAIS information packages:

| Layer | Package | Home |
|---|---|---|
| 1 | SIP (Producer input) | [`producer-deposit-manual.md`](./producer-deposit-manual.md) |
| 2–3 | AIP (model + storage) | [`archiving-manual.md`](./archiving-manual.md) |
| 4 | DIP (Consumer output) | **this manual** |

Rationale + decision log: [`dao-discovery.md`](./dao-discovery.md). Glossary: [`CONTEXT.md`](./CONTEXT.md) and [`../../UBIQUITOUS_LANGUAGE.md`](../../UBIQUITOUS_LANGUAGE.md). Bounded-context map: [`../../CONTEXT-MAP.md`](../../CONTEXT-MAP.md).

---

## 1. What the DIP is

The DIP is the same fat events the Archive emits — **not** a reshaped or repackaged form (decision 52). A Consumer receives the content over two channels plus one query channel:

| Channel | gRPC surface | Carries |
|---|---|---|
| **`EventStream`** | server-streaming (decisions 31/32/47) | fat events — the full DAO metadata snapshot per Version |
| **Binary retrieval** | streaming (`/bitstreams/{multihash}`) | Preservation File bytes, fetched on demand by hash |
| **`QueryAPI`** | unary (decision 54, read-model B) | synchronous point lookups (current state, existence checks) |

Events carry bitstream **multihashes**, not bytes; a Consumer fetches bytes from the Binary API at derivation time and caches them locally so user read-paths never round-trip to the Archive (decision 31).

## 2. The `EventStream` contract

- **Full firehose** (decision 31): every subscriber receives the entire event stream and filters client-side. The Archive carries no subscriber-side filter grammar.
- **Resumable** via cursor (`global_offset` / `Last-Event-ID`); **at-least-once** delivery; **per-entity ordering** (`stream_version`) with a monotonic `global_offset` across all events (decision 37); heartbeats for proxy traversal.
- **Cold-replay** by use case (decision 32): (α) subscriber-side snapshots for routine restarts; (γ) subscriber-to-subscriber replication for duplicating an existing subscriber kind; (δ) full replay from genesis for a brand-new subscriber kind.

## 3. The dissemination profile (what a Consumer may rely on)

Event payloads are DAO-vocabulary RDF validated against the **dissemination** SHACL profile (one DAO vocabulary, multiple profiles). A Consumer can rely on:

- the flattened value payloads (decisions 48/51): `xsd`/self-contained datatypes, lists as node-IRIs, standoff as XML (+ XSLT for custom mappings);
- access-rights facts (`dao:accessRights`, `dao:hasAccessPolicy`) — decisions 44/45;
- references (URN/ARK), relationships (context), and per-Version fixity (embedded checksums, decision 53).

Bytes are referenced by multihash and fetched via the Binary API.

## 4. Consumer obligations

- **Enforce access rights at delivery (γ).** The Archive ships the full firehose unfiltered; access composition and enforcement (most-restrictive-wins, the open vs. authorized-restricted partitioning) are read-side concerns the Consumer implements (decisions 44/46; Q12.1). DAO stores explicit facts; γ computes.
- **Honor `Tombstoned` / `Redacted`** (decision 28): hide a `Tombstoned` Version from dissemination paths and return a tombstone landing page on ARK resolution; resolve a redacted entity to its post-redaction current Version.
- **Treat events as read-only and your store as derived** (decision 32): each subscriber builds its own read store; it is regenerable by replay. Service Files / Service Projections are derivations under a rule (decision 30, §3.3), not preservation artifacts and not versioned.

## 5. What a Consumer does *not* receive

- **OCFL internals** — exclusive to the Archive boundary (decision 31); no Consumer reaches into OCFL.
- **The Archive's own read models** (decision 54) — read-model A (validation/GUI) is internal; read-model B is reached only through the `QueryAPI`, not directly.

## 6. Subscriber kinds (decision 32)

The Access Area is one bounded context with N independent subscriber services, each with its own cursor, storage, and derivation logic: IIIF (SIPI), HTML/Web Discovery (DPE), Custom Presentation (CPE), Asset/Download, SPARQL. The ARK Resolver and future projectors are also `EventStream` subscribers.

> **Pending:** the dissemination SHACL profile file; the precise `QueryAPI` surface; confirmation of which (if any) Access-Area needs are served by `QueryAPI` (read-model B) vs. each subscriber's own `EventStream`-built store.
