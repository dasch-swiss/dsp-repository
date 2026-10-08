# sync

The Access Area's `sync` capability: the single writer of the archive projection (ADR-0003, ADR-0008) and
the first Access-Area capability without a screen (ADR-0007). It exists so far at its minimum, the crate
`sync-store` in `store/`, which serves CPE's archive projection port from one committed snapshot per known
project in `data/`, with no Chischtli, replay or bus; nothing constructs it until DEV-7400. ADR-0007's
DEV-7399 amendment records why the committed projection is interim-DAO. The port's terms (Archive
projection port, Project snapshot, Source property, Archive-shaped fact) are in
[`../cpe/CONTEXT.md`](../cpe/CONTEXT.md), the Access Area's in [`../CONTEXT.md`](../CONTEXT.md), the
contract terms (Project, Shortcode) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Interim format**:
The shape of a committed snapshot: N-Quads that follow DAO wherever its decisions have settled and deviate
only where the port needs a fact DAO drops. `dao-lift` in `dasch-swiss/dsp-incubator` writes it, and its
`FORMAT.md` there defines it and lists each deviation with its reason. The `dao-lift` commit in
`data/PROVENANCE`, today `dasch-swiss/dsp-incubator@b2226ff4a987bf5f4b5cd0f33a8bf923a3033c7f`, pins the
`FORMAT.md` the file follows; `store/src/vocab.rs` records the same commit, and the two move together. The
format is temporary: it moves to DAO once DAO is published (ADR-0007).
_Avoid_: DAO (the format is DAO-shaped, not DAO), knora-base export, dump (the VRE export it is made from).

**Committed snapshot**:
One known project's file in the interim format, `data/<shortcode>.nq`, produced by `dao-lift` from a VRE
dump and committed with its `PROVENANCE`. It is everything `sync` holds of a project today. The same dump
and the same `dao-lift` commit give the same bytes.
_Avoid_: Project snapshot (the port's DTO, what one call returns; the file is its source), data product
(what the archive will deliver in its place), the projection (the target design, held in Chischtli).

**Known project**:
A shortcode in `sync-store`'s `KNOWN` set, today only `0803`. A shortcode outside the set is
`UnknownProject`, whatever `data/` holds; a known project whose committed snapshot cannot be served is
`Unavailable`. A project becomes known by adding its shortcode and its committed snapshot together.
_Avoid_: active project (CPE's activation is its own).

**`LiveArchiveProjection`**:
`sync-store`'s adapter for CPE's archive projection port. Each call re-reads the known project's committed
snapshot, parses it strictly and maps it to the port's DTOs. It serves the whole snapshot or refuses it:
a file that is missing, unreadable, not valid N-Quads, without resources, or that breaks a fact the port
serves makes the call `Unavailable`, as does a snapshot that fails the port's contract, run on every call.
What `FORMAT.md` lets a reader omit, and what the port does not carry, is omitted and never an error.
_Avoid_: the sync port (CPE declares the port; `sync` implements it), cache (it holds nothing between
calls), `FakeArchiveProjection` (CPE's in-memory adapter for its own tests).
