---
title: "feat: Serve Regions and LinkObjs through cpe-ports"
type: feat
date: 2026-10-08
author: "Balduin Landolt"
status: complete
linear: DEV-7486
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7486
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator
---

# feat: Serve Regions and LinkObjs through cpe-ports

## Overview

`cpe-ports` and its `sync-store` adapter start serving the archive's annotations: the 77 `kb:Region` and
40 `kb:LinkObj` of the committed 0803 projection, with their geometry, color, comment, `isRegionOf` and
`hasLinkTo` facts. Annotations are ordinary `Resource`s. The port gains two value kinds,
`ValueKind::Geometry` and `ValueKind::Color`, which carry the archive's lexical forms verbatim. A new field,
`Resource.annotation`, carries what DAO marks an annotation with: `oa:Annotation`, its `oa:motivatedBy` and
its `oa:hasTarget`. The incubator then re-vendors `cpe-ports` at the merged commit.

There is no PRD. The requirements are Linear DEV-7486 and the "Port gaps" section of the DEV-7402
pre-planning inventory (Linear document "DEV-7402 pre-planning inventory: translate.py vs. cpe-ports",
2026-10-08).

## Problem Statement / Motivation

Incunabula's Slice 6 needs the annotations for:
- the reader overlay;
- the annotation register;
- the reuse links;
- the Region entries in "Show in dataset".

DEV-7402 builds Incunabula's store from the port, so it is blocked on them. The facts are already in
`areas/access/sync/data/0803.nq`, but the port does not serve them:

- `sync-store` drops every `oa:Annotation` subject and every link to one
  (`areas/access/sync/store/src/mapping/resources.rs:15-27`).
- `ValueKind` (`areas/access/cpe/ports/src/snapshot.rs:106-127`) has no geometry or color kind, and the
  mapping omits `dao:geometry` and `dao:color` values (`mapping/values.rs:137-170`, the `Ok(None)` arm).
- `areas/access/cpe/CONTEXT.md` lists both as out.

Two earlier documents deferred this until CPE could present annotations:
- The port plan (`docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`)
  left geometry and color out because "No CPE component presents them". It left Regions and LinkObjs out
  because "Region storage is an open engine gap (the prototype's G5)".
- The sync PRD (`docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md`, REQ-R2.4)
  says "Regions and LinkObjs in the port: added as CPE's port change when annotation storage (the
  prototype's engine gap G5) lands."

Both triggers have now fired. The incubator's CPE has stored and rendered Regions, their geometry and their
color since Slice 6. This plan is the port change those documents anticipated, not a reversal of them.

## Proposed Solution

The owner decided on 2026-10-08 that the port stays archive-shaped (ADR-0003, ADR-0007, ADR-0008) and gets no
dedicated annotation DTO:
- In DAO an annotation is a `dao:Resource`, and its values, links and representation are unchanged
  (`FORMAT.md` §10).
- A separate struct would remodel the archive, which is CPE's job.
- It would also duplicate the contract's identity and UUID rules, and drop a project subclass's extra
  properties.

### Boundary DTOs (`cpe-ports`, `snapshot.rs`)

```rust
pub struct Resource {
    // … unchanged fields …
    /// `Some` exactly for a resource the archive types `oa:Annotation`: a Region or LinkObj, or a
    /// project subclass of either. Identify annotations by this field, never by `class`
    /// (`FORMAT.md` §10).
    ///
    /// Provisional: DAO's minimal Web Annotation mapping (d.71, `FORMAT.md` §12 D11).
    pub annotation: Option<Annotation>,
}

/// What the archive records about an annotation beyond its values and links.
pub struct Annotation {
    pub motivation: Motivation,
    /// `oa:hasTarget`: at least one, each a resource of the snapshot; sorted by IRI, without repeats.
    /// They are the targets of the annotation's own links, whichever of its properties those are.
    pub targets: Vec<ResourceIri>,
}

/// `oa:motivatedBy`, as the archive records it (`FORMAT.md` §10).
pub enum Motivation { Commenting, Highlighting, Linking }

pub enum ValueKind {
    // … unchanged variants …
    /// knora-base's geometry JSON (`dao:geometry`). Provisional: DAO's datatype name (§12 D10).
    Geometry(String),
    /// knora-base's color lexical, e.g. `#ff3333` (`dao:color`). Provisional: as `Geometry`.
    Color(String),
}
```

The `ValueKind` doc's sentence "`Decimal`, `Uri` and `lang` are passed through as the archive records them;
the port does not check their syntax" is extended to name `Geometry` and `Color`. Parsing geometry is CPE's
remodel.

**Derives and exports**
- `Annotation` derives `Debug, Clone, PartialEq, Eq`.
- `Motivation` derives `Debug, Clone, Copy, PartialEq, Eq`, as `Calendar` and `DatePrecision` do.
- Both are re-exported from `lib.rs`.
- No type is `#[non_exhaustive]` (crate doc, `lib.rs:8-11`), so a consumer's exhaustive `ValueKind` match
  breaks on purpose.

