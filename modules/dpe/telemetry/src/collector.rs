//! The `POST /telemetry/collect` endpoint: browser beacons in, OTel metrics and
//! structured logs out.
//!
//! Shared by every DaSCH service that renders the beacon script, so the beacon
//! contract has exactly one implementation. The only per-service value is the
//! OTel instrumentation scope, set once at startup with [`set_meter_namespace`].

use std::sync::{LazyLock, OnceLock};

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use opentelemetry::{global, KeyValue};
use url::Url;

use crate::beacon::{BeaconPayload, Signal, VALID_ERROR_KINDS, VALID_RATINGS, VALID_VITAL_NAMES};
use crate::origin::is_allowed_origin;
use crate::page_url::normalize_page_url;
use crate::traceparent::validated_traceparent;

/// Extract the host from an Origin or Referer URL.
fn extract_host(value: &str) -> Option<String> {
    Url::parse(value).ok().and_then(|u| u.host_str().map(String::from))
}

// --- OTel metrics ---

/// The resolved OTel instrumentation scope name, e.g. `dpe.browser`.
static METER_SCOPE: OnceLock<String> = OnceLock::new();

/// Scope name for a service namespace. `dpe` → `dpe.browser`.
///
/// A named function rather than an inline `format!` so the mapping DPE's
/// dashboards depend on is pinned by a test.
fn meter_scope_name(namespace: &str) -> String {
    format!("{namespace}.browser")
}

/// Scope name used if the metrics are built without any route ever being
/// declared. Unreachable in either binary — [`collect_route`] is the only way to
/// wire the collector, and it always sets the scope — but a sane name rather
/// than the `.browser` that an empty namespace would produce.
const FALLBACK_METER_SCOPE: &str = "browser";

/// The `POST /telemetry/collect` route, for the service identified by
/// `namespace`.
///
/// The namespace is an argument here rather than a separate startup call
/// precisely so it cannot be forgotten: the scope name is what a dashboard
/// filters on as `otel_scope_name`, DPE has published `dpe.browser` since the
/// beacon shipped, and a missed init would silently rename it. Taking it at the
/// one place the route can be declared makes that a compile error instead.
///
/// Layer as usual — the return type is a `MethodRouter`, so
/// `collect_route("dpe").layer(rate_limiter)` still works.
pub fn collect_route<S>(namespace: &str) -> axum::routing::MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Ignore a second call rather than panicking: a double-init is not worth
    // taking a service down for, and both callers pass a constant.
    let _ = METER_SCOPE.set(meter_scope_name(namespace));
    axum::routing::post(collect_handler)
}

struct BrowserMetrics {
    web_vital: opentelemetry::metrics::Histogram<f64>,
    navigation_timing: opentelemetry::metrics::Histogram<f64>,
    loaf_duration: opentelemetry::metrics::Histogram<f64>,
    loaf_blocking: opentelemetry::metrics::Histogram<f64>,
    page_transfer_size: opentelemetry::metrics::Histogram<f64>,
    error_count: opentelemetry::metrics::Counter<u64>,
}

static BROWSER_METRICS: LazyLock<BrowserMetrics> = LazyLock::new(|| {
    let scope_name = METER_SCOPE
        .get()
        .map(String::as_str)
        .unwrap_or(FALLBACK_METER_SCOPE)
        .to_string();
    // `global::meter` takes a `&'static str`, which a name assembled at startup
    // cannot be; `meter_with_scope` accepts an owned name and produces the same
    // instrumentation scope, so the exported `otel_scope_name` is unchanged.
    let scope = opentelemetry::InstrumentationScope::builder(scope_name).build();
    let meter = global::meter_with_scope(scope);
    BrowserMetrics {
        web_vital: meter
            .f64_histogram("browser.web_vital")
            .with_description("Core Web Vitals from real users")
            .build(),
        navigation_timing: meter
            .f64_histogram("browser.navigation_timing")
            .with_description("Navigation timing breakdown")
            .with_unit("ms")
            .build(),
        loaf_duration: meter
            .f64_histogram("browser.long_animation_frame.duration")
            .with_description("Long Animation Frame total duration")
            .with_unit("ms")
            .build(),
        loaf_blocking: meter
            .f64_histogram("browser.long_animation_frame.blocking")
            .with_description("Long Animation Frame blocking duration")
            .with_unit("ms")
            .build(),
        page_transfer_size: meter
            .f64_histogram("browser.page.transfer_size")
            .with_description("Page transfer size in bytes")
            .with_unit("By")
            .build(),
        error_count: meter
            .u64_counter("browser.error")
            .with_description("Browser errors by kind")
            .build(),
    }
});

