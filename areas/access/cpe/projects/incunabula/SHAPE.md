# Incunabula (0803) — project shape

The single reference that Movement-2 Stage B (curation), Stages C–D (design) and
Stage E (data) build on. It records **what the project is and what its data looks like** on the server. It
does not make modelling decisions — those belong to the design brief and `config.kdl`.

Everything below was read from **`api.stage.dasch.swiss`** on 2026-09-02 via `dsp` 0.1.3
(`dsp vre project describe`, `data-model describe`/`structure`, `resource-type describe
--count`, `resource list --all`, `project dump --skip-assets`), cross-checked against the
project metadata record served by DPE (`repository.dasch.swiss/dpe/projects/0803`; source file
`modules/dpe/server/data/projects/0803_incunabula.json` in `dasch-swiss/dsp-repository`). Prod
(`api.dasch.swiss`) has the same three classes with the same instance counts, so the two
environments agree on shape; stage is the working target for now.

## 1. The project

| | |
|---|---|
| Shortcode / shortname | `0803` / `incunabula` |
| Longname (DSP) | Bilderfolgen Basler Frühdrucke |
| Full title | Die Bilderfolgen der Basler Frühdrucke: Spätmittelalterliche Didaxe als Bild-Text-Lektüre |
| Alternative title | Incunabula |
| Period | 2008-06-01 → 2012-08-31, status **Finished** |
| Discipline | 10404 Visual arts and Art history |
| Coverage | Basel (Geonames 2661604); Late Middle Ages |
| Data languages | German, Latin |
| Institutions | Kunsthistorisches Seminar and Digital Humanities Lab, Universität Basel (per the DSP project description); the copies are held at Universitätsbibliothek Basel (per `hasLocation`) |
| People | Barbara Schellewald (applicant), Hannes Hug, Lukas Rosenthaler (co-applicants) — all project leaders; Lothar Schmitt, Katrin Graf Lamei (data collectors). Contact: Rosenthaler |
| Funding | SNSF project funding, grant 120378 |
| License / access | CC BY 4.0, open |
| Permalink | `ark:/72163/1/0803` |
| Ontology IRI | `…/ontology/0803/incunabula/v2`, last modified 2024-08-14 |

**What it is about.** An art-historical monograph of the richly illustrated early prints
of Basel — the most important centre of early letterpress printing on the territory of
today's Switzerland. The corpus is **18 works from four Basel printing offices**, together
holding over 1,000 woodcuts: the most significant surviving Basel image sequences of the
late Middle Ages after the destruction of the Reformation. The texts are almost all German
and Latin compilations of religious, didactic content. Famous books (the *Narrenschiff*,
the *Heilsspiegel*) sit next to barely known ones (the pastoral writings of Ludwig Moser and
Johannes Meder). The research question is the **image–text relationship**: how the woodcuts
functioned for late-medieval readers and listeners, treating each print as a calculated
image-text compilation rather than "a text with illustrations".

**Metadata gotcha.** The DSP project description (multi-paragraph EN/DE/FR, rich HTML in
the API) and the DPE metadata record (DE-only long description, EN abstract) differ. The
metadata record is the more curated source for a home-page narrative; the DSP description
is what `project describe` returns. (`dasch-swiss/dsp-meta` is archived and stale — don't
cite it.)

## 2. Classes and properties

Three classes; every property is project-local (prefix `incunabula:`). Cardinalities are
the declared ones; the "used on" column is the observed instance coverage from the dump.

### `Book` — 19 instances

The bibliographic unit: one physical copy of one printed edition, held at Basel UB.
Label = `hasTitle` (first value) in all 19 cases.