**What the contract checks**
- `targets` is non-empty and every target resolves.
- Sort order and the absence of repeats are the adapter's promise. The contract does not check them,
  just as it does not check value order.

**Naming.** Code, test names and docs spell it "color", following `dao:color`, so one grep finds it.

### Contract (`contract.rs`)

There are two new invariants. Both are declared directly after `DanglingLink`, with `resource` as the first
field, because the derived `Ord` follows declaration order and then field order. Each gets:
- a `Display` arm in the existing `write!(f, "resource {} …", …)` form;
- a `test_violations_…` test;
- a case in the documented-order test (`contract.rs:758-805`).

The invariants:
- `Violation::DanglingTarget { resource, target }`: an `annotation.targets` entry that is not a resource in the
  snapshot.
- `Violation::UntargetedAnnotation { resource }`: an `annotation` with no target.

`sync-store` cannot produce either one, because its `UnknownTarget` and `MissingAnnotationTarget` refuse such a
file first. The contract still checks both, since that is where the DTO's cross-fact promises are kept for
every adapter and every `FakeArchiveProjection` fixture.

The contract does not check that `targets` equals the annotation's own link targets. The port cannot tell which
of an annotation's properties are sub-properties of `isRegionOf` or `hasLinkTo`, so the committed-file test
checks that equality for 0803 instead.

The new value kinds need no new rule. They fall into the existing UUID arms (`contract.rs:205-224`, the
`(None, _)` arm), so a geometry or color without a UUID is `MissingValueUuid`.

### Mapping (`sync-store`)

`mapping/mod.rs:3-6` describes the order as "drop, then validate, then map". Nothing is dropped any more, so the
doc becomes "validate, then map". Code comments describe the result in the present tense, with no "now" and no
"no longer" (CONVENTIONS.md).

**Pre-pass over every subject.** `Index.subjects` (`mod.rs:51`) holds every subject: resources, value nodes,
representation nodes and list nodes. One walk over it, in `resources::map` before the resources are mapped,
rejects:
- a subject typed `oa:Annotation` but not `dao:Resource`, as `InvalidFact::UnservedAnnotation`. Today
  `index.typed(DAO_RESOURCE)` never reaches such a subject, so it is silently ignored.
- `oa:motivatedBy` or `oa:hasTarget` on a subject not typed `oa:Annotation`, as
  `InvalidFact::StrayAnnotationFact { predicate }`. Today these facts are ignored too, because `is_link`
  excludes the `oa:` namespace.

The two checks cannot overlap: the stray check skips subjects typed `oa:Annotation`.

**Resources.**
- `served` becomes every `dao:Resource` (`resources.rs:15-27`).
- The type check skips both `dao:Resource` and `oa:Annotation`, and any other type is `ExtraResourceType`
  naming that type. Today it reports the first type that is not `dao:Resource` (`resources.rs:36`), which
  for an annotation would be `oa:Annotation`.

**Annotation facts**, read on each served subject typed `oa:Annotation`:

`oa:motivatedBy`, read with `one()`:

