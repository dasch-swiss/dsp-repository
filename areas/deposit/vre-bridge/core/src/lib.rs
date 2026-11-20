//! The VRE bridge's pure logic: its settings, parsed from the strings `deposit-server`'s loader
//! hands over; the allowlist and server ids; whether a project is refreshed and from which cutoff;
//! the digest of a project's shipped inputs; and the outcome of a refresh.
//!
//! No network and no tokio. The only file-system access is [`inputs_digest`], which reads the
//! shipped inputs and is blocking, so an async caller runs it in `spawn_blocking`. Nothing here
//! writes anything.

mod decide;
mod digest;
mod outcome;
mod settings;

pub use decide::{decide, Decision, RefreshReason, Running};
pub use digest::{inputs_digest, DigestError, RESERVED_NAMES};
pub use outcome::{FailureReason, Outcome, RefusalReason};
pub use settings::{
    Allowlist, AllowlistEntry, EntryRef, RawSettings, ServerId, ServerSettings, Settings, SettingsError,
    SigningKeyFiles,
};

#[cfg(test)]
mod decide_tests;
#[cfg(test)]
mod digest_tests;
#[cfg(test)]
mod settings_tests;
