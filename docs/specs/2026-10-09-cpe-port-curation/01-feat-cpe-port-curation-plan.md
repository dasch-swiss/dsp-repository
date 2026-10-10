---
title: "feat: Serve project curation through cpe-ports"
type: feat
date: 2026-10-09
author: "Balduin Landolt"
status: reviewed
linear: DEV-7495
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7495
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator/.claude/worktrees/DEV-7495
---

# feat: Serve project curation through cpe-ports

## Overview

`cpe_ports::ProjectSnapshot` gains `curation: Vec<CuratedValue>`: per-resource values a project's editors
authored and the archive does not record, such as a Book's slug or whether a test Region is shown.
`sync-store` reads them from one hand-authored CSV per known project, `data/<shortcode>-curation.csv`,
beside the committed snapshot, and treats every column as an opaque key. The contract gains three
violations. Incunabula's curation is committed as `0803-curation.csv`: 341 values on 137 resources. The
incubator then re-vendors `cpe-ports` at the merged commit.

There is no PRD. The requirements are Linear DEV-7495, the interim decision DEV-7488 and eight owner
decisions of 2026-10-09 (*Proposed Solution*). The ticket asks for a short design note first. That note is
a new decision record, ADR-0010, with the status `proposed`: it is the first deliverable, and no separate
note is written. Ivan accepts or rejects it under DEV-7488.

## Problem Statement / Motivation

DEV-7402 builds Incunabula's store from the port alone: ADR-0008 lets a read model be fed by its port and
by nothing else. Thirteen of Incunabula's 52 store outputs are not archive data, though. They come from
files the port does not serve:

- `dsp-incubator/cpe/projects/incunabula/raw/curation.csv`: per Book, the slug (the public URL), the
  office lane, the display date, a title override, the cover page, a sort key and two teasers.
- `raw/annotation-curation.csv`: per Region and LinkObj, whether it is shown, its language and a link name.
- Constants in `translate.py`: the office and side vocabularies with their labels, the rights line of the
  scans, and the id and name of the one Band without a signature.
- `raw/catalogue-overrides.csv`, temporary and self-retiring.

DEV-7488 decided, as an interim, that `sync` serves a project's curation, so the read model stays fed by
its port. Ivan has not confirmed it yet. This plan implements that decision for per-resource curation and
leaves everything project-wide to the project's KDL.

Today:
- `ProjectSnapshot` holds resources and list nodes only (`areas/access/cpe/ports/src/snapshot.rs:66-79`).
- `sync-store` reads one file per project, `<dir>/<shortcode>.nq` (`areas/access/sync/store/src/lib.rs:65-66`).
- `areas/access/sync/CONTEXT.md:25` says the committed snapshot "is everything `sync` holds of a project
  today".
- Nothing in the incubator uses `cpe_ports` outside `cpe/vendor/` (grep, 2026-10-09: only `cpe/Cargo.toml`
  names it).

## Proposed Solution

### Decisions (owner, 2026-10-09)

1. **The KDL configuration stays in the project's folder** (ADR-0007, "a project is a folder"). Only
   curation goes through `sync`. DEV-7488's "configuration" is read narrowly; ADR-0010 states this for
   Ivan to confirm.
2. **The line is "keyed by one resource".** The port serves values keyed by a resource IRI. For
   Incunabula: a Book's slug, office lane (a plain key such as `amerbach`), display date, title override,
   cover page, sort key and teasers; a Region's or LinkObj's keep flag, language and name; and the slug
   and name of one Band. Everything project-wide is configuration and goes to the project's KDL under
   DEV-7402: the office and side vocabularies with their labels and order, and the rights constants.
3. **The format is one wide, generic CSV**, hand-authored and committed beside `0803.nq`. The first column
   is the full resource IRI; every other column is a curated key, `key@lang` for a language-tagged value.
   `sync-store` knows no Incunabula key. `cover_page` holds a Page IRI, not a CPE slug.
4. **`raw/catalogue-overrides.csv` is not ported.**
5. **The curation is one file per project**, `0803-curation.csv`, not a directory of files (*The curation
   file* gives the reasons).
6. **The Band without a signature carries `slug` = `strip-38`** beside its name. It is a hand-chosen public
   id for one resource, so it is curation. The other 37 Bands keep their rule-derived ids, which are
   DEV-7402's hook.
7. **`date_display` is served as `date_display@de`.** The language belongs with the authored value, and the
   hook then needs no constant for it. A link's `name` stays untagged; its language is the row's `lang`
   key, as in the source.
8. **The change of two accepted ADRs is recorded as a new ADR, `proposed`.** ADR-0006 says an amendment
   "refines a decision in place" and a decision that "changes rather than narrows becomes a new ADR"
   (`docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md:23-24`). This plan changes a
   sentence of ADR-0007 and of ADR-0008, so it adds ADR-0010 and edits neither (*Technical
   Considerations*, "The ADRs").

The committed tests pin literal counts, so a curation edit changes them in the same PR. The owner accepted
that (2026-10-09).

### Boundary DTO (`cpe-ports`, `snapshot.rs`)

```rust
pub struct ProjectSnapshot {
    pub shortcode: String,
    pub resources: Vec<Resource>,
    pub list_nodes: Vec<ListNode>,
    /// The project's curation: what its editors authored about single resources and the archive does
    /// not record. Never an archive fact. Order unspecified; resource, key and language together
    /// are unique, and every resource is one of `resources`.
    ///
    /// Provisional: `sync` serves curation by an interim decision (ADR-0010, proposed; DEV-7488).
    pub curation: Vec<CuratedValue>,
}

/// One curated value of one resource. The port gives `key` no meaning: the project's KDL and hook do.
/// `Ord` is resource, then key, then language (none first), then text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CuratedValue {
    pub resource: ResourceIri,
    /// The project's own name for the value; a curation name (`contract::is_curation_name`).
    pub key: String,
    /// The value's language, `None` where the project tags none; a curation name where there is one.
    pub lang: Option<String>,
    /// Never empty: a value the editors left out is absent. The port promises no more than that.
    /// `sync-store` also refuses padding and control characters, but the contract does not, so a
    /// fixture may hold them.
    pub text: String,
}
```

And in `contract.rs`, beside the rules that use it:

```rust
/// Whether `name` can be a curated key or language: `[a-z][a-z0-9_-]*`, so `key@lang` reads one way.
/// A language tag with an upper-case part, such as `de-CH`, is refused by design.
#[must_use]
pub fn is_curation_name(name: &str) -> bool
```

- The field sits on the snapshot, not on `Resource`: `Resource` stays what the archive records, and
  curation stays visibly separate from it.
- `key` is a plain `String`. A `CurationKey` newtype would add a type for a value the port never
  interprets.
- A value that names another resource (`cover_page`) is a plain text for the port. Whether it resolves is
  the reader's rule, because only the reader knows which keys hold IRIs.
- `CuratedValue` derives `PartialOrd, Ord`, as `Violation` does. The field order gives the order
  `sync-store` serves in, so its reader sorts with `sort()`.
- `cpe_ports::contract::is_curation_name` is the one spelling of the name rule. It is a rule, not a DTO,
  so it is not re-exported from the crate root. The contract uses it, and so does `sync-store`'s reader
  for a column name; `sync-store` already imports `contract` (`areas/access/sync/store/src/lib.rs:38`).
- Doc examples in `cpe-ports` and `sync-store` use neutral keys (`colour`, `caption@en`), never one of
  Incunabula's.
- `CuratedValue` is re-exported from `lib.rs`. No type is `#[non_exhaustive]`, so every `ProjectSnapshot`
  literal breaks on purpose (*Consumers*).
- Docs that say the port serves archive facts only gain one sentence each (`lib.rs:1-6` and `:26-35`,
  `snapshot.rs:1-9` and `:66-68`): the snapshot also carries the project's curation, which is not an
  archive fact.

### Contract (`contract.rs`)

