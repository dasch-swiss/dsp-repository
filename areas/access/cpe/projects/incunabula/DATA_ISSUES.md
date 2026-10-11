# Incunabula (0803) — data issues register

Places in the **Page** data (and, since Slice 8, the **Book** data — see the last section) of **Incunabula** (shortcode `0803`, ontology `incunabula`) where something looks like an
error or an artefact, or forced a rule in CPE's extraction. Compiled 2026-09-23 for Slice 2 (the Book reader), from the
committed `raw/pages.csv`.

The register **only observes**. It records what the data says, not whether the scholarship is right. A duplicate
sequence number or a placeholder pagenum counts as an issue here. A collation decision (how a quire is signed, which
leaves are numbered) does not. Where an entry says "looks wrong" it means "looks unintended", and every entry is a
question for the project, not a verdict.

**The files this register cites are gone (DEV-7496, 2026-10-10).** `raw/`, its CSVs, its README and `translate.py`
were deleted when the store builder replaced them ([`PROVENANCE.md`](./PROVENANCE.md)). The entries below stay as
written: every count was computed from the CSV named beside it, at the commit that added the entry, and "How to
reproduce" needs that commit's files. The findings are about the archive's data and hold for the store as well,
apart from the last section, which is about the change of pipeline itself.

**Sources.** The sanitised stage dump (`dasch-swiss/dsp-repository-design`,
`spikes/profile-as-data/dumps-stage/0803-incunabula.zip`, bagged 2026-07-19 from `api.stage.dasch.swiss`, DSP-API
v37.1.0, knora-base 52: `data/rdf/data.nq`, 312,747 quads), lifted through the DAO stage (`../../tools/dao-lift`) into
`raw/pages.csv`. **Every number below was recomputed from the committed `raw/pages.csv` with `csv.DictReader`**, not
copied from the plan. See "How to reproduce". Baseline: **4,024 Pages in 19 Books**, 4,024 distinct ids.

**Severity scale.** *blocks presentation*: a visitor would see something broken or misleading · *degrades
presentation*: visibly wrong or noisy but not broken · *cosmetic*: an inconsistency a reader may not notice · *model
hygiene*: invisible to a visitor, but matters for reuse, export and future editing.

---

## 1. `hasSeqnum` is not a clean 1..n in 3 of 19 books

16 of the 19 books number their Pages exactly `1..n`. The other three do not. CPE never presents the raw number as a
position. `pages.csv`'s `seq` is a **positional index** 1..n per book, built by sorting on `hasSeqnum` ascending and then
on the DSP IRI's local part ascending (plan decision 10). The archival number is kept verbatim in `seq_source` and stays
visible as data.

| # | Book | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| S1 | `walfart` | 247 Pages, `hasSeqnum` runs 1–248, **6 is missing**. `a2v` is seq 5, `a3v` is seq 7 | A gap in a sequence that is otherwise contiguous. `a3r` would be the expected leaf between them, but no Page carries it | model hygiene | Was a leaf never scanned, or deleted without renumbering? CPE: positional seq 1..247, so `?member=N` and "n / N" agree |
| S2 | `brandan` | 63 Pages, `hasSeqnum` runs 1–62, **two Pages at 1**: `[missing]` and `Einband, Vorderdeckel, aussen` | Two Pages claim the first position (see S4) | degrades presentation | Which page is first? CPE: the IRI-local tie-break puts `[missing]` at seq 1 and the front cover at seq 2 |
| S3 | `zeitgloecklein-1490` | 131 Pages, `hasSeqnum` runs 1–130, **two Pages at 1**: `a1r` and `[missing]` | Same shape as S2 | degrades presentation | Same question. CPE: the tie-break puts `a1r` at seq 1 and `[missing]` at seq 2 |

## 2. Duplicate sequence number 1: the 2 Pages

S2 and S3 are the same defect. In each book, a Page whose pagenum is the placeholder `[missing]` shares `hasSeqnum 1`
with the book's real first page.

| # | Page id | Book | `seq` | `seq_source` | `page_num` |
|---|---|---|---|---|---|
| S4a | `brandan-missing` | `brandan` | 1 | 1 | `[missing]` |
| S4b | `brandan-einband-vorderdeckel-aussen` | `brandan` | 2 | 1 | `Einband, Vorderdeckel, aussen` |
| S4c | `zeitgloecklein-1490-a1r` | `zeitgloecklein-1490` | 1 | 1 | `a1r` |
| S4d | `zeitgloecklein-1490-missing` | `zeitgloecklein-1490` | 2 | 1 | `[missing]` |

The tie order comes only from the DSP IRI local part, which is an opaque random id. The resulting order is
**deterministic** (it is stable under re-extraction) but **arbitrary**: in `brandan` the placeholder lands first, and
in `zeitgloecklein-1490` it lands second. Nothing in the data says which order is intended. **Moot in CPE since
2026-10-02:** both `[missing]` Pages are colour-calibration scans and are dropped (§3), so neither book has a tie left.

**Knock-on for Slice 1:** in **both** books, the `[missing]` Page is the one whose scan `books.csv` uses as the cover
(`first_scan_file`). The home shelf and the `/books` row thumbnail for these two books therefore show the scan the archive
itself could not number. *Degrades presentation.* Question: is that scan the real title page, and was only its pagenum
lost? **Since Slice 8** `first_scan_file` is derived as the scan of positional `seq` 1, so this now holds for `brandan`
only; `zeitgloecklein-1490`'s cover became its `a1r` scan (D8 below). **Resolved for the thumbnail (2026-09-29):**
the cover is now the curated `curation.csv` `cover_page` (`raw/README.md`, "Cover picks"), and `brandan`'s is its
`a1r` title page. `translate.py` refuses a `cover_page` naming a `[missing]` page. The `[missing]` page label itself
(P1) is gone: the Page is a calibration scan and is dropped (§3, owner checklist item 8).

## 3. Placeholder pagenum `[missing]`: 2 Pages

