//! `cpe-web` — page assembly for the Configurable Presentation Environment.
//!
//! - `page` — `render_page`, `render_compound_fragment` and the N1/N2 full-page not-found
//!   renderers.
//! - `project` — the loaded-project type (`Project`) and its loader.
//! - `share_link` — the canonical share URL of a view.
//!
//! The crate holds no routes: the HTTP routes stay in the `cpe` binary.
//!
//! `pub` in this crate means visible to the sibling CPE crates. The host's surface is
//! only what `cpe-engine` re-exports: the module `share_link` and a list of names from
//! `page` and `project`; neither `page` nor `project` is exposed as a module. Items
//! marked `#[doc(hidden)]` are not host API.

pub mod page;
pub mod project;
pub mod share_link;