Three violations, declared after `LinkWithValueUuid`, so the order of the existing ones is unchanged.
`resource` is the first field of each (the rule in `Violation`'s doc, `contract.rs:20-23`).

| Variant | Meaning | Reported |
|---------|---------|----------|
| `DanglingCuration { resource }` | a curated value names a resource that is not in the snapshot | once per IRI |
| `DuplicateCuration { resource, key, lang }` | two or more values share resource, key and language | once per triple |
| `MalformedCuration { resource, key, lang }` | a value whose `key`, or whose `lang` where it has one, is no curation name, or whose `text` is empty | once per entry of `curation` |

- `MalformedCuration` is one shape rule for the three ways a fixture can get a value wrong (`key: ""`,
  `lang: Some("")`, `text: ""`). It is not deduplicated: two identical malformed entries report it twice,
  beside one `DuplicateCuration`, as two entries of one IRI report `MalformedDataArk` twice.
- Each gets a `Display` arm in the existing `resource {} …` form. The arm shows the key as `key`, or
  `key@lang` where the value has a language. `MalformedCuration` shows it with `{:?}`, since it may be
  empty, and its message is true for all three causes: "resource {} has a curated value {name:?} whose
  key or language is not a curation name, or whose text is empty".
- `sync-store` cannot produce any of them: its reader refuses the file first. The contract still checks,
  for every adapter and every `FakeArchiveProjection` fixture, which is what DEV-7402's tests build on.
- The same key in two languages, tagged and untagged, or on two resources is no violation.
- **Not contract.** The contract checks no key's meaning and nothing of a non-empty `text`. Which keys
  exist, which class carries which key, and whether a Book has a slug are project rules. They stay with
  DEV-7402's reader (*Handed to DEV-7402*). The module doc (`contract.rs:1-13`) says so.

### The curation file (`areas/access/sync/data/<shortcode>-curation.csv`)

One file per known project. The path is built from the `KNOWN` shortcode, never from the caller's string
(`lib.rs:87-92`).

**Format**
- **Lines.** UTF-8. The file is lines, each ended by a line feed (U+000A); the last line may lack it. So
  one final line feed is the end of the file, and a second one is a blank line. One line is one record,
  and a fault's line is the line an editor shows.
- **Refused characters.** A control character (Unicode category Cc, `char::is_control`), which includes
  the tab, the carriage return of a CRLF line end and U+0085; and U+2028 and U+2029, which are not Cc but
  which editors show as line breaks. The only one the file may hold is the line feed that ends a line.
  Any other is refused wherever it stands: header, cell, comment cell, inside quotes or outside.
- **Quoting**, RFC 4180 but strict. A cell is quoted exactly when its first character is `"`. It then runs
  to the next `"` that is not doubled, `""` inside it standing for one quote, and that closing quote is
  followed by a comma or the end of the line. A `"` anywhere else is refused. A quoted cell that reaches
  the end of its line unclosed is refused, so no cell holds a line break.
- **A cell's value is what the quotes hold.** Quotes protect a comma and a quote, nothing else: `" a"` is
  the value ` a`, which the padding rule refuses like the bare cell.
- **Header.** Line 1. Its first column is exactly `iri`. Every other column is `key` or `key@lang`, both
  parts curation names (`contract::is_curation_name`), or a comment column. No two columns have the same
  name; a later column named `iri` is a duplicate.
- **Comment columns.** A column whose name starts with `#`, whatever follows, `#` alone included. It is
  for the file's editors and is never served. Its cells are held to the refused-character rule and the
  quoting rule, and to no other.
- **Rows.** Every later line is one resource: its full IRI, then one cell per column. A row has exactly
  as many cells as the header; a trailing comma is a trailing empty cell. A line with no character at
  all is a blank line and is refused, also under a header of `iri` alone.
- **The IRI** is a resource of the project's snapshot, and no IRI has two rows. Rows may be in any order.
- **Values.** An empty cell is an absent value, and nothing is served for it; `""` as a whole cell is an
  empty cell. A row whose value cells are all empty is allowed and serves nothing. Any other value is
  served verbatim.
- **Padding.** A value that starts or ends with white space (Unicode White_Space, `char::is_whitespace`,
  which includes U+00A0), with U+200B or with U+FEFF is refused, not trimmed: `sync` never repairs what
  it serves (ADR-0007, DEV-7399 amendment), and such a character is invisible in a diff. Inside a value
  all three are served as written.
- **Not checked.** Other format characters (category Cf) and Unicode normalisation. A byte-order mark
  makes the first column something other than `iri` and is refused that way.
- **Words.** "Cell" is one comma-separated item of a line, the header's included.

**Why one file, not a directory of files.** A directory (`0803-curation/books.csv`, `annotations.csv`,
`bands.csv`) gives dense rows but adds rules: which entries count, their order, what an empty directory
means, and a value given twice across files. One file has one path, as `<shortcode>.nq` has, and one row
per resource, so uniqueness is two simple rules (no column twice, no IRI twice). The cost is sparse rows:
a Book row ends in four empty cells and an annotation row starts with eight. The owner chose the one
file (decision 5). The exact cell count and the committed tests' per-key counts catch a miscounted
comma. Splitting later adds a rule and breaks nothing in the port.

**An absent file.** A known project without its curation file is `Unavailable` (`SnapshotError::Read`). A
project with no curation commits a file holding the header `iri` alone. An absent file therefore never
reads as "no curation", and no test can pass on a file that is not there.

**Git.** The format refuses CRLF, so `.gitattributes`, which today holds only the Git LFS rules for
`docs/specs/`, gains `areas/access/sync/data/*-curation.csv text eol=lf`.

### Reader (`sync-store`, new `src/curation.rs`)

```rust
/// The curated values of a curation file's bytes, sorted, or the first fault met. `known` holds the
/// IRIs of the served resources: a row for any other IRI is a fault. Pure: no I/O.
pub(crate) fn parse(bytes: &[u8], known: &BTreeSet<&str>) -> Result<Vec<CuratedValue>, CurationFault>
```

- **I/O and parsing are apart**, as in the rest of the crate: `read_quads` (`lib.rs:97`) does the I/O and
  `mapping::map` (`mapping/mod.rs:35`) is pure. `LiveArchiveProjection::serve` (`lib.rs:65-81`) reads the
  bytes with `fs::read`, a failure being `SnapshotError::Read` with the curation file's path; calls
  `curation::parse`; and wraps a fault into `SnapshotError::Curation { path, fault }`. It does so after
  the mapping and the `Empty` check, sets `snapshot.curation`, then runs the contract. `mapping::map`
  leaves `curation` empty: the quads hold none.
- The file is re-read on every call, as the snapshot file is.
- **Order.** The served values are sorted with `CuratedValue`'s `Ord`. Nothing depends on the file's row
  or column order or on a hash.
- **The first fault only.** `parse` stops at the first fault it meets, in this order:
  1. Decoding. Any invalid byte is `NotUtf8`, at the line of the first one, whatever else the file holds.
  2. No line, or an empty first line, is `MissingHeader`.
  3. Then the lines from first to last. Each line is first read into cells, left to right: a refused
     character, a stray quote, or a quote still open at the end of the line is the fault. There is no
     separate scan for refused characters.
  4. Then the line is checked. The header, column by column: column 1 is `FirstColumnNotIri` unless it is
     `iri`; a later column is `MalformedColumn` if its name has none of the three forms, else
     `DuplicateColumn` if a column to its left has the same name.
  5. A row: `BlankLine` if it has no character; `WrongCellCount`; then the `iri` cell, `UnknownResource`
     or else `DuplicateRow`; then the value cells left to right, `PaddedValue`.
- **The parser is hand-written.** The text is split at line feeds with `split_terminator('\n')`, never
  with `str::lines()`, which would swallow the carriage return the refused-character rule must see. A
  small state machine reads each line's cells over `char_indices`.
- **Why not the `csv` crate.** It has no strict mode. It does not report whether a cell was quoted, so a
  stray quote cannot be refused without scanning the raw text again, and that scan is this parser. By
  default it treats `\r`, `\n` and `\r\n` alike as record ends, which hides the CRLF this format refuses.
  How it handles each lenient case is not documented and was not probed. No dependency is added. The
  repository has no Bazel files (ADR-0001 is not implemented), so nothing but Cargo lists dependencies.

**Errors** (`error.rs`). A file that cannot be read is the existing `SnapshotError::Read` with the
curation file's path. Every other fault is a new variant:

```rust
/// The curation file breaks its format: the first fault met.
#[error("{} is not a valid curation file at {fault}", path.display())]
Curation { path: PathBuf, fault: CurationFault },

/// A fault of a curation file. `line` is 1-based; the header is line 1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {reason}")]
pub struct CurationFault { pub line: usize, pub reason: InvalidCuration }
```

`InvalidCuration` derives `Debug, Clone, PartialEq, Eq, thiserror::Error`, one variant per rule, with
lowercase noun-phrase messages like `InvalidFact`'s:

| Variant and its fields | Message |
|------------------------|---------|
| `NotUtf8` | "bytes that are not UTF-8" |
| `MissingHeader` | "no header" |
| `ControlCharacter { character: char }` | "the control or line-break character {character:?}" |
| `StrayQuote` | "a quote inside a bare cell, or text after a closing quote" |
| `UnterminatedQuote` | "a quoted cell that does not end on its line" |
| `FirstColumnNotIri { found: String }` | "a first column named {found:?}, not iri" |
| `MalformedColumn { column: String }` | "a column named {column:?}, which is neither key, key@lang nor a comment" |
| `DuplicateColumn { column: String }` | "the column {column:?} twice" |
| `BlankLine` | "a blank line" |
| `WrongCellCount { expected: usize, found: usize }` | "a row of {found} cells under a header of {expected}" |
| `UnknownResource { iri: String }` | "a row for {iri:?}, which is not a resource of the snapshot" |
| `DuplicateRow { iri: String, first_line: usize }` | "a second row for {iri}, after line {first_line}" |
| `PaddedValue { column: String }` | "a value under {column} that starts or ends with white space or an invisible character" |

- Lines and counts are `usize`: they come from `enumerate()` and `len()`. `SnapshotError::Syntax.line`
  (`error.rs:22`) is `u64` only because the N-Quads parser hands one over.
- `NotUtf8`'s line is one more than the line feeds before the first invalid byte. `UnterminatedQuote`'s
  is the line the quote opens on; the lines after it are numbered as the file has them.
- `InvalidCuration` and `CurationFault` are re-exported from `lib.rs`, beside `InvalidFact`; `mod curation`
  stays private.

**Where a check goes** (`lib.rs:14-22`). The file's rules are the reader's. Three of them the contract
checks again: a value for a resource that is not in the snapshot, a repeated resource, key and language,
and a key or language that is no curation name. The reader names them first, with the line, as the
mapping does for a dangling parent.

**Test tables.** Each named test of Phase 2 is one table; every row is an input and its one outcome. `⏎`
is a line feed. `X`, `Y` and `Z` stand for three known IRIs, `U` for an IRI outside the known set. Where a
row gives only a row of the file, the header is `iri,a` (or `iri,a,b` for three cells) on line 1 and the
row is line 2. `X` sorts before `Y`, and `Y` before `Z`. `\r` is a carriage return; a tab or any other
invisible character is named in words, never typed. A table of header cases gives line 1 alone and its
fault is on line 1; a fault without a stated line is on line 2.

`test_curation_parse_values_returns_them_sorted`

| Input | Values (resource, key, language, text) |
|-------|------------------------------------------|
| `iri⏎` | none |
| `iri` | none |
| `iri,colour,size⏎X,red,big⏎` | (X, colour, –, red), (X, size, –, big) |
| `iri,caption@de,caption@en⏎X,Rot,Red` (no final line feed) | (X, caption, de, Rot), (X, caption, en, Red) |
| `iri,size,caption@en,caption,colour⏎Y,s,c,b,r⏎X,big,,plain,⏎` | (X, caption, –, plain), (X, size, –, big), (Y, caption, –, b), (Y, caption, en, c), (Y, colour, –, r), (Y, size, –, s) |
| `iri,"colour"⏎X,red⏎` | (X, colour, –, red) |

`test_curation_parse_empty_cells_are_absent`

| Input | Values |
|-------|--------|
| `iri,a,b⏎X,,v⏎` | (X, b, –, v) |
| `iri,a,b⏎X,"",v⏎` | (X, b, –, v) |
| `iri,a,b⏎X,v,⏎` | (X, a, –, v) |
| `iri,a,b,c⏎X,v,,⏎` | (X, a, –, v) |
| `iri,a,b⏎X,,⏎` | none |
| `iri,a,#note,#⏎X,v, kept ,x⏎` | (X, a, –, v) |

`test_curation_parse_text_is_verbatim` (the cell under `a` → the served text)

| Cell | Text |
|------|------|
| `"red, dark"` | `red, dark` |
| `"say ""hi"""` | `say "hi"` |
| `""""` | `"` |
| `a  b` (two spaces) | `a  b` |
| `März für` | `März für` |
| `a`, U+200B, `b` | unchanged |
| `a`, U+FEFF, `b` | unchanged |

`test_curation_parse_invalid_bytes_return_not_utf8`

| Input | Fault |
|-------|-------|
| `iri⏎X⏎` then the byte `0xFF` | `NotUtf8`, line 3 |
| `iri,a\r⏎X,v\r⏎Y,v\r⏎Z,` then the byte `0xFF` | `NotUtf8`, line 4 (not the `\r` of line 1) |

`test_curation_parse_no_header_returns_missing_header`

| Input | Fault |
|-------|-------|
| no bytes | `MissingHeader`, line 1 |
| `⏎X,v⏎` | `MissingHeader`, line 1 |

`test_curation_parse_refused_characters_return_control_character`

| Input | Fault |
|-------|-------|
| `iri,a⏎X,v⏎Y,v` + tab + `w⏎` | `'\t'`, line 3 |
| `iri,a\r⏎X,v\r⏎` | `'\r'`, line 1 |
| `X,a` + U+0085 + `b` | U+0085, line 2 |
| `X,a` + U+2028 + `b` | U+2028, line 2 |
| `X,a` + U+2029 + `b` | U+2029, line 2 |
| `iri,"a` + tab + `b"⏎` | `'\t'`, line 1 |
| `iri,a,#note⏎X,v,n` + tab + `⏎` | `'\t'`, line 2 |
| `X,"a` + tab + `b` (the quote never closes) | `'\t'`, line 2, not `UnterminatedQuote` |

`test_curation_parse_misplaced_quotes_return_stray_quote`

| Row | Fault |
|-----|-------|
| `X,a"b` | `StrayQuote`, line 2 |
| `X, "b"` | `StrayQuote`, line 2 |
| `X,"a"b` | `StrayQuote`, line 2 |
| `X,"a" ,b` | `StrayQuote`, line 2 |
| `X,""x` | `StrayQuote`, line 2 |

`test_curation_parse_open_quotes_return_unterminated_quote`

| Input | Fault |
|-------|-------|
| row `X,"a""` | line 2 |
| row `X,"""` | line 2 |
| row `X,"` | line 2 |
| `iri,a⏎X,v⏎Y,"one⏎two"⏎` | line 3 |
| `iri,a⏎X,"v` (no final line feed) | line 2 |
| `iri,"a⏎` | line 1 |

`test_curation_parse_wrong_first_column_returns_first_column_not_iri`

| Header | Fault |
|--------|-------|
| `id,a` | `found: "id"` |
| U+FEFF then `iri,a` | `found: "\u{feff}iri"` |
| `Teaser,Teaser` | `found: "Teaser"` |

`test_curation_parse_bad_column_names_return_malformed_column`

| Header | Fault |
|--------|-------|
| `iri,Caption` | `column: "Caption"` |
| `iri,caption@` | `column: "caption@"` |
| `iri,@de` | `column: "@de"` |
| `iri,a@b@c` | `column: "a@b@c"` |
| `iri,1st` | `column: "1st"` |
| `iri,` | `column: ""` |
| `iri,caption@de-CH` | `column: "caption@de-CH"` |
| `iri,Teaser,Teaser` | `column: "Teaser"` (the first fault; no `DuplicateColumn`) |

`test_curation_parse_repeated_columns_return_duplicate_column`

| Header | Fault |
|--------|-------|
| `iri,slug,slug` | `column: "slug"` |
| `iri,caption@de,caption@de` | `column: "caption@de"` |
| `iri,#note,#note` | `column: "#note"` |
| `iri,iri` | `column: "iri"` |

`test_curation_parse_empty_lines_return_blank_line`

| Input | Fault |
|-------|-------|
| `iri⏎⏎` | line 2 |
| `iri,a⏎X,v⏎⏎Y,v⏎` | line 3 |

`test_curation_parse_uneven_rows_return_wrong_cell_count`

| Input | Fault |
|-------|-------|
| `iri,a,b⏎X,v⏎` | `expected: 3, found: 2`, line 2 |
| `iri,a⏎X,v,w⏎` | `expected: 2, found: 3`, line 2 |
| `iri,a,b⏎X, v⏎` | `expected: 3, found: 2` (not `PaddedValue`) |

`test_curation_parse_rows_for_other_iris_return_unknown_resource`

| Row | Fault |
|-----|-------|
| `U,v` | `iri: U` |
| `,v` | `iri: ""` |
| `U, v` | `iri: U` (not `PaddedValue`) |

`test_curation_parse_second_row_for_an_iri_returns_duplicate_row`

| Input | Fault |
|-------|-------|
| `iri,a⏎X,v⏎Y,v⏎X,w⏎` | `iri: X, first_line: 2`, line 4 |

`test_curation_parse_padded_values_return_padded_value` (the cell under `a`; each gives `column: "a"`)

| Cell |
|------|
| space + `v` |
| `v` + space |
| three spaces |
| `v` + U+00A0 |
| U+200B + `v` |
| `v` + U+FEFF |
| `" a"` (quoted) |
| and under the header `iri,caption@de`, a padded cell gives `column: "caption@de"` |

`test_curation_parse_returns_the_first_fault_only`

| Input | Fault |
|-------|-------|
| `iri,A⏎U, v⏎` | `MalformedColumn`, line 1 |
| `iri,a⏎X, v⏎U,v⏎` | `PaddedValue`, line 2 |
| `iri,slug⏎X,a"b⏎Y,` + tab + `c⏎` | `StrayQuote`, line 2 (not the tab of line 3) |

### Incunabula's file (`areas/access/sync/data/0803-curation.csv`)

Derived once, mechanically, from the incubator's files at commit `6e8e4063b54603f4615a6ed96025362a37dae70b`
(`origin/main` on 2026-10-09). From then on it is edited by hand.

Header: `iri,slug,office,date_display@de,title_override,cover_page,sort_key,teaser@de,teaser@en,keep,lang,name,#note`

The sources are three CSV files under `cpe/projects/incunabula/raw/` and two constants of
`cpe/projects/incunabula/translate.py`, which is one directory up.

| Source | Column | Becomes |
|--------|--------|---------|
| `raw/curation.csv` | `dsp_id` | `iri`: `http://rdfh.ch/0803/<dsp_id>` |
| | `id` | `slug` |
| | `date_display` | `date_display@de`, verbatim (decision 7) |
| | `office`, `title_override`, `sort_key` | the same key, verbatim |
| | `cover_page` (a `pages.csv` id) | `cover_page`: that Page's IRI, through `raw/pages.csv` |
| | `teaser_de`, `teaser_en` | `teaser@de`, `teaser@en` |
| `raw/annotation-curation.csv` | `id` | `iri`: `http://rdfh.ch/0803/<id>` |
| | `keep`, `lang`, `name` | the same key, verbatim |
| | `note` | `#note` |
| | `kind` | dropped: it restates the resource's class (77 `annotation` are Regions, 40 `link` are LinkObjs) |
| `translate.py:1030` | `UNSIGNED_STRIP_ID` | `slug` = `strip-38` on `http://rdfh.ch/0803/EJZQcYisXECHxchG_KQ27w` (decision 6) |
| `translate.py:1031` | `UNSIGNED_STRIP_NAME` | `name` = `Randleiste 38` on the same Band |

- Rows: the 19 Books in `curation.csv` order, the 117 annotations in `annotation-curation.csv` order, then
  the Band. 138 lines with the header.
- The Band's row is written from the three literals above; the script reads no file for it.
- `raw/pages.csv` has no IRI column. A Page's IRI is recovered from its `ark` cell: take the last path
  segment, drop the check digit (the last character) and turn every `=` into `-`.
- Key names follow the source columns, apart from `id` → `slug`, the two teasers and the tag on
  `date_display`. Renaming more is a curation decision, not part of a mechanical port.
- Three rows, as the file must hold them:
  - `http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ,bereitung,amerbach,1489,,http://rdfh.ch/0803/s-hBmajpVteY3fYcRjsJIQ,,Der umfangreichste Band des Bestandes; drei wiederverwendete Holzschnitte sind darin markiert.,"The largest volume in the corpus, with three re-used woodcuts marked across its pages.",,,,`
  - `http://rdfh.ch/0803/0JJDCMvsV_e8ZQ5C-icf3w,,,,,,,,,yes,,,"questionable, kept for the owner (D7)"`
  - `http://rdfh.ch/0803/EJZQcYisXECHxchG_KQ27w,strip-38,,,,,,,,,,Randleiste 38,`
- The one-off script is not committed: `PROVENANCE` records the mapping above, and the file has one
  writer from then on, a person.

### Facts (checked during planning, 2026-10-09)

Checked with Python's `csv.DictReader` over the incubator's files at `6e8e4063` and a line scan of
`areas/access/sync/data/0803.nq` at `8a680637`.

**Counts**

| Key | Values | On |
|-----|--------|----|
| `slug` | 20 | every Book (19) and the Band `EJZQcYisXECHxchG_KQ27w` |
| `office`, `date_display@de`, `cover_page` | 19 each | every Book |
| `title_override` | 4 | Books |
| `sort_key` | 3 | Books |
| `teaser@de`, `teaser@en` | 4 each | the same 4 Books |
| `keep` | 117 (99 `yes`, 18 `no`) | every Region (77) and LinkObj (40) |
| `lang` | 97 (93 `de`, 4 `en`) | kept Regions and LinkObjs |
| `name` | 35 | 34 LinkObjs and 1 Band |
| **Total** | **341** | **137 resources** |

- As a map of key and language to count, eleven entries over ten keys: `cover_page` 19,
  `date_display@de` 19, `keep` 117, `lang` 97, `name` 35, `office` 19, `slug` 20, `sort_key` 3,
  `teaser@de` 4, `teaser@en` 4, `title_override` 4.
- The Band's two values are `slug` = `strip-38` and `name` = `Randleiste 38`.
- The 18 `keep` = `no` are 13 Regions and 5 LinkObjs. None of them has `lang` or `name`.
- Two kept Regions have no `lang`: `0JJDCMvsV_e8ZQ5C-icf3w` and `fh-snkNvV9eEOCzY2m2dMw`.
- `office`: `amerbach` 5, `furter` 4, `other` 4, `bergmann` 3, `ysenhut` 3.
- `name`: 30 × `identischer Holzschnitt`, and `im selben Band gebunden` (`E7VBj9uBXKyrhvFLEc7Zrg`),
  `Übersetzung` (`Z6pN6v4FUSOdLc_ghfCfng`), `gleiche Druckermarke` (`_j3T1ZACWzGNux_T6tlnCA`),
  `Holzschnitte identisch?` (`oinJvxRSWC6VCHoXjEHrtA`), `Randleiste 38` (the Band).
- `#note` is filled on 35 annotation rows.
- The four `en`: `GgE0hrMoUpWc4VAW299D6Q`, `O-R69zOkWRyCUXE7d5k-PA`, `YFEZah_aUQq6IilZwtRGIQ`,
  `zFblYIEZXqauJirfO-7D8A`.

**Shape**
- Every resource IRI in `0803.nq` is `http://rdfh.ch/0803/<id>`. All 19 `dsp_id` are Books there, the 77
  `annotation` ids are Regions and the 40 `link` ids are LinkObjs.
- Exactly one Band has the label `[missing]`: `http://rdfh.ch/0803/EJZQcYisXECHxchG_KQ27w`.
- No curated cell has leading or trailing white space, a control character or a double quote. 15 cells
  hold a comma. 14 are not ASCII. The longest is 114 characters.
- No new check fires on the derived file.

**Title overrides and sort keys**

| Book | `title_override` | `sort_key` |
|------|------------------|------------|
| `cpQ3-JfqVZOkd7hUQ26kTg` | — | `Narrenschiff (dt.)` |
| `KyeQjCqTXLqLdFcRkEO9Rw` | `[Das] Narrenschiff (lat.) [Aug. 1497]` | `Narrenschiff (lat.) [Aug. 1497]` |
| `oZyOub3jUm2H0AGmZL3tyQ` | `[Das] Narrenschiff (lat.) [März 1497]` | `Narrenschiff (lat.) [März 1497]` |
| `g3cP7N0-XuGSRFI52RIvig` | `Zeitglöcklein des Lebens und Leidens Christi [1490]` | — |
| `70aWaB2kWsuiN6ujYgM0ZQ` | `Zeitglöcklein des Lebens und Leidens Christi [1492]` | — |

The four Books with teasers: `CDYZPN5zVVKbIcjA1DZxKQ`, `cpQ3-JfqVZOkd7hUQ26kTg`, `2B-ew2G6Vua3qoLmH9_5nw`,
`70aWaB2kWsuiN6ujYgM0ZQ`. The first one's English teaser holds a comma: `The largest volume in the corpus,
with three re-used woodcuts marked across its pages.`

**Cover pages.** Each is a Page whose only `isPartOf` parent is its Book. The label is the Page's
`rdfs:label` in `0803.nq`; it equals the `page_num` the incubator's `raw/README.md` ("Cover picks")
records for the pick, which is a hand-written list and so an independent check of the slug-to-IRI step.

| Slug | Book | Cover Page | Page label |
|------|------|------------|------------|
| `bereitung` | `CDYZPN5zVVKbIcjA1DZxKQ` | `s-hBmajpVteY3fYcRjsJIQ` | `a1r, Titelblatt, recto` |
| `brandan` | `PZcNpxILXBuKqGfglAZ4Vg` | `KOzqPpNTVrWUrnTPu5iDSA` | `a1r` |
| `de-generatione` | `qEKDZ8HuX2GEmsd-h9WIVQ` | `aQ7om2V5W4SrxDLY-nH9Ug` | `a2v` |
| `itinerarius-peregrinarius` | `OkoKj6FMWAOFloB5e5MzuQ` | `1rymDqQlV5iKvmD0FuRSiQ` | `a1r, Titelblatt` |
| `itinerarius-peregrinatio` | `9J7PlFFYWeS9pCADladXbw` | `iul24DMZVqyNSyNC1kiZWQ` | `1 recto` |
| `lob-der-glieder` | `L9wVqnhfXA-7c5qmKBTylg` | `oN9UfbVIVmqgtjSOQasFvg` | `a1r, Titelblatt` |
| `melusine` | `KM3SkDVOU76UayyT8QuB_Q` | `3D-FlcdLWg63ZE3UfJt2WA` | `5` |
| `methodij` | `lkpFkuy1UUW5vB4SLqoGEQ` | `QO6RxSpSUZOZm3sJS2qkLg` | `A1r; Titelblatt, recto` |
| `narrenschiff-dt` | `cpQ3-JfqVZOkd7hUQ26kTg` | `Lt_cBhxfVwyZpz6i_hc8vw` | `b1v` |
| `narrenschiff-lat-august` | `KyeQjCqTXLqLdFcRkEO9Rw` | `cl31Ych4WHS4MLgyDyXU4A` | `a1r; Titelblatt, recto` |
| `narrenschiff-lat-maerz` | `oZyOub3jUm2H0AGmZL3tyQ` | `V50dRbdfV4qdIIgX8rfY8Q` | `a1r; Titelblatt, recto` |
| `orationes` | `6jKWDQbjU-K2n6VNEjl8sA` | `Q5USL-JbXwKW017d4zSqBQ` | `a1r, Titelblatt, recto` |
| `passio-meynrhadi` | `yHkPDLw2V6KlD_ySoaN3Iw` | `Mr6WXCctUFOUFpwbtCxopA` | `a1r, Titelblatt` |
| `postilla` | `2B-ew2G6Vua3qoLmH9_5nw` | `-34FYx0jVMGTEi8aItQFxQ` | `a1r; Titelblatt` |
| `quadragesimale` | `f6pSYFILX1OT1uqXBXotnA` | `IrW_TAe7Wg6k_gbkK30_lQ` | `a1r; Titelblatt recto` |
| `reise` | `OjQkb5b8Xxq4p24nOQigdg` | `P6tmCLeJW-2LrDnW-1A40g` | `6r` |
| `walfart` | `RIoP4kEzWiWPz8pj3Ci9ew` | `fueeXbXCWTCSMywfMZ8QBw` | `a1r, Titelblatt` |
| `zeitgloecklein-1490` | `g3cP7N0-XuGSRFI52RIvig` | `IkREiZlDWuG8q_ofPJ8dBg` | `a1r` |
| `zeitgloecklein-1492` | `70aWaB2kWsuiN6ujYgM0ZQ` | `_qXHiyf6WsOfLFEXr7Zdng` | `a1r, Titelblatt` |

### Handed to DEV-7402

Nothing below is built here. The list is written so that it can be posted to DEV-7402 as it stands (H4).

**Project rules, the reader's to check** (line numbers are `dsp-incubator/cpe/projects/incunabula/translate.py`
at `6e8e4063`):
- Every Book has exactly one `slug`, and slugs are unique (`index_by_dsp_id` at `:509-520`, the join check
  at `:523-537`, `:677-678`, and the id-collision check from `:963`). `translate.py` checked the join both
  ways (`:526-535`): a row naming no Book, and a Book with no row. The adapter's `UnknownResource` covers the first; the second is the reader's.
- The Band `EJZQcYisXECHxchG_KQ27w` has the curated `slug` `strip-38` (decision 6). The other 37 Bands
  have none, and the hook derives their ids from their labels as `cpe-export` does.
- `office` is set and is one of the five lanes (`:664-675`).
- `date_display` is set, in German: the file serves it as `date_display@de` (decision 7), so the hook
  takes the language from the value (`:679-680`).
- `cover_page` is set, names one of the Book's own Pages, never a `[missing]` Page, and no two Books
  share one (`:927-961`). The port serves the Page's IRI; the reader turns it into the CPE page id it
  derives for that Page.
- A sort key, curated or defaulted to the display title, starts with an ASCII character (`:727-739`).
- No curated cell holds `http` (`:691-693`; `translate.py` exempted only `catalogue_uri` and `ark`).
  `cover_page` now holds an IRI by design, so the reader's rule exempts it and checks that it resolves.
- No curated cell holds the `[missing]` sentinel (`:696-717`; `MISSING_SENTINEL` at `:439`).
- `teaser` is set in both languages or in neither (`:741-746`).
- Every Region and LinkObj has `keep`, and it is `yes` or `no` (`:1266-1307`).
- `lang` is `de`, `en` or absent (`:1283-1287`).
- `name` is allowed on a LinkObj and on the Band above, never on a Region; a kept link without `name`
  falls back to its comment (`:1288-1289`, `:1411-1415`). A link's `name` is untagged; its language is
  the row's `lang`.
- The census after curation: 64 annotations, 35 links, 69 link targets (`:1430-1480`).
- An unknown key, and a key on a resource of the wrong class, are errors of the reader, because the
  port's keys are opaque. This per-class rule replaces the dropped `kind` column, which paired a row
  with an exported annotation or link (`:1279-1300`): `keep` and `lang` belong to Regions and LinkObjs.

**Already covered by the adapter**
- A second row for one resource (`translate.py:1275`, `:509-520`) is `DuplicateRow`.
- A row for something that is no resource of the project is `UnknownResource`.
- A padded `sort_key` (`:730-734`) is `PaddedValue`, as is any padded value.
- `#note` is not served, and nothing downstream reads it: `translate.py` never emitted `note`.

**Project-wide configuration, for Incunabula's KDL**
- The office vocabulary with its labels and order (`:252-263`).
- The side vocabulary (`:278-282`).
- The rights constants of the scans (`:209-211`).
- The annotation languages (`:1252`).

**Open points**
- `raw/catalogue-overrides.csv` is not served.
- A curation name is lower-case, so a language tag such as `de-CH` cannot be a column's language. If a
  project needs regional tags, `is_curation_name` changes, in the port.
- The incubator vendors `cpe-ports` only. Reading `0803-curation.csv` there needs `sync-store` and
  `areas/access/sync/data/`, and `cpe/tools/vendor.sh` cannot vendor a directory that is not a crate
  (`cpe/vendor/README.md`, "Adding a crate").
- When DEV-7402 lands, `raw/curation.csv` and `raw/annotation-curation.csv` are deleted, and the
  comparison of its store against `data.sql` is the check that the two copies did not drift.

## Alternative Approaches Considered

- **A second port method, `curation(shortcode)`.** It keeps `ProjectSnapshot` purely archive-shaped.
  Rejected: curation names resources of one snapshot, and two calls can read two states of the files, so
  "whole or refused" would have to span two calls and the contract could not check a dangling value.
- **A field on `Resource`.** A dangling value becomes impossible. Rejected: it puts authored input inside
  the DTO that restates the archive, and it breaks six struct literals that name every field of
  `Resource` instead of four `ProjectSnapshot` literals. (The other `Resource { … }` expressions under
  `areas/` spread from a helper with `..resource(…)` or `..page(…)` and do not break. The six are `contract.rs:409`, `fake.rs:66`, `ark_tests.rs:110`, `mapping/resources.rs:79`
  and `mapping_tests.rs:191` and `:1092`.)
- **Typed Incunabula fields** (`BookCuration { slug, office, … }`). Rejected by decision 3: `sync` would
  know one project, and every new key would be a port change and a re-vendor.
- **A curation graph in N-Quads**, read with the parser `sync-store` already has. Language tags and IRIs
  come for free. Rejected: nobody edits 341 quads by hand, and a table is what the editors have today.
- **TOML.** Rejected: 137 tables of repeated keys read and diff worse than rows, and it needs a parser
  dependency.
- **A directory of CSV files**, and **the `csv` crate**: see *The curation file* and *Reader*.
- **Every fault of a file in one run.** Friendlier to an editor with several mistakes. Rejected for now:
  it needs an order among faults, rules for what a broken row still counts for, and tests for both. The
  file is edited by developers under CI, where one fault per run is enough.
- **Amendments to ADR-0007 and ADR-0008** instead of a new record. Rejected by ADR-0006: the decision
  changes, it does not narrow (decision 8).
- **Curation in the project's folder, read by CPE.** This is the question DEV-7488 answered the other
  way for now: the read model would have a second feed (ADR-0008).

## Technical Considerations

**The ADRs: what this plan changes.** Five earlier statements say the port carries archive facts only.

| Where | What it says |
|-------|--------------|
| `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md:30-34` | "The port speaks archive-shaped facts; the remodelling lives with the reader." The port serves the projection as the archive records it, "never the consumer's model", and `sync` "knows nothing of what the reader builds" |
| the same file, `:38-39` | A read model "carries no fact the projection does not" |
| `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md:25-33` (the sentence at `:28-30`) | "The port speaks archive-shaped facts as the archive records them (classes, typed values, links, files, ordered membership), never CPE's presentation model" |
| `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md:368` | Acceptance criterion: "No DTO names a CPE presentation concept" |
| `docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md:254` | "No presentation remodelling in `sync`: it maps to archive-shaped DTOs only (ADR-0007)" |

- **Their reason.** Both ADRs reject the option that `sync` serves a consumer's shape (ADR-0008 `:54-56`,
  ADR-0007 `:88-90`): the remodel would then live in `sync`, every reader or project would edit `sync` to
  change how it reads, and `sync` would become the second writer of the reader's model.
