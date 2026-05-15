# Context Map

Map of the bounded contexts that implement the `dsp-repository` domain, and how they integrate. Each context's ubiquitous language is defined in its own `CONTEXT.md`.

## Domain

**Trusted Repository (OAIS-based)** — the long-term preservation and trustworthy dissemination of research data, governed by the OAIS reference model (CCSDS 650.0-M-3). The entire `dsp-repository` codebase implements subdomains of this single domain.

The domain decomposes into subdomains. Most are OAIS functional entities; a few are DaSCH-specific concerns that fall outside OAIS proper.

| Subdomain | Origin | Implementing bounded context |
|---|---|---|
| Ingest | OAIS | Archive |
| Archival Storage | OAIS | Archive |
| Preservation Planning | OAIS | Archive (future) |
| Data Management | OAIS | Archive (future) |
| Administration | OAIS | Archive (future) |
| Access | OAIS | Access Area |
| Identification (long-term citation) | DaSCH-specific | ARK Resolver |
| Producer-side preparation | DaSCH-specific (Producer-facing tooling) | RDU-Tooling |

**External to the domain:** VRE (a Producer in OAIS terms; out of scope of `dsp-repository`).

## Bounded contexts

| Context | Purpose | Code location | Status |
|---|---|---|---|
| **Archive** | Holds the preservation-grade record. Owns the OCFL store, the event log, and the three public APIs (Commands, Events SSE, Binary retrieval). Implements OAIS Ingest, Archival Storage, and the supporting functional entities. | `modules/archive/` | Specification in progress; implementation pending. |
| **Access Area** | Produces DIPs for Consumers (OAIS Access entity). Implemented as N federated subscriber services, one per DIP-shape subdomain. Subscribes to Archive events; materialises Service Files / Service Projections per subdomain; serves Access Files on demand. | TBD | Not yet built. |
| **ARK Resolver** | Persistent-identifier service. Mints, binds, and resolves ARKs. Own event store. Subscribes to Archive events. | TBD | Specified in `modules/archive/dao-discovery.md §2.3`. Implementation pending. |
| **RDU-Tooling** | Producer-side data preparation. Translates VRE export into Archive Commands. Issues `SubmitDeposition`, `ReserveArk`, etc. | TBD (producer-side, may live in a separate repo) | Not yet built. |
| **VRE** (Virtual Research Environment) | Active-research data platform. Produces exports consumed by RDU-Tooling. External to the Trusted Repository domain. | Out of scope (separate codebase) | Existing; being separated from the Repository. |

## Subdomains

Subdomains are problem-side slices implemented inside a bounded context. One bounded context can span multiple codebases when its subdomains warrant separate implementations.

### Archive subdomains

| Subdomain | OAIS entity | Implementation |
|---|---|---|
| **Ingest Area** | Ingest | Producer-facing deployment of the Archive (separate service from Archival Storage for bandwidth and failure isolation). See `modules/archive/CONTEXT.md` → Internal structure. |
| **Archival Storage** | Archival Storage | The OCFL store + event log + public APIs deployment. The non-Ingest-Area part of the Archive. |
| **Preservation Planning, Data Management, Administration** | (same names in OAIS) | Future. Implementation pending. |

### Access Area subdomains

| Subdomain | DIP shape | Implementation |
|---|---|---|
| **IIIF** | IIIF Manifest + image tiles served per IIIF Image/Presentation API. | SIPI (separate codebase, C++ → Rust rewrite underway). |
| **HTML / Web Discovery** | Rendered HTML pages with discovery, faceting, navigation. | `modules/dpe/` (DPE — Discovery and Presentation Environment). |
| **Custom Presentation** | Project-specific HTML/CSS presentations. | CPE (Configurable Presentation Environment). Not yet built. |
| **Asset / Download** | File-bytes download (HTTP Range supported). | Asset server. Not yet built. |
| **SPARQL** | SPARQL query results over a denormalized projection of preserved RDF. | SPARQL endpoint. Not yet built. |

## Not a bounded context

