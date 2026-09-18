# Execution journal — FAIR metadata exposure

Companion to `01-feat-fair-metadata-exposure-plan.md`. One row per chunk, in
execution order. `commit` is the SHA the chunk landed as, or `—` when the chunk
produced no commit (verification-only). This file and the plan stay uncommitted
until the session ships the PR.

Base commit: `f1532fac`. Branch: `worktree-fair-assessment`.

## Round 1 — Phase 0 (2026-09-17)

| # | Chunk | Plan checkboxes | Commit | Notes |
|---|-------|-----------------|--------|-------|
| 0.1 | Move `modules/platform` to `shared/`, rename the crates | 1, 2 | `909aedf8` | `chore(shared-metadata,shared-telemetry)`; landed as `c64c7a08`, squashed into `909aedf8` in Round 2 |
| 0.2 | Paths gate, justfile, workflows, bacon watch lists | 3, 4 | `909aedf8` | `chore(ci)`; landed as `89897d81`, squashed into `909aedf8` in Round 2 |
| 0.3 | README, agent-context layer, conventions, ADRs, developer docs | 5–10 | `f08d3cfb` | `docs(docs)`; landed as `eafc7a37`, replayed as `ced50fd0` in Round 2, amended to `f08d3cfb` in Round 3 |
| 0.4 | Gate: grep acceptance, `just check`, `just test`, gate self-test | 11, 12 | — | all green |

Phase 0 has **thirteen** checkboxes, not twelve. Checkboxes 1–12 are ticked;
checkbox 13 (`eng:reviewing` with the complete reviewer set) is left unticked,
because the interactive session owns reviews.

