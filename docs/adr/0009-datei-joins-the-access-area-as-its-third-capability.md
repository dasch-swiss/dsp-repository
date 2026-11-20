# DaTEI joins the Access Area as its third capability

DaTEI, a viewer for TEI-based digital editions, exists as a prototype in `dsp-incubator/da-tei`: a configurator
GUI that loads TEI, ODD and CSS into a SQLite read model and lets a person configure how an edition renders, and
a read-only presenter that serves it. It is wanted here as the presentation of edition projects that need a
custom one, in place of a CPE page: first the Gottfried Keller edition (eHKKA, 083C), then the Sephardic Bible
edition of *Mapping the Scriptures in Western Sephardic Literature* (086A). DaTEI is close to what CPE does,
tailored to digital editions rather than generic projects, and it is meant to become fully configuration-based,
so that a project's files plus a configuration are turned into what the presenter serves, and edition pages are
set up by RDU, with no code or infrastructure change. The vocabulary is in `areas/access/datei/CONTEXT.md`. The
decision, phrased so a violation is describable in code:

**The capability and its crates**

- **DaTEI is the third capability of the Access Area's modulith (ADR-0003), at `areas/access/datei/`**, a
  sibling of DPE and CPE, never a project kind inside CPE. Normally a project has a DaTEI or a CPE presentation;
  nothing forbids both.
- **Only the presenter and the rebuild move here.** The configurator is an authoring surface and belongs on the
  Deposit side; where it lives there is not decided, and it stays in the incubator. The configuration document,
  `datei.toml`, is the contract between the two.
- **DaTEI's crates follow ADR-0003's roles:** `datei-ports`, `datei-core`, `datei-store`, `datei-store-build`,
  `datei-web` and `datei-server`. `datei-store-build` is the role ADR-0007 names for CPE: the library that
  writes the read model, the Rebuild included. `datei-store` holds the database type `IrDb` (opening, pragmas,
  schema, the step that finishes a built model, reads) and the read-only pool; `datei-store-build` holds the
  Rebuild's writes, through `IrDb`. The edges:
  - `datei-core` depends on `datei-ports`;
  - `datei-store` on `datei-core` and `datei-ports`;
  - `datei-store-build` on `datei-core`, `datei-ports` and `datei-store`, unlike CPE's, which does not depend on
    `cpe-store`;
  - `datei-web` on `datei-core` and `datei-store`, and on `datei-store-build` from `testonly` targets only, to
    seed its tests;
  - `datei-server` on every DaTEI crate.

  `datei-web` reads through `datei-store`, the edge `cpe-web` has on `cpe-store`: the viewer's SQL lives in
  `datei-store` functions over a handle type, and neither `datei-web` nor `datei-core` names a database crate.
  The presenter's own SQL (search with its query deadline, index pages, finders) lives in `datei-server`, which
  may name `rusqlite` for it; moving it into `datei-store` is DEV-7939. Filesystem work (the read-model
  directory, the marker, start-up cleanup) lives in `datei-server`, the host crate; `datei-ports`, `datei-core`
  and `datei-store-build` do none.
- **`datei-server` is DaTEI's host crate and its facade**, `Datei`, which `access-server` mounts. The incubator's
  `tei-serve` binary is not carried over.

**The port and a project's files**

- **DaTEI's store is a derived read model (ADR-0008):** rebuilt from empty at activation from the files its port
  serves, the configuration among them, never committed, never shipped in the image, never repaired in place.
- **DaTEI declares one port, `DateiFiles` in `datei-ports`, and `media` implements it.** File bytes are what
  `media` serves, and `media` is the Access bucket's one consumer (ADR-0003). `sync` stays the writer of the
  archive projection and serves DaTEI nothing; DaTEI declares no port against `sync`. The port has two
  operations, keyed like CPE's `DepositedFiles`:
  - `projects()`: the shortcodes whose directory holds both the Ready token and DaTEI's configuration under its
    fixed name, `datei.toml`. A project gets DaTEI pages when its configuration arrives; nothing else registers
    it, and DaTEI never sees a half-written project. This is the listing of projects ADR-0012 allows; the port
    lists no files.
  - `file(shortcode, name)`: a file's bytes by stored name, whole or refused, with *not found* distinct from
    *unavailable*.

  The Rebuild reads one project through a narrower trait in `datei-ports`, `FileSource::file(name)`, by stored
  name, which a wrapper binds to one shortcode. `ProjectFiles` in `datei-core` reads the layout and manifest
  through it and resolves each logical path to its stored name.
