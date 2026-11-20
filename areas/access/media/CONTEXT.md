# media

The Access Area's `media` capability: the Access bucket's only consumer, as `sync` is the projection stream's, and
the implementer of the bytes ports through which a sibling capability reads a project's deposited files at build
time (ADR-0012). It exists so far at its minimum, the crate `media-store` in `store/`, which serves CPE's bytes
port from a **Stand-in directory**, today the committed fixture `data/`. `access-server` constructs it over the directory
`MEDIA_DATA_DIR` names (default `areas/access/media/data`; an empty value is unset) and hands it to CPE's
activation; a directory that does not exist is logged when the binary starts, and every CPE project then fails to
load and answers 503. Its adapter of DaTEI's port, not yet in code, will read the same directory (*Serve DaTEI*,
below). The terms of CPE's port (Bytes port, Curation) are in [`../cpe/CONTEXT.md`](../cpe/CONTEXT.md), the media
engine's (Media capability, Access bucket, Byte source, Authorisation check) in
[`../../../vitrinli/CONTEXT.md`](../../../vitrinli/CONTEXT.md), the Access Area's in
[`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) and the file vocabulary (Service File)
in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`, with the Generation and the Ready token.

## Language

**Stand-in directory**:
The directory that stands in for the Access bucket until the archive publishes to it, laid out as the VRE bridge
writes its `media` subtree (`areas/deposit/ADR-0005`, ADR-0012): one directory per project, named by its shortcode,
holding the project's Ready token `.ready` and one directory per Generation, `<shortcode>/<generation>/`, which holds
the files under their names. On a server it is the `media` subtree the directory `MEDIA_DATA_DIR` names, and the
VRE bridge is its only writer: each generation holds the text files from the VRE under their Stored names, the File
manifest that names the VRE resource each came from (DaTEI's vocabulary, in
[`../datei/CONTEXT.md`](../datei/CONTEXT.md)), the files the bridge's shipped inputs hold for the project (for
DaTEI, `datei.toml`, `file-layout.xml` and the file mapping) and `access-rights.json`, the rights of its files. The
shipped inputs are the one authoritative copy of the files the VRE does not hold. The File manifest, the Ready
token and the shipped inputs record provenance. The committed `data/` is a test fixture in the same layout, which
`access-server`'s image still carries: `0803/.ready`, hand-written, and the generation
`0803/20261007T000000Z-00000000/` holding Incunabula's `config.kdl`, `curation.csv` and
`narrative/<name>.<lang>.md`, text only, in plain git; `data/PROVENANCE` records where each file came from. Not yet
in code: the VRE bridge, and the fixture pinned to its shipped inputs (P-DEV-301).
_Avoid_: the bucket (the stand-in is not it), project folder (CPE's, which holds code and the documents about
the data), committed snapshot (`sync`'s file in a generation is its Interim snapshot).

**Known project by token**:
A project whose Ready token exists in the stand-in directory. `media` knows no other list of projects: a project
directory without a token is `UnknownProject`, whatever it holds. A token that cannot be read or parsed, or one
naming a generation that is not a real directory, is `Unavailable`, never `UnknownProject`, so a broken project is
not taken for a removed one. Only files of the generation the token names are served; nothing beside it, the token
included.
_Avoid_: known by directory.

**Reserved name**:
A name in a generation that is not a project's deposited file: `.ready` (the token, outside every generation and
refused by `is_file_name` as dot-named), `access-rights.json` and `file-manifest.json`, both written by the VRE
bridge into a generation. A shipped input of either name is a configuration error of the bridge. `LiveDepositedFiles`
serves either like any other name, so a project's configuration must not name them (**review**).
_Avoid_: hidden file.

**`LiveDepositedFiles`**:
`media-store`'s adapter for CPE's bytes port, `cpe_ports::DepositedFiles`. Each call reads the project's Ready
token through `shared_stand_in::read_token`, then `<dir>/<shortcode>/<generation>/<name>` whole, and holds nothing
between calls. A caller's string reaches the file system only once `shared_names::is_shortcode` and
`shared_names::is_file_name` accept it: a refused shortcode, or a project without a token, is `UnknownProject`; a
refused name, or one that names no regular file, is `UnknownFile`. No component at or below the generation
directory may be a symbolic link, checked segment by segment without following one: a linked generation directory
is `Unavailable`, a linked directory or file of the name `UnknownFile`. A linked project directory or token is
refused by the token's reader, and is `Unavailable` too. Any other failure, a directory under the file's name or one
the process may not search included, is `Unavailable`, whose source names the path and is logged, never served.
The window between that check and the read is accepted, since only the stand-in's one writer, the VRE bridge,
writes it; nothing a reader does may write there or rely on file notifications, since the subtree may be NFS.
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
  since activation runs once. `LiveDepositedFiles` serves a project's provenance from its Ready token, equal to
  the `sync` token's when both subtrees hold the same generation. Not yet in code: `media` announces change on its bytes ports
  (DEV-7938), and CPE follows `sync`'s announcements (DEV-7622); until the bucket exists `media` owns the
  Ready-token poll, which reads each project's Ready token in its subtree at a fixed interval and
  makes its adapter announce, and a reader rebuilds the one project named and grows no endpoint of its own
  (ADR-0012).
- **Serve DaTEI.** `media-store` implements DaTEI's port, `datei_ports::DateiFiles`, beside
  `LiveDepositedFiles`: `projects()` lists the projects `shared_stand_in::known_projects` returns whose token's
  generation holds `datei.toml`, and the adapter resolves a project's generation with `read_token` and
  `generation_dir` once, when it creates the project's file source, and `file()` reads only below that pinned
  generation, `<dir>/<shortcode>/<generation>/<stored name>`, with CPE's name check. DaTEI has no provenance
  comparison as CPE has, so a resolve per call could mix two generations while the bridge replaces one.
  `access-rights.json` and `file-manifest.json` are Reserved names there. Not yet in code: that adapter, and
  `media-store`'s dependency on `datei_ports` (DEV-7574).
- **Read the Access bucket.** The adapter that replaces `LiveDepositedFiles` there resolves a name without
  following a symbolic link (`openat2`, `cap-std`) rather than checking first.

`media-store` depends on `cpe-ports`, `shared-names` and `shared-stand-in` alone of this repository's crates, and on `datei-ports` too once it
implements DaTEI's port. Bazel `visibility` holds both sides: `media_store` is granted to the composition root
alone and a reader's tests use `media_store_for_tests`, which is `testonly`; and no `sync` or sibling target
but `cpe_ports`, and `datei_ports` once it exists, grants `media-store`.

A new consumer of `media` declares its own bytes port in its own ports crate, as DaTEI does with `datei-ports`,
and `media-store` implements it beside `LiveDepositedFiles`. `is_shortcode` and `is_file_name` live in
`shared-names` (ADR-0002), which `media-store`, `sync-store`, `cpe-ports` and `shared-stand-in` use; the VRE bridge is
granted it in `shared/names/BUILD.bazel`.