| Property | Type | Card. | Used on | Notes |
|---|---|---|---|---|
| `hasTitle` | text | 1-n | 19/19 (26 values, max 3) | Alternate titles are additional values (e.g. Latin + German title) |
| `hasCreator` | text | 0-n | 15/19 | Author as a plain string, e.g. "Sebastian Brant" |
| `hasPublisher` | text | 0-n | 17/19 | Printer as a plain string — the "four offices": Amerbach, Bergmann de Olpe, Furter, Ysenhut (+ Richel, Wolff, Petri de Langendorff) |
| `hasPubloc` | text | 1 | 19/19 | "Basel" for 18; one `[missing]` |
| `hasPubdate` | **date** | 0-1 | 19/19 | See §4. Julian for 17, Gregorian for 2; precision YEAR (13), DAY (5), a YEAR range (2) |
| `hasLocation` | text | 0-1 | 19/19 | Shelfmark, e.g. "Basel UB, Sign: Ai II 22b:1" |
| `hasUri` | uri | 0-1 | 18/19 | Link into the Basel UB Aleph catalogue |
| `hasDescription` | text | 0-1 | 3/19 | Sparse and inconsistent ("Masse: ca. 145 x 100 mm", "Beschreibung fehlt!") |
| `hasPhysicalDesc` | text | 0-1 | 19/19 | "Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°" |
| `hasNote` | text | 0-n | 16/19 (75 values, max 13) | Copy-specific notes: completeness, colouring, ownership marks, binding |
| `hasCitation` | text | 0-n | 17/19 (217 values, max 28) | Reference-work sigla: "GW 5046", "Goff B1082", "ISTC ib01082000", "Schramm XXII p. 47" |
| `hasComment` | text | 0-n | 8/19 | |

### `Page` — 4,024 instances — `StillImageRepresentation`

One scanned page (one side of one leaf). Every Page carries exactly one
`knora-base:hasStillImageFileValue` (JP2, typically ~2,600–3,600 × 3,500–5,200 px) and
exactly one `isPartOfBook`. Label = `hasPagenum` in 4,020 of 4,024 cases (the four
exceptions differ only in quote characters).

| Property | Type | Card. | Used on | Notes |
|---|---|---|---|---|
| `isPartOfBook` | link → Book | 1 | 4024/4024 | The compound membership, declared `⊑ knora-base:isPartOf` in the ontology — the standard DSP compound pattern |
| `hasSeqnum` | integer | 1 | 4024/4024 | Declared `⊑ knora-base:seqnum`. Scan order within the book, starting at 1. Contiguous in 16 books; 3 have one gap or one duplicate (§5) |
| `hasPagenum` | text | 1 | 4024/4024 | Signature-style page id: `a1r`, `b4v`, `(5)r`, `Vorsatzblatt, recto`, `a1r; Titelblatt`; one book (*Melusine*) uses plain numbers |
| `hasOrigname` | text | 1 | 4024/4024 | Original scan filename, e.g. `IBB_1_002755521_0006_r.tif`. Housekeeping; not for display |
| `hasDescription` | text | 0-1 | 2980/4024 | The scholarly content per page: "Beginn von Kap. 1 des Narrenschiffs; Feder (braun); Rubrizierung in Rot." Median 88 chars, max 855. Missing on 1,044 pages, concentrated in 4 books (§5) |
| `hasComment` | text | 0-n | 947/4024 (1120 values) | Short remarks, median 27 chars |
| `hasCitation` | text | 0-n | 3/4024 | Effectively unused |
| `hasBandTypeL` | link → Band | 0-1 | 207/4024 | Left border-strip type (`⊑ knora-base:hasLinkTo`, i.e. a plain link, not part-of). **Only used in *Narrenschiff (dt.)*** |
| `hasBandTypeR` | link → Band | 0-1 | 208/4024 | Right border-strip type. Same single book |

### `Band` ("Randleiste") — 38 instances — `StillImageRepresentation`

A **border-strip type**: one of the reusable ornamental woodcut strips framing the text
block in the German *Narrenschiff*. Each Band is a typology entry with its own reference
image. Labels are codes: `l_1a`…`l_15` (left, 19) and `r_1`…`r_18` (right, 18), plus one
`[missing]`.

| Property | Type | Card. | Used on | Notes |
|---|---|---|---|---|
| `hasTitle` | text | 1 | 38/38 | Same as the label code |
| `hasDescription` | text (with markup) | 1 | 38/38 | "Schmale Randleiste mit drei Vögeln, 1.2 x 16.6 cm." — the **only** systematically marked-up text in the project (StandardMapping); every value ends in a stray `\x1e\n` record-separator artefact. Two are `[missing]` |
| `hasComment` | text | 0-n | 4/38 | |

### Relations (from `data-model structure`)

```
Page → Book   isPartOfBook   [link]   1     — the compound
Page → Band   hasBandTypeL   [link]   0-1   — typology reference (one book only)
Page → Band   hasBandTypeR   [link]   0-1   — typology reference (one book only)
```

