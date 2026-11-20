---
title: "feat: VRE bridge, increment 1 and the bridge side of increment 2"
type: feat
date: 2026-11-20
author: "Balduin Landolt"
status: reviewed(4)
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository
  - name: ops-deploy
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/ops-deploy
prd: 01-vre-bridge-PRD.md
linear_project: P-DEV-301
---

# feat: VRE bridge, increment 1 and the bridge side of increment 2

## Overview

Build the VRE bridge the PRD (`01-vre-bridge-PRD.md`) defines: a capability of the Deposit Area, inside
`deposit-server`, that refreshes allowlisted VRE projects nightly into the `sync` and `media` stand-ins, as
generations made current by a Ready token; and make `sync`, `media` and CPE read those stand-ins by token and
follow a refresh without a restart. Increment 1 (0803 from prod) is planned in full; of increment 2 (Keller)
only the bridge's side (DaTEI file mapping, text downloads, file manifest, IIIF entries) against test
fixtures. The work ships as a stack of draft PRs on #610 in dsp-repository, one phase per PR, plus one draft
PR in ops-deploy.

## Problem Statement / Motivation

0803 reaches CPE as a snapshot a person made by hand and committed; every change needs a person, a commit and
a release, and a project that is still edited in the VRE cannot follow (PRD, *Context*). DaTEI's first
projects and further CPE projects need their data on the server now, and Keller is due mid-December 2026.

## Proposed Solution

The design in short; the phases hold the detail, and *Owner decisions (provisional)* the choices this plan
made.

**Crates.** `areas/deposit/vre-bridge/` holds five crates and the shipped inputs:

| Crate | Role | Holds | Third-party |
|---|---|---|---|
| `vre-bridge-lift` | `lift` (new, `areas/deposit/ADR-0005`) | the ported `dao-lift` snapshot closure, `FORMAT.md`, the selection and the public guard, the format and transformation versions | `oxigraph` (no default features), `thiserror` |
| `vre-bridge-core` | `core` | settings parsing, allowlist and server ids, refresh decisions (REQ-1.5 to 1.8, 1.18), the shipped-input digest, outcomes, (increment 2) the file mapping and manifest | `serde`, `serde_json`, `sha2`, `chrono`, `thiserror` |
| `vre-bridge-store` | `store` | the subtrees: generation names, writing a generation, the rights record, replacing and removing Ready tokens, recovery, cleanup, the lock and the nightly-run record | `serde_json`, `chrono`, `rand`, `thiserror`, `tracing` |
| `vre-bridge-client` | `client` (new) | DSP-API: service-identity JWTs, project IRI, change check, export lifecycle, dump download and private unpacking, anonymous file downloads | `reqwest`, `jsonwebtoken`, `uuid`, `zip`, `tempfile`, `serde`, `serde_json`, `url`, `base64`, `chrono`, `thiserror`, `tracing` |
| `vre-bridge-server` | `server` | `VreBridge` facade: settings, start (recovery, schedule), the nightly run, one refresh, the operator's refresh, the metrics | `tokio`, `chrono`, `chrono-tz`, `opentelemetry`, `tracing` |

Shared: `shared/names` (`shared-names`: `is_shortcode`, `is_file_name`) and `shared/stand-in`
(`shared-stand-in`: the layout, the Ready token's type and its encoding, resolving a project, listing known
projects).

**Layout.** For each subtree root (`sync`, `media`), per project:

```
<root>/<SHORTCODE>/.ready                         the Ready token (JSON), replaced by one rename
<root>/<SHORTCODE>/.ready.<generation>.tmp        a token being written; never read
<root>/<SHORTCODE>/<generation>/                  a generation, written whole before a token names it
    sync:  snapshot.nq
    media: access-rights.json, config.kdl, curation.csv, narrative/…, (DaTEI) datei.toml,
           file-layout.xml, file-mapping.json, file-manifest.json, <stored names>
```

**Rights of `media`'s files** (REQ-2.6; P20). Every `media` generation holds `access-rights.json`:
`{"format":"access-rights","version":1,"default":{"access_rights":"public","data_classification":"unclassified"}}`,
covering every file of the generation (shipped inputs, downloads, the manifest, itself). `access-rights.json`
is a reserved name, like `file-manifest.json`. No reader reads it yet (ADR-0012's "no port serves rights"
gap); it is where `media`'s port will take the rights from.

**Modes.** The bridge sets directories to `0755` and files to `0644` with `fs::set_permissions` after creating
each (`OpenOptionsExt::mode` and `DirBuilder::mode` are masked by the umask, so they are not enough), so a
reader with another uid can read them; H1 makes the subtrees readable for `access-server`'s uid.

`<generation>` is `YYYYMMDDTHHMMSSZ-xxxxxxxx`: T in UTC, then 8 lowercase hex digits from `rand`; the same
name in both subtrees. A reader resolves `<root>/<SC>/.ready`, validates it, and reads only below
`<root>/<SC>/<generation>/`. Entries of a root whose name fails `is_shortcode` (`lost+found`, `PROVENANCE`,
`0803-arks.txt`) are never read as projects and never deleted.

**Ready token** (`shared-stand-in`, token version 1), UTF-8 JSON, compact, fields in this order, one
trailing newline:

```json
{"token_version":1,"generation":"20261120T010203Z-9f3a1c2e","previous_generation":"20261119T010155Z-0b1d2e3f","as_of":"2026-11-20T01:02:03.456Z","server":"prod","interim_format_version":1,"transformation_version":1,"inputs_digest":"sha256:<64 lowercase hex>"}
```

`previous_generation` is `null` for a project's first generation. `as_of` is T: the change check's `asOf`
parsed with `chrono` and written `%Y-%m-%dT%H:%M:%S%.fZ` in UTC, keeping every fractional digit it had (the
cutoff derived from it, T − 60 s, is written the same way, so nothing is truncated). `server` is the allowlist's
server id
(`[a-z0-9-]{1,32}`). Provenance is every field but `token_version` and `previous_generation`; the two tokens of
a project hold equal provenance (REQ-3.3). A reader refuses a token whose `token_version` is not `1`, whose
generation fails the pattern, or that is not a regular file (`symlink_metadata`).

**Shipped-input digest.** Over the regular files below `inputs/<SC>/` (none: the empty sequence), sorted by
their relative path's bytes with `/` separators: for each, the path's UTF-8 bytes, one `0x00`, the content's
length as 8 bytes big-endian, the content. SHA-256 over the concatenation, written `sha256:` plus 64 lowercase
hex digits. A symlink, or a path that fails `is_file_name`, below `inputs/` is a configuration error.

**A refresh** (`vre-bridge-server`), for one allowlisted project, holding the lock (`refresh` takes a `&Lock`,
so only its callers acquire it: the nightly run, the operator's refresh and start-time recovery). Every
`vre-bridge-store` call, `unpack`, `inputs_digest` and the work directory's removal run in `spawn_blocking`
with owned arguments; no `GenerationWriter` is held across an `.await`; a `JoinError` (a panic included) ends
the refresh as `Failed`.

0. Recover this project first (`recover` of REQ-3.7, 3.8 for its shortcode), so an operator's refresh, which
   runs in its own process, starts from agreeing tokens too.
1. Read both tokens as `TokenState` (`Missing`, `Broken`: unreadable, malformed or naming a generation
   directory that does not exist, or `Valid`). Forced when either is not `Valid` (REQ-1.7), names another
   server (REQ-1.8), another format or transformation version or inputs digest (REQ-1.18), or the operator
   asked (REQ-1.9). T and `previous_generation` for the new token come from the `sync` token.
2. Resolve the project IRI: `GET /admin/projects/shortcode/{SC}` → `project.id` (public).
3. Change check: `GET /v2/resources/info?cutoff=<T − 60 s>`, header `X-Knora-Accept-Project: <iri>`. Not
   forced and no resource listed: outcome `unchanged`, tokens untouched. A forced refresh, or a project without
   T, asks with `cutoff` = the bridge's clock plus one day, which lists nothing and only yields `asOf` (the
   endpoint has no paging, so a full listing of a large project is never requested; dsp-api parses `cutoff` as
   an `Instant` with no upper bound, released in v40.6.0). T for the refresh is the response's `asOf` (REQ-1.5,
   REQ-1.10). The check sees resources only: an ontology or list change with no resource change waits for the
   next resource change or a version bump (PRD *Context*, known gaps).
4. Export: `POST /v3/projects/{enc iri}/exports?skipAssets=true` with a fresh JWT. `202` → poll
   `GET …/exports/{id}` every `EXPORT_POLL_SECS` until `completed` (`failed` or `EXPORT_TIMEOUT_SECS` →
   failure). `409 export_exists`: `GET …/exports/{details.id}`; an answer means it is ours (REQ-1.20): wait
   (bounded by `EXPORT_TIMEOUT_SECS`) until it is not `in_progress`, `DELETE` it, ask again; this recovery
   happens at most once per refresh and spends no retry, a second `409` naming our own export fails the
   refresh. `404` means another caller's: busy (REQ-1.12, REQ-1.13).
5. `GET …/exports/{id}/download` into the private work directory, then `DELETE …/exports/{id}` (REQ-1.19).
   The delete runs on every path once an id is known. dsp-api refuses to delete a running export
   (`409 export_in_progress`); after a time-out the bridge polls once more for up to `EXPORT_TIMEOUT_SECS`
   and deletes when it ends, else logs the id at error level, and the next refresh's own-export recovery
   (step 4) deletes it.
6. Unpack into the private directory (`vre-bridge-dump-<random>`, mode 0700, under `WORK_DIR`): first validate
   every entry name of the central directory (an absolute path, `..` or a symlink entry fails the refresh),
   then extract exactly `data/rdf/data.nq` and `data/rdf/ontology-<n>.nq` by name; no other entry is extracted
   (REQ-2.10, REQ-2.11). The `tempfile::TempDir` is dropped, deleting zip and tree, on every outcome; a
   directory left by a killed process is swept at start and at each nightly run (*The nightly run*).
7. Lift (`spawn_blocking`): select, guard, transform (REQ-2.1 to 2.6, 2.9). Violations refuse the project and
   are logged one line each (IRI, class, reason), never values, the first 50, then one line with the total.
   A lift error names an IRI or a fixed label, never a literal; a parse error is logged as its kind and line
   number only.
8. (Increment 2) resolve the file mapping, download texts anonymously (REQ-2.7, 2.8, 4.x).
9. Write the new generation in `media` (shipped inputs, `access-rights.json`, then downloads and manifest)
   and in `sync` (`snapshot.nq`), each file `fsync`ed; replace the `media` token, then the `sync` token
   (REQ-3.1, 3.2, 3.13); if the `sync` rename fails, restore the `media` token exactly as it was before this
   refresh (its captured bytes, or no token if it had none) at once and fail the refresh (REQ-3.5); delete
   generations other than the new one and `previous_generation` (REQ-3.4). Generation names are never compared
   for order.
10. Report: log line (REQ-5.1), outcome counter and duration (REQ-5.2), sizes (REQ-5.3).

**The nightly run** at `NIGHTLY_AT` in `TIME_ZONE` (default 02:00 Europe/Zurich): take the lock (`try_lock`
in a loop with a 1 s async sleep, never a blocking `File::lock` on a runtime thread, so shutdown is not held
up), skip if the nightly-run record already holds today's local date (REQ-1.3), write today's date, delete
every `vre-bridge-dump-*` directory in `WORK_DIR` (left by a killed process; REQ-2.11), run recovery
(REQ-3.7, 3.8), then removal (REQ-1.14, 1.15), then each allowlisted project; a busy project goes to a retry
queue with a not-before time `DUMP_RETRY_SECS` later. After the first pass the run sleeps until the earliest
not-before time, retries the projects due, and repeats until each has had `DUMP_RETRIES` retries (default
3 × 20 min); the lock is held throughout, so an operator's refresh meanwhile exits `75` (documented: wait for
the run's end). A missed run is not caught up: the next run is always the next future occurrence (REQ-1.4);
a run cut short by a restart is not resumed that night either (the record is written first), so the operator's
refresh is the lever. A removal (REQ-1.14) is logged at warning level and counted as outcome `removed`, so a
mistyped allowlist that withdraws a live project shows (P21). Each nightly pass runs as its own spawned task
whose `JoinHandle` the scheduler awaits: a panic is logged at error level and the schedule goes on. Time
comes from an injected `Clock` (`now() -> DateTime<Utc>`, `sleep_until`), and after waking the scheduler
re-checks `now >= target`, since a monotonic sleep drifts across host suspend or clock steps.

**Readers.** `sync-store` and `media-store` resolve through the token (REQ-6.1, 6.14); `sync-store` reads
interim format versions in `READABLE = [1]` (REQ-6.2, 6.3). Both serve provenance; `sync-store` polls its
tokens every `SYNC_POLL_SECS` (default 30) on a thread of its own and announces a project whose token appeared,
changed (bytes differ) or was removed (REQ-6.5). `subscribe` reads every token once, synchronously, as the
poll's baseline before it returns, and CPE subscribes before its first activation, so a token replaced during
that activation is announced. CPE re-activates on an announcement for a shortcode it has a folder for, and
keeps its previous activation unless every project it served before loads again with stable, equal provenance
on both ports, or is gone because its token was removed (REQ-6.6 to 6.10, 6.13). A project whose `sync`
snapshot answers `UnknownProject` is `Unknown` before any provenance is compared: the bridge removes the `sync`
token first (REQ-1.14), so during a removal `media` still has a token, and comparing first would hold the
re-activation back for good. A held-back re-activation is
logged at error level naming the project and retried once after ten poll intervals; a restart is the
operator's lever beyond that.

**Configuration** (`deposit-server`'s figment loader, env `EDITOR_*`, optional `editor.toml`; every setting
settable by env alone):

| Variable | Default | Meaning |
|---|---|---|
| `EDITOR_VRE_BRIDGE_SYNC_DIR` | unset | `sync` subtree root. Unset (or `MEDIA_DIR` unset): the bridge does nothing (REQ-1.17) |
| `EDITOR_VRE_BRIDGE_MEDIA_DIR` | unset | `media` subtree root |
| `EDITOR_VRE_BRIDGE_LOCK_FILE` | unset | lock file on local disk; the nightly-run record is `<LOCK_FILE>.nightly`. Unset with output dirs set: configuration error, bridge inactive |
| `EDITOR_VRE_BRIDGE_ALLOWLIST` | empty | `SC=server,SC=server`, e.g. `0803=prod,083C=stage` |
| `EDITOR_VRE_BRIDGE_SERVERS` | empty | `id=base URL,…`, e.g. `prod=https://api.dasch.swiss`; the configured string, verbatim (no trailing slash), is also the JWT `aud` |
| `EDITOR_VRE_BRIDGE_IIIF_BASES` | empty | `id=IIIF base,…`, e.g. `prod=https://iiif.dasch.swiss` (increment 2) |
| `EDITOR_VRE_BRIDGE_KEY_IDS` | empty | `id=kid,…` |
| `EDITOR_VRE_BRIDGE_SIGNING_KEYS` | empty | `Secret`: `id=<base64 PKCS#8 DER Ed25519 private key>,…` |
| `EDITOR_VRE_BRIDGE_SERVICE_NAME` | `dsp-repository` | JWT `iss` and `sub`; the `name` in dsp-api's trusted keys. One per deployment (`dsp-repository-stage`, `dsp-repository-prod`), because dsp-api scopes "own export" by it |
| `EDITOR_VRE_BRIDGE_INPUTS_DIR` | `areas/deposit/vre-bridge/inputs` (image: `/app/vre-bridge/inputs`) | shipped inputs |
| `EDITOR_VRE_BRIDGE_WORK_DIR` | the OS temp dir | parent of the private dump directories; never an output location; in deployment a local host mount (Phase O) |
| `EDITOR_VRE_BRIDGE_NIGHTLY_AT` | `02:00` | local time of the nightly run; stage uses another time than prod (Phase O), since both dump from prod |
| `EDITOR_VRE_BRIDGE_TIME_ZONE` | `Europe/Zurich` | IANA zone |
| `EDITOR_VRE_BRIDGE_DUMP_RETRIES` | `3` | retries of a busy dump slot in a nightly run |
| `EDITOR_VRE_BRIDGE_DUMP_RETRY_SECS` | `1200` | wait before each retry |
| `EDITOR_VRE_BRIDGE_REQUEST_TIMEOUT_SECS` | `120` | project lookup, change check, export trigger, status, delete |
| `EDITOR_VRE_BRIDGE_EXPORT_TIMEOUT_SECS` | `3600` | trigger to `completed` |
| `EDITOR_VRE_BRIDGE_EXPORT_POLL_SECS` | `15` | status poll interval |
| `EDITOR_VRE_BRIDGE_DOWNLOAD_TIMEOUT_SECS` | `3600` | the dump download |
| `EDITOR_VRE_BRIDGE_FILE_TIMEOUT_SECS` | `300` | one anonymous file download (increment 2) |

List values are parsed by the bridge from strings: entries split on `,`, trimmed, empty ones dropped; each
entry split on its first `=`. A duplicate key, an allowlist shortcode failing `is_shortcode`, an allowlist
server without a `SERVERS` entry, or a server the allowlist uses without a key id and signing key is a
configuration error: logged once at start naming the variable (never a secret value), and the bridge stays
inactive; `deposit-server` starts regardless (`areas/deposit/ADR-0005`: the bridge never stops it). So are an
unparsable `NIGHTLY_AT` or `TIME_ZONE`, a signing key that does not sign a probe token, a `WORK_DIR` or
`LOCK_FILE` at or below an output root (compared after `canonicalize`; a symlinked `WORK_DIR` is refused), and
a shipped input named `access-rights.json` or `file-manifest.json`.

