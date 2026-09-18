# DSP Repository — Bounded Contexts

Index of the bounded contexts in this monorepo and the rules that govern how they relate. Each context keeps its own vocabulary in its own `CONTEXT.md`; this file is an index, not a glossary — do not add a context's vocabulary here. The one exception is `## Shared`, which holds the research-metadata contract both areas speak.

`dsp-repository` is DaSCH's Research Data Repository: the **Trusted Repository**, long-term preservation and trustworthy dissemination of humanities research data, framed by the OAIS reference model (CCSDS 650.0-M-3). OAIS separates the producer who submits data, the archive that preserves it, and the consumer who reads it; the platform follows that separation with three **areas**, which are its bounded contexts, and brings the producer side under the repository roof. The decisions that bind the code are in `docs/adr/`.

## Bounded contexts

- **Deposit Area** — the producer side, brought under the repository roof: where a depositing project team creates and edits its metadata, uploads its media in preparation for archiving, and RDU reviews the result before anything is submitted to the archive. Today the metadata editor (`modules/editor/`). Target directory `areas/deposit/` (ADR-0002). → [`modules/editor/CONTEXT.md`](modules/editor/CONTEXT.md)
  _Avoid_: Ingest Area (the earlier name, see Flagged ambiguities), Producer side, self-service preservation frontend (an earlier name for the eventual single application: editor + data-model creator + data creation).
- **Archive Area (Spycherli)** — the OAIS archive: Ingest, Archival Storage and the supporting functional entities; the sealed heart of the platform. No code yet; target directory `areas/archive/` (ADR-0002). → [`areas/archive/CONTEXT.md`](areas/archive/CONTEXT.md)
  _Avoid_: Spycherly (misspelling), the Archive (ambiguous with the OAIS functional entity), Repository-Core.
- **Access Area** — the consumer side, OAIS Access: produces Dissemination Information Packages for Consumers. Today DPE (`modules/dpe/`); planned as further capabilities of the same modulith: `sync` (the single writer of the archive projection in Chischtli), `profile` (per-user settings), `media` (Service Files via Vitrinli), CPE, the SPARQL endpoint and the admin view. Target directory `areas/access/` (ADR-0002). → [`modules/dpe/CONTEXT.md`](modules/dpe/CONTEXT.md)

## Shared infrastructure (not bounded contexts)

- **Mosaic** (`modules/mosaic/`) — the design system: `mosaic-tiles`, a Maud component library, and its playground. Shared kernel of every hypermedia server here; no domain, no `CONTEXT.md`.
- **`shared-telemetry`** (`shared/telemetry/`) — the browser-beacon contract and collector shared by DPE and the editor. Mechanism, not domain: its few terms (Beacon, Signal, Web vital, Page URL normalization) are documented in `shared/README.md`.
- **`shared-fair`** (`shared/fair/`) — the FAIR exposure engine of ADR-0005: one resolved graph per published object (`ProjectGraph`, `RecordGraph`) and one writer per representation reading it, so no two representations disagree. Mechanism, not domain: its few terms (Resolved graph, Writer, Part, Resolve context) are documented in `shared/README.md`; the facts a graph carries are the contract terms below.
- **Vitrinli** — the media engine (IIIF rendering, range-served downloads, Service File derivation), sipi under its new name, maintained separately today and being rewritten from C++ to Rust. Moves into this monorepo as a root-level library (`vitrinli/`, ADR-0002), like Mosaic: no routes, no tables, no authentication of its own. A **media** capability in the Deposit Area and one in the Access Area depend on it and give it their area's rules through its traits (ADR-0003). Its `CONTEXT.md` holds the engine's own vocabulary (its roles and traits); each area's rules for media live with that area's `media` capability. → [`vitrinli/CONTEXT.md`](vitrinli/CONTEXT.md) (a seed; the code brings its own vocabulary when it moves in).
  _Avoid_: sipi (the old name, kept only for the existing codebase), IIIF server (one of its roles), asset server (absorbed into it).
- **Chischtli** — the triplestore engine (embedded RDF store with graph-granular write dispatch, exact update deltas, full-text search, optional SHACL on update), the in-house replacement for Fuseki, designed and built separately today. Moves into this monorepo as a root-level library (`chischtli/`, ADR-0002), like Vitrinli: no routes, no authentication, no opinion about which graphs exist. Its unit of ownership is the named graph, and every graph has one writing capability (ADR-0003): in the Access Area `sync` writes everything that comes from the archive and `profile` its own per-user settings, with DPE, CPE and the SPARQL endpoint reading through `sync`'s ports; in the Deposit Area the data-creation capability writes the working graphs. → [`chischtli/CONTEXT.md`](chischtli/CONTEXT.md) (a seed).
  _Avoid_: Fuseki (the system it replaces), the triplestore (ambiguous between the engine and an area's instance), dataset (Fuseki's word for a whole store).