No inheritance between project classes. There are no list-kind (`ListValue`) properties
in this ontology. Standard-vocabulary crosswalks exist for `hasTitle`/`hasCreator`/
`hasDescription`/`hasPublisher` but all four point at a malformed Dublin Core IRI
(`…/dc/elements/1.1title`, slash missing) — a known, systematic defect across several DSP
ontologies (see the corpus analysis in `dsp-repository-design`); don't trust them as-is.

## 3. The Book → Page compound

The whole project is one shape repeated 19 times: **a Book is an ordered sequence of Pages
by `hasSeqnum`**, each Page a single image with a signature-style page id and a
one-sentence scholarly description. Pages never exist outside a Book.

| Book (label) | Pages | Date | Printer | Author |
|---|---|---|---|---|
| Bereitung zu dem Heiligen Sakrament | 511 | 1489 | Amerbach; Petri de Langendorff | Ludwig Moser |
| De generatione Christi | 59 | 1487–1490 | Ysenhut | Franciscus de Retza |
| Die wunderbare Meerfahrt des hl. Brandan | 63 | 1491-02-19 | Wolff | Brendanus Sanctus |
| Eyn biechlin sancti Methodij … | 66 | 1504 | Furter | Olympius Methodius |
| Geschichte von der schönen Melusine | 194 | 1471-03-21 | — | Thüring von Ringoltingen |
| Itinerarius seu peregrinatio beatae virginis Mariae | 80 | 1475–1499 | — | — |
| Itinerarius sive peregrinarius Beatissime Virginis Marie | 220 | 1489 (Greg.) | Ysenhut | — |
| Lob der Glieder Mariens | 23 | 1492 (Greg.) | Amerbach | — |
| Orationes | 24 | 1492 | Amerbach | Birgitta, Hl. |
| Passio sancti Meynrhadi martyris et heremite | 34 | 1496-09-20 | Furter | Meynradus Sanctus; Albrecht von Bonstetten |
| Postilla super epistolas et evangelia | 340 | 1491-07-28 | Furter | Guillelmus Parisiensis |
| Quadragesimale de filio prodigo | 468 | 1495 | Furter | Johannes Meder |
| Reise ins Heilige Land | 208 | 1481 | Richel | Jean Mandeville |
| Zeitglöcklein des Lebens und Leidens Christi | 402 | 1492 | Amerbach | Berthold, der Bruder |
| Zeitglöcklein des Lebens und Leidens Christi | 131 | 1490 | Amerbach | Berthold, der Bruder |
| [Das] Narrenschiff (dt.) | 328 | 1495-03-03 | Bergmann de Olpe | Sebastian Brant |
| [Das] Narrenschiff (lat.) | 302 | 1497-03-01 | Bergmann de Olpe | Sebastian Brant |
| [Das] Narrenschiff (lat.) | 324 | 1497-08-01 | Bergmann de Olpe | Sebastian Brant |
| walfart oder bilgerschafft der aller seligisten Jungfrouwen Marie … | 247 | 1489 | Ysenhut | — |

Note the **duplicate labels**: two *Zeitglöcklein* (1490 and 1492 editions) and two Latin
*Narrenschiff* (March and August 1497 editions). Any list of books must disambiguate by
date or printer, not by title alone. Sizes range from 23 to 511 pages; the median book is
~200 pages.

**Images.** IIIF on stage serves every Page image at
`https://iiif.stage.dasch.swiss/0803/<internalFilename>/…` (Image API 3; verified with
`info.json` and a `!400,400` thumbnail). `internalFilename` is on the file value, e.g.
`2vbIabBOEvq-EU9jwmgEe9j.jp2`. All 4,062 file values and all resources carry the same open
permission string (`V knora-admin:UnknownUser`), so there is **no restricted-view case** in
this project.

## 4. Value shapes worth knowing

- **Text is monolingual and untagged.** All 25,647 text values have no language tag. The
  content language is German (descriptions, notes) with Latin titles; the *interface*
  labels of the ontology are multilingual (§6).
- **Dates are `knora-base:DateValue`s with mixed calendars.** 17 Julian, 2 Gregorian; serialised as
  `JULIAN:1495-03-03 CE`, `JULIAN:1487 CE:1490 CE`, `GREGORIAN:1492 CE`. The dump also carries the
  start/end Julian Day Numbers, so a translate step can produce a calendar-neutral ISO date and keep
  the display string. Precision is YEAR for 13 books, DAY for 5, one is a YEAR–YEAR range.