- **What changed.** Incunabula's store needs input the archive does not record: which test Regions are
  hidden, what a Book's public URL is. ADR-0008 `:25-29` makes the port the only feed, and a file CPE
  reads beside the port would be a second feed. Of the two rules that now pull against each other,
  DEV-7488 keeps "the only feed" and loosens "archive-shaped facts only", as an interim.
- **What stops holding.** A slug or a teaser is presentation data. The port now carries something that
  is not a fact of the archive, and the read model carries something the projection does not. The port
  plan's criterion no longer holds for `CuratedValue`; it holds for every other DTO.
- **What still holds, and why.** The reason behind the rejected option is kept. `sync` holds the values
  but gives no key a meaning and remodels nothing. Changing how a project is presented means editing a
  data file and the project's KDL, never `sync`'s code. The reader stays the read model's only writer and
  the port its only feed. Project-wide configuration stays in the project's folder. The sync PRD's
  sentence holds as written.
- **How it is recorded: ADR-0010, `proposed`.** A new file,
  `docs/adr/0010-the-port-also-serves-per-resource-curation.md`. 0010 is the next free number: the series
  ends at `0009-datei-joins-the-access-area-as-its-third-capability.md`. Its form follows ADR-0008 and
  ADR-0009: frontmatter `status` and `date`, a title, the context and the decision as bullets with bold
  leads, `## Considered Options`, `## Consequences`, and a closing `Enforced by:` line. Every record of
  the series is `status: accepted` today, so `proposed` is new to it.
