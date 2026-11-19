---
title: "VRE bridge: VRE project data into the Access Area's stand-in channels"
date: 2026-11-19
author: "Balduin Landolt"
status: reviewed
linear_project: P-DEV-301
repositories:
  - dsp-repository
  - dsp-api
  - ops-deploy
  - dsp-incubator
---

# VRE bridge: VRE project data into the Access Area's stand-in channels

## Normative sources

In precedence order:

1. **The live ADRs**, as amended by the rewrites this PRD calls for (Decisions to record), chiefly ADR-0002
   (areas and their boundaries), ADR-0003 (one binary per area), ADR-0012 (how inputs reach a reader) and
   ADR-0013 (access rights). The VRE bridge feeds the stand-ins ADR-0012 describes; it adds no channel.
2. **The interim format**, as `FORMAT.md` defines it once `dao-lift` is ported into this repository, amended
   by Story 2 (access rights and data classification) in the change that ports it. Until then, the incubator
   commit `sync`'s `PROVENANCE` pins governs.
3. **This PRD** governs everything else: what the bridge reads, what it writes and where, when, and what it
   refuses.

## Context

The archive (Spycherli) will publish a projection stream that `sync` consumes and an Access bucket that
`media` consumes; CPE and DaTEI rebuild from those two channels. The archive is the last thing built. Until
it exists, the VRE (DSP-API) stays the source of truth for stored data, as it has been for years, while
presentation moves to CPE and DaTEI in this repository.

Today that works for one project only. 0803 (Incunabula) reaches CPE as a committed snapshot that a person
made from a stage dump with the incubator's `dao-lift` and checked into `areas/access/sync/data/0803.nq`;
`sync-store` serves only the shortcodes in its constant `KNOWN`, today `0803`. `media`'s stand-in directory
(`areas/access/media/data/0803/`: `config.kdl`, `curation.csv`, `narrative/`) is filled by hand the same way,
and `access-server` carries both in its image. That is acceptable for a project that does not change, but it
does not scale, and it cannot work for a project that still edits its data: every change needs a person, a
commit and a release. DaTEI's first projects (086A, then the Keller edition 083C) and further CPE projects
need their data on the server now.

The **VRE bridge** is a capability of the Deposit Area that does the hand-feeding automatically: for each
project on an allowlist, it notices when the project changed in the VRE, fetches it, turns it into the
interim format, and writes it, with the files and configuration the readers need, into the locations that
`sync` and `media` read. Data ingress belongs in the Deposit Area, so the bridge is the area's third
capability, after `identity` and the editor, and runs in `deposit-server`, the area's one binary (ADR-0003).
It is temporary: when the archive exists, deposits replace it.

P-DEV-301 ("VRE→Archive transformation pipeline") is an older Linear project picked up again for this; this
PRD redefines it.

### What exists to build on (verified 2026-11-19 and 2026-11-20 unless marked)

- **Change detection.** `GET /v2/resources/info` with `X-Knora-Accept-Project` and `cutoff=<instant>` lists
  the resources changed at or after an instant, deleted and non-visible ones included, without
  authentication and without paging, and returns an `asOf`, the server's time read before the query
  (DSP-API spec `2026-10-23-resources-info-incremental-sync`, DEV-7632). DSP-API documents an overlap: the
  next `cutoff` is the previous `asOf` minus 60 seconds, and an offset `+` must be sent as `%2B`. DEV-7632
  is in DSP-API since v40.6.0; DSP release `2026.11.04` (API v40.7.0) is pinned for stage and prod in
  ops-deploy (verified by the pins, not by a live probe). DSP-API has no project-level modification time;
  this endpoint plays that role. Its known gaps: ontology changes do not bump dates, hard-erased resources
  are invisible, and client-supplied back-dated dates are missed.
- **Dump.** `POST /v3/projects/{iri}/exports` (the project IRI URL-encoded; asynchronous: `202` with the
  export's `id` and `status`, then status, then download, then delete) returns a BagIt zip of knora-base
  N-Quads (`data/rdf/data.nq`, `ontology-N.nq`, `admin.nq`, `permission.nq`, `bag-info.txt`), assets skipped
  with `skipAssets=true`: the dump `dsp-cli`'s `CONTEXT.md` names. A server has one export slot (`dsp-cli`:
  "Dump slot (server-wide)"): a second request gets `409 export_exists` with the occupant's id, whoever
  started it, and a finished or failed export keeps the slot until it is deleted. `admin.nq` holds password
  hashes and email addresses. It is `dao-lift`'s input today.
- **Machine identity, provisional.** DSP-API authenticates users only; every dump today is made on a
  person's SystemAdmin token (`just fetch-records`, `dsp vre project dump`). DEV-7936 adds a service identity
  that is not a user: dsp-api PR #4448, open and unreviewed, so its contract is provisional. A service signs
  an Ed25519 JWT (`alg` `EdDSA`, header `kid`; `iss` and `sub` the service's name, `aud` the server's external
  base URL, `iat`, `exp`, `jti`; at most 15 minutes) that the four export endpoints alone accept. Each server
  trusts the public keys listed in `KNORA_WEBAPI_SERVICE_IDENTITY_TRUSTED_KEYS`. A service sees only its own
  exports: another caller's export id answers `404`.
- **Readers.** `sync-store` reads `<SYNC_DATA_DIR>/<shortcode>.nq` on every call and knows the projects in
  `KNOWN`. `media-store` serves `<MEDIA_DATA_DIR>/<shortcode>/<name>`, knows a project by its directory,
  accepts only names `cpe_ports::is_file_name` accepts (segments of `[A-Za-z0-9._-]`, none starting with `.`),
  and refuses any symlink below its directory. CPE activates once per start: `Cpe::activate` loads every
  project and replaces what is served as a whole. ADR-0012, as it read before this PRD's rewrite, decided
  that change reaches a reader through its port, announced by an operator lever on `sync` and `media`, and
  that "the binary is never restarted to refresh data"; it listed under _Not yet in code_ that neither port
  announces change yet and "a restart is the rebuild" (DEV-7622). DEV-7514 moves both data directories from
  the image to host mounts.
