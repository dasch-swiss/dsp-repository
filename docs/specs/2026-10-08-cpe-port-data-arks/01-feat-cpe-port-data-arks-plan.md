---
title: "feat: Serve data ARKs through cpe-ports"
type: feat
date: 2026-10-08
author: "Balduin Landolt"
status: complete
linear: DEV-7487
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7487
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator
---

# feat: Serve data ARKs through cpe-ports

## Overview

Every `cpe_ports::Resource` gains its plain data ARK, `Resource.ark: DataArk`, e.g.
`https://ark.dasch.swiss/ark:/72163/1/0803/CDYZPN5zVVKbIcjA1DZxKQO`. `sync-store` computes it from the
resource IRI with a port of `dao-lift`'s `resource_iri_to_ark`, until DAO carries the ARK itself. The
contract gains a shape check, `Violation::MalformedDataArk`. A committed list of the incubator's 4,143
0803 ARKs is the oracle for the committed-file tests. One incubator re-vendor then picks up this change and
DEV-7486's together.

There is no PRD. The requirements are Linear DEV-7487 and the DEV-7402 pre-planning inventory (Linear
document, 2026-10-08). The work is stacked on PR #464 (DEV-7486, branch `worktree-DEV-7486`), whose
annotations this plan's Region ARKs need.

## Problem Statement / Motivation

CPE's "Show in dataset" panel (Incunabula Slice 9) links each resource and Region back to its DSP data,
and the incubator engine's `MalformedDataArk` boot check rejects any stored ARK that is not a plain data
ARK. For 0803 that is 4,079 resource ARKs and 64 Region ARKs. DEV-7402 builds Incunabula's store from the
port, so it needs the port to serve them.

Today:
- `0803.nq` carries no ARK, and the port has no field for one (`areas/access/cpe/ports/src/snapshot.rs:72-98`).
- `areas/access/cpe/CONTEXT.md:42` lists ARKs as out: "A data ARK is added when CPE links back to the
  data it presents". "Show in dataset" now does, so this is the change that row anticipated.
- The port plan (`docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`)
  deferred ARKs because "No ARK points at CPE yet (ADR-0007)". That stays true: ADR-0007's 2026-10-02
  amendment separates data ARKs, which resolve to the DSP data and never to CPE, from presentation ARKs.
  This plan serves data ARKs only, so it fulfils the deferral rather than reversing it.
- The incubator computes the ARKs in `dao-lift` (`cpe/tools/dao-lift/src/ark.rs`, `resource_iri_to_ark`),
  a port of dsp-api's `resourceIriToArkUrl` and its `Base64UrlCheckDigit`.

**Decision (Linear DEV-7487):** the port serves the ARK, computed in `sync-store` from the IRI until DAO
carries it. CPE does not compute it: the algorithm is archive identity, and a copy in CPE would be a
second implementation to keep in step with dsp-api.

## Proposed Solution

The owner decided on 2026-10-08:
- the field is a required newtype, `ark: DataArk`;
- the contract checks the ARK's shape;
- the oracle is the full list of the incubator's 0803 ARKs, committed beside `0803.nq`;
- one incubator re-vendor covers DEV-7486 and DEV-7487.

### Boundary DTO (`cpe-ports`, `snapshot.rs`)

```rust
iri! {
    /// A resource's plain data ARK URL, e.g. `https://ark.dasch.swiss/ark:/72163/1/0803/<id><digit>`;
    /// never a value or version ARK, and never resolving to CPE (ADR-0007). [`violations`] checks its shape.
    DataArk
}

pub struct Resource {
    pub iri: ResourceIri,
    /// The resource's data ARK. The adapter derives it from `iri` with dsp-api's algorithm.
    ///
    /// Provisional: DAO does not carry the ARK yet; once it does, the adapter reads it.
    pub ark: DataArk,
    // … unchanged fields …
}
```

- `DataArk` is declared with the existing `iri!` macro (`snapshot.rs:13-31`), so it derives what the IRI
  newtypes derive and has `as_str()` and `Borrow<str>`. It is an ARK URL, not an IRI, and its doc says so.
- `ark` sits directly after `iri`: both name the resource.
- `DataArk` is re-exported from `lib.rs`.
- Not `Option`: every DSP resource has a data ARK, so `None` would only hide an adapter fault.

### Contract (`contract.rs`)

`Violation::MalformedDataArk { resource: ResourceIri, ark: DataArk }`: a resource whose `ark` is not
`https://ark.dasch.swiss/ark:/72163/1/<shortcode>/<id>`, with a `[0-9A-F]{4}` shortcode and a non-empty
`[A-Za-z0-9_=]+` id.

