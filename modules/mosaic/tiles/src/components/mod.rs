//! The tiles themselves, re-exported flat by `lib.rs`.
//!
//! `form` is a directory grouping rather than a public path: its contents are
//! re-exported here, so `mosaic_tiles::text_field::text_field` is the import
//! path whether or not the tile sits in a subdirectory. See `form/mod.rs`.

pub mod alert;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod copy_button;
pub mod icon;
pub mod link;
pub mod loading;
pub mod table;
pub mod tabs;

mod form;
pub use form::*;
