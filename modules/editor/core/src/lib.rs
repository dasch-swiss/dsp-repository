//! `editor-core` — pure domain types for the metadata editor.
//!
//! Mirrors `dpe-core`'s role: framework-free types shared by `editor-web` and
//! `editor-server`, with no Axum, Maud or database dependency.
//!
//! It owns two things. The persistence contract: the records the editor stores
//! ([`records`]) and one port per aggregate ([`repository`]); the SQLite
//! implementations of those ports live in `editor-server`, which keeps the
//! `server → web → core` direction intact and the driver out of the domain. And
//! the project representation: the permissive draft ([`draft`]) that a
//! `drafts.payload` holds and the multilingual editing view ([`multilingual`])
//! a form renders.

pub mod draft;
pub mod json;
pub mod multilingual;
pub mod records;
pub mod repository;

#[cfg(test)]
mod test_support;
