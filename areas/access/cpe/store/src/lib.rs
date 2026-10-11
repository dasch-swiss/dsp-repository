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
//! `pub` in this crate means visible to CPE's other crates. A composition root sees none of
//! it: `cpe-server`, the crate a composition root depends on, re-exports nothing of this
//! crate. Items marked `#[doc(hidden)]` are for the sibling crates and their tests.

pub mod db;
pub mod query;
