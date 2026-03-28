#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

/// Mirrors the query parameters accepted by the projects listing page.
/// Fuzzing deserialization catches panics in serde, unexpected enum values,
/// and edge cases in optional field handling.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProjectQuery {
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    page_size: Option<usize>,
    #[serde(default)]
    ongoing: Option<bool>,
    #[serde(default)]
    finished: Option<bool>,
    #[serde(default)]
    type_of_data: Option<String>,
    #[serde(default)]
    data_language: Option<String>,
    #[serde(default)]
    access_rights: Option<String>,
    #[serde(default)]
    tab: Option<String>,
}

// Fuzz query parameter parsing for the project listing and tab endpoints.
// Goal: ensure no panics when parsing arbitrary query strings.
fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // Try parsing as a query string (key=value&key=value format)
    let _ = serde_urlencoded::from_str::<ProjectQuery>(input);

    // Also try parsing as JSON (in case of malformed Content-Type)
    let _ = serde_json::from_str::<ProjectQuery>(input);

    // Fuzz the raw ProjectRaw JSON deserialization from dpe-core
    let _ = serde_json::from_str::<dpe_core::ProjectRaw>(input);
});
