# Access Area

The consumer side of the Trusted Repository, OAIS Access: the services that produce Dissemination Information Packages for Consumers. Today one service, the Discovery and Presentation Environment, DPE (`dpe-core`, `dpe-api-oai`, `dpe-web`, `dpe-server`), which serves published project metadata as HTML, as JSON and over OAI-PMH. Designed but not built as further capabilities of this area's modulith: `sync` (the single writer of the archive projection in Chischtli, which DPE will read through its ports), `profile` (per-user settings in its own graphs), `media` (archive-made Service Files via the Vitrinli library), CPE, the SPARQL endpoint and the admin view — see the root `CONTEXT.md`, `chischtli/CONTEXT.md` and `vitrinli/CONTEXT.md`. Contract terms — Project, Shortcode, Person, Organization, Multilingual, Placeholder, Temporal coverage, Record — are defined once in the root [`CONTEXT.md`](../../CONTEXT.md) `## Shared` and only used here.

## Language

### Services

**DPE**:
The Discovery and Presentation Environment: the public, unauthenticated, read-only hypermedia server for discovering and reading project metadata.
_Avoid_: Discovery and Presentation Platform, the repository (DPE is one window onto it), the frontend.

**CPE**:
The Configurable Presentation Environment: the planned hypermedia server for project-specific presentations over the same data, configured per project. Not built.
_Avoid_: custom frontend, project website.

**DIP shape**:
The form a Dissemination Information Package takes for one kind of Consumer request — rendered HTML, JSON, OAI-PMH XML, IIIF tiles, a byte download; each Access-Area service produces one or more of them.

**Landing page**:
The page a persistent identifier (an ARK) resolves to — today `/dpe/projects/{shortcode}` — and therefore the page a FAIR assessor or harvester reads: it embeds its metadata in the served HTML (schema.org JSON-LD, Dublin Core `<meta>`), carries FAIR Signposting `Link` headers, and links each machine-readable representation at its own URL (ADR-0005).
_Avoid_: detail page, project page (fine informally, but the obligations attach to being the resolution target, not to the content).

**Machine-readable representation**:
A standards-shaped rendering of a Landing page's metadata at its own URL beside the page — JSON-LD, DataCite JSON, Turtle if it earns its place — advertised by `describedby` and answering with `rel="describes"`; the only negotiation is a `303` from the Landing page on `Accept` (ADR-0005).
_Avoid_: API response (the JSON API under `/dpe/api/v2` is DPE's own shape, not a standard's), content negotiation (the page itself never renders differently).

### What DPE serves

**View model**:
`dpe_core::Project`, DPE's rendering-oriented projection of a contract Project; its conversions from and to `ProjectRaw` are lossy on `url` and `clusters`, which is why no other area may use it.
_Avoid_: calling it "the Project" without qualification (see the root Flagged ambiguities).

**Corpus**:
The committed published data under `modules/dpe/server/data/`: `projects/`, `persons/`, `organizations/`, `clusters/`, `records/`, plus the two lookup tables (`chronontology-periods.json`, `temporal-coverage-enrichment.json`). DPE owns it; the editor reads an image-baked copy.
_Avoid_: database (there is none), the data directory (that is the deployment knob `DPE_DATA_DIR`, not the concept).

**Cluster**:
A named grouping of projects with its own membership list and description, defined in `clusters/*.json` and exposed as the OAI set `cluster:{id}`.
_Avoid_: collection (a different, flatter grouping), category, theme.

**Collection**:
A grouping a project belongs to by carrying its id (`collection_ids`, resolved to `CollectionRef`); unlike a Cluster it owns no member list.
_Avoid_: dataset (the retired v1 word), cluster.

**Cover image**:
The optional `<shortcode>.webp` under `public/assets/images/`, whose filename is the lookup key, matched case-sensitively; absent, the page renders a placeholder and emits no `<img>`.
_Avoid_: thumbnail, hero image.

**Contributor**:
A Person or Organization named by a project's `Attribution`, resolved through `ContributorLookup`; `is_organization_id` tells the two kinds apart.
_Avoid_: agent, author (one role among several).

