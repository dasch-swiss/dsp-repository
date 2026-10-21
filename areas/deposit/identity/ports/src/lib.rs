//! What identity needs from the editor (`areas/deposit/ADR-0004`, amendment of 2026-10-13): the
//! work an account still has open, which the depositor-removal confirmation lists. `editor-store`
//! implements [`OpenWork`] beside the editor's data; identity's tests use
//! `FakeOpenWork` (feature `test-support`).
//!
//! The crate depends on `std` alone: a consumer's `ports` crate may reach only `std` and
//! `shared-*` (ADR-0003). Account ids cross as opaque strings, so neither side names the other's
//! types.

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[cfg(any(test, feature = "test-support"))]
mod fake;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeOpenWork;

/// What every port method returns. `editor-ports` declares its own twin of this and of
/// [`Unavailable`]: a ports crate may depend on no other.
///
/// Boxed rather than `async fn`, which is not dyn-compatible on
/// the pinned toolchain, and identity holds the port as an `Arc<dyn …>`.
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The port could not answer: a storage error behind it.
#[derive(Debug)]
pub struct Unavailable(Box<dyn Error + Send + Sync>);

impl Unavailable {
    #[must_use]
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

/// The source's own message, because call sites log with `%error` and the text stays what the
/// storage error said.
impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The source's cause rather than the source itself, which `Display` already prints.
impl Error for Unavailable {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

/// The editor work still open under an account.
///
/// Two reads rather than one, because the removal page names which one failed.
pub trait OpenWork: Send + Sync {
    /// Shortcodes of the drafts this account saved last.
    fn drafts<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>>;

    /// Shortcodes of this account's submissions short of `Approved`: work that can no longer be
    /// returned to its depositor once the account is gone.
    fn submissions<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>>;
}