- Declared directly after `DuplicateResource`, with `resource` first, since `Ord` follows declaration order
  and then field order (`contract.rs:20-22`).
- A `Display` arm in the existing `resource {} …` form, a `test_violations_…` test, and a case in the
  documented-order test (`contract.rs:845`).
- The check is hand-written, because `cpe-ports` depends on `std` alone (`ARCH-MAP.md`). It is the same
  pattern as `dao-lift`'s `is_plain_data_ark` and the incubator engine's boot check.
- One violation per resource entry, like the other per-resource checks: two entries of one IRI with the same
  bad ARK report it twice, beside `DuplicateResource`.
- It checks shape only. Whether the ARK belongs to the IRI is the adapter's promise: recomputing it here
  would put the algorithm into CPE's crate, which the decision rules out.
- The check does not compare the ARK's shortcode with the snapshot's; the shortcode is part of the
  identifier, and checking it adds nothing the adapter's derivation does not already guarantee.
- `sync-store` cannot produce a malformed ARK, since its derivation refuses first. The contract still
  checks, for every adapter and every `FakeArchiveProjection` fixture.

### Derivation (`sync-store`, new `src/ark.rs`)

A port of `dsp-incubator/cpe/tools/dao-lift/src/ark.rs` at `c2b3e10a8d7c695347e11ed7a5cf31bf0f3dca64`, without
`is_plain_data_ark` (the contract has that rule now). That commit is the only one that touched `ark.rs`, and it is
an ancestor of the `dao-lift` pin `b2226ff4` in `PROVENANCE`, so the file is the same at both (checked
2026-10-08).

- The prefix `https://ark.dasch.swiss/ark:/72163/1/` (resolver, whatever environment the dump came from;
  NAAN; ARK version) is `cpe_ports::DATA_ARK_PREFIX`, used by both the contract check and the derivation
  (Phase 1 review: one constant instead of two spellings).
- `check_digit(id) -> Result<char, ArkError>` (`dao-lift` returns a boxed error): over the base64url
  alphabet, position `i` of an id of length `n` weighs `n + 1 - i`; the digit is the character of value
  `(64 - total % 64) % 64`.
- `data_ark(iri: &str) -> Result<DataArk, ArkError>`: requires `http://rdfh.ch/<shortcode>/<id>` with a
  `[0-9A-Fa-f]{4}` shortcode and one path segment of id, as dsp-api's `ResourceIri` does
  (`slice/common/Iris.scala:53-54`); upper-cases the shortcode in the ARK, as dsp-api's `Shortcode.from`
  (`KnoraProject.scala:109-112`) and ark-resolver do; appends the check digit; escapes every `-` as `=`,
  the check digit's included (`StringFormatter.scala:1358-1419`).
- **One deliberate difference from `dao-lift`:** `dao-lift` rejects a lowercase shortcode. This port follows
  dsp-api, the authority both are ports of, so a project whose IRIs use a lowercase shortcode is served
  rather than taken offline. 0803's shortcode has no letters, so the oracle cannot tell the two apart; a
  unit test pins the upper-casing.
- `ArkError { NotResourceIri, InvalidChar(char), Empty, ZeroSum }`, deriving
  `Debug, Clone, PartialEq, Eq, thiserror::Error`, re-exported from `lib.rs` because `InvalidFact` carries
  it. `NotResourceIri` carries no IRI: the `Invalid` error already names the subject. The ported tests
  therefore assert `ArkError::NotResourceIri` without a payload, and compare `ArkError` directly, without
  `dao-lift`'s `ark_error` downcast helper.
- `check_digit` and `data_ark` are `pub(crate)`; `mod ark` stays private, and only `ArkError` is
  re-exported, beside `InvalidFact` (`lib.rs:43`). `ZeroSum` is reachable: a non-empty all-`A` id weighs 0.
