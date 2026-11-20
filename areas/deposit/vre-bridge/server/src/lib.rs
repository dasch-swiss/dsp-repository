//! The VRE bridge's entry crate (`areas/deposit/ADR-0005`, role `server`): [`VreBridge`], which
//! `deposit-server` builds from its configuration and starts; the start-time recovery, the
//! nightly run's schedule, the nightly run and the operator's refresh; and one refresh of an
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

mod bridge;
mod clock;
mod nightly;
mod refresh;
mod report;
mod schedule;
mod vre;

pub use bridge::{Inactive, VreBridge, EXIT_LOCKED, EXIT_UNUSABLE};
pub use clock::{Clock, SystemClock};
pub use refresh::{sweep_dump_dirs, Busy, Refresher, SNAPSHOT_FILE};
pub use report::{LogReport, RefreshReport, RefreshSizes, Report};
pub use vre::Vre;
/// The settings as `deposit-server`'s loader reads them; `vre-bridge-core` is visible to this
/// crate alone.
pub use vre_bridge_core::{RawSettings, SigningKeys};

#[cfg(test)]
mod bridge_tests;
#[cfg(test)]
mod nightly_tests;
#[cfg(test)]
mod refresh_tests;
#[cfg(test)]
mod schedule_tests;
#[cfg(test)]
mod test_support;
