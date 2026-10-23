//! Every route that changes state is `POST`: the root's `Sec-Fetch-Site` control exempts `GET`.
//! The one deliberate exception is the session's idle-timeout touch in [`identity_core::session`].

use axum::routing::MethodRouter;
use shared_rate_limit::RightmostXffKeyExtractor;

use crate::shell::IdentityState;

/// Per-IP budget for `POST /login`, the endpoint that sends mail.
///
/// Deliberately generous: DaSCH staff and a depositing project team can share
/// one NAT address, so a tight bucket denies sign-in to a whole office. This is
/// not the control that protects the relay quota (the global daily cap is) or
/// bounds guessing (the per-account counter is); what it stops is one host
/// driving the endpoint flat out.
pub const ISSUE_BURST: u32 = 20;
const ISSUE_REPLENISH_SECS: u64 = 30;

/// Per-IP budget for `POST /login/code`. Higher than the issue budget: entering
/// a code is a thing a legitimate user does repeatedly and a thing that sends no
/// mail.
const VERIFY_BURST: u32 = 30;
const VERIFY_REPLENISH_SECS: u64 = 5;

/// A per-IP rate limit keyed by [`RightmostXffKeyExtractor`].
///
/// A macro rather than a function because the layer's type parameters come from
/// the `governor` crate, which `tower_governor` re-exports only in part — naming
/// the return type would mean a direct dependency on `governor` for a
/// signature.
///
/// `replenish_secs` is the interval at which one unit of the burst comes back,
/// so the sustained rate is `1 / replenish_secs` per second per IP.
macro_rules! per_ip_limit {
    ($replenish_secs:expr, $burst:expr, $what:literal) => {{
        use tower_governor::governor::GovernorConfigBuilder;
        use tower_governor::GovernorLayer;

        let config = GovernorConfigBuilder::default()
            .per_second($replenish_secs)
            .burst_size($burst)
            .key_extractor(RightmostXffKeyExtractor)
            .finish()
            .expect(concat!($what, " GovernorConfig should build from its constants"));
        GovernorLayer { config: std::sync::Arc::new(config) }
    }};
}

pub(crate) type Route = (&'static str, MethodRouter<IdentityState>);

/// Declared once: [`crate::Identity::router`] folds them, and [`route_paths`] hands their paths
/// to the root's page-URL normalizer.
pub(crate) fn routes() -> Vec<Route> {
    use axum::routing::{get, post};

    use crate::handlers::{depositors, login};

    vec![
        // The two login endpoints. The rate limit is on the POST alone, merged
        // in rather than layered over the whole route: a limit that counted page
        // loads would spend an office's budget on people reading the form.
        (
            "/login",
            get(login::login_form).merge(post(login::login_submit).layer(per_ip_limit!(
                ISSUE_REPLENISH_SECS,
                ISSUE_BURST,
                "login"
            ))),
        ),
        (
            "/login/code",
            get(login::code_form).merge(post(login::code_submit).layer(per_ip_limit!(
                VERIFY_REPLENISH_SECS,
                VERIFY_BURST,
                "code"
            ))),
        ),
        // Not rate-limited: signing out costs nothing and refusing it would
        // strand a user in a session they asked to end.
        ("/logout", post(login::logout)),
        // --- RDU-only routes ---
        // Every handler below takes identity's `Rdu`, which composes `Authenticated`: no session
        // redirects to login, and a depositor's session gets the 403 page. Each write shares a
        // URL with the `GET` that renders its form, so a rejected submission re-renders somewhere
        // bookmarkable rather than on a bare 405.
        ("/depositors", get(depositors::list).post(depositors::create)),
        ("/depositors/new", get(depositors::create_form)),
        ("/depositors/{id}/edit", get(depositors::edit_form).post(depositors::update)),
        ("/depositors/{id}/remove", get(depositors::remove_form).post(depositors::remove)),
    ]
}

/// The paths of identity's routes, so the root can label a page view with its route's template
/// without seeing `IdentityState`. A path may repeat.
pub fn route_paths() -> impl Iterator<Item = &'static str> {
    routes().into_iter().map(|(path, _)| path)
}
