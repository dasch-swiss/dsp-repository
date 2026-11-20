# AI Agent Guide for the VRE bridge

Guidance for the Deposit Area's VRE bridge capability; project-wide guidance is in the top-level `CLAUDE.md`.
Vocabulary: `areas/deposit/vre-bridge/CONTEXT.md`. The decision: `areas/deposit/docs/adr/0005-the-vre-bridge-feeds-the-access-areas-stand-ins.md`
(`areas/deposit/ADR-0005`; root ADR-0005 is another record).

## Where the bridge lives

Five crates, each in its own directory; only `vre-bridge-lift` exists so far.

| Crate | Directory | Role | Holds | Exists |
|---|---|---|---|---|
| `vre-bridge-lift` | `lift/` | `lift` | dsp-incubator's `dao-lift` `snapshot` as a library (`snapshot`, `Snapshot`, `Stats`, `LiftError`), `FORMAT.md` | yes |
| `vre-bridge-core` | `core/` | `core` | settings, the allowlist, refresh decisions, the shipped-input digest, outcomes | no |
| `vre-bridge-store` | `store/` | `store` | the subtrees: generations, Ready tokens, recovery, cleanup, the lock | no |
| `vre-bridge-client` | `client/` | `client` | DSP-API with the service identity: change check, export, the dump's download and unpacking | no |
| `vre-bridge-server` | `server/` | `server` | the `VreBridge` facade `deposit-server` starts: the nightly run, one refresh, the operator's refresh | no |

`vre-bridge-lift` grants its library to `vre-bridge-core` and `vre-bridge-server` alone (Bazel `visibility`). Its
role bans web frameworks, Maud, database crates, HTTP clients and `tokio` (`.github/scripts/check-capability-deps.sh`,
`just check`): it reads an unzipped dump from a directory it is given and returns the snapshot in memory; the
caller writes it.

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
- **A bad dump is an error, not a panic.** A dump that breaks the format is `LiftError::Violations`, listing every
  violation; production code calls no `expect`, `unwrap`, `panic!` or `unreachable!` (denied in `lift/src/lib.rs`).
  `snapshot` blocks and holds the whole dump in memory: an async caller runs it in `spawn_blocking`.
- **`LiftError` goes to the log only.** Its text names paths, graph IRIs and resource IRIs of the dump.

## Checking the lift on a real dump

`bazel test //areas/deposit/vre-bridge/lift/...` runs the fixture tests. The real-dump tests are `#[ignore]`d;
`just vre-bridge-lift-real-dump <dump dir> <reference snapshot>` runs them, with the reference written by
dsp-incubator's `dao-lift snapshot` at the ported commit from the same dump, both as absolute paths.
`real_dump_matches_the_incubator_lift` compares the two as sets of quads.