- **Every input of an edition is deposited and comes through the port.** The configuration, the Edition files,
  the ODD, the Project CSS, the Driving files and the Tables of contents reach DaTEI through its port against
  `media` (ADR-0012), whoever authored them. Nothing under `areas/access/datei/` is one of them, and DaTEI never
  reads `media`'s directory itself.
- **DaTEI learns a project's files from two documents read by fixed names.** `file-layout.xml` lists every file
  of the edition by logical path (its path in the edition's own directory tree) and role; its SHA-256 values are
  advisory, and a mismatch is a warning naming the file. `file-manifest.json` maps each logical path to the
  stored name under which `media` holds the file, or to an IIIF image; without a manifest, stored names are the
  logical paths. Logical paths are DaTEI's identity for files and the manifest's keys, so cross-references, ODD
  includes and durable ids do not depend on stored names. DaTEI reads only the manifest format and version it
  knows, and refuses others.
- **The file mapping is the contract between a DaTEI page and the VRE bridge.** The mapping, made by hand with a
  DaTEI page, maps each logical path to the IRI of the VRE resource that holds the file. The VRE bridge (P-DEV-301)
  resolves each IRI, writes the file under its stored name, and writes the manifest from the mapping; DaTEI does
  not read the mapping. `datei.toml`, `file-layout.xml` and the mapping are among the bridge's shipped inputs,
  and the bridge writes them into the project's directory; DaTEI reads the first two. Each refresh is a new
  generation of that directory, made current by replacing its Ready token with one file rename
  (`areas/deposit/ADR-0005`), so a project's files do not change while a reader reads them.
- **`media` starts at its minimum, as `sync` does (ADR-0007):** a stand-in for the Access bucket (ADR-0012),
  the directory `MEDIA_DATA_DIR` with one directory per shortcode, laid out as the bucket will be and behind the
  port the bucket will stand behind; a `Fake<Port>` in DaTEI's tests is the second adapter. An edition's files
  are not committed to this repository: on a server they arrive through the VRE bridge, which records their
  provenance, and DaTEI's tests build their projects from fixtures.

**Activation and the read model on disk**

- **DaTEI activates in the background, after the bind.** `access-server` binds and reports healthy at once,
  then starts DaTEI's activation once CPE's has ended, or at once when CPE is not mounted, so the two do not
  build at the same time. `datei-server` names no CPE type. DaTEI then asks its port for projects and
  builds each read model from empty, one project at a time, into a writable directory, `DATEI_READ_MODEL_DIR`,
  as `<shortcode>.sqlite`. A model is built under another name and renamed into place only when finished, so a
  half-built model is never served.
- **A project answers 503 until its model is ready, and fails alone**, as for CPE (ADR-0007). Every DaTEI route
  answers 503 with `Retry-After` until DaTEI knows its projects; then each project answers 503 until its model
  is ready, and a project whose Rebuild fails answers 503 while every other project keeps serving.
- **A marker ends a crash loop.** A build writes `<shortcode>.rebuilding` and removes it when it ends; a marker
  survives only when the process dies during the build. A project whose marker shows two interrupted builds in a
  row is marked failed and not built, with a log line saying how to clear it.
- **Models are rebuilt at every start.** After a restart a project is away until its model is rebuilt (minutes
  for a large edition). Keeping a model across restarts and serving it during a rebuild is DEV-7938, before
  production.

**The mount**

- **`access-server` nests DaTEI's router at `DATEI_BASE`** (`/datei` in the image), as it nests CPE's at
  `CPE_BASE`. A project's pages are under `{base}/{shortcode}`, and the static files and the self-hosted fonts
  under `{base}/_static/`. There is no overview at `{base}/`. DaTEI's router ends in its own fallback, so every
  path under the base is answered by DaTEI. An empty `DATEI_BASE` leaves DaTEI unmounted.
- **Telemetry stays under `access-server`'s identity**, with spans named after the matched route.
- **DaTEI keeps its own CSS**, as CPE keeps its own component library; it does not adopt Mosaic.
- **DaTEI honours a Hidden project** with the four obligations `areas/access/CONTEXT.md` lists, for the set
  `access-server` hands it.

**Where DaTEI deviates for now**

Each of these is accepted for the first projects. Where its end is known, it is named.

- **No rights per file.** DaTEI's read model stores no rights and the presenter does not filter on them, against
  the rights clauses of ADR-0008, ADR-0012 and ADR-0013. The bridge downloads
  only what the VRE serves publicly, so the anonymous consumer may see every file DaTEI reads. Ends when `media`'s
  port serves the rights the VRE bridge records (ADR-0013).
- **Only the current configuration format.** DaTEI reads `datei.toml` in the format the configurator writes
  today, not version N and N-1 as ADR-0012 asks. Ends with a second DaTEI project, or a configuration DaSCH does
  not maintain.
- **Data refreshed by restart.** New or changed files reach DaTEI at the next restart, not through a change its
  port announces (ADR-0012). Ends with DEV-7938, which also decides how DaTEI learns of a change through its
  port.
- **Images served by the VRE's Sipi**, as IIIF URLs from the manifest, not by `media` (ADR-0012). Ends when
  `media` serves assets.
- **Fragments as `text/html` responses**, with `datastar-selector` and `datastar-mode` headers, not server-sent
  events as ADR-0004 describes, for DaTEI's one fragment endpoint, the search popover. No end is
  scheduled.
- **No end-to-end suite with JavaScript on and off.** No end is scheduled.

## Considered Options

- **DaTEI as a capability of the Access Area's modulith (chosen).**
- **DaTEI as a project kind inside CPE** — rejected: either CPE's engine grows a TEI branch in its shared roots,
  or DaTEI's engine is bolted onto CPE's project convention; both are the branches-in-shared-roots shape that
  isolated files exist to avoid.
- **The SQLite snapshot as the deployed artifact, as the incubator's showcase does, or a rebuild before the
  bind** — rejected: a shipped snapshot is a read model with a second feed, which ADR-0008 rules out, and a
  rebuild before the bind would hold up DPE and CPE for minutes.
- **`sync` as the byte provider** — rejected: file bytes would flow through a capability built for the archive
  projection's RDF facts, and `sync` would become a second thing for every reader that needs a file.
- **A port that lists a project's files, or serves them by content hash, as the incubator's did** — rejected:
  the layout names every file with its role, a listing makes a port drift toward a file system, and strict
  hashes would refuse every file edited in the VRE.
- **Projects listed per deployment, or as committed project folders, as CPE does** — rejected: every new edition
  would need a developer or an infrastructure change.
- **Stored names in `file-layout.xml`, or a join on original file names** — rejected: a layout with stored names
  needs regenerating whenever a file is re-uploaded, and the VRE keeps original names as base names only, which
  are not unique. A mapping by IRI survives re-uploads.
- **One `datei-engine` crate** — rejected: ADR-0003's one crate per role.
- **Views over plain structs, with no `datei-web` → `datei-store` edge and the presenter's SQL in
  `datei-store`** — *deferred*: the cleaner shape, but it touches all rendering and search. DEV-7939 brings it
  back.

## Consequences

- When the Access bucket exists, `media`'s on-disk adapter is replaced and the bridge no longer writes DaTEI's
  inputs. What marks a project complete in the bucket, as the Ready token does in the stand-in, is not decided.
- The incubator's configurator keys projects by its own project id and keeps its configuration in its database,
  so it reads one project through `FileSource` and has no `projects()`.
- Once the crates are here, they are the one copy of DaTEI's engine: the incubator's configurator builds against
  a pinned, read-only copy of them, as its CPE does, so a configuration it previews renders the same here
  (DEV-7937). Engine changes are made here first.

Not yet in code: `areas/access/datei/` holds only `CONTEXT.md`; DaTEI's engine is `datei-engine` in
`dsp-incubator/da-tei`, and there is no `datei-ports`, `datei-core`, `datei-store` or `datei-store-build`
(DEV-7549).
Not yet in code: there is no `datei-web` or `datei-server`; the presenter is the incubator's `tei-serve`, which
serves a prebuilt database and keeps no hidden-project obligation (DEV-7549).
Not yet in code: no activation in the background, no `DATEI_READ_MODEL_DIR`, no marker and no 503 per project
(DEV-7574).
Not yet in code: `media-store` implements no DaTEI port, and `access-server` does not mount DaTEI (DEV-7574).
Not yet in code: the image sets no `DATEI_BASE` or `DATEI_READ_MODEL_DIR`, and no deployment gives DaTEI a
read-model directory (DEV-7935).
Not yet in code: no model is kept across a restart, and none is served during its rebuild (DEV-7938).
Not yet in code: `check-capability-deps.sh` bans no database crate in the `web` role, and no gate script refuses
filesystem calls in `datei-ports`, `datei-core` and `datei-store-build` (DEV-7549).

Enforced by: review. The target is Bazel `visibility` per ADR-0001 for the capability and port seams and the
edges above (**structure**); `check-capability-deps.sh`'s ban on database crates in the `web` role, and a gate
script refusing filesystem calls in `datei-ports`, `datei-core` and `datei-store-build` (**static-analysis**);
and one test per read model that rebuilds it from empty against a `Fake<Port>` and asserts what the presenter
serves is unchanged (**static-analysis**).
