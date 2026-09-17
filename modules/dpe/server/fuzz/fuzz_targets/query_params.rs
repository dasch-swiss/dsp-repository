#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

/// Mirrors the query parameters accepted by the projects listing page.
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

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = serde_urlencoded::from_str::<ProjectQuery>(input);

    // Also as JSON, which is what a malformed Content-Type produces.
    let _ = serde_json::from_str::<ProjectQuery>(input);

    let _ = serde_json::from_str::<platform_metadata::ProjectRaw>(input);
});
