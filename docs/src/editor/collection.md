# Collection

How an approved record leaves the editor and becomes a commit in this repository. The editor publishes the records and accepts a report about what happened to them; a GitHub Actions workflow does the work in between. This page is the contract between the two.

That workflow does not exist yet. Everything this page says about it is a requirement on the work that builds it, not a description of something already running; everything it says about the editor is live and testable today.

## The invariant

**Reported collection state may drive what RDU is shown and how a record is classified. It may never drive what is served or published.**

Every rule below is checkable against that one line. The editor holds no credential granting write access to any GitHub repository, in either direction: it holds a verifier for a token the workflow presents to it, never a token it presents to GitHub. Compromising the editor yields a token that authenticates to the editor and nothing else.

## `GET /api/v1/approved-records`

Unauthenticated, read-only, rate-limited per client IP. Serves **every** approved record the editor holds, with no filter, so there is no state-dependent selection for a stale flag to hide. The set is already bounded: startup reconciliation deletes a record once the published set carries it, and a release is a redeployment, so a startup follows every release.

The response performs no write and carries no generation timestamp, so two requests with no intervening write return byte-identical bodies.

```json
{
  "records": [
    {
      "id": "0f9c…",
      "shortcode": "0803",
      "approvedAt": "2026-09-17T14:22:03Z",
      "project": { "…": "the project, as ProjectRaw" },
      "entities": [
        { "kind": "person", "operation": "new", "id": "person-417", "body": { "…": "Person" } }
      ],
      "problem": null,
      "collection": { "collectedAt": null, "pullRequest": null, "state": null, "lastFailure": null }
    }
  ]
}
```

Every member is always present, `null` when empty — there is no absent-versus-null distinction to handle. `project` is the publishable project, not the editor's draft. `problem` is non-null only when a record's draft could not be converted; such a record is still served, carrying its error, because dropping it would hide a row and failing the response would let one bad record block every other project.

**`collection` is advisory.** It is the last thing a run reported, provided so RDU can see what happened. The collecting workflow must not branch on it when deciding what to publish — it derives that from GitHub, as below.

## The collector is a Rust binary in this workspace

Not a shell or JavaScript script. REQ-5.4's entity renumbering can only run where `main` is visible, so the payload carries structured records rather than file bytes and the collector re-serializes after renumbering. That means calling `editor_core::canonical`, the same writer the editor uses. A second implementation of the canonical form in another language is exactly what breaks the byte-identical round-trip over the committed project files that the PRD's Success Criterion 3 requires.

The payload carries no file paths. Path resolution belongs with the data, in this repository.

## Deriving collection state from GitHub, keyed on the project

**Collection state is keyed on the project's shortcode, never on the record id.** The branch is `editor-collect/<shortcode>`.

This is the load-bearing choice on this page, so here is why. A record id identifies a row the editor may legitimately replace: approving a project again supersedes its earlier record, giving the project a new record with a new id. The *conflict* being avoided, though, is per project file — two pull requests rewriting `0803`'s file. Keying idempotency on the record id would make a superseded record's pull request invisible to the next run, which would then open a second pull request against the same file and orphan the first, with no record referencing it and nothing displaying it. Keying on the shortcode makes that state unreachable: there is at most one collection branch per project, and at most one pull request open from it.

Because the branch no longer encodes the record, a report identifies its record by the `record` id from the payload.

**The skip rule.** Per project, list the pull requests whose head branch is `editor-collect/<shortcode>`:

| What GitHub shows | What the collector does |
|---|---|
| A **merged** pull request | Skip. The change has landed; reconciliation discards the record when a release carries it. |
| An **open** pull request | Force-push the newest approved record onto the branch. The open pull request then carries the newest approved state instead of a stale one. Do not open a second. |
| No pull request, or only **closed-unmerged** ones | Force-push the branch and open a **new** pull request. |

Reopening a closed pull request is never correct: somebody deliberately ended that review.

**Branch lifecycle, one rule covering every case.** The collector owns every `editor-collect/<shortcode>` branch and force-pushes over it whenever no *merged* pull request references it. Stating it as "no pull request references the branch" would leave both the closed-pull-request retry and the stale-open-pull-request update unauthorised, and a branch left behind by a run that died before `gh pr create` is invisible to a rule phrased in terms of pull requests.

