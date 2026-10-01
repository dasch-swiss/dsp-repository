---
title: "feat: Serve the committed 0803 projection behind CPE's port"
type: feat
date: 2026-09-29
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
prd: 01-minimal-sync-capability-PRD.md
linear: DEV-7399
linear_project: CPE establish production path
---

# feat: Serve the committed 0803 projection behind CPE's port

## Overview

This plan adds the Access Area's `sync` capability at its minimum: one crate, `sync-store` at
`areas/access/sync/store/`, which implements `cpe_ports::ArchiveProjection` as `LiveArchiveProjection`
over a committed interim-DAO snapshot of 0803, `areas/access/sync/data/0803.nq`. It also records the
decision as an amendment to ADR-0007 and brings `ARCH-MAP.md`, the `CONTEXT.md` files and the crate
lists in line.

The PRD (`01-minimal-sync-capability-PRD.md`) is the source of truth for behaviour; the REQ ids below
are its. The port amendment (PRD R3) landed with PR #452 and `dao-lift`'s interim output (PRD R4) in
DEV-7443. Both are prerequisites, not phases of this plan, and both are done: DEV-7443 landed as
dsp-incubator#411, pinned at `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`. Nothing constructs the adapter in
`access-server`; that is DEV-7400.

The work lands as two PRs of one commit each, stacked on #453 with `gh stack`:
- #455, `docs(docs): add the PRD and plan for the minimal sync capability`, the specs;
- a PR stacked on #455, `feat(sync-store): serve the committed 0803 projection behind CPE's port`, the work.

## Problem Statement / Motivation

CPE's store is a read model rebuilt from `ArchiveProjection` (ADR-0007, ADR-0008). Without a provider
the port has only its fake, and DEV-7400 cannot build CPE against real data. `ARCH-MAP.md` leaves the
on-disk format and the parser to this ticket. The PRD decides the format: an interim, DAO-shaped
N-Quads snapshot produced by `dao-lift`. This plan decides the parser and builds the adapter.

## Proposed Solution

### Crate

`areas/access/sync/store/` → package `sync-store`, with `version.workspace = true`,
`edition = "2021"` and `publish = false`, like `cpe-ports`. It is added to the explicit `members` list
in the root `Cargo.toml`, after `areas/access/cpe/ports`.

Dependencies:

- `cpe-ports` (path): the one permitted capability-to-capability edge (ADR-0003).
- `oxttl` 0.2.4 and `oxrdf` 0.3.4, added to `[workspace.dependencies]` with a comment on why: a
  strict, pure-Rust N-Quads parser, and no triplestore at this minimum. Both are MIT OR Apache-2.0,
  have MSRV 1.87 and no default features. `rdf-12` stays off.
- `thiserror` (workspace), for the crate's own error.
- `tracing` (workspace). `snapshot` carries `#[tracing::instrument(skip(self), fields(otel.kind = "internal"))]`, as
  `CONVENTIONS.md` asks of new service functions.
- Dev: `tempfile = "3"`, declared the way `dpe-core` does.

### Parser

`oxttl::NQuadsParser::new().for_slice(&bytes)`, in the default strict mode. `lenient()` is never
used: a strict parse is what makes "not valid" in REQ-R1.4 real. The bytes come from `std::fs::read`,
and `for_slice`'s error type has no I/O variant, which keeps read failures and parse failures apart.
The mapping indexes the quads by subject and sorts and deduplicates each subject's facts (`oxrdf::Quad`
is not `Ord`, so a `BTreeSet` of quads is not available), so identical quads are one fact and iteration
order is deterministic.
Every duplicate check below runs after that deduplication. The file has the single project graph that
`FORMAT.md` §1 names, `urn:dsp:project:0803`; a quad in the default graph or any other graph makes
the file invalid.

### The adapter

```rust
/// `sync`'s adapter for CPE's port: the committed interim-DAO snapshot of each known project,
/// read from `<dir>/<shortcode>.nq` on every call.
pub struct LiveArchiveProjection {
    dir: PathBuf,
}

/// The projects `sync` holds. A shortcode outside this set is `UnknownProject`, whatever the
/// directory contains; one inside it whose file cannot be served is `Unavailable`.
const KNOWN: &[&str] = &["0803"];

impl LiveArchiveProjection {
    pub fn new(dir: impl Into<PathBuf>) -> Self { /* … */ }
}

impl ArchiveProjection for LiveArchiveProjection {
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
        // 1. not in KNOWN            -> UnknownProject
        // 2. read <dir>/<code>.nq    -> Unavailable(SnapshotError::Read)
        // 3. strict N-Quads parse    -> Unavailable(SnapshotError::Syntax)
        // 4. map to DTOs             -> Unavailable(SnapshotError::Invalid { .. })
        // 5. no resources            -> Unavailable(SnapshotError::Empty)
        // 6. contract::violations    -> Unavailable(SnapshotError::Contract { .. })
    }
}
```

The file path and the snapshot's `shortcode` are both taken from the matching `KNOWN` entry, never
from the argument, so no shortcode can reach outside the directory. `UnknownProject` names the
caller's string.

