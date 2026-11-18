//! The `beacon_payload` fuzz target. A panic aborts the process, which libFuzzer reports as a
//! crash.

use std::hint::black_box;

use shared_telemetry::beacon::BeaconPayload;

// The beacon endpoint receives untrusted data from the internet via sendBeacon.
fn fuzz(data: &[u8]) {
    black_box(serde_json::from_slice::<BeaconPayload>(data).ok());

    if let Ok(s) = std::str::from_utf8(data) {
        black_box(serde_json::from_str::<BeaconPayload>(s).ok());
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
