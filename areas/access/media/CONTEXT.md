# media

The Access Area's `media` capability: the Access bucket's only consumer, as `sync` is the projection stream's, and
the implementer of the bytes ports through which a sibling capability reads a project's deposited files at build
time (ADR-0012). It exists so far at its minimum, the crate `media-store` in `store/`, which serves CPE's bytes
port from a committed **Stand-in directory**, `data/`. `access-server` constructs it over the directory
`MEDIA_DATA_DIR` names (default `areas/access/media/data`; an empty value is unset) and hands it to CPE's
activation; a directory that does not exist is logged when the binary starts, and every CPE project then fails to
load and answers 503. Its adapter of DaTEI's port, not yet in code, will read the same directory (*Serve DaTEI*,
below). The terms of CPE's port (Bytes port, Curation) are in [`../cpe/CONTEXT.md`](../cpe/CONTEXT.md), the media
engine's (Media capability, Access bucket, Byte source, Authorisation check) in
[`../../../vitrinli/CONTEXT.md`](../../../vitrinli/CONTEXT.md), the Access Area's in
[`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) and the file vocabulary (Service File)
in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Stand-in directory**:
`data/`, the directory that stands in for the Access bucket until the archive publishes to it. It is
laid out as the bucket's prefixes will be: one directory per project, named by its shortcode, holding the files
the project deposited under their names, `<shortcode>/<name>`. Today `0803/` holds Incunabula's `config.kdl`,
`curation.csv` and `narrative/<name>.<lang>.md`. The committed `data/` holds text only, in plain git, and
`data/PROVENANCE` records where each file came from and how it is changed: every file there is edited in place
by a person and never generated. On a server, the directory `MEDIA_DATA_DIR` names is the `media` subtree of the
stand-in directory the VRE bridge writes (`areas/deposit/ADR-0005`, ADR-0012), not the committed `data/`: one
directory per project, holding its text files from the VRE under their Stored names, the File manifest that
names the VRE resource each came from (DaTEI's vocabulary, in [`../datei/CONTEXT.md`](../datei/CONTEXT.md)), the
files the bridge's shipped inputs hold for the project (for DaTEI, `datei.toml`, `file-layout.xml` and the file mapping), and its
Ready token. The shipped inputs are the one authoritative copy of the files the VRE does not hold, and `data/`
becomes a test fixture pinned to them. There the File manifest, the Ready token and the shipped inputs record
provenance, as `data/PROVENANCE` does for the committed files. Today a project is in `media` when its directory
is; on a server it is when its Ready token is. There is no list of known projects beside it. Not yet in code:
the VRE bridge, and `media` knowing a project by its Ready token (P-DEV-301).
_Avoid_: the bucket (the stand-in is not it), project folder (CPE's, which holds code and the documents about
the data), committed snapshot (`sync`'s).

**`LiveDepositedFiles`**:
`media-store`'s adapter for CPE's bytes port, `cpe_ports::DepositedFiles`. Each call reads
`<dir>/<shortcode>/<name>` whole and holds nothing between calls. A caller's string reaches the file system
only once `shared_names::is_shortcode` and `shared_names::is_file_name` accept it: a refused shortcode, or one that
names no directory, is `UnknownProject`; a refused name, or one that names no regular file, is `UnknownFile`.
No component below `<dir>` may be a symbolic link: a linked project directory is `UnknownProject`, a linked
directory or file of the name `UnknownFile`. Any other failure, a directory under the file's name or one the
process may not search included, is `Unavailable`, whose source names the path and is logged, never served.
The window between that check and the read is accepted, since only the stand-in's one writer writes it.
_Avoid_: the media port (CPE declares the port; `media` implements it), cache (it holds nothing between calls),
`FakeDepositedFiles` (CPE's in-memory adapter for its own tests).

**Anonymous consumer**:
The principal a reader decides for until an authorisation mechanism exists (ADR-0012, ADR-0013). A bytes port
never decides for anyone: it serves every file it holds, in the target with the file's rights attached, and a
rebuild reads on no one's behalf. The reader decides at request time, at its own edge, with the shared decision
function over the rights it stored; in the interim it decides every request as the anonymous consumer.
No adapter refuses a file for want of a right, `LiveDepositedFiles` included: a port that pre-filters would
leave the reader nothing to enforce. Not yet in code: rights on the port, and the decision function; the stand-in
holds public files only, so serving every file serves no file the anonymous consumer may not see.
_Avoid_: public user, guest (a request's viewer; a bytes port has no request).

## What `media` does not do yet

- **Serve at request time.** It has no routes, so no asset (an image, a download) comes through it. CPE serves
  a project's `assets/` from the project folder until `media` has those routes; then they leave the folder
  (ADR-0012). An image is an IIIF URL, served by the IIIF server CPE's store names.
- **Render or derive.** Vitrinli is not here, and the Access Area's `media` never derives: the archive already
  did (`vitrinli/CONTEXT.md`).
- **Announce change.** A file edited under `MEDIA_DATA_DIR` reaches CPE only when the Access binary restarts,
  since activation runs once. Not yet in code: the ports announce change, `media` for its bytes ports
  (DEV-7938) and `sync` for the archive projection port (DEV-7622); until the bucket exists `media` owns the
  Ready-token poll, which reads each project's Ready token in its subtree at a fixed interval and
  makes its adapter announce, and a reader rebuilds the one project named and grows no endpoint of its own
  (ADR-0012).
- **Serve DaTEI.** `media-store` implements DaTEI's port, `datei_ports::DateiFiles`, beside
  `LiveDepositedFiles`: `projects()` lists the shortcode directories holding both the Ready token and
  `datei.toml`, and `file()` reads `<dir>/<shortcode>/<stored name>` with CPE's name check. Not yet in code:
  that adapter, and `media-store`'s dependency on `datei_ports` (DEV-7574).
- **Read the Access bucket.** The adapter that replaces `LiveDepositedFiles` there resolves a name without
  following a symbolic link (`openat2`, `cap-std`) rather than checking first.

`media-store` depends on `cpe-ports` and `shared-names` alone of this repository's crates, and on `datei-ports` too once it
implements DaTEI's port. Bazel `visibility` holds both sides: `media_store` is granted to the composition root
alone and a reader's tests use `media_store_for_tests`, which is `testonly`; and no `sync` or sibling target
but `cpe_ports`, and `datei_ports` once it exists, grants `media-store`.

A new consumer of `media` declares its own bytes port in its own ports crate, as DaTEI does with `datei-ports`,
and `media-store` implements it beside `LiveDepositedFiles`. `is_shortcode` and `is_file_name` live in
`shared-names` (ADR-0002), which `media-store`, `cpe-ports` and `shared-stand-in` use; `sync-store` and the VRE bridge are
granted it in `shared/names/BUILD.bazel`.
