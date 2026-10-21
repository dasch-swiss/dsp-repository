# Incunabula: where the store comes from

Incunabula's store is built by the store builder, [`cpe-store-build`](../../store-build/README.md), from the
`cpe-ports` snapshot of DSP project 0803, which `sync`'s archive projection serves, and from the project's curation
file and the `remodel {}` block of its `config.kdl`, which `media` serves as deposited files from
`areas/access/media/data/0803/` (whose `PROVENANCE`, `areas/access/media/data/PROVENANCE`, records where they came
from). Nothing in this directory is an input to it: there is no `raw/` directory, no CSV and no `translate.py` any
more.

This repository commits no store file. `dsp-incubator`, where this document was written, commits the builder's
output as `cpe/projects/incunabula/data.sql` and keeps the test that regenerates it and compares the two byte for
byte (`cpe/store-build/tests/it/fresh.rs`). Where this document names `data.sql`, the binary, `vendor/`, the pin
or a `just cpe` recipe, it describes that repository; "the monorepo" is this one.

The decisions behind this are recorded in
`dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md` (the store is built from the port) and
`dsp-incubator cpe/docs/adr/0016-remodelling-is-a-mapping-and-a-registered-hook.md` (the rules are a mapping and a
hook).

## The port and the pin

From the ports the builder reads two things: a `cpe_ports::ProjectSnapshot` and the project's curated values. Here
CPE's activation reads the snapshot through `sync-store`'s adapter and the curation file,
`areas/access/media/data/0803/curation.csv` (what the editors authored about single resources, one row per resource),
through `media-store`'s. In `dsp-incubator`, the binary gets the snapshot from `sync-store`'s adapter, which serves
the vendored data directory `vendor/dsp-repository/areas/access/sync/data`:

| File | What it is |
|---|---|
| `0803.nq` | the archive's facts: the snapshot of project 0803 that the `snapshot` subcommand of `dsp-incubator cpe/tools/dao-lift` wrote from a DSP dump |
| `0803-arks.txt` | a frozen list of the data ARKs Incunabula's store held when the file was made; `sync-store`'s tests check the served ARKs against it |
| `PROVENANCE` | the dump, the `dao-lift` commit and the commands behind the files |

Its copy of the curation file comes from that vendored `sync` directory until its next re-vendor, which takes it, and
Incunabula's `config.kdl` and narrative, from `areas/access/media/data/0803/` instead.

`sync-store`, `cpe-ports` and that directory are copies of the monorepo (`dasch-swiss/dsp-repository`) at the commit
in `vendor/PIN`, and are never edited there (`dsp-incubator cpe/vendor/README.md`, `dsp-incubator cpe/docs/adr/0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`). `just cpe
vendor-diff` prints whether the copy still equals the pin.

Not from the archive projection, and declared in the `remodel {}` block of `areas/access/media/data/0803/config.kdl`: the office and side
vocabularies with their labels, the images' authorship and licence, the IIIF host (production,
`https://iiif.dasch.swiss/0803/`, owner decision O3 of the design handoff), the sentinel `[missing]`, the curated keys
and the classes annotations may touch. What the block cannot say is Incunabula's hook,
`store-build/src/projects/incunabula/`: the calibration scans, the strips' sides, the Books' sort key and the
catalogue override below. The block's `hook` node declares what the hook may do.

The DSP dump itself is never committed (`SHAPE.md` §9, `CURATION.md` §6): its admin graph carries user records
with password hashes.

## Changing the data

The store is never edited by hand, and neither is the incubator's `data.sql`.

