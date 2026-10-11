//! `cpe-store` — the SQLite store behind a project: `db` builds a project's
//! in-memory database from `schema.sql`, and `query` is the query layer over it.
//!
//! The store implements `cpe_core::validate::facts::ProjectFacts` as
//! `query::LiveProjectFacts`, which is how the validator reads the data. It also
//! builds a request's `cpe_core::view_data::PageData`: `query::page::page_data`
//! assembles it over the reads in `query::reads`.
//!
//! The lock rule's home is `query::reads`: every lock on a project's connection is
//! taken there, short-scoped, and never returned. Page assembly (`Project`, in
//! `cpe-web`) owns the connection's `Mutex`.
//!
//! `pub` in this crate means visible to the sibling CPE crates. The host's surface is
//! only what `dsp-incubator cpe/engine` re-exports, which is three names (`DbError`, `QueryError`
//! and `search_index_name`); neither `db` nor `query` is exposed as a module. Items
//! marked `#[doc(hidden)]` are not host API.

pub mod db;
pub mod query;
