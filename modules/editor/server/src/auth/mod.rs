//! Email one-time-code login (US-6): issuing a code, verifying it, and the
//! session it produces.
//!
//! ## This is a documented deviation, not a compliant design
//!
//! NIST SP 800-63B-4 §3.1.3.1 and OWASP ASVS 5.0 V6.6 both prohibit email as an
//! authentication mechanism outright, and the carve-out is narrow: it covers
//! codes sent to *verify an address* or as *recovery* codes, not codes used as
//! the login. ASVS 6.1.1 and 6.3.3 require such a deviation and its rationale to
//! be written down, so it is — in `docs/src/editor/authentication.md`. Nothing
//! in this module should be read as evidence that the design complies.
//!
//! ## What the flow defends, in order
//!
//! 1. **Enumeration** (REQ-6.2). Every `POST /login` answers the same way — the same status, the
//!    same location, the same cookie behaviour — whether the address is known, unknown, cooled
//!    down, locked out or hit the daily cap. Only the mail differs, and only the address's owner
//!    sees that.
//! 2. **Interception.** The code is bound to the browser that asked for it. NIST's stated objection
//!    to email codes is interception in transit or at intermediate mail servers; a code read out of
//!    a mailbox by anyone else is useless without that browser's cookie. It does **not** defend
//!    attacker-initiated social engineering — an attacker who starts the login holds the binding —
//!    and it is not claimed to.
//! 3. **Guessing.** Three wrong entries kill a code (REQ-6.4), and an account-level counter that
//!    survives invalidation and resend throttles the account itself. The per-code counter alone
//!    hands out a fresh budget on every resend, which at a 60-second cooldown is ~4,320 guesses a
//!    day against one address — around a 12% chance of hitting a six-digit code inside a month.
//! 4. **Replay.** A code authenticates once; the single-use check is the `WHERE consumed_at IS
//!    NULL` in the update, so two simultaneous submissions cannot both win.
//! 5. **Quota exhaustion.** Two daily send caps. A global one across all users, because the relay
//!    quota is shared: an attacker looping resend across the known addresses would otherwise lock
//!    out everyone, RDU included. And a per-account one, because the global cap alone is
//!    exhaustible from a single address — the cooldown is per address, so 1,440 codes a day fit
//!    inside it against a global default of 500. Both count messages actually sent
//!    ([`editor_core::repository::MailSendRepository`]), not live code rows.
//! 6. **Fixation.** The session id is a fresh token, so nothing held before authentication can
//!    become the authenticated session.

pub(crate) mod cookie;
pub(crate) mod guard;
mod handlers;
pub(crate) mod secret;
pub(crate) mod session;

use std::time::Duration;

use chrono::{DateTime, Utc};
use editor_core::records::User;
pub(crate) use handlers::{code_form, code_submit, login_form, login_submit, logout};

use crate::config::EditorConfig;

/// The knobs the login flow reads, resolved once at startup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AuthConfig {
    /// How long before another code may be sent to one address (REQ-6.5).
    pub cooldown: Duration,
    /// Consecutive account-level failures tolerated before throttling.
    pub max_failed: u32,
    /// How long the throttle lasts.
    pub lockout: Duration,
    /// Codes that may be sent across all users in 24 hours.
    pub daily_cap: u64,
    /// Codes that may be sent to one account in 24 hours. Without it the global
    /// cap is exhaustible from a single known address.
    pub account_daily_cap: u64,
    /// Absolute session lifetime, never extended.
    pub session_absolute: Duration,
    /// Idle session timeout.
    pub session_idle: Duration,
    /// Whether a failing relay writes the code to the log instead of rolling it
    /// back. Off by default — see [`EditorConfig::smtp_break_glass`].
    pub break_glass: bool,
}