## Shared

The research-metadata **contract**, `shared-metadata` (`shared/metadata/`), is the published language between the Deposit Area and the Access Area. Its terms are defined once, here; a context's file refers to them and adds only its own meaning.

**Project**:
A research project archived in DSP, described by one `projects/<shortcode>_<slug>.json` file whose 37 members are `ProjectRaw`.
_Avoid_: Dataset (a retired v1 concept), Collection (a different thing, see the Access Area), `Project` without a crate prefix where the view model could be meant.

**Shortcode**:
The project's short identifier, the key of every cross-area reference; four hexadecimal characters for all but five projects (`0801a`–`0801e`), which is why `is_valid_shortcode` checks shape only.
_Avoid_: project id, project code.

**Person** / **Organization**:
The two kinds of contributor entity, each its own corpus file (`persons/person-NNN.json`, `organizations/organization-NNN.json`); a project's `Attribution` names one by id string.
_Avoid_: Agent (the archive's term; in the editor `Agents` is only the store of these two), Contributor as a type (it is a role, not a kind).

**Multilingual**:
A language-tag-to-text map with an open tag set (`ar` is live), serialized alphabetically.
_Avoid_: LangString, i18n map.

**Placeholder**:
One of the two case-sensitive sentinel strings `MISSING` and `CALCULATED`, meaning "no value yet"; recognised only through `is_placeholder`, never rendered as a live value.
_Avoid_: sentinel (fine in prose, but the contract term is Placeholder), null (a placeholder is present, a null is absent).

**Temporal coverage**:
A period the project covers, as a ChronOntology reference or free text, resolved to a W3CDTF range through the period table and then the offline enrichment table; `completeness_gap` is the one rule all three enforcement points apply.

**Record**:
In the contract, an individual data record of a project as exported by DSP-API (`Record`, `RecordPid`). See Flagged ambiguities: the word means something else in every other context.

The file vocabulary below is published language across the Deposit Area, the Archive Area, the Access Area and Vitrinli. Exactly three kinds of file exist, distinguished by **purpose** in the preservation chain, not by format or location; what a server sends for one request is a response, not a fourth kind.

**Preservation File**:
The authoritative, write-once bytes of a Representation inside the archive, content-addressed by hash; owned by the Archive Area and never read by any other context.
_Avoid_: Archival Master, original, the file (ambiguous).

**Service File**:
A regenerable mezzanine sized for delivery (a pyramidal TIFF for IIIF, for example), carrying no preservation commitment, no ARK and no version. In the Access Area it is derived by the archive from Preservation Files under a derivation rule and owned by the Access Area's `media` capability, which serves it; in the Deposit Area it is derived from an Original before archiving, by Vitrinli on behalf of the Deposit Area's `media` capability, which owns it.
_Avoid_: Service Master, derivative, Access File (retired: what Vitrinli sends for one request — a tile, a rendered region, a download — is a response, not a file anyone stores or owns).

**Original**:
A file a depositor uploaded in the Deposit Area in preparation for archiving: the candidate Preservation File, held by the Deposit Area's `media` capability until the archive accepts it. Not yet preserved, and not the archive's.
_Avoid_: master, source file, Preservation File (it becomes one only on ingest).

## Relationships

- The **Deposit Area** reads the **Access Area**'s published project files (the corpus under `modules/dpe/server/data/`, baked into the editor image as `EDITOR_DATA_DIR`) and is meant to return approved records as a pull request against this repository; git is the source of truth today, the archive is designed to replace it.
- The **Archive Area** receives Submission Information Packages from the Deposit Area over the intent protocol and feeds every read-side service of the **Access Area** through NATS pointers plus immutable S3 payloads (target design; none of this exists here yet).
- Each area is a separate deployable on its own origin; DPE and the editor share a process, an image or an origin with nothing.
- One **Project** has exactly one **Shortcode**; a **Project** references zero or more **Persons** and **Organizations**; a **Project** has zero or more **Records**.

## Boundary rules

- No crate under one area depends on a crate under another; areas integrate over the wire (ADR-0002). Enforcement: review today, Bazel `visibility` once ADR-0001 lands.
- `shared-*`, `mosaic-*`, Vitrinli and Chischtli depend on no area crate; the arrow is `areas → shared, mosaic, vitrinli, chischtli`. Vitrinli and Chischtli are engines — libraries behind capabilities, never capabilities or services of their own (ADR-0003); a Chischtli graph has exactly one writing capability, and other capabilities read it through that owner's ports. Enforcement: Cargo cycle for a crate dependency (structure); `.github/scripts/check-shared-paths.sh` for a hardcoded path (static-analysis).
- Each area is one modulith: one binary, composed at `<area>/server` out of capabilities that own their tables and collaborate only through consumer-defined ports (ADR-0003). Each capability is the sole reader and writer of its durable state; a cross-capability or cross-area reference is an opaque identifier (Shortcode, ARK). Enforcement: review today, Bazel `visibility` once ADR-0001 lands.
- Share concepts, never shapes: a concept two areas need lives in `shared/`; a shape one area needs from another does not.
- Preservation storage is exclusive to the Archive Area; no other context reaches into the sealed store (`areas/archive/CONTEXT.md` → Boundary commitments, binding once code exists).
- Every user-facing surface is a server-rendered hypermedia application — Maud views, Datastar fragments, no client framework, no BFF, every write a `POST`, no page rendered differently by header (ADR-0004). Enforcement: static-analysis for the Datastar and no-JavaScript rules, review for the rest.
- Every Access-Area page a persistent identifier resolves to embeds its metadata in the served HTML, carries FAIR Signposting headers, and offers each machine-readable representation at its own URL (ADR-0005). Enforcement: docs-only until DEV-7268 lands, then static-analysis.

## Flagged ambiguities

- **"Ingest Area" → "Deposit Area"** (2026-09-16). Earlier design material called the producer side "Ingest Area"; this repository uses **Deposit Area**: "Deposit" is the producer-side vocabulary the platform already uses (depositor, Deposition, DepositAgreement), and "Ingest" is an OAIS functional entity inside the archive, so it is the wrong context's word (ADR-0002). Resolution: Deposit Area is canonical; "Ingest Area" in older material means this area.
- **"Spycherly"** is a misspelling of **Spycherli** (Swiss German, "little granary"; the working name of the Archive Area).
- **"Record"** means four things: the contract's `Record` (a DSP-API data record), DPE's `OaiRecord` (an OAI-PMH item wrapping a Project or a Record), the OAI-PMH protocol's own record, and the editor's persisted rows (`editor_core::records`, drafts / submissions / users). In the Archive Area it is not a domain term at all — the archived units are Resources and Representations. Resolution: qualify the word outside the contract (`OaiRecord`, "editor row"); the archive never adopts it.
- **"Graph"** means two things: `shared-fair`'s resolved graph (`ProjectGraph`, `RecordGraph` — one published object's facts held in memory after resolution, so that every representation reads the same ones) and Chischtli's named graph (a unit of ownership in an RDF store, with exactly one writing capability). Resolution: "resolved graph" for the exposure engine's, "named graph" for the store's; neither is ever just "the graph" in prose that could mean the other.
- **"Representation"** means two things: DPE's *machine-readable representation* (one serialization of a published object's metadata — the schema.org JSON-LD, the DataCite XML, the Dublin Core record; `shared-fair` writes one per format off a single resolved graph, and FAIR Signposting's `describedby` links point at them) and the Archive Area's `Representation` entity (a preservation-grade bundle of Preservation Files plus their metadata, ARK-identified and versioned, pinned by Resource versions). Resolution: "machine-readable representation" for the serialization, and the archive's is never qualified inside `areas/archive/`; neither is just "the representation" in prose that could mean the other.
- **"Submission"** is the editor's durable `submissions` row (at most one per project, deleted by every review outcome) and, in OAIS, the wire-format sense (Submission Information Package). Resolution: Submission unqualified means the editor row; the package is always "SIP".
- **"Project"** is `ProjectRaw` (the contract, the file as read) and `dpe_core::Project` (DPE's view model, whose `From` impls are lossy on `url` and `clusters`). Resolution: the editor's path is `ProjectRaw` → draft → `ProjectRaw` and never touches the view model; say "view model" when you mean DPE's.
- **"module" / "area" / "capability" / "service"**: a *module* is a directory under `modules/` today (a transitional word, ADR-0002 removes the directory); an *area* is a bounded context and, per ADR-0003, one deployable binary; a *capability* is one bounded unit inside an area's modulith (DPE, CPE, the editor); a *service* is a deployable, which today coincides with the area's single capability. Resolution: prefer area and capability; "service" for the binary; "module" only while `modules/` exists.
- **"platform"** (resolved 2026-09-17). The word named both the shared root and its crates, and the DaSCH Service Platform (DSP), the product family. The shared root, `modules/platform/`, and its crates were renamed to `shared/` and `shared-{role}` (ADR-0002, *Considered Options*), because Bazel's own platform vocabulary — the `@platforms` repository and the conventional `//platforms` package for target and host definitions — arrives with ADR-0001 and would have meant a third thing. Resolution: "platform" now means the DaSCH Service Platform only; the crates are "shared crates". Older material using the word for the shared root means `shared/`.
- **"Agent"**: a Person or Organization entity in the editor and the archive; never an AI coding agent in domain prose.
- **"DPE"** is the Discovery and Presentation **Environment**, not Platform (older material has both). **"CPE"** is the Configurable Presentation Environment.
