# AI Agent Guide for the VRE bridge

Guidance for the Deposit Area's VRE bridge capability; project-wide guidance is in the top-level `CLAUDE.md`.
Vocabulary: `areas/deposit/vre-bridge/CONTEXT.md`. The decision: `areas/deposit/docs/adr/0005-the-vre-bridge-feeds-the-access-areas-stand-ins.md`
(`areas/deposit/ADR-0005`; root ADR-0005 is another record).

## Where the bridge lives

Five crates, each in its own directory.

| Crate | Directory | Role | Holds |
|---|---|---|---|
| `vre-bridge-lift` | `lift/` | `lift` | the lift (`lift`, `Lifted`, `LiftError`, `Violation`, `SelectedFiles`): the selection, the selected file values, the public guard and dsp-incubator's `dao-lift` `snapshot` as a library; `FORMAT.md`; the format and transformation versions |
| `vre-bridge-core` | `core/` | `core` | settings (`RawSettings`, `Settings::parse`), the allowlist and server ids, refresh decisions (`decide`), the shipped-input digest (`inputs_digest`), DaTEI's file mapping and file manifest (`FileMapping`, `plan_files`, `manifest_bytes`), outcomes and their exit status |
| `vre-bridge-store` | `store/` | `store` | the subtrees: generations, Ready tokens, recovery, cleanup, the lock |
| `vre-bridge-client` | `client/` | `client` | DSP-API with the service identity (`ServiceIdentity`, `Server`): project lookup, change check, export, the dump's download and private unpacking (`private_work_dir`, `unpack`), anonymous file downloads |
| `vre-bridge-server` | `server/` | `server` | the `VreBridge` facade (`from_raw`, `start`, `operator_refresh`), the schedule and the nightly run, one refresh (`Refresher`) over the `Vre`, `Report` and `Clock` traits, a DaTEI project's mapped files |

`vre-bridge-lift` grants its library to `vre-bridge-core` and `vre-bridge-server` alone (Bazel `visibility`). Its
role bans web frameworks, Maud, database crates, HTTP clients and `tokio` (`.github/scripts/check-capability-deps.sh`,
`just check`): it reads an unzipped dump from a directory it is given and returns the snapshot in memory; the
caller writes it. `vre-bridge-store` grants its library to `vre-bridge-server` alone; every call is synchronous
`std::fs`, for `spawn_blocking`.

`vre-bridge-core` grants its library to `vre-bridge-server` alone. Its role bans web frameworks, Maud and database
crates; it has no network code and reads no file but the shipped inputs, in `inputs_digest`, which blocks. It parses
the settings from the strings `deposit-server`'s loader reads (`Settings::parse` answers `None` when no output
location is set: the bridge then does nothing), and takes the format and transformation versions from
`vre-bridge-lift`. The loader hands every `EDITOR_VRE_BRIDGE_*` value over as text, numbers included, `None` when
unset; the defaults are applied, and the numbers parsed, in `Settings::parse` alone. A new variable is a name
constant, a `RawSettings` field, a line in `parse` and a test in `settings_tests.rs`, then a `VreBridgeConfig` field,
a line in `vre_bridge_settings` and in its test table in `deposit-server`'s `config.rs`, and a row in
`docs/src/vre-bridge/operations.md`.

`vre-bridge-client` grants its library to `vre-bridge-server` alone. Its role bans web frameworks, Maud and database
crates. It never installs a rustls crypto provider: the binary does, before the first `Server::new`, which returns
`ClientError::NoCryptoProvider` without one. Its tests run
against `wiremock` with the test-only key pair in `client/testdata/`, which no server trusts; `vre-bridge-server`'s
tests and `deposit-server`'s `vre_bridge_test` read its private key too (`exports_files` in `client/BUILD.bazel`).

`vre-bridge-server` is the only crate that links the other four. A refresh (`refresh.rs`) holds the lock its
caller took and runs the steps in order: recovery of the project, its tokens and the decision, the project
lookup, the change check, the export, the download into the private work directory, the export's deletion, the
unpacking, the lift, a DaTEI project's mapped files, the generation in both subtrees, the report. Three traits
keep it testable:

- **`Vre`**: DSP-API, each call naming the allowlist entry's server; implemented for
  `BTreeMap<ServerId, vre_bridge_client::Server>`. The refresh's tests use that implementation against
  `wiremock`; a fake `Vre` is for the nightly run's scheduling tests.
- **`Report`**: where an outcome goes. `LogReport` writes the outcome line; `OtelReport` (below) writes it
  and records the metrics. A test records the reports. `refresh` never reports `Busy`; a caller that
  gives up on a busy slot reports `Failed(FailureReason::Busy)` (CONTEXT.md, **Busy**).
- **`Clock`**: wall-clock time (`SystemClock`; `FakeClock` in tests). Poll intervals and time limits are
  `tokio::time`, so the refresh's tests run with real time and limits of a few hundred milliseconds: paused
  tokio time would fire them at once while a task waits on a socket or a blocking thread. The schedule and
  the nightly run's retry waits go through the `Clock` (the wait for the lock is a plain one-second
  `tokio::time::sleep`); their tests use a gated `FakeClock`, which reports each sleep and returns when the
  test releases it, over a fake `Vre` (`test_support.rs`).

`VreBridge` (`bridge.rs`) is what `deposit-server` builds and starts; `vre-bridge-core` is visible to this crate
alone, so `RawSettings` and `SigningKeys` are re-exported. `from_raw` makes the bridge inactive, with a reason
naming the variable, when no output location is set (`Inactive::NotConfigured`: it then touches no file), or
for a configuration error: `Settings::parse`'s, a `TIME_ZONE` `chrono-tz` does not know, a `WORK_DIR` or
`LOCK_FILE` at or below an output location (compared after `canonicalize`, and kept canonical; either one a
symlink is refused), an output location or `LOCK_FILE` the work directory's sweep would delete, or a signing key that does not
sign a probe token. It reads the file system and builds each server's client, so the binary calls it before
serving, after installing the rustls provider. `start` returns at once; its task takes the lock without waiting and, if it gets it,
sweeps the leftover dump directories and recovers every allowlisted project, then runs the schedule.

