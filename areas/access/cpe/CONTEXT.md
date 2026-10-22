# CPE

The Configurable Presentation Environment: the Access Area's second capability, a hypermedia server for project-specific presentations over the archive's data (ADR-0007). Its eight crates are here: its two ports, onto the archive projection and for bytes, `cpe-ports` in `ports/`, its domain crate, `cpe-core` in `core/`, its component library, `cpe-components` in `components/`, its image viewer, `cpe-image-viewer` in `image-viewer/`, its store, `cpe-store` in `store/`, the builder that fills the store, `cpe-store-build` in `store-build/`, its page assembly, `cpe-web` in `web/`, and its host crate, `cpe-server` in `server/`. Its one project so far, Incunabula, is the folder `projects/incunabula/`, and its shared static files are in `public/`. A project's KDL, narrative, curation and assets are deposited files: CPE reads the first three at activation through a second port, for bytes, against `media`, and `media` is to serve the assets at request time (ADR-0012). A project's folder holds no code (ADR-0007). Incunabula's KDL, narrative and curation are in `media`'s stand-in directory, `areas/access/media/data/0803/` ([`../media/CONTEXT.md`](../media/CONTEXT.md)); until `media` serves assets, they are in the project folder. `access-server` mounts the host crate's router at the base. CPE's store is a derived read model, rebuilt from empty from what its two ports serve and fed by nothing else (ADR-0008); in the target both ports serve it with rights attached, it stores each row's rights and CPE filters every request on them, as the anonymous consumer until an authorisation mechanism exists (ADR-0012, ADR-0013). The Access Area's own terms (DPE, CPE, DIP shape, Landing page) are in [`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Archive projection port**:
`cpe_ports::ArchiveProjection`, what CPE reads from the Access Area's archive projection. CPE declares it and `sync` implements it beside its data (ADR-0003); CPE's tests use `FakeArchiveProjection`, and every adapter's output must pass `cpe_ports::contract::violations`, which `sync` also enforces at serve time.
_Avoid_: CPE's API (it is what CPE consumes, not what it offers), the sync port (`sync` implements it; it does not own it).

**Source property**:
The project property a value was recorded under, e.g. `incunabula:hasTitle` (`dao:sourceProperty`); what `cpe_ports::PropertyIri` holds and what a project's KDL names. Two project properties that crosswalk to one standard term stay distinct under it.
_Avoid_: canonical predicate, data predicate (DAO's `dcterms:title` or project-term IRI in the data, which can merge two properties into one).

**Project snapshot**:
`cpe_ports::ProjectSnapshot`, one project's current facts, whole, as one call to the port returns them: its resources and its list nodes. Its **Curation** is not in it. It carries no revision and announces no change, so every rebuild starts from empty.
_Avoid_: dump (the archive's export format), delta, page (a snapshot is never partial).

**Archive-shaped fact**:
A fact in the archive's shape: the canonical DAO model wherever its decisions have settled. Until DAO is published, `sync` serves it from an **interim format**, DAO-shaped where DAO has decided, with the deviations listed in `dao-lift`'s `FORMAT.md` in `dsp-incubator` (defined in [`../sync/CONTEXT.md`](../sync/CONTEXT.md)). That is a resource's class, label, typed values in archive order, file and membership, a list node's place in its tree, a date as calendar plus Julian Day Number and precision per bound, and what marks an **Annotation**: its motivation and targets. Values are named by their **Source property**, each value carries its UUID (a link has none), and ties in value order break by UUID. The port lifts membership (`isPartOf`, `seqnum`) out of the values and otherwise reshapes nothing. Fields DAO drops or leaves open (value order, a text's language, Julian Day Numbers, the file's asset name, the lists) say so in `cpe_ports` and are provisional. A resource carries its data ARK, which the adapter derives from the IRI until DAO carries it. What CPE makes of these facts (positional order from `seqnum`, reverse links, calendar dates, titles, IIIF URLs) is CPE's remodel, not the port's.
_Avoid_: CPE's presentation types (`cpe_core::ir::Resource`, `Property`, `Representation`, `PropertyValue`, a compound), which the port must never speak; `cpe_ports::Resource` is the archive's resource, not CPE's.

**Annotation**:
A resource the archive marks `oa:Annotation`: a Region or LinkObj, or a project subclass of either. It is an ordinary `cpe_ports::Resource` whose `annotation` field carries its `oa:motivatedBy` and `oa:hasTarget`; find annotations by that field, never by `class`, since a project may subclass `kb:Region` or `kb:LinkObj`.
_Avoid_: Region as the name for every annotation (a LinkObj is one too), a separate annotation DTO.

**Data ARK**:
`cpe_ports::DataArk`, a resource's plain data ARK (`https://ark.dasch.swiss/ark:/72163/1/<SHORTCODE>/<id+checkdigit>`), carried by every `cpe_ports::Resource`. It resolves to the DSP data and never to CPE (ADR-0007). Distinct from the project or record **Pid** (`RecordPid`) defined in [`../CONTEXT.md`](../CONTEXT.md).
_Avoid_: presentation ARK (ADR-0007's open kind), version ARK, Pid.

**Curation**:
What a project's editors authored about one resource and the archive does not record: a resource, a key, an
optional language and a text, one row per resource in the project's curation file, `curation.csv`. It is a
deposited file and reaches CPE through the **Bytes port** (ADR-0012). At **Activation**
`cpe_store_build::CuratedValues::parse` reads it against the project's **Project snapshot**, and refuses a row
whose resource the snapshot does not hold, a repeated row and a malformed one; the builder then takes the
parsed values. A project that deposited no curation file has none, unless its **Mapping** reads a curated
key (`Remodel::reads_curation`); then it is an **Unavailable project**. A key means nothing to the port, to
`sync` or to `media`: the project's KDL and hook give it its meaning.
_Avoid_: configuration (project-wide, declared in the project's KDL), archive fact.

**Bytes port**:
`cpe_ports::DepositedFiles`, CPE's second port: the deposited files of one project by shortcode and name,
whole or refused, with their bytes: its KDL (`config.kdl`), its curation file (`curation.csv`), its narrative
(`narrative/<name>`). In the target each file comes with its rights attached and CPE decides at request time
(ADR-0013); not yet in code, the port carries no rights. CPE declares it and `media` implements it beside its data as `LiveDepositedFiles`
(ADR-0012); CPE's tests use `FakeDepositedFiles`. It lists nothing: CPE asks for the narrative files its KDL
names. A project it does not hold is `UnknownProject`, a file it does not hold `UnknownFile`, and any other
failure `Unavailable`. It carries no fact and gives no file a meaning; an asset (an image, a download) never
comes through it, since assets are fetched at request time by URL.
Its name rule, `cpe_ports::is_file_name`, is stricter than `cpe-core`'s for a narrative file: segments of
`[A-Za-z0-9._-]` separated by `/`, none empty or starting with `.`. A name the KDL may write and the port
refuses is answered `UnknownFile`, so validation reports it as `NarrativeFileNotFound`, naming the file.
Its shortcode rule, `cpe_ports::is_shortcode`, is four upper-case hex digits; it refuses `0801a`–`0801e` on
purpose, a metadata-only split of BEOL (one DSP project, `0801`) with no Access-bucket prefix of its own.
_Avoid_: the media port (`media` implements it; it does not own it), asset port, file API.

**Project id**:
The name of a project's folder under `projects/`, `incunabula` for Incunabula. It is also the project's URL
segment and the key its **Hook** is looked up by. `cpe-core` has its rule: kebab-case, and none of the
reserved ids (`DiagnosticKind::InvalidProjectId`, `ReservedProjectId`). It is not the **Shortcode** of the
root `CONTEXT.md`, which is the archive's key for the project (`0803`) and which the folder declares in its
one-line `SHORTCODE` file; that file avoids "project id" for the Shortcode, and in CPE the term means the
folder's name only.
_Avoid_: shortcode, slug, project name.

## The domain crate

`cpe-core` in `core/` holds what a project's configuration means, with no framework in it: the KDL config
language and its validation (`validate`), the IR a valid configuration lowers to (`ir`, `remodel`), the
diagnostics a broken one produces (`diagnostic`), labels, language and theme, and the per-request view data
(`view_data`). It depends on no other CPE crate and names no `maud`, `axum` or `rusqlite`.
`cpe_core::ir::Resource` is CPE's presentation type, what a page shows; it is not `cpe_ports::Resource`, the
archive's resource (see **Archive-shaped fact**).

The config-language specification is [`docs/src/cpe/config-language.md`](../../../docs/src/cpe/config-language.md).
A change to the language changes the code and that document in one PR. `cpe-store-build`'s test
`worked_example` holds the specification's worked example of the `remodel` block equal to what Incunabula's
`config.kdl` lowers to. The glossary of CPE's own terms (Resource, Property, compound, component, remodel) is
not in this repository: it is `dsp-incubator cpe/CONTEXT.md`.

One test that ties this crate to another file is still the incubator's, run against its vendored copy: that
the header budget in `validate/header_budget.rs` matches `tokens.css` in `components/style/`.

## The component library

`cpe-components` in `components/` renders what `cpe-core`'s IR describes, as Maud functions that return
`Markup`. It holds:

- the components a page is assembled from (`component`): the editorial and narrative renderers, the resource
  list with its facet panel and search bar, the record with its media viewer and compound viewer, the
  annotation register, the page picker and the data links;
- the chrome message catalogue (`chrome`): `ChromeMsg`, the strings of the interface itself in every language
  CPE offers, as distinct from the labels a project's KDL declares;
- the per-project components (`component::project`) and their registry, `PROJECT_RENDERERS` (private to the crate): a component
  one project needs and no other, found by its `(project, name)` pair, with `cpe-core`'s
  `validate/project_components.rs` as the validation half of the same seam;
- the seven stylesheets the markup is written against, as plain files in `components/style/`: `tokens`,
  `shell`, `editorial`, `record`, `grid`, `media` and `incunabula`.

It depends on `cpe-core` and `cpe-image-viewer`, and on no `axum`, no `rusqlite` and no Mosaic crate
(ADR-0007). The crate neither embeds nor serves the stylesheets: `cpe-server` does, from a compiled-in
table. `shell.css` refers to `dasch-fingerprint.svg` by a URL relative to itself, so that file, which is in
`public/`, is served from the same directory of URLs. No playground shows the components here; the crate's
snapshot tests guard their markup.

The glossary of the components' terms (component, compound, chrome message, register) is
`dsp-incubator cpe/CONTEXT.md`, and the catalogue of components is
`dsp-incubator cpe/docs/COMPONENT_LIBRARY.md`. A comment in the crate that cites a decision record of the
incubator names it by its path, `dsp-incubator cpe/docs/adr/<file name>`: the incubator's numbers are not
this repository's.

## The image viewer

`cpe-image-viewer` in `image-viewer/` is the `<image-viewer>` web component, an IIIF deep-zoom viewer on a
vendored OpenSeadragon, with the `Viewer` Maud builder that renders the element and `asset()`, which serves
the element's files to a host. It depends on `maud` alone and on no other CPE crate.

It is interim: a cut-down copy of `dsp-incubator shared/dasch-image-viewer`, reduced to the one engine CPE
uses (ADR-0007). The full viewer, with its other engines, stays in the
incubator, which chooses the final engine; the result replaces this crate. Until then a viewer fix lands here
and is applied by hand there. Its README says what was removed.

## The store

`cpe-store` in `store/` is CPE's derived read model (ADR-0008): one in-memory SQLite database per project.
It holds the schema (`schema.sql`), the loader (`db`) and the query layer (`query`). The query layer does two
things. It implements `cpe_core::validate::facts::ProjectFacts` as `LiveProjectFacts`, which is how the
validator reads a project's data, and it assembles a request's `cpe_core::view_data::PageData`
(`query::page::page_data`) over the reads of `query::reads`. It depends on `cpe-core` and on no `maud` or
`axum`.

The crate does not fill the store. `db::ProjectStore::open` takes SQL text and executes it over the schema; what
writes that text from a project snapshot is the builder, `cpe-store-build`, called by `cpe-server` at
**Activation**. A store exists in memory only: none is committed or written to disk, and each start builds it
again. The builder does not depend on
this crate, but it includes `schema.sql` as a file: a change to the schema is a change to what the builder
writes, and the builder's tests load their output under it.

The crate owns the connection. `ProjectStore` keeps it in a `Mutex` and hands out neither the connection nor
a guard: the facts, the two load steps (the search indexes, the data ARKs) and `query::reads::ProjectReads`
lock inside the crate, short, never across the hydration of a row. A caller (`cpe-web`'s `Project`) therefore
cannot hold a lock across an `.await`. No `rusqlite` type is part of the crate's `pub` surface, the error
types included, so `cpe-web` does not depend on `rusqlite`. The `pub` surface is for CPE's other crates and
not for a host. A read runs on the thread that serves the request.

The schema's tables are CPE's presentation shape, not the archive's.

## The store builder

`cpe-store-build` in `store-build/` is the remodelling of ADR-0007: it turns a **Project snapshot**, the
project's **Curation** and its mapping into the rows of the store. It is a library with one entry,
`build(&ProjectSnapshot, &CuratedValues, &Remodel, Option<&Hook>) -> Result<Store, BuildErrors>`, and it is
pure: it reads no file, no clock and no environment; `CuratedValues::parse` reads the curation file's bytes,
which its caller fetched. `sql::to_sql` renders the built `Store` as the SQL text `cpe-store` opens.

**Mapping**:
The `remodel {}` block of a project's `config.kdl`, lowered by `cpe-core` to `cpe_core::remodel::Remodel`: which
archive classes and properties become which of CPE's, how a resource gets its id, the list vocabularies, the
shortcode, the curated keys. The builder's `mapping` and `generic` modules apply it and name no project.
_Avoid_: schema (the store's tables), configuration (the whole `config.kdl`).

**Hook**:
What one project needs and its mapping cannot say, as code: a module under `store-build/src/projects/` and
one row of the const table `hooks::HOOKS`, looked up by the **Project id** (`hooks::resolve`). The
block's `hook` node declares what the hook may read and change, and `build` refuses a hook without a
declaration, a declaration without a hook, and any read or change beyond it. A hook works on a narrow
`hook::Draft` of the mapped resources and never sees the snapshot. Incunabula has the only one. A project
with a hook therefore adds a module and a table row to this crate, and its names to two hand-kept lists of
the crate's `tests/it/hygiene.rs`; the crate's `README.md`, "Adding a project", has every step. A project
without one adds no registry row: a folder with its `SHORTCODE`, and its deposited files (ADR-0007,
ADR-0012).
_Avoid_: script, plugin (a hook is compiled in and typed), registry entry in the project's folder.

The crate depends on `cpe-core` and `cpe-ports`, and not on `cpe-store`: it includes `store/schema.sql` as text. Its tests over Incunabula read the committed
0803 snapshot through `sync-store`'s adapter, which is a dev-dependency and never a dependency (ADR-0008),
and Incunabula's `config.kdl` and `curation.csv` from `media`'s stand-in directory as files. The rule table
of what the builder does to each value is in its `README.md`.

## The page assembly

`cpe-web` in `web/` assembles pages. It holds no routes and names no `axum`: a host calls it and turns what it
returns into responses.

**Loaded project**:
`cpe_web::project::Project`: one project's validated configuration, its store, its narrative and the maps
derived from them at load (class routes, media properties, data ARKs), immutable afterwards.
`Project::load` takes the KDL source, the store's SQL text and the narrative files as text; it opens the
store, validates the configuration against it and returns either a project or every diagnostic. It reads no
file itself. A project that does not validate is not loaded.
_Avoid_: site (the `site {}` node of the KDL), tenant.

`page` renders from a loaded project's data: a whole page (`render_page`), the compound viewer's fragment
(`render_compound_fragment`), the browse breadcrumb, the **Overview** (`render_site_index`), and the two
not-found pages: the project's
not-found page (an unknown path under a loaded project, in that project's chrome) and CPE's own not-found
page (a path under no loaded project, in bare chrome). It composes `cpe-components`' renderers and writes
the document head and the chrome around them. `share_link` composes the canonical URL of a view, so that two requests for the same view give the
same link.

Every URL the crate writes is built by `cpe_core::url` from the **Base** its caller hands it.
Its markup is pinned by 30 snapshots in `web/src/snapshots/`, and `web/tests/projects/` holds 234 fixture
projects, one per validation rule or page shape; `tests/validation.rs` loads each through `Project::load`.
Some fixtures and snapshots carry the names of the incubator's other projects as test data; those projects
are not in this repository.

## The static files

`public/` holds the files every CPE page refers to and no project owns: the two DaSCH logos and
`dasch-fingerprint.svg`, which `components/style/shell.css` refers to by a relative URL. `public/vendor/`
holds the vendored `datastar.js` with a `README.md` that records its checksum, which `just verify-checksums`
recomputes. `cpe-server` embeds these files and the seven stylesheets and serves them under each project's
`_static/` and, for the **Overview** and CPE's own not-found page, under CPE's own; a
test there fails when a file of the three directories has no row in its table. `cpe-web`'s tests read
`datastar.js` and four of the stylesheets to hold the markup's attributes, breakpoints and class names
against them.

## The host crate

`cpe-server` in `server/` is what a composition root sees of CPE: `CpeConfig` and `Cpe`. It holds the routes,
the request's language and appearance preferences, the discovery of project folders, the table of static
files and the inventory of the projects' assets. No type of another CPE crate is in its `pub` items,
`cpe-ports`' excepted; a test of the crate holds that. It installs no tracing layer and binds no port: its
host does. A host's tracing layer sees the router as one service, so every response of a route carries
the route's template in its extensions (`MatchedRoute`, the base in front), and the host names its request
span by it. `access-server` is that host: it nests the router at the base and activates CPE over
`sync-store`'s and `media-store`'s adapters.

`CpeConfig` is read from `cpe.toml` and `CPE_*`: the projects directory (`CPE_PROJECTS_DIR`), the public
origin (`CPE_PUBLIC_ORIGIN`) and the base (`CPE_BASE`). It names no data directory: the deposited files and the
snapshots come through the two ports, whose adapters the host constructs over its own directories
(`MEDIA_DATA_DIR`, `SYNC_DATA_DIR`). Reading it cannot fail. A value that cannot be used
is kept as a refusal, and `Cpe`'s router then answers 503 on every path.

**Base**:
The path CPE is mounted at: empty at the root of the host, else a path with a leading slash and no trailing
one (`/cpe`). `Cpe::base()` is its one source: the host mounts `Cpe::router()` there, and `cpe_core::url`
writes it in front of every link, redirect and cookie path. The router's own routes do not hold it. A
project's pages are at `<base>/<project id>/…`, the shared static files at `<base>/<project id>/_static/…`
(and at `<base>/_static/…` for the **Overview** and CPE's own not-found page; the 503 page loads no file), a
project's own files at `<base>/<project id>/_assets/…`. A link in a narrative that starts with one `/` gets
the base in front.
_Avoid_: prefix, mount path

**Overview**:
The page at the **Base**, with and without a trailing slash: one teaser per loaded project (title,
description, eyebrow, cover image and credit from the project's `site {}` node), each a link to the
project's start page, in CPE's bare chrome and one of the four built-in languages. It is the route at the
root of `Cpe::router()`, rendered by `cpe-web`'s `page::render_site_index`. An **Unavailable project** is
not on it, and with no project loaded it says so and is still answered 200. The code and the config
language's documentation call it the site index, the prototype's name (`render_site_index`,
`ChromeMsg::SiteIndex*`).
_Avoid_: home, landing page (a project's `/` page is its start page), index (DPE's project list). Not the
`overview` tab of DPE's project page (`areas/access/CONTEXT.md`).

**Activation**:
`Cpe::activate(&dyn ArchiveProjection, &dyn DepositedFiles)`: the one step that loads the projects. For each
project folder it reads the shortcode of its `SHORTCODE` and asks the **Bytes port** for that project's
`config.kdl`; refuses the project when the **Mapping** declares another
shortcode; asks for each narrative file the KDL names, the base name and its variant in each site language,
and takes an `UnknownFile` as absent; asks for `curation.csv`; asks the archive projection port once for the
**Project snapshot**; reads the **Curation** against it; builds the store with `cpe-store-build` and the
project's **Hook**; reads the assets from the folder; and loads the project (`cpe-web`'s `Project::load`), which validates the configuration
against the store. It is blocking, returns nothing and does not panic; a host calls it once, off the async
runtime, after its listener is bound. `Cpe::router` may be taken and mounted before: it answers 503 until
activation has finished and the projects' routes from then on.
_Avoid_: boot, start-up (the process is already serving), warm (DPE's verb, which fills a cache; activation
builds what is served). One project's step inside it is its load.

**Unavailable project**:
A project folder that did not activate: another folder declares its shortcode, a port does not hold the
project or cannot be read, a deposited file is not UTF-8 or a required one is not there (its `config.kdl`; its
`curation.csv` when the **Mapping** reads a curated key), its configuration does not parse or validate (a
narrative file it names and the port does not hold included), the KDL declares another shortcode than
`SHORTCODE`, the curation file or the build refuses the data, or any of these panics. It is
logged with its id, and every path under its prefix is answered 503 with `Retry-After` and
`Cache-Control: no-store`; the other projects are served ("a project fails alone", ADR-0007), and the **Overview** does not list it. 404 is for a path that names no project
folder. A folder is a project when its
name passes the **Project id** rule and it holds a regular `SHORTCODE` file whose trimmed text passes
`cpe_ports::is_shortcode`. A folder that fails either is not a project, so its paths are 404; it is logged,
unless it holds no `SHORTCODE` at all.
ADR-0007 says such a project is "offline on its own prefix"; the offline page it designs is DEV-7403's.
_Avoid_: missing project (that is the 404).

Nothing in a request reaches the file system. Discovery lists the projects directory without following a
symbolic link, the assets are read into memory at activation, and a request's project segment and path are
looked up as strings.

`Cpe::from_sql_folders` is legacy, hidden from the documentation: it loads project folders that hold a
`config.kdl` and a committed `data.sql`, and reads the configuration and the flat `narrative/` from the folder,
not through a port. It is for the incubator's host and for tests (`server/tests/fixtures/`), and ends with
DEV-7498.

## The project folders

A project's folder under `projects/` is named by its **Project id** and holds no code: the project's hook is
a `HOOKS` row and its components are `PROJECT_RENDERERS` rows (ADR-0007). Its KDL, narrative, curation and
assets are deposited files, the first three read through the **Bytes port**, which `media` serves from
`areas/access/media/data/<shortcode>/` (ADR-0012). `projects/incunabula/` holds `SHORTCODE` (`0803`), the
files under `assets/`, and four documents about the project's data (`SHAPE.md`, `CURATION.md`,
`PROVENANCE.md`, `DATA_ISSUES.md`); its hook is a module of the builder. Those four were written in the
incubator and still describe its pipeline in places: its
committed `data.sql`, its `vendor/` directory, its `just cpe` recipes and other files of the incubator (its
design documents, its mockup, its scripts, `dao-lift`) are not in this repository. A citation that starts
with `dsp-incubator`, as `dsp-incubator cpe/docs/…` does in these documents, in
`docs/src/cpe/config-language.md` and in `store-build/README.md`, names a file of that repository and not of
this one.

The `assets/` are an interim. `cpe-server` reads them from `CPE_PROJECTS_DIR` at **Activation** and serves them
under `_assets/` until `media` serves assets at request time; then they leave the folder (ADR-0012).

## The dependency seam

`just check` runs `.github/scripts/check-cpe-seam.sh` over every tracked `Cargo.toml` (ADR-0007). No crate
outside `areas/access/cpe/` depends on a CPE crate other than `cpe-ports`, which `sync-store` and
`media-store` implement, and `cpe-server`, which `access-server` mounts; no manifest under
`areas/access/cpe/` depends on a Mosaic crate;
`cpe-core`, `cpe-ports` and `cpe-image-viewer` depend on no CPE crate; no manifest under
`areas/access/cpe/` names a crate of `areas/access/sync/` or `areas/access/media/` outside a
dev-dependencies table (ADR-0008, ADR-0012); and `cpe-store` depends on no CPE crate but `cpe-core`, `cpe-store-build` on none
but `cpe-core` and `cpe-ports`, `cpe-server` on every CPE crate but `cpe-store` (the script's `CPE_EDGES`;
ADR-0007). A
rename (`package = "…"`) of a CPE, Mosaic, `sync` or `media` crate in the workspace root is reported where it is
declared, whichever crate inherits it. The
first, second and fourth rules cover a new crate without an edit. A new crate that must stay a leaf has to be
added to the script's `LEAVES`, and one whose CPE dependencies are to be held to its `CPE_EDGES`; one that a
crate outside the capability may depend on has to be added to its exception list. What the script does not
check is review: that no CPE crate depends on DPE's domain, store or web crates, which CPE crates a crate
outside `LEAVES` and `CPE_EDGES` depends on, `cpe-components`' own
dependency list (no `axum`, no `rusqlite`), and `cpe-store`'s (no `maud`, no
`axum`).

## What the port serves

An adapter serves current, live facts only, and omits every fact not listed as in: it never errors on one and never substitutes a string form. A link or `part_of` whose target is omitted is itself omitted; a child of an omitted parent keeps its `seqnum`. An annotation target that is not a resource of the file is refused, unlike a link: the snapshot is not served.
This is the **Archive projection port**; the **Bytes port** serves the deposited files, **Curation** among
them (ADR-0012).

| In | Out, deliberately | Why out |
|---|---|---|
| Resources: IRI, class, label | Ontology definitions (class and property labels, cardinalities, the subproperty hierarchy) | KDL declares CPE's labels |
| The data ARK of every resource, annotations included: the plain data ARK only, never a value or version ARK; the adapter derives it from the IRI until DAO carries it | Value and version ARKs | CPE links back to the data it presents; data ARKs resolve to the DSP data and never to CPE (ADR-0007) |
| Text (plain text plus language), texts with markup included | Standoff markup | Nothing in CPE presents it yet; it is added as `Text`'s own field when a project needs it, never as an opt-in flag |
| Integer, decimal, boolean, URI; geometry (knora-base's JSON) and color, both verbatim | Geoname, time and interval values; the planned structured geolocation value | No CPE component presents them. Map display is deferred to CPE by the geolocation PRD (`dasch-specs/specs/2026-09-07-support-geographic-location-data-in-dsp/01-geolocation-value-type-PRD.md`); it comes in with the first project that shows a map |
| Date (calendar, start and end as JDN plus precision) | Islamic calendar | Accepted by DSP-API, but the corpus has Gregorian and Julian only |
| List nodes (tree, position, labels) and list values | — | — |
| Links, subject to object, `isRegionOf`, `hasLinkTo` and `isAnnotationOf` included; Regions and LinkObjs as resources with their `oa:motivatedBy` and `oa:hasTarget` | LinkValue reifications | DAO strips them (`FORMAT.md` §6) |
| Ordered membership (`isPartOf` parents and `seqnum`) | `isSequenceOf` | No project uses it yet; added when one needs it |
| One file per resource: still image (asset and dimensions), audio, moving image, document | Other file kinds (archive, text, external and vector still images), file bytes, original filename, MIME types, checksums | CPE builds IIIF URLs from the asset name; the rest is not presented |
| — | Per-resource curation, project-wide configuration (vocabularies with their labels and order, rights constants) | Configuration and curation are deposited files that come through the **Bytes port** against `media` (ADR-0012) |
| — | Rights and legal information (license, copyright holder, authorship) | Its own follow-up, needed before CPE serves ADR-0005 landing pages (ADR-0007's ARK clause) |
| — | Permissions, creation and deletion metadata | Not presented yet; in the target the port serves every resource with its rights attached (ADR-0012, ADR-0013) |
| — | Deleted resources and values, superseded value versions | Not current facts |
| — | Change announcements, a snapshot revision | ADR-0008: a full rebuild until a port carries change |
| — | Links to resources outside the project | The adapter omits them; an annotation target outside the project makes the snapshot unavailable |
