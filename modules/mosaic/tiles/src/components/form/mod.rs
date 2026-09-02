//! The form tiles: the field shell every input shares, and the inputs built on
//! it.
//!
//! Grouped as a directory, **not** as a public path. `components/mod.rs`
//! re-exports this module's contents, so a caller still writes
//! `mosaic_tiles::text_field::text_field` and moving a tile in or out of the
//! group is not a breaking change. That is what makes the grouping cheap: the
//! public API stays flat for every tile, so the form tiles being in a
//! subdirectory is not an inconsistency the other tiles have to follow.
//!
//! What earns the directory is the shared `field-*` shell in
//! `text_field/text_field.css` — the label, border, hint and error treatment
//! every input here renders. A reader looking for "how does a field look" has
//! one place to go.

pub mod text_field;
