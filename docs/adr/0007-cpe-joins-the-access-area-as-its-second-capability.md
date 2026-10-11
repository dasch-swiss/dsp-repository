---
status: accepted
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
  The store is a derived read model under ADR-0008, CPE's own shape over the projection's facts, and this
  clause is its first instance. CPE declares in `cpe/ports` what it needs from the projection; `sync`,
  the Access Area's single writer of the archive projection, implements the adapter beside its data. The port speaks archive-shaped facts as
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

  Note (2026-10-11, DEV-7465): the hook is a module of the store builder, not a file of the project's
  folder, and it works on the builder's draft of the mapped resources, not on the port's types; see the
  two amendments of 2026-10-11 (DEV-7465) below.
- **A project fails alone.** Validation of a project's configuration against its store happens when the
  project is activated, not when the process boots; a project that does not validate is offline on its own
  prefix while DPE and every other project keep serving. A strict mode for development, CI and tests fails
  the whole start on the first invalid project, as the prototype's `--check` does; it is the same verb as
  DPE's `validate`, on the area binary.

  Note (2026-10-11, DEV-7465): the first half is built, the strict mode is not; see the amendment of
  2026-10-11 (DEV-7465) on activation below.
- **A project is a folder.** Everything that belongs to one project lives under one directory of the
  capability, discovered by convention: its KDL, its narrative, its assets, and its hook if it has one.
  Adding a project adds a folder and touches no shared registry.

  Note (2026-10-11, DEV-7465): a project with a hook also adds a module and a table row to the store
  builder; see the amendment of 2026-10-11 (DEV-7465) below.
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
- **New projects are prototyped in the incubator, not here, and arrive with their engine changes first.**
  The engine and the design system are expected to change with every new project until they settle, and a
  prototype must be quick, must not touch the monorepo, and must get its preview link from the incubator's
  own CI as it does today (rationale as stated by the owner, 2026-09-25). So `dsp-incubator` keeps a CPE
  project lane on a vendored, unchanged copy of this capability at a pinned commit of this repository; how
  that copy is built, patched and diffed is the incubator's decision
  (`0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`). What binds this repository: the
  changes a prototype made to the copy land here as ordinary PRs before the project folder does, and a
  project folder moves in only from an empty diff; and CPE's crates stay copyable, meaning what they
  depend on inside this repository is a small, known closure.

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
- **Prototyping new projects in the monorepo, and the incubator prototype archived outright** — rejected:
  it loses both things the incubator gives today, a prototype that touches no monorepo branch and a
  preview link for free. The mechanisms for the incubator's copy that were weighed (the published image
  with a run-time projects directory, a git subtree, a git dependency, per-crate Cargo manifests kept
  here as a verified export) are the incubator's decision and are recorded in its record
  `0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`, not here.

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
- Migrate, then delete: once the monorepo CPE serves Incunabula, the prototype's own engine is removed
  from `dsp-incubator/cpe` and the vendored copy of this capability takes its place, so no second engine
  is maintained; the prototype's ADRs stay as history.
