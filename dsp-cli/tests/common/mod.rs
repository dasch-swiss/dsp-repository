//! Environment-variable helpers shared by live integration tests (`tests/live_*.rs`).
//!
//! This module is compiled into every integration test binary that declares
//! `mod common;`, so a binary that only calls one of the two functions will
//! warn on the other — `#[allow(dead_code)]` on each item suppresses that
//! (mirrors `tests/support/mod.rs`, the existing precedent for a shared test
//! module in this crate).

use std::env;

/// Read a required environment variable. Returns `None` and emits a skip
/// message if the variable is absent or empty.
///
/// When `DSP_LIVE_STRICT=1` is set, an absent or empty variable panics naming
/// the missing variable instead of returning `None` — turning a silently
/// skipped live run into a loud failure when the caller asks for strictness.
#[allow(dead_code)]
pub fn require_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => {
            if env::var("DSP_LIVE_STRICT").as_deref() == Ok("1") {
                panic!("DSP_LIVE_STRICT=1: required environment variable {name} is not set");
            }
            eprintln!("skipping live test: {name} not set");
            None
        }
    }
}

/// Read an optional environment variable. Returns `None` silently if absent.
///
/// Unaffected by `DSP_LIVE_STRICT` — an optional variable is optional in both
/// modes.
#[allow(dead_code)]
pub fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}