**Reporting** (`report.rs`): `from_raw` gives the bridge `OtelReport`, which writes `LogReport`'s outcome line and
records `vre_bridge.refresh.outcomes` and `vre_bridge.refresh.duration` by `project`, `server` and `outcome` (a
removal: `server` `(none)`, no duration), `vre_bridge.refresh.size` by `project` and `kind` for a refreshed
project, and the gauge `vre_bridge.active`. The instruments are made in `from_raw` on the meter it is given:
`serve` passes the global one after `init_otel` installs its provider, the operator's refresh one with no reader.
An attribute's values come from the allowlist, a subtree's shortcode directories or a fixed set, never from
the data.

**Panics** (`withhold.rs`): a panic's payload may quote the data a refresh handles. `spawn_blocking` runs every
closure inside `withheld`, and every task this crate spawns (start, each nightly run, each refresh) is wrapped
in `withhold`; `deposit-server`'s panic hook logs no payload while `panic_payload_withheld()` is true. A new
`tokio::spawn` here wraps its future in `withhold`.

**The schedule** (`schedule.rs`) runs the next `NIGHTLY_AT` in `TIME_ZONE` strictly after now: a local time
that occurs twice takes its first occurrence, one in a gap moves forward 15 minutes at a time until it
occurs. A missed run is never caught up. Each nightly run is its own task, awaited, and is given the local
date it was scheduled for.

**A nightly run** (`nightly.rs`), in this order:

1. Wait for the lock, trying again every second with an async sleep; a long wait is logged every 10 minutes.
2. Stop if the nightly-run record holds the run's date; else write it, before anything else, so a run cut short
   is not run again that night. A record that cannot be written skips the run.
3. Note the shortcode directories without a token in either subtree.
4. Sweep the `vre-bridge-dump-*` directories in `WORK_DIR`.
5. Recover every allowlisted project.
6. Remove the projects no longer allowlisted (nothing for an empty allowlist): each withdrawn one is reported
   as outcome `removed`, at warning level; a directory noted in 3 that still has no token is deleted.
7. Refresh each allowlisted project in order; a project's failure does not stop the others. A busy dump slot
   queues the project with a not-before time `DUMP_RETRY_SECS` later; the run sleeps until the earliest is
   due, retries those due, and gives up after `DUMP_RETRIES` retries with `Failed(Busy)`.

The lock is held to the run's end, retry waits included, so an operator's refresh meanwhile exits `75` for up
to the retries' length. **The operator's refresh** (`operator_refresh`) takes the lock without waiting:
`64` (`EXIT_UNUSABLE`) for an inactive bridge or a project not on the allowlist, `75` (`EXIT_LOCKED`) for a
held lock, else one forced refresh and the outcome's exit status; a busy slot fails it at once. Every refresh,
the nightly run's and the operator's, runs in a spawned task that is awaited and never aborted, since a dropped
refresh would release the lock while a blocking write goes on; a refresh that panics is reported as
`Failed(Internal)` and the run goes on.

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
- **No configuration error and no settings `Debug` shows a signing key.** `SigningKeys` and `SigningKey` write their
  `Debug` by hand; a `SettingsError` names an entry of `EDITOR_VRE_BRIDGE_SIGNING_KEYS` by its position, never by its
  text, since a bare key split on its first `=` reads as a server id. It quotes no value and no key longer than a
  server id, so a key pasted into another variable is not quoted either. A new variant keeps to that.
- **The shipped-input digest's definition is fixed.** Changing what `inputs_digest` hashes, or in which order,
  changes every project's digest and so refreshes every project once.
- **`inputs/<shortcode>/` is the one authoritative copy of a project's shipped inputs.** A change to
  `inputs/0803/` is copied in the same commit into the generation of `areas/access/media/data/0803/`, which
  `just check-stand-in-fixture` holds equal byte for byte, and changes the digest that
  `server/tests/refresh_0803.rs` pins and both 0803 fixture tokens (`media` and `sync`) record: the three are
  updated together, and the test and `just check-stand-in-fixture` fail until they agree.
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

- **A mapped file must be in the guarded selection.** The lift returns the selected files (`Lifted::files`):
  the current file values of each resource the snapshot holds, taken from the same `Selection` the guard
  checked. The refresh resolves a file mapping against them alone and reads nothing else of the dump.
- **A file is downloaded only below the server's configured bases.** `Server::download_file` refuses any other
  URL or scheme, and a URL with credentials, a query, a fragment or an encoded separator, before a request, follows no redirect, stops at
  `EDITOR_VRE_BRIDGE_MAX_FILE_BYTES` and deletes what it wrote on failure. Every download lands in the private
  work directory before the generation is begun; only `GenerationWriter::copy_file` takes it into `media`.

## Checking the lift on a real dump

`bazel test //areas/deposit/vre-bridge/lift/...` runs the fixture tests. The real-dump tests are `#[ignore]`d;
`just vre-bridge-lift-real-dump <dump dir> <reference snapshot>` runs them, with the reference written by
dsp-incubator's `dao-lift snapshot` at the ported commit from the same dump, both as absolute paths.
`real_dump_matches_the_incubator_lift` compares the two as sets of quads, apart from the access rights and data
classification the incubator never wrote. When the public guard refuses the dump, every test fails after printing
the guard's violations.