- **What ADR-0010 says.** The port also serves per-resource curation, as an interim under DEV-7488. It
  cites ADR-0007 and ADR-0008, names the sentence of each it changes (the lines above) and restates their
  reason. It says what stays true, as listed above. Its options are the alternatives of this plan that
  bear on the decision. It states the line between curation and configuration and asks Ivan to confirm
  it. It says that Ivan accepts or rejects it under DEV-7488, and that a rejection removes the record
  together with the code.
- **ADR-0007 and ADR-0008 are not edited.** ADR-0006 `:24-25` gives a changed record
  `status: superseded by`, which fits neither: one sentence of each changes and every other clause stands.
  ADR-0010's Consequences therefore say what its acceptance entails: `status: accepted` there, and a dated
  `## Amendment` in ADR-0007 and in ADR-0008 that points to it, in a follow-up PR (H3). Nothing of that is
  done for a proposal. The two specs are records of their time and are not edited either.
- **Where a new ADR is registered.** `docs/src/decisions.md` lists the root series in a table (ADR-0009's
  row is `:37`); it gains a row. `check-adr-refs.sh`, part of `just check`, needs every cited `ADR-NNNN`
  to resolve to a file and nothing else, so `ADR-0010` may be cited once the file is staged. There is no
  other index: `docs/adr/` has no README.