- `ARCH-MAP.md`'s planned entry for `areas/access/cpe` gains a `sync` sibling and the boundary rules above
  (`/dune:dune-map`); `areas/access/CONTEXT.md` (moved there from DPE's directory on 2026-09-28) stops
  being DPE's vocabulary alone and splits into one file per capability once CPE's terms arrive.
- The interaction convention the two capabilities differ on today (DPE's tab fragment writes the URL, CPE's
  swaps never do) is recorded as a per-capability rule, not unified by this record.

Enforced by: review until the move lands; then Bazel `visibility` per ADR-0001 for the capability, port
and component-library seams (structure), a `Cargo.toml` grep in the style of `check-shared-paths.sh` for
the `mosaic-tiles` and cross-capability rules until then (static-analysis), the strict validation mode in
CI for "a project fails alone" (static-analysis), and review for the port speaking archive-shaped facts
and for a project folder moving in only with an empty diff against the incubator's copy.

## Amendment (2026-09-28, DEV-7397) — the composition root moved out ahead of CPE

This record's first clause makes CPE's arrival what turns `dpe-server` into a capability-internal crate
and creates `areas/access/server`. DEV-7397 extracts that composition root now, before CPE lands, to
unblock DEV-7400 and DEV-7405: both need `areas/access/server` to exist so CPE's own crates and wiring
have somewhere to land without a second, simultaneous move of DPE's crate. `dpe-server` is a library from
this point on, exposing `DpeConfig`, `Dpe` and `validate` to the new `access-server` binary; nothing in
this decision's reasoning or its shape changes, only its timing relative to CPE.

## Amendment (2026-09-29, DEV-7405) — CPE's public URL layout

The first clause makes a project's presentation "a route prefix inside CPE's router" without saying which
prefix, or how CPE's routes share the origin with DPE's. This amendment narrows it:

- **CPE lives under `/cpe/`, as DPE lives under `/dpe/`, and a project under `/cpe/<shortname>/`.** The
  segment is the project's DSP shortname (`/cpe/incunabula/`). It is resolved in one place, so it can
  become the Shortcode later without touching every route.
  Note (2026-10-11, DEV-7465): the segment is the project id, the name of the project's folder, which is
  neither the shortname nor the Shortcode; see the amendment of 2026-10-11 (DEV-7465) below.
- **CPE's router does not know its own prefix.** `areas/access/server` mounts it with `nest("/cpe", …)`,
  and CPE builds every URL it emits (links, redirects, asset references) from a configured base, never from
  a hardcoded `/cpe`. A project on its own hostname is not built, but stays possible by mounting the same
  router at a different base.
  Note (2026-10-12, DEV-7438): `access-server` nests it at `Cpe::base()` and writes no `/cpe` itself; see
  the amendment of 2026-10-12 (DEV-7438) below.
- **A project is self-contained under its prefix, assets included.** The engine's shared files are served
  at `/cpe/<shortname>/_static/…` and the project's own at `/cpe/<shortname>/_assets/…`, keeping the
  prototype's reserved `_` segments. The same engine file is then fetched once per project, which
  content-hashed names and long cache lifetimes make negligible; a single shared `/cpe/_static/` would not
  exist on a project's own hostname.
  Note (2026-10-11, DEV-7438): `/cpe/_static/…` exists too, for CPE's own not-found page; see the
  amendment of 2026-10-11 (DEV-7438) below.
- **`/` and everything outside `/cpe` stay DPE's, including its 404; CPE answers its own not-found under
  `/cpe`.** CPE's router keeps its own catch-all (a project's 404 under a known project, a plain CPE 404
  otherwise), and a nested router's catch-all wins over DPE's for paths under the prefix (axum 0.8.9,
  checked 2026-09-29). Axum's `nest` does not match `/cpe/` itself, so the composition root routes that one
  path to CPE's not-found explicitly; without it, `/cpe/` gets DPE's 404.
- **No record URL form is fixed for ARKs yet.** The ARK clause above already keeps every ARK away from CPE
  until the FAIR adapter exists; when it does, a uniform record route can redirect to each project's own
  record pages.

Deferred, not decided here: links between DPE's project page and a project's CPE presentation; browser
telemetry for CPE pages, which send no beacons today (the `shared-telemetry` collector holds one meter scope
and one path normaliser per process, so it needs a change when they do).

Under `nest`, CPE sees paths with the prefix removed, and a leading `//` after it collapses to one slash
(`/cpe//evil.example/x` arrives as `/evil.example/x`). The prototype's rule that its catch-all never
redirects, and never echoes the request path, holds unchanged. Its open-redirect tests carry over and run
both under the `/cpe` mount and at a root mount.

Enforced by: routing tests in `areas/access/server` for the prefix, `/cpe/`, and which side answers a 404
(static-analysis, landing with DEV-7438); review for URLs built from the configured base.
Note (2026-10-11, DEV-7438): URLs built from the base are tested now; see the amendment of 2026-10-11
(DEV-7438) below.

## Amendment (2026-09-29, DEV-7399) — the committed projection is interim-DAO

The clause on CPE's read-side store starts `sync` at "the committed projection of one shortcode read from
disk" without saying in which shape. This amendment fixes the shape:

