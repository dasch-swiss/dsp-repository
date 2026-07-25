#![no_main]

use dpe_telemetry::origin::is_allowed_origin;
use libfuzzer_sys::fuzz_target;

// Fuzz the origin validation function used by the telemetry collector.
// Goal: ensure no panics, and that only properly-structured dasch.swiss
// subdomains and localhost are accepted.
fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let result = is_allowed_origin(input);

    if result {
        let is_exact = input == "dasch.swiss";
        let is_subdomain = input.ends_with(".dasch.swiss") && input.len() > 12;
        let is_localhost = input == "localhost";
        assert!(
            is_exact || is_subdomain || is_localhost,
            "accepted unexpected origin: {input}"
        );

        if !is_exact && !is_localhost {
            let prefix_end = input.len() - "dasch.swiss".len();
            assert!(
                prefix_end > 0 && input.as_bytes()[prefix_end - 1] == b'.',
                "accepted origin without dot separator: {input}"
            );
        }
    }
});
