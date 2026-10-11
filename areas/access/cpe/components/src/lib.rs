//! `cpe-components` — the chrome message catalog and the component renderers.
//!
//! `chrome` is the one catalog of the engine's language-varying UI strings;
//! `component` holds the renderers (and the per-project components) that turn
//! view data into Maud markup.
//!
//! This crate depends on `cpe-core` and `cpe-image-viewer`. Page assembly, which calls these
//! renderers and builds their config carriers, lives elsewhere.
//!
//! `pub` in this crate means visible to CPE's other crates. A composition root sees none of
//! it: `cpe-server`, the crate a composition root depends on, re-exports nothing of this
//! crate. Items marked `#[doc(hidden)]` are for the sibling crates and their tests.

pub mod chrome;
pub mod component;