- **The committed projection is interim-DAO.** `dao-lift` in `dasch-swiss/dsp-incubator`
  (`cpe/tools/dao-lift`) produces it from a VRE dump, one N-Quads file per project. It follows DAO wherever
  DAO has decided and deviates only where the port needs a fact DAO drops. Each deviation is listed with its
  reason in `dao-lift`'s `FORMAT.md`, which stays in the incubator, so the format is visibly neither DAO nor
  a commitment of this repository. `areas/access/sync/data/PROVENANCE` pins the `dao-lift` commit whose
  `FORMAT.md` the committed file follows. `sync-store` refuses the whole snapshot (`Unavailable`) when the
  file breaks a fact the port serves, and omits only what `FORMAT.md` lets a reader omit and what the port
  does not carry, so CPE never receives a partial or repaired snapshot.
- **The port follows the interim format (PR #452).** "Archive-shaped facts as the archive records them" in the
  same clause means the facts as the interim format carries them, not as knora-base records them. Values
  are named by their source property, the project property they were recorded under, which is also what a
  project's KDL names.
- **The interim format is temporary.** When DAO is published, `dao-lift` and `FORMAT.md` move to it, and
  each deviation is resolved either upstream in DAO or by a change to the port.

Enforced by: `sync-store`'s tests, for refused and omitted facts and for the committed file passing
`cpe_ports::contract::violations` (static-analysis); `sync-store` refusing a snapshot the contract
rejects (run-time); review for the port speaking the format.

## Amendment (2026-10-02) — data ARKs and presentation ARKs

The ARK clause ("No persistent identifier resolves to a CPE page until CPE's pages carry ADR-0005's
metadata") and the last bullet of the DEV-7405 amendment read as if the existing ARKs of a CPE-presented
project would be pointed at CPE once the FAIR adapter (DEV-7409) lands. They will not be (as stated by the
owner, 2026-10-02). There are two kinds of ARK, and only the second ever resolves to CPE:

- **Data ARKs identify DSP data and keep resolving to it.** The existing `ark:/72163/1/<shortcode>/<resource-id>`
  and its value ARKs name a data point, not a view of it. They resolve to the data (today the VRE, at
  `app.dasch.swiss`) and are never re-pointed at CPE, whether or not a project is presented by CPE. CPE
  pages link back to the data they present by its data ARKs (a "Show in dataset" affordance, being planned
  in `dsp-incubator`'s CPE).
- **Presentation ARKs identify a CPE presentation view and resolve to CPE.** They are a separate, new kind;
  their form and the resolver's mapping for them are not designed and stay open. The ARK clause's gate
  applies to them: no presentation ARK resolves to a CPE page until that page carries ADR-0005's metadata,
  and on such a page `cite-as` is its presentation ARK. CPE's share link may later copy it.
- **The DEV-7405 amendment's ARK bullet is narrowed accordingly.** No URL form is fixed for presentation
  ARKs yet; the uniform route it anticipates, if built, serves presentation ARKs, never data ARKs.

In effect nothing changes today: no ARK of either kind resolves to CPE.

Enforced by: review, for any change to how ARKs resolve or to what CPE emits as `cite-as`.

## Amendment (2026-10-02, DEV-7400) — CPE arrives crate by crate, shaped in the incubator

The record reads as if CPE arrives in one move: the prototype is moulded into the target anatomy on entry,
its dependency pins are "reconciled on entry", and the vendored copy replaces the prototype's engine only
once the monorepo serves Incunabula. A single move of that size cannot be reviewed: it would relocate the
engine, split it into the capability's crates, reconcile its dependencies, build the store from the port
and wire it into `areas/access/server` in one change, with the prototype no longer beside it as the
reference. So the move is split (as decided by the owner, 2026-10-02; the plan is phase 1b of
`dsp-incubator/cpe/docs/INCUNABULA_GO_LIVE.md`):

- **The engine takes this capability's crate shape in the incubator first.** Its module cycles are broken,
  page assembly receives view data instead of querying the store, the engine is split into the crates
  `areas/access/cpe` will hold (domain, components, store, page assembly), and its dependency pins are
  aligned with this workspace, all in `dsp-incubator`, each step checked against the prototype's
  unchanged HTML. "Reconciled on entry" therefore happens before entry.
- **The store built from the port, and the per-project remodelling, are built in the incubator first.**
  The incubator vendors `cpe-ports` from this repository and builds Incunabula's store from the committed
  0803 projection, compared against the prototype's `data.sql`. The remodelling clause above is unchanged;
  only where it is first implemented moves.
- **Crates move one at a time, once they have stopped changing.** A crate moves when its churn in the
  incubator has dropped, not in dependency order; after it moves, the incubator vendors it back under its
  own record (`0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`), and changes to it
  are PRs here. The store and page assembly move last, together with the
  wiring and the retirement of the prototype's binary.

  Note (2026-10-11, DEV-7401): the churn sentence is replaced by ADR-0011; see the amendment of 2026-10-11
  (DEV-7401) below.
- **The vendored copy therefore starts before Incunabula is served from here,** one crate at a time.
  "Migrate, then delete" is unchanged: the prototype's own engine is removed once the monorepo CPE serves
  Incunabula, which by then is the removal of whatever has not moved yet.

The target shape, the port, the guardrails and the done-criterion (Incunabula byte-comparable to the
prototype apart from the `/cpe` prefix) are unchanged.

Enforced by: review, for a crate moving into `areas/access/cpe` with its incubator copy re-vendored and
an empty `just vendor-diff` there.

## Amendment (2026-10-11, DEV-7464) — `cpe-core` moved before its churn dropped

The DEV-7400 amendment says: "A crate moves when its churn in the incubator has dropped, not in dependency
order". Its reason is in the incubator's go-live plan (`dsp-incubator/cpe/docs/INCUNABULA_GO_LIVE.md`,
"Moving"): "moving a still-busy crate turns every prototype change into a cross-repo change". This
amendment makes one exception to that sentence and leaves it standing:

- **`cpe-core`, CPE's domain crate, moved to `areas/access/cpe/core` while it was still changing** (as
  decided by the owner, 2026-10-11). Its churn had not dropped: 23 commits in 30 days on its paths in the
  incubator (`cpe/core` and, before the crate split of 2026-10-10, `cpe/engine/src`), measured 2026-10-11,
  12 of them in October; the latest, dsp-incubator#502 of that day, added about 5,400 lines. That showed no
  quiet period to wait for, and waiting kept the component library (DEV-7401) and the store, page assembly
  and wiring (DEV-7465) blocked: `cpe-core` depends on no other CPE crate, and both depend on it.
- **The rule stands for the crates that follow.** A later crate still moves once its churn in the incubator
  has dropped. Dropping the rule for them would be a changed decision and a new record (ADR-0006).

  Note (2026-10-11, DEV-7401): that new record now exists. ADR-0011 replaces the churn sentence, so the rule
  no longer stands for the crates that follow, and this amendment's "Enforced by" line no longer applies;
  see the amendment of 2026-10-11 (DEV-7401) below.
- **The cost the rule avoided is accepted for this crate.** From the move on, a change to the config
  language is a PR here, followed by a pin move in the incubator, which vendors the crate back.
- **The config-language specification and CPE's glossary stay in the incubator until DEV-7465.**
  `dsp-incubator/cpe/docs/CONFIG_LANGUAGE.md` and `dsp-incubator/cpe/CONTEXT.md` are neither moved nor
  copied; `areas/access/cpe/CONTEXT.md` points to them. A config-language change therefore lands in two
  steps: the code here, then the document in the incubator's pin-move PR.

  Note (2026-10-11, DEV-7465): the specification is in this repository now, and the glossary is still the
  incubator's; see the amendment of 2026-10-11 (DEV-7465) below.

Enforced by: review, for a later crate that moves before its churn in the incubator has dropped.

Note (2026-10-11, DEV-7401): replaced by ADR-0011; see the amendment of 2026-10-11 (DEV-7401) below.

## Amendment (2026-10-11, DEV-7401) — the churn rule is replaced, the image viewer's interim packaging, the stylesheets, and the seam as checked

- **The churn sentence of the amendment of 2026-10-02 (DEV-7400) is replaced by ADR-0011.** "A crate moves
  when its churn in the incubator has dropped, not in dependency order" no longer holds: a CPE crate moves
  when the next piece of work needs it here, in dependency order. The reasons and the decision are in
  ADR-0011. Every other sentence of that amendment stands, and this record is not superseded.
- **The image viewer is packaged, for now, as a cut-down copy inside the capability.** This record carries
  the viewer's packaging "as an open question, not a decision". Until the incubator has chosen the final
  engine, the answer is `cpe-image-viewer` at `areas/access/cpe/image-viewer`: a copy of the incubator's
  `shared/dasch-image-viewer`, reduced to the one engine CPE uses (OpenSeadragon in a shadow root). It is
  not in `shared/`, because it has one consumer, and it is not part of CPE's component library. The
  incubator keeps developing its viewer; what it chooses replaces this crate. A viewer fix CPE needs before
  then lands here and is applied by hand in the incubator. The question itself stays open.
- **The workspace still has no `include_dir`.** The viewer's asset files (its JavaScript, the vendored
  OpenSeadragon and its sprites) are compiled into its crate through an enumerated `include_bytes!` table.