| Case | Result |
|------|--------|
| Missing | `MissingMotivation` |
| Two different IRIs | `RepeatedPredicate`, raised by `one()` (`mod.rs:135-145`). Identical duplicate quads are already merged in the index. |
| A literal | `UnfitLiteral`, through `unfit()` |
| An IRI other than `oa:commenting`, `oa:highlighting` or `oa:linking` | `UnknownMotivation { motivation }` |

`oa:hasTarget`, read with `objects()`, since it is multi-valued:

| Case | Result |
|------|--------|
| None | `MissingAnnotationTarget` |
| A literal | `UnfitLiteral` |
| A target that is not a served resource | `UnknownTarget { target }` |
| A blank-node target | already `BlankNodeObject`, file-wide (`mod.rs:82-84`) |

Why `UnknownTarget` is an error when a link to an unserved IRI is only omitted:
- `values.rs` omits such a link because, by the sync plan's decision, a link to an IRI outside the file
  "and an untyped value node … look the same". A target can never be a value node, so that reason does not
  apply.
- The target rule therefore follows `UnknownParent`.
- FORMAT §11 drops every target "whose object is not a live resource of the dump", and §10 makes an
  annotation with no target a producer violation. In a conforming file every target is served.

The mapping does not cross-check the motivation against the class or the comments. It has no sub-property
closure, and the rule belongs to the producer (FORMAT §10).

**Values and links**
- **New value kinds.** `vocab.rs` gains `DAO_GEOMETRY` and `DAO_COLOR`. They come from `FORMAT.md` §4.8,
  §4.9 and §15 at the existing pin `b2226ff4`, which lists both (checked 2026-10-08), so `PROVENANCE` does
  not move. `kind()` maps them to `Geometry` and `Color`, unchecked, as it does `Uri`.
- **Links.** `kb:isRegionOf` and `kb:hasLinkTo` become `Link` values once their subjects are served. So do
  `kb:isAnnotationOf` and any other link a project file carries (0803 has none). `oa:hasTarget` stays out of
  `values`, because it "is annotation structure and never a link" (FORMAT §10).
- **`part_of`.** Its annotation-parent arm (`resources.rs:72`) becomes unreachable and is removed, along
  with its doc (`resources.rs:60-61`).

### Facts in 0803 (checked during planning, 2026-10-08)

Checked by a script over `areas/access/sync/data/0803.nq` at `a7e8df14`.

**Annotations**
- 117 subjects are typed `oa:Annotation`, and every one is also `dao:Resource`.
- 77 have `dao:sourceClass` `kb:Region` and 40 `kb:LinkObj`. There are no subclasses.
- Motivations: 77 `oa:commenting` and 40 `oa:linking`, with no `oa:highlighting`.

**Regions**
- Each has exactly one `kb:hasGeometry`, one `kb:hasColor`, one `kb:hasComment` (the knora-base property)
  and one `kb:isRegionOf`, and no other value property.
- All 77 `isRegionOf` targets are served Pages.
- Geometry `type`: 52 rectangles, 24 polygons (4–15 points) and 1 circle (with `radius`).
- Colors: `#ff3333` ×73, `#3333ff` ×3, `#33ff33` ×1.

**LinkObjs**
- Each has one `kb:hasComment` and between 1 and 4 `kb:hasLinkTo`, 79 in all.
- The `hasLinkTo` targets are 64 Regions, 6 Books and 9 Pages, all of them served.

**Targets**
- There are 156 `oa:hasTarget` (77 + 79).
- Each annotation's targets equal its `isRegionOf` or `hasLinkTo` link targets.
- No resource that is not an annotation links to an annotation.

Serving the annotations adds no dangling link, and no new check fires on 0803.

## Technical Considerations

**Archive shape (ADR-0007, DEV-7399 amendment)**
- Geometry and color stay the archive's lexical forms. Parsing knora's geometry JSON and normalising colors
  is CPE's remodel, in DEV-7402's store builder.
- `Annotation` restates DAO's two `oa:` facts and derives nothing from them.
- ADR-0007 needs no amendment. Its clause "omits only what `FORMAT.md` lets a reader omit and what the port
  does not carry" still holds.

