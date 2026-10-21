# Incunabula (0803) — curation

**What this site showcases and how it is presented.** Decided with the owner on 2026-09-02
(Movement 2, Stage B), on the basis of [`SHAPE.md`](./SHAPE.md). This is the input the
Stage-C site concept and the Stage-D design briefs depend on: the design decides *how* these things look and behave,
not *what* is shown. Where a decision strains the engine, the strain is named here and
tracked in `dsp-incubator cpe/docs/PROJECT_PLAN.md` under "Engine extensions the real projects will force".

> **Terminology note.** This document predates owner decision **O2** (see
> `dsp-incubator cpe/projects/incunabula/DESIGN.md` §3), which fixed **annotation** as the CPE noun for what DSP's knora-base calls a `Region`
> — the glossary in `dsp-incubator cpe/CONTEXT.md` follows O2; in this repository `areas/access/cpe/CONTEXT.md`
> governs the vocabulary. The wording "region" is left
> as written: it is correct where it records the DSP source data or an IIIF `pct:` region URL.
> Where it names a **CPE** concept, read "annotation". "Reuse" as an **editorial** concept
> (the woodcut reuse cases, "Alle Wiederverwendungen") is domain vocabulary and is **not**
> affected — only the capitalised `Reuse` **class** noun is superseded (see §6).

## 1. The story

**A reading library of Basel's illustrated early prints, told through curated text.**
Nineteen books from four Basel printing offices, 1471–1504, each one openable and
readable page by page — but the site is not an image gallery. Our own editorial writing
leads: the home page introduces the corpus and its offices, each book gets a short
introduction, and two showcase pages tell the stories the project was actually about.
The project's own apparatus (page descriptions, notes, citations, shelfmarks) reads as
supporting text beside the scans. The feel is a scholarly edition, not a picture viewer.

Two **showcase entry points** sit beside the library:

1. **The Narrenschiff** — Sebastian Brant's *Ship of Fools*: the German edition of 1495
   (1495) and the two Latin editions (March and August 1497), the border-strip system that
   frames its text, and the way the same woodcuts travel between editions.
2. **The re-used woodcut** — the project's research trace: the ~30 documented cases of
   one woodblock printed on two (or four) pages, across editions and within single books,
   shown as the crops side by side with the annotator's remark.

## 2. What a visitor can do

| Verb | Where | How |
|---|---|---|
| **Read** a book | `/incunabula/books/<id>` → reader | Page by page: scan large, page id + description + comments beside it, prev/next and jump-to-page. On the 208 Narrenschiff pages with border strips, the left/right strip is a linked chip. Pages carrying a reuse annotation draw the region on the scan. |
| **Choose** a book | home | A hand-crafted shelf of the 19 books grouped by printing office, and a **timeline** (1471–1504, lanes per office, year-ranges as spans). Both open the book. |
| **Follow** the Narrenschiff | `/incunabula/narrenschiff` | Editorial page: the three editions compared, border-strip section, links into each edition's reader and to the woodcut page. |
| **Compare** woodcuts | `/incunabula/woodcuts` | Editorial walkthrough of the reuse cases; each case = 2–4 IIIF region crops side by side + remark + links to the pages. |
| **Browse** border strips | `/incunabula/bands` (+ detail) | The 38 strip types with their reference image and description; each lists the Narrenschiff pages that use it. |
| **Switch language** | chrome | DE (default) and EN. |

**Not offered:** free-text search (19 books and one-line descriptions do not justify it);
faceted browse of books (the shelf and the timeline are the browse); FR/IT.

## 3. Site map

```
/incunabula/                       home — editorial intro · shelf by office · timeline (custom)
/incunabula/books                  the shelf again as a plain list (fallback / share target)
/incunabula/books/<id>             book: bibliographic header + our introduction + reader
/incunabula/narrenschiff           showcase 1 — the three editions + border strips
/incunabula/woodcuts               showcase 2 — the re-used woodcut
/incunabula/bands                  border-strip typology (browse)
/incunabula/bands/<id>             one strip: image, description, pages using it
```

