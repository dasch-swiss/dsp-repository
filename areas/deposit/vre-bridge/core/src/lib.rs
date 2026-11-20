//! The VRE bridge's pure logic: its settings, parsed from the strings `deposit-server`'s loader
//! hands over; the allowlist and server ids; whether a project is refreshed and from which cutoff;
//! the digest of a project's shipped inputs; DaTEI's file mapping and file manifest; and the
//! outcome of a refresh.
//!
//! No network and no tokio. The only file-system access is [`inputs_digest`] and
//! [`shipped_names`], which read the shipped inputs and block, so an async caller runs them in
//! `spawn_blocking`. Nothing here writes anything.

mod datei;
mod decide;
mod digest;
mod outcome;
mod settings;

pub use datei::{
    manifest_bytes, plan_files, Download, FileMapping, FilePlan, FileRefusal, FileValue, ManifestEntry, MappingEntry,
    MappingError, Target, FILE_MANIFEST, FILE_MAPPING,
};
pub use decide::{decide, Decision, RefreshReason, Running};
pub use digest::{inputs_digest, shipped_names, DigestError, RESERVED_NAMES};
pub use outcome::{FailureReason, Outcome, RefusalReason};
pub use settings::{
    Allowlist, AllowlistEntry, EntryRef, RawSettings, ServerId, ServerSettings, Settings, SettingsError,
    SigningKeyFiles,
};

#[cfg(test)]
mod datei_tests;
#[cfg(test)]
mod decide_tests;
#[cfg(test)]
mod digest_tests;
#[cfg(test)]
mod settings_tests;