| # | Page id | Book | Evidence | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| P1 | `brandan-missing` | `brandan` | `hasPagenum` is the literal string `[missing]`; the scan is a colour-calibration shot of the front cover | degrades presentation | Mark calibration shots as such in the archive (CPE drops it) |
| P2 | `zeitgloecklein-1490-missing` | `zeitgloecklein-1490` | Same; a calibration shot of the `a1r` title page | degrades presentation | Same |

Both Pages are also the duplicate-seq-1 Pages of §2. **Curation decision (owner, 2026-10-02, owner checklist
item 8): calibration scans excluded.** Neither is a page of the book: each is a colour-calibration shot of a page that
has its own clean scan — `brandan-missing` is the front cover with a Kodak colour bar and ruler (the clean scan is
`brandan-einband-vorderdeckel-aussen`), `zeitgloecklein-1490-missing` is the `a1r` title page with a colour checker
(clean: `zeitgloecklein-1490-a1r`). Each shares its neighbour's `hasSeqnum` and carries no description. `translate.py`
drops both: no Page, no `compound_parts` row, no Image. The positional `seq` is renumbered 1..n over the kept Pages, and
the Book's `extent` is `books.csv` `page_count` minus the dropped scans (`brandan` 63 → 62, `zeitgloecklein-1490`
131 → 130), so the gloss and the reader's count agree. The run asserts exactly 2 such Pages and fails if one carried a
description, comment or strip link. Nothing is invented; the archive itself still holds both scans.

## 4. 408 Page ids needed a `-N` suffix

A Page's id is `<book-slug>-<slug(page_num)>` (plan decision 3). A second, third, … Page in the same book whose pagenum
slugs to the same string gets `-2`, `-3`, … in positional order. The full rule is in
`raw/README.md`. **408 of 4,024 ids carry a suffix**, spread over
**6 books**. The highest suffix is `-3`.

The planning measure put all 408 down to "repeated pagenums". **That is not what the data shows.** Only **18** come from
a pagenum that literally repeats. The other **390** come from **distinct** pagenums that the slug folds together.

| Cause | Ids | Books |
|---|---|---|
| **A. True repeat**: the same pagenum string occurs twice or more in the book | **18** | `quadragesimale` 14 · `postilla` 2 · `narrenschiff-lat-august` 1 · `reise` 1 |
| **B. Case-fold collision**: the book signs quires in both lower case and capitals (`a1v` … and `A1v` …), which are **different leaves** in incunabula collation, and the lowercasing slug merges them | **374** | `bereitung` 158 · `postilla` 125 · `zeitgloecklein-1492` 46 · `quadragesimale` 45 |
| **C. Punctuation-strip collision**: two quire signatures made of a symbol plus a leaf number, `&1r` … `&8v` and `?1r` … `?8v`, both slug to `1r` … `8v` | **16** | `quadragesimale` 16 |
| **Total** | **408** | 6 books |

**The 6 books** (all of them, ranked; no other book has a suffixed id):

| Book | Suffixed ids | A (repeat) | B (case) | C (punct.) |
|---|---|---|---|---|
| `bereitung` | 158 | 0 | 158 | 0 |
| `postilla` | 127 | 2 | 125 | 0 |
| `quadragesimale` | 75 | 14 | 45 | 16 |
| `zeitgloecklein-1492` | 46 | 0 | 46 | 0 |
| `narrenschiff-lat-august` | 1 | 1 | 0 | 0 |
| `reise` | 1 | 1 | 0 | 0 |

| # | What | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| I1 | The true repeats (cause A) | `quadragesimale`: `a1v`–`a7v` (13 pagenums) and `b6r`, each twice. `postilla`: `D5v (doppelt gescannt???)` three times, at seq 266–268. `narrenschiff-lat-august`: `r6v` twice. `reise`: `21v` twice | Two Pages cannot be the same leaf. Either the leaf was scanned twice (the `postilla` pagenum says so itself) or one pagenum is a typo | degrades presentation | Delete the duplicate scans, or correct the pagenum. `postilla` also has `B8v (doppelt gescannt???)` once, at seq 239: the pagenum doubts itself without having a twin |
| I2 | Case-fold collisions (cause B) | E.g. `bereitung-a1v` is `a1v`, but `bereitung-a1v-2` is `A1v`, a different leaf | **Not a data error.** Capital-letter quires are normal collation. The id scheme is what makes `A1v` read as a duplicate of `a1v` | cosmetic (CPE-side) | A CPE question, not a project one: does the owner want a case-preserving id scheme before the ids are relied on in URLs? (Journal entry "Decisions 3/5 were approved on a mistaken premise".) The reader's `?page=` lookup tries a case-SENSITIVE exact match first, so typing `A1v` still lands on `A1v` |
| I3 | `?`-signed quire in `quadragesimale` (cause C) | Pagenums `?1r` … `?8v` (seq 388–403) follow `&1r` … `&8v` (seq 372–387) | A `?` in a transcribed signature looks like a lost glyph (an encoding casualty of a special signature mark) rather than an authored character | cosmetic | What symbol does the print use for this quire? If it is a non-ASCII mark, restore it |

## 5. Other pagenum oddities (observed, not acted on)

| # | What | Evidence | Severity | Question |
|---|---|---|---|---|
| O1 | A colour target catalogued as a page | `itinerarius-peregrinarius-testbild-farben-und-massstab` (seq 3), pagenum `Testbild, Farben und Massstab` | degrades presentation | A scanning test target is not a leaf of the book. Should it be a Page at all? CPE: kept, per "never invent, never drop" |
| O2 | Descriptive pagenums | Binding and endpaper Pages carry prose pagenums (`Einband, Vorderdeckel, aussen`, `Einband, Spiegel, vorne`, `a1r, Titelblatt, recto`, `o4r; Nachsatzblatt`, `1 recto`). The longest page-local id part is 28 characters, and 90 % are 5 or fewer | cosmetic | None. These are legitimate. The slug rule handles them |

---

## How to reproduce

Every count above comes from the committed CSV alone. No dump is needed:

```python
import csv, collections as C
rows = list(csv.DictReader(open("raw/pages.csv", encoding="utf-8", newline="")))
by = C.defaultdict(list)
for r in rows:
    by[r["book"]].append(r)
# §1: books whose seq_source is not 1..n
for b, rs in sorted(by.items()):
    ss = sorted(int(r["seq_source"]) for r in rs)
    if ss != list(range(1, len(rs) + 1)):
        print(b, len(rs), ss[0], ss[-1])
# §3: [missing] pagenums
print([r["id"] for r in rows if r["page_num"] == "[missing]"])
# §4: suffixed ids = id != f"{book}-{slug(page_num)}", slug as in raw/README.md.
# Cause A if the exact page_num occurred earlier in the book. Otherwise it is
# B (earlier pagenum equal ignoring case) or C (differs beyond case).
```

Never use `wc -l` on `pages.csv`. 2,416 descriptions and 45 comments carry embedded newlines, so the file has 9,532
physical lines for 4,025 records (9,533 before Slice 5 removed the `\x1e` artefact from one comment, see B2 below).

---

## Slice 5: border strips (`Band`)

Compiled 2026-09-24 for Slice 5 (the border strips), from the committed `raw/bands.csv` (38 rows)
and the `strip_left`/`strip_right` columns of `raw/pages.csv`. Both come from the same dump and DAO
stage as above, through `cpe-export` (see `raw/README.md`). Every number was recomputed from
the committed CSVs with `csv.DictReader`. Baseline: **38 Bands**, 18 left (`l_…`), 19 right (`r_…`), 1 with no side;
**415** page→strip links on **208** Pages of `narrenschiff-dt`.