Nav: Home · Books · Narrenschiff · Woodcuts · Border strips.

## 4. What is foregrounded, what is supporting, what is omitted

**Foregrounded.** Our editorial text (home, per-book intro, two showcase pages); the
scan in the reader; book title, author, printer, date, place; the timeline.

**Supporting, rendered where present.** Page description and comments; book notes,
physical description, shelfmark (`hasLocation`), catalogue URI, reference sigla
(`hasCitation`, as a compact list); page `hasOrigname` (scan filename) and the three page
`hasCitation`s; the three book `hasDescription`s. Border-strip chips on Narrenschiff pages.
Region overlays on annotated pages.

**Omitted.** Search. FR/IT. The `[missing]` sentinels (normalised to absent at
translate time). The orphan lists (Lingua, Cantons, colours). Test artefacts among the
link objects (`DEMO_`, `DEMOZ`, `TEST3`, `SBFI 3`, `Verkündigung, Test Lothar`) and the
one non-woodcut link (`Zorn als Laster …`) — the woodcut page is curated, not generated.

## 5. Component mapping and where the shape strains the engine

| Surface | Component | Status |
|---|---|---|
| Home editorial | `narrative` (editorial archetype) | exists |
| Shelf by office | `narrative` invite-lists, one `##` per office — or `resource-list` × 4 with `where printer=…` | exists; office is plain text on `hasPublisher`, so grouping is either hand-written or needs a translate-time `office` list property |
| **Timeline** | **per-project custom component** | **new** — first use of the cpe-poc §8.2 escape hatch; input = 19 books with ISO start/end + office; lookback: is a date-spread view generic? |
| Book page header | `resource-detail` | exists; wants the "compound-level header" extension (bibliographic block + "start reading") |
| Per-book introduction | `narrative` per book, or a `richtext` property authored at translate time | exists; decide in Stage D |
| Reader | `compound-viewer` + record viewer | exists; **first real paging stress** (511 pages, Alice max 16); jump-to-page is new behaviour |
| Border-strip chips | link-property rendering on the page | exists (link → label) |
| **Region overlays** | record viewer extension | **new** — draw the region shape(s) on the scan; geometry is normalised JSON in three types (rectangle, **polygon**, circle — incunabula has all three), so an SVG overlay, not a CSS box |
| Woodcut page | `narrative` with images | exists; IIIF `pct:` region URLs derived from each region's bounding box; side-by-side layout needs a small CSS convention (two-up / four-up figure) |
| Narrenschiff page | `narrative` | exists |
| Bands browse + detail | `resource-list` + `resource-detail` | exists; "pages using this strip" = incoming links, which the detail page does not render today |
| Display titles | `title-property` | **generalise** — Book = `hasTitle`, Page = `hasPagenum`, Band = `hasTitle`; none is `name` |
| Dates | ISO text | translate converts Julian/Gregorian `DateValue`s (JDN available) to ISO and keeps the display string |
| Languages | chrome + labels | DE + EN; project text stays untagged German/Latin |

