# Entity Proposals

How a depositor proposes a new person or organisation, or a change to one, and how the proposal survives the submission it rides in.

A depositor can propose a new person or organisation, or a change to one their project references. Proposals live in `entity_proposals`, **not** in the draft payload: `ProjectDraft` is `#[serde(transparent)]` over the project's members and `to_raw` deserializes it into `ProjectRaw`, so a non-contract member would be dropped on the way to a file with nothing saying so.

The table is keyed by shortcode rather than being a child of `submissions`, although a proposal rides inside the project's pending submission. Every review outcome deletes the submission row while the proposal outlives it — an accepted one is on its way into a `persons/` or `organizations/` file, and a returned one is still the depositor's to finish. `review_rounds` is keyed the same way for the same reason.

**`status` and `decision` are separate columns.** `status` is the lifecycle (`draft`, `submitted`, `accepted`, `rejected`, `withdrawn`); `decision` is what RDU recorded in the round now running (`accept`, `reject`, or null while undecided). The split is the one `submissions.review_state` makes for a project field: a decision is taken during a review and only becomes a status when the round ends, which is what lets request-changes hand the proposal back as a draft while retaining what was decided — which is exactly what is required for fields, and a proposal reviewed on the same surface must not lose it.

Two predicates read those, and confusing them is the mistake to avoid. `EntityProposal::is_live` (`draft` or `submitted`) is "still in play — the depositor may edit it and a reviewer may still decide it". `is_referenceable` adds `accepted`: an accepted entity is not a file yet, so a project field naming it must still resolve, while a rejected one must fail resolution — that failure is what makes the referential-integrity gate possible. **Neither says anything about the allocated id**, which every row holds permanently.

## Id allocation is collision-free within the editor, and never reuses

An id — `person-NNN` / `organization-NNN` — is allocated at proposal time, and renumbering happens only on collision *with the repository*, so nothing in the requirements stops two proposals inside the editor taking the same next id. Two things close that:

- **The allocation is one transaction.** `EntityProposalRepository::create_new` selects the ids already taken and inserts the new row inside a single `write` closure, which is `BEGIN IMMEDIATE` on the pool's single writer connection. The second of two concurrent proposals always sees the first one's insert.
- **A partial unique index makes it structural.** `entity_proposals_allocated_id` is `UNIQUE (entity_id) WHERE operation = 'new'`. It is partial because a `change` names an id somebody else allocated, and several projects may propose changes to one entity.

The allocator is `proposals::next_entity_id`: one past the highest number it has seen, over the union of the published store and every id the editor has ever allocated — **terminal rows included**. Gaps are therefore deliberate. Nothing depends on the sequence being dense, while reuse would hand an id to a second entity after a sibling collection pull request may already carry it. `Agents::highest_id_number` supplies the published half, which the persistence layer cannot see.

A second partial index, `entity_proposals_live_per_entity` on `(shortcode, entity_id) WHERE status IN ('draft', 'submitted')`, allows two projects to hold change proposals for one entity but not one project to hold two live ones for it. Two competing rows would show a reviewer separate decisions over one file, and whichever applied last would silently win.

## A change proposal is seeded with the whole published entity

Accepting a change proposal writes its payload as the entity file, so a payload holding only the members a form happens to render would silently drop `affiliations`, `sameAs`, `email`, `alternativeName`, `canton` and `additional`. This is the property `ProjectDraft` gives a project — carry every member the editor does not manage unchanged, survive a member added to the contract without an editor change — and an entity needs it for the same reason, so `Agents` keeps each entity's file body and `Agents::seed_payload` hands it over with `id` removed. A save merges into the stored payload rather than rebuilding it.

**The payload never carries `id`.** It lives in `entity_id`, the column the allocator and the uniqueness index work on; a second copy would be free to drift. Every reader fills it in from `entity_id` and overwrites whatever it finds, so a payload that does carry one cannot make an entity resolve under an id nothing claimed.

## The entity form