`SnapshotError` is the crate's own `thiserror` enum. It is `Send + Sync + 'static`, which a
compile-time assertion checks, so `ProjectionError::unavailable` can wrap it directly with no second
box. Its variants are structured, so tests can match them after `downcast_ref::<SnapshotError>()` on
the `Unavailable` source rather than on message text:
- `Read { path, source }`;
- `Syntax { path, line, source }`, where `line` is 1-based (from `TurtleSyntaxError::location()`, which
  is 0-based, plus one);
- `Invalid { path, subject, reason: InvalidFact }`, with `InvalidFact` a small enum, one variant per invalid
  rule below;
- `Empty { path }`;
- `Contract { path, violations }`, the `cpe_ports::contract::Violation`s of the mapped snapshot, listed in its
  message.

`vocab.rs` holds the IRIs as `oxrdf::NamedNodeRef` constants, so the mapping compares terms, not
strings. Its module doc records the `dao-lift` commit the constants were transcribed from,
`dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`, and `PROVENANCE` repeats that
commit in Phase 2. `FORMAT.md` §15 lists every IRI the format fixes.

### Mapping

The mapping reads the vocabulary exactly as `FORMAT.md` defines it at the `dao-lift` commit pinned in
`PROVENANCE`, `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`. That commit is
the pin: `FORMAT.md` is not copied here. The IRIs live in one module,
`vocab.rs`, and each constant's doc comment cites the `FORMAT.md` section it comes from. No IRI is
written anywhere else in the crate.

The table writes terms with `FORMAT.md` §1's prefixes as shorthand only; the file, `vocab.rs` and every
port string carry full IRIs.

| Port | From the interim file |
|---|---|
| `Resource` | an IRI subject typed `dao:Resource` (§2); `class` from `dao:sourceClass`, a full IRI copied verbatim; `label` from `rdfs:label`, exactly one plain literal |
| `Value.property` | the **source property** (REQ-R3.5), a full IRI: `dao:sourceProperty` on a value node, equal to the edge predicate (§3); for a direct link, the link's predicate itself, which is the source property (§6) |
| `Value.uuid` | `dao:valueHasUUID` on a value node, equal to the suffix of its IRI `urn:dsp:value:<uuid>` (§3); absent for links (REQ-R3.3) |
| `ValueKind` | a value edge is one whose object is typed `dao:Value` (§3.1). A node with `dao:sourceListNode` is a list value, which has no `rdf:value` (§4.7). Otherwise the kind is the datatype of the node's `rdf:value` (§4): plain or `rdf:langString` text, `xsd:integer`, `xsd:decimal`, `xsd:boolean`, `xsd:anyURI`, and `dao:date` with the date deviation triples `dao:dateCalendar`, `dao:dateStartJDN`, `dao:dateEndJDN`, `dao:dateStartPrecision` and `dao:dateEndPrecision` (§4.6). These are §4's "must" kinds. `dao:color` and `dao:geometry` are "may" kinds the port has no kind for, so they are omitted. A link is a direct resource-to-resource triple whose predicate lies outside the `dao:`, `oa:`, `rdf:` and `rdfs:` namespaces (§3.1, §6) |
| value order | `dao:valueHasOrder` (`xsd:integer`, present only when the dump records an order, §5); a missing order counts as 0; ties by UUID in byte order, links by target IRI (REQ-R2.2) |
| `File` | `dao:hasRepresentation` to a `urn:dsp:representation:<uuid>` node typed `dao:Representation` (§8), by its `dao:representationType`: `dao:StillImageRepresentation` (dimensions from `dao:dimX` and `dao:dimY`), `dao:AudioRepresentation`, `dao:MovingImageRepresentation`, `dao:DocumentRepresentation`; every one of these reads its `asset` from `dao:internalFilename`; any other type is omitted |
| `part_of`, `seqnum` | `dao:isPartOf` (the parent only, after R3) and `dao:seqnum` (§7) |
| `list_nodes` | the list-node deviation (§9): each `skos:Concept`, its parent from `skos:broader` (roots have none), its position from `dao:listNodePosition`, its labels from `skos:prefLabel` |

Integers, decimals and booleans arrive in their XSD 1.1 canonical lexical form (§1); the mapping refuses
any other form and passes a decimal's lexical through unchanged. `dao-lift` fails its run, writing no file,
when any of the 41 violation rules `FORMAT.md` names (§1–§11) is broken. `sync-store` does not rely on that:
it refuses (`Unavailable`, through `SnapshotError::Invalid`) every `FORMAT.md` violation that touches a fact
the port serves, and omits only what `FORMAT.md` lets a reader omit. The one exception is an untyped value
node, below.

Omitted and invalid are kept apart, as the port's doc demands.

- **Omitted, never an error:**
  - any fact the port does not carry;
  - a value whose `rdf:value` datatype the port has no kind for, `dao:color` and `dao:geometry` included
    (never read as text, §4, §4.10);
  - a representation whose `dao:representationType` the port has no `File` for (§8);
  - a link or `part_of` to an annotation; a child of such a parent keeps its `seqnum`;
  - an edge whose object is an IRI neither typed `dao:Value` nor a resource of the file. This is the one
    violation accepted, by decision: an untyped value node (§3) and a link to an IRI outside the file
    (§3.1), also a violation, look the same, and telling them apart would rest on the IRI's shape. The
    crate doc says so, and a test pins it.

The file's line order and repeated lines (§1) are serialisation, not facts: the parse does not depend on
order, and identical quads are one fact.

- **Invalid, which makes the call `Unavailable`:** a fact the port serves that breaks `FORMAT.md`.
  - a quad outside the project graph, or a blank node as subject or object (§1);
  - a resource without a class or with two, with an `rdf:type` other than `dao:Resource`, or with no
    label, two labels, a language-tagged label or one that is not a plain literal (§2);
  - a literal under a link predicate (§3.1);
  - a value node without a UUID or with two, whose UUID is not a plain base64url literal equal to the suffix
    of its IRI, or that two resources reach (§3);
  - a value node without a source property, with two, or with one other than its edge's predicate (§3);
  - a value node with two `rdf:value`s, an `rdf:value` that is not a literal, or neither `rdf:value` nor
    `dao:sourceListNode`; a list value that also has `rdf:value` (§3, §4.7);
  - an integer, decimal or boolean not in its canonical form, or an order, integer, JDN, seqnum, dimension
    or list position that does not fit its type (§1, §4);
  - a date without a calendar or a bound, or with one of them twice; a calendar that is not a plain
    `GREGORIAN` or `JULIAN`, or a precision that is not a plain `YEAR`, `MONTH` or `DAY` (§4.6);
  - a list value naming a node that is not in the lists, or naming two (§4.7);
  - a second order or seqnum (§5, §7), or a `dao:isPartOf` to anything but a resource of the file (§3.1, §7);
  - a resource with two representations, or one whose representation is not a node typed
    `dao:Representation`; a representation without a type, with two, or with a literal one; a representation
    of a known type without exactly one plain `dao:internalFilename`, or a still image without exactly one
    `dao:dimX` and one `dao:dimY`; a representation node two resources reach (§8);
  - a list node whose parent is not a list node, with two parents or two positions, a root with a position,
    a label that is not a plain or language-tagged literal, or two labels in one language (§9).

An inverted date, a membership or list cycle and two siblings sharing a position stay the contract's
(`cpe_ports::contract::violations`), not mapping checks. `LiveArchiveProjection` runs the contract on every
mapped snapshot before serving it, as a last guard, and a violation is `Unavailable`
(`SnapshotError::Contract`, REQ-R1.6, REQ-R2.1): the strictness rule refuses a broken file, and the contract stays the
one implementation of these rules. `oxttl` lowercases language tags when it parses and
rejects one that is not BCP 47, so §1's tag rule needs no check of its own.

List nodes are never omitted, so a dangling reference to one is a broken file, not a filtered fact.

Annotations are identified by the one marker `FORMAT.md` §10 gives them, `rdf:type oa:Annotation`
(d.71). It covers Regions and LinkObjs alike, project subclasses of either included. They are never
identified by `dao:sourceClass`, because a project may subclass `kb:Region` or `kb:LinkObj`. A class
match would then miss the subclass silently (`dasch-specs/learnings/logic-errors/csv-export-blank-superclass-exact-type-match.md`).
They are dropped, and with them every value node of theirs and every link or `part_of` whose subject
or object is one (REQ-R2.4). Their `oa:motivatedBy` and `oa:hasTarget` triples go with them.

The order is: drop, then validate, then map. A broken fact inside a dropped resource therefore does
not make the file invalid. A `seqnum` sits on the child, so a child whose parent is a dropped annotation
keeps its `seqnum` without a `part_of`; the port calls that an archive fact. A parent absent from the
file is `UnknownParent`. The contract's `DanglingLink` and `DanglingParent` then guard the filtering.

The mapping is split by concept, with modules for resources, values, files, dates and list nodes, over
an index of the deduplicated quads by subject. It borrows from that index rather than cloning terms.
Integer conversions use `try_from`, and a failure is `InvalidFact`, never an `as` cast.

### Tests

Tests come first, and are shown to the developer before implementation in an interactive session.
They are named `test_{what}_{condition}_{expected}`. Fixture data is deliberately non-sorted
(`dasch-specs/learnings/test-failures/alphabetical-test-data-masks-ordering.md`). The REQ ids in this
plan are for review. They never appear in code, test names or comments (`CONVENTIONS.md`).

- **Synthetic.** These live in adjacent `_tests.rs` files: `snapshot_tests.rs` for serving and invalid
  files, and `mapping_tests.rs` for the mapping.
  - Each case is a small interim-format N-Quads string, written into a `tempfile` directory as
    `0803.nq` and read through `LiveArchiveProjection`. The `TempDir` guard lives for the whole test.
  - Repetition over helpers, with at most one helper, which writes a string as `<dir>/0803.nq`.
  - REQ-R2.5 (no markup) cannot fail synthetically, because the format carries no markup. It is pinned
    by the Phase 2 test on a value that carries standoff in the dump.
- **Committed file.** `store/tests/committed_0803.rs` reads the real file through the adapter from
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../data")`. It never uses `include_str!`, which would embed
  about 20 MB in the test binary. These tests `expect` a served snapshot and never check whether the file
  exists first. A missing file therefore fails them (REQ-R2.7), and review checks that this holds.

