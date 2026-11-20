# AI Agent Guide for the VRE bridge

Guidance for the Deposit Area's VRE bridge capability; project-wide guidance is in the top-level `CLAUDE.md`.
Vocabulary: `areas/deposit/vre-bridge/CONTEXT.md`. The decision: `areas/deposit/docs/adr/0005-the-vre-bridge-feeds-the-access-areas-stand-ins.md`
(`areas/deposit/ADR-0005`; root ADR-0005 is another record).

## Where the bridge lives

Five crates, each in its own directory; all but `vre-bridge-server` exist so far.

| Crate | Directory | Role | Holds | Exists |
|---|---|---|---|---|
| `vre-bridge-lift` | `lift/` | `lift` | the lift (`lift`, `Lifted`, `LiftError`, `Violation`): the selection, the public guard and dsp-incubator's `dao-lift` `snapshot` as a library; `FORMAT.md`; the format and transformation versions | yes |
| `vre-bridge-core` | `core/` | `core` | settings (`RawSettings`, `Settings::parse`), the allowlist and server ids, refresh decisions (`decide`), the shipped-input digest (`inputs_digest`), outcomes and their exit status | yes |
| `vre-bridge-store` | `store/` | `store` | the subtrees: generations, Ready tokens, recovery, cleanup, the lock | yes |
| `vre-bridge-client` | `client/` | `client` | DSP-API with the service identity (`ServiceIdentity`, `Server`): project lookup, change check, export, the dump's download and private unpacking (`private_work_dir`, `unpack`), anonymous file downloads | yes |
| `vre-bridge-server` | `server/` | `server` | the `VreBridge` facade `deposit-server` starts: the nightly run, one refresh, the operator's refresh | no |

`vre-bridge-lift` grants its library to `vre-bridge-core` and `vre-bridge-server` alone (Bazel `visibility`). Its
role bans web frameworks, Maud, database crates, HTTP clients and `tokio` (`.github/scripts/check-capability-deps.sh`,
`just check`): it reads an unzipped dump from a directory it is given and returns the snapshot in memory; the
caller writes it. `vre-bridge-store` grants its library to `vre-bridge-server` alone; every call is synchronous
`std::fs`, for `spawn_blocking`.

`vre-bridge-core` grants its library to `vre-bridge-server` alone. Its role bans web frameworks, Maud and database
crates; it has no network code and reads no file but the shipped inputs, in `inputs_digest`, which blocks. It parses
the settings from the strings `deposit-server`'s loader reads (`Settings::parse` answers `None` when no output
location is set: the bridge then does nothing), and takes the format and transformation versions from
`vre-bridge-lift`. The loader hands every `EDITOR_VRE_BRIDGE_*` value over as read, `None` when unset; the defaults
are applied in `Settings::parse` alone. A new variable is a name constant, a `RawSettings` field, a line in `parse`
and a test in `settings_tests.rs`.

`vre-bridge-client` grants its library to `vre-bridge-server` alone. Its role bans web frameworks, Maud and database
crates. It never installs a rustls crypto provider: the binary does, before the first `Server::new`, which returns
`ClientError::NoCryptoProvider` without one. Its tests run
against `wiremock` with the test-only key pair in `client/testdata/`, which no server trusts.

## Rules

- **Only `vre-bridge-store` writes the subtrees.** No other crate, in the bridge or outside it, creates, replaces or
  removes anything below the `sync` and `media` output locations; a new write path is a function of
  `vre-bridge-store`.
- **A dump is unpacked only into the private work directory**: a directory of its own, mode `0700`, under the
  configured work directory, which is never inside an output location. Only `data/rdf/data.nq` and
  `data/rdf/ontology-<n>.nq` are extracted, never `admin.nq` or `permission.nq`, and the directory is deleted on
  every outcome.
- **`lift/FORMAT.md` is normative.** Where the code and `FORMAT.md` disagree, the code is wrong. An output change
  updates `FORMAT.md` and adds a §16 line; sections are never renumbered (`format_doc_tests.rs` checks both).
- **The guard sees every resource and value that is written.** `lift` reads the dump (`snapshot::Read`), selects
  (`selection.rs`), checks the selection (`guard.rs`), then builds from that same `Selection` with the guard's
  `Rights`; nothing in the snapshot code re-derives what is current, and a subject without rights from the guard is
  `LiftError::Internal`, never public. List nodes are outside the selection: they carry no permissions and no
  rights (`FORMAT.md` §11). The guard refuses on any violation and fails closed: a missing or unreadable
  `knora-base:hasPermissions` refuses, and only `knora-admin:UnknownUser` counts.
- **A bad dump is an error, not a panic.** A refused project is `LiftError::Refused`, a dump that breaks the format
  `LiftError::ClosedWorld`, each listing every case; production code calls no `expect`, `unwrap`, `panic!` or
  `unreachable!` (denied in `lift/src/lib.rs`). `lift` blocks and holds the whole dump in memory: an async caller
  runs it in `spawn_blocking`.
- **`TRANSFORMATION_VERSION` rises with any change to the bytes `lift` writes for the same dump**, in the commit that
  changes them; `the_rich_fixture_lifts_to_the_pinned_golden_bytes` fails until `lift/testdata/golden.nq` is
  replaced, so the change is noticed.
- **`INTERIM_FORMAT_VERSION` rises when a reader would read a snapshot differently** (`FORMAT.md` §0). Raising it to
  n needs an `access-server` release whose `sync-store` reads versions n-1 and n first.
- **`LiftError` goes to the log only.** Its text names paths, graph IRIs and resource and value IRIs of the dump; a
  closed-world violation of a label names the label's text.
- **`ClientError` carries no secret and no data.** No token, key, URL or response body reaches its text: a
  `reqwest::Error` goes through `without_url()`, a JSON parse error is dropped (serde quotes the input), and
  `ServiceIdentity` and `Server` write their `Debug` by hand, without key or token (`Server` shows its base URL,
  which may not carry credentials). A new variant keeps to that.
- **No configuration error and no settings `Debug` shows a signing key.** `SigningKeys` and `SigningKey` write their
  `Debug` by hand; a `SettingsError` names an entry of `EDITOR_VRE_BRIDGE_SIGNING_KEYS` by its position, never by its
  text, since a bare key split on its first `=` reads as a server id. It quotes no value and no key longer than a
  server id, so a key pasted into another variable is not quoted either. A new variant keeps to that.
- **The shipped-input digest's definition is fixed.** Changing what `inputs_digest` hashes, or in which order,
  changes every project's digest and so refreshes every project once.
- **`unpack` reads the zip by name only.** It checks every central-directory entry before writing anything (an
  absolute path, `..`, a symlink or two entries of one name fail the dump) and never calls `ZipArchive::extract`;
  it is blocking, so an async caller runs it in `spawn_blocking`.

## Checking the lift on a real dump

`bazel test //areas/deposit/vre-bridge/lift/...` runs the fixture tests. The real-dump tests are `#[ignore]`d;
`just vre-bridge-lift-real-dump <dump dir> <reference snapshot>` runs them, with the reference written by
dsp-incubator's `dao-lift snapshot` at the ported commit from the same dump, both as absolute paths.
`real_dump_matches_the_incubator_lift` compares the two as sets of quads, apart from the access rights and data
classification the incubator never wrote. When the public guard refuses the dump, every test fails after printing
the guard's violations.
