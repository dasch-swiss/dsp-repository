# The Project Form

How the project form decides what it accepts: which fields it insists on, how a reference to a person or an organization is picked, where a fixed vocabulary is a constraint rather than an offer, and what a save refuses. The service's structure is in [Architecture](./architecture.md); this page is the behaviour that sits on top of it.

Every bound stated here is measured against the 85 committed projects under `data/projects/`, and the measurements are collected in [What the corpus measures](#what-the-corpus-measures) at the end, so the source that depends on them has one place to point at.

## Obligation is a submit gate, and the corpus bounds which fields carry it

`Obligation::Required` reads "must be present to submit" and is enforced as written: `obligation::unsatisfied_required` is what the submit route gates on, and it refuses with one field-level error per unanswered field.

That is only safe because the tier was first bounded by the data. Five of the seventeen fields carrying `Required` were unsatisfied by the published corpus — **all 85** projects lacked `documentationMaterial`, 13 lacked `url`, 9 lacked `contactPoint`, and one each lacked `typeOfData` and `dataLanguage` — so a literal gate would have refused every project already live. All five are now `Recommended`, with the publication requirement moved into the hint where three of them had been claiming "Required before publishing" in words while the pill said the same thing in a tier that could not be enforced.

So the rule is: **a field is `Required` only if the published corpus answers it.** `obligation`'s `UNANSWERED_BY_THE_CORPUS` is empty and its test is now a gate rather than a baseline — tier a field `Required` that the corpus cannot answer and it fails, naming the field and the count. Fix the tier or fix the data, not the constant.

What the gate adds over the conversion beside it is narrow and worth stating, because the two look redundant: every `Required` field is a non-`Option` member of `ProjectRaw`, so an **absent** one already fails `ProjectDraft::to_raw`. The gate catches the state the contract cannot see — present and empty. `keywords: []`, `description: {}`, `name: ""` and a `MISSING` sentinel all deserialize perfectly well, and none of them is an answer.

Presence is read through `obligation::is_satisfied`, the same function the section rail counts with. That sharing is deliberate: a gate stricter than the rail would refuse a submission the rail had just counted complete, and a depositor looking at "5 of 5 required" would have no way to tell which of the two was lying. All 85 published projects come out complete for both audiences, which is the same fact stated per project.

## The agent store, and how a reference is picked

Three fields hold ids rather than values — `contactPoint`, `attributions[].contributor` and `funding[].funders` — 674 references across the 85 projects. `editor_core::agents` loads the 558 committed persons and organizations, mirroring `PublishedProjects`: an explicit `load_from` returning what loaded plus one error per file. It does **not** reuse `dpe-core`'s caches, which are a process-wide `OnceLock` keyed on a global data directory: the editor's directory is `EDITOR_DATA_DIR` on its own `AppState`, so a global would make two tests with different fixtures see each other's entities.

Persons and organizations are flattened to one `Agent` at load time, because every consumer wants exactly an id, a name and a kind from both while the two contract types have different name shapes.

The picker is a **server-side search**. A row shows who it refers to by name, keeps the id in a hidden input, and offers a search box; the search re-renders that row with its matches as a real `<select>`, capped at 25, with the current choice first and selected.

It replaced an `<input list=>` pointed at one shared `<datalist>` of all 558 agents, which a reviewer reported as "seems to be a pull-down menu, but when I click it, nothing opens". Both halves were true. Chromium draws a dropdown arrow for any `input[list]`, and it filters the options against what the box already holds — which was the full id, so the only thing left to offer was the value already there. The datalist also matched ids rather than the names it displayed, and a depositor had to know `person-417` to type it.

A `<select>` of everything is what the datalist existed to avoid: the list is 31.7 KB and `attributions` reaches 56 rows on one project, so a select per row would be 1.8 MB of markup re-sent on every save. Searching first bounds it: one row's menu is at most 25 options, and a page that has not been searched carries no menu at all. The search is a named submit on the section's own form — `intent=find-agent` — so it needs no route of its own, and it re-renders with the posted body kept, which is what carries the query back into the box.

**What posts is still the id, and nothing else.** Before a search that is a hidden input holding the stored value, so an untouched save round-trips byte-for-byte with no lookup in between that could resolve differently — the property the old text input had, and the reason its value was the id rather than the name. After a search the `<select>` posts under that same name with the current choice selected, so leaving it alone is still the identity and no applier changed for any of it. An id that resolves to nobody says so in the row as well as at submit: the form is where it can be fixed, and told only at submit a depositor would have to work out which of 56 rows the refusal meant. Without JavaScript the search is an ordinary form submission and the menu an ordinary `<select>`.

Resolution is a submit gate rather than an applier's job, because a draft is allowed to hold a value that does not validate and deciding that is submit's. A dangling reference in a field nobody touched is still refused: every committed reference resolves, so a dangling one can only have arrived after the fact — an agent file removed from under a project, which is a thing to report rather than to publish.

**`funding[].funders` is a fourth agent field, and the gate used to miss it.** A grant's funders hold agent ids exactly as the other three do — 125 of them across the 85 projects — but `funding` declares `Shape::FundingRows`, so a filter over `AgentRows | AttributionRows` walked past it: the form warned about an unresolvable funder in its label and submit accepted the project anyway. `dpe-server validate` does not catch it either, because `checks::contributor_refs` reports `attributions` and `contactPoint` only. A dangling funder therefore reached a published file and rendered as a bare `organization-008` on the public project page. All 125 committed funder references resolve, so closing it refuses nothing the corpus already contains. One function, `agent_ids_in_row`, now knows how all three shapes spell a reference, so a field that refers to an agent cannot be added to one reader and forgotten in the others.

**Resolution is scoped to the project.** `AgentScope` overlays the project's own referenceable entity proposals on the published store, so a just-allocated `person-417` resolves in the picker and passes this gate while it is still only a database row. It borrows the published store rather than copying it — that snapshot is 558 agents behind an `Arc` precisely so cloning it per request does not happen. A published id wins over a proposed one, because a change proposal names an id the store already holds and the surface is meant to show the published value beside the proposed one; the listing applies the same precedence, or a change proposal would offer one organisation twice under two names.

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

**Whether the field has rows is `Shape::has_rows`, which is exhaustive.** The check used to be a `matches!` allowlist of four shapes written out at the route, and the four row shapes that landed later — `PublicationRows`, `ReferenceRows`, `TextOrReferenceRows`, `FundingRows` — were never added to it. The renderer emits an add control for every row shape, so eleven of the twenty-three controls a depositor could see answered `404` and threw the form away with it: `disciplines`, `spatialCoverage`, `temporalCoverage`, `publications` and `funding`. Nothing failed to compile and no rendering test could see it, which is why the predicate lives on `Shape` — a new shape cannot be added without answering the question — and why one test now posts to every `formaction` each section renders.

**The enhanced path posts the submitter's `formAction`.** A row control is a submit button carrying a `formaction`, which is how it reaches its own route while the whole form body goes with it — and Datastar calls `preventDefault` on the form's `submit` unconditionally, which discards the submitter's URL along with the native submission. `data-on:submit` therefore reads it back: `@post(evt.submitter?.formAction || '<action>')`. Posting the form's own action instead made every add and remove button a plain save — the route was never reached, no row changed, and nothing failed. That was the enhanced path only; with no script the browser honours `formaction` itself, so the plain path always worked and no test that did not run a browser could see it. `submitter.formAction` falls back to the form's action for a button with no `formaction`, so save, submit and the propose intents are unaffected.

Removing a row **is** the applier seeing one fewer key, so there is no separate delete. The empty marker is re-added when the last key goes: without it the field reads as absent, the applier leaves it alone, and the last removal does not stick.

A blank added row is never stored — an empty row must not reach a published file — so it lives in the form. The tile emits a hidden `{field}.row` for it, the body carries its key back, and the re-render finds it there. Nothing is held server-side between requests. Row keys are positional on a fresh render (`r0`, `r1`, …) and only have to stay stable for the life of one rendered form: order comes from the repetition of the hidden field, never from a number.

## Discarding a draft

The only thing in the service that removes one. A review that rejects a submission and a depositor who withdraws one both **keep** the draft, so without this an abandoned draft sits in RDU's list for good and keeps the hand-edit collision armed.

"Discard" rather than "delete", on the control and on the wire: what goes is the draft and never the project, and the form re-opens pre-filled from the published metadata. Refused without a stored draft, and refused while a submission is pending — the draft is what the depositor comes back to when RDU returns the project, so it must not vanish from under a live review; the refusal names the way out, which is to take the submission back first.

## A concurrent save is refused once, not silently applied

Two members of one project team is the normal case, the draft is one row, and `upsert` is last-write-wins. The form posts the revision it was rendered from, under `baseline`, and a save whose baseline no longer matches the stored row is refused with the other editor's name and the time they saved.

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
| Values with surrounding whitespace or a bare `\r` | 0 (stripped by DEV-7129; the trimming and newline rules remain for depositor input) |
| Files whose bytes differ between a CRLF and an LF submit | 26 |
| `Required` fields the corpus does not answer | 0, after re-tiering five: `documentationMaterial` (absent in all 85), `url` (13), `contactPoint` (9), `typeOfData` (1), `dataLanguage` (1) |
| Agents, and references to them | 558 persons and organizations; 674 references from `contactPoint`, `attributions[].contributor` and `funding[].funders`, all of which resolve |
| Why the picker searches rather than listing | a `<select>` of all 558 agents is 31.7 KB, and 1.8 MB across the 56-row `attributions` project; a searched row offers at most 25 |
| `typeOfData` | 5 kinds across all 85, no tail — closed |
| `dataLanguage` | 24 distinct tags against the 4 the UI offers; `la` alone in 10 projects — open |
| `contributorType` | 195 spellings of 181 roles, plus comma-stuffed entries and one biography — open |
| Reference sources | `temporalCoverage`: all 271 entries are references, from 4 sources. `disciplines`: 73 references, all `Skos` |
| Variant branches, both live | `temporalCoverage` 86 of 141 entries free text; `disciplines` 131 of 204 free text; `funding` 77 projects hold grants, 8 a single string |
| Grant members | Of 121 grants, 91 carry all three optional members and 12 only a number |
| Widest committed language map | 3 tags, against the cap of 250 |

## The decoder knows shapes, the registry knows fields

`editor_core::form` reads a posted body back into a draft; `editor_web::form::registry` says what the form knows about each field. The split is deliberate: the decoder knows *shapes* (a scalar, a language map, a closed choice, a URL slot, a list of strings, and four kinds of row) and has no idea which field is which, so choosing a shape per field belongs with the registry, keyed by the same field ids as the renderers. A field's control and its decoder are then declared together and cannot drift, and the audience check — which fields a depositor may write at all — has one home rather than one in the view and one in the handler.

The body is read as `Form<Vec<(String, String)>>`, not as a struct. `axum::Form` deserializes with `serde_urlencoded` 0.7 (via axum 0.8), which **errors** on a repeated key rather than collecting it into a `Vec`, and cannot deserialize a struct containing a `Vec` at all — so a checkbox group and a repeatable list's row keys have no struct representation. The pair list gives body order with duplicates intact, which is what opaque row keys plus DOM order need, and adds no second urlencoded parser. `serde_html_form`, which does decode repeated keys, is not a dependency of this tree.

Three rules make an untouched save a no-op, all three pinned by `editor-web/tests/untouched_form_round_trip.rs`:

- A stored `MISSING`/`CALCULATED` placeholder survives an empty submit. Those sentinels are filtered out of DPE's UI and of OAI-PMH's output, so a control holding one renders empty and an untouched form posts empty for it. 131 across the 85 files, 24 of them `endDate`.
- A value differing from the stored one only in surrounding whitespace is left alone; a genuinely new value is stored trimmed. The committed corpus (DEV-7129) no longer carries a value with surrounding whitespace, but the rule stays because a depositor's paste routinely does, and every field a declared shape reads has to round-trip one untouched. The row appliers grow a way to preserve them, too: `resolve_against` looks a submitted value up against every value the field already holds, rather than against one position, because a row key is opaque and need not map to a position. After one removal the body carries `r0` and `r2` against a two-row list, so a positional lookup would preserve one row's bytes into another.

  It prefers an **exact** byte match before the whitespace-insensitive one, which is not redundant: two values in one project's field can differ only by surrounding whitespace (`0121_societesavoie` once held `"Project Member, Data Collector"` alongside the same value with a trailing space, before DEV-7129 stripped it), and the loose comparison alone cannot tell such a pair apart, so whichever came first would rewrite the other.
- Newlines are normalised to `\n`. A native submit posts CRLF where `FormData` posts LF, so the no-JS and enhanced paths would otherwise write different bytes for the same value in 26 of the 85 files. Normalisation applies to **both sides of the comparison but only to a value being stored**, because a bare `\r` from pasted input is not representable in a `<textarea>` control, which converts it to `\n` before any submit reaches the server; the corpus itself no longer carries one.

## Where a field's shape and empty state are declared

`registry::Field` carries a `shape: Option<Shape>`, and that is the only place either is stated. `Shape` is `editor_core::form`'s, one arm per applier, and `editor_core::form::apply` is the single entry point a handler uses — so naming a shape is the only way to reach an applier, and the shape a field declares is the applier that runs.

`WhenCleared` rides *inside* `Shape::Text` rather than beside it. It is meaningful for nothing else — a language map's empty state is "no tags", and there is no placeholder to write — so carrying it separately would have let a `Multilingual` field declare one and a `Text` field declare none. Passing it per call would let a handler disagree with the registry: `Drop` on a field the contract types as a required `String` makes every ongoing project unpublishable until an end date it does not have is entered, with no test failing.

Two registry tests hold it to the data rather than to a list repeated in prose:

- `Text(Placeholder)` requires the member to be present in **all 85** committed projects, which is what a required `String` looks like; `Text(Drop)` requires it to be null or absent in at least one, which is what an `Option` looks like. That is the inversion above, checked in the direction the corpus can actually decide.
- A declared shape has to match the JSON kind the contract holds, so a `Multilingual` on a string member — a control posting under names no applier reads, and a save that is silently a no-op — fails rather than shipping.

A field with **no** shape is display-only, and written back unchanged. It used to also mean "a control that has not landed", of which there were eighteen; that set is now empty, and the registry test that pinned it exactly is what keeps it so — a new `ProjectRaw` member placed in a section without a shape fails there rather than rendering as a note nobody notices.

The note itself stays in `widgets::stated`, unreached by any field today and deliberately kept: it is the fallback such a field lands on, and a depositor who cannot find a field the published page shows would otherwise conclude the form lost it.

`untouched_form_round_trip.rs` derives its table from `FIELDS` and therefore lives in **`editor-web`**: the dependency direction is `server -> web -> core`, so a test in `editor-core` cannot read the registry at all. Deriving it is the point — a field whose shape is declared is covered automatically, where a hand-written table agrees with the registry only by inspection.

A depositor who types the word `MISSING` is refused at submit, by `form::submit::typed_sentinels`. The sentinels are recognised by an exact string match and a submitted value is stored verbatim, so typing one stores something the rest of the platform reads as "no value" — filtered out of DPE and of OAI-PMH — and because a recognised placeholder renders as an empty control, the next empty submit leaves it alone rather than clearing it. The field then reads as empty, will not clear, and is only editable by typing some other value first.

The check reads the **declared shape**, not the value, because a stored sentinel is usually correct: `endDate` is `"MISSING"` in 24 of the 85 committed projects, and `WhenCleared::Placeholder` means the editor writes one itself when a depositor clears the field. So the question is not "is this a sentinel" but "could clearing this field have produced one" — `Placeholder` yes and allowed (typing one is then indistinguishable from clearing, and has the same effect), `Drop` no and refused, `Multilingual` no and refused per tag, since an empty text drops its tag. Deriving it from the shape also answers for a project with no published counterpart, which a comparison against the published value could not.

**One number caps what a field may carry, and it is both bounds at once.** `MAX_VALUES_PER_PREFIX` is 64. As a bound on **work** it always mattered: every `FormBody` reader is linear in the number of pairs, and `entries` — which discovers a field's language tags — returns each value with its suffix rather than leaving the caller to fetch each with `get`, which would be quadratic. `DraftMultilingual` is an order-preserving `Vec` whose `get` and `set` scan, right for a map the data holds two entries of and wrong for one holding twenty thousand: 20,000 tags under one prefix measured 2.6 s of CPU in a debug build, from a single request, against Axum's 2 MB limit of roughly 100,000 short pairs.

It is now also the cap a depositor can **see**. `form::submit::over_cap` refuses a body carrying more, with a field-level error naming the field and the number, and `FormBody::exceeds_entries` is what counts — `entries` cannot, because it stops at the cap, so through it a body of twenty thousand tags and one of sixty-four are indistinguishable.

Two facts about where it runs, both load-bearing:

- **Before any applier, and on a save as much as a submit.** An applier truncates silently, so an over-cap save left to submit would store the truncated value and the submit after it would see a draft already within the cap and pass. Nothing is written on the refused branch, which is what makes "nothing was saved" true.
- **One number, not a visible cap above the work bound.** A product cap set higher would silently discard every value between the two — exactly the failure a visible cap exists to remove. So `entries`' truncation is now a fail-safe floor rather than the behaviour: unreachable through the route, kept because `FormBody` is public and a future caller might not check first.

Sixty-four is sixteen times the four tags the UI offers and twenty-one times the three the widest committed language map holds, and a test asserts the cap stays above what the corpus carries — so the day a project needs more, the cap is raised deliberately rather than a save being refused.

## Submit

`POST /projects/{shortcode}/sections/{section}` with `intent=submit` records the draft as the project's pending submission. The draft is written first and on both intents, so a refused submission costs the depositor the submission and never the editing.

Seven gates, in order:

1. **The draft must be a complete `ProjectRaw`.** A type-level failure means a member the contract requires has no value, and no per-field rule below can say anything useful about a shape that does not exist.
2. **Every `Obligation::Required` field the submitter sees must be answered**, through `obligation::unsatisfied_required`. Narrower than it looks beside gate 1 and not redundant with it: every required field is a non-`Option` contract member, so an *absent* one already failed above, and what this catches is present-and-empty — `[]`, `{}`, `""`, a `MISSING` sentinel. It reads presence through the same function the section rail counts with, and the tier it gates on is bounded by what the published corpus answers; see [Obligation is a submit gate](#obligation-is-a-submit-gate-and-the-corpus-bounds-which-fields-carry-it).
3. **Every agent reference must resolve**, through `form::submit::unresolved_agents`. The applier stores whatever id arrives, because a draft may hold a value that does not validate — this is what stops an unresolvable reference reaching a published file, where the public project page would render a bare `person-001`. It resolves against the published store **plus this project's own referenceable proposals**, so a depositor can reference an entity they have just proposed. `funding[].funders` is included; see [the agent store](./project-form.md#the-agent-store-and-how-a-reference-is-picked) for how long it was not.
4. **Every live entity proposal must satisfy its own rules**, through `proposals::check_person` and `check_organization` — the organisation rules, the person rules, and the project-role guard on `jobTitles` that nothing asks for but `dpe-server validate` enforces, so without it the editor could produce data that fails validation in a crate it never touches. It runs directly after the reference gate above, because both are about the entities a project points at and a depositor fixing one is usually fixing the other. The refusal names each proposal and each finding, with a link to the form that can fix it, the way a field outside the current section is already listed with a link to its section.
5. **No field may hold a placeholder sentinel a depositor typed**, through `form::submit::typed_sentinels`. Decided from the field's declared shape rather than from the value, because a stored sentinel is usually correct; see [where a field's shape and empty state are declared](#where-a-fields-shape-and-empty-state-are-declared).
6. **Every `temporalCoverage` entry must resolve**, through `editor_core::submission::unresolved_temporal_coverage` — the same decision `dpe-server validate` and `dpe-api-oai` apply, over the same two tables, which `AppState` reads once at startup from `EDITOR_DATA_DIR`. With no data directory the tables are empty, so every free-text period is unresolvable and the submission is refused: the fail-safe direction, since the alternative opens a pull request that fails CI in a crate the editor never touches. It re-runs on **every** submit, which is what makes a resubmission revalidated rather than trusted because it was reviewed once.
7. **The submission must change something.** The comparison is `editor_core::review::diff`, the one the review surface itself renders from, so "changes nothing" means the same thing in both places. Allowed through, an unchanged submission locks the depositor's own form on a queue entry a reviewer can only clear by rejecting it.

Two further checks run on **every** write rather than only on a submit, and therefore before these: the per-field cap, because an applier truncates silently and deferring it would let an over-cap save store the truncated value; and the concurrent-save baseline, because a save is exactly what would overwrite somebody else's work.

A refusal re-renders rather than redirecting, so nothing typed is lost, and per-field errors render beside the control they name. Submit validation is whole-project while the form is sectioned, so an error routinely names a field the depositor is not looking at — those are listed separately with a link to the section that holds them, otherwise the refusal says "the fields below say what needs changing" and nothing below says anything.

RDU submits through the same path: `User::may_reach` is already true for every project for an RDU account, so the direct-editing half was already there and this is the half that makes the result reviewable.
