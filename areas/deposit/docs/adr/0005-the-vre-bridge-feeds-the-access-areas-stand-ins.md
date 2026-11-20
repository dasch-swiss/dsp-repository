# The VRE bridge feeds the Access Area's stand-ins from the VRE until the archive exists

The archive (Spycherli) is to publish a projection stream that `sync` consumes and an Access bucket that `media`
consumes, and CPE and DaTEI rebuild from those two channels (ADR-0012). The archive is the last part of the
platform to be built. Until it exists, the VRE (DSP-API) stays the source of truth for project data, while
presentation moves to CPE and DaTEI. Each channel has a stand-in in the meantime, and something has to fill it:
by hand it takes a person, a commit and a release for every change, and a project that is still edited in the
VRE cannot follow. Data ingress belongs on the producer side (ADR-0002), so the Deposit Area fills the stand-ins.
The plan of record is `docs/specs/2026-11-19-vre-bridge/01-vre-bridge-PRD.md`. The bridge's terms belong in
`areas/deposit/vre-bridge/CONTEXT.md`, and the terms both areas speak in the root `CONTEXT.md` (ADR-0002). The
decision, phrased so a violation is describable in code:

- **The VRE bridge is the Deposit Area's third capability (`areas/deposit/ADR-0004`), at
  `areas/deposit/vre-bridge/`, and it is temporary.** It runs in `deposit-server`, the area's one binary
  (ADR-0003). When the archive exists, deposits replace it and nothing on the Access side changes but the
  producer, with one exception: DaTEI's use of the file manifest, whose VRE IRIs and VRE IIIF URLs become ARKs
  and `media` over Vitrinli.
- **It writes the stand-ins of `sync` and `media` into two locations that `access-server` reads**, the second
  path that crosses the areas (ADR-0002): a `sync` subtree and a `media` subtree, each holding one directory per
  project. Each location is a setting on both sides, and may be a network file system on another host. The
  bridge is the subtrees' only writer; `sync` and `media` each read only their own subtree. Nothing else in
  either area names them.
