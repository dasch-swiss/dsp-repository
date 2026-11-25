//! The `query_params` fuzz target. A panic aborts the process, which libFuzzer reports as a
//! crash.

use std::hint::black_box;

fn fuzz(data: &[u8]) {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let query = dpe_web::domain::ProjectQuery::from_query_str(input);
    black_box(query.to_query_string());

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
