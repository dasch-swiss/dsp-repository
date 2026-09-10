# The Project Form

How the project form decides what it accepts: which fields it insists on, how a reference to a person or an organization is picked, where a fixed vocabulary is a constraint rather than an offer, and what a save refuses. The service's structure is in [Architecture](./architecture.md); this page is the behaviour that sits on top of it.

Every bound stated here is measured against the 85 committed projects under `data/projects/`, and the measurements are collected in [What the corpus measures](#what-the-corpus-measures) at the end, so the source that depends on them has one place to point at.

## Obligation is a submit gate, and the corpus bounds which fields carry it

`Obligation::Required` reads "must be present to submit" (REQ-1.12) and is enforced as written: `obligation::unsatisfied_required` is what the submit route gates on, and it refuses with one field-level error per unanswered field.

That is only safe because the tier was first bounded by the data. Five of the seventeen fields carrying `Required` were unsatisfied by the published corpus — **all 85** projects lacked `documentationMaterial`, 13 lacked `url`, 9 lacked `contactPoint`, and one each lacked `typeOfData` and `dataLanguage` — so a literal gate would have refused every project already live. All five are now `Recommended`, with the publication requirement moved into the hint where three of them had been claiming "Required before publishing" in words while the pill said the same thing in a tier that could not be enforced.

So the rule is: **a field is `Required` only if the published corpus answers it.** `obligation`'s `UNANSWERED_BY_THE_CORPUS` is empty and its test is now a gate rather than a baseline — tier a field `Required` that the corpus cannot answer and it fails, naming the field and the count. Fix the tier or fix the data, not the constant.

What the gate adds over the conversion beside it is narrow and worth stating, because the two look redundant: every `Required` field is a non-`Option` member of `ProjectRaw`, so an **absent** one already fails `ProjectDraft::to_raw`. The gate catches the state the contract cannot see — present and empty. `keywords: []`, `description: {}`, `name: ""` and a `MISSING` sentinel all deserialize perfectly well, and none of them is an answer.

Presence is read through `obligation::is_satisfied`, the same function the section rail counts with. That sharing is deliberate: a gate stricter than the rail would refuse a submission the rail had just counted complete, and a depositor looking at "5 of 5 required" would have no way to tell which of the two was lying. All 85 published projects come out complete for both audiences, which is the same fact stated per project.

## The agent store, and how a reference is picked

Three fields hold ids rather than values — `contactPoint`, `attributions[].contributor` and `funding[].funders` — 674 references across the 85 projects. `editor_core::agents` loads the 558 committed persons and organizations, mirroring `PublishedProjects`: an explicit `load_from` returning what loaded plus one error per file. It does **not** reuse `dpe-core`'s caches, which are a process-wide `OnceLock` keyed on a global data directory: the editor's directory is `EDITOR_DATA_DIR` on its own `AppState`, so a global would make two tests with different fixtures see each other's entities.

Persons and organizations are flattened to one `Agent` at load time, because every consumer wants exactly an id, a name and a kind from both while the two contract types have different name shapes.

The picker is a **shared `<datalist>`**, rendered once per page and only on a section that has an agent field, and the numbers are what forced it: the list is 31.7 KB and `attributions` reaches 56 rows on one project, so a `<select>` per row would be 1.7 MB of markup re-sent on every save, against 31.7 KB once plus about fifty bytes per row. It degrades honestly — without JavaScript the input is still a working text field.

**The input holds the id, not the name.** That is what keeps an untouched save byte-exact: the stored value goes into the control and comes back out unchanged, with no lookup in between that could resolve differently. The name is rendered beside it, and an id that resolves to nobody says so there as well as at submit — the form is where it can be fixed, and told only at submit a depositor would have to work out which of 56 rows the refusal meant.

Resolution is a submit gate rather than an applier's job, because a draft is allowed to hold a value that does not validate (REQ-1.9) and deciding that is submit's (REQ-1.12). A dangling reference in a field nobody touched is still refused: every committed reference resolves, so a dangling one can only have arrived after the fact — an agent file removed from under a project, which is a thing to report rather than to publish.

**`funding[].funders` is a fourth agent field, and the gate used to miss it.** A grant's funders hold agent ids exactly as the other three do — 125 of them across the 85 projects — but `funding` declares `Shape::FundingRows`, so a filter over `AgentRows | AttributionRows` walked past it: the form warned about an unresolvable funder in its label and submit accepted the project anyway. `dpe-server validate` does not catch it either, because `checks::contributor_refs` reports `attributions` and `contactPoint` only. A dangling funder therefore reached a published file and rendered as a bare `organization-008` on the public project page. All 125 committed funder references resolve, so closing it refuses nothing the corpus already contains. One function, `agent_ids_in_row`, now knows how all three shapes spell a reference, so a field that refers to an agent cannot be added to one reader and forgotten in the others.

**Resolution is scoped to the project.** `AgentScope` overlays the project's own referenceable entity proposals on the published store, so a just-allocated `person-417` resolves in the picker and passes this gate while it is still only a database row. It borrows the published store rather than copying it — that snapshot is 558 agents behind an `Arc` precisely so cloning it per request does not happen. A published id wins over a proposed one, because a change proposal names an id the store already holds and the surface is meant to show the published value beside the proposed one; the listing applies the same precedence, or a change proposal would put two `<option>`s with one `value` into the shared datalist.

## Vocabularies are closed only where the corpus lets them be

Four fields offer a fixed set, and whether the set is a *constraint* or an *offer* is decided by the data every time, never by preference:

| Field | Set | Why |
|---|---|---|
| `status`, `accessRights` | Closed, from the contract enums | A value outside them does not deserialize. |
| `typeOfData` | Closed | All 85 projects use exactly five kinds; there is no tail. |
| Reference sources | Closed, per field | These are the resolvers the platform consults, so an unknown one is a link nothing can dereference. |
| `dataLanguage` | **Open** | 24 distinct tags where the UI offers four; `la` alone is in ten projects. |
| `contributorType` | **Open** | 195 spellings of 181 roles, plus comma-stuffed entries and one biography. |

An open set's slice is the *offer*: the widget unions it with whatever the project already holds, so an unusual value keeps its control instead of vanishing on the next save — a value with no control posts nothing, and a list rebuilt from the body would not carry it.

`CONTRIBUTOR_ROLES` measures its own coverage by **how many projects share a role, not how many times it is used**: `Database programming` is used ten times by one project, which makes it that project's wording, while `Contributor` is used ten times across three. `ROLES_NOT_OFFERED` names the shared ones still left out, so each omission is a decision on the record.

## A variant row narrows on its discriminant alone

`temporalCoverage` and `disciplines` are each either an authority reference or free text per language, and both variants are live — 86 of 141 temporal entries and 131 of 204 disciplines are free text. `funding` is the same idea with the discriminant on the **field** rather than the row: 77 projects hold grants, 8 hold a single string.

The inactive branch is rendered `hidden`, **not `disabled`**. A `hidden` input is still submitted, so the server receives both candidates plus the discriminant and a depositor who switches and switches back finds their work intact; `disabled` would submit nothing and lose it. That is exactly why the discriminant has to be the only thing that narrows: reading the values instead is how a half-filled reference gets silently stored as text, which is what serde's untagged enums would do.

The corpus round trip **cannot** catch that mistake — an untouched form's values and its discriminant agree by construction — so the unit test is the only guard, and it was verified by making the applier narrow on the values and watching it fail.

A text branch is namespaced under `.text.<tag>`, which is load-bearing rather than tidy: `FormBody::entries` reads every dotted suffix under a prefix as a language tag, so texts posted directly under the row would have turned `kind`, `type` and `url` into languages of those names.

## Rows are added and removed by round-trip

`POST …/sections/{section}/fields/{field}/add` and `…/{field}/{key}/remove`, under the section's own URL and handled by the same module, so a row action resolves through one `context()` — the audience gate, the lock check and the shortcode fold are the section's rather than a second copy. The URL names the field, so it is checked against `fields_for` too; without that it would be a way to reach an RDU-only or a display-only field. `apply_and_store` is shared with the save path, so a row action cannot become a second, weaker write.

Removing a row **is** the applier seeing one fewer key, so there is no separate delete. The empty marker is re-added when the last key goes: without it the field reads as absent, the applier leaves it alone, and the last removal does not stick.

A blank added row is never stored — an empty row must not reach a published file — so it lives in the form. The tile emits a hidden `{field}.row` for it, the body carries its key back, and the re-render finds it there. Nothing is held server-side between requests. Row keys are positional on a fresh render (`r0`, `r1`, …) and only have to stay stable for the life of one rendered form: order comes from the repetition of the hidden field, never from a number.

## Discarding a draft

The only thing in the service that removes one. A review that rejects a submission and a depositor who withdraws one both **keep** the draft, so without this an abandoned draft sits in RDU's list for good and keeps the hand-edit collision of PRD Edge Case 5 armed.

"Discard" rather than "delete", on the control and on the wire: what goes is the draft and never the project, and the form re-opens pre-filled from the published metadata (REQ-1.1). Refused without a stored draft, and refused while a submission is pending — the draft is what the depositor comes back to when RDU returns the project (REQ-1.13), so it must not vanish from under a live review; the refusal names the way out, which is to take the submission back first.

## A concurrent save is refused once, not silently applied

Two members of one project team is the normal case, the draft is one row, and `upsert` is last-write-wins per the PRD's Constraints. The form posts the revision it was rendered from, under `baseline`, and a save whose baseline no longer matches the stored row is refused with the other editor's name and the time they saved.

Refused **once**: the re-render carries a refreshed baseline, so a depositor who decides to keep their version simply saves again. The row is still last-write-wins; what changed is that it is no longer silent. Their own name is never shown — "saved by you" is the ordinary case and reads as noise, which also keeps the user lookup off every ordinary render.

It closes the human-scale race, two people with the form open for minutes, and **not** the instant between the read and the write, which needs a transaction rather than a baseline. A body carrying no baseline is saved without complaint: it was not posted from a form this service rendered, so there is no revision it could have been looking at.

The refusal applies the posted body to the **in-memory** draft before rendering, and stores nothing. A scalar control renders from the draft, so without that the refusal showed the *other* person's values under a notice claiming the page still held yours.

## Autosave, and the sign-out warning

Nothing specified what happened to a POST arriving after the session had gone, so a long editing session could be discarded whole. Two halves, one per path.

Autosave is a debounced `@post` of the form on the enhanced path, reusing the save handler. It carries no `intent`, and an absent or unknown verb is read as `save` — the recoverable branch, deliberately — so an autosave can never submit or withdraw; it also keeps the idle timeout alive, because every request touches the session. It fires on `change` rather than `input`: the response patches the whole region, so a trigger on every keystroke would patch the field being typed into. It is rendered only on a form a save can change.

The warning is server-side, which is the half the no-JavaScript path gets. `session::current` returns the earlier of the two deadlines with the user, from the row it already read: the absolute expiry is fixed at sign-in, while the idle timeout advances on every request, so a page showing the idle one alone would promise a time that moves the moment anything happens. It appears within half an hour and not before, because a warning permanently on screen is one nobody reads.

## What the corpus measures

Every closed vocabulary, enforced tier and cap above is decided by measuring the 85 committed projects rather than by preference, so the numbers are recorded here and the source points at this section instead of carrying them.

| What | Measured |
|---|---|
| Placeholder sentinels (`MISSING`, `CALCULATED`) | 131 across 8 paths in the 85 files; 24 of them `endDate` |
| Values differing only in surrounding whitespace | 20 files, spread across `disciplines.text` (7), `publications.text` (6), `attributions.contributorType` (3), `abstract.en` (2), and one each in `keywords.ar`, `description.ar`, `spatialCoverage.text`, `legalInfo.license.licenseURI`, `shortDescription` |
| Two values in one project differing only by a trailing space | `0121_societesavoie`, `attributions[].contributorType` |
| Abstracts holding a bare `\r` | 10 |
| Files whose bytes differ between a CRLF and an LF submit | 26 |
| `Required` fields the corpus does not answer | 0, after re-tiering five: `documentationMaterial` (absent in all 85), `url` (13), `contactPoint` (9), `typeOfData` (1), `dataLanguage` (1) |
| Agents, and references to them | 558 persons and organizations; 674 references from `contactPoint`, `attributions[].contributor` and `funding[].funders`, all of which resolve |
| Shared `<datalist>` size, against a `<select>` per row | 31.7 KB once plus ~50 bytes per row, against 1.7 MB on the 56-row `attributions` project |
| `typeOfData` | 5 kinds across all 85, no tail — closed |
| `dataLanguage` | 24 distinct tags against the 4 the UI offers; `la` alone in 10 projects — open |
| `contributorType` | 195 spellings of 181 roles, plus comma-stuffed entries and one biography — open |
| Reference sources | `temporalCoverage`: all 271 entries are references, from 4 sources. `disciplines`: 73 references, all `Skos` |
| Variant branches, both live | `temporalCoverage` 86 of 141 entries free text; `disciplines` 131 of 204 free text; `funding` 77 projects hold grants, 8 a single string |
| Grant members | Of 121 grants, 91 carry all three optional members and 12 only a number |
| Widest committed language map | 3 tags, against the cap of 250 |