**Consumers**
- The change breaks consumers on purpose. In this repository only `cpe-ports` and `sync-store` use
  `ValueKind`, and both have fallback arms (`contract.rs:178-202`, `values.rs:82-87`), so Phase 1 compiles on
  its own.
- `Resource` struct literals are at:
  - `contract.rs:349`, the `resource()` test helper; every other use is `..resource(…)`;
  - `mapping_tests.rs:844`;
  - `fake.rs:66`;
  - `resources.rs:49`.
- In the incubator, `cpe-ports` is vendored with no consumer yet (dsp-incubator#471, decision D1, and no
  `cpe_ports` use under `cpe/` on `main`). Phase 3 checks this again, and fixes any use it finds.

**Tests**
- Names follow `test_{what}_{condition}_{expected}`, with each module's prefix: `test_violations_…`,
  `test_mapping_…`, `test_snapshot_…`, `test_committed_0803_…`.
- `mapping_tests.rs` and `snapshot_tests.rs` are `src/` modules (`lib.rs:103-106`); `committed_0803.rs` is
  under `tests/`.
- Each new `InvalidFact` gets a single-fault test that fails without its check, as in the sync plan's
  strictness round. A test that asserts a specific variant asserts it exactly, e.g. the `ExtraResourceType`
  test names the third type.
- Tests that assert annotations are omitted flip:
  - `mapping_tests.rs:142`, `:181` and `:210`. Their fixtures already carry `oa:motivatedBy` and
    `oa:hasTarget`.
  - `mapping_tests.rs:797`, whose fixture lacks both, so they are added.
  - `mapping_tests.rs:897`, which uses `dao:color` as its unlisted datatype. It moves to one the port still
    omits.
  - `snapshot_tests.rs:900`. Its `c0mM` node lacks `valueHasUUID`, so the file becomes `Unavailable` with
    `MissingValueUuid`.
- `committed_0803.rs:104` (the superseded value on Region `GOkuI…`) passes vacuously today. Once the Region is
  served it becomes a real check, and a new assertion pins the Region's live comment.