A wording fix in `bacon.toml` ("the shared `shared-*` crates" → "the `shared-*`
crates") was folded into 0.2 with `git commit --fixup` plus an autosquash
rebase, so `75ac3ec2` and `da25abc0` were rewritten as `89897d81` and
`eafc7a37`. `just check` and `just test` were re-run after the rebase.

## Round 2 — Phase 0 review fix (2026-09-17)

No source edits. `c64c7a08` was a red commit: the crate directories had already
moved to `shared/`, but `justfile` still called `check-platform-paths.sh`, whose
pathspec was still `modules/platform/*/src/*.rs`. That matched zero files, and
the script treats absence as an error, so `just check` failed there. The six
workflow path filters were still `modules/platform/**` at that commit too, so CI
would have under-triggered. This breaks the plan's acceptance criterion that
`just check` and `just test` pass on every commit, which outranks the phase
text's suggestion that `chore(ci)` could stand alone — any split leaves a red
commit, the same argument the plan already used for merging the directory move
with the import rename.

`c64c7a08` and `89897d81` were squashed into `909aedf8` with a merged commit
message; `eafc7a37` was replayed unchanged on top as `ced50fd0`. The tree at
`HEAD` is byte-identical to the tree at the old `eafc7a37`
(`git diff eafc7a37 HEAD` is empty). All three old commits were local only;
`origin/worktree-fair-assessment` is still at `f1532fac`, so nothing published
was rewritten.

Verified with
`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' f1532fac`.
All six steps green — both recipes pass at both commits, not only at `HEAD`.

`just commit-lint` message rules still pass. The one-commit-per-PR cap now
counts five commits over `origin/main` rather than six: two for this phase
instead of three. Still a ship-time decision, not this round's.

## Round 3 — dune review fixes (2026-09-17)

The nine-reviewer set reported in full; the dune reviewer returned
approve-with-findings. Three findings fixed, two deliberately deferred.

**DUNE-004 (medium) — the plan file described pre-Phase-0 state as current.**
Five passages in `01-feat-fair-metadata-exposure-plan.md` put in the past
tense, nothing else touched (the file is `reviewed(2)`; no argument,
structure, checkbox or acceptance criterion was changed):

| Line | Was | Now |
|------|-----|-----|
| 72 | "today the tree still says `modules/platform/`…" | "before Phase 0 the tree said …" |
| 195-196 | "`check-shared-paths.sh`, today `check-platform-paths.sh`, whose pathspec Phase 0 points at" | "…called `check-platform-paths.sh` before Phase 0, whose pathspec Phase 0 pointed at" |
| 626 | *Line today* (table heading) | *Line before Phase 0* |
| 954 | "which Phase 0 moves to" | "which Phase 0 moved to"; the test reference reworded to "then `…check-platform-paths.test.sh:43`" |
| 955 | "are today's `modules/platform/…` until Phase 0 lands" | "were `modules/platform/…` until Phase 0 landed in `909aedf8`"; the line-number clause now reads "as they were when the plan was written" |

The other thirteen occurrences of "today" (lines 84, 100, 252, 259, 262, 267,
285, 287, 300, 562, 755, 769, 950) describe DPE code Phase 0 did not touch and
are still true. Left alone. No row in the line-626 table names Phase 0 in its
*Changes in* column, so the heading rename needed no follow-on edit.

**DUNE-002 (low) — broken ADR cross-reference in root `CONTEXT.md`.** Verified
directly: `docs/adr/0002-areas-at-the-repository-root.md` has three headings,
`## Considered Options` (line 34), `## Consequences` (line 50) and the title;
*Alternatives considered* does not exist, so the citation at `CONTEXT.md:93`
failed a literal lookup. Corrected to *Considered Options*. The same entry now
names `modules/platform/` once, the second half of the finding: ADR-0002's
Considered Options discusses only a hypothetical root-level `platform/`, so the
path that actually existed for months was named in no live document. The
repository holds exactly one live instance of the bad citation (checked across
`CONTEXT.md`, `ARCH-MAP.md` and `docs/`; the only other hit is this journal
quoting the finding).

**DUNE-003 (low) — overclaim in `ARCH-MAP.md`.** Line 21 said "the four planned
components hold only a `CONTEXT.md` at their target path". Verified against the
index: `areas/archive`, `vitrinli` and `chischtli` each hold exactly one
`CONTEXT.md`; `git ls-files -- areas/access/cpe` returns zero files, which its
own entry (line ~374) already recorded as "(no files yet)". Corrected to "three
of the four planned components … (`areas/access/cpe` has no files yet)". The
empty component is named rather than left implicit, so the sentence stays true
when a fourth `CONTEXT.md` lands.

**Deliberately not actioned.**

- **DUNE-001** — `ARCH-MAP.md`'s `last_verified_commit: 8c9bde61`. Bumping it
  would claim a whole-map re-verification that did not happen; only the touched
  component entries were edited. The honest value is the stale one. Closes at
  ship time with a full `/dune:dune-map update` pass.
- **DUNE-005** — `.gitattributes` missing from ARCH-MAP's *Cross-cutting
  concerns*. Pre-existing, introduced in `5e8d248f`, which is below this
  branch's base `f1532fac` and already pushed. Not worth rewriting published
  history. Recorded as a PR follow-up.

**Deviation — checkbox 11's acceptance grep now has one hit.** Checkbox 11 asks
that a case-insensitive grep for `platform_metadata`, `platform-metadata`,
`platform-telemetry`, `modules/platform`, `PLATFORM_PATHSPECS` and
`non_platform` outside `CHANGELOG.md` and `.git/` come back empty. Naming
`modules/platform/` in `CONTEXT.md:93`, as DUNE-002 asks, trips it: the grep now
returns exactly that line and nothing else. The two readings conflict directly
and no wording satisfies both — evading the pattern with separators would game
the check rather than meet it.

Kept, on the same reasoning Round 1 used to exclude `docs/specs/`: the grep's
purpose is that no **live** reference to the old name survives, and a
past-tense entry in a *Flagged ambiguities* glossary, whose whole job is to
explain a name that no longer exists, is not one. The checkbox is left ticked
per the round's instruction not to touch checkboxes; this entry carries the
truth. **This is the session's call to confirm or reverse.** Reverting is
small: drop `, `modules/platform/`,` from `CONTEXT.md:93`, amend `f08d3cfb`,
re-run the gate.

**Commits.** Fixes 2 and 3 edit `CONTEXT.md` and `ARCH-MAP.md`, both introduced
in the docs commit, so both were staged by explicit path and folded in with
`git commit --amend --no-edit` rather than a `fix:` commit on top:
`ced50fd0` → `f08d3cfb`, message unchanged. Fix 1 edits the plan file, which
stays uncommitted until the session ships. Phase 0 is still two commits:

| | |
|---|---|
| `909aedf8` | `chore(shared-metadata,shared-telemetry): move the shared root to shared/` |
| `f08d3cfb` | `docs(docs): say shared/ and shared-* everywhere the rename touched` |

**Gate.** `git rebase --exec 'env -u GIT_DIR just check' f1532fac` — green at
both commits. `env -u GIT_DIR` is load-bearing; see the side finding on
`check-commit-count.test.sh`. The rebase preserved both SHAs this time, so
`f08d3cfb` is the amend SHA, not a replay. The two spec files were copied to the
scratchpad before the gate and restored after; `git status` afterwards shows
exactly them and nothing else. All commits remain local —
`origin/worktree-fair-assessment` is still at `f1532fac`.

## Round 4 — Phase 1 (2026-09-17)

**Round-3 escalation resolved by the user: the `CONTEXT.md:93` mention stays.**
Phase 0's checkbox 11 and the matching acceptance criterion now carve the root
`CONTEXT.md`'s *Flagged ambiguities* entry out of the grep, beside `CHANGELOG.md`
and `.git/`, with the reason written into both lines: the grep's purpose is that
no *live pointer* to the old path survives, and a glossary entry explaining a name
that no longer exists is not one. Checkbox 11 is therefore honestly ticked. Plan
file edited in the working tree only, never staged.

**Hash baseline is not committed.** The user decided against the plan's committed
fixture: ~51,000 records x 2 formats is several MB of hex that would stay in
`main`'s history permanently even after the phase deletes it, and the intermediate
checkpoint commits need the file for `just test` to pass, so it could not be
squashed out cleanly. The baseline lives at
`.claude/tmp/oai-baseline-hashes.txt` — gitignored, on disk, surviving
orchestrator respawns. The hash test skips with a clear message when the file is
absent, so CI (which has no `.claude/tmp/`) stays green on the intermediate
commits. Consequently the phase's first checkbox reads "capture the baseline to
`.claude/tmp/`", not "commit the fixture", and the later checkbox deletes the
hash **test** only.

| # | Chunk | Plan checkboxes | Commit | Notes |
|---|-------|-----------------|--------|-------|
| 1.1 | OAI output hash baseline test | 1 | `588ee18d` | `test(dpe-api-oai)`; 102,158 entries, 8.6 MB, 7.2 s |
| 1.2 | `shared/fair` crate skeleton, workspace member, gate fixture | 2 | `ca1a285d` | `chore(shared-fair)`; dependencies deferred to the move chunk, see below |
| 1.3 | `ContributorLookup`, `is_organization_id`, `parse_url_value` to `shared-metadata` | 3, 4 | `af849e5d` | `refactor(shared-metadata,dpe-core)`; hash checkpoint green |
| 1.4 | Raw project accessors on the repository; OAI test double over `ProjectRaw` | 5 | `5ad01192` | `refactor(dpe-core,dpe-api-oai)`; no call site and no golden file changed |
| 1.5 | Move the four mappings, helpers, resolver and record types to `shared/fair`; `ResolveContext` | 6 | `19d23aca` | `refactor(shared-fair,dpe-api-oai)`; **hash checkpoint 1: `compared 102158 entries`** |
| 1.6 | `RecordGraph`, `RecordCreator`, `AgentKind`, `PartRef` and their tests | 7 | `52214684` | `feat(shared-fair)`; nothing consumes them yet |
| 1.7 | `dpe_core::resolve_inputs()`; OAI call site takes its tables | 9 | `221754dc` | `refactor(dpe-core)`; done before checkbox 8, which it does not depend on |
| 1.8 | `ProjectGraph` and its supporting refs | 8 | `0448ec5a` | `feat(shared-fair)`; lives in a new `project_graph.rs`, not `graph.rs`. Round 5 folded fix 3 in and rewrote it as `35d57a0f` |
| 1.9 | Re-plumb the two project writers onto `ProjectGraph` | 11 | `bedd3889` | `refactor(shared-fair)`; hash green, `raw_name` added to the graph. Round 5 folded fix 5 in and rewrote it as `174fd463` |
| 1.10 | Re-plumb the two record writers onto `RecordGraph` | 12 | `764a1ba4` | `refactor(shared-fair)`; **hash checkpoint 2: `compared 102158 entries`**. Round 5 folded fix 4 in and rewrote it as `7a2c243b` |
| 1.11 | Unit-test top-up: multilingual rule, `url` reading rule | 14 | `df0b4aec` | `test(shared-fair)`; the graph chunks already covered the other seven. Replayed in Round 5 as `4cfc0c94` |
| 1.12 | ARCH-MAP, CONTEXT, shared/README, developer docs, scope tables | 15–19 | `8969ebe6` | `docs(docs)`; the cross-check also caught `REVIEW.md` and `docs/src/git-conventions.md`. Round 5 folded fixes 7–10 in and rewrote it as `235206cd` |
| 1.13 | Delete the hash test | 13 | `13b31939` | `test(dpe-api-oai)`; ran last, not at its position in the list. Replayed in Round 5 as `4cdf06d8` |
| 1.14 | Creator-fallback accessor, `extract_year` char guard, corpus no-panic guard | — | `cb9ba381`, `b5ef8fab`, `97abd613` | added in Round 5; see that section |
| 1.14 | Gate: rebase-verify `just check` and `just test` at all thirteen commits | 20 | — | 26 exec steps green; all SHAs preserved |

### Round-4 findings

- **Baseline path: `.claude/tmp/oai-baseline-hashes.txt`** (gitignored via
  `.gitignore:32`). A fresh orchestrator re-creates it with
  `OAI_HASH_BASELINE_WRITE=1 cargo test -p dpe-api-oai --lib -- --ignored oai_output_matches_hash_baseline`
  — but only from a commit whose mappings still produce the pre-refactor output,
  i.e. `588ee18d`. Do not regenerate it mid-phase; that would erase the very
  thing it pins.

- **The hash test is `#[ignore]`d, and must be.** It calls
  `dpe_core::set_data_dir`, a first-call-wins `OnceLock`. The handler tests in
  the same crate reach `get_data_dir()` through
  `project_to_datacite` → `resolve_temporal_coverage` →
  `chronontology_cache::all_periods()` on their first DataCite render, which
  initialises it to the relative default `modules/dpe/server/data`. In a shared
  test process whichever runs first wins, so the test would hash against empty
  temporal tables roughly half the time. `#[ignore]` gives it a process of its
  own. Run it with
  `cargo test -p dpe-api-oai --lib -- --ignored --nocapture oai_output_matches_hash_baseline`
  and **confirm the `compared N entries` line**; a skip is not a passed
  checkpoint.

- **Lines, not a map.** The five `0801*` project files share one PID and so one
  OAI identifier. Keying the baseline on `identifier + prefix` would have
  silently dropped four of them.

- **`just commit-lint` does not validate the scope vocabulary.**
  `.commitlintrc.yml` enforces the eight types and `scope-empty` only, and says
  so in a comment: "There is deliberately NO `scope` allowlist". So
  `shared-fair`-scoped commits pass from the start, and the `CONVENTIONS.md`
  table row can land in the phase's documentation chunk where the plan puts it.

- **The skeleton crate ships without dependencies.** The plan's checkbox has
  `shared-fair`'s `Cargo.toml` declare `shared-metadata`, `serde` and
  `serde_json` at creation, several commits before any code uses them.
  `cargo-machete`, which `just check` runs, fails that manifest, and the plan's
  own acceptance criterion is that every commit is green. The dependencies land
  with the move chunk instead; the manifest carries a comment saying which are
  coming and that `serde_json` must be the workspace one, for `preserve_order`.

- **`is_organization_id` moved with the trait**, although no checkbox names it.
  `resolve.rs` calls it and is one of the six files the move chunk relocates to
  `shared/fair`, which cannot depend on `dpe-core`. It is a rule about the
  contract's own ID strings, so it belongs beside the trait. Its one other
  caller, `dpe-web/src/domain/contributors.rs`, was re-pointed.

- **No re-export shim was left in `dpe-core`.** The plan says to update every
  `use dpe_core::ContributorLookup` to the `shared_metadata` path; a re-export
  would have made that optional and given the trait two apparent homes.

- **`dpe-server` did not get the `shared-fair` dependency in the move chunk.**
  The plan's checkbox adds it to both `dpe-api-oai` and `dpe-server`, but nothing
  in `dpe-server` uses the crate until Phase 2, and `cargo-machete` — inside
  `just check` — fails a manifest listing an unused dependency. It arrives with
  `modules/dpe/server/src/metadata.rs`. Same reasoning as the skeleton manifest
  above; the `serde`/`serde_json` the plan wanted on `shared-fair` from the start
  are not there yet either, because the moved code uses neither. They arrive with
  `schema_org.rs`.

- **`to_oai_record` builds the `ResolveContext`; no handler signature changed.**
  Threading a context parameter through the handlers would have touched about
  fifty test call sites for no gain, and would have risked the golden XML: the
  handler tests run with the process-global data dir at its relative default, so
  the temporal tables are empty there, and reading the same globals through the
  context reproduces that exactly. Checkbox 9 replaces the three-line
  construction with `dpe_core::resolve_inputs()`.

- **Only the three handlers that feed a mapping switched to the raw accessors.**
  `get_record.rs` (`OaiEntity::Project(Box<ProjectRaw>)`) and the two collectors
  in `handlers/mod.rs`. `identify.rs`, `list_sets.rs` and
  `list_metadata_formats.rs` never touch a mapping and stay on the view model.

- **All six moved files were recorded as renames** (78% similarity on
  `datacite.rs`, higher on the rest), so history follows them across the crate
  boundary.

- **`RecordGraph` does not apply the `DaSCH` creator fallback, although the plan
  says it does.** DataCite requires at least one creator and its writer appends
  an organizational `DaSCH` when `authorship` is empty; Dublin Core neither
  requires nor emits one. Putting the fallback in the graph would have added a
  creator to `oai_dc` for every record with empty authorship — a real output
  change, which this phase forbids. The fallback stays in the DataCite writer and
  a unit test guards the graph against acquiring it.

- **`RecordGraph` carries `mime_type`, against the plan's "no file pointer"
  line.** The plan cites `record_dublin_core.rs`'s rule that OAI Dublin Core
  publishes the ARK only; that rule is about `dc:identifier`, not `dc:format`.
  Both record writers emit `formats` from `record.file.mime_type` today
  (DataCite property 14 and `dc:format`), so a graph without it could not
  reproduce the committed output. The download URL, checksum, file name and file
  size are not carried, and a test pins that.

- **`publisher` is carried but deliberately unread.** `Record.publisher` exists;
  both writers emit the constant `"DaSCH"`. The graph records the fact and its
  doc comment says the writers ignore it, so a later reader does not "fix" it
  into the output.

- **`alternative_titles` keeps a duplication that looks like a bug.** When a
  record has no `en` label, `multilingual_value` returns the lexicographically
  smallest tag's text as the title, and `record_to_datacite` then re-emits that
  same entry as an `AlternativeTitle`. It is in the published output, so the
  graph reproduces it and a comment says why.

- **The two `typeOfData` matches were byte-identical** (`type_of_data_to_general`
  in `record_datacite.rs`, `type_of_data_to_dc_type` in `record_dublin_core.rs`),
  so one `general_data_type` in the graph serves both. The writer-side copies stay
  until the re-plumbing chunk deletes them.

- **The OAI call site discards `resolve_inputs()`'s lookup.** It takes only the
  two temporal tables and keeps the `lookup` parameter the handlers already
  thread through, because that parameter is how roughly fifty handler tests
  inject an in-memory double. `dpe-server` will use all three in Phase 2. A
  comment at the call site says so, since dropping one element of a tuple
  otherwise reads like an oversight.

- **`ProjectGraph` lives in `shared/fair/src/project_graph.rs`, not `graph.rs`.**
  The plan puts both graphs in one file; with the record graph already there it
  would have passed 1,200 lines. `ResolveContext`, `AgentKind`, `RecordGraph` and
  `PartRef` stay in `graph.rs`; `lib.rs` re-exports both.

- **Six fields are carried raw because the two project writers disagree about
  them**: `name`/`official_name` (DataCite takes the longer, Dublin Core prefers
  the official one), `description`/`abstract` (opposite order, different dedup),
  `legal_info` (DataCite emits an entry even for an all-placeholder element),
  temporal coverage (name for Dublin Core, resolved date for DataCite), each
  agent's raw `contributorType`, and the whole `data_language` list. A graph that
  resolved any of these to one answer could not reproduce both writers' output.

- **`DataCiteNameIdentifier` gained `Clone` and `PartialEq`.** The graph carries a
  resolved agent's ORCID and GND identifiers, and the unit tests compare them.
  It derived only `Debug, Default` before.

- **`ProjectGraph` gained `raw_name` during the re-plumbing.** Both writers fall
  back to the raw `name` when neither title is real, and emit the placeholder
  string itself. `name: Option<String>` had resolved that away. The fix was to
  add the fact, not to let a writer reach past the graph — the plan's own rule
  when a re-plumbing would move the output. The field was folded into the
  re-plumbing commit rather than amended into `0448ec5a`, because the two
  commits are adjacent and the reason for the field is only legible beside its
  use.

- **Checkbox 14's unit tests were largely written by the graph chunks.** Seven of
  the nine assertions it names landed with `RecordGraph` and `ProjectGraph`,
  because a builder without them was not worth committing. Only the
  multilingual-preference rule and the `url` reading rule were missing, so this
  chunk is a top-up rather than the whole test suite the checkbox describes.
  `shared-fair` gained `serde_json` as a **dev**-dependency for the `url` test.

- **Checkbox 13 (delete the hash test) is deliberately deferred to the end of the
  phase's code work**, not run at its position in the list. Deleting it costs
  nothing later and keeps the byte-identity net under the remaining chunks. If a
  reviewer finding forces a code change after it is deleted, restore
  `metadata/corpus.rs` from `764a1ba4` — the baseline file at
  `.claude/tmp/oai-baseline-hashes.txt` is untouched and still valid.

- **Two documents beyond the plan's list were stale.** `REVIEW.md`'s shared-crate
  checklist items named two crates and its layering line named only the contract
  and the view model; `docs/src/git-conventions.md` carries a second copy of the
  commit-scope list that `CONVENTIONS.md` also holds. Both were fixed in the
  documentation commit.

- **ARCH-MAP's *Conventions* rule needed an exception.** It said shared code
  lands under `shared/` "the moment a second service depends on it".
  `shared-fair` is there with one consumer, deliberately, because ADR-0005 names
  the consumers to come. The rule now records the exception rather than being
  quietly false.

- **`last_verified_commit` in `ARCH-MAP.md` is still `8c9bde61`.** Only the
  touched entries were re-verified, so bumping it would overstate the check, the
  same call Round 3 made.

- **The FNV-1a hash is hand-written** rather than `DefaultHasher`, whose output
  is not stable across Rust versions. The baseline has to survive a toolchain
  bump mid-phase.

**Phase 1 gate.**
`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' f08d3cfb` —
twenty-six exec steps, all green, and the rebase preserved every SHA
(`588ee18d` … `13b31939`), so nothing was replayed. `env -u GIT_DIR` held:
afterwards `git config --local --get-regexp '^user\.'` is empty and there is no
stray `feature` branch, the two symptoms Round 2 had to clean up. The two spec
files were copied to the scratchpad before the gate and restored after;
`git status --porcelain` afterwards shows exactly them.

Phase 1 is **thirteen commits**, all local — `origin/worktree-fair-assessment` is
still at `f1532fac`. Checkbox 21 (`eng:reviewing` with the complete reviewer set)
is the session's and is left unticked.

**If a review finding needs the byte-identity check again**, take the deleted
test out of `7a2c243b` (`764a1ba4` in Round 4, rewritten by Round 5's rebase;
the file is identical there, as fix 4 touched only `helpers.rs`):

```
git show 7a2c243b:modules/dpe/api-oai/src/metadata/corpus.rs > <scratch>/hash-corpus.rs
```

Do **not** check it back out over `corpus.rs` — since Round 5 that file holds a
live guard. Copy it in beside as `metadata/hash_check.rs`, trim the duplicate
`every_committed_temporal_coverage_resolves` off the end, add `#[cfg(test)] mod
hash_check;` to `metadata/mod.rs`, then run
`cargo test -p dpe-api-oai --lib oai_output_matches_hash_baseline -- --ignored --nocapture`
and remove both again. That is exactly the dance Round 5 used.
The baseline at `.claude/tmp/oai-baseline-hashes.txt` is untouched and still
valid; do **not** regenerate it. A skip is not a pass — the run must print
`compared 102158 entries`.


## Round 5 — Phase 1 review fixes (2026-09-18)

Thirteen fixes from ten reviewers. Devops, maud, performance and patterns found
nothing blocking. The brief attributed three fixes to named reviewers; the rest
it collected without naming who raised which, and they are recorded that way
rather than guessed at.

Nothing here changes OAI output. That was re-proven, not assumed — see
**Byte identity**.

| Fix | Raised by | Landed in |
|-----|-----------|-----------|
| 1 — resolve the creator fallback once, on the graph | ivan, dune, consistency (independently) | `b5ef8fab`, own commit |
| 2 — `extract_year` panicked on non-ASCII input | security, rust (independently, both with a repro) | `cb9ba381`, own commit |
| 3 — remove the speculative `ProjectGraph.data_type` | brief, unattributed | folded into `35d57a0f` (was `0448ec5a`) |
| 4 — delete the dead `get_multilingual_value` passthrough | brief, unattributed | folded into `7a2c243b` (was `764a1ba4`) |
| 5 — rename the `record` accumulator in the two project writers | brief, unattributed | folded into `174fd463` (was `bedd3889`) |
| 6 — corpus-wide no-panic guard | rust; converges with dune's DUNE-003 | `97abd613`, own commit |
| 7, 8, 10 — documentation | brief, unattributed | folded into `235206cd` (was `8969ebe6`) |
| 9 — corpus-test wording | brief, unattributed | **no edit needed**; see below |
| 11–13 — the plan file | brief, unattributed | working tree only, never committed |

### Fix 1 — the headline

ADR-0005 counts creator fallback among the facts the graph resolves once. The
code had `if creators.is_empty() { push an organizational DaSCH }` written out
verbatim in `datacite.rs` and `record_datacite.rs`, and both graphs deliberately
did not apply it.

The reasoning behind that omission was right and is preserved. Resolving the
fallback into `graph.creators` would put a fabricated creator into the `oai_dc`
of every object with no attributed authorship: it breaks byte-identity and it
invents data (ADR-0005, *Nothing is invented for a score*). Phase 2's JSON-LD
needs the fallback too, so leaving it inline would have produced a third
hand-written copy.

Both graphs now carry `creators_with_fallback()` holding the single
implementation, and the two DataCite writers call it. `graph.creators` is
untouched, and Dublin Core still reads it, so `oai_dc` does not move. Each
accessor's doc comment says why Dublin Core does not call it — beside the code
the next representation will. It returns `Cow<'_, [T]>`: the fallback allocates,
the common case borrows, and the record writer runs over every record of a dump.

**ADR-0005 was not amended.** dune's DUNE-001 recommended amending the decision
bullet to match the code. Declined in favour of changing the code: the ADR's
text is the intent, the accessor makes it true as written, and amending it would
have recorded the duplication as the decision.

### Fix 2 — the only behaviour change in the round

`extract_year` guarded a byte slice with a byte length, so `extract_year("123ä5")`
panicked on a char boundary. Nothing could reach it: every committed date is
ASCII ISO-8601, which is why the hash baseline never caught it. It becomes
reachable in Phase 2, where these graphs back an HTTP response, and again in the
Deposit Area, where the same function runs over a date a depositor is typing.

The guard now counts characters, and that is the whole care of the fix. The
obvious `chars().take(4)` on its own *widens* the guard, letting a
three-character string through to a truncated "year" where the byte length sent
it to the 2015 fallback. The test pins both ends: `"123ä5"` no longer panics, and
`"äää"` (six bytes, three characters) still falls back.

It is its own commit rather than folded into the move commit: it is a
pre-existing bug and the only fix here that changes behaviour, and folding it
into a commit whose whole point is "no behaviour change" would muddy that story.

### Fix 6 — and it runs under `just test`

The hash test it replaces had to be `#[ignore]`d, because it set the
process-global data dir and so could not share a process with the handler tests.
The new guard sidesteps that: it loads the two temporal tables through
`shared_metadata::*::load_from` and reads the persons and organizations off disk
into its own `ContributorLookup`, touching no global. It builds a `ProjectGraph`
for all 85 committed projects and a `RecordGraph` for the first and last hundred
records of each dump, runs all four writers, and asserts only that nothing
panics — no byte assertions, no agreement assertions. Phase 2 grows it into the
agreement test. Verified present in `just test` output as
`metadata::corpus::every_committed_project_and_a_record_sample_survive_all_four_writers`.

### Fix 9 — verified, no edit

`ARCH-MAP.md`'s `modules/dpe` *Used by* bullet, its `shared/fair` local-context
kit and boundary rule, and `shared/README.md` all say `corpus.rs` holds
`shared-fair`'s committed-data tests. Fix 6 makes that true as written, so
nothing was reworded.

One-commit window, deliberately left: at `4cdf06d8` (the hash-test removal) those
sentences are momentarily false, because Fix 6 lands at `97abd613` one commit
later. Moving Fix 6 earlier would conflict on `sorted_json_files` and `DATA_DIR`
with the very commit that deleted them, and the window is one commit wide.

### Deviations from the brief's placement

Two folds landed in a different commit than the brief named. Both were checked
against the history first; both still satisfy "fold into the commit that
introduced it, never a `fix:` stacked on top".

- **Fix 4 → `7a2c243b` (`764a1ba4`), not the move commit `19d23aca`.**
  `git log -S get_multilingual_value` puts the last call site's disappearance at
  the record-writer re-plumbing, not at the move: at `bedd3889` both record
  writers still called it. Deleting the function at the move commit would have
  meant rewriting its callers there and conflicting through every commit that
  touched them.
- **Fix 5 → `174fd463` (`bedd3889`), not the move commit `19d23aca`.**
  `bedd3889` rewrote both project-writer bodies onto `ProjectGraph`, touching
  nearly every line holding the `record` accumulator; renaming at the move
  commit would have conflicted on almost every hunk of that replay. It is also
  where the "Record" ambiguity became acute — the function now has a graph *and*
  an output record in scope.

### Commit order

Fixes 1 and 2 were inserted between `4cfc0c94` and the docs commit rather than
appended at the tip. Fix 7 rewords `ARCH-MAP.md` and `shared/README.md` to
describe the accessor; had the accessor landed after the docs commit, that
commit would have described code that did not exist yet, and this repository
rebase-merges, so every intermediate commit lands on `main` verbatim. The
insertion used `git rebase --onto` on a detached HEAD, not an interactive
reorder.

### Byte identity

Re-proven on the finished tree. The hash test was copied out of `764a1ba4`
before the rebase, restored afterwards as `metadata/hash_check.rs` beside the new
corpus test, and run:

```
compared 102158 entries
took 7.0s
test metadata::hash_check::oai_output_matches_hash_baseline ... ok
```

All 102,158 entries match the baseline at `.claude/tmp/oai-baseline-hashes.txt`,
which was **not** regenerated. The file was removed again afterwards.

One change landed after that run: a two-line comment reword inside `97abd613`'s
`#[cfg(test)]` module (`RECORD_SAMPLE` described Phase 2's agreement test in the
present tense). A comment in a test module cannot reach a writer, so the proof
stands; `just check` and `just test` were re-run on the amended tip.

### Convention adopted for the rest of the run

**When execution correctly deviates from the plan, the checkbox text is amended
to match reality at the time it is ticked.** The journal records why; the plan
must not state something false. Round 4 left three ticked checkboxes textually
wrong even though every deviation behind them was correctly reasoned and
recorded here, so a reader of the plan alone got a wrong picture. Fixes 11–13
corrected them: the property-table creator row, the unit-test checkbox claiming
`ProjectGraph::build` applies the `DaSCH` fallback when its own linked test
asserts the opposite, the `dpe-server` dependency clause (correctly deferred to
Phase 2 for `cargo-machete`), and two places naming `graph.rs` where
`ProjectGraph` actually lives in `project_graph.rs`.

### Gate

`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' f08d3cfb`
— green at every commit, 32 exec steps over 16 commits. All 16 SHAs
fast-forwarded unchanged, so the byte-identity run above was on the final trees.

One interruption, not caused by this work: `editor-server`'s
`auth::handlers::tests::test_simultaneous_wrong_guesses_cannot_outrun_the_three_strike_limit`
failed once under the load of repeated full-suite runs. It passed 5/5 in
isolation and the full suite passed at that same commit. Recorded under
*Side findings*.

`just commit-lint`: commit messages all pass. The commit-count gate reports
21 > 1 as it has all phase, which is what the PR's `allow-many-commits` box is
for.

`git status --porcelain` shows exactly the two spec files. All commits are local;
`origin/worktree-fair-assessment` is still at `f1532fac`.

## Decisions and deviations

- **The `include_str!` fixture paths are part of the depth arithmetic.**
  `modules/dpe/api-oai/src/handlers/get_record.rs:110` and `test_utils.rs:257`
  reach `shared/metadata/testdata/0803-records.json` by relative path. They sit
  inside `#[cfg(test)]`, so `cargo build` compiles neither and the tree looked
  correct while `cargo test` would have failed. `cargo check --all-targets` is
  what catches this class; a plain build is not enough for this phase.

- **The out-of-workspace fuzz crate was checked separately**, with
  `cargo check --manifest-path modules/dpe/server/fuzz/Cargo.toml`. Its
  `Cargo.lock` is tracked and regenerated in commit 0.1.

- **rustfmt reorders imports after the rename.** `platform_metadata` sorts
  before `serde_json`; `shared_metadata` sorts after it. `just fmt` was run
  inside chunk 0.1 rather than left to the gate, so the reordering lands in the
  commit that caused it.

- **`non_shared_modules` dropped its `$2 != "platform"` awk filter.** Nothing
  under `modules/` is shared any more, so the exclusion had become dead code
  that named a directory which no longer exists.

- **The gate test's fixtures now use `modules/`-rooted violation paths.** In the
  old layout a violation escaped `modules/platform/…/src` as `../../../dpe/`,
  hitting the regex's `\.\./` alternation. From `shared/<crate>/src` the same
  escape passes through `modules/`, so tests 2, 3 and 5 exercise the `modules/`
  alternation instead. The regex itself is unchanged.

- **The grep acceptance excludes `docs/specs/`, not just `CHANGELOG.md`.** The
  plan file and this journal quote every one of the six patterns in their own
  text; they cannot be made to pass without falsifying the record. Excluding
  `.git/`, `CHANGELOG.md`, `target/` and `docs/specs/`, the grep is empty.
  From Round 3 the grep has exactly one hit, `CONTEXT.md:93`, and it is
  deliberate; see Round 3.

- **The root `CONTEXT.md` "platform" ambiguity entry names no dead path.** It
  records the rename by pointing at ADR-0002's *Alternatives considered*, which
  holds the old name, rather than repeating `modules/platform/` — which would
  have kept the acceptance grep permanently red for a purely historical
  mention. **Superseded in Round 3**: DUNE-002 asked for the old path to be
  named once so a reader grepping for it lands somewhere that explains it, and
  for the citation to name ADR-0002's real heading. See Round 3.

- **ADR-0001 was updated although no checkbox names it.** Checkbox 10 covers
  ADR-0002 only, but ADR-0001 carried three live references (`platform-*`,
  `check-platform-paths.sh`, `modules/platform/`) that the acceptance grep
  would have caught.

- **`modules/editor/server/src/db/migrations/0001_initial.sql` was edited.** The
  hit is a SQL comment naming `platform_metadata::Person`. Safe: the runner in
  `db/schema.rs` guards on `PRAGMA user_version`, not a content checksum, and
  the file's own header records that the list is not append-only yet because
  the editor has never been deployed.

- **`just commit-lint` message rules pass; the one-commit-per-PR cap does not.**
  It counts three commits for this phase (plus the pre-existing ones in the
  `origin/main..HEAD` range). Squashing or ticking `allow-many-commits` is a
  ship-time decision for the session, not this round's.

- **The container builds were checked and need nothing.** Both Dockerfiles copy
  a prebuilt `*-server` binary plus `public/` and `data/` into a staging
  directory; neither runs `cargo` inside the image, and the editor's local
  build mounts the whole worktree (`-v "$PWD":/work`). Nothing enumerates
  workspace members, so `shared/` arriving at the root breaks no image. The
  root `.dockerignore` excludes nothing under `shared/`.

- **No workflow filters on a bare `modules/**`.** Every path filter names a
  specific module, so none silently stopped covering the shared crates. The
  three mosaic workflows correctly gained nothing: mosaic depends on neither
  shared crate.

- **`ARCH-MAP.md`'s `last_verified_commit` frontmatter was left at
  `8c9bde61`.** The renamed paths were updated, but the map as a whole was not
  re-verified against the tree, so bumping the field would overstate what was
  checked.

## Side findings

For the session's closeout and the PR body.

### Phase 1

- **Byte-identity evidence is the twelve hand runs, not the rebase gate.** The
  hash test is `#[ignore]`d, so `just test` skips it and the twenty-six green
  exec steps prove `check` and `test` at every commit but say nothing about the
  OAI output. What proves it is the explicit run after every code chunk —
  `cargo test -p dpe-api-oai --lib -- --ignored --nocapture oai_output_matches_hash_baseline`
  — each of which printed `compared 102158 entries`, including the two named
  checkpoints at `19d23aca` and `764a1ba4`. A skipped run was never accepted as a
  pass.

- **Two wording candidates for the reviewer pass**, both currently true but worth
  tightening, and both cheaper to fold into their own commits during the review
  than to rebase for now:
  - `ARCH-MAP.md`'s `shared/fair` boundary rule "Only `ProjectGraph::build`,
    `RecordGraph::build` and `PartRef::from_record` take a wire-contract type".
    True of the **public** surface — verified, those are the only three `pub fn`
    signatures naming `ProjectRaw` or `Record`. Private helpers inside
    `project_graph.rs` (`discipline_ref`, `license_ref`, `funding_refs`,
    `attributed_agents`) do take `&Discipline`, `&LegalInfo` and `&ProjectRaw`.
    Adding the word *public* to the rule would stop a reviewer tripping on them.
    Belongs in `8969ebe6`.
  - `shared/fair/Cargo.toml`'s header comment ends "The DataCite and Dublin Core
    writers read the wire contract and nothing else." Accurate for the
    `[dependencies]` block it heads, but the manifest now also carries a
    `serde_json` dev-dependency with its own comment below. Belongs in
    `df0b4aec`.

- **`raw_name` landed in the re-plumbing commit, not the graph commit.**
  `0448ec5a` adds `ProjectGraph`; `bedd3889`, three commits later in the same
  phase, adds its `raw_name` field. The plan's rule is that an in-phase fix folds
  into the commit that introduced the problem. It was kept separate deliberately:
  `raw_name` exists because both writers fall back to the raw `name` — placeholder
  string and all — when neither title is real, and that reason is only legible
  beside the fallback it serves. Folding it back would leave a field in the graph
  commit with no visible reason to exist. Worth stating in the PR body so a
  consistency reviewer does not have to dig for it.

- **Three deliberate omissions a reviewer may read as oversights.**
  `dpe-server` still has no `shared-fair` dependency (nothing there uses it until
  Phase 2, and `cargo-machete` fails an unused one). `shared-fair` has no `serde`
  or `serde_json` runtime dependency (the moved code uses neither; they arrive
  with `schema_org.rs`). `ARCH-MAP.md`'s `last_verified_commit` is still
  `8c9bde61` — only the touched entries were re-verified.

### Phase 0

- **The devops reviewer found the red commit; the other eight returned clean.**
  `c64c7a08` failed `just check` because the paths gate had not moved with the
  directory. Resolved by squashing it into `89897d81` — see Round 2. Notable
  confirmations from the clean reviewers: the boundary gate's regex still
  catches every escape, because from `shared/` any path into a service
  necessarily contains the literal `modules/<service>/`; the edited SQL comment
  in `0001_initial.sql` is inert, because `schema.rs` guards on
  `PRAGMA user_version` and the editor has never been deployed; and
  `git log --follow` survives the crate move.

- **`check-shared-paths.sh` appears in history as a delete plus a create**, not
  a rename: similarity against `check-platform-paths.sh` fell below 50%, so
  `git log --follow` on it stops at the rename commit. Its test file did cross
  the threshold (57%) and is recorded as a rename. Cosmetic; recorded, not
  fixed.

- **`check-commit-count.test.sh` is not hermetic against an inherited
  `GIT_DIR`, and it mutates the host repository when one is set.**
  `git rebase --exec` exports `GIT_DIR` into the command it runs (verified: a
  probe exec saw
  `GIT_DIR=…/.git/worktrees/fair-assessment`). The test's `make_repo` fixture
  does `cd "$dir"; git init …; git config user.email …; git commit …`, and with
  `GIT_DIR` set every one of those targets the real repository rather than the
  throwaway one. Six of twenty-nine assertions fail, but the failure is the
  lesser problem. The run also:

  - wrote a `[user]` section into the **shared** `.git/config`
    (`user.email = test@example.com`, `user.name = Test`) — repo-local config
    is shared by every worktree and session, so this would have mis-authored
    every subsequent commit from anywhere in the repository;
  - created a stray `refs/heads/feature`, an orphan lineage of eight empty
    fixture commits authored by `Test <test@example.com>`.

  Both were removed: `git config --local --remove-section user` (the effective
  identity now resolves from `~/.gitconfig` to `Ivan Subotic
  <400790+subotic@users.noreply.github.com>`, matching every commit on this
  branch) and `git branch -D feature` (was `d711ac6a`, recoverable from the
  reflog). The two commits on this branch predate the tainted run and are
  correctly authored; the worktree gitdir picked up no init artifacts.

  The tree itself is sound: the same `just test` passes in a normal shell and
  under `env -u GIT_DIR`. The practical consequences are that `just test`
  cannot be verified through a bare `git rebase --exec`, and that the fixture
  should `git init` with `env -u GIT_DIR` or `GIT_DIR=` cleared. Not fixed here
  — no checkbox covers it, and the script behaves correctly in every
  environment CI and developers actually use — but it is worth its own issue.

## Side findings — out of scope, recorded by the session (2026-09-17)

Three defects surfaced during Phase 0 and Phase 1 that are outside this plan's
scope. None blocks the run. All belong in the PR body as follow-ups.

1. **Five projects share one ARK PID.** `0801_bebb`, `0801_meditationes`,
   `0801_euler-goldbach`, `0801_condorcet-turgot` and `0801_reisebuechlein` all
   carry `pid: https://ark.dasch.swiss/ark:/72163/1/0801` despite distinct
   shortcodes (`0801a`, `0801c`, `0801d`, …). Verified: it is the only PID
   collision in the corpus, 5 of 85 projects; every other PID is unique.
   Consequence once Phase 2 ships: those five landing pages each assert the same
   JSON-LD `@id` and the same Signposting `cite-as`, i.e. five datasets claiming
   one identifier. F-UJI F1 (identifier uniqueness) is one of the three tests
   that pass today.
   Deliberately **not** worked around in code: emitting the recorded PID is
   faithful, and minting `ark:/72163/1/0801a` to force uniqueness would be
   inventing identifiers (ADR-0005, *Nothing is invented for a score*) and would
   likely not resolve. This is a source-data fix, RDU territory.
   Project 0862, the target of every Success Metric, is unaffected.
   It also makes OAI `GetRecord` ambiguous for that identifier today, which is
   why the Phase 1 hash baseline is a sorted line multiset rather than a map
   (a map dropped four of the five projects).

2. **`.github/scripts/check-commit-count.test.sh` mutates the host repository.**
   Its `make_repo` helper runs `git init` / `git config` / `git commit` without
   clearing `GIT_DIR`. `git rebase --exec` exports `GIT_DIR`, so under the
   end-of-phase gate all of that hits the real repository instead of the
   throwaway fixture: it wrote `user.email=test@example.com` into the shared
   `.git/config` (read by every worktree and session) and created an orphan
   `feature` branch of 8 empty commits. Both cleaned up in round 2; verified
   afterwards that all commits on this branch are authored
   `Ivan Subotic <400790+subotic@users.noreply.github.com>` from the global
   `~/.gitconfig`, so nothing was mis-authored and the removed `[user]` section
   was the taint itself.
   Workaround in use: every gate invocation is `env -u GIT_DIR just check`.
   Real fix is one line in the helper. Not applied — no checkbox covers it.

3. **The `oai_datacite` golden tests pin degraded output.** They resolve
   temporal coverage against an empty enrichment table, because the relative
   data dir does not resolve from the crate's test CWD. They therefore assert
   degraded output and would break if they ever saw the real data dir, so they
   do not test what they appear to test. Pre-existing, unrelated to this plan.

4. **`editor-server`'s three-strike auth test is flaky under load.**
   `auth::handlers::tests::test_simultaneous_wrong_guesses_cannot_outrun_the_three_strike_limit`
   failed once during Round 5's `git rebase --exec` gate, at
   `modules/editor/server/src/auth/handlers.rs:924` with "the account counter
   recorded 0 failures from one code". It passed 5/5 when re-run in isolation
   and the full `just test` passed at that same commit, so the concurrent
   guesses it spawns lose their race under the load of back-to-back full-suite
   runs. Pre-existing; `editor-server` is untouched by this plan. Worth a look
   before it fails in CI, where it will look like a real regression.

## Round 6 — Phase 2 (2026-09-18)

Starts from `97abd613`. Phase 2 stops before its `eng:reviewing` checkbox; the
session owns the reviewer set.

### Decisions taken before writing

1. **The F-UJI source check, done first** because it could have changed the
   JSON-LD shape. `metadata_mapper.py` read from
   `ghcr.io/pangaea-data-publisher/fuji@sha256:3eca94076b2272a18dcd8cddd1adad7675c2ac98a390799dd60f62369688a6fa`
   (the digest of `:latest` on 2026-09-18). The schema.org JMESPath mapping
   reads `object_identifier: [((identifier.value || identifier[*].value ||
   identifier || "@id") || (url || url."@id")), …]`. A `PropertyValue` with a
   `value` key is therefore read, and the plan's array-of-both fallback is **not
   needed**. Confirmed in the same file: `creator` is read off
   `creator[?"@type" =='Person'].name`, `publisher` off `publisher.url` and
   `publisher.name`, `access_level` off `conditionsOfAccess`, `access_free` off
   `isAccessibleForFree` — all shapes the property table already specifies.

   Pulling the image needed a workaround on this machine and Phase 3's
   `just fair-check` will hit the same wall: `~/.docker/config.json` sets
   `"credsStore": "desktop"` but Docker Desktop is not installed (the engine is
   colima), so every registry call dies with `docker-credential-desktop: not
   found`. Run docker with `DOCKER_CONFIG` pointing at a directory holding a
   `config.json` of `{}`. ghcr.io serves this image anonymously, so no
   credentials are actually needed.

2. **`DPE_PUBLIC_BASE_URL` landed without the `AppState` half.** The plan's
   checkbox asks for the config value *and* the `AppState` thread in one breath.
   `just check` runs with `-D warnings`, and the gate runs it on every commit
   independently, so an `AppState` field with no reader fails the commit that
   adds it. The config field, its validation and its documentation are commit 1;
   the `AppState` fields arrive in the `metadata.rs` commit that reads them.
   Checkbox amended to say so.

3. **`AppState` carries both base URLs.** `head_extras_for_project` needs
   `oai_base_url` for the two `describedby` targets. Taking it from state rather
   than reaching for `dpe_api_oai`'s process-global keeps one place that says
   what a landing page is built from, and keeps both URLs variable in a test.

4. **`ProjectGraph::build` takes an iterator of records, not a slice**
   (carry-forward 6: the `hasPart` cap must not be paid for in the builder).
   `PartRef`-per-record is 27,026 allocations for project 081C to emit 100. An
   `impl IntoIterator<Item = &Record>` lets the landing page bound what it
   materialises at the call site, the OAI writers pass `std::iter::empty()`, and
   Phase 3's uncapped representation passes the whole slice. The writer keeps
   `opts.has_part_cap` as the *emission* contract — separately testable with an
   oversized fixture — and the landing page passes the same constant to both.
   Not redundant: one bounds the allocation, the other bounds the output.

5. **`ProjectGraph.data_type` was not reintroduced** (carry-forward 3). `DC.type`
   reads `DublinCoreRecord.resource_type`, which `project_to_dublin_core` sets to
   the constant `"Project"`. No graph field is needed.

### The record index, and what the hash test does not prove

`records_for_shortcode` repoints the OAI `set=project:{shortcode}` filter, which
is an OAI-path change, so the brief asked for the Phase 1 hash baseline to be
run once. It was, from a temporary checkout of `4cdf06d8^`'s `corpus.rs`:
`compared 102158 entries`, took 6.8s, passed, and the file was restored.

**It proves nothing about this change.** Reading it showed `compute_lines`
hashes `to_oai_record` and `to_oai_record_from_record` per item — the payload
writers. It never calls `collect_filtered_records` and never exercises a set
filter at all. The 102,158 entries are 50,994 records × 2 prefixes plus 85
projects × 2, which is the whole corpus item by item and none of the paging.

So the real guard is new and permanent:
`the_shortcode_index_serves_what_the_scan_served` in `dpe-api-oai`'s
`corpus.rs`. It runs the index builder over every committed dump — the same flat
vector the cache builds — and compares it, shortcode by shortcode and record by
record, against the exact filter expression it replaced. Order is the point:
`ListRecords` pages that sequence and a resumption token is an offset into it.

To make that testable, the grouping is a free function,
`record_cache::index_by_shortcode(&[Record])`, with the cache's `OnceLock`
calling it. The cache itself is keyed on the process-global `DPE_DATA_DIR`,
which a test sharing a process with the handler tests cannot set — the same
constraint that forced the Phase 1 hash test to be `#[ignore]`d.

The filter reaches the index through a new `RecordRepository` method rather than
around it, so the in-memory double the OAI handler tests inject still answers.
Its default implementation is the scan it replaces, which is the right one
behind a handful of records; `FsRecordRepository` overrides it with the index.

One behaviour change, deliberate: the set filter now looks records up by the
**resolved project's** canonical shortcode instead of the set spec's spelling.
The existence check immediately above it already resolved the project through
`to_uppercase`, while the filter used `eq_ignore_ascii_case`; the two could
disagree on a non-ASCII spelling. Now they cannot. Committed shortcodes are
ASCII, so no committed output moves — which the hash run and the equivalence
test both confirm.

### Deviations in the writers, all amended into the checkbox text

1. **`ProjectGraph::titles()`.** The plan says the JSON-LD `name` uses "the same
   precedence as DataCite titles". Two implementations of "the same precedence"
   is the drift ADR-0005 exists to stop, so the precedence moved onto the graph
   and the DataCite writer now calls it. The hash baseline was run again
   afterwards: `compared 102158 entries`, unchanged. That extraction *is*
   covered by the baseline, unlike the `records_for_shortcode` change.

2. **`UrlLayout` carries a `catalog` URL**, which the checkbox does not list.
   `includedInDataCatalog` needs `{public_base_url}/dpe/projects`, and
   `check-shared-paths.sh` plus the crate's own rule forbid `shared-fair`
   holding that path. Every URL a writer emits arrives through `UrlLayout`; the
   catalogue is one of them.

3. **`DC.title` and the JSON-LD `name` can differ, by design.** `oai_dc` prefers
   `officialName` and adds only the recorded alternative names; DataCite takes
   the longer of `name`/`officialName` and demotes the other. The meta-tag
   writer is a second *rendering* of the `oai_dc` record, not a second mapping,
   so it inherits that choice. The agreement test therefore compares each
   writer's titles against the graph's title set rather than against each other.

4. **The meta-tag writer drops placeholders, `project_to_dublin_core` does
   not.** The committed `oai_dc` output has always carried `MISSING` through and
   moving that is outside this plan. A landing page telling a harvester
   `DC.title = "MISSING"` is a different claim and a false one, and the
   acceptance criteria forbid it. Same rule in the JSON-LD writer.

5. **`datePublished` carries `extract_year`'s `"2015"` fallback** for a project
   with no usable date, because `publication_year` is resolved on the graph for
   DataCite, where the field is mandatory. Suppressing it here would make the
   JSON-LD and the DataCite record disagree, which is what the agreement test
   exists to prevent. Recorded as a residual: the graph cannot currently say
   whether that year was read or invented.

6. **Graph fixtures moved to `shared/fair/src/test_support.rs`.** Four writers
   now describe the same object; a second copy of the fixture would let them
   drift. `project_graph`'s test module keeps its assertions and imports the
   fixtures. The fixture gained `person-002`, the one person with an ORCID and
   an affiliation, so the identifier branches have something to read.

7. **No `load()`-level test for `DPE_PUBLIC_BASE_URL`.** The first version of
   commit 1 had one, using `figment::Jail`. `Jail` sets the variable in the
   *process* environment, so a jailed test that makes `load` fail makes it fail
   for every test loading a config at the same moment —
   `config::tests::load_with_defaults` went red under `--test-threads` > 1 and
   green in isolation. Amended out of commit 1 (reset to it, amend,
   cherry-pick the five later commits back; the plan file was copied aside and
   restored, never stashed). The rule is tested on `validate_public_base_url`
   directly.

### The landing page

**`project_oai_identifier` is now public in `dpe-api-oai`**, extracted from the
inline block in `to_oai_record`. The page links its own OAI records and has to
name them exactly as `GetRecord` answers to; deriving that identifier a second
time in `dpe-server` would be a second rule. The hash baseline keys on
`header.identifier`, so it *does* cover this extraction: run again afterwards,
`compared 102158 entries`, unchanged.

**`urlencoding::encode` rather than a hand-rolled encoder.** The crate is
already a `dpe-server` dependency and already the idiom here
(`fragments.rs:161`, `dpe-web`'s `project.rs:195`). It over-encodes `:` and `/`,
so the OAI URL reads `oai%3Adasch.swiss%3Aark%3A%2F72163%2F1%2F0862` rather than
the bare form a harvester would type. Legal, unambiguous, and decoded by the
OAI endpoint's own query extraction. Not worth hand-rolling an encoder to keep
two characters bare.

**`test_state()` sets the process-global data and public dirs.** `cargo test`
runs from the package directory, where `dpe-core`'s relative defaults miss, and
both are `OnceLock`s whose first caller wins. The pure-render tests reach
`resolve_inputs()` — and so `get_data_dir()` — without building a router, so
before this they could pin the *empty* corpus for the whole binary and the
corpus tests then failed depending on thread order. That was observed: the same
tests passed alone and failed under a filter. Putting both calls in the one
function every test goes through removes the order dependence.
`fragments.rs`'s `init_test_data` sets the same two paths, so the two agree; it
also sets `show_placeholder_values(true)`, which affects only the `Project` view
model and not the graph the head is written from.

**The header-injection test goes through the percent-encoded segment.** `"` and
`>` are not legal raw in a request URI and the `http` crate would reject them
before the handler ran, so the test sends `%22`, `%3E`, `%0D%0A` and a bare `;`.
Axum decodes them into the path parameter, no project resolves, and the existing
always-200 "Project Not Found" body comes back with no `Link` header and no
JSON-LD.

**The `<link>`-set-equals-header-set check runs on `render`, not through
`oneshot`.** It is a property of the two renderings, not of the route. Maud
escapes `&` in an attribute value and the header does not; the test unescapes
that one difference before comparing.

### The agreement test replaced the smoke test

`every_committed_project_and_a_record_sample_survive_all_four_writers` said in
its own doc comment that it stood in "until Phase 2's agreement test lands", and
`RECORD_SAMPLE`'s comment said the agreement test "grows out of this one". So
`every_representation_of_a_committed_object_agrees_with_the_others` replaces it
rather than sitting beside it; a stale "four writers" test next to a seven-writer
agreement test would read as two guards where there is one.

`the_largest_committed_project_embeds_a_small_json_ld_block` is the second half.
It asserts the cap is actually filled (100 parts) before asserting the budget, so
it cannot pass by measuring an empty graph.

### Phase 2 commits

Nine, from `97abd613`:

1. `feat(dpe-server)` — `DPE_PUBLIC_BASE_URL` and its validation
2. `perf(dpe-core,dpe-api-oai)` — the shortcode index
3. `feat(shared-fair)` — the Signposting link set and `UrlLayout`
4. `feat(shared-fair)` — the schema.org JSON-LD writer and `script_safe_json`
5. `feat(shared-fair)` — the Dublin Core meta-tag writer
6. `refactor(dpe-server)` — `HeadExtras`
7. `feat(dpe-server)` — `metadata.rs`, the handler, the header
8. `test(dpe-api-oai)` — the corpus-wide agreement test
9. `docs(docs)` — `machine-readable-metadata.md`

### The truncation test was weaker than it read

Caught while answering a question about it, after the first gate run, and
amended into commit 5 (reset, amend, cherry-pick the four later commits, gate
re-run — all green).

The original test filled the description with `ä`. That is **two** bytes, so
byte 1000 is a character boundary: the naive `&text[..1000]` would not have
panicked there, it would have silently returned 500 characters. The test
asserted the character count, so it did catch that — but it did not exercise
the panic the comment claimed it did.

`—` is three bytes, and byte 1000 lands inside a character, so `&text[..1000]`
panics. The test now runs both fillers, and a second test states the two
failure modes directly (`is_char_boundary(1000)` is `true` for the two-byte
filler and `false` for the three-byte one). The production code was already
correct — `char_indices().nth()` cannot land off a boundary — so this was a
test-strength gap, not a defect.

Worth carrying into any later truncation: "multi-byte" is not enough to
reproduce the bug. The character width has to be coprime with the limit.

The OAI hash baseline was run three times during the phase — after the
`records_for_shortcode` change, after the `titles()` extraction, and after the
`project_oai_identifier` extraction — and reported `compared 102158 entries` and
no difference each time. It is not committed; each run restored `corpus.rs`
afterwards.


## Round 7 — Phase 2 review fixes (2026-09-18)

Sixteen fixes from ten reviewers on the nine Phase 2 commits. Maud found nothing
blocking; several fixes were raised independently by two or three reviewers.

Phase 2 is now **ten commits**, `97abd613..` — nine rewritten in place and one
new `fix:` at the tip. Still local; `origin/worktree-fair-assessment` is at
`f1532fac`.

| Fix | Raised by | Landed in |
|-----|-----------|-----------|
| 1 — stop inventing a publication year | ivan, rust | own commit at the tip, `fix(shared-fair,dpe-api-oai)` |
| 2 — widen the `PreEscaped` guard to the whole crate | maud, rust, security | folded into the landing-page commit |
| 3 — delimiter-safety for `Link` values | security | folded into the link-set commit |
| 4 — consolidate `real()` into `helpers.rs` | simplicity, patterns | folded into the schema.org commit (+ imports at the DC-meta commit) |
| 5 — one license-URI dedup, `ProjectGraph::license_uris()` | simplicity, patterns | folded into the link-set commit, called from the schema.org commit |
| 6 — `coar_access_right` beside `access_rights_to_string` | patterns | folded into the schema.org commit |
| 7 — writer naming, `project_to_schema_org` / `project_to_link_set` | patterns | folded into the two writer commits |
| 8 — warm the shortcode index at startup | performance | folded into the shortcode-index commit |
| 9 — the agreement test compares DataCite ORCIDs | consistency | folded into the agreement-test commit |
| 10 — the `PreEscaped` site count, four places | consistency, dune DUNE-101 | folded into the landing-page commit |
| 11 — ARCH-MAP's `shared/fair` consumers | dune DUNE-102 | folded into the landing-page commit |
| 12 — `serde_json` is a runtime dependency | dune DUNE-103 | folded into the landing-page commit |
| 13 — "Representation" in Flagged ambiguities | dune DUNE-104 | folded into the landing-page commit |
| 14 — `oai_base_url` doc honesty | patterns | `AppState` at the landing-page commit; `config.rs` at the config commit |
| 15 — trim the docs page's "How it is kept safe" | simplicity | folded into the docs commit |
| 16 — plan line 232 | consistency | working tree only, never committed |

### Fix 1 — the headline, and the only behaviour change

`to_schema_org` inserted `datePublished` unconditionally — the one field in that
writer with no `real()` guard — from a graph field `extract_year` had already
turned into `"2015"` whenever the input carried no readable year. So a landing
page told every FAIR assessor that a project with no usable date was published
in 2015. Nothing caught it: `"2015"` is not a `MISSING`/`CALCULATED` sentinel,
so the placeholder assertions in three separate tests went straight past it.

The fix follows Phase 1's creator-fallback shape exactly. `extract_year` returns
`Option<String>`; both graphs carry `publication_year: Option<String>`, the year
the input actually yields; `publication_year_with_fallback()` on each graph
holds the `"2015"` rule once, against a `FALLBACK_PUBLICATION_YEAR` constant
beside `DASCH` in `graph.rs`. Both DataCite writers call the accessor, so
**DataCite and OAI output are byte-identical** — re-proven, see *Byte identity*.
The JSON-LD reads the raw `Option` and omits the key.

Two things worth recording:

- **Presence-not-validity is preserved and now stated.** A recorded but unusable
  `dataPublicationYear` yields `None` and does *not* fall through to
  `start_date`. That is what the byte-identical output has always done, and a
  project that declared a publication year is not making a claim about its start
  date. `publication_year`'s doc comment previously said `extract_year` turns an
  unusable value into its own fallback, which stopped being true.
- **`extract_year` changed signature rather than gaining a sibling.** The brief
  asked for "a `None`-returning sibling". Once both accessors exist, nothing
  calls a `String`-returning version, and a `pub fn` with no callers is cruft.
  Deviation recorded here rather than silently taken.

The corpus agreement test gained the assertion that pins the split, on both the
project and the record path: where the graph has a year both representations
carry it; where it has none, DataCite carries `"2015"` and the JSON-LD carries
no key at all. That assertion is in fix 1's own commit, not amended into the
agreement-test commit — it pins a behaviour change that does not exist until
fix 1 lands.

### Fix 3 — where the escaping goes, and what is deliberately left alone

`escape_delimiters` percent-encodes `<`, `>`, `"`, `;`, `,` and space, and runs
in `Link::to_field_value` — at **serialization**, not at construction. So
`Link.href` stays the identifier the JSON-LD `@id`, the `cite-as` link and the
`<link>` elements all carry, and the agreement test's "cite-as is the ARK" check
still compares an ARK against an ARK. `<`, `>`, `"` and space may not appear in
a URI at all (RFC 3986), so encoding them costs nothing; `;` and `,` are
sub-delimiters none of these identifier schemes uses.

**CR and LF are deliberately not escaped.** The HTTP layer rejects them, the
consumer drops the whole header with a `tracing::warn!`, and that is the louder
and more correct outcome for a PID nobody can have meant — it is also what
`a_header_value_the_http_layer_refuses_is_dropped_not_panicked` and the docs
page's account of that behaviour depend on. A test pins both halves.

### Fix 4 — and what was deliberately not swept in

`real()` moved to `helpers.rs` and the two byte-identical copies now call it.
`datacite.rs` was **not** swept in: it tests the license URI for a placeholder
only and keeps an empty one, which is in the committed OAI output, and it emits
one `rightsList` entry per `legalInfo` element rather than the deduplicated set.
Both differences now carry a comment at the call site saying they are deliberate.

One asymmetry, deliberate: `coar_access_right` keeps a root re-export
(`pub use helpers::coar_access_right`) where its sibling `access_rights_to_string`
has none. It has an out-of-crate consumer — `dpe-api-oai`'s agreement test — and
the sibling does not. Both now live in `helpers.rs`, which is what fix 6 asked
for; only the re-export differs, and it differs because the call sites do.

### Fix 5 — introduced one commit earlier than the brief named

`ProjectGraph::license_uris()` lands at the **link-set** commit rather than the
schema.org one. `signposting::distinct_license_uris` was born there, one commit
before `schema_org::license_uris`; introducing the shared accessor at the first
user means the duplication never exists at any commit, which folding into the
later one would not have achieved.

### Fix 10 — the count, and where the mosaic edits landed

Three sanctioned sites, not two: the Mosaic `IconData` SVG, the Mosaic
`textarea`'s leading newline (present since `c24223f6`, and named by ADR-0005:33
as something the rule must name), and the JSON-LD splice. Corrected in all four
places the brief listed: `ARCH-MAP.md`, `modules/dpe/CLAUDE.md`,
`modules/mosaic/CLAUDE.md` and `modules/mosaic/tiles/src/components/icon/mod.rs`.

The last two are pre-existing staleness with no Phase 2 commit of their own.
They were folded into the landing-page commit, which is the commit whose own
"exactly two" claim re-broke the count, and its scope widened to
`feat(dpe-server,mosaic-tiles)` to match — the same treatment the shortcode-index
commit got for its one `main.rs` line. Flagged for re-sequencing if the lead
would rather they stood alone.

### Fixes 11 and 12 — extended to `shared/README.md`

The brief named `ARCH-MAP.md`. `shared/README.md` carried the *same two*
sentences — "`dpe-api-oai` is the only consumer today" and "`serde_json` is a
dev-dependency" — and is itself in `shared/fair`'s local-context kit. Both were
corrected in the same commit. This is an extension of fixes 11 and 12, not a new
finding; called out here because the brief did not name the file.

**One-commit window, deliberately left.** `serde_json` moves to `[dependencies]`
at the schema.org commit, and both documents are corrected one commit later at
the landing-page commit — the only Phase 2 commit that touches either. Moving
the doc edits earlier would conflict through the landing-page commit's own
ARCH-MAP hunks. Same shape as Round 5's fix 9 window, and one commit wide.

### Fix 11 — the kit is at its budget

`shared/fair`'s local-context kit holds seven files. Rather than add an eighth,
the second call site (`modules/dpe/server/src/metadata.rs`) is **named in a
sentence** inside the kit entry, and the stale "the only consumer's call site"
parenthetical on `api-oai/src/metadata/mod.rs` became "the OAI call site".

### Byte identity

Re-proven on the finished tip, not assumed. The Phase 1 hash test was recovered
from `7a2c243b` exactly as this journal's Round 4 note describes — copied in as
`metadata/hash_check.rs`, trimmed after `oai_output_matches_hash_baseline`, with
`#[cfg(test)] mod hash_check;` added to `metadata/mod.rs`:

```
compared 102158 entries
took 6.7s
test metadata::hash_check::oai_output_matches_hash_baseline ... ok
```

All 102,158 entries match the baseline at `.claude/tmp/oai-baseline-hashes.txt`,
which was **not** regenerated. Both edits were removed afterwards and
`git status --porcelain` was clean before the gate ran. This matters most for
fix 1, which changes the shape of the field DataCite's mandatory
`publicationYear` reads, and for fixes 4 to 7, which touch the writers.

### The two reviewer claims the brief rejected

Recorded because both are the kind of thing that will be raised again:

- **"The plan file carries no ticked checkboxes and the amendments never
  landed."** False. The plan file is uncommitted **by design** — Round 1 records
  the decision, and the standing constraint every brief carries is never to
  stage or commit it. The reviewer diffed committed revisions of a file that has
  none. The working tree has its 60 ticks and the amended
  `&'static [&'static Record]` text at line 769.
- **"`Candidate` / `UrlLayout::candidates()` belong in Phase 3."** Declined. The
  plan specifies them by name at line 770 so that the workspace builds with the
  negotiation type in place; they stay.

### A second `editor-server` flake, not the named one

The gate stalled at the landing-page commit for ten minutes on two
`editor-server` SQLite **file**-database tests:

```
db::schema::tests::test_reopening_a_file_database_keeps_its_data_and_does_not_re_migrate
db::tests::test_file_database_uses_wal_and_creates_its_siblings_in_the_directory
```

Not the flake the briefs name (`test_simultaneous_wrong_guesses_cannot_outrun_the_three_strike_limit`)
— a different pair, and a hang rather than a failure. Both pass in 0.01 s when
run alone, all 133 `db::` tests pass in 0.59 s, and the full `just test` at that
same commit passed on re-run. The shape is an intra-binary race on a shared
SQLite path under the parallel test runner, with one test holding the shm lock.

`editor-server` is untouched by Phase 2 — the only file this round changed that
it transitively depends on is a doc comment in `mosaic-tiles` — so this is
pre-existing and was treated as a flake rather than a regression, after
verifying in isolation as the brief requires. Worth its own ticket.

### Environment note

`git add` refuses `modules/mosaic/tiles/src/components/icon/mod.rs`: the user's
global ignore file (`~/.config/git/ignore`) carries macOS's `Icon` rule, and
`core.ignorecase` on macOS makes it match the `icon/` directory. The file is
tracked, so the rule is a false positive and `git check-ignore` reports nothing;
`git add -u <path>` stages it without complaint. Anything touching that
directory in future will hit the same thing.

## Round 8 — Phase 3 (code complete, reviewer set not yet run)

Seven commits on top of `7fe29df2`, stopping before the `eng:reviewing`
checkbox as the brief asked.

| | commit | what |
|-|--------|------|
| A | `7cb164cd` | `feat(shared-fair)` — `project_to_datacite_json` |
| B | `e4a7bc15` | `test(shared-fair,dpe-api-oai)` — the DataCite JSON schema fixture and corpus validation |
| C | `e06ffacd` | `feat(shared-fair)` — `negotiate.rs` and `representation_to_link_set` |
| D | `9775b5da` | `feat(dpe-server)` — the two representation routes |
| E | `8b06ad73` | `feat(dpe-server)` — the `303` and `Vary: Accept` |
| F | `d7b399a3` | `chore(ci)` — `just fair-check`, `jq` |
| G | `677b1ae3` | `docs(docs)` — the docs and the four enforcement flips |

### The F-UJI result, and what it means for Phase 4

`just fair-check http://host.docker.internal:4000/dpe/projects/0862 12` against
a local server: **14 of 24**, exit 0. Up from the 3 of 24 baseline.

```
FsF-F1-01D 1/1   FsF-F1-02D 0/1   FsF-F2-01M 2/2   FsF-F3-01M 0/1
FsF-F4-01M 1/2   FsF-A1-01M 1/1   FsF-A1-02M 1/1   FsF-A1-03D 0/1
FsF-I1-01M 2/2   FsF-I2-01M 0/1   FsF-I3-01M 1/1   FsF-R1-01MD 1/4
FsF-R1.1-01M 2/2 FsF-R1.2-01M 1/2 FsF-R1.3-01M 1/1 FsF-R1.3-02D 0/1
```

**Phase 4's skip condition is not met as written.** It asks for I1 at 2/2, I2 at
1/1 and I3 at 1/1. I1 and I3 are there; **I2 scores 0/1** (its `test_status` is
reported as `pass`, but the earned score is 0). The session decides what follows
from that. Worth weighing before it does: I2 is "metadata uses semantic
resources for its vocabulary terms", which a Turtle serialisation of the same
graph would not change — it is about the terms, not the syntax. I1, the test
that *is* about machine-readable RDF, is already full marks off the JSON-LD.

### F-UJI: the pinned digest had to move, and why

The carried-forward digest `sha256:3eca9407…` is **4.0.0**, and its published
image **does not start**: it launches a headless Chromium at startup that is not
installed in it (`Executable doesn't exist at
/root/.cache/ms-playwright/chromium_headless_shell-1234/…`, `ERROR: Application
startup failed. Exiting.`). Phase 2 only ever read files out of it with
`--entrypoint cat`, which never starts the app, so nobody had hit this.

Repinned to **3.5.0**, `sha256:3cde9d30bc148798a512b9e3a8a9ee6e63c4d09a6a33bb7651ac007c5824c687`.
That is also the better pin on its own merits: 3.5.0 is the version the
2026-09-15 baseline row was taken with, so the new row is directly comparable
and the Success Metrics target of 12/24 is against the same metric set.

Both Phase 2 findings were **re-verified against 3.5.0**, not assumed:

- `metadata_mapper.py:218` reads the object identifier as
  `identifier.value || identifier[*].value || identifier || "@id"`. The
  `PropertyValue` decision holds, and the plan's array fallback stays
  unnecessary.
- The merge-order checkbox: `metadata_harvester.py::merge_metadata` coerces
  `object_type` into a **list** and extends-and-uniquifies it, and
  `fair_evaluator_data_content_metadata.py::subtestResourceTypeGiven` iterates
  every entry, setting `valid_type_found` if **any** matches. So DataCite's
  `Project` **cannot** override JSON-LD's `Dataset`. Identical in 4.0.0. No
  resource-type regression, no residual to record, and the `describedby` link
  needed no defending.

### The DataCite JSON schema: three patches, not two, and one of them is mine

The plan's premise that "4.6 additions over 4.3 are optional properties" is
wrong, and it matters: `resourceTypeGeneral: "Project"` and
`dateType: "Coverage"` are 4.6 **enum values**. Unpatched, every committed
project fails. `download-schemas.sh` widens both enums from the kernel-4.6 XSDs
already committed for the OAI tests, so the two copies of the vocabulary cannot
drift.

Running it surfaced three further classes of failure across nine projects. The
line taken, after weighing it: the fixture is a **shape** check, and the
corpus's own data quality is not this plan's business.

1. **`uniqueItems`** — four projects repeat a keyword, and one person file
   repeats an ORCID and an affiliation. Removed from the schema wholesale, with
   the reason in the script header: the kernel XSD imposes no uniqueness
   anywhere, the Invenio-derived JSON schema adds it, and de-duplicating in the
   writer would make the JSON and the XML disagree about content — the one thing
   the two representations may not do.
2. **`format: uri`** — `085C_wiborada.json` records a CC-BY-NC URI with a
   trailing space. Handled in the *validator*, not the schema:
   `should_validate_formats(false)`. `format` is an annotation in draft-07 and
   the crate is stricter than the specification. Trimming it in the writer would
   be rewriting a recorded value. (085C, not 0862, so it does not touch the
   F-UJI row.)
3. **`/dates/1: "date" is a required property`** — **a real bug in my writer**,
   not a corpus defect. `put` treated an empty string as an absence, and a
   name-only `Coverage` date legitimately carries `date: ""` (the XML emits an
   empty element). Fixed by deleting that arm — only `optional` produces a
   `Null`, and that is what "nobody recorded this" looks like. Amended into
   commit A with a test, per the standing rule, rather than landed as a `fix:`.

`rights_uri: Some("")` now emits `"rightsUri": ""` for the same reason, which is
parity with the XML's `rightsURI=""`; the corpus assertion gained the
`!uri.is_empty()` filter the XML comparison three lines above already had.

`jsonschema` 0.56 is a new **dev-dependency of `dpe-api-oai`**, not of
`shared-fair` — the acceptance criteria pin that crate's dependency list, and
the check reads the committed corpus, which lives beside `dpe-api-oai`.
`default-features = false` drops `reqwest` and a TLS stack. The `regex-automata`
and `zmij` bumps in `Cargo.lock` are its own tree's constraints
(`fancy-regex` needs `regex-automata >= 0.4.18`), not stray churn.

### `oai_router` became `rate_limited_router`

The brief said the representations sit behind "the same per-IP `tower_governor`
layer the OAI route uses" from day one. Taken literally: they joined the same
sub-router, so there is one layer and one bucket rather than two layers sharing
an `Arc`. The function that carried one route now carries three, so its name
had to follow. `gates_oai_only` became `gates_oai_and_the_representations` and
gained the landing page as a second negative control beside the download route.

`corpus_app()` in `metadata.rs` previously passed an empty `Router` for the
rate-limited sub-router, which would have left the representation routes
unregistered in every handler test. It now builds the real sub-router with
`tower::layer::util::Identity` — the routes are real, the limiter is a
passthrough, and whether the limiter is wired to exactly those routes stays
`router.rs`'s own test rather than becoming a function of how fast the suite
runs.

### Three Phase 2 tests changed shape, all for the same reason

Filling `UrlLayout.representations` grows `describedby` from two links to four.
`the_describedby_targets_are_the_oai_records_of_this_project` became
`…_are_the_representations_and_the_oai_records`;
`hostile_text_cannot_escape_the_script_or_the_query_string` now picks the OAI
target by name rather than taking the first `describedby`, because the
representation URLs carry no PID at all.

`the_json_ld_representation_is_the_uncapped_graph` uses **0803**, not 0862. Only
three committed projects have records (0803: 4,198; 0868: 19,770; 081C: 27,026)
and 0862 has none, so it could not have shown a cap being lifted. The same test
now also reads the embedded block on 0803's page and asserts it is still capped
at 100, so the two limits are checked against each other in one place.

### The live run: two deviations from the checkbox, both recorded in it

- **`DPE_SITE_ADDR=0.0.0.0:4000`** is required and the plan does not mention it.
  The default binds `127.0.0.1`, which a container under colima cannot reach
  through `host.docker.internal` — Docker Desktop proxies host loopback, a
  Lima-backed runtime does not.
- **Not `just dev`.** That drives `bacon`, a TUI, which does not belong in a
  detached process; the built `dpe-server serve` binary was started instead.
  The Tailwind watcher `just dev` also runs has no bearing on what F-UJI reads.

The recipe's own 60-second readiness wait was also wrong twice over, and is now
180 seconds against **any** HTTP response: F-UJI refreshes its re3data DOI table
before it starts listening, and `/fuji/api/v1/metrics` answers 401 without
credentials, so `curl -sf` would have failed even once it was up.

The `DOCKER_CONFIG` carry-forward was confirmed necessary and is baked into the
recipe as the brief asked.

### Byte identity

Re-proven on the finished tip. The Phase 1 hash test was recovered from
`4cdf06d8^` — **not** `7a2c243b`, which the Round 7 journal names and which does
not carry the file; the test lived inside `corpus.rs` and was removed by
`4cdf06d8`, so its parent is where it is. Copied in as
`metadata/hash_check.rs`, trimmed to `oai_output_matches_hash_baseline` and its
helpers, with `#[cfg(test)] mod hash_check;` added to `metadata/mod.rs`:

```
compared 102158 entries
took 6.8s
test metadata::hash_check::oai_output_matches_hash_baseline ... ok
```

All 102,158 entries match `.claude/tmp/oai-baseline-hashes.txt`, which was
**not** regenerated. Both edits were removed afterwards and
`git status --porcelain` showed only the two spec files. This matters because
the DataCite JSON writer reads the same `DataCiteRecord` the XML writer does.

### Enforcement flips: attributed, and nothing over-claimed

All four places name the tests by function rather than by description, so a
reader can check the claim. `REVIEW.md`'s new line is the one mechanism that is
not automatic, and it says why it exists: a test says the metadata is there,
only `fair-check` says it is readable.

The representation routes get **no** `KNOWN_ROUTES` entry, as the brief
required — they load no `telemetry.js`.

## Phase 4 decision — skipped, on evidence the plan's condition did not anticipate (2026-09-18, session)

> **Later note (Round 13).** The skip was correct and the reasoning below on
> *serialisation* holds: Turtle would have earned nothing. Two details did not
> survive. I2-01M is no longer 0/1 — Phase 6 earned it with a PROV-O statement,
> which is the controlled-vocabulary link this section itself said it would take,
> so the condition is now met by other means. And one of the two mechanisms below
> is wrong: the namespace strip does not leave nothing to score, and no sub-test
> adds a false status to the score. Corrected reading in *Round 12*. The text is
> left as written, because this section records what was known when the decision
> was made.

**Decision: Phase 4 (Turtle) is skipped.** The plan's literal skip condition is
*not* met, and skipping is nevertheless the right call. Recorded here because the
plan's suggested note ("skipped, Phase 3 passed I1 to I3") would be false.

### What the run showed

`just fair-check … 0862 12` → **14 of 24**, exit 0 (baseline 2026-09-15: 3 of 24).
Target was ≥ 12. The skip condition asks for I1 2/2, I2 1/1, I3 1/1:

- I1-01M **2/2** ✓ — formal representation
- I3-01M **1/1** ✓ — related entities
- I2-01M **0/1** ✗ — semantic resources (`test_status: pass`, earned 0)

### Why Turtle cannot move I2 — two independent reads of the pinned image

Both were taken from F-UJI 3.5.0, `sha256:3cde9d30bc148798a512b9e3a8a9ee6e63c4d09a6a33bb7651ac007c5824c687`.

1. **The vocabularies are stripped before scoring** (found by the ivan reviewer).
   `fuji_server/data/default_namespaces.txt` excludes `schema.org` *and*
   `purl.org/dc/elements/1.1/` + `purl.org/dc/terms/` — exactly the two
   vocabularies this landing page emits. `removeDefaultVocabularies` in
   `fair_evaluator_semantic_vocabulary.py` removes them before either I2 subtest
   runs, so `namespace_uri` is empty by the time scoring happens.
2. **One subtest is unearnable in 3.5.0** (found by the session).
   In `testSemanticNamespaceURIsAvailable`, `self.score.earned += test_status`
   executes while `test_status` is still `False`, adding zero, before it is set
   to `True`. That is exactly the observed `test_status: pass` earning 0. The
   other subtest, `testKnownSemanticResourcesUsed`, earns only when a namespace
   resolves in the LOD/LOV registry — again a question of *which* vocabularies
   are used.

Both obstacles are about **which vocabulary namespaces appear**, not about
serialisation syntax. A Turtle rendering of the same graph carries the same
namespaces, so it cannot change I2 by construction.

### Why skipping is right

- The skip condition's stated rationale is "JSON-LD is RDF and F-UJI accepts it".
  I1 at 2/2 is the test that measures machine-readable RDF, and it is full marks.
- Phase 4 would add either a hand-rolled Turtle writer or a new `oxrdf`/`oxttl`
  dependency for a measured gain of zero.
- The plan itself calls Turtle "last and droppable" for that reason, and the
  Success Metrics target (≥ 12) is already exceeded at 14.

### Consequences

- Phase 4's checkboxes are ticked as skipped with the reason above, **not** with
  the plan's suggested wording.
- I2-01M is recorded in *Known residuals* with the evidence above. Moving it
  needs a controlled-vocabulary link F-UJI's registry recognises, which is new
  scope, not a serialisation change.
- The plan's Success Metrics row for I2 (target 1/1) is not met and is not
  reachable by anything in this plan. Recorded as a follow-up, not a silent miss.

## Round 10 — Phase 3 review fixes (2026-09-18)

Twelve fixes from ten reviewers, the last round before ship. Rust and maud found
nothing. The brief named the reviewer behind some findings and collected the rest
without attribution; they are recorded that way rather than guessed at.

Nothing here changes OAI output. Re-proven, not assumed — see **Byte identity**.

Phase 3 is still seven commits. Five were rewritten; `7cb164cd` and `e4a7bc15`
were not touched.

| Was | Folded in | Final SHA |
|-----|-----------|-----------|
| `e06ffacd` | fixes 7, 10, 11 in `shared-fair` | `ce56115b` |
| `9775b5da` | fixes 2, 3, 7, 10, 11 in `metadata.rs` | `9df1c3de` |
| `8b06ad73` | fixes 10, 11 in `metadata.rs` and `main.rs` | `d700fc20` |
| `d7b399a3` | fix 1 whole, fixes 10 and 11 in the `justfile` | `320c9d3d` |
| `677b1ae3` | fixes 4, 5, 6, 7, 8, 9, 10 in the documentation | `62d7b861` |

The intermediate SHAs the fix table below names are the autosquash results; a
later message-only pass moved all seven. See *The commit messages were
reworded*.

| Fix | Raised by | Landed in |
|-----|-----------|-----------|
| 1 — the enforcement claim made true both ways | dune and devops, in opposition; see below | `9e2c9cb2` (script, gate, gate test, `justfile`) + `1717ee7f` (the three claims) |
| 2 — CPU-bound work off the Tokio runtime | security, with a measurement | `9110af80` |
| 3 — one `build_graph` helper | brief, unattributed | `9110af80` |
| 4 — the I2-01M residual, and two unattributed zeros | ivan (the namespace strip) and the session (the unearnable sub-test) | `1717ee7f` |
| 5 — the Assessment row said `just dev` | brief, unattributed | `1717ee7f` |
| 6 — three unreachable ADR links | brief, unattributed | `1717ee7f` |
| 7 — stale references the rename left | brief, unattributed | `1717ee7f`, `9110af80`, `ffa2fc65` |
| 8 — `operations.md` on the rate limiter | brief, unattributed | `1717ee7f` |
| 9 — three subsets of one test list | brief, unattributed | `1717ee7f` |
| 10 — narration and doc restatements trimmed | brief, unattributed | `9e2c9cb2`, `9110af80`, `ffa2fc65`, `e7e80ca3`, `1717ee7f` |
| 11 — small test and comment corrections | brief, unattributed; 11e from devops | `9110af80`, `e7e80ca3`, `ffa2fc65`, `9e2c9cb2` |
| 12 — Phase 4 skipped, recorded honestly | the session's own Phase 4 decision | plan file, working tree only |

### Fix 1 — dune versus devops, and why both halves were taken

Two reviewers reached opposite conclusions on the same evidence. **Dune** ruled
the flipped claim false and wanted the gate widened. **Devops** ruled the gate
correctly scoped and wanted the script parameterised. Both are right about one
half of a two-part defect, and neither half fixes it alone:

- The claim in `ARCH-MAP.md`, root `CONTEXT.md` and ADR-0005 was stated
  **unqualified** — "`shared-fair` holds no path into an area" — while
  `check-shared-paths.sh` enforced the narrower `shared/*/src/*.rs`. So the
  sentence was false as written, which is dune's point.
- Widening the gate alone would have caught
  `shared/fair/testdata/schemas/download-schemas.sh`'s hardcoded
  `../../../../modules/dpe/api-oai/...` and left nowhere for that path to go,
  because the schema fixture genuinely needs the committed XSDs. Parameterising
  is what gives it somewhere, which is devops's point.

So: the script now takes the XSD include directory as a **required positional
argument**, matching `shared-metadata`'s `load_from(data_dir)` precedent and
`shared/README.md`'s own rule, and fails with a usage message without it. It
resolves the argument to an absolute path before `cd`-ing to its own directory,
so the output still lands beside it and a relative argument is read from the
caller's working directory. The invocation moved to the **repo-root `justfile`**
as `just refresh-datacite-schema`, which passes
`modules/dpe/api-oai/src/handlers/testdata/schemas/include`. That is the answer
to dune's objection that parameterising merely relocates the literal: it
relocates it out of the shared crate, into the one file that legitimately knows
the module layout.

`SHARED_PATHSPECS` is now `('shared/*/src/*.rs' 'shared/*/testdata/**')`. The
gate's own header had invited exactly this — "No shared crate has a tests/
directory yet; widen this when one does" — and Phase 3 created the trigger.

The widening also matched the script's **prose**, which named the sibling OAI
script, the XSD include directory and `corpus.rs` by full relative path. Those
were reworded to name the same things without a slash-bearing module path,
rather than carving a comment exception into the gate: the gate stays a dumb,
auditable grep, which is the whole reason it is trustworthy.

The gate test gained a seventh case, a violation under `shared/*/testdata/`, so
the new pathspec is exercised rather than merely declared. `SHARED_PATHSPECS`
must stay a quoted array for it to recurse, and an untested pathspec is how that
bug was caught the first time.

The three documents were requalified to what the gate actually enforces: "no
hardcoded path into a service module", over each shared crate's `src/` and
`testdata/`. ARCH-MAP's *Banned constructs* row gained "in a source file or a
test fixture", the supported alternative now names the root `justfile`, and its
enforcement cell names both pathspecs.

**Result: the gate exits 0 over 38 files, up from 34.**

### Fix 2 — the headline, and the only behaviour change in the round

`project_json_ld_handler` and `project_datacite_json_handler` were `async fn`
with no `.await` in them. The graph build — up to 27,026 `PartRef`s — and a
~3.5 MB `serde_json::to_string` ran synchronously on a runtime worker. The
security reviewer built the binary and measured it: under 40 concurrent requests
from one IP, **well inside the default burst of 60, so the limiter never
engages**, `/healthz` went from trivial to **697 ms** and an unrelated,
non-rate-limited landing page from 85 ms to 192 ms. A degraded `/healthz` is an
orchestrator restart waiting to happen.

The work now runs in `tokio::task::spawn_blocking`. The `Sync` comment the code
carried does not block this, and the comment was rewritten to say why:
`spawn_blocking` requires the closure to be `Send + 'static`, not its internals
to be `Sync`. `raw` is a `&'static` cache reference, `UrlLayout` is owned
`String`s, and `resolve_inputs()` and `ProjectGraph::build` both run **inside**
the closure, so no `ContributorLookup` ever exists across the await.

`representation` became an `async fn` holding `&AppState` across the await,
which is sound because axum already requires `AppState: Sync`. A `JoinError` —
a panic in the writer, or a runtime shutting down — answers `500` with a
`tracing::error!` rather than a second `expect`, so one bad document cannot take
its connection task down.

Fix 3 was done first, as the brief said, and it made this a much smaller change.

### Fix 3 — one `build_graph` helper

`resolve_inputs()` → `ResolveContext::new(...)` → `ProjectGraph::build(...)` was
written out verbatim three times, differing only in the records iterator.
`ProjectGraph::build` returns an owned value with no lifetime tied to the
context, so a private
`fn build_graph<'a>(raw: &ProjectRaw, records: impl IntoIterator<Item = &'a Record>) -> ProjectGraph`
collapses all three. The "built synchronously, never held across `.await`"
property moved onto the helper's doc comment rather than being lost — it is now
stated in the one place that makes it true.

### Fix 4 — the quietly-omitted zero

*Known residuals* named three causes and never explained **I2-01M**, although it
is a named Success Metric target, a required-passing acceptance item and the
condition gating Phase 4. Both explanations were already in this journal's
*Phase 4 decision* section and that text was used: F-UJI's default-namespace
list excludes schema.org and both Dublin Core namespaces — exactly what this page
emits — and strips them before either sub-test runs; and
`testSemanticNamespaceURIsAvailable` adds its status to the score while that
status is still false, so it earns zero regardless. The residual says plainly
that moving I2 needs a controlled-vocabulary link F-UJI's registry recognises,
which is new scope rather than a serialisation change.

Two unattributed zeros were attributed: **R1.3-02D** to the "no project-level
distribution" cause already named for F3-01M and A1-03D, and **F1-02D** to the
already-named "no DataCite registration", its sub-test being PID-registry
registration.

Per fix 10, the residual pins the version (3.5.0) and names the mechanism but
carries no `fuji_server/...` source paths, which rot across versions.

### Fix 6 — links no relative depth could fix

mdBook ships only `docs/src/`, so no relative link from inside it can reach
`docs/adr/` — `../../adr/` would be as broken as `../adr/`. The two files already
cite ADRs correctly four times as prose plus a backtick path, and all three links
now conform. Deleting the two link definitions alone would have left
`[ADR-0004]` and `[ADR-0005]` rendering as literal brackets in the body, so those
inline references were converted in the same edit.

### Fix 8 — the number Ops actually needs

`DPE_OAI_RATE_LIMIT_*` no longer claims to govern `/dpe/oai` alone: both
representation routes share the same per-IP bucket. And the paragraph now says
what "rate limited" does *not* mean here. The limiter bounds the request rate,
not the per-request size, and the uncapped JSON-LD for project 081C is ~3.5 MB
**measured, not estimated**. At the defaults (`per_second = 1`, `burst = 60`)
that is ≈210 MB of transient allocation and egress per IP per burst window,
settling to ~3.5 MB/s/IP.

### Fix 9 — three subsets of one list

All five handler tests exist, but ARCH-MAP's *Conventions* named four (omitting
`an_unknown_shortcode_never_redirects_whatever_it_was_asked_for`), ADR-0005 named
all five, and *Banned constructs* named a different four. The ADR is canonical,
so *Conventions* now cites the same five and says so. *Banned constructs* keeps
its narrower four — its clause is narrower — and now says explicitly that it is
the subset bearing on that clause, pointing at *Conventions* for ADR-0005's full
list.

