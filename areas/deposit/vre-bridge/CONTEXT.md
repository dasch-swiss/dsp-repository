# VRE bridge

The Deposit Area's VRE bridge capability: it takes an allowlisted project's dump from its VRE server, turns it into
an interim snapshot and the project's media files, and writes them into the Access Area's stand-in locations, the
`sync` and `media` subtrees (`areas/deposit/ADR-0005`). It is five crates: `vre-bridge-lift` (`lift/`), the
transformation of a dump into an interim snapshot, with `FORMAT.md` beside it; `vre-bridge-core` (`core/`),
`vre-bridge-store` (`store/`), `vre-bridge-client` (`client/`) and `vre-bridge-server` (`server/`). `CLAUDE.md`
beside this file lists what each holds and which exist. Terms both areas use, such as **Shortcode**, are in the
root [`CONTEXT.md`](../../../CONTEXT.md).

## Language

**VRE bridge**:
This capability, `areas/deposit/vre-bridge/`, crates `vre-bridge-*`.
_Avoid_: sync (the Access Area's `sync`), pipeline, ingest (the archive's word).

**Refresh**:
The bridge's unit of work for one project: change check, dump, checks, transformation, downloads, writing a
**Generation**, replacing the **Ready tokens**.
_Avoid_: sync.

**Nightly run**:
One pass of the bridge over the **Allowlist**, at a configured local time (default 02:00 Europe/Zurich), at most
once per local date across processes (the **Nightly-run record**); a missed one is not caught up. Under the
**Lock file**'s lock for its whole length, it sweeps leftover **Private work directories**, recovers each
allowlisted project, removes the projects no longer on the **Allowlist**, then refreshes each allowlisted
project, retrying one whose slot is **Busy** later in the same run a bounded number of times.

**Removal**:
What the **Nightly run** does to a project no longer on a non-empty **Allowlist**: its **Ready tokens** go first,
`sync`'s before `media`'s, reported as outcome removed; its directories go in a later run, once it had no token
when that run began.
_Avoid_: deletion (only the second step deletes).

**Operator's refresh**:
A **Forced refresh** of one allowlisted project that an operator starts in the bridge's process environment,
`deposit-server vre-bridge-refresh <SHORTCODE>`; it takes the **Lock file**'s lock without waiting and gives up on a **Busy** slot at once. Its exit statuses are
listed under **Outcome**.

**Inactive bridge**:
A bridge that does nothing because no output location is set, or because its configuration is invalid; the reason
names the variable. It never stops `deposit-server`.
_Avoid_: disabled.

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
under: the **Nightly run** waits for it; the **Operator's refresh**, and the bridge's start for its sweep and
recovery, take it without waiting. So at most one refresh runs at a time across processes. It is never in a subtree; the subtrees themselves are never locked.
_Avoid_: lock on the subtrees (there is none: they may be NFS).

**Nightly-run record**:
`<lock file>.nightly`: the local date of the last **Nightly run** that started, written under the **Lock file**'s
lock before the run does anything else, so a second process, or a restart that night, does not run it again.

**Allowlist**:
The deployment's list of projects the bridge refreshes, each with its VRE server. An empty allowlist refreshes
nothing and removes nothing.

**Shipped inputs**:
A project's files that come with the bridge rather than from its **Dump** (`config.kdl`, `curation.csv`,
`narrative/…`), below the inputs directory in `<shortcode>/`; copied into each `media` **Generation**. None may be
named `access-rights.json` or `file-manifest.json`, in any case, file or directory: the bridge writes those itself.
A DaTEI project's shipped inputs also hold `datei.toml`, `file-layout.xml` and its **File mapping**.

**File mapping** and **File manifest** are DaTEI's terms (`areas/access/datei/CONTEXT.md`). The bridge reads the
mapping from the **Shipped inputs** (`vre-bridge-core`'s `FileMapping`) and writes the manifest into the `media`
**Generation**, its entries in the mapping's order (`manifest_bytes`).

**Mapped file**:
The one current file value of a resource a **File mapping** entry names, in the **Selection** the **Public
guard** passed. A text file is downloaded without credentials and stored under its Stored name, the VRE's
internal file name; a still image is recorded by its IIIF reference and stays in the VRE. Any other kind, a
resource outside the selection or a Stored name that is not a file name or collides refuses the project.
`vre-bridge-core`'s `FileValue`; a mapping entry is a `MappingEntry`.
_Avoid_: asset, download link.

**Inputs digest**:
The SHA-256 of a project's **Shipped inputs**, recorded in its **Ready tokens**, so that a change to them refreshes
the project. `vre-bridge-core`'s `inputs_digest` defines what is hashed and in which order.

**Forced refresh**:
A **Refresh** that does not wait for the **Change check** to report a change: a token is missing or broken in a
subtree, the `sync` token's **T** cannot be read, a token names another server than the **Allowlist** entry or
records another interim format version, transformation version or **Inputs digest** than the running bridge's,
or an operator asked. Otherwise the change check asks from **T** minus 60 seconds. `vre-bridge-core`'s `decide`.

**Outcome**:
How a **Refresh** ended: unchanged (the change check listed nothing), refreshed, refused or failed; and, from
the **Nightly run**'s removal only, removed. The **Operator's refresh** exits `0` for unchanged or refreshed,
`2` for refused and `1` for failed; before any refresh, `64` when the bridge is inactive, the project not
allowlisted or `deposit-server`'s configuration does not load, `75` when the lock is held. A refused or failed refresh leaves the project's tokens as they were.

**Refusal**:
An **Outcome** the project's data causes: the **Public guard**, the format's closed world, the server refusing the
**Service identity** or an anonymous **Mapped file** download (`401`, `403`), a **File mapping** that does not
resolve.
_Avoid_: failure.

**Failure**:
Every other unsuccessful **Outcome**: the transport, a time limit, a dump slot still busy when the caller gives up (**Busy**), any other answer of the
server, the **Shipped inputs**, or the bridge itself.
_Avoid_: refusal, error (the log's word for one step).

**Service identity**:
The bridge's identity on a VRE server's DSP-API: a service name, a key id and an Ed25519 signing key that the
server trusts. The bridge signs a short-lived token with it per request; DSP-API accepts it on the four export
endpoints only and treats an export as the service's own by the service name. `vre-bridge-client`'s
`ServiceIdentity`.
_Avoid_: service account, user (it is not a DSP user).

**Change check**:
The question a **Refresh** starts with: which of the project's resources changed since a cutoff, answered by
DSP-API's public `GET /v2/resources/info` together with the server's `asOf`, which becomes **T**.

**Export**:
DSP-API's task that produces a **Dump** (`/v3/projects/{iri}/exports`). The slot is server-wide: a finished export
blocks the next until it is deleted, so the bridge deletes its own on every outcome.
_Avoid_: dump (the zip the export produces).

**Busy**:
Another caller's **Export** holds a server's slot, so a **Refresh** cannot start one (`vre-bridge-server`'s
`Busy`). Not an **Outcome** in itself: the caller tries again or gives up, and only giving up is reported, as
a **Failure** with the reason busy (`FailureReason::Busy`): by the **Nightly run** after its retries, by the
**Operator's refresh** at once.

**Private work directory**:
The directory `vre-bridge-dump-<random>`, mode `0700`, into which a **Dump** is downloaded and unpacked; deleted
on every outcome. It lies below the work directory the caller passes to `private_work_dir`, a deployment setting
that is never inside an output location.
