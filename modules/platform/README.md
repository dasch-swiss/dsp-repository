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
- **URL normalization**: `normalize_page_url()` — cardinality-safe page URL mapping
- **Traceparent validation**: `is_valid_traceparent()` — W3C traceparent format validation
- **Collector endpoint**: `collector::collect_route(namespace)` — converts beacons to OTel metrics and structured logs. `namespace` sets the instrumentation scope (`dpe.browser`, `editor.browser`) and is a required argument so it cannot be omitted and silently rename the scope a dashboard filters on

The contract modules depend on `serde` only; `collector` additionally pulls in `axum`, `opentelemetry`, `tracing` and `url`.

`normalize_page_url()` is the one part not yet service-neutral: its route table is DPE's, so it does not normalize the editor's paths. Tracked as [DEV-6977](https://linear.app/dasch/issue/DEV-6977).