### Fix 11 — the five that changed code or the recipe

- `a_harvester_asking_for_a_representation_is_redirected_to_it` hand-copied the
  `(accept, suffix)` pairs its four siblings iterate. It iterates
  `REPRESENTATIONS` now, so the module's own promise that "adding a row here does
  both at once" holds for the test that checks it.
- The `text/plain` rationale named a risk that is not real: `project_json_handler`'s
  bare `StatusCode` never reaches the HTML shell either — that is the `ServeDir`
  fallback. Rewritten to the real reason, which is giving a machine client an
  unambiguous content type. The same false claim in
  `machine-readable-metadata.md` was corrected with it; it is one claim in two
  places.
- `negotiate.rs`'s `q` parse silently took the last of a repeated `q`. That is
  now a written rule in the module's rule list and a comment at the assignment,
  with a test pinning both directions — `q=0.1;q=0.9` wins and `q=0.9;q=0.1`
  loses, which proves "last wins" rather than "highest wins".
- `Redirect::to(&url)` **panics** on a control character, while `link_header`
  warns and drops the same class of failure. `validate_public_base_url` does not
  close the gap: it checks scheme, host and the three path characters, so a
  control character in the configured origin passes startup. `LandingPage::Redirect`
  now carries a `HeaderValue`, built in `landing_page` beside `link_header`'s
  own `from_str` with the same warn, falling through to the page on `Err`;
  `main.rs` assembles `303` + `Location` + `Vary` and can no longer panic. One
  test pins it.
