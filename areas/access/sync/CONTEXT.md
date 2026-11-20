# sync

The Access Area's `sync` capability: the single writer of the archive projection (ADR-0003, ADR-0008) and
the first Access-Area capability without a screen (ADR-0007). It exists so far at its minimum, the crate
`sync-store` in `store/`, which serves CPE's archive projection port from the `sync` stand-in subtree, one
Interim snapshot per known project, with no Chischtli, replay or bus. `sync` is the single consumer of the archive's projection stream; the Access
bucket is `media`'s, and a deposited file such as a project's curation file reaches CPE through its bytes port
against `media` (ADR-0012). `access-server`
constructs it over the directory `SYNC_DATA_DIR` names (default `areas/access/sync/data`, the committed fixture) and
hands it to CPE's activation. ADR-0007 records why the projection is interim-DAO. The
port's terms (Archive projection port, Project snapshot, Source property, Archive-shaped fact, Annotation)
are in
[`../cpe/CONTEXT.md`](../cpe/CONTEXT.md), the Access Area's in [`../CONTEXT.md`](../CONTEXT.md), the
contract terms (Project, Shortcode, Generation, Ready token) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`,
and the Interim format and Interim snapshot in [`../../deposit/vre-bridge/CONTEXT.md`](../../deposit/vre-bridge/CONTEXT.md).

## Language

**Interim format**:
Defined by the VRE bridge, which writes it (Interim format, in
[`../../deposit/vre-bridge/CONTEXT.md`](../../deposit/vre-bridge/CONTEXT.md)); `store/src/vocab.rs` is transcribed from
its `FORMAT.md`. `sync-store` reads the format versions in its `READABLE` list, the current one and the one before
it once there is one, today only `1`; a token naming any other is `Unavailable`. A version enters `READABLE` in an
`access-server` release before the bridge writes it.
_Avoid_: DAO (the format is DAO-shaped, not DAO), knora-base export, dump (the VRE export it is made from).

**Known project by token**:
A project whose Ready token, `<root>/<shortcode>/.ready`, exists in the `sync` subtree: `sync-store` serves it from
`<root>/<shortcode>/<generation>/snapshot.nq`, the Interim snapshot of the generation the token names, and reads
nothing else. A shortcode `is_shortcode` refuses, or one without a token, is `UnknownProject`, whatever the
subtree holds. A known project whose token cannot be read or parsed, names an unread format version or a
generation that is not there, or whose snapshot cannot be served is `Unavailable`, never `UnknownProject`: a
broken project is not a removed one. A project becomes known when the VRE bridge writes its token; only the
Ready-token poll lists projects, and serving one never does.
_Avoid_: active project (CPE's activation is its own), known list, known by directory.

**Provenance**:
What a project's current data was made from, as its Ready token records it (generation, as-of time, server,
interim format and transformation versions, inputs digest), served by the port beside the snapshot and read from
the token on every call. `media`'s port serves the same project's provenance from its own token, and the two are
equal when both subtrees hold the same generation. A reader compares it and never interprets it.
_Avoid_: version (the format and transformation versions are two of its fields), `PROVENANCE` (the committed
fixture's file on where its bytes came from).

**Ready-token poll**:
How `sync` announces change before the projection stream exists (ADR-0012). The first `subscribe` on
`LiveArchiveProjection` reads every known project's token as a baseline, then a thread of its own,
`sync-ready-token-poll`, reads every token again each interval (`SYNC_POLL_SECS` in `access-server`, default 30 s)
and tells each listener the shortcode of every project whose token appeared, changed or went. A token that cannot
be read is skipped for the round and logged once when it starts and once when it stops failing. A poll, never a
file-system notification: the subtree may be NFS. Dropping the adapter stops the thread. A project whose
token went was removed while the subtree still holds its directory or another project's; `location_present`
answers `false` for a subtree that holds no project directory or cannot be read, as when a mount loses its
target, and CPE then keeps what it served.
_Avoid_: watcher, inotify, reload, refresh (the bridge's; the poll only announces).

**Committed fixture**:
`data/`, project 0803 laid out as the VRE bridge writes it: its token and one generation, whose `snapshot.nq` is
`dao-lift`'s output on a VRE dump, committed with its `PROVENANCE`. The tests read it, and
`access-server`'s image carries it until the image stops carrying stand-in data.
_Avoid_: Project snapshot (the port's DTO, what one call returns), committed snapshot (this directory's Committed fixture), data product
(what the archive will deliver in its place), the projection (the target design, held in Chischtli).

**`LiveArchiveProjection`**:
`sync-store`'s adapter for CPE's archive projection port. Each call reads the project's Ready token, then the
Interim snapshot of the generation it names, parses it strictly and maps it to the port's DTOs. The generation
directory must be a real directory and `snapshot.nq` a regular file, each checked without following a symlink;
the window between that check and the read is accepted, since only the VRE bridge writes the subtree. It serves the
whole snapshot or refuses it: a snapshot file that is missing, a symlink, unreadable, not valid N-Quads, without
resources, or that breaks a fact the port serves makes the call `Unavailable`, as does a snapshot that fails
the port's contract, run on every call.
An annotation target that is not a resource of the file refuses the snapshot (`UnknownTarget`), unlike a
link to an IRI outside the file, which is dropped.
What `FORMAT.md` lets a reader omit, and what the port does not carry, is omitted and never an error.
The **Data ARK** ([`../cpe/CONTEXT.md`](../cpe/CONTEXT.md)) of each resource is derived from its IRI
in `ark.rs`, not read from the file, until DAO carries it;
a served resource whose IRI yields none refuses the snapshot (`NoDataArk`).
_Avoid_: the sync port (CPE declares the port; `sync` implements it), cache (it holds nothing between
calls), `FakeArchiveProjection` (CPE's in-memory adapter for its own tests).