- The module doc says what a reader would otherwise break, not history (CONVENTIONS.md, *Comments*):
  - it must stay equal to dsp-api's `resourceIriToArkUrl` and `Base64UrlCheckDigit`;
  - the prefix is also spelled in `cpe-ports`' `MalformedDataArk` check and in the incubator's
    `translate.py` and `engine/src/ir.rs`, so a change goes to all of them.
  The `dao-lift` commit it was ported from is recorded in this plan, not in the code.
- Module tests in `src/ark_tests.rs`, a `#[cfg(test)]` module like `mapping_tests.rs` (`lib.rs:104-107`),
  carry `dao-lift`'s vectors: dsp-api's `cmfk1DMHRBiR4-_6HXpEFA` → `n`, the three mutated ids, the eleven
  production 0803 ARKs, the `-` check digit, every rejected IRI shape and each `ArkError` kind.
- The production resolver and NAAN are confirmed in ops-deploy: `group_vars/dsp_prod.yml:5`
  (`ARK_RESOLVER_URL: "https://ark.dasch.swiss"`) and `roles/dsp-deploy/templates/docker-compose-svc.yml.j2:70-71`
  (`KNORA_WEBAPI_ARK_NAAN=72163`).

**Mapping.** `resources.rs::resource` (`:51-88`) sets `ark: ark::data_ark(iri)`, mapping an error to
`InvalidFact::NoDataArk { reason: ArkError }` with the message "a resource whose IRI has no data ARK:
{reason}". It is a struct variant like every other payload-carrying `InvalidFact` (`error.rs:43-122`), and the
field is not named `source`, which thiserror would treat as the error's source. A file
whose served resource yields no ARK is `Unavailable`, like every other broken fact the port serves.

**Docs in `sync-store`.**
- `lib.rs`'s "Where a check goes" paragraph (`:13-19`) adds the ARK to the rules the contract checks
  again, with the mapping naming the fault first.
- The crate doc says the ARK is derived, not read, and why.

### Oracle (`areas/access/sync/data/0803-arks.txt`)

- One ARK per line, sorted, 4,143 lines: every `https://ark.dasch.swiss/ark:/72163/1/0803/…` in
  `dsp-incubator/cpe/projects/incunabula/data.sql` at `c2b3e10a8d7c695347e11ed7a5cf31bf0f3dca64` (Slice 9,
  the commit that last changed it), deduplicated. That is the `resources.ark` and `annotations.ark` columns.
- `PROVENANCE` gains a section for it, appended after the existing text: the source file, the incubator
  commit, the extraction command, and that the list is `dao-lift`'s output as Incunabula stores it, a
  subset of the served resources (no LinkObj, 64 of 77 Regions, no calibration-scan Pages). It names the
  commit as `commit c2b3e10a…`, never in the `dsp-incubator@<sha>` form:
  `test_committed_0803_provenance_pin_matches_vocab_pin` reads the first `dsp-incubator@` in the file as
  the `dao-lift` pin.
- The extraction command, run in the incubator checkout at that commit:
  `grep -oE 'https://ark\.dasch\.swiss/ark:/72163/1/0803/[A-Za-z0-9_=]+' cpe/projects/incunabula/data.sql | LC_ALL=C sort -u`.
- `LiveArchiveProjection` reads only `<dir>/<shortcode>.nq` (`lib.rs:61`), so the extra file is not served.
- The committed test reads the list, recovers each ARK's resource IRI (drop the last character, unescape
  `=` to `-`; base64url has no `=`, so this is unambiguous), and asserts the served resource's `ark`
  equals the line. Count is the literal 4,143; a missing resource fails the test.

### Facts in 0803 (checked during planning, 2026-10-08)

Checked by script over `areas/access/sync/data/0803.nq` at `2b0d86fe` and `data.sql` at `c2b3e10a`.

- 4,198 subjects are typed `dao:Resource`. Every IRI is `http://rdfh.ch/0803/<id>` with `id` in
  `[A-Za-z0-9_-]`, so every one yields an ARK.