// --- Collector endpoint ---

/// Maximum beacon payload size (16 KiB).
const MAX_PAYLOAD_SIZE: usize = 16 * 1024;

/// Maximum signals per beacon to limit processing cost.
const MAX_SIGNALS: usize = 50;

/// POST /telemetry/collect
///
/// Receives browser telemetry beacons and converts them to OTel signals.
/// Always returns 204 (or 413 if payload too large) — never blocks on failures.
pub async fn collect_handler(headers: HeaderMap, body: Bytes) -> StatusCode {
    // REQ-4.7: Reject oversized payloads
    if body.len() > MAX_PAYLOAD_SIZE {
        return StatusCode::PAYLOAD_TOO_LARGE;
    }

    // Origin validation: reject cross-origin requests.
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());

    let is_same_origin = match origin.or(referer) {
        Some(value) => {
            if let Some(host) = extract_host(value) {
                is_allowed_origin(&host)
            } else {
                false
            }
        }
        None => false,
    };

    if !is_same_origin {
        tracing::debug!(origin = ?origin, referer = ?referer, "rejected telemetry beacon");
        return StatusCode::NO_CONTENT;
    }

    // Parse body as JSON regardless of Content-Type.
    let payload: BeaconPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(error = %e, "malformed telemetry beacon, ignoring");
            return StatusCode::NO_CONTENT;
        }
    };

    // Limit signal count to prevent CPU abuse
    let signals = if payload.signals.len() > MAX_SIGNALS {
        &payload.signals[..MAX_SIGNALS]
    } else {
        &payload.signals
    };

    for signal in signals {
        process_signal(signal);
    }

    StatusCode::NO_CONTENT
}

