//! `identity-core`: accounts, sessions and the email one-time-code login.
//!
//! Framework-free, with no Axum, Maud or database dependency. The SQLite
//! implementations of the [`repository`] ports and the HTTP handlers live in
//! `editor-server`.

pub mod cookie;
pub mod login;
pub mod mail;
pub mod records;
pub mod repository;
pub mod secret;
pub mod session;