- **Topology.** `deposit-server` runs on `deposit-{stage,prod}-01` (no dev host) and `access-server` on
  `repository-{dev,stage,prod}-01`, different VMs. How a location both reach is provided is infrastructure's
  answer, still pending; a network file system (NFS) is likely.
- **Hidden projects.** `ACCESS_HIDDEN_PROJECTS` (DEV-7919, merged 2026-11-19) keeps a project out of
  overviews and search indexes; it is not access control. ops-deploy sets it to `0803,083C,086A` in every
  environment.

## Goals

- Once a project is on the allowlist, its VRE data reaches CPE and DaTEI without a person, a commit or a
  release. Limit until DEV-7948: a CPE project new to the Access Area still needs its project folder in
  `access-server`'s image, so its first appearance needs one release.
- Only data the VRE treats as public reaches the Access Area, and the format carries access rights and data
  classification from the start, so that restricted data can follow later without a format change.
- A reader never observes a half-written or mixed project, and a failed refresh keeps the last good state.
- The data volume each project produces is measured, so the allowlist grows by evidence.
- Replacing the bridge with the archive changes the producer, not the consumers, except DaTEI's use of the
  file manifest: its IRIs and the VRE's IIIF URLs become ARKs and `media` over Vitrinli.

## Core features

1. **Allowlist.** A deployment's list of projects to refresh; each entry names the project's shortcode and the
   VRE server it lives on (prod, stage or a test server). Projects go on the list one at a time: finished
   projects first, edited projects later. That is configuration, not code: the bridge treats both alike.
2. **Nightly run.** Once a night the bridge checks each allowlisted project for changes since its T and
   refreshes the changed ones. An operator can refresh one project on demand.
3. **Refresh.** Run the change check; take a dump (assets skipped) with the service identity and delete the
   export on the server; check it (the public guard and the file checks); transform it with the ported
   `dao-lift` into an interim snapshot; download the text files the project's DaTEI file mapping names,
   without credentials; write the project's new generation in both subtrees and point its Ready tokens at it,
   `media` first. Formatted text carries the XML DSP-API stores in `knora-base:valueHasXml`; standoff is not
   read, and a formatted text value without that XML refuses the project.
4. **Shipped project inputs.** Files the VRE does not hold (CPE's `config.kdl`, curation and narrative;
   DaTEI's `datei.toml`, `file-layout.xml` and file mapping; Keller's files) ship in the bridge's package, per
   project, and the bridge writes them into the project's `media` generation. The bridge's package is their
   one authoritative copy.