| # | What | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| B1 | The literal sentinel `[missing]` | One Band carries `[missing]` as its `rdfs:label`, its `hasTitle` **and** its `hasDescription`. A second Band, `r_10`, carries `[missing]` as its `hasDescription` | A placeholder string where a value should be, the same sentinel as the Page pagenums in §3 | degrades presentation | Supply the strip's signature and the two descriptions. CPE (plan decision D1): the export keeps `label`, `title` and `description` **verbatim**, sentinel included. The id-less Band gets the id **`strip-38`** instead of the slug `missing`, and `translate.py` names it `Randleiste 38` and emits no `description` and no `side` for it. `r_10`'s `[missing]` description is dropped the same way. The token is never displayed |
| B2 | A trailing `\x1e` record separator | **All 43** Band text values (the 38 `hasDescription`s and the 5 `hasComment`s) end in U+001E followed by a newline and eight spaces. So does **one Page comment**, on `narrenschiff-dt-g2v` (… `Randleiste l_3c ist verkehrt herum angebracht.`). 44 values in the DAO graph in total | A control character left over from an earlier export, not text anyone wrote | degrades presentation (the byte reached the reader page of `g2v` through Slice 2's `data.sql`) | Clean the values at the source. CPE: `cpe-export` strips the trailing U+001E **and the whitespace around it**, and nothing else. A value with no U+001E stays verbatim, trailing newline and all. The export then fails the run if any cell still contains the byte. For `pages.csv` this changed exactly one cell of the Slice-2 columns, the `g2v` comment |
| B3 | One page with a right strip and no left one | `narrenschiff-dt-l8r` (seq 177) has `hasBandTypeR` `r_3b` and no `hasBandTypeL`. The other 207 linking Pages have both. Its own comment says why: `die linke Randleiste ist nicht zu erkennen` | Not an error. The left strip cannot be read off the scan | cosmetic | None. CPE: `strip_left` is empty for this Page, and no link row is invented |
| B4 | One strip no Page uses | `strip-38` (the `[missing]` Band of B1) is the target of **zero** `hasBandTypeL`/`hasBandTypeR` links. The other 37 are used 1–23 times | A catalogued strip that no page cites. With no signature it may never have been identified on a page | model hygiene | Which strip is it, and where is it printed? CPE: kept (it has a real scan). Its detail page shows the empty state, which is real data, not an invented one |
| B5 | A comment that reads like the missing description | `r_10`'s description is `[missing]` (B1), and its one comment is `Randleiste mit fünf Narren, 2.2 x 16.6 cm.`. That has the shape of every other strip's description (motif, then size) | The description looks as if it was entered in the wrong field | degrades presentation | Move it into `hasDescription` at the source? CPE: **deliberately not promoted**. The export does not move text between fields, so `r_10` has no description and keeps the text as its comment |
| B6 | A duplicated comment | `l_3b` carries two `hasComment` values with the same text (`Die Randleist ist fast gleich wie l_3a, aber zur Hälfte nach unten versetzt.`) | The same note entered twice | cosmetic | Delete one. CPE: both kept, joined with ` \| ` like any multi-value comment |

Observed and consistent, nothing to fix: no strip is used on both sides, so `side` follows from the label prefix alone;
each Page has at most one strip per side; the 208 Pages form **89** distinct (left, right) pairs; the 38 strip scans
share no file with the 4,024 Page scans.

How to reproduce the Slice-5 counts:

```python
import csv, collections as C
bands = list(csv.DictReader(open("raw/bands.csv", encoding="utf-8", newline="")))
pages = list(csv.DictReader(open("raw/pages.csv", encoding="utf-8", newline="")))
print(len(bands), C.Counter(b["side"] for b in bands))              # 38, 19 right / 18 left / 1 empty
print([b["id"] for b in bands if b["description"] == "[missing]"])  # B1: r-10, strip-38
print([p["id"] for p in pages if p["strip_right"] and not p["strip_left"]])  # B3: narrenschiff-dt-l8r
use = C.Counter([p["strip_left"] for p in pages if p["strip_left"]]
                + [p["strip_right"] for p in pages if p["strip_right"]])
print(sum(use.values()), sorted({b["id"] for b in bands} - set(use)))  # 415, B4: ['strip-38']
```

---

## Slice 8: `books.csv` regenerated from the DAO

Compiled 2026-09-24 for Slice 8 (plan step 10, decision 10). Until Slice 8, `raw/books.csv` was a one-off hand
derivation of two design artefacts, `DESIGN_ASSETS.md` §2 and the mockup's `incunabula-content.js`. It is now a
mechanical projection of the same DAO graph as `pages.csv` and `bands.csv`, written by the same `cpe-export` run (see
`raw/README.md`). The three authored columns (`id`, `office`,
`date_display`) moved to the hand-authored `raw/curation.csv`, joined on `dsp_id`, copied
verbatim from the old file. Every number below was recomputed with `csv.DictReader` from the committed pre-Slice-8
`books.csv` (`git show <pre-Slice-8 commit>:cpe/projects/incunabula/raw/books.csv`) and the regenerated one, matched
book by book through `curation.csv`.

**The rule.** The DAO wins on every dump-derivable column; nothing in the regenerated file was edited to match the old
one. The one place an authored value still beats the DAO is the display title of the four disambiguated editions: that
goes through `curation.csv`'s `title_override` column, never through `books.csv`.

**Result: 66 field differences over 9 of the 14 shared columns.** `page_count`, `date_source`, `date_iso_start`,
`date_iso_end` and `date_precision` agree for all 19 books, and so do 15 of 19 `title`s, 18 of 19 `author`s, `printer`s,
`place`s and `first_scan_file`s. The disagreements split by where the old value came from (`raw/README.md`'s
pre-Slice-8 attribution table):

- **`DESIGN_ASSETS.md`-sourced columns agree**, apart from two formatting choices of that document: the bracketed
  edition suffix on 4 titles (D1), and the `, ` join of a multi-valued author or printer (D3).
- **`incunabula-content.js`-sourced columns mostly do not.** `shelfmark` differs for **18 of 19**, `catalogue_uri` for
  **19 of 19**, `physical` for **15 of 19**. The old values follow patterns the dump does not have: every old
  shelfmark is a `Basel UB, Sign: …` one, every old URI is `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=0022…`,
  and all 19 old physical descriptions are one template, `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: N°`
  (the DAO really carries that template for 4 books, which is why those 4 agree). They look like mockup placeholders,
  the preview-artefact kind `DESIGN.md` §9 warns about, not an older state of the catalogue. The DAO's values replace
  them outright.

| # | What | Evidence | Which wins | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| D1 | The bracketed edition suffixes are not in the dump | The two *Zeitglöcklein* and the two Latin *Narrenschiff* editions share one `rdfs:label` per pair. `[1490]`, `[1492]`, `[Aug. 1497]`, `[März 1497]` were added by `DESIGN_ASSETS.md` to tell them apart | DAO in `books.csv` (`title` is the bare label). The suffixed forms are kept, verbatim, as `curation.csv` `title_override` for exactly these 4 books, so the displayed name does not change | degrades presentation (two pairs of identically named books, without the override) | Should the archive itself carry distinguishing titles? CPE: authored override, decision 10 |
| D2 | Alternative titles | 6 books carry more than one `hasTitle`. The DAO keeps every title that is not the label; the old file had only `Stultifera navis`, twice, and without the `(...)` the dump writes | DAO | cosmetic | None. `alt_title` now holds every non-label `hasTitle`, ordered by text; `reise` has two, joined with ` \| ` |
| D3 | Multi-valued `author` / `printer` | `passio-meynrhadi` has two `hasCreator` values and `bereitung` two `hasPublisher` values. The old file joined them with `, `, which is ambiguous: single values contain commas too (`Birgitta, Hl.`, `Berthold, der Bruder`) | DAO values, joined with ` \| ` (the `comment` separator; the export asserts no value contains it), **ordered by text** because the DAO keeps no value order. So `passio-meynrhadi`'s authors swap order | cosmetic | Is the order of the two authors meaningful? The dump cannot say |
| D4 | The sentinel `[missing]` as a place | `itinerarius-peregrinatio`'s `hasPubloc` is the literal `[missing]`. The old file said `Basel` for all 19 | DAO, verbatim in `books.csv`. **`translate.py` treats the literal `[missing]` place as ABSENT** (orchestrator decision, Slice 8 step 11): no `pub-place` row is emitted, because the placeholder is not a place. So the book page shows no place rather than `[missing]` or the old invented `Basel` | degrades presentation | Supply the place, if known. The shelfmark (`FP VII2 7:1`) sits with the other Basel UB copies |
| D5 | Shelfmarks | 18 of 19 differ (only `narrenschiff-dt` agrees). 5 books are not Basel UB copies at all (Köln: `orationes`, `zeitgloecklein-1492`, `lob-der-glieder`; Bamberg: `brandan`; Ulm: `methodij`) | DAO | degrades presentation (the old values were invented) | None |
| D6 | Catalogue links | 19 of 19 differ. The DAO's are `http://` (not `https://`): 14 are Basel `aleph` links with `local_base=DSV01`/`DSV05`, 3 point at the same Köln record, 1 is an `nbn-resolving` URN. `methodij` has **no** `hasUri` (the old file invented one), `itinerarius-peregrinatio` has one (the old file had none) | DAO, except the 3 Köln links: overridden in CPE, **temporarily** (see the note below this table) | degrades presentation (the old links were invented, and may resolve to other books) | Checked 2026-10-05: the Köln link is dead (see the note); the 14 Basel `aleph.unibas.ch` links still work, but only through a redirect to a swisscovery search; the Bamberg `nbn-resolving` link could not be verified by script. Fix the Köln link in DSP, then delete the override |
| D7 | Physical descriptions | 15 of 19 differ. The old file's 15 templated strings are replaced by the real, heterogeneous catalogue text (`[12] Bl. : Ill. ; 8°`, `1 Schachtel (8 einzeln gebundene Lagen) …`). `lob-der-glieder`'s value ends in a **trailing space**, kept verbatim | DAO | degrades presentation (the old values were invented) | Normalise the catalogue text at the source? CPE: verbatim |
| D8 | The cover scan of `zeitgloecklein-1490` | `first_scan_file` is now **the scan of the Page at positional `seq` 1** (§1's order). For 18 books that is the old value. For `zeitgloecklein-1490` it is not: the old cover was the `[missing]` Page at `seq` 2 (§2, S4d), the new one is `a1r` at `seq` 1 (`2s0yglODk6V-F4amCTIXPMk`) | DAO rule. It also resolves half of §2's knock-on: this book's cover is no longer the unnumbered scan. `brandan`'s still is (its `[missing]` Page is `seq` 1) | cosmetic (a different cover thumbnail) | Which scan is the intended cover, if any? The dump has no cover concept, so any rule is a CPE choice |
| D9 | Three books share one shelfmark and one link | `orationes`, `zeitgloecklein-1492` and `lob-der-glieder` all carry `Universitäts- und Stadtbibliothek Köln, Sign: AD+S167` and the same `ub.uni-koeln.de` record. One of them notes it is bound with two further Amerbach prints | Not a disagreement: the old file was wrong for all three | cosmetic | **Answered (owner, 2026-10-05):** one bound volume (Sammelband), catalogued as the *Zeitglöcklein*; *Lob der Glieder* starts on page 431. The shared shelfmark and record are correct; only the URL is stale (D6) |

**The Köln catalogue link: a temporary CPE-only override (2026-10-05).** The DAO's link for the three Köln books,
`http://www.ub.uni-koeln.de/cdm/compoundobject/collection/inkunabeln/id/1878/rec/1`, is dead: it redirects to
`ub.uni-koeln.de/sammlungen-und-schwerpunkte/digitalisat-nicht-aufrufbar`. Until it is fixed at the source, in DSP,
`raw/catalogue-overrides.csv` replaces it, for all three books, with the library's live
catalogue record of the volume,
`https://search.ub.uni-koeln.de/portal/inkunabeln/databases/id/inkunabeln/titles/id/cdm_inkunabeln_1878.html?l=de`
(owner decision). The digitised copy is at `https://doi.org/10.58016/retro_inc-1878_026106`; it is not used as the link.
This is **not** the fix. `books.csv` still carries the DAO value, and `translate.py` aborts as soon as it no longer
does, asking for the override row to be deleted. Before go-live: fix the link in DSP and delete the override, or keep
it consciously (`../../docs/INCUNABULA_GO_LIVE.md`).
*(From DEV-7496 the override was the constant `CATALOGUE_OVERRIDES` in the store builder, with the same check: the
build refuses the Book as soon as the archive no longer has the dead address. The check is a rule of the project's
hook, `store-build/src/projects/incunabula/catalogue.rs`. Since DEV-7497 (2026-10-11) the two
addresses are curated values of the three Books, `catalogue_uri` and `catalogue_uri_replaces` in the monorepo's
`0803-curation.csv`; the hook rule reads them and keeps the check.)*

Observed and consistent: `dsp_id` joins 19 of 19 Books to 19 of 19 `curation.csv` rows (asserted by `cpe-export`); the
old `books.csv` titles of the 15 other books equal the DAO labels exactly; `page_count` equals the DAO's Page count for
all 19 (the export now derives it, so this can no longer drift).

### Every field-level difference

`\|` is the ` | ` multi-value separator. A trailing `␠` marks a real trailing space in the value.

#### `title` — 4 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `narrenschiff-lat-august` | `[Das] Narrenschiff (lat.) [Aug. 1497]` | `[Das] Narrenschiff (lat.)` |
| `narrenschiff-lat-maerz` | `[Das] Narrenschiff (lat.) [März 1497]` | `[Das] Narrenschiff (lat.)` |
| `zeitgloecklein-1490` | `Zeitglöcklein des Lebens und Leidens Christi [1490]` | `Zeitglöcklein des Lebens und Leidens Christi` |
| `zeitgloecklein-1492` | `Zeitglöcklein des Lebens und Leidens Christi [1492]` | `Zeitglöcklein des Lebens und Leidens Christi` |

#### `alt_title` — 6 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `de-generatione` | *(empty)* | `Defensorium inviolate perpetueque virginitatis castissime de genitricis Mariae` |
| `narrenschiff-lat-august` | `Stultifera navis` | `Stultifera navis (...)` |
| `narrenschiff-lat-maerz` | `Stultifera navis` | `Stultifera navis (...)` |
| `orationes` | *(empty)* | `Sant Brigitten gebettly` |
| `reise` | *(empty)* | `Itinerarius \| Reysen und wanderschafften durch das Gelobte Land` |
| `zeitgloecklein-1490` | *(empty)* | `Horologium devotionis circa vitam Christi` |

#### `author` — 1 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `passio-meynrhadi` | `Meynradus Sanctus, Albrecht von Bonstetten` | `Albrecht von Bonstetten \| Meynradus Sanctus` |

#### `printer` — 1 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `bereitung` | `Johann Amerbach, Johann Petri de Langendorff` | `Johann Amerbach \| Johann Petri de Langendorff` |

#### `place` — 1 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `itinerarius-peregrinatio` | `Basel` | `[missing]` |

#### `shelfmark` — 18 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `bereitung` | `Basel UB, Sign: FP VI 6` | `Basel UB, Sign: KiAr A VII 1` |
| `brandan` | `Basel UB, Sign: FP VII 15` | `Bamberg SB, Sign: Inc.typ.Jc.II.7` |
| `de-generatione` | `Basel UB, Sign: FN VI 12` | `Basel UB, Sign: FP VII2 5:4` |
| `itinerarius-peregrinarius` | `Basel UB, Sign: FP VII 21` | `Basel UB, Sign: FP VII2 4:1` |
| `itinerarius-peregrinatio` | `Basel UB, Sign: FP VII 22` | `Basel UB, Sign: FP VII2 7:1` |
| `lob-der-glieder` | `Basel UB, Sign: FP VI 12:4` | `Universitäts- und Stadtbibliothek Köln, Sign: AD+S167` |
| `melusine` | `Basel UB, Sign: Aa I 12` | `Basel UB, Sign: O I 18` |
| `methodij` | `Basel UB, Sign: FP VII 3` | `Stadtbibliothek Ulm, Sign: vBB593` |
| `narrenschiff-lat-august` | `Basel UB, Sign: Ai II 22b:2` | `Basel UB, Sign: DA III 1` |
| `narrenschiff-lat-maerz` | `Basel UB, Sign: Ai II 22a` | `Basel UB, Sign: DA III 2` |
| `orationes` | `Basel UB, Sign: FP VI 12:3` | `Universitäts- und Stadtbibliothek Köln, Sign: AD+S167` |
| `passio-meynrhadi` | `Basel UB, Sign: FP VII 8` | `Basel UB, Sign: FN VI 4e` |
| `postilla` | `Basel UB, Sign: FL III 5` | `Basel UB, Sign: Aleph A VI 3:1` |
| `quadragesimale` | `Basel UB, Sign: FN VI 4` | `Basel UB, Sign: Aleph E XI 24a` |
| `reise` | `Basel UB, Sign: EU II 9` | `Basel UB, Sign: Aleph D III 13:1` |
| `walfart` | `Basel UB, Sign: FP VII 20` | `Basel UB, Sign: FP VIII2 1` |
| `zeitgloecklein-1490` | `Basel UB, Sign: FP VI 12:1` | `Basel UB, Sign: Aleph E X 18:2` |
| `zeitgloecklein-1492` | `Basel UB, Sign: FP VI 12:2` | `Universitäts- und Stadtbibliothek Köln, Sign: AD+S167` |

#### `catalogue_uri` — 19 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `bereitung` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002237441` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002647120` |
| `brandan` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002239012` | `http://www.nbn-resolving.de/urn:nbn:de:bvb:22-dtl-0000003140` |
| `de-generatione` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002236601` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002237546` |
| `itinerarius-peregrinarius` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002237990` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002237638` |
| `itinerarius-peregrinatio` | *(empty)* | `http://aleph.unibas.ch/F/?local_base=DSV05&con_lng=GER&func=find-b&find_code=SYS&request=000084910` |
| `lob-der-glieder` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002239669` | `http://www.ub.uni-koeln.de/cdm/compoundobject/collection/inkunabeln/id/1878/rec/1` |
| `melusine` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002234166` | `http://aleph.unibas.ch/F/?local_base=DSV05&con_lng=GER&func=find-b&find_code=SYS&request=000084198` |
| `methodij` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002244501` | *(empty)* |
| `narrenschiff-dt` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002240871` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002755521` |
| `narrenschiff-lat-august` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002242332` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002758921` |
| `narrenschiff-lat-maerz` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002242331` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002758751` |
| `orationes` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002239668` | `http://www.ub.uni-koeln.de/cdm/compoundobject/collection/inkunabeln/id/1878/rec/1` |
| `passio-meynrhadi` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002241502` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002715545` |
| `postilla` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002239214` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002709195` |
| `quadragesimale` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002241001` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002712355` |
| `reise` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002244103` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002610320` |
| `walfart` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002238001` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002237584` |
| `zeitgloecklein-1490` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002238461` | `http://aleph.unibas.ch/F/?local_base=DSV01&con_lng=GER&func=find-b&find_code=SYS&request=002645085` |
| `zeitgloecklein-1492` | `https://aleph.unibas.ch/F?func=find-b&find_code=SYS&request=002239670` | `http://www.ub.uni-koeln.de/cdm/compoundobject/collection/inkunabeln/id/1878/rec/1` |

Note: `lob-der-glieder`, `orationes` and `zeitgloecklein-1492` — three distinct Books, all Köln copies — carry the
SAME DAO `catalogue_uri` (`.../id/1878/rec/1`). This is an archive-side data observation, not a translation bug,
and it is correct: the three prints are bound in one volume (D9, answered 2026-10-05). The URL itself is dead, so CPE
overrides it temporarily (D6 and the note below the D-table); `books.csv` keeps the DAO value verbatim per decision 10.

#### `physical` — 15 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `bereitung` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 8°` |
| `brandan` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: 30 Bl.; Dimension: 4°` |
| `de-generatione` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: [30] Bl.; Other physical details: Ill. (53 Holzschnitte); Dimensions: 23 cm (4°)` |
| `itinerarius-peregrinarius` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: 108 Bl.; Ill. (59 Holzschnitte); Dimensions: 4°` |
| `itinerarius-peregrinatio` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Bl. 1-40 eines Sammelbandes` |
| `lob-der-glieder` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 8°` | `[12] Bl. : Ill. ; 8° `␠ |
| `melusine` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 2°` | `1 Schachtel (8 einzeln gebundene Lagen) : mit Buchschmuck/Illustration ; 28,5 x 20,5 cm` |
| `methodij` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: [33] Bl.; Dimension: 16°` |
| `narrenschiff-lat-august` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: CLIX, [1] Bl.; Other physical details: Ill.; Dimensions: 23 cm (4°)` |
| `narrenschiff-lat-maerz` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: CXXXXV, [3] Bl.; Other physical details: Halbband weiss Blindprägung, Ill., Holzschnitte unkoloriert, vereinzelt mit Tinte beschmiert; Dimensions: 23 cm (4°)` |
| `orationes` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 8°` | `[12] Bl. : Ill. ; 8°` |
| `quadragesimale` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 8°` |
| `reise` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 2°` | `Extent: 1 Bd.; Dimensions: f°` |
| `walfart` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 4°` | `Extent: 124 Bl.; Other physical details: Ill. (58 Holzschnitte); Dimensions: 8°` |
| `zeitgloecklein-1492` | `Extent: 1 Bd.; Other physical details: Ill.; Dimensions: 8°` | `Dimension: 8°` |

#### `first_scan_file` — 1 of 19 differ

| Book | Committed (pre-Slice-8) | DAO (regenerated) |
|---|---|---|
| `zeitgloecklein-1490` | `JCpROM5rUD4-DTtJ4vmAMya` | `2s0yglODk6V-F4amCTIXPMk` |


How to reproduce:

```python
import csv
rd = lambda p: list(csv.DictReader(open(p, encoding="utf-8", newline="")))
old = {r["id"]: r for r in rd("books.pre-slice-8.csv")}  # git show <pre-Slice-8 commit>:cpe/projects/incunabula/raw/books.csv
cur = {r["dsp_id"]: r["id"] for r in rd("raw/curation.csv")}
new = {cur[r["dsp_id"]]: r for r in rd("raw/books.csv")}
diffs = [(c, b, old[b][c], new[b][c]) for b in new for c in new[b] if c != "dsp_id" and old[b][c] != new[b][c]]
print(len(diffs))  # 66
```

---

## Slice 6: annotations (`Region`, `LinkObj`)

Compiled 2026-10-02 for Slice 6 (plan Phase 2), from the committed
`raw/annotations.csv` (77 rows),
`raw/annotation_links.csv` (40) and
`raw/annotation_link_targets.csv` (79). All three come from the same dump and
DAO stage as above, through `cpe-export`, verbatim (see
`raw/README.md`). Every number was recomputed from the
committed CSVs with `csv.DictReader`. Baseline: **77** annotations (52 rectangle, 24 polygon, 1 circle) on **72**
Pages, **40** links with **79** targets (64 annotation, 6 book, 9 page). The export keeps every row below; what
reaches the book page is decided by Phase 3's curation layer (plan D6–D8).

| # | What | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| A1 | A colour that disagrees with its outline | `EDqDFUAsXtSuBreIXyuCiQ` on `narrenschiff-dt-b3v`: `hasColor` is `#3333ff`, the geometry JSON's `lineColor` is `#ff3333`. The other 76 agree | Two fields for one colour, set apart | degrades presentation | Which did the researcher mean? CPE (plan D6): `hasColor` wins; `color` is `#3333ff` and `lineColor` is never read |
| A2 | A rectangle stored bottom-right first | `H-OnOBzuVgi6MlBo2V60TQ` on `bereitung-o6r`: its first point is (0.552, 0.444), its second (0.279, 0.500), so `x` runs right to left. The other 51 rectangles start top-left | Corner order the drawing tool should have normalised | cosmetic | None at the source. CPE: the CSV keeps the points verbatim; `translate.py` takes the min and max per axis (plan Phase 3) |
| A3 | Trailing whitespace without the `\x1e` artefact | 6 remarks end in a space or a newline (`reise-46r`, `bereitung-q4v`, `reise-68r`, `brandan-a4v`, `narrenschiff-dt-b7v`, `narrenschiff-lat-august-c6r`) and so does 1 link comment (`ylNmIB2eWUOLl3MtTZ5dYw`). 3 more remarks ended in `\x1e` plus whitespace (B2's artefact), which the export strips | Not text anyone meant, but the export cannot tell it from authored whitespace | cosmetic | Trim at the source. CPE: the export strips only the `\x1e` form (as B2) and leaves these verbatim; Phase 3 trims every remark and comment |
| A4 | Links with one target | 4 LinkObjs link a single target: `E1BDVABJXzS2omxCRrvgNw` and `QkahE-G7WSmGm3CTibPOhw` (both the same Region on `narrenschiff-lat-august-c7r`), `HY_mz7JFWOiHZB90Vq7sZg` (the b3v Region of A1) and `K7SBwA60Uwetq9pjMzvEWw` (the Page `narrenschiff-dt-f4v`). Two of the comments name partner pages the link does not carry (`… auf den Seiten b2v … und c2r …`) | A link object is a relation; one target relates nothing | model hygiene | Add the missing targets? CPE: kept in the CSVs (a link needs ≥ 1 target, plan D3); a one-target link gives no partner line on the book page (plan D8) |
| A5 | Duplicate links | `narrenschiff-dt-c5v` ↔ `-u1r` are joined by **4** LinkObjs (3 with the identical two Regions), `narrenschiff-lat-august-f6r` ↔ `-i8r` by **2**, and the two one-target links of A4 share their one Region | The same relation entered more than once | cosmetic | Merge at the source. CPE: all kept; the book page deduplicates partners by page (plan D8). The plan's facts table also lists c5r↔b6v and d1r↔c3v; each is two links joining the German page to the *two different* Latin editions (`narrenschiff-dt-b6v` to `narrenschiff-lat-august-c5r` and to `narrenschiff-lat-maerz-c5r`; `narrenschiff-dt-c3v` to both editions' `d1r`), so they are not duplicates |
| A6 | Test artefacts | 13 Regions whose remark is a test string (`TEST`, `TEST2`, `DEMOX`, `DEMOY`, `DEMO BERN`, `DEMO UNI ZH`, `SBFI`, `SBFI2`, `BLABLA`, `DIe ist ein Symbol für BLABLA`, `asdsad`, `ghjghjghj`, `CADWORK`) and 5 LinkObjs (`DEMO_`, `TEST3`, `DEMOZ`, `SBFI 3`, `Verkündigung, Test Lothar`) | Left behind by demos and tests on the stage server | degrades presentation | Delete them at the source. CPE (plan D7): exported verbatim; Phase 3's `annotation-curation.csv` drops them, and with them every link left without a target |
| A7 | A wrong shape count in our own docs | [`SHAPE.md`](./SHAPE.md) §7 said "48 polygon and 2 circle geometries". The dump has 24 polygons and 1 circle, beside 52 rectangles | A documentation error, not a data one | cosmetic | CPE: the export asserts 52 / 24 / 1. `SHAPE.md` §7 and `CURATION.md` §6 now give these counts (Slice 6) |

How to reproduce the Slice-6 counts:

```python
import csv, json, collections as C
rd = lambda p: list(csv.DictReader(open(p, encoding="utf-8", newline="")))
ann, links, tg = rd("raw/annotations.csv"), rd("raw/annotation_links.csv"), rd("raw/annotation_link_targets.csv")
geo = {a["id"]: json.loads(a["geometry"]) for a in ann}
print(len(ann), C.Counter(g["type"] for g in geo.values()))                    # 77, 52 / 24 / 1
print([a["page"] for a in ann if geo[a["id"]]["lineColor"] != a["color"]])     # A1: narrenschiff-dt-b3v
print([a["page"] for a in ann if geo[a["id"]]["type"] == "rectangle"
       and geo[a["id"]]["points"][0]["x"] > geo[a["id"]]["points"][1]["x"]])  # A2: bereitung-o6r
print(sum(a["remark"] != a["remark"].strip() for a in ann),
      sum(l["comment"] != l["comment"].strip() for l in links))                # A3: 6, 1
per = C.Counter(t["link"] for t in tg)
print(len(links), len(tg), C.Counter(per.values()))                             # 40, 79, {2: 34, 1: 4, 3: 1, 4: 1}
```

---

## Slice 7: the woodcut showcase (`/woodcuts`)

Found while building the `/woodcuts` crop cases (plan D16, Q7), from the committed `data.sql`. CPE changes nothing in
the data for any of them; each case shows what the store holds.

| # | What | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| S1 | A remark that names its own page | `IbLaQ8JHU_WH1Mx2BWqr4Q` on `narrenschiff-lat-august-c5r` reads "Derselbe Holzschnitt wird auf Seite c5r der lateinischen Ausgabe des Narrenschiffs verwendet." | It sits on Aug. c5r; the woodblock's other impression is b6v of the German edition | cosmetic | Correct the remark to b6v at the source. CPE: the remark is not shown on `/woodcuts` (E3 has an authored remark); the reader's register shows it verbatim |
| S2 | Three impressions joined by no link | E6, Kap. 12: `narrenschiff-dt-b8v`, `narrenschiff-lat-august-c7r` and `narrenschiff-lat-maerz-c7r` (Schramm 1123). Aug. c7r carries two one-target links (`E1BDVABJXzS2omxCRrvgNw`, `QkahE-G7WSmGm3CTibPOhw`, A4); only the three Regions' remarks name each other | The same woodblock, three times, with no LinkObj relating any two of them | model hygiene | Add the links at the source. CPE: E6 is configured by its three crops, with an authored remark saying it is not joined in the data |
| S3 | A link comment that names another page (observed, not checked against the scan) | `HY_mz7JFWOiHZB90Vq7sZg`, the one-target link on `narrenschiff-dt-b3v`, reads "… auf den Seiten b2v (Das Narrenschiff (dt.)) und c2r …" | It sits on b3v, and b3v ↔ März c2r is E5's pair | cosmetic | Check the scan; correct to b3v at the source if so |
| S4 | A woodcut line run into another line, or a woodcut page whose description names no subject | `narrenschiff-dt-b3v` (E5): its description does carry the woodcut line, but run into the end of the note line, with no line break: "Im Bild handschriftliche Notiz "peter schellenbeg" und Kritzeleien Holzschnitt zu Kap. 7: Vom Zwietracht stiften." (its März partner c2r has "Holzschnitt zu Kap. 7: Vom Zwietracht stiften." on its own line). `bereitung-q4v`, `brandan-a6v`, `quadragesimale-f6v` and `-g6v` give only "Holzschnitt:" or "Holzschnitt." with no short subject: q4v's next line is the size, the other three's a long scene description | A page with a marked woodcut whose description does not set the woodcut out as a line of its own | cosmetic | Break the b3v line, and add short subjects, at the source. CPE: Slice 7's config reads b3v's merged line as no woodcut line, so its crop gets the composed alt ("Holzschnitt, {book}, {page}") and E5 no title (a case's title comes from its first crop's page), like the four pages without a short subject. The boot check would accept the merged text as an alt (it is part of one line), so using it is a config change, not a data one |

---

## Gregorian calendar flag on year-only pre-1582 dates

| # | What | Evidence | Why it looks wrong | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| C1 | Year-only dates carrying calendar GREGORIAN before 1582 | `itinerarius-peregrinarius` (1489) and `lob-der-glieder` (1492) store `calendar GREGORIAN` on their date value | The Gregorian calendar dates from 1582, so a 1489 or 1492 date cannot have been written in it; almost certainly a cataloguing default | cosmetic | Set the calendar to Julian, or leave it unset, at the source. CPE: changes nothing in the data; it does not show the calendar word at year precision (engine rule `component::values::calendar_word`, from commit 4007f27) |

## The store builder: two named differences from the last Python-written store (DEV-7496)

On 2026-10-10 the store builder's output replaced the `data.sql` that `translate.py` had written. The two stores
are equal in all ten tables apart from two shapes of difference, both in `properties` and both the archive's own
data showing through where the old pipeline had changed it. They are deliberate (owner decisions of 2026-10-09,
`dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md`) and closed: the ADR names each
class and property and records the counts ("Comparison tool removed"), which this register does not repeat.

| # | What | Where | Reason | Severity | Suggested fix / question |
|---|---|---|---|---|---|
| N1 | The order of one resource's values of one property | Page `comment`, Book `alt-title`, Book `creator` | The old export sorted a resource's values as text. The port serves them in the order their author gave them in DSP (`valueHasOrder`), and the builder keeps it: that order is the archive's and a text sort discards it | model hygiene; on the site, the order of two Books' alternative titles and creators (`reise`, `passio-meynrhadi`). No page shows a Page's `comment` | None. If an authored order is wrong, it is corrected in DSP |
| N2 | A text ends in whitespace | BorderStrip `description`, BorderStrip `comment`, Page `comment`, Book `physical` | The archive's text ends in whitespace (a line break and spaces after a strip's text; one trailing space after `lob-der-glieder`'s physical description). The old pipeline trimmed it; the builder stores a text as the port serves it and does not alter archive text | model hygiene: Incunabula has no richtext value and no stylesheet preserves whitespace, so nothing is visible | The project may want to trim these values in DSP |

