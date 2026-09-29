---
title: "Minimal sync capability: an interim-DAO 0803 projection behind CPE's port"
date: 2026-09-29
author: "Balduin Landolt"
status: reviewed
linear: DEV-7399
linear_project: CPE establish production path
repositories:
  - dsp-repository
  - dsp-incubator
---

# Minimal sync capability: an interim-DAO 0803 projection behind CPE's port

## Normative sources

In precedence order:

1. **`cpe-ports`** (`areas/access/cpe/ports/`) governs the boundary DTOs, their ordering and
   omission rules, and `contract::violations`, once amended by R3 below; where the crate as it
   stands in PR #452 differs from R3, R3 wins. This PRD does not otherwise restate the port.
2. **The DAO decisions** in `dasch-swiss/dsp-repository-design`, `spycherli/decisions-active.md`
   (d.48, d.51, d.69–d.73 for the RDF shape, d.94 for the per-project snapshot as a container),
   govern the shape of the committed projection file, except where `dao-lift`'s `FORMAT.md` lists a
   deviation. The `FORMAT.md` that counts is the one at the pinned commit,
   `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d` (R4.5).
3. **This PRD** governs everything else: where the file lives, how `sync` reads it, what its tests
   pin, and how the file is produced.

## Context

ADR-0007 starts the Access Area's `sync` capability at its minimum: "the committed projection of one
shortcode read from disk, behind the same port Chischtli and replay will later stand behind". CPE's
port for it, `ArchiveProjection`, was declared in DEV-7398 (PR #452). `ARCH-MAP.md` leaves the
on-disk format and the RDF parser to this ticket. This PRD decides the format. The parser is the plan's
choice, bounded by the constraints below.

The archive will eventually deliver per-project snapshots and per-resource objects in the canonical
DAO shape (d.94), produced on the producer side from VRE exports (d.50, d.90). DAO is decided at the
vocabulary level but not published: `spycherli/docs/data-model.md` is unwritten and several points
are open (Q10 file properties, Q29 date vocabulary, the resource IRI form). Two nearer sources were
rejected:

- **A knora-base dump, unchanged.** Faithful today, but far from what `sync` will receive; CPE would
  come to depend on facts the archive does not deliver (the `isPartOf` subproperty, value IRIs,
  LinkValue reifications), and `sync`'s mapping would be rewritten when data products arrive.
- **The incubator's CSV export** (`dao-lift` → CSV → `translate.py`). It drops value order and class
  identity.

So the committed projection uses an **interim format**: DAO-shaped wherever DAO has decided, deviating
only where the port needs a fact DAO drops, and declared temporary. It is produced from a VRE dump by
the incubator's `dao-lift`, which already reads the bagit dump with oxigraph. The format's
specification lives beside `dao-lift` so that it is visibly not DAO and not a commitment of this
repository; this repository pins the version it follows, today
`dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d` (dsp-incubator#411).

This reverses three choices of the port's plan
(`docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`).
That plan served membership with "the subproperty used, e.g. `incunabula:isPartOfBook`", served each
list node's `kb:listNodeName`, and broke value-order ties "by value IRI". It did so because the port
was to serve archive-shaped facts "as knora-base records them". That reason stops holding once the
archive's shape is DAO rather than knora-base. DAO keeps only `dao:isPartOf` in the data (d.69). It
has no list-node name (d.70). It re-mints value IRIs, so the UUID is the stable key (d.48). Because
#452 is unmerged, the port is corrected there rather than worked around (R3). Its plan gets a note
recording the amendment.

The source is the stage dump of 0803 bagged 2026-07-19 (DSP-API v37.1.0, knora-base 52): 312,747
quads in one data graph, 19 Books, 4,024 Pages, 38 Bands, 77 Regions, 40 LinkObjs, 23 list nodes in
four lists no property references, 3 superseded versions of 2 values, no deleted resources or values,
and no language-tagged, boolean, decimal or list values (the only language tags are on list-node
labels). Its only integer values are the 4,024 Page seqnums, which the format carries as `dao:seqnum`,
not as values. The snapshot `dao-lift` makes of it has 123,938 quads (about 20 MB). Incunabula is not being edited, so stage data serves
until go-live.

## Goals

- `sync` exists as a capability and serves 0803 through `ArchiveProjection`.
- Its input is as close to the future archive format as today's decisions allow, so that replacing it
  with real data products changes vocabulary, not architecture.
- The port serves exactly what the interim format carries. Every fact kept against DAO is listed in
  `FORMAT.md`:
  - value order and text language are raised upstream (DEV-7445);
  - dates and files follow DAO's open questions Q29 and Q10;
  - lists follow whether DAO's profile is delivered to the Access Area.

## Core features

- **The interim format and its producer** (`dsp-incubator`). `dao-lift` emits one project snapshot as
  sorted N-Quads, in the container shape of d.94's per-project snapshot. `FORMAT.md` beside it
  specifies the format, cites the DAO decisions it follows, and lists each deviation with its reason.
  The deviations that motivated the format are below; `FORMAT.md` §12 lists all thirteen (D1–D13),
  among them the source property as predicate (D3), the colour and geometry datatypes (D10), the
  minimal Web Annotation (D11), the dropped standoff (D12) and the omitted interval, time and
  geoname values (D13):

  | Kept against DAO | Why |
  |---|---|
  | Value order (`valueHasOrder`), tie-break by `valueHasUUID` | Presentation needs it (0803 has 443 non-zero value orders, 482 with the LinkValues the format does not carry); d.48 drops it |
  | Text language | d.48's ApiV2Simple wording implies it; not stated |
  | Date as calendar plus start/end JDN and precision, beside DAO's lexical | d.48 drops the JDN as recomputable, and Q29 (the date vocabulary) is open; the JDN is lossless and is what Chischtli's planned date histograms use |
  | `internalFilename` and dimensions of a still image | The only key today's IIIF serves; Q10 and d.94's Service File keys replace it |
  | The project's lists (node tree, position, labels) inside the snapshot | d.70 places them in the application profile, and no decision delivers the profile to the Access Area; the port serves them with the snapshot |
  | Value nodes as IRIs minted from their UUID | d.48 and d.51 describe blank nodes; IRIs make the sorted output deterministic, as in the design spike |

  Where DAO decides, the format follows:
  - `dao:Resource` with `dao:sourceClass`;
  - thin value nodes with `dao:valueHasUUID` and `dao:sourceProperty`;
  - direct resource-to-resource links;
  - `dao:isPartOf` and `dao:seqnum`, with the subproperty not in the data;
  - Regions and LinkObjs as annotations (d.71), both typed `oa:Annotation`;
  - list values carrying their node (d.70), with no node name.

  LinkObjs are carried as resources with their links, like Regions. Standoff markup is not carried;
  the plain text is.

  Filtering is the producer's job, as in DAO (d.48), and the file is a snapshot of current state, as
  d.94's snapshots are. It holds no deleted resources or values and no superseded value versions, and
  it carries no marker from which they could be recovered. Tombstones belong to d.94's per-resource
  change objects, which this minimum does not have. Checking the dump is the producer's job too:
  `FORMAT.md` names 41 violation rules, and `dao-lift` fails the run, writing no file, when any of
  them is broken.

- **The committed projection** (`dsp-repository`). `areas/access/sync/data/0803.nq`, plain N-Quads, not
  compressed, not in LFS, beside a `PROVENANCE` file.

- **`sync-store`** (`areas/access/sync/store/`). The single crate of the capability at its minimum. It
  reads the interim file, maps it to `cpe-ports` DTOs (leaving out what the port does not carry),
  and implements `LiveArchiveProjection`. Its dependencies are `cpe-ports` and an RDF parser, and
  nothing of CPE or DPE. No `sync-domain` and no `sync/ports` yet: all logic is adapter logic beside
  the data (ADR-0003). The file is read at run time from a directory given to the adapter when it is
  constructed, not embedded.

- **The port amendment** (PR #452). Membership without its subproperty, no list-node name, a UUID on
  every value that has one (all but links), and every field kept against DAO marked provisional in
  its doc comment. The change reaches everything in the port that states the old rules: the
  contract's violations (and three new ones, `MissingValueUuid`, `DuplicateValueUuid` and
  `LinkWithValueUuid`), the fake, the doc comments, and `areas/access/cpe/CONTEXT.md`. That file's definition of an archive-shaped fact
  changes from "as knora-base records it" to the archive's shape as the interim format carries it.
  The port's plan gets a note recording the amendment.

- **Records.** An ADR-0007 amendment recording the interim format, the rule "DAO where decided,
  deviations listed", the port following it, and the exit condition. A seed
  `areas/access/sync/CONTEXT.md`. `ARCH-MAP.md` brought in line with what is built and decided: the
  `areas/access/sync` entry's paths, format, parser, durable state and local-context kit, and the
  sentences elsewhere that still call `sync` empty or planned. The crate lists in `docs/src/` and
  `CONVENTIONS.md`.

## Repository impact

- **dsp-repository.** The #452 amendment (in #452's own commit, before it merged). Then, stacked on
  #453 (ADR-0009 and the map entries for `sync`, DaTEI and `media`): this PRD and its plan in #455,
  and above it the implementation PR with `areas/access/sync/{store,data}`, `CONTEXT.md`, the
  ADR-0007 amendment and the documentation updates. `access-server` is not touched.
- **dsp-incubator.** `cpe/tools/dao-lift` gains the interim N-Quads output, its `FORMAT.md` and tests.
  A separate ticket that blocked DEV-7399; it landed as dsp-incubator#411, pinned at
  `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`. The CSV output stays while
  the prototype still builds `data.sql` from it.

## Requirements

### R1 Serving the snapshot

Protects ADR-0007's "a project fails alone": CPE receives one whole snapshot or a typed error, never a
half-read file.

`sync` holds a fixed set of projects, today only 0803. A shortcode is *known* when it is in that set,
whether or not its file can be read.

- REQ-R1.1 (Event-driven): When `snapshot("0803")` is called, `LiveArchiveProjection` shall return a
  snapshot built from `0803.nq` in the directory it was constructed with.
- REQ-R1.2 (Event-driven): When the file changes between two calls, `LiveArchiveProjection` shall
  serve the changed content on the second call.
- REQ-R1.3 (Unwanted-behaviour): If the requested shortcode is not known, then `LiveArchiveProjection`
  shall return `UnknownProject`.
- REQ-R1.4 (Unwanted-behaviour): If a known project's file is missing, unreadable or not valid in the
  interim format, then `LiveArchiveProjection` shall return `Unavailable` and never a partial snapshot.
- REQ-R1.5 (Unwanted-behaviour): If a known project's file is valid but holds no resources, then
  `LiveArchiveProjection` shall return `Unavailable`. An empty file is a failed regeneration, not an
  empty project.

### R2 Fidelity to the committed file

Covers what `contract::violations` cannot see, so that a mapping bug fails in `sync`'s tests and not on
a CPE page.

- REQ-R2.1 (Ubiquitous): `contract::violations("0803", …)` shall return no violations on the committed
  file. A violation is a test failure, i.e. a bug in `dao-lift` or `sync-store`. `sync` does not run
  the contract at run time.
- REQ-R2.2 (Ubiquitous): Within one property, `sync-store` shall order values by `valueHasOrder`
  ascending, a missing order counting as 0, ties broken by value UUID (by target IRI for links).
- REQ-R2.3 (Ubiquitous): `sync-store` shall serve each 0803 value that has superseded versions, and
  sits on a served resource, exactly once, under the current version's UUID. 0803 has two such values
  (three superseded versions in all); one sits on a Region, so R2.4 drops it with its resource, and
  one is served. Only the current version carries a UUID, and the oldest version of the served value
  has the same text as the current one, so content alone cannot tell them apart. The filtering itself
  is `dao-lift`'s (R4.4).
- REQ-R2.4 (Ubiquitous): `sync-store` shall serve no resource typed `oa:Annotation`, and no value,
  link or `part_of` whose subject or object is one. `FORMAT.md` §10 gives that one marker to every
  Region and LinkObj, project subclasses of either included; `sync-store` shall identify annotations
  by it, never by `dao:sourceClass`.
- REQ-R2.5 (Ubiquitous): `sync-store` shall serve a text value's plain text without its markup.
- REQ-R2.6 (Ubiquitous): `sync-store`'s tests shall have one synthetic fixture in the interim format
  for each value kind, and each file kind, that the mapping handles and 0803 lacks: today language
  tags, integer, boolean, decimal and list values, and audio, moving-image and document files.
- REQ-R2.7 (Unwanted-behaviour): If the committed 0803 file is absent, then the tests that read it
  shall fail, not skip.

### R3 Port alignment with the interim format

Keeps CPE from building on facts the archive will not deliver. It is cheapest before #452 merges.

- REQ-R3.1 (Ubiquitous): `cpe-ports` shall serve membership as the parent resource's IRI only, without
  the property it was recorded under.
- REQ-R3.2 (Ubiquitous): `cpe-ports` shall carry no list-node name.
- REQ-R3.3 (Ubiquitous): Each `cpe_ports::Value` shall carry its value UUID. The value-order
  tie-break shall use it. A link is a bare triple in DAO (d.69) with no UUID and no order, so its UUID
  is absent, it counts as order 0, and it ties by target IRI.
- REQ-R3.4 (Ubiquitous): Every `cpe-ports` field kept against DAO shall say so in its doc comment,
  naming the DAO decision or open question it deviates from.
- REQ-R3.5 (Ubiquitous): `cpe_ports::Value.property` shall be the value's source property as a full
  IRI (e.g. `http://www.knora.org/ontology/0803/incunabula#hasTitle`), never its data predicate and
  never compacted. Source properties are lossless where two properties crosswalk to one standard
  term, and they are what CPE's KDL names. So the interim format shall let every direct link name its
  source property (decided 2026-09-29, in planning). `FORMAT.md` §6 does so: a link's predicate is
  the source property itself.

### R4 Producing the interim snapshot

Makes regeneration reproducible and reviewable as a diff, and keeps the format visibly temporary.

- REQ-R4.1 (Event-driven): When run on a VRE project dump, `dao-lift` shall emit one project snapshot
  as sorted N-Quads in the interim format.
- REQ-R4.2 (Ubiquitous): `dao-lift` shall produce byte-identical output for the same dump. This is
  verified by `dao-lift`'s own tests in DEV-7443; the dump is not committed here.
- REQ-R4.3 (Ubiquitous): `dao-lift`'s `FORMAT.md` shall be headed as temporary and not DAO, and shall
  list every deviation from DAO with its reason.
- REQ-R4.4 (Ubiquitous): `dao-lift` shall omit deleted values as well as deleted resources, and
  superseded value versions.
- REQ-R4.5 (constraint, no EARS trigger): `areas/access/sync/data/PROVENANCE` records the dump's date,
  server and DSP-API version, and the `dao-lift` commit, which fixes the `FORMAT.md` the file follows:
  `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`.

## Constraints

- `sync-store` depends on `cpe-ports` and on no other CPE or DPE crate (ADR-0003).
- The RDF parser is pure Rust, like the rest of the build, and reads N-Quads as a stream; a SPARQL
  store is not needed at this minimum.
- No presentation remodelling in `sync`: it maps to archive-shaped DTOs only (ADR-0007). Computing the
  port's DTOs from the interim format is mapping, not remodelling.
- The committed file is outside Git LFS, at about 20 MB uncompressed (123,938 quads for the
  2026-07-19 dump). Should it become a burden, the LFS scope of this repository can be widened.
- The dump's admin and permission graphs are never read or committed; the interim format carries no
  users, permissions or creation metadata.

## Success criteria

- `contract::violations("0803", …)` is empty on the committed file, and the R2 fidelity tests pass
  against it.
- DEV-7400 builds CPE's read model on `cpe-ports` without changing it for anything the interim format
  already carries.

## Out of scope

- Wiring `LiveArchiveProjection` into `access-server` and CPE: DEV-7400, the first ticket with a
  consumer.
- Chischtli, snapshot-plus-replay, NATS.
- Moving DPE onto `sync`.
- Regions and LinkObjs in the port: added as CPE's port change when annotation storage (the
  prototype's engine gap G5) lands.
- Standoff markup: added as a field of `Text` when DSP-API emits its HTML-like XML form and a project
  presents it; d.51 already places the XML in the value node.
- The DAO application profile (class, property and list labels): KDL declares CPE's labels.
- Rights and legal information (ADR-0007's ARK clause).
- The resource IRI form: `rdfh.ch` IRIs are kept; nothing depends on the choice yet.

## Follow-up tickets

- **DEV-7443**: `dao-lift` interim output in `dsp-incubator` (R4). Done: dsp-incubator#411, pinned at
  `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`.
- **DEV-7444**: regenerate 0803 from a prod dump before CPE goes live, scheduled ahead of go-live.
- **DEV-7445**: raise value order and text language upstream against d.48 in `dsp-repository-design`,
  with the interim format's deviations as evidence.
- **DEV-7400** now carries the wiring of `LiveArchiveProjection` into `access-server` and CPE.

## Open questions

None blocking this PRD. The fields kept against DAO are tracked by the follow-up tickets above.
