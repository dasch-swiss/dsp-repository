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
| `EDITOR_DATA_DIR` | No | `modules/dpe/server/data` | Directory holding the published project/person/organization set baked into the image. Reported at startup. |
| `EDITOR_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`. |
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
- The writable data volume for SQLite. Development and tests use an in-memory database, so only production waits on it.

### Data volume (once persistence lands)

- Mount the **directory**, not the database file, so SQLite can create its `-wal` and `-shm` siblings next to it.
- It must be **writable by uid 65532**. Chowning to 65534 produces `unable to open database file`, which reads like a wrong mount path and sends the reader hunting for a typo in a path that is perfectly correct.
- Docker Swarm does **not** create missing bind-mount host directories the way `docker run -v` does, so Ansible must create *and* chown the directory before the stack starts.
- Keep `replicas: 1` with `order: stop-first`, so a rolling update never briefly runs two processes against one database file. Node pinning is unnecessary — there is no multi-node Swarm.

## Resource Requirements

- **Memory**: ~50–100 MB typical
- **CPU**: Minimal (server-side rendering; the published data set is cached in memory)
- **Disk**: Data files + static assets, plus the SQLite database once persistence lands

## Logging

Structured logging via `init-tracing-opentelemetry` (an OTel-aware tracing subscriber).

- **Production** (`EDITOR_ENV=PROD`): JSON logs to stdout only.
- **Local development** (`EDITOR_ENV=DEV` with `OTEL_EXPORTER_OTLP_ENDPOINT` set): logs additionally exported over OTLP.

Panics are routed through `tracing` as structured events using the OTel exception semantic conventions (`exception.message`, `exception.stacktrace`, `exception.type`, `thread.name`, plus `code.filepath`/`code.lineno`), so a production panic lands in the same log pipeline as everything else instead of on stderr. `RUST_BACKTRACE=1` is set in the image, and the release profile keeps DWARF line tables, so those backtraces carry file and line.
