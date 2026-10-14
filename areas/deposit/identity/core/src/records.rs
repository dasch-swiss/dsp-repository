//! The records identity persists: accounts, sessions and login codes.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The two roles the editor recognises.
///
/// `rdu` members come from configuration and always exist without
/// provisioning; depositors are rows created by RDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Depositor,
    Rdu,
}

impl Role {
    /// The stored form. Pinned by a `CHECK` constraint in the schema, so this
    /// and the migration have to agree.
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

/// A stored value outside the known set. Reachable only if the database is
/// edited by hand or a migration adds a variant the code does not know.
#[derive(Debug, thiserror::Error)]
#[error("unknown {kind} {value:?}")]
pub struct UnknownVariant {
    pub kind: &'static str,
    pub value: String,
}

impl FromStr for Role {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "depositor" => Ok(Self::Depositor),
            "rdu" => Ok(Self::Rdu),
            other => Err(UnknownVariant { kind: "role", value: other.to_string() }),
        }
    }
}

/// A person who can log in.
///
/// `email` is stored as entered and in plaintext: the application must read it
/// to send mail, so a key would sit beside the data. `email_normalized`,
/// lowercased, carries the uniqueness constraint and every lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    /// Project shortcodes this user may reach. Empty for an RDU member, whose
    /// access is role-based rather than per-project.
    pub shortcodes: Vec<String>,
    /// Consecutive failed authentications **for the account**, not for a code.
    /// NIST SP 800-63B-4: "Generating a new authentication secret SHALL NOT
    /// reset the failed authentication count" — so this survives code
    /// invalidation and resend, and only a successful login clears it.
    pub failed_logins: u32,
    /// When [`Self::failed_logins`] last went up, and so when a lockout started.
    /// The counter alone cannot express a lockout that ends, since it resets only
    /// on success; throttling measures from this. Cleared with the counter.
    pub failed_login_at: Option<DateTime<Utc>>,
    /// When a login code was last issued, so RDU can answer "I never got a
    /// code" without an address reaching a log.
    pub last_code_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Whether this account administers the service.
    #[must_use]
    pub fn is_rdu(&self) -> bool {
        self.role == Role::Rdu
    }

    /// The lookup and uniqueness key: `email` lowercased.
    ///
    /// `to_lowercase` rather than `to_ascii_lowercase`, so a non-ASCII address
    /// folds too. The whole address is folded: RFC 5321 makes the local part
    /// case-sensitive, but no provider does, and two spellings of one address
    /// would make "already taken" depend on how it was typed.
    #[must_use]
    pub fn normalize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }

    /// This user's normalized address.
    #[must_use]
    pub fn email_normalized(&self) -> String {
        Self::normalize_email(&self.email)
    }
}

/// An authenticated session.
///
/// `id` is the opaque token carried by the cookie, not a UUID: how it is minted
/// is the auth layer's decision, and this layer only stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    /// Advanced on use, for the idle timeout.
    pub last_seen_at: DateTime<Utc>,
    /// Absolute expiry, set at creation and never extended.
    pub expires_at: DateTime<Utc>,
}

/// A one-time login code.
///
/// Stored unhashed, deliberately: it lives ten minutes, and anyone who can read
/// this table already holds `sessions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    /// Wrong entries against *this* code. Three invalidates it; the
    /// account-level counter that survives a resend is [`User::failed_logins`].
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Set when the code is accepted. A code is usable once, for replay
    /// resistance (NIST SP 800-63B-4 §3.1.3.2).
    pub consumed_at: Option<DateTime<Utc>>,
    /// The opaque token held by the browser that asked for this code, and the
    /// only browser that may spend it.
    ///
    /// Blocks the attack email codes are most exposed to: triggering a login for
    /// the victim and spending the read-out code from another machine. `None`
    /// binds to no browser and can never be verified.
    pub browser_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_round_trips_through_its_stored_form() {
        for role in [Role::Depositor, Role::Rdu] {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), role);
        }
    }

    #[test]
    fn test_unknown_stored_variant_is_an_error_not_a_default() {
        // A row the code does not understand must surface, not silently become
        // `Depositor` — that would hand an unknown role a depositor's access.
        assert!("admin".parse::<Role>().is_err());
    }

    #[test]
    fn test_normalize_email_folds_case_and_trims() {
        assert_eq!(User::normalize_email("  A.User@Example.TEST "), "a.user@example.test");
    }

    fn user(role: Role, shortcodes: &[&str]) -> User {
        User {
            id: Uuid::nil(),
            email: "a@x.test".to_string(),
            name: "A".to_string(),
            role,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: DateTime::<Utc>::MIN_UTC,
        }
    }

    #[test]
    fn test_only_an_rdu_account_administers_the_service() {
        assert!(user(Role::Rdu, &[]).is_rdu());
        assert!(!user(Role::Depositor, &[]).is_rdu());
    }
}