5. **Ready token.** One per project in each subtree, naming the project's current generation and holding its
   provenance (T, the server, the interim format's version, the transformation's version and a digest of the
   project's shipped inputs). It is replaced by one file rename. A project is known to `sync` and to `media`
   exactly when its Ready token exists, and is read only from the generation its token names. The tokens are
   the bridge's only state about projects; beside them it keeps a lock file and the date of its last nightly
   run, on local disk.
6. **Refusal keeps the last good state.** A failed or refused refresh leaves that project's Ready tokens, and
   so the data readers see, as they were; other projects are unaffected.

## Vocabulary

Terms this PRD introduces or fixes; they go into the `CONTEXT.md` files with the code.

- **VRE bridge** — the Deposit Area capability described here (`areas/deposit/vre-bridge/`, crates
  `vre-bridge-*`). _Avoid_: sync (the Access Area's `sync`), pipeline, ingest (the archive's word).
- **Refresh** — the bridge's unit of work for one project: change check, dump, checks, transformation,
  downloads, writing a generation, replacing the Ready tokens. _Avoid_: sync.
- **Nightly run** — one pass of the bridge over the allowlist.
- **T** — the `asOf` of the change check a refresh started with: the instant the project's data is from.
- **Dump** — as `dsp-cli`'s `CONTEXT.md` defines it: the BagIt zip of a project DSP-API's export endpoint
  returns. (Kept for consistency with `dsp-cli` and `sync`.)
- **Selection** — what an interim snapshot is made from: the dump's current resources and values, those not
  superseded through `knora-base:previousValue` and not deleted, without the values of a deleted resource.
- **Public guard** — the check on a dump's selection that refuses a project when anything in it is not public
  or restricted view as REQ-2.4 to REQ-2.5 define; it lists every violation.
- **Interim format** (redefined) — the shape of an interim snapshot, defined by `FORMAT.md` in the VRE bridge;
  its version, an integer starting at `1`, is recorded in each Ready token, and `sync` checks it. _Avoid_: DAO.
- **Interim snapshot** — one project's data in the interim format, as the bridge writes it. Replaces `sync`'s
  _Committed snapshot_ once 0803 comes through the bridge.
- **Generation** — one refresh's output for a project in one subtree: the directory
  `<subtree>/<shortcode>/<generation>/`, written whole before any Ready token names it, and never changed
  after. The same generation name is used in both subtrees.
- **Ready token** — the per-project, per-subtree file `<subtree>/<shortcode>/.ready` that names the
  project's current generation, holds its provenance and marks it known; replaced by one file rename.
- **Known project** (`sync` and `media`, redefined) — a project whose Ready token exists in that subtree.
- **Allowlist** — the deployment's list of projects the bridge refreshes, each with its VRE server.
- **Public** — the anonymous user may view the resource, value or file as a whole (DSP-API: view permission or
  better for `knora-admin:UnknownUser`).
- **Restricted view** — DSP-API's restricted-view permission for the anonymous user on a raster still image's
  file value (`knora-base:StillImageFileValue`): the image is viewable at a reduced size, which the VRE's Sipi
  enforces. Restricted view on any other value or file counts as not viewable, because the VRE serves other
  media in full under it (DEV-7244).
- **Access rights** — rights as ADR-0013 records them: per Representation version, served with each resource
  and file, never per value. _Avoid_: permission.
- **Data classification** — a second, independent value with the same granularity and travel: how sensitive
  the data is. Today one value exists, `unclassified`.
- **DaTEI file mapping** — the hand-made, per-project input of a DaTEI page: Logical path → the IRI of the VRE
  resource that holds the file (`file-mapping`, version 1; `areas/access/datei/CONTEXT.md`). _Avoid_: mapping
  alone (DaTEI's Driving file is a different artefact).
- **File manifest** — what the bridge generates from the DaTEI file mapping, `file-manifest.json`: Logical path →
  Stored name or IIIF reference, with the IRI it came from.
- **Stored name** — the VRE's internal file name, under which a downloaded file is stored in `media`.

## Decisions to record

The bridge changes who writes the Access Area's stand-ins, how two areas integrate, and where the interim
format lives. Each record below is rewritten in place (`docs/adr/CLAUDE.md`) in the same pull request as this
PRD, with _Not yet in code_ lines for what the code does not do yet:

- **ADR-0002, the cross-area path.** Today areas "integrate over the wire, never by Rust import", with
  `areas/access/dpe/server/data` as "the one file path either area names in the other's tree", and each service
  is the sole reader and writer of its durable state. After: the stand-in locations shared between
  `deposit-server` (writes) and `access-server` (reads) are a second named cross-area path, for the stand-ins
  only, ending with them; so is the `just check` pin of the `media` 0803 fixture against the bridge's shipped
  inputs. `ARCH-MAP.md`'s Conventions and its single-writer entries for `sync`'s and `media`'s data change with
  it; ADR-0002's "Today identity and the metadata editor" line gains the bridge, and its list of `shared/`
  crates gains the stand-in crate (below).
- **ADR-0012, the stand-ins.** Today each stand-in "is fed by hand from today's systems, and records its
  provenance", and changing a presentation "means editing a file in the stand-in and rebuilding". After: the
  VRE bridge writes the stand-ins and records provenance in each Ready token; changing a presentation means a
  change in the VRE, or a change to the bridge's shipped inputs and a `deposit-server` release. That cost is
  accepted. A poll of the Ready tokens by their own reader is the stand-ins' lever.
- **ADR-0013, who writes rights.** "No capability authors, overrides or infers a right": the bridge writes
  the rights it found ("public", or "restricted view" for a raster still image the anonymous user may see only
  at reduced size) as the archive's stand-in for policy data, and only for data the public guard let through.
  The same record introduces data classification: a value beside the access rights, carried the same way,
  `unclassified` until a classification level is agreed with a depositor.
- **ADR-0003, the stand-ins' description** ("hand-fed stand-in … committed directory", "committed N-Quads
  snapshot"), its Deposit capability list and Deposit entry crates (today `web` crates only), and its `server`
  role, which today holds routes and now also covers an entry crate without any. The facade owns a router only
  where the capability has routes, and the rejected "`server` host crate per Deposit capability" option is
  restated so that it does not cover a capability whose `server` crate has no routes.
- **ADR-0007, the interim format.** Today the facts follow "one N-Quads file per project", and `FORMAT.md`
  "stays in the incubator, so the format is visibly neither DAO nor a commitment of this repository", pinned by
  `PROVENANCE`. That reason held while the producer was an incubator tool run by hand. The producer now runs in
  this repository's deployment, and a format specified outside the repository that produces it can drift from
  it unnoticed. The format stays visibly interim: it lives in the VRE bridge, a capability declared temporary,
  keeps the name "interim format", and moves to DAO when DAO is published. The pin becomes the format version
  in the Ready token, which `sync` checks. This also reverses the minimal-sync PRD's "the interim format
  carries no users, permissions": ADR-0012 and ADR-0013 have since required rights on every port.
- **The shared crates.** `is_shortcode` and `is_file_name` move out of `cpe-ports` into `shared-names`, a
  permanent crate, because they outlive the stand-ins (`media/CONTEXT.md`: "moving them out of `cpe-ports` is
  an architecture change, made by an ADR"). The stand-in layout and the Ready token's format, reading side,
  go into `shared-stand-in`, which ends with the stand-ins; the writing side lives only in `vre-bridge-*`.
  The record is ADR-0002's rewrite above; `media/CONTEXT.md`'s closing line changes with it. CPE's copyable
  closure (ADR-0007) gains `shared-names`, and `shared-stand-in` through `sync-store`.
- **`areas/deposit/ADR-0004`.** "The Deposit Area is two capabilities" becomes three. The VRE bridge has no web
  crate; its entry crate is `vre-bridge-server` in ADR-0003's `server` role, with no routes. `deposit-server`
  depends on that crate only, held by Bazel `visibility` (the bridge's other crates grant `deposit-server`
  nothing), as every capability edge is.
- **`areas/deposit/ADR-0005`, new.** The VRE bridge itself: what it refreshes and how, the generation
  directories and Ready tokens, the public guard, the shipped inputs, and the two roles it adds to ADR-0003's
  (`lift`, the ported transformation with `FORMAT.md`; `client`, its DSP-API client).
- **ADR-0012 and ADR-0008, change.** The lever becomes a poll of the reader's own Ready tokens, which the
  bridge triggers by replacing or removing them: `sync` announces the project, CPE re-activates (DEV-7622, in
  increment 1). The target stays that a reader rebuilds the one project announced; until the per-project
  rebuild exists, CPE re-activates every project on an announcement and keeps its previous activation when
  the new one fails. Their _Not yet in code_ lines keep DEV-7622 until increment 1 lands, then shrink to the
  per-project rebuild and `media`'s announcement for DaTEI (DEV-7938).

## Repository impact

- **dsp-repository**
  - `areas/deposit/vre-bridge/`: the new capability with its `CONTEXT.md`, `CLAUDE.md` and the per-project
    shipped inputs; `deposit-server` constructs it and offers an operator's refresh as a subcommand.
  - `shared-names` (`is_shortcode`, `is_file_name`) and `shared-stand-in` (the stand-in layout and the Ready
    token's format, reading side).
  - `dao-lift` and `FORMAT.md` ported in from the incubator, as the CPE engine was, with the access-rights and
    data-classification amendment and the two-phase guard (selection, then permissions).
  - `areas/access/sync`: each project becomes a directory of generations with its Ready token; projects are
    known by Ready token instead of `KNOWN`; the format version in the token replaces the `PROVENANCE` pin;
    `sync` polls its tokens and announces a changed project; `CONTEXT.md` updated (Known project; its Interim
    format entry, which names the incubator's `dao-lift` and the pin, points to the root definition; Committed
    snapshot retired in favour of Interim snapshot, in the prose and in `LiveArchiveProjection`'s docs alike).
  - `areas/access/media`: projects known by Ready token instead of by directory, read from the generation the
    token names; the token's name starts with `.`, so the port never serves it; `CONTEXT.md` updated,
    including why the symlink check's race window stays accepted (the only writer is the bridge, which writes
    no symlinks).
  - `areas/access/cpe`: both ports serve a project's provenance; `sync`'s port announces change; CPE
    re-activates on an announcement and keeps its previous activation when the new one fails or the
    provenance does not hold still.
  - The committed 0803 data stops being a deployment input. 0803's `media` files move into the bridge's
    shipped inputs; the committed copies become `testonly` fixtures, each with a Ready token, and a check
    script in `just check` (not a Bazel grant across the areas) pins the `media` fixture byte for byte to the
    shipped inputs. The incubator re-vendors from the shipped inputs.
  - `areas/access/server`: the image carries neither data subtree (`BUILD.bazel` `image_data`, `env`), kept
    out by `testonly` data targets, which a production target cannot depend on; `docs/src/dpe/operations.md`'s
    environment table.
  - `ARCH-MAP.md` and the ADRs under Decisions to record; the root `CONTEXT.md`: `## Shared` gains the terms
    both areas speak (Interim format, Interim snapshot, Generation, Ready token, Access rights, Data
    classification), and its opening exception for shared contract terms widens from the research-metadata
    contract to the stand-in contract. Known project stays per capability.
- **dsp-api** — DEV-7936, the service identity (PR #4448); DEV-7632 deployed on every server the allowlist
  names.
- **ops-deploy** — DEV-7514 extended: a `sync` and a `media` location written by `deposit-server` and mounted
  read-only into `access-server`, each path and size a per-environment variable for infrastructure to fill;
  the bridge's settings (allowlist, servers, signing keys, lock file) per deployment; and, per DSP server
  the allowlist names, the bridge's public key in dsp-api's `KNORA_WEBAPI_SERVICE_IDENTITY_TRUSTED_KEYS`.
- **dsp-incubator** — `dao-lift` there reuses the ported crate or is retired; its 0803 copy re-vendors from the
  bridge's shipped inputs; its vendored CPE closure gains `shared-names` and, through `sync-store`,
  `shared-stand-in`.

## User stories and requirements

### Story 1 — A project's data arrives on its own

As the person responsible for a CPE or DaTEI presentation, I want an allowlisted project's VRE data to
reach the Access Area by itself, so that the presentation follows the VRE without hand-made snapshots.

- **REQ-1.1 (Ubiquitous):** The VRE bridge shall refresh only the projects on its deployment's allowlist.
- **REQ-1.2 (Ubiquitous):** The VRE bridge shall fetch each allowlisted project from the VRE server its
  allowlist entry names.
- **REQ-1.3 (Ubiquitous):** The VRE bridge shall make at most one nightly run per night, across every process
  of `deposit-server`.
- **REQ-1.4 (Unwanted-behaviour):** If a nightly run is missed, then the VRE bridge shall not catch it up.
- **REQ-1.5 (Event-driven):** When the nightly run reaches an allowlisted project, the VRE bridge shall run a
  change check for resources changed at or after the project's T minus 60 seconds, or, for a project without
  T, with a cutoff later than any change, which lists nothing and yields only the `asOf`.
- **REQ-1.6 (Event-driven):** When the change check reports a change, the VRE bridge shall refresh the project.
- **REQ-1.7 (State-driven):** While a project lacks a Ready token in one subtree or both, the VRE bridge shall
  refresh it whatever the change check reports.
- **REQ-1.8 (State-driven):** While a project's Ready token names another server than its allowlist entry, the
  VRE bridge shall refresh it whatever the change check reports.
- **REQ-1.9 (Event-driven):** When an operator asks for a refresh of one allowlisted project, the VRE bridge
  shall refresh it whatever the change check reports.
- **REQ-1.10 (Ubiquitous):** The VRE bridge shall take a refresh's T from the `asOf` of the change check made
  at the start of that refresh.
- **REQ-1.11 (Ubiquitous):** The VRE bridge shall run at most one refresh of a project at a time, across every
  process of `deposit-server` that can start one.
- **REQ-1.12 (Unwanted-behaviour):** If a server answers a dump request with `409` during a nightly run, then
  the VRE bridge shall retry that project later in the same run, a bounded number of times.
- **REQ-1.13 (Unwanted-behaviour):** If a server is unreachable, the dump slot stays busy after the retries of
  REQ-1.12 or for an operator's refresh, a change check, dump or download fails or exceeds its time limit
  (other than REQ-2.8), or the bridge's own processing fails, then the VRE bridge shall end that project's
  refresh as failed.
- **REQ-1.14 (Complex):** While the allowlist is non-empty, when the nightly run finds a shortcode with a Ready
  token in either subtree that is not on the allowlist, the VRE bridge shall remove the project's Ready tokens.
- **REQ-1.15 (State-driven):** While the allowlist is non-empty and a shortcode directory has no Ready token in
  either subtree and is not on the allowlist, the VRE bridge shall delete it.
- **REQ-1.16 (Ubiquitous):** The VRE bridge shall authenticate dump requests with DSP-API's service identity
  (DEV-7936) and no user account.
- **REQ-1.17 (Unwanted-behaviour):** If no output location is configured, then the VRE bridge shall do
  nothing.
- **REQ-1.18 (State-driven):** While a project's Ready token names an interim format version, transformation
  version or shipped-input digest other than the running bridge's, the VRE bridge shall refresh the project
  whatever the change check reports.
- **REQ-1.19 (Ubiquitous):** The VRE bridge shall delete every export it started on a server once the refresh
  no longer needs it, whether the refresh succeeds, is refused or fails.
- **REQ-1.20 (Unwanted-behaviour):** If a server answers a dump request with `409` and the export it names is
  one of the bridge's own, then the VRE bridge shall finish and delete that export before it asks again.

Note: a server's `409` can come from anyone's dump, including a person's; the server already allows one dump
at a time, so the bridge adds no limit of its own. The bridge tells its own export from another caller's by
asking for its status: a service sees only its own exports, so another caller's answers `404` and counts as
"busy" (REQ-1.12, REQ-1.13). Because a project is known only by its Ready token, removing the tokens first
(REQ-1.14) makes the project unknown in both subtrees before any of its data goes, and a crash part-way
through removal is finished by the next run (REQ-1.14 and REQ-1.15 both re-apply). The nightly run's time and
time zone are configuration, settled in the plan; which night was run is remembered beside the lock file, so
a second process, or a restart after the run, does not run it again. An empty allowlist removes nothing, so a
missing or mistyped list deletes nothing; an unconfigured output location (REQ-1.17) keeps a local run or a
public preview of the image from touching anything. REQ-1.18 makes a `deposit-server` release that changes
the format, the transformation or a project's shipped inputs reach every project on the next nightly run;
because `sync` reads the current and the previous format version (REQ-6.2), a format bump releases
`access-server` first (reading the new version and the old), then `deposit-server`.

### Story 2 — Only public data reaches the Access Area

As DaSCH, I want only data the VRE treats as public to be published through the Access Area, and the
format to be ready for more, so that nothing restricted leaks and nothing has to be rebuilt when
restricted data is allowed later.

- **REQ-2.1 (Ubiquitous):** The interim format shall carry access rights for every Representation version, and
  with every resource and file it describes.
- **REQ-2.2 (Ubiquitous):** The interim format shall carry a data classification wherever it carries access
  rights.
- **REQ-2.3 (Ubiquitous):** The VRE bridge shall write the data classification `unclassified` wherever it
  writes one.
- **REQ-2.4 (Unwanted-behaviour):** If a dump's selection contains a resource or value that the anonymous user
  may not view, then the VRE bridge shall refuse the project and log every such resource or value.
- **REQ-2.5 (State-driven):** While a raster still image's file value grants the anonymous user restricted view
  only, the VRE bridge shall record the access rights "restricted view" for that image's Representation
  version.
- **REQ-2.6 (Ubiquitous):** The VRE bridge shall record the access rights "public" for everything else it
  writes, the shipped inputs and the file manifest included.
- **REQ-2.7 (Ubiquitous):** The VRE bridge shall download every file without credentials.
- **REQ-2.8 (Unwanted-behaviour):** If an anonymous download is answered with `401` or `403`, then the VRE
  bridge shall refuse the project.
- **REQ-2.9 (Ubiquitous):** The VRE bridge shall decide whether the anonymous user may view a resource or value
  from that resource's or value's own `knora-base:hasPermissions` in the dump's `data/rdf/data.nq`, counting
  only grants to `knora-admin:UnknownUser`, and shall treat a selected resource or value without
  `knora-base:hasPermissions` as not viewable.
- **REQ-2.10 (Ubiquitous):** The VRE bridge shall never read, copy or log a dump's `admin.nq` or
  `permission.nq`.
- **REQ-2.11 (Ubiquitous):** The VRE bridge shall unpack a dump only in a directory private to its process,
  outside the output locations, delete the zip and the unpacked tree whatever the refresh's outcome, and never
  log a dump's contents.

Note: REQ-2.6 is safe only because of REQ-2.4. A raster still image with restricted view for the anonymous
user is viewable (at a reduced size the VRE's Sipi enforces), so it passes the guard, and its restriction is
recorded explicitly (REQ-2.5) rather than flattened to "public". The VRE serves every other medium in full
under restricted view (DEV-7244), so there restricted view refuses. The VRE grants rights per value too; the
interim format records none per value (ADR-0013), so a restricted value in an otherwise public resource is
refused, never silently dropped. The guard looks only at the selection: a superseded or deleted value cannot
reach the snapshot, so its permissions do not matter, and the values of a deleted resource go with it although
DSP-API does not mark them deleted. The guard first selects, then checks, and yields every violation; a later
increment can drop what it finds instead of refusing the project (Follow-up tickets). Mapping the real
rights, and letting projects with restricted data onto the allowlist, is a later increment; the format does
not change for it.

### Story 3 — Readers never see a broken state

As `sync` and `media` (and through them CPE and DaTEI), I want a project's data to change only as a whole,
so that a reader never builds from a mixture of old and new, or from half a refresh.

- **REQ-3.1 (Ubiquitous):** The VRE bridge shall write each refresh of a project into a new generation in each
  subtree and make it current by replacing the project's Ready token with one file rename, so that a reader
  that resolves the project through its token finds either the previous generation whole or the new one whole.
- **REQ-3.2 (Ubiquitous):** The VRE bridge shall replace a project's Ready token only once the generation it
  names is completely written, in each subtree.
- **REQ-3.3 (Ubiquitous):** The VRE bridge shall write the same provenance into a project's Ready tokens in
  both subtrees.
- **REQ-3.13 (Ubiquitous):** The VRE bridge shall replace a project's `media` Ready token before its `sync`
  Ready token, so that `sync`'s announcement (REQ-6.5) finds both subtrees new.
- **REQ-3.4 (Ubiquitous):** The VRE bridge shall keep a project's previous generation in each subtree until the
  project's next successful refresh, and delete every older one.
- **REQ-3.5 (Unwanted-behaviour):** If any step of a project's refresh fails or refuses, then the VRE bridge
  shall leave that project's Ready tokens and the generations they name as they were.
- **REQ-3.6 (Unwanted-behaviour):** If one project's refresh fails, then the VRE bridge shall go on with the
  other projects.
- **REQ-3.7 (Event-driven):** When the VRE bridge starts, and when a nightly run begins, the VRE bridge shall
  point the `media` Ready token of every allowlisted project whose two tokens hold different provenance back
  to the provenance its `sync` token holds. (A project with a token in one subtree only is refreshed,
  REQ-1.7.)
- **REQ-3.8 (Event-driven):** When the restoring of REQ-3.7 is done, the VRE bridge shall delete every
  generation of an allowlisted project that neither of its Ready tokens names as current or previous, and
  every temporary token file an interrupted refresh left behind.
- **REQ-3.9 (Ubiquitous):** The VRE bridge shall write no symlinks below the `media` subtree.
- **REQ-3.10 (Ubiquitous):** The VRE bridge shall write only below the shortcodes on its allowlist, and its
  lock file and nightly-run record.
- **REQ-3.11 (Ubiquitous):** The VRE bridge shall record in each Ready token the generation, T, the server,
  the interim format's version, the transformation's version and a digest of the project's shipped inputs.
- **REQ-3.12 (Ubiquitous):** The VRE bridge shall delete below shortcodes not on its allowlist only as
  REQ-1.14 and REQ-1.15 say.
- **REQ-3.14 (Ubiquitous):** The VRE bridge and the readers shall rely on no file-system feature a network
  file system lacks: no change notification, no exchange of two directories, no symlink, and no lock on the
  shared locations; the one atomic step is the rename of a file within one directory.

Note (readers' side; the bridge's guarantee depends on it): one rename does not make a read of many files
consistent. CPE's side is required in Story 6 (REQ-6.5 to REQ-6.10, REQ-6.13, REQ-6.14); DaTEI's follows the
same rules with DEV-7938. Keeping the previous generation until the next refresh lets a read that resolved the
old token finish on consistent data.

### Story 4 — A DaTEI page gets its files

As the person creating a DaTEI page, I want to name the files my edition uses once, by their VRE resources,
and have the bridge fetch the texts and point to the images, so that DaTEI reads everything through `media`.

- **REQ-4.1 (Ubiquitous):** The VRE bridge shall write a project's shipped inputs into the project's `media`
  generation under the names they ship with.
- **REQ-4.2 (Optional):** Where a project ships a DaTEI file mapping, the VRE bridge shall resolve each mapped
  IRI in the dump to its file value.
- **REQ-4.3 (State-driven):** While a mapped resource holds a text file, the VRE bridge shall store the file
  in the project's `media` generation under its stored name.
- **REQ-4.4 (State-driven):** While a mapped resource holds a still image, the VRE bridge shall record the
  image's IIIF URL instead of downloading the image.
- **REQ-4.5 (Optional):** Where a project ships a DaTEI file mapping, the VRE bridge shall write a file
  manifest: a versioned JSON object whose entries give the key, the IRI, and either the stored name or the
  IIIF URL.
- **REQ-4.6 (Unwanted-behaviour):** If a mapped IRI is missing from the dump's selection or has no file value,
  then the VRE bridge shall refuse the project.
- **REQ-4.7 (Unwanted-behaviour):** If a stored name collides with a shipped input's name or the file
  manifest's name, or is not a name `is_file_name` accepts, then the VRE bridge shall refuse the project.

Note: the DaTEI file mapping is made by hand as part of creating a DaTEI page; how its author finds the IRIs
(`dsp-tools`' id2iri file, DSP-APP, by hand) is theirs, and no `dsp-tools` format is adopted. The key is
the Logical path DaTEI's `file-layout.xml` uses; the shapes of the mapping and the manifest are DaTEI's
(ADR-0009). A mapping keyed by IRI survives replacing a file on the same resource; a re-upload
that recreates resources needs a new mapping, which REQ-4.6 catches.

### Story 5 — Operators can see what happens and how big it gets

As an operator, I want each refresh to report what it did and how much data it produced, so that I can grow
the allowlist by evidence and notice a project that stopped updating.

- **REQ-5.1 (Event-driven):** When a project's refresh ends, the VRE bridge shall log the project, its server,
  the outcome (unchanged, refreshed, refused, failed), the reason for a refusal or failure, and the duration.
- **REQ-5.2 (Event-driven):** When a project's refresh ends, the VRE bridge shall emit its outcome as a metric
  an alert can be defined on.
- **REQ-5.3 (Event-driven):** When a project's refresh succeeds, the VRE bridge shall record the size of its
  dump, its interim snapshot and its `media` generation.

Note: a refused project stays at its last good state indefinitely; for an edited project that is the case an
alert exists for. Which alerts fire is ops' choice. A refusal is what the data causes (the guard, the file
checks, `401`/`403`); a failure is everything else: the transport, any other non-success answer, or the
bridge itself.

### Story 6 — The first increment proves itself on 0803

As the team, I want the first increment to reproduce what already works, so that a regression is visible
before new projects depend on the bridge.

- **REQ-6.1 (Ubiquitous):** `sync` and `media` shall each treat a project as known when its Ready token exists
  in their subtree.
- **REQ-6.2 (Ubiquitous):** `sync` shall read the current interim format version and the one before it.
- **REQ-6.3 (Unwanted-behaviour):** If a Ready token names an interim format version `sync` does not read, then
  `sync` shall answer for the project as unavailable (the port's `Unavailable`, as for an unreadable snapshot).
- **REQ-6.4 (Ubiquitous):** The ported transformation shall produce, from the same dump, the same facts as the
  incubator's `dao-lift` at `dasch-swiss/dsp-incubator@6106ee38166617cd8be5d58411d5bb14741ebc03` (its
  `origin/main` on 2026-11-20; `cpe/tools/dao-lift` last changed in `039547f5`), apart from the added access
  rights and data classification and the text markup carried from `knora-base:valueHasXml` (DEV-8012).
- **REQ-6.5 (Event-driven):** When a project's Ready token in the `sync` subtree appears, changes or is
  removed, `sync` shall announce that project through its port within one polling interval.
- **REQ-6.6 (Event-driven):** When `sync` announces a project CPE has a project folder for, CPE shall
  re-activate its projects.
- **REQ-6.13 (Ubiquitous):** `sync`'s port and `media`'s port shall each serve a project's provenance, from
  its Ready token, with the project's data.
- **REQ-6.7 (Unwanted-behaviour):** If the provenance the two ports serve for a project changes during CPE's
  re-activation, or differs between them, or a project CPE served before does not load for a reason other than
  its Ready token's removal, then CPE shall keep its previous activation and re-activate on the next
  announcement.
- **REQ-6.8 (Unwanted-behaviour):** If CPE's re-activation fails or panics, then CPE shall keep its previous
  activation.
- **REQ-6.9 (Event-driven):** When `sync` announces a project whose Ready token was removed, CPE shall stop
  presenting the project.
- **REQ-6.10 (Ubiquitous):** CPE shall ignore an announcement for a shortcode it has no project folder for.
- **REQ-6.14 (Ubiquitous):** `sync` and `media` shall read a project only from the generation its Ready token
  names, and shall refuse a token whose generation or shortcode is not a valid name before touching the file
  system.
- **REQ-6.11 (Ubiquitous):** The VRE bridge's test suite shall pin the public guard, restricted view, the
  dump handling, the no-symlink rule, the allowlist confinement, token-first removal and the version refresh
  (REQ-2.4, REQ-2.5, REQ-2.9 to REQ-2.11, REQ-3.9, REQ-3.10, REQ-3.12, REQ-1.14, REQ-1.18), the guard with a
  seeded restricted resource and restricted view with a seeded restricted-view image.
- **REQ-6.12 (Ubiquitous):** `access-server`'s test suite shall replace 0803's generation under a running
  `access-server` and show that CPE serves the new facts without a restart.

Note: until the per-project rebuild exists (Follow-up tickets), an announcement re-activates every project,
and REQ-6.7 judges the re-activation as a whole: one project that fails to load where it loaded before keeps
the previous activation for every project.

## Increments

1. **0803 from prod** (needs DEV-7936, DEV-7632 on prod, DEV-7514 as extended; builds DEV-7622's minimum):
   `dao-lift` ported first, then `shared-names` and `shared-stand-in`, allowlist, change check, dump with the
   service identity, public guard, interim snapshot into the `sync` subtree, 0803's shipped `media` files,
   generations with Ready tokens, `sync` and `media` known by token, metrics, and DEV-7622's minimum (REQ-6.5
   to REQ-6.10, REQ-6.13). Acceptance, beyond Story 6: the facts the bridge writes for 0803 from prod differ
   from the committed snapshot only in ways the prod data explains, each difference listed and reviewed.
   Release order: an `access-server` release reads per-project generations by Ready token, from fixtures its
   image still carries; the bridge fills each environment's locations with 0803; then ops-deploy points
   `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` at the mounts and an `access-server` release drops the data from the
   image, so 0803 is served throughout the switch. Dev has no `deposit-server`; how dev's `access-server`
   gets data is a follow-up.
2. **The first DaTEI project, Keller (083C)** (needs DEV-7920, DEV-7921, and the DaTEI plan's port): DaTEI
   file mapping, text downloads, file manifest, IIIF entries, a test server on the allowlist. Keller is in
   `ACCESS_HIDDEN_PROJECTS` (DEV-7919) until it is ready to be found. A refreshed Keller reaches DaTEI
   without a restart only once `media` announces the change and DaTEI rebuilds the project (DEV-7938, the
   DaTEI plan's); until then DaTEI is in ADR-0012's _Not yet in code_ gap and rebuilds at start. Keller's work
   runs in parallel with increment 1 (tracked separately); the bridge's side is built against test fixtures
   first.
3. **Later, each its own decision:** real access rights and restricted projects; dropping non-public items
   instead of refusing the project; CPE rebuilding the one project announced instead of re-activating all;
   CPE's discovery via `media` (DEV-7948); a generated file mapping.

## Constraints

- Timeline: Keller's deployment is due mid-December 2026 and depends on increment 2 (P-DEV-301).
- Runs inside `deposit-server`, the Deposit Area's one binary (ADR-0003); it exposes no routes, so
  `deposit-server` never serves the data the bridge handles.
- Imports no crate of the Access Area (ADR-0002); the name rules come from `shared-names`, the stand-in layout
  and the Ready token's format from `shared-stand-in`, which the bridge and the readers both use.
- The output locations may be on a network file system on another VM, so every location is a setting (where
  the bridge writes `sync`'s and `media`'s subtrees; where `access-server` reads them), and REQ-3.14 holds:
  readers poll instead of watching, a generation is made current by renaming one file, and `media` keeps
  refusing symlinks. A lock on a network file system is not reliable, so the bridge's lock file is a setting
  of its own, on local disk.
- The bridge's credentials (one signing key per deployment and VRE server) reach it through the environment;
  nothing secret is in the image or the output locations.
- Allowlisting does not hide a project: whether a project is in `ACCESS_HIDDEN_PROJECTS` is set in
  ops-deploy, today `0803,083C,086A` in every environment.
- New crates need Bazel targets; `just check` and `just test` cover them.
- Component and path names are plain ASCII.

## Success criteria

- 0803 is served in deployment from data the bridge produced from prod, increment 1's acceptance met, and no
  committed snapshot is a deployment input any more.
- A change in an allowlisted CPE project's VRE data shows on the presentation side within one night, with no
  human step and no restart.
- The data volume of each allowlisted project is visible in the bridge's logs or metrics.

## Out of scope

- **A periodic full resync.** Hard erasures are rare, ontology changes after data exists are not allowed,
  and the first projects are finished ones. The gap is known: a hard-erased resource stays visible until the
  project's next on-demand refresh.
- **Restricted data on the presentation side.** The format carries access rights; mapping real rights and
  filtering downstream is a later increment.
- **Assets.** Images stay in the VRE and are served by its Sipi as IIIF; other binary files are not fetched.
  Text files a reader needs are the exception (Story 4).
- **A generated DaTEI file mapping.** Made by hand for now.
- **Change announcements to DaTEI** (DEV-7938) and CPE's discovery via `media` (DEV-7948).
- **How the output locations are provided** (same host or a network file system): infrastructure's choice,
  within the constraint above.
- **The service identity's design** (DEV-7936).
- **`sync` serving legal information** (FORMAT.md's deviation D14, `dao:authorship` and
  `dao:copyrightHolder`): the ported transformation writes it, `sync` omits it (Follow-up tickets).

## Follow-up tickets

Created 2026-11-20 in P-DEV-301.

**The stack's own tickets**, one per pull request: listed in the plan (`02-feat-vre-bridge-plan.md`,
*Pull requests and tickets*). Existing tickets they use: DEV-7514 (ops-deploy
mounts, extended), DEV-7622 (`sync`'s announcement and CPE's re-activation, its minimum), DEV-7936 (the
service identity, dsp-api).

**Follow-ups outside the stack**, each its own ticket:

- **CPE rebuilds the one project announced** (DEV-7989). Replace the whole re-activation with a per-project rebuild
  behind one router, so that one project failing to load no longer holds back the others' refresh (REQ-6.7's
  note). dsp-repository, `cpe-server`.
- **The public guard drops instead of refusing** (DEV-7990). Filter non-public resources and values out of the selection
  instead of refusing the project, with handling for links that would dangle; mirrors what the anonymous user
  sees in the VRE and admits projects with a few restricted items. Needs its own decision on failing closed.
- **`sync` serves a file's legal information** (DEV-7991). Map `dao:authorship` and `dao:copyrightHolder` (FORMAT.md D14)
  onto `cpe_ports` (the incubator's `FileLegal`, DEV-7630's patch), once a project on the allowlist has them.
- **Data for dev's `access-server`** (DEV-7992). Dev has no `deposit-server`: run the bridge on a dev deposit host, or
  mount a copy of stage's locations read-only, before dev loses its image-baked data.
- **The incubator re-vendors** (DEV-7994). dsp-incubator's `dao-lift` reuses the ported crate or is retired, its 0803
  copy re-vendors from the bridge's shipped inputs, and its vendored CPE closure gains `shared-names` and
  `shared-stand-in`.
- **Real access rights and restricted projects** (DEV-7995). Map the VRE's permissions to access rights beyond public
  and restricted view, and let projects with restricted data onto the allowlist (increment 3).
- **A periodic full resync** (DEV-7996; deferred; Out of scope). Re-fetch every allowlisted project on a slower
  schedule, so hard erasures and ontology changes the change check misses reach the Access Area.
- **A generated DaTEI file mapping** (DEV-7997; deferred). Derive the mapping from `dsp-tools`' id2iri file or the
  upload, once hand-made mappings become the bottleneck.
- Increment 2's DaTEI side, with the DaTEI plan (DEV-7934); `media`'s announcement (DEV-7938) and CPE's
  discovery via `media` (DEV-7948) exist as tickets.

## Settled in the plan

- **Where the allowlist lives:** deployment configuration in ops-deploy, as `deposit-server`'s environment
  (`EDITOR_VRE_BRIDGE_ALLOWLIST`), because it differs between stage and prod.
- **How an operator asks for a refresh** (REQ-1.9, REQ-1.11): `deposit-server vre-bridge-refresh <shortcode>`,
  a subcommand run in the container, which takes the same lock as the nightly run and exits with the
  refresh's outcome.

## Open questions

- **Infrastructure's answer on the output locations** (same host or a network file system, paths and
  sizes): pending. The plan assumes nothing beyond REQ-3.14.
