---
status: accepted
date: 2026-10-13
---

# Build-time inputs come through two channels, one consumer each

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
- **A configuration is an opaque file with a declared format.** A project's configuration carries a format
  identifier and a schema version (`cpe-config`, version N; DaTEI's TOML under an identifier of its own), and
  the capability reads version N and N-1. A schema bump is a migration deposit that DaSCH runs on the
  project's behalf, an ordinary format migration on the archive's side. The configuration is never lifted
  into RDF: the archive records that a Representation is the presentation configuration for engine X at
  schema Y, and the bytes stay opaque to the archive, to `media` and to the port.
  Not yet in code: the format identifier, the schema version and reading N and N-1 (not scheduled).
- **`sync` serves archive-shaped facts only.** ADR-0007's and ADR-0008's sentences stand as written: the
  port against `sync` "speaks archive-shaped facts as the archive records them", and a read model "carries no
  fact the projection does not". ADR-0010 is superseded by this record; the code that followed it is retired
  under DEV-7511.
- **A project folder holds code.** ADR-0007's "a project is a folder" narrows to what is code: the project's
  hook and its per-project components, with the documents that describe the project's data. Its KDL, its
  narrative and its curation are not in the folder; they come through the bytes port. A project is still
  discovered by its folder, and adding one still touches no shared registry.
- **A presentation serves what the requesting user may see, and filters as the anonymous consumer until
  an authorisation mechanism exists.** The target is that CPE and DaTEI answer each request for its
  principal, with the decision ADR-0013 records. The ports are built that way from the start: the port
  against `sync` serves every resource with its rights attached, the bytes port serves every file with its
  rights attached, the read model stores rights per row, and the presenter filters at request time, a page,
  a search, a facet count and a listing alike. No port pre-filters for the anonymous consumer: a read model
  built from a pre-filtered port cannot get to the target. A rebuild (ADR-0008) reads on no one's behalf.
  Until an authorisation mechanism exists, no request carries a principal, so the presenter filters every
  request as the anonymous consumer and serves everyone the same page.
  Not yet in code: rights on either port, rights per row in a read model, and the presenter's filter; today
  the port and CPE's store carry no rights (not scheduled).
- **An asset is fetched at request time, through `media`, never consumed at build time.** An image is an
  IIIF URL and every other file a download, both served by `media` over Vitrinli when the request comes, so
  that the authorisation check (ADR-0013) runs then. A reader stores a file's identity and builds its URL; it never
  copies, embeds or parses an asset's bytes into its read model. DaTEI's Edition files, ODD, Project CSS,
  Driving files and Tables of contents are build-time inputs, not assets: the Rebuild parses them into the
  read model and the Presenter serves rendered text. The rights a rebuild stores with each row are what
  make that consistent: a rendered text is filtered as the file it came from would be.
- **Change reaches a reader only through its port, and the port announces it.** A port carries a change
  subscription, and the trigger lives on the provider's side. In the target, `sync`'s stream consumer
  announces a project's new sequence through the port, and `media` does the same for the bucket. For the
  stand-ins, `sync` and `media` each own an operator lever, a file watch or an admin endpoint behind a
  token, that makes its adapter announce. A reader rebuilds the one project the port named, and every
  project at start; CPE and DaTEI grow no endpoint of their own, and the binary is never restarted to
  refresh data. The lever is the operator's way in today, and there is no second trigger to remove later.
  Not yet in code: the change subscription on either port, the stand-ins' levers and a rebuild of one named
  project; until they land, a restart is the rebuild (DEV-7622).
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
- **A configuration without a declared format** — rejected: a schema change in CPE or DaTEI would break
  every deposited configuration at once, with no record of which version a file was written for.
- **Presentations that hold public data only, from ports that pre-filter for the anonymous consumer** —
  rejected: a read model built that way cannot serve a signed-in user what they may see, so reaching the
  target would mean a second port and a different read model. Rights per row cost a predicate in every
  search, facet and count, which ADR-0013 accepts.
- **An operator endpoint or signal on CPE or DaTEI that re-runs activation** — rejected: a second trigger
  beside the port, to be removed once the port carries change. The stand-ins' lever lives with the provider
  and makes the port announce, so it is the trigger the target keeps.

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
- ADR-0013 records the authorisation decision behind the presentation clause: the archive owns the policy
  data, one shared function decides, and each capability enforces in its own query layer.
- The port against `sync` and both bytes ports gain rights per resource or file (not scheduled), and every
  port a change subscription (DEV-7622).
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
`Fake<Port>` for each of its ports (**static-analysis**); "no port pre-filters", "the presenter filters at
request time", "no reader grows a trigger of its own" and "an asset is never consumed at build time" stay
with review.