- **A curated value** (a Book's id, office, date display, title override, sort key, cover, teaser or catalogue
  override; a strip's id or name; whether an annotation or a link is kept, its language and its short name) is
  edited in `areas/access/media/data/0803/curation.csv`. The incubator holds a copy of it under `vendor/` and
  nowhere else.
- **The archive's facts** change with a new dump: `0803.nq` in `areas/access/sync/data` is written again with
  `dao-lift snapshot`, a tool of `dsp-incubator` (`cpe/tools/dao-lift`), as the `PROVENANCE` file beside it
  describes.
- **A constant** (an office's or a side's label, the image rights, the IIIF host) is edited in the `remodel {}`
  block of `areas/access/media/data/0803/config.kdl`.
- **A rule** (how a value becomes a row) is changed in `store-build/`: in the block where a node says it, in the
  builder's generic code, or in Incunabula's hook.

Nothing is regenerated here: the builder reads the snapshot each time it builds. In `dsp-incubator`, after a
change of this repository's data, from that repository's root:

```bash
just cpe vendor <full commit sha on the monorepo's main>   # copies the crates and the data, moves vendor/PIN
just cpe store-build incunabula --inplace                  # writes projects/incunabula/data.sql
```

`data.sql` is committed there together with the change that caused it. The builder refuses a snapshot that breaks one of its
rules and writes nothing; `store-build/README.md`, "Where each rule comes from", lists the rules.

## Data decisions that still hold

Each is a rule of the builder or a line of the block; the row named is in the table of `store-build/README.md`,
"Where each rule comes from", and the file is under `store-build/src/`.

### Ids

Ids are URL-bearing (`/incunabula/books/<book id>?page=<page id>`), so they are a contract with every link ever
shared. Each class's `id` node in the block says how its ids are made.

- **Book**: the curated `slug`, exactly one per Book (row "Book id"; `mapping/ids.rs`).
- **Page**: `<book id>-<slug of the page number>`, with `-2`, `-3`, … on a repeat within the Book, in the order of
  `(seqnum, IRI local part)` (row "Page id"; `generic/slug.rs`, `generic/order.rs`, `mapping/ids.rs`). The
  calibration scans take part in the numbering although they get no row, so dropping them moved no other Page's
  id. 408 Page ids carry a suffix; `store-build/tests/it/committed_0803.rs` holds that number.
- **Border strip**: the curated `slug` where there is one, else the slug of the label (`l_1a` gives `l-1a`). The
  one Band labelled `[missing]` has the curated id `strip-38` and the curated name "Randleiste 38" (row
  "BorderStrip id"; `mapping/ids.rs`).
- **Image**: the stem of the asset's file name; one Image per file (row "Image id"; `generic/images.rs`).
- **Annotation and annotation link**: the last part of the Region's or LinkObj's IRI (rows `annotations`,
  `annotation_links`; `generic/annotations.rs`).
- **Slug**: NFKD, combining marks dropped, ASCII lower case, every run outside `[a-z0-9]` one `-`, ends trimmed
  (row "Slug"; `generic/slug.rs`).
- Every id is checked once it is made: it is not empty and holds only `A-Z a-z 0-9 - . _ ~` (row "Every id";
  `finalise.rs`).

### Calibration scans

A Page whose page number is exactly `[missing]` is a shot of a page beside a colour chart: 2 Pages. It is dropped,
with no Page, no part and no Image, and its Book's Pages are numbered 1..n without it. The archive serves 4,024
Pages and the store holds 4,022. Such a Page carrying a description, a comment or a strip link is refused, and so
is a kept annotation on one (rows "Calibration scans", `compound_parts`, "Annotation checks"). The drop is a rule of
Incunabula's hook, `projects/incunabula/scans.rs`, which the block declares as `drops class="Page"`.

### The `[missing]` sentinel

The archive writes `[missing]` where a value is not known. The builder never shows it:

- a Book's `pub-place` that is exactly `[missing]` gives no row (`itinerarius-peregrinatio`; `DATA_ISSUES.md` D4);
- a strip's description that is exactly `[missing]` gives no row;
- a Page's page number that is exactly `[missing]` makes it a calibration scan, see above;
- in the other values the builder reads it is refused; the rows say for each value whether that holds for the
  whole value or for any part of it (rows "Book `pub-place`", "BorderStrip `description`", "Every Book value read,
  every curated value", "Page `description`", "Page `comment`"; the block's `sentinel` node and the `sentinel=` and
  `sentinel-within=` attributes of its `property` nodes).

### Covers

A Book's cover is the Image of one of its own Pages, named by the curated `cover_page` (that Page's IRI). The cover
link and that Page's scan link point at **one** Image, never two: 19 of the 4,022 Page scans are also a cover, and
the store holds 4,060 Images (4,022 Page scans and 38 strip plates). A cover that is not the Book's own Page, is a
calibration scan or is the cover of two Books is refused (rows "Image id", "Book `image`"; the `cover` node of the
Book class, `generic/images.rs`, `covers`).