- `data.sql` holds 4,143 distinct 0803 ARKs, and every one's resource IRI is among the 4,198 served.
- The 4,143 are 4,079 resources (19 Books, 4,022 Pages, 38 Bands) and 64 Regions; the
  ticket's counts.

## Technical Considerations

**Archive shape (ADR-0007).** The ARK is archive identity, not a remodel: the adapter derives it the way
dsp-api does, and CPE only stores and checks it. ADR-0007 needs no amendment; its 2026-10-02 amendment
already says data ARKs identify the DSP data and are never re-pointed at CPE.

**Consumers**
- The change breaks consumers on purpose: no type is `#[non_exhaustive]` (`lib.rs:8-11`).
- `Resource` struct literals that need `ark` (repo research, 2026-10-08):
  - `contract.rs:379`, the `resource()` test helper; every other test spreads from it. It builds a
    shape-valid ARK from the id, e.g. `…/0803/<id with '-' as '='>`; the contract checks shape only.
  - `fake.rs:66`, `zz-book`: `https://ark.dasch.swiss/ark:/72163/1/0803/zz=booko`, the ARK of
    `http://rdfh.ch/0803/zz-book` (check digit `o`, computed during planning with the algorithm above).
  - `mapping_tests.rs:191` and `:1091`, with the ARK of each fixture IRI.
  - `resources.rs:78`, the production site.
- In the incubator, nothing outside `cpe/vendor/` uses `cpe_ports` (grep, 2026-10-08). Phase 3 greps again.

**Tests**
- Names follow `test_{what}_{condition}_{expected}` with each module's prefix: `test_ark_…`,
  `test_violations_…`, `test_mapping_…`, `test_snapshot_…`, `test_committed_0803_…`.
