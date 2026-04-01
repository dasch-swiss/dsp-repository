#![no_main]

use dpe_core::project::{VALID_TABS, is_valid_shortcode};
use libfuzzer_sys::fuzz_target;

// Fuzz the tab name and shortcode validation logic.
// Goal: ensure no panics or undefined behavior when processing arbitrary
// shortcode + tab combinations.
fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // Split input into shortcode and tab parts (separated by first '/')
    let (shortcode, tab) = match input.find('/') {
        Some(pos) => (&input[..pos], &input[pos + 1..]),
        None => (input, ""),
    };

    // Exercise shortcode validation — must not panic on any input
    let _ = is_valid_shortcode(shortcode);

    // Exercise tab validation — must not panic on any input
    let _ = VALID_TABS.contains(&tab);

    // Exercise combined validation path (as the handler would)
    if is_valid_shortcode(shortcode) && VALID_TABS.contains(&tab) {
        // Valid input — exercise URL construction (as replaceState does)
        let url = format!("/projects/{}?tab={}", shortcode, tab);
        assert!(url.starts_with("/projects/"));
    }
});
