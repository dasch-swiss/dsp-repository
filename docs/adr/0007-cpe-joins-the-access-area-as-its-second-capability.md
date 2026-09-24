---
status: proposed
date: 2026-09-25
---

# CPE joins the Access Area as its second capability

CPE, the Configurable Presentation Environment, exists today as a prototype in `dasch-swiss/dsp-incubator`
(`cpe/`): a Rust workspace of an engine crate and a binary, per-project bundles of a KDL configuration, a
`data.sql` built by a per-project script from a DSP export, narrative Markdown and assets, embedded into one
binary that hosts every project under a path prefix. The prototype was built to inform DPE's roadmap and was
never meant to ship; it is now wanted as a product, as the public presentation of Incunabula first and of
further projects after it. This record defines the shape the prototype has to be moulded into so that it can
be integrated here, and it is deliberately not bound by the prototype's implementation: as much of the code
survives as is good, and everything may change to fit the target architecture; what must survive is the looks
and the behaviour a visitor sees, and KDL as the authoring language (rationale as stated by the owner,
2026-09-24). The decision, phrased so a violation is describable in code:

- **CPE is the second capability of the Access Area's modulith (ADR-0003), at `areas/access/cpe/`.** Its
  arrival is what turns `dpe-server` into a capability-internal crate and creates the composition root
  `areas/access/server`, which mounts DPE's and CPE's routers on the area's origin and holds wiring only.
  The prototype's single-binary, path-prefix topology (its record `0002-deployment-topology.md`) and its
  "one process per project" production guidance are both retired: there is one Access-Area binary, and a
  project's presentation is a route prefix inside CPE's router, not a deployment.
- **CPE owns its own read-side store, built from `sync`, and never opens the archive projection itself.**
  CPE declares in `cpe/ports` what it needs from the projection; `sync`, the Access Area's single writer of
  the archive projection, implements the adapter beside its data. The port speaks archive-shaped facts as
  the archive records them (classes, typed values, links, files, ordered membership), never CPE's
  presentation model. CPE's store is disposable and is rebuilt from what the port serves; nothing else
  writes it. `sync` starts at its minimum: the committed projection of one shortcode read from disk, behind
  the same port Chischtli and replay will later stand behind. A `Fake<Port>` in CPE's tests is the second
  adapter, so the seam is real from the first commit.
- **Per-project remodelling is declarative first, a typed hook second, never a script.** The mapping from
  archive shape to CPE's presentation model (class and property names and types, link direction, the title
  property, which properties become list vocabularies) is declared in the project's KDL. What the mapping
  cannot say (composed titles, derived vocabularies such as a season or a decade, rule tables, image
  re-modelling) is one Rust function per project, over the port's types, in the project's own folder. A
  project that needs no hook is configuration, narrative and assets only. No embedded scripting language:
  ADR-0002 already rejected untyped per-deployment configuration for Vitrinli because it is what no test
  covers and no agent can trace to an owner, and the authors of a CPE project are DaSCH developers.
- **A project fails alone.** Validation of a project's configuration against its store happens when the
  project is activated, not when the process boots; a project that does not validate is offline on its own
  prefix while DPE and every other project keep serving. A strict mode for development, CI and tests fails
  the whole start on the first invalid project, as the prototype's `--check` does; it is the same verb as
  DPE's `validate`, on the area binary.
- **A project is a folder.** Everything that belongs to one project lives under one directory of the
  capability, discovered by convention: its KDL, its narrative, its assets, and its hook if it has one.
  Adding a project adds a folder and touches no shared registry.
- **CPE has its own component library, inside the capability.** A `cpe-{role}` crate holds CPE's design
  language (the token CSS and the components), because its editorial register is not Mosaic's and is not
  meant to be. Mosaic stays a root peer because the Deposit and Access Areas both use it. The seam is
  one-directional and mechanical: CPE's web crate depends on CPE's library and never on `mosaic-tiles`; no
  crate outside `areas/access/cpe/` depends on CPE's library.
- **Incunabula first, alone.** The first integrated CPE serves one project, 0803, at the state the prototype
  has reached when the move starts. The other prototype projects follow one at a time, each as a folder.
