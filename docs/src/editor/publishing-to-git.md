# Publishing to Git

How an approved record leaves the editor and becomes a commit in this repository. The editor publishes the records and a GitHub Actions workflow does the rest: it reads them and opens the pull requests, and nothing flows back. This page is the contract between the two.

The workflow is `.github/workflows/open-editor-pull-requests.yml` and the binary it runs is the `editor-pull-requests` crate (`areas/deposit/editor/pull-requests`). Both sides are live; this page stays the contract, and on any difference between it and the code, it wins.

## The invariant

**Nothing the editor serves or publishes depends on pull-request state.**

Every rule below is checkable against that one line. The editor holds no credential at all: it is never told what the pull-request run did, and it presents nothing to GitHub. The endpoint applies no filter, so there is no state-dependent selection to go stale. Compromising the editor yields no access to GitHub.

## `GET /api/v1/approved-records`

Unauthenticated, read-only, rate-limited per client IP. Serves **every** approved record the editor holds, with no filter ([the invariant](#the-invariant)). The set is already bounded: startup reconciliation deletes a record once the published set carries it, and a release is a redeployment, so a startup follows every release.

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
      "problem": null
    }
  ]
}
```

Every member is always present, `null` when empty — there is no absent-versus-null distinction to handle. `project` is the publishable project, not the editor's draft. `problem` is non-null only when a record's draft could not be converted; such a record is still served, carrying its error, because dropping it would hide a row and failing the response would let one bad record block every other project.

`entities` carries the project's **accepted** entity proposals, minus any the editor has retired. Retirement applies to `operation: "new"` only: the startup reconciliation retires such a proposal once its allocated `entity_id` appears in the published set, so an entity that already exists as a committed file does not ride along with the project's next approved record and get created twice. Retirement waits for a release, so a `new` entity whose pull request merged before one still rides along with a re-approval. The pull-request run recognises it from the merged pull requests on the project's branch, by the entity files they changed or, for an entity renumbered by the run, by their `Renumbered: <from> <to>` body line, and neither writes it again nor renumbers it: the project references the id it landed under.

An `operation: "change"` proposal is never retired. Its `entity_id` names an entity the published set carries by definition, so membership cannot distinguish a change that has shipped from one that has not, and retiring on it would drop the edit from the payload and the entity file would never receive it. A change may therefore be sent again with a later approved record for the same project. **Applying a change must be idempotent** — it writes a whole entity body, so re-applying the same one is a no-op, and the pull-request run needs no extra check.

## The pull-request run is a Rust binary in this workspace

Not a shell or JavaScript script. REQ-5.4's entity renumbering can only run where `main` is visible, so the payload carries structured records rather than file bytes and the binary re-serializes after renumbering. That means calling `editor_core::canonical`, the same writer the editor uses. A second implementation of the canonical form in another language is exactly what breaks the byte-identical round-trip over the committed project files that the PRD's Success Criterion 3 requires.

The payload carries no file paths. Path resolution belongs with the data, in this repository.

## Deriving pull-request state from GitHub, keyed on the project

**Pull-request state is keyed on the project's shortcode, never on the record id.** The branch is `editor/<shortcode>`. The record id only tells which merged pull request landed which record (below).

This is the load-bearing choice on this page, so here is why. A record id identifies a row the editor may legitimately replace: approving a project again supersedes its earlier record, giving the project a new record with a new id. The *conflict* being avoided, though, is per project file — two pull requests rewriting `0803`'s file. Keying idempotency on the record id would make a superseded record's pull request invisible to the next run, which would then open a second pull request against the same file and orphan the first, with no record referencing it and nothing displaying it. Keying on the shortcode makes that state unreachable: there is at most one project branch per project, and at most one pull request open from it.

**The skip rule.** Per project, list the pull requests whose head branch is `editor/<shortcode>`:

| What GitHub shows | What the pull-request run does |
|---|---|
| A **merged** pull request naming this record | Skip. The change has landed; reconciliation discards the record when a release carries it. |
| An **open** pull request | Force-push the newest approved record onto the branch and rewrite the pull request's body to name it. The open pull request then carries the newest approved state instead of a stale one. Do not open a second. |
| No pull request, or only **closed-unmerged** ones, or only merged ones naming other records | Force-push the branch and open a **new** pull request. |

Before any push, a record whose files would not differ from `main` (a re-approval of what a merged pull request already landed) gets nothing pushed and nothing opened: its row reads `skipped`, "main already carries this record", or `failed` when an open pull request is on the branch, naming it, since that pull request then carries a superseded record and should be closed.

Reopening a closed pull request is never correct: somebody deliberately ended that review.

**A pull request names its record.** The body ends with an `Approved-record: <id>` line. The pull-request run writes the whole body when it opens the pull request and overwrites it after every force-push onto it, so an open pull request names the record its branch carries and a merged one names the record it landed. A failed rewrite is a failed row in the job summary and is repeated by the next run. A merged pull request therefore skips only the record it names. A project approved again after its merge or its release has a newer record, which a pull request merged for the old one says nothing about: it gets a new pull request from the same branch. A merged pull request whose body names no record skips nothing, so a reviewer editing the body keeps that line: without it, the next run opens a pull request for the landed record a second time. The same holds for the `Renumbered: <from> <to>` lines below it: without one, a re-approval writes a renumbered entity a second time.

**Branch lifecycle, one rule covering every case.** The pull-request run owns every `editor/<shortcode>` branch and force-pushes over it whenever no *merged* pull request names the record being carried and the record differs from `main`. Stating it as "no pull request references the branch" would leave both the closed-pull-request retry and the stale-open-pull-request update unauthorised, and a branch left behind by a run that died before `gh pr create` is invisible to a rule phrased in terms of pull requests.

**One runtime guard, not merely a documented convention.** The pull-request run refuses to force-push when the branch tip is not a commit it made, and fails that record's row in the job summary. Reviewer edits are an expected part of this flow, so a reviewer who pushes fixups to the branch must not have them silently overwritten by the next trigger.

**Concurrency.** REQ-5.3's trigger is manual and any RDU member can fire it, so two runs can overlap. The workflow declares a `concurrency` group with `cancel-in-progress: false`.

**Entity ids are re-derived after each record is pushed**, not read once per run. Without that, two records in one run — or two overlapping runs — can allocate the same `person-NNN`.

## What RDU sees in the editor

`GET /approved` (RDU only), the Approved records page, lists every approved record with the columns Shortcode, Project, Approved, Status and Actions. Status is a badge derived only from the startup comparison of the record's payload against the published set, plus a **Pull requests on GitHub** link built from the stored shortcode: `https://github.com/dasch-swiss/dsp-repository/pulls?q=is%3Apr+head%3Aeditor%2F<shortcode>`.

