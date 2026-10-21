# cpe-store-build

Builds a project's CPE store from the `cpe-ports` snapshot of the project's DSP project, from the project's curated
values and from its `remodel {}` block, with the project's hook where the block declares one. It is a library: it has no binary and
writes no file. Incunabula (project 0803) is the one project that has a block.

`build(&ProjectSnapshot, &CuratedValues, &Remodel, Option<&Hook>) -> Result<Store, BuildErrors>` is pure, and so are
the SQL writers of `sql` and the curation file's reader, `CuratedValues::parse`: the library reads no file, no clock
and no environment. Its caller, `cpe-server`'s activation, asks the bytes port for the project's `config.kdl`, lowers
its `remodel` block through `cpe_core::validate::load_remodel`, asks the bytes port for `curation.csv` and the
archive projection port for the snapshot of the block's shortcode, reads the curation against the snapshot with
`CuratedValues::parse` (`CuratedValues::none` when the project deposited none and its block reads no curated key),
resolves the hook by the project's folder name with `hooks::resolve`, calls `build`, and renders the store as SQL
text with `sql::to_sql` under the header `sql::header` gives. `cpe-store` opens that text over its schema
(`../store/schema.sql`, which this crate includes as `SCHEMA_SQL`). What the store holds that the port does not serve
is declared in the project's block (the shortcode, the sentinel, the list vocabularies, the image rights and the IIIF
base, the curated keys, the classes annotations may touch) or is a rule of its hook.

`build` checks the port's contract first and returns its violations alone. Every other refusal is collected; each
has a `Rule` in `src/error.rs` and a case in `tests/it/refusals.rs`, `tests/it/hookless.rs` or `tests/it/hooks.rs`.
A resource that is refused takes no part in the rules after its own, so a refusal that needs two resources may show
only once the first is mended.

The writer refuses an empty table where the project's block is present and writes no statement for a table whose
block is absent: a project with no `list` has no list tables, one with no `member-of` no `compound_parts`.
`resources` and `properties` are never empty.

Below its header, the rendered text for Incunabula is, byte for byte, the `data.sql` the prototype in
`dsp-incubator` embeds. That file is not in this repository; `cpe-server`'s test
`incunabulas_store_text_below_its_header_is_the_prototypes` holds the text after the first blank line to that file's
length and SHA-256, and `incunabulas_whole_store_text_is_pinned` holds the whole text, header included (ADR-0007,
amendment of 2026-10-21). The header names both ports, and `sql::header` still writes the prototype's regenerate
command (`just cpe store-build <project> --inplace`), which does not exist here. Changing the header changes the
whole text's two constants; changing the layout changes all four.

## How a build runs

A rule is one of three kinds (the column "Bucket" of the table below):

- **The mapping** (`M`): what a node of the block says. `cpe-core` lowers the block into a typed `Remodel`
  (`cpe_core::remodel`), and the builder's code applies it: ids (`src/mapping/ids.rs`), the value steps of a
  `property` node (`src/mapping/values.rs`), the label (`src/mapping/label.rs`), the `curated` nodes
  (`src/mapping/curated.rs`), and in `src/generic/` the lists, membership, images, counts and annotations. The
  grammar is in [`docs/src/cpe/config-language.md`](../../../../docs/src/cpe/config-language.md), "`remodel` block".
- **Generic code** (`G`): what every project gets whatever its block says, such as slugs, dates, an annotation's
  geometry, the row order and the file layout. It lives in the same places. All of `src/mapping/`, `src/generic/`
  and `src/finalise.rs` name no project; `tests/it/hygiene.rs` searches for it.
- **The hook** (`H`): the rules of one project that the block cannot say. A hook is a module,
  `src/projects/<project>/`, and one row of `HOOKS` in `src/hooks.rs`, a plain `fn` pointer. The block's `hook` node
  declares the curated keys the hook reads and the properties it provides, overrides and drops; the builder refuses a
  `hook` node without a row (`hook-not-registered`), a row without a `hook` node (`hook-not-declared`), and any read
  or change beyond the declaration (`hook-exceeds-declaration`).

A build runs these steps, and a refused resource leaves the next ones:

