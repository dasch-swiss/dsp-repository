//! `identity-core`: accounts, sessions and the email one-time-code login.
//!
//! Framework-free, with no Axum, Maud or database dependency. The SQLite
//! implementations of the [`repository`] ports live in `identity-store`, the
//! HTTP handlers in `identity-web`.

pub mod cookie;
pub mod login;
pub mod mail;
pub mod records;
pub mod repository;
pub mod secret;
pub mod session;
