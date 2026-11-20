//! The VRE bridge's entry crate (`areas/deposit/ADR-0005`, role `server`): one refresh of an
//! allowlisted project, from the change check to both subtrees' Ready tokens.
//!
//! Every `vre-bridge-store` call, the unpacking, the lift, the shipped-input digest and the
//! removal of the private work directory block, so they run in `spawn_blocking` with owned
//! arguments; a `GenerationWriter` never lives across an `.await`. The network goes through
//! [`Vre`], the outcome through [`Report`], the time through [`Clock`], so tests replace each.

#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unreachable)
)]

mod clock;
mod refresh;
mod report;
mod vre;

pub use clock::{Clock, SystemClock};
pub use refresh::{sweep_dump_dirs, Busy, Refresher, SNAPSHOT_FILE};
pub use report::{LogReport, RefreshReport, RefreshSizes, Report};
pub use vre::Vre;

#[cfg(test)]
mod refresh_tests;
