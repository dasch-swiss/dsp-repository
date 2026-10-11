//! `cpe-web` — page assembly for the Configurable Presentation Environment.
//!
//! - `page` — `render_page`, `render_compound_fragment` and the two full-page not-found renderers.
//! - `project` — the loaded-project type (`Project`) and its loader.
//! - `share_link` — the canonical share URL of a view.
//!
//! The crate holds no routes: the HTTP routes are the host's. The host is the crate that
//! owns the routes and calls this one: `cpe-server`.
//!
//! `pub` in this crate means visible to the sibling CPE crates, the host among them, and
//! not to a composition root, which sees the host alone. Items marked `#[doc(hidden)]`
//! are not host API.

pub mod page;
pub mod project;
pub mod share_link;
