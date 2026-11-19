# The repository root is shaped by the three areas of the Trusted Repository

`dsp-repository` is DaSCH's research data repository: a trusted repository in the sense of the OAIS reference
model, whose job is the long-term preservation and trustworthy dissemination of humanities research data. OAIS
separates the producer who submits data, the archive that preserves it, and the consumer who reads it. The
platform follows that separation with three **areas**. The producer side is deliberately inside the repository
rather than left to external tools: everything the designated community needs, including creating and curating
the data, is part of the repository. The areas are the platform's bounded contexts, and the boundary that
matters most, the sealed archive against everything else, should be the first thing the tree shows. A flat
`modules/` hides which context a service belongs to, and Bazel visibility (ADR-0001) keys on directories, so the
directory is the boundary.

Every crate lives under exactly one of six root directories, and there is no `modules/` directory. The three
areas sit together under `areas/`, so the root shows the two kinds of thing the platform is made of: the bounded
contexts, and what they share or what serves them.

- `areas/deposit/` — the producer side: where a depositing project team creates and edits its data and metadata
  and RDU reviews it before anything is submitted to the archive. Today identity, the metadata editor and the
  VRE bridge, which feeds the Access Area's stand-ins from the VRE until the archive exists
  (`areas/deposit/ADR-0005`). The name is "Deposit", not "Ingest": "Deposit" is the producer-side vocabulary
  the platform already uses (depositor, Deposition, DepositAgreement), while "Ingest" is an OAIS functional
  entity inside the archive, so naming the producer side after it puts the wrong context's word on the door.
- `areas/archive/` — the OAIS archive, working name **Spycherli**: ingest, archival storage and the supporting
  functional entities; the sealed heart of the platform. It holds its `CONTEXT.md` and no crate.
- `areas/access/` — the consumer side, OAIS Access: the services that produce dissemination packages for
  consumers. Today DPE, the Discovery and Presentation Environment, `sync`, `media` and CPE, the Configurable
  Presentation Environment; the further read-side capabilities are listed in ADR-0003.
- `shared/` — `shared-{role}` crates used by more than one area, or decided to be: the record that creates a
  shared crate with one consuming area names the second. Today `shared-metadata` (the research-metadata
  contract), `shared-telemetry` (the browser-beacon collector), `shared-rate-limit` (the client-IP key
  every per-IP rate limit buckets on), `shared-fair` (the FAIR exposure engine, whose Deposit-Area
  consumer ADR-0005 names), `shared-datastar` (typed Datastar actions), `shared-names` (the name rules
  `is_shortcode` and `is_file_name`, by which the VRE bridge writes and `sync`, `media` and CPE read; they
  outlive the stand-ins) and `shared-stand-in` (the stand-in directory below: its layout and the Ready token's
  types, which the VRE bridge and the readers both depend on, while only the write and replace code lives in
  `vre-bridge-*`; it ends with the stand-ins).
- `mosaic/` — the design system, a component library plus its playground, used by the hypermedia servers of
  both areas (CPE keeps its own component library, ADR-0007).
- `vitrinli/` — the media engine (IIIF Image API rendering, range-served downloads, derivation of Service
  Files), sipi under its new name. A library, not a service and not a capability: each area's `media`
  capability depends on it (ADR-0003).
- `chischtli/` — the in-house triplestore engine (an embedded RDF store with graph-granular write dispatch,
  exact update deltas, full-text search and optional SHACL on update), the replacement for Fuseki. A library,
  not a service and not a capability: it stores named graphs for whichever capability owns them (ADR-0003). In
  the Deposit Area it is the working store of data creation and, long-term, the successor of the
  active-research platform's triplestore.
- `dsp-cli/` — the platform's command-line client. It talks to the VRE over DSP-API and to the area moduliths
  over their public HTTP surfaces.

Mosaic, Vitrinli and Chischtli are root peers rather than area members because each is used by more than one
area: Mosaic by DPE, identity and the editor, Vitrinli by both `media` capabilities, Chischtli by the
Deposit and the Access Area moduliths. dsp-cli is a root peer for a different reason: it is used by no area and is a client of
every area, so it belongs to none of them.

