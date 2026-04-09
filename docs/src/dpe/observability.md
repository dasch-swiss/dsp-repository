# DPE Observability

Developer guide for working with DPE's observability instrumentation.

## Overview

DPE uses [OpenTelemetry](https://opentelemetry.io/) for distributed tracing, metrics, and structured logging. The telemetry pipeline has two halves:

- **Server-side**: OTel-native tracing via `axum-tracing-opentelemetry` middleware. Every HTTP request (except `/healthz`) produces W3C-compliant spans exported via OTLP.
- **Client-side**: A lightweight JavaScript module (`telemetry.js`) captures Core Web Vitals, JS errors, Long Animation Frames, and navigation timing. Signals are sent via `navigator.sendBeacon` to a server-side collector (`POST /telemetry/collect`), which converts them into OTel metrics and structured logs flowing through the same OTLP pipeline.

Trace correlation between server and client uses the W3C `traceparent` standard: the server renders a `<meta name="traceparent">` tag in the HTML shell, and the client includes it in every beacon payload.

## Local Observability Stack

Run the Grafana LGTM (Loki, Grafana, Tempo, Mimir) all-in-one container:

```bash
# Terminal 1: Start local LGTM stack
docker run --rm -p 3000:3000 -p 4317:4317 -p 4318:4318 -p 4040:4040 grafana/otel-lgtm

# Terminal 2: Run DPE with OTel enabled
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
OTEL_SERVICE_NAME=dpe \
OTEL_RESOURCE_ATTRIBUTES="service.namespace=dpe,service.version=0.2.1,deployment.environment=dev" \
PYROSCOPE_ENDPOINT=http://localhost:4040 \
just watch-dpe

# Terminal 3: Generate traffic
curl http://localhost:4000/projects
curl http://localhost:4000/oai?verb=Identify
curl http://localhost:4000/healthz
```

## Navigating Grafana Locally

Open <http://localhost:3000> (no login required):

- **Tempo** (Explore → Tempo): traces for `/projects` and `/oai`, none for `/healthz`
- **Service map**: "dpe" service with Rust tech icon
- **Loki** (Explore → Loki): OTel log records bridged from the `tracing` subscriber (severity, span context, structured fields)
- **Mimir** (Explore → Mimir): browser telemetry metrics (`browser.web_vital`, `browser.error`, etc.)
- **Pyroscope** (Explore → Pyroscope): CPU flame graphs for `dpe-server`

## Adding Instrumentation

Use `#[tracing::instrument]` on new handler and service functions:

```rust
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "descriptive name",
    )
)]
pub async fn my_handler(/* ... */) -> /* ... */ {
    // ...
}
```

- Use `otel.kind = "internal"` on handler-level spans — the OTel middleware (`OtelAxumLayer`) already creates the `SPAN_KIND_SERVER` span for the HTTP request.
- Do not create nested `"server"` spans — that confuses the trace waterfall.

## Client Telemetry

The `telemetry.js` module (served from `/telemetry.js`) captures:

- **Core Web Vitals** (LCP, INP, CLS, TTFB, FCP) with attribution data
- **JavaScript errors** and unhandled promise rejections
- **Datastar SSE errors**
- **Long Animation Frames** (LoAF, ≥200ms threshold)
- **Navigation timing** breakdown

All signals are buffered and flushed via `navigator.sendBeacon` on `visibilitychange` (page hide). The server collector converts them to OTel metrics (bounded attributes only) and structured logs (high-cardinality attribution data).

## Logging

- **Production** (`LEPTOS_ENV=PROD`): JSON-formatted logs to stdout only. No OTel log export — traces and metrics are exported via OTLP, but logs stay on stdout.
- **Local development** (`LEPTOS_ENV=DEV` with `OTEL_EXPORTER_OTLP_ENDPOINT` set): Logs go to both stdout and Loki via OTLP. An `OpenTelemetryTracingBridge` layer converts `tracing` events into OTel log records, which are batched and exported alongside traces and metrics. Query them in Grafana Explore → Loki.
- Set `RUST_LOG` to control log levels. Use `RUST_LOG=debug` for verbose output.
- When `OTEL_EXPORTER_OTLP_ENDPOINT` is not set, the OTel SDK falls back to no-op export — no traces, metrics, or logs are sent, but structured stdout logging still works.