**Consumers**
- `ProjectSnapshot` struct literals that need `curation` (grep, 2026-10-09):
  - `areas/access/cpe/ports/src/contract.rs:448`, the `snapshot()` test helper; `valid_snapshot()` and
    every test go through it. A test with curation spreads from it: `ProjectSnapshot { curation, ..snapshot(…) }`.
  - `areas/access/cpe/ports/src/fake.rs:64`, `book_snapshot()`.
  - `areas/access/sync/store/src/ark_tests.rs:123`.
  - `areas/access/sync/store/src/mapping/mod.rs:37`, the production site.
- `mapping_tests.rs` and `tests/committed_0803.rs` build no `ProjectSnapshot`.
- In the incubator nothing outside `cpe/vendor/` uses `cpe_ports`. Phase 4 greps again.

**Tests**
- Names follow `test_{what}_{condition}_{expected}` with each module's prefix: `test_violations_…`,
  `test_display_…`, `test_curation_…`, `test_snapshot_…`, `test_committed_0803_…`.
- The format tests are a new `#[cfg(test)]` module, `src/curation_tests.rs`, like `ark_tests.rs`
  (`lib.rs:109-114`). Each is one of the *Test tables*: it calls `curation::parse` on every row's bytes
  with the set of known IRIs and asserts the exact outcome, and needs no directory. A fault row asserts
  variant, fields and line.
- Only the file-level tests go through `LiveArchiveProjection` on a temporary directory.
- `write_0803` (`snapshot_tests.rs:10-13`) is the crate's one fixture helper, with about 120 call sites.
  It keeps its name and also writes a header-only `0803-curation.csv`, so the existing tests keep
  serving; its doc says that it writes both files. A new helper beside it, `write_0803_curation`,
  overwrites that file for the file-level curation tests.
- A missing-file test asserts the path of the `Read` error. Without that, a missing curation file and a
  missing snapshot file could not be told apart (`snapshot_tests.rs:148`, `:163`).
- Counts are literal constants from *Facts* and are never relaxed
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`). They come from the incubator's
  CSVs, read with a different tool, never from `curation.rs`'s own output.
- The committed tests know Incunabula's keys; the adapter does not. The test file's module doc says so.
- Every phase ends green: its last deliverable before the phase review leaves `just check && just test`
  passing (in Phase 4, `just cpe ci`).
- A `grep` that finds nothing exits 1. A step that wants "no hit" is therefore written `! grep -q …`,
  which exits 0 exactly then.
- One record per line is a rule of the format: a quoted cell cannot cross a line, so a multi-line value
  cannot pass a line-wise check
  (`dsp-incubator/cpe/docs/learnings/logic-errors/shell-input-checks-with-line-wise-grep-accept-multi-line-values.md`).

**Two copies until DEV-7402.** `translate.py` keeps reading the incubator's CSVs, and they stay
authoritative for the public prototype. `0803-curation.csv` is the copy DEV-7402 builds from. A curation
change before DEV-7402 lands is made in the incubator and repeated by hand here, in a dsp-repository PR.
The sources are compared with the pinned commit twice: Phase 2 does it before deriving, Phase 4 after the
merge. On a difference either one stops and reports the phase as blocked. In Phase 2 that is because every
count and literal of this plan is a fact of the pinned files, and re-deriving them is a change of the plan
for the owner to see; in Phase 4 because the merged file can then only be changed by a new dsp-repository
PR. Both compare the three CSV files whole: `pages.csv` has cells that span lines, so a
narrower comparison would need a CSV-aware script. After that nothing checks the two against each other
until DEV-7402's comparison does; `PROVENANCE`, the incubator's `raw/README.md` and
`INCUNABULA_GO_LIVE.md` each say so (Phases 2 and 4).

**Superseded 2026-10-10:** dsp-incubator#500 (DEV-7496, the first part of DEV-7402) builds Incunabula's
store from the port and deleted `translate.py` and the incubator's CSVs. `0803-curation.csv` is from then
on the only copy of Incunabula's curation: a change is made here and reaches the incubator when it
re-vendors. `PROVENANCE`, `areas/access/sync/CONTEXT.md` and `ARCH-MAP.md` say so.

**Commits (dsp-repository)**
- In every command, `<dsp-repository path>` and `<dsp-incubator path>` are the two `repositories:` paths
  of this plan's frontmatter.
- Branch `feature/dev-7495-serve-cpes-project-curation-through-sync-and-cpe-ports`, on `origin/main`. The
  planning session has pushed it and opened draft PR #468, assigned to Balduin. Its body has the sections
  Motivation, Summary, Test Plan, Human Actions and Commit hygiene, with `allow-many-commits` ticked.
- **Precondition:** the worktree is clean before Phase 1. The planning session amends the plan commit to
  this text and removes the backup copy beside it.
- The PR carries two commits, as #464 and #465 did:
  - this plan, already committed: `docs(docs): add the plan for serving curation through cpe-ports (DEV-7495)`;
  - one feature commit: `feat(cpe-ports,sync-store): serve project curation through the port (DEV-7495)`.
- Phase 1 creates the feature commit, ADR-0010 in it. Phases 2 and 3 and every review fix are amended
  into it, which works because it is `HEAD` from then on.
- **Stage by path, never `git add -A`.** Ticks in this plan stay out of the feature commit: the precedent
  `f73009e2` touched no file under `docs/specs/`. They stay unstaged while the plan runs and land
  afterwards in a `docs(docs): close out …` commit, as `8a680637` did for the two precedents. That
  commit is the session's, after ship, and no deliverable of a phase.
- An amended commit has no "this phase's commits". Each dsp-repository phase therefore starts by printing
  `git rev-parse HEAD`; that sha is the phase's base, and the phase review reads `git diff <base> HEAD`.
  The old commit stays in the object store after an amend, so the diff works.
- Every dsp-repository phase ends with a push (`git push --force-with-lease`), so the draft PR shows the
  work as it grows and ADR-0010 can go to Ivan after Phase 1. The push comes before the phase review,
  which is the phase's last step. A fix from that review is amended into the feature commit and pushed
  again the same way before the phase is reported done.
- `just commit-lint` reads the PR body from the environment (`.github/scripts/check-commit-count.sh:22`,
  `:31`), and without it two commits fail the count. The command that passes, with exit status 0, is
  `PR_BODY="$(gh pr view 468 --repo dasch-swiss/dsp-repository --json body -q .body)" just commit-lint`.
- No `CHANGELOG.md` edit: release-please writes it.

**Incubator**
- `cpe/vendor/PIN` is `f73009e2` (DEV-7487). Re-vendoring is described in `cpe/vendor/README.md` ("Moving
  the pin"): `just cpe vendor <sha>`, then `just cpe test`; commit `PIN`, the tree and any `Cargo.lock`
  change together; never `cargo fmt` in `cpe/`; the pin is a commit on the monorepo's `main`.
- Every git command names the worktree: `git -C <dsp-incubator path> …`. `just` recipes run from the
  worktree's root in one shell invocation (`dsp-incubator/CLAUDE.md`).
- PRs land as one squashed commit titled as a conventional commit. The PR is a draft assigned to
  Balduin; auto-merge is never enabled. A fix from Phase 4's review is a further commit on the branch,
  pushed before the phase is reported done; the squash merge folds it in.
- An empty `vendor-diff` means the vendored tree is the code reviewed in the dsp-repository PR. Phase 4's
  phase review therefore has only the edits to the two documents to review, and it still runs.
- No `cpe/CONTEXT.md` term and no `cpe/docs/ARCHITECTURE.md` or `PROJECT_PLAN.md` edit: nothing in the
  incubator reads the port's curation before DEV-7402, and none of the three says how curation reaches
  the store (`PROJECT_PLAN.md` points to `INCUNABULA_GO_LIVE.md`).

**Reviewers**, for each phase review:
- `eng:review:rust-reviewer`
- `eng:review:dune-reviewer`
- `eng:review:consistency-reviewer`
- `eng:review:code-simplicity-reviewer`

## Implementation Phases

#### Phase 1: ADR-0010, and `cpe-ports` carries curation

### dsp-repository
- [x] Run `git rev-parse HEAD` and keep the sha as this phase's base; the phase review reads `git diff <base> HEAD`
- [x] Write `docs/adr/0010-the-port-also-serves-per-resource-curation.md`, `status: proposed`, `date: 2026-10-09`, with the content and form *Technical Considerations*,
      "The ADRs" gives; it is done when it has the decision bullets, `## Considered Options`, `## Consequences` and an `Enforced by:` line that names
      `cpe_ports::contract::violations` and `sync-store`'s tests
