//! `editor-web` — the metadata editor's view layer.
//!
//! Mirrors `dpe-web`: a plain library of `fn(...) -> maud::Markup` functions,
//! with components in `components/` and pages in `pages/`. No component macro
//! and no re-export shim — import `editor-core` and `mosaic-tiles` types
//! directly once there are any to import.
//!
//! Unlike DPE, the HTML document shell lives here rather than in the server
//! crate: the editor's server is a composition root for routing, auth and
//! persistence, and the shell is a view concern like every other partial.

pub mod components;
pub mod pages;
pub mod view;
