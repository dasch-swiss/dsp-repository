# Context Map

System-level view of the DaSCH Repository's bounded contexts and how they integrate. Each context's ubiquitous language is defined in its own `CONTEXT.md`.

## Bounded contexts

| Context | Purpose | Code location | Status |
|---|---|---|---|
| **Archive** | OAIS Archive Component. Holds Archival Masters (Representations) and their descriptive/preservation metadata in OCFL. Source of truth for all preserved data. Self-built (`modules/archive/dao-discovery.md` decision 17). | `modules/archive/` | Specification in progress; implementation pending. |
| **Access Area** | Holds Service Masters (derived projections of Representations). Single source for all read paths. Subscribes to Archive events. | TBD | Not yet built. |
| **ARK Resolver** | Persistent-identifier service. Mints, binds, and resolves ARKs. Own event store. Subscribes to Archive events. | TBD | Specified in `modules/archive/dao-discovery.md §2.3`. Implementation pending. |
| **RDU-Tooling** | Producer-side data preparation. Translates VRE export into Archive Commands. Issues `SubmitDeposition`, `ReserveArk`, etc. | TBD (producer-side, may live in a separate repo) | Not yet built. |
| **DPE** (Discovery and Presentation Environment) | Default public read interface over Access Area projections. | `modules/dpe/` | In development. |
| **CPE** (Configurable Presentation Environment) | Project-specific custom presentations over Access Area projections. | TBD | Not yet built. |
| **VRE** (Virtual Research Environment) | Active-research data platform. Produces exports consumed by RDU-Tooling. | Out of scope (separate codebase) | Existing; being separated from the Repository. |

## Not a bounded context

- **`modules/mosaic/`** — UI component library / design system. Shared kernel for DPE and (future) CPE. No domain, no `CONTEXT.md`.

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
                                          └─────────────────────────┘      (during Service Master
                                                                            derivation)

Access Area  ────►  DPE   ────►  Mosaic (shared kernel)
             ────►  CPE   ────►  Mosaic
             ────►  IIIF server
             ────►  asset server

ARK Resolver  ──── HTTP redirect ───►  DPE
```

## Integration patterns

| Upstream → Downstream | DDD pattern | Wire shape |
|---|---|---|
| RDU-Tooling → Archive | Customer/Supplier; Open Host Service | Commands API (HTTP); validation gate at the Archive |
| Archive → Access Area | Customer/Supplier; Published Language (DAO events) over Open Host Service | SSE event feed (`text/event-stream`, resumable via `Last-Event-ID`) + Binary retrieval API (`GET /bitstreams/{multihash}`) |
| Archive → ARK Resolver | Customer/Supplier; Published Language | SSE event feed (subset: identity-creation events) |
| VRE → RDU-Tooling | Anti-Corruption Layer | RDU-Tooling translates VRE export into Archive Commands; insulates the Archive from VRE retirement |
| Access Area → DPE / CPE / IIIF server / asset server | Customer/Supplier | Read API or shared read-store access (shape TBD) |
| DPE / CPE → Mosaic | Shared Kernel | Cargo crate dependency |
| ARK Resolver → DPE | Non-domain integration | HTTP redirect (ARK string → DPE URL, Version suffix preserved) |

## Boundary commitments

- **OCFL is exclusive to the Archive context.** No other context reaches into the OCFL store directly. All external access is via the Archive's three public APIs (Commands, Events SSE, Binary retrieval). See `modules/archive/dao-discovery.md §9.3`.
- **DAO is the Archive context's ubiquitous language.** It appears at boundaries (Commands input, Events output) — not as a universal language. Other contexts have their own internal vocabularies and translate at the seam. See `modules/archive/dao-discovery.md §1a` and decision 8.
- **ARKs are the only long-term-stable identifiers.** Internal IRIs and presentation URLs may change across system migrations; ARK strings may not. See `modules/archive/dao-discovery.md §2.7`.
- **Push, not pull, for Archive → Access.** The Archive emits events; subscribers project. The Archive does not run queries on behalf of downstream contexts. See `modules/archive/dao-discovery.md §9.3`.

## Where to write decisions

| Scope | Location |
|---|---|
| System-wide (cross-context: deployment topology, identifier schemes, event-bus choices) | `docs/adr/` at repo root |
| Context-internal (storage layout, internal class structure, internal API shape) | `modules/<context>/docs/adr/` |

Write an ADR only when the decision is (a) hard to reverse, (b) surprising without context, and (c) the result of a real trade-off with genuine alternatives. If any of the three is missing, skip it.
