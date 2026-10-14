//! What the editor needs from identity (`areas/deposit/ADR-0004`): who is asking
//! ([`Authenticator`]) and the names behind the account ids its records hold
//! ([`UserDirectory`]). `identity-store` implements both beside identity's data; the editor's
//! tests use `FakeAuthenticator` and `FakeUserDirectory` (feature `test-support`).
//!
//! The crate depends on `std` alone: a consumer's `ports` crate may reach only `std` and
//! `shared-*` (ADR-0003). Ids cross as opaque strings
//! and instants as [`SystemTime`], so neither side names the other's types.

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

#[cfg(any(test, feature = "test-support"))]
mod fake;

#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeAuthenticator, FakeUserDirectory};

/// What every port method returns. `identity-ports` declares its own twin of this and of
/// [`Unavailable`]: a ports crate may depend on no other.
///
/// Boxed rather than `async fn`, which is not dyn-compatible on
/// the pinned toolchain, and the editor holds each port as an `Arc<dyn …>`.
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Depositor,
    Rdu,
}

impl Role {
    /// The form logs carry, `auth.role`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Depositor => "depositor",
            Self::Rdu => "rdu",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who is asking. Built by identity, consumed by the editor's extractors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    /// The account's id. The editor parses it into the `Uuid` its records hold.
    pub account_id: String,
    /// The name, never the address: it is what the page header shows.
    pub display_name: String,
    pub role: Role,
    /// Project shortcodes this account may reach. Empty for an RDU member, whose access is
    /// role-based rather than per-project.
    pub shortcodes: Vec<String>,
    /// The earlier of the session's absolute expiry and its idle deadline.
    pub signed_out_at: SystemTime,
}

impl Principal {
    #[must_use]
    pub fn is_rdu(&self) -> bool {
        self.role == Role::Rdu
    }

    /// Whether this account may reach the project identified by `shortcode`.
    ///
    /// RDU is unconditional: access is role-based, which is why an RDU account's `shortcodes` is
    /// empty. A depositor is scoped to their assignments.
    ///
    /// The comparison ignores ASCII case: the published set mixes `080C` with `0801a`, and no two
    /// published shortcodes differ only in case. This is
    /// `editor_core::records::normalize_shortcode`'s rule, compared rather than keyed. It does
    /// not trim: the argument has passed `is_valid_shortcode`, which admits no whitespace, and
    /// an authorization check must not be more permissive than the validator.
    /// `the_assignment_comparison_agrees_with_the_storage_key`, in `editor-server`'s `identity`
    /// tests, pins the two against each other.
    #[must_use]
    pub fn may_reach(&self, shortcode: &str) -> bool {
        match self.role {
            Role::Rdu => true,
            Role::Depositor => self.shortcodes.iter().any(|assigned| assigned.eq_ignore_ascii_case(shortcode)),
        }
    }
}

/// The port could not answer: a storage error behind it. Callers fail closed.
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

/// Who a request's session belongs to.
pub trait Authenticator: Send + Sync {
    /// The signed-in account, or `None` when the request carries no live session.
    ///
    /// `cookie_headers` is every `Cookie` header value: HTTP/2 clients may split them. Touches the
    /// session's idle timer, which is identity's write on identity's table. Fails closed: a store
    /// that cannot answer is `None`, never a guess.
    fn authenticate<'a>(&'a self, cookie_headers: &'a [&'a str], now: SystemTime) -> PortFuture<'a, Option<Principal>>;
}

/// The names behind account ids.
pub trait UserDirectory: Send + Sync {
    /// `(account_id, display_name)` for each id identity still knows. A removed account is absent,
    /// which is how the editor tells it from a known one.
    fn display_names<'a>(
        &'a self,
        account_ids: &'a [String],
    ) -> PortFuture<'a, Result<Vec<(String, String)>, Unavailable>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(role: Role, shortcodes: &[&str]) -> Principal {
        Principal {
            account_id: "00000000-0000-0000-0000-000000000000".to_string(),
            display_name: "A".to_string(),
            role,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            signed_out_at: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn test_a_depositor_reaches_only_the_projects_assigned_to_them() {
        let depositor = principal(Role::Depositor, &["0801", "0812"]);
        assert!(depositor.may_reach("0801"));
        assert!(depositor.may_reach("0812"));
        assert!(!depositor.may_reach("0803"));
    }

    #[test]
    fn test_an_assignment_matches_however_it_is_capitalised() {
        let depositor = principal(Role::Depositor, &["080c"]);
        assert!(depositor.may_reach("080C"));
        assert!(depositor.may_reach("080c"));
        assert!(!depositor.may_reach("080E"));
    }

    #[test]
    fn test_rdu_reaches_every_project_without_an_assignment() {
        let rdu = principal(Role::Rdu, &[]);
        assert!(rdu.may_reach("0801"));
        assert!(rdu.may_reach("anything"));
        assert!(rdu.is_rdu());
        assert!(!principal(Role::Depositor, &[]).is_rdu());
    }
}