- Each `ArkError` kind and `NoDataArk` get a single-fault test that fails without its check.
- Counts are literal constants and never relaxed to fit
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`). The oracle test reads a committed
  file and is never skipped.
- The ARK check digit depends only on the id: the NAAN and shortcode are not inputs
  (`dasch-specs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`). That learning
  also records that ark-resolver upper-cases the shortcode, as dsp-api does; this port does too (see
  *Derivation*).

**Commits (dsp-repository)**
- Branch `worktree-DEV-7487`, stacked on `worktree-DEV-7486` with `gh stack` (already initialised).
- The PR carries two commits and ticks `allow-many-commits`, with Review Notes, as #464 does:
  - this plan: `docs(docs): add the plan for serving data ARKs through cpe-ports (DEV-7487)`;
  - one feature commit: `feat(cpe-ports,sync-store): serve data ARKs through the port (DEV-7487)`.
- Phase 1 creates the feature commit; Phase 2's changes and every review fix are amended into it.
- Phase 1 keeps the field and its derivation together: a required field cannot land without the adapter
  filling it, and a placeholder value would be a fake ARK in a serving path.
- The PR is opened with `gh stack submit --auto` as a draft, based on `worktree-DEV-7486`, assigned to
  Balduin.

**Incubator**
- `cpe/vendor/PIN` is `a7e8df14`, from before DEV-7486. dsp-incubator#471 (`just cpe vendor`) has merged.
- DEV-7486's plan (`docs/specs/2026-10-08-cpe-port-annotations/01-feat-cpe-port-annotations-plan.md`)
  still has its own incubator phase. This plan's Phase 3 replaces it; the plan commit adds a note there.
- Re-vendoring is described in `cpe/vendor/README.md` ("Moving the pin"): `just cpe vendor <sha>`, then
  `just cpe test`; commit `PIN`, the tree and any `Cargo.lock` change together; never `cargo fmt` the
  vendored tree; the pin is a commit on the monorepo's `main`.
- PRs land as one squashed commit titled as a conventional commit (`dsp-incubator/CLAUDE.md`).

**Reviewers**, for each phase review:
- `eng:review:rust-reviewer`
- `eng:review:dune-reviewer`
- `eng:review:consistency-reviewer`
- `eng:review:code-simplicity-reviewer`

## Implementation Phases

#### Phase 1: The port serves a data ARK, and `sync-store` derives it

### dsp-repository
- [x] Commit this plan, with a note under DEV-7486's Phase 3 heading that this plan's Phase 3 replaces it, as `docs(docs): add the plan for serving data ARKs through cpe-ports (DEV-7487)`
- [x] Declare `DataArk` with the `iri!` macro in `snapshot.rs`, documented as above
- [x] Add `Resource.ark: DataArk` after `iri` in `snapshot.rs`, documented as above
- [x] Re-export `DataArk` from `cpe-ports`' `lib.rs`
- [x] Declare `Violation::MalformedDataArk` after `DuplicateResource`, with its `Display` arm
- [x] Write `contract.rs` test `test_violations_version_ark_reports_malformed_data_ark`
- [x] Write `contract.rs` test `test_violations_lowercase_shortcode_ark_reports_malformed_data_ark`
- [x] Write `contract.rs` test `test_violations_unescaped_dash_ark_reports_malformed_data_ark`
- [x] Write `contract.rs` test `test_violations_foreign_resolver_ark_reports_malformed_data_ark`
- [x] Add a `MalformedDataArk` case to the documented-order test (`contract.rs:845`)
- [x] Set a shape-valid `ark` in `contract.rs`'s `resource()` helper (`:379`), so `test_violations_valid_snapshot_reports_nothing` covers the check
- [x] Implement the `MalformedDataArk` check in `violations`
- [x] Review `contract.rs`'s module doc and the `violations` doc against the new invariant
- [x] Write `src/ark_tests.rs` with `dao-lift`'s vectors: dsp-api's check digit, the three mutated ids, the eleven production 0803 ARKs, the `-` check digit
- [x] Write `ark_tests.rs` tests: each rejected IRI shape returns `NotResourceIri` (`dao-lift`'s list without `http://rdfh.ch/080a/x`, plus `http://rdfh.ch/080g/x`)
- [x] Write `ark_tests.rs` test: `http://rdfh.ch/081c/a` yields an ARK with shortcode `081C`
- [x] Write `ark_tests.rs` tests: `InvalidChar`, `Empty` and `ZeroSum`, one each
- [x] Write `ark_tests.rs` test: every computed ARK passes the contract's shape rule (no `MalformedDataArk` on a one-resource snapshot)
- [x] Add `src/ark.rs` with `check_digit`, `data_ark` and `ArkError`, documented as above
- [x] Register `ark` and `ark_tests` in `sync-store`'s `lib.rs`, and re-export `ArkError`
- [x] Add `InvalidFact::NoDataArk { reason: ArkError }` to `error.rs` with the message above
- [x] Write a snapshot test: a resource IRI under `http://example.org/` returns `Unavailable` with `NoDataArk { reason: ArkError::NotResourceIri }`
- [x] Write a snapshot test: a resource id with `.` returns `Unavailable` with `NoDataArk { reason: ArkError::InvalidChar('.') }`
- [x] Set `ark` in `resources.rs::resource` from `ark::data_ark`, mapping the error to `NoDataArk`
- [x] Set `ark` in `mapping_tests.rs:191` and `:1091` to the ARK of each fixture IRI
- [x] Write a mapping test: a served resource's `ark` is its IRI's data ARK (one of the eleven production vectors)
- [x] Set `ark` in `fake.rs:66` to `https://ark.dasch.swiss/ark:/72163/1/0803/zz=booko`
- [x] Write `ark_tests.rs` test: `http://rdfh.ch/0803/zz-book` yields `fake.rs`'s ARK
- [x] Update `sync-store`'s `lib.rs` crate doc: the ARK is derived from the IRI, and the contract checks its shape again
- [x] Run `just check && just test`; it passes
- [x] Commit as `feat(cpe-ports,sync-store): serve data ARKs through the port (DEV-7487)`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: 0803 serves `dao-lift`'s ARKs

