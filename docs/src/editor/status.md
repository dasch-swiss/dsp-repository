# Project State and Online Detection

The five depositor-facing states, which of them are stored, and how Online is derived. The decision is `areas/deposit/ADR-0003`.

A depositor sees exactly five states (REQ-2.1): Draft, Submitted, In review, Approved and Online. Only three of them are stored. `submissions.state` carries Submitted, In review and Approved; Draft is the presence of a `drafts` row; and Online is derived, because nothing in the editor learns that a change has shipped except by looking at the data the deployment carries.

`editor_core::status` owns the vocabulary and the derivation, and `editor-server`'s `reconcile` module runs the comparison once at startup. Once is enough: the published set is baked into the image and cannot change while the process runs, so the moment a deployment carrying an approved change starts is the moment that change is Online.

## The comparison is `review::diff`

The startup comparison asks the same question the review surface asks — does this local record differ from the published project, member by member? — so it asks it through the same function rather than a second one. A separate implementation would be a second definition of "changed", and the two would drift.

It compares parsed values rather than bytes, which is what keeps language-map key order from registering as a change. That is a property of the types rather than a rule a caller has to remember: a draft holds each member as a `serde_json::Value`, this workspace builds `serde_json` with `preserve_order`, and `IndexMap`'s `PartialEq` compares by key lookup rather than by position. Nothing sorts or canonicalises before comparing, and nothing needs to.

## The fourth branch

REQ-2.3 names three outcomes — published only, local only, and in both. There is a fourth it admits only by omission: a project the published set has **dropped** while a local record survives. Presence alone cannot distinguish that from a new, never-published project, so the two are separated by whether the record carries the `id` and `pid` DaSCH assigns on first publication — display-only fields (REQ-1.5) that a depositor can neither set nor clear. An upstream deletion is never resolved automatically: the local record may be the only surviving copy of that work.

## What the pass writes

Exactly one thing: it deletes an approved record whose data the published set now carries (REQ-2.4). That is safe by construction — the comparison authorising the delete is the proof that the content is already published. Everything else is reported and left alone, including the sharp case: a record already **collected** that still differs is *stranded*, because its pull request merged with reviewer edits or was closed unmerged, and REQ-2.4 can therefore never fire for it. Startup names it in a `warn`; resolving it is an RDU decision, and the force-online and force-discard tools belong with outbound collection.

A failure here is deliberately not fatal, unlike the RDU account bootstrap. Every project keeps working and the only symptom is a stale label; refusing to start would take the service down to avoid one.

## Online is a resting state

Online is what a published project with nothing pending *is*, not the instant of a transition. It has to be: REQ-2.4 discards the local record and a review round does not keep the approved payload, so after the discard nothing anywhere records that a change shipped. Were Online only the moment of the transition, no depositor would ever see it. A project leaves Online as soon as there is something pending again, and returns when that lands — which is also what makes the five states total over every project.

## v1 edits existing projects only

Settled as a scope decision, because the review surface depends on the answer. No requirement allocates a project's `id`, `pid` or `shortcode`, and REQ-1.5 makes all three display-only, so a project created in the editor would carry three empty fields nobody can fill. There is therefore no creation surface: every project a depositor edits comes from the published set.

The `NewAndUnpublished` branch is kept and documented as unreachable rather than removed, so a later version that adds creation finds the branch instead of inventing it. This also settles what the review diff does with an absent published side: it cannot arise in v1.