- **`modules/mosaic/`** — UI component library / design system. Shared kernel for the HTML/Web Discovery and Custom Presentation subdomains. No domain, no `CONTEXT.md`.

## Topology

```
                Commands API
RDU-Tooling ───────────────────────────►  ┌─────────────────────────┐
   ▲                                      │                         │      Events SSE
   │ ACL (translates VRE export)          │        Archive          │ ───────────────────►  Access Area
   │                                      │                         │                          │
VRE export                                │   ┌─────────────────┐   │ ───────────────────►  ARK Resolver
                                          │   │   OCFL store    │   │      Events SSE
                                          │   │ (Archive-only)  │   │
                                          │   └─────────────────┘   │
                                          │                         │ ◄───  Binary GET
                                          └─────────────────────────┘      (during Service File
                                                                            derivation)

                                                                                    │ Consumers read DIPs
                                                                                    ▼ per subdomain:
                                                                                    │
                                                          ┌────────────── IIIF (SIPI) ──────────────►  IIIF clients
                                                          ├────────────── HTML / DPE ──────────────►  Browsers
                                          Access Area  ───┼────────────── Custom / CPE  ───────────►  Browsers
                                                          ├────────────── Asset / Download  ───────►  Downloads
                                                          └────────────── SPARQL  ──────────────────►  SPARQL clients

ARK Resolver  ──── HTTP redirect ───►  Access Area (subdomain selected by suffix / Accept header)
```

## Integration patterns

| Upstream → Downstream | DDD pattern | Wire shape |
|---|---|---|
| RDU-Tooling → Archive (Ingest Area) | Customer/Supplier; Open Host Service | SIP upload + Commands API (HTTP); async validation gate at the Ingest Area |
| Archive → Access Area | Customer/Supplier; Published Language (DAO events) over Open Host Service | SSE event feed (`text/event-stream`, resumable via `Last-Event-ID`) + Binary retrieval API (`GET /bitstreams/{multihash}`) |
| Archive → ARK Resolver | Customer/Supplier; Published Language | SSE event feed (subset: identity-creation events) |
| VRE → RDU-Tooling | Anti-Corruption Layer | RDU-Tooling translates VRE export into Archive Commands; insulates the Archive from VRE retirement |
| ARK Resolver → Access Area | Non-domain integration | HTTP redirect; subdomain selected by URL suffix or Accept header |
| Access Area subdomain implementations → Mosaic | Shared Kernel | Cargo crate dependency (used by HTML/Web and Custom Presentation subdomains) |

## Boundary commitments

- **OCFL is exclusive to the Archive context.** No other context reaches into the OCFL store directly. All external access is via the Archive's three public APIs (Commands, Events SSE, Binary retrieval). See `modules/archive/dao-discovery.md §9.3`.
- **DAO is the Archive context's ubiquitous language.** It appears at boundaries (Commands input, Events output) — not as a universal language. Other contexts have their own internal vocabularies and translate at the seam. See `modules/archive/dao-discovery.md §1a` and decision 8.
- **The three-tier preservation-chain vocabulary (Preservation File / Service File / Access File) is shared Published Language across contexts.** Each tier is owned by a specific context: Preservation File by Archive; Service File by Access Area; Access File by the Access Area subdomain that serves the request. See `modules/archive/CONTEXT.md` → Preservation chain roles.
- **ARKs are the only long-term-stable identifiers.** Internal IRIs and presentation URLs may change across system migrations; ARK strings may not. See `modules/archive/dao-discovery.md §2.7`.
- **Push, not pull, for Archive → Access.** The Archive emits events; Access Area projects. The Archive does not run queries on behalf of downstream contexts. See `modules/archive/dao-discovery.md §9.3`.

## Where to write decisions

| Scope | Location |
|---|---|
| System-wide (cross-context: deployment topology, identifier schemes, event-bus choices) | `docs/adr/` at repo root |
| Context-internal (storage layout, internal class structure, subdomain decisions, internal API shape) | `modules/<context>/docs/adr/` |

Write an ADR only when the decision is (a) hard to reverse, (b) surprising without context, and (c) the result of a real trade-off with genuine alternatives. If any of the three is missing, skip it.
