# Build-time inputs come through two channels, one consumer each

A reading capability of the Access Area needs input that is not an RDF fact of the archive: a CPE project's
KDL configuration, its narrative, its per-resource curation and its assets; DaTEI's TOML configuration and an
edition's TEI, ODD, CSS, Driving files and Tables of contents. It also needs a rule for what its read model
holds when the archive holds restricted data, and for how a change reaches it. The archive publishes on two
channels, the projection stream that `sync` alone consumes and the Access bucket of Service Files that `media`
alone consumes (ADR-0003). This record says how the other inputs reach a reader over those channels, what a
presentation serves, how change arrives, and what a capability does before the archive exists. The decision,
phrased so a violation is describable in code:

- **A deposited file reaches a reader as bytes through a port against `media`.** A project's configuration
  (CPE's KDL, DaTEI's TOML), its curation, its narrative, its assets and an edition's files are files the
  project or DaSCH deposits on the project's behalf, whoever authored them. The archive preserves them and
  publishes each as a Service File that is the file itself. A reading capability that needs them at build time
  declares a port for bytes, as DaTEI's port does (ADR-0009), and `media` implements it beside its data; CPE
  declares such a port too. The port carries files by shortcode and name, whole or refused, and nothing else:
  no fact, no meaning of a key. It may also list the projects whose directory holds the capability's
  configuration under its fixed name, so that a project gets a presentation when its configuration arrives, as
  DaTEI's does. That listing walks no project's files, which is what makes a port drift toward a file
  system, and the Access bucket answers it by prefix. What a CPE project folder holds is in ADR-0007.