- **No project bundle is embedded.** The retirement of "bundles embedded at build time" above is about
  project bundles and stands: the viewer's files are engine files, and no project content is compiled into
  any crate.
- **The stylesheets live with the component library, as plain files.** `cpe-components` moved to
  `areas/access/cpe/components`, and the seven stylesheets its markup is written against moved with it, to
  that crate's `style/` directory. The crate neither embeds nor serves them: the host does. Until DEV-7465
  brings CPE's host here, that host is the incubator's binary, which serves them from its vendored copy;
  nothing in this repository serves them yet.

  Note (2026-10-11, DEV-7465): CPE's host crate is here and serves them; see the amendment of 2026-10-11
  (DEV-7465) on activation below.
- **The seam is checked, and wider than the sentence above states it.** The component-library clause says
  that CPE's web crate never depends on `mosaic-tiles` and that no crate outside `areas/access/cpe/` depends
  on CPE's library. `.github/scripts/check-cpe-seam.sh` checks three rules over the tracked `Cargo.toml`
  files, for every crate of the capability:
  1. No crate outside `areas/access/cpe/` depends on a CPE crate, the port excepted. `cpe-ports` is the one
     crate another capability may depend on, because the provider of a port implements it (`sync-store`
     today).
  2. No manifest under `areas/access/cpe/` depends on a Mosaic crate (any crate of `modules/mosaic/`, not
     `mosaic-tiles` alone).
  3. `cpe-core`, `cpe-ports` and `cpe-image-viewer` depend on no CPE crate, each other included.

  `cpe-image-viewer` has no exemption for a later consumer outside the capability: such a consumer changes
  the check and this record together. The script excepts crates from rule 1 by a list of paths. DEV-7465
  adds the host-facing crates to it (`cpe-server` and, for a `Live<Port>` adapter, `cpe-store`) when
  `access-server` mounts CPE.

  Note (2026-10-12, DEV-7438): the list got `cpe-server` and not `cpe-store`; see the amendment of
  2026-10-12 below.

  Rule 3 is by a second list, `LEAVES`; a new crate that must stay a leaf is
  added to it. What the script does not check is review: that no CPE crate depends on DPE's domain, store or
  web crates or on `sync`'s store (ADR-0008), and `cpe-components`' own dependency list (no `axum`, no
  `rusqlite`).

  Note (2026-10-11, DEV-7465): the script has a fourth rule, for `sync`'s store as a dev-dependency
  (ADR-0008, amendment of 2026-10-11), and a fifth, for the CPE crates that `cpe-store` and `cpe-store-build`
  may depend on; see "Enforced by" of the amendment of 2026-10-11 (DEV-7465) below.

