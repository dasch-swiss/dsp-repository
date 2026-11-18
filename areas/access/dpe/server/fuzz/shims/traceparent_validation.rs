//! `traceparent_validation` (body from `fuzz_targets/traceparent_validation.rs`). A panic aborts
//! the process, which libFuzzer reports as a crash.

use shared_telemetry::traceparent::is_valid_traceparent;

// Accepted values must conform to the W3C Trace Context format.
fn fuzz(data: &[u8]) {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let result = is_valid_traceparent(input);

    if result {
        assert_eq!(input.len(), 55);
        assert!(input.starts_with("00-"));
        assert_eq!(input.as_bytes()[35], b'-');
        assert_eq!(input.as_bytes()[52], b'-');
        assert_ne!(&input[3..35], "00000000000000000000000000000000");
        assert_ne!(&input[36..52], "0000000000000000");
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