Inside an area, each capability is a directory named by its crate prefix, holding its `{capability}-{role}`
crates (`areas/access/dpe/core` holds `dpe-core`, `areas/deposit/editor/web` holds `editor-web`). The editor keeps
the name "editor". The area's composition root is `areas/<area>/server`, and the area's container image is built
in the same directory. What a composition root and a capability's crates may depend on is ADR-0003's.

Vocabulary is placed by specificity. A term specific to one capability or component lives in that capability's
or component's `CONTEXT.md`; a term true across an area lives in `areas/<area>/CONTEXT.md`; a term shared by more
than one area lives in the root `CONTEXT.md`. An area holds an area-level file only when it has area-wide terms,
and every file is indexed from the root `CONTEXT.md`.

Between areas, the rules are:

- **No crate under one area depends on a crate under another area.** Areas integrate over the wire, never by
  Rust import. Until the archive exists, the Deposit Area publishes to the Access Area through this repository:
  the editor's pull-request run turns approved records into pull requests against the committed project files under
  `areas/access/dpe/server/data`, and the VRE bridge writes the stand-ins of `sync` and `media` into a directory
  that `deposit-server` and `access-server` share by mount (ADR-0012). These are the two paths that cross the
  areas, and the only ones; the `just check` script that pins the 0803 `media` fixture byte for byte to the
  bridge's shipped inputs belongs to the second. Both end when the archive exists. In the
  target design, the Deposit Area submits packages to Spycherli over an intent protocol (register an intent with
  a declared manifest, upload to a quarantine bucket, complete, validate), and Spycherli feeds every Access-Area
  service with pointer messages on a message bus plus immutable objects in object storage.
- **`shared-*`, `mosaic-*`, Vitrinli and Chischtli depend on no area crate**; an area depends on them freely. The
  dependency arrow is `areas → shared, mosaic, vitrinli, chischtli`.
- **Each service owns its durable state and is its sole reader and writer** — the editor's SQLite database,
  DPE's data directory and in-process caches, Spycherli's sealed store. A cross-area reference is an opaque
  identifier (a shortcode, an ARK), resolved through the owning area's published surface. The stand-in directory
  is the one exception: the VRE bridge is its only writer, and `sync` and `media` each read only their own
  subtree of it. Inside an area, ADR-0003 applies the same rule per capability.
- **A concept shared by two areas lives in `shared/`** (`shared-metadata` is the research-metadata contract both
  speak); a shape one area happens to need from another does not. Share concepts, never shapes.
- **Preservation storage is exclusive to the Archive Area.** No other area reaches into the sealed store, its
  log, or the preserved bytes; they receive data products the archive publishes.

dsp-cli stands outside the area arrow on both sides. No area depends on `dsp-cli`, and `dsp-cli` depends on no
area crate. It may depend on a `shared-*` crate only if that crate is itself published to crates.io, because
`cargo publish` rejects a path dependency on an unpublished crate. No `shared-*` crate is published, so none
is reachable from dsp-cli; a concept that an area and dsp-cli both need is shared by publishing the
crate, or by vendoring the concept, never by a path dependency.

## Considered Options

- **Areas grouped under `areas/`, shared infrastructure beside it (chosen).**
- **Seven directories flat at the root** (`deposit-area/`, `archive-area/`, `access-area/`, `platform/`,
  `mosaic/`, `vitrinli/`, `chischtli/`) — three bounded contexts and four pieces of shared infrastructure side by
  side do not show which are which, and each area needs an `-area` suffix to say what it is. `areas/` says it
  once.
- **Keep `modules/` as a container above everything** (`modules/areas/…`, `modules/shared/…`) — rejected: one
  more level in every path and every Bazel label, and the word "module" would survive with no meaning; the root
  already separates code from `docs/`, `scripts/` and `.github/` by name.
- **A flat `modules/<service>/` layout** — the area a service belongs to is invisible in the tree, and the seam
  that matters most has no directory of its own.
- **One modulith binary for the whole platform** — rejected. DPE and the editor are separate deployables on
  separate origins on purpose: on a shared origin an XSS in the public, unauthenticated DPE could drive
  authenticated editor mutations past the `Sec-Fetch-Site` CSRF control (`docs/src/editor/architecture.md`,
  "Relationship to DPE"). The modulith is adopted one level down instead, one per area (ADR-0003).