The covers were picked on 2026-09-29 by viewing 300×400 IIIF renders of each Book's early pages, in this order of
preference: the title woodcut, then a title page, then the first full-page woodcut (for a manuscript, the first
illustration), then the first text page. Bindings, pastedowns, flyleaves, colour targets, blanks and `[missing]`
pages are skipped. No Incunabula scan is restricted or limited-view. A re-pick is a one-cell edit of
`areas/access/media/data/0803/curation.csv`.

| Book | Cover (Page id) | Page number | Reason |
|---|---|---|---|
| `bereitung` | `bereitung-a1r-titelblatt-recto` | a1r, Titelblatt, recto | Title page (text only); the book has no title woodcut, and its first cut (e2v) is a small one in the text. |
| `brandan` | `brandan-a1r` | a1r | Title page (text only), after the `[missing]` scan and the binding; a1v carries the first full-page woodcut. |
| `de-generatione` | `de-generatione-a2v` | a2v | No title page (seq 1 is a flyleaf of notes); the first near-full-page woodcut, a coloured standing Virgin. |
| `itinerarius-peregrinarius` | `itinerarius-peregrinarius-a1r-titelblatt` | a1r, Titelblatt | Title page (text only), after the binding, spine, colour target, pastedown and flyleaf. |
| `itinerarius-peregrinatio` | `itinerarius-peregrinatio-1-recto` | 1 recto | A manuscript with no title page or picture in its opening leaves: the first text page, with a red incipit and a painted initial. |
| `lob-der-glieder` | `lob-der-glieder-a1r-titelblatt` | a1r, Titelblatt | Title page (two lines of text); a1v carries the Madonna woodcut. |
| `melusine` | `melusine-5` | 5 | A manuscript with no title page (seq 1 is a colour target over a blank leaf): the first coloured drawing. |
| `methodij` | `methodij-a1r-titelblatt-recto` | A1r; Titelblatt, recto | Title woodcut (the angel bringing Methodius the prophecies, coloured), after a flyleaf. |
| `narrenschiff-dt` | `narrenschiff-dt-b1v` | b1v | The first quire is lost and supplied by hand, so there is no printed title page: the first woodcut (chapter 6, the gaming children). |
| `narrenschiff-lat-august` | `narrenschiff-lat-august-a1r-titelblatt-recto` | a1r; Titelblatt, recto | Title woodcut (the ship of fools), after two flyleaves. |
| `narrenschiff-lat-maerz` | `narrenschiff-lat-maerz-a1r-titelblatt-recto` | a1r; Titelblatt, recto | Title woodcut (the ship of fools), after the pastedown and two flyleaf fragments. |
| `orationes` | `orationes-a1r-titelblatt-recto` | a1r, Titelblatt, recto | Title page (one line of text); a1v carries the Crucifixion woodcut. |
| `passio-meynrhadi` | `passio-meynrhadi-a1r-titelblatt` | a1r, Titelblatt | Title woodcut (Meinrad seeking a dwelling in the forest), after the pastedown and two flyleaves. |
| `postilla` | `postilla-a1r-titelblatt` | a1r; Titelblatt | Title page (text only); a1v carries the Crucifixion woodcut. |
| `quadragesimale` | `quadragesimale-a1r-titelblatt-recto` | a1r; Titelblatt recto | Title page (text only), after the pastedown and two flyleaves. |
| `reise` | `reise-6r` | 6r | No printed title page (the text opens at 1r): the first full-page woodcut, the author's departure. |
| `walfart` | `walfart-a1r-titelblatt` | a1r, Titelblatt | Title page (text only), after a flyleaf; a1v carries the Schutzmantelmadonna woodcut. |
| `zeitgloecklein-1490` | `zeitgloecklein-1490-a1r` | a1r | Title page (one line of text). |
| `zeitgloecklein-1492` | `zeitgloecklein-1492-a1r-titelblatt` | a1r, Titelblatt | Title woodcut (the clock face in a coloured border). |

