use std::sync::LazyLock;

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use dpe_telemetry::beacon::{BeaconPayload, Signal, VALID_ERROR_KINDS, VALID_RATINGS, VALID_VITAL_NAMES};
use dpe_telemetry::origin::is_allowed_origin;
use dpe_telemetry::page_url::normalize_page_url;
use dpe_telemetry::traceparent::validated_traceparent;
use opentelemetry::{global, KeyValue};
use url::Url;

/// Extract the host from an Origin or Referer URL.
fn extract_host(value: &str) -> Option<String> {
    Url::parse(value).ok().and_then(|u| u.host_str().map(String::from))
}

// --- OTel metrics ---

struct BrowserMetrics {
    web_vital: opentelemetry::metrics::Histogram<f64>,
    navigation_timing: opentelemetry::metrics::Histogram<f64>,
    loaf_duration: opentelemetry::metrics::Histogram<f64>,
    loaf_blocking: opentelemetry::metrics::Histogram<f64>,
    page_transfer_size: opentelemetry::metrics::Histogram<f64>,
    error_count: opentelemetry::metrics::Counter<u64>,
}

static BROWSER_METRICS: LazyLock<BrowserMetrics> = LazyLock::new(|| {
    let meter = global::meter("dpe.browser");
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
                message = %truncated_message,
                page_url,
                trace_parent = ?trace_parent,
                page_load_id = %e.page_load_id,
                filename = ?e.filename,
                lineno = ?e.lineno,
                colno = ?e.colno,
                "browser error"
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
    use dpe_telemetry::beacon::{ErrorSignal, WebVitalSignal};
    use tower::ServiceExt;

    use super::*;

    fn test_app() -> Router {
        Router::new().route("/telemetry/collect", post(collect_handler))
    }

    fn valid_beacon_json() -> String {
        serde_json::json!({
            "signals": [{
                "type": "web_vital",
                "name": "LCP",
                "value": 2500.0,
                "rating": "good",
                "navigationType": "navigate",
                "pageUrl": "/projects",
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
