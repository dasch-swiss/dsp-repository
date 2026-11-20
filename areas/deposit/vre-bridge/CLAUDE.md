# AI Agent Guide for the VRE bridge

Guidance for the Deposit Area's VRE bridge capability; project-wide guidance is in the top-level `CLAUDE.md`.
Vocabulary: `areas/deposit/vre-bridge/CONTEXT.md`. The decision: `areas/deposit/docs/adr/0005-the-vre-bridge-feeds-the-access-areas-stand-ins.md`
(`areas/deposit/ADR-0005`; root ADR-0005 is another record).

## Where the bridge lives

Five crates, each in its own directory.

| Crate | Directory | Role | Holds |
|---|---|---|---|
| `vre-bridge-lift` | `lift/` | `lift` | the lift (`lift`, `Lifted`, `LiftError`, `Violation`): the selection, the public guard and dsp-incubator's `dao-lift` `snapshot` as a library; `FORMAT.md`; the format and transformation versions |
| `vre-bridge-core` | `core/` | `core` | settings (`RawSettings`, `Settings::parse`), the allowlist and server ids, refresh decisions (`decide`), the shipped-input digest (`inputs_digest`), outcomes and their exit status |
| `vre-bridge-store` | `store/` | `store` | the subtrees: generations, Ready tokens, recovery, cleanup, the lock |
| `vre-bridge-client` | `client/` | `client` | DSP-API with the service identity (`ServiceIdentity`, `Server`): project lookup, change check, export, the dump's download and private unpacking (`private_work_dir`, `unpack`), anonymous file downloads |
| `vre-bridge-server` | `server/` | `server` | one refresh (`Refresher`) over the `Vre`, `Report` and `Clock` traits; to come: the `VreBridge` facade `deposit-server` starts, the nightly run, the operator's refresh |

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
against `wiremock` with the test-only key pair in `client/testdata/`, which no server trusts; `vre-bridge-server`'s
tests read its private key too (`exports_files` in `client/BUILD.bazel`).

`vre-bridge-server` is the only crate that links the other four. A refresh (`refresh.rs`) holds the lock its
caller took and runs the steps in order: recovery of the project, its tokens and the decision, the project
lookup, the change check, the export, the download into the private work directory, the export's deletion, the
unpacking, the lift, the generation in both subtrees, the report. Three traits keep it testable:

- **`Vre`**: DSP-API, each call naming the allowlist entry's server; implemented for
  `BTreeMap<ServerId, vre_bridge_client::Server>`. The refresh's tests use that implementation against
  `wiremock`; a fake `Vre` is for the nightly run's scheduling tests.
- **`Report`**: where an outcome goes. `LogReport` writes the outcome line; the metrics adapter is
  `deposit-server`'s concern. A test records the reports. `refresh` never reports `Busy`; a caller that
  gives up on a busy slot reports `Failed(FailureReason::Busy)` (CONTEXT.md, **Busy**).
- **`Clock`**: wall-clock time (`SystemClock`; `FakeClock` in tests). Poll intervals and time limits are
  `tokio::time`, so the refresh's tests run with real time and limits of a few hundred milliseconds: paused
  tokio time would fire them at once while a task waits on a socket or a blocking thread.

## Rules

- **Only `vre-bridge-store` writes the subtrees.** No other crate, in the bridge or outside it, creates, replaces or
  removes anything below the `sync` and `media` output locations; a new write path is a function of
  `vre-bridge-store`.
- **A dump is unpacked only into the private work directory**: a directory of its own, mode `0700`, under the
  configured work directory, which is never inside an output location. Only `data/rdf/data.nq` and
  `data/rdf/ontology-<n>.nq` are extracted, never `admin.nq` or `permission.nq`, the zip is deleted as soon as
  it is unpacked, and the directory is deleted on every outcome.
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
- **No configuration error and no settings `Debug` shows a signing key.** The keys are files, read in `from_raw`
  and handed straight to `ServiceIdentity`; an error about a key file names the variable, the server and the path,
  never the file's bytes. `SigningKeyFiles` writes its `Debug` by hand, and a `SettingsError` names an entry of
  `EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES` by its position, never by its text, since a key pasted where its path
  belongs, split on its first `=`, reads as a server id; a pasted key is refused as a relative path. It quotes no
  value and no key longer than a server id, so a key pasted into another variable is not quoted either. A new
  variant keeps to that.
- **The shipped-input digest's definition is fixed.** Changing what `inputs_digest` hashes, or in which order,
  changes every project's digest and so refreshes every project once.
- **A refresh blocks no runtime thread.** Every `vre-bridge-store` call, `unpack`, `lift`, `inputs_digest` and
  the work directory's removal run in `spawn_blocking` with owned arguments; no `GenerationWriter` is held
  across an `.await`. A panic there ends the refresh as `Failed(Internal)`; its payload is not logged.
- **Once an export id is known, the export is deleted on every path**, the outcome unchanged; a running export
  is waited for once more. A refused or failed refresh leaves the tokens as they were.
- **A refresh logs no value of the dump.** A guard violation is logged as its IRI, class and reason (the first
  50, then the total), a closed-world refusal as its rules and counts, a load error as its kind and line;
  `LiftError`'s text is logged only for `Dump` and `Internal`, which name no literal.
- **`unpack` reads the zip by name only.** It checks every central-directory entry before writing anything (an
  absolute path, `..`, a symlink or two entries of one name fail the dump) and never calls `ZipArchive::extract`;
  it is blocking, so an async caller runs it in `spawn_blocking`. `download` and `unpack` stop at the caps
  `EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES` and `EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES` (`ClientError::TooLarge`).

## Checking the lift on a real dump

`bazel test //areas/deposit/vre-bridge/lift/...` runs the fixture tests. The real-dump tests are `#[ignore]`d;
`just vre-bridge-lift-real-dump <dump dir> <reference snapshot>` runs them, with the reference written by
dsp-incubator's `dao-lift snapshot` at the ported commit from the same dump, both as absolute paths.
`real_dump_matches_the_incubator_lift` compares the two as sets of quads, apart from the access rights and data
classification the incubator never wrote. When the public guard refuses the dump, every test fails after printing
the guard's violations.