### The catalogue override

Three Books' catalogue address in the archive is a dead Cologne link. The override is curated: each of the three
has two values in the monorepo's curation file, `areas/access/media/data/0803/curation.csv`, `catalogue_uri` (the
address to write) and `catalogue_uri_replaces` (the dead address), and CPE reads them through the bytes port (DEV-7497). A rule of Incunabula's hook,
`store-build/src/projects/incunabula/catalogue.rs`, reads the two keys and replaces the address while the archive
has exactly the one the override names; with any other address, or none, the Book is refused until both values are
taken out of the curation file (row "Book `catalogue-uri`"). The rule holds no address and names no Book. The
block's `hook` node declares it as the two `key` lines and `overrides "catalogue-uri" class="Book"`. The override
is temporary: it goes once the address is corrected in DSP and the corrected data is vendored (`DATA_ISSUES.md`,
"The Köln catalogue link"; `dsp-incubator cpe/docs/INCUNABULA_GO_LIVE.md`, "After go-live").

### ARKs

`resources.ark` holds the plain data ARK of every Book, Page and border strip (4,079) and is `NULL` for an Image;
`annotations.ark` holds the ARK of every kept annotation (64). The builder takes each from the port, and
`sync-store` computes it from the resource's IRI; none is fetched. `cpe-core`'s check `MalformedDataArk`
admits only a plain data ARK in either column (`dsp-incubator cpe/docs/SCHEMA.md`).

The ARKs were checked against the resolver once, on 2026-10-06: one `GET` each, following no redirect, for 141 of
them (all 19 Books, all 38 Bands, the 64 kept Regions and 20 Pages drawn with a fixed seed) answered 302. A 302
confirms an ARK's form and check digit; it does not by itself show that the resource exists in production.

### Named differences from the last Python-written store

The builder's store equals the store the old pipeline last wrote, in all ten tables, apart from two shapes on
seven class and property pairs: the named differences N1 and N2. `store-build/README.md`, "Named differences",
defines them; `dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md` records their counts and how the two stores were compared; `DATA_ISSUES.md` gives the
reasons.

## Owner decision O1

This is the O1 of Incunabula's earlier slices, not the O1 of `dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md`.

It decided that Incunabula's data comes through the canonical DAO stage: DSP dump → `tools/dao-lift` → `data.sql`.
As built then, the pipeline had a CSV hop in the middle: `dao-lift cpe-export` projected the DAO graph into CSV
files committed under `raw/`, and a stdlib-only `translate.py` turned them into `data.sql`. The hop existed because
`just cpe translate-check` had to run in CI, where the dump and the DAO graph never exist. Pages came through it
from Slice 2 and Books from Slice 8, when the curated columns moved into a hand-authored `raw/curation.csv`.

The hop is retired (DEV-7496, 2026-10-10). The DAO stage is still the pipeline: the incubator's `dao-lift snapshot` writes
`0803.nq` in the monorepo, `sync-store` serves it through the port, and the store builder writes `data.sql` from
that. `translate.py` and the CSV files are deleted; the curated layer is the monorepo's curation file, today `areas/access/media/data/0803/curation.csv`.
`dao-lift cpe-export` still exists and nothing in `cpe` reads its output (`dsp-incubator cpe/tools/dao-lift/README.md`).
