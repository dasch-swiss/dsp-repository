#![no_main]

use shared_telemetry::traceparent::is_valid_traceparent;
use libfuzzer_sys::fuzz_target;

// Accepted values must conform to the W3C Trace Context format.
fuzz_target!(|data: &[u8]| {
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
});