**Operator's refresh.** `deposit-server vre-bridge-refresh <SHORTCODE>` (run with `docker exec` in the
container, so it has the deployment's env): installs rustls' ring provider, builds its own tokio runtime (it
does not go through `serve`), loads the configuration, takes the lock without waiting, runs one forced
refresh, prints the outcome line, exits `0` refreshed, `2` refused, `1` failed, `75` lock busy, `64` not
configured or not allowlisted. It exports no metrics (its series would collide with the server's); its outcome
line is the record.

## Alternative Approaches Considered

- **Atomic directory exchange (`renameat2`)** and **file-system notifications** — rejected by the owner: the
  subtrees may be NFS on another VM (`areas/deposit/ADR-0005` Considered Options).
- **One crate for the whole bridge** (the cheapest shape) — rejected: ADR-0003's role bans keep the lift free
  of network and storage code, Bazel `visibility` can hold "only `vre-bridge-server` reaches `deposit-server`"
  and "only `vre-bridge-store` writes the subtrees" only between crates, and the lift's ~7.8k ported lines
  build and test apart from the rest.
- **Shipped inputs compiled into the binary** (`include_bytes!` per file) — rejected: a directory tree needs a
  file list kept by hand in `BUILD.bazel` or a build script; a `files_layer` and a setting follow the editor's
  `EDITOR_PUBLIC_DIR` precedent.
- **Polling in the composition root instead of in `sync-store`** — rejected: ADR-0012 puts the trigger on the
  provider's side, inside the port.
- **Per-project CPE rebuild now** — deferred by the owner to a follow-up ticket (DEV-7989; PRD Follow-up tickets).
- **A guard that drops non-public items** — deferred by the owner (`areas/deposit/ADR-0005` Considered
  Options).

## Technical Considerations

- **NFS (REQ-3.14).** The only atomic step is `rename(<SC>/.ready.<gen>.tmp, <SC>/.ready)` within one
  directory. A reader may see a stale token for an attribute-cache interval; it then reads the previous
  generation, which is kept until the next successful refresh. `sync_all` of the generation's files, then of
  each created directory bottom-up (`<SC>/` included), precedes the token rename, and `<SC>/` is synced again
  after it; a directory `fsync` failure is logged at debug level and ignored (not portable on NFS). The
  rollback token has its own temp name (`.ready.<gen>.rollback.tmp`); recovery deletes names matching
  `.ready.*.tmp`. No lock on the subtrees: the lock file is a local-disk setting, locked with
  `std::fs::File::try_lock` (stable since Rust 1.89; the workspace is on 1.95; `flock(2)` on Linux and macOS,
  owned by the open file description, so a second open of the same path conflicts even in one process),
  opened with `read(true).write(true).create(true).truncate(false)`; `TryLockError::WouldBlock` is "busy",
  any other error a failure; the lock is released on drop. No crate.
- **Security.** The private work directory is created with mode 0700 under `WORK_DIR`, which must not be
  inside an output location (checked at start, see *Configuration*). `admin.nq` and `permission.nq` are never
  extracted (REQ-2.10). The JWT and signing keys are never logged: `deposit-server`'s `Secret` holds them in
  the configuration, and `vre-bridge-core` has its own redacting newtype for the parsed keys (it cannot depend
  on `deposit-server`); `ServiceIdentity` and `Server` implement `Debug` by hand; the JWT goes through
  `bearer_auth` (a sensitive header) and never into a URL; reqwest errors are logged through `without_url()`;
  every bridge function with a span uses `#[instrument(skip_all, fields(project, server))]`, so no argument is
  `Debug`-printed into a span. The bridge has no routes. dsp-api accepts the service token on the four export
  endpoints only; `jti` is not checked for replay there, so tokens live 5 minutes and are minted per request.
  dsp-api refuses to start on a `kid` that two trusted keys share, so the stage and prod deployments' kids
  differ (e.g. `dsp-repository-prod-2026-11`).
- **TLS.** `deposit-server` does not install a rustls provider today except inside `start_pyroscope`
  (`observability.rs:136`, only with `PYROSCOPE_ENDPOINT`), and the workspace's reqwest is built with
  `rustls-no-provider`. Phase 10 installs the ring provider at the start of `serve()` and of the
  `vre-bridge-refresh` path, before any client is built; the client's tests run in their own test binary and
  install it themselves
  (`docs/learnings/build-errors/missing-rustls-provider-install-panics-reqwest-at-client-construction.md`).
- **Metrics.** `init-tracing-opentelemetry` 0.36 with its `metrics` feature (on in `MODULE.bazel`) builds an
  `SdkMeterProvider` and calls `opentelemetry::global::set_meter_provider` (`tracing_subscriber_ext.rs:183`)
  when `otel_config.enabled`, which defaults to true; `deposit-server`'s `init_otel` uses
  `TracingConfig::production()`; `shared-telemetry`'s collector already records through
  `global::meter_with_scope`. Instruments are created after `init_otel` has run (instruments made earlier stay
  no-ops). Instruments, scope `vre_bridge`: counter `vre_bridge.refresh.outcomes`
  `{project, server, outcome}`; histogram `vre_bridge.refresh.duration` (unit `s`) `{project, server,
  outcome}`; histogram `vre_bridge.refresh.size` (unit `By`) `{project, kind=dump|snapshot|media}`. Label
  cardinality is the allowlist's size. `vre_bridge.active` is an observable gauge over an `AtomicBool`.
- **Determinism.** `0803.nq` was produced under oxrdf 0.3.3; the repository pins oxrdf 0.3.4, and the lift
  writes lines through oxrdf's `Display`. The ignored parity test (Phase 1) shows any byte difference on the
  real dump; the synthetic golden test pins today's bytes.
- **Edition.** `dao-lift` is edition 2024 and was gated on Rust 1.93; the port keeps edition 2024 on its
  targets and adds its path to the 2024 line of `justfile`'s `_rustfmt`, as `dsp-cli` is.
- **Bazel.** New third-party crates are `crate.spec` lines in `MODULE.bazel` plus `just repin`; after a repin,
  build the new aliases by name, also with `--platforms=//platforms:linux_x86_64_musl`
  (`docs/learnings/build-errors/crates-hub-aliases-are-manual-wildcard-builds-nothing.md`). Only `oxigraph`
  (Phase 1), `zip` (Phase 8) and `chrono-tz` (Phase 9c) are new; every other crate the plan names already has
  a spec (reqwest 0.13 without `stream`, so the download reads with `Response::chunk()`; jsonwebtoken 10 with
  `rust_crypto`; `opentelemetry_sdk` without `testing`). Files read by `include_str!` need `compile_data`.
  Bazel 9 has no native `sh_test` and the repository has no `rules_shell`, and the image layers are
  `target_compatible_with` musl only, so no test here runs over the image's tars. Bazel `glob` includes dot
  files (checked on Bazel 9.3). Bazel runfiles are per-file symlinks, and the stand-in readers refuse symlinks:
  a test that reads a fixture through `shared-stand-in` takes it from a `copy_to_directory` target (precedent:
  `//areas/access/server:test_files`), never from a `filegroup` in `data`.
- **Interaction with the DaTEI plan** (`docs/specs/2026-11-19-datei-integration/01-feat-datei-integration-plan.md`):
  its `media-store` phase reads `<MEDIA_DATA_DIR>/<shortcode>/<stored name>` and lists the shortcodes whose
  directory holds the Ready token and `datei.toml`. After Phase 5 here, everything but `.ready` lives below
  `<shortcode>/<generation>/`. That adapter resolves `shared_stand_in::read_token` and `generation_dir`, lists
  with `shared_stand_in::known_projects`, tests for `datei.toml` in the token's generation, and pins the
  generation when it creates a project's file source, reading only below it (it has no provenance comparison
  as CPE has, so a per-call resolve could mix generations during a rebuild). `access-rights.json` and
  `file-manifest.json` are reserved names in the generation. Whichever plan lands second adapts; Phase 5
  records it in `media/CONTEXT.md`. `Cpe::follow` returns after the first activation, so DaTEI's start
  trigger is unchanged; a later CPE re-activation can overlap DaTEI's first build (accepted: DaTEI does not
  rebuild until DEV-7938, so the overlap is limited to minutes after a start).

## Implementation Phases

Common to every phase:

- Read first: `CLAUDE.md`, `CONVENTIONS.md`, `docs/src/git-conventions.md`, `ARCH-MAP.md` (the entries the
  phase touches), the `CONTEXT.md` of every capability touched, `docs/adr/CLAUDE.md` before touching an ADR.
- Tests first: each phase lists its tests before its implementation steps; write them failing, then make them
  pass.
- Fast verify while iterating: the phase's `bazel test` line. Before the commit: `just check` and `just test`
  (CONVENTIONS.md *Before Committing*), then `just commit-lint <stack parent branch>` (Phase 1:
  `just commit-lint docs/vre-bridge-prd`): the default base `origin/main` would count the parent PRs' commits
  and trip the one-commit cap.
- One commit per PR, `type(scope): subject (DEV-nnnn)` with the PR's ticket from the table below.
  Reviewers for every phase: `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`,
  `eng:review:security-reviewer`, `eng:review:ivan-reviewer`, `eng:review:dune-reviewer`,
  `eng:review:reference-reviewer`; add `eng:review:observability-reviewer` for Phases 9b, 9c and 10 and
  `eng:review:devops-reviewer` for Phases 10, 12 and O.
- A phase that removes a gap deletes its `Not yet in code:` line in every ADR, `ARCH-MAP.md` and `CONTEXT.md`
  that states it, and adds one for a gap it leaves; the follow-up tickets' ids replace `(not scheduled)` once
  they exist. A bare `ADR-NNNN` is the root series; the bridge's record is always written
  `areas/deposit/ADR-0005` (root ADR-0005 is another record).
- Trait default methods and stubs name unused parameters with a leading `_` (`-Dwarnings` is set in
  `.bazelrc`).

### Pull requests and tickets

Stack order (`gh stack`, each PR one commit, draft, assigned to Balduin). Phases keep their numbers; Phases 6
and 9 are split into lettered parts, one PR each:

| Order | Phase | PR title (proposed) | Ticket | Depends on | Human gate |
|---|---|---|---|---|---|
| 0 | — | #610 `docs(docs): define the VRE bridge and record its decisions (P-DEV-301)` | — | — | — |
| 1 | 1 | `feat(vre-bridge-lift): port dao-lift's snapshot from dsp-incubator` | DEV-7980 Port dao-lift and FORMAT.md into the VRE bridge | — | none |
| 2 | 2 | `feat(vre-bridge-lift): select, guard and record access rights in the interim format` | DEV-7982 Interim format version 1: access rights, data classification, the public guard | 1 | none |
| 3 | 3 | `refactor(shared-names): move the name rules out of cpe-ports` | DEV-7981 shared-names | — | none |
| 4 | 4 | `feat(shared-stand-in): the stand-in layout and the Ready token` | DEV-7983 shared-stand-in | 3 | none |
| 5 | 5 | `feat(sync-store,media-store): know projects by Ready token and read their generation` | DEV-7985 sync and media read generations by Ready token | 2, 4 | none |
| 6 | 6a | `feat(cpe-ports,sync-store,media-store): serve provenance and announce a changed project` | DEV-7622 (existing) | 5 | none |
| 7 | 6b | `feat(cpe-server,access-server): re-activate CPE on an announcement` | DEV-7622 (existing) | 6a | none |
| 8 | 7 | `feat(vre-bridge-store): write generations and Ready tokens` | DEV-7986 VRE bridge: the stand-in writer | 4 | none |
| 9 | 8 | `feat(vre-bridge-client): the DSP-API client with the service identity` | DEV-7984 VRE bridge: DSP-API client | 1 | none |
| 10 | 9a | `feat(vre-bridge-core): settings, refresh decisions and the shipped-input digest` | DEV-7987 VRE bridge: settings and refresh decisions | 2, 4 | none |
| 11 | 9b | `feat(vre-bridge-server): refresh one project` | DEV-7988 VRE bridge: one refresh | 7, 8, 9a | none |
| 12 | 9c | `feat(vre-bridge-server): nightly run, recovery at start and the operator's refresh` | DEV-7993 VRE bridge: nightly run and scheduling | 9b | none |
| 13 | 10 | `feat(deposit-server): start the VRE bridge and offer an operator's refresh` | DEV-7998 VRE bridge: deposit-server wiring, metrics, operations | 9c | none to merge; H3, H4 to operate |
| 14 | 11 | `feat(vre-bridge-server): ship 0803's inputs and pin the media fixture to them` | DEV-7999 VRE bridge: 0803's shipped inputs | 5, 10 | none |
| 15 | 13 | `feat(vre-bridge-core,vre-bridge-server): DaTEI file mapping, text downloads and the file manifest` | DEV-8000 VRE bridge: DaTEI files (increment 2, bridge side) | 11 | none |
| 16 | — | `feat(vre-bridge-lift): read text markup from valueHasXml and ignore standoff` | DEV-8012 (added 2026-11-20, *Execution record*) | 13 | none |
| 17 | 12 | `feat(access-server): carry no stand-in data in the image` | DEV-8001 access-server reads sync and media from the mounts only | 6b, 11 (stacked on 13) | **H5, H6, H7, H8** before merge |
| — | O | ops-deploy: `feat(deposit,repository,dsp): mount the stand-in locations and configure the VRE bridge (DEV-7514)` | DEV-7514 (existing, extended) | — | H1, H2 before merge |

Phase 12 (the image drop) is stacked last although it is numbered before Phase 13: it waits on operations in
every environment, and a gated PR mid-stack would hold back Keller's PR (*Owner decisions*, P8). Its stack
parent is Phase 13's PR (order 15), although it needs nothing of Phase 13's code. Commit scopes name crates
(`CONVENTIONS.md`: no catch-all scope); Phase 11's shipped inputs are read by `vre-bridge-server`, whose test
pins them.

### Execution order and parallel tracks

```
track A:  1 ──► 2 ─────────┬──► 5 ──► 6a ──► 6b ─────────────────────────────────┐
track B:  3 ──► 4 ──┬──────┘                                                     ├──► 12
                    ├──► 7 ───────────┐                                          │
                    └──► 9a (needs 2) ┼──► 9b ──► 9c ──► 10 ──► 11 ──► 13 ───────┘
track C:  8 (needs 1) ────────────────┘                    (11 needs 5)
          O (ops-deploy, standalone)
```

