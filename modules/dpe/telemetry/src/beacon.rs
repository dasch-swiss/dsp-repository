use serde::Deserialize;

/// Valid vital names (bounded set for metric cardinality).
pub const VALID_VITAL_NAMES: &[&str] = &["LCP", "INP", "CLS", "TTFB", "FCP"];
/// Valid vital ratings (bounded set).
pub const VALID_RATINGS: &[&str] = &["good", "needs-improvement", "poor"];
/// Valid error kinds (bounded set).
pub const VALID_ERROR_KINDS: &[&str] = &["js_error", "promise_rejection", "resource_error", "datastar_sse"];

/// Top-level beacon payload sent by the browser telemetry module.
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct BeaconPayload {
    pub signals: Vec<Signal>,
    pub connection: Option<ConnectionInfo>,
}

/// A single telemetry signal from the browser.
/// Unknown signal types are silently skipped (forward compatibility).
#[derive(Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum Signal {
    #[serde(rename = "web_vital")]
    WebVital(WebVitalSignal),
    #[serde(rename = "error")]
    Error(ErrorSignal),
    #[serde(rename = "loaf")]
    LongAnimationFrame(LoafSignal),
    #[serde(rename = "navigation")]
    Navigation(NavigationSignal),
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct WebVitalSignal {
    pub name: String,
    pub value: f64,
    pub rating: String,
    #[serde(rename = "navigationType")]
    pub navigation_type: Option<String>,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    pub traceparent: Option<String>,
    #[serde(rename = "pageLoadId")]
    pub page_load_id: String,
    pub timestamp: u64,
    // LCP attribution
    #[serde(rename = "lcpElement")]
    pub lcp_element: Option<String>,
    #[serde(rename = "lcpUrl")]
    pub lcp_url: Option<String>,
    #[serde(rename = "timeToFirstByte")]
    pub time_to_first_byte: Option<f64>,
    #[serde(rename = "resourceLoadDelay")]
    pub resource_load_delay: Option<f64>,
    #[serde(rename = "resourceLoadDuration")]
    pub resource_load_duration: Option<f64>,
    #[serde(rename = "elementRenderDelay")]
    pub element_render_delay: Option<f64>,
    // INP attribution
    #[serde(rename = "inpTarget")]
    pub inp_target: Option<String>,
    #[serde(rename = "inpType")]
    pub inp_type: Option<String>,
    #[serde(rename = "inputDelay")]
    pub input_delay: Option<f64>,
    #[serde(rename = "processingDuration")]
    pub processing_duration: Option<f64>,
    #[serde(rename = "presentationDelay")]
    pub presentation_delay: Option<f64>,
    // CLS attribution
    #[serde(rename = "clsTarget")]
    pub cls_target: Option<String>,
    // TTFB attribution
    #[serde(rename = "dnsDuration")]
    pub dns_duration: Option<f64>,
    #[serde(rename = "connectionDuration")]
    pub connection_duration: Option<f64>,
    #[serde(rename = "requestDuration")]
    pub request_duration: Option<f64>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ErrorSignal {
    pub kind: String,
    pub message: String,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    pub traceparent: Option<String>,
    #[serde(rename = "pageLoadId")]
    pub page_load_id: String,
    pub timestamp: u64,
    pub filename: Option<String>,
    pub lineno: Option<u32>,
    pub colno: Option<u32>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct LoafSignal {
    pub duration: f64,
    #[serde(rename = "blockingDuration")]
    pub blocking_duration: f64,
    #[serde(rename = "firstScript")]
    pub first_script: Option<String>,
    #[serde(rename = "scriptCount")]
    pub script_count: u32,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    pub traceparent: Option<String>,
    #[serde(rename = "pageLoadId")]
    pub page_load_id: String,
    pub timestamp: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct NavigationSignal {
    pub dns: f64,
    pub tcp: f64,
    pub tls: f64,
    pub ttfb: f64,
    pub download: f64,
    #[serde(rename = "domParse")]
    pub dom_parse: f64,
    #[serde(rename = "domReady")]
    pub dom_ready: f64,
    #[serde(rename = "fullLoad")]
    pub full_load: f64,
    #[serde(rename = "transferSize")]
    pub transfer_size: f64,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    pub traceparent: Option<String>,
    #[serde(rename = "pageLoadId")]
    pub page_load_id: String,
    pub timestamp: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ConnectionInfo {
    #[serde(rename = "effectiveType")]
    pub effective_type: Option<String>,
    pub downlink: Option<f64>,
    pub rtt: Option<f64>,
    #[serde(rename = "saveData")]
    pub save_data: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
                "timestamp": 1700000000000_u64,
                "lcpElement": "img",
                "lcpUrl": "https://example.com/img.jpg",
                "timeToFirstByte": 100.0,
                "resourceLoadDelay": 50.0,
                "resourceLoadDuration": 200.0,
                "elementRenderDelay": 150.0
            }],
            "connection": {
                "effectiveType": "4g",
                "downlink": 10.0,
                "rtt": 50,
                "saveData": false
            }
        })
        .to_string()
    }

    #[test]
    fn deserialize_web_vital_with_all_attribution() {
        let payload: BeaconPayload = serde_json::from_str(&valid_beacon_json()).unwrap();
        assert_eq!(payload.signals.len(), 1);
        match &payload.signals[0] {
            Signal::WebVital(v) => {
                assert_eq!(v.name, "LCP");
                assert_eq!(v.lcp_element.as_deref(), Some("img"));
                assert!(v.time_to_first_byte.is_some());
            }
            _ => panic!("expected WebVital"),
        }
    }

    #[test]
    fn deserialize_unknown_signal_type() {
        let json = r#"{"signals":[{"type":"future_signal","data":"value"}],"connection":null}"#;
        let payload: BeaconPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(payload.signals[0], Signal::Unknown));
    }

    #[test]
    fn deserialize_error_signal() {
        let json = serde_json::json!({
            "signals": [{
                "type": "error",
                "kind": "js_error",
                "message": "ReferenceError: foo is not defined",
                "pageUrl": "/projects",
                "traceparent": null,
                "pageLoadId": "test-456",
                "timestamp": 1700000000000_u64,
                "filename": "telemetry.js",
                "lineno": 42,
                "colno": 10
            }],
            "connection": null
        })
        .to_string();
        let payload: BeaconPayload = serde_json::from_str(&json).unwrap();
        match &payload.signals[0] {
            Signal::Error(e) => {
                assert_eq!(e.kind, "js_error");
                assert_eq!(e.lineno, Some(42));
            }
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn deserialize_navigation_signal() {
        let json = serde_json::json!({
            "signals": [{
                "type": "navigation",
                "dns": 5.0, "tcp": 10.0, "tls": 15.0,
                "ttfb": 100.0, "download": 50.0,
                "domParse": 200.0, "domReady": 20.0,
                "fullLoad": 10.0, "transferSize": 50000.0,
                "pageUrl": "/projects",
                "traceparent": null,
                "pageLoadId": "test-789",
                "timestamp": 1700000000000_u64
            }],
            "connection": null
        })
        .to_string();
        let payload: BeaconPayload = serde_json::from_str(&json).unwrap();
        match &payload.signals[0] {
            Signal::Navigation(n) => {
                assert_eq!(n.dns, 5.0);
                assert_eq!(n.transfer_size, 50000.0);
            }
            _ => panic!("expected Navigation"),
        }
    }

    mod properties {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn deserialize_never_panics(s in "\\PC{0,500}") {
                let _ = serde_json::from_str::<BeaconPayload>(&s);
            }
        }
    }
}