- Counts are literal constants taken from the committed file, and are never relaxed to fit
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`).

**InvalidFact messages** are lowercase noun phrases, like the existing ones:

| Variant | Message |
|---------|---------|
| `MissingMotivation` | "an annotation without a motivation" |
| `UnknownMotivation` | "an annotation with the unknown motivation {motivation}" |
| `MissingAnnotationTarget` | "an annotation without a target" |
| `UnknownTarget` | "an annotation targeting {target}, which is not a resource" |
| `UnservedAnnotation` | "an annotation that is not a resource" |
| `StrayAnnotationFact` | "{predicate} on a node that is not an annotation" |

`StrayAnnotationFact`'s doc comment says that its `subject` can be a value or representation node.

**Commits (dsp-repository)**
- The work runs on the worktree's branch `worktree-DEV-7486`.
- The PR carries two commits and ticks `allow-many-commits`, with Review Notes:
  - this plan: `docs(docs): add the plan for serving annotations through cpe-ports (DEV-7486)`;
  - one feature commit: `feat(cpe-ports,sync-store): serve Regions and LinkObjs through the port (DEV-7486)`.
- Comma-separated scopes are in use on `main`, e.g. `6663ba00 fix(shared-metadata,dpe-server)`.
- Phase 1 creates the feature commit. Phase 2's changes and every review fix are amended into it, which works
  because it is `HEAD` from then on. The phases are review checkpoints, not separate commits: a field nothing
  fills is not a feature of its own (`docs/src/git-conventions.md`, *One PR, several commits, or a stack*).

**Incubator**
- PRs land as one squashed commit, titled as a conventional commit with a `(#<PR>)` suffix
  (`dsp-incubator/CLAUDE.md`).
- `cpe/vendor/README.md` ("Moving the pin", on #471):
  - Run `just cpe vendor <sha>`, then `just cpe test`.
  - Commit `PIN`, the changed tree and any `Cargo.lock` change together.
  - Afterwards, `git status --porcelain --ignored cpe/vendor` lists no ignored file.
  - The pin must be a commit on the monorepo's `main`.
- `just cpe ci` runs `vendor-diff` first, and needs network access to GitHub.

**Reviewers**, for each phase review:
- `eng:review:rust-reviewer`
- `eng:review:dune-reviewer`
- `eng:review:consistency-reviewer`
- `eng:review:code-simplicity-reviewer`

## Implementation Phases

#### Phase 1: `cpe-ports` gains annotations and the two value kinds

### dsp-repository
- [x] Commit this plan as `docs(docs): add the plan for serving annotations through cpe-ports (DEV-7486)`
- [x] Add `ValueKind::Geometry(String)` to `snapshot.rs`, documented as above
- [x] Add `ValueKind::Color(String)` to `snapshot.rs`, documented as above
- [x] Extend the `ValueKind` doc's pass-through sentence to name `Geometry` and `Color`
- [x] Add `Annotation` and `Motivation` to `snapshot.rs`, documented as above
- [x] Add `Resource.annotation: Option<Annotation>` to `snapshot.rs`
- [x] Re-export `Annotation` and `Motivation` from `lib.rs`
- [x] Declare `Violation::DanglingTarget` and `Violation::UntargetedAnnotation` after `DanglingLink`, with their `Display` arms
- [x] Set `annotation: None` in `fake.rs:66`
- [x] Set `annotation: None` in `contract.rs`'s `resource()` helper (`:349`)
- [x] Set `annotation: None` in `mapping_tests.rs:844`
- [x] Set `annotation: None` in `resources.rs:49`, without changing behaviour
- [x] Write `contract.rs` test `test_violations_annotation_target_missing_reports_dangling_target`
- [x] Write `contract.rs` test `test_violations_annotation_without_target_reports_untargeted_annotation`
- [x] Extend `valid_snapshot()` with an annotation whose targets are all in the snapshot, so `test_violations_valid_snapshot_reports_nothing` covers it
- [x] Write `contract.rs` test: a `Geometry` value without a UUID reports `MissingValueUuid`
- [x] Write `contract.rs` test: a `Color` value without a UUID reports `MissingValueUuid`
- [x] Add `DanglingTarget` and `UntargetedAnnotation` cases to the documented-order test (`contract.rs:758-805`)
- [x] Implement the `DanglingTarget` check in `violations`
- [x] Implement the `UntargetedAnnotation` check in `violations`
- [x] Review `contract.rs`'s module doc (`:3-8`) and the `violations` doc against the two new invariants
- [x] Run `just check && just test`; it passes
- [x] Commit as `feat(cpe-ports,sync-store): serve Regions and LinkObjs through the port (DEV-7486)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: `sync-store` serves Regions and LinkObjs from 0803

### dsp-repository
- [x] Add `DAO_GEOMETRY` and `DAO_COLOR` to `vocab.rs`, citing `FORMAT.md` §4.8 and §4.9
- [x] Add `OA_MOTIVATED_BY`, `OA_HAS_TARGET`, `OA_COMMENTING`, `OA_HIGHLIGHTING` and `OA_LINKING` to `vocab.rs`, citing `FORMAT.md` §10
- [x] Add the six new `InvalidFact` variants to `error.rs` with the messages above
- [x] Rewrite the Region test (`mapping_tests.rs:142`): served with class, label, motivation `Commenting`, the page as target, geometry, color and comment values and an `isRegionOf` link; the page keeps its `hasRegion` link
- [x] Rewrite the project-subclass test (`mapping_tests.rs:181`): a project subclass of `kb:Region` typed `oa:Annotation` is served with `annotation` set
- [x] Rewrite the LinkObj test (`mapping_tests.rs:210`): two `hasLinkTo` targets, motivation `Linking`, both targets sorted, two `Link` values
- [x] Rewrite the part_of test (`mapping_tests.rs:797`): give the annotation `oa:motivatedBy` and `oa:hasTarget`; a resource `isPartOf` it keeps the parent and its seqnum
- [x] Write a mapping test: a Region motivated by `oa:highlighting` is served with motivation `Highlighting`
- [x] Write a mapping test: a resource not typed `oa:Annotation` is served with `annotation: None`
- [x] Write a mapping test: an annotation with two `oa:hasTarget` to one IRI (one quad each in the fixture's graph) lists it once
- [x] Write a mapping test: a geometry value is served as `Geometry` with its JSON lexical unchanged, whitespace and key order included
- [x] Write a mapping test: a color value is served as `Color` with its lexical unchanged
- [x] Switch the unlisted-datatype test (`mapping_tests.rs:897`) from `dao:color` to a datatype the port still omits
- [x] Invert `snapshot_tests.rs:900`: a broken value node on an annotation returns `Unavailable` with `MissingValueUuid`
- [x] Write a snapshot test: an annotation without `oa:motivatedBy` returns `MissingMotivation`
- [x] Write a snapshot test: an annotation with two different `oa:motivatedBy` IRIs returns `RepeatedPredicate`
- [x] Write a snapshot test: an annotation motivated by `oa:tagging` returns `UnknownMotivation`
- [x] Write a snapshot test: an annotation whose `oa:motivatedBy` is a literal returns `UnfitLiteral`
- [x] Write a snapshot test: an annotation without `oa:hasTarget` returns `MissingAnnotationTarget`
- [x] Write a snapshot test: an annotation whose `oa:hasTarget` is a literal returns `UnfitLiteral`
- [x] Write a snapshot test: an annotation targeting an IRI that is no resource of the file returns `UnknownTarget`
- [x] Write a snapshot test: a resource not typed `oa:Annotation` carrying `oa:hasTarget` returns `StrayAnnotationFact`
- [x] Write a snapshot test: a value node carrying `oa:motivatedBy` returns `StrayAnnotationFact`
- [x] Write a snapshot test: a subject typed `oa:Annotation` but not `dao:Resource` returns `UnservedAnnotation`
- [x] Write a snapshot test: an annotation with a third `rdf:type` returns `ExtraResourceType` whose `class` is that third type
- [x] Rewrite and rename the annotations test (`committed_0803.rs:58-68`): 4,198 resources; Region `GOkuI_IxVuSKMRZmCypz7Q` and LinkObj `00bnHlmDVIq_Blb4DvKGiQ` are served
- [x] Rename and extend the by-class test (`committed_0803.rs:48-54`) with `http://www.knora.org/ontology/knora-base#Region` 77 and `…#LinkObj` 40
- [x] Write a committed test: every Region has exactly one `Geometry` value under `kb:hasGeometry`
- [x] Write a committed test: every Region has exactly one `Color` value under `kb:hasColor`
- [x] Write a committed test: every Region has exactly one `Text` value under `kb:hasComment`
- [x] Write a committed test: every Region's `annotation.targets` equals the target of its single `kb:isRegionOf` link
- [x] Write a committed test: the LinkObjs carry 79 `kb:hasLinkTo` links in all
- [x] Write a committed test: each LinkObj's `annotation.targets` equals its `kb:hasLinkTo` link targets
- [x] Write a committed test: 117 resources have `annotation: Some`, 77 with `Commenting` and 40 with `Linking`
- [x] Write a committed test: Region `089fJhP1WuylV1wftl5Y_Q` serves color `#ff3333`
- [x] Write a committed test: Region `089fJhP1WuylV1wftl5Y_Q` serves a geometry starting `{"status":"active","lineColor":"#ff3333"` with `"type":"rectangle"`
- [x] Extend the superseded-value test (`committed_0803.rs:104`): Region `GOkuI_IxVuSKMRZmCypz7Q` serves its live comment
- [x] Add `Geometry` and `Color` arms to `kind()` (`values.rs`)
- [x] Add the pre-pass over `index.subjects` that rejects `UnservedAnnotation`
- [x] Extend that pre-pass to reject `StrayAnnotationFact`
- [x] Serve every `dao:Resource` in `resources.rs`, and rewrite its doc comment (`:13-14`)
- [x] Relax the `rdf:type` check (`resources.rs:36`) to skip `oa:Annotation`
- [x] Remove the unreachable annotation-parent arm in `part_of` (`resources.rs:72`)
- [x] Rewrite the `part_of` doc (`resources.rs:60-61`)
- [x] Map `oa:motivatedBy` to `Motivation` for each subject typed `oa:Annotation`
- [x] Refuse a missing, literal or unknown `oa:motivatedBy`
- [x] Map `oa:hasTarget` to sorted, deduplicated `targets`
- [x] Refuse a missing, literal or unserved `oa:hasTarget`
- [x] Update the comment at `values.rs:36-37` that names annotations
- [x] Update the `sync-store` crate doc (`lib.rs:20-27`), including "as a link to an annotation is"
- [x] Rewrite the `mapping/mod.rs` module doc's "drop, then validate, then map" (`:3-6`)
- [x] Update `areas/access/cpe/CONTEXT.md` row 31: geometry and color move to In; geoname, time and interval stay out with the geolocation reason
- [x] Update `areas/access/cpe/CONTEXT.md` row 34: Regions, LinkObjs and their `oa:` facts move to In; `isAnnotationOf` is served like any link; LinkValue reifications stay out; the "open engine gap" reason goes
- [x] Add annotations to the **Archive-shaped fact** paragraph of `areas/access/cpe/CONTEXT.md` (lines 19-20)
- [x] Add to `areas/access/cpe/CONTEXT.md`'s omission lines (`:25`, `:41`) that an annotation target outside the file is refused, unlike a link
- [x] Add an **Annotation** term to `areas/access/cpe/CONTEXT.md`: a resource marked `oa:Annotation`, found by `Resource.annotation`, never by class
- [x] Grep `areas/access/` for wording that still says annotations are omitted or dropped, and fix each hit
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry with `dune:dune-map`
- [x] Refresh `ARCH-MAP.md`'s `areas/access/sync` entry with `dune:dune-map`
- [x] Run `just check && just test`; it passes
- [x] Amend Phase 2's changes into the `feat(cpe-ports,sync-store)` commit, which is `HEAD`
- [x] Run `just commit-lint`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Incubator vendors the new `cpe-ports`

**Done separately:** this phase ran as dsp-incubator#479 (pin `7118cdbf`), before Phase 3 of
`docs/specs/2026-10-08-cpe-port-data-arks/01-feat-cpe-port-data-arks-plan.md` (DEV-7487), which then
vendored only the data ARKs (dsp-incubator#486). The checkboxes below are ticked from #479's commit and
test plan; its Phase review is unrecorded.

**Gate: H1**: resolve before starting this phase.

### dsp-incubator
- [x] Create a branch off up-to-date `origin/main`
- [x] Run `just cpe vendor <sha>` with the merged dsp-repository commit on `main`; `cpe/vendor/PIN` holds that sha
- [x] Grep `cpe/` outside `cpe/vendor/` for `cpe_ports`; record in the commit body whether any use exists
- [x] Add `Geometry` and `Color` arms to every `ValueKind` match that grep found
- [x] Add `annotation` to every `cpe_ports::Resource` literal that grep found
- [x] Run `just cpe test`; it passes
- [x] Run `git status --porcelain --ignored cpe/vendor`; it lists no ignored file
- [x] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<sha>)`
- [x] Run `just cpe ci`; it passes
- [x] Commit `PIN`, the vendored tree and any `Cargo.lock` change together as `chore(cpe): vendor cpe-ports with annotations (DEV-7486)`
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| ID | Action | Who | When | Why an agent cannot |
|----|--------|-----|------|---------------------|
| H1 | Merge the dsp-repository PR for this plan, and dsp-incubator#471 (DEV-7462, `just cpe vendor`) | Balduin | before Phase 3 (Why mid-plan: the vendor pin must be the merged `main` commit, which a rebase-merge mints only on merge, and `just cpe vendor` exists only once #471 merges) | Merging is the owner's decision |
| H2 | Merge the dsp-incubator re-vendor PR | Balduin | after ship | Merging is the owner's decision |

## Acceptance Criteria

**Serving 0803**
- [x] `LiveArchiveProjection` serves 4,198 resources for 0803, among them 77 `kb:Region` and 40 `kb:LinkObj`.
- [x] `contract::violations` is empty for that snapshot.
- [x] Every Region carries its geometry (verbatim JSON), its color, its comment and its `isRegionOf` link.
- [x] Every LinkObj carries its comment and its `hasLinkTo` links, 79 in all.
- [x] Exactly the 117 annotations have `Resource.annotation`: 77 `Commenting` and 40 `Linking`, with targets
      equal to their links' targets.

**Strictness**
- [x] Each way the file can break an annotation rule makes it `Unavailable` with its own `InvalidFact`.
- [x] A dangling target or an untargeted annotation in a snapshot is a `Violation`.

**Documentation**
- [x] `areas/access/cpe/CONTEXT.md` lists geometry, color, Regions and LinkObjs as served, with no stale
      reason left.

**Incubator**
- [x] The incubator vendors the merged commit; `just cpe vendor-diff` is empty and `just cpe ci` passes.

## Dependencies & Risks

- **dsp-incubator#471 (DEV-7462)** is open but complete. The owner expects it to merge soon. Phases 1–2 do not
  depend on it, and execution stops at the H1 gate.
- **The incubator may gain a consumer first.** If DEV-7402 work lands a `ValueKind` match or a `Resource` literal
  before Phase 3, the re-vendor adds the new arms (Phase 3 greps for them).
- **DAO d.71 may grow.** The full Web Annotation mapping, with selectors, would change `Annotation`. The field is
  marked provisional, and a change would land as a port PR here followed by a re-vendor.
- **A new check can take a project offline.** A new contract invariant or `InvalidFact` can do so (see the
  ARCH-MAP note on `sync-store`). The 0803 facts above were checked, and none of the new checks fires on them.
  For an unknown motivation the cost is accepted: the motivation set is closed, as are the other DTO enums.
- **Not in scope:**
  - ARKs (DEV-7487);
  - value order and trailing whitespace (DEV-7402 open questions 4 and 5);
  - curation of test Regions and LinkObjs (DEV-7488);
  - geometry normalisation (DEV-7402's hook).

## Success Metrics

| 0803 | Before | After |
|------|--------|-------|
| Resources served | 4,081 | 4,198 |
| Regions / LinkObjs | 0 / 0 | 77 / 40 |
| `hasLinkTo` / `isRegionOf` links | 0 / 0 | 79 / 77 |
| Resources with `annotation` | — | 117 (77 `Commenting`, 40 `Linking`) |

Two further conditions must also hold:
- `contract::violations` on 0803 stays empty.
- The incubator's `vendor/PIN` names a commit that contains this change.

## References

**Tickets**
- Linear DEV-7486 (blocks DEV-7402; related to DEV-7462).
- The DEV-7402 pre-planning inventory (Linear document, 2026-10-08): "Port gaps" and open question 2.

**Decisions**
- ADR-0003 (consumer-defined ports).
- ADR-0007 (the archive-shaped port; the DEV-7399 and DEV-7400 amendments).
- ADR-0008 (the read model is fed by its port).
- dsp-incubator ADR-0013, with its 2026-10-02 amendment.
- dsp-incubator `cpe/tools/dao-lift/FORMAT.md`: §4.8, §4.9, §10, §11, §12 D10/D11 and §15.

**Earlier plans**
- `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`
- `docs/specs/2026-09-29-minimal-sync-capability/01-minimal-sync-capability-PRD.md` (REQ-R2.4)
- `docs/specs/2026-09-29-minimal-sync-capability/02-feat-minimal-sync-capability-plan.md`
- dsp-incubator#471:
  - `cpe/docs/plans/vendor-monorepo-crates/implementation-plan.md`
  - `cpe/vendor/README.md`

**Code**

`cpe-ports`:
- `areas/access/cpe/ports/src/{snapshot,contract,fake,lib}.rs`

`sync-store`:
- `areas/access/sync/store/src/{lib,error,vocab}.rs`
- `areas/access/sync/store/src/mapping/{mod,resources,values}.rs`
- `areas/access/sync/store/src/{mapping_tests,snapshot_tests}.rs`
- `areas/access/sync/store/tests/committed_0803.rs`
