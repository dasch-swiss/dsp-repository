---
status: accepted
date: 2026-10-13
---

# Build-time inputs come through two channels, and a presentation holds public data only

Three records of the Access Area pull against each other where a reading capability needs input that is
not an RDF fact of the archive. ADR-0008 makes `sync` "the Access Area's single consumer of the archive's
data products", while `vitrinli/CONTEXT.md` and `areas/archive/CONTEXT.md` have `media` read Service Files
from the Access bucket, which are data products too (DEV-7442). ADR-0007 puts a CPE project's configuration
in a folder of the capability and ADR-0008 lets a read model be fed by its port and nothing else, so
Incunabula's curation, which is neither a fact of the archive nor configuration, had nowhere to go; ADR-0010
proposed, as an interim, that `sync` serve it beside the facts (DEV-7488). DaTEI's record left the same
question open for its TOML configuration and for which of an edition's files are archived (ADR-0009). This
record settles the three together and says what an Access-Area capability may do before the archive exists. The decision, phrased so a violation is describable
in code:

- **The archive publishes on two channels, and each has one consumer in the Access Area.** The projection
  stream, snapshots and deltas announced on the bus, is consumed by `sync` alone, which writes the
  projection. The Access bucket, where Service Files land, is consumed by `media` alone, which serves them.
  ADR-0003's and ADR-0008's "single consumer of the data products" means one consumer per channel; neither
  capability reads the other's channel, and no third capability reads either.
- **A deposited file reaches a reader as bytes through a port against `media`.** A project's configuration
  (CPE's KDL, DaTEI's TOML), its curation and its narrative are files the project or DaSCH deposits on the
  project's behalf. The archive preserves them and publishes each as a Service File that is the file
  itself. A reading capability that needs them declares a port for bytes, as DaTEI already does
  (ADR-0009), and `media` implements it beside its data. CPE declares such a port too. The port carries
  files by shortcode and name, whole or refused, and nothing else: no fact, no meaning of a key.
- **`sync` serves archive-shaped facts only.** ADR-0007's and ADR-0008's sentences stand as written: the
  port against `sync` "speaks archive-shaped facts as the archive records them", and a read model "carries no
  fact the projection does not". ADR-0010 is superseded by this record; the code that followed it is retired
  under DEV-7511.
- **A project folder holds code.** ADR-0007's "a project is a folder" narrows to what is code: the project's
  hook and its per-project components, with the documents that describe the project's data. Its KDL, its
  narrative and its curation are not in the folder; they come through the bytes port. A project is still
  discovered by its folder, and adding one still touches no shared registry.
- **A build-time read runs as the anonymous consumer.** A rebuild (ADR-0008) reads from `sync` and `media`
  on no one's behalf, so both ports serve it what an anonymous consumer may see and nothing else. This is
  what the Access Area can enforce today, when neither CPE nor DaTEI has an authentication mechanism.
- **A presentation holds public data only, and authorises nothing.** CPE's and DaTEI's read models hold
  what an anonymous consumer may see, and their presenters check no right per request: a page, a search,
  a facet count and a listing are the same for everyone. This is a boundary commitment, not a consequence of
  the previous clause: a read model that might one day hold restricted data would carry rights per row and
  filter every search and count on them, and the Access Area chooses not to pay for that. It may be
  revisited when a project needs a restricted presentation; the revisit is a new record, never a widening in
  place.
- **An asset is fetched at request time, through `media`, never consumed at build time.** An image is an
  IIIF URL and every other file a download, both served by `media` over Vitrinli when the request comes, so
  that the authorisation check runs then. A reader stores a file's identity and builds its URL; it never
  copies, embeds or parses an asset's bytes into its read model. DaTEI's Edition files, ODD, Project CSS,
  Driving files and Tables of contents are build-time inputs, not assets: the Rebuild parses them into the
  read model and the Presenter serves rendered text. The public-only clause is what makes that consistent.
- **Change reaches a reader only through its port.** A reader rebuilds when its port announces change, or
  at start; there is no operator endpoint or signal that re-runs activation. Until a port carries change
  (ADR-0008), a restart is the rebuild.
