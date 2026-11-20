# VRE bridge

The Deposit Area's VRE bridge capability: it takes an allowlisted project's dump from its VRE server, turns it into
an interim snapshot and the project's media files, and writes them into the Access Area's stand-in locations, the
`sync` and `media` subtrees (`areas/deposit/ADR-0005`). Five crates are planned: `vre-bridge-lift` (`lift/`), the
transformation of a dump into an interim snapshot, with `FORMAT.md` beside it; `vre-bridge-core` (`core/`),
`vre-bridge-store` (`store/`), `vre-bridge-client` (`client/`) and `vre-bridge-server` (`server/`). Only
`vre-bridge-lift` exists; `CLAUDE.md` beside this file lists what each crate holds. Terms both areas use, such as
**Shortcode**, are in the root [`CONTEXT.md`](../../../CONTEXT.md).

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
Its version, an integer starting at `1`, is recorded in each **Ready token**.
_Avoid_: DAO (the format follows DAO where DAO has decided and lists every deviation, but it is not DAO).

**Interim snapshot**:
One project's data in the **Interim format**: sorted N-Quads in one named graph, `urn:dsp:project:<shortcode>`.
`vre-bridge-lift`'s `snapshot` builds it in memory as a `Snapshot` from a **Dump**, or refuses with every
violation of the format's closed world.

**Generation**:
One **Refresh**'s output for a project in one subtree: the directory `<subtree>/<shortcode>/<generation>/`, written
whole before any **Ready token** names it, and never changed after. The same generation name is used in both
subtrees.

**Ready token**:
The per-project, per-subtree file `<subtree>/<shortcode>/.ready` that names the project's current **Generation**,
holds its provenance and marks the project known; replaced by one file rename.

**Allowlist**:
The deployment's list of projects the bridge refreshes, each with its VRE server.