`GET | POST /projects/{shortcode}/entities/{proposal}`, plus the two row-action paths the repeatable fields need. A write shares the `GET` that renders it, like every other write here, and that `GET` is marked the same way the section form's is (see [the form's two renderings](./routing.md#the-forms-two-renderings)).

`{proposal}` is a proposal's **`entity_id`** (`person-417`), not its row `id`. That is the value the propose controls post back and the summary links carry, and it makes a readable URL. A project may hold several rows for one `entity_id` over time — a withdrawn proposal, then a fresh one — but never more than one *live* one, because `entity_proposals_live_per_entity` says so; the resolver prefers the live row and otherwise the most recently touched, so a stale link still shows something coherent. A proposal under the wrong shortcode is a 404 rather than a 403, for the reason an unknown section id is: the reader invented the pairing.

**A save merges into the stored payload and never rebuilds it.** That is the seeding property above, enforced at the write: only the members this form posts are applied, so anything else in the payload survives untouched.

`jobTitles` is the one field whose applier needs help. `Shape::StringRows` removes a member when no row survives, which is right for every field of that shape except this one: `check_person` reads an *absent* `jobTitles` as unanswered and an *empty* one as a person with no job title, which 59 of the 416 committed persons already are. So an emptied `jobTitles` is kept as `[]` rather than dropped.

The start controls live on the agent rows themselves, where the picker already says an id resolves to nobody — the form is where it can be fixed, the same argument that comment makes about saying it at the row as well as at submit. Both a person and an organisation are offered, because the picker cannot know which was meant. A row whose id *does* resolve offers propose-changes instead. All three are named submits on the section's own form, so they need no route of their own.

## An incomplete address inherited from the published entity is passed through

A proposed organisation is asked for all four of `street`, `postalCode`, `locality` and `country`, or no `address` at all. Six of the 142 committed organizations satisfy neither — `organization-009`, `-033`, `-065`, `-089`, `-090` and `-137` are missing `street`, `postalCode` or both, and three of them sit outside Switzerland where a postal code may not apply. Applied literally, the rule would leave a depositor proposing any other change to one of those six a choice between inventing a street and deleting a locality and country that *are* recorded.

So `check_organization` judges what the depositor wrote, not what they inherited: an `address` byte-equal to the published one passes, and any other incomplete one is refused. This is the carve-out `form::submit::typed_sentinels` already makes for a reference's `url` — `0110_h-steiner` holds `{"url": "MISSING"}` — and for the same stated reason: a live record must not become unsubmittable over data it did not write. `proposals::tests::six_committed_organizations_carry_an_incomplete_address` enumerates the six and fails if the corpus stops needing the carve-out.

## The review surface gives a proposal its own rows

The review surface compares changed project *fields*, and a proposed person is not one, so entity proposals would otherwise reach an approval without ever being displayed — and the referential-integrity question could not even arise, there being no per-entity action to invoke. Proposed entities therefore get their own rows below the field diff, each with an accept/reject control posting under `entity.{entity_id}`. That namespace cannot collide with the field surface's `decision.{field}` or with a member name.

Approve gains two refusals beside the undecided-fields one:

- **while any proposal is undecided**, for the reason the field gate exists: approving an undecided row commits bytes nobody looked at. `review_rounds`' `approve` leaves an undecided proposal `submitted` and names this gate as what prevents that, so the two halves are one coupling.
- **while the payload still references a rejected entity.** The approval is blocked and the referencing fields are named, so RDU substitutes them in place or requests changes; nothing is stripped silently.

The second gate has a trap worth stating, because getting it wrong disables it without any test failing. At approve time every proposal's `status` is still `submitted` — the statuses change inside `approve`'s own transaction — so `is_referenceable`, which is status-based, answers **true** for a proposal RDU has just rejected. The scope the check runs against is therefore built from only the proposals that will *survive* the approval, filtered on `decision`, and the check runs over the **decided** payload rather than the submitted one, since a revert can remove a reference and a substitution can add one. It reuses `form::submit::unresolved_agents` rather than walking references a second way.

## What the two projects case does

Two projects may propose changes to one entity, and the last collected wins. The review surface says so rather than blocking: blocking would strand one project on another project's review, while a silent overwrite would let RDU approve a change that is about to be replaced with nothing saying so.