**One runtime guard, not merely a documented convention.** The collector refuses to force-push when the branch tip is not a commit it made, and reports that as a failure. Reviewer edits are an expected part of this flow, so a reviewer who pushes fixups to the branch must not have them silently overwritten by the next trigger.

**Concurrency.** REQ-5.3's trigger is manual and any RDU member can fire it, so two runs can overlap. The workflow declares a `concurrency` group with `cancel-in-progress: false`.

**Entity ids are re-derived after each record is pushed**, not read once per run. Without that, two records in one run — or two overlapping runs — can allocate the same `person-NNN`.

## `POST /api/v1/collection-report`

Bearer token, held only by CI, verified against `EDITOR_COLLECTION_TOKEN`. An absent token and a wrong one are refused identically. **One record per call**, reported immediately after that record is handled, so a run that dies halfway still leaves a signal for every record it had already finished.

```json
{ "record": "0f9c…", "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/412", "state": "open" }
```

```json
{ "record": "0f9c…", "failure": "renumbering could not resolve person-417" }
```

Exactly one of `pullRequest` or `failure`. A `pullRequest` requires a `state` of `open`, `merged` or `closed`, and must be under `https://github.com/dasch-swiss/dsp-repository/pull/` so a forged report cannot point RDU elsewhere. A report naming an unknown or already-discarded record is rejected whole, with nothing applied.

**Report every record the run considered, not only the ones it acted on.** The workflow is already listing those pull requests for the skip rule, so the refreshed state costs nothing, and it is what keeps the editor's picture current for records the run merely inspected.

Three fields with three lifetimes come out of a report, and the two report shapes write different ones. `collectedAt` is stamped once and never moved. A pull request report replaces the URL and state and clears `lastFailure`. A failure report records `lastFailure` and **leaves the URL and state alone** — it says nothing about that pull request, and erasing a reference an earlier report established would leave the record reading as uncollected while its pull request is still open.

## What the editor does with a report

`collectedAt` is a display timestamp. The classification that matters keys on the reported **state**:

| Published data | Reported state | How the record reads |
|---|---|---|
| Differs | `open` | Waiting for a release. Normal. |
| Differs | `merged` | Merged with reviewer edits. The genuine anomaly; RDU resolves it (below). |
| Differs | `closed` | The next run re-collects it. No action. |
| Differs | nothing reported | Not collected yet. Normal. |
| Matches | any | The record is discarded at the next startup; the project is Online. |

**RDU resolves a stranded record by discarding it.** `GET /collection` lists every approved record with the classification above, and a record reading `merged` over differing data offers a force-discard at `GET`/`POST /collection/{id}/discard`, behind a confirmation naming what is destroyed. Discarding deletes the record, so the project stops appearing in this payload and stops being compared against the published set. There is no way back — the record is the only copy of what was approved — and no workflow action is involved: the pull request has already merged. A record discarded this way is indistinguishable afterwards from one that never existed, which is what the report endpoint's "no such approved record" answer means.

**One live record per project, enforced at approval.** Approving a project whose earlier record has no live pull request supersedes that record in the approving transaction. Approving one whose earlier record has an open or merged pull request is refused, because publishing a second file version over a live pull request is PRD Edge Case 4 and stays out of v1.

## The token the workflow uses against GitHub

`secrets.GH_TOKEN`, not `github.token`. A pull request opened with `GITHUB_TOKEN` does not trigger workflows, so a collection pull request would arrive with no `check`, no `test` and no `commit-hygiene` run — looking green because nothing ran. `secrets.GH_TOKEN` is already used by `release-please.yml`, which is also the in-repo precedent for querying open pull requests rather than trusting local state.

The collection pull request must satisfy `commit-hygiene.yml` like any other: `type(scope): subject`, one commit, and a scope from the vocabulary in `CONVENTIONS.md`. Project metadata files take `dpe-data`.

## The staleness limitation

A reported state is only as fresh as the last run, and the collection trigger is manual with no schedule. A pull request merged with reviewer edits *after* the last run leaves its record reading `open`, with nobody prompted to re-run, so it never reaches the merged-with-edits classification above.

The surface therefore shows the last reported state **with the time it was reported**, and links to the pull request, so a reader can see the state is old and check it for themselves.

Closing this properly needs a **refresh-only trigger** that re-reports pull request states without collecting anything. That belongs to the collecting workflow and is named here rather than assumed: it is distinct from the collection trigger, which REQ-5.3 keeps manual.
