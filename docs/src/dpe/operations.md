# DPE Operations Guide

Operations documentation for the DPE infrastructure team.

## Docker Image

- **Base**: `gcr.io/distroless/static-debian12:nonroot`
- **User**: uid 65534 (nonroot, built-in to distroless)
- **Shell**: None (distroless — no SSH possible)
- **Binary**: Static musl-linked `dpe-server`

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 8080 | HTTP | Application server |

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUST_LOG` | No | `info` | Log level filter (e.g., `dpe_server=info,tower_http=debug`) |
| `DPE_DATA_DIR` | No | `modules/dpe/server/data` | Path to project/record JSON data files |
| `DPE_FATHOM_SITE_ID` | No | *(none)* | Fathom Analytics site ID (not a secret) |
| `LEPTOS_SITE_ADDR` | No | `0.0.0.0:8080` | Listen address and port |
| `LEPTOS_SITE_ROOT` | No | `site` | Path to static site assets |
| `LEPTOS_SITE_PKG_DIR` | No | `pkg` | JS/CSS package subdirectory |
| `LEPTOS_OUTPUT_NAME` | No | `dpe` | CSS/JS output filename prefix |
| `LEPTOS_ENV` | No | `PROD` | Leptos environment (DEV or PROD) |

## Health Check

- **Endpoint**: `GET /healthz`
- **Response**: `200 OK` (no body)
- **Purpose**: Lightweight probe for Traefik/load balancers. Does not hit Leptos SSR.

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

Structured logging via `tracing-subscriber`. Configure levels with `RUST_LOG`:

```bash
# Default (info level)
RUST_LOG=info

# Debug HTTP requests
RUST_LOG=dpe_server=info,tower_http=debug

# Verbose debugging
RUST_LOG=debug
```

## Observability

- **Fathom Analytics**: Set `DPE_FATHOM_SITE_ID` to enable privacy-friendly analytics. The tracking script is injected into the HTML shell. GDPR-compliant (no cookies).
