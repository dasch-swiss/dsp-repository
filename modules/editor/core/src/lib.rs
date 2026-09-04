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
//! `drafts.payload` holds, the multilingual editing view ([`multilingual`]) a
//! form renders, how a posted form body is read back into a draft ([`form`]),
//! the published set the form pre-fills from ([`published`]), the
//! field-by-field comparison RDU reviews a submission through ([`review`]), the
//! checks a draft must pass to be submitted ([`submission`]), and the canonical
//! writer ([`canonical`]) that turns an approved draft back into a
//! `projects/*.json` file byte-for-byte.

pub mod canonical;
pub mod draft;
pub mod form;
pub mod json;
pub mod multilingual;
pub mod published;
pub mod records;
pub mod repository;
pub mod review;
pub mod submission;

#[cfg(test)]
mod test_support;
