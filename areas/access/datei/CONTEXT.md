# DaTEI

The Access Area's third capability: a hypermedia server presenting TEI-based digital editions, one project per route prefix, used instead of a CPE presentation where an edition project wants a custom one. Normally the two are exclusive per project; nothing forbids both. It moves in from `dsp-incubator/da-tei`, and only the Presenter and the Rebuild move: the Configurator is a Deposit-Area concern (see Flagged ambiguities). Its store is a derived read model under ADR-0008, rebuilt from empty from the archived files its port serves plus the project's Configuration, and fed by nothing else. No code here yet (ADR-0009); this file records the vocabulary and the boundary commitments the code will have to honour. The first projects in are the Sephardic Bible edition of *Mapping the Scriptures in Western Sephardic Literature* (086A) and then the Gottfried Keller edition (eHKKA, 083C); the URL layout and the component library are open, left to DaTEI's owner. The Access Area's own terms (DPE, CPE, DIP shape, Landing page) are in [`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) and the file vocabulary (Preservation File, Service File) in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`. DaTEI's presentation vocabulary (view, display unit, chunk, window, witness, band) is defined in the incubator's `da-tei/docs/architecture/glossary.md` and moves here with the code.

## Language

### The two halves

**Presenter**:
The read-only public server, DaTEI's only surface in this repository; `tei-serve` in the incubator. It renders views over the Read model and never writes it.
_Avoid_: the viewer (the rendering module both incubator binaries share), the admin, da-tei (the incubator's Configurator binary and crate).

**Configurator**:
The authoring GUI that presets a project's Configuration from its files, lets a person edit it, and writes it out as a TOML document. Not a capability of the Access Area: it belongs on the Deposit side, where things are authored.
_Avoid_: admin, the editor (the metadata editor is a different thing).

### What a project is made of

**Configuration**:
One TOML document per project, the sole authority on how its edition renders; the Presenter reads it and never writes it. Neither the ODD nor the Project CSS may override it.
_Avoid_: YAML, KDL (CPE's language), settings, "the user's configuration".

**Edition file**:
A TEI file the project handed over and the archive preserves; DaTEI reads its bytes through its port and never from a file under its own directory.
_Avoid_: witness (in DaTEI a witness is one column of a synoptic axis, declared by `<witness xml:id>`; a file may hold several, or one may span files), media file, source, asset (the incubator's word for TEI, ODD and CSS uploads together).

**ODD** / **Project CSS**:
The project's TEI customisation and its own stylesheet, handed over beside the Edition files and read only to preset the Configuration; never authoritative on rendering.
_Avoid_: schema (Chischtli's Shape validation is also one), the styles (Applied CSS is what the browser gets).

**Driving file**:
A TEI artefact declaring which nodes of the Edition files align across a synoptic view's columns, loaded verbatim and never derived; asserted editorial fact, not operator preference.
_Avoid_: mapping, config, synoptic definition.

**Table of contents**:
A TEI artefact declaring how a project's units are navigated: a tree of entries whose leaves name a window of a view. Editorial pages are planned as an extension of it.
_Avoid_: navigation file, index (a Full-text index is Chischtli's).

**Editorial page** (planned):
A page of the edition's own prose, an introduction or an about page, declared through a Table of contents.
_Avoid_: narrative (CPE's Markdown), landing page (ADR-0005's term; no persistent identifier resolves to DaTEI yet).

### How it is served

**Rebuild**:
Building a project's Read model from empty: read the Edition files, ODD, Project CSS, Driving files and Tables of contents through the port, apply the Configuration, recompute derived state. Runs when a project is activated and is the only writer of the Read model. In the incubator it is `ingest` plus `apply-config` plus the derived recompute, minus the uploaded snapshot.
_Avoid_: ingest (the OAIS functional entity inside the Archive Area), transformer, import, restore (the incubator's snapshot-swap path).

**Read model**:
The project's SQLite database (`.tei-ir.sqlite` in the incubator): ingested, configuration and derived tables plus the loaded artefacts, in the sense of ADR-0008. Disposable, never committed, never repaired in place.
_Avoid_: IR (fine inside the code; as a concept name it says nothing about ownership), snapshot, showcase bundle (the incubator's committed copies, retired here).

## What the port serves

The files of one project, with their bytes: Edition files, ODD, Project CSS, Driving files, Tables of contents. An adapter serves current, archived files only and omits what the archive does not hold. DaTEI needs none of the archive-shaped facts CPE's port serves and declares no port against `sync`. The provider is the Access Area's `media` capability, which reads Service Files from the Access bucket; a TEI file's Service File is the file itself. Whether `media` may consume the Access bucket beside `sync`'s consumption of the projection stream is DEV-7442.

## Relationships

- One DaTEI project presents exactly one contract **Project**, keyed by its **Shortcode**, under its own route prefix.
- A project has one **Configuration**, one or more **Edition files**, zero or one **ODD**, zero or more **Project CSS** files, zero or more **Driving files** (one per synoptic view) and zero or more **Tables of contents** (one per view is possible).
- A **Rebuild** produces exactly one **Read model** per project from those; the **Presenter** reads it and nothing else.
- DaTEI declares one port; `media` implements it (pending DEV-7442).

## Boundary commitments

- The Read model is fed by the port and the Configuration only. Nothing under DaTEI's directory is an Edition file, an ODD, a Project CSS, a Driving file or a Table of contents. Target enforcement: one test that rebuilds from empty against a `Fake<Port>` and asserts what the Presenter serves is unchanged (ADR-0008); review until then.
- The Presenter never writes the Read model; the Rebuild is its only writer.
- DaTEI reads nothing from `sync` and never opens Chischtli.
- A project fails alone, as for CPE (ADR-0007): a project whose Rebuild fails is offline on its own prefix while every other project keeps serving.

## Example dialogue

> **Dev:** "The showcase bundle commits each project's SQLite. Do we commit it here too?"
> **Domain expert:** "No. Here it is a **Read model**: the **Rebuild** produces it at activation from what the port serves plus the **Configuration**, and losing it loses nothing. A committed copy would be a second feed, which ADR-0008 rules out."

> **Dev:** "A depositor wants to change how their edition is chunked. Where do they do that?"
> **Domain expert:** "In the **Configurator**, which is not here. It writes the **Configuration** as TOML; how that TOML reaches DaTEI is the open question below. The **Presenter** only ever reads."

## Flagged ambiguities

- **Archived data versus authored content** (2026-09-29, provisional, open for debate). What goes through the archive and what is authored beside the code is decided by who handed the file over. Driving files and Tables of contents default to archived, because the incubator says a project generates them with its own scripts, but that is unconfirmed; a synoptic-view definition or an editorial page authored by DaSCH may later be deposited, in which case the Configuration must reference it as an archived file rather than assume it is local.
- **Where the Configuration is authored, and how it reaches DaTEI** (2026-09-29, tentative). The Configurator lives on the Deposit side, with the metadata editor. If the TOML it writes is then deposited, it arrives through the port as archived data and DaTEI's project folder holds nothing; if DaSCH authors it, it is authored content in the project folder, like CPE's KDL. Open; the Rebuild is the same either way.
- **"ingest"** is the incubator's word for TEI to IR and its `src/ir/ingest` module; the Archive Area owns the word. Resolution: **Rebuild** in all prose here; the identifier is renamed on entry.
- **"asset"** is the incubator's word for its uploads and its runtime directory (`assets/<id>/`); this repository already avoids it (see `vitrinli/CONTEXT.md`). Resolution: not used here; say Edition file, ODD, Project CSS, or "public assets" for static web files.
- **"project"** in the incubator is an id-keyed directory (`assets/<id>`, a server-derived slug); here a DaTEI project is the contract Project by Shortcode. Resolution: say "DaTEI project" only for the presented unit; the identifier is the Shortcode.
- **`<cpe-{name}>`**: the incubator renders every TEI element as a custom element with that prefix, which here names the sibling capability. Resolution: renamed on entry; the new prefix is not decided.
