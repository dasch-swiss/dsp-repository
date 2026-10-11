//! `cpe-core` — CPE's domain: the config language and its validation, the IR,
//! diagnostics, labels, language and theme, the per-request view data and the
//! pure helpers they share.
//!
//! The crate is framework-free: it depends on no `maud`, `axum` or `rusqlite`.
//! The store reaches the validator through `validate::facts::ProjectFacts`.
//!
//! `pub` in this crate means visible to CPE's other crates. A composition root sees none of
//! it: `cpe-server`, the crate a composition root depends on, names no type of this crate in a
//! `pub` signature. Items marked `#[doc(hidden)]` are for the sibling crates and their tests.
//!
//! `ir::DataArks::from_stored` is for the store alone: a value built from unchecked strings
//! reaches an `href`.

mod config;
pub mod diagnostic;
pub mod iiif;
pub mod ir;
pub mod labels;
pub mod lang;
pub mod markup_contract;
pub mod narrative_outline;
pub mod remodel;
pub mod theme;
pub mod util;
pub mod validate;
pub mod view_data;

pub use config::ConfigError;
