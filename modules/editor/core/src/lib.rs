//! `editor-core`: pure domain types for the metadata editor.
//!
//! Framework-free types shared by `editor-web` and `editor-server`, with no
//! Axum, Maud or database dependency. The SQLite implementations of the
//! [`repository`] ports live in `editor-server`, which keeps the
//! `server → web → core` direction intact and the driver out of the domain.

pub mod agents;
pub mod canonical;
pub mod draft;
pub mod form;
pub mod json;
pub mod multilingual;
pub mod proposals;
pub mod published;
pub mod records;
pub mod repository;
pub mod review;
pub mod status;
pub mod submission;

#[cfg(test)]
mod test_support;
