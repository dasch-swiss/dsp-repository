# CPE

The Configurable Presentation Environment: the Access Area's second capability, a hypermedia server for project-specific presentations over the archive's data (ADR-0007). Four of its crates are here: its port onto the archive projection, `cpe-ports` in `ports/`, its domain crate, `cpe-core` in `core/`, its component library, `cpe-components` in `components/`, and its image viewer, `cpe-image-viewer` in `image-viewer/`. Its store, page assembly and routes are still in `dsp-incubator` (`cpe/`) and move in crate by crate, in dependency order (ADR-0007, 2026-10-02 and 2026-10-11 amendments; ADR-0011). CPE's store is a derived read model, rebuilt from empty from what that port serves and fed by nothing else (ADR-0008). The Access Area's own terms (DPE, CPE, DIP shape, Landing page) are in [`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Archive projection port**:
`cpe_ports::ArchiveProjection`, what CPE reads from the Access Area's archive projection. CPE declares it and `sync` implements it beside its data (ADR-0003); CPE's tests use `FakeArchiveProjection`, and every adapter's output must pass `cpe_ports::contract::violations`, which `sync` also enforces at serve time.
_Avoid_: CPE's API (it is what CPE consumes, not what it offers), the sync port (`sync` implements it; it does not own it).

**Source property**:
The project property a value was recorded under, e.g. `incunabula:hasTitle` (`dao:sourceProperty`); what `cpe_ports::PropertyIri` holds and what a project's KDL names. Two project properties that crosswalk to one standard term stay distinct under it.
_Avoid_: canonical predicate, data predicate (DAO's `dcterms:title` or project-term IRI in the data, which can merge two properties into one).

**Project snapshot**:
`cpe_ports::ProjectSnapshot`, one project's current facts, whole, as one call to the port returns them: its resources and its list nodes, and beside them its **Curation**. It carries no revision and announces no change, so every rebuild starts from empty.
_Avoid_: dump (the archive's export format), delta, page (a snapshot is never partial).

**Archive-shaped fact**:
A fact in the archive's shape: the canonical DAO model wherever its decisions have settled. Until DAO is published, `sync` serves it from an **interim format**, DAO-shaped where DAO has decided, with the deviations listed in `dao-lift`'s `FORMAT.md` in `dsp-incubator` (defined in [`../sync/CONTEXT.md`](../sync/CONTEXT.md)). That is a resource's class, label, typed values in archive order, file and membership, a list node's place in its tree, a date as calendar plus Julian Day Number and precision per bound, and what marks an **Annotation**: its motivation and targets. Values are named by their **Source property**, each value carries its UUID (a link has none), and ties in value order break by UUID. The port lifts membership (`isPartOf`, `seqnum`) out of the values and otherwise reshapes nothing. Fields DAO drops or leaves open (value order, a text's language, Julian Day Numbers, the file's asset name, the lists) say so in `cpe_ports` and are provisional. A resource carries its data ARK, which the adapter derives from the IRI until DAO carries it. What CPE makes of these facts (positional order from `seqnum`, reverse links, calendar dates, titles, IIIF URLs) is CPE's remodel, not the port's.
_Avoid_: CPE's presentation types (`cpe_core::ir::Resource`, `Property`, `Representation`, `PropertyValue`, a compound), which the port must never speak; `cpe_ports::Resource` is the archive's resource, not CPE's.

**Annotation**:
A resource the archive marks `oa:Annotation`: a Region or LinkObj, or a project subclass of either. It is an ordinary `cpe_ports::Resource` whose `annotation` field carries its `oa:motivatedBy` and `oa:hasTarget`; find annotations by that field, never by `class`, since a project may subclass `kb:Region` or `kb:LinkObj`.
_Avoid_: Region as the name for every annotation (a LinkObj is one too), a separate annotation DTO.

**Data ARK**:
`cpe_ports::DataArk`, a resource's plain data ARK (`https://ark.dasch.swiss/ark:/72163/1/<SHORTCODE>/<id+checkdigit>`), carried by every `cpe_ports::Resource`. It resolves to the DSP data and never to CPE (ADR-0007, 2026-10-02 amendment). Distinct from the project or record **Pid** (`RecordPid`) defined in [`../CONTEXT.md`](../CONTEXT.md).
_Avoid_: presentation ARK (ADR-0007's open kind), version ARK, Pid.

**Curation**:
`cpe_ports::CuratedValue`, what a project's editors authored about one resource and the archive does not record:
a resource, a key, an optional language and a text. The port serves it beside the **Archive-shaped facts**, in
`ProjectSnapshot.curation`, never as one and never inside a `cpe_ports::Resource`.
A key means nothing to the port or to `sync`: the project's KDL and hook give it its meaning.
Interim: `sync` serves curation by a decision that is still proposed (ADR-0010; DEV-7488).
_Avoid_: configuration (project-wide, declared in the project's KDL), archive fact.

## The domain crate

`cpe-core` in `core/` holds what a project's configuration means, with no framework in it: the KDL config
language and its validation (`validate`), the IR a valid configuration lowers to (`ir`, `remodel`), the
diagnostics a broken one produces (`diagnostic`), labels, language and theme, and the per-request view data
(`view_data`). It depends on no other CPE crate and names no `maud`, `axum` or `rusqlite`.
`cpe_core::ir::Resource` is CPE's presentation type, what a page shows; it is not `cpe_ports::Resource`, the
archive's resource (see **Archive-shaped fact**).

Its vocabulary and its specification are not in this repository yet. Until DEV-7465 moves them:

- the glossary of CPE's own terms (Resource, Property, compound, component, remodel) is `dsp-incubator cpe/CONTEXT.md`;
- the config-language specification is `dsp-incubator cpe/docs/CONFIG_LANGUAGE.md`. A change to the language
  lands here as code and there, in the PR that moves the incubator's pin, as the document.

The tests that tie this crate to files of the incubator are the incubator's, run against its vendored copy of
the crate. They check two things: the specification's worked example lowers to what Incunabula's `config.kdl`
lowers to, and the header budget in `validate/header_budget.rs` matches `tokens.css`, which the incubator reads
from its vendored copy of `components/style/`.

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
(ADR-0007). The crate neither embeds nor serves the stylesheets, and nothing in this repository serves them
yet: a host does, and until DEV-7465 brings CPE's host here that host is the incubator's binary, from its
vendored copy. `shell.css` refers to `dasch-fingerprint.svg` by a URL relative to itself; that file, the two
logos and `datastar.js` stay with the incubator's binary until then. No playground shows the components
here; the crate's snapshot tests guard their markup.

The glossary of the components' terms (component, compound, chrome message, register) is
`dsp-incubator cpe/CONTEXT.md` until DEV-7465, and the catalogue of components is
`dsp-incubator cpe/docs/COMPONENT_LIBRARY.md`. A comment in the crate that cites a decision record of the
incubator names it by its path, `dsp-incubator cpe/docs/adr/<file name>`: the incubator's numbers are not
this repository's.

## The image viewer

`cpe-image-viewer` in `image-viewer/` is the `<image-viewer>` web component, an IIIF deep-zoom viewer on a
vendored OpenSeadragon, with the `Viewer` Maud builder that renders the element and `asset()`, which serves
the element's files to a host. It depends on `maud` alone and on no other CPE crate.

It is interim: a cut-down copy of `dsp-incubator shared/dasch-image-viewer`, reduced to the one engine CPE
uses (ADR-0007, amendment of 2026-10-11, DEV-7401). The full viewer, with its other engines, stays in the
incubator, which chooses the final engine; the result replaces this crate. Until then a viewer fix lands here
and is applied by hand there. Its README says what was removed.

## The dependency seam

`just check` runs `.github/scripts/check-cpe-seam.sh` over every tracked `Cargo.toml` (ADR-0007, amendment
of 2026-10-11, DEV-7401). No crate outside `areas/access/cpe/` depends on a CPE crate other than `cpe-ports`,
which `sync-store` implements; no manifest under `areas/access/cpe/` depends on a Mosaic crate; and
`cpe-core`, `cpe-ports` and `cpe-image-viewer` depend on no CPE crate. The first two rules cover a new CPE
crate without an edit. A new crate that must stay a leaf has to be added to the script's `LEAVES`; one that a
crate outside the capability may depend on has to be added to its exception list. What the script does not
check is review: that no CPE crate depends on DPE's domain, store or web crates or on `sync`'s store
(ADR-0008), and `cpe-components`' own dependency list (no `axum`, no `rusqlite`).

## What the port serves

An adapter serves current, live facts only, and omits every fact not listed as in: it never errors on one and never substitutes a string form. A link or `part_of` whose target is omitted is itself omitted; a child of an omitted parent keeps its `seqnum`. An annotation target that is not a resource of the file is refused, unlike a link: the snapshot is not served.
**Curation** is the one thing served that is not a fact of the archive.

| In | Out, deliberately | Why out |
|---|---|---|
| Resources: IRI, class, label | Ontology definitions (class and property labels, cardinalities, the subproperty hierarchy) | KDL declares CPE's labels |
| The data ARK of every resource, annotations included: the plain data ARK only, never a value or version ARK; the adapter derives it from the IRI until DAO carries it | Value and version ARKs | CPE links back to the data it presents; data ARKs resolve to the DSP data and never to CPE (ADR-0007, 2026-10-02 amendment) |
| Text (plain text plus language), texts with markup included | Standoff markup | Nothing in CPE presents it yet; it is added as `Text`'s own field when a project needs it, never as an opt-in flag |
| Integer, decimal, boolean, URI; geometry (knora-base's JSON) and color, both verbatim | Geoname, time and interval values; the planned structured geolocation value | No CPE component presents them. Map display is deferred to CPE by the geolocation PRD (`dasch-specs/specs/2026-09-07-support-geographic-location-data-in-dsp/01-geolocation-value-type-PRD.md`); it comes in with the first project that shows a map |
| Date (calendar, start and end as JDN plus precision) | Islamic calendar | Accepted by DSP-API, but the corpus has Gregorian and Julian only |
| List nodes (tree, position, labels) and list values | — | — |
| Links, subject to object, `isRegionOf`, `hasLinkTo` and `isAnnotationOf` included; Regions and LinkObjs as resources with their `oa:motivatedBy` and `oa:hasTarget` | LinkValue reifications | DAO strips them (`FORMAT.md` §6) |
| Ordered membership (`isPartOf` parents and `seqnum`) | `isSequenceOf` | No project uses it yet; added when one needs it |
| One file per resource: still image (asset and dimensions), audio, moving image, document | Other file kinds (archive, text, external and vector still images), file bytes, original filename, MIME types, checksums | CPE builds IIIF URLs from the asset name; the rest is not presented |
| Per-resource curation: a key, an optional language and a text | Project-wide configuration (vocabularies with their labels and order, rights constants) and any meaning of a key | The project's KDL declares configuration (ADR-0007); only curation comes through `sync` (ADR-0010, proposed) |
| — | Rights and legal information (license, copyright holder, authorship) | Its own follow-up, needed before CPE serves ADR-0005 landing pages (ADR-0007's ARK clause) |
| — | Permissions, creation and deletion metadata | Not presented yet |
| — | Deleted resources and values, superseded value versions | Not current facts |
| — | Change announcements, a snapshot revision | ADR-0008: a full rebuild until a port carries change |
| — | Links to resources outside the project | The adapter omits them; an annotation target outside the project makes the snapshot unavailable |
