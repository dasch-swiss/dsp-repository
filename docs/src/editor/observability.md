# Editor Observability

Developer guide for the metadata editor's observability instrumentation. The pipeline is the same shape as [DPE's](../dpe/observability.md); this page records what differs.

## Overview

Two halves, as in DPE:

- **Server-side**: OTel-native tracing via `axum-tracing-opentelemetry` middleware. Every HTTP request except `/healthz` and `/telemetry/collect` produces W3C-compliant spans exported over OTLP.
- **Client-side**: `telemetry.js` captures Core Web Vitals, JS errors, Long Animation Frames and navigation timing, and flushes them via `navigator.sendBeacon` to `POST /telemetry/collect`, which converts them into OTel metrics and structured logs on the same OTLP pipeline.

Trace correlation uses the W3C `traceparent` standard: the server renders a `<meta name="traceparent">` tag in the HTML shell and the client includes it in every beacon payload.

There is **no third-party analytics script**. DPE injects Fathom; the editor is an authenticated tool and does not.

## The shared collector

The beacon endpoint is not editor code. `platform-telemetry` owns the wire contract *and* the collector, and both services wire the same handler:

```rust
.route(
    "/telemetry/collect",
    platform_telemetry::collector::collect_route("editor", page_url::normalize_page_url).layer(rate_limit),
)
```

The `namespace` argument sets the OTel instrumentation scope — `editor.browser` here, `dpe.browser` in DPE. It is a required argument rather than a separate startup call precisely so it cannot be forgotten: the scope name is what a dashboard filters on as `otel_scope_name`, and a missed init would silently rename it.

`page_url::normalize_page_url` is not shared code either: a platform crate cannot hold one service's route table, so the editor owns its own `page_url.rs` (`server/src/page_url.rs`) and passes the fn in. New full-page routes need a matching entry there — see `REVIEW.md`.

So a Grafana query distinguishes the two services by `otel_scope_name` (`editor.browser` vs `dpe.browser`) or by the resource attribute `service.namespace` (`editor` vs `dpe`). Metric *names* (`browser.web_vital`, `browser.error`, …) are identical across both.

The `dpe-` crate prefix is historical — DPE was the first consumer. Nothing in the crate reads DPE's data or configuration.

## Local observability stack

```bash
# Terminal 1: start the local LGTM stack
just lgtm-up

# Terminal 2: run the editor with OTel enabled (exports to localhost:4317)
just dev-editor-otel

# Terminal 3: generate traffic
curl http://localhost:4100/healthz
curl http://localhost:4100/no-such-page
```

`dev-editor-otel` sets `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`, `OTEL_RESOURCE_ATTRIBUTES` and `PYROSCOPE_ENDPOINT` for you.

In Grafana at <http://localhost:3000>: traces for page routes in Tempo and **none for `/healthz`**, log records in Loki, browser metrics under `editor.browser` in Mimir, and CPU flame graphs for `editor-server` in Pyroscope.

## Adding instrumentation

Use `#[tracing::instrument]` on new handler and service functions, with `otel.kind = "internal"` — the `OtelAxumLayer` middleware already creates the `SPAN_KIND_SERVER` span for the request, and a nested `"server"` span confuses the trace waterfall.

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

## Rules specific to an authenticated service

DPE serves public data, so almost nothing it logs is sensitive. The editor holds accounts, so two constraints apply that DPE's guide does not carry:

- **No email address may reach a log or a trace, on any path — including error paths.** Auth events are correlated by an opaque per-user id instead. This is a hard requirement, and the test that enforces it lands with authentication — there are no addresses in the service yet, so there is nothing to assert today. Note there is a second capture channel besides explicit logging: `axum-tracing-opentelemetry` records `url.query` on every server span, so a query string is exported to Tempo verbatim. Anything address-bearing must therefore stay out of query strings as well as out of log fields.
- **Metric attributes must stay bounded.** Validate against known sets and normalise dynamic values; a project shortcode is bounded, a user identifier or a free-text field is not. High-cardinality data goes to structured logs only.

## Logging

- **Production** (`EDITOR_ENV=PROD`): JSON logs to stdout only.
- **Local development** (`EDITOR_ENV=DEV` with `OTEL_EXPORTER_OTLP_ENDPOINT` set): also exported over OTLP, via an `OpenTelemetryTracingBridge` layer that converts `tracing` events into OTel log records.
- With `OTEL_EXPORTER_OTLP_ENDPOINT` unset the OTel SDK falls back to no-op export — nothing is sent, but structured stdout logging still works, so local development needs no collector.

Panics go through the same pipeline: a hook emits them as structured `tracing::error!` events using the OTel exception semantic conventions, so they share Grafana Sift and Loki query surface with `RecordException` events from instrumented spans. The default stderr hook fires only if the structured emission itself panics, so a panic is never silently swallowed and never double-reported.

On shutdown the Pyroscope agent is stopped and the OTel providers flushed inside `spawn_blocking`: both do blocking I/O (condvar waits, thread joins, HTTP uploads), and doing them on the async runtime deadlocks it. Logs are flushed before the trace guard drops, because log records can reference trace context that becomes invalid afterwards.