- **Markup is marginal.** 48 of 25,647 text values carry a standoff mapping: all 38 Band
  descriptions, 5 Band comments, 1 Page comment. Tags used: paragraph, `<br>`, one bold.
- **`[missing]` is a literal sentinel** used for absent data: one Book `hasPubloc`, one Band
  and two Band descriptions. Worth normalising to "absent" rather than displaying.
- **Two standoff links** exist (text → resource) in the whole project.

## 5. Data-quality notes (observed, not judged)

- **Page descriptions are unevenly present.** Complete or near-complete for 8 books;
  entirely absent for *Melusine* (0/194) and *Itinerarius seu peregrinatio* (0/80); largely absent
  for *Quadragesimale* (23/468) and *Postilla* (80/340).
- **Sequence irregularities in 3 of 19 books.** *walfart* has a gap (1..248 for 247 pages);
  *Zeitglöcklein* 1490 and *Brandan* each have one duplicated `hasSeqnum` and correspondingly
  one missing number. A page order derived purely from `hasSeqnum` will show one tie in each.
- **`hasPagenum` conventions vary by book.** Signature style (`a1r`, `b4v`) dominates;
  *Narrenschiff (dt.)* uses `(5)r` for its handwritten replacement leaves; *Melusine* uses plain
  numbers; a handful of ids carry editorial remarks (`a2r (falsch nummeriert "a 3")`).
