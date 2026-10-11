//! `cpe-store` — the SQLite store behind a project: `db` builds a project's
//! in-memory database from `schema.sql` and owns it as `db::ProjectStore`, and
//! `query` is the query layer over it.
//!
//! The store implements `cpe_core::validate::facts::ProjectFacts` as
//! `query::LiveProjectFacts`, which is how the validator reads the data. It also
//! builds a request's `cpe_core::view_data::PageData`: `query::page::page_data`
//! assembles it over the reads in `query::reads`.
//!
//! `ProjectStore` owns the connection's `Mutex`, and no `pub` item hands out the
//! connection or a guard on it, so every lock is taken inside this crate. The lock
//! rule's home is `query::reads`: short-scoped, and never returned.
//!
//! No `pub` item names a `rusqlite` type either (the error enums carry the driver's
//! error in the wrapper of the private module `error`), so a crate that reads the
//! store needs no `rusqlite`.
//!
//! `pub` in this crate means visible to CPE's other crates. A composition root sees none of
//! it: `cpe-server`, the crate a composition root depends on, re-exports nothing of this
//! crate. Items marked `#[doc(hidden)]` are for the sibling crates and their tests.

pub mod db;
mod error;
pub mod query;