- The `justfile`'s 180-second port wait stays — devops confirmed it is correct
  and documented, F-UJI rebuilding its re3data table before listening. Its nit is
  fixed: the loop's success now carries through in a flag instead of being
  re-probed, so a transient blip after a successful wait can no longer be
  reported as "F-UJI did not answer".

### Fix 10 — what was cut

The `justfile`'s `:latest` narration lost the raw Chromium error path and keeps
the durable constraint. The decision table in `machine-readable-metadata.md`
stays (it is the acceptance spec) and the pointer to `negotiate` stays; the prose
restatement of the same rules is gone, so there is one source of truth. The
F-UJI merge-order constraint keeps its sentence and loses its three
`fuji_server/...` paths.

`describedby` ⟺ negotiable was wrong in five places, not two, and tightening only
some of them would have left the wrong reading available. All five now say the
same thing — every candidate is a `describedby` target, the converse does not
hold, and the two OAI-record links are deliberately never candidates:
`negotiate.rs`'s module doc, `UrlLayout::candidates`'s doc, the `REPRESENTATIONS`
table comment, the inline comment in `landing_page`, and the prose in
`machine-readable-metadata.md`.

Two test doc comments in `negotiate.rs` lost their phase numbers and the F-UJI
name, and say the durable fact instead.

