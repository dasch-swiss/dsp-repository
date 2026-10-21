# media

The Access Area's `media` capability: the Access bucket's only consumer, as `sync` is the projection stream's,
and the implementer of the bytes ports through which a sibling capability reads a project's deposited files at
build time (ADR-0012). It exists so far at its minimum, the crate `media-store` in `store/`, which serves CPE's
bytes port from a committed **Stand-in directory**, `data/`. `access-server` constructs it over the directory
`MEDIA_DATA_DIR` names (default `areas/access/media/data`; an empty value is unset) and hands it to CPE's activation; a directory that does not
exist is logged when the binary starts, and every CPE project then fails to load and answers 503. The port's
terms (Bytes port, Curation) are in [`../cpe/CONTEXT.md`](../cpe/CONTEXT.md), the media engine's (Media
capability, Access bucket, Byte source, Authorisation check) in
[`../../../vitrinli/CONTEXT.md`](../../../vitrinli/CONTEXT.md), the Access Area's in
[`../CONTEXT.md`](../CONTEXT.md), the contract terms (Project, Shortcode) and the file vocabulary (Service File)
in the root [`CONTEXT.md`](../../../CONTEXT.md) `## Shared`.

## Language

**Stand-in directory**:
`data/`, the hand-fed directory that stands in for the Access bucket until the archive publishes to it. It is
laid out as the bucket's prefixes will be: one directory per project, named by its shortcode, holding the files
the project deposited under their names, `<shortcode>/<name>`. Today `0803/` holds Incunabula's `config.kdl`,
`curation.csv` and `narrative/<name>.<lang>.md`. It holds text only, in plain git, and `data/PROVENANCE`
records where each file came from and how it is changed: every file is edited in place by a person and never
generated. A project is in `media` when its directory is; there is no list of known projects beside it.
_Avoid_: the bucket (the stand-in is not it), project folder (CPE's, which holds code and the documents about
the data), committed snapshot (`sync`'s).

**`LiveDepositedFiles`**:
`media-store`'s adapter for CPE's bytes port, `cpe_ports::DepositedFiles`. Each call reads
`<dir>/<shortcode>/<name>` whole and holds nothing between calls. A caller's string reaches the file system
only once `cpe_ports::is_shortcode` and `cpe_ports::is_file_name` accept it: a refused shortcode, or one that
names no directory, is `UnknownProject`; a refused name, or one that names no regular file, is `UnknownFile`.
No component below `<dir>` may be a symbolic link: a linked project directory is `UnknownProject`, a linked
directory or file of the name `UnknownFile`. Any other failure, a directory under the file's name or one the
process may not search included, is `Unavailable`, whose source names the path and is logged, never served.
The window between that check and the read is accepted, since only an operator writes the directory.
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
  since activation runs once. Not yet in code: the ports announce change, `media` for its bytes ports and
  `sync` for the archive projection port; until the bucket exists `media` owns the operator lever, a file watch
  or an admin endpoint behind a token, that makes its adapter announce, and CPE rebuilds the one project named
  and grows no endpoint of its own (ADR-0012, DEV-7622).
- **Serve DaTEI.** DaTEI has no code and declares no port yet; its adapter here waits for that port.
- **Read the Access bucket.** The adapter that replaces `LiveDepositedFiles` there resolves a name without
  following a symbolic link (`openat2`, `cap-std`) rather than checking first.

`media-store` depends on `cpe-ports` alone of this repository's crates. That no CPE manifest names a `media`
crate outside a dev-dependencies table is checked by `check-cpe-seam.sh` (rule 4); that `media-store` depends on
no `sync` crate and no sibling's domain or web crate is review.

A new consumer of `media`, DaTEI for one, declares its own bytes port in its own ports crate, and `media-store`
implements it beside `LiveDepositedFiles`. `is_shortcode` and `is_file_name` are CPE's rules until a second
consumer needs them; moving them out of `cpe-ports` is an architecture change, made by an ADR or an amendment.
