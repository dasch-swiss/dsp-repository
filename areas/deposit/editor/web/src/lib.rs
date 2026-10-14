//! `editor-web`: the metadata editor's view layer.
//!
//! A plain library of `fn(...) -> maud::Markup` functions: pages in `pages/`, the
//! form in `form/`. The document they are wrapped in is the area's, in `deposit-shell`.

pub mod entity;
pub mod form;
pub mod pages;
