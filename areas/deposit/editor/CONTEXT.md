# Deposit Area

The producer side of the Trusted Repository: where a depositing project team edits its project metadata, RDU reviews it field by field, and an approved record is handed on for publication. Today the metadata editor (`editor-core`, `editor-web`, `deposit-server`), with identity beside it in its own crates, `identity-core` to `identity-web` (see [`areas/deposit/identity/CONTEXT.md`](../identity/CONTEXT.md)); planned beside it in the same modulith: `media` (uploaded Originals and their previews, using the Vitrinli library), the data-model creator, data creation (the working graphs in Chischtli, of which it is the single writer), and the DaTEI configurator (the authoring surface for a digital edition's configuration; tentative, see `areas/access/datei/CONTEXT.md`) — see the root `CONTEXT.md`, `vitrinli/CONTEXT.md` and `chischtli/CONTEXT.md`. Contract terms — Project, Shortcode, Person, Organization, Multilingual, Placeholder, Temporal coverage — are defined once in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared` and only used here.

## Language

### People and access

Depositor, RDU, Assigned project, Login code and Session are identity's terms, defined in [`areas/deposit/identity/CONTEXT.md`](../identity/CONTEXT.md).

**Actor**:
Who is asking, as the editor receives a live Session from identity: the account id, its display name, its role, its Assigned projects and when the Session ends (`editor-ports`). The editor's own word for the signed-in account; it never sees a `users` row. `review_rounds.actor` and the `auth.actor` log field hold the Actor's account id.
_Avoid_: user, viewer (the page header's name for the same person), principal (security vocabulary that nobody depositing or reviewing says).

**Account** (in `editor-web`):
An Actor with its account id parsed once into the `Uuid` the editor's records hold; what the editor's `Authenticated` and `Rdu` extractors yield. The Actor crosses the port; the Account never leaves `editor-web`.

### The record and its journey

**Draft**:
The depositor's working copy of one project: a `drafts` row holding a `ProjectDraft`, which is the project's JSON members verbatim — a field not yet filled in is an absent key, an invalid value is kept as typed, and validity is decided once, at submit, by `editor_core::submission::check`. One draft per project, last write wins.
_Avoid_: working copy, unsaved changes (a draft is saved), "the JSON" or "the export" anywhere a depositor reads (REQ-2.2).

**Submission**:
A `submissions` row, at most one per project, created when a depositor submits a Draft and deleted by every review outcome; it carries the submitted payload untouched and, while a round runs, the Review state beside it.
_Avoid_: SIP (the OAIS package; nothing here produces one yet), request, ticket.

**Review round**:
An append-only `review_rounds` row written by the transition that ended a review — approve, request changes, reject or withdraw — and the only surviving evidence of it, since the Submission row is deleted in the same transaction.
_Avoid_: review (ambiguous with the act and the queue page), audit entry.

**Review state**:
The per-field decisions RDU records while a round is running (`ReviewState`, field id → `FieldReview`), stored on the Submission and snapshotted into the Review round.
_Avoid_: confusing with Project state or Submission state — four different `*State` types.

**Decision**:
RDU's per-field verdict on a submitted value, Accept or Revert (`review::Decision`); distinct from a Proposal decision on an Entity proposal.

**Substitution**:
A reviewer's replacement for a submitted value (`FieldReview::value`), stored beside the Submission rather than over its payload, so what the depositor submitted stays answerable; `Some(Null)` is a real substitution that clears the field.
_Avoid_: correction, override, edit (a reviewer does not edit the depositor's draft).

**Withdrawal**:
The depositor's own discard of a pending Submission, recorded as a Review round with outcome `Withdrawn`, so it leaves the same trail as a reviewer's decision.
_Avoid_: cancel, delete.

**Approved record**:
An `approved_records` row: the submitted draft with every Decision applied, written by approve and waiting for release: the Pull-request run opens its pull request, and the merge plus a release carry it into the published set. At most one per project: re-approving supersedes the old row in the same transaction, whatever its pull request's state. The row type is `records::ApprovedRecord`; its public wire view is `approved_records_api::ApprovedRecordView`.
_Avoid_: published record (Online is the state after the merge and a release, not this), export.

**Pull-request run**:
One invocation of `.github/workflows/open-editor-pull-requests.yml`: it reads every Approved record, writes each project's into its Project branch, opens or updates one pull request per project, and writes one row per record to the run's job summary. Manual, dispatched after approving. [`docs/src/editor/publishing-to-git.md`](../../../docs/src/editor/publishing-to-git.md) is its contract.
_Avoid_: sync, export, deploy, publish (publication is the merge plus a release, not the run), collection, collect (the metadata model's Collection is a different thing), collector (the telemetry beacon collector is a different thing).

**Project branch**:
`editor/<shortcode>` (`editor_core::approved_records_api::BRANCH_PREFIX`): the one branch a project's pull request lives on, keyed on the Shortcode so at most one pull request is ever open against a project's file. Owned by the Pull-request run, which refuses to force-push over a tip it did not write.
_Avoid_: record branch (it is keyed on the project, not the record), collection branch, feature branch, `editor-collect/` (the earlier prefix).

**Waiting for release**:
The RDU-facing classification of an Approved record whose data the published set does not carry yet, shown as a badge on the Approved records page. Distinct from the depositor-facing waiting-for-release notice, which `areas/deposit/editor/CLAUDE.md` names.
_Avoid_: pending, not yet opened (the record may already have a pull request).

**Approved records page**:
`/approved`: the RDU page listing every Approved record against the published set, with a link to its pull requests on GitHub and Discard.
_Avoid_: collection surface, `/collection`.

**Project state**:
One of exactly five depositor-facing values, `ProjectState`: Draft, Submitted, In review, Approved, Online — normative per REQ-2.1, where Online is derived from the published set, never stored.
_Avoid_: status (the contract's `ProjectStatus` is the research project's own lifecycle, a different thing), any sixth word ("rejected" is shown as the outcome of a round, not as a state).

**Submission state**:
The three stored values of a Submission (`SubmissionState`: Submitted, In review, Approved); a reviewer's claim moves Submitted to In review, which the project form reads as a reason to lock the depositor out. Submitted and In review are pending (`SubmissionState::is_pending`, the one definition).

**Published set**:
The `projects/*.json` files baked into the editor image (`PublishedProjects`), keyed by the case-folded `shortcode` field — not the filename stem, which disagrees with the field in five files.
_Avoid_: corpus (fine for DPE's directory; here the set is the image-baked snapshot), the DPE data.

### The form

**Section**:
A grouping of Fields in the project form (`registry::Section`), one URL each, whose `fields_for` is the sole audience gate for both rendering and applying a field.
_Avoid_: tab, step, page (the section renders as a page, but the page is not the concept).

**Field**:
One editable contract member as the form knows it (`registry::Field`, in `editor-core`): label, hint, obligation, audience and Shape, keyed by the contract member's id.

**Shape**:
How a posted body is read back into a Draft for one Field (`form::Shape`): a scalar, a language map, a closed choice, a URL slot, a list of strings, or one of four kinds of row. The widget (`editor-web`) is chosen by field id, the decoder by Shape, and a test ties each Shape to the names its widget posts so they cannot drift.

**Obligation**:
Whether a Field is required, and the count the rail shows; refusal happens at submit, never at save.

**Entity proposal**:
An `entity_proposals` row through which a depositor proposes a new Person or Organization, or a change to one their project references; keyed by Shortcode, it outlives the Submission it rides in, and its allocated id is never reused.
_Avoid_: entity draft, agent proposal (in prose "Agent" is the archive's word), person request.

## Relationships

- A **Depositor** has zero or more **Assigned projects**; an **Assigned project** names exactly one **Project** by Shortcode.
- A **Project** has at most one **Draft** and at most one **Submission** at a time, and zero or more **Review rounds**.
- A **Submission** carries one **Review state**, which holds one **Decision** and at most one **Substitution** per changed **Field**.
- A **Review round** ends exactly one **Submission** and snapshots its **Review state**.
- An approve writes exactly one **Approved record**, superseding the project's earlier one; a **Project** is **Online** once that record matches the **Published set**.
- A **Project** has zero or more **Entity proposals**; at most one live proposal per entity per project.
- A **Section** groups one or more **Fields**; every contract member is either a **Field** or deliberately omitted (`OMITTED`), which a test enforces.

## Example dialogue

> **Dev:** "When RDU approves, does the **Submission** become the **Approved record**?"
> **Domain expert:** "No. Approve writes an **Approved record** from the submitted **Draft** with every **Decision** applied, deletes the **Submission**, and writes the **Review round** — all in one transaction. The record then waits for release; the project shows **Approved** until the **Published set** contains it, and only then reads **Online**."

> **Dev:** "A depositor changed a field RDU had already accepted in the previous round. Do we keep the new value?"
> **Domain expert:** "While the latest round asked for changes, an accepted **Field** is fixed: the section handler skips its applier, so a hand-built body cannot alter it either. A reverted field is not locked — the depositor has nothing to preserve there."

> **Dev:** "Can I add a sixth **Project state** for 'rejected'?"
> **Domain expert:** "No. REQ-2.1 closes the list at five. A rejection is the outcome of a **Review round**, shown with its note on the project form until the next **Submission**."

## Flagged ambiguities

- **"Draft"** is three things: the `drafts` row, the `ProjectDraft` value, and `ProjectState::Draft` (which also covers an unpublished project with nothing pending). Resolution: unqualified "Draft" is the row; say "draft value" or "Draft state" otherwise.
- **"Approved"** is four things: `SubmissionState::Approved` (the window between the decision and the record write), `ProjectState::Approved` (a record exists, not yet Online) and `ReviewOutcome::Approved`. Resolution: qualify by type in code; in depositor-facing prose only the Project state exists. `/approved`, `handlers::approved` and `pages::approved` are the Approved records page and its `GET /api/v1/approved-records` sibling, not a state.
- **"State"** names three types plus a page (`/states`). Resolution: never say "state" alone; say Project state, Submission state or Review state.
- **"Section"** is a form grouping, a handler module (`web/src/handlers/sections.rs`), a view module and a path segment. Resolution: the domain term is the grouping; the others are named after it.
- **"Collection"** is the metadata model's (`ProjectRaw.collections`, defined in `areas/access/CONTEXT.md`), not editor vocabulary: the form does not offer it (`registry::OMITTED`) and drafts carry it through verbatim; no editor term uses the word.
- **"Retire"** is the system stamping `entity_proposals.retired_at` once an accepted `new`-entity proposal's entity appears in the published set, so it stops riding along with later approved records. _Avoid_ for **Withdrawal**, which is the depositor abandoning their own proposal before a decision. Retirement is never a person's action and never applies to a `change`.
- **"Submission" vs "SIP"**: see the root `CONTEXT.md`. Nothing in the editor produces a SIP today.
- **Depositor-facing vocabulary is normative (REQ-2.2)**: "export", "JSON", "transfer", "commit" and "pull request" may not appear where a depositor reads; `depositor_vocabulary.rs` and the E2E suite assert it against rendered markup. RDU-facing strings are exempt.
- **"Deposition"** is the platform's domain word for the act of depositing (depositor, DepositAgreement, ADR-0002) and named the editor capability in DEV-7374's first draft. Resolution (2026-09-25): the capability is **editor** (`areas/deposit/editor/`, `editor-*` crates, ADR-0002); "deposition" never names a crate, a directory or a capability.
