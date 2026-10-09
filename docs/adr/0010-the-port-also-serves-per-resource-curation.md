---
status: proposed
date: 2026-10-09
---

# The port also serves per-resource curation

ADR-0008 lets a reading capability keep a read model that is fed by its port against `sync` and by nothing
else, and ADR-0007 makes CPE's store its first instance. Both say that the port carries archive-shaped facts
only. Incunabula's store needs input the archive does not record: what a Book's public URL is, which office
lane it is filed under, which of the project's test Regions are shown. Its editors authored those values, and
in the prototype they sit in files beside the project that the port does not serve. Two rules of the earlier
records therefore pull against each other. A file CPE reads beside the port is a second feed, which ADR-0008
rules out; a port that serves the values carries something that is not a fact of the archive. DEV-7488 decided,
as an interim, to keep "the only feed" and to loosen "archive-shaped facts only". This record states that
decision so that it can be accepted or rejected. It is **proposed**: Ivan accepts or rejects it under DEV-7488,
and the code that follows it is provisional until then. The decision, phrased so a violation is describable in
code:

- **The port also serves a project's per-resource curation, as an interim.** Curation is what a project's
  editors authored about one resource and the archive does not record. `cpe_ports::ProjectSnapshot` carries it
  as `curation`, beside its resources and list nodes and never inside a `Resource`, which stays what the
  archive records. A curated value is a resource IRI, a key, an optional language and a text.
- **This changes one sentence of ADR-0008 and one of ADR-0007.** ADR-0008: "The port speaks archive-shaped
  facts; the remodelling lives with the reader", with its consequence that a read model "carries no fact the
  projection does not". ADR-0007: "The port speaks archive-shaped facts as the archive records them (classes,
  typed values, links, files, ordered membership), never CPE's presentation model". A slug or a teaser is
  presentation data, so neither sentence holds for a curated value. Both hold for every other DTO of the port.
- **The reason behind those sentences is kept.** Both records reject a `sync` that serves a consumer's shape,
  because the remodel would then live in `sync`, every project would edit `sync` to change how it is
  presented, and `sync` would become a second writer of the reader's model. None of that follows here: `sync`
  holds the values, gives no key a meaning and remodels nothing. Changing how a project is presented means
  editing a data file and the project's KDL, never `sync`'s code.
- **The port gives a key no meaning.** Which keys exist, which class carries which key, whether a value is
  required and what a text may hold are rules of the project, checked by the reader. The port's contract
  checks only what one snapshot shows: a curated value names a resource of the snapshot, a resource, key and
  language occur once, a key and a language are well-formed names, and a text is not empty. An adapter names
  no project's key in its code.
- **The line is "keyed by one resource".** A value keyed by one resource is curation and comes through the
  port. Everything project-wide is configuration and stays in the project's folder, in its KDL (ADR-0007, "a
  project is a folder"): vocabularies with their labels and order, rights constants, the mapping from archive
  shape to presentation model. DEV-7488 speaks of "configuration and curation"; this record reads it narrowly,
  as curation only, and asks Ivan to confirm that reading.
- **What stays true.** The reader is the read model's only writer and its port the only feed. The read model
  is derived state, rebuilt from empty against the port. `sync` stays the Access Area's single consumer of the
  archive's data products.
- **`sync` starts at its minimum, as for the facts (ADR-0007):** one hand-authored file per known project,
  committed beside the project's snapshot, with a person as its single writer. A project without its curation
  file is unavailable, never served as a project without curation. `sync` serves the file's values verbatim or
  refuses the project; it never repairs one.

## Considered Options

- **The port serves per-resource curation beside the facts (proposed).**
- **Curation in the project's folder, read by CPE beside the port** — the question DEV-7488 answered the other
  way for now: the read model would have a second feed, which ADR-0008 rules out, and a rebuild against a
  `Fake<Port>` would no longer cover what the capability serves.
- **A second port method for curation** — rejected: it keeps `ProjectSnapshot` purely archive-shaped, but
  curation names resources of one snapshot, and two calls can read two states. "Whole or refused" would have
  to span two calls, and the contract could not check a value that names a missing resource.
- **Curated fields on `Resource`** — rejected: a dangling value becomes impossible, but authored input then
  sits inside the DTO that restates the archive, and the two can no longer be told apart.
- **Typed fields per project** (a slug, an office) — rejected: `sync` would know one project's vocabulary, and
  every new key would be a change of the port. This is the option ADR-0007 and ADR-0008 reject, restated.
- **Project-wide configuration through the port as well** — rejected: ADR-0007 puts the configuration in the
  project's folder, and nothing about a vocabulary or a rights line is keyed by a resource of the snapshot.
- **Amendments to ADR-0007 and ADR-0008 alone, without a new record** — rejected: ADR-0006 gives a decision
  that changes rather than narrows a record of its own. What the two earlier records take on acceptance is in
  Consequences.

## Consequences

- Until this record is accepted, the code that follows it is provisional, and `ProjectSnapshot.curation` says
  so. A rejection removes the record together with the code: the field, the contract's curation rules,
  `sync`'s reader and the committed curation files.
- Acceptance sets `status: accepted` here and adds a dated `## Amendment` to ADR-0007 and to ADR-0008. Each
  amendment quotes the sentence named above, says that it no longer holds for curated values, and points to
  this record. Neither record is edited before that.
- ADR-0007 and ADR-0008 stay `accepted`; neither takes `status: superseded by`. That departs from ADR-0006,
  which gives the status to a record whose decision changes: here one sentence of each changes and every other
  clause stands. Whether that form is right is part of what Ivan accepts.
- `main` carries a proposed record that the code already follows. That is chosen over code that contradicts
  two accepted records without saying so.
- The acceptance criterion of the port's first plan, "No DTO names a CPE presentation concept", no longer
  holds for `CuratedValue`; it holds for every other DTO. Specs are records of their time and are not edited.
- A curated value's key is a string the compiler does not check. A key that no reader knows, or a key on a
  resource of the wrong class, is caught by the reader's validation at activation (ADR-0007, "a project fails
  alone"), not by the port.
- A fault in a hand-authored curation file makes its project unavailable, as a fault in its snapshot does.
- `areas/access/cpe/CONTEXT.md` and `areas/access/sync/CONTEXT.md` gain the vocabulary, and `ARCH-MAP.md`
  lists the curation file as durable state of `sync` with a person as its single writer (`/dune:dune-map`).

Enforced by: `cpe_ports::contract::violations`, which every adapter's output must pass, for the shape of
curation (**static-analysis**); `sync-store`'s tests for the file's format and for the committed curation of
each known project (**static-analysis**); "an adapter names no project's key", for the committed keys written
as string literals, by `sync-store`'s `test_source_outside_tests_names_no_committed_curation_key`
(**static-analysis**); the rest of that rule and the line between curation and configuration stay with review.