fn process_signal(signal: &Signal) {
    match signal {
        Signal::WebVital(v) => {
            let vital_name = if VALID_VITAL_NAMES.contains(&v.name.as_str()) {
                v.name.as_str()
            } else {
                return;
            };
            let rating = if VALID_RATINGS.contains(&v.rating.as_str()) {
                v.rating.as_str()
            } else {
                "unknown"
            };
            let page_url = normalize_page_url(&v.page_url);
            let trace_parent = validated_traceparent(&v.traceparent);

            BROWSER_METRICS.web_vital.record(
                v.value,
                &[
                    KeyValue::new("vital.name", vital_name.to_string()),
                    KeyValue::new("vital.rating", rating.to_string()),
                    KeyValue::new("page.url", page_url),
                ],
            );

            tracing::info!(
                vital_name,
                value = v.value,
                rating,
                navigation_type = ?v.navigation_type,
                page_url,
                page_load_id = %v.page_load_id,
                trace_parent = ?trace_parent,
                lcp_element = ?v.lcp_element,
                lcp_url = ?v.lcp_url,
                time_to_first_byte = ?v.time_to_first_byte,
                resource_load_delay = ?v.resource_load_delay,
                resource_load_duration = ?v.resource_load_duration,
                element_render_delay = ?v.element_render_delay,
                inp_target = ?v.inp_target,
                inp_type = ?v.inp_type,
                input_delay = ?v.input_delay,
                processing_duration = ?v.processing_duration,
                presentation_delay = ?v.presentation_delay,
                cls_target = ?v.cls_target,
                dns_duration = ?v.dns_duration,
                connection_duration = ?v.connection_duration,
                request_duration = ?v.request_duration,
                "browser web vital"
            );
        }
        Signal::Error(e) => {
            let error_kind = if VALID_ERROR_KINDS.contains(&e.kind.as_str()) {
                e.kind.as_str()
            } else {
                "unknown"
            };
            let page_url = normalize_page_url(&e.page_url);
            let trace_parent = validated_traceparent(&e.traceparent);

            BROWSER_METRICS.error_count.add(
                1,
                &[
                    KeyValue::new("error.kind", error_kind.to_string()),
                    KeyValue::new("page.url", page_url),
                ],
            );

            let truncated_message: String = e.message.chars().take(256).collect();
            tracing::warn!(
                kind = error_kind,
                page_url,
                trace_parent = ?trace_parent,
                page_load_id = %e.page_load_id,
                filename = ?e.filename,
                lineno = ?e.lineno,
                colno = ?e.colno,
                "browser error: {}",
                truncated_message
            );
        }
        Signal::LongAnimationFrame(loaf) => {
            let page_url = normalize_page_url(&loaf.page_url);
            let trace_parent = validated_traceparent(&loaf.traceparent);

            let attrs = [KeyValue::new("page.url", page_url)];
            BROWSER_METRICS.loaf_duration.record(loaf.duration, &attrs);
            BROWSER_METRICS.loaf_blocking.record(loaf.blocking_duration, &attrs);

            tracing::info!(
                duration = loaf.duration,
                blocking_duration = loaf.blocking_duration,
                first_script = ?loaf.first_script,
                script_count = loaf.script_count,
                page_url,
                page_load_id = %loaf.page_load_id,
                trace_parent = ?trace_parent,
                "browser long animation frame"
            );
        }
        Signal::Navigation(nav) => {
            let page_url = normalize_page_url(&nav.page_url);

            let phases: [(&str, f64); 8] = [
                ("dns", nav.dns),
                ("tcp", nav.tcp),
                ("tls", nav.tls),
                ("ttfb", nav.ttfb),
                ("download", nav.download),
                ("domParse", nav.dom_parse),
                ("domReady", nav.dom_ready),
                ("fullLoad", nav.full_load),
            ];
            let page_url_kv = KeyValue::new("page.url", page_url);
            for (phase, value) in phases {
                BROWSER_METRICS
                    .navigation_timing
                    .record(value, &[KeyValue::new("nav.phase", phase), page_url_kv.clone()]);
            }

            BROWSER_METRICS.page_transfer_size.record(nav.transfer_size, &[page_url_kv]);

            let trace_parent = validated_traceparent(&nav.traceparent);
            tracing::info!(
                transfer_size = nav.transfer_size,
                page_url,
                page_load_id = %nav.page_load_id,
                trace_parent = ?trace_parent,
                dns = nav.dns, tcp = nav.tcp, tls = nav.tls,
                ttfb = nav.ttfb, download = nav.download,
                dom_parse = nav.dom_parse, dom_ready = nav.dom_ready,
                full_load = nav.full_load,
                "browser navigation timing"
            );
        }
        Signal::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;

    use super::*;
    use crate::beacon::{ErrorSignal, WebVitalSignal};

    fn test_app() -> Router {
        Router::new().route("/telemetry/collect", post(collect_handler))
    }

    #[tokio::test]
    async fn collect_route_serves_the_collector() {
        // The route helper is the only sanctioned way to wire the collector, so
        // it has to behave identically to the bare handler.
        let app: Router = Router::new().route("/telemetry/collect", collect_route("test"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dasch.swiss")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn meter_scope_name_is_the_name_the_dashboards_filter_on() {
        // `dpe.browser` has been the exported `otel_scope_name` since the beacon
        // shipped. Changing this mapping renames every existing DPE browser
        // metric series, so it is pinned here rather than left to a format string.
        assert_eq!(meter_scope_name("dpe"), "dpe.browser");
        assert_eq!(meter_scope_name("editor"), "editor.browser");
    }

    #[test]
    fn fallback_scope_is_a_usable_name() {
        // Reached only if the metrics are built without any route being declared.
        // It must be a plain name, not the `.browser` that an empty namespace
        // would yield through meter_scope_name.
        assert!(!FALLBACK_METER_SCOPE.is_empty());
        assert!(!FALLBACK_METER_SCOPE.starts_with('.'));
        assert_ne!(FALLBACK_METER_SCOPE, meter_scope_name(""));
    }

    fn valid_beacon_json() -> String {
        serde_json::json!({
            "signals": [{
                "type": "web_vital",
                "name": "LCP",
                "value": 2500.0,
                "rating": "good",
                "navigationType": "navigate",
                "pageUrl": "/dpe/projects",
                "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                "pageLoadId": "test-123",
                "timestamp": 1700000000000_u64
            }],
            "connection": null
        })
        .to_string()
    }

    // --- Handler integration tests ---

    #[tokio::test]
    async fn handler_oversized_payload_returns_413() {
        let app = test_app();
        let body = vec![b'x'; MAX_PAYLOAD_SIZE + 1];
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dasch.swiss")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn handler_malformed_json_returns_204() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dasch.swiss")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_valid_beacon_returns_204() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dasch.swiss")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_text_plain_content_type_returns_204() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dasch.swiss")
                    .header("content-type", "text/plain")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_no_origin_returns_204() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Silently rejected (no Origin/Referer)
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_foreign_origin_returns_204() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://evil-dasch.swiss.attacker.com")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_dasch_subdomain_accepted() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://repository.dev.dasch.swiss")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn handler_localhost_accepted() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "http://localhost:4000")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // --- Validation tests ---

    #[test]
    fn unknown_vital_name_is_silently_dropped() {
        // process_signal should not panic
        let signal = Signal::WebVital(WebVitalSignal {
            name: "UNKNOWN_VITAL".to_string(),
            value: 1.0,
            rating: "good".to_string(),
            navigation_type: None,
            page_url: "/".to_string(),
            traceparent: None,
            page_load_id: "test".to_string(),
            timestamp: 0,
            lcp_element: None,
            lcp_url: None,
            time_to_first_byte: None,
            resource_load_delay: None,
            resource_load_duration: None,
            element_render_delay: None,
            inp_target: None,
            inp_type: None,
            input_delay: None,
            processing_duration: None,
            presentation_delay: None,
            cls_target: None,
            dns_duration: None,
            connection_duration: None,
            request_duration: None,
        });
        process_signal(&signal); // should not panic
    }

    #[test]
    fn unknown_error_kind_normalized_to_unknown() {
        let signal = Signal::Error(ErrorSignal {
            kind: "alien_error".to_string(),
            message: "something weird".to_string(),
            page_url: "/".to_string(),
            traceparent: None,
            page_load_id: "test".to_string(),
            timestamp: 0,
            filename: None,
            lineno: None,
            colno: None,
        });
        process_signal(&signal); // should not panic, uses "unknown" kind
    }

    // Origin property tests live in dpe_telemetry::origin::tests

    #[tokio::test]
    async fn handler_lookalike_domain_rejected() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry/collect")
                    .header("origin", "https://evil-dasch.swiss")
                    .body(Body::from(valid_beacon_json()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn extract_host_parses_urls() {
        assert_eq!(
            extract_host("https://repository.dasch.swiss"),
            Some("repository.dasch.swiss".to_string())
        );
        assert_eq!(extract_host("http://localhost:4000"), Some("localhost".to_string()));
        assert_eq!(extract_host("not-a-url"), None);
    }
}