- Group 1, concurrent: Phase 1 (track A), Phase 3 (track B), Phase O (track C, another repository).
- Group 2, concurrent: Phase 2, Phase 4, Phase 8 (each needs only its track's predecessor; 8 needs 1).
- Group 3, concurrent: Phase 5 (needs 2 and 4), Phase 7 (needs 4), Phase 9a (needs 2 and 4).
- Group 4, concurrent: Phase 6a (needs 5) and Phase 9b (needs 7, 8, 9a).
- Group 5, concurrent: Phase 6b (needs 6a) and Phase 9c (needs 9b).
- Then 10, 11, 13 in sequence (all touch `vre-bridge-server`); then 12 (needs 6b and 11, stacked on 13).

**Stack assembly.** The session that runs the plan (`eng:eng-work-execute`) owns it, with the `gh-stack`
skill: when a phase's branch was built in parallel with another, it is rebased onto its stack predecessor
before its PR is pushed; the shared files below are reconciled by keeping both edits; `just repin` is re-run
when `MODULE.bazel` changed on both sides; then `just check`, `just test` and `just commit-lint` run on the
rebased branch. A phase's review findings are amended into that phase's one commit (CI caps a PR at one
commit) before the branch is rebased, and the branches above it are restacked; "before the next phase
starts" means the next phase on the same track.

Shared files edited by phases that may run concurrently, and the kind of edit (reconciled mechanically when
the stack is assembled; keep each edit to its own lines):

| File | Phases | Kind of edit |
|---|---|---|
| `MODULE.bazel`, `MODULE.bazel.lock`, `Cargo.Bazel.lock` (or whatever `just repin` rewrites) | 1 (`oxigraph`), 8 (`zip`), 9c (`chrono-tz`), 10 (`json` on `tracing-subscriber`) | one added `crate.spec` or feature; re-run `just repin` after rebasing |
| `ARCH-MAP.md` | 1, 3, 4, 5, 6a, 6b, 7, 8, 9a, 9b, 9c, 10, 11, 12, 13 | 1 adds the VRE bridge entry; others edit their component's lines or add a `shared/*` entry |
| `docs/src/repo_structure.md` | 1, 3, 4, 5, 7, 8, 9a, 9b | `shared/` tree, crate table rows |
| `CONTEXT.md` (root) | 4, 5, 2 | 4 and 5 add `## Shared` terms; 2 adds Access rights / Data classification |
| `areas/deposit/vre-bridge/CONTEXT.md` | 1, 2, 7, 8, 9a, 9b, 9c, 13 | added terms |
| `docs/src/git-conventions.md`, `CONVENTIONS.md` (crate scopes) | 1, 3, 4, 7, 8, 9a, 9b | one added scope each |
| `.github/scripts/check-capability-deps.sh` (+ `.test.sh`) | 1 (`lift`), 5 (`store` bans file-notification crates), 8 (`client`) | one added `case` arm each, the header comment, the test's fixtures |
| `justfile` | 1 (`_rustfmt` 2024 line, `vre-bridge-lift-real-dump`), 11 (`check` gains the pin script) | separate lines |

Phases that change the same code stay sequential: 3 → 4 → 5 → 6a → 6b (names, sync-store, media-store, CPE),
9b → 9c → 10 → 11 → 13 (vre-bridge-server). At most three tracks run at once; Phase O is small and runs
whenever a track is free.

---

#### Phase 1: Port `dao-lift`'s snapshot closure as `vre-bridge-lift`

PR 1 · DEV-7980 · **Depends on:** none · **Parallel with:** 3, O · **Human gate:** none

Goal: the incubator's snapshot transformation, byte-identical in behaviour, as a library crate in this
repository, with `FORMAT.md` beside it and the VRE bridge registered as a capability.

Source: `dasch-swiss/dsp-incubator@6106ee38166617cd8be5d58411d5bb14741ebc03` (`origin/main` 2026-11-20;
`cpe/tools/dao-lift/` last changed in `039547f523ee03a41782629b11b450793c9b3cb1`, which added deviation D14).
Closure: `src/snapshot/*` (13 modules, 9 test files), `src/dump.rs`, `src/model.rs`, `src/test_support.rs`,
`FORMAT.md`; nothing else (`main.rs`, `transform`, `cut`, `verify`, `standoff*`, `cpe_export*`, `census`,
`analyze`, `ark`, `classify`, `class_identity`, `gate_zero` stay behind). Only `oxigraph 0.5.8`
(`default-features = false`) is used by the closure.

Files: `areas/deposit/vre-bridge/{CONTEXT.md,CLAUDE.md}`,
`areas/deposit/vre-bridge/lift/{BUILD.bazel,FORMAT.md,src/**}`,
`MODULE.bazel` (+ lock), `.github/scripts/check-capability-deps.sh` (+ test), `justfile` (`_rustfmt`),
`ARCH-MAP.md`, `docs/src/repo_structure.md`, `docs/src/git-conventions.md`, `CONVENTIONS.md` (scope list),
`areas/deposit/docs/adr/0005-…md`
(`Not yet in code` for `dao-lift`).

### dsp-repository

- [x] Before editing: `git -C /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator fetch`; confirm
  `git -C … diff 6106ee38 origin/main -- cpe/tools/dao-lift` is empty, else stop and report (the baseline in
  REQ-6.4 would move).
- [x] Export the source: `git -C <incubator> archive 6106ee38 cpe/tools/dao-lift` into
  `.claude/tmp/dao-lift-src/`; record the closure's test names (`grep -h -A2 '#\[test\]'` or `cargo test --
  --list` in the incubator checkout at that commit) into `.claude/tmp/dao-lift-tests-before.txt`.
- [x] Copy the closure to `areas/deposit/vre-bridge/lift/src/` and `FORMAT.md` to
  `areas/deposit/vre-bridge/lift/FORMAT.md`; add `src/lib.rs` declaring the modules (replacing `main.rs`), with
  `#![allow(clippy::collapsible_if, clippy::too_many_arguments)]` carried over.
- [x] Format with the repository's rustfmt at edition 2024; save a copy as
  `.claude/tmp/dao-lift-formatted-baseline/`; every later edit in this phase is checked with `diff -r` against it
  (only the listed edits may differ).
- [x] `MODULE.bazel`: `crate.spec(package = "oxigraph", version = "=0.5.8", default_features = False)`; `just
  repin`; `bazel build $(bazel query 'kind(alias, @crates//:oxigraph)')`.
- [x] `lift/BUILD.bazel`: `rust_library` `vre_bridge_lift` (crate name `vre_bridge_lift`, `edition = "2024"`, deps
  `@crates//:oxigraph`, `@crates//:thiserror`), `visibility` the bridge's own packages only
  (`//areas/deposit/vre-bridge/core:__pkg__`, `//areas/deposit/vre-bridge/server:__pkg__`); `rust_test` with
  `compile_data = ["FORMAT.md"]` (`test_support.rs` has its own temp directory over `std::env::temp_dir()`, so
  no `tempfile`); `rustc_env` `CARGO_MANIFEST_DIR` as `sync/store/BUILD.bazel` does. The closure's only
  third-party crate is `oxigraph`; `thiserror` is new, for `LiftError`.
- [x] `justfile` `_rustfmt`: add `areas/deposit/vre-bridge/lift/**` to the edition-2024 line and exclude it from
  the 2021 line (`':!areas/deposit/vre-bridge/lift/**'` beside `':!dsp-cli/**'`); the justfile stays
  `just --fmt` clean.
- [x] Clippy on the port: `bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect
  --output_groups=clippy_checks //areas/deposit/vre-bridge/lift:all` (what `just check` runs, with `-Dwarnings`
  from `.bazelrc`); fix each finding, or add a crate-level `allow` with a one-line reason where the fix would
  change behaviour.
- [x] Library API (edit): `pub fn snapshot(dump_dir: &Path) -> Result<Snapshot, LiftError>` wrapping `Dump::load`
  and `build`; `Snapshot::write_to(&self, &mut dyn Write) -> io::Result<()>` public; `Snapshot::stats() ->
  &Stats`; `Stats` public with the counts `Stats::print` printed.
- [x] Edit: remove every `eprintln!` (22, `snapshot/mod.rs`): the caller logs `Stats`; `run(args)` and
  `write_atomic` go (the bridge writes files itself).
- [x] Edit: the 7 `.expect` (`index.rs:116`, `resource.rs:56,329,330,333,347,350`) and the `unreachable!`
  (`resource.rs:314`) become `LiftError::Internal { what: &'static str, subject: String }`; production code has no
  panic path (`clippy::expect_used` and `clippy::unwrap_used` denied in this crate's non-test code).
- [x] Edit: the 15 ignored real-dump tests read `VRE_BRIDGE_LIFT_DUMP` instead of `DAO_LIFT_DUMP`; their
  `#[ignore]` reason names `just vre-bridge-lift-real-dump` (below).
- [x] Test (REQ-6.4, ignored): `real_dump_matches_the_incubator_lift` reads `VRE_BRIDGE_LIFT_DUMP` (an
  unzipped dump) and `VRE_BRIDGE_LIFT_REFERENCE` (the incubator lift's output at the source commit on the same
  dump) and asserts the parsed quads, as sets, are equal (facts, not bytes: oxrdf moved from 0.3.3 to 0.3.4),
  printing every quad on one side only; with either variable unset it fails, not returns
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`).
- [x] `justfile`: recipe `vre-bridge-lift-real-dump DUMP REFERENCE` running `bazel test
  //areas/deposit/vre-bridge/lift:all --test_arg=--ignored --test_env=VRE_BRIDGE_LIFT_DUMP=…
  --test_env=VRE_BRIDGE_LIFT_REFERENCE=…`.
- [x] Test name list after the port equals `.claude/tmp/dao-lift-tests-before.txt` plus the one new test; any
  other difference is a defect.
- [x] `format_doc_tests.rs`: its `include_str!("../../FORMAT.md")` and `include_str!("vocab.rs")` resolve
  unchanged in the new layout (`src/snapshot/`, `FORMAT.md` two levels up); it still checks `vocab.rs` against
  `FORMAT.md` §15.
- [x] `FORMAT.md`: §0 says where it lives now and that this repository's copy is authoritative; no other content
  change in this phase.
- [x] `check-capability-deps.sh`: role `lift` denies `$WEB_FRAMEWORK maud $DATABASE reqwest reqwest-* ureq tokio`
  (a `case` arm in `third_party_allowed`, and the role in the header comment that lists roles and bans); in
  `check-capability-deps.test.sh`, a `lift` fixture in `new_ws`'s per-role workspace and a failing case (a
  `lift` BUILD naming `@crates//:reqwest`).
- [x] `areas/deposit/vre-bridge/CONTEXT.md`: the capability's vocabulary from the PRD (VRE bridge, Refresh,
  Nightly run, T, Dump, Interim format, Interim snapshot, Generation, Ready token, Allowlist), pointing to the
  root `CONTEXT.md` for shared terms.
- [x] `areas/deposit/vre-bridge/CLAUDE.md`: crate map (the five crates, marking which exist), the rule that only
  `vre-bridge-store` writes the subtrees, the private work directory rule.
- [x] `ARCH-MAP.md`: a component entry `areas/deposit/vre-bridge` (purpose, crates with "planned" for the four to
  come, depends on, used by `deposit-server` (planned), durable state: none but the subtrees and the lock file),
  and the Deposit Area summary's `Not yet in code` adjusted.
- [x] `docs/src/git-conventions.md` and `CONVENTIONS.md`: scope `vre-bridge-lift`; `docs/src/repo_structure.md`:
  the crate.
- [x] `areas/deposit/docs/adr/0005-…md`: delete `Not yet in code: dao-lift and FORMAT.md are in the incubator`;
  ADR-0007's line of the same content likewise.
- [x] Commit body names the source repository, the full source SHA and the closure, and lists the edits above (no
  history).
- [x] Verify: `bazel test //areas/deposit/vre-bridge/lift/...` passes; `bash
  .github/scripts/check-capability-deps.test.sh` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-6.4 (the comparison exists as an ignored test, run in H6).

#### Phase 2: Interim format version 1: selection, public guard, access rights, data classification

PR 2 · DEV-7982 · **Depends on:** 1 · **Parallel with:** 4, 8 · **Human gate:** none

Goal: the lift selects, then checks permissions on exactly the selection, then writes rights and
classification; it refuses with every violation listed. Confined to `areas/deposit/vre-bridge/lift/**`, the
bridge's `CONTEXT.md` and the root `CONTEXT.md`.

Design (fixed here):

- `pub const INTERIM_FORMAT_VERSION: u32 = 1;` `pub const TRANSFORMATION_VERSION: u32 = 1;` in `lib.rs`.
- `pub fn lift(dump_dir: &Path) -> Result<Lifted, LiftError>`; `Lifted { snapshot: Snapshot, stats: Stats }`;
  `LiftError::Refused { violations: Vec<Violation> }`, `LiftError::Dump(..)`, `LiftError::Internal {..}`.
- `select(&Dump) -> Selection`: resources not `knora-base:isDeleted true`; values (every value node, link
  values and file values included) not deleted, not superseded (object of `knora-base:previousValue`
  anywhere), and whose resource is selected. Phase 1's `Subjects::dropped` logic moves here; `build` takes a
  `&Selection` and never re-derives it.
- `check(&Dump, &Selection) -> (Rights, Vec<Violation>)`: for each selected resource and value, its own
  `knora-base:hasPermissions` literal from the data graph. Parse: split on `|`; each entry splits on its first
  space into a token and a `,`-separated group list; tokens `RV` < `V` < `M` < `D` < `CR`; order in the literal
  is not relied on. A group counts only when it is `knora-admin:UnknownUser` or
  `http://www.knora.org/ontology/knora-admin#UnknownUser`. The anonymous grant is the highest token on such an
  entry. `V` or better passes; `RV` passes only on a value whose `rdf:type` is `knora-base:StillImageFileValue`,
  and records `restricted-view` for its representation; everything else is a violation:
  `NotViewable { granted: Option<Token> }`, `RestrictedViewNotAllowed`, `MissingPermissions`,
  `MalformedPermissions`. A `Violation` carries the subject IRI, whether it is a resource or a value, and its
  `rdf:type`, never a value.
- Output: every `dao:Resource` and every representation node gains `dao:accessRights` (`"public"` or
  `"restricted-view"`) and `dao:dataClassification` (`"unclassified"`), plain literals. FORMAT.md records them
  as deviations D15 (access rights) and D16 (data classification): rows D15, D16 in §12 (the deviation table;
  `format_doc_tests` requires D1…Dn in order and every §15 status to be `DAO` or a §12 id), and the terms in
  §8 (representations), §2 (resources), §11 (filtering: the selection and the guard; permissions themselves
  still never carried), §14 (the worked example gains the two lines), §15 (vocabulary index) and §16
  (revisions); no section is added, so the section-count test holds. §0 gains the version rule:
  `interim_format_version` 1, bumped when a reader would read a snapshot differently; a reader may omit both
  new terms until its port carries rights.
- The existing tests that call `build` directly (the `tests.rs` `run` helper, the §14 worked-example fixture)
  get their `Selection` from `select` and an all-public `Rights` from a test helper; they do not go through the
  guard, since their fixtures carry no `hasPermissions`.

### dsp-repository

- [x] Tests (guard, `lift/src/guard_tests.rs`, synthetic dumps built in a `TempDir` as `snapshot/tests.rs`
  does): a fully public dump lifts; a seeded resource granting `UnknownUser` nothing refuses with one
  `NotViewable` naming it (REQ-2.4, REQ-6.11); a value granting `UnknownUser` `RV` on a `TextFileValue`
  refuses with `RestrictedViewNotAllowed` (REQ-2.4); a `StillImageFileValue` with `RV` lifts and its
  representation carries `"restricted-view"` (REQ-2.5, REQ-6.11).
- [x] Tests (permission parsing): a selected value without `hasPermissions` refuses with `MissingPermissions`
  (REQ-2.9); a grant `V knora-admin:KnownUser` alone refuses (only UnknownUser counts); the orders
  `V knora-admin:UnknownUser|CR knora-admin:Creator` and the reverse both pass; an unknown token refuses with
  `MalformedPermissions`.
- [x] Tests (selection): a superseded value whose `hasPermissions` grants nothing does not refuse and is not in
  the output; a deleted value likewise; a value of a deleted resource, itself not marked deleted, is neither
  checked nor written; two violations in one dump are both reported.
- [x] Tests (output): every resource and representation in a lifted synthetic dump has exactly one
  `dao:accessRights` and one `dao:dataClassification "unclassified"` (REQ-2.1 to 2.3, 2.6).
- [x] Test: `lift` never reads `admin.nq` or `permission.nq` even when present (Phase 1's `dump.rs` test kept,
  extended to `lift`) (REQ-2.10).
- [x] Golden test: the bytes `lift` writes for the richest synthetic fixture are pinned in
  `lift/testdata/golden.nq`; its failure message says a change of output needs `TRANSFORMATION_VERSION` bumped and
  the golden file regenerated in the same commit.
- [x] Implement `select`, `check`, `Rights`, `Violation` (`lift/src/selection.rs`, `lift/src/guard.rs`) and thread
  `Selection` and `Rights` through `build`.
- [x] Ignored parity test (Phase 1) compares quad sets after removing quads whose predicate is
  `dao:accessRights` or `dao:dataClassification`, and prints the guard's violations when `lift` refuses the
  real dump.
- [x] `FORMAT.md` amended as in *Design*; `format_doc_tests` pass with the new §15 terms; `vocab.rs` gains
  `DAO_ACCESS_RIGHTS`, `DAO_DATA_CLASSIFICATION`.
- [x] `areas/deposit/vre-bridge/CONTEXT.md`: Selection, Public guard, Violation, Public, Restricted view; root
  `CONTEXT.md` `## Shared`: Access rights, Data classification (as the PRD's Vocabulary).
- [x] `vre-bridge/CLAUDE.md`: the transformation-version rule (bump with any output change; the golden test
  enforces noticing), and the format-bump rule: raising `INTERIM_FORMAT_VERSION` to n needs an
  `access-server` release whose `sync-store` reads `[n-1, n]` first (REQ-6.2).
- [x] ADR-0013 and `areas/deposit/ADR-0005`: delete `Not yet in code: the interim format carries no access rights
  or data classification` where it holds only for the format (keep "no VRE bridge writes them" until Phase 10).
- [x] Verify: `bazel test //areas/deposit/vre-bridge/lift/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-2.1, REQ-2.2, REQ-2.3, REQ-2.4, REQ-2.5, REQ-2.6 (for the snapshot), REQ-2.9, REQ-2.10, the
lift part of REQ-6.11; REQ-6.4 still holds modulo the rights lines.

#### Phase 3: `shared-names`

PR 3 · DEV-7981 · **Depends on:** none · **Parallel with:** 1, O · **Human gate:** none

Goal: `is_shortcode` and `is_file_name` live in `shared/names`; `cpe-ports` re-exports them so CPE's
callers and the incubator's vendored copy keep compiling (a ports crate may depend on `shared-*`, ADR-0003).

Files: `shared/names/{BUILD.bazel,src/lib.rs}`, `areas/access/cpe/ports/{BUILD.bazel,src/lib.rs,src/names.rs}`,
`areas/access/media/store/{BUILD.bazel,src/lib.rs}`, `shared/README.md`, `ARCH-MAP.md`,
`docs/src/repo_structure.md` (`shared/` tree, crate table, the `cpe-ports` row naming the name rules),
`areas/access/media/CONTEXT.md`, `areas/access/cpe/CONTEXT.md`, ADR-0002 / ADR-0007 / `areas/deposit/ADR-0005`
`Not yet in code` lines, `docs/src/git-conventions.md`, `CONVENTIONS.md`. Callers checked: `cpe-ports`
(`lib.rs`, `fake.rs`, `contract.rs`), `cpe-server` (`discovery.rs`), `access-server` (`serve.rs`) use
`cpe_ports::is_shortcode` and keep compiling through the re-export; `media-store` is the only crate that
switches its import. `sync-store` gains its `shared_names` dep in Phase 5, where it uses it.

### dsp-repository

- [x] Move the two functions and their tests from `cpe/ports/src/names.rs` to `shared/names/src/lib.rs` unchanged
  (tests move with them; doc comments drop the port-specific links and say what each name rule guards, keeping
  in plain text the sentence that `is_shortcode` is stricter than `shared-metadata`'s `is_valid_shortcode`; no
  `areas/…` path in the shared crate's comments, which `check-shared-paths.sh` refuses).
- [x] `shared/names/BUILD.bazel`: `rust_library` `shared_names`, std only, `visibility` listing
  `//areas/access/cpe/ports:__pkg__`, `//areas/access/media/store:__pkg__`, `//areas/access/sync/store:__pkg__`,
  `//shared/stand-in:__pkg__`, and the `vre-bridge` packages `core`, `store`, `server`; `rust_test`.
- [x] `cpe-ports`: `pub use shared_names::{is_file_name, is_shortcode};`, `names.rs` removed; its crate doc says
  the rules come from `shared-names`.
- [x] `media-store` imports from `shared_names`.
- [x] `check-shared-paths.sh` passes for the new crate: it reads `git ls-files`, so run `git add -N shared/names`
  first, then the script.
- [x] Docs: `shared/README.md`, `ARCH-MAP.md` (new `shared/names` entry; `cpe`, `media` depends-on lines),
  `media/CONTEXT.md` closing line, `cpe/CONTEXT.md` where it names `cpe_ports::is_shortcode`; delete the
  `shared-names does not exist` lines in ADR-0002, ADR-0007 and `areas/deposit/ADR-0005`.
- [x] Scope `shared-names` in `docs/src/git-conventions.md` and `CONVENTIONS.md`.
- [x] Verify: `bazel test //shared/names/... //areas/access/cpe/ports/... //areas/access/media/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: the PRD's *Decisions to record*, "The shared crates" (name rules); no REQ changes behaviour.

#### Phase 4: `shared-stand-in`

PR 4 · DEV-7983 · **Depends on:** 3 · **Parallel with:** 2, 8 · **Human gate:** none

Goal: the reading side of the stand-in contract, used by `sync-store`, `media-store` and the bridge.

API (`shared/stand-in/src/lib.rs`, crate `shared_stand_in`; deps `shared_names`, `serde`, `serde_json`,
`thiserror`):

- `pub const TOKEN_FILE: &str = ".ready";` `pub const TOKEN_VERSION: u32 = 1;`
- `pub struct Generation(String)` with `Generation::parse(&str) -> Option<Self>` (pattern above),
  `as_str`, `Display`. Making a new generation name is the writer's (Phase 7), not this crate's.
- `pub struct Provenance { pub generation: Generation, pub as_of: String, pub server: String,
  pub interim_format_version: u32, pub transformation_version: u32, pub inputs_digest: String }`
  (`PartialEq, Eq, Clone, Debug`).
- `pub struct ReadyToken { pub provenance: Provenance, pub previous_generation: Option<Generation> }`;
  `ReadyToken::to_bytes(&self) -> Vec<u8>` (the canonical encoding above) and `ReadyToken::parse(&[u8]) ->
  Result<Self, TokenError>` (rejects another `token_version`, a bad generation, a bad server id, an
  `inputs_digest` not `sha256:` + 64 lowercase hex, unknown fields allowed and ignored).
- `pub fn read_token(root: &Path, shortcode: &str) -> Result<Option<ReadyToken>, TokenError>`: `None` when
  the shortcode fails `is_shortcode` (no file-system access), when `<root>/<SC>` or the token is absent;
  `Err(TokenError::Unreadable(io::ErrorKind))` when either is a symlink or not the expected kind, or an I/O
  error occurs; `Err(TokenError::Invalid(..))` when the bytes fail `ReadyToken::parse` (another version, a bad
  generation, server id or digest, not JSON).
- `pub fn generation_dir(root: &Path, shortcode: &str, generation: &Generation) -> Option<PathBuf>`.
- `pub fn known_projects(root: &Path) -> io::Result<Vec<String>>`: sorted names of real directories directly
  below `root` that pass `is_shortcode` and hold a regular `.ready`.

Files: `shared/stand-in/**`, `shared/README.md`, `ARCH-MAP.md`, root `CONTEXT.md` (`## Shared`: Generation,
Ready token; the opening exception widened to the stand-in contract), ADR-0002's `Not yet in code` line for
`shared-stand-in`, scopes.

### dsp-repository

- [x] Tests: encoding round-trip byte for byte against a literal fixture string; `previous_generation: null`
  round-trips; parse refuses `token_version: 2`, a generation `2026-11-20`, an `inputs_digest` with upper-case
  hex; `read_token` returns `None` for `../x`, `0803` without a directory, `0803` without a token;
  `Err(Unreadable)` for a symlinked token, a symlinked project directory (unix) and a directory named `.ready`;
  `Err(Invalid)` for a malformed token; `known_projects` skips `lost+found`, `PROVENANCE`, a
  lower-case `083c`, a directory without a token, and a symlinked directory (REQ-6.14), and returns sorted names
  for directories created out of order (`0804` before `0803`).
- [x] Implement as specified; `visibility`: `//areas/access/sync/store:__pkg__`,
  `//areas/access/media/store:__pkg__`, the `vre-bridge` packages `core`, `store`, `server`.
- [x] `check-shared-paths.sh` passes (after `git add -N shared/stand-in`).
- [x] Docs as listed, and `docs/src/repo_structure.md`; scope `shared-stand-in`.
- [x] Verify: `bazel test //shared/stand-in/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: the reading side of REQ-3.11, REQ-6.1 and REQ-6.14 (used in Phase 5).

#### Phase 5: `sync` and `media` know projects by Ready token and read their generation

PR 5 · DEV-7985 · **Depends on:** 2, 4 · **Parallel with:** 7, 9a · **Human gate:** none

Goal: both stores resolve a project through its token; the committed 0803 data becomes a fixture laid out as
the bridge writes, still carried in `access-server`'s image (so deployments keep serving 0803 until Phase 12).

Fixture layout (generation `20261007T000000Z-00000000`, T `2026-10-07T00:00:00Z`, the dump's bagging date):

```
areas/access/sync/data/0803/.ready
areas/access/sync/data/0803/20261007T000000Z-00000000/snapshot.nq      (git mv of 0803.nq, bytes unchanged)
areas/access/media/data/0803/.ready
areas/access/media/data/0803/20261007T000000Z-00000000/{config.kdl,curation.csv,narrative/…}  (git mv)
```

Both tokens: `server` `prod`, `interim_format_version` 1, `transformation_version` 1, `previous_generation`
null, `inputs_digest` `sha256:` + 64 zeros (no reader checks it; P7). `0803-arks.txt` and the two `PROVENANCE`
files stay at the roots.

Files:
`areas/access/sync/store/{BUILD.bazel,src/lib.rs,src/error.rs,src/vocab.rs,src/snapshot_tests.rs,tests/committed_0803.rs}`
(and `src/mapping/**` only if the format-version-1 test below needs it),
`areas/access/sync/{BUILD.bazel,CONTEXT.md,data/**}`,
`areas/access/media/store/{BUILD.bazel,src/lib.rs,src/files_tests.rs}`,
`areas/access/media/{BUILD.bazel,CONTEXT.md,data/**}`, the CPE tests that name the fixture paths
(`areas/access/cpe/store-build/tests/it/{main.rs,worked_example.rs,curation_0803.rs}`, whose `include_str!` /
`include_bytes!` paths gain the generation segment; `areas/access/cpe/server/src/**` tests),
`areas/access/server/{BUILD.bazel,src/serve.rs}`, `.github/scripts/check-capability-deps.sh` (+ test),
`ARCH-MAP.md`, ADR-0003 / ADR-0007 / ADR-0012 `Not yet in code` lines, and the documents that name the old
paths: `areas/access/cpe/store-build/README.md`, `areas/access/cpe/CONTEXT.md`,
`areas/access/cpe/projects/incunabula/{CURATION,DATA_ISSUES,PROVENANCE}.md` and `assets/README.md`,
`docs/src/cpe/config-language.md`, `docs/src/dpe/operations.md`, `docs/src/cpe/operations.md`,
`docs/src/repo_structure.md`, `vitrinli/CONTEXT.md`.

### dsp-repository

- [x] Tests (`sync-store`): a project without a token is `UnknownProject`; a lower-case or `../` shortcode is
  `UnknownProject` without touching the file system; a token with `interim_format_version` 2 is `Unavailable`
  (REQ-6.3); a token naming a missing generation is `Unavailable`; a symlinked generation directory or
  `snapshot.nq` is `Unavailable`; two generations present, the token naming the older: the older is served
  (REQ-6.14); the committed fixture still passes `committed_0803.rs` unchanged in its expectations about the
  data (REQ-6.1), and a new test there asserts the two fixture tokens hold equal provenance (REQ-3.3).
  `test_committed_0803_provenance_pin_matches_vocab_pin` is deleted: the `dsp-incubator@<commit>` pin it
  compares leaves `vocab.rs`'s header (below).
- [x] Tests (`media-store`): the file comes from the token's generation; a project directory without a token
  is `UnknownProject`; `.ready` is never served (`is_file_name` refuses it, and it is outside the generation);
  a symlink as the generation directory, and a token naming a missing generation, are `Unavailable`, never
  `UnknownProject`, so a broken project is not read as a removed one (REQ-6.1, 6.14).
- [x] Test (`sync-store`, format version 1 output): an inline snapshot with `dao:accessRights`,
  `dao:dataClassification` (D15, D16) on resources and representation nodes and `dao:authorship`,
  `dao:copyrightHolder` (D14) on a representation maps to the same `ProjectSnapshot` as without them. If the
  mapping refuses unknown predicates there, teach it to omit exactly these terms, as `FORMAT.md` lets a
  reader.
- [x] `sync-store`: `KNOWN` removed; `const READABLE: &[u32] = &[1];`; path
  `<root>/<SC>/<generation>/snapshot.nq` via `shared_stand_in`; `SnapshotError` gains `Token`,
  `UnreadableFormat { version }`, `MissingGeneration`; crate docs rewritten (Interim snapshot, no `KNOWN`, no
  `PROVENANCE` pin, the format-bump rule: `READABLE` gains n before the bridge writes n). `sync-store` has no
  symlink handling today (`fs::read` follows links): it walks `<SC>/<generation>/snapshot.nq` segment by segment
  with `symlink_metadata`, as `media-store`'s `entry()` does, and any symlink or non-regular entry is
  `Unavailable`. Neither store uses a file-system notification crate (REQ-3.14, readers' side):
  `check-capability-deps.sh` gains a `store` arm denying `notify notify-* inotify inotify-*` (header comment
  and a test case with it).
- [x] Tests read the fixture as the readers will: through a `copy_to_directory` target (precedent
  `//areas/access/server:test_files`), never a `filegroup` in `data`, because Bazel runfiles are per-file
  symlinks and the readers refuse a symlinked token or file. `sync-store`'s tests that read `0803.nq` through
  `CARGO_MANIFEST_DIR` switch to such a target.
- [x] `sync-store/src/vocab.rs` header: transcribed from `areas/deposit/vre-bridge/lift/FORMAT.md` §15, interim
  format version 1; D14, D15, D16 terms deliberately omitted (`sync` omits facts its port does not carry). No
  constant changes.
- [x] `media-store`: resolve through `shared_stand_in::read_token`, then the existing segment-by-segment
  `symlink_metadata` walk below `<root>/<SC>/<generation>/`; crate docs say the only writer is the bridge.
- [x] Fixtures moved as above with `git mv`; tokens written; `sync/data/PROVENANCE` and `media/data/PROVENANCE`
  rewritten: the fixture's producer stays `dao-lift@b2226ff4` (true for these bytes), it lacks the rights and
  classification lines format version 1 has (P7), the token is hand-written, and the bridge's output replaces it
  in deployment.
- [x] `filegroup`s: `glob(["data/**"])` already includes `.ready` (Bazel `glob` matches dot files; checked on
  Bazel 9.3); no explicit listing.
- [x] `grep -rn -e '0803\.nq' -e '0803/config\.kdl' -e '0803/curation\.csv' -e '0803/narrative' --include='*.rs'
  --include='*.md' --include='*.bazel' --exclude-dir=specs --exclude-dir=learnings --exclude-dir=.claude .` finds
  no hit to the old layout (`docs/specs/**` and `docs/learnings/**` are historic and stay); a hit that already names the
  generation segment (`0803/20261007T000000Z-00000000/…`) is correct, and roots-only uses such as `sync/data` as
  a directory stay.
- [ ] `access-server`: a Rust test over `test_files` (the `copy_to_directory` of the same data the image layer
  copies) asserts `sync/data/0803/.ready` and `media/data/0803/.ready` are there and the server answers 0803 from
  them, so the image keeps serving 0803. No test runs over the image tars (no `sh_test` without `rules_shell`
  in Bazel 9, and the layers are musl-only); `image_data` copies the same `data` filegroups, so the copy test
  stands for it. `just image-smoke` (needs Docker) still starts the image; without Docker, skip it and say so in
  the PR body.
- [x] `sync/CONTEXT.md` (Known project by token, Interim snapshot, Committed snapshot retired, Interim format →
  root definition), `media/CONTEXT.md` (Known project by token, generation, symlink-window reason, the reserved
  names `.ready`, `access-rights.json`, `file-manifest.json`, and the DaTEI adapter as *Technical
  Considerations* states it), `ARCH-MAP.md` (sync, media durable state and depends-on).
- [x] ADRs: in ADR-0007 delete `sync reads one committed file … KNOWN` and narrow `sync-store reads one format
  version and its port serves no provenance` to the provenance (Phase 6a deletes it); keep
  `sync-store and media-store read files committed to this repository` (ADR-0003) and ADR-0008's line on the
  tests reading the data `access-server` also serves until Phase 12, which makes the data `testonly`.
- [x] Verify: `bazel test //areas/access/sync/... //areas/access/media/... //areas/access/cpe/...
  //areas/access/server/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-6.1, REQ-6.2, REQ-6.3, REQ-6.14, REQ-3.14 (readers' side, with Phase 6a's poll).

#### Phase 6a: both ports serve provenance; `sync` announces a changed project

PR 6 · DEV-7622 · **Depends on:** 5 · **Parallel with:** 9b · **Human gate:** none

Goal: the providers' half of DEV-7622's minimum on std-only ports. CPE does not use the new methods yet; the
default trait methods keep every other implementation (fakes, the incubator's vendored adapters) compiling.

Port design (`cpe-ports`, std only):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {            // as the Ready token holds it; CPE compares and logs, never interprets
    pub generation: String, pub as_of: String, pub server: String,
    pub interim_format_version: u32, pub transformation_version: u32, pub inputs_digest: String,
}
pub trait ChangeListener: Send + Sync { fn changed(&self, shortcode: &str); }

pub trait ArchiveProjection: Send + Sync {
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError>;
    /// `Ok(None)`: the adapter records no provenance (fakes, hand-fed data).
    fn provenance(&self, _shortcode: &str) -> Result<Option<Provenance>, ProjectionError> { Ok(None) }
    /// Calls `listener` for every project whose data appeared, changed or went, from now on.
    fn subscribe(&self, _listener: std::sync::Arc<dyn ChangeListener>) {}
}
pub trait DepositedFiles: Send + Sync {
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError>;
    fn provenance(&self, _shortcode: &str) -> Result<Option<Provenance>, FilesError> { Ok(None) }
}
```

`FakeArchiveProjection` gains `with_provenance(sc, Provenance)` and `announce(sc)` (calls its listeners);
`FakeDepositedFiles` gains `with_provenance`.

`sync-store`: `LiveArchiveProjection::with_poll_interval(Duration)`; `subscribe` reads every token's bytes
synchronously as the baseline before it returns (announcing nothing for them), then starts (once) a thread
`sync-ready-token-poll` that every interval lists `known_projects` plus the shortcodes it saw last time, reads
each `.ready`'s bytes, and calls the listeners for each shortcode whose bytes appeared, changed or disappeared
since the last round (REQ-6.5). An I/O error on one project skips that project for the round and is logged
when the project's state changes (first failure, recovery), not every round. The thread sleeps with
`mpsc::Receiver::recv_timeout(interval)` on a stop channel that the adapter's `Drop` signals and then joins, so
a test's adapter does not leave a polling thread behind. `provenance` reads the token. `media-store`:
`provenance` reads the token.

`access-server`: reads `SYNC_POLL_SECS` (default 30; `0` or unparsable logs and uses 30) and passes it to
`with_poll_interval`; nothing subscribes yet (Phase 6b).

Files: `areas/access/cpe/ports/src/{lib.rs,fake.rs}`, `areas/access/sync/store/src/lib.rs` (+ new `poll.rs`),
`areas/access/media/store/src/lib.rs`, `areas/access/server/src/serve.rs`, `docs/src/dpe/operations.md` (env
table), `areas/access/{sync,media}/CONTEXT.md`, `ARCH-MAP.md`, ADR-0007's provenance `Not yet in code` line.

### dsp-repository

- [x] Tests (`sync-store`, poll): with a 50 ms interval and a recording listener: tokens present at
  `subscribe` are not announced; writing a token announces its shortcode within 2 s; rewriting it with other
  bytes announces again; rewriting identical bytes does not; removing it announces; a token for a
  non-shortcode directory never does; dropping the adapter stops the thread (REQ-6.5).
- [x] Tests (`sync-store`, `media-store`): `provenance` equals the token's provenance; `None` without a token
  (REQ-6.13).
- [x] Tests (`cpe-ports` fakes): `announce` reaches a subscribed listener; `with_provenance` is returned.
- [x] Test (`access-server`): the `SYNC_POLL_SECS` parsing helper gives 30 for unset, `0` and `abc`, and the
  value otherwise.
- [x] Implement as designed.
- [x] Docs: `SYNC_POLL_SECS` in `docs/src/dpe/operations.md`; CONTEXT files; ADR-0007's line that the port
  serves no provenance deleted.
- [x] Verify: `bazel test //areas/access/cpe/ports/... //areas/access/sync/... //areas/access/media/...
  //areas/access/server/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-6.5, REQ-6.13 (ports' side), REQ-3.14 (readers' side, with Phase 5).

#### Phase 6b: CPE re-activates on an announcement

PR 7 · DEV-7622 · **Depends on:** 6a · **Parallel with:** 9c · **Human gate:** none

Goal: CPE follows the announcements of Phase 6a and keeps its previous activation when a re-activation is not
clean.

CPE (`cpe-server`):

- `port_sources` returns a typed failure instead of `Result<_, String>`, so `Unknown` (the `sync` snapshot
  answered `UnknownProject`) is told apart from other failures. It reads the `sync` snapshot's status first: an
  `UnknownProject` makes the project `Unknown` before any provenance is compared (during a removal the `sync`
  token goes first while `media` still has one). Otherwise it reads `p_files_0 = files.provenance(sc)` and
  `p_sync_0 = projection.provenance(sc)` first and `p_files_1`, `p_sync_1` after building the store, and fails
  the project with `ProvenanceMoved` unless all four are equal (two `None` on both ports count as equal)
  (REQ-6.7, 6.13).
- `Activated` gains `statuses: BTreeMap<String, ProjectStatus>` (`Loaded`, `Unknown`, `Failed`).
- `Cpe::follow(&self, projection: Arc<dyn ArchiveProjection>, files: Arc<dyn DepositedFiles>, retry_after:
  Duration)`, blocking, which `access-server` calls instead of `activate` (inside `spawn_blocking`): it
  subscribes first, then runs the first activation, then starts a thread `cpe-reactivate` and returns. The
  listener sends shortcodes into an `mpsc` channel; the thread receives one, drains the rest, ignores
  shortcodes no project folder declares (REQ-6.10, by re-running discovery), runs `activation::from_ports`
  inside `catch_unwind(AssertUnwindSafe(..))` (the workspace builds with `panic = unwind`; confirm in the Bazel
  profile, else REQ-6.8's test fails and says so), and stores the result only if every project `Loaded` in the
  current activation is `Loaded` or `Unknown` in the new one (REQ-6.6 to 6.9). Otherwise it logs at error level
  which project held it back, keeps the current one, and retries once after `retry_after` (a
  `recv_timeout`, so a new announcement is not lost meanwhile; `access-server` passes ten poll intervals). An
  announcement that arrives during the first activation is queued and handled after it. A poisoned lock is
  recovered with `into_inner`.

`access-server`: `activate_in_background` calls `cpe.follow(…)` with the adapters in `Arc`s.

Files: `areas/access/cpe/server/src/{lib.rs,activation.rs}`, `areas/access/server/src/serve.rs`,
`docs/src/cpe/operations.md`, `areas/access/server/CLAUDE.md`, `areas/access/cpe/CONTEXT.md`, `ARCH-MAP.md`,
ADR-0007 / ADR-0008 / ADR-0012 / `areas/deposit/ADR-0005` `Not yet in code` lines.

### dsp-repository

- [x] Test (`cpe-server`): a token replaced after `subscribe` and before the first activation ends is
  announced and leads to a re-activation (no lost update); a held-back re-activation is retried once after
  `retry_after`.
- [x] Tests (`cpe-server`, accepted re-activation, with the fakes): an announcement for a folder's shortcode
  re-activates and serves the new facts (REQ-6.6); a project whose snapshot became `UnknownProject` is no
  longer presented (its prefix answers 503) and the activation is accepted (REQ-6.9), also while its `media`
  provenance is still `Some` and its `sync` provenance `None` (a removal in progress); an announcement for a
  shortcode without a folder runs no activation (REQ-6.10, a fake counting `snapshot` calls).
- [x] Tests (`cpe-server`, kept activation): provenance differing between the ports (REQ-6.7); provenance that
  changes between the before and after reads (REQ-6.7, a fake answering differently on its second call); a
  project that loaded before and now fails (REQ-6.7); a panicking load (REQ-6.8): each keeps the previous
  activation of every project.
- [x] Test (`access-server`, REQ-6.12, in `serve.rs`'s tests): copy the 0803 fixtures to a temp root, build
  the adapters with `LiveArchiveProjection::new(..).with_poll_interval(Duration::from_millis(100))`, run
  `activate_in_background` (which calls `follow`) and the app as the existing tests do, request a page that
  shows a known label; write a new generation in both subtrees whose `snapshot.nq` changes that label, replace
  the `media` then the `sync` token (written by hand from a JSON literal in the token format, since
  `shared-stand-in` is not visible to `access-server`); the page shows the new label within 10 s; no restart.
  It copies the fixtures from `test_files` (`areas/access/server/test_files/areas/access/{sync,media}/data/`), as
  Phase 5's test reads them.
- [x] Implement as designed; `Cpe::activate` keeps its signature (tests and the incubator's host use it).
- [x] Docs: `docs/src/cpe/operations.md` *Refresh* rewritten (poll, re-activation, keep-previous rule, the one
  retry and the restart beyond it); CLAUDE.md and CONTEXT files; ADR-0007's sentence that `access-server`
  "calls `Cpe::activate` on the blocking pool" now names `Cpe::follow`; ADR-0012's two `Not yet in code` lines
  for `sync` replaced by "`media` polls no Ready token (DEV-7938)", "CPE re-activates every project instead of
  rebuilding the one named" and "a project that fails to load keeps the previous activation of every project,
  not only its own" (both naming the per-project rebuild follow-up, DEV-7989); ADR-0007's and ADR-0008's DEV-7622 lines
  likewise; `areas/deposit/ADR-0005`'s DEV-7622 line deleted.
- [x] Verify: `bazel test //areas/access/cpe/... //areas/access/server/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-6.6, REQ-6.7, REQ-6.8, REQ-6.9, REQ-6.10, REQ-6.12, REQ-6.13 (CPE's side).

#### Phase 7: `vre-bridge-store`: generations, Ready tokens, recovery, the lock

PR 8 · DEV-7986 · **Depends on:** 4 · **Parallel with:** 5, 9a · **Human gate:** none

API (`areas/deposit/vre-bridge/store`, crate `vre_bridge_store`; deps `shared_names`, `shared_stand_in`,
`serde_json`, `chrono`, `thiserror`, `tracing`, `rand`; synchronous `std::fs` throughout, so the server calls
it in `spawn_blocking`):

- `pub struct Subtrees { sync: PathBuf, media: PathBuf }` with `Subtrees::new(sync, media)`;
  `pub enum Subtree { Sync, Media }`.
- `pub fn new_generation_name(as_of_utc: &DateTime<Utc>) -> Result<Generation>` (format above, `rand` suffix;
  it formats the string and returns `Generation::parse` of it, a failure being `StoreError::Internal`, no
  `expect`).
- `pub enum TokenState { Missing, Broken(Broken), Valid(ReadyToken) }` with
  `pub enum Broken { Token(TokenError), MissingGeneration(ReadyToken) }`: an unreadable or invalid token, or a
  valid one naming a generation directory that does not exist (kept, so its `previous_generation` is known).
- `pub struct GenerationWriter` (`Subtrees::begin(shortcode, generation) -> Result<GenerationWriter>`): creates
  `<root>/<SC>/<gen>/` in both subtrees (`create_dir`, so it fails if it exists; `begin`'s caller draws a new
  name and retries once on that), `write(Subtree, name, bytes)` (name must pass `is_file_name`; creates parent
  directories; `create_new`; `sync_all`), `copy_tree(Subtree, src_dir)` (refuses a symlink in the source),
  `write_rights_record()` (`access-rights.json` in `media`, as in *Proposed Solution*), `sizes()`. After
  creating each directory and file it sets `0755` / `0644` with `fs::set_permissions` (the umask masks
  `OpenOptionsExt::mode`). `commit(self, token: &ReadyToken) -> Result<()>`: captures the current `media`
  token's bytes (or its absence), syncs the generation's directories bottom-up, writes `.ready.<gen>.tmp`,
  `sync_all`, renames over `.ready` in `media`, then in `sync` (REQ-3.1, 3.2, 3.13); if the `sync` rename
  fails, it restores the captured `media` token (temp `.ready.<gen>.rollback.tmp` + rename) or removes
  `media`'s `.ready` if there was none, and returns the error, leaving the new generation for recovery to
  delete; on success it deletes generations other than `token.generation` and `token.previous_generation`
  (REQ-3.4). `abort(self)` and `Drop` without commit delete the new generation directories (REQ-3.5).
- `pub fn read_tokens(&self, sc) -> (TokenState, TokenState)` (`sync`, `media`).
- `pub fn recover_project(&self, sc) -> Result<()>`: `recover` for one shortcode (used first in every
  refresh).
- `pub fn recover(&self, allowlist: &[String]) -> Result<RecoveryReport>`: for each allowlisted shortcode
  whose tokens are both `Valid` with different provenance, rewrite the `media` token from the `sync` token's
  bytes (REQ-3.7); a lone token is left as it is (the next refresh is forced, REQ-1.7); then delete every
  generation no token names as current or previous, and every `.ready.*.tmp` (REQ-3.8). A token that is
  `Broken(MissingGeneration(t))` still names `t.generation` and `t.previous_generation` for this rule. A project
  with a token that is `Broken(Token(_))` (unreadable, or invalid, which includes a newer `token_version`
  written by a later bridge before a rollback) is skipped for the deletion and logged at warning level: its
  generations are unknown, and an I/O error on NFS can be transient, so deleting would take away the generation
  a reader serves (REQ-3.4).
- `pub fn remove_unlisted(&self, allowlist: &[String], tokenless_at_start: &BTreeSet<String>) -> Result<RemovalReport>`:
  no-op for an empty allowlist; for a shortcode directory not on the allowlist with a token: remove the `sync`
  token, then the `media` token (REQ-1.14; the opposite of a replacement's order, so `sync`, which CPE
  follows, sees the project go first); one in `tokenless_at_start` and still without a token: delete the
  directory in both subtrees (REQ-1.15). Names failing `is_shortcode` are never touched (REQ-3.12).
- `pub fn tokenless_projects(&self) -> BTreeSet<String>`: shortcode directories with a token in neither
  subtree (for the run's start).
- `pub struct Lock`: `Lock::try_acquire(path) -> Result<Option<Lock>>` (`File::try_lock`; `WouldBlock` →
  `Ok(None)`, any other error → `Err`; opened `read(true).write(true).create(true).truncate(false)`; released
  on drop); `Lock::nightly_done_on(&self, date: NaiveDate) -> bool` and `record_nightly(&self, date)` reading
  and writing `<path>.nightly` (temp + rename) (REQ-1.3, REQ-1.11). No blocking acquire: waiting callers loop
  on `try_acquire` with an async sleep.
- `pub fn sweep_work_dir(work_dir: &Path, prefix: &str) -> Result<usize>`: deletes every entry directly below
  `work_dir` whose name starts with `prefix`, judged by `symlink_metadata` (a directory is removed recursively,
  a symlink is unlinked and never followed) (the server passes the client's `DUMP_DIR_PREFIX`; called
  with the lock held; REQ-2.11).
- Every path written is below `<root>/<allowlisted SC>/` or is the lock and its record (REQ-3.10); no symlink
  is ever created (REQ-3.9).

Files: `areas/deposit/vre-bridge/store/**`, `areas/deposit/vre-bridge/CONTEXT.md`, `ARCH-MAP.md` (crate
line), `docs/src/repo_structure.md`, scopes.

### dsp-repository

- [x] Tests (commit): a committed generation is readable through `shared_stand_in::read_token` and
  `generation_dir` in both subtrees with equal provenance (REQ-3.3); before `commit`, no token names it
  (REQ-3.2); between the `media` and the `sync` rename (a test hook), `sync` still names the old generation
  (REQ-3.13); a `sync` rename that fails (a test hook) leaves both tokens as they were: naming the old
  generation, or, on a project's first generation, absent in both subtrees, or, with diverged tokens, the
  `media` token's old bytes (REQ-3.5); a dropped writer leaves no new directory and the old tokens untouched
  (REQ-3.5); after three commits exactly the last two generations remain (REQ-3.4), also when the new T is
  older than the previous one; the `media` generation holds `access-rights.json` with the literal bytes
  specified (REQ-2.6).
- [x] Test (modes, its own `rust_test` target with one test): the test re-executes its own binary through
  `sh -c 'umask 077; exec "$0" --exact <inner> --ignored'` (no `libc` crate in the workspace), and the inner,
  `#[ignore]`d test asserts created directories are `0755` and files `0644`.
- [x] Tests (recovery): `recover` points a diverging `media` token back to the `sync` token's provenance and
  deletes the orphaned generation and stale `.ready.*.tmp` files, the rollback temp included (REQ-3.7, 3.8); a
  lone `media` token and its generation are left alone; a token replaced by a directory of the same name (a
  read error) and a token with `token_version` 2 each leave every generation of that project in place; a token
  naming a missing generation keeps its `previous_generation`.
- [x] Tests (removal): `remove_unlisted` with an empty allowlist does nothing; with `0803` listed, `0804` with
  tokens loses its `sync` token, then its `media` token, and keeps its data; `0805`, tokenless at the run's
  start, is deleted; `lost+found` is untouched (REQ-1.14, 1.15, 3.12).
- [x] Tests (confinement): a name with `..` or a leading `.` is refused; a source tree with a symlink is refused
  (REQ-3.9); a walk after every operation above finds no written path outside the allowlisted shortcodes
  (REQ-3.10).
- [x] Tests (lock): `try_acquire` returns `None` while another handle on the same file holds the lock, and
  `Some` again after that handle is dropped; the nightly-run record round-trips a date; `sweep_work_dir`
  deletes a seeded `vre-bridge-dump-x`, leaves an unrelated directory, and removes a symlink named
  `vre-bridge-dump-y` without following it (checked with `symlink_metadata`).
- [x] Implement as designed; `visibility`: `//areas/deposit/vre-bridge/server:__pkg__` only.
- [x] Docs: CONTEXT terms (Lock file, Nightly-run record), `ARCH-MAP.md` crate line, scope `vre-bridge-store`.
- [x] Verify: `bazel test //areas/deposit/vre-bridge/store/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-2.6 (the rights record), REQ-1.14, REQ-1.15, REQ-3.1, REQ-3.2, REQ-3.3, REQ-3.4, REQ-3.5, REQ-3.7,
REQ-3.8, REQ-3.9, REQ-3.10, REQ-3.11 (writing the token), REQ-3.12, REQ-3.13, REQ-3.14 (writer side); the store
part of REQ-1.3, REQ-1.11, REQ-2.11, REQ-6.11.

#### Phase 8: `vre-bridge-client`: DSP-API with the service identity

PR 9 · DEV-7984 · **Depends on:** 1 · **Parallel with:** 2, 4 · **Human gate:** none

Goal: the bridge's own client (ADR-0002 forbids depending on `dsp-cli`; shapes may be copied from
`dsp-cli/src/client/http.rs`), async `reqwest`, tested with `wiremock`.

API (`areas/deposit/vre-bridge/client`, crate `vre_bridge_client`):

- `pub struct ServiceIdentity { service_name: String, kid: String, key: jsonwebtoken::EncodingKey }` built by
  `ServiceIdentity::new(name, kid, base64_pkcs8_der) -> Result<Self, ClientError>`: decodes base64 (STANDARD,
  trimmed), `EncodingKey::from_ed_der`, then signs a probe token, because `from_ed_der` itself does not
  validate the key; a key that does not sign is an error at start (the bridge stays inactive), never at
  02:00. Errors never echo the input. `fn token(&self, audience: &str) -> Result<String, ClientError>`: header
  `alg EdDSA`, `typ JWT`, `kid`; claims `iss` = `sub` = service name, `aud` = the server's base URL, `iat` =
  now, `exp` = now + 300 s (dsp-api allows at most 900 s), `jti` = UUID v4. `Debug` is written by hand and
  shows the name and kid only.
- `pub struct Server { base: String, identity: ServiceIdentity, client: reqwest::Client, timeouts: Timeouts }`:
  `base` is the configured string verbatim (validated with `Url::parse`, then kept as given, no trailing
  slash added), used both to build request URLs and as the JWT `aud` (dsp-api normalises both sides). The
  client has a `connect_timeout`, `redirect::Policy::none()`, and no client-wide total timeout (it would cut
  the dump download): each non-streaming request sets `.timeout(REQUEST_TIMEOUT)`, the dump download a
  `read_timeout` and an overall `DOWNLOAD_TIMEOUT` around the loop. `Debug` by hand, without the identity's
  key. The JWT is sent with `bearer_auth`.
- `project_iri(sc) -> Result<String>`: `GET /admin/projects/shortcode/{SC}`, no credentials.
- `changes(iri, cutoff: DateTime<Utc>) -> Result<Changes { as_of: String, count: usize }>`: `GET
  /v2/resources/info`, header `X-Knora-Accept-Project`, `cutoff` formatted `%Y-%m-%dT%H:%M:%S%.fZ` (never a
  `+`), no credentials; always with a cutoff (*A refresh*, step 3).
- `export(iri) -> Result<ExportStarted { id } | Busy | Occupied { id }>`: `POST
  /v3/projects/{iri}/exports?skipAssets=true`, the IRI pushed as one path segment with
  `Url::path_segments_mut` (so `/` and `:` are percent-encoded); `409` with `errors[].code == "export_exists"`
  yields `Occupied { id: errors[].details.id }`.
- `export_status(id) -> Result<Option<ExportStatus>>` (`None` on `404`; `in_progress`, `completed`, `failed`);
  `wait_completed(id, poll, limit)`; `wait_finished(id, poll, limit)` (until not `in_progress`);
  `download(id, into: &Path) -> Result<u64>` (reads with `Response::chunk()` into a file, since the
  workspace's reqwest has no `stream` feature; checks the status first and the byte count against
  `Content-Length` when present; returns bytes); `delete_export(id) -> Result<Deleted | StillRunning | Gone>`
  (`204`, `409 export_in_progress`, `404`).
- `pub fn unpack(zip: &Path, into: &TempDir) -> Result<DumpDir>`: first checks every entry name of the central
  directory (`enclosed_name`; an absolute path, `..` or a symlink entry is an error, whatever the entry), then
  extracts exactly `data/rdf/data.nq` and `data/rdf/ontology-<digits>.nq` by name; no other entry is read; never
  `ZipArchive::extract`; requires `data.nq`.
- `pub const DUMP_DIR_PREFIX: &str = "vre-bridge-dump-";` `pub fn private_work_dir(parent: &Path) ->
  Result<TempDir>`: `tempfile::Builder` with prefix `DUMP_DIR_PREFIX` and mode 0700 (set and verified with `std::os::unix::fs::PermissionsExt`).
- `download_file(url) -> Result<Vec<u8>, FileError { Refused(status) | Failed(..) }>`: no credentials; `401`
  and `403` are `Refused` (REQ-2.8), anything else not `2xx` is `Failed` (used in Phase 13).
- Errors (`ClientError`, `thiserror`) carry status codes and the endpoint's template, never a token, a body
  that could hold data, or a URL: a `reqwest::Error` is converted through `without_url()`, never by a blanket
  `#[from]`.

Files: `areas/deposit/vre-bridge/client/**`, `MODULE.bazel` (`zip` 9.x pinned exactly (`=9.8.1` or the newest
9.x when executed; the crate releases often), `default_features = False`, `features =
["deflate-flate2-zlib-rs"]`, reading only: `deflate` would also compile the zopfli compressor; `is_symlink()`
and `enclosed_name()` are in its API) + lock,
`check-capability-deps.sh` (role `client` denies `$WEB_FRAMEWORK maud $DATABASE`; header comment; a `client`
fixture in the test's workspace and a failing case) + test, `areas/deposit/vre-bridge/CONTEXT.md`,
`ARCH-MAP.md`, `docs/src/repo_structure.md`, scopes.

### dsp-repository

- [x] After `just repin`: `bazel build $(bazel query 'kind(alias, @crates//:zip)')`, also with
  `--platforms=//platforms:linux_x86_64_musl`.
- [x] Tests (identity, with a test-only key committed under `client/testdata/`, made by
  `openssl genpkey -algorithm ed25519 -outform DER | base64`, the H2 procedure): the JWT verifies with the
  matching public key under `EdDSA` and carries `kid`, `iss`, `sub`, `aud` equal byte for byte to the configured
  `https://api.example.test` (no trailing slash), `exp - iat == 300`, and a fresh `jti` per token (REQ-1.16);
  `ServiceIdentity::new` with a malformed key is an error that does not contain the input; `format!("{:?}")`
  of `ServiceIdentity` and of `Server` contains no key material.
- [x] Tests (wiremock, real time with timeouts of a few hundred ms; the test binary installs the ring provider
  itself): `changes` sends `cutoff` with `Z` and the project header and no `Authorization` header; `export`
  sends `skipAssets=true`, the encoded IRI as one segment and `Authorization: Bearer`; `409 export_exists`
  yields `Occupied` with `details.id`; `export_status` maps `404` to `None` and the three status strings; a
  `3xx` is not followed.
- [x] Tests (wiremock, lifecycle): `wait_completed` fails on `failed` and on the time limit; `download` writes
  the body to a file and fails on a short body; `delete_export` maps `204`, `409 export_in_progress` and `404`;
  `download_file` sends no `Authorization` header and maps `401` and `403` to `Refused` (REQ-2.7, 2.8); an
  error's `Display` contains no URL.
- [x] Tests (`unpack`): a zip with `data/rdf/data.nq`, `ontology-1.nq`, `admin.nq`, `permission.nq`,
  `bag-info.txt` extracts only the first two (REQ-2.10); a zip holding `../evil.nq` anywhere is an error and
  extracts nothing; the `TempDir` is mode 0700, named `vre-bridge-dump-*`, and gone after drop (REQ-2.11).
- [x] Implement as designed; `visibility`: `//areas/deposit/vre-bridge/server:__pkg__` only.
- [x] Docs and scope `vre-bridge-client`.
- [x] Verify: `bazel test //areas/deposit/vre-bridge/client/...` passes; `bash
  .github/scripts/check-capability-deps.test.sh` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-1.16, REQ-2.7, REQ-2.8 (client side), REQ-2.10, REQ-2.11 (client side), the client part of
REQ-1.19, REQ-1.20, REQ-6.11.

#### Phase 9a: `vre-bridge-core`: settings, refresh decisions, the shipped-input digest

PR 10 · DEV-7987 · **Depends on:** 2, 4 · **Parallel with:** 5, 7 · **Human gate:** none

Goal: the bridge's pure logic, with no file system beyond the shipped inputs and no network.

`vre-bridge-core` (deps `shared_names`, `shared_stand_in`, `vre_bridge_lift` (versions only), `sha2`,
`serde`, `serde_json`, `chrono`, `thiserror`):

- `pub struct RawSettings` with one field per variable of *Configuration* (`sync_dir`, `media_dir`, `lock_file`,
  `allowlist`, `servers`, `iiif_bases`, `key_ids`, `signing_keys` (`Option<SigningKeys>`: a raw string with a
  redacting `Debug` and `From<String>`, re-exported by `vre_bridge_server` beside `RawSettings`, so
  `deposit-server` builds it with `secret.expose().to_owned().into()`), `service_name`, `inputs_dir`, `work_dir`, `nightly_at`, `time_zone`, `dump_retries`,
  `dump_retry_secs`, `request_timeout_secs`, `export_timeout_secs`, `export_poll_secs`,
  `download_timeout_secs`, `file_timeout_secs`), each `Option<String>` or `Option<u64>` except `signing_keys`,
  and
  `Settings::parse(RawSettings) -> Result<Settings, SettingsError>`, pure and testable, every variable included
  (`IIIF_BASES` and `FILE_TIMEOUT_SECS` too; Phase 13 only uses them); `NIGHTLY_AT` parsed as `HH:MM`;
  `TIME_ZONE` kept as a string and validated in 9c, where `chrono-tz` arrives.
- `SigningKey`, a redacting newtype for each parsed key (`Debug` prints `<redacted>`), since `core` cannot use
  `deposit-server`'s `Secret`.
- `ServerId` (`Clone, Eq, Ord, Hash, Debug`), `pub struct AllowlistEntry { pub shortcode: String, pub server: ServerId }`, `Allowlist` with
  `entries() -> &[AllowlistEntry]`, `inputs_digest(dir) -> Result<String>` (*Proposed Solution*), which also refuses a
  shipped file named `access-rights.json` or `file-manifest.json` (a configuration error).
- `RefreshReason`, `Running { interim_format_version, transformation_version, inputs_digest }`,
  `decide(sync: Option<&ReadyToken>, media: Option<&ReadyToken>, entry: &AllowlistEntry, running: &Running,
  operator: bool) -> Decision { Forced(reason) | CheckChanges { cutoff } }`; the server passes `None` for a
  token that is `Missing` or `Broken` (Phase 7's `TokenState`), so both count as missing (REQ-1.7); both tokens
  are compared with `running` and the entry's server.
- `Outcome { Unchanged, Refreshed, Refused(RefusalReason), Failed(FailureReason), Removed }` with typed reasons,
  and one exhaustive mapping to an exit code used by 9c (`Refreshed` and `Unchanged` `0`, `Refused` `2`,
  `Failed` `1`; `Removed` never comes from a refresh and maps to `1` if it ever does).

Files: `areas/deposit/vre-bridge/core/**`, `areas/deposit/vre-bridge/{CONTEXT.md,CLAUDE.md}`, `ARCH-MAP.md`,
`docs/src/repo_structure.md`, scopes.

### dsp-repository

- [x] Tests (settings): the list grammar, duplicates, a lower-case shortcode, an unknown server, a missing key,
  an unparsable `NIGHTLY_AT`: each a named error that never echoes a secret; `format!("{:?}", …)` of
  `RawSettings` and of `Settings` contains no key.
- [x] Tests (digest): `inputs_digest` of a fixed tree (files created out of sorted order) equals a hard-coded
  value, changes with one byte and with a renamed file, is the empty-sequence digest for a missing directory,
  and refuses a symlink and a file named `access-rights.json`.
- [x] Tests (`decide`): no token in one subtree, a broken token (REQ-1.7); server differs (REQ-1.8); each
  version differs, the digest differs (REQ-1.18); operator (REQ-1.9); otherwise `CheckChanges` with
  `cutoff = T − 60 s` at full precision (a T with 6 fractional digits keeps them) (REQ-1.5).
- [x] Implement as designed; `visibility`: `//areas/deposit/vre-bridge/server:__pkg__` only.
- [x] Docs: `vre-bridge/CONTEXT.md` (Outcome, Refusal vs Failure), `ARCH-MAP.md` crate line, scope
  `vre-bridge-core`.
- [x] Verify: `bazel test //areas/deposit/vre-bridge/core/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-1.5 (cutoff), REQ-1.7, REQ-1.8, REQ-1.9 (decision), REQ-1.18.

#### Phase 9b: `vre-bridge-server`: one refresh

PR 11 · DEV-7988 · **Depends on:** 7, 8, 9a · **Parallel with:** 6a · **Human gate:** none

Goal: *A refresh* steps 0 to 10, end to end against wiremock and temp subtrees, without `deposit-server`.

`vre-bridge-server` (deps the four crates, `tokio`, `chrono`, `tracing`; `opentelemetry` comes with Phase 10;
dev-deps `wiremock`, `tempfile`, all already in `MODULE.bazel`):

- `trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; async fn sleep_until(&self, t: DateTime<Utc>); }`
  (or a boxed-future equivalent), a system implementation and a fake for tests.
- `struct Refresher { vre: Arc<dyn Vre>, report: Arc<dyn Report>, subtrees: Subtrees, settings: Arc<Settings>,
  clock: Arc<dyn Clock> }` with `refresh(&self, _held: &Lock, sc, operator: bool) -> Result<Outcome, Busy>`
  implementing *A refresh*. `Busy` (another caller's export holds the slot) is not an `Outcome`: it is never
  reported or counted; 9c retries it in a nightly run and turns it into `Failed` for an operator. Two traits
  are defined here for testability: `trait Vre`, whose methods take the entry's `&ServerId` (implemented over
  a `BTreeMap<ServerId, vre_bridge_client::Server>`), and
  `trait Report` (log line + metrics; a recording fake in tests, the log-line adapter real here, the OTel
  adapter in Phase 10). The subtrees use `vre_bridge_store` directly over temp dirs, every call in
  `spawn_blocking`.
- `#[instrument(skip_all, fields(project, server))]` on `refresh`.
- The dump directories' prefix is one constant, `vre_bridge_client::DUMP_DIR_PREFIX`, which the server passes
  to `vre_bridge_store::sweep_work_dir` (a test asserts a directory made by `private_work_dir` is swept).

Files: `areas/deposit/vre-bridge/server/**`, `areas/deposit/vre-bridge/{CONTEXT.md,CLAUDE.md}`,
`ARCH-MAP.md`, `docs/src/repo_structure.md`, scopes.

### dsp-repository

Tests use wiremock with real time and timeouts of a few hundred ms from `Settings`; no wiremock or real
`spawn_blocking` runs under paused tokio time (paused time auto-advances while a task waits on a socket or a
blocking thread, so timeouts would fire at once).

- [x] Tests (change check): a forced refresh and a project without T call `/v2/resources/info` with a
  `cutoff` one day ahead of the bridge's clock, never without one, and take T from `asOf` (REQ-1.5, 1.10).
- [x] Tests (recovery first): an operator's refresh of a project whose tokens diverge first points the `media`
  token back, then refreshes (REQ-3.7).
- [x] Tests (success paths; a recording `Report`): an unchanged project is `Unchanged`, tokens untouched, no
  export call; a changed one is `Refreshed`, both tokens name a new generation with T = `asOf` (REQ-1.6, 1.10),
  the export is deleted (REQ-1.19), sizes are reported (REQ-5.3).
- [x] Tests (refusal and failure): a guard refusal is `Refused`, tokens untouched, export deleted, violations
  logged, at most 50 lines plus a total (REQ-2.4, 3.5); an export `failed` is `Failed`, export deleted (REQ-1.13,
  1.19); an unreachable server is `Failed`; a panic in the blocking lift is `Failed`.
- [x] Tests (`409`): naming our own export: finished, deleted, asked again, `Refreshed` (REQ-1.20); a second
  `409` naming our own export in the same refresh: `Failed`; our own export still `in_progress` after
  `EXPORT_TIMEOUT_SECS`: `Failed`; naming another caller's (its status `404`): `Err(Busy)`, tokens untouched,
  nothing reported (the retry is 9c's).
- [x] Tests (delete): an export that timed out answers `DELETE` with `409 export_in_progress`; the bridge
  waits, deletes it when it ends, and the outcome stays `Failed` (REQ-1.19).
- [x] Tests (scope): each project goes to the server its entry names (REQ-1.2).
- [x] Test (logs, REQ-2.11 and REQ-5.1): a tracing subscriber capturing output across a refusing and a
  malformed-dump refresh finds the outcome line (project, server, outcome, reason, duration) and no literal of
  the dump's `data.nq` or `admin.nq`, no key and no JWT.
- [x] Test (REQ-2.11): after every outcome above, the `WORK_DIR` used is empty.
- [x] Implement as designed.
- [x] Docs: `vre-bridge/CLAUDE.md` (the traits, the blocking rule), `ARCH-MAP.md` crate line, scope
  `vre-bridge-server`.
- [x] Verify: `bazel test //areas/deposit/vre-bridge/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-1.2, REQ-1.5, REQ-1.6, REQ-1.10, REQ-1.13 (one refresh), REQ-1.19, REQ-1.20, REQ-2.4 (logging),
REQ-2.11, REQ-3.5, REQ-3.7 (refresh), REQ-5.1 (content), REQ-5.2 and REQ-5.3 (values, through the recording
`Report`), REQ-6.11 (bridge part).

#### Phase 9c: `vre-bridge-server`: nightly run, start, the operator's refresh

PR 12 · DEV-7993 · **Depends on:** 9b · **Parallel with:** 6b · **Human gate:** none

Goal: the bridge's schedule and entry points, still without `deposit-server`.

- `vre_bridge_server` re-exports 9a's `RawSettings` and `SigningKeys` (core's `visibility`
  stays `server` only; `deposit-server` depends on `vre_bridge_server` alone). `VreBridge` wraps a
  `Refresher`; `VreBridge::from_raw(RawSettings) -> VreBridge` (`Inactive(reason)` when not configured or
  invalid, REQ-1.17; `TIME_ZONE` validated with `chrono-tz` here; each `ServiceIdentity::new` signs its probe
  token here); `VreBridge::start(&self)` spawns start-time work
  (try-lock; skipped when busy: sweep `WORK_DIR`, recovery) and the scheduler, returns nothing and never
  blocks or panics.
- `nightly_run()` as in *Proposed Solution*; the scheduler computes the next `NIGHTLY_AT` in `TIME_ZONE` with
  `chrono-tz` (`from_local_datetime`: `Single` as is, `Ambiguous(a, _)` takes `a`, a gap steps forward 15 min at
  a time until valid; the next occurrence is the first candidate after now), through the injected `Clock`.
- `operator_refresh(sc) -> ExitCode` (try-lock, `64`/`75` rules, then the 9a mapping).

Files: `areas/deposit/vre-bridge/server/**`, `MODULE.bazel` (`chrono-tz`, newest 0.10.x; record the version)
+ lock, `areas/deposit/vre-bridge/{CONTEXT.md,CLAUDE.md}`, `ARCH-MAP.md`.

### dsp-repository

- [x] After `just repin`: build the `chrono-tz` aliases by name, also for musl.
- [x] Tests (scheduler, a fake `Clock` and a fake `Vre`, `#[tokio::test(start_paused = true)]` on a
  current-thread runtime, no wiremock): start at 03:00 schedules tomorrow 02:00, not now (REQ-1.4); DST days
  (2027-03-28 and 2027-10-31 in Europe/Zurich) give the documented instants; a restart at the second 02:30 of
  2027-10-31 with the record holding that date schedules the next day; a panicking pass is logged and the next
  night still runs.
- [x] Tests (nightly): two runs on one local date make one run (REQ-1.3); the run order is sweep, recovery,
  removal, projects (REQ-3.7, 3.8, 1.14, 1.15); a busy project is retried after `DUMP_RETRY_SECS` up to
  `DUMP_RETRIES` times, then `Failed` (REQ-1.12, 1.13), the queue sleeping to the earliest not-before time and
  retrying only the projects due; an operator refresh during the run, between retries, exits `75` (REQ-1.11);
  one project failing leaves the next one refreshed (REQ-3.6); a removal is reported as outcome `Removed` and
  logged at warning level; a shortcode not on the allowlist is never requested (REQ-1.1).
- [x] Tests (start): `start` with tokens diverging runs recovery (REQ-3.7) and deletes a seeded
  `vre-bridge-dump-x` in `WORK_DIR` (REQ-2.11); with the lock held elsewhere it does neither; `from_raw` without
  output dirs is `Inactive` and `start` spawns nothing that touches the file system (a temp dir given only as
  `INPUTS_DIR` stays unread) (REQ-1.17); a `WORK_DIR` or `LOCK_FILE` inside an output root, an invalid
  `TIME_ZONE`, and a signing key that does not sign are `Inactive` with a named reason.
- [x] Tests (operator): busy dump slot: `Failed` at once (REQ-1.13); not allowlisted: `64`; lock held: `75`.
- [x] Implement as designed.
- [x] Docs: `vre-bridge/CLAUDE.md` (run order), CONTEXT (Nightly run details).
- [x] Verify: `bazel test //areas/deposit/vre-bridge/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-1.1, REQ-1.3, REQ-1.4, REQ-1.9 (operator function), REQ-1.11, REQ-1.12, REQ-1.13 (retries and
operator), REQ-1.14, REQ-1.15 (run order), REQ-1.17, REQ-2.11 (the sweep), REQ-3.6, REQ-3.7 (at start and per
run), REQ-3.8.

#### Phase 10: `deposit-server` starts the VRE bridge; metrics; operator's refresh; operations docs

PR 13 · DEV-7998 · **Depends on:** 9c · **Parallel with:** 6b (if still running) · **Human gate:** none to
merge; operating it needs H3 and H4 and the ops-deploy PR (H1, H2)

Files:
`areas/deposit/server/{BUILD.bazel,src/config.rs,src/main.rs,src/cli.rs,src/serve.rs,src/observability.rs,CLAUDE.md}`,
`MODULE.bazel` (+ lock), `areas/deposit/vre-bridge/server/{BUILD.bazel,src/report.rs}` (the OTel `Report`;
`opentelemetry` and `opentelemetry_sdk` deps), `areas/deposit/vre-bridge/inputs/BUILD.bazel`
(`filegroup` `inputs`, `glob(["*/**"], allow_empty = True)`, so the BUILD file itself is not an input;
`visibility` `//areas/deposit/server:__pkg__` and `//areas/deposit/vre-bridge/server:__pkg__`, the latter for
Phase 11's test),
`docs/src/vre-bridge/operations.md` (new) + `docs/src/SUMMARY.md`, `docs/src/editor/operations.md` (the shared
image's env), `ARCH-MAP.md`, `areas/deposit/docs/adr/0004-…md` and `0005-…md` `Not yet in code` lines.

### dsp-repository

- [x] Tests (`config.rs`, `figment::Jail`): every variable of *Configuration*, `IIIF_BASES` and
  `FILE_TIMEOUT_SECS` included, is read from env alone and from `editor.toml`; `EDITOR_VRE_BRIDGE_SIGNING_KEYS`
  is an `Option<Secret>` (as `smtp_password` is) and, given as a number-like value, does not leak through an
  error (as `Secret`'s existing tests); `Debug` of the config shows no key; defaults as tabled.
- [x] Test: `deposit-server`'s start with no bridge variables and no `PYROSCOPE_ENDPOINT` logs the bridge
  inactive and serves `/healthz` (REQ-1.17); with bridge variables set and no `PYROSCOPE_ENDPOINT`, building
  the bridge's client does not panic (the ring provider is installed by `serve()` itself). The image's `env`
  sets `EDITOR_VRE_BRIDGE_INPUTS_DIR` and no output location: no test reads an image's env here (no `sh_test`,
  musl-only layers), so the BUILD comment above the env dict says why no output location may be added, and
  review holds it; the start test above holds the behaviour.
- [x] Test: `vre-bridge-refresh` without configuration exits `64`; with a held lock exits `75` (`assert_cmd`
  on the binary, wired as `dsp-cli/BUILD.bazel` does: `rustc_env` `CARGO_BIN_EXE_…` = `$(rootpath …)` and the
  binary in `data`).
- [x] Test (`report.rs`): with an `SdkMeterProvider` whose reader is `PeriodicReader::builder(exporter).build()`
  over a test-local `PushMetricExporter` that stores each `ResourceMetrics` it is given (all public with the
  workspace's `metrics` feature in `opentelemetry_sdk` 0.31; `ManualReader` needs
  `experimental_metrics_custom_reader` and `InMemoryMetricExporter` needs `testing`, and neither feature is
  added, since crate features are unioned into production builds), then `provider.force_flush()`: one
  refreshed and one failed outcome produce `vre_bridge.refresh.outcomes` points with
  `{project, server, outcome}`, a duration point, and three size points for the refreshed one (REQ-5.2, 5.3);
  the observable gauge `vre_bridge.active` reads `1` for an active bridge and `0` for an inactive one, so a
  misconfigured deployment is visible. (The `PeriodicReader` route is unrun; if it fails, stop and report
  rather than adding a feature.)
- [x] `observability.rs` / `serve.rs`: install rustls' ring provider at the start of `serve()`
  (`let _ = rustls::crypto::ring::default_provider().install_default();`, as `start_pyroscope` does), before
  any client is built; the metrics instruments are created after `init_otel`.
- [x] `config.rs`: the fields, `vre_bridge_settings() -> vre_bridge_server::RawSettings` (as
  `identity_settings`, `editor_settings` do; the core crate stays invisible to `deposit-server`).
- [x] `serve.rs`: construct `VreBridge` and `start` it after `Editor`, off the listener's path.
- [x] `cli.rs`/`main.rs`: subcommand `vre-bridge-refresh <SHORTCODE>` beside `serve` and `healthcheck`; it
  installs the ring provider, builds its own tokio runtime (`main()` is synchronous; `serve()` is
  `#[tokio::main]`), keeps `install_tracing_panic_hook`, sets up a plain
  `tracing_subscriber::fmt().json().with_env_filter(..)` subscriber without `init_otel` (which would install a
  global meter provider and export) and sets `SUBSCRIBER_READY` as `init_otel` does, and calls
  `operator_refresh`. `MODULE.bazel`'s `tracing-subscriber` spec gains the `json` feature explicitly (today it
  arrives only through `init-tracing-opentelemetry`'s feature union); `just repin`. A test runs the subcommand with
  `OTEL_EXPORTER_OTLP_ENDPOINT` pointing at a closed local port and asserts it exits within the test's time
  limit with its outcome line and no export error logged.
- [x] `BUILD.bazel`: dep `//areas/deposit/vre-bridge/server:vre_bridge_server` (granted to
  `//areas/deposit/server:__pkg__` only); `files_layer` `image_vre_bridge_inputs` from
  `//areas/deposit/vre-bridge/inputs:inputs` to `app/vre-bridge/inputs` (build it with
  `--platforms=//platforms:linux_x86_64_musl` while the filegroup is still empty, to confirm an empty layer
  builds); image env `EDITOR_VRE_BRIDGE_INPUTS_DIR=/app/vre-bridge/inputs`; no output location in the image env
  (the Cloud Run previews run this image publicly).
- [x] `docs/src/vre-bridge/operations.md`: settings table, layout, the operator's refresh (`docker exec
  <container> /app/deposit-server vre-bridge-refresh 0803`, exit codes; `75` during a nightly run means wait for
  its end, which can last the retries), recovery, a run cut short by a restart is not resumed that night,
  metrics and suggested alerts (a project `refused` or `failed` two nights running; any `removed`;
  `vre_bridge.active` 0; no outcome for 48 h), key generation (`openssl genpkey -algorithm ed25519`, PKCS#8
  DER base64 for the bridge, SPKI DER base64 for dsp-api; one key, kid and service name per deployment, kids
  unique across the trusted keys of a DSP server), the change check's blind spots (ontology and list changes;
  *A refresh* step 3), a busy slot from the other deployment's run being retried, the format-bump release
  order, CPE's held-back re-activation and the restart as the lever beyond its one retry, one writer per
  location (two deployments must never share a location), and the accepted race: two operator refreshes within
  a reader's NFS attribute-cache interval can delete the generation a reader still reads.
- [x] ADRs: delete `the VRE bridge has no crate` (ADR-0003, `areas/deposit/ADR-0004`, `areas/deposit/ADR-0005`'s
  crate line); ADR-0013's "no VRE bridge writes them" deleted.
- [x] Verify: `bazel test //areas/deposit/server/... //areas/deposit/vre-bridge/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-1.9 (operator path), REQ-1.17, REQ-5.1, REQ-5.2, REQ-5.3.

#### Phase 11: 0803's shipped inputs; the `media` fixture pinned to them

PR 14 · DEV-7999 · **Depends on:** 5, 10 · **Parallel with:** none · **Human gate:** none

Files: `areas/deposit/vre-bridge/inputs/0803/{config.kdl,curation.csv,narrative/…}` (copies of the `media`
fixture's generation), `areas/deposit/vre-bridge/inputs/PROVENANCE`, `.gitattributes`
(`areas/deposit/vre-bridge/inputs/** -text`, so the bytes and the digest do not depend on a checkout's line
endings), `.github/scripts/check-stand-in-fixture.sh` (+ `.test.sh`), `justfile` (`check` dependency and
recipe with its comment line), `areas/access/media/data/PROVENANCE`, `areas/access/media/data/0803/.ready` and
`areas/access/sync/data/0803/.ready` (real digest in both), `areas/access/sync/store/tests/committed_0803.rs`
(if its equal-provenance test names the digest), `areas/deposit/vre-bridge/server/{BUILD.bazel,tests/refresh_0803.rs}`,
ADR-0002 / ADR-0008 `Not yet in code` lines, `ARCH-MAP.md`.

### dsp-repository

- [x] Test (`check-stand-in-fixture.test.sh`): the script passes on identical trees, fails naming the file on one
  differing byte, an extra file, a missing file.
- [x] `check-stand-in-fixture.sh`: compares `areas/deposit/vre-bridge/inputs/0803/` with the generation
  `areas/access/media/data/0803/.ready` names, byte for byte, both ways; wired into `just check` (a recipe with a
  comment line and a `check` dependency; `just test-scripts` finds the `.test.sh` by itself). It is the only
  thing naming both trees (ADR-0002's second path); no Bazel grant crosses the areas.
- [x] Copy the fixture files into `inputs/0803/`; `inputs/PROVENANCE` says they are the authoritative copy, edited
  here, and the fixture follows by the check.
- [x] Compute 0803's `inputs_digest` with `vre_bridge_core::inputs_digest` (a unit test printing it, then
  pinning it) and write it into both fixture tokens, so their provenance stays equal (Phase 5's test);
  `media/data/PROVENANCE` updated.
- [x] Test (`areas/deposit/vre-bridge/server/tests/refresh_0803.rs`, a new file, its `data` the `inputs`
  filegroup through a `copy_to_directory`, as the inputs digest refuses symlinks): a forced refresh of 0803
  against wiremock with a synthetic dump writes a `media` generation byte-identical to `inputs/0803/` plus
  `access-rights.json` (REQ-4.1, 2.6) and a token whose digest is the computed one.
- [x] ADR lines for the pin script deleted; `ARCH-MAP.md` media durable state.
- [x] Verify: `bash .github/scripts/check-stand-in-fixture.test.sh` and `bazel test
  //areas/deposit/vre-bridge/... //areas/access/sync/...` pass; `just check` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-4.1 for 0803, REQ-2.6 (shipped inputs), the PRD's "committed copies become `testonly`
fixtures … pinned byte for byte".

#### Phase 13: Increment 2, bridge side: DaTEI file mapping, text downloads, file manifest, IIIF entries

PR 15 (stacked before Phase 12) · DEV-8000 · **Depends on:** 11 · **Parallel with:** 6b (if still running) ·
**Human gate:** none

Contract (DaTEI's, `docs/specs/2026-11-19-datei-integration/01-feat-datei-integration-plan.md`, H1 accepted):

- Mapping `inputs/<SC>/file-mapping.json`: `{"format":"file-mapping","version":1,"files":[{"key":…,"iri":…}]}`.
- Manifest `file-manifest.json` written into the `media` generation: `{"format":"file-manifest","version":1,
  "files":[{"key":…,"iri":…,"stored":…} | {"key":…,"iri":…,"iiif":…}]}`, entries in the mapping's order.
- Resolution in the selection: the mapped resource's current file value. `knora-base:TextFileValue` → download
  `<IIIF base>/<SC>/<internalFilename>/file` without credentials, store under `internalFilename` (REQ-4.3);
  `knora-base:StillImageFileValue` → `iiif` = `<IIIF base>/<SC>/<internalFilename>` (REQ-4.4); any other file
  value class, a missing IRI, or no file value refuses (REQ-4.6). A stored name failing `is_file_name` or equal
  to a shipped input's name, `file-manifest.json`, `file-layout.xml` or `access-rights.json` refuses (REQ-4.7).
  Two keys with the same IRI share one download. Every download happens before `begin` (step 8 precedes step 9),
  so no `GenerationWriter` waits on the network.

Files: `areas/deposit/vre-bridge/core/src/datei.rs` (+ tests), `areas/deposit/vre-bridge/lift` (a read-only
query `file_value(&Dump, &Selection, iri) -> Option<FileValue { class, internal_filename }>`),
`areas/deposit/vre-bridge/server/src/refresh.rs`, `areas/deposit/vre-bridge/core/testdata/datei/**` (a
synthetic project `0001`), `vre-bridge-core` settings validation (the start-time `IIIF_BASES` rule below; the
variable itself is parsed since Phase 9a and read since Phase 10),
`docs/src/vre-bridge/operations.md`, `areas/deposit/vre-bridge/CONTEXT.md`, `areas/access/datei/CONTEXT.md` if
a term's wording changes.

### dsp-repository

- [x] Tests (`core`): mapping parse refuses another format or version and duplicate keys; manifest bytes for a
  fixed input equal a literal; stored names `file-layout.xml`, `file-manifest.json`, `access-rights.json`,
  one equal to a shipped input's name, `.hidden` and `a/../b` each refuse (REQ-4.7).
- [x] Tests (`server`, success; wiremock and a synthetic dump `0001` with a TEI `TextFileValue`, an ODD, a CSS
  and a `StillImageFileValue`): a forced refresh writes the three texts under their internal names,
  `file-manifest.json` with three `stored` and one `iiif` entry, the shipped `datei.toml`, `file-layout.xml` and
  `file-mapping.json`, and `access-rights.json` (REQ-4.1 to 4.5, 2.6); the downloads carry no `Authorization`
  header (REQ-2.7).
- [x] Tests (`server`, refusals): a `403` on one text refuses the project, tokens untouched (REQ-2.8, 3.5); a
  mapped IRI absent from the selection (a deleted resource) refuses (REQ-4.6); a mapped resource with a
  `DocumentFileValue` refuses (REQ-4.6); a text file value granting the anonymous user `RV` is refused by the
  guard before any download (REQ-2.4).
- [x] Implement as designed; `IIIF_BASES` required for a server the allowlist uses only when a project of that
  server ships a mapping (configuration error otherwise, at start).
- [x] Docs as listed.
- [x] Verify: `bazel test //areas/deposit/vre-bridge/... //areas/deposit/server/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: REQ-4.1, REQ-4.2, REQ-4.3, REQ-4.4, REQ-4.5, REQ-4.6, REQ-4.7, REQ-2.7, REQ-2.8, REQ-2.6 (the manifest
is covered by `access-rights.json`).

#### Phase 12: `access-server`'s image carries no stand-in data

PR 16 (last in the stack, on Phase 13's PR) · DEV-8001 · **Depends on:** 6b, 11 · **Parallel with:** none ·
**Human gate:** H5, H6, H7, H8 before merge (the PR is built and reviewed earlier; it merges only when they are
done)

Files: `areas/access/server/BUILD.bazel` (`image_data` without `sync/data` and `media/data`; `env` without
`SYNC_DATA_DIR`, `MEDIA_DATA_DIR`), `areas/access/{sync,media}/BUILD.bazel` (`data` filegroups
`testonly = True`; visibility unchanged, since `access-server`'s own `test_files` still needs the grant),
`docs/src/dpe/operations.md` (env table: both variables required in deployment, pointing at the mounts),
`areas/access/server/CLAUDE.md`, ADR-0002 / ADR-0003 / ADR-0007 / ADR-0008 / ADR-0012 `Not yet in code` lines,
`ARCH-MAP.md`.

### dsp-repository

- [x] `testonly = True` on both `data` filegroups is the guard: a non-`testonly` target such as `image_data`
  can no longer depend on them, so the build fails if one does. Every `copy_to_directory` that wraps them (Phase
  5's test wrappers, `test_files`) is `testonly = True` too: list them with
  `bazel query 'rdeps(//areas/..., //areas/access/sync:data + //areas/access/media:data, 1)' --output=label_kind`.
  Verify with `bazel build --platforms=//platforms:linux_x86_64_musl //areas/access/...` (the layers are
  musl-only) and confirm `CPE_PROJECTS_DIR`'s layer is unchanged. No tar test (see Phase 5).
- [x] `BUILD.bazel` and filegroups as listed; `access-server`'s, `cpe-server`'s, `store-build`'s and
  `sync-store`'s tests keep reading the fixtures through `testonly` targets.
- [ ] Docs and ADR lines (`the stand-ins are carried in access-server's image`, `sync-store and media-store read
  files committed to this repository`, ADR-0008's line on tests reading the data `access-server` serves)
  deleted; ADR-0012's and `areas/deposit/ADR-0005`'s DEV-7514 lines deleted once H5 is done.
- [x] Verify: `bazel test //areas/access/...` passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: PRD *Success criteria* "no committed snapshot is a deployment input"; increment 1's release order.

#### Phase O: ops-deploy, DEV-7514 extended

Separate draft PR in ops-deploy (not part of the `gh stack`), assigned to Balduin · DEV-7514 ·
**Depends on:** none (variable names from this plan) · **Parallel with:** 1, 3 (and everything after) ·
**Human gate:** H1 and H2 before merge.

Work on a branch from `origin/main` (the local checkout is stale: `git fetch` first). ops-deploy's
convention: `type(scope): subject (TICKET)`, no PR template; run `just lint` for touched roles.

### ops-deploy

- [x] `deposit.yml`: two `deposit_mounts` entries, `type: dir`, uid/gid 65532, mode `0755` (the reader on
  another host must read them), sources `{{ VRE_BRIDGE_SYNC_HOST_DIR }}` → `/var/lib/deposit/vre-bridge/sync`
  and `{{ VRE_BRIDGE_MEDIA_HOST_DIR }}` → `/var/lib/deposit/vre-bridge/media`, both only when both variables are
  defined; a pre-task asserts each source is a mount point (`mountpoint -q`), because `dasch.docker_prepare_mounts`
  creates a missing `dir` source, which would put the stand-ins silently on the root disk; a third mount,
  always, for the work directory: source `/data/vre-bridge-work/{{ inventory_hostname }}` (local disk) →
  `/var/lib/deposit/vre-bridge/work`, mode `0700`.
- [x] `deposit.yml` `deposit_env`: `EDITOR_VRE_BRIDGE_SYNC_DIR`, `EDITOR_VRE_BRIDGE_MEDIA_DIR` (only with the
  mounts), `EDITOR_VRE_BRIDGE_LOCK_FILE=/var/lib/deposit/db/vre-bridge.lock` (the existing local db mount),
  `EDITOR_VRE_BRIDGE_WORK_DIR=/var/lib/deposit/vre-bridge/work`, `EDITOR_VRE_BRIDGE_SERVICE_NAME`,
  `EDITOR_VRE_BRIDGE_NIGHTLY_AT`, `EDITOR_VRE_BRIDGE_ALLOWLIST`, `EDITOR_VRE_BRIDGE_SERVERS`,
  `EDITOR_VRE_BRIDGE_KEY_IDS`, `EDITOR_VRE_BRIDGE_SIGNING_KEYS`, `EDITOR_VRE_BRIDGE_IIIF_BASES`, from per-host
  variables.
- [x] `host_vars/deposit-{stage,prod}-01.yml`: `VRE_BRIDGE_SERVICE_NAME` (`dsp-repository-stage`,
  `dsp-repository-prod`), `VRE_BRIDGE_NIGHTLY_AT` (prod `02:00`, stage `03:30`, so the two bridges, both
  dumping from prod, do not meet in the slot), `VRE_BRIDGE_ALLOWLIST` (prod: `0803=prod`; stage: `0803=prod`,
  Keller's test server added with increment 2), `VRE_BRIDGE_SERVERS`, `VRE_BRIDGE_KEY_IDS` (kids unique per
  DSP server, e.g. `dsp-repository-prod-2026-11`), `VRE_BRIDGE_IIIF_BASES`; `VRE_BRIDGE_SIGNING_KEYS` as an
  inline `!vault |` value left as a commented placeholder for H2 (no secret in the PR).
- [x] The work directory's size (the largest dump, zipped and unpacked, about three times the dump) documented
  beside the variable for infrastructure.
- [x] Per-environment variables for infrastructure, declared and documented with no value: in
  `host_vars/deposit-{stage,prod}-01.yml` (`group_vars/deposit.yml` is shared by stage and prod)
  `VRE_BRIDGE_SYNC_HOST_DIR`, `VRE_BRIDGE_MEDIA_HOST_DIR`; in `group_vars/repository_{dev,stage,prod}.yml` the
  repository side's two; `VRE_BRIDGE_STAND_IN_SIZE` (documentation for the dataset's size, provisioned outside
  ops-deploy).
- [x] `repository.yml`: `deploy_mounts` for the two locations read-only at `/app/stand-in/sync` and
  `/app/stand-in/media`, and `SYNC_DATA_DIR`, `MEDIA_DATA_DIR` pointing there, all four only when both host
  variables are defined (one without the other would mix the bridge's data with the image's fixture and fail
  0803 on the provenance comparison; until then the image's fixtures serve 0803); `SYNC_POLL_SECS` left at its
  default.
- [x] `ACCESS_HIDDEN_PROJECTS`: unchanged (`0803,083C,086A` in every environment by `repository.yml`'s default).
- [x] dsp-api: `roles/dsp-deploy/templates/docker-compose-svc.yml.j2` gains `-
  'KNORA_WEBAPI_SERVICE_IDENTITY_TRUSTED_KEYS={{ DSP_API_SERVICE_IDENTITY_TRUSTED_KEYS | to_json }}'`;
  `roles/dsp-deploy/defaults/main.yml` `DSP_API_SERVICE_IDENTITY_TRUSTED_KEYS: []` (off);
  `host_vars/vre-{stage,prod}-01.yml` hold one entry per deployment whose allowlist names that server, e.g. on
  `vre-prod-01` both `{name: dsp-repository-prod, kid: …, publicKey: …}` and
  `{name: dsp-repository-stage, kid: …, publicKey: …}` with distinct kids (dsp-api refuses to start on a
  duplicate), as commented placeholders for H2 (public keys, not secret; per host, so `dsp-demo-01` and the RDU
  hosts get nothing).
- [ ] Verify: `just lint dsp-deploy` (and `just lint ALL` if quick) passes; `ansible-playbook --syntax-check`
  needs the vault password and an inventory, so it is left to H10's reviewer if it cannot run.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

Acceptance: the PRD's ops-deploy impact; REQ-1.17 holds in every environment until the host variables are
filled.

## Execution record

The stack is 18 PRs. DEV-8012, `feat(vre-bridge-lift): read text markup from valueHasXml and ignore standoff`,
was added on 2026-11-20 after the lift refused a real stage dump of 0803; it is stacked between Phase 13 and
Phase 12, so Phase 12's stack parent is DEV-8012's PR.

Where the code differs from this plan's text:

- `vre-bridge-core`'s `serde` and `serde_json` arrived in Phase 13; `vre-bridge-store` has no `serde_json`.
- The poll compares parsed Ready tokens, not their bytes.
- `ArchiveProjection::location_present` holds back a re-activation when a location is lost (P5).
- `oxigraph` is pinned `=0.5.11`, with `md-5` held at 0.10.6.
- The lift's API is `lift() -> Lifted { snapshot, stats, files }`; there is no `snapshot()` and no
  `Snapshot::stats()`.
- `Closure::reaches` returns `false` behind a `debug_assert`.
- `TRANSFORMATION_VERSION` is 2.
- `LiftError` has the variants `Dump`, `Refused`, `ClosedWorld` and `Internal`.
- The selection's signatures are `select(&Index, &Subjects)` and `check(&Index, &Subjects, &Selection)`;
  `NotViewable` is a unit variant.
- The client's calls are `export_status(iri, id)` and `download(iri, id, &TempDir) -> DumpZip`; `Timeouts` has
  `connect` and `read`; the client crate is edition 2021.
- `Settings::parse` returns `Result<Option<_>, _>`; numbers are read as text; one output location set without
  the other is an error.
- Three size caps exist: `EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES` (256 MiB), `EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES`
  (512 MiB) and `EDITOR_VRE_BRIDGE_MAX_FILE_BYTES` (512 MiB); *Configuration*'s table does not list them.
- `refresh` takes `&AllowlistEntry`.
- A refresh has failure paths the plan does not list: the size caps, retries of the export-status poll, and
  `Failed(Inputs)`.
- `from_raw(raw, &Meter)`; `async fn operator_refresh(&self, &str)`.
- The scheduler's tests use a gated fake clock, not `start_paused`.
- A removal reports `server: None`.
- The panic hook withholds the payload of a panic in the bridge's work.
- The shipped inputs' provenance is `areas/deposit/vre-bridge/inputs/PROVENANCE`; the 0803 test is
  `refresh_0803.rs`.
- Phase 12 also deleted ADR-0007's line; PR previews serve no CPE project.
- Phase O's work mount and trusted-keys line are conditional, and only `vre-prod-01` is touched.
- Phase 1 removed 3 tests and added 4.

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Infrastructure answers where the stand-in locations live (same host or NFS), creates them with sizes, makes them writable by uid 65532 on the deposit host and readable by `access-server`'s uid on the repository host (a `chown` of an NFS root can fail under `root_squash`), mounts them before the playbook runs, provides the local work directory, keeps one writer per location (stage and prod never share one), and fills the `VRE_BRIDGE_*_HOST_DIR` variables per environment in ops-deploy | Infrastructure | before Phase O merges (Why mid-plan: the answer is pending and does not block the code) | infrastructure's decision and provisioning |
| H2 | Generate one Ed25519 key pair per (deployment, DSP server) pair the allowlists use (today: stage→prod, prod→prod); vault each private key (PKCS#8 DER base64) into `host_vars/deposit-<env>-01.yml`; put each public key (SPKI DER base64), its kid (unique on that server) and the deployment's service name into the server's `host_vars/vre-<env>-01.yml` | Balduin / ops | before Phase O merges (Why mid-plan: keys are minted when the ops PR is ready) | secret material |
| H3 | Get dsp-api PR #4448 reviewed, merged and released, and the release deployed on every server the allowlist names; re-check the merged contract (that "own export" is scoped by the service name; `DELETE` of a running export answers `409 export_in_progress`) and amend Phases 8 and 9b if it changed. The `cutoff` of `/v2/resources/info` is in dsp-api since v40.6.0 and accepts a future instant (read in the source, 2026-11-20) | Balduin / dsp-api team | before the bridge runs (after Phase 10 ships) | another team's review and release |
| H4 | Deploy `deposit-server` with Phase 11 (the shipped inputs) and the ops PR, run `vre-bridge-refresh 0803` on stage, then let one nightly run pass; check logs and metrics | Balduin | after ship (Phase 11) | deployment and judgement |
| H5 | Confirm the bridge has filled the locations with 0803 in stage and prod (tokens present, `sync` and `media` provenance equal) and switch `access-server` to the mounts (fill the repository host variables, both at once). Hazard: once the image carries no data (Phase 12), an environment whose location variables are empty serves no CPE project; the deploy and the health check still pass. | Balduin / ops | before Phase 12 merges (Why mid-plan: needs the deployed bridge of Phases 10 and 11) | deployment |
| H6 | Run the REQ-6.4 parity test on a real 0803 prod dump: `just vre-bridge-lift-real-dump <dump> <reference>` with the reference made by the incubator's `dao-lift` at `6106ee38`; resolve every difference other than the rights lines; if the guard refuses 0803, decide what to do with the listed items before going on. Run against a stage dump of 0803, not a prod dump | Balduin | before Phase 12 merges (recommended right after Phase 2) | needs a SystemAdmin dump and judgement |
| H7 | Increment 1's acceptance: list and review every difference between the facts the bridge wrote for 0803 from prod and the committed snapshot | Balduin | before Phase 12 merges | review of data |
| H8 | Decide dev: accept that dev's `access-server` serves no CPE data after Phase 12 until the follow-up "Data for dev's access-server" (DEV-7992), or provide dev's locations first | Balduin | before Phase 12 merges | owner decision (P9) |
| H9 | Create the Linear tickets (the stack's and the PRD's follow-ups), then replace the placeholders in commit subjects and `Not yet in code` lines. Done 2026-11-20: the stack's tickets are in the table above, the follow-ups are listed in the PRD's *Follow-up tickets* | Balduin | before start | ticket creation is the owner's |
| H10 | Merge the stack in order with `gh stack`, then the ops-deploy PR when H1 and H2 are done | Balduin | after ship | merging is the owner's decision |
| H11 | On every allowlisted DSP server the `valueHasXml` backfill has run (`POST /admin/maintenance/backfill-value-has-xml`), before the bridge operates; the lift refuses a project with formatted text lacking it (DEV-8012) | Balduin / dsp-api team | before the bridge runs | an administrative call on another team's servers |

## Acceptance Criteria

- [ ] Every REQ of the PRD is covered by the phase named in *Coverage* below and pinned by a test there, except
  REQ-6.4's real-dump run (H6) and the image's env (REQ-1.17 in the image: a BUILD comment and review, Phase 10).
- [ ] `just check` and `just test` pass on every PR of the stack.
- [ ] Each PR is one commit, conventional, with its ticket.
- [ ] No crate of the Access Area depends on a `vre-bridge-*` crate and no `vre-bridge-*` crate on an Access crate
  (Bazel `visibility`).
- [ ] With no bridge variables set, `deposit-server` touches no stand-in path (REQ-1.17), in tests and in the image.
- [ ] 0803 is served throughout: by the image fixtures until H5, by the mounts after.

Coverage:

| REQ | Phase(s) |
|---|---|
| 1.1, 1.3, 1.4, 1.11, 1.12 | 9c (1.3, 1.11 with 7) |
| 1.2, 1.6, 1.10, 1.19, 1.20 | 9b (1.19, 1.20 with 8) |
| 1.5 | 9a (cutoff), 9b (the call) |
| 1.7, 1.8, 1.18 | 9a |
| 1.9 | 9a (decision), 9c (operator function), 10 (subcommand) |
| 1.13 | 9b (one refresh), 9c (retries, operator) |
| 1.14, 1.15 | 7, 9c |
| 1.16 | 8 |
| 1.17 | 9c, 10, O |
| 2.1–2.6, 2.9, 2.10 | 2 (2.6 for `media` files in 7, 11, 13; 2.10 also 8) |
| 2.7, 2.8 | 8, 13 |
| 2.11 | 7 (sweep), 8, 9b, 9c |
| 3.1–3.5, 3.7–3.10, 3.12, 3.13 | 7 (3.5, 3.7 also 9b; 3.7, 3.8 also 9c) |
| 3.14 | 7 (writer), 5 and 6a (readers) |
| 3.6 | 9c |
| 3.11 | 4, 7 |
| 4.1 | 11, 13 |
| 4.2–4.7 | 13 |
| 5.1 | 9b (content), 10 |
| 5.2, 5.3 | 9b (values), 10 |
| 6.1–6.3, 6.14 | 5 |
| 6.4 | 1, 2, H6 |
| 6.5 | 6a |
| 6.6–6.10, 6.12 | 6b |
| 6.13 | 6a (ports), 6b (CPE) |
| 6.11 | 2, 7, 8, 9b |

## Owner decisions (provisional)

Each was taken by this plan without the owner; each can be vetoed before execution.

- **P1. Crate layout and two new roles.** Options: one crate; lift + rest; five crates. Chosen: five
  (`lift`, `core`, `store`, `client`, `server`), adding roles `lift` and `client` in ADR-0005. Reason:
  `visibility` and the role bans hold the single-writer and no-network-in-the-lift rules; the cost is more
  BUILD files.
- **P2. Ready token and generation format.** JSON as specified, token at `<SC>/.ready`, generation
  `YYYYMMDDTHHMMSSZ-<8 hex>`, `previous_generation` in the token so cleanup and recovery need no other state.
  Alternative: a token per generation plus a pointer file (more files, no gain).
- **P3. Configuration encoding.** Flat `EDITOR_VRE_BRIDGE_*` variables with `key=value,…` lists; signing keys
  as one `Secret` list; service name default `dsp-repository`; timeouts as tabled. Alternative: figment nested
  keys (`EDITOR_VRE_BRIDGE__SERVERS__PROD__URL`), which needs `Env::split` and changes the existing loader.
- **P4. Lock file.** A required setting (no default); the nightly-run record at `<LOCK_FILE>.nightly`; ops
  puts it in the existing local db mount. Alternative: default under `EDITOR_DB_DIR` (couples the bridge to
  the editor's database setting).
- **P5. CPE's keep-previous rule.** The re-activation is accepted only if every project served before is
  served again or gone because its token was removed. Alternative: accept per project (needs the per-project
  rebuild, a follow-up). Trade-off: one broken project holds back every other project's refresh. A project
  whose `sync` snapshot is `UnknownProject` counts as gone before provenance is compared (review, run 1:
  otherwise a removal, which removes the `sync` token first, would hold every re-activation back). A mount
  that loses its target also looks like "every token removed" and would be accepted; holding back when the
  project directory itself is gone was considered and left to the owner.
- **P6. Ports.** Provenance as a std-only struct mirroring the token; `subscribe` with a `ChangeListener`
  callback; default trait methods so fakes and the incubator's vendored adapters compile; the poll on a
  `std::thread` inside `sync-store`, `SYNC_POLL_SECS` default 30. Alternative: an opaque provenance string
  (less readable logs).
- **P7. The 0803 fixtures.** `snapshot.nq` keeps its bytes (no rights or classification lines, which format
  version 1 has; `sync` omits them anyway); the tokens are hand-written, the digest a placeholder until Phase
  11 computes it. Alternative: regenerate the fixture, impossible without a real dump.
- **P8. Stack order.** The image drop (Phase 12) is the last PR, after increment 2's bridge side, because it
  waits on operations in every environment and Keller is due mid-December. Phases 6 and 9 are split into 6a/6b
  and 9a/9b/9c (review, run 1: each was too large for one executor run; 9a's pure logic can start in group 3),
  giving 16 PRs; the new tickets are DEV-7988 and DEV-7993, and 6a and 6b share DEV-7622.
- **P9. Dev.** Dev has no `deposit-server`; after Phase 12 dev's `access-server` serves no CPE data until the
  follow-up DEV-7992 (0803 is hidden there anyway). Alternative: keep image fixtures for dev only (two image variants).
  H8 confirms.
- **P10. Operator's refresh.** Subcommand `vre-bridge-refresh <SHORTCODE>`, run with `docker exec`; exit
  codes `0/2/1/75/64`; it does not wait for a busy lock.
- **P11. Format terms.** `dao:accessRights` (`"public"`, `"restricted-view"`) and `dao:dataClassification`
  (`"unclassified"`) as plain literals on resources and representation nodes, deviations D15 and D16.
  Alternative: IRIs for the values (closer to a future DAO term, more vocabulary now).
- **P12. Dump unpacking.** Only `data.nq` and `ontology-<n>.nq` are extracted; `admin.nq` and
  `permission.nq` never reach the disk.
- **P13. The port.** Crate `vre-bridge-lift` at edition 2024; panics turned into errors; `eprintln!` replaced
  by returned statistics; test names kept.
- **P14. Transformation version.** A hand-bumped integer, guarded by a golden test on a synthetic dump.
  Alternative: the build's git hash (refreshes every project on every release).
- **P15. Unchanged project.** Tokens are not rewritten, so T stays; the next change check repeats from the old
  T (cost: a larger listing, no paging; acceptable for the first projects).
- **P16. Removal.** A shortcode directory removed from the allowlist loses its tokens in one run and its data
  in the next (a directory is deleted only if it had no token when the run began), so in-flight reads finish.
- **P17. ops-deploy.** Mounts and env conditional on host variables, so the PR can merge before
  infrastructure answers; the signing keys as a vaulted env value, which differs from ops-deploy's practice of
  mounting the DKIM key as a file (the owner chose env).
- **P18. Own export on `409`.** Waiting for and deleting the bridge's own leftover export spends no retry,
  once per refresh, bounded by `EXPORT_TIMEOUT_SECS`.
- **P19. Project IRI.** Resolved per refresh with the public `GET /admin/projects/shortcode/{SC}`; the
  allowlist names shortcodes only.
- **P20. Rights of `media`'s files (REQ-2.6).** One `access-rights.json` per `media` generation with a
  default for every file. Alternatives: rights per manifest entry (changes the manifest contract DaTEI
  accepted, and covers DaTEI projects only); fields in the `media` token (mixes provenance and policy).
- **P21. Allowlist typos.** A removal is logged at warning level and counted as outcome `removed`; there is no
  safeguard beyond the empty-list rule. Alternative: refuse removals whose token's server is still configured
  (blocks deliberate removals too).
- **P22. One service identity per deployment.** `dsp-repository-stage` and `dsp-repository-prod`, each with
  its own key, both trusted by `vre-prod-01`; stage's nightly run at 03:30. Reason: dsp-api treats an export
  as one's own by the service, so two deployments sharing a name would recover, and delete, each other's
  exports.
- **P23. Change check without listing.** A forced refresh, or one without T, asks `/v2/resources/info` with a
  `cutoff` one day ahead only to get `asOf`, since the endpoint has no paging. dsp-api parses `cutoff` as an
  `Instant` without an upper bound (read in the source, released in v40.6.0), so a future cutoff lists nothing.
  Alternative: `cutoff` = the bridge's clock (a short listing). REQ-1.5 is amended to match (it said "without a
  cutoff", which lists the whole project).
- **P24. Held-back CPE activation.** Logged at error level and retried once after ten poll intervals; a
  restart beyond that. Alternative: retry on every poll round (load on a broken project every 30 s).
- **P25. Token repair.** A malformed token, or one naming a missing generation, forces a refresh like a missing
  one; `sync` and `media` answer `Unavailable` for it meanwhile, never "unknown".
- **P26. Leftover dump directories.** The private directories are named `vre-bridge-dump-*`, and every one in
  `WORK_DIR` is deleted at start and at each nightly run, under the lock, because a killed process never runs
  `TempDir`'s drop and the zip holds `admin.nq`. Alternative: rely on the drop alone (leaves password hashes on
  disk after an OOM kill).
- **P27. The image check.** No test reads the image's tars or env (Bazel 9 has no native `sh_test`, the
  repository has no `rules_shell`, and the layers are musl-only, so such a test is skipped on a host
  platform). Phase 5 tests the `copy_to_directory` of the same data; Phase 12 relies on `testonly` to keep the
  data out of `image_data`; the image env is held by a BUILD comment and review. Alternative: add `rules_shell`
  and a musl-transitioned test (a new module dependency for two assertions).
- **P28. Rollback of a failed `sync` rename.** `commit` restores the `media` token it captured before the
  rename (or removes it if there was none) rather than copying the `sync` token, so a first generation and a
  diverged pair are restored exactly (REQ-3.5).
- **P29. Recovery with a token it cannot read.** A project whose token is unreadable or invalid (a newer
  `token_version` included) is skipped by recovery's deletion (REQ-3.8 applies to tokens the bridge can read),
  since deleting by the other token alone could remove the generation a reader serves; a valid token naming a
  missing generation still protects its previous one. Alternative: treat such a token as missing (simpler; risks
  `Unavailable` until the next refresh after a transient NFS error, and data loss after a rollback).

Owner-confirmed, not provisional: the guard refuses on any violation (decision 7); dropping instead is a
follow-up (DEV-7990).

## Dependencies & Risks

- **dsp-api #4448 is provisional** (open, unreviewed). The client is written against the branch
  `feature/dev-7936-export-service-identity` as read on 2026-11-20; H3 re-checks after merge.
- **0803 on prod may not pass the guard.** Nobody has checked prod's permissions; H6 shows it before the
  image drop, and the bridge refuses (keeps no data) rather than leaks.
- **Infrastructure's answer is pending.** Nothing in the code depends on it beyond REQ-3.14; H1 and the
  conditional ops variables absorb it.
- **NFS attribute caching** can delay a reader seeing a new token; harmless (previous generation kept).
- **The DaTEI plan's `media-store` adapter** must resolve through the token, find `datei.toml` in the
  generation and pin the generation per file source (*Technical Considerations*).
- **A CPE re-activation can overlap DaTEI's first build** (memory for two builds at once, minutes after a
  start); accepted until DEV-7938 (*Technical Considerations*).
- **Bazel runfiles are symlinks** and the readers refuse symlinks: fixture tests go through
  `copy_to_directory` (Phase 5).
- **oxrdf 0.3.3 → 0.3.4**: byte differences in literals would show in H6; the snapshot's facts, not bytes,
  are REQ-6.4's measure.
- **Two operator refreshes in quick succession** can delete a generation that a reader with a stale NFS
  attribute cache still reads (it then answers `Unavailable` until its next read); accepted and documented in
  Phase 10.
- **A mistyped allowlist** withdraws a live project on the next nightly run (P21).

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| A refresh leaks restricted data | L | H | guard fails closed (missing or malformed permissions refuse); only UnknownUser counts; RV only on raster images; tests with seeded restricted data |
| A reader serves a mixed project | L | H | generations + one rename; provenance compared before and after; keep-previous rule |
| The export slot stays blocked by the bridge | M | M | delete on every path once an id is known; on `409` recover our own export |
| A preview or local run writes somewhere | L | M | no output location by default; test |
| Secrets in logs | L | H | `Secret` and a redacting key type, hand-written `Debug`, `bearer_auth`, `without_url()`, `skip_all` spans, error types without bodies; a log-capture test (Phase 9b) |
| Dump data left on disk | L | H | `TempDir` on every outcome; `vre-bridge-dump-*` swept at start and nightly (P26) |
| The stack waits on one gated PR | M | M | the gated PR is last |

## Success Metrics

- Phase 9b's and 9c's tests and Phase 6b's REQ-6.12 test pass; H4's first nightly run reports `refreshed` for 0803 on
  stage and prod, then `unchanged` on the following nights while 0803 does not change.
- `vre_bridge.refresh.size` shows 0803's dump, snapshot and media sizes (baseline for growing the allowlist).
- H6: the parity test reports no difference other than the rights lines.

## References

- PRD: `docs/specs/2026-11-19-vre-bridge/01-vre-bridge-PRD.md`; ADRs `areas/deposit/ADR-0004`, `ADR-0005`,
  ADR-0002, ADR-0003, ADR-0007, ADR-0008, ADR-0009, ADR-0012, ADR-0013.
- Code read: `areas/access/sync/store/src/lib.rs` (`KNOWN`), `areas/access/media/store/src/lib.rs`,
  `areas/access/cpe/ports/src/{lib.rs,names.rs}`, `areas/access/cpe/server/src/{lib.rs,activation.rs}`,
  `areas/access/server/{BUILD.bazel,src/serve.rs}`,
  `areas/deposit/server/{BUILD.bazel,src/config.rs,src/observability.rs}`,
  `shared/telemetry/src/collector.rs`, `.github/scripts/check-capability-deps.sh`, `justfile` (`check`, `_rustfmt`).
- dsp-api (PR #4448 branch): `docs/03-endpoints/api-v3/project-migration.md`,
  `docs/04-publishing-deployment/configuration.md#service-identity`, `ServiceTokenVerifier.scala`,
  `V3ProjectsEndpoints.scala`; `docs/03-endpoints/api-v2/reading-and-searching-resources.md` (polling);
  `PermissionUtilADM.scala`; `AssetAccess.scala` (DEV-7244).
- dsp-incubator `cpe/tools/dao-lift` at `6106ee38`; the CPE engine's port for the technique:
  `docs/specs/2026-10-11-move-cpe-core/`, `docs/specs/2026-10-11-move-cpe-store-web-server/`.
- ops-deploy `origin/main` (`e3daa83e`): `deposit.yml`, `repository.yml`, `roles/dasch.docker_prepare_mounts`,
  `roles/dsp-deploy/templates/docker-compose-svc.yml.j2`, `group_vars/dsp_{stage,prod}.yml`.
- Learnings: `docs/learnings/build-errors/missing-rustls-provider-install-panics-reqwest-at-client-construction.md`,
  `docs/learnings/build-errors/crates-hub-aliases-are-manual-wildcard-builds-nothing.md`,
  `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`,
  `docs/learnings/build-errors/remote-download-minimal-leaves-toplevel-outputs-remote-in-ci.md`.
