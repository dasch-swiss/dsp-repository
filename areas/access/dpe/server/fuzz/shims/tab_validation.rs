//! `tab_validation` (body from `fuzz_targets/tab_validation.rs`). A panic aborts the process,
//! which libFuzzer reports as a crash.

use std::hint::black_box;

use dpe_core::project::VALID_TABS;
use shared_metadata::project::is_valid_shortcode;

fn fuzz(data: &[u8]) {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let (shortcode, tab) = match input.find('/') {
        Some(pos) => (&input[..pos], &input[pos + 1..]),
        None => (input, ""),
    };

    black_box(is_valid_shortcode(shortcode));

    black_box(VALID_TABS.contains(&tab));

    if is_valid_shortcode(shortcode) && VALID_TABS.contains(&tab) {
        // The URL the SSE handler's replaceState builds.
        let url = format!("/dpe/projects/{}?tab={}", shortcode, tab);
        assert!(url.starts_with("/dpe/projects/"));
    }
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
