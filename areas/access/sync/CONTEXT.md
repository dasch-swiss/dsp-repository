# sync

The Access Area's `sync` capability: the single writer of the archive projection (ADR-0003, ADR-0008) and
the first Access-Area capability without a screen (ADR-0007). It exists so far at its minimum, the crate
`sync-store` in `store/`, which serves CPE's archive projection port from one committed snapshot and one
committed curation file per known project in `data/`, with no Chischtli, replay or bus; nothing constructs
it until DEV-7400. ADR-0007's DEV-7399 amendment records why the committed projection is interim-DAO. The
port's terms (Archive projection port, Project snapshot, Source property, Archive-shaped fact, Annotation,
Curation) are in
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
dump and committed with its `PROVENANCE`. With the project's **Committed curation** it is everything `sync`
holds of a project today. The same dump and the same `dao-lift` commit give the same bytes.
_Avoid_: Project snapshot (the port's DTO, what one call returns; the two files are its source), data
product (what the archive will deliver in its place), the projection (the target design, held in Chischtli).

**Committed curation**:
One known project's **Curation** ([`../cpe/CONTEXT.md`](../cpe/CONTEXT.md)) as a file, `data/<shortcode>-curation.csv`:
one row per resource, one column per key or key and language, and comment columns (`#…`) that are never
served. It is hand-authored: its single writer is a person, and nothing generates it. Since dsp-incubator#500
(DEV-7496), `0803-curation.csv` is the only copy of Incunabula's curation: a change is made here and reaches the
incubator when it re-vendors (`data/PROVENANCE`). Its keys are opaque to
`sync-store`, which names none of them (ADR-0010, proposed). A project without curation commits a file
holding the header alone. Its line ends are LF, which `.gitattributes` sets and the reader enforces; it is
UTF-8 without a byte-order mark, which the reader enforces. `store/src/curation.rs` defines the format.
_Avoid_: configuration (project-wide, in the project's KDL), generated file, Committed snapshot (the
archive's facts; curation is not one).

**Known project**:
A shortcode in `sync-store`'s `KNOWN` set, today only `0803`. A shortcode outside the set is
`UnknownProject`, whatever `data/` holds; a known project whose committed snapshot or committed curation
cannot be served is `Unavailable`. A project becomes known by adding its shortcode, its committed snapshot
and its committed curation together.
_Avoid_: active project (CPE's activation is its own).

**`LiveArchiveProjection`**:
`sync-store`'s adapter for CPE's archive projection port. Each call re-reads the known project's committed
snapshot and its committed curation, parses both strictly and maps them to the port's DTOs. It serves the
whole snapshot or refuses it: a snapshot file that is missing, unreadable, not valid N-Quads, without
resources, or that breaks a fact the port serves makes the call `Unavailable`, as does a snapshot that fails
the port's contract, run on every call. A curation file that is missing or unreadable, or that breaks
its format (`InvalidCuration`), refuses the snapshot the same way: an absent file never reads as "no curation".
An annotation target that is not a resource of the file refuses the snapshot (`UnknownTarget`), unlike a
link to an IRI outside the file, which is dropped.
What `FORMAT.md` lets a reader omit, and what the port does not carry, is omitted and never an error.
The **Data ARK** ([`../cpe/CONTEXT.md`](../cpe/CONTEXT.md)) of each resource is derived from its IRI
in `ark.rs`, not read from the file, until DAO carries it;
a served resource whose IRI yields none refuses the snapshot (`NoDataArk`).
_Avoid_: the sync port (CPE declares the port; `sync` implements it), cache (it holds nothing between
calls), `FakeArchiveProjection` (CPE's in-memory adapter for its own tests).