- **Before the archive exists, each capability's minimum is a hand-fed stand-in for its channel.** `sync`'s
  committed snapshot stands in for the projection stream, as ADR-0007 has it; `media`'s committed or
  on-disk directory, laid out as the Access bucket will be, stands in for the bucket (DEV-7512). Each is
  behind the port and adapter the real channel will later stand behind, fed by hand from today's systems,
  with its provenance recorded. Changing a project's presentation in the interim means editing a file in
  the stand-in and rebuilding; once the archive exists it means a deposit, which is the target's cost and
  is accepted.

## Considered Options

- **Two channels, one consumer each; deposited files through a bytes port against `media` (chosen).**
- **`sync` as the sole consumer, bytes through a port against `sync`** — rejected, as ADR-0009 rejects it:
  file bytes would flow through a capability built for the projection's RDF facts, and `sync` would become a
  second thing for every reader that needs a file.
- **Curation through `sync` beside the facts (ADR-0010)** — superseded: it loosened "archive-shaped facts
  only" for one kind of value and put a hand-authored file with `sync`'s data, where a second one would have
  followed; once a deposited file is a Service File, the bytes port carries curation as it carries
  configuration, and the loosening is not needed.
- **Configuration and curation authored in the project folder as a named second feed** — rejected: the
  project folder would hold durable, hand-edited input beside code, and a rebuild against a `Fake<Port>`
  would no longer cover what the capability serves (ADR-0008's reason for "the only feed"). It was the
  shape of the prototype.
- **Configuration as facts in the projection** — rejected: a KDL or TOML document is not a fact of a
  resource, and lifting it into RDF would make `sync` carry a consumer's shape, which ADR-0007 and ADR-0008
  rule out.
- **Read models that carry rights, so a presentation may hold restricted data** — not chosen now: the cost
  lands in every search, facet and count; recorded as revisitable rather than as a rejected option.
- **An operator endpoint that re-runs activation** — rejected: a second trigger beside the port that would
  survive the port carrying change.

## Consequences

- ADR-0010 takes `status: superseded by ADR-0012`. It was never accepted; the code that followed it
  (`ProjectSnapshot.curation`, the contract's curation rules, `sync-store`'s curation reader and the committed
  curation file) is retired under DEV-7511, which moves the file to `media`'s stand-in and the reader to
  CPE's side of the bytes port. Until then the code follows a superseded record and says so.
- ADR-0003, ADR-0007, ADR-0008 and ADR-0009 each gain a dated `## Amendment` pointing here. ADR-0007's
  "a project is a folder" narrows; ADR-0008's single-consumer sentence narrows to per channel and its "only
  feed" clause admits a second port of the same capability against `media`; ADR-0009's two open questions
  close; ADR-0003's Chischtli clause gains the per-channel reading.
- `cpe-ports` gains a bytes port and `media` its first adapters (DEV-7511, DEV-7512). `areas/access/media`
  arrives as the second stand-in, with a `MEDIA_DATA_DIR` beside `SYNC_DATA_DIR`.
- DEV-7442 and DEV-7488 are closed by this record.
- `areas/archive/CONTEXT.md`'s **Data product** names the two channels; `vitrinli/CONTEXT.md`,
  `areas/access/CONTEXT.md`, `areas/access/cpe/CONTEXT.md`, `areas/access/sync/CONTEXT.md` and
  `areas/access/datei/CONTEXT.md` follow, and `ARCH-MAP.md`'s entries for `cpe`, `sync`, `media` and
  `datei` with them (`/dune:dune-map`).
- The root `CONTEXT.md`'s **Service File** gains the case of a file whose delivery form is itself: a
  configuration, a TEI file, a curation file.
- Incunabula's `config.kdl`, `narrative/` and curation leave CPE's project folder and `sync`'s data for
  `media`'s stand-in directory (DEV-7511, DEV-7512). The incubator's vendored copy follows when it
  re-vendors.

Enforced by: review until the crates exist; then Bazel `visibility` per ADR-0001 for "one consumer per
channel" and for "a project folder holds code" (**structure**); `check-cpe-seam.sh` for the `media` edge as
it has the `sync` rule (**static-analysis**); one test per read model that rebuilds it from empty against a
`Fake<Port>` for each of its ports (**static-analysis**); "as the anonymous consumer", "public data only"
and "an asset is never consumed at build time" stay with review.
