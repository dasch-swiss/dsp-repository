//! The VRE bridge's lift: one project's dump as the interim snapshot that
//! `FORMAT.md` beside this crate specifies.

// Ported from dsp-incubator's dao-lift, which pre-dates these two (purely
// stylistic) lints: allowed crate-wide rather than rewriting ported code, so the
// diff against the source stays readable; `-D warnings` stays on for everything else.
#![allow(clippy::collapsible_if, clippy::too_many_arguments)]
// A lift failure is a `LiftError`: production code calls no `expect`, `unwrap`,
// `panic!` or `unreachable!`.
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unreachable)
)]

mod dump;
mod error;
mod model;
mod snapshot;
#[cfg(test)]
mod test_support;

pub use error::LiftError;
pub use snapshot::{Handling, Kind, Snapshot, Stats, ValueKind, ViolationReport, snapshot};