- [x] Add ADR-0010's row to the root-series table of `docs/src/decisions.md`, after ADR-0009's (`:37`), its title marked "(proposed)"
- [x] Add `CuratedValue` to `snapshot.rs`, with its derives and docs as above
- [x] Add `ProjectSnapshot.curation: Vec<CuratedValue>` after `list_nodes`, documented as above
- [x] Extend `snapshot.rs`'s module doc (`:1-9`): the snapshot also carries curation, which is not an archive fact
- [x] Extend `ProjectSnapshot`'s doc (`snapshot.rs:66-68`) the same way
- [x] Extend `cpe-ports`' crate doc (`lib.rs:1-6`) the same way
- [x] Extend the `ArchiveProjection` docs (`lib.rs:26-35`) the same way
- [x] Re-export `CuratedValue` from `lib.rs` (`:21-24`)
- [x] Add `contract::is_curation_name`, public, documented as above
- [x] Write `contract.rs` test `test_is_curation_name_accepts_lowercase_names_and_refuses_the_rest`: it accepts `slug`, `date_display`, `sort-key`, `de` and `a1`, and
      refuses the empty name, `Slug`, `1st`, `_a`, `-a`, `a b`, `a@b`, `de-CH` and `ü`
- [x] Write `contract.rs` test `test_curated_values_sort_by_resource_key_and_language`: three values of one resource sort as untagged, `de`, `en`, after a value of a
      smaller resource IRI
- [x] Write `contract.rs` test `test_violations_padded_curation_text_reports_nothing`: a non-empty `text` with white space at
      its ends is no violation
- [x] Set `curation: vec![]` in `contract.rs`'s `snapshot()` helper (`:448`)
- [x] Set `curation: vec![]` in `ark_tests.rs:123`
- [x] Set `curation: Vec::new()` in `mapping/mod.rs:37`, with a doc line on `map`: the quads hold no curation
- [x] Give `fake.rs`'s `book_snapshot()` (`:64`) two curated values in non-alphabetical order (`teaser` in `en`, then `slug`), so the unchanged-snapshot test covers their
      order
- [x] Declare `Violation::DanglingCuration` after `LinkWithValueUuid`, with its `Display` arm
- [x] Declare `Violation::DuplicateCuration` after it, with its `Display` arm
- [x] Declare `Violation::MalformedCuration` after that, with its `Display` arm
- [x] Write `contract.rs` test `test_violations_curation_for_missing_resource_reports_one_dangling_curation` (two values for one missing IRI, one violation)
- [x] Write `contract.rs` test `test_violations_curated_value_given_three_times_reports_one_duplicate_curation`
- [x] Write `contract.rs` test `test_violations_empty_curated_text_reports_malformed_curation`
- [x] Write `contract.rs` test `test_violations_empty_curation_key_reports_malformed_curation`
- [x] Write `contract.rs` test `test_violations_empty_curation_language_reports_malformed_curation` (`lang: Some("")`)
- [x] Write `contract.rs` test `test_violations_uppercase_curation_key_reports_malformed_curation`
- [x] Write `contract.rs` test `test_violations_malformed_value_given_twice_reports_two_malformed_and_one_duplicate`
- [x] Write `contract.rs` test `test_violations_one_key_in_two_languages_reports_nothing`
- [x] Write `contract.rs` test `test_violations_one_key_tagged_and_untagged_reports_nothing`
- [x] Write `contract.rs` test `test_violations_one_key_on_two_resources_reports_nothing`
- [x] Write `contract.rs` test `test_display_dangling_curation_names_resource`
- [x] Write `contract.rs` test `test_display_duplicate_curation_shows_key_and_language` (`teaser@de` for a tagged value, `slug` for an untagged one)
- [x] Write `contract.rs` test `test_display_malformed_curation_quotes_empty_key` (the message shows `""`)
- [x] Extend `valid_snapshot()` (`:451`) with curation on the book and on the region, one key in two languages among it, so
      `test_violations_valid_snapshot_reports_nothing` covers the checks
- [x] Add a `DanglingCuration`, a `DuplicateCuration` and a `MalformedCuration` case to the documented-order test (`:915`), listed out of order; they sort last, in that
      order
- [x] Implement the `DanglingCuration` check in `violations` (`:165-304`)
- [x] Implement the `DuplicateCuration` check in `violations`
- [x] Implement the `MalformedCuration` check in `violations`
- [x] Extend `contract.rs`'s module doc (`:1-13`): what it checks of curation, and that a key's meaning, a non-empty text and totality are the reader's
- [x] Run `just check && just test`; it passes
- [x] Commit the files of this phase, staged by path, as `feat(cpe-ports,sync-store): serve project curation through the port (DEV-7495)`
- [x] Push with `git push --force-with-lease`; a fix from this phase's review is amended and pushed again the same way before the phase is reported done
- [x] Edit the body of PR #468 with `gh pr edit 468 --repo dasch-swiss/dsp-repository`: keep every section; in Summary name ADR-0010 as proposed and link it; replace the
      Human Actions table with this plan's; leave `allow-many-commits` ticked
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: `sync-store` reads the curation file

### dsp-repository
- [x] Run `git rev-parse HEAD` and keep the sha as this phase's base; the phase review reads `git diff <base> HEAD`
- [x] Compare the curation sources with the pin: after `git -C <dsp-incubator path> fetch origin`,
      `git -C <dsp-incubator path> diff --quiet 6e8e4063b54603f4615a6ed96025362a37dae70b origin/main -- cpe/projects/incunabula/raw/curation.csv cpe/projects/incunabula/raw/annotation-curation.csv cpe/projects/incunabula/raw/pages.csv`
      exits 0, and `git -C <dsp-incubator path> show origin/main:cpe/projects/incunabula/translate.py` holds `UNSIGNED_STRIP_ID = "strip-38"` and
      `UNSIGNED_STRIP_NAME = "Randleiste 38"`. If either differs, stop: do not derive the file and do not continue this phase; report the
      phase as blocked, with the diff, so the owner can have the plan's pin and counts revised
- [x] Derive `areas/access/sync/data/0803-curation.csv` with a one-off script kept under `.claude/tmp/` (not committed): read `curation.csv`, `annotation-curation.csv`
      and `pages.csv` with `git -C <dsp-incubator path> show <pin>:cpe/projects/incunabula/raw/<file>`, parse them with `csv.DictReader`, apply the mapping table of
      *Incunabula's file*, append the Band's row from its three literals, and write with `csv.writer` (`lineterminator="\n"`, minimal quoting)
- [x] Check the file with plain commands, from the repository root, `<f>` being `areas/access/sync/data/0803-curation.csv`: `wc -l < <f>` prints 138; `head -1 <f>` prints
      the header literal of *Incunabula's file*; `grep -c '^http://rdfh.ch/0803/' <f>` prints 137; `! grep -q $'\r' <f>` exits 0; and `grep -cxF -f <rows> <f>` prints 3,
      `<rows>` being a scratch file that holds the three rows of *Incunabula's file* exactly
- [x] Append a `0803-curation.csv` section to `areas/access/sync/data/PROVENANCE`: the three CSV files under `cpe/projects/incunabula/raw/` and
      `cpe/projects/incunabula/translate.py:1030-1031`; the source as `commit <pin>`, all 40 characters, never in the `dsp-incubator@` form, which
      `test_committed_0803_provenance_pin_matches_vocab_pin` (`tests/committed_0803.rs:395`) reads as the `dao-lift` pin; the column mapping; that it was derived once and
      is edited by hand from then on; and that the incubator's CSVs stay authoritative until DEV-7402, a change there being repeated here by hand
- [x] Add `areas/access/sync/data/*-curation.csv text eol=lf` to the root `.gitattributes`, under a comment that says why;
      `git check-attr eol -- areas/access/sync/data/0803-curation.csv` then prints a line ending in `eol: lf`
- [x] Add `InvalidCuration` to `error.rs`, with the variants, field types and messages of the table above
- [x] Add `CurationFault` to `error.rs`, with its derives and `Display`
- [x] Add `SnapshotError::Curation { path, fault }` to `error.rs`
- [x] Widen `SnapshotError`'s doc (`error.rs:8-9`) to both files
- [x] Re-export `CurationFault` and `InvalidCuration` from `lib.rs` (`:47-48`)
- [x] Make `write_0803` (`snapshot_tests.rs:10-13`) also write `0803-curation.csv` holding `iri` and a line feed, and say in its doc that it writes both files
- [x] Add `write_0803_curation(dir, csv)` beside it, which overwrites that file
- [x] Assert the `Read` error's path ends in `0803.nq` in `test_snapshot_missing_file_returns_unavailable` (`snapshot_tests.rs:148`)
- [x] Assert the same in `test_snapshot_directory_in_place_of_file_returns_unavailable` (`snapshot_tests.rs:163`)
- [x] Extend `test_snapshot_every_known_shortcode_has_committed_file` (`snapshot_tests.rs:138`): every known shortcode also has `<shortcode>-curation.csv`
- [x] Create `src/curation_tests.rs` with a `known()` helper: the set of three IRIs the tables call `X`, `Y`, `Z`, sorting in that order
- [x] Write `test_curation_parse_values_returns_them_sorted` from its table
- [x] Write `test_curation_parse_empty_cells_are_absent` from its table
- [x] Write `test_curation_parse_text_is_verbatim` from its table
- [x] Write `test_curation_parse_invalid_bytes_return_not_utf8` from its table
- [x] Write `test_curation_parse_no_header_returns_missing_header` from its table
- [x] Write `test_curation_parse_refused_characters_return_control_character` from its table
- [x] Write `test_curation_parse_misplaced_quotes_return_stray_quote` from its table
- [x] Write `test_curation_parse_open_quotes_return_unterminated_quote` from its table
- [x] Write `test_curation_parse_wrong_first_column_returns_first_column_not_iri` from its table
- [x] Write `test_curation_parse_bad_column_names_return_malformed_column` from its table
- [x] Write `test_curation_parse_repeated_columns_return_duplicate_column` from its table
- [x] Write `test_curation_parse_empty_lines_return_blank_line` from its table
- [x] Write `test_curation_parse_uneven_rows_return_wrong_cell_count` from its table
- [x] Write `test_curation_parse_rows_for_other_iris_return_unknown_resource` from its table
- [x] Write `test_curation_parse_second_row_for_an_iri_returns_duplicate_row` from its table
- [x] Write `test_curation_parse_padded_values_return_padded_value` from its table
- [x] Write `test_curation_parse_returns_the_first_fault_only` from its table
- [x] Write `test_display_curation_error_shows_path_and_line`: `SnapshotError::Curation`'s message holds the path and `line 2: `
- [x] Write `test_snapshot_curation_row_for_served_resource_is_served`
- [x] Write `test_snapshot_curation_row_for_value_node_returns_unavailable_naming_path_and_line`: the error is `Unavailable`, its source `Curation` with a path ending in
      `0803-curation.csv` and `UnknownResource` at line 2