Enforced by: static-analysis (`check-cpe-seam.sh`, run by `just check`) until Bazel `visibility`, for the
seam; review for the rest.

## Amendment (2026-10-11, DEV-7465) — the store builder, the config-language specification and Incunabula's folder

- **`cpe-store-build` is a crate of the capability, in a role ADR-0003 does not list.** ADR-0003's crate
  anatomy names a domain crate, a store, a web crate and `ports`. The builder, at
  `areas/access/cpe/store-build`, is none of them, as `cpe-components` and `cpe-image-viewer` were not: it
  turns the snapshot the port serves, and a project's `remodel {}` block, into the rows of CPE's store. It
  is a library with no binary. It depends on `cpe-core` and `cpe-ports` and not on `cpe-store`, whose
  schema it includes as a file. Folding it into `cpe-store` was weighed and not chosen: it would merge two
  crates during a move (as decided by the owner, 2026-10-11).
- **A project's hook is a module of the builder, not a file of the project's folder.** The remodelling
  clause puts the hook "in the project's own folder", and "A project is a folder" says that adding a
  project "touches no shared registry". The builder moved as it is in the incubator: a hook is a module
  under `store-build/src/projects/` and one row of the const table `hooks::HOOKS` (as decided by the owner,
  2026-10-11). The reasons that hold here: a function the builder calls has to be linked into the builder,
  which needs one place that names it, and Rust under `projects/` would need its own rules for the
  workspace, formatting, linting and unused dependencies. The cost is that a project with a hook adds a
  module and one table row to `cpe-store-build`; a project without one is still a folder alone.
