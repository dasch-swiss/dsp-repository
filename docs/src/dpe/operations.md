# DPE Operations Guide

Operations documentation for the DPE infrastructure team.

## Docker Image

- **Base**: `gcr.io/distroless/static-debian12:nonroot`
- **User**: uid 65534 (nonroot, built-in to distroless)
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
- Project roles misplaced in a person's `jobTitles` (e.g. "Project Leader", "Project staff", "Creator"). Such a role belongs in the project's `attributions` (`contributorType`), where the OAI-PMH creator/contributor logic can read it. The role vocabulary is `JOB_TITLE_ROLE_WORDS` in `dpe-core`.

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
| `DPE_OAI_RATE_LIMIT_PER_SECOND` | No | `1` | Per-IP rate limit on `/dpe/oai`: seconds per request once the burst is spent. `1` ≈ 60 requests/minute sustained. See [OAI-PMH](./oai-pmh.md). |
| `DPE_OAI_RATE_LIMIT_BURST` | No | `60` | Per-IP burst allowance on `/dpe/oai`: back-to-back requests before the sustained rate applies. |
| `DPE_OAI_PAGE_SIZE` | No | `100` | Items per page in `ListRecords` / `ListIdentifiers` responses before a resumption token is emitted. Non-positive or non-numeric values fall back to the default. See [OAI-PMH](./oai-pmh.md). |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | *(none)* | OTLP gRPC endpoint (e.g., `http://alloy:4317`). When unset, OTel falls back to no-op export. |
| `OTEL_SERVICE_NAME` | No | *(none)* | Service name for OTel resource attributes (e.g., `dpe`) |
| `OTEL_RESOURCE_ATTRIBUTES` | No | *(none)* | Comma-separated OTel resource attributes (e.g., `service.namespace=dpe,service.version=0.2.1,deployment.environment=prod`) |
| `PYROSCOPE_ENDPOINT` | No | *(none)* | Pyroscope HTTP endpoint (e.g., `http://pyroscope:4040`). When unset, profiling is disabled. |
| `DPE_SITE_ADDR` | No | `127.0.0.1:4000` | Listen address and port. The Docker image sets `0.0.0.0:8080`. |
| `DPE_PUBLIC_DIR` | No | `modules/dpe/public` | Directory served as static assets by `ServeDir` (favicon, logo, vendored JS, project images, and the compiled `app.<hash>.css`). |
| `DPE_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`. |

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