1. the port's contract, checked against the block's shortcode; then U+0000 is refused in every text that can reach
   the store (`interior-nul`): here the snapshot's texts and the mapping's own (its names, labels, languages and the
   images' address and rights), and in step 4 each text a hook provides or overrides, where the hook gives it;
2. the hook against the block: a declaration without a hook, and a hook without a declaration;
3. the mapping: lists, classes, the curated keys, membership, files, ids, the values of every `property` and
   `curated` node;
4. the hook, which works on a narrow `hook::Draft` (`src/hook.rs`) and never on the builder's own draft, then the
   refusals the hook's calls caused;
5. the value checks of the properties the hook overrides, on the values it left;
6. covers, counts, then finalisation: Images, rows and their order, annotations.

Incunabula's hook (`src/projects/incunabula/`) holds four rules:

| File | Rule |
|------|------|
| `scans.rs` | A Page whose page number is the sentinel alone is a calibration scan and is dropped; one that has a description, a comment or a strip link is refused instead (`calibration-scan-content`) |
| `sides.rs` | A border strip's `side` from the prefix of its label, and a check that a Page's strip on a side is a strip of that side (`strip-side-mismatch`) |
| `sort_key.rs` | A Book's `sort-key`: the curated `sort_key`, else its `name`; it must start with an ASCII character and not be padded (`sort-key-not-ascii`, `padded-sort-key`, `curated-count`) |
| `catalogue.rs` | The replacement of a Book's dead catalogue address by the one the editors curated, while the archive still has the dead address (the curated `catalogue_uri` and `catalogue_uri_replaces`; `stale-catalogue-override`, `unpaired-catalogue-override`, `curated-count`) |

`dsp-incubator cpe/docs/adr/0016-remodelling-is-a-mapping-and-a-registered-hook.md` records the decisions, and
ADR-0007's amendment of 2026-10-11 (DEV-7465) why the hook is in this crate and not in the project's folder.

## Tests

`cargo test -p cpe-store-build`. The unit tests use hand-built stores. `tests/it/` is one integration test binary:

- the tests over the committed 0803 data (`committed_0803.rs`: its census, and the build's independence of the
  snapshot's order; `curation_0803.rs`, the census of the curation file; and the statics of `main.rs`). They read
  `areas/access/sync/data` through `sync-store`'s `LiveArchiveProjection`, which is why `sync-store` is a
  dev-dependency of this crate and never a dependency (ADR-0008; `check-cpe-seam.sh`, rule 4), and Incunabula's
  `config.kdl` and `curation.csv` from `areas/access/media/data/0803/` with `include_str!` and `include_bytes!`;
- the refusal cases (`refusals.rs`) and the row shapes over hand-built snapshots (`books.rs`, `pages.rs`,
  `strips.rs`, `annotations.rs`);
- a build of a snapshot the fake port serves (`rebuild.rs`);
- `hookless.rs`, which builds the fixture projects `plain` and `minimal` (`tests/fixtures/`), whose blocks have no
  `hook` node, and `hooks.rs` and `hook_rules.rs`, which build `hooked` and Incunabula's hook;
- `worked_example.rs`, which holds the worked example of the config-language specification equal to Incunabula's
  block;
- two searches of `src/` (`hygiene.rs`): one for what the library may not hold (`BINARY_ONLY`: the adapter's
  crate, file access, the environment, the network, the clock), and one that holds generic code to naming no
  project and a hook module to the seam.

## Adding a project

Both ports must serve the project: `areas/access/sync/data` holds `<shortcode>.nq` (`areas/access/sync/CONTEXT.md`),
and `areas/access/media/data/<shortcode>/` holds its `config.kdl`, its narrative and, when its block reads a curated
key, its `curation.csv` (`areas/access/media/CONTEXT.md`). Its folder, `areas/access/cpe/projects/<project>/`, holds
a `SHORTCODE` file with the shortcode.

**A project without a hook** is configuration.

1. Write `areas/access/media/data/<shortcode>/config.kdl` with a `site` block that declares at least one language
   and a `remodel {}` block: the shortcode, the classes with their `id`, the properties, and whatever else the
   project needs ([`docs/src/cpe/config-language.md`](../../../../docs/src/cpe/config-language.md), "`remodel`
   block"). `tests/fixtures/plain/config.kdl` uses every node a project without a hook can write;
   `tests/fixtures/minimal/config.kdl` is the least that lowers. The builder reads only the block and the site
   languages. CPE's host validates the whole file, so the committed file also needs a site title, pages, narrative
   and chrome (`dsp-incubator cpe/docs/NEW_PROJECT.md`, Stage F).
2. Build it. The outcomes are: the config's diagnostics with line and column, from `load_remodel`;
   a `ProjectionError` from the port when it serves no snapshot of the shortcode; `BuildErrors` with every
   refusal, each with its rule and its subject; or `sql::EmptyTable` from the writer when a block is present and its
   table has no row. A class of the archive that the block does not declare is refused (`unmapped-class`), and so
   is every Region and LinkObj when the block has no `annotations` node: a project whose archive has annotations
   declares `annotations { on …; link-targets … }` and no class for them, and a project without annotations must not
   declare the node. A property that the block does not declare is dropped without a word, so read "What the port
   serves and the block does not map" below.
3. The tests over real data (`tests/it/committed_0803.rs` and the statics of `tests/it/main.rs`) build Incunabula
   only. Nothing in this crate builds a new project from its committed data until a test is written for it.
4. Two lists are kept by hand, both in `tests/it/hygiene.rs`. `PROJECT_NOUNS` holds the project and class
   names that generic code may not spell: add the new project's name and its class names, with or without a hook.
   `BINARY_ONLY` holds, beside what the library may not use, the folder path of a project that has a hook
   (`projects/<project>`), which no source of the library may spell: add the new project's path when it has a hook.
   Until someone does, nothing forbids the names in generic code or the path in the library.

**A project with a hook** has a block that says so, and code. A rule belongs in the hook only when the block cannot
say it: it needs a condition written as code, a rule table, or a fact about named resources
(`dsp-incubator cpe/docs/adr/0016-remodelling-is-a-mapping-and-a-registered-hook.md`, "The mapping's test for a
feature"). A rule that other projects would need is a node of the grammar, which is a change to `cpe-core` and the
builder.

1. Give the block a `hook` node that declares what the hook does: `key "<curated key>" class="<Class>"` for each
   curated key it reads, `provides "<name>" class="<Class>"` for each property it writes that no node yields,
   `overrides "<name>" class="<Class>"` for each mapped property whose values it sets, and `drops class="<Class>"` for
   each class it drops from. `tests/fixtures/hooked/config.kdl` has one of each.
2. Write the module `src/projects/<project>/mod.rs` with `pub(crate) fn remodel(draft: &mut Draft<'_>, refusals: &mut
   Refusals)`, and add `pub(crate) mod <module>;` to `src/projects/mod.rs`; the module is named for the project with
   underscores for hyphens, and the row of step 3 keeps the folder name. The module reads and changes the draft
   through `crate::hook::Draft` and its `View` alone, names properties by their CPE names, and names no ontology IRI
   (`knora.org`) and no type of the port or of the store. `tests/it/hygiene.rs` searches the module for an import
   beyond the seam and for the ontology host. The seam's methods are in `src/hook.rs`: `Draft::{sentinel, entry,
   entries_of, all_entries_of, drop, provide_text, provide_list_value, override_values}`, `View::{values, served,
   curated, curated_served, label_is_text, …}` and `Refusals::refuse(rule, subject, detail)`. `label_is_text` says
   whether the label may be written as a text (not empty, no address on a `no-addresses` class, no sentinel),
   whether or not the mapping read it: the mapping reads no label while a curated text stands in for it. `entries_of`
   gives the resources of a class that get a row, so a rule that walks a class need not test `is_kept`;
   `all_entries_of` has the refused and the dropped ones too; either is refused for a name that is no class of the
   block. A hook cannot mint or change an id: the ids are final before it is called, and a resource it drops keeps
   its id. A property told from the label (`from="label"`) that a hook overrides has exactly one value (`value-count`
   otherwise). A provided text is a `&str` borrowed from the snapshot or a constant, so a hook cannot provide a
   string it builds. `tests/it/hygiene.rs` names Incunabula's six nouns (`PROJECT_NOUNS`) and its path
   (`BINARY_ONLY`): add the new project's.
3. Add the project's row to `HOOKS` in `src/hooks.rs`: `Hook { project: "<project>", remodel:
   projects::<module>::remodel }`, with the folder name as `project`. Without the `hook` node or without the row the
   build is refused; `tests/it/hooks.rs` holds every row to a block that declares a hook, and every such block to a
   row.
4. A refusal the hook raises is a new variant of `Rule` in `src/error.rs` with a case in `tests/it/hooks.rs`,
   `hookless.rs` or `refusals.rs`: the completeness test compares the rules the cases of the three produce with
   `Rule::ALL`. Every case builds Incunabula or a fixture with a test-local hook `fn`, so a refusal of a new project's
   real hook needs a harness that builds that project. The recording hook in `tests/it/hooks.rs` shows how to hold
   the module to its declaration: after it runs, only the declared classes lost resources, only the declared
   properties changed or appeared.
5. Then continue at step 2 of the project without a hook.

## Named differences

Until 2026-10-10 the prototype's store for Incunabula was written by a Python pipeline in `dsp-incubator`. The store
this crate builds differs from that pipeline's last one in two shapes, both in `properties`:

- **N1**: one resource's values of one property are numbered (`seq`) in the port's order, where the pipeline
  numbered them in text order. On Page `comment`, Book `alt-title` and Book `creator`.
- **N2**: a text keeps the whitespace the archive's text ends in, which the pipeline trimmed. On BorderStrip
  `description` and `comment`, Page `comment` and Book `physical`.

The rule table below marks the rules N1 and N2 come from.
`dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md` records their counts and how the
builder was held to the pipeline's file; [`DATA_ISSUES.md`](../projects/incunabula/DATA_ISSUES.md) gives the reasons.

## Where each rule comes from

`T` is `cpe/projects/incunabula/translate.py` and `X` is `tools/dao-lift/src/cpe_export.rs`, both in
`dsp-incubator`: the pipeline that wrote the store before this crate. The line numbers are theirs in the parent of
the commit that added this crate there (`git log --diff-filter=A -- cpe/store-build/Cargo.toml` in `dsp-incubator`
names that commit). "NEW" marks a rule that pipeline did not have. `inc:` stands for `http://www.knora.org/ontology/0803/incunabula#` and `kb:` for
`http://www.knora.org/ontology/knora-base#`; properties are matched by their full IRI.

"Bucket" says which code applies the rule, as above. `M` names the node of Incunabula's block that says it, `G` is
generic code (the file where one is worth naming), `H` is a hook rule and names its file under
`src/projects/incunabula/`.

| Output | Bucket | Rule | Source |
|--------|--------|------|--------|
| Class of a resource | `M` `class`; `G` annotations by motivation (`generic/classes.rs`) | The block's `class` nodes map `inc:Book`, `inc:Page` and `inc:Band` to `Book`, `Page`, `BorderStrip`. A resource with an annotation body is a Region (motivation commenting or highlighting) or a LinkObj (linking), and its class IRI must be `kb:Region` or `kb:LinkObj` accordingly. Any other class is refused, not dropped. NEW | `Resource.class` |
| Every value read | `G` the node's `type` | A value of another kind than the `property` node's `type` is refused, and a list-node value always is (NEW); the property's other values are then not counted or judged. A text's language is carried into `value_lang` | port |
| `lists`, `list_nodes`, `list_node_labels` | `M` `list` | Two vocabularies, office and side, with German and English labels. A node's `seq` is its declared position; rows are written sorted by node id | `T:252-288` |
| Book id | `M` `id curated-key=` | the curated `slug`: exactly one per Book (`T:509-548`), not empty (`T:677-678`), and no two resources share an id (`T:965-972`) | curation |
| Page id | `M` `id from="property" under-parent=#true` | `<book id>-<slug of the page number>`, with `-2`, `-3`, … on a repeat in positional order, over all of a Book's Pages. A Page has exactly one `seqnum` (`X:1092`) | `X:715-740`, `X:1078-1125`, `X:1391-1401` |
| BorderStrip id | `M` `id curated-key= from="label"` | the curated `slug` if present, else the slug of the label; a label that holds `[missing]` is refused where no slug is curated (NEW) | `X:759-768` |
| Slug | `G` (`generic/slug.rs`) | NFKD, combining marks dropped, ASCII lower case, every run outside `[a-z0-9]` one `-`, ends trimmed | `X:693-709` |
| Every id | `G` (`finalise.rs`) | the id of a resource the build keeps, of an Image, and of an annotation and an annotation link that get a row, is not empty and holds only `A-Z a-z 0-9 - . _ ~` (`id-not-url-safe`). It is checked once the ids are made, so a curated slug, an asset stem and an IRI's last part are all covered, and a Book's slug that fails is refused with every Page id made from it. Ids that differ only in case are two ids. `.` and `..` pass. NEW | `src/finalise.rs` |
| Image id | `G` `images`, `shared-file` (`generic/images.rs`) | one Image per file; the id is the asset's stem. A Book's cover is its cover Page's own Image, written once (`T:1685-1692`). Refused: two Pages with one file (`T:821-825`); two strips with one file (`T:1104-1105`); a stem equal to a Book, Page or strip id, or a Page and a strip with one file (`T:884-885`, `T:1124-1128`); an empty stem (`X:1057-1073`). Calibration scans are checked like every other Page | `T:1565-1572`, `T:1724` |
| `representations` | `G` `images`, `image=#true` | one row per Image: `kind` `image`; `uri` the IIIF base, the asset and `/full/max/0/default.jpg` (`T:186-199`); `width` and `height` from the file; `alt` NULL; `attribution` the rights declared in the `images` node; `seq` 0. Refused: a zero width or height (`T:760-770`); a file that is not a still image (NEW); an asset not ending in `.jp2` (NEW) | `T:1640-1643`, `T:1693-1696`, `T:1725-1728` |
| `resources.ark` | `G` | the port's ARK for Book, Page and BorderStrip; NULL for Image | port |
| Calibration scans | `H` `scans.rs`; `M` `hook › drops` | a Page whose `inc:hasPagenum` is exactly `[missing]` after a Python-style strip is dropped: no Page, no part, no Image. Refused: such a Page carrying a description, a comment or a strip (`T:993-1004`); a page number that contains but is not `[missing]`; an empty page number (`T:836`) | `T:797-881`, `T:988-1012` |
| `compound_parts` | `M` `member-of` | a Page has exactly one parent, a Book (`X:1091`); its position is by `(seqnum, IRI local part)`, renumbered 1..n after the drop | `X:1391-1401`, `T:1005-1012`, `T:1683` |
| Book `name` | `M` `property from="label" curated-key=` | the curated `title_override`, else the label; a label that is empty after a strip is refused, also where an override is curated (`T:681`) | `T:602-604` |
| Book `alt-title` | `M` `exclude-label` | the `inc:hasTitle` values with the first one equal to the label removed, a title being compared without a trailing U+001E and the whitespace around it (`X:786-793`, `X:1199`); a Book whose label is none of its titles is refused; port order (N1) | `X:1349-1356` |
| Book `creator`, `publisher` | `M` `property` | `inc:hasCreator`, `inc:hasPublisher`; port order (N1 on `creator`) | `T:1538-1558` |
| Every Page and BorderStrip text | `G` value step 7 | written as served, a U+001E included: the old pipeline cut a trailing one with the whitespace around it (`X:1096-1101`, `X:1231`, `X:1251-1255`) and failed on one elsewhere in a strip's text (`T:1093-1099`) | port |
| Every Book text | `G` value steps 6 and 7 | written as served; one that is empty after a strip gives no row (`T:1521-1531`). Two departures from the old pipeline: it refused an empty value among several of `alt-title`, `creator` or `publisher` (`T:612-622`), and it cut a trailing U+001E with the whitespace around it from every Book text (`X:1189-1207`) and failed on one elsewhere in a text (`X:1655-1688`). Here the cut is made only where a title is compared with the label, and a U+001E is written as served | `T:1521-1531` |
| Book `pub-place` | `M` `max=1 sentinel="absent"` | `inc:hasPubloc`, at most one (`X:1202`); exactly `[missing]` after a strip gives no row. The old validation compared the unstripped value (`T:701-712`) and so refused a padded sentinel; none exists | `T:1579-1583` |
| Book `shelfmark`, `physical` | `M` `max=1` | `inc:hasLocation`, `inc:hasPhysicalDesc`, at most one of each (`X:1204`, `X:1206`), as served (N2 on `physical`) | `T:1551-1553` |
| Book `catalogue-uri` | `H` `catalogue.rs`; `M` the `property` node, `hook › overrides` | `inc:hasUri` as text, at most one (`X:1205`). An override is two curated values of the Book, in `areas/access/media/data/0803/curation.csv`: `catalogue_uri`, the address to write, and `catalogue_uri_replaces`, the address it replaces. Both are read as served, since each is an address. The override wins while the archive has exactly the address it replaces; with any other address, or none, the Book is refused until the editors take both values out (`stale-catalogue-override`). One of the two without the other is refused (`unpaired-catalogue-override`, NEW), and so is more than one value under either (`curated-count`). `tests/it/committed_0803.rs` holds that the three overridden Books are written with the replacement | `T:550-599` |
| Every Book value read, every curated value | `M` `sentinel`, `no-addresses`; `G` curated texts (`generic/curation.rs`) | none contains `[missing]` (apart from `pub-place`) and none contains `http` (apart from `catalogue-uri` and the curated `cover_page`, `catalogue_uri` and `catalogue_uri_replaces`) | `T:691-717` |
| Book `sort-key` | `H` `sort_key.rs`; `M` `hook › key`, `provides` | the curated `sort_key`, else the display title; its first character after a strip is ASCII; one for every Book. A padded curated `sort_key` is refused, also one that is whitespace alone | `T:607-609`, `T:727-739` |
| Book `teaser` | `M` `curated type="per-language"` | `teaser@de` at `seq` 0 and `teaser@en` at `seq` 1; both or neither, and in no other language | `T:741-746`, `T:1596-1600` |
| Book `office` | `M` `curated type="list"` | a list value, `office-<curated office>`; set, and one of the vocabulary | `T:664-675`, `T:1604-1607` |
| A Book's curated values | `G` (`generic/curation.rs`) | read without the whitespace around them, as `T:342-343` read a cell; one that is empty then is no value. A key has at most one value, `teaser` one per language | curation |
| Book `pub-date` | `M` `property type="date" min=1 max=1 display-key=`; `G` the conversion (`generic/dates.rs`) | `inc:hasPubdate`: each Julian Day Number converted in the value's own calendar and written zero-padded to its precision; `date_display` and `value_lang` from the curated `date_display@<lang>`, which is set and has a language. Refused: a Book without a date or with several (`X:1195`); mixed precision; a bound outside the years 1 to 9999 (NEW); a `date_display` in a language that is not one of the site's (`site { languages }`), which no visitor would be shown (`unknown-date-display-language`, NEW) | `T:367-419`, `T:1609-1619` |
| Book `extent` | `M` `member-count` | the number of member Pages after the calibration drop | `T:1625-1628` |
| Book `image` | `M` `cover` | a link, `seq` 0, to the Image of the curated `cover_page`: the Book's own Page, not a calibration scan, unique per Book | `T:927-961`, `T:1635` |
| Page `name`, `page-id` | `M` two names on one `property` | both `inc:hasPagenum`, with its language where it has one | `T:1670-1671` |
| Page `seq-number` | `M` `member-of seq=` | the port's `seqnum` | `T:1672` |
| Page `description` | `M` `max=1` | `inc:hasDescription`, at most one (`X:1096`), as served; empty after a strip gives no row (`T:874`); `[missing]` as a substring is refused (`T:850`) | `T:847-854`, `T:1673-1674` |
| Page `comment` | `M` `property` | `inc:hasComment` in port order (N1), as served (N2); empty after a strip gives no row (`T:858`), also among several, where the old pipeline refused it (`T:860`); `[missing]` as a substring is refused (`T:865`) | `T:856-870`, `T:1675-1676` |
| Page `image` | `G` | a link, `seq` 0, to the Image of the Page's own file | `T:1679` |
| Page `strip-left`, `strip-right` | `M` `property type="link" max=1 target=`; `H` `sides.rs` the side check | the `inc:hasBandTypeL` and `inc:hasBandTypeR` links, at most one of each (`X:1104-1105`); the target is a Band (`T:1144`) and its side must match (`T:1149`), so a strip of no side is refused on either | `T:1132-1173`, `T:1734-1735` |
| BorderStrip `name` | `M` `curated-key= min=1 max=1 empty="refuse"` | the curated `name` if present (at most one, read as a Book's curated values are, its language not read), else `inc:hasTitle`: exactly one (`X:1145`), as served, and refused when it is empty or `[missing]` after a strip (`T:1062`) | `T:1059-1066`, `T:1709` |
| BorderStrip `description` | `M` `sentinel="absent" sentinel-within="keep"` | `inc:hasDescription`, at most one (`X:1146`), as served (N2); `[missing]` or empty after a strip gives no row | `T:1078-1080`, `T:1710-1711` |
| BorderStrip `comment` | `M` `empty="refuse" sentinel-within="keep"` | `inc:hasComment` in port order, as served (N2); the old file numbered them in text order (`X:1147`), which no strip's port order differs from; empty or exactly `[missing]` after a strip is refused, also a strip's only comment, which the old pipeline dropped when it was empty (`T:1084`) | `T:1084-1090`, `T:1712-1713` |
| BorderStrip `side` | `H` `sides.rs`; `M` `hook › provides` | a list value, `side-left` or `side-right`, from the label's prefix, `l_` or `r_`; no row otherwise. The old pipeline refused a strip of no side unless it was the unsigned one (`T:1068-1073`) | `X:771-779`, `T:1715-1716` |
| BorderStrip `page-uses` | `M` `incoming-count` | the number of strip links that point at it, 0 for a strip no Page uses | `T:1133`, `T:1155`, `T:1719` |
| BorderStrip `image` | `G` | a link, `seq` 0, to the Image of the Band's own file | `T:1720` |
| Image `authorship`, `license` | `M` `images` | the strings of the `images` node's `authorship` and `license` | `T:209-210` |
| Curated keys | `M` declared where read | the keys are `slug` (Book, Band), `cover_page`, `office`, `date_display`, `title_override`, `sort_key`, `teaser`, `catalogue_uri`, `catalogue_uri_replaces` (Book), `name` (Band, LinkObj), `keep` and `lang` (Region, LinkObj). A value under another key, or on a resource of another class than its key's, is refused (NEW): the old pipeline read named columns of two tables. The block declares the Book's and the strip's keys; `keep`, `lang` and `name` on an annotation are the builder's | curation |
| A Region's and a LinkObj's curated values | `G` (`generic/annotations.rs`) | `keep`: exactly one, `yes` or `no` (`T:1281-1282`). `lang`: at most one, `de` or `en` (`T:1283-1287`); it is the language of the Region's remark, or of the link's name and comment. `name`: a LinkObj's only, at most one. Read as a Book's curated values are | curation |
| `annotations` | `G` (`generic/annotations.rs`) | Regions with `keep` = `yes`. The id is the last part of the Region's IRI (`X:1517`). The geometry (`kb:hasGeometry`) is normalised to a box in 0..1 and, for a polygon, its points (`T:1194-1236`); the colour (`kb:hasColor`) is stripped and lower-cased, `#rrggbb`; the remark is `kb:hasComment` stripped, NULL when empty; `remark_lang` is the curated `lang` when there is a remark; `resource_id` is the id of its `kb:isRegionOf` Page; `ark` is the port's | `T:1194-1247`, `T:1314-1349` |
| `annotation_links` | `G` | LinkObjs with `keep` = `yes` and at least one target left; the id is the last part of the IRI; `name` is the curated `name`, else the stripped `kb:hasComment`, and a kept link with neither is refused once it is known to have a target left (`T:1412-1415`); `name_lang` is the curated `lang`; `comment_lang` is the curated `lang` when there is a comment (`T:1416`) | `T:1389-1416` |
| `annotation_link_targets` | `G`; `M` `annotations › link-targets` | the `kb:hasLinkTo` targets, a target served twice being one, sorted by kind (a Region, then the resources of the `link-targets` classes in their declared order) and id; a target that is a Region without a row is left out, and a link left with no target is not written; renumbered from 1 | `X:1601-1634`, `T:1353-1422` |
| Annotation checks | `G`; `M` `annotations › on` | run on every Region and LinkObj before `keep` is asked for, as `T:1320-1342` did: no two of them share an id (`T:1317`, `T:1392`, `T:1397`); a Region has exactly one geometry and one colour, and a Region or a LinkObj at most one comment (`X:1473-1481`, `X:1578`); the geometry is JSON with numbers for its points and for a circle's radius, of a known shape with that shape's number of points, and its box has an area and is inside the image, with a slack of 1e-9 (`T:1328-1333`); no number of it is written with an exponent (NEW); the colour is `#rrggbb`; a Region's remark holds no U+001E inside it (one at its ends goes with the strip, as it did); a Region has exactly one `kb:isRegionOf` target, a resource of an `on` class (`X:1478`, `X:1511`); a link target is a Region or a resource of a `link-targets` class; a link has a target (`T:1355-1373`, `T:1399-1400`). On kept annotations only (NEW): a kept Region on a calibration scan, and a kept link to one, is refused. A Region on a resource, or a link to one, that was refused for a defect of its own or for its class is not written and not refused again | `T:1320-1373` |
| Row order in the file | `G` (`finalise.rs`); `M` the declared class order | `lists` by id; `list_nodes` by id; `list_node_labels` by node, `de` then `en`; `resources` by class in the order the block declares them (Book, Image, Page, BorderStrip: the Image rows stand where the `images` node stands among the `class` nodes; `T:1760`), then id; `properties` by (resource, name, `seq`); `representations` by (resource, `seq`); `compound_parts` by (parent, `seq`); the annotation tables by id, targets by (link, `seq`). Every sort is bytewise | `T:1881-1911`, `T:1426-1428` |
| File layout | `G` (`sql.rs`) | a generated header comment, a blank line, one multi-row `INSERT` per table; a text literal doubles its apostrophe and escapes nothing else; a REAL is written as Python's `repr` writes it, which Rust's `{:?}` equals for every value without an exponent | `T:312-331`, `T:1236` |
| Every "strip" | `G` (`generic/text.rs`) | Python's `str.strip()`, which removes U+001C to U+001F as well as what Rust's `trim` removes | `T:342-343`, `T:1239-1247` |

### Rules that moved

The generic code names no class, so a rule that was raised once per class is one rule now: the thirteen rules of
the "before" column moved onto six names, five of them new (`value-count`, `empty-value`, `unknown-list-node`,
`curated-languages`, `shared-file`) and one that existed (`missing-in-value`). A refusal case keeps its
snapshot and its subject and names the new rule; where more than one rule stands in the "now" column, the value
decides which one the refusal is.

| Rule before | Rule now | Why |
|-------------|----------|-----|
| `book-value-count`, `page-value-count`, `strip-value-count`, `pagenum-count` | `value-count` | `min` and `max` are one feature |
| `empty-pagenum`, `empty-title` | `empty-value` | `empty="refuse"` and an empty label are one check |
| `empty-strip-comment` | `empty-value` (a comment empty once stripped) or `missing-in-value` (the sentinel alone) | value step 5 reports the first offending value, once per property |
| `strip-without-name` | `value-count` (not exactly one title), `empty-value` or `missing-in-value` | the same checks on the title |
| `pagenum-holds-missing` | `missing-in-value` | `sentinel-within="refuse"` |
| `unknown-office` | `unknown-list-node` | `curated type="list"` |
| `teaser-languages` | `curated-languages` | `curated type="per-language"` |
| `pages-share-file`, `strips-share-file` | `shared-file` | one class, one file each |

Five rules have no predecessor: `link-target-without-row` (a link to a resource a hook dropped), the three that hold
a hook to its block (`hook-not-registered`, `hook-not-declared`, `hook-exceeds-declaration`) and `id-not-url-safe`.
A rule whose name carries a class of Incunabula but whose meaning is generic keeps its name (`page-parent`,
`page-without-seqnum`, `book-without-curation`, `strip-link-target`, `cover-not-own-page`,
`cover-is-calibration-scan`, `region-on-dropped-page`, `link-to-dropped-page`); `dsp-incubator cpe/docs/BACKLOG.md` lists them.

## What the port serves and the block does not map

The properties the port serves and Incunabula's block declares no `property` node for are dropped without error: a
Book's `inc:hasCitation`, `inc:hasNote`, `inc:hasComment` and `inc:hasDescription`, and a Page's `inc:hasCitation`
and `inc:hasOrigname`. The set is held by a guard test over the committed data,
`test_snapshot_unmapped_properties_are_the_listed_six` in `tests/it/committed_0803.rs`, with its list `IGNORED`: it
fails when the properties the snapshot serves on the block's classes and the block does not map are not exactly
these six. A change of the committed snapshot that brings a property the block does not map therefore fails that test, and the choice is to
map it in the block or to list it in `IGNORED` and here. A second test of that file
(`test_build_reads_every_declared_property`) takes each mapped property away in turn and fails when the store does
not change. Only properties are dropped this way: a resource of a class the block does not declare is refused.

## Limits that are known

- A property IRI that a `property` node declares and the snapshot never serves is silent unless the node sets `min`:
  inside one project's build it cannot be told from a legitimate state.
- Nothing refuses two projects that declare one shortcode.
- A list node's id is `<list>-<slug>`, so a list `a` with a node `b-c` and a list `a-b` with a node `c` give one id;
  the lowering checks a slug within its list only.
- `.` and `..` pass `id-not-url-safe`, and ids that differ only in case are two ids.
- A date's display text (`display-key`) on a resource that has no date is not read and not written, and nothing says
  so: the builder cannot tell a text the editors wrote too early from one that is left over.
- The check that generic code names no project (`hygiene.rs`) searches for six whole words, case-sensitively: the
  project's name in two cases and its four class names. It does not see a lowercase or plural form, a CPE property name,
  the shortcode or a host in a string, or a file read with `include_str!`. The check that the library reads nothing
  from outside (`BINARY_ONLY`) does see a module of `std` that a `use` names in a group or under an alias, and one
  named below an alias a `use` gave `std` itself.
- Generic code still holds the annotation curated keys (`keep`, `lang`, `name`), refuses `http` in every curated
  text, whatever a class's `no-addresses` says, and handles U+001E (cut from a title that `exclude-label` compares,
  refused inside a Region's remark), which is one archive's data.
