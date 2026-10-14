//! Only these modules use axum; the views in `pages/` do not.
//!
//! **Email one-time-code login is a documented deviation, not a compliant design.** NIST
//! SP 800-63B-4 §3.1.3.1 and OWASP ASVS 5.0 V6.6 both prohibit email as an authentication
//! mechanism outright, and their carve-out for address verification and recovery codes does not
//! cover using one as the login. ASVS 6.1.1 and 6.3.3 require the deviation and its rationale to
//! be written down: they are in `docs/src/editor/authentication.md`.
//!
//! The flow defends enumeration, interception, guessing, replay, quota exhaustion and fixation.
//! Which control answers which, and the numbers behind the caps and the throttle, are in the
//! same page.

pub(crate) mod accounts;
pub(crate) mod cookie;
pub(crate) mod depositors;
pub(crate) mod guard;
pub(crate) mod login;