- **`mosaic/`, `vitrinli/` and `chischtli/` under `shared/`** — not chosen. All three are substantial systems with
  their own vocabulary and test infrastructure (and, for Vitrinli, C++ still being rewritten), not single shared
  library crates; `shared/` stays the home of `shared-{role}` crates.
- **`platform/` as the name of the shared root** — rejected: it collides with Bazel's platform vocabulary (the
  `@platforms` repository and the conventional `//platforms` package, ADR-0001), and "platform" already means the
  DaSCH Service Platform (root `CONTEXT.md`). `kernel/` (the DDD "shared kernel") and `contract/` were
  considered; `shared/` is the plain word for what the directory is.
- **Vitrinli inside `areas/access/`** — rejected: the Deposit Area's `media` capability depends on it as well,
  so an area directory would misstate who may depend on it.
- **`areas/deposit/metadata-editor/`** — rejected: the crates would have to follow (`metadata-editor-core`),
  touching every commit scope and every runbook for no gain in meaning.
- **dsp-cli under a new `tools/` root** — rejected: a root-level category for one member states nothing that
  `dsp-cli/` itself does not.
- **dsp-cli inside an area, as a capability** — rejected: dsp-cli talks to every area over the wire, and an
  area's members are capabilities of its one modulith (ADR-0003); dsp-cli is neither a capability of one area
  nor owned by one.

## Consequences

- Commit scopes stay crate names (`CONVENTIONS.md`); an area is not a scope.
- "Deposit Area" is the canonical name; design material that says "Ingest Area" means this area, and the root
  `CONTEXT.md` records the alias under Flagged ambiguities.
- A gate that globs application directories (`check-shared-paths.sh`, `check-datastar-attributes.sh`,
  `verify-checksums.sh`, the `eng.yaml` overrides) covers every root that holds an application crate. A glob
  that misses a root drops its crates silently: the gates' absence checks fire only when nothing at all matches.

Not yet in code: Mosaic is at `modules/mosaic/` (`mosaic-tiles`, `mosaic-playground`), not `mosaic/` (not
scheduled).
Not yet in code: `vitrinli/` and `chischtli/` hold only a `CONTEXT.md`; both engines live in their own
repositories (not scheduled).
Not yet in code: the Archive Area has no crate, so the Deposit Area publishes through pull requests against
`areas/access/dpe/server/data` and through the stand-in directory rather than to Spycherli (not scheduled).
Not yet in code: DPE's terms are in `areas/access/CONTEXT.md`, not in `areas/access/dpe/CONTEXT.md` (not
scheduled).
Not yet in code: dsp-cli calls DSP-API only; no area modulith serves it (not scheduled).
Not yet in code: no stand-in directory is shared by mount; `sync`'s and `media`'s data is carried in
`access-server`'s image (DEV-7514).
Not yet in code: `shared-stand-in` and the VRE bridge do not exist (P-DEV-301).
Not yet in code: no `just check` script pins the 0803 `media` fixture to the bridge's shipped inputs
(P-DEV-301).
Not yet in code: `shared-names` does not exist; `is_shortcode` and `is_file_name` are in `cpe-ports`
(P-DEV-301).

Enforced by: Bazel `visibility` under ADR-0001 for crate edges (structure): every first-party target lists its
consumers, no grant crosses `areas/access` and `areas/deposit` except the published corpus
(`//areas/access/dpe/server:corpus`), which the editor's crates, their tests and the editor image read, no
area target grants a `shared-*` or `mosaic-*` crate, and nothing grants dsp-cli an area or `shared-*` crate; a new
grant is one line in the provider's BUILD file and is reviewed there; `check-capability-deps.sh`, run by
`just check`, for no `//visibility:public` in a `BUILD.bazel` under `areas/`, `shared/`, `modules/` or
`dsp-cli/`, which would undo every grant (static-analysis); the stand-in directory's single writer by
the write and replace code living only in `vre-bridge-*` crates, so that no other crate has it (structure), and
by no other code writing the directory with `std::fs` (review); `access-server` mounting the directory
read-only is deployment-level, in
ops-deploy, and not checked in this repository; `check-shared-paths.sh` for paths from a shared crate into an
area, which no dependency edge shows (static-analysis); `cargo publish --dry-run --locked --manifest-path
dsp-cli/Cargo.toml` in `check.yml` for dsp-cli's publishing manifest (static-analysis); review for state
ownership, shared concepts and the archive's storage.
