#![no_main]

use dpe_telemetry::beacon::BeaconPayload;
use libfuzzer_sys::fuzz_target;

// Fuzz the telemetry beacon payload deserialization.
// Goal: ensure no panics when parsing arbitrary bytes as a beacon payload.
// This endpoint receives untrusted data from the internet via sendBeacon.
fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<BeaconPayload>(data);

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<BeaconPayload>(s);
    }
});
