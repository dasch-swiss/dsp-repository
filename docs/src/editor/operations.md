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
| `EDITOR_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`; **`PROD` requires `EDITOR_SMTP_HOST`** or startup is refused, because a relay-less production writes every login code to the log. The PR preview and `just run-docker-editor` override it to `DEV`. |
| `EDITOR_DB_DIR` | No | *(none — in-memory)* | Directory holding the SQLite database. **Unset means in-memory**, not a path — see [Database](#database). Names the *directory*, never the database file. |
| `EDITOR_DB_READERS` | No | `4` | Size of the reader connection pool. The writer pool is always one connection. |
| `EDITOR_DB_BUSY_TIMEOUT_MS` | No | `5000` | SQLite `busy_timeout`, applied per connection. |
| `EDITOR_RDU_EMAILS` | No | *(none)* | Comma-separated addresses that always have an RDU account (REQ-7.2). Reconciled on every start: missing accounts are created, an existing depositor listed here is promoted. Removing an address **does not revoke** the account — see [Login and mail](#login-and-mail). |
| `EDITOR_SMTP_HOST` | No | *(none — console)* | SMTP relay host. **Unset means codes are written to the log** (REQ-6.8) and the service stays usable. That is the development and PR-preview default. |
| `EDITOR_SMTP_PORT` | No | `587` | Submission with STARTTLS, which is what `smtp-relay.gmail.com` speaks. |
| `EDITOR_SMTP_USERNAME` | No | *(none)* | Relay username. Must be set together with the password. |
| `EDITOR_SMTP_PASSWORD` | No | *(none)* | Relay password. Redacted in any debug rendering of the configuration. |
| `EDITOR_SMTP_FROM` | No | `noreply@dasch.swiss` | Envelope sender. Must be a domain with DKIM enabled in the Workspace Admin Console — see [Authentication](./authentication.md#relay-prerequisites-not-the-applications-job-but-they-block-delivery). |
| `EDITOR_SMTP_BREAK_GLASS` | No | `false` | When a **configured** relay fails, write the undelivered code to the log instead of rolling it back. Off by default: it puts a live credential in the log pipeline. See [Login and mail](#login-and-mail). |
| `EDITOR_LOGIN_COOLDOWN_SECS` | No | `60` | Before another code may be sent to the same address (REQ-6.5). Must be shorter than the ten-minute code lifetime, which startup validates. |
| `EDITOR_LOGIN_MAX_FAILED` | No | `10` | Consecutive account-level failures before throttling. NIST SP 800-63B-4's ceiling is 100, which startup also validates. |
| `EDITOR_LOGIN_LOCKOUT_SECS` | No | `900` | How long throttling lasts after the cap is reached. |
| `EDITOR_MAIL_DAILY_CAP` | No | `500` | Codes that may be sent across **all** users in 24 hours. Sits below the relay's 10,000/day so a resend loop cannot exhaust a quota shared with other senders. |
| `EDITOR_SESSION_ABSOLUTE_SECS` | No | `43200` (12 h) | Absolute session lifetime, set at creation and never extended. |
| `EDITOR_SESSION_IDLE_SECS` | No | `7200` (2 h) | Idle session timeout. Must not exceed the absolute lifetime, which startup validates. |
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
- **SMTP relay host, port and credentials**, provisioned as vaulted group vars. Development and the PR previews are unblocked by the console fallback, so only production login waits on it. DKIM and SPF for `dasch.swiss` are already set up.

## Login and mail

Design and rationale live in [Authentication](./authentication.md). What an operator needs:

### Accounts

`EDITOR_RDU_EMAILS` is reconciled on every start. Adding an address creates or promotes an account on the next deploy. **Removing one does not revoke anything** — startup logs a warning naming any `rdu` account the configuration no longer lists, and the account has to be removed by hand until account management lands. Treat that warning as a to-do, not as noise.

A malformed entry stops startup rather than being skipped: every entry becomes an account that administers the service, so a typo is an administrator who can never sign in, and the symptom would otherwise be "my code never arrives" weeks later.

### No relay configured

With `EDITOR_SMTP_HOST` unset, every code is written to the log at `WARN` and the service stays usable (REQ-6.8). The log line carries the message body and **not** the recipient — whoever is testing knows the address they typed, and REQ-6.10 forbids one in a log.

### A broken relay

Default behaviour: the send fails, the code and its cooldown are rolled back, the user's response is unchanged (it has to be — see the anti-enumeration reasoning in [Authentication](./authentication.md#anti-enumeration-is-a-property-of-the-response-not-a-branch)), and the failure is logged with a classification and an SMTP status code.

If the relay is broken long enough that people are locked out:

1. Set `EDITOR_SMTP_BREAK_GLASS=true` and redeploy. Codes are still attempted through the relay, and undelivered ones are written to the log, where an operator can read them out to the person waiting.
2. Fix the relay.
3. **Set it back to `false`.** While it is on, every relay hiccup writes a live login code into a log pipeline that retains it for weeks and is readable by everyone with log access.

Unsetting `EDITOR_SMTP_HOST` entirely is the heavier version of the same escape hatch: it routes everything to the log without trying the relay at all, and needs a second redeploy to undo.

### Diagnosing "I never got a code"

No address appears in any log, so the trail is the opaque `auth.subject` correlation id (the account's UUID) in the auth events, plus the per-account "last code issued at" that RDU will see once the depositor list lands. A user who is throttled after repeated wrong entries is told only "that code is not valid" — deliberately, because a distinct message would confirm to anyone that the address has an account. Check the auth log for `auth.outcome = "locked_out"` before assuming a delivery problem.

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

### Expired rows

An hourly background sweep deletes expired login codes and sessions. Nothing depends on it succeeding — a failure is logged and the next hour tries again — and the first sweep runs one interval after start, so a restart loop never spends its time sweeping.

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