- **It refreshes the projects on its deployment's allowlist, each from the VRE server its entry names.** Once a
  night it runs a change check per project (resources changed since T, the instant its data is from, on
  DSP-API's unauthenticated changed-resources endpoint) and refreshes the projects that changed, that have no
  Ready token in one subtree or both, or whose token names another server, or another interim format version,
  transformation version or shipped-input digest than the running bridge's. An operator refreshes one project
  on demand with a `deposit-server` subcommand. The nightly run and an operator's refresh take one lock file,
  a setting of its own on local disk, so a project has one refresh at a time and a night one run. A project
  that leaves a non-empty allowlist loses its Ready tokens first and its data after.
- **A refresh takes a dump with DSP-API's service identity**, never a user account, skipping assets, deletes
  its export on the server whatever the outcome, and transforms the dump with `dao-lift`, which lives in the
  bridge together with the interim format's `FORMAT.md` (ADR-0007). The dump is unpacked only in a directory
  private to the process and deleted on every outcome; `admin.nq` and `permission.nq` are never read.
- **The public guard refuses any dump whose selection holds something the anonymous user may not see.** The
  selection is what the snapshot is made from: current, non-deleted resources and values, without the values
  of a deleted resource. The guard reads only each selected resource's and value's own `knora-base:hasPermissions`
  and only grants to `knora-admin:UnknownUser`; a selected item without permissions is not public. The bridge
  writes the access rights it found as the archive's stand-in for policy data (ADR-0013): "public", or
  "restricted view" for a raster still image the anonymous user may see only at a reduced size. Restricted view
  on any other value or file, and anything the anonymous user may not see at all, refuses the project, and
  every violation is logged. Rights are recorded per Representation version, so a restricted value in an
  otherwise public resource refuses the project too. Every Representation version it writes carries the data
  classification `unclassified`.
- **Files are fetched without credentials, and only the ones a reader needs.** Images stay in the VRE and are
  referenced by their IIIF URL. The text files a DaTEI project's file mapping names (TEI, ODD, CSS) are
  downloaded anonymously and stored under the VRE's internal name, beside a generated file manifest; a refused
  anonymous download refuses the project.
- **Files the VRE does not hold ship in the bridge's package**, per project: CPE's configuration, curation and
  narrative, DaTEI's configuration, file layout and file mapping, and any file a project has outside the VRE.
  The package is their one authoritative copy, and the bridge writes them into the project's `media` directory.
- **A project changes as a whole, by generation.** Each refresh writes a new generation,
  `<subtree>/<shortcode>/<generation>/`, in both subtrees, and makes it current by replacing the project's
  Ready token `<subtree>/<shortcode>/.ready` with one file rename, the `media` token before the `sync` token. A
  reader resolves a project through its token and reads only the generation it names, so it sees the previous
  generation whole or the new one whole. The previous generation stays until the project's next successful
  refresh, so a read that resolved the old token finishes on consistent data; older ones are deleted. The
  token names the generation and its predecessor and records T, the server, the interim format's version, the
  transformation's version and a digest of the project's shipped inputs, the same in both subtrees; a project
  is known to `sync` and to `media` exactly when its Ready token exists. Nothing in this relies on a feature a
  network file system lacks: no change notification, no exchange of directories, no symlink, no lock on the
  subtrees. The Ready tokens are the bridge's only state about projects; beside them it keeps its lock file
  and the date of its last nightly run. A failed or refused refresh leaves the project's tokens, and so what
  readers see, in place, and the other projects go on; at start and before each nightly run the bridge points
  a `media` token that disagrees with its `sync` token back, and deletes the generations no token names.
- **The bridge has no routes and no `web` crate.** Its entry crate is `vre-bridge-server`, in ADR-0003's `server`
  role, exposing the `VreBridge` facade; `deposit-server` depends on it alone, held by Bazel `visibility`. Its
  other crates are `vre-bridge-core` (what to refresh and when, the shipped inputs' digest, outcomes),
  `vre-bridge-store` (the subtrees, generations, Ready tokens and the lock), and two roles this record adds to
  ADR-0003's: `lift`, the ported `dao-lift` with `FORMAT.md` and the public guard, which like `core` uses no
  web framework, network or database crate, and `client`, the DSP-API client (the service
  identity, the export, the change check, anonymous downloads and the dump's private unpacking), which no crate
  but `vre-bridge-server` uses. The bridge depends on `shared-stand-in` for the subtrees' layout and the Ready
  token's types, and on `shared-names` for the name rules (ADR-0002); the readers use both. Only the write and
  replace code (writing a generation, replacing and removing the Ready tokens) lives in `vre-bridge-*` crates.
  The bridge's credentials come from the environment, never from the image or the subtrees. With no output
  location configured, the bridge does nothing, which keeps a public preview of the image inert.
- **The bridge never stops or delays `deposit-server`.** `VreBridge::start` returns no error; recovery and the
  nightly schedule run off the listener's path, and a failure is logged and reported per project.

The DaTEI file mapping and file manifest are DaTEI's contract with the bridge, keyed by Logical path (ADR-0009;
**File mapping** and **File manifest** in `areas/access/datei/CONTEXT.md`).

The allowlist and the servers are `deposit-server`'s environment, and each signing key is a file mounted into its
container, whose path that environment names; all are set per deployment. The
DSP-API service identity's contract (DEV-7936) is provisional until dsp-api merges it.

## Considered Options

- **A capability of the Deposit Area in `deposit-server` (chosen).**
- **A binary of its own, or a separate process run as a scheduled job** — rejected: one deployable binary per
  area (ADR-0003), and the nightly run belongs to the running capability.
- **A capability of the Access Area** — rejected: it would be simpler to deploy, sharing no directory across
  areas, but data ingress is the producer side's, and an Access-Area writer would be a second writer beside
  `sync` and `media`.
- **`media` following the VRE's download links at request or rebuild time** — rejected: the VRE would have to be
  reachable at every rebuild, a file fetched later could be newer than the snapshot it belongs to, and `media`
  would make external requests the bucket's adapter never makes.
- **Assets in the dump** — rejected: the export's assets are all or nothing, most of the download, and come
  without the anonymous filter a download without credentials gives.
- **A scheduled restart of `access-server` after the nightly run** — rejected: a second trigger beside the port
  (ADR-0012); `sync` and `media` announce the project whose Ready token appeared, changed or was removed.
- **Replacing a project's directory with an atomic exchange of two directories** (`renameat2` with
  `RENAME_EXCHANGE`) — rejected: it needs a local file system that supports it, and the subtrees may be on a
  network file system on another host. A symlink swap is ruled out because `media` refuses symlinks.
- **Readers watching their subtree with file-system notifications** — rejected: notifications do not cross
  a network file system; `sync` polls its Ready tokens instead.
- **An operator's refresh by a request file the running bridge picks up** — rejected: a subcommand gives the
  operator the outcome as its exit status, and takes the same lock without a second protocol.
- **A public guard that drops what is not public instead of refusing the project** — *deferred*: it would
  mirror what the anonymous user sees in the VRE and admit projects with a few restricted items, but it needs
  handling for links that would dangle, and a bug in it fails open. The guard yields every violation, so the
  structure allows it; it comes back when a project the allowlist should hold is refused for a few items.
- **A periodic full resync** — *deferred*: the change check misses hard-erased resources and ontology changes,
  but both are uncommon, and neither is expected in the projects synchronised first. It comes back when a project
  on the allowlist shows the gap; until then an operator's refresh closes it per project.
- **A generated file mapping** (from `dsp-tools`' id2iri file or an upload) — *deferred*: the mapping is made by
  hand while a DaTEI page is created, and no `dsp-tools` format is adopted. It comes back when hand-made mappings
  become the bottleneck.

## Consequences

- Changing a project's presentation means a change in the VRE, or a change to the bridge's shipped inputs and a
  `deposit-server` release (ADR-0012).
- Allowlisting a project does not hide it; `ACCESS_HIDDEN_PROJECTS`, set in ops-deploy, stays the setting for
  that.
- A CPE project new to the Access Area still needs its project folder in `access-server`'s image (ADR-0007),
  until CPE discovers its projects through `media` (DEV-7948).

Not yet in code: of the VRE bridge's crates only `vre-bridge-lift` exists; `deposit-server` starts no bridge
(P-DEV-301).
Not yet in code: `shared-stand-in` does not exist (P-DEV-301).
Not yet in code: 0803's data is committed to this repository, not written by the bridge (P-DEV-301).
Not yet in code: of the bridge's shared terms, the root `CONTEXT.md` has only Access rights and Data
classification (P-DEV-301).
Not yet in code: DSP-API has no service identity, so no dump can be taken without a person's account
(DEV-7936).
Not yet in code: no location is shared by `deposit-server` and `access-server` (DEV-7514).
Not yet in code: `sync` announces no changed project and CPE re-activates on none (DEV-7622).
Not yet in code: `media` announces no changed project (DEV-7938).

Enforced by: Bazel `visibility` per ADR-0001 for the bridge's crate edges, `deposit-server` granted
`vre-bridge-server` alone (**structure**); `check-capability-deps.sh` for the `lift` and `client` roles'
third-party bans (**static-analysis**); the single writer by the writing code living only in `vre-bridge-*`
crates, so that no other crate has the write code (**structure**), and no other code writing the subtrees
with `std::fs` (**review**); `access-server` mounting the subtrees read-only is deployment-level, in
ops-deploy, and not checked in this repository; the bridge's tests for the public guard against a seeded
restricted resource and a seeded restricted-view image, for the dump's private unpacking and deletion, for
writing no symlinks, for writing only below allowlisted shortcodes, for removing a project's Ready tokens
before its data, for a Ready token never naming an incomplete generation, and for refreshing on a format,
transformation or shipped-input mismatch (**static-analysis**); an ignored test, run by hand on a real dump,
that the ported `dao-lift` produces the incubator's facts from the same dump (**static-analysis**, manual);
review for the bridge never deciding a right of its own and for the shipped inputs being the one copy.
