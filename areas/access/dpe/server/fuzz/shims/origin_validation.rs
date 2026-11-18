//! The `origin_validation` fuzz target. A panic aborts the process, which libFuzzer reports as a
//! crash.

use shared_telemetry::origin::is_allowed_origin;

// Only properly-structured dasch.swiss subdomains and localhost may be accepted.
fn fuzz(data: &[u8]) {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let result = is_allowed_origin(input);

    if result {
        let is_exact = input == "dasch.swiss";
        let is_subdomain = input.ends_with(".dasch.swiss") && input.len() > 12;
        let is_localhost = input == "localhost";
        assert!(is_exact || is_subdomain || is_localhost, "accepted unexpected origin: {input}");

        if !is_exact && !is_localhost {
            let prefix_end = input.len() - "dasch.swiss".len();
            assert!(
                prefix_end > 0 && input.as_bytes()[prefix_end - 1] == b'.',
                "accepted origin without dot separator: {input}"
            );
        }
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
