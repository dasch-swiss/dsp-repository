# VRE Bridge Operations Guide

The VRE bridge runs inside `deposit-server` (`areas/deposit/ADR-0005`). For each project on its allowlist it takes
the project's dump from its DSP server, turns it into an interim snapshot, and writes it, with the project's media
files, into the `sync` and `media` locations that `access-server` reads. It has no routes. Its vocabulary is in
`areas/deposit/vre-bridge/CONTEXT.md`; its design is in `areas/deposit/vre-bridge/CLAUDE.md`.

## When it runs

- **Inactive** with no output location set (`EDITOR_VRE_BRIDGE_SYNC_DIR` and `EDITOR_VRE_BRIDGE_MEDIA_DIR` both
  unset): it touches no file and makes no request. The image sets neither, so a Cloud Run preview, which runs the
  image publicly, stays inert. Only a deployment sets them.
- **Inactive** with an invalid configuration: logged once at start at error level (`the VRE bridge is inactive:
  its configuration is invalid`), with a reason naming the variable and never a key. `deposit-server` starts and
  serves regardless.
- **Active** otherwise: at start it takes the lock if no one holds it, deletes leftover dump directories in the
  work directory and recovers every allowlisted project (see [Recovery](#recovery)), then schedules the nightly run.

The **nightly run** starts at `EDITOR_VRE_BRIDGE_NIGHTLY_AT` in `EDITOR_VRE_BRIDGE_TIME_ZONE`. It waits for the
lock, skips the night if it already ran (the date is recorded in `<LOCK_FILE>.nightly`, written first), deletes
leftover dump directories, recovers, withdraws projects no longer on the allowlist, then refreshes each
allowlisted project. A project whose server's dump slot is busy is retried `EDITOR_VRE_BRIDGE_DUMP_RETRIES` times,
`EDITOR_VRE_BRIDGE_DUMP_RETRY_SECS` apart, then fails. A missed run is not caught up, and a run cut short by a
restart is not resumed that night: the [operator's refresh](#the-operators-refresh) is the lever.

A withdrawal removes the project's Ready tokens in one run and its data in a later one. An empty allowlist
withdraws nothing, so that a lost setting removes no project: to withdraw the last project, delete its
`.ready` file in both locations by hand, `sync` first.

## Settings

Every setting is an environment variable of `deposit-server` (or a key of the optional `editor.toml`, in lower
case without the `EDITOR_` prefix). Each is read as text and checked by the bridge: a value it refuses leaves the
bridge inactive with the variable named, and never stops `deposit-server`. A blank value counts as unset. Lists are
`key=value` entries separated by `,`; entries are trimmed, empty ones dropped, and each splits on its first `=`.

| Variable | Default | Meaning |
|---|---|---|
| `EDITOR_VRE_BRIDGE_SYNC_DIR` | unset | `sync` location. Unset together with `MEDIA_DIR`: the bridge is inactive. One without the other is an error. |
| `EDITOR_VRE_BRIDGE_MEDIA_DIR` | unset | `media` location. |
| `EDITOR_VRE_BRIDGE_LOCK_FILE` | unset, required | Lock file on a local disk, never inside a location; the nightly-run record is `<LOCK_FILE>.nightly`. |
| `EDITOR_VRE_BRIDGE_ALLOWLIST` | empty | `SC=server,…`, e.g. `0803=prod`. Shortcodes are four upper-case hexadecimal digits. |
| `EDITOR_VRE_BRIDGE_SERVERS` | empty | `id=base URL,…`, e.g. `prod=https://api.dasch.swiss`; no trailing `/`. The URL, verbatim, is the token's `aud`. `https` only: `http` is accepted for a loopback host alone (a test server), since the service token would otherwise travel in cleartext. |
| `EDITOR_VRE_BRIDGE_IIIF_BASES` | empty | `id=IIIF base URL,…`, e.g. `prod=https://iiif.dasch.swiss`, for DaTEI projects' files; `https`, as for `SERVERS`. Required for a server from which an allowlisted project ships `file-mapping.json`; without it the bridge is inactive. |
| `EDITOR_VRE_BRIDGE_KEY_IDS` | empty | `id=kid,…`: the key id dsp-api knows the deployment's public key by. |
| `EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES` | empty | `id=<absolute path>,…`: per server, a file holding one Ed25519 private key as PEM PKCS#8 ([Keys](#keys)), read at start and never inside a location; a symlink is followed. The value is not logged: a malformed entry is named by its position, a file that cannot be used by its server and path, never by its content. |
| `EDITOR_VRE_BRIDGE_SERVICE_NAME` | `dsp-repository` | The token's `iss` and `sub`, and the `name` of the key in dsp-api's trusted keys. One per deployment. |
| `EDITOR_VRE_BRIDGE_INPUTS_DIR` | `areas/deposit/vre-bridge/inputs` (image: `/app/vre-bridge/inputs`) | Shipped inputs, one directory per shortcode. An allowlisted project without its directory is refused (exit `2`, reason `inputs-missing`) before anything is asked of its server, and logs `the project has no shipped-inputs directory` with the `path`; an empty directory is accepted. Looked for on every refresh, so no restart is needed once a release ships it. |
| `EDITOR_VRE_BRIDGE_WORK_DIR` | the OS temporary directory | Parent of the private dump directories (`vre-bridge-dump-*`, mode `0700`); never inside a location, never a symlink. |
| `EDITOR_VRE_BRIDGE_NIGHTLY_AT` | `02:00` | `HH:MM`, local time of the nightly run. |
| `EDITOR_VRE_BRIDGE_TIME_ZONE` | `Europe/Zurich` | IANA time zone of `NIGHTLY_AT`. |
| `EDITOR_VRE_BRIDGE_DUMP_RETRIES` | `3` | Retries of a busy dump slot within a nightly run. |
| `EDITOR_VRE_BRIDGE_DUMP_RETRY_SECS` | `1200` | Wait before each retry. |
| `EDITOR_VRE_BRIDGE_REQUEST_TIMEOUT_SECS` | `120` | The project lookup, the change check, the export's trigger, status and delete. |
| `EDITOR_VRE_BRIDGE_EXPORT_TIMEOUT_SECS` | `3600` | From the export's trigger to its completion. |
| `EDITOR_VRE_BRIDGE_EXPORT_POLL_SECS` | `15` | Interval of the export's status poll. |
| `EDITOR_VRE_BRIDGE_DOWNLOAD_TIMEOUT_SECS` | `3600` | The dump's download. |
| `EDITOR_VRE_BRIDGE_FILE_TIMEOUT_SECS` | `300` | One anonymous file download. |
| `EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES` | `268435456` (256 MiB) | Largest dump zip downloaded. |
| `EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES` | `536870912` (512 MiB) | Most the extracted entries of a dump may unpack to. |
| `EDITOR_VRE_BRIDGE_MAX_FILE_BYTES` | `536870912` (512 MiB) | Largest single file downloaded. |

The time limits and the caps must be greater than 0.

**The caps bound memory as well as disk.** The lift reads a project's unpacked RDF into an in-memory store and
holds about 5 to 6 times the unpacked size in memory at its peak, inside `deposit-server`'s process, or inside the
operator's refresh, a second process in the same container whose memory adds to the server's. The defaults are
provisional. To raise them, set `EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES` (and `EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES`
with it) in the deployment's environment, and only together with the container's memory limit (6 times the
unpacked cap, plus what the server needs) and with room in `WORK_DIR` for the zip and its extracted entries at
once. A project over a cap fails with reason `dump`; `vre_bridge.refresh.size` with `kind=dump` shows how close
each project is.

## Layout

Per location (`sync`, `media`) and project:

```
<location>/<SHORTCODE>/.ready                    the Ready token, replaced by one rename
<location>/<SHORTCODE>/.ready.<generation>.tmp   a token being written; never read
<location>/<SHORTCODE>/<generation>/             a generation, written whole before a token names it
    sync:  snapshot.nq
    media: access-rights.json, the shipped inputs, (DaTEI projects) the downloaded files and file-manifest.json
```

`<generation>` is `YYYYMMDDTHHMMSSZ-xxxxxxxx`, the same in both locations. A refresh writes `media`'s generation,
then `sync`'s, then replaces `media`'s token and then `sync`'s, and keeps the new generation and the previous one.
Entries whose names are not shortcodes (`lost+found`, `PROVENANCE`) are never read or deleted. Directories are
`0755` and files `0644`, so `access-server`'s user can read them.

**DaTEI projects.** A project whose shipped inputs hold `file-mapping.json` (`{"format":"file-mapping",
"version":1,"files":[{"key":…,"iri":…}]}`, made with a DaTEI page) has its mapped files resolved after the lift:
each IRI must be a resource the snapshot holds, with one current file value. A text file is downloaded without
credentials from `<IIIF base>/<SHORTCODE>/<internal name>/file`, within `EDITOR_VRE_BRIDGE_MAX_FILE_BYTES` and
`EDITOR_VRE_BRIDGE_FILE_TIMEOUT_SECS`, and stored under its internal name; a still image is recorded by its IIIF
URL, `<IIIF base>/<SHORTCODE>/<internal name>`, and not downloaded. `file-manifest.json` lists each key with its
IRI and its stored name or IIIF URL, in the mapping's order. The bridge fetches only below the server's configured
base URL and IIIF base and follows no redirect. The project is refused (exit `2`, reason `file-mapping`) for a
mapped IRI that is deleted, not public, without a file value or holding another kind of file, or whose internal
name is not a file name or collides with a shipped input, `file-manifest.json`, `file-layout.xml` or
`access-rights.json`; an anonymous `401` or `403` on a download refuses it with reason `unauthorized`. A
mapping that does not parse fails it with reason `inputs`.

**One writer per location.** The lock is a local file lock and does not reach across hosts: two deployments, stage
and prod, must never share a location, a lock file or a work directory.

## The operator's refresh

A forced refresh of one allowlisted project, in the running container, so it has the deployment's settings:

```sh
docker exec <container> /app/deposit-server vre-bridge-refresh 0803
```

It takes the lock without waiting, refreshes the project even if nothing changed, writes the outcome line to
stdout as JSON, and exits with the outcome. Its log, the outcome line included, goes to the `docker exec` output
only, not to the container's log and so not to the log pipeline: keep it if it is needed later. A refresh can
take as long as the export and the download, up to an hour each by default: run it where a dropped session does
not lose its output (`tmux`, or `docker exec … > file`).

| Exit status | Meaning |
|---|---|
| `0` | Refreshed, or unchanged. |
| `2` | Refused: the project's data does not pass the public guard or the format's closed world, its file mapping does not resolve, or it has no shipped-inputs directory. Its tokens are unchanged. |
| `1` | Failed, with the reason in the outcome line; a busy dump slot fails at once. Its tokens are unchanged. |
| `75` | The lock is held by the nightly run or another refresh. Try again once it has ended. |
| `64` | The bridge is inactive, the project is not on the allowlist, or the configuration does not load. |

The nightly run holds the lock from its start to its end, through its retry waits, so `75` can last as long as
the run: with the defaults, up to three waits of 20 minutes plus the refreshes themselves, so about an hour or
more. The operator's refresh exports no metrics; its outcome line is its record.

## Recovery

At start and before each nightly run, under the lock, the bridge points a `media` token that disagrees with its
`sync` token back to `sync`'s generation, deletes `.ready.*.tmp` files and the generations no token names, and
deletes every `vre-bridge-dump-*` directory in the work directory, left there by a killed process. A project whose
token it cannot read is left as it is.

## Logs

One line per ended refresh, `refresh ended`, with `project`, `server`, `outcome` (`unchanged`, `refreshed`,
`refused`, `failed`), `reason` (for refused and failed), `duration_secs` and, when refreshed, `dump_bytes`,
`snapshot_bytes` and `media_bytes`: at info level, warning for refused, error for failed. A withdrawn project logs
`project withdrawn: it is no longer allowlisted` at warning level. A refusal by the public guard logs each
violation (an IRI, a class, a reason; never a value), the first 50, then the total. No line holds a value of the
dump, a key or a token, and a panic in the bridge's work is logged without its message.

## Metrics

Exported with `deposit-server`'s other metrics over OTLP (`OTEL_EXPORTER_OTLP_ENDPOINT`), meter scope `vre_bridge`:

| Instrument | Kind | Attributes | Meaning |
|---|---|---|---|
| `vre_bridge.refresh.outcomes` | counter | `project`, `server`, `outcome` | One per ended refresh, one per withdrawn project (`outcome=removed`, `server=(none)`), and one per failed removal (`outcome=failed`, `server=(none)`; `project=(none)` when the locations could not be listed). |
| `vre_bridge.refresh.duration` | histogram, `s` | `project`, `server`, `outcome` | How long a refresh took; a busy project's from its first try. Not recorded for a withdrawal; `0` for a failed removal. |
| `vre_bridge.refresh.size` | histogram, `By` | `project`, `kind` (`dump`, `snapshot`, `media`) | What a refreshed project's refresh handled. |
| `vre_bridge.active` | gauge | none | `1` when the bridge is active, `0` when it is not configured or its configuration is invalid. |

Attribute values come from the allowlist, the locations' shortcode directories and fixed sets, so the series stay
few. Without `OTEL_EXPORTER_OTLP_ENDPOINT` nothing is exported. The counter starts at each process start, and a
series exists only from its first outcome, so an alert on its increase sees nothing before then: alert on the
series' absence as well, and expect a project's first `removed` or `failed` to show as a new series rather than as
an increase. The gauge stops being reported when the process stops. A failed refresh logs its detail, then
`refresh ended` at the same level, so an alert on the level counts each failure more than once: alert on the
counter, or on `message="refresh ended"`, not on the level. Suggested alerts:

- a project `refused` or `failed` two nights running;
- any `failed` outcome with `server=(none)`: a project no longer allowlisted could not be removed;
- any `removed` outcome: a project left the allowlist, perhaps by a typo;
- `vre_bridge.active` at `0` in a deployment that configures the bridge;
- no outcome for 48 hours while `vre_bridge.active` is `1`.

## Keys

Each deployment has its own service name (`dsp-repository-stage`, `dsp-repository-prod`), key pair and key id,
because dsp-api treats an export as the caller's own by the service name: two deployments sharing one would
recover, and delete, each other's exports. A key id must be unique among the trusted keys of a DSP server, which
refuses to start otherwise; a dated one such as `dsp-repository-prod-2026-11` avoids clashes.

The private key is a file, mounted read-only into the container; `EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES` names it
per server, e.g. `prod=/run/secrets/vre_bridge_signing_key.pem`. The key never travels in an environment
variable, so it does not appear in the container's configuration.

```sh
# the file to mount: PEM PKCS#8, as written
openssl genpkey -algorithm ed25519 -out vre-bridge.pem
# for dsp-api's trusted keys, with the key id and the service name
openssl pkey -in vre-bridge.pem -pubout -outform DER | base64 | tr -d '\n'
```

The bridge reads each key file and signs a probe token with its key at start; a file that is missing, cannot be
read or holds no key that signs leaves the bridge inactive.

## What to know

- **The change check sees resources only.** A project whose ontology or lists change with no resource change, or
  whose resources were erased, is not refreshed until a resource changes or a release bumps the format, the
  transformation or its shipped inputs; an operator's refresh closes the gap at once.
- **A busy dump slot** usually means another caller's export on that server, often the other deployment's nightly
  run: stage and prod both dump from prod, so their `NIGHTLY_AT` differ. The retries cover an overlap; a project that
  fails with reason `busy` every night needs the two times further apart.
- **A format bump** (a new interim format version) releases `access-server` first, reading the new version and the
  old, then `deposit-server`. The other way round, `sync` answers the project as unavailable until it is released.
- **CPE's held-back re-activation.** When a refreshed project does not load in CPE, CPE keeps serving what it served,
  logs `CPE's re-activation is held back` and tries once more; beyond that, a restart of `access-server` is the lever
  ([CPE operations](../cpe/operations.md)).
- **An accepted race.** Two refreshes of one project within a reader's NFS attribute-cache interval can delete the
  generation a reader still reads through a stale token, since only the newest two are kept. Do not run the
  operator's refresh twice in a row for a project on an NFS location.