impl From<&EditorConfig> for AuthConfig {
    fn from(config: &EditorConfig) -> Self {
        Self {
            cooldown: Duration::from_secs(config.login_cooldown_secs),
            max_failed: config.login_max_failed,
            lockout: Duration::from_secs(config.login_lockout_secs),
            daily_cap: config.mail_daily_cap,
            account_daily_cap: config.mail_account_daily_cap,
            session_absolute: Duration::from_secs(config.session_absolute_secs),
            session_idle: Duration::from_secs(config.session_idle_secs),
            break_glass: config.smtp_break_glass,
        }
    }
}

/// `std::time::Duration` as the `chrono` delta the records are stamped with.
///
/// Saturates rather than unwrapping: every value here comes from config that
/// validation has already bounded, and a panic in an auth path is a worse
/// failure than an implausibly distant deadline.
///
/// `pub(crate)` because the hourly sweep converts [`crate::config::SEND_WINDOW`]
/// the same way, and two copies of the saturation policy is one too many.
pub(crate) fn delta(duration: Duration) -> chrono::TimeDelta {
    chrono::TimeDelta::from_std(duration).unwrap_or(chrono::TimeDelta::MAX)
}

/// Whether `candidate` could be an address at all.
///
/// Deliberately not a validator. RFC 5322 addresses are stranger than any regex
/// anyone writes for them, and the only judgement that counts is the relay's.
/// This rejects what cannot be an address, so an obvious typo gets a message
/// rather than a silent nothing — which, with REQ-6.2's identical response, is
/// otherwise indistinguishable from success.
pub(crate) fn is_plausible_address(candidate: &str) -> bool {
    if candidate.len() > 254 || candidate.matches('@').count() != 1 {
        return false;
    }
    let Some((local, domain)) = candidate.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !candidate.contains(char::is_whitespace)
}

