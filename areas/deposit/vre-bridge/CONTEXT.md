# VRE bridge

The Deposit Area's VRE bridge capability: it takes an allowlisted project's dump from its VRE server, turns it into
an interim snapshot and the project's media files, and writes them into the Access Area's stand-in locations, the
`sync` and `media` subtrees (`areas/deposit/ADR-0005`). Five crates are planned: `vre-bridge-lift` (`lift/`), the
transformation of a dump into an interim snapshot, with `FORMAT.md` beside it; `vre-bridge-core` (`core/`),
`vre-bridge-store` (`store/`), `vre-bridge-client` (`client/`) and `vre-bridge-server` (`server/`). Only
`vre-bridge-lift` and `vre-bridge-store` exist; `CLAUDE.md` beside this file lists what each crate holds. Terms
both areas use, such as **Shortcode**, are in the root [`CONTEXT.md`](../../../CONTEXT.md).

## Language

**VRE bridge**:
This capability, `areas/deposit/vre-bridge/`, crates `vre-bridge-*`.
_Avoid_: sync (the Access Area's `sync`), pipeline, ingest (the archive's word).

**Refresh**:
The bridge's unit of work for one project: change check, dump, checks, transformation, downloads, writing a
**Generation**, replacing the **Ready tokens**.
_Avoid_: sync.

**Nightly run**:
One pass of the bridge over the **Allowlist**.

**T**:
The `asOf` of the change check a **Refresh** started with: the instant the project's data is from.

**Dump**:
As `dsp-cli`'s `CONTEXT.md` defines it: the BagIt zip of a project that DSP-API's export endpoint returns.
`vre-bridge-lift` reads it unzipped, and only its `data/rdf/data.nq` and `data/rdf/ontology-*.nq`.

**Interim format**:
The shape of an **Interim snapshot**, defined by `lift/FORMAT.md`; this repository's copy is the authoritative one.
Its version, an integer starting at `1` (`INTERIM_FORMAT_VERSION`), is recorded in each **Ready token**.
_Avoid_: DAO (the format follows DAO where DAO has decided and lists every deviation, but it is not DAO).

**Interim snapshot**:
One project's data in the **Interim format**: sorted N-Quads in one named graph, `urn:dsp:project:<shortcode>`.
`vre-bridge-lift`'s `lift` builds it in memory (the `Snapshot` of a `Lifted`) from a **Dump**'s **Selection**, or
refuses: with every **Violation** of the **Public guard**, or with every closed-world violation of the format.

**Selection**:
What an **Interim snapshot** is made from: the **Dump**'s resources that are not deleted, and their values (link
and file values included) that are neither deleted nor superseded through `knora-base:previousValue`. The values
of a deleted resource are not selected, whatever their own flags say.

**Public guard**:
The check of every selected resource and value against its own `knora-base:hasPermissions` in `data.nq`. It
refuses the whole project when anything selected is neither **Public** nor, on a raster still image's file
value, **Restricted view**, listing every **Violation**. It never drops an item (DEV-7990 would).
_Avoid_: permission check (DSP-API's own), filter.

**Violation**:
One selected resource or value the **Public guard** refuses: its IRI, whether it is a resource or a value, its
class and the reason. It never carries the value's content or the permission literal.
_Avoid_: closed-world violation (the format's own breaches, `ViolationReport`).

**Public**:
The anonymous user may view the resource, value or file as a whole: DSP-API's view permission or better for
`knora-admin:UnknownUser`, the only group that counts. Recorded as the **Access rights** "public".

**Restricted view**:
DSP-API's restricted-view permission for the anonymous user on a raster still image's file value
(`knora-base:StillImageFileValue`), which the VRE's Sipi serves at a reduced size. Recorded as the **Access
rights** "restricted-view" on that image's representation. On anything else it counts as not viewable, since
the VRE serves other media in full under it (DEV-7244).

**Access rights** and **Data classification** are shared terms of the root `CONTEXT.md`.

**Generation**:
One **Refresh**'s output for a project in one subtree: the directory `<subtree>/<shortcode>/<generation>/`, written
whole before any **Ready token** names it, and never changed after. The same generation name is used in both
subtrees.

**Ready token**:
The per-project, per-subtree file `<subtree>/<shortcode>/.ready` that names the project's current **Generation**,
holds its provenance and marks the project known; replaced by one file rename.

**Lock file**:
The file on local disk, named by a setting of its own, whose lock every **Refresh** runs
under: the **Nightly run** and the operator's refresh take it without waiting, so at most one refresh runs at a
time across processes. It is never in a subtree; the subtrees themselves are never locked.
_Avoid_: lock on the subtrees (there is none: they may be NFS).

**Nightly-run record**:
`<lock file>.nightly`: the local date of the last **Nightly run** that started, written under the **Lock file**'s
lock before the run does anything else, so a second process, or a restart that night, does not run it again.

**Allowlist**:
The deployment's list of projects the bridge refreshes, each with its VRE server.