- **The config-language specification is here.** It is `docs/src/cpe/config-language.md`, in the developer
  documentation. This replaces the sentence of the amendment of 2026-10-11 (DEV-7464) that kept it in the
  incubator: a change to the language now changes the code and the document in one PR. CPE's glossary is
  still `dsp-incubator/cpe/CONTEXT.md`.
- **Incunabula's folder is `areas/access/cpe/projects/incunabula/`.** The folder's name is the project id
  (CPE's term, `areas/access/cpe/CONTEXT.md`): the project's URL segment and the key the hook table is read
  by. It is not the project's Shortcode, `0803`. The folder holds what CPE reads (`config.kdl`,
  `narrative/`, `assets/`) and the documents that describe the project's data (`SHAPE.md`, `CURATION.md`,
  `PROVENANCE.md`, `DATA_ISSUES.md`). It holds no `data.sql`: the store is built from the port
  (ADR-0008). The prototype's design documents stay in the incubator.

Enforced by: static-analysis for the worked example of the specification, which a test of
`cpe-store-build` holds equal to Incunabula's block, and for which CPE crates the builder and the store
depend on (`check-cpe-seam.sh`, rule 5: `cpe-store-build` on `cpe-core` and `cpe-ports` alone, `cpe-store` on
`cpe-core` alone; `cpe-web` and CPE's host depend on both and are not held to the rule); review for the rest.

Note (2026-10-12, DEV-7438): the host crate is held to rule 5 now, and does not depend on `cpe-store`; see
the amendment of 2026-10-12 below.

## Amendment (2026-10-11, DEV-7465) — the host crate, activation, and what of "a project fails alone" is built

- **A hook works on the mapped draft, not on the port's types.** The remodelling clause says a hook is a
  function "over the port's types". As built, the builder first applies the project's mapping to the
  snapshot, and the hook is then given a narrow draft of the mapped resources (`cpe_store_build::hook::Draft`):
  it reads and changes what the block's `hook` node declares and never sees the snapshot. The port's types
  stop at the builder's generic code.
- **`cpe-server` is CPE's host crate.** It is at `areas/access/cpe/server` and holds CPE's configuration,
  router and activation behind one type, `Cpe`, in the shape of `dpe-server`'s `Dpe`. No type of another
  CPE crate appears in its `pub` items, `cpe-ports`' types excepted, so a composition root depends on
  `cpe-server` and `cpe-ports` and on nothing else of CPE. It serves the component library's stylesheets
  and the files of `areas/access/cpe/public/` from a compiled-in table. Nothing mounts it yet.

  Note (2026-10-12, DEV-7438): `access-server` mounts it; see the amendment of 2026-10-12 below.

- **A project's store is built at activation, from SQL text.** `Cpe::activate` is given the port. For each
  project folder it lowers the `remodel {}` block, asks the port once for the snapshot of the block's
  shortcode, builds the store's rows with `cpe-store-build` and the hook registered for the folder's name,
  renders them as SQL text, and loads the project from that text as it would from a file. No store is
  committed, embedded or kept: it is built again at every start (ADR-0008). Rendering text and parsing it
  again costs a parse; in exchange the text for Incunabula is byte for byte the file the prototype embeds,
  which a test holds. A builder that writes SQLite directly is left for later.
- **What of "A project fails alone" is built.** `Cpe::new` reads nothing and cannot fail, and `activate`
  returns nothing and never panics, so CPE cannot stop or delay the area binary. A project that does not
  activate (its configuration, the projection, the build, validation, or a panic in any of them) is logged
  with its id and is offline on its own prefix: every path under it is answered 503 with `Retry-After` and
  `Cache-Control: no-store`, while the other projects are served. Until `activate` has finished, every CPE
  path is answered that way. "Not ready" is kept apart from "not there": 404 is for a path that names no
  project folder, and a missing projects directory therefore leaves every path not found, while missing
  `sync` data leaves every project at 503. Validation happens at activation, as the clause says.
  Not built: the strict mode that fails the whole start (the `validate` verb on the area binary) and the
  designed offline page. Both are DEV-7403's. Until then nothing in CI fails on an invalid project
  except `cpe-server`'s test that Incunabula activates from the committed data.

Enforced by: static-analysis for the facade (`cpe-server`'s test `no_pub_item_names_another_cpe_crate`, a
text check over the crate's two files with `pub` items), for the 503 and 404 behaviour (the tests of
`cpe-server`'s `activation` module over `FakeArchiveProjection`) and for Incunabula's store text (the same
module's test of its length and SHA-256); review for the rest, the strict mode included until DEV-7403.

## Amendment (2026-10-11, DEV-7438) — the base as built, pages without a project, links in content

The amendment of 2026-09-29 (DEV-7405) says CPE builds every URL from a configured base. This amendment
records how, and two cases that amendment did not cover:

- **The base has one source.** It is `CPE_BASE` in `cpe-server`'s configuration: empty for a mount at the
  root of the host, else a path with a leading slash and no trailing one (`/cpe`). `Cpe::base()` returns it,
  and a composition root mounts the router there and writes no prefix of its own. A value of another shape
  is refused: CPE then answers 503 and its host keeps running.
- **One module builds the URLs.** `cpe_core::url` joins the base, the project id and the two reserved
  segments, and nothing else does. A handler sees the request path without the base; the page's own path,
  both redirects, the share links and the reader's history URLs are the base and that path. The `lang` and
  `appearance` cookies have `Path=<base>` (`/` at a root mount), so they reach every project of CPE and
  nothing else on the host.
- **The layout holds at every base, the empty one included.** The shared files are at
  `<base>/<project id>/_static/…` and a project's own at `<base>/<project id>/_assets/…`. The prototype's
  `/_static/…` and `/_assets/<project id>/…` are gone, at a root mount too.
- **A page without a project loads the shared files from `<base>/_static/…`.** CPE's own not-found page
  has no project, so there is no project prefix to load them from. The same table is served at both places,
  and a project's pages use only their own. The reason for the per-project path is untouched: that page does
  not exist on a project's own hostname. The 503 page has no project either and loads no file at all: while
  it is the answer, every path under the base is answered by it, the static files included.
- **A link in a project's content that starts with one `/` gets the base in front.** Narrative Markdown
  and its invite lists name pages as `/incunabula/books`; the renderer writes `<base>/incunabula/books`.
  A destination that starts with `//`, has a scheme, is a fragment or is relative is written as it is.
  The content is not changed, and so it still names the project segment, against "resolved in one place"
  above: a project-relative link form would close that and is not built. A link in a property value (a
  rich-text value from the archive) is not narrative and is written as it is.

Enforced by: static-analysis for the URLs of the pages a crawl reaches (`cpe-server`'s `base_tests`: a crawl
of Incunabula and of the fixture projects under `/cpe` that fails on any path or public-origin URL outside
the base, with both redirects, both cookies and the refused base; `cpe-core`'s tests of `url`); review for a
URL being built through `cpe_core::url` at all. The crawl does not see a page it does not reach, nor a path
that carries the base and was joined by hand, and no check reads the source for one.

## Amendment (2026-10-12, DEV-7438) — `access-server` mounts CPE, and the seam exception is `cpe-server`

- **The area binary mounts CPE under its base.** `access-server` builds `Cpe` from `CpeConfig`, nests
  `Cpe::router()` at `Cpe::base()` inside its OTel layers, and routes the base followed by `/` to CPE's
  root, which a nested router does not match. It writes no path of CPE's. Every path under the base is
  CPE's to answer, its not-found and its 503 included; every other path is answered as with no CPE in the
  binary, DPE's `ServeDir` fallback included. CPE is nested and not merged because both routers end in a
  fallback. `/healthz` and the telemetry collector stay outside the tracing layers.
- **No base stops the binary.** A base that is a path the binary or DPE has a route for (`/healthz`,
  `/telemetry/collect`, `/dpe/projects`) is mounted: the route keeps answering, and CPE answers what no route
  does under it. Should the router's build panic with CPE at its base, `access-server` builds it without
  CPE, logs an error that names `CPE_BASE` and its value, and serves DPE alone, as without a base. No base
  does that with today's routes: the mount declares one route, `<base>/`, and the build panics only on a
  second route for that path that answers every method.
- **Without a base CPE is not mounted.** The amendment of 2026-10-11 (DEV-7438) allows an empty base, for a
  host that serves CPE at its root, and says a refused base answers 503. In `access-server` neither is
  mounted: a router cannot be nested at an empty path, and CPE merged at the root would answer DPE's paths.
  With `CPE_BASE` unset or refused the binary serves DPE alone and logs it, as a warning for the unset value
  and as CPE's own error for the refused one. The local run recipes set `CPE_BASE=/cpe`, and a deployment
  has to set it.
- **Activation runs in the background, after the listener is bound.** `access-server` constructs
  `sync-store`'s `LiveArchiveProjection` over the directory `SYNC_DATA_DIR` names (default
  `areas/access/sync/data`, for a run from the repository root) and calls `Cpe::activate` on the blocking
  pool. DPE, `/healthz` and the collector answer at every moment; CPE answers 503 until activation has
  finished. A data directory that is not there leaves each project at 503 and stops nothing.
- **The seam exception is `cpe-server`, beside the port.** The amendment of 2026-10-11 (DEV-7401) expected
  `cpe-server` "and, for a `Live<Port>` adapter, `cpe-store`" in the list of crates another crate may depend
  on. The adapter of CPE's port is `sync-store`'s, so the composition root constructs it without naming
  `cpe-store`, and the list is `cpe-ports` and `cpe-server`. The host crate is now also held to the rule
  about which CPE crates a crate may depend on: every CPE crate but `cpe-store`, which it reaches through
  `cpe-web`. This replaces "CPE's host ... not held to the rule" in the amendment of 2026-10-11 (DEV-7465).
- **A CPE request's span is the area binary's, named by CPE's route.** `cpe-server` installs no request
  span. To the host's tracing layer its router is one service, so each response of a route carries the
  route's template (`cpe_server::MatchedRoute`), and `access-server` names the span by it:
  `GET /cpe/incunabula/books/{id}`. A request that matches no route keeps the method alone, so a path never
  becomes a span name. The spans are under DPE's telemetry identity, which is the binary's; `http.route`
  stays empty on them.

Enforced by: static-analysis for the mount, the request table, the unmounted cases, a base that is a route
and a mount that panics, the held activation, the missing data directory and the span name (the tests of `access-server`'s `serve.rs`, one of them a search of
every file of the crate's `src/` for a `"/cpe` literal), and for the seam (`check-cpe-seam.sh`, rules 1 and 5;
`check-composition-root-deps.sh`); review for `serve()` itself, which no test runs: that it awaits and
unwraps nothing of CPE's.
