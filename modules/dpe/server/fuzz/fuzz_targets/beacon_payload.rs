#![no_main]

use platform_telemetry::beacon::BeaconPayload;
use libfuzzer_sys::fuzz_target;

// The beacon endpoint receives untrusted data from the internet via sendBeacon.
fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<BeaconPayload>(data);

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<BeaconPayload>(s);
    }
});