- [x] Write `test_snapshot_curation_file_changed_between_calls_serves_changed_values`
- [x] Write `test_snapshot_unknown_shortcode_with_curation_file_present_returns_unknown_project` (`0804.nq` and `0804-curation.csv` present)
- [x] Write `test_snapshot_missing_curation_file_returns_unavailable_naming_its_path` (`Read`)
- [x] Write `test_snapshot_directory_in_place_of_curation_file_returns_unavailable` (`Read` with that path)
- [x] Add `src/curation.rs` with `parse`, documented as above, using `contract::is_curation_name` for column names
- [x] Register `curation` and `curation_tests` in `lib.rs` (`:42-45`, `:109-114`)
- [x] Read the curation file and call `curation::parse` in `serve` (`lib.rs:65-81`), after the `Empty` check and before the contract, with the path
      `<dir>/<shortcode>-curation.csv`
- [x] Update `sync-store`'s crate doc (`lib.rs:1-33`): the curation file, an absent one being `Unavailable`, the keys being opaque, and in "Where a check goes" the rules
      the contract checks again
- [x] Update `LiveArchiveProjection`'s doc (`lib.rs:54-55`) to both files
- [x] Update the `KNOWN` comment in `snapshot` (`lib.rs:87-88`) to both files
- [x] Run, from the repository root,
      `! grep -rqE '"(slug|office|date_display|title_override|cover_page|sort_key|teaser|keep|lang|name|note)(@[a-z]+)?"' areas/access/sync/store/src --include="*.rs" --exclude="*_tests.rs"`;
      it exits 0, as it does on `8a680637`, so no code outside the tests names one of Incunabula's keys
- [x] Run `just check && just test`; it passes, `test_committed_0803_snapshot_passes_contract` among them, which now reads `0803-curation.csv`
- [x] Amend Phase 2's files, staged by path, into the `feat(cpe-ports,sync-store)` commit, which is `HEAD`
- [x] Push with `git push --force-with-lease`; a fix from this phase's review is amended and pushed again the same way before the phase is reported done
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: 0803's curation is pinned and documented

### dsp-repository
- [x] Run `git rev-parse HEAD` and keep the sha as this phase's base; the phase review reads `git diff <base> HEAD`
- [x] Extend `committed_0803.rs`'s module doc (`:1-3`): the curation expectations are facts of the incubator's CSVs at the commit `PROVENANCE` names, the tests know
      Incunabula's keys and the adapter does not, and an expectation changes only together with `0803-curation.csv`
- [x] Write a committed test: `curation` holds 341 values on 137 distinct resources
- [x] Write a committed test: the count per key and language equals the literal map of *Facts* (eleven entries, among them `slug` 20, `date_display` in `de` 19 and `name`
      35), so no other key or language is served
- [x] Write a committed test: every Book has exactly one `slug`; the Band `EJZQcYisXECHxchG_KQ27w` has `strip-38`; the 20 slugs are distinct; and no other resource has
      one
- [x] Write a committed test: `office` is on the 19 Books only, with `amerbach` 5, `furter` 4, `other` 4, `bergmann` 3, `ysenhut` 3
- [x] Write a committed test: every Book has exactly one `date_display`, tagged `de`, no other resource has one, and Book `PZcNpxILXBuKqGfglAZ4Vg` serves
      `19. Februar 1491`
- [x] Write a committed test: each Book's `cover_page` is the IRI of a `Page` whose `part_of` is that Book alone, and the 19 are distinct
- [x] Write a committed test: for each of the 19 slugs, the label of the Book's cover Page equals the literal of the *Cover pages* table
- [x] Write a committed test: every Region and LinkObj has exactly one `keep`, 99 `yes` and 18 `no`, the 18 being 13 Regions and 5 LinkObjs, and no other resource has one
- [x] Write a committed test: `lang` is served 93 times as `de` and 4 times as `en`, the four being the Regions of *Facts*, and never on a resource whose `keep` is `no`
- [x] Write a committed test: `name` is untagged on 34 LinkObjs and on the Band `EJZQcYisXECHxchG_KQ27w` (`Randleiste 38`, label `[missing]`), on no Region, with the five
      literal names of *Facts* and 30 × `identischer Holzschnitt`
- [x] Write a committed test: the four title overrides and the three sort keys of *Facts*, by Book
- [x] Write a committed test: the four Books of *Facts* have `teaser` in `de` and in `en`, no other resource has a teaser, and `CDYZPN5zVVKbIcjA1DZxKQ`'s English teaser
      is the literal with its comma
- [x] Write a committed test: the file's row for Region `0JJDCMvsV_e8ZQ5C-icf3w` ends in the note `questionable, kept for the owner (D7)`, and the Region serves exactly
      one value, `keep` = `yes`
