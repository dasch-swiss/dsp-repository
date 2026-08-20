# Platform

Crates shared by more than one service. A crate lands here as soon as a second service depends on it, and is named `platform-{role}`; see `docs/src/repo_structure.md` → *Shared Crates* for the rule and why the directory, not just the crate name, is what matters.

```txt
platform/
└── telemetry/         # Browser beacon contract + collector endpoint (crate: platform-telemetry)
```

Platform crates depend on no service crate. Anything that needs to know about one service's routes, data or configuration takes it as a parameter instead of reaching for it.

## `platform-telemetry` (telemetry/)

The browser telemetry contract and the endpoint that consumes it. A library crate so fuzz targets can test the real code, and so the beacon has one implementation across `dpe-server` and `editor-server` rather than a fork per service. Contains:

- **Beacon types**: `BeaconPayload`, `Signal`, `WebVitalSignal`, `ErrorSignal`, etc. (serde deserialization for browser beacons)
- **Origin validation**: `is_allowed_origin()` — validates dasch.swiss subdomains
- **Traceparent validation**: `is_valid_traceparent()` — W3C traceparent format validation
- **Collector endpoint**: `collector::collect_route(namespace, normalize_page_url)` — converts beacons to OTel metrics and structured logs. `namespace` sets the instrumentation scope (`dpe.browser`, `editor.browser`) and is a required argument so it cannot be omitted and silently rename the scope a dashboard filters on. `normalize_page_url` bounds the `page.url` metric attribute to a known set of routes

The contract modules depend on `serde` only; `collector` additionally pulls in `axum`, `opentelemetry`, `tracing` and `url`.

Page-URL normalization is **not** in this crate: a platform crate depends on no service crate, and a route table is exactly one service's data. `dpe-server` and `editor-server` each own a `page_url.rs` and pass its `normalize_page_url` fn into `collect_route`.