### dsp-repository
- [x] Extract every distinct 0803 ARK from `dsp-incubator/cpe/projects/incunabula/data.sql` at `c2b3e10a` into `areas/access/sync/data/0803-arks.txt` with the command above; `wc -l` is 4143
- [x] Append a `0803-arks.txt` section to `areas/access/sync/data/PROVENANCE`: source file, `commit c2b3e10a…` (not the `dsp-incubator@` form), extraction command, and that it is a subset of the served resources
- [x] Write a committed test: every line of `0803-arks.txt` equals the `ark` of the served resource its id names; the file has 4,143 lines
- [x] Write a committed test: Book `CDYZPN5zVVKbIcjA1DZxKQ` serves `https://ark.dasch.swiss/ark:/72163/1/0803/CDYZPN5zVVKbIcjA1DZxKQO`
- [x] Write a committed test: Region `089fJhP1WuylV1wftl5Y_Q` serves the ARK `0803-arks.txt` lists for it
- [x] Update `areas/access/cpe/CONTEXT.md:42`: the data ARK moves to a new In row (every resource, annotations included, derived by the adapter from the IRI, plain data ARK only); permissions and creation and deletion metadata stay out
- [x] Add a **Data ARK** term to `areas/access/cpe/CONTEXT.md`: the resource's plain data ARK, resolving to the DSP data, never to CPE; distinct from the project or record **Pid** of `areas/access/CONTEXT.md` (`RecordPid`); _Avoid_: presentation ARK, version ARK, Pid
- [x] Update `areas/access/sync/CONTEXT.md` where it lists what `sync` serves or inherits from the port: the data ARK, derived from the IRI, not read from the file
- [x] Append a sentence on the data ARK to the **Archive-shaped fact** paragraph of `areas/access/cpe/CONTEXT.md` (`:19-20`)
- [x] Grep `areas/access/`, `docs/src/`, the root `CONTEXT.md` and `ARCH-MAP.md` for wording that says ARKs are not served, and fix each hit (`docs/src/dpe/operations.md`'s host rewrite concerns DPE's project ARKs, not data ARKs, and stays)
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry with `dune:dune-map`: key entities gain `DataArk`, `DATA_ARK_PREFIX` and `MalformedDataArk`
- [x] Refresh `ARCH-MAP.md`'s `areas/access/sync` entry with `dune:dune-map`: `ArkError`, `NoDataArk`, and that the ARK is derived from the IRI, not read
- [x] Run `just check && just test`; it passes, `test_committed_0803_snapshot_passes_contract` (`committed_0803.rs:80`) unchanged among them
- [x] Amend Phase 2's changes into the `feat(cpe-ports,sync-store)` commit, which is `HEAD`
- [x] Run `just commit-lint`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Incubator vendors `cpe-ports` with annotations and data ARKs

**Gate: H1** — resolve before starting this phase.