- **A configuration is an opaque file with a declared format.** A project's configuration carries a format
  identifier and a schema version (`cpe-config`, version N; DaTEI's TOML under an identifier of its own), and
  the capability reads version N and N-1. A schema bump is a migration deposit that DaSCH runs on the
  project's behalf, an ordinary format migration on the archive's side. The configuration is never lifted into
  RDF: the archive records that a Representation is the presentation configuration for engine X at schema Y,
  and the bytes stay opaque to the archive, to `media` and to the port.
- **`sync` serves archive-shaped facts only.** The port against `sync` speaks archive-shaped facts as the
  archive records them, and a read model carries no fact the projection does not (ADR-0007, ADR-0008). A value
  the archive does not record, such as a project's curation, comes through the bytes port.
- **A presentation serves what the requesting user may see, and filters as the anonymous consumer until an
  authorisation mechanism exists.** The target is that CPE and DaTEI answer each request for its principal,
  with the decision ADR-0013 records. The ports are built that way from the start: the port against `sync`
  serves every resource with its rights attached, the bytes port serves every file with its rights attached,
  the read model stores rights per row, and the presenter filters at request time, a page, a search, a facet
  count and a listing alike. No port pre-filters for the anonymous consumer: a read model built from a
  pre-filtered port cannot get to the target. A rebuild (ADR-0008) reads on no one's behalf. Until an
  authorisation mechanism exists, no request carries a principal, so the presenter filters every request as
  the anonymous consumer and serves everyone the same page.
- **An asset is fetched at request time, through `media`, never consumed at build time.** An image is an IIIF
  URL and every other file a download, both served by `media` over Vitrinli when the request comes, so that
  the authorisation check (ADR-0013) runs then. A reader stores a file's identity and builds its URL; it never
  copies, embeds or parses an asset's bytes into its read model. DaTEI's Edition files, ODD, Project CSS,
  Driving files and Tables of contents are build-time inputs, not assets: the Rebuild parses them into the read
  model and the Presenter serves rendered text. The rights a rebuild stores with each row are what make that
  consistent: a rendered text is filtered as the file it came from would be.
- **Change reaches a reader only through its port, never by watching the provider's directory, and the port
  announces it.** A port carries a change subscription, and the trigger lives on the provider's side. In the
  target, `sync`'s stream consumer announces a project's new sequence through the port, and `media` does the
  same for the bucket. For the stand-ins, `sync` and `media` each own a Ready-token poll, which reads each
  project's Ready token in its own subtree at a fixed interval and makes its adapter announce the project whose
  token appeared, changed or was removed; the VRE bridge triggers it by replacing or removing a project's
  token. A poll, not a file-system notification, because the subtrees may be on a network file system. Both
  ports serve a project's provenance, from its Ready token, with the project's data, and a reader that finds
  the provenance changed during its rebuild, or differing between the two ports, keeps its previous build of
  the project. A reader rebuilds the one project the port named, and every project at start; CPE and DaTEI grow
  no endpoint of their own, and the binary is never restarted to refresh data. The Ready-token poll is the
  stand-ins' way in before the archive exists, and there is no second trigger to remove later.
- **Before the archive exists, each channel has a stand-in, written by the VRE bridge.** `sync`'s interim
  snapshots stand in for the projection stream (ADR-0007); `media`'s on-disk directory, laid out as the Access
  bucket will be, stands in for the bucket. Each sits behind the port and adapter the real channel will stand
  behind. Both are subtrees of one directory that the Deposit Area's VRE bridge writes from the VRE and from
  the files it ships (`areas/deposit/ADR-0005`), and that `access-server` reads (ADR-0002). The bridge replaces
  a project's directory in a subtree as a whole, with a Ready token that records its provenance and makes the
  project known to `sync` or `media`. A project's `media` directory holds its files under their stored names
  and, for a DaTEI project, the file manifest, which names the VRE resource each file came from; a reader
  learns of both only through its port. Before the archive exists, changing a project's presentation means a
  change in the VRE, or a change to the bridge's shipped inputs and a `deposit-server` release; afterwards it
  means a deposit. That cost is accepted.

## Considered Options

- **Deposited files through a bytes port against `media` (chosen).**
- **`sync` as the sole consumer, bytes through a port against `sync`** — rejected, as ADR-0009 rejects it:
  file bytes would flow through a capability built for the projection's RDF facts, and `sync` would become a
  second thing for every reader that needs a file.
- **Curation served by the port against `sync`, beside the facts** — rejected: it loosens "archive-shaped
  facts only" for one kind of value and puts a hand-authored file with `sync`'s data, where a second one would
  follow. Once a deposited file is a Service File, the bytes port carries curation as it carries
  configuration, and the loosening is not needed.
- **Configuration and curation authored in the project folder as a named second feed** — rejected: the
  project folder would hold durable, hand-edited input the archive never preserves, and a rebuild against a
  `Fake<Port>` would no longer cover what the capability serves (ADR-0008's reason for feeding a read model
  from its ports only).
- **Configuration as facts in the projection** — rejected: a KDL or TOML document is not a fact of a resource,
  and lifting it into RDF would make `sync` carry a consumer's shape, which ADR-0007 and ADR-0008 rule out.
- **A configuration without a declared format** — rejected: a schema change in CPE or DaTEI would break every
  deposited configuration at once, with no record of which version a file was written for.
- **Presentations that hold public data only, from ports that pre-filter for the anonymous consumer** —
  rejected: a read model built that way cannot serve a signed-in user what they may see, so reaching the
  target would mean a second port and a different read model. Rights per row cost a predicate in every
  search, facet and count, which ADR-0013 accepts.
- **An operator endpoint or signal on CPE or DaTEI that re-runs activation** — rejected: a second trigger
  beside the port, to be removed once the port carries change. The stand-ins' Ready-token poll lives with the
  provider and makes the port announce, so it is the trigger the target keeps. A scheduled restart of `access-server`
  after the VRE bridge's nightly run is rejected for the same reason.
- **Stand-ins fed by hand, committed to this repository** — rejected: every change to a project needs a person,
  a commit and a release, and a project that is still edited in the VRE cannot follow.

## Consequences

- A configuration, a TEI file or a curation file is its own Service File (the root `CONTEXT.md`, **Service
  File**), and `areas/archive/CONTEXT.md`'s **Data product** names the two channels.
- `areas/access/media` holds the stand-in for the Access bucket, configured by a `MEDIA_DATA_DIR` beside
  `sync`'s `SYNC_DATA_DIR`; both name a subtree of the directory the VRE bridge writes.
- ADR-0013 records the authorisation decision behind the presentation clause: the archive owns the policy
  data, one shared function decides, and each capability enforces in its own query layer.
- DaTEI deviates for now from the clauses on rights, configuration versions, change subscription and asset
  serving; ADR-0009 lists each deviation with its end.

Not yet in code: a configuration carries no format identifier or schema version, and no capability reads
version N and N-1 (not scheduled).
Not yet in code: no port serves rights, no read model stores them per row, and no presenter filters on
them; the port against `sync` and CPE's store carry no rights (not scheduled).
Not yet in code: CPE subscribes to no change the port against `sync` announces, and a restart is the rebuild
(DEV-7622).
Not yet in code: CPE rebuilds no single named project; once `sync` announces, CPE re-activates every project
(not scheduled).
Not yet in code: the bytes port against `media` carries no change subscription and `media` polls no Ready
token (DEV-7938).
Not yet in code: the stand-ins are committed to this repository and fed by hand, and no VRE bridge writes
them (P-DEV-301).
Not yet in code: the stand-ins are carried in `access-server`'s image, not in a mounted directory (DEV-7514).
Not yet in code: CPE compares neither port's provenance before it keeps a rebuild (DEV-7622).
Not yet in code: CPE's rebuild test runs against `FakeArchiveProjection` only, with curated values passed in by
hand; no rebuild test runs against a fake of the bytes port (not scheduled).
Not yet in code: CPE reads Incunabula's `assets/` from `areas/access/cpe/projects/incunabula/` at activation
and serves them from memory; `media` does not serve assets at request time (not scheduled).
Not yet in code: CPE's images are served by DSP's IIIF server (`iiif-base` in a project's KDL), not by the
Access Area's `media` over Vitrinli (not scheduled).
Not yet in code: the VRE bridge's file manifest points DaTEI's images at the VRE's IIIF server, not at `media`
over Vitrinli (not scheduled).

Enforced by: Bazel `visibility` per ADR-0001 for "a reader reaches `media` only through its bytes port", as for
the `sync` edge: `media-store` grants only the composition root, and another capability sees only its `testonly`
variant, from its tests (**structure**); one test per read model that rebuilds it from empty against a `Fake<Port>` for
each of its ports (**static-analysis**; for CPE's port against `sync`,
`areas/access/cpe/store-build/tests/it/rebuild.rs`). "No port pre-filters", "the presenter filters at request
time", "no reader grows a trigger of its own" and "an asset is never consumed at build time" stay with
**review**.
