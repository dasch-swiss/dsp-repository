//! The driver's error, wrapped so that no driver type is part of this crate's surface.

use std::fmt;

/// The SQLite failure behind a `DbError` or a `QueryError`.
///
/// It prints as the driver's error does and has no `source()`: the driver's own source is a
/// `libsqlite3-sys` type, which a caller could downcast to. This module is private, which keeps the
/// type from being named outside the crate: do not re-export it.
pub struct SqliteError(rusqlite::Error);

impl SqliteError {
    pub(crate) fn new(error: rusqlite::Error) -> Self {
        Self(error)
    }
}

impl fmt::Display for SqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for SqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for SqliteError {}
