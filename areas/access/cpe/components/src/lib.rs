//! `cpe-components` — the chrome message catalog and the component renderers.
//!
//! `chrome` is the one catalog of the engine's language-varying UI strings;
//! `component` holds the renderers (and the per-project components) that turn
//! view data into Maud markup.
//!
//! This crate depends on `cpe-core` and `cpe-image-viewer`. Page assembly, which calls these
//! renderers and builds their config carriers, lives elsewhere.
//!
//! `pub` in this crate means visible to the sibling CPE crates. The host's surface is
//! only what `dsp-incubator cpe/engine` re-exports: the module `chrome` and a list of names from
//! `component`'s children; `component` itself is not exposed as a module. Items marked
//! `#[doc(hidden)]` are not host API.

pub mod chrome;
pub mod component;
