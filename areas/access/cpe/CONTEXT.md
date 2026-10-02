# CPE

The Configurable Presentation Environment: the Access Area's second capability, a hypermedia server for project-specific presentations over the archive's data (ADR-0007). So far only its port onto the archive projection exists, the crate `cpe-ports` in `ports/`; its engine, store and routes are not built. CPE's store is a derived read model, rebuilt from empty from what that port serves and fed by nothing else (ADR-0008). The Access Area's own terms (DPE, CPE, DIP shape, Landing page) are in [`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Archive projection port**:
`cpe_ports::ArchiveProjection`, what CPE reads from the Access Area's archive projection. CPE declares it and `sync` implements it beside its data (ADR-0003); CPE's tests use `FakeArchiveProjection`, and every adapter's output must pass `cpe_ports::contract::violations`.
_Avoid_: CPE's API (it is what CPE consumes, not what it offers), the sync port (`sync` implements it; it does not own it).

**Source property**:
The project property a value was recorded under, e.g. `incunabula:hasTitle` (`dao:sourceProperty`); what `cpe_ports::PropertyIri` holds and what a project's KDL names. Two project properties that crosswalk to one standard term stay distinct under it.
_Avoid_: canonical predicate, data predicate (DAO's `dcterms:title` or project-term IRI in the data, which can merge two properties into one).

**Project snapshot**:
`cpe_ports::ProjectSnapshot`, one project's current facts, whole, as one call to the port returns them: its resources and its list nodes. It carries no revision and announces no change, so every rebuild starts from empty.
_Avoid_: dump (the archive's export format), delta, page (a snapshot is never partial).

**Archive-shaped fact**:
A fact in the archive's shape: the canonical DAO model wherever its decisions have settled. Until DAO is published, `sync` serves it from an **interim format**, DAO-shaped where DAO has decided, with the deviations listed in `dao-lift`'s `FORMAT.md` in `dsp-incubator` (defined in `areas/access/sync/CONTEXT.md` once `sync` exists; see `docs/specs/2026-09-29-minimal-sync-capability/`). That is a resource's class, label, typed values in archive order, file and membership, a list node's place in its tree, a date as calendar plus Julian Day Number and precision per bound. Values are named by their **Source property**, each value carries its UUID (a link has none), and ties in value order break by UUID. The port lifts membership (`isPartOf`, `seqnum`) out of the values and otherwise reshapes nothing. Fields DAO drops or leaves open (value order, a text's language, Julian Day Numbers, the file's asset name, the lists) say so in `cpe_ports` and are provisional. What CPE makes of these facts (positional order from `seqnum`, reverse links, calendar dates, titles, IIIF URLs) is CPE's remodel, not the port's.
_Avoid_: the CPE engine's presentation types (its `Resource`, `Property`, `Representation`, compound, property value), which the port must never speak; `cpe_ports::Resource` is the archive's resource, not CPE's.

## What the port serves

An adapter serves current, live facts only, and omits every fact not listed as in: it never errors on one and never substitutes a string form. A link or `part_of` whose target is omitted is itself omitted; a child of an omitted parent keeps its `seqnum`.

| In | Out, deliberately | Why out |
|---|---|---|
| Resources: IRI, class, label | Ontology definitions (class and property labels, cardinalities, the subproperty hierarchy) | KDL declares CPE's labels |
| Text (plain text plus language), texts with markup included | Standoff markup | Nothing in CPE presents it yet; it is added as `Text`'s own field when a project needs it, never as an opt-in flag |
| Integer, decimal, boolean, URI | Geometry, color, geoname, time and interval values; the planned structured geolocation value | No CPE component presents them. Map display is deferred to CPE by the geolocation PRD (`dasch-specs/specs/2026-09-07-support-geographic-location-data-in-dsp/01-geolocation-value-type-PRD.md`); it comes in with the first project that shows a map |
| Date (calendar, start and end as JDN plus precision) | Islamic calendar | Accepted by DSP-API, but the corpus has Gregorian and Julian only |
| List nodes (tree, position, labels) and list values | — | — |
| Links, subject to object | Regions, LinkObjs, annotations (`isRegionOf`, `isAnnotationOf`); LinkValue reifications | Region storage is an open engine gap |
| Ordered membership (`isPartOf` parents and `seqnum`) | `isSequenceOf` | No project uses it yet; added when one needs it |
| One file per resource: still image (asset and dimensions), audio, moving image, document | Other file kinds (archive, text, external and vector still images), file bytes, original filename, MIME types, checksums | CPE builds IIIF URLs from the asset name; the rest is not presented |
| — | Rights and legal information (license, copyright holder, authorship) | Its own follow-up, needed before CPE serves ADR-0005 landing pages (ADR-0007's ARK clause) |
| — | ARKs, permissions, creation and deletion metadata | Not presented yet. A data ARK is added when CPE links back to the data it presents; data ARKs never resolve to CPE (ADR-0007, 2026-10-02 amendment) |
| — | Deleted resources and values, superseded value versions | Not current facts |
| — | Change announcements, a snapshot revision | ADR-0008: a full rebuild until a port carries change |
| — | Links to resources outside the project | The adapter omits them |
