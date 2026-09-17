#![no_main]

use dpe_core::project::VALID_TABS;
use platform_metadata::project::is_valid_shortcode;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let (shortcode, tab) = match input.find('/') {
        Some(pos) => (&input[..pos], &input[pos + 1..]),
        None => (input, ""),
    };

    let _ = is_valid_shortcode(shortcode);

    let _ = VALID_TABS.contains(&tab);

    if is_valid_shortcode(shortcode) && VALID_TABS.contains(&tab) {
        // The URL the SSE handler's replaceState builds.
        let url = format!("/dpe/projects/{}?tab={}", shortcode, tab);
        assert!(url.starts_with("/dpe/projects/"));
    }
});