### dsp-incubator
- [x] Create a branch off up-to-date `origin/main`
- [x] Find the merged DEV-7487 commit on dsp-repository's `origin/main` (`git log origin/main --grep DEV-7487 -1` in the dsp-repository checkout)
- [x] Run `just cpe vendor <sha>` with that commit; `cpe/vendor/PIN` holds that sha
- [x] Grep `cpe/` outside `cpe/vendor/` for `cpe_ports`; record in the commit body whether any use exists
- [x] Add `Geometry` and `Color` arms to every `ValueKind` match that grep found (none found)
- [x] Add `annotation` and `ark` to every `cpe_ports::Resource` literal that grep found (none found)
- [x] Run `just cpe test`; it passes
- [x] Run `git status --porcelain --ignored cpe/vendor`; it lists no ignored file
- [x] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<sha>)`
- [x] Run `just cpe ci`; it passes
- [x] Commit `PIN`, the vendored tree and any `Cargo.lock` change together as `chore(cpe): vendor cpe-ports with annotations and data ARKs (DEV-7486, DEV-7487)`
      (landed as `chore(cpe): vendor cpe-ports with data ARKs (DEV-7487)`, dsp-incubator#486: #479 had already
      vendored the annotations)
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts
      (skipped: `vendor-diff` is empty, so the diff is byte-identical to the code reviewed in dsp-repository#465)

## Human Actions

| ID | Action | Who | When | Why an agent cannot |
|----|--------|-----|------|---------------------|
| H1 | Merge PR #464 (DEV-7486), then this plan's dsp-repository PR | Balduin | before Phase 3 (Why mid-plan: the vendor pin must be the merged `main` commit, which a rebase-merge mints only on merge) | Merging is the owner's decision |
| H2 | Merge the dsp-incubator re-vendor PR | Balduin | after ship | Merging is the owner's decision |

## Acceptance Criteria

**Serving 0803**
- [x] Every one of the 4,198 resources `LiveArchiveProjection` serves for 0803 carries a plain data ARK.
- [x] Each of the 4,143 ARKs in `0803-arks.txt` equals the served ARK of its resource.
- [x] `contract::violations` is empty for that snapshot.

**Strictness**
- [x] A served resource whose IRI yields no data ARK makes the file `Unavailable` with `NoDataArk`.
- [x] A snapshot with an ARK that is not a plain data ARK is a `MalformedDataArk` violation.

**Documentation**
- [x] `areas/access/cpe/CONTEXT.md` lists the data ARK as served, and permissions and creation and
      deletion metadata as still out.

**Incubator**
- [x] The incubator vendors the merged DEV-7487 commit; `just cpe vendor-diff` is empty and `just cpe ci`
      passes.

## Dependencies & Risks

- **PR #464 (DEV-7486)** is the base of the stack. A review change there means `gh stack rebase` here; a
  change to `Resource`'s fields there means a conflict at the struct literals listed above.
- **DAO will carry the ARK.** When it does, `sync-store` reads it instead of deriving it, and `ark.rs`
  goes. The field doc marks the derivation provisional.
- **An ARK outlives its IRI.** `areas/archive/CONTEXT.md` (**ARK**): "the binding may change across
  migrations, the ARK string never does". Deriving the ARK from the IRI is right only while no IRI has been
  re-minted, which holds for DSP today and for 0803 (the oracle confirms it). This is why the derivation is
  provisional: once IRIs can change, the ARK has to come from the archive.
- **dsp-api's algorithm could change.** It has not since ARK v1. The eleven production vectors pin it to
  what `knora-api:arkUrl` returned; a change would show there first.
- **A new check can take a project offline** (ARCH-MAP note on `sync-store`). `NoDataArk` and
  `MalformedDataArk` do not fire on 0803 (planning check above). A project whose IRIs are not
  `http://rdfh.ch/<shortcode>/<id>` would go offline, which is wanted: it has no data ARK to link to.
- **The oracle is a frozen copy.** If the incubator recomputes its ARKs, `0803-arks.txt` does not follow.
  That is intended: it is `dao-lift`'s output at a pinned commit, the same way `0803.nq` is.
- **Not in scope:** value and version ARKs; presentation ARKs (ADR-0007's open question); permissions and
  creation and deletion metadata; the incubator engine's boot check, which stays as it is.

## Success Metrics

| 0803 | Before | After |
|------|--------|-------|
| Resources with an ARK | 0 | 4,198 |
| ARKs equal to `dao-lift`'s | — | 4,143 of 4,143 |
| `contract::violations` | empty | empty |

The incubator's `cpe/vendor/PIN` names a commit that contains this change and DEV-7486's.

## References

**Tickets**
- Linear DEV-7487 (blocks DEV-7402); DEV-7486 (PR #464, the stack's base); DEV-7462 (vendoring).
- The DEV-7402 pre-planning inventory (Linear document, 2026-10-08).

**Decisions**
- ADR-0003 (consumer-defined ports); ADR-0007 with its 2026-10-02 amendment (data ARKs); ADR-0008.
- `areas/archive/CONTEXT.md` (ARK, Internal IRI).

**Earlier plans**
- `docs/specs/2026-10-08-cpe-port-annotations/01-feat-cpe-port-annotations-plan.md` (DEV-7486)
- `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`

**Sources of the algorithm**
- dsp-api: `StringFormatter.scala` (`resourceIriToArkUrl`, `makeArkUrl`), `Base64UrlCheckDigit.scala`.
- dsp-incubator: `cpe/tools/dao-lift/src/ark.rs`; `cpe/projects/incunabula/data.sql` at `c2b3e10a`.

**Code**
- `cpe-ports`: `areas/access/cpe/ports/src/{snapshot,contract,fake,lib}.rs`
- `sync-store`: `areas/access/sync/store/src/{lib,error,ark,ark_tests}.rs`,
  `src/mapping/resources.rs`, `src/{mapping_tests,snapshot_tests}.rs`, `tests/committed_0803.rs`
- Data: `areas/access/sync/data/{0803.nq,0803-arks.txt,PROVENANCE}`
