# DaTEI joins the Access Area as its third capability

DaTEI, a viewer for TEI-based digital editions, exists as a prototype in `dsp-incubator/da-tei`: a configurator
GUI that ingests TEI, ODD and CSS into a SQLite intermediate representation and lets a person configure how an
edition renders, and a read-only presenter that serves committed copies of that database. It is wanted here as
the presentation of edition projects that need a custom one, in place of a CPE page: first the Sephardic Bible
edition of *Mapping the Scriptures in Western Sephardic Literature* (086A), then the Gottfried Keller edition
(eHKKA, 083C). DaTEI is close to what CPE does, tailored to digital editions rather than generic projects, and it
is meant to become fully configuration-based, so that archived files plus a configuration are turned into what
the presenter serves. The vocabulary is in `areas/access/datei/CONTEXT.md`. The decision, phrased so a violation
is describable in code:

- **DaTEI is the third capability of the Access Area's modulith (ADR-0003), at `areas/access/datei/`**, a
  sibling of DPE and CPE, never a project kind inside CPE. Normally a project has a DaTEI or a CPE presentation;
  nothing forbids both.
- **Only the presenter and the rebuild move here.** The configurator is an authoring surface and belongs on the
  Deposit side; where it lives there is not decided, and it stays in the incubator. The TOML configuration
  document is the contract between the two.
- **DaTEI's store is a derived read model (ADR-0008):** rebuilt from empty at project activation from the files
  its port serves, the configuration among them, never committed, never repaired in place. The incubator's
  committed snapshots are not carried over.
- **DaTEI declares one port, for the bytes of a project's archived files, and `media` implements it.** File
  bytes are what `media` serves, and `media` is the Access bucket's one consumer (ADR-0003). `sync` stays the
  writer of the archive projection and serves DaTEI nothing; DaTEI declares no port against `sync`.
- **Every input of an edition is deposited and comes through the port.** The Configuration, the Edition files,
  the ODD, the Project CSS, the Driving files and the Tables of contents are deposited, whoever authored them,
  and reach DaTEI through its port against `media` (ADR-0012). Nothing under `areas/access/datei/` is one of
  them. A DaTEI project is discovered as a CPE project is (ADR-0007), and a
  project folder, if DaTEI has one, holds only the documents that describe the project's data.
- **`media` starts at its minimum, as `sync` does (ADR-0007):** a hand-fed stand-in for the Access bucket
  (ADR-0012), the files of one shortcode read from disk under `areas/access/media/`, laid out as the bucket will
  be and behind the same port the bucket will stand behind; a `Fake<Port>` in DaTEI's tests is the second
  adapter. The files are plain git, revisited if one project exceeds 100 MB. A file committed there records its
  provenance with it.
- **DaTEI's read model stores rights per row, and the Presenter filters.** The port serves each file with its
  rights, the Rebuild stores them with what it derives, and the Presenter filters every request, as the
  anonymous consumer until an authorisation mechanism exists (ADR-0012, ADR-0013). The Edition files are
  build-time inputs that the Rebuild parses, not assets fetched at request time; the rights stored with what
  they become are what make that consistent.
- **A project fails alone**, as for CPE (ADR-0007).

Not decided here, and left to DaTEI's owner: the URL layout under the area's origin (CPE's is ADR-0007's), and
whether DaTEI keeps its own CSS and component library, as CPE does, or adopts Mosaic.

## Considered Options

- **DaTEI as a capability of the Access Area's modulith (chosen).**
- **DaTEI as a project kind inside CPE** — rejected: either CPE's engine grows a TEI branch in its shared roots,
  or DaTEI's engine is bolted onto CPE's project convention; both are the branches-in-shared-roots shape that
  isolated files exist to avoid.
- **The SQLite snapshot as the deployed artifact, as the incubator's showcase does** — rejected: a read model
  with a second feed, hand-curated and repaired in place, which ADR-0008 rules out.
- **`sync` as the byte provider** — rejected: file bytes would flow through a capability built for the archive
  projection's RDF facts, and `sync` would become a second thing for every reader that needs a file.
- **The committed files under `areas/access/server/`** — rejected: durable state at the composition root, which
  ADR-0003 forbids; the files sit with the capability that serves them.

## Consequences

- When the Access bucket exists, `media`'s on-disk adapter is replaced and its directory holds no data.
- The incubator's `ingest` is DaTEI's **rebuild** here; the word "ingest" belongs to the Archive Area, and the
  identifier is renamed on entry. The incubator's `<cpe-{name}>` custom-element prefix names a sibling capability
  here and is renamed on entry; the new prefix is not decided.

Not yet in code: `areas/access/datei/` holds only `CONTEXT.md`; DaTEI's presenter and rebuild are in
`dsp-incubator/da-tei`, with no port, no rebuild against it and no test against a `Fake<Port>`, and 086A's TEI
is in no repository DaSCH controls; committing it under `media`, with its provenance, is the first step of the
move (not scheduled).

Enforced by: review. The target is Bazel `visibility` per ADR-0001 for the capability and port seams
(**structure**), and one test per read model that rebuilds it from empty against a `Fake<Port>` and asserts what
the presenter serves is unchanged (**static-analysis**).
