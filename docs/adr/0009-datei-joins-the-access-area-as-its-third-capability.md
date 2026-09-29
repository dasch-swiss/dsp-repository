---
status: accepted
date: 2026-09-29
---

# DaTEI joins the Access Area as its third capability

DaTEI, a viewer for TEI-based digital editions, exists as a prototype in `dsp-incubator/da-tei`: a configurator
GUI that ingests TEI, ODD and CSS into a SQLite intermediate representation and lets a person configure how an
edition renders, and a read-only presenter that serves committed copies of that database. It is wanted here as
the presentation of edition projects that need a custom one, in place of a CPE page, starting with the Sephardic
Bible edition of *Mapping the Scriptures in Western Sephardic Literature* (086A) and then the Gottfried Keller
edition (eHKKA, 083C). DaTEI is very similar to what CPE is meant to do, tailored to digital editions rather than
generic projects, and it is meant to become fully configuration-based, so that archived files plus a
configuration can be turned into what the presenter serves (rationale as stated by the owner, 2026-09-29). The
vocabulary is in `areas/access/datei/CONTEXT.md`. The decision, phrased so a violation is describable in code:

- **DaTEI is the third capability of the Access Area's modulith (ADR-0003), at `areas/access/datei/`**, a
  sibling of DPE and CPE, never a project kind inside CPE. Normally a project has a DaTEI or a CPE presentation;
  nothing forbids both.
- **Only the presenter and the rebuild move here.** The configurator is an authoring surface and belongs on the
  Deposit side; it stays in the incubator until that is settled. The TOML configuration document is the contract
  between the two.
- **DaTEI's store is a derived read model (ADR-0008):** rebuilt from empty at project activation from the files
  its port serves plus the project's configuration, never committed, never repaired in place. The incubator's
  committed snapshots are retired.
- **DaTEI declares one port, for the bytes of a project's archived files, and `media` implements it.** File
  bytes are what `media` serves; `sync` stays the writer of the archive projection and serves DaTEI nothing.
  Whether `media` may consume the Access bucket beside `sync`'s consumption of the projection stream is
  DEV-7442; this record proceeds on the reading that it may, and is amended if the call goes the other way.
- **`media` starts at its minimum, as `sync` does (ADR-0007):** the committed files of one shortcode read from
  disk under `areas/access/media/`, behind the same port the Access bucket will later stand behind; a
  `Fake<Port>` in DaTEI's tests is the second adapter. The files are plain git, revisited if one project
  exceeds 100 MB.
- **A project fails alone, and a project is a folder**, as for CPE (ADR-0007).

Deferred, not decided here, and left to DaTEI's owner: the URL layout under the area's origin (ADR-0007's
amendment fixes CPE's; DaTEI's is open), and whether DaTEI keeps its own CSS and component library, as CPE does,
or adopts Mosaic. Two further questions stay open in `areas/access/datei/CONTEXT.md`: which files go through the
archive and which are authored beside the code, and how the configuration reaches DaTEI once the configurator
lives on the Deposit side.

## Considered Options

- **DaTEI as a capability of the Access Area's modulith (chosen).**
- **DaTEI as a project kind inside CPE** — rejected: either CPE's engine grows a TEI branch in its shared roots,
  or DaTEI's engine is bolted onto CPE's project-folder convention; both are the branches-in-shared-roots shape
  that isolated files exist to avoid.
- **The SQLite snapshot as the deployed artifact, as the incubator's showcase does** — rejected: a read model
  with a second feed, hand-curated and repaired in place, which ADR-0008 rules out.
- **`sync` as the byte provider** — rejected: file bytes would flow through a capability built for the archive
  projection's RDF facts, and `sync` would become a second thing for every reader that needs a file.
- **The committed files under `areas/access/server/`** — rejected: durable state at the composition root,
  which ADR-0003 forbids; the files sit with the capability that will serve them.

## Consequences

- `areas/access/media/` exists ahead of the Access bucket, as `cpe-ports` did ahead of CPE's engine: a
  planned capability holding the on-disk adapter and the committed files of the first project. When the Access
  bucket arrives the adapter is replaced and the directory emptied of data.
- The incubator's `ingest` is DaTEI's **rebuild** here; the word "ingest" belongs to the Archive Area, and the
  identifier is renamed on entry. The incubator's `<cpe-{name}>` custom-element prefix names a sibling
  capability here and is renamed on entry; the new prefix is not decided.
- `ARCH-MAP.md` gains planned entries for `areas/access/datei` and `areas/access/media` (`/dune:dune-map`).
- The 086A edition's TEI is not in any repository DaSCH controls today; committing it under `media` is the
  first step of the move, and its provenance is recorded with it.

Enforced by: review until the crates exist; then Bazel `visibility` per ADR-0001 for the capability and port
seams (structure), and one test per read model that rebuilds it from empty against a `Fake<Port>` and asserts
what the presenter serves is unchanged (static-analysis).