The two **new** rows (timeline, region overlays) are the deliberate cost of this curation.
Both are candidates for the Movement-3 lookback ("became a custom component but should be
promoted?"). Nothing here needs facets, list values, restricted images, or search.

## 6. The data slice (for Stage E)

Everything, not a slice: 19 Books, 4,024 Pages with their IIIF filenames and dimensions,
38 Bands, plus the **77 Regions** (page, geometry, colour, remark; 52 rectangles, 24 polygons
and 1 circle) and the **woodcut link objects** (curated subset of the 40). *(As built in Slice 6:
dropping the test artefacts keeps 64 annotations on 59 pages and 35 links; which ones are kept is
the `keep` column of `areas/access/media/data/0803/curation.csv`, see [`PROVENANCE.md`](./PROVENANCE.md).)* How Regions and links map onto CPE's generic schema
is a Stage D/E decision — the likely shape is a `Region` class with a link to its Page and
the geometry JSON as text, and a `Reuse` class linking 2–4 Regions — and is itself a finding
about the schema. *(**Superseded by O2** — see `dsp-incubator cpe/projects/incunabula/DESIGN.md` §3: annotations are a
first-class engine table, `annotations`, with geometry normalised in the schema and a generic
overlay capability, **not** a per-project class with geometry as plain properties. The sentence
stands as the record of what Stage B guessed.)* Align it with the canonical treatment in `dsp-repository-design`
(`spikes/profile-as-data/examples/region-builtin.ttl`: a Region is an ordinary resource,
`dao:isRegionOf` the page, geometry/colour as self-contained lexicals). Dates become ISO text with the original display string kept alongside.
`[missing]` → absent.

**Source-data hygiene.** The DSP dump is fetched locally by someone with a sysadmin token
and is **never committed**: its admin graph carries user records with password hashes (see
`SHAPE.md` §9). Only derived artefacts are committed, stripped of
`attachedToUser` and every other knora-base admin predicate, and checked to contain no
`rdfh.ch/users/` IRI. *(As built since DEV-7496: this directory holds no `raw/`, and in this repository no `data.sql`. The derived
artefacts are the snapshot `0803.nq` in `areas/access/sync/data` and the hand-edited curation file
`areas/access/media/data/0803/curation.csv`, of which the incubator's `vendor/` holds a copy; a curated value is edited in this repository, see
[`PROVENANCE.md`](./PROVENANCE.md).)* The project's content itself is open (CC BY 4.0, every resource and
file value viewable by unknown users), so committing derived data raises no access question
for Incunabula; HdM and Solec need the same check before anything of theirs is committed.

**Pipeline shape (open).** Two options for the transform: translate straight from the Knora
N-Quads, or copy the `profile-as-data` spike from `dsp-repository-design` as a first,
project-agnostic stage (DSP export → canonical DAO, adding the Region/LinkObj pass-through
it deliberately omits) and translate DAO → CPE as the second. The second makes CPE the first
real reader of the archive shape, which is a finding in itself. Not decided yet.
*(Decided since: the second, and from DEV-7496 through the monorepo's port; see
[`PROVENANCE.md`](./PROVENANCE.md), "Owner decision O1".)*

## 7. Demo path (draft, ten beats)

1. Home — the lede on Basel as the printing centre; the four offices; the shelf.
2. The timeline — read the spread 1471–1504, see Amerbach and Furter cluster in the 1490s.
3. Open *Postilla* (Furter, 1491) — the bibliographic header, our introduction, "start reading".
4. Read three pages — description beside the scan; jump to `e3r`.
5. `e3r` → `e4r` — the reuse annotation on the scan; follow it to the woodcut page.
6. Woodcut page — the Postilla pair, then the Narrenschiff German/Latin pairs side by side.
7. Narrenschiff page — three editions, what changed, the border-strip system.
8. Into the German edition, page `b1r` — the strip chips `l_1a` / `r_1`; follow `r_1`.
9. Border strips — the typology, the strip's reference image, the pages that use it.
10. Switch to EN — chrome and editorial in English, the German apparatus unchanged.

## 8. Open for the design briefs (Stage D)

- The **shelf**: designed grouping by office — cards, a list, or the timeline's own lanes?
- **Timeline** form: horizontal band with lanes; how year-ranges, same-year books, and the
  1504 outlier are drawn; mobile behaviour; is it also the shelf on small screens?
- **Reader** at 500 pages: jump-to-page affordance, position indicator, keyboard paging.
- **Region overlay** treatment: colour, hover/label, and how a page with two regions reads.
- **Two-up / four-up figure** convention for the woodcut page.
- **Book header** anatomy: title(s), author, printer · place · date, shelfmark, sigla.
- Whether per-book introductions are written for all 19 or only the featured ones.