- **`hasBandType*` is a single-book feature** (208 of *Narrenschiff (dt.)*'s 328 pages) —
  the Band typology is effectively a *Narrenschiff* sub-project.
- **Book-level `hasDescription` is unusable** (3 values, two of them measurements).
- **No deleted resources** in the dump.

## 6. Multilingual interface labels (ontology)

Class and property labels exist in up to four languages, unevenly:

| | en | de | fr | it |
|---|---|---|---|---|
| `Book` | Book | Buch | Livre | Libro |
| `Page` | Page | Seite | Page | — |
| `Band` | — | Randleiste | — | — |
| `hasTitle` | Title | Titel | Titre | Titolo |
| `hasDescription` | Description | Beschreibung | Description | Descrizione |
| `hasPublisher` | Publisher | Verleger | Editions | Casa editrice |
| `hasPubdate` | — | Datum der Herausgabe | — | — |
| `hasPubloc` | Publication location | Ort der Herausgabe | — | — |
| `hasCreator` | Creator | Creator | — | — |
| `hasLocation` | Location | Standort | Emplacement | Postazione |
| `hasPagenum` | Page identifier | Seitenbezeichnung | Désignation de page | — |
| `hasSeqnum` | Sequence number | Sequenznummer | Numéro de Séquence | Numero di scala |
| `hasBandTypeL/R` | — | Randleistentyp links / rechts | — | — |
| `hasNote` | Note | Anmerkung | — | — |
| `hasCitation` | Citation/reference | Verweis | — | — |
| `hasComment` | Comment | Kommentar | Commentaire | Commento |
| `hasPhysicalDesc` | Physical description | Physische Beschreibung | — | — |
| `hasOrigname` | — | Ursprünglicher Dateiname | Nom de fichier original | — |

DE is the only complete column. EN is near-complete; FR/IT are partial. Several property
comments are long Wikipedia excerpts (`hasPublisher`), not usable as UI help text.

## 7. Things in the dump that are not part of the project model

- **77 `knora-base:Region`s**, all on Pages (Narrenschiff dt./lat. carry 42 of them): red
  shapes — 52 rectangle, 24 polygon and 1 circle geometries (SALSAH-era; measured in Slice 6,
  `DATA_ISSUES.md`) — with a comment ("identischer Holzschnitt", "Wiederverwendung",
  page-order remarks). These are the project's **woodcut-reuse annotations** — the actual
  image–text research trace, but tiny in volume and attached to the image, not to a property.
- **40 `knora-base:LinkObj`s**: named links, mostly Region↔Region (28) — "identischer
  Holzschnitt" pairs; a few Page↔Page and two Book links: "Lateinische Übersetzung" joining
  the German and Latin *Narrenschiff*, and a binding note joining three books.
- **23 `ListNode`s** in four lists (Lingua, Cantons, Texthintergrundfarbe, Textfarbe) that
  **no incunabula property references**. Leftover platform test data; ignore.

## 8. What this means for CPE (observations, not decisions)

Facts from above that touch known engine surfaces or plan items. Each is for Stage B / the
design brief to decide on, and for the Movement-2 "engine extensions" list to track.

- **Compound is the whole story.** Book → Page is exactly CPE's `compound_parts` shape
  (ordered by `hasSeqnum`), with each part a single image + a one-sentence description. The
  `compound-viewer` and the record viewer are the components in play; the plan's
  "compound-level header (`.cv-compound-head`)" item is likely triggered here (a Book wants
  a bibliographic header and a "start reading" affordance).
- **Display title is not `name`.** Book title is `hasTitle` (multi-valued), Page title is
  `hasPagenum`. This is the trigger for the plan's "generalize the display property" item.
- **The date facet arrives.** `hasPubdate` is the plan's canonical `date-year` / `range`
  facet case: 19 values spanning 1471–1504, mixed calendars, some year-ranges. A Book browse
  over 19 items barely needs a facet at all; the *demonstration* value is what matters.
- **Printer and author are plain text, not lists.** A "by printing office" facet (the
  project's own framing: four offices) would need a `text`-kind facet or a translate-time
  normalisation into a list vocabulary. There is no list to reuse.
- **No `list_value` anywhere; no restricted images; no multilingual content.** Alice
  exercised all three; Incunabula exercises none. Chrome/label localization (DE-first) is the
  only multilingual axis, and the ontology's DE column is the one complete source.
- **Page count per book (up to 511) is the first real paging stress** for the compound
  viewer and any Page browse list. Alice's largest compound is 16 chapters.
- **Band is a small third class** with reference images — a natural "typology" browse or a
  Page-detail side link, but relevant to one book only.
- **Regions / LinkObjs are out of the v1 component set.** The woodcut-reuse annotations are
  the most research-specific content in the project and had no CPE surface at Stage A. Worth
  naming in the design brief as either a deliberate omission or a per-project custom-component
  candidate (`cpe-poc.md` §8.2). *(Since Slice 6 they are a first-class engine store,
  `dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`, drawn on the book reader's scan; see `DESIGN.md` §10's "Slice 6 status".)*

## 9. Reproducing this record

```bash
dsp vre project describe      -s stage -p 0803
dsp vre data-model describe   -s stage -p 0803 --data-model incunabula
dsp vre data-model structure  -s stage -p 0803 --data-model incunabula
dsp vre resource-type describe -s stage -p 0803 --data-model incunabula --resource-type Book --count
dsp vre resource-type describe -s stage -p 0803 --data-model incunabula --resource-type Page --count
dsp vre resource-type describe -s stage -p 0803 --data-model incunabula --resource-type Band --count
dsp vre project dump          -s stage -p 0803 --skip-assets -o incunabula-0803.zip   # needs sysadmin token
```

The dump (3 MB zipped, 70 MB of N-Quads in `data/rdf/data.nq`) is the source for every
"used on" count and sample above.

**⚠️ The fresh dump is not safe to commit — anywhere, private repos included.** Its
`data/rdf/admin.nq` holds the project's user records: names, e-mail addresses and **bcrypt
password hashes**. `permission.nq` is harmless (16 permission triples). `data.nq` carries an
opaque `knora-base:attachedToUser` IRI on every value and file value (~30k triples) — not a
credential, but nothing a presentation needs. Rule for Stage E: unzip, keep only `data.nq` +
`ontology-1.nq`, delete the rest immediately; commit only derived artefacts (and check them
for `rdfh.ch/users/` before committing).

**A sanitised copy of the same stage dump is committed** (Git LFS) in
`dasch-swiss/dsp-repository-design` at `spikes/profile-as-data/dumps-stage/0803-incunabula.zip`
— only `data.nq` + `ontology-1.nq`, the admin/permission graphs and bagit files stripped
(verified 2026-09-02) — next to a per-project census
(`census/0803-incunabula.json`) and the `profile-as-data` spike that normalises it to the
canonical DAO shape (see that spike's `findings.md`). HdM (`081C-hdm.zip`) and Solec
(`0868-solec.zip`) dumps and censuses sit in the same folders — Stage A for both can start
offline.
