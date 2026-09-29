---
title: "feat: Declare CPE's port onto the archive projection"
type: feat
date: 2026-09-29
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7398
linear_project: CPE establish production path
---

# feat: Declare CPE's port onto the archive projection

> **Amended 2026-09-29 (DEV-7399), before merge.** The port now follows the archive's DAO shape
> wherever DAO has decided, not knora-base. Three choices below are reversed:
> - `PartOf.property` is gone, and `part_of` holds only the parents (DAO keeps only `dao:isPartOf`, d.69).
> - `ListNode.name` is gone (DAO has no list-node name, d.70).
> - Value-order ties break by the new `Value.uuid`, not by value IRI (DAO re-mints value IRIs, d.48).
>   `uuid` is `None` for links, which tie by target IRI.
>
> `Value.property` is documented as the source property, and the contract gains `MissingValueUuid`,
> `DuplicateValueUuid` and `LinkWithValueUuid`. The Boundary DTOs, Value order, Membership, the In/Out
> table, the `Violation` list and the acceptance criteria below show the pre-amendment design;
> `snapshot.rs`, `contract.rs` and `areas/access/cpe/CONTEXT.md` are authoritative. The reasons are in
> `docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md` (Context, R3).

## Overview

This plan adds CPE's first crate, `cpe-ports` at `areas/access/cpe/ports/`. It declares what CPE reads from the
Access Area's archive projection, as ADR-0003 (consumer-defined ports), ADR-0007 (second clause) and ADR-0008
(derived read models) require. The crate contains:

- the trait `ArchiveProjection`;
- its boundary DTOs, which serve archive-shaped facts: a project's resources with their classes, typed values,
  links, files and ordered membership, as knora-base records them. The one reshaping is that membership
  (`isPartOf`, `seqnum`) is lifted out of the values into fields of its own. It adds no CPE presentation concept;
- `FakeArchiveProjection`, an in-memory adapter for CPE's tests;
- `contract::violations`, the invariants every adapter's output must satisfy. The fake's tests run it now, and
  `sync`'s `LiveArchiveProjection` runs it on the committed 0803 data in DEV-7399.

It also adds a seed `areas/access/cpe/CONTEXT.md`, stating which facts the port serves and which it deliberately
leaves out.

The work touches no production code path and needs no CPE engine code. The whole PR is one commit:
`feat(cpe-ports): declare CPE's port onto the archive projection`.

## Problem Statement / Motivation