- [x] Write a committed test: `curation` is sorted without a repeat, and a second, fresh call returns the same `curation`
- [x] Add a **Curation** term to `areas/access/cpe/CONTEXT.md` after **Data ARK** (`:27`): `cpe_ports::CuratedValue`, what a project's editors authored about one resource
      and the archive does not record; served beside the archive-shaped facts, never as one; its keys mean nothing to the port; interim (ADR-0010, proposed; DEV-7488);
      _Avoid_: configuration (project-wide, the project's KDL), archive fact
- [x] Update the **Project snapshot** term (`areas/access/cpe/CONTEXT.md:15`): resources, list nodes and curation
- [x] Add a row to "What the port serves" (`areas/access/cpe/CONTEXT.md:35-50`): In, per-resource curation as key, optional language and text; Out, project-wide
      configuration (vocabularies with labels and order, rights constants) and any meaning of a key; Why, the project's KDL declares configuration (ADR-0007) and only
      curation comes through `sync` (ADR-0010, proposed)
- [x] Add one sentence to the lead paragraph of "What the port serves" (`areas/access/cpe/CONTEXT.md:33`): curation is the one thing served that is not a fact of the
      archive
- [x] Add a **Committed curation** term to `areas/access/sync/CONTEXT.md` after **Committed snapshot** (`:23`): `data/<shortcode>-curation.csv`, hand-authored, single
      writer a person, opaque keys, a header-only file where a project has none; UTF-8 with LF line ends and no byte-order mark, which `.gitattributes` sets and the
      reader enforces
- [x] Fix `areas/access/sync/CONTEXT.md:25` ("It is everything `sync` holds of a project today"): `sync` holds the committed snapshot and the committed curation
- [x] Fix the intro of `areas/access/sync/CONTEXT.md` (`:3-10`): it serves the port from one committed snapshot and one committed curation file per known project, and
      Curation joins the inherited port terms
- [x] Update **Known project** (`areas/access/sync/CONTEXT.md:30-33`): a project becomes known with its shortcode, its committed snapshot and its committed curation
      together
- [x] Update **`LiveArchiveProjection`** (`areas/access/sync/CONTEXT.md:36`): it reads both files on every call, and a missing or faulty curation file refuses the
      snapshot
- [x] Update the `cpe-ports` row of `docs/src/repo_structure.md` (`:65`): the DTOs include curation
- [x] Update the `sync-store` row of `docs/src/repo_structure.md` (`:66`): the adapter serves the committed snapshots and curation
- [x] Read every hit of
      `grep -rnE "archive-shaped DTOs|as the archive records them|in the archive's shape|everything .sync. holds|one committed snapshot|committed snapshots? (of|in|under)|shortcode>\.nq|one file per" areas/access docs/src CONTEXT.md ARCH-MAP.md`
      (hits are expected; the exit status is not the check). End state: no hit says that the snapshot holds archive facts only, or that `sync` holds or reads the snapshot
      file alone; a hit about resources, values or the `.nq` file itself stays as it is
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry with `dune:dune-map` (this checkbox authorises the overwrite). Done when its Key entities name `CuratedValue`,
      `contract::is_curation_name` and the three violations, and its Purpose no longer says "Where the per-project configuration lives is an open item" but: the
      configuration is in the project's folder (ADR-0007) and per-resource curation comes through the port (ADR-0010, proposed)
- [x] Refresh `ARCH-MAP.md`'s `areas/access/sync` entry with `dune:dune-map` (this checkbox authorises the overwrite). Done when Paths and Purpose name
      `0803-curation.csv`; Key entities name `InvalidCuration` and `CurationFault`; the kit names `areas/access/sync/store/src/curation.rs`; Boundary rules say
      `sync-store` knows no curation key (ADR-0010, proposed); Durable state lists `0803-curation.csv`, **single writer** a person, hand-authored, never generated
      (**review**), read by `sync-store` on every call; and the Fingerprint is no longer `ec1861e4ca2c`
- [x] Run `just check && just test`; it passes
- [x] Amend Phase 3's files, staged by path, into the `feat(cpe-ports,sync-store)` commit, which is `HEAD`
- [x] Run `PR_BODY="$(gh pr view 468 --repo dasch-swiss/dsp-repository --json body -q .body)" just commit-lint`; it exits 0
- [x] Push with `git push --force-with-lease`; a fix from this phase's review is amended and pushed again the same way before the phase is reported done
- [x] Edit the body of PR #468 with `gh pr edit 468 --repo dasch-swiss/dsp-repository`: keep every section; rewrite Summary and Test Plan to what landed; add Review Notes
      for the two commits; leave the Human Actions table and the ticked `allow-many-commits` as they are
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: The incubator vendors `cpe-ports` with curation

**Gate: H1** — resolve before starting this phase.

### dsp-incubator
- [ ] Confirm `git -C <dsp-incubator path> branch --show-current` prints `worktree-DEV-7495`
- [ ] Run `git -C <dsp-incubator path> fetch --prune origin`
- [ ] Run `git -C <dsp-incubator path> rebase origin/main`
- [ ] Check that the curation sources did not move since the port:
      `git -C <dsp-incubator path> diff --quiet <pin> origin/main -- cpe/projects/incunabula/raw/curation.csv cpe/projects/incunabula/raw/annotation-curation.csv cpe/projects/incunabula/raw/pages.csv`
      exits 0, `<pin>` being the commit `PROVENANCE` names on dsp-repository's `origin/main`, and
      `git -C <dsp-incubator path> show origin/main:cpe/projects/incunabula/translate.py` still holds `UNSIGNED_STRIP_ID = "strip-38"` and
      `UNSIGNED_STRIP_NAME = "Randleiste 38"`. If either differs, stop: do not edit dsp-repository's merged file and do not continue this phase; report the phase as
      blocked, with the diff, so the owner can decide on a follow-up dsp-repository PR
- [ ] Find the merged feature commit on dsp-repository's `origin/main`: `git -C <dsp-repository path> fetch origin`, then
      `git -C <dsp-repository path> log origin/main --grep "serve project curation through the port" --format=%H -1`; confirm that
      `git -C <dsp-repository path> show <sha>:areas/access/cpe/ports/src/snapshot.rs` contains `CuratedValue`
- [ ] Run `just cpe vendor <sha>` with that commit; `cpe/vendor/PIN` holds that sha
- [ ] Grep `cpe/` outside `cpe/vendor/` for `cpe_ports`; record in the commit body whether any use exists
- [ ] Add `curation` to every `cpe_ports::ProjectSnapshot` literal that grep found
- [ ] Run `just cpe test`; it passes
- [ ] Run `git -C <dsp-incubator path> status --porcelain --ignored cpe/vendor`; it lists no ignored file
- [ ] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<sha>)`
- [ ] Add `- [x] DEV-7495` to Phase 1b of `cpe/docs/INCUNABULA_GO_LIVE.md`, before DEV-7402 (`:155`): the port serves per-resource curation from `sync`'s
      `0803-curation.csv`, `cpe-ports` is vendored at `<sha>`, and project-wide configuration stays for the KDL
- [ ] Update the DEV-7402 item of `cpe/docs/INCUNABULA_GO_LIVE.md` (`:155-161`): curation comes through `ProjectSnapshot.curation`; until DEV-7402 lands a change to
      `raw/curation.csv` or `raw/annotation-curation.csv` is repeated by hand in dsp-repository's `0803-curation.csv`; and the port serves the data ARKs since DEV-7487,
      so the sentence "nothing in `sync`'s projection carries them today" goes
- [ ] Add a note to "`curation.csv`: the authored layer" in `cpe/projects/incunabula/raw/README.md`: dsp-repository's `areas/access/sync/data/0803-curation.csv` is a copy
      served through the port (DEV-7495), and a change here is repeated there until DEV-7402 retires this file
- [ ] Add the same note to "`annotation-curation.csv`: the authored layer" in `cpe/projects/incunabula/raw/README.md`
- [ ] Run `just cpe ci`; it passes
- [ ] Commit `PIN`, the vendored tree, the two documents and any `Cargo.lock` change together, staged by path, as `chore(cpe): vendor cpe-ports with curation (DEV-7495)`,
      after checking the staged paths with `git -C <dsp-incubator path> diff --cached --name-only`
- [ ] Run `git -C <dsp-incubator path> diff --name-only origin/main..HEAD`; it lists only paths under `cpe/`
- [ ] Push with `git -C <dsp-incubator path> push -u origin worktree-DEV-7495`, a plain push after the `fetch --prune` above; a fix from this phase's review is a further
      commit, pushed before the phase is reported done
- [ ] Open the draft PR with `gh pr create --repo dasch-swiss/dsp-incubator --head worktree-DEV-7495 --base main --draft --assignee @me`, titled
      `chore(cpe): vendor cpe-ports with curation (DEV-7495)`, its body naming the pin and H2; never enable auto-merge
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| ID | Action | Who | When | Why an agent cannot |
|----|--------|-----|------|---------------------|
| H1 | Merge the dsp-repository PR for this plan | Balduin | before Phase 4 (Why mid-plan: the vendor pin must be the merged `main` commit, which a rebase-merge mints only on merge) | Merging is the owner's decision |
| H2 | Merge the dsp-incubator re-vendor PR | Balduin | after ship | Merging is the owner's decision |
| H3 | Ask Ivan to accept or reject ADR-0010 under DEV-7488, with its narrow reading of "configuration"; on acceptance, a follow-up PR sets its status and adds the pointing amendments to ADR-0007 and ADR-0008 | Balduin | after ship | An architecture decision between two people |
| H4 | Post the *Handed to DEV-7402* list to Linear DEV-7402 | Balduin | after ship | Posting on the owner's behalf needs his instruction |

## Acceptance Criteria

**The record**
- [x] `docs/adr/0010-the-port-also-serves-per-resource-curation.md` exists with `status: proposed`, names
      the sentences of ADR-0007 and ADR-0008 it changes, and is listed in `docs/src/decisions.md`. ADR-0007
      and ADR-0008 are unchanged: `git diff --quiet origin/main -- docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`
      exits 0.

**The port**
- [x] `ProjectSnapshot` carries `curation`, documented as not an archive fact and as provisional.
- [x] A value for a resource outside the snapshot, a repeated resource, key and language, and a value
      with a malformed key or language or an empty text are each a `Violation`.

**Serving 0803**
- [x] `LiveArchiveProjection` serves 341 curated values on 137 resources for 0803, sorted, the same on
      every call.
- [x] Every Book has its slug, office, German display date and cover Page; every Region and LinkObj its
      `keep`; the Band without a signature its slug `strip-38` and its name.
- [x] `contract::violations` is empty for that snapshot.

**Strictness**
- [x] A known project without its curation file is `Unavailable`.
- [x] Each way the file can break its format is its own `InvalidCuration` with its line: one named
      `test_curation_parse_…` test per variant.
- [x] One file-level test shows that a fault surfaces as `Unavailable` with the file's path.
- [x] `sync-store` names no Incunabula key: the Phase 2 `! grep -rqE …` step exits 0.
- [x] `git check-attr eol -- areas/access/sync/data/0803-curation.csv` prints a line ending in `eol: lf`.

**Documentation**
- [x] `areas/access/cpe/CONTEXT.md` lists curation as served and project-wide configuration as out.
- [x] `areas/access/sync/CONTEXT.md` and `ARCH-MAP.md` name the committed curation and its single writer.

**Incubator**
- [ ] The incubator vendors the merged commit; `just cpe vendor-diff` is empty and `just cpe ci` passes.
- [ ] `INCUNABULA_GO_LIVE.md` and `raw/README.md` say where the second copy is and how a change follows.

## Dependencies & Risks

- **DEV-7488 is not confirmed.** If Ivan rejects ADR-0010, the record is removed together with the code:
  the field, the reader and the file. Nothing consumes the field before DEV-7402, so the cost is this PR.
  Phase 1 ends with a push, so ADR-0010 is in draft PR #468 from then on. H3 is listed after ship, and
  nothing stops Balduin from asking Ivan as soon as Phase 1 is pushed.
- **`main` will carry a proposed ADR that the code already follows.** The record says that it is proposed
  and who decides. The owner chose that over code that contradicts two accepted records silently
  (decision 8). `proposed` is a status the series has not used before.
- **Two copies until DEV-7402.** A curation change made only in the incubator leaves `0803-curation.csv`
  stale without any test failing. Phases 2 and 4 each check once that the sources did not move. After
  that the three documents that name the rule are the only guard until DEV-7402 compares its store with
  `data.sql`. **Superseded 2026-10-10:** since dsp-incubator#500 (DEV-7496) there is one copy, this
  repository's (*Technical Considerations*, "Two copies until DEV-7402").
- **A faulty edit takes the project offline.** Any fault in the hand-edited file makes 0803 `Unavailable`
  (ARCH-MAP note on `sync-store`). The committed tests read the file, so a bad edit fails CI before it is
  deployed. Nothing constructs the adapter until DEV-7400.
- **One fault per run.** An editor with several mistakes sees them one run at a time. That is acceptable
  for a file developers edit under CI.
- **Every curation edit touches the committed tests.** The literal counts change with the file. The owner
  accepted that; if editors other than developers take over the file, the counts need another home.
- **Sparse rows.** A value shifted by one column keeps the cell count right. For 0803 the per-key counts
  catch it; DEV-7402's per-class key check is the lasting guard.
- **A hand-written parser.** It is small and every rule has a table of cases, but it is ours to maintain.
  If the format grows (multi-line values, another separator), the `csv` crate is the next step.
- **The incubator cannot read the file yet.** It vendors `cpe-ports` only (*Handed to DEV-7402*).
- **Not in scope:** project-wide configuration and the KDL mapping (DEV-7402); `catalogue-overrides.csv`;
  totality and vocabulary rules; the decision on DEV-7488 and any edit of ADR-0007, ADR-0008 or the two
  earlier specs; curation for any project but 0803; a tool or an editor configuration for the file.

## Success Metrics

| 0803 | Before | After |
|------|--------|-------|
| Curated values served | 0 | 341 |
| Resources with curation | 0 | 137 (19 Books, 77 Regions, 40 LinkObjs, 1 Band) |
| Keys | 0 | 10, in 11 columns (`teaser` in `de` and `en`; `date_display` in `de`) |
| `contract::violations` | empty | empty |

The incubator's `cpe/vendor/PIN` names a commit that contains this change.

## References

**Tickets**
- Linear DEV-7495 (blocks DEV-7402); DEV-7488 (the interim decision, to be confirmed by Ivan); DEV-7462
  (vendoring). dsp-repository PR #468.

**Decisions**
- ADR-0003 (consumer-defined ports); ADR-0006 (`:23-25`: an amendment refines, a change is a new ADR).
- ADR-0007 (a project is a folder; the DEV-7399 amendment, "whole or refused") and ADR-0008 (the read
  model is fed by its port): one sentence of each is changed by ADR-0010, which this PR proposes.
- `docs/src/decisions.md` (the index of the root series).
- dsp-incubator ADR-0013 (vendored monorepo crates).

**Earlier plans**
- `docs/specs/2026-10-08-cpe-port-data-arks/01-feat-cpe-port-data-arks-plan.md` (DEV-7487)
- `docs/specs/2026-10-08-cpe-port-annotations/01-feat-cpe-port-annotations-plan.md` (DEV-7486)
- `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md` (`:368`)
- `docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md` (`:254`)

**Learnings**
- `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`
- `dsp-incubator/cpe/docs/learnings/logic-errors/shell-input-checks-with-line-wise-grep-accept-multi-line-values.md`

**Sources of the curation**
- dsp-incubator at `6e8e4063`: `cpe/projects/incunabula/raw/{curation.csv,annotation-curation.csv,pages.csv,README.md}`,
  `cpe/projects/incunabula/translate.py`, `cpe/docs/INCUNABULA_GO_LIVE.md` (Phase 1b), `cpe/vendor/README.md`.

**Code**
- `cpe-ports`: `areas/access/cpe/ports/src/{snapshot,contract,fake,lib}.rs`
- `sync-store`: `areas/access/sync/store/src/{lib,error,curation,curation_tests,ark_tests,snapshot_tests}.rs`,
  `src/mapping/mod.rs`, `tests/committed_0803.rs`
- Data: `areas/access/sync/data/{0803.nq,0803-curation.csv,PROVENANCE}`
- Docs: `docs/adr/0010-the-port-also-serves-per-resource-curation.md`, `docs/src/decisions.md`,
  `areas/access/cpe/CONTEXT.md`, `areas/access/sync/CONTEXT.md`, `ARCH-MAP.md`, `docs/src/repo_structure.md`
- Repository: `.gitattributes`