### Fix 7 — and one comment that could not land where it went stale

`ARCH-MAP.md`'s `dpe-server` key entities still listed `head_extras_for_project`;
it now lists `landing_page` / `LandingPage` / `render`. `HeadExtras` is still
real and stayed. `HAS_PART_CAP`'s comment now says the standalone representation
*does* serve the list uncapped.

`signposting.rs:35` ("Empty until the representation routes exist") is the
exception. It was introduced in `aeda6c6a`, a **Phase 2** commit below this
round's base, and it goes stale at `9110af80`, which is `feat(dpe-server)`-scoped
and may not touch `shared/fair`. The reword landed in `ffa2fc65` instead — the
last `shared-fair`-scoped commit before the field is filled — and is phrased to
be true at every commit: filled by the consuming service from its own route
table, empty for a consumer that serves none, as the OAI writers do.

### Fix 12 — Phase 4

Ticked as skipped with the real reason, not the plan's suggested wording. The
plan's literal condition is **not** met (I2-01M is 0/1) and the phase is skipped
anyway, because Turtle cannot move I2 by construction: the obstacle is which
vocabulary namespaces appear, not how they are serialised, verified twice from
the pinned image. The full argument is in the plan's Phase 4 section and in this
journal's *Phase 4 decision*.

The Success Metrics table now records that the I2 row (target 1/1) is missed at
0/1 and is not reachable by anything in this plan, as a follow-up rather than a
silent miss. Every other F-UJI row is at or above target and the total is 14/24
against a target of 12.

### Byte identity

Re-proven on the finished tip, because fixes 2 and 3 touch the graph-build path.
The Phase 1 hash test was recovered from `4cdf06d8^` — the commit that removed
it, whose parent is where it lives — trimmed to `oai_output_matches_hash_baseline`
and its helpers, added as `metadata/hash_check.rs` with
`#[cfg(test)] mod hash_check;`:

```
compared 102158 entries
took 6.8s
test metadata::hash_check::oai_output_matches_hash_baseline ... ok
```

All 102,158 entries match `.claude/tmp/oai-baseline-hashes.txt`, which was
**not** regenerated. Both edits were removed afterwards.

### Decided not to fix

Five items, each with the reason the brief gave, recorded so the next reader does
not re-raise them:

- **`docs(docs)` rather than `docs(dpe-server)` on the documentation commit.** It
  is one atomic cross-cutting "the decision landed" update, and `docs` is a valid
  scope.
- **`JSON_LD = 0` / `DATACITE_JSON = 1` positional indices.** Brittle, but the
  content-type tests catch a reorder.
- **The five test names living in three documents.** A rename hazard, noted; not
  worth a mechanism.
- **Retaining the raw F-UJI JSON.** Optional in the plan.
- **A test driving the real `GovernorLayer` to a 429.** Deferred to
  `tower_governor`'s own suite by design; re-testing a third-party timing
  algorithm here would be brittle and slow. Fix 8 documents the number instead.

### Gate

`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' 7fe29df2`
— 14 exec steps over 7 commits, all green, every SHA preserved through the gate,
so the byte-identity run above was on the final trees. (The message-only pass
below moved those SHAs afterwards without changing a tree.) `env -u GIT_DIR` remains
load-bearing; see the `check-commit-count.test.sh` side finding.

The fixes were staged as five `git commit --fixup` commits and folded in with one
`GIT_SEQUENCE_EDITOR=true git rebase -i --autosquash 7fe29df2`, which applied
cleanly with no conflicts. `metadata.rs` carries hunks belonging to two different
commits, so it was split into two fixups written in order — the second against
the tree the first produces — rather than staged by hunk, which `git add -p`
cannot do unattended.

`just commit-lint`: messages all pass. The commit-count gate reports 38 > 1, as
it has all run; that is what the PR's `allow-many-commits` box is for.

`git status --porcelain` shows exactly the two spec files. All commits remain
local — `origin/worktree-fair-assessment` is still at `f1532fac`.

### The commit messages were reworded, and why they had to be

This repository rebase-merges, so every commit body lands on `main` verbatim.
Three of the five rewritten commits carried a claim this round had just made
false, and a fourth described none of what was folded into it. Autosquash
preserves messages, so this was a second pass:

- `serve the machine-readable representations` repeated the error rationale fix
  11 corrects ("an HTML error page it cannot parse is worse than nothing"), said
  the table feeds "the `describedby` links" without the representation
  qualification fix 10 adds, and described none of the `spawn_blocking` change
  or the `build_graph` helper.
- `decide the landing page's one negotiation step` said the candidates are built
  from the same pairs as the `describedby` links "so a page cannot advertise a
  representation it would not redirect to" — the reversed reading fix 10 exists
  to close. It also enumerated the `q` rules without the repeated-`q` rule.
- `describe the representations…` carried the unqualified "`shared-fair` holding
  no path into an area", which is the sentence fix 1 requalified three documents
  to stop saying.
- `add just fair-check` described none of fix 1 — the parameterised script, the
  widened gate, the new recipe — although that is now most of its diff. It also
  kept the raw Chromium narration fix 10 trimmed from the recipe itself.
- `redirect the landing page…` gained a sentence on the `HeaderValue` fallback,
  which is a real behaviour in it that the body did not mention.

Done as a message-only `git rebase -i` with a `GIT_EDITOR` that maps each subject
to a prepared body. Every commit in the range was re-committed, so all seven SHAs
moved even where the message did not change, and `7cb164cd` → `c7c423df` and
`e4a7bc15` → `c3f920e8` carry their original messages untouched. **No tree
changed**: `git diff <old> <new>` is empty for all seven pairs, so the gate above
still stands on exactly these trees and was not re-run. `just check` and
`just test` were re-run at the tip anyway, after one further amend.

| Was (Round 8) | After the fixes | After the reword |
|---------------|-----------------|------------------|
| `7cb164cd` | `7cb164cd` | `c7c423df` |
| `e4a7bc15` | `e4a7bc15` | `c3f920e8` |
| `e06ffacd` | `ffa2fc65` | `ce56115b` |
| `9775b5da` | `9110af80` | `9df1c3de` |
| `8b06ad73` | `e7e80ca3` | `d700fc20` |
| `d7b399a3` | `9e2c9cb2` | `320c9d3d` |
| `677b1ae3` | `1717ee7f` | `62d7b861` |

### Two checks beyond the brief

- **Fix 8's defect had a second home.** `oai-pmh.md` said "the limiter is scoped
  to `/dpe/oai` alone" where it explains that the record-file endpoint is *not*
  limited. The conclusion is still right and the reason was not, so both that
  line and the *Rate limiting* section now name the shared bucket. Amended into
  the documentation commit, `62d7b861`, and `just check` / `just test` re-run at
  the tip afterwards.

- **`just refresh-datacite-schema` was run end to end**, not only checked for its
  usage message. It re-downloaded and re-patched the fixture and
  `git status --porcelain -- shared/fair/testdata/schemas/` came back **empty**:
  the parameterised script reproduces the committed schema byte-for-byte. That is
  a stronger claim than fix 1 asked for — it proves the argument-resolution
  order is right, since `$1` is resolved against the caller's working directory
  before the script `cd`s to its own.

## Closeout

status: **complete** — Phases 0–3 landed, Phase 4 skipped on evidence (see the
Phase 4 decision section). 35 commits on `worktree-fair-assessment` from
`f1532fac`. Ten reviewers ran on each of Phases 1, 2 and 3; nine on Phase 0.

- **root_cause**: the metadata was never missing — it already existed and was
  already mapped to DataCite kernel 4 for OAI-PMH, with creators, ORCIDs,
  publisher, year, subjects, SPDX rights and funding. What was missing was
  *exposure*: the page an ARK resolves to served HTML with no JSON-LD, no meta
  tags, no `Link` header, and the same bytes for every `Accept` value. Every
  F-UJI failure downstream of F1 followed from that single gap, which is why a
  repository with good metadata scored 3 of 24.

- **investigation**: the work was exposure, not modelling, so the risk was never
  "can we produce the fields" but "will the representations agree, and will the
  move change what OAI already serves". Three things shaped the run.
  First, a hash baseline over 102,158 OAI payloads pinned byte-identity through
  the extraction and was re-proven at every subsequent round; it never diverged,
  which retired the plan's largest stated risk early. Second, the reviewers
  repeatedly found defects that every mechanical gate passed: a fabricated
  `datePublished` of `"2015"` that was not a placeholder sentinel and so slipped
  through the placeholder assertions; an `extract_year` that byte-sliced
  `date[..4]`; a char-boundary test whose two-byte filler meant byte 1000 was
  still a boundary, so it never exercised the panic it claimed to; and
  CPU-bound work inside `async fn` handlers that degraded `/healthz` to 697 ms
  under requests the rate limiter was designed to allow. None of these were
  reachable by `just check` or `just test`.
  Third, documentation drifted from code in both directions and needed the same
  scrutiny as code: a boundary rule that was literally false, a `PreEscaped`
  site count wrong in four places, an enforcement claim flipped to
  "static-analysis" while the gate could not see the violating file, and four
  commit bodies asserting things the fixes had just made untrue.

- **solution**: `modules/platform/` became `shared/` (ADR-0002's shared-root
  half). A new `shared/fair` crate holds `ProjectGraph`/`RecordGraph`, their
  builders over the contract types, and every writer: the four moved DataCite
  and Dublin Core mappings plus schema.org JSON-LD, Dublin Core meta tags, the
  Signposting link set, DataCite JSON and the `Accept` decision function. One
  resolved graph feeds them all, so the representations cannot disagree.
  Format-mandated fallbacks that are not facts about the object —
  `creators_with_fallback()`, `publication_year_with_fallback()` — are resolved
  once on the graph and called only by the writers whose vocabulary requires
  them, leaving Dublin Core and schema.org free to omit what was never recorded.
  `dpe-server` renders the head extras and the `Link` header, serves two
  representation URLs behind the OAI rate limiter, and answers `303 See Other`
  when `Accept` prefers one — the single negotiation step ADR-0005 carves out of
  ADR-0004, with `Vary: Accept` on every landing-page answer.
  Measured result for project 0862: **F-UJI 14 of 24, from a baseline of 3**,
  against a target of ≥ 12.

- **prevention**: the corpus-wide agreement test asserts title, ARK, licences,
  creator names and ORCIDs agree across JSON-LD, Dublin Core meta, the link set
  and DataCite for every committed project, and pins the embedded block under
  64 KB. `check-shared-paths.sh` now also scans `shared/*/testdata/**`, so a
  fixture or refresh script cannot reintroduce a path into a service module.
  A crate-wide grep pins `PreEscaped(` to exactly one site in `dpe-server`.
  `just fair-check` scores a landing page against a digest-pinned F-UJI and is a
  `REVIEW.md` step, so FAIRness stays measured rather than asserted.
  Patterns worth carrying: resolve once on the graph and let each writer hold
  its own vocabulary; keep format-mandated fallbacks out of the raw fact;
  `git rebase --exec` at each phase boundary, which caught a red commit the
  per-phase gates missed. Anti-patterns worth naming: a test whose fixture
  cannot reach the failure it claims to cover (multi-byte is not enough — the
  character width must be coprime with the limit); an enforcement claim whose
  gate cannot see the file it governs; and a commit body describing behaviour a
  later fix removed, which on a rebase-merge repo lands on `main` verbatim.

## Round 11 — HANDOFF: fixes found by the first live assessment (2026-09-18)

**Status: executed — see *Round 12* below.** This section exists so a fresh
session can continue cold. The work is written up as **Phase 5** in the plan file; run
`/eng:workflows:work-orchestrate docs/specs/2026-09-15-fair-metadata-exposure/01-feat-fair-metadata-exposure-plan.md`
and execute that phase. Everything below is the evidence and the state.

### Where the branch stands

