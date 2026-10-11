//! `cpe-core` — CPE's domain: the config language and its validation, the IR,
//! diagnostics, labels, language and theme, the per-request view data and the
//! pure helpers they share.
//!
//! The crate is framework-free: it depends on no `maud`, `axum` or `rusqlite`.
//! The store reaches the validator through `validate::facts::ProjectFacts`.
//!
//! `pub` in this crate means visible to CPE's other crates. Until DEV-7465, the host's surface
//! is only what `dsp-incubator cpe/engine` re-exports: the modules `diagnostic`, `ir`, `lang` and
//! `theme`, and a list of names from `labels`, `markup_contract`, `util`, `validate`
//! and `view_data`. The other modules are not exposed. Items marked `#[doc(hidden)]`
//! are not host API.
//!
//! That facade re-exports `ir::DataArks`, so `DataArks::from_stored` is reachable from
//! the host. A host must never call it: a value built from unchecked strings reaches
//! an `href`.

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
