# DaTEI

The Access Area's third capability: a hypermedia server presenting TEI-based digital editions, one project per route prefix, used instead of a CPE presentation where an edition project wants a custom one. Normally the two are exclusive per project; nothing forbids both. It moves in from `dsp-incubator/da-tei`, and only the Presenter and the Rebuild move: the Configurator is a Deposit-Area concern (see Flagged ambiguities). Its store is a derived read model under ADR-0008, rebuilt from empty from the files its port serves, the project's Configuration among them, and fed by nothing else. Its crates are `datei-ports`, `datei-core`, `datei-store`, `datei-store-build`, `datei-web` and `datei-server`, with the roles and edges ADR-0009 records. `access-server` mounts it at `DATEI_BASE` (`/datei` in the image): a project's pages are under `/datei/<shortcode>`, its static files and fonts under `/datei/_static/`, and there is no overview at `/datei/`. DaTEI keeps its own CSS and does not adopt Mosaic. The first projects in are the Gottfried Keller edition (eHKKA, 083C) and then the Sephardic Bible edition of *Mapping the Scriptures in Western Sephardic Literature* (086A). Not yet in code: DaTEI's crates, with its engine and presenter still in `dsp-incubator/da-tei` (DEV-7549); `media`'s adapter of its port, the mount, activation in the background, Project state, the 503 per project and the Read model on disk (DEV-7574); `/datei` and `DATEI_READ_MODEL_DIR` in the image and the deployments (DEV-7935); keeping a Read model across a restart (DEV-7938); the VRE bridge (P-DEV-301). The Access Area's own terms (DPE, CPE, DIP shape, Landing page) are in [`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) and the file vocabulary (Preservation File, Service File) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`. DaTEI's presentation vocabulary (view, display unit, chunk, window, witness, band) is defined in the incubator's `da-tei/docs/architecture/glossary.md` and moves here with the code.

## Language

### The two halves

**Presenter**:
The read-only public server, DaTEI's only surface in this repository: `datei-server` with its views in `datei-web`; `tei-serve` in the incubator. It renders views over the Read model and never writes it.
_Avoid_: the viewer (the rendering module both incubator binaries share), the admin, da-tei (the incubator's Configurator binary and crate).

**Configurator**:
The authoring GUI that presets a project's Configuration from its files, lets a person edit it, and writes it out as a TOML document. Not a capability of the Access Area: it belongs on the Deposit side, where things are authored. It reads one project through a **File source** over its own project directory, and has no listing of projects.
_Avoid_: admin, the editor (the metadata editor is a different thing).

### What a project is made of

**Configuration**:
One TOML document per project, `datei.toml`, the sole authority on how its edition renders; deposited like the Edition files and read through the port (ADR-0012). It is a `ConfigDoc` in the format the Configurator's `backup` writes. DaTEI reads only the current format, a deviation ADR-0009 records. Its presence, with the Ready token, makes `media`'s port list the project, and so makes it a DaTEI project. The Presenter reads it and never writes it. Neither the ODD nor the Project CSS may override it.
_Avoid_: YAML, KDL (CPE's language), settings, "the user's configuration".

**Edition file**:
A TEI file the project handed over and the archive preserves; DaTEI reads its bytes through its port and never from a file under its own directory.
_Avoid_: witness (in DaTEI a witness is one column of a file-driven view's axis, declared by `<witness xml:id>`; a file may hold several, or one may span files), media file, source, asset (the incubator's word for TEI, ODD and CSS uploads together).

**ODD** / **Project CSS**:
The project's TEI customisation and its own stylesheet, handed over beside the Edition files and read to preset the Configuration; the ODD also stamps derived marker classes onto the loaded nodes, which the Configuration's styles may target. Never authoritative on rendering.
_Avoid_: schema (Chischtli's Shape validation is also one), the styles (Applied CSS is what the browser gets).

**Driving file**:
A TEI artefact declaring the units of a file-driven view and which nodes of the Edition files align across its columns, loaded verbatim and never derived; asserted editorial fact, not operator preference. Deposited, whoever authored it, and read through the port (ADR-0012).
_Avoid_: mapping (the **File mapping**), config.

**Table of contents**:
A TEI artefact declaring how a project's units are navigated: a tree of entries whose leaves name a window of a view. Deposited and read through the port (ADR-0012). Editorial pages are planned as an extension of it.
_Avoid_: navigation file, index (a Full-text index is Chischtli's).

**Editorial page** (planned):
A page of the edition's own prose, an introduction or an about page, declared through a Table of contents.
_Avoid_: narrative (CPE's Markdown), landing page (ADR-0005's term; no persistent identifier resolves to DaTEI yet).

### How a project's files are named

**Logical path**:
A file's path in the edition's own directory tree, with spaces, umlauts and folders as the edition has them. It is DaTEI's identity for a file: cross-references, ODD includes and durable ids use it, and it is the key of the File manifest and the File mapping, compared exactly. It is relative, uses `/` separators and has no empty, `.` or `..` segment, no leading `/`, no drive letter, no `\` and no NUL.
_Avoid_: file name (ambiguous, see Flagged ambiguities), stored name.

**Stored name**:
The name under which `media` holds a file in the project's directory, `<shortcode>/<stored name>`: for a file the VRE bridge wrote, the VRE's internal name, flat. Where there is no File manifest, the Stored name is the Logical path itself (the incubator, test fixtures). DaTEI refuses a Stored name that is empty or has an empty, `.` or `..` segment, a leading `/`, a `\` or a NUL. The port serves files by Stored name only, and `media` holds Stored names to CPE's strict name rule, so a project without a manifest is readable through `media` only if its Logical paths pass that rule.
_Avoid_: original file name (the name the file had when uploaded to the VRE, which DaTEI never uses), logical path.

**File layout**:
`file-layout.xml`, read by its fixed name: every file of the edition by Logical path and role (edition, odd, css, driving-file, toc). Its SHA-256 values are advisory: a mismatch is a warning naming the file, never a refusal. Made with the Configurator. `file-layout.xml` and `file-manifest.json` are reserved: no project file may be stored under either name, and the Configurator refuses such a file at upload.
_Avoid_: manifest (the **File manifest**), index.

**File manifest**:
`file-manifest.json`, read by its fixed name and written by the VRE bridge: for each Logical path, either the Stored name of the downloaded file or the IIIF reference of an image, with the IRI of the VRE resource it came from. DaTEI finds each File layout entry's Stored name through it; a layout entry the manifest does not hold, or holds as an image, is an error naming its Logical path, and a manifest entry the layout does not list is ignored. Without one, Stored names are the Logical paths (the incubator, test fixtures). DaTEI reads only the format and version it knows (`file-manifest`, version 1) and refuses others.
_Avoid_: layout, mapping (the **File mapping**, which the bridge reads and DaTEI does not).

**File mapping**:
The document, made by hand with a DaTEI page and shipped in the VRE bridge's image, that maps each Logical path to the IRI of the VRE resource holding the file (`file-mapping`, version 1). It is the contract between a DaTEI page and the bridge, which resolves each IRI and writes the File manifest from it; DaTEI never reads it. An IRI survives replacing the file on the same resource; how an author finds the IRIs is theirs.
_Avoid_: manifest, Driving file.

**Ready token**:
The file the VRE bridge writes into a project's directory after all of the project's data, under a fixed name, holding the as-of time. A project without the token is not complete, and the port does not list the project; `media`'s adapter checks for it, and DaTEI does not read it. Not decided here: the token's file name, settled with the bridge and recorded with `media`'s adapter (DEV-7574).
_Avoid_: ready, marker (DaTEI's own `<shortcode>.rebuilding`, below), lock.

### How it is served

**DaTEI's port** (`DateiFiles`, in `datei-ports`):
Two operations, keyed like CPE's `DepositedFiles`: `projects()`, the shortcodes whose directory holds both the Ready token and `datei.toml`, and `file(shortcode, name)`, a file's bytes by Stored name, whole or refused, with *not found* distinct from *unavailable*. It lists no files. `media` implements it.
_Avoid_: file port (the incubator's retired `FilePort`, which served bytes by content hash), the media port.

**File source** (`FileSource`):
The narrower trait in `datei-ports` the Rebuild reads one project through: `file(name)` by Stored name. In this repository a wrapper binds the port to one Shortcode; in the incubator the Configurator's directory adapter implements it for one project root. `ProjectFiles`, in `datei-core`, reads the File layout and File manifest through it and resolves each Logical path to its Stored name, so the Rebuild asks for files by Logical path.
_Avoid_: port (the File source has no projects), file system.

**Rebuild**:
Building a project's Read model from empty, in five steps: load the Edition files (with the ODD and Project CSS) through the File source, apply the Configuration, compute derived state, load the Driving files and Tables of contents, build the search index and the fields. Runs when a project is activated and is the only writer of the Read model, in `datei-store-build`.
_Avoid_: ingest (the OAIS functional entity inside the Archive Area), transformer, import, restore (the incubator's snapshot-swap path).

**Read model**:
The project's SQLite database, `<shortcode>.sqlite` in the directory `DATEI_READ_MODEL_DIR` names: loaded, configuration and derived tables plus the loaded artefacts, in the sense of ADR-0008. Built under another name and renamed into place when finished; disposable, never committed, never shipped in the image, never repaired in place. Rebuilt at every start.
_Avoid_: IR (fine inside the code; as a concept name it says nothing about ownership), snapshot, showcase bundle (the incubator's committed copies, retired here).

**Project state**:
What DaTEI knows of a listed project, one of three: **pending**, until its Read model is built; **ready**, when the model and what the Presenter loads with it (views, search, index pages, finders) are swapped in; **failed**, when its Rebuild or that loading failed, or its marker (`<shortcode>.rebuilding`) shows two interrupted builds in a row. A pending or failed project answers 503 with `Retry-After`; before DaTEI has asked its port for projects, every DaTEI route does. Projects are built one at a time, in the background, after `access-server` has bound its port.
_Avoid_: loading, offline, disabled.

## What the port serves

The files of one project, by Stored name: the Configuration, File layout, File manifest, Edition files, ODD, Project CSS, Driving files and Tables of contents; and the list of projects that hold a Configuration and a Ready token. DaTEI needs none of the archive-shaped facts CPE's port serves and declares no port against `sync`. The provider is the Access Area's `media` capability, the Access bucket's only consumer; a TEI file's or a TOML document's Service File is the file itself (ADR-0003, ADR-0012). Until the bucket exists, `media` serves the stand-in directory `MEDIA_DATA_DIR`, into which the VRE bridge (P-DEV-301) writes a project's text files (TEI, ODD, CSS) from the VRE, the Configuration, File layout and File manifest beside them, and the Ready token last; the bridge keeps a project's files from changing while a reader reads them. Images stay in the VRE and are served as IIIF by its Sipi. DaTEI never reads that directory itself, and knows nothing of the bridge. The port serves no rights: DaTEI's Read model stores none and the Presenter filters on none until the port carries the VRE's permissions, which ADR-0009 accepts because the bridge downloads only what the VRE serves publicly. Not yet in code: `datei-ports` and `media`'s adapter of it (DEV-7549, DEV-7574).

## Relationships

- One DaTEI project presents exactly one contract **Project**, keyed by its **Shortcode**, under its own route prefix.
- A project has one **Configuration**, one **File layout**, zero or one **File manifest**, one or more **Edition files**, zero or one **ODD**, zero or more **Project CSS** files, zero or more **Driving files** (one per file-driven view) and zero or more **Tables of contents** (one per view is possible); all of them deposited.
- Every file of a project has one **Logical path** and one **Stored name**; the **File manifest** joins them, and the **File mapping** is what the bridge builds the manifest from.
- A **Rebuild** produces exactly one **Read model** per project from those, through a **File source**; the **Presenter** reads it and nothing else.
- Each listed project has one **Project state**.
- DaTEI declares one port; `media` implements it (ADR-0012).

## Boundary commitments

- The Read model is fed by the port only, the Configuration included. Nothing under DaTEI's directory is a Configuration, an Edition file, an ODD, a Project CSS, a Driving file or a Table of contents, and DaTEI has no project folders (ADR-0009). Target enforcement: one test that rebuilds from empty against a `Fake<Port>` and asserts what the Presenter serves is unchanged (ADR-0008); review until then.
- The Presenter never writes the Read model. The Rebuild, in `datei-store-build`, is its only writer, apart from the step in `datei-store` that finishes a built model (checkpoint, journal mode, close), which `datei-server` calls after the Rebuild and which adds no rows.
- `datei-web` and `datei-core` name no database crate; `datei-ports`, `datei-core` and `datei-store-build` do no filesystem work (ADR-0009).
- DaTEI reads nothing from `sync` and never opens Chischtli.
- A project fails alone, as for CPE (ADR-0007): a project whose Rebuild fails answers 503 on its own prefix while every other project keeps serving.
- The Presenter honours a **Hidden project** (`../CONTEXT.md`) with the same four obligations as CPE, using the set of Shortcodes the composition root hands it from `ACCESS_HIDDEN_PROJECTS`; it reads no variable of its own for this. The root hands the set upper case; the Presenter upper-cases it again, as CPE does, so a host that does not still hides the project.

## Example dialogue

> **Dev:** "The showcase bundle commits each project's SQLite. Do we commit it here too?"
> **Domain expert:** "No. Here it is a **Read model**: the **Rebuild** produces it at activation from what the port serves plus the **Configuration**, and losing it loses nothing. A committed copy would be a second feed, which ADR-0008 rules out."

> **Dev:** "A depositor wants to change how their edition is chunked. Where do they do that?"
> **Domain expert:** "In the **Configurator**, which is not here. It writes the **Configuration** as TOML, which is deposited and reaches DaTEI through the port like every other file of the edition (ADR-0012). The **Presenter** only ever reads."

> **Dev:** "Someone re-uploaded a TEI file in the VRE, and it has a new internal name. Does the **File layout** need regenerating?"
> **Domain expert:** "No. The layout names the file by its **Logical path**, and the **File mapping** names the VRE resource by IRI, which did not change. The bridge writes the new **Stored name** into the **File manifest**, and DaTEI picks it up at the next restart."

## Flagged ambiguities

- **Archived data versus authored content** (2026-09-29; resolved 2026-10-13, ADR-0012). Everything the Rebuild consumes is deposited, whoever authored it: Driving files, Tables of contents and the Configuration included. Nothing the Rebuild reads is authored beside the code.
- **Where the Configuration is authored, and how it reaches DaTEI** (2026-09-29; resolved 2026-10-13, ADR-0012). The Configurator lives on the Deposit side; the TOML it writes is deposited and arrives through the port. Until the archive exists, it ships in the VRE bridge's image with the File layout and File mapping, and the bridge writes it into `media`'s stand-in directory.
- **"ready"**: the **Ready token** says the bridge has written a project completely; the Project state *ready* says its Read model is built and serving. Resolution: say Ready token for the first, and "ready" only for the Project state.
- **"file name"** can mean a file's Logical path, its Stored name, or its original name in the VRE. Resolution: say Logical path or Stored name; DaTEI never uses original names, which the VRE keeps as base names only and which are not unique.
- **"ingest"**: the Archive Area owns the word. Resolution: **Rebuild** in all prose here; the incubator's code says Rebuild, and "load" for turning TEI into rows (resolved there 2026-10).
- **"asset"** is the incubator's word for its uploads and its runtime directory (`assets/<id>/`); this repository already avoids it (see `vitrinli/CONTEXT.md`). Resolution: not used here; say Edition file, ODD, Project CSS, or "public assets" for static web files.
- **"project"** in the incubator is an id-keyed directory (`assets/<id>`, a server-derived slug); here a DaTEI project is the contract Project by Shortcode. Resolution: say "DaTEI project" only for the presented unit; the identifier is the Shortcode.
- **`<cpe-{name}>`**: the incubator rendered every TEI element as a custom element with that prefix, which here names the sibling capability. Resolution: the incubator renders `<tei-{name}>` and names its own classes `datei-{group}` (resolved there 2026-10).
