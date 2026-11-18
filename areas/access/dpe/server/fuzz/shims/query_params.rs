//! The `query_params` fuzz target. A panic aborts the process, which libFuzzer reports as a
//! crash.

use std::hint::black_box;

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

fn fuzz(data: &[u8]) {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    black_box(serde_urlencoded::from_str::<ProjectQuery>(input).ok());

    // Also as JSON, which is what a malformed Content-Type produces.
    black_box(serde_json::from_str::<ProjectQuery>(input).ok());

    black_box(serde_json::from_str::<shared_metadata::ProjectRaw>(input).ok());
}

/// The entry point `fuzz_target.cc` calls with each input.
///
/// # Safety
///
/// `data` points to `len` initialised bytes, or `len` is 0 (libFuzzer's `LLVMFuzzerTestOneInput`
/// contract).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpe_fuzz_entry(data: *const u8, len: usize) -> i32 {
    let data = if len == 0 {
        &[]
    } else {
        // SAFETY: the caller passes `len` initialised bytes at `data`.
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    fuzz(data);
    0
}
