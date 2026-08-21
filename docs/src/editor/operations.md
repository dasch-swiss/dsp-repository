# Editor Operations Guide

Operations documentation for the metadata editor.

## Docker Image

- **Image**: `daschswiss/metadata-editor`
- **Base**: `gcr.io/distroless/static-debian12:nonroot`
- **User**: uid **65532** — distroless `NONROOT`. Not 65534, which is `nobody`; verified in [`common/variables.bzl`](https://github.com/GoogleContainerTools/distroless/blob/main/common/variables.bzl).
- **Shell**: None (distroless — no SSH possible)
- **Binary**: Static musl-linked `editor-server`

The Dockerfile packages pre-built artifacts; the Rust build happens in
`.github/actions/build-editor`. `just build-docker-editor` reproduces the same
staging locally, compiling the static musl binary inside a `rust:<channel>-bookworm`
container — a host `cargo build --target *-unknown-linux-musl` needs a musl
cross-linker, which macOS does not have. It defaults to the host architecture so
the build is native; `just build-docker-editor arch=x86_64` reproduces the amd64
image CI publishes, under emulation.

### Data directory (a deliberate build input)

The published project/person/organization set is **DPE's content**, not the editor's. Both `.github/actions/build-editor` and `just build-docker-editor` copy `modules/dpe/server/data` into the staging directory, and the Dockerfile places it at `/app/server/data`, where `EDITOR_DATA_DIR` points. Git stays the source of truth and the editor reads an image-baked snapshot, so a data change reaches the editor by rebuilding the image, not at runtime.

That copy is the seam, and it is explicit on both sides. `EditorConfig` carries **no default** for `data_dir`: the only plausible one is a relative path into DPE's tree, which would let a records reader resolve another module's directory instead of failing on an unconfigured seam. Every environment that reads records names the directory — the image via `ENV EDITOR_DATA_DIR`, local development via `just dev-editor`. Unset is a legitimate state while nothing reads records, and is reported as `<unset>` at startup rather than as an invented path.

Moving the directory under `modules/platform/` was considered and rejected: the shared-crate rule in [Repo Structure](../repo_structure.md#shared-crates) is about crates, and this is DPE's owned content consumed through an existing explicit seam. Reopen it only if a third consumer appears.

## CLI Commands

| Command | Description |
|---------|-------------|
| `editor-server serve` | Start the web server |
| `editor-server healthcheck [--url URL]` | Check if the server is healthy (default: `http://localhost:8080/healthz`) |

`healthcheck` only accepts loopback URLs (`http://localhost`, `http://127.0.0.1`, `http://[::1]`). The URL arrives as a CLI flag from the Docker `HEALTHCHECK`, and restricting it stops a mistyped or tampered value turning the container into an SSRF probe.

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 8080 | HTTP | Application server |

Locally the default is `127.0.0.1:4100`, deliberately not DPE's 4000, so `just dev` and `just dev-editor` can run at the same time.

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUST_LOG` | No | `info` | Log level filter (e.g. `editor_server=info,tower_http=debug`) |
| `EDITOR_SITE_ADDR` | No | `127.0.0.1:4100` | Listen address and port. The Docker image sets `0.0.0.0:8080`. |
| `EDITOR_PUBLIC_DIR` | No | `modules/editor/public` | Directory served as static assets by `ServeDir` (favicon, logo, vendored JS, the telemetry module, and the compiled `app.<hash>.css`). |
| `EDITOR_DATA_DIR` | Yes, to read records | *(none)* | Directory holding the published project/person/organization set baked into the image. No default — see [Data directory](#data-directory-a-deliberate-build-input). Reported at startup, as `<unset>` when absent. |
| `EDITOR_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`. |
| `EDITOR_DB_DIR` | No | *(none — in-memory)* | Directory holding the SQLite database. **Unset means in-memory**, not a path — see [Database](#database). Names the *directory*, never the database file. |
| `EDITOR_DB_READERS` | No | `4` | Size of the reader connection pool. The writer pool is always one connection. |
| `EDITOR_DB_BUSY_TIMEOUT_MS` | No | `5000` | SQLite `busy_timeout`, applied per connection. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | *(none)* | OTLP gRPC endpoint (e.g. `http://alloy:4317`). When unset, OTel falls back to no-op export. |
| `OTEL_SERVICE_NAME` | No | *(none)* | Service name for OTel resource attributes (e.g. `editor`) |
| `OTEL_RESOURCE_ATTRIBUTES` | No | *(none)* | Comma-separated OTel resource attributes (e.g. `service.namespace=editor,service.version=0.8.2,deployment.environment=prod`) |
| `PYROSCOPE_ENDPOINT` | No | *(none)* | Pyroscope HTTP endpoint (e.g. `http://pyroscope:4040`). When unset, profiling is disabled with zero overhead. |

Configuration is layered: code defaults → optional `editor.toml` → `EDITOR_*` environment variables, which override everything.

> **Rate limiting and reverse proxies.** The telemetry beacon (`/telemetry/collect`) rate limit keys on the client IP taken from the **rightmost** `X-Forwarded-For` entry — the address Traefik itself appends — falling back to the connection peer address. Reading the rightmost entry, not the leftmost, is deliberate: Traefik appends the real client after any value the client supplied, so the rightmost entry is proxy-authored and cannot be spoofed, while the leftmost stays attacker-controlled. This holds only while Traefik is the sole hop in front of the editor; a second proxy that appends to `X-Forwarded-For` would shift the trusted entry and require counting hops from the right.

## Health Check

- **Endpoint**: `GET /healthz`
- **Response**: `200 OK` (no body)
- **Purpose**: Lightweight probe for Traefik/load balancers. Declared after the OTel layers, so probes are not traced.

> **`/healthz` returns 404 on the Cloud Run PR previews.** Something in front of the container answers that exact path with Google's own error page — the response carries no `server: Google Frontend` header and no `traceparent`, unlike every response the app produces, and neighbouring paths (`/healthz2`, `/health`, `/healthz/`) all reach the app normally. DPE's preview behaves identically and has had the same route for far longer, so this is not editor-specific and not new. The precise mechanism is unconfirmed; treat it as "previews cannot be used to test the health endpoint".
>
> It does not affect any real deployment: production runs under Docker Swarm behind Traefik, where `/healthz` answers 200, and the image's own `HEALTHCHECK` calls `editor-server healthcheck` against loopback *inside* the container, which never crosses a proxy. To check the endpoint by hand, run the image locally (`just build-docker-editor && just run-docker-editor`) rather than probing a preview URL.

## Deployment

The editor deploys as its **own Swarm stack on the same VMs as DPE**, with its own inventory host entry and its own playbook — not as a second stack inside the `repository` inventory group. Co-habiting VMs with separate inventory hosts is how the platform already runs multiple stacks (`ark` and `mosaic` share a host that way). The `repository` group and `repository.yml` playbook are a holdover that should really be called `dpe`; splitting them is separate work.

It deploys on a **separate hostname** from DPE, not a path under `repository.dasch.swiss` — see the CSRF reasoning in [Architecture](./architecture.md#relationship-to-dpe).

Not yet in place, and blocking production deployment only:

- The editor is **not registered as a deployable service in Jenkins**, so the DEV deploy trigger in `editor-docker-publish.yml` is marked `continue-on-error` and only warns. Remove that guard once the Jenkins job exists, so a genuinely broken webhook is loud again.
- The writable data volume `EDITOR_DB_DIR` points at, tracked as [INFRA-1378](https://linear.app/dasch/issue/INFRA-1378/extend-deploy-volumes). Development and tests use the in-memory database, so only production waits on it.

## Database

One SQLite database, holding users, sessions, one-time codes, drafts, submissions and approved records. See [Architecture](./architecture.md#persistence) for the pool and PRAGMA design.

### In-memory is the default, deliberately

`EDITOR_DB_DIR` has **no default**, and unset means in-memory rather than some path the editor invented. That is preview safety, not a convenience: the Cloud Run PR preview has no mounted volume, runs `--max-instances=1`, and is publicly reachable, so a publicly reachable preview must not be able to accumulate login codes, sessions or drafts. It also means `just dev-editor` and `cargo test` need no volume.

The Docker image deliberately does **not** set `EDITOR_DB_DIR`, for the same reason — the previews run that image. Production sets it through the deployment.

### Data volume

- Mount the **directory**, not the database file, so SQLite can create its `-wal` and `-shm` siblings next to it. `EDITOR_DB_DIR` names the directory and the filename is fixed at `editor.sqlite3`, so pointing it at a file fails at startup with a message saying so rather than at the first WAL write.
- It must be **writable by uid 65532**. Chowning to 65534 produces `unable to open database file`, which reads like a wrong mount path and sends the reader hunting for a typo in a path that is perfectly correct.
- Docker Swarm does **not** create missing bind-mount host directories the way `docker run -v` does, so Ansible must create *and* chown the directory before the stack starts.
- Keep `replicas: 1` with `order: stop-first`, so a rolling update never briefly runs two processes against one database file. Node pinning is unnecessary — there is no multi-node Swarm.

### Startup pre-flight

Before opening the database, startup writes and removes a probe file in `EDITOR_DB_DIR` and reports the three failures separately: the directory is missing, it is a file rather than a directory, or it cannot be written to. Without the probe all three arrive as SQLite's `unable to open database file`, which is the same message for a typo in the path, a wrong uid and a root-owned mount. The unwritable-directory message names uid 65532 explicitly.

### Storage type

virtiofs is preferred over a block device with ext4, and only virtiofs gets the automatic ZFS snapshots. WAL is safe on it: SQLite's restriction is about multiple **hosts**, not multiple processes, and every accessor lives inside one guest VM.

If virtiofs `mmap` ever misbehaves, the escape hatch is `locking_mode=EXCLUSIVE` set before first access, which skips the wal-index entirely — valid here because access is single-process. It is deliberately **not** a configuration knob: an untested code path is worse than a one-line change in `init_connection`, and the situation calling for it has never occurred.

### Backups

Optional, per the PRD. Git holds everything irreplaceable; a total loss of the volume costs drafts, in-flight submissions, approved records not yet collected and the depositor table, all re-creatable. `synchronous=NORMAL` is set with WAL on that basis: a commit no longer fsyncs, so an OS crash or power loss can lose the last transactions — never corrupt the database, and never on an application crash.

## Resource Requirements

- **Memory**: ~50–100 MB typical
- **CPU**: Minimal (server-side rendering; the published data set is cached in memory)
- **Disk**: Data files + static assets, plus the SQLite database and its `-wal`/`-shm` siblings when `EDITOR_DB_DIR` is set

## Logging

Structured logging via `init-tracing-opentelemetry` (an OTel-aware tracing subscriber).

- **Production** (`EDITOR_ENV=PROD`): JSON logs to stdout only.
- **Local development** (`EDITOR_ENV=DEV` with `OTEL_EXPORTER_OTLP_ENDPOINT` set): logs additionally exported over OTLP.

Panics are routed through `tracing` as structured events using the OTel exception semantic conventions (`exception.message`, `exception.stacktrace`, `exception.type`, `thread.name`, plus `code.filepath`/`code.lineno`), so a production panic lands in the same log pipeline as everything else instead of on stderr. `RUST_BACKTRACE=1` is set in the image, and the release profile keeps DWARF line tables, so those backtraces carry file and line.