- **No persistent identifier resolves to a CPE page until CPE's pages carry ADR-0005's metadata.** CPE
  record pages will be FAIR-assessable landing pages; the adapter that feeds `shared-fair`'s graphs from
  CPE's store is deferred, and until it exists no ARK points at CPE.

Prototype decisions that carry over as decisions of this repository, by the file name of the record in
`dsp-incubator/cpe/docs/adr/`: KDL as the configuration language (`0006-config-language-kdl.md`);
fragment swaps are transient and never write the URL (`0007-fragment-swaps-are-transient.md`); images are
first-class, embedded-only Resources shared by links (`0008-images-are-embedded-only-resources.md`); IIIF
presentation is CPE-native, no Presentation manifests (`0009-iiif-presentation-cpe-native.md`); the
component vocabulary, the empty-state family and the chrome catalog. Prototype decisions retired by this
record: the single binary with path prefixes (`0002-deployment-topology.md`); bundles embedded at build
time; `data.sql` produced by a per-project script; the settings page as demonstration scaffolding. The
image viewer's packaging (`0010-openseadragon-integration.md`) is carried over as an open question, not a
decision.

## Considered Options

- **CPE as a capability of the Access Area's modulith (chosen).**
- **CPE as its own deployable, the prototype's shape extended** — rejected by ADR-0003: a read-side service
  of its own carries its own consumer of the archive's data products, which is the cost the modulith
  exists to avoid; and "one binary per area" would be false the day the second capability arrived.
- **`sync` mocked to serve CPE's presentation shape directly** — rejected: the remodel would then live in
  `sync`, and every project would edit `sync` to change how it is presented, giving CPE's store a second
  writer.
- **Per-project remodelling in an embedded scripting language (Rhai, Roc)** — rejected: the ADR-0002
  reason above; and Roc is pre-1.0 with a compiler mid-rewrite, a second toolchain inside the Bazel build
  ADR-0001 introduces.
- **One component library for DPE and CPE** — rejected: CPE's editorial design language is a deliberate
  difference, not drift; forcing it into Mosaic's tiles would flatten exactly what a project-specific
  presentation is for.
- **A component ADR series for CPE (`cpe/ADR-NNNN` as ADR-0006 allows root-level components)** — not
  chosen: CPE is a capability of an area, not a root-level component, and its decisions are the area's;
  they live in the root series, as this record does. The prototype's records stay in the incubator as
  history and are cited by file name, never by number.

## Consequences

- The `modules/` to `areas/` move of ADR-0002 for the Access Area (DPE to `areas/access/dpe/`) lands before
  or together with CPE's arrival: `areas/access/server` composes area crates, not `modules/` crates.
- `dpe-core`'s process-global caches and the process-global `set_base_url` of `dpe-api-oai` become
  capability-owned state passed in by the composition root, because a second capability shares the process.
- `sync` becomes the first Access-Area capability without a screen, and the place DPE's own reading of the
  committed corpus moves to when DPE stops reading it directly; that move is not part of this record.
- The prototype's dependency pins are reconciled on entry: the workspace has one `rusqlite` (the editor
  pins 0.38 through `deadpool-sqlite`, the prototype 0.39, and Cargo refuses two `libsqlite3-sys`), one
  CLI and configuration stack (`clap`, `figment`), and no `include_dir`.
- Migrate, then delete: the incubator prototype is archived through the incubator's own lifecycle
  (`/archive-prototype cpe`) once the monorepo CPE serves Incunabula; the two are never maintained side by
  side.
- `ARCH-MAP.md`'s planned entry for `areas/access/cpe` gains a `sync` sibling and the boundary rules above
  (`/dune:dune-map`); `modules/dpe/CONTEXT.md` stops being the whole Access Area's vocabulary and becomes
  `areas/access/CONTEXT.md` with one file per capability once CPE's terms arrive.
- The interaction convention the two capabilities differ on today (DPE's tab fragment writes the URL, CPE's
  swaps never do) is recorded as a per-capability rule, not unified by this record.

Enforced by: review until the move lands; then Bazel `visibility` per ADR-0001 for the capability, port
and component-library seams (structure), a `Cargo.toml` grep in the style of `check-shared-paths.sh` for
the `mosaic-tiles` and cross-capability rules until then (static-analysis), the strict validation mode in
CI for "a project fails alone" (static-analysis), and review for the port speaking archive-shaped facts.