| Badge | The comparison found |
|---|---|
| Waiting for release | The published project differs from the payload. Normal: the record goes Online with the release that carries it. |
| Published | The published set carries the payload. Startup discards the record. |
| Project dropped upstream | The published set no longer holds the project. The record is kept: it may be the only copy of that work. |
| Payload unreadable | The payload could not be parsed. |
| Unexpected comparison | The comparison produced a result none of the others describes. |

Whether a pull request is open, merged or closed, and why a record failed, is read on GitHub through that link and in the pull-request run's job summary. The editor does not know.

**Discard.** Every row offers Discard, at `GET`/`POST /approved/{id}/discard`, behind a confirmation naming what is destroyed. It deletes the record, so the project stops appearing in the endpoint's payload and stops being compared against the published set. There is no way back: the record is the only copy of what was approved. The pull request is not touched and has to be closed on GitHub. The typical reason is a record whose pull request merged with reviewer edits: the published data never matches the approved payload, so the record reads Waiting for release indefinitely. RDU spots it through the link and discards it.

**One record per project.** Approving a project deletes every earlier approved record of it in the approving transaction, whatever its pull request's state: a record is whole-project, so the newest approval carries everything. The next run force-pushes it onto an open pull request, or opens a new one after a merge. A re-approval built from a discarded draft therefore drops the earlier approved changes, and the review diff, taken against the published set, does not show it.

## The token the workflow uses against GitHub

`secrets.GH_TOKEN`, not `github.token`. A pull request opened with `GITHUB_TOKEN` does not trigger workflows, so a pull request opened by the run would arrive with no `check`, no `test` and no `commit-hygiene` run — looking green because nothing ran. `secrets.GH_TOKEN` is already used by `release-please.yml`, which is also the in-repo precedent for querying open pull requests rather than trusting local state.

A pull request opened by the run must satisfy `commit-hygiene.yml` like any other: `type(scope): subject`, one commit, and a scope from the vocabulary in `CONVENTIONS.md`. Project metadata files take `dpe-data`.

## Running it

`workflow_dispatch` only. Dispatch it after approving: until a run force-pushes the newer record, an open pull request still carries the older one.

| Name | Kind | Why |
|---|---|---|
| `EDITOR_BASE_URL` | repository variable | The editor's origin. A variable, not a secret: it is not one, and the hostname is still open with Infra (DEV-6921), so settling it needs no code change. |
| `GH_TOKEN` | secret | Used against GitHub. Not `github.token`, see above. |

The workflow refuses to start when the variable is empty, before checkout: GitHub substitutes an empty string for an unset variable rather than failing the run.

Each record the run handles gets one row in the run's job summary, written as soon as that record is handled: project, record, outcome (`opened`, `updated`, `skipped` or `failed`), pull request and, for a skipped or failed record, the reason. A run that aborts keeps the rows it already wrote, so a record with no row was not reached. The binary fails the job when any record failed.