The expected values in Phase 2 come from the 2026-07-19 stage dump and are written into the tests as
literals.

## Technical Considerations

- **Architecture.** `sync-store` depends on `cpe-ports` and on no other CPE or DPE crate.
  `check-composition-root-deps.sh` does not scan it, and nothing machine-checks the edge: it stays at
  review until Bazel `visibility` (ADR-0003). `access-server` is not touched.
- **Performance.** Each call strictly parses the file's 123,938 quads into memory. CPE calls it once per project
  activation, off the async runtime (`spawn_blocking`, per the port's doc). The committed file's tests
  may be slow in a debug build. If `just test` grows noticeably, the fix is
  `[profile.dev.package.oxttl] opt-level = 3` in the root `Cargo.toml`. The file is not embedded.
- **Security.** The input is committed, public archive data, and the caller cannot influence the path.
  The file carries no users, permissions or creation metadata: that is the PRD's constraint, enforced
  by `dao-lift`. The adapter omits only what the Mapping section lists as omitted, and REQ-R2.4's
  annotation drop is the one filter that removes whole resources.
- **Size.** The file is about 20 MB (19,989,010 bytes) and stays outside LFS.
  - The access-server image is built from a staged context and does not see `areas/access/sync/`.
  - The mosaic playground's Docker build uses the repo root as its context, so
    `areas/access/sync/data/` goes into `.dockerignore`.
- **Clippy.** `just check` runs clippy without `--all-targets`, so test code is checked separately
  below.
- **Release.** No release-please entry: internal `publish = false` crates have none
  (`docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`).
- **Reviewers for each phase review** (`docs/specs/README.md`): `eng:review:rust-reviewer`,
  `eng:review:dune-reviewer`, `eng:review:consistency-reviewer`,
  `eng:review:code-simplicity-reviewer`, and for Phase 1 also `eng:review:performance-reviewer`.

## Implementation Phases

#### Phase 1: `sync-store` over synthetic fixtures

The adapter and its mapping, proven against synthetic interim-format files. It needs the amended port
and `FORMAT.md`, but not the real 0803 file.

**Scaffold**
- [x] Add `oxttl` and `oxrdf` to `[workspace.dependencies]` in the root `Cargo.toml`, with a comment on why a strict streaming parser and no store
- [x] Create `areas/access/sync/store/Cargo.toml` (`sync-store`, `publish = false`, deps `cpe-ports`, `oxttl`, `oxrdf`, `thiserror`, `tracing`; dev-dep `tempfile = "3"`)
- [x] Add `areas/access/sync/store` to the root `Cargo.toml` `members` after `areas/access/cpe/ports`
- [x] Write `vocab.rs` with every interim-format IRI the mapping reads, as `NamedNodeRef` constants, each doc comment citing its `FORMAT.md` section
- [x] Record in `vocab.rs`'s module doc the `dao-lift` commit whose `FORMAT.md` the constants were transcribed from,
  `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`

**Tests first: serving**
- [x] Write a test: a known shortcode whose file holds two resources with distinct labels returns a snapshot of both, with shortcode `0803` and those labels (REQ-R1.1)
- [x] Write a test: a quad written twice is served as one fact
- [x] Write a test: changing the file between two calls serves the changed content on the second (REQ-R1.2)
- [x] Write a test: an unknown shortcode returns `UnknownProject` naming it, even when `<dir>/<shortcode>.nq` exists (REQ-R1.3)
- [x] Write a test: a missing file for 0803 returns `Unavailable` (REQ-R1.4)
- [x] Write a test: a directory in place of `0803.nq` returns `Unavailable` (REQ-R1.4)
- [x] Write a test: a syntax error on a later line returns `Unavailable` whose message names that line (REQ-R1.4)
- [x] Write a test: a quad in a second graph returns `Unavailable` (REQ-R1.4)
- [x] Write a test: a valid file with no resources returns `Unavailable` (REQ-R1.5)

**Tests first: invalid facts** (REQ-R1.4)
- [x] Write a test: a value node without a UUID returns `Unavailable`
- [x] Write a test: a resource with two labels returns `Unavailable`
- [x] Write a test: a resource without a class returns `Unavailable`
- [x] Write a test: two value nodes of one resource sharing a UUID return `Unavailable` (reworked in the strictness round
  below: one value node reached from two resources)
- [x] Write a test: an order that is not an integer returns `Unavailable`
- [x] Write a test: a date without a calendar returns `Unavailable`
- [x] Write a test: a list value naming a node absent from the lists returns `Unavailable`
- [x] Write a test: a list node whose parent is absent returns `Unavailable`
- [x] Write a test: a blank-node resource subject returns `Unavailable`
- [x] Write a test: a resource with no label returns `Unavailable`
- [x] Write a test: a resource whose label is language-tagged returns `Unavailable`
- [x] Write a test: a resource whose label is a typed literal returns `Unavailable`, an `UnfitLiteral` naming `rdfs:label`
  (final review)
- [x] Write a test: a value node with two different `rdf:value`s returns `Unavailable`
- [x] Write a test: a value node with two source properties returns `Unavailable`
- [x] Write a test: a JDN that does not fit `i64` returns `Unavailable`
- [x] Write a test: a date without an end bound returns `Unavailable`
- [x] Write a test: a date whose calendar is neither `GREGORIAN` nor `JULIAN` returns `Unavailable`
- [x] Write a test: a date whose precision is not `YEAR`, `MONTH` or `DAY` returns `Unavailable`
- [x] Write a test: a value node with neither `rdf:value` nor `dao:sourceListNode` returns `Unavailable`
- [x] Write a test: a broken value node on a dropped annotation (a Region typed `oa:Annotation`) does not make the file invalid
- [x] Write a compile-time assertion that `SnapshotError` is `Send + Sync + 'static`

**Tests first: mapping**
- [x] Write a test: three values of one property with orders 2, none and 1 are served in the order none, 1, 2 (REQ-R2.2)
- [x] Write a test: two values of one property with equal orders are served by UUID in byte order (REQ-R2.2)
- [x] Write a test: two links under one property are served by target IRI, each without a UUID (REQ-R2.2, REQ-R3.3)
- [x] Write a test: a Region typed `oa:Annotation` is not served, nor a link or `part_of` to it (REQ-R2.4)
- [x] Write a test: a resource typed `oa:Annotation` whose `dao:sourceClass` is a project subclass of `http://www.knora.org/ontology/knora-base#Region` is not served (REQ-R2.4)
- [x] Write a test: a LinkObj typed `oa:Annotation` is not served, nor a link from it (REQ-R2.4)
- [x] Write a test: a text value is served as its `rdf:value` verbatim, whitespace included
- [x] Write a test: a language-tagged text value is served with its `lang` (REQ-R2.6)
- [x] Write a test: an integer value is served as `Integer` (REQ-R2.6)
- [x] Write a test: a boolean value is served as `Boolean` (REQ-R2.6)
- [x] Write a test: a decimal value is served as `Decimal` with its canonical lexical form unchanged (REQ-R2.6)
- [x] Write a test: a URI value is served as `Uri` with its lexical form unchanged
- [x] Write a test: a list value names its node, and the node's parent, position and labels are served in `list_nodes` (REQ-R2.6)
- [x] Write a test: a still-image file is served as `StillImage` with asset, width and height
- [x] Write a test: an audio file is served as `Audio` with its asset (REQ-R2.6)
- [x] Write a test: a moving-image file is served as `MovingImage` with its asset (REQ-R2.6)
- [x] Write a test: a document file is served as `Document` with its asset (REQ-R2.6)
- [x] Write a test: a file of another kind is omitted and the resource is served without one
- [x] Write a test: a Julian date, its `rdf:value` typed `https://ontology.dasch.swiss/dao#date`, is served with calendar Julian
  and each bound's JDN and precision, not omitted as an unknown datatype
- [x] Write a test: a Gregorian date with a day-precision start and month-precision end is served with those precisions
- [x] Write a test: a page's `dao:isPartOf` and `dao:seqnum` are served as `part_of` and `seqnum`
- [x] Write a test: a resource with two parents is served with both
- [x] Write a test: a `part_of` to a resource not in the file is omitted and the `seqnum` kept (reworked in the strictness
  round below: such a parent is invalid, and the test keeps the `seqnum` of a child whose parent is an annotation)
- [x] Write a test: a resource with no values is served with none
- [x] Write a test: a link is served under its predicate, the source property's full IRI, never compacted (REQ-R3.5)
- [x] Write a test: a value whose `rdf:value` datatype the port does not list (e.g. `https://ontology.dasch.swiss/dao#color`) is omitted and the snapshot is served (REQ-R2.6)
- [x] Write a test: every served snapshot in the mapping tests passes `cpe_ports::contract::violations`

**Implementation**
- [x] Implement `SnapshotError` (`Read`, `Syntax`, `Invalid`, `Empty`; `Contract` added in Phase 4) and `InvalidFact`, with `Display` naming the file and, for `Syntax`, the 1-based line
- [x] Implement `LiveArchiveProjection::new`
- [x] Implement `LiveArchiveProjection::snapshot`, with the path and shortcode taken from `KNOWN` and `#[tracing::instrument]`
- [x] Implement the strict parse of the file into quads
- [x] Implement the mapping from quads to `ProjectSnapshot`, keeping omitted facts apart from invalid ones
- [x] Write the crate doc in `lib.rs`: what `sync` is at its minimum, the interim format and its pin, the known set, omitted versus invalid, and no wiring until DEV-7400
- [x] Run `cargo clippy -p sync-store --all-targets -- -D warnings`; it passes (`just check` does not lint test targets)
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

**Strictness round (decided after Phase 3)**

The developer decided Phase 1's two deferred scope findings: `sync-store` refuses every `FORMAT.md` violation on a fact
the port serves, as the Mapping section now says. Each case is its own synthetic test in `snapshot_tests.rs`, failing
without its check.

- [x] Write tests: a repeated class, UUID, order, seqnum, date term, list node of a list value, list parent and list
  position each return `Unavailable` (`RepeatedPredicate`)
- [x] Implement `one()` in place of the lenient `first()` at every predicate `FORMAT.md` allows once
- [x] Write tests: a representation of a known type without `dao:internalFilename`, a still image without `dao:dimX` or
  without `dao:dimY`, a representation without `dao:representationType` or with a literal one, a `dao:hasRepresentation`
  to a node not typed `dao:Representation` or to a literal, two `dao:hasRepresentation`s, two filenames, and a filename
  that is not a plain string each return `Unavailable`; the existing test keeps an unknown representation type omitted
- [x] Implement `IncompleteRepresentation` and `UnknownRepresentation` in `files.rs`
- [x] Write tests: a value node without `dao:sourceProperty`, with one other than its edge's predicate, or reached by two
  edges under different predicates; a UUID that is not the suffix of its node's IRI, not base64url, or not a plain
  literal; a list value that also has `rdf:value`; and an `rdf:value` that is an IRI each return `Unavailable`
- [x] Rework the test of two value nodes sharing a UUID: a node's IRI fixes its UUID, so the test is now one value node
  reached from two resources (`FORMAT.md` §3, "on one resource or on two")
- [x] Implement `MissingSourceProperty`, `UnfitSourceProperty`, `UnfitValueUuid`, `ListValueWithContent` and the
  file-wide `DuplicateValueUuid` in `values.rs`
- [x] Write tests: a boolean `"1"`, an integer `"+12"`, a decimal `"1.50"` and a decimal that is not a number; a list
  label that is a typed literal or an IRI; two list labels in one language and two untagged ones; and a date calendar
  that is not a plain literal each return `Unavailable`
- [x] Implement the canonical-form checks in `integer` (every integer the mapping reads) and for decimals and booleans,
  `DuplicateListLabel`, and plain date terms
- [x] Write tests: a blank-node object, a resource with a second `rdf:type`, a literal under a link predicate, a
  `dao:isPartOf` to a resource absent from the file or to a literal, and a list root with a position each return
  `Unavailable`
- [x] Rework the test of a `part_of` to an absent resource: such a parent is now invalid, so the test keeps the `seqnum`
  of a child whose parent is an annotation
- [x] Implement `BlankNodeObject`, `ExtraResourceType`, `UnknownParent` and `PositionedListRoot`, and a literal edge as
  `UnfitLiteral`
- [x] State the rule and the untyped-value-node exception in the crate doc in `lib.rs` and the mapping's module doc
- [x] Recompute `ARCH-MAP.md`'s `areas/access/sync` fingerprint; `areas/access/sync/CONTEXT.md` still reads true
- [x] Run `cargo clippy -p sync-store --all-targets -- -D warnings`, `just check` and `just test`; they pass
- [x] Review fix round 1: a test that a representation node reached from two resources returns `Unavailable`
  (`DuplicateRepresentation`); a test that an edge to an untyped value node is dropped and the snapshot served;
  each representation kind named once in `files.rs`; the docs give the leniency's reason and how a newly served
  fact gets its refusal rule; fingerprint recomputed
- [x] Review fix round 2: the value loop's comment names every dropped edge, the leniency test says "accepted"; fingerprint
  recomputed

#### Phase 2: The committed 0803 snapshot and its fidelity tests

The real file, its provenance, and the tests that pin what the contract cannot see.

- [x] Run `dao-lift snapshot <dump-dir> <out.nq>` at the pinned commit `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`
  on the pinned stage dump (`dsp-repository-design/spikes/profile-as-data/dumps-stage/0803-incunabula.zip`, unzipped)
  and write the output to `areas/access/sync/data/0803.nq`; it has 123,938 lines
- [x] Write `areas/access/sync/data/PROVENANCE`: the dump's bag date and server (stage), the DSP-API version from its `bag-info.txt`,
  and the `dao-lift` repository and commit, `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`,
  which is the pin for `FORMAT.md` and the same commit `vocab.rs` records
- [x] Add `areas/access/sync/data/` to `.dockerignore`
- [x] Write `store/tests/committed_0803.rs` with a test: the file read through the adapter passes `contract::violations("0803", …)` with no violations (REQ-R2.1)
- [x] Write a test: 19 resources of source class `http://www.knora.org/ontology/0803/incunabula#Book`,
  4,024 of `http://www.knora.org/ontology/0803/incunabula#Page` and 38 of `http://www.knora.org/ontology/0803/incunabula#Band`
  are served (REQ-R2.4)
- [x] Write a test: exactly 4,081 resources are served, so none of the file's 117 `oa:Annotation`s, the dump's 77 Regions and 40 LinkObjs (REQ-R2.4)
- [x] Write a test: resource `http://rdfh.ch/0803/-34FYx0jVMGTEi8aItQFxQ` serves exactly one value with UUID `U_J3GLDmTJuj24vddKrJMA`, text `a1r; Titelblatt` (REQ-R2.3)
- [x] Write a test: book `http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ` serves exactly one value under
  `http://www.knora.org/ontology/0803/incunabula#hasTitle`, from node `urn:dsp:value:FYQyJ2K3RT6yyqxuAE6EIg`, with UUID
  `FYQyJ2K3RT6yyqxuAE6EIg` and text `Bereitung zu dem Heiligen Sakrament` (REQ-R3.5). The other value with superseded versions,
  `xdvQATb3TOeS875AuWi6Rw`, sits on Region `http://rdfh.ch/0803/GOkuI_IxVuSKMRZmCypz7Q`, an `oa:Annotation`, and is not served
  (REQ-R2.3, REQ-R2.4)
- [x] Write a test: book `http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ` serves its three `http://www.knora.org/ontology/0803/incunabula#hasCitation`
  values in the order `Schramm Bd. XXI, S. 27`, `GW 4168`, `ISTC ib00512000` (orders 0, 1, 2; the reverse of their UUID order) (REQ-R2.2)
- [x] Write a test: that book's `http://www.knora.org/ontology/0803/incunabula#hasPubdate` is a Julian date, start JDN 2266011 and end JDN 2266376, both of year precision
- [x] Write a test: value `-UFjZccyRaWal2r0EhOACw`, which carries standoff in the dump, is served as the plain text `"[missing]\n        "`
  verbatim, the U+001E that follows `[missing]` in the dump stripped by `dao-lift` (FORMAT.md §4.1) (REQ-R2.5)
- [x] Write a test: 23 list nodes are served, 4 of them roots
- [x] Write a test: every Page and every Band has a still-image file with a non-empty asset and non-zero dimensions (the dump's 4,062 still-image file values)
- [x] Write a test: every Page has exactly one `part_of`, to a Book, and a `seqnum`
- [x] Write a test: every `KNOWN` shortcode has a committed `<shortcode>.nq` in `areas/access/sync/data/`
- [x] Write a test: the `dao-lift` commit `PROVENANCE` pins equals the one `vocab.rs` records
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: The decision and the map

The records that say what now exists and why.

**ADR-0007 amendment** (`## Amendment (2026-09-29, DEV-7399) — the committed projection is interim-DAO`)
- [x] Write the amendment's statement: the committed projection is interim-DAO, produced by `dao-lift` from a VRE dump, following DAO where decided, with deviations listed in `FORMAT.md` in the incubator
- [x] Write the amendment's port clause: the port follows the interim format (#452) and names properties by source property
- [x] Write the amendment's exit condition: when DAO is published, `dao-lift` and `FORMAT.md` move to it, and each deviation is resolved upstream or by a port change
- [x] Write the amendment's `Enforced by:` line in the existing amendments' plain form: `sync-store`'s tests (static-analysis); review for the port speaking the format

**Vocabulary**
- [x] Write `areas/access/sync/CONTEXT.md` defining interim format, committed snapshot, known project and `LiveArchiveProjection`, each with an `_Avoid_` line
- [x] Add to `areas/access/sync/CONTEXT.md` that `PROVENANCE`'s `dao-lift` commit,
  `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`, pins the `FORMAT.md` the file follows
- [x] Link `areas/access/sync/CONTEXT.md` from the Access Area bullet of the root `CONTEXT.md`
- [x] Point `areas/access/CONTEXT.md`'s sentence on `sync` at the new file and drop "designed but not built" for `sync`

**Map and crate lists**
- [x] Update the status and Paths of `ARCH-MAP.md`'s `### areas/access/sync` entry
- [x] Update that entry's Purpose and Depends on: the format and parser as decided, `oxttl` as the third-party parser
- [x] Update that entry's Durable state: the committed snapshot from `dao-lift`, with `PROVENANCE`
- [x] Add `areas/access/sync/CONTEXT.md` and `PROVENANCE` to that entry's local-context kit
- [x] Update `ARCH-MAP.md`'s overview sentence (lines 20–22) that says `areas/access/sync` holds nothing yet
- [x] Update the "seven planned components" count in that sentence
- [x] Update `ARCH-MAP.md`'s `areas/access/cpe` entry, whose Depends on and Used by still call `sync` planned
- [x] Add `sync-store` to the Crates row of `CONVENTIONS.md`'s scope table, after `cpe-ports`
- [x] Add `sync-store` to the crate scopes in `docs/src/git-conventions.md`
- [x] Add `sync/store/` and `sync/data/` to the tree in `docs/src/repo_structure.md`
- [x] Add a `sync-store` row to the crate table in `docs/src/repo_structure.md`
- [x] Add `areas/access/sync/CONTEXT.md` to the list of context files in `docs/src/repo_structure.md` (line 7)

**Verification**
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: History

Runs after every other phase review, so that no reviewed commit is rewritten before it has been reviewed.

**The contract at run time (decided before Phase 4)**

- [x] Write tests: an inverted date, a `dao:isPartOf` cycle, a `skos:broader` cycle and two siblings sharing a
  position each return `Unavailable` with `SnapshotError::Contract` naming the one violation (REQ-R1.6)
- [x] Run `cpe_ports::contract::violations` in `LiveArchiveProjection` on every mapped snapshot and refuse it on any
  violation (`SnapshotError::Contract { path, violations }`); the committed-0803 tests still pass
- [x] Amend REQ-R2.1 (and add REQ-R1.6) and the plan's Mapping and adapter sections to say `sync` runs the contract at run time, and
  why; update the crate doc; `areas/access/sync/CONTEXT.md` and ADR-0007's amendment still read true; recompute
  `ARCH-MAP.md`'s `areas/access/sync` fingerprint
- [x] Correct `docs/src/repo_structure.md`: nine ADRs, ADR-0009 summarised, `areas/access/datei/CONTEXT.md` in the
  context-file list
- [x] Run `cargo clippy -p sync-store --all-targets -- -D warnings`, `just check` and `just test`; they pass

**History**

- [x] Squash every commit on the implementation PR (the phase commits, their review fixes, the run-time contract change and the spec-state commits of this plan and its journal) into one commit, `feat(sync-store): serve the committed 0803 projection behind CPE's port`, on top of #455's unchanged `docs(docs): add the PRD and plan for the minimal sync capability`
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Run `PR_BODY="$(gh pr view 457 --json body --jq .body)" just commit-lint origin/worktree-cpe-sync`; it passes (the default base `origin/main` would count #453's and #455's commits too)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Confirm the pinned stage dump is at `dsp-repository-design/spikes/profile-as-data/dumps-stage/0803-incunabula.zip` on the executing machine, or fetch it again (`dsp vre project dump -s stage --project 0803 --skip-assets`) | Balduin | before start | A fresh dump needs a sysadmin token and takes stage's single dump slot |
| H2 | Review and merge the stack in order, #453, #455, the implementation PR (`gh stack merge <implementation PR> --yes`); #452 is merged | Balduin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] `LiveArchiveProjection` implements `ArchiveProjection` for the known set {0803} over `<dir>/0803.nq`, re-reading it on every call
- [x] An unknown shortcode is `UnknownProject`
- [x] A missing, unreadable, syntactically or structurally invalid, or empty file is `Unavailable`
- [x] `contract::violations("0803", …)` is empty on the committed file
- [x] A snapshot with any contract violation is `Unavailable` (`SnapshotError::Contract`)
- [x] The synthetic tests of Phase 1 and the fidelity tests of Phase 2 pass
- [x] `sync-store` depends on `cpe-ports`, `oxttl`, `oxrdf`, `thiserror` and `tracing`, and on nothing else of CPE or DPE
- [x] ADR-0007 carries the amendment
- [x] `ARCH-MAP.md`, the `CONTEXT.md` files, `CONVENTIONS.md`, `git-conventions.md` and `repo_structure.md` name the crate
- [x] `just check` and `just test` pass
- [x] `just commit-lint` passes against each PR's own base

## Dependencies & Risks

- **Done (2026-09-29): #452 carries the R3 amendment,** and this branch is rebased onto it.
  - `Resource.part_of` is `Vec<ResourceIri>`; `PartOf` and `DanglingParent`'s `property` are gone.
  - `ListNode` has no `name`.
  - `Value.uuid` is `Option<String>`, `None` exactly for links.
  - Order ties break by UUID, and links order by target IRI.
  - The kept-against-DAO fields say so in their doc comments.
  - `Value.property` is the source property.
  - The contract gains `MissingValueUuid`, `DuplicateValueUuid` and `LinkWithValueUuid`.

  `sync-store` also rejects a value node without a UUID, or two sharing one, as an invalid file.
  That overlaps the contract's checks on purpose: the mapping refuses a broken file with the rule it
  breaks, and the contract, run on every snapshot before it is served, catches an adapter bug.
- **Done (2026-10-01): DEV-7443 has landed `dao-lift`'s interim output and `FORMAT.md`** in
  dsp-incubator#411, pinned at `dasch-swiss/dsp-incubator@1826d49f1632fd7502297b845cedffc278b48c2d`.
  `FORMAT.md` defines the two things this plan relies on:
  - a direct link names its source property by being its predicate, the full IRI (§6, REQ-R3.5);
  - every Region and LinkObj, project subclasses included, is `rdf:type oa:Annotation` (§10).

  Phase 1 and Phase 2 are unblocked. The Phase 2 counts and literals above were checked against the
  pinned commit's 0803 output.
- **Risk: `FORMAT.md` differs from this plan's mapping table.** `FORMAT.md` at the pinned commit wins.
  The table names terms by prefix as shorthand, and `vocab.rs` is transcribed from `FORMAT.md` §15.
- **Risk: `dao-lift`'s output breaks a Phase 2 expectation** (a count, the order, a date). The literal
  comes from the dump, so the failure is a `dao-lift` bug. It is fixed in `dao-lift`, the pin moves to
  the fixing commit, and the file is regenerated. The test is never adjusted to fit the output.
- **Risk: a 20 MB file slows `just test` in debug.** The mitigation is the `oxttl` opt-level override
  above.
- **Follow-ups outside this plan:** DEV-7400 (wiring), DEV-7444 (a prod dump before go-live), and
  DEV-7445 (order and `lang` upstream).

## Success Metrics

- `contract::violations("0803", …)` returns an empty vector on the committed file.
- DEV-7400 constructs `LiveArchiveProjection` and rebuilds CPE's store from it without changing
  `cpe-ports` for anything the interim format carries.

## References

- PRD: `docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md`
- The port: `areas/access/cpe/ports/src/lib.rs`, `snapshot.rs` and `contract.rs`; `areas/access/cpe/CONTEXT.md`
- The port's plan, for structure: `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`
- Decisions: `docs/adr/0003-one-modulith-per-area.md`, `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`, `docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md`
- DAO decisions: `dsp-repository-design/spycherli/decisions-active.md` (d.48, d.51, d.69–d.73, d.94)
- Fixture paths: `areas/access/dpe/core/src/project_cache.rs:107` (`CARGO_MANIFEST_DIR`)
- `oxttl` 0.2.4: https://crates.io/crates/oxttl; its N-Quads parser at `lib/oxttl/src/nquads.rs` in the oxigraph repository
- Learnings:
  - `dasch-specs/learnings/test-failures/alphabetical-test-data-masks-ordering.md`
  - `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`
  - `dasch-specs/learnings/logic-errors/value-has-order-optional-cardinality.md`