**Pid**:
The ARK persistent identifier of a project or record (`RecordPid`, `ARK_PATH_PREFIX`), distinct from the internal `id` and from the Shortcode.
_Avoid_: DOI, permalink, URL (a Pid resolves to one).

### Presentation and protocol

**Tab**:
One of the three views of a project page — `overview`, `publications`, `contributors` (`VALID_TABS`) — rendered as the `#project-tabs` morph root by the same function for the full page and the SSE fragment.

**Fragment**:
A Datastar SSE response carrying a server-rendered piece of a page (`PatchElements`, optionally `ExecuteScript`), served from a deeper route than the page it patches (`/projects/{id}/tab/{tab}`).
_Avoid_: partial (a Maud helper function is a partial; a fragment is what goes over the wire), component.

**Page** (pagination):
`dpe_core::models::Page`, the paging state of a project list.
_Avoid_: confusing with a rendered HTML page in `dpe-web`.

**OAI set**:
An OAI-PMH selective-harvesting group, either `project:{shortcode}` or `cluster:{id}`.

**Metadata prefix**:
The OAI-PMH output format of a record: `oai_dc` or `oai_datacite`.

**Resumption token**:
The OAI-PMH continuation handle for a paged list response.

**Record dump**:
One `records/<shortcode>-records.json` file of contract Records fetched from DSP-API for a project (three today).

## Relationships

- **DPE** renders one **View model** per contract **Project** and serves it in three **DIP shapes**: HTML (pages and **Fragments**), JSON (`/dpe/api/v2`), OAI-PMH (`/dpe/oai`).
- A **Project** belongs to zero or more **Clusters** and zero or more **Collections**; a **Cluster** lists its members, a **Collection** is referenced from the project.
- A **Project** has zero or one **Cover image**, keyed by its Shortcode.
- A **Project** names zero or more **Contributors**; each resolves to exactly one Person or Organization.
- A **Project** and a **Cluster** each define one **OAI set**; a **Record dump** holds the Records of exactly one **Project**.
- A project page has exactly three **Tabs**; each tab switch is one **Fragment**.

## Example dialogue

> **Dev:** "The editor needs the **Cluster** memberships. Can it use DPE's **View model**?"
> **Domain expert:** "No. The **View model** drops `clusters` on the way back to `ProjectRaw`, and the editor must write every member it does not manage unchanged. The editor reads `ProjectRaw` from the **Corpus** snapshot and never touches `dpe_core::Project`."

> **Dev:** "A new project's **Cover image** is not showing."
> **Domain expert:** "Check the filename against the Shortcode character for character — `081b.webp` does not serve `081B` — and remember the scan runs once per process, so a file added while the server runs is invisible until restart."

> **Dev:** "Is a **Tab** switch a separate page?"
> **Domain expert:** "It is a **Fragment**: the server returns the whole `#project-tabs` region — tab bar and panel — and pushes the bookmarkable URL. Without JavaScript the same link loads the full page with `?tab=`; both paths render through one function, so they cannot drift."

## Flagged ambiguities

- **"Record"**: the contract's `Record`, DPE's `OaiRecord` (an OAI-PMH item wrapping a Project or a Record), the OAI-PMH protocol's record, and the `records/` **Record dump** directory. Resolution: qualify every use outside the contract; see the root `CONTEXT.md`.
- **"Project"**: `ProjectRaw` vs the **View model** vs the JSON file vs the OAI set `project:{shortcode}`. Resolution: "View model" for `dpe_core::Project`; "project file" for the JSON; "project set" for OAI.
- **"Page"**: pagination state vs a rendered HTML page. Resolution: say "pagination" for the former.
- **"Cluster" vs "Collection"**: both group projects; the difference is who owns the membership. Resolution: as defined above; never use one for the other.
- **"Placeholder"** is also what a missing **Cover image** renders as. Resolution: the contract term is the `MISSING` / `CALCULATED` sentinel; say "placeholder image" for the visual.
