//! `editor-web`: the metadata editor's view layer.
//!
//! A plain library of `fn(...) -> maud::Markup` functions, components in
//! `components/` and pages in `pages/`. Unlike DPE, the HTML document shell
//! lives here: the server crate is a composition root for routing, auth and
//! persistence, and the shell is a view concern like every other partial.

pub mod components;
pub mod entity;
pub mod form;
pub mod pages;
pub mod view;