ADR-0007 decides that CPE's store is a disposable read model, rebuilt only from what `sync` serves through a port
CPE declares. It also requires that port to speak the archive's shape, never CPE's presentation model
(`Resource`/`Property`/`Representation`/compound as `cpe-engine`'s `ir.rs` defines them). Two tickets are blocked
until the port exists:

- DEV-7399 (minimal `sync`): it needs a trait to implement.
- DEV-7400 (bring CPE in): it needs a trait to rebuild its store from, and a fake to test against.

In the prototype today, the remodel from archive to presentation is split across `tools/dao-lift`
(dump → CSV) and a per-project `translate.py` (CSV → `data.sql`). The CSV layer already loses facts. For example,
it orders multi-valued text by value, which drops the archive's value order. The port is the point where
"what the archive records" is separated from "what CPE makes of it", so it has to be precise about both.

## Proposed Solution

### Crate

`areas/access/cpe/ports/` → package `cpe-ports`, `publish = false`, `version.workspace = true`,
`edition = "2021"`, no `[lints]` table (the workspace has none). It depends on **nothing but `std`**, which rules
out `thiserror` and `serde`. ADR-0003 allows `std` and `shared-*`, and nothing in `shared-*` is needed:
`shared-metadata` has no shortcode or IRI type, and a resource label is a plain literal, not `Multilingual`. The
crate doc states this rule. It stays with review until Bazel `visibility` lands (ADR-0003, "Enforced by"); no
script is added for it.

Modules:

| Module | Contents |
|---|---|
| `lib.rs` | `//!` crate doc (what the port is, who implements it, the in/out rule, the std-only rule, why no `#[non_exhaustive]`), the trait, `ProjectionError`, re-exports |
| `snapshot.rs` | the boundary DTOs |
| `fake.rs` | `FakeArchiveProjection` |
| `contract.rs` | `violations` and `Violation` |

### The trait

The decision taken in planning is a **synchronous** trait that returns **one whole snapshot** per project. This is
ADR-0008's rebuild-from-empty baseline. 0803 has about 4,100 resources and 12,700 values, so the snapshot fits in
memory. CPE calls the method at project activation inside `spawn_blocking`.

```rust
/// What CPE reads from the Access Area's archive projection: one project's facts as the archive
/// records them. Implemented by `sync` beside its data (`LiveArchiveProjection`); CPE's tests
/// use [`FakeArchiveProjection`].
pub trait ArchiveProjection: Send + Sync {
    /// The project's current facts, whole. Every call is a full snapshot; the port announces no change.
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError>;
}

#[derive(Debug)]
pub enum ProjectionError {
    /// The projection holds no project under this shortcode.
    UnknownProject { shortcode: String },
    /// The projection could not be read; the source is the adapter's own error.
    Unavailable(Box<dyn std::error::Error + Send + Sync>),
}

impl ProjectionError {
    pub fn unavailable(source: impl std::error::Error + Send + Sync + 'static) -> Self { /* … */ }
}
// Display and Error written by hand (no thiserror). `source()` is `Some` for `Unavailable`, `None` otherwise.
// `Display` for `Unavailable` includes the source's message, as `RepositoryError` does, because call sites log
// with `%error`. No blanket `From<E>`: it would conflict with the reflexive `From<T> for T`.
```

The trait bound is `Send + Sync` and the trait has no generic methods, so it stays dyn-compatible and the
composition root can hold an `Arc<dyn ArchiveProjection>`. The editor's `repository.rs` follows the same
reasoning. `ProjectionError` is neither `Clone` nor `PartialEq`, so tests match it with `matches!`.

### Boundary DTOs (`snapshot.rs`)

Every DTO derives `Debug, Clone, PartialEq, Eq`. There is no `f64`: a decimal is kept in its lexical form. All
fields are `pub`, so tests build snapshots with struct literals and need no builder.

The four IRI newtypes additionally derive `Hash, PartialOrd, Ord` and have `as_str()` and `Borrow<str>`, so the
contract and the fake can key sets and maps on them. They get no `Display` and no `From` until a consumer needs
one. `shortcode` stays a plain `String`/`&str`, because the contract's `Shortcode` is a concept `shared-metadata`
would own and does not yet type. No DTO or enum is `#[non_exhaustive]`: that would forbid struct literals in CPE's
fixtures, and it would hide a new `ValueKind` behind a `_ =>` arm, when a new kind is meant to fail CPE's compile.

```rust
pub struct ResourceIri(pub String);   // the archive's resource IRI, e.g. http://rdfh.ch/0803/<id>
pub struct ClassIri(pub String);      // the resource's own class IRI as the archive records it (a project class)
pub struct PropertyIri(pub String);   // the property IRI the archive records the value, or the membership, under
pub struct ListNodeIri(pub String);

pub struct ProjectSnapshot {
    pub shortcode: String,
    pub resources: Vec<Resource>,     // order unspecified; IRIs unique
    pub list_nodes: Vec<ListNode>,    // every list of the project, flattened; a root has no parent
}

pub struct Resource {
    pub iri: ResourceIri,
    pub class: ClassIri,
    pub label: String,                // rdfs:label: exactly one, a plain literal
    pub values: Vec<Value>,           // per property, in archive order (see below)
    pub file: Option<File>,           // a knora-base Representation has exactly one file value
    pub part_of: Vec<PartOf>,         // kb:isPartOf and its subproperties, lifted out of `values`; order unspecified
    pub seqnum: Option<i64>,          // kb:seqnum (or a subproperty), verbatim: gaps and ties are legitimate
}

pub struct PartOf {
    pub property: PropertyIri,        // the subproperty used, e.g. incunabula:isPartOfBook
    pub parent: ResourceIri,
}

pub struct Value { pub property: PropertyIri, pub kind: ValueKind }

pub enum ValueKind {
    Text { text: String, lang: Option<String> },   // valueHasString (also for standoff texts); valueHasLanguage
    Integer(i64),
    Decimal(String),                                // xsd:decimal lexical form
    Boolean(bool),
    Date(DateValue),
    Uri(String),                                    // xsd:anyURI lexical form
    ListNode(ListNodeIri),
    Link(ResourceIri),                              // subject → object, as the archive records it
}

pub struct DateValue {
    pub calendar: Calendar,
    pub start: DateBound,             // valueHasStartJDN + valueHasStartPrecision
    pub end: DateBound,               // valueHasEndJDN + valueHasEndPrecision
}
pub struct DateBound { pub jdn: i64, pub precision: DatePrecision }
pub enum DatePrecision { Year, Month, Day }
pub enum Calendar { Gregorian, Julian }

pub struct ListNode {
    pub iri: ListNodeIri,
    pub parent: Option<ListNodeIri>,  // None for a root (kb:isRootNode); a child's parent via kb:hasSubListNode
    pub position: Option<u32>,        // kb:listNodePosition, verbatim; roots have none
    pub name: Option<String>,         // kb:listNodeName
    pub labels: Vec<LangString>,      // rdfs:label, language-tagged
}
pub struct LangString { pub text: String, pub lang: Option<String> }

pub enum File {
    StillImage { asset: String, width: u32, height: u32 },  // internalFilename, dimX, dimY (both required by knora-base)
    Audio { asset: String },
    MovingImage { asset: String },
    Document { asset: String },
}
```

`Calendar`, `DatePrecision` and `DateBound` also derive `Copy` and `Hash`.

The doc comments on these types carry the rules below, because the rules are what an adapter author must not
break:

- **Current, live facts only.** An adapter serves neither deleted resources and values (`kb:isDeleted true`) nor
  superseded value versions (the object of any `kb:previousValue`). The superseded versions carry
  `isDeleted false`, so soft-delete filtering alone misses them. 0803 has three.
- **Value order.** Within one property, `values` holds the archive's order: `valueHasOrder` ascending, a missing
  order counting as 0 (the property is optional, and a few 0803 values lack it), and ties broken by value IRI so
  the order is deterministic. The relative order of *different* properties is unspecified.
- **Membership is not a value.** Every subproperty of `kb:isPartOf` appears only in `part_of`, and every
  subproperty of `kb:seqnum` only in `seqnum`. Neither also appears in `values`. They are separate facts, as in
  knora-base: a resource may have a `seqnum` without a parent, and it is kept. It may also have several parents,
  since only project cardinalities limit that; 0803 Pages have exactly one.
- **CPE's derivations stay in CPE.** Positional order from `seqnum`, and its tie-break, are CPE's decisions, in
  DEV-7400. So is how a resource with several parents is placed. Ties, gaps and missing seqnums under one parent
  are archive facts (three 0803 books have them), and the contract does not reject them.
- **Text is always plain.** Every text value is served with its `valueHasString`, including those that carry
  standoff; the markup is what is left out, never the value. `lang` is `valueHasLanguage` and is `None` where
  the archive records none, which is every 0803 value.
- **Dates are what the archive stores.** A date is its calendar plus a start and an end, each a Julian Day Number
  and a precision. A single-point date has equal bounds. Converting to year, month, day and era is CPE's remodel,
  not the port's.
- **Links point one way.** `Link` points from subject to object. A reverse link is CPE's derivation, not a
  second fact. LinkValue reifications are not served; the direct link is the fact.
- **Out means omitted.** An adapter leaves out every fact not in the list below. It never errors on such a fact
  and never substitutes a string form. A link or `part_of` whose target is an omitted resource is itself omitted,
  and so is a link to a resource outside the project (the corpus has none today). A child of an omitted parent
  keeps its `seqnum` and loses only that `part_of`. The adapter filters before it returns. `DanglingLink` and
  `DanglingParent` are therefore the only guard against a missed filtering step.
- **One snapshot or none.** `snapshot` never returns a partial snapshot: a read that fails midway is
  `Unavailable`. A snapshot carries no revision or hash; ADR-0008's full rebuild makes one unnecessary for now.
- **Not validated by the port.** `Decimal`, `Uri` and `lang` are passed through as the archive records them;
  their syntax is not checked here.

### In and out (the note for `CONTEXT.md`)

| In | Out, deliberately | Why out |
|---|---|---|
| Resources: IRI, class, label | Ontology definitions (class/property labels, cardinalities, subproperty hierarchy) | KDL declares CPE's labels |
| Text (plain `valueHasString` + language), standoff texts included | Standoff markup | Nothing in CPE presents it yet; it is added as `Text`'s own field when a project needs it, never as an opt-in flag |
| Integer, decimal, boolean, uri | Geometry, color, geoname, time, interval values; the planned structured geolocation value | No CPE component presents them. Map display is deferred to CPE by the geolocation PRD (`dasch-specs/specs/2026-09-07-support-geographic-location-data-in-dsp/01-geolocation-value-type-PRD.md:577`); it comes in with the first project that shows a map |
| Date (calendar, start and end as JDN + precision) | Islamic calendar | Accepted by DSP-API, but the corpus has Gregorian and Julian only |
| List nodes (tree, position, name, labels) and list values | — | — |
| Links, subject → object | Regions, LinkObjs, annotations (`isRegionOf`, `isAnnotationOf`); LinkValue reifications | Region storage is an open engine gap (the prototype's G5) |
| Ordered membership (`isPartOf` + `seqnum` and their subproperties) | `isSequenceOf` | Not used by 0803; added when a project needs it |
| One file per resource: still image (asset + dimensions), audio, moving image, document | Other file kinds (archive, text, external and vector still images), file bytes, original filename, MIME types, checksums | CPE builds IIIF URLs from the asset name; the rest is not presented |
| — | Rights and legal info (license, copyright holder, authorship) | Hard-coded in the prototype today; its own follow-up, needed before ADR-0005 landing pages (ADR-0007's ARK clause) |
| — | ARKs, permissions, creation and deletion metadata | No ARK points at CPE yet (ADR-0007) |
| — | Deleted resources and values, superseded value versions | Not current facts |
| — | Change announcements, snapshot revision | ADR-0008: full rebuild until a port carries change |
| — | Links to resources outside the project | None in the corpus; the adapter omits them |

### `FakeArchiveProjection` (`fake.rs`)

- `FakeArchiveProjection::default()`, then `with_project(ProjectSnapshot)`, which keys the snapshot by its own
  `shortcode`, and `with_unavailable(&str)`. Both are `self -> Self` builders. The last call for a shortcode wins.
- `snapshot()` returns a clone for a known shortcode, `UnknownProject` for an unknown one, and a fresh
  `Unavailable` (built with `ProjectionError::unavailable` and a fixed source message) for a shortcode registered
  as unavailable. `Unavailable` is what CPE's "a project fails alone" tests (ADR-0007) need.
- Storage is a `BTreeMap<String, Entry>` with `enum Entry { Snapshot(ProjectSnapshot), Unavailable }`. The fake
  preserves every `Vec` exactly as given and never reorders.

### `contract::violations` (`contract.rs`)

`#[must_use] pub fn violations(requested: &str, snapshot: &ProjectSnapshot) -> Vec<Violation>`. It is a pure
function over one snapshot, so DEV-7399 calls it as `violations("0803", &live.snapshot("0803")?)`. `Violation`
derives `Debug, Clone, PartialEq, Eq`, so tests compare whole vectors with `assert_eq!`. Each variant names the
offending IRIs and has a `Display`. The output order is deterministic, because `resources` order is unspecified:
violations are sorted by variant in the order listed below, then by the first IRI they name.
`ShortcodeMismatch` names none and occurs at most once.

- `ShortcodeMismatch` — `snapshot.shortcode != requested`.
- `DuplicateResource` — two resources share an IRI.
- `DanglingLink` — a `Link` target is not a resource in the snapshot.
- `DanglingParent` — a `part_of` target is not a resource in the snapshot.
- `MembershipCycle` — a resource is its own ancestor through `part_of`, walking every parent of a resource with
  several. This includes the case where a resource is its own parent. The walk is iterative with a visited set,
  not recursive, so a deep chain cannot overflow the stack. Each cycle is reported once, named by its smallest
  IRI.
- `DanglingListNode` — a `ListNode` value, or a node's `parent`, names no node in `list_nodes`.
- `DuplicateListNode` — two nodes share an IRI.
- `ListNodeCycle` — a node is its own ancestor through `parent`, with the same iterative walk and once-per-cycle
  reporting as `MembershipCycle`.
- `DuplicateSiblingPosition` — two nodes with the same `Some(parent)` share a `Some(position)`. Roots are separate
  lists and are not compared with each other.
- `InvertedDate` — a date whose start JDN is greater than its end JDN.

The contract never checks what it cannot see: value order, `lang`, `Decimal` and `Uri` syntax, or whether an
omitted fact was omitted correctly. DEV-7399 pins these with 0803 fixtures in its own tests, so "no violations" is
a consistency check, not proof of fidelity. A violation is always an adapter bug. The function is a test tool
first; whether CPE also runs it at activation, so that a broken snapshot fails its project alone, is DEV-7400's
decision.

## Technical Considerations

- **Architecture.** This crate is the one CPE crate that `sync` (DEV-7399) will depend on. ADR-0003 permits that
  capability-to-capability edge: a provider depends on a consumer's `ports` crate.
  `check-composition-root-deps.sh` already permits `access-server` to depend on a `ports` crate.
- **No composition-root change.** Nothing constructs an adapter yet, and `access-server` is not touched.
- **Release.** No release-please entry is needed. The crate falls under the root package `.`
  (`docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`), and a `feat` commit
  bumps the workspace minor version.
- **Lints.** `just check` runs clippy with `-D warnings` but without `--all-targets`, so warnings in test code
  are not caught there. Run `cargo clippy -p cpe-ports --all-targets -- -D warnings` as well.
- **Formatting.** The crate is doc-comment heavy. Run `just check` before pushing, not a hand reflow
  (`docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`).
- **Conventions.**
  - Test names follow `test_{what}_{condition}_{expected}`, in `#[cfg(test)]` modules.
  - Comments state only what a reader must not break (CONVENTIONS.md).
  - No test checks derives or the compiler.
  - Tests are written first, and in an interactive session they are shown to the developer before
    implementation (CLAUDE.md).
- **Reviewers for the review pass** (docs/specs/README.md):
  - `eng:review:rust-reviewer`
  - `eng:review:dune-reviewer` (a dune-enabled repo: new component, boundary, vocabulary)
  - `eng:review:consistency-reviewer` (ARCH-MAP, CONTEXT, CONVENTIONS, repo_structure)
  - `eng:review:code-simplicity-reviewer`

## Implementation Approach

Test fixtures use non-sorted data, following
`dasch-specs/learnings/test-failures/alphabetical-test-data-masks-ordering.md`. Values are given in an order that is
neither alphabetical nor by IRI, and at least one property has more than two values.

**Crate scaffold**
- [x] Add `areas/access/cpe/ports/Cargo.toml` (`cpe-ports`, `publish = false`, `version.workspace = true`, `edition = "2021"`, no `[dependencies]`, no `[lints]`)
- [x] Add `"areas/access/cpe/ports"` to the root `Cargo.toml` `members`, after `areas/access/server`, and commit the resulting `Cargo.lock` change

**Tests first**
- [x] Write `fake.rs` tests, calling through `&dyn ArchiveProjection`: a known shortcode returns the registered snapshot unchanged, including value order
- [x] Write `fake.rs` tests: an unknown shortcode returns `UnknownProject` naming it
- [x] Write `fake.rs` tests: an unavailable shortcode returns `Unavailable`, whose `source()` is `Some`
- [x] Write `fake.rs` tests: `with_project` after `with_unavailable` for the same shortcode serves the snapshot
- [x] Write `contract.rs` tests: a snapshot exercising every `ValueKind`, every `File` variant, two lists (roots without positions), a two-level list, a resource with two parents, and a book with pages (seqnum gap, tie and a missing seqnum included) has no violations
- [x] Write `contract.rs` tests: one test per `Violation` variant, each on a minimal snapshot that breaks exactly that invariant and reports exactly that violation
- [x] Write `contract.rs` tests: `DanglingListNode` from a list value and, separately, from a node's `parent`
- [x] Write `contract.rs` tests: a two-resource `part_of` cycle reports one `MembershipCycle`, and a self-parent reports one
- [x] Write `contract.rs` tests: a two-node `parent` loop reports one `ListNodeCycle`, and a node that is its own parent reports one
- [x] Write `contract.rs` tests: two siblings under one parent with equal positions report `DuplicateSiblingPosition`, and two roots without positions report nothing
- [x] Write `contract.rs` tests: a single-point date (equal bounds) reports nothing, and a start JDN one day after the end reports `InvertedDate`
- [x] Write `contract.rs` tests: a snapshot with three violations, its resources in non-sorted order, returns them in the documented order
- [x] Write `lib.rs` tests: `ProjectionError`'s `Display` names the shortcode for `UnknownProject` and includes the source's message for `Unavailable`

**Implementation**
- [x] Implement `snapshot.rs` with the DTOs above and their doc comments carrying the rules above
- [x] Implement `lib.rs`: crate doc, `ArchiveProjection`, `ProjectionError` with `unavailable`, hand-written `Display` and `Error`, re-exports
- [x] Implement `fake.rs`: `FakeArchiveProjection`
- [x] Implement `contract.rs`: `violations` and `Violation` with `Display`
- [x] Run `cargo test -p cpe-ports`; every test passes
- [x] Run `cargo clippy -p cpe-ports --all-targets -- -D warnings`; it reports nothing

**Documentation**
- [x] Add `areas/access/cpe/CONTEXT.md`: CPE's context seed, with the terms *Archive projection port*, *Project snapshot* and *Archive-shaped fact*, and the in/out table above with its reasons
- [x] Update `areas/access/CONTEXT.md` line 3: CPE is no longer only "designed but not built"; its port is declared
- [x] Update `areas/access/CONTEXT.md`'s **CPE** entry (lines 13-14): no longer "Not built", its port declared, and a pointer to `cpe/CONTEXT.md`
- [x] Link `areas/access/cpe/CONTEXT.md` from the root `CONTEXT.md` Access Area bullet (line 13)
- [x] Add `areas/access/cpe/CONTEXT.md` after `areas/access/CONTEXT.md` in the context-file list in `docs/src/repo_structure.md` line 7
- [x] Add a `cpe-ports` row after `dpe-server` in the crate table in `docs/src/repo_structure.md`
- [x] Add `cpe-ports` to the Crates row of `CONVENTIONS.md`'s scope table (line 54)
- [x] Run `dune:dune-map` to refresh `ARCH-MAP.md`. The `areas/access/cpe` entry gains real Paths for `ports/`, the `ArchiveProjection` public interface and the durable-state line ADR-0008 asks for. Line 21's "no files yet" goes, as does the Paths line's. The entry stays `planned` for everything except `ports/` (done by hand, not through the skill: the entry stays `planned`, so dune-map's format keeps `Fingerprint: none`)
- [x] Correct the `areas/access/cpe` entry in `ARCH-MAP.md` in three places. "Depends on" (lines 551-555) names CPE's port against `sync` as the first `cpe/ports` edge, and keeps DPE ↔ CPE ports as hypothetical. "Public interface" (lines 545-546) no longer implies that `cpe/ports` is what siblings need *from* CPE. "Used by" (line 556) names `sync` as the provider depending on `cpe/ports`

**Verification**
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Run `just commit-lint`; the single commit `feat(cpe-ports): declare CPE's port onto the archive projection` passes

## Acceptance Criteria

- [x] `cpe-ports` exists at `areas/access/cpe/ports/` with no `[dependencies]`
- [x] `ArchiveProjection` is synchronous, `Send + Sync` and dyn-compatible (the fake's tests call it as `&dyn ArchiveProjection`), with one method `snapshot(&str) -> Result<ProjectSnapshot, ProjectionError>`
- [x] Dates are served as calendar plus JDN and precision per bound, as knora-base stores them
- [x] No DTO names a CPE presentation concept (no "representation", "compound", "property value", no display title, no IIIF URL) — checked in review
- [x] `FakeArchiveProjection` serves registered snapshots unchanged and can simulate unknown and unavailable projects
- [x] `contract::violations` detects each listed invariant, in a deterministic order. It reports none for a valid snapshot that contains several list roots, a resource with two parents, and seqnum gaps, ties and absences
- [x] `areas/access/cpe/CONTEXT.md` states every fact that is in and every fact that is deliberately out, each with its reason
- [x] `ARCH-MAP.md`, both `CONTEXT.md` indexes, `docs/src/repo_structure.md` and `CONVENTIONS.md` name the new crate
- [x] `just check` and `just test` pass

## Dependencies & Risks

- **Unblocked by** DEV-7393 (ADR-0007, done). **Blocks** DEV-7399 and DEV-7400.
- **The DTOs may be wrong in ways only DEV-7399 or DEV-7400 reveal.** An example is a fact the 0803 remodel needs
  that the table leaves out. The port has one consumer and one future provider, so it is cheap to change. The
  risk is accepted, and a change lands in whichever PR finds the gap.
- **JDN puts calendar arithmetic in CPE.** The prototype parses DSP's date strings (`translate.py`,
  `parse_date_source`). DEV-7400 needs a JDN-to-Julian/Gregorian conversion in its remodel instead. This is the
  price of serving what the archive stores, and it was chosen in review.
- **Out of scope.** The following belong to DEV-7399 and DEV-7400:
  - the `sync` capability and its Live adapter;
  - CPE's store and its rebuild test (ADR-0008's "one test per read model");
  - any KDL mapping;
  - splitting DPE's terms out of `areas/access/CONTEXT.md`;
  - `ARCH-MAP.md`'s `sync` entry.
- **Rights are out.** The prototype hard-codes license and attribution. Serving them from the archive is its own
  follow-up, and it gates ADR-0005 landing pages for CPE. If a ticket needs rights before DEV-7399 ships, the
  success metric below does not hold.

## Success Metrics

- DEV-7399 implements `LiveArchiveProjection` without changing `cpe-ports`, and
  `contract::violations("0803", …)` returns no violations on the committed 0803 data.
- DEV-7400 rebuilds CPE's store for 0803 from the port alone, with no side channel to the dump or to CSVs.

## References

- ADRs: `docs/adr/0003-one-modulith-per-area.md` (ports, naming), `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`,
  `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`
- Port precedent: `areas/deposit/editor/core/src/repository.rs:1-40` (dyn-compatible `Send + Sync`, error with a boxed source)
- ARCH-MAP entry: `ARCH-MAP.md:537` (`### areas/access/cpe`)
- knora-base (`dsp-api/modules/webapi/src/main/resources/knora-ontologies/knora-base.ttl`):
  - `isPartOf`: 634-644
  - `seqnum`: 704-713
  - `valueHasLanguage`: 750-754
  - date properties on `DateBase`: 1172-1369
  - `valueHasOrder`: 1332-1337
  - `Representation` has one `hasFileValue`: 1964-1969
  - `StillImageFileValue` has `dimX`/`dimY`: 2203-2211
  - `listNodePosition`: 1072
  - `rdfs:label` on `Resource`: 1981-1983
  - `previousValue`: 693
  - `isDeleted`: 1032
- Source shape: `dsp-repository-design/spikes/profile-as-data`:
  - `src/transform.rs:204-487` (`dao:Resource`, `dao:Value`, `dao:Representation`, `isPartOf`/`seqnum`)
  - `src/class_identity.rs:72` (`dao:sourceClass`)
  - `corpus-pattern-analysis.md` (value kinds in the corpus)
  - `census/0803-incunabula.json`
- Prototype consumer: `dsp-incubator/cpe/engine/schema.sql`, `engine/src/ir.rs:14-361`, `projects/incunabula/translate.py`,
  `projects/incunabula/raw/README.md`, `tools/dao-lift/src/cpe_export.rs`
- Contract shape: `shared/metadata/src/record.rs:148` (`Record` — project metadata, not resource facts; not reused)
- Learnings:
  - `dasch-specs/learnings/logic-errors/value-has-order-optional-cardinality.md`
  - `dasch-specs/learnings/test-failures/alphabetical-test-data-masks-ordering.md`
  - `dasch-specs/learnings/logic-errors/csv-export-missing-standoff-parameter.md`
  - `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`
  - `docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`
