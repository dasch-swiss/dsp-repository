# Project State and Online Detection

The five depositor-facing states, which of them are stored, and how Online is derived. The decision is `areas/deposit/ADR-0003`.

A depositor sees exactly five states (REQ-2.1): Draft, Submitted, In review, Approved and Online. Only three of them are stored. `submissions.state` carries Submitted, In review and Approved; Draft is the presence of a `drafts` row; and Online is derived, because nothing in the editor learns that a change has shipped except by looking at the data the deployment carries.

`editor_core::status` owns the vocabulary and the derivation, and `editor-web`'s `reconcile` module runs the comparison once at startup, inside `Editor::start`. Once is enough: the published set is baked into the image and cannot change while the process runs, so the moment a deployment carrying an approved change starts is the moment that change is Online.

## The comparison is `review::diff`

The startup comparison asks the same question the review surface asks — does this local record differ from the published project, member by member? — so it asks it through the same function rather than a second one. A separate implementation would be a second definition of "changed", and the two would drift.

It compares parsed values rather than bytes, which is what keeps language-map key order from registering as a change. That is a property of the types rather than a rule a caller has to remember: a draft holds each member as a `serde_json::Value`, this workspace builds `serde_json` with `preserve_order`, and `IndexMap`'s `PartialEq` compares by key lookup rather than by position. Nothing sorts or canonicalises before comparing, and nothing needs to.

`howToCite` and `legalInfo` (`editor_core::form::registry::GIT_OWNED`) are left out of the comparison. Both are display-only, and once the published set carries a project RDU maintains them in the repository, so a record's copy predating RDU's edit there is not a change: a created project whose licence RDU added in its collection pull request goes Online when the release carries it. The review surface still diffs them, the submit gate does not count them as a change, and the pull-request run writes the published file's values, never the record's ([Publishing to Git](./publishing-to-git.md#the-pull-request-run-is-a-rust-binary-in-this-workspace)).

## The fourth branch

REQ-2.3 names three outcomes — published only, local only, and in both. There is a fourth it admits only by omission: a project the published set has **dropped** while a local record survives. Presence alone cannot distinguish that from a new, never-published project, so the two are separated by whether the record carries the `id` and `pid` DaSCH assigns on first publication — display-only fields (REQ-1.5) that a depositor can neither set nor clear. An upstream deletion is never resolved automatically: the local record may be the only surviving copy of that work.

## What the pass writes

It deletes an approved record whose data the published set now carries (REQ-2.4), it retires an accepted entity proposal whose entity the published set now holds, and it deletes a created project's registration once the published set carries its shortcode ([Created projects](#created-projects)). Each is safe by construction — what authorises the write is the proof that the content is already published. Everything else is reported and left alone, including a record whose pull request merged with reviewer edits: REQ-2.4 can never fire for it, and RDU resolves it by discarding ([Publishing to Git](./publishing-to-git.md#what-rdu-sees-in-the-editor)).

A failure here is deliberately not fatal, unlike the RDU account bootstrap. Every project keeps working and the only symptom is a stale label; refusing to start would take the service down to avoid one.

## Online is a resting state

Online is what a published project with nothing pending *is*, not the instant of a transition. It has to be: REQ-2.4 discards the local record and a review round does not keep the approved payload, so after the discard nothing anywhere records that a change shipped. Were Online only the moment of the transition, no depositor would ever see it. A project leaves Online as soon as there is something pending again, and returns when that lands — which is also what makes the five states total over every project.

## Created projects

RDU can create a project in the editor (`/projects/new`): the published set does not carry it yet, and its draft is seeded with an `id` and `pid` derived from the shortcode ([Project representation](./project-representation.md#created-projects)). Those are exactly the members the fourth branch reads as "published once", so a created project's approved record would compare as dropped upstream. The registration in `created_projects` is what tells the two apart: `editor_core::status::classify_record` takes the set of created shortcodes, and a record for one of them that the published set does not carry is waiting for its first release, with every member it holds counted as changed. `Comparison::classify` does not read the created set. `Comparison::NewAndUnpublished` is what a local record with no `id` or `pid` compares as, such as one for a shortcode assigned to a depositor but never created; `classify_record` reports such an approved record as anomalous.

A depositor's state needs no change: a created project's draft reads Draft, its submission Submitted or In review, and its approved record Approved, as for any project.

The startup pass sweeps the created projects before it compares the records, in a pass of its own: a project published by a hand-merged file, or never submitted, has no approved record for the record loop to find. A registration whose shortcode the published set now carries is deleted, and from then on the project is an ordinary published one. Its draft is kept, as the draft of that published project: a depositor may have saved edits after the approval the release carries. A failure to sweep is counted and retried on the next start.