- Branch `worktree-fair-assessment`, HEAD **`6f53d3c7`**, **pushed**.
- PR **#391** (`dasch-swiss/dsp-repository`) is open, **draft**, retitled
  `feat(dpe): machine-readable project metadata for FAIR assessment`, body
  rewritten, `allow-many-commits` ticked. Base is `docs/adr-areas-and-shared-root`
  (PR #399), which must merge first.
- CI on #391 at `6f53d3c7`: **20 pass, 4 skipping, 0 pending, 0 fail.**
- 36 commits from `f1532fac`: Phase 0 (2), Phase 1 (16), Phase 2 (10),
  Phase 3 (7), plus one `docs(docs)` commit carrying plan + journal.
- Phases 0–3 complete and reviewed (ten reviewers each on 1, 2 and 3; nine on 0).
  Phase 4 skipped on evidence — see the *Phase 4 decision* section above.
- **Everything at or below `6f53d3c7` is pushed.** Phase 5 commits go *on top*.
  Do not amend or rebase anything at or below it.
- A companion PR is open in ops-deploy: **dasch-swiss/ops-deploy#1435** (draft),
  one entry setting `DPE_PUBLIC_BASE_URL` to `https://repository.{{ DEPLOY_DOMAIN }}`
  for dev/stage/prod. That is human action H1. It does **not** cover PR previews,
  which is why Phase 5 has its own workflow fix.

### What prompted Phase 5

The user deployed the PR preview and pointed both assessors at
`https://dpe-pr-391-pbjdzenira-oa.a.run.app/dpe/projects/0862`:

- **F-UJI 3.5.0: 13 of 24.** The local Phase 3 run scored 14 of 24.
- **FAIR Champion 1.1.11: 7 of 15 passing** (LicenseWeak, QualifiedRefs,
  MachineSyntax, MachineSemantic, MetadataIdentifierUniqe, MetadataAuthentication,
  MetadataOpenProtocol). Baseline was 6 of 15 with 2 hollow passes; the plan
  targets ≥ 10.

Three causes, separated:

**1. Missing `DPE_PUBLIC_BASE_URL` on the preview — worth exactly 1 F-UJI point.**
It is the entire difference between 13 and 14. DPE fell back to its compiled
default of `https://repository.dasch.swiss`, so the typed links and the `303`
target pointed at production, which does not carry this code. F-UJI's log:

```
Found e.g. Typed Links in HTML Header linking to RDF Metadata
  -: (application/ld+json https://repository.dasch.swiss/dpe/projects/0862/metadata.jsonld)
WARN: Request failed, status code -: …/metadata.jsonld - 404
```

That fails `I1-01M-2` ("graph data accessible through content negotiation, typed
links or sparql endpoint"), so I1 scored 1/2 instead of 2/2. Fixed by the
workflow change in Phase 5.

**2. `license` is emitted as a string, so it is an RDF literal, not a Resource.**
FAIR Champion *LicenseStrong*:

```
WARN: Found the Schema license predicate, but it does not have a Resource as its
      value. … Please update your metadata to point to a URL containing the license.
FAILURE: No correctly structured license property was found in the metadata.
```

Verified in source: `shared/fair/src/schema_org.rs` line ~61 does
`insert_list(&mut root, "license", graph.license_uris())`, and the unit test at
~line 389 asserts a bare string. schema.org's remote context does not coerce
`license` to `@id`, so nothing rescues it downstream. *LicenseStrong* is a named
Success Metric target (baseline fail → target pass).

**3. `identifier` carries only the ARK.** FAIR Champion *MetadataIdentifierFound*:

```
FAILURE: While (apparent) metadata record identifiers were found
  (["https://ark.dasch.swiss/ark:/72163/1/0862", …]) none of them matched the
  initial GUID provided to the test (https://dpe-pr-391-…/dpe/projects/0862).
  Exact identifier match is required.
```

The test reads `schema:identifier` only — it does not consider `url`, which does
already carry the landing page. Also a named Success Metric target.

### What is *not* a defect

- **F-UJI `F1-02D`** (persistent identifier) failed with `Landing page domain
  resolved from PID found in metadata does not match with input URL domain
  -: run.app <> dasch.swiss`. The ARK correctly resolves to production; the test
  was pointed at a preview host. Same cause for FAIR Champion's
  `MetadataIdentifierPersistent` (indeterminate). Artifacts of the target, not
  bugs. They would behave differently against the ARK or the production URL.
- **F3-01M, A1-03D, DataIdentifierFound, R1-01MD, R1.3-02D** — all downstream of
  "no project-level data pointer", already a documented residual. ADR-0005 says
  nothing is invented for a score; record landing pages are the follow-up.
- **I2-01M** — vocabulary, not syntax. Unreachable by anything in this plan; see
  the *Phase 4 decision* section for the two independent proofs.
- **F4-01M-2, DiscoverableInBing, MetadataPersistence** — DataCite registration,
  search indexing, and H3's persistence-policy URL. Out of reach by design.

### A rejected alternative, recorded so it is not re-proposed

Making DPE derive its base URL from the request `Host` header would "fix" the
preview without a workflow change. **Do not do this.** It would make `@id` and
`Link` vary by how the page was reached and would put an attacker-influenceable
value into an identifier — precisely what the canonical-shortcode rule exists to
prevent, and what the Phase 2 security review verified. The workflow change is
the correct fix.

### Carry-forwards a fresh session needs

- **Bash guard**: this worktree's path contains `_github.com`, so any compound
  command that also names git is rejected. One plain command per call. A heredoc
  whose *text* mentions git is rejected too — write such content with the Write
  or Edit tool instead.
- **`env -u GIT_DIR` on every `git rebase --exec`.** `check-commit-count.test.sh`'s
  `make_repo` does not clear `GIT_DIR`, so under rebase it writes into the real
  repository — it already polluted `.git/config` and created an orphan `feature`
  branch once this run. Cleaned up; identity now resolves from `~/.gitconfig`.
- **Never `git stash`** — the stack is shared across worktrees and sessions. Copy
  the two spec files to a scratch dir, restore the tracked one, remove the
  untracked journal, rebase, copy back.
- **Never stage or commit the plan file or the journal** during a round; the
  session commits them at ship time. Stage code explicitly by path.
- `git add` silently refuses paths under an `icon/` directory (macOS `Icon` rule
  plus `core.ignorecase`); `git add -u <path>` works.
- **Known `editor-server` flakes**, untouched by this work:
  `test_simultaneous_wrong_guesses_cannot_outrun_the_three_strike_limit`, and
  `db::tests::test_file_database_uses_wal_and_creates_its_siblings_in_the_directory`
  / `db::schema::tests::test_reopening_a_file_database_keeps_its_data_and_does_not_re_migrate`
  (these two hung a gate for ten minutes, then passed in 0.01s in isolation).
- **`just fair-check`** needs Docker; on this machine it sets `DOCKER_CONFIG` to a
  scratch dir itself because `~/.docker/config.json` names a `desktop` credential
  helper that is not installed under colima. The F-UJI image is pinned to 3.5.0,
  `sha256:3cde9d30bc148798a512b9e3a8a9ee6e63c4d09a6a33bb7651ac007c5824c687`.
  **Do not** use `:latest` or the 4.0.0 digest — 4.0.0's published image does not
  start (it launches a headless Chromium that is not installed in it).
- The **OAI hash baseline** at `.claude/tmp/oai-baseline-hashes.txt` (102,158
  entries, gitignored) is still valid and must not be regenerated. Recover the
  hash test from history when a round needs to re-prove byte identity, run it,
  confirm `compared 102158 entries`, then remove it. A skip is not a pass.

### Still open after Phase 5

- Re-assess a preview that carries the Phase 5 fixes, and record the run in the
  *Assessment results* table. Until then, no new score may be claimed anywhere.
- Mark PR #391 ready for review (deliberately left draft — that notifies the team
  and is the user's call).
- H1: merge ops-deploy#1435. H2: FAIR Champion against DEV. H3: persistence policy.
- The follow-ups listed in the PR body, unchanged: the five `0801*` projects
  sharing one ARK PID, `check-commit-count.test.sh`'s `GIT_DIR` mutation, the
  `oai_datacite` golden tests pinning degraded output, I2-01M's unreachability,
  the two `editor-server` flakes, and whether `/dpe/oai` shares the synchronous
  handler shape fixed for the representation routes.

## Round 12 — Phase 5 (2026-09-18)

The three things the first live assessment found: two defects in what the JSON-LD
asserts, and one deployment gap. Four commits on top of `f3d3ae9f`, which is
pushed, so nothing at or below it was amended or rebased.

| # | Chunk | Plan checkboxes | Commit | Notes |
|---|-------|-----------------|--------|-------|
| 5.1 | `license` as an IRI node; corpus test reads `@id` | 1, 2 | `a8bdad36` | `fix(shared-fair,dpe-api-oai)`; adopted from the working tree, verified, completed |
| 5.2 | `identifier` gains the landing page URL; corpus test picks the ARK by `@type` | 3, 4 | `1a4786af` | `fix(shared-fair,dpe-api-oai)`; plus the F-UJI regression gate below |
| 5.3 | Cloud Run preview sets `DPE_PUBLIC_BASE_URL` to its own URL | 5, 6 | `378d7dea` | `chore(ci)`; checkbox 6 is a prohibition and was honoured, not implemented |
| 5.4 | `machine-readable-metadata.md`, `operations.md`; plan Success Metrics | 8, 9, 10 | `03585c0d` | `docs(docs)`; the plan-file edit is in the working tree, unstaged, as every round |
| 5.5 | Editor workflow investigation | 7 | — | journal-recorded below; no code change, by instruction |
| 5.6 | OAI byte identity; standing gate | 11, 12 | — | both green, see below |
| 5.7 | Residuals ledger: all ten missing F-UJI points, in five causes | 8 (extended) | `1d9403f8` | `docs(docs)`; new evidence, added after 5.4 had landed — see below |

Checkbox 13 (`eng:reviewing` with the complete reviewer set) is left unticked:
the interactive session owns reviews, as in every prior phase.

### The F-UJI regression guard on the `identifier` array

The plan asserts F-UJI reads the `PropertyValue` form but never verifies it reads
it *out of a list*. After 5.2, `identifier` is
`[PropertyValue, "…/dpe/projects/0862"]`. If F-UJI 3.5.0 indexed
`identifier.value` on the object directly, F1 would have regressed silently.

Checked in the pinned image first, before spending a run.
`fuji_server/helper/metadata_collector_rdf.py:523-538` is an rdflib graph
traversal, not a JSON path:

```python
for identifier in (
    list(g.objects(item, DC.identifier)) + … + list(g.objects(item, SDO.identifier))
    + … + list(g.objects(item, SMA.url)) + list(g.objects(item, SDO.url))
):
    idvalue = g.value(identifier, SDO.value) or g.value(identifier, SMA.value)
    if idvalue:
        identifier = idvalue
    meta["object_identifier"].append(str(identifier))
```

It iterates *every* object of `schema:identifier` and substitutes `schema:value`
where the object is a node, so a list costs nothing. Note also that it already
folded `SDO.url` into `object_identifier` — F-UJI had the landing page all along;
it is FAIR Champion that reads `schema:identifier` alone.

Then the empirical gate. `just fair-check` against
`http://host.docker.internal:4000/dpe/projects/0862`, local `dpe-server serve`
started with `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000` and
`DPE_SITE_ADDR=0.0.0.0:4000`, F-UJI 3.5.0 at the pinned digest. Every per-metric
score is identical to the 2026-09-18 local row:

```
F1-01D 1/1 pass   F1-02D 0/1 fail   F2-01M 2/2 pass   F3-01M 0/1 fail
F4-01M 1/2 pass   A1-01M 1/1 pass   A1-02M 1/1 pass   A1-03D 0/1 fail
I1-01M 2/2 pass   I2-01M 0/1 pass   I3-01M 1/1 pass   R1-01MD 1/4 pass
R1.1-01M 2/2 pass R1.2-01M 1/2 pass R1.3-01M 1/1 pass R1.3-02D 0/1 fail
```

**F1 passes. No regression.** The total is unchanged; it is recorded here only
as the gate's evidence, and no new score is claimed anywhere in the
documentation — the plan's rule holds until a preview carrying these fixes is
assessed.

An identical total is a weak signal on its own, since F1-01D passed at the
3-of-24 baseline with no JSON-LD at all. The discriminating evidence is in the
run's `test_debug`, which only appears when the ARK was extracted *from
metadata*:

```
WARNING: Landing page domain resolved from PID found in metadata does not match
         with input URL domain -:internal. <> dasch.swiss
```

and F-UJI's collected `object_identifier`, which now carries both:

```
["https://ark.dasch.swiss/ark:/72163/1/0862",
 "http://host.docker.internal:4000/dpe/projects/0862"]
```

The licence change was confirmed the same way. R1.1 stays 2/2 and the debug log
still reads `Found CreativeCommons license -: https://creativecommons.org/licenses/by-sa/4.0/`
and `SUCCESS: Found SPDX license representation (spdx url, osi_approved)`, so the
node object parses as the licence resource rather than costing the SPDX match.

### Checkbox 7 — does `cloud-run-editor-pull-request.yml` have the same shape?

**Structurally yes, materially no. No change is needed there, in this phase or a
later one.**

The editor preview workflow has the same skeleton: a `Deploy to Cloud Run` step
with `id: deploy`, and a comment step reading `${{ steps.deploy.outputs.url }}`.
What it does not have is anything to point at itself with. `editor-server` has no
public-base-URL setting: `EDITOR_*` covers the listen address, data and public
directories, the database, SMTP and the login limits, and nothing else. Grepping
all of `modules/editor` for `BASE_URL`, `base_url` and `format!("http` finds
nothing — the editor builds no absolute URL anywhere. Its login flow mails a
*code*, not a link, so it never needs to name its own origin.

The gap fixed in 5.3 is specific to a service that publishes absolute
identifiers. The answer is carried in the orchestration report for the session to
put in the PR body at ship, as the plan asks; the orchestrator does not edit it.

### Byte identity

The plan's claim is that neither JSON-LD fix touches DataCite or Dublin Core.
Proven, not assumed.

The hash test was recovered from `4cdf06d8^` (the parent of the commit that
removed it), trimmed to `oai_output_matches_hash_baseline` and its helpers, and
added as `modules/dpe/api-oai/src/metadata/hash_check.rs` with a
`#[cfg(test)] mod hash_check;` line. Run with `-- --ignored`, since it is
`#[ignore]`d for setting the process-global data dir:

```
compared 102158 entries
took 6.6s
test metadata::hash_check::oai_output_matches_hash_baseline ... ok
```

Both edits were then removed; the working tree is clean of them. The baseline at
`.claude/tmp/oai-baseline-hashes.txt` was **not** regenerated.

### Standing gate

`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' 6f53d3c7`
— green on every pick above `6f53d3c7`, `Successfully rebased and updated`. Run
three times: at five picks, after chunk 5.7 added a sixth, and once more on the
final tip `1d9403f8`.
`f3d3ae9f` and `6f53d3c7` still resolve to the same SHAs afterwards: an
interactive rebase fast-forwards unchanged picks, so the pushed history is
preserved rather than rewritten. The `env -u GIT_DIR` carry-forward did its job:
no orphan `feature` branch exists, and `git config --local --list` carries no
`user.*` entry, which is the write that polluted the repository last time.

`just commit-lint` reports all commit messages OK. Its commit-count check fails
at 44 over a maximum of 1, which is the expected reading for this branch: PR #391
ticks `allow-many-commits`, and the local recipe cannot see the checkbox.

The rebase needs a clean tree and the plan file is tracked as of `f3d3ae9f`, so
the carry-forward's "remove the untracked journal" is now stale. What works: copy
the plan file to a scratch dir, `git checkout --` it, rebase, copy it back. The
ticks survive intact.

### Side findings

- **The plan says "the property table" for `machine-readable-metadata.md`, but
  the schema.org section was prose and had no table.** The Signposting table's
  `license` row describes the `Link` header, which this phase does not change. A
  two-row table was added to the schema.org section instead of editing the wrong
  one, and the now-redundant sentence explaining the `PropertyValue` choice was
  folded into it.
- **`identifier` is written as an array literal, not through `insert_list`.** Its
  cardinality is fixed at two and does not depend on the graph, so
  `insert_list`'s collapse-to-scalar branch would be dead code that hides the
  invariant. `license` still goes through `insert_list`, where the cardinality is
  genuinely the graph's.
- **The corpus assertion picks the ARK entry by `@type`, not by index.** Pinning
  position would make a reordering of the two entries a test failure for no
  reason. `test_layout()` is now bound once in that function so the landing-page
  assertion compares against the same layout the writer was given.
- **The top-level key-order test is unaffected.** It asserts the first five
  *keys*, and `identifier` is still one key.

### Chunk 5.7 — the residuals ledger (added after 5.4 had landed)

New evidence, not a re-derivation. The user supplied the full F-UJI 3.5.0 result
JSON for the 2026-09-18 preview run, taken with `test_debug: true`. Sub-test
detail had never been recorded in this run: it is the first time anyone could see
*which half* of a metric failed. *Known residuals* accounted for six of the ten
missing points; it now accounts for all ten, in five causes rather than four, in
a table a reader can add up.

Landed as its own commit rather than folded into `03585c0d`, which had already
been made.

| Cause | Where the points go | Points |
|-------|---------------------|--------|
| No project-level data pointer | F3-01M, A1-03D, R1-01MD-2 to -4, R1.3-02D | 6 |
| No formal provenance vocabulary | R1.2-01M-2 | 1 |
| Vocabularies absent from F-UJI's LOD registry | I2-01M | 1 |
| ARKs are not registered with DataCite | F4-01M-2 | 1 |
| The assessment ran against a non-production host | F1-02D | 1 |

Three things the doc did not previously say, and one it said wrongly.

**R1-01MD's three lost points were never attributed.** They belong to the
"no project-level data pointer" cause already named for F3-01M and A1-03D, which
makes that cause worth six points rather than three. R1-01MD-1 passes on the
resource type alone; R1-01MD-2 does run and returns an empty
`data_content_descriptor`; only -3 and -4 cannot run at all.

**R1.2-01M-2 was missing entirely.** F-UJI maps `publication_date` to
`prov:generatedAtTime` and the agent fields to `prov:wasAttributedTo`, so
R1.2-01M-1 passes on facts this page already carries; -2 wants those facts in a
formal provenance ontology and reports `Formal provenance metadata is
unavailable`. Recorded as new scope for a later plan, not as something declined.

**F1-02D is not a DataCite-registration failure, and Round 10's fix 4 attributed
it to that cause in error.** The preview JSON shows F-UJI harvesting the ARK from
the metadata twice — once through the Signposting `cite-as` link — confirming the
`ark` syntax and resolving it successfully:

```json
"persistent_identifiers": [{ "pid": "http://ark.dasch.swiss/ark:/72163/1/0862",
  "pid_scheme": "ark", "resolved_url": "https://repository.dasch.swiss/dpe/projects/0862",
  "resolvable_status": true }]
```

and then discarding it on a domain comparison:

```
WARNING: Landing page domain resolved from PID found in metadata does not match
         with input URL domain -: run.app <> dasch.swiss
WARNING: PID syntax is OK but the PID seems to resolve to a different entity,
         will not use this PID for content negotiation
INFO:    Could not find any persistent identifier for metadata which complies with
         a known PID syntax
```

The PID was found, well-formed and resolvable. It was rejected because the domain
it resolves to is not the domain the assessment ran against. The corollary is not
"assess the ARK" but "assess from a host under `dasch.swiss`", which is exactly
what post-merge verification step 1 does against DEV.

The hedge needed one turn of precision beyond the brief's wording. "F1-02D is
unmeasured from a `dasch.swiss` host" is not quite true: the 2026-09-15 baseline
*did* target the ARK, and its row reports `F 2/7` with "only F1 and A1-02M
passed", which is ambiguous about F1-02D and is not worth resolving by guessing.
What is unambiguous is that no run against a build carrying this work has been
made from such a host, so that is what the doc says. No pass is predicted either
way.

**The I2-01M residual's second mechanism was wrong and is dropped.** Round 10's
fix 4 recorded two mechanisms. The first is confirmed verbatim by the JSON:
`Removing default namespaces from 26 vocabulary namespaces found in structured
metadata`, with `http://schema.org` and `http://purl.org/dc/elements/1.1` among
the excluded list. But "so there is nothing left to score" does not follow —
seventeen namespaces survive the strip and *are* checked, including OAI-DC, the
DaSCH URLs, geonames, ORCID, the DataCite kernel-4 schema and creativecommons.
And the second mechanism, that `testSemanticNamespaceURIsAvailable` adds a false
status to the score and so earns zero whatever it found, is not what the output
shows: `I2-01M-1` is explicitly unscored (`metric_test_score {total: 0,
earned: 0}`, "This test is not scored") and `I2-01M-2` is a plain `total: 1,
earned: 0`. The real failure is registry membership — `NO known vocabulary
namespace URI is found which is listed in the LOD registry`. The conclusion is
unchanged: moving I2 needs a controlled-vocabulary link F-UJI's registry
recognises. Round 10's text is left as written, since this journal is the record
of what was believed when; this section supersedes it.

**Which run the evidence comes from.** The per-metric totals in the ledger are
the local 14-of-24 run; the sub-test detail is the preview run of the same day.
They differ in `I1-01M-2` alone, which the preview lost to its missing base URL
and which this phase fixes, so the sub-test detail carries over to the ten points
unchanged. The doc says so rather than leaving a reader to assume one run.

**F4-01M-2 is a one-registry miss, not a three-registry one.** Of the registries
F-UJI consults, two never answered: `Mendeley Data API not available` (a DNS
resolution failure inside the container) and `Google Search Cache DB does not
exist, see F-UJI installation instructions`. DataCite was the only one actually
queried. Not in the doc — the conclusion there is the same either way — but worth
recording so a later reader does not read that sub-test as a clean sweep of three
registries. It also means the sub-test is partly a property of how the container
is provisioned.

### Side finding — `just fair-check` discards the only data that explains a score

The recipe prints `metric_identifier`, `score.earned/total` and `test_status`,
and throws the rest of the response away. Everything in this section — which
sub-test failed, the log line that says why, the resolved PID, the namespace
lists — was in the JSON the recipe already received and did not keep. Every
attribution in *Known residuals* had to come from a result set captured outside
the recipe. Worth a follow-up: keep the raw JSON, or print the sub-test rows.

### Status

Checkboxes 1–12 ticked. Checkbox 13 belongs to the session.

## Round 13 — Phase 6: provenance in PROV-O (2026-09-18)

Two commits: the documentation gaps Phase 5's ledger left, then the one code
change that earns `R1.2-01M-2`.

| # | Chunk | Plan checkboxes | Commit | Notes |
|---|-------|-----------------|--------|-------|
| 6.1 | The re-run row, ADR-0005 on the data-pointer cause, and its framing | Phase 5, 8 (extended) | `0de950ec` | `docs(docs)`; closes the `REVIEW.md:17` tension |
| 6.2 | PROV-O: context, `prov:wasAttributedTo`, agent `@id`s, tests, docs | Phase 6, 1–7 | `a35f1ba7` | `feat(shared-fair)`; measured, see below |
| 6.3 | OAI byte identity; standing gate | Phase 6, 4 and 8 | — | both green |

Phase 6 checkbox 9 (`eng:reviewing`) is the session's, as always.

### The sub-test is a set intersection, which is why this is one predicate

Read out of the pinned 3.5.0 image,
`fuji_server/evaluators/fair_evaluator_data_provenance.py`:

```python
provenance_namespaces = [
    "http://www.w3.org/ns/prov", "http://www.w3.org/ns/prov#",
    "http://purl.org/pav/", "http://purl.org/pav",
]
used_provenance_namespace = list(set(provenance_namespaces).intersection(set(self.fuji.namespace_uri)))
```

It never inspects the provenance statement. It asks only whether a PROV or PAV
URI is among the namespaces F-UJI collected, and those come from the *parsed
graph*, not from the raw `@context` — so a prefix declared and never used
expands away and does not register. The honest route and the only working route
are the same one: emit a real statement. That is why this is a one-predicate
change and not a provenance modelling exercise.

### What is asserted, and what is not

`prov:wasAttributedTo` over DaSCH and every credited creator with an ORCID.
Both halves restate what `creator` and `publisher` already say.

Agent nodes gained an `@id` — the ORCID, by Signposting's `author` rule, and the
publisher's URL — so the statement references those nodes rather than describing
second copies of them. That is worth having on its own: a consumer merging this
page with another can now see that two mentions of the same person are one node.

Two omissions, both deliberate:

- **`prov:generatedAtTime`.** F-UJI's own mapping treats `publication_date` that
  way, but the corpus records a publication *year*. This run already produced one
  fabricated-date defect — the `"2015"` `datePublished` — and one predicate that
  is unambiguously true beats two where one is arguable.
- **Creators without an ORCID.** No node to point at. Inlining a copy would put a
  *different* blank node in the graph and so assert a second, unidentified agent,
  which is the opposite of what attribution means. Naming some agents does not
  deny the others, so the shorter statement is still true. A unit test pins both
  the identified and the unidentified case.

On ADR-0005's "nothing is invented for a score": the rule bites on asserting what
the corpus does not record. Re-expressing a recorded fact in the W3C vocabulary
for provenance is interoperability. The reading held throughout implementation;
had it not, the instruction was to stop rather than proceed.

### Measured: 14 to 16, and the second metric was not sought

`just fair-check` against a local `dpe-server serve`, F-UJI 3.5.0 at the pinned
digest. **Exactly two metrics moved**, verified by diffing the full per-metric
list against the recorded local run:

- `FsF-R1.2-01M` **1/2 → 2/2** — `SUCCESS: Found use of dedicated provenance ontologies`
- `FsF-I2-01M` **0/1 → 1/1** — `SUCCESS: Namespace matches found -: ['http://www.w3.org/ns/prov']`

Every other metric's earned/total is unchanged, and **no metric's `test_status`
flipped** — `F1-02D`, `F3-01M`, `A1-03D` and `R1.3-02D` still report `fail`,
everything else still `pass`, including `I2-01M`, which reported `pass` at 0/1
before and reports `pass` at 1/1 now. Total **16 of 24**.

What was compared, precisely: the recipe's printed `metric_identifier`,
`earned/total` and `test_status` table for this run against the same table for
the 14-of-24 run. Not a diff of two saved JSON documents — the recipe discards
everything else, which is the side finding Round 12 recorded, and the earlier
run's captured JSON was overwritten by this one's.

The brief's acceptance said no other metric should move, so the I2 movement is a
deviation and is reported as one. It is not a second change, and it is not the
intersection being gamed. I2-01M failed on registry membership — `NO known
vocabulary namespace URI is found which is listed in the LOD registry` — and PROV
is both a provenance ontology and a vocabulary that registry lists. One true
statement satisfies both tests.

It is also exactly what the I2 residual predicted. That residual said moving I2
needs "a controlled-vocabulary link F-UJI's registry recognises", and that is
what happened. **Phase 4's decision is therefore confirmed, not contradicted**:
Turtle could not have earned I2, because the obstacle was never serialisation.
The residual's own diagnosis named the fix, and a phase two rounds later supplied
it for an unrelated reason.

Worth noting for a later reader: the namespace F-UJI collected is the slash form
`http://www.w3.org/ns/prov`, truncated at the `#`. Both forms are in the
intersection list, so either serialisation works.

### Byte identity

The hash test was recovered again from `4cdf06d8^`, with the write mode replaced
by a panic this time so a regenerated baseline cannot be mistaken for a pass, and
the absent-baseline skip replaced by a panic for the same reason.

```
compared 102158 entries
took 6.8s
```

Both edits removed afterwards. The baseline was not regenerated.

### Standing gate

`git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' 6f53d3c7`
green over every pick, with `f3d3ae9f` and `6f53d3c7` unchanged afterwards.

### Side finding — the residuals ledger is now a maintenance surface

Phase 5 built a table whose arithmetic has to close at the number of unearned
points. Phase 6 earned two of them, and the table, its lead-in sentence, two
cause paragraphs and the *Assessment results* note all had to move together or
the section would have contradicted itself. That is the cost of stating
arithmetic in prose, and it is worth paying — the alternative, six of ten points
explained and four silently missing, is what Phase 5 was called in to fix. But
anything that earns a point from here on has to edit the ledger in the same
commit.

## Round 14 — Phase 7 baselines, then HANDOFF (2026-09-18)

**Status: baselines measured and recorded; implementation not started.** Phase 7
surfaces record file pointers as `schema:distribution`. The brief requires a
baseline for 0868 and 0803 *before* any code, because neither has one and without
it no post-change movement is attributable. Those two runs are done and are below.
The orchestrator that ran them stopped there on context, so a fresh one starts
from this section with the measurement already banked.

One documentation commit landed first, `f50c162b`, which is Phase 5/6 cleanup
rather than Phase 7: it makes the data-pointer residual say which project it is
about, and narrows the ADR-0005 clause to "do not fabricate a download for a
project that has none". Deliberately interim. Phase 7 rewrites that cause with
measured numbers.

### Baselines — 2026-09-18, F-UJI 3.5.0, `sha256:3cde9d30bc14…`

Local `dpe-server serve`, `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000`,
against HEAD `f50c162b` — that is, *after* PROV-O and before any distribution
work.

| Project | Records | With a file | Total |
|---------|--------:|------------:|-------|
| 0862 `gotthelf` | — | 0 | 16 of 24 |
| 0868 `solec` | 19,770 | 7,716 | **16 of 24** |
| 0803 `incunabula` | 4,198 | 4,062 | **16 of 24** |

**All three score identically, metric for metric.** Every one of them:

```
F1-01D 1/1   F1-02D 0/1   F2-01M 2/2   F3-01M 0/1
F4-01M 1/2   A1-01M 1/1   A1-02M 1/1   A1-03D 0/1
I1-01M 2/2   I2-01M 1/1   I3-01M 1/1   R1-01MD 1/4
R1.1-01M 2/2 R1.2-01M 2/2 R1.3-01M 1/1 R1.3-02D 0/1
```

That is the useful result, and it is worth more than either number on its own.
The four failing metrics — F3-01M, A1-03D, R1-01MD's three sub-tests, R1.3-02D —
are the exact six points the residuals ledger attributes to "no data pointer for
this project". A project holding 7,716 public file URLs scores them the same as a
project holding none, which is direct evidence that the pointers are invisible to
an assessor today rather than merely unhelpful. It also means any movement after
the change is unambiguously caused by the change: there is no pre-existing
difference between the three projects to confound it.

It also fixes the comparison for **0862 must not move**. 0862's baseline is the
same 16 of 24, so "did not move" is checkable against this table directly.

### What the next orchestrator should not re-derive

These come from the team lead's brief and were **not** independently verified
here. Treat them as leads with stated provenance, not as measurements of mine.

- Ingest URLs are `https://ingest.dasch.swiss/projects/{shortcode}/assets/{asset}/original`;
  a GET returns 200 with `Content-Type` and `Content-Length` matching the
  record's `mimeType` and `fileSize` exactly. **HEAD answers 405** and range
  requests are ignored (a ranged GET returns the whole body, 200). Those two are
  risks to `R1-01MD-3`/`-4`, not to F3-01M.
- F-UJI reads `distribution` at `metadata_collector_rdf.py:884-921`: root-level
  `SDO.distribution`/`SMA.distribution`, or a `hasPart` typed `MediaObject`. It
  takes `contentUrl or url`, `encodingFormat or fileFormat`, `contentSize or
  fileSize`, never checks the distribution's own type, and stringifies the size.
  `R1-01MD-3` then downloads and compares content type and byte length
  (`fair_evaluator_data_content_metadata.py:246-328`).

### The licence disagreement — a data question, not a code question

Project 0868's own licence is `https://creativecommons.org/licenses/by/4.0/deed.en`.
All 7,716 of its file-carrying records carry
`https://creativecommons.org/publicdomain/zero/1.0/`. 0803 disagrees the same
way. **These are different licences, and nothing in this work should decide which
is right.**

The consequence for Phase 7 is concrete: a root-level `distribution` inherits no
licence, so under a root `license` a reader would take the project's licence to
govern every file, which would misstate all 7,716. Each `DataDownload` therefore
carries its own `license` from the record's `legalInfo.license.licenseURI`.
`DataDownload` is a `CreativeWork`, so that is valid schema.org.

**For the user, as an open question:** is the project licence or the record
licence the correct statement about these files, and should one of them be
corrected in the corpus? The metadata will faithfully report the disagreement
until someone decides. All 11,778 file-carrying records across both dumps are
`Full Open Access`, so access level is not in question — only the licence.

### Phase 7 must amend ADR-0005, and this is governance, not wording

`docs/adr/0005-fair-landing-pages-in-the-access-area.md:16` currently reads:

> **Nothing is invented for a score.** A project has no project-level download,
> so no `distribution` is emitted; record pages are the right target for
> data-level tests.

The plan's own rule is that where plan and ADR disagree, the ADR wins and the
plan is amended. Phase 7 emits a `distribution`, so shipping it against an
unamended ADR would make the ADR false on the day the code lands — precisely the
failure that rule exists to prevent.

The **principle is untouched and must be restated, not weakened**: nothing is
invented, and a project whose records carry no file still emits no
`distribution`. What is wrong is the *premise* that no project has one. 0868
carries 7,716 records with files and 0803 carries 4,062. Record pages remain the
right target for record-level tests, and the rest of that sentence — DataCite
registration, persistence policy, search indexing as residuals a code change
cannot fix — stays true.

Amend it **with the code that makes it true**, in Phase 7, not before. The team
lead has raised with the user whether the wording is theirs to set; absent a
decision, draft it and flag it prominently for review rather than landing it
quietly.

### Where the branch stands

HEAD `7754a800`. Phase 5 complete (checkboxes 1–12; 13 is the session's),
Phase 6 complete (1–8; 9 is the session's), Phase 7 not started beyond these
baselines. Working tree carries the plan and the journal, modified and unstaged,
as every round. Standing gate green as of `a35f1ba7`; `f50c162b` and `7754a800`
are documentation-only commits on top of it, each with `just check` and
`mdbook build docs` run but not the full rebase gate — the next orchestrator
should run it as part of Phase 7's close.

Two further documentation commits landed after the baselines, both Phase 5/6
cleanup rather than Phase 7 work: `f50c162b` (the data-pointer residual says
which project it is about) and `7754a800` (the FAIR Maturity Indicators index,
linked where the FAIR Champion test names appear). The I2 knock-ons from Phase 6
are closed across the plan, this journal and `docs/src`; the grep sweep is in the
orchestration report.


## Round 15 — Phase 7, record file distributions (2026-09-18)

Phase 7 was written into the plan in this round; it did not exist before. The
brief, the baselines and the licence disagreement all come from Round 14, which
stopped deliberately before any code.

### The change, in one paragraph

`PartRef` gained an optional `FileRef` — url, MIME type, file name, byte size,
and the *record's* licence URI — filled by `PartRef::from_record`, which is
already one of the three functions permitted to take a `&Record`. `RecordGraph`
was not touched: a record page's own `distribution` belongs to the plan that
adds record pages, and the existing
`the_file_pointer_is_not_carried_beyond_the_mime_type` test still holds. The
schema.org writer emits one root-level `DataDownload` per listed part that has a
file, capped with `hasPart` over the same parts in the same order, so the files
described are the files of the records listed. `/metadata.jsonld` is uncapped.

Open access is enforced in the builder, not in the writer. A record the corpus
does not record as `Full Open Access` yields no `FileRef` at all, so no future
writer can advertise a restricted record's ingest URL, and the comparison fails
closed — `Record::access_rights` is free text, unlike a project's typed
`AccessRightsType`, so a spelling the builder does not recognise yields nothing.
Every file-carrying record in the corpus is fully open today, which is precisely
why this is a rule rather than a filter: it is unenforced by the data.

### What the served pages carry

Measured against the local `dpe-server serve` at `162d5f4f`:

| Project | Embedded block | `hasPart` | `distribution` |
|---------|---------------:|----------:|---------------:|
| 0862 | 6,729 B | 0 | 0 |
| 0868 | 45,147 B | 100 | 100 |
| 0803 | 22,986 B | 100 | 4 |
| 081C | 14,219 B | 100 | 0 |

Two things in that table are worth keeping.

**0803 shows only four downloads in the embedded block** although 4,062 of its
records carry a file, because only 4 of its *first hundred* records do. The cap
counts parts, not files. The uncapped `/metadata.jsonld` carries all 4,062.

**0862 has no committed record dump at all.** Its `hasPart` is empty, so "0862
did not move" is a statement about the committed corpus and not about the
project: `docs/src/dpe/oai-pmh.md` shows a real 0862 record with a PNG on
dev-03, so a deployment carrying 0862's real records would emit distributions
for it too. The residuals ledger now says this rather than "0862's records carry
no file".

The 64 KB budget holds with room to spare at 45 KB for the worst case, which is
0868. The corpus test that asserts it now runs over every committed dump rather
than the largest one alone — the largest is 081C, which carries no file, so it
was measuring nothing about a download — and additionally asserts that at least
one dump produces one, so the budget cannot be met by emitting nothing.

### The measurement

F-UJI 3.5.0, `sha256:3cde9d30bc14…`, local `dpe-server serve`,
`DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000`, at `162d5f4f`. All three
projects scored **16 of 24, metric for metric identical**, before the change.

| Metric | 0862 | 0868 | 0803 |
|--------|-----:|-----:|-----:|
| F1-01D | 1/1 | 1/1 | 1/1 |
| F1-02D | 0/1 | 0/1 | 0/1 |
| F2-01M | 2/2 | 2/2 | 2/2 |
| **F3-01M** | 0/1 | **1/1** | **1/1** |
| F4-01M | 1/2 | 1/2 | 1/2 |
| A1-01M | 1/1 | 1/1 | 1/1 |
| A1-02M | 1/1 | 1/1 | 1/1 |
| **A1-03D** | 0/1 | **1/1** | **1/1** |
| I1-01M | 2/2 | 2/2 | 2/2 |
| I2-01M | 1/1 | 1/1 | 1/1 |
| I3-01M | 1/1 | 1/1 | 1/1 |
| **R1-01MD** | 1/4 | **3/4** | 1/4 |
| R1.1-01M | 2/2 | 2/2 | 2/2 |
| R1.2-01M | 2/2 | 2/2 | 2/2 |
| R1.3-01M | 1/1 | 1/1 | 1/1 |
| **R1.3-02D** | 0/1 | **1/1** | 0/1 |
| **Total** | **16/24** | **21/24** | **18/24** |

**0862 did not move, metric for metric.** It is the control and it behaved like
one. It is also the weakest of the three as evidence, because no record dump is
committed for it — its `hasPart` is empty, so there was never anything for the
change to do. `docs/src/dpe/oai-pmh.md` shows a real 0862 record with a PNG on
dev-03, so a deployment carrying 0862's records would describe them too. The
ledger now says that rather than "0862's records carry no file".

**0803 stops at 18, and the reason is in its data.** It gained `F3-01M` and
`A1-03D` like 0868 and nothing beyond. None of its 4,062 files records a
`mimeType`, so no `encodingFormat` is emitted; F-UJI reports `NO info about file
type available in given metadata` for every file it sampled, which fails
`R1-01MD-2a` and takes `-2` and `-3` with it, and leaves `R1.3-02D` with no
format list. dsp-ingest serves those files as `application/octet-stream`, so the
header fallback does not rescue it either. Both ends would have to change. This
is a data-quality residual and was left as one: emitting a guessed format would
be exactly the invention ADR-0005 forbids.

### Three things the measurement corrected

**One. The data-pointer cause was worth five points, not six.** The ledger
attributed `R1-01MD-2`, `-3` and `-4` to it. `-2` and `-3` moved on 0868; `-4`
did not, and its requirement is `variableMeasured` — measured variables or
observation types, which the DaSCH schema does not record and which no data
pointer will ever earn. The original attribution was inferred from sub-test
names in a `test_debug` capture, and only running the metric separated them.
`R1-01MD-4` is now its own cause, one point, for every project. 0862's ledger is
therefore eight points across four causes, not three, and it still closes:
5 + 1 + 1 + 1.

**Two. The embedded cap bounds the page, not the assessor.** F-UJI read the
embedded block and then followed the `describedby` link to `/metadata.jsonld`,
merging what it found: `Found data links in MetadataFormats.JSONLD metadata -:
100` followed by `-: 7716`, and `Number of object content identifier found -:
7716`. So the five points were earned off the *uncapped* representation, which
is an argument for keeping it uncapped. The 64 KB assertion still governs the
page, and the cap is still doing its job there.

**Three. `R1-01MD-3` passing is a check on our own data.** F-UJI downloaded
files and compared `Content-Type` and byte length against each record's
declaration. For 0868 every sampled file matched — `Sucessfully verified content
size ... (expected: 944.0, found: 944)`, `content type ... (expected: text/csv
... via header text/csv)` — with mismatches only where F-UJI itself truncated at
1,000,000 bytes. That is stronger evidence that the corpus's `fileSize` and
`mimeType` are right than any test in this repository.

The HEAD `405` and the ignored range requests, flagged in the brief as risks to
`R1-01MD-3`/`-4`, turned out not to matter: F-UJI issues a plain GET and
truncates client-side.

### What an assessment now costs

Advertising distributions makes an assessment fetch the files. The bytes come
from `ingest.dasch.swiss`, not from DPE, so DPE's per-IP limiter does not bound
them.

| | Embedded | Bytes if all embedded are fetched | Uncapped | Bytes if all are fetched | F-UJI actually pulled | Wall clock |
|---|---:|---:|---:|---:|---|---:|
| 0868 | 100 | 90.9 MB | 7,716 | 2.72 GB | 26 files, 11.3 MB | 558 s |
| 0803 | 4 | 81.9 MB | 4,062 | 139.7 GB | 6 files, 6.0 MB | 307 s |

The gap between the last two columns is F-UJI's own restraint: five files per
MIME type, each truncated at 1,000,000 bytes. Its draw scales with how many
distinct file types a project has, not with how many records it holds — which is
why 0803, whose files are all untyped, sampled six of 4,062. The worst-case
columns are what an unrestrained follower would pull, and 0803's 140 GB is what
28 MB TIFFs multiply to.

`just fair-check` could not complete against 0868: its curl `--max-time 600`
expired against a 558 s run, and `REVIEW.md` makes that recipe a review step for
any `shared-fair` change. Raised to 1800 s, with the measurement and the reason
in the comment so a later reader does not trim it back. The recipe now also
writes the full result JSON to gitignored `.claude/tmp/`, because `test_debug`
is where every attribution above comes from and a run that takes minutes should
not throw it away.

Nothing here was acted on beyond the timeout. The cap was not changed, the
uncapped representation was not truncated, and no file was selected by size —
picking small files to be cheap would be shaping output to the assessor, which
is the thing this work has refused throughout.

### Side finding — 0803 records no MIME type for any of its 4,062 files

The most actionable thing this phase turned up, and it is about the data rather
than the code. **Both ends are DaSCH's and both are broken**: the corpus export
records no `mimeType` for a single one of project 0803's 4,062 file-carrying
records, and dsp-ingest serves those files as `application/octet-stream`.

The chain, from `test_debug` of the 0803 run:

1. No `mimeType` in the record, so no `encodingFormat` in the `DataDownload`.
2. `NO info about file type available in given metadata for -:
   https://ingest.dasch.swiss/projects/0803/assets/…/original`, for every file
   F-UJI sampled.
3. `R1-01MD-2a` ("File size and type information are specified in metadata")
   fails, which fails `R1-01MD-2`, which fails `R1-01MD-3`.
4. `R1.3-02D` has no format list to check and reports nothing identified.
5. The header fallback does not rescue it: `Could not verify content type from
   downloaded file -: (expected: None, found: via tika
   ['application/octet-stream'] or via header application/octet-stream)`.

Three points for that project, and fixing either end earns them. More to the
point it is a real metadata gap on real research data, independent of any score:
nothing downstream can tell a TIFF from a CSV without fetching it.

**A guessed `encodingFormat` was refused, deliberately.** The file names carry
`.tif`, and inferring a MIME type from an extension would have earned the three
points immediately. That is exactly the invention ADR-0005 forbids — the corpus
does not record the type, and a metadata field that says what the repository
guessed rather than what it knows is the failure mode this whole plan is against.
The omission is a statement, not an oversight, and a later reader should not
"fix" it by adding a table of extensions.

**The 1,000,000-byte truncation was not the blocker**, though it is the first
explanation anyone reaches for, because every 0803 file logged `Could not verify
content size ... (expected: 3820444.0, found: 1000000)`. 0868's zip files logged
the same truncation warning — `(expected: 2003814.0, found: 1000000)` — and
`R1-01MD-3` passed there anyway. The type check is what carries that sub-test;
the size mismatch is F-UJI truncating its own download, not a wrong `fileSize`.

### Side finding — every `contentUrl` names a host with a retirement date

All 11,778 published `contentUrl` values point at `ingest.dasch.swiss`. Media
moves to Vitrinli and ingest is then retired; per the root `CONTEXT.md` Vitrinli
is an engine library with no routes of its own, so downloads will be served by
the Access Area's `media` capability. The host in those URLs changes.

A **migration obligation**, not a defect, and smaller than it first looks:

- The URLs are true today, which is the standard this plan applied throughout.
  Publishing a currently-correct URL is not a defect.
- They largely self-heal. `RecordFile.url` comes from the corpus export, so the
  first export after the migration carries the new host and DPE re-serves it
  with no code change. Nothing in `shared-fair` or the writer hardcodes ingest.
- What does not self-heal is a third-party harvested copy. `oai-pmh.md` already
  records this tradeoff for the file endpoint's `downloadUrl`, mitigated by the
  consumer fetching it fresh per request. That mitigation does not apply to
  `schema:distribution`, which exists to be harvested and cached, so this use
  has the worse exposure of the two. An aggregator that read `/metadata.jsonld`
  before the move holds dead links until it re-harvests.
- Therefore: **retiring ingest carries a metadata obligation — re-export the
  corpus, then re-publish.** Recorded in `docs/src/dpe/operations.md` where
  whoever plans the retirement will find it.

**Rejected, with the reason, so it is not re-proposed lightly.** A DPE-owned
stable download URL redirecting to wherever the bytes live would remove the
problem entirely. It contradicts the documented decision in
`docs/src/dpe/oai-pmh.md` that DPE serves metadata only and does not redirect to
bytes. That is an ADR question for a later plan, not something this phase
decides — the same treatment Phase 5 gave the `Host`-header fallback.

### Deviation — the standing rebase gate was narrowed, on evidence

The plan's standing criterion is `git rebase --exec 'env -u GIT_DIR just check'
--exec 'env -u GIT_DIR just test' 6f53d3c7`. It was run and **hung on the first
replayed commit for 65 minutes** before being killed deliberately. This is a
deviation, decided on evidence and authorised, not a shortcut.

The evidence that it was hung rather than slow:

- `editor_server-a71b43f86ca53ba5` (PID 11990) had 65 minutes of wall time and
  **12.75 seconds of CPU**. Its parent `cargo` had 0.47 s, `just test` 0.02 s.
- `lsof` showed it holding `/…/T/editor-db-reopen-11990/editor.sqlite3` open
  together with its `-wal` (218 KB) and `-shm`.
- On SIGTERM the runner named the two tests:
  `db::schema::tests::test_reopening_a_file_database_keeps_its_data_and_does_not_re_migrate`
  and `db::tests::test_file_database_uses_wal_and_creates_its_siblings_in_the_directory`,
  both reported as "has been running for over 60 seconds".

Neither is related to this work. **No commit in the range `6f53d3c7..HEAD`
touches `editor-server`.** `shared-metadata` is a dependency of it and this phase
does change that crate, but the change is a doc comment on `RecordFile`, so there
is no behavioural path from it to those tests.

What was run instead, after `git rebase --abort` restored the branch intact:

- Per commit: **full `just check`**, unnarrowed — keeping fmt, clippy and the
  repo's guard scripts, including `check-shared-paths.sh`, which enforces a
  constraint this very work has to satisfy — and a **`just test` restricted to
  the blast radius**: `shared-fair`, `shared-metadata`, `dpe-api-oai`,
  `dpe-server`.
- At the tip: **full `just check` and full `just test`**, unnarrowed, so any
  `editor-server` breakage this branch could have caused is still caught. What
  the narrowing costs is per-commit *attribution* for a suite no commit in the
  range can affect, not coverage of it.

The unnarrowed tip run — full `just check` and full `just test` at `8610109e` —
passed, **including the two `editor-server` tests that hung the gate**. So the
hang is intermittent rather than deterministic, which is why narrowing was the
right response to it and re-running was not.

### Promote the `editor-server` db flake from "known" to "blocking"

It is listed as a known test flake. It has now blocked a mandatory pre-merge gate
twice — ten minutes the first time, over an hour here — while work unrelated to
`editor-server` waited on it. That is not a flaky assertion; it is a hang with a
resource signature, and whoever picks it up should start from evidence:

- Tests: `db::schema::tests::test_reopening_a_file_database_keeps_its_data_and_does_not_re_migrate`
  and `db::tests::test_file_database_uses_wal_and_creates_its_siblings_in_the_directory`.
- Fixture: `$TMPDIR/editor-db-reopen-<pid>/editor.sqlite3`, with its `-wal`
  (218 KB observed) and `-shm` held open throughout.
- **CPU time stays near zero while wall time runs**: 12.75 s of CPU against 65
  minutes elapsed. It is blocked on something, not computing.
- Intermittent: the same suite passed on the same machine minutes later.

Both tests exercise reopening a file-backed SQLite database in WAL mode, and both
hang together, which points at the fixture's lock or WAL handling rather than at
either assertion.

### 081C assessed — the control the other three could not be

A fourth run, added after the first three, because the control was weaker than it
looked. 0862 scores 16 of 24 and did not move, but **no record dump is committed
for it at all** — `modules/dpe/server/data/records/` holds exactly three files,
for 0803, 081C and 0868 — so its `hasPart` is empty and there was never anything
for the change to do. It shows the change invents nothing where there is nothing
to describe, which is true but nearly vacuous.

081C is the case the rule is about: **27,026 records and not one file among
them** (`[.[] | select(.file == null)] | length` equals the dump length). Its
landing page emits `hasPart` 100 capped, 27,026 uncapped, and **no
`distribution` key in either**; `DataDownload` appears nowhere in the served
HTML.

```
FsF-F1-01D 1/1   FsF-F1-02D 0/1   FsF-F2-01M 2/2   FsF-F3-01M 0/1
FsF-F4-01M 1/2   FsF-A1-01M 1/1   FsF-A1-02M 1/1   FsF-A1-03D 0/1
FsF-I1-01M 2/2   FsF-I2-01M 1/1   FsF-I3-01M 1/1   FsF-R1-01MD 1/4
FsF-R1.1-01M 2/2 FsF-R1.2-01M 2/2 FsF-R1.3-01M 1/1 FsF-R1.3-02D 0/1
total: 16.0/24.0
```

**Metric for metric identical to 0862**, which is the result worth having: a
project with 27,026 parts scores exactly what a project with none scores, so the
parts themselves earn nothing and only files do. F-UJI states the absence rather
than skipping for want of a graph — `Valid data (content) identifier missing`
for `F3-01M`, `Skipping protocol test for data since NO content (data)
identifier is given in metadata` for `A1-03D`, and `Could not perform file format
checks as data content identifier(s) unavailable/inaccesible` for `R1.3-02D`.

The run also exercised the `chore(ci)` change from this phase in anger: it
printed `full result: .claude/tmp/fair-check-20260918T113302Z.json`, and the
sub-test attributions above were read out of that file rather than from a
re-run.

## Round 16 — rebase onto main (2026-09-18)

No new capability. `main` moved underneath the branch while the PR sat
conflicted, so CI could not compute a merge ref and no `pull_request` workflow
had run since 08:58. This round rebases the 55 commits onto `78c4a7c7` and
carries in the one functional change `main` made to code this branch had moved.

### What `main` contributed

`origin/main` was five commits ahead of the merge base `79dc4168`. Two mattered.

`70acb525 docs(docs): group the areas under areas/ and name the shared root
shared/` is the ADR branch this PR was originally based on, merged into `main`.
The branch already carries it verbatim as `01a8a456`, so `git rebase` matched it
by patch id and skipped it. That is why the thirteen conflicts a test merge
predicted came out as eight: the docs halves of ARCH-MAP.md, CONTEXT.md,
ADR-0001 and ADR-0002 never conflicted at all.

`78c4a7c7 fix(oai): add ARK resolver for identifiers in the payload` changed the
DataCite `identifier` from the bare `ark:/…` path to the resolvable URL, deleted
`Pid::ark_path()`, and wrote the reason into `docs/src/dpe/oai-pmh.md`: the OAI
`<identifier>` header is the only place an ARK appears without the resolver, and
there is deliberately no accessor for the bare form.

### The graph-shape decision

`RecordGraph` carried both ARK forms — `ark` (the URL) and `ark_path` (the bare
path) — because the DataCite writer used the path form. With `main`'s fix that
premise is gone, so **`ark_path` is removed from the graph along with the method
it read**. Grepping first confirmed the field had exactly two consumers,
`record_datacite.rs`'s `identifier` and the agreement test's two assertions;
nothing needs the path-only form. The OAI identifier header, the one place a
bare ARK still appears, builds its own from `Pid::ark_suffix()` and never touched
the graph.

Keeping the field would have put the tree in direct contradiction with the
paragraph `main` added to `oai-pmh.md`, which is what settled it rather than
taste. Applied at `52214684`, the commit that introduced the field, so no commit
in the branch ever compiles against a method that does not exist:

| Site | Before | After |
|------|--------|-------|
| `shared/fair/src/graph.rs` | `pub ark_path: String` plus its doc comment | field gone; `ark`'s doc comment now states why the URL is the only form |
| `shared/fair/src/graph.rs` | `ark_path: record.pid.ark_path()` | gone |
| `shared/fair/src/graph.rs` (test) | `identifiers_carry_both_ark_forms` | `identifiers_are_resolvable_ark_urls` |
| `shared/fair/src/record_datacite.rs` | `identifier: graph.ark_path.clone()` | `identifier: graph.ark.clone()` |
| `shared/fair/src/record_datacite.rs` (test) | `identifier_is_ark_path` | `identifier_is_resolvable_ark_url`, mirroring `main`'s rename |
| `modules/dpe/api-oai/…/corpus.rs` | `datacite.identifier == graph.ark_path`; `graph.ark.ends_with(&graph.ark_path)` | `datacite.identifier == graph.ark`; the `ends_with` assertion and its comment gone |

### The eight conflicts

All but one were `57ce87db`'s comment trim colliding with a line this branch had
moved or rewritten. In each, `main`'s trim is the intent and the branch's
identifier is the fact, so the resolution takes both.

| File | Resolution |
|------|-----------|
| `justfile` | `main`'s two-block comment shape and its trimmed `test`-recipe labels, with `check-shared-paths` as the name. The neighbouring `check-datastar-delimiters` recipe already has that shape, so the file is now internally consistent. |
| `.github/scripts/check-shared-paths.test.sh` | `main` modified `check-platform-paths.test.sh`, which this branch deleted. The modification was incidental — `57ce87db` dropping ", which is how that bug was caught" from one comment. Ported into the renamed file rather than resurrecting the old one. |
| `modules/dpe/server/fuzz/fuzz_targets/query_params.rs` | `main`'s deletion of the label comment, with `shared_metadata`. |
| `modules/dpe/web/src/domain/project.rs` | `main`'s three-line header, with `shared-metadata`. |
| `modules/dpe/api-oai/src/handlers/test_utils.rs` | `main` deleted `/// In-memory repository for testing.` as a label restating the name. The branch's replacement said that *and* explained the two fields. Kept the explanation, dropped the label. |
| `modules/dpe/api-oai/src/handlers/mod.rs` | `main`'s deletion of `// Existence check: …`, with the branch's `let … else`. |
| `modules/dpe/server/src/router.rs` | `main`'s deletion of `// Datastar SSE + JSON endpoints.`, with the branch's `rate_limited` merge. |
| `shared/fair/src/datacite.rs` | Three hunks, all consequences of the two above: the temporal test block and the creator fallback that later commits move out anyway. |

`docs/adr/0005` auto-merged and still reads correctly beside this branch's
amendment. `docs/src/dpe/oai-pmh.md` auto-merged; `main`'s two added lines sit
under *Identifiers* and are accurate for the rebased tree — a grep confirms every
ARK any writer emits is the URL form, and the only bare one is the OAI header
they describe.

### Re-proving byte identity against the new main

The old baseline at `.claude/tmp/oai-baseline-hashes.txt` predates `78c4a7c7`,
which deliberately changed OAI output, so it could not be used as-is;
regenerating it from this branch's own tip would have proved nothing. Instead a
fresh baseline was generated from `origin/main` — `main`'s code, fix included,
none of this branch's work — in a throwaway detached worktree, by hand-porting
`588ee18d`'s version of the hash test (`platform_metadata` for `shared_metadata`
is the only difference; `main`'s `to_oai_record` signature already matches it).

**102,158 entries from `78c4a7c7`.** 50,994 of them differ from the pre-fix
baseline, which is the record-level `oai_datacite` half plus `18a8f36f`'s `0105`
data change.

The recovered hash test then ran at the rebased tip against that file:
`compared 102158 entries`, pass. Byte identity now means what it always meant —
this branch's refactor changes no OAI output — measured against the new `main`.
Both edits (`hash_check.rs` and its `mod` line) were removed afterwards. The
pre-fix baseline is kept beside the new one as
`.claude/tmp/oai-baseline-hashes.pre-78c4a7c7.txt`.

### Verification

`cargo check --workspace --all-targets` ran per commit through the rebase, so
every one of the 55 compiles. At the tip: `cargo +nightly fmt --check`, `cargo
clippy --all-features -D warnings`, `cargo machete`, the maudfmt no-op check and
all three gate scripts pass; `mdbook build docs` succeeds; the DataCite golden
carries the resolvable URL.

One pre-existing failure, not caused by this round: `just --check --fmt
--unstable` reports a difference on `justfile` at `origin/main` as well as here,
under local `just` 1.49.0. The reformat it wants inserts a blank line into an
unrelated wrapped comment above `_tailwind-bin`. Left alone rather than
committed, since CI installs its own `just` and the same check passes there.

## Round 17 — Phase 8: decouple previews from production ARKs (2026-09-18)

Six commits on top of `bf360357`, which is pushed, so nothing at or below it was
amended or rebased.

| # | Chunk | Commit | Notes |
|---|-------|--------|-------|
| 8.1 | `ArkHost` and the substitution in both graph builders | `b7bc87f0` | `feat(shared-fair,dpe-api-oai)`; includes the two corpus-wide tests |
| 8.2 | `DPE_ARK_RESOLVER_BASE_URL`, validation, `AppState`, the OAI global | `2d4aacfa` | `feat(dpe-server,dpe-api-oai)` |
| 8.3 | The `/ark:/72163/1/{shortcode}` resolver route | `3918d1dc` | `feat(dpe-server)` |
| 8.4 | The preview workflow, all three variables | `60e8581f` | `chore(ci)` |
| 8.5 | The unset-default test on the OAI crate's global | `3c516d22` | `test(dpe-api-oai)` |
| 8.6 | Docs, ADR note, plan and journal | this commit | `docs(docs)` |

Checkbox "run `eng:reviewing`" is left unticked, as in every prior phase: the
interactive session owns reviews.

### `sameAs` would have defeated the whole change

`ProjectGraph.pid` feeds `schema_org.rs`'s `sameAs`, emitted when the recorded
PID differs from the resolved ARK. Today it never fires, because `canonical_ark`
returns the `pid` whenever it is real. Substituting the host on `ark` alone
would have made it fire for **every** project, putting
`https://ark.dasch.swiss/…` back into the JSON-LD.

That is not cosmetic. F-UJI's object-identifier collection reads `sameAs`:

```python
for identifier in (
    list(g.objects(item, DC.identifier)) + … + list(g.objects(item, SDO.identifier))
    + list(g.objects(item, SDO.sameAs)) + list(g.objects(item, SMA.sameAs))
    + list(g.objects(item, SMA.url)) + list(g.objects(item, SDO.url))
):
```

`fuji_server/helper/metadata_collector_rdf.py:523-538`, read out of the pinned
3.5.0 image. So the production ARK would have gone straight back into the pool
`F1-02D` reads, and the fix would have looked complete and measured as nothing.
`pid` is substituted alongside `ark`, and the corpus test asserts it by name.

#### The transferable lesson, which is bigger than this bug

Note what would have happened. `graph.ark` would have been right. Every
writer's identifier field would have been right. The `@id`, the DataCite
`identifier`, `dc:identifier`, `cite-as` — all correct, all substituted, all
passing. The change would have looked complete at **every level a structural
assertion inspects**, and it would have moved nothing, because one writer
derived a second output from a *different* field that no assertion named.

That is the general shape: **a test that inspects the structure the code builds
cannot catch a writer deriving the wrong thing from a second field.** It can
only check the fields it was told to check, and the field it was not told about
is exactly where this class of bug lives. The bytes a consumer actually reads
have no such blind spot — every emitted identifier is in them, whichever field
it came from and whichever writer emitted it.

So the "set" test renders every representation to a string and counts ARK hosts
in those bytes; the "unset" test does the same against the recorded host. A
field comparison would have passed with `sameAs` broken. The byte sweep caught
it on the first run, and then caught the 083D boundary below on the second —
two findings neither of which was in the brief, from one test that declines to
trust the code's own vocabulary.

The same reasoning is why this round's OAI check is still the 102,158-entry hash
baseline rather than a set of field assertions. It is the same instrument
pointed at the same risk.

### The 083D boundary, found by the test rather than by reading

The first version of the "set" test asserted that every `ark:/72163/1/`
occurrence in the rendered output carried the substituted host. It failed on one
project out of 85: **083D records its own ARK as the project's `url`**, and
`producer.url` carries it verbatim.

That is the right behaviour and the test was wrong. `raw.url` is a recorded fact
about the project — its website — not an identifier the code resolves.
Rewriting it would be inventing a fact, which ADR-0005 rules out. 083D also
records the ARK inside `howToCite`, which no writer emits today; the same
reasoning would apply.

The test now accounts for it explicitly: the recorded host may survive only on a
recorded website, counted from the graph, and the count must match. A second
project acquiring one would fail it rather than slip through.

### Record ARKs: 404, decided on evidence

The brief asked for a decision on evidence rather than taste. Three pieces:

1. **`ark.dasch.swiss` does not resolve a record ARK to DPE at all.** Measured
   2026-09-18: `GET https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh`
   → `302` to `https://app.dasch.swiss/resource/0803/lklK7rVuVOmpBZYWrF8o-g`,
   the VRE. A project ARK → `302` to
   `https://repository.dasch.swiss/dpe/projects/0862`, which is what the new
   route mirrors, status code included.
2. **Nothing a project-page assessment dereferences reads a record ARK.** F-UJI
   puts `hasPart` into `related_resources` (`metadata_mapper.py:181`,
   `metadata_collector_rdf.py:435`) and only treats a `hasPart` as a
   distribution — the thing it fetches — when its `rdf:type` contains
   `MediaObject` (`metadata_collector_rdf.py:887-892`). Ours are `Dataset`.
3. **DPE has no record landing page.** A redirect to the project page would
   answer for a different entity, which is the very thing F-UJI warns about
   ("the PID seems to resolve to a different entity").

So the four-segment route is the whole resolver, a record ARK's five segments
never match it, and it falls through to the site's 404. A test pins that rather
than leaving it to be inferred from routing.

### `DPE_OAI_BASE_URL` — the same defect, third instance

Confirmed. It defaults to `https://repository.dasch.swiss/dpe/oai`
(`config.rs`, and `DEFAULT_BASE_URL` in `dpe-api-oai`), and
`cloud-run-dpe-pull-request.yml` never set it. So a preview's OAI responses
advertised production as their `baseURL` and echoed it in every `<request>`: a
harvester reading the preview was told to harvest production. One line in the
same `--update-env-vars`, fixed here.

### The unknown-shortcode mismatch with the brief

The brief said an unknown shortcode should "behave like the landing page's
unknown-shortcode case". The landing page answers **200** with a "Project Not
Found" body, which cannot be mirrored here: there is no page to render, and no
canonical URL to redirect to — echoing the raw path segment into `Location`
would break `metadata.rs`'s rule that no identifier comes from the request path.
The resolver answers 404. Recorded as a deliberate divergence, not an oversight.

### Where the ARK host is *not* applied

- **The sidebar permalink.** `project_sidebar/mod.rs` renders `proj.pid` from
  `dpe-core`'s view model, not from the graph, so a preview still shows the
  recorded ARK to a human reader. Not a machine-readable representation, not
  read by either assessor, and arguably correct: a person looking at a preview
  is being shown the project's real PID. Left alone and recorded in the plan's
  out-of-scope note.
- **`ProjectGraph.website` / `secondary_website` and `how_to_cite`.** Recorded
  data, per the 083D finding above.

### Two globals, and why

`dpe-server` threads the value through `AppState`, which is what ADR-0005 asks
of the Access Area's own URLs. `dpe-api-oai` reads it from a `OnceLock` beside
the one it already keeps for its base URL, because nothing but the request
reaches its handlers and `main` sets both from the same `DpeConfig` field. That
adds to pre-existing debt rather than introducing a new pattern, and the
`AppState` doc comment now covers both second copies.

The OAI global takes **no environment-variable fallback**, unlike `base_url()`.
An unset value has to mean "the recorded host" and nothing else, or the corpus
test proving the unset case could be steered by the environment it runs in.

### Verification

- `cargo check --workspace --all-targets`: clean.
- Full check gate at every commit and at the tip: `verify-checksums`,
  `check-shared-paths`, `check-datastar-delimiters`, the maudfmt no-op check,
  `cargo +nightly fmt --check --all`, `cargo clippy --all-features -D warnings`,
  `cargo machete`. All pass.
- `just test` narrowed to `shared-fair` (186), `shared-metadata` (73),
  `dpe-api-oai` (109) and `dpe-server` (107 + 103): all pass.
- **OAI byte identity: `compared 102158 entries`, pass.** The hash test was
  recovered from `4cdf06d8^` into `metadata/hash_baseline.rs`, adapted to the
  two new parameters (both `ArkHost::Recorded`), run against the existing
  `.claude/tmp/oai-baseline-hashes.txt` — regenerated from `origin/main` at
  `78c4a7c7` last round and **not** regenerated here — and removed again, with
  its `mod` line.
- `mdbook build docs`: succeeds.
- `just --check --fmt --unstable` still fails identically on `origin/main` under
  local `just` 1.49.0, as Round 16 recorded. Not touched. Because it is the
  first line of `just check`'s body, the rest of the recipe is unreachable
  locally; the gate was run as a script replicating `check` without that one
  line, which is why the list above is spelled out.

### Still open

- **No score is claimed.** Nothing has been re-assessed since this change. Two
  measurements would settle `F1-02D`: a preview carrying this work, and a
  `dasch.swiss` host (assessing DEV after merge, H1).
- Everything Round 11 listed as still open remains so.
