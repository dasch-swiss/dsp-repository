# DPE Operations Guide

Operations documentation for the DPE infrastructure team.

## Docker Image

- **Base**: `gcr.io/distroless/static-debian12:nonroot`
- **User**: uid **65532** — distroless `NONROOT`. Not 65534, which is `nobody`; verified in [`common/variables.bzl`](https://github.com/GoogleContainerTools/distroless/blob/main/common/variables.bzl).
- **Shell**: None (distroless — no SSH possible)
- **Binary**: Static musl-linked `dpe-server` (CLI with subcommands)

## CLI Commands

The `dpe-server` binary provides three subcommands:

| Command | Description |
|---------|-------------|
| `dpe-server serve` | Start the web server |
| `dpe-server validate <data_dir>` | Validate all data files under the given directory |
| `dpe-server healthcheck [--url URL]` | Check if the server is healthy (default: `http://localhost:8080/healthz`) |

### `dpe-server validate`

Validates JSON data files for structural correctness and cross-reference integrity.

```bash
dpe-server validate ./data
```

**What it checks:**
- JSON schema validity for all data file types (projects, persons, organizations, records, clusters, collections)
- Cross-references between projects, persons, and organizations
- Orphaned files that are not referenced by any parent entity
- Project roles misplaced in a person's `jobTitles` (e.g. "Project Leader", "Project staff", "Creator"). Such a role belongs in the project's `attributions` (`contributorType`), where the OAI-PMH creator/contributor logic can read it. The role vocabulary is `JOB_TITLE_ROLE_WORDS` in `shared-metadata`.
- Every distinct `temporalCoverage` name resolves to a structured date (ChronOntology or the offline enrichment table), or is explicitly marked `source: "unresolved"`. See `docs/src/dpe/oai-pmh.md` → *Temporal coverage*.

**Exit codes:**
- `0` — all data files are valid
- `1` — validation errors found (details printed to stderr)

### `dpe-server healthcheck`

Lightweight probe for Docker HEALTHCHECK or monitoring:

```bash
dpe-server healthcheck                             # default: http://localhost:8080/healthz
dpe-server healthcheck --url http://localhost:9090/healthz # custom URL
```

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 8080 | HTTP | Application server |

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUST_LOG` | No | `info` | Log level filter (e.g., `dpe_server=info,tower_http=debug`) |
| `DPE_DATA_DIR` | No | `modules/dpe/server/data` | Path to project/record JSON data files. Legacy alias: `DATA_DIR` (checked if `DPE_DATA_DIR` is unset) |
| `DPE_FATHOM_SITE_ID` | No | *(none)* | Fathom Analytics site ID (not a secret) |
| `DPE_SHOW_PLACEHOLDER_VALUES` | No | `false` | Show placeholder values (MISSING, CALCULATED) in the UI, styled in red. Enable on DEV/STAGE for QA visibility. |
| `DPE_OAI_BASE_URL` | No | `https://repository.dasch.swiss/dpe/oai` | Public base URL emitted as the OAI-PMH `baseURL` and echoed in `<request>` elements. Set per environment to match the public endpoint (e.g. `https://api.dev.dasch.swiss/dpe/oai` on DEV, `http://localhost:4000/dpe/oai` locally). See [OAI-PMH](./oai-pmh.md). |
| `DPE_PUBLIC_BASE_URL` | No | `https://repository.dasch.swiss` | Public origin of the site itself, used to build the landing-page and machine-readable-representation URLs carried by the embedded metadata and the `Link` header. Origin only: scheme `http` or `https`, no path, query or trailing slash. A bad value fails startup. Independent of `DPE_OAI_BASE_URL` (see the note below). |
| `DPE_ARK_RESOLVER_BASE_URL` | No | *(none)* | Origin every emitted ARK is rewritten to carry, and the origin the deployment's own `/ark:/{naan}/{shoulder}/{shortcode}` resolver answers on. **Leave unset everywhere but a PR preview.** Unset, the ARKs are the ones the corpus records, resolving through `ark.dasch.swiss`, and no resolver route is mounted. Origin only, validated exactly as `DPE_PUBLIC_BASE_URL` is. See the note below. |
| `DPE_OAI_RATE_LIMIT_PER_SECOND` | No | `1` | Per-IP rate limit, seconds per request once the burst is spent. `1` ≈ 60 requests/minute sustained. Governs `/dpe/oai` and both machine-readable representation routes, which share one bucket. See [OAI-PMH](./oai-pmh.md) and [Machine-Readable Metadata](./machine-readable-metadata.md). |
| `DPE_OAI_RATE_LIMIT_BURST` | No | `60` | Per-IP burst allowance on the same three routes: back-to-back requests before the sustained rate applies. |
| `DPE_OAI_PAGE_SIZE` | No | `100` | Items per page in `ListRecords` / `ListIdentifiers` responses before a resumption token is emitted. Non-positive or non-numeric values fall back to the default. See [OAI-PMH](./oai-pmh.md). |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | *(none)* | OTLP gRPC endpoint (e.g., `http://alloy:4317`). When unset, OTel falls back to no-op export. |
| `OTEL_SERVICE_NAME` | No | *(none)* | Service name for OTel resource attributes (e.g., `dpe`) |
| `OTEL_RESOURCE_ATTRIBUTES` | No | *(none)* | Comma-separated OTel resource attributes (e.g., `service.namespace=dpe,service.version=0.2.1,deployment.environment=prod`) |
| `PYROSCOPE_ENDPOINT` | No | *(none)* | Pyroscope HTTP endpoint (e.g., `http://pyroscope:4040`). When unset, profiling is disabled. |
| `DPE_SITE_ADDR` | No | `127.0.0.1:4000` | Listen address and port. The Docker image sets `0.0.0.0:8080`. |
| `DPE_PUBLIC_DIR` | No | `modules/dpe/public` | Directory served as static assets by `ServeDir` (favicon, logo, vendored JS, project images, and the compiled `app.<hash>.css`). |
| `DPE_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`. |

> **Two base URLs, on purpose.** `DPE_PUBLIC_BASE_URL` and `DPE_OAI_BASE_URL` are set independently and neither is derived from the other. The OAI endpoint advertises its own `baseURL` in every response, and on DEV that endpoint lives on a different host (`https://api.dev.dasch.swiss/dpe/oai`) from the site (`https://repository.dev.dasch.swiss`). Set both per environment.
>
> **Assessing a local run from a container.** A FAIR assessor running in Docker resolves the URLs the page emits, so `DPE_PUBLIC_BASE_URL` must name a host the container can reach — `http://host.docker.internal:4000`, not `http://localhost:4000`, which inside the container is the container itself.
>
> **PR previews set all three themselves.** `cloud-run-dpe-pull-request.yml` runs `gcloud run services update` right after the deploy, setting `DPE_PUBLIC_BASE_URL`, `DPE_OAI_BASE_URL` and `DPE_ARK_RESOLVER_BASE_URL` to the preview's own Cloud Run URL. It is a second step because Cloud Run assigns that URL only once the service exists. Each one left unset makes the preview advertise production: the typed links and the `303` target, the OAI `baseURL`, and every emitted ARK respectively.
>
> **Why a preview rewrites its ARKs.** A preview runs code that is not merged. If it publishes the recorded ARK, a FAIR assessor harvests an identifier that resolves to production — so the deployment it assessed and the deployment it dereferenced are two different things running different code, and the assessment says nothing about either. `DPE_ARK_RESOLVER_BASE_URL` makes the preview's identifiers point at the preview, and mounts the resolver that answers them, so what is measured is one deployment throughout.
>
> **Applied once, as corpus data enters.** `dpe-core`'s caches normalise the ARK host on load, so everything downstream — the embedded JSON-LD, the Dublin Core tags, the `Link` header, the OAI payloads, the sidebar permalink a person clicks and copies, and the `pid` in `/dpe/api/v2/projects` — is correct without any of them knowing a resolver exists. Conceptually this is the `sync` capability's work and moves there when `sync` lands; `dpe_core::ark` records that.
>
> Only the host is rewritten. The ARK path — `ark:/72163/1/{shortcode}` — is the identifier and passes through untouched, and **recorded text is quoted rather than asserted**: a `howToCite` citation whose author wrote the ARK into the sentence, and a project's recorded website (083D records its own ARK there), are served as written.
>
> The resolver answers project ARKs only, with a `302` to the landing page, mirroring `ark.dasch.swiss`. A record ARK gets a `404`: DPE serves no record landing page, and `ark.dasch.swiss` resolves a record ARK to the VRE rather than to DPE.
>
> **Do not set it on DEV, STAGE or PROD.** Those deployments *are* what the recorded ARK resolves to, or are meant to become it. Setting it there would make them publish identifiers that only they answer, and mount a second ARK authority beside `ark.dasch.swiss`.

> **What the rate limit does not bound.** It bounds the request *rate*, not the
> per-request size, and the uncapped JSON-LD representation is large: project
> 081C measures ~3.5 MB. At the defaults (`per_second = 1`, `burst = 60`) one IP
> can therefore pull roughly 210 MB of transient allocation and egress in a
> burst window, settling to ~3.5 MB/s/IP afterwards. Lower `DPE_OAI_RATE_LIMIT_BURST`
> if that matters more than a harvester's throughput does.

> **Rate limiting and reverse proxies.** The OAI (`/dpe/oai`) and telemetry (`/telemetry/collect`) rate limits key on the client IP, taken from the **rightmost** `X-Forwarded-For` entry (the address Traefik itself appends), falling back to the connection peer address. Reading the rightmost entry — not the leftmost — is deliberate: Traefik appends the real client after any `X-Forwarded-For` value the client supplied, so the rightmost entry is proxy-authored and cannot be spoofed, while the leftmost stays attacker-controlled. This holds only while Traefik is the sole hop in front of DPE; a second proxy that appends to `X-Forwarded-For` would shift the trusted entry and require counting hops from the right.

## Health Check

- **Endpoint**: `GET /healthz`
- **Response**: `200 OK` (no body)
- **Purpose**: Lightweight probe for Traefik/load balancers. Does not render any page.

## Data Volume

- **Mount point**: Value of `DPE_DATA_DIR`
- **Access**: Read-only
- **Contents**: Project metadata JSON files, organized by type (`projects/`, `persons/`, `organizations/`, `clusters/`, `collections/`, `records/`)

## Resource Requirements

The DPE is lightweight — it serves static data with no database.

- **Memory**: ~50-100 MB typical
- **CPU**: Minimal (SSR rendering is fast, data is cached in-memory)
- **Disk**: Data files + static assets (~50 MB)

### Landing-page metadata points at dsp-ingest

A project landing page advertises its records' files as `schema:distribution`,
each with the dsp-ingest URL that serves the bytes
([Machine-Readable Metadata](./machine-readable-metadata.md#record-files)). A
consumer that follows those URLs downloads from **`ingest.dasch.swiss`, not from
DPE**, so DPE's per-IP rate limiter does not bound the traffic.

How much there is to follow, for the two largest file-carrying projects in the
committed corpus:

| | In the embedded block | Bytes if all of those are fetched | In `/metadata.jsonld` | Bytes if all of those are fetched |
|---|---:|---:|---:|---:|
| 0868 | 100 | 90.9 MB | 7,716 | 2.72 GB |
| 0803 | 4 | 81.9 MB | 4,062 | 139.7 GB |

A well-behaved assessor pulls far less. F-UJI 3.5.0 takes at most five files per
MIME type and truncates each download at 1,000,000 bytes, so a full assessment
drew **26 files and 11.3 MB for 0868 (558 s)** and **6 files and 6.0 MB for 0803
(307 s)**, measured on 2026-09-18. Its draw scales with how many distinct file
types a project has, not with how many records it holds.

This is a change in convenience rather than in capability. The same bytes were
already reachable by enumerating the OAI-PMH set `project:{shortcode}` and
calling `/dpe/records/{shortcode}/{record_id}/file` per record, which returns
the same `downloadUrl`. What changed is that one request now returns every URL
at once.

**Retiring ingest carries a metadata obligation.** All 11,778 published
`contentUrl` values name `ingest.dasch.swiss`. When media moves to Vitrinli and
downloads are served by the Access Area's `media` capability, the host in them
changes. Metadata heals going forward on its own — the URL comes from the corpus
export, so the first export after the migration carries the new host and DPE
re-serves it without a code change — but copies already harvested by third
parties do not, and `schema:distribution` exists to be harvested. Plan the
retirement to include a corpus re-export and a re-publish, or external copies
404.

## Logging

Structured logging via `init-tracing-opentelemetry` (OTel-aware tracing subscriber). In production (`DPE_ENV=PROD`), logs are JSON-formatted to stdout only. In local development (`DPE_ENV=DEV`), logs are additionally exported via OTLP to Loki when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. Configure levels with `RUST_LOG`:

```bash
# Default (info level)
RUST_LOG=info

# Debug HTTP requests
RUST_LOG=dpe_server=info,tower_http=debug

# Verbose debugging
RUST_LOG=debug
```

## Observability

### Fathom Analytics

Privacy-friendly, GDPR-compliant analytics. No cookies, no personal data collected.

**Configuration:** Set the `DPE_FATHOM_SITE_ID` environment variable to your Fathom site ID (not a secret). The tracking script is automatically injected into the HTML shell.

**What gets tracked:**
- Page views
- Tab switches (detected automatically via `history.replaceState`)

**Disable:** Omit the `DPE_FATHOM_SITE_ID` environment variable — no tracking script is rendered.

### OpenTelemetry

DPE exports traces, metrics, and structured logs via OTLP gRPC. In production, the OTLP endpoint points to Grafana Alloy, which forwards to Grafana Cloud (Tempo for traces, Mimir for metrics, Loki for logs).

When `OTEL_EXPORTER_OTLP_ENDPOINT` is not set, the OTel SDK falls back to no-op export — the application runs normally without telemetry export. See `docs/src/dpe/observability.md` for the developer guide.

### Continuous Profiling (Pyroscope)

CPU profiling via Grafana Pyroscope. Samples at 100Hz and pushes profiles to the configured endpoint.

**Configuration:** Set `PYROSCOPE_ENDPOINT` to the Pyroscope HTTP endpoint. When unset, no profiling agent runs and there is zero overhead.

**What gets profiled:**
- CPU time per function (sampling-based, 100 samples/second)
- Flame graphs viewable in Grafana (Explore > Pyroscope)
