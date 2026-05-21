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
| **Archive** | Holds the preservation-grade record. Owns the OCFL store, the event log, and the gRPC `CommandAPI` / `QueryAPI` / `EventStream` (internal-only after decision 47). Implements OAIS Ingest, Archival Storage, and the supporting functional entities. Deployed as two services: Ingest Service (sole producer-facing surface) + Archive Storage (sealed core). | `modules/archive/` | Specification in progress; implementation pending. |
| **Access Area** | Produces DIPs for Consumers (OAIS Access entity). Implemented as N federated subscriber services, one per DIP-shape subdomain. Subscribes to Archive events (gRPC `EventStream`, server-streaming); materialises Service Files / Service Projections per subdomain; serves Access Files on demand. | TBD | Not yet built. |
| **ARK Resolver** | Persistent-identifier service. Mints, binds, and resolves ARKs. Own event store. Subscribes to Archive events. | TBD | Specified in `modules/archive/dao-discovery.md §2.3`. Implementation pending. |
| **RDU-Tooling** | Producer-side data preparation. Translates VRE export into SIPs submitted to Ingest. Submits via gRPC. Future role: translation gateway for any external-Producer formats (BagIt etc.) per Q22. | TBD (producer-side, may live in a separate repo) | Not yet built. |
| **Self-service preservation frontend** | Browser-based Rust SSR service for RDU staff + external users to create / edit project-level metadata and submit Depositions. Packages user actions into SIPs submitted to Ingest via gRPC. Likely a new bounded context; status open per Q21 in `modules/archive/dao-discovery.md §7`. | TBD | Not yet built; design parked as Q21. |
| **VRE** (Virtual Research Environment) | Active-research data platform. Produces exports consumed by RDU-Tooling. External to the Trusted Repository domain. | Out of scope (separate codebase) | Existing; being separated from the Repository. |

## Subdomains

Subdomains are problem-side slices implemented inside a bounded context. One bounded context can span multiple codebases when its subdomains warrant separate implementations.

### Archive subdomains

| Subdomain | OAIS entity | Implementation |
|---|---|---|
| **Ingest Service** | Ingest | **Sole producer-facing surface of the Archive** (decision 47). Receives all SIP submissions over gRPC; runs SIP-shape-appropriate validation (SHACL always; ClamAV + format ID on bitstream-bearing SIPs; `DepositAgreement` enforcement always); on success commits via Archive Storage's internal `CommandAPI`. Holds no event-log or OCFL write path. SIPs retained on Ingest for operational backup window only — no SIP preservation in the WORM event log. Separated from Archive Storage for trust-boundary (security), bandwidth, and failure isolation. Working name "Ingest Area" retired (it framed Ingest as a deployment rather than a sole surface). See `modules/archive/CONTEXT.md` → Internal structure. |
| **Archive Storage** | Archival Storage | OCFL store + event log + internal gRPC `CommandAPI` / `QueryAPI` / `EventStream`. Accepts authenticated principals only (Ingest Service; DaSCH-internal preservation admin tooling — mTLS-authenticated with distinct roles). Self-defends with re-validation of every command regardless of source. |
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
PRODUCER SIDE (untrusted)      DASCH EDGE (sole gate)            DASCH CORE (sealed)             SUBSCRIBER SIDE

RDU-Tooling ────────┐                                                                          ┌──► DPE (HTML)
   ▲                │                                                                          │
   │ ACL            │                                            ┌─────────────────────────┐   ├──► CPE (custom)
   │                ├── gRPC SIP submission ──►  Ingest          │     Archive Storage     │   │
VRE export          │   (streaming)              Service ── gRPC │                         │ ──┼──► SIPI (IIIF)
                    │                            (AV, SHACL,     │  ┌─────────────────┐    │   │   gRPC EventStream
Self-service        │                             format ID,     │  │   OCFL store    │    │   │   (server-streaming)
preservation     ───┘                             SIP commit)    │  │  + event log    │    │   ├──► Asset / Download
frontend                                                         │  │ (Archive-only)  │    │   │
                                                                 │  └─────────────────┘    │   ├──► SPARQL endpoint
                                                                 │                         │   │
Preservation admin tooling ───── VPN, gRPC CommandAPI ───────────►                         │   └──► ARK Resolver
(DaSCH-internal)                  (distinct mTLS role)           │                         │
                                                                 │      ◄── gRPC bitstream retrieval (during Service File derivation)
                                                                 └─────────────────────────┘


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
| RDU-Tooling → Archive (Ingest Service) | Customer/Supplier; Open Host Service | gRPC SIP submission (streaming); async validation gate at Ingest Service |
| Self-service preservation frontend → Archive (Ingest Service) | Customer/Supplier; Open Host Service | gRPC SIP submission (streaming); same gate as RDU-Tooling; SIPs may be metadata-only, content-bearing, or mixed |
| Ingest Service → Archive Storage | Internal; gRPC | Unary `CommandAPI` calls (mTLS-authenticated; Ingest is one of two authorised principals); internal-only, never producer-facing |
| Preservation admin tooling → Archive Storage | Internal; gRPC | Unary `CommandAPI` + `QueryAPI` calls (mTLS-authenticated; distinct role from Ingest); VPN-scoped at network layer; for preservation actions (ARK reservation, format migration, GDPR redaction, etc.) |
| Archive Storage → Access Area | Customer/Supplier; Published Language (DAO events) over Open Host Service | gRPC `EventStream` (server-streaming, resumable via `last_event_id`) + gRPC bitstream retrieval (for Service File derivation) |
| Archive Storage → ARK Resolver | Customer/Supplier; Published Language | gRPC `EventStream` (filtered to identity-creation events client-side) |
| VRE → RDU-Tooling | Anti-Corruption Layer | RDU-Tooling translates VRE export into SIPs; insulates the Archive from VRE retirement |
| ARK Resolver → Access Area | Non-domain integration | HTTP redirect; subdomain selected by URL suffix or Accept header |
| Access Area subdomain implementations → Mosaic | Shared Kernel | Cargo crate dependency (used by HTML/Web and Custom Presentation subdomains) |

## Boundary commitments

- **OCFL is exclusive to the Archive context.** No other context reaches into the OCFL store directly. All external access is via the Archive's gRPC interfaces (`CommandAPI` internal-only per decision 47; `QueryAPI` + bitstream retrieval per-role; `EventStream` server-streaming for subscribers). See `modules/archive/dao-discovery.md §9.3` and decision 47.
- **Ingest Service is the sole producer-facing surface of the Archive.** All Producer submissions (RDU-Tooling, self-service preservation frontend, future Producer-side tools) go through Ingest; the SIP is the universal Producer wire-form. Archive Storage's `CommandAPI` is internal-only, accepting mTLS-authenticated principals (Ingest Service; DaSCH-internal preservation admin tooling). See decision 47.
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
