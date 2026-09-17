//! The form tiles: the field shell every input shares, and the inputs built on
//! it.
//!
//! Grouped as a directory, **not** as a public path: `components/mod.rs`
//! re-exports this module's contents, so a caller still writes
//! `mosaic_tiles::text_field::text_field` and moving a tile in or out of the
//! group is not a breaking change.
//!
//! What earns the directory is the shared `field-*` shell in
//! `text_field/text_field.css`, the label, border, hint and error treatment every
//! input here renders.

mod choice;
mod shell;

pub mod checkbox_group;
pub mod radio_group;
pub mod repeatable_list;
pub mod select;
pub mod text_field;
pub mod textarea;
