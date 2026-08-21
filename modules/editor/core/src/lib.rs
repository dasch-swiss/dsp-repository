//! `editor-core` — pure domain types for the metadata editor.
//!
//! Mirrors `dpe-core`'s role: framework-free types shared by `editor-web` and
//! `editor-server`, with no Axum, Maud or database dependency.
//!
//! What it owns today is the persistence contract: the records the editor
//! stores ([`records`]) and one port per aggregate ([`repository`]). The SQLite
//! implementations of those ports live in `editor-server`, which keeps the
//! `server → web → core` direction intact and the driver out of the domain.
//!
//! Still to land: the permissive draft representation, the order-preserving
//! multi-language map, and the canonical `ProjectRaw` conversion. Until then
//! drafts, submissions and approved records carry their body as an opaque JSON
//! `payload` — see [`records`].

pub mod records;
pub mod repository;