/// Whether this account is currently throttled.
///
/// Time-based rather than a latch, and that is forced rather than chosen. NIST
/// SP 800-63B-4 says the counter resets only on a successful authentication, and
/// an account at the cap cannot authenticate — so a latch would be a permanent
/// lock needing an unlock control that does not exist.
///
/// Attempts made *during* a lockout are refused here, before they are counted,
/// so the window runs out rather than extending. What stops the lockout becoming
/// permanent in the other direction is the decay in
/// [`editor_core::repository::UserRepository::record_failed_login`]: once the
/// window has passed, the next failure starts the count over, so re-locking an
/// account costs a fresh `max_failed` failures rather than one.
pub(crate) fn locked_out(user: &User, auth: &AuthConfig, now: DateTime<Utc>) -> bool {
    if user.failed_logins < auth.max_failed {
        return false;
    }
    match user.failed_login_at {
        Some(at) => now < at + delta(auth.lockout),
        // Unreachable: the counter and the instant are written together. An
        // account at the cap with no instant is a hand-edited or pre-migration
        // row, and refusing it is the fail-closed reading of one.
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_reads_every_knob_from_the_editor_config() {
        let config = EditorConfig::default();
        let auth = AuthConfig::from(&config);
        assert_eq!(auth.cooldown, Duration::from_secs(config.login_cooldown_secs));
        assert_eq!(auth.max_failed, config.login_max_failed);
        assert_eq!(auth.lockout, Duration::from_secs(config.login_lockout_secs));
        assert_eq!(auth.daily_cap, config.mail_daily_cap);
        assert_eq!(auth.account_daily_cap, config.mail_account_daily_cap);
        assert_eq!(auth.session_absolute, Duration::from_secs(config.session_absolute_secs));
        assert_eq!(auth.session_idle, Duration::from_secs(config.session_idle_secs));
        assert!(!auth.break_glass, "the break-glass log fallback must be off unless asked for");
    }

    #[test]
    fn test_the_code_lifetime_is_not_configurable() {
        // Ten minutes is a ceiling NIST §3.1.3.2 and ASVS 6.5.5 both impose, so
        // the only thing a knob could express is a violation of both. It is a
        // constant for that reason, and `AuthConfig` deliberately has no field
        // for it.
        assert_eq!(crate::config::CODE_TTL, Duration::from_secs(600));
    }

    fn at(hour: u32) -> DateTime<Utc> {
        use chrono::TimeZone;
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    fn user(failed_logins: u32, failed_login_at: Option<DateTime<Utc>>) -> User {
        User {
            id: uuid::Uuid::new_v4(),
            email: "a@x.test".to_string(),
            name: "A".to_string(),
            role: editor_core::records::Role::Depositor,
            shortcodes: vec![],
            failed_logins,
            failed_login_at,
            last_code_at: None,
            created_at: at(9),
        }
    }

    fn throttle() -> AuthConfig {
        AuthConfig {
            max_failed: 3,
            lockout: Duration::from_secs(3600),
            ..AuthConfig::from(&EditorConfig::default())
        }
    }

    #[test]
    fn test_a_plausible_address_is_accepted() {
        for candidate in [
            "a@x.test",
            "a.user+tag@sub.example.org",
            "UPPER@Example.TEST",
            "a@x.y.z.test",
        ] {
            assert!(is_plausible_address(candidate), "{candidate}");
        }
    }

    #[test]
    fn test_what_cannot_be_an_address_is_refused() {
        for candidate in [
            "",
            "nobody",
            "@x.test",
            "a@",
            "a@localhost",
            "a@.test",
            "a@test.",
            "a b@x.test",
            "a@x.test\n",
            "a@@x.test",
            "a@x.test@y.test",
        ] {
            assert!(!is_plausible_address(candidate), "{candidate}");
        }
    }

    #[test]
    fn test_an_absurdly_long_address_is_refused() {
        let long = format!("{}@x.test", "a".repeat(250));
        assert!(!is_plausible_address(&long));
    }

    #[test]
    fn test_an_account_below_the_cap_is_never_locked_out() {
        assert!(!locked_out(&user(0, None), &throttle(), at(10)));
        assert!(!locked_out(&user(2, Some(at(10))), &throttle(), at(10)));
    }

    #[test]
    fn test_the_lockout_starts_at_the_cap_and_ends_with_the_window() {
        let capped = user(3, Some(at(10)));
        assert!(locked_out(&capped, &throttle(), at(10)));
        // One hour later, exactly at the boundary, it is over — a window that
        // never ended would need an unlock control, and there is none.
        assert!(!locked_out(&capped, &throttle(), at(11)));
        assert!(!locked_out(&capped, &throttle(), at(12)));
    }

    #[test]
    fn test_a_further_failure_while_capped_re_arms_the_window() {
        // The counter only ever goes up, so the instant is what moves. Without
        // this, a guesser waits out one window and then has the run of the
        // account again.
        let capped = user(9, Some(at(11)));
        assert!(locked_out(&capped, &throttle(), at(11)));
        assert!(!locked_out(&capped, &throttle(), at(12)));
    }

    #[test]
    fn test_a_capped_account_with_no_instant_fails_closed() {
        // Unreachable through the repository, which writes both together. If a
        // row ever says otherwise, refusing is the safe reading.
        assert!(locked_out(&user(3, None), &throttle(), at(10)));
    }

    #[test]
    fn test_every_configurable_duration_produces_a_deadline_that_does_not_panic() {
        // `delta` saturates rather than panicking, but saturating to
        // `TimeDelta::MAX` is itself the value that overflows a `DateTime`
        // addition — so the guarantee that matters is not about `delta`, it is
        // about the deadline. Config bounds every duration to a year; this pins
        // that the arithmetic at that bound is fine.
        let now = Utc::now();
        let year = Duration::from_secs(365 * 24 * 60 * 60);
        assert_eq!(delta(Duration::from_secs(60)), chrono::TimeDelta::seconds(60));
        let _ = now + delta(year);
        let _ = now - delta(year);
    }
}
