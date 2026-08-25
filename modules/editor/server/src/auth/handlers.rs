//! The five login routes: `GET`/`POST /login`, `GET`/`POST /login/code`, and
//! `POST /logout`.
//!
//! Every `POST /login` answers identically (REQ-6.2). That is not one branch
//! being careful — it is the shape of [`issue`]: the outcome decides only
//! whether mail is sent, never what the browser is told.

use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, TimeDelta, Utc};
use editor_core::records::{LoginCode, Session};
use editor_core::repository::{Attempt, Issued, LoginCodeRepository, UserRepository};
use maud::Markup;
use serde::Deserialize;
use uuid::Uuid;

use super::guard::{destination_or_root, login_url, next_from, NEXT};
use super::{cookie, delta, is_plausible_address, locked_out, secret, session};
use crate::config::CODE_TTL;
use crate::mail::Mail;
use crate::AppState;

/// The one message every code-entry failure produces.
///
/// Wrong digits, an expired code, a consumed code, a code with its three strikes
/// spent, a token that binds to nothing, and a throttled account all say this.
/// A message that distinguished them would undo REQ-6.2: a token that resolves
/// to a live code only exists for an address that has an account, so "too many
/// attempts" confirms the address is known to anyone willing to spend ten
/// guesses. The cost is real — a throttled user is told nothing useful — and it
/// is why RDU gets "last code issued at" rather than the user getting detail.
const CODE_REJECTED: &str = "That code is not valid, or it has expired. Request a new one.";

/// The one message a malformed address produces. Not an enumeration leak: an
/// address that cannot be an address cannot be a registered one.
const EMAIL_REJECTED: &str = "Enter a valid email address.";

const MAIL_SUBJECT: &str = "Your DaSCH Metadata Editor sign-in code";

const SIGN_IN_TITLE: &str = "Sign in — DaSCH Metadata Editor";
const ENTER_CODE_TITLE: &str = "Enter your code — DaSCH Metadata Editor";

#[derive(Deserialize)]
pub(crate) struct EmailForm {
    email: String,
}

#[derive(Deserialize)]
pub(crate) struct CodeForm {
    code: String,
}

/// Render a login screen. Always anonymous: the shell's signed-in header has no
/// business on a page reached without a session, which is what the `None`
/// viewer says.
fn render(state: &AppState, title: &str, status: StatusCode, content: Markup) -> Response {
    crate::render(state, title, status, None, content)
}

/// `/login/code`, carrying the destination on to the second screen.
///
/// Built here rather than in [`super::guard`] beside [`login_url`] because it is
/// a step inside this flow rather than a way into it. The same no-encoding
/// argument applies: `safe_next` admits only unreserved characters.
fn code_url(next: Option<&str>) -> String {
    match next {
        Some(next) => format!("/login/code?{NEXT}={next}"),
        None => "/login/code".to_string(),
    }
}

/// Attach a `Set-Cookie` to a response already built.
fn with_cookie(mut response: Response, cookie: axum::http::HeaderValue) -> Response {
    // `append`, not `insert`: signing in sets the session cookie and clears the
    // login cookie in the same response, and `insert` would drop the first.
    response.headers_mut().append(SET_COOKIE, cookie);
    response
}

/// The message body. Plain text: an HTML mail buys nothing here and gives a
/// filter one more reason to be suspicious of a message carrying a login code.
fn mail_body(code: &str) -> String {
    format!(
        "Your sign-in code for the DaSCH Metadata Editor is:\n\n    {code}\n\n\
         It is valid for ten minutes, can be used once, and only in the browser that asked for it.\n\n\
         If you did not ask to sign in, you can ignore this message: the code cannot be used\n\
         anywhere but that browser.\n"
    )
}

/// `GET /login` — the address form.
///
/// `next` is where the reader was going when the guard sent them here. It is
/// carried on the form's action rather than in a field, and re-validated at
/// every step it crosses.
pub(crate) async fn login_form(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let next = next_from(&uri);
    if session::current(&state.db, &state.auth, &headers, Utc::now()).await.is_some() {
        // Already signed in: honour the destination rather than dropping them on
        // the root, so a link followed in a browser that still has a session
        // behaves the same as one followed in a browser that does not.
        return Redirect::to(destination_or_root(next)).into_response();
    }
    render(
        &state,
        SIGN_IN_TITLE,
        StatusCode::OK,
        editor_web::pages::login::request_code(next, None),
    )
}

/// `POST /login` — issue a code, or convincingly appear to.
pub(crate) async fn login_submit(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Form(form): Form<EmailForm>,
) -> Response {
    let next = next_from(&uri);
    let email = form.email.trim();
    if !is_plausible_address(email) {
        // Worth a line: this is the only `POST /login` that answers differently
        // from every other, so it is the one an operator can count. Only that it
        // could not be parsed is logged, never the value.
        tracing::info!(
            auth.outcome = "malformed_address",
            "a login was submitted with an unusable address"
        );
        return render(
            &state,
            SIGN_IN_TITLE,
            StatusCode::BAD_REQUEST,
            editor_web::pages::login::request_code(next, Some(EMAIL_REJECTED)),
        );
    }

    let presented = cookie::read(&headers, cookie::LOGIN);
    let token = issue(&state, email, presented.as_deref(), Utc::now()).await;

    // Unconditional. Whether a cookie comes back must not depend on anything the
    // request revealed about the address — see [`issue`]. The destination is
    // taken from the request, never from the outcome, so it cannot vary either.
    with_cookie(
        Redirect::to(&code_url(next)).into_response(),
        cookie::set(cookie::LOGIN, &token, CODE_TTL),
    )
}

/// The code this browser's binding owns, when this deployment may show it.
///
/// Returns `None` unless [`AppState::reveal_login_code`] is set — resolved once
/// at startup from three conditions, all of which have to hold: no mail relay,
/// no database that outlives the process, and not `PROD`. See
/// [`crate::config::EditorConfig::reveals_login_code`].
///
/// Only ever the code bound to the token the request carries, so a browser sees
/// what it asked for and nothing else. That is still a real disclosure — an
/// attacker who starts a sign-in for somebody else's address sees the code for
/// it — which is why the gate is what it is and not a convenience flag.
///
/// **It costs REQ-6.2 on this page.** The code-entry screen now differs between
/// an address with an account and one without, because a binding that resolves
/// to nothing shows nothing. That is inherent to showing a code at all, it is
/// confined to the same throwaway condition, and it is recorded in
/// `docs/src/editor/authentication.md` rather than left to be discovered.
///
/// A storage failure yields `None`: a screen that cannot show the code is a
/// nuisance on a preview, and guessing is not an option for a credential.
async fn revealed_code(state: &AppState, token: &str, now: DateTime<Utc>) -> Option<String> {
    if !state.reveal_login_code {
        return None;
    }
    match LoginCodeRepository::find_by_browser_token(&state.db, token).await {
        Ok(Some(code)) if code.consumed_at.is_none() && now < code.expires_at => Some(code.code),
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(error = %error, "could not read the code to show on screen");
            None
        }
    }
}

/// `GET /login/code` — the code form.
pub(crate) async fn code_form(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let next = next_from(&uri);
    if session::current(&state.db, &state.auth, &headers, Utc::now()).await.is_some() {
        return Redirect::to(destination_or_root(next)).into_response();
    }
    let Some(token) = cookie::read(&headers, cookie::LOGIN) else {
        // Nothing has been asked for in this browser, so there is nothing to
        // enter. Sending the user to the form they skipped reveals nothing: the
        // cookie is set for every address, known or not.
        return Redirect::to(&login_url(next)).into_response();
    };
    let revealed = revealed_code(&state, &token, Utc::now()).await;
    render(
        &state,
        ENTER_CODE_TITLE,
        StatusCode::OK,
        editor_web::pages::login::enter_code(next, None, revealed.as_deref()),
    )
}

/// `POST /login/code` — spend the code.
pub(crate) async fn code_submit(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Form(form): Form<CodeForm>,
) -> Response {
    let next = next_from(&uri);
    let Some(token) = cookie::read(&headers, cookie::LOGIN) else {
        // Silent until now, and this is exactly the symptom of the `__Host-`
        // cookie being refused — a proxy stripping `Set-Cookie`, a browser
        // blocking it, or the page served over plain HTTP. The user submits a
        // valid code, lands back on the form, and the log says nothing.
        tracing::info!(
            auth.outcome = "no_binding",
            "a code was submitted by a browser carrying no binding"
        );
        return Redirect::to(&login_url(next)).into_response();
    };

    match verify(&state, &headers, &token, form.code.trim(), Utc::now()).await {
        Ok(session) => {
            // Re-validated at the point of use rather than trusted from the
            // query it crossed: this is the one redirect a signed-in browser
            // follows, so it is the one worth checking twice.
            let response = Redirect::to(destination_or_root(next)).into_response();
            let response =
                with_cookie(response, cookie::set(cookie::SESSION, &session.id, state.auth.session_absolute));
            // The binding has done its job, and a login cookie left behind would
            // outlive the code it addresses.
            with_cookie(response, cookie::clear(cookie::LOGIN))
        }
        // 200 rather than 401: this is a form being redisplayed, and a 401
        // without `WWW-Authenticate` is not a conformant response. The outcome
        // is in the log, which is where alerting reads it from.
        // Shown again on a rejected entry: the reader mistyped a code they can
        // see, and hiding it now would be the one moment it is actually needed.
        Err(()) => {
            let revealed = revealed_code(&state, &token, Utc::now()).await;
            render(
                &state,
                ENTER_CODE_TITLE,
                StatusCode::OK,
                editor_web::pages::login::enter_code(next, Some(CODE_REJECTED), revealed.as_deref()),
            )
        }
    }
}

/// `POST /logout` (REQ-6.6).
///
/// Idempotent: a sign-out with no session still clears the cookie and lands on
/// the login page, so a stale tab does not produce an error page.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "auth sign_out",
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let span = tracing::Span::current();
    match session::end(&state.db, &headers).await {
        session::Ended::Deleted(user_id) => {
            span.record("auth.subject", tracing::field::display(user_id));
            span.record("auth.outcome", "signed_out");
            tracing::info!("signed out");
        }
        session::Ended::NoSession => {
            span.record("auth.outcome", "no_session");
            tracing::info!("sign-out with no live session");
        }
        // The cookie is cleared either way, so the browser is signed out — but
        // the row survives and will authenticate anyone still holding the token
        // until it expires. Reporting that as `no_session` asserted the opposite
        // of what happened.
        session::Ended::Failed => {
            span.record("auth.outcome", "end_failed");
            tracing::error!("sign-out could not delete the session; it stays live until it expires");
        }
    }
    with_cookie(Redirect::to("/login").into_response(), cookie::clear(cookie::SESSION))
}

/// Issue a code for `email` and return the token the browser's login cookie must
/// now carry.
///
/// **Always a token, on every path.** That is the anti-enumeration property, and
/// the first version of this got it wrong in a way worth spelling out: it set a
/// cookie only when a code had been issued *or* the browser presented none, on
/// the reasoning that a browser already holding a binding should be left alone.
/// But an attacker supplies the presented cookie themselves — any non-empty
/// value will do — so "known address" answered with a `Set-Cookie` and "unknown
/// address" answered without one. One request per address, no timing needed,
/// REQ-6.2 gone.
///
/// So the token is minted before anything is looked up and handed back whatever
/// happens. When no code was issued, a binding the browser already owns is moved
/// onto the new token, so the code it owns stays spendable; when it owns
/// nothing, the token binds to nothing. Both produce the same response.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "auth issue_code",
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
        auth.rebind = tracing::field::Empty,
    )
)]
async fn issue(state: &AppState, email: &str, presented: Option<&str>, now: DateTime<Utc>) -> String {
    let token = secret::token();
    if issue_to_account(state, email, &token, now).await {
        return token;
    }

    // Nothing was issued — unknown address, cooldown, lockout, daily cap or a
    // failure. Carry an existing binding across so a legitimate user re-posting
    // their address does not strand the code they are holding. `presented` is
    // the authorisation: only a browser that already has the token can move it.
    if let Some(presented) = presented {
        match LoginCodeRepository::rebind_browser_token(&state.db, presented, &token).await {
            Ok(moved) => {
                tracing::Span::current().record("auth.rebind", if moved { "moved" } else { "nothing_to_move" });
            }
            Err(error) => {
                // Its own field rather than the outcome: the outcome already
                // says why no code was issued, and overwriting it would
                // attribute this failure to nothing.
                tracing::Span::current().record("auth.rebind", "failed");
                tracing::error!(error = %error, "could not move an existing binding onto the new token");
                // Leave the browser on the binding it already holds. Handing it
                // the new token would strand a legitimate user: the cookie would
                // address nothing while their live code still pointed at the old
                // token, so every code they entered would be refused until the
                // cooldown ran out. The response is unchanged in shape — a token
                // of the same length comes back either way.
                return presented.to_string();
            }
        }
    }
    token
}

/// The half of [`issue`] that may actually send something. `true` means a code
/// was issued carrying `token`.
async fn issue_to_account(state: &AppState, email: &str, token: &str, now: DateTime<Utc>) -> bool {
    let span = tracing::Span::current();
    let outcome = |value: &'static str| span.record("auth.outcome", value);

    let user = match UserRepository::find_by_email(&state.db, email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // REQ-6.2. Nothing stored, nothing sent, and no identifier logged:
            // there is no account to correlate against, and the address itself
            // may never reach a log (REQ-6.10).
            outcome("unknown_address");
            tracing::info!("a login code was requested for an address with no account");
            return false;
        }
        Err(error) => {
            outcome("lookup_failed");
            tracing::error!(error = %error, "could not look up the account for a login request");
            return false;
        }
    };
    // The correlation id (REQ-6.10). The account's own primary key is already
    // opaque — a v4 UUID, not derived from the address — so a second identifier
    // would be one more thing to keep in step for no gain.
    span.record("auth.subject", tracing::field::display(user.id));

    if locked_out(&user, &state.auth, now) {
        // No code is issued while an account is throttled. Issuing one would
        // spend relay quota on an account that cannot use it, which is exactly
        // what an attacker driving the counter up wants.
        outcome("locked_out");
        tracing::warn!(
            failed_logins = user.failed_logins,
            "refused to issue a login code: the account is throttled after consecutive failures"
        );
        return false;
    }

    match LoginCodeRepository::count_issued_since(&state.db, now - TimeDelta::hours(24)).await {
        Ok(sent) if sent >= state.auth.daily_cap => {
            // Alarmed, not silent. This is either an attack looping resend
            // across the known addresses or a genuine surge, and both need
            // someone to look: the relay quota is shared, and exhausting it
            // locks out every user including RDU.
            outcome("daily_cap");
            tracing::error!(
                mail.sent_24h = sent,
                mail.daily_cap = state.auth.daily_cap,
                "refused to issue a login code: the global daily send cap is reached"
            );
            return false;
        }
        Ok(_) => {}
        Err(error) => {
            // Fail closed. A cap that cannot be read is a cap that is not being
            // enforced, and sending anyway is how a quota gets drained.
            outcome("cap_unreadable");
            tracing::error!(error = %error, "could not read the daily send count; refusing to issue a code");
            return false;
        }
    }

    let code = LoginCode {
        id: Uuid::new_v4(),
        user_id: user.id,
        code: secret::code(),
        attempts: 0,
        created_at: now,
        expires_at: now + delta(CODE_TTL),
        consumed_at: None,
        browser_token: Some(token.to_string()),
    };

    match LoginCodeRepository::create_unless_issued_since(&state.db, &code, now - delta(state.auth.cooldown)).await {
        Ok(Issued::New) => {}
        Ok(Issued::Cooled) => {
            outcome("cooldown");
            tracing::info!("refused to re-send a login code inside the cooldown");
            return false;
        }
        Err(error) => {
            outcome("store_failed");
            tracing::error!(error = %error, "could not store a login code");
            return false;
        }
    }

    let mail = Mail {
        to: user.email.clone(),
        subject: MAIL_SUBJECT.to_string(),
        body: mail_body(&code.code),
    };
    match state.mailer.send(&mail).await {
        Ok(()) => {
            stamp_issued(state, user.id, now).await;
            outcome("issued");
            tracing::info!("issued a login code");
            true
        }
        Err(error) => {
            // REQ-6.9. The error is a classification and an SMTP status code and
            // nothing else — a relay's reply text routinely quotes the recipient.
            tracing::error!(
                error = %error,
                "could not send a login code; set EDITOR_SMTP_BREAK_GLASS=true to write undelivered codes to the log \
                 while the relay is broken"
            );
            if state.auth.break_glass {
                // The code stays live, because the log is now the delivery
                // channel. Loud, because a live credential is in the logs.
                tracing::error!(
                    login.code = %code.code,
                    "EDITOR_SMTP_BREAK_GLASS is on: writing an undelivered login code to the log"
                );
                stamp_issued(state, user.id, now).await;
                outcome("send_failed_break_glass");
                true
            } else {
                // Roll the code back, and the cooldown with it. Leaving it would
                // make the user wait out a cooldown for a code they never
                // received, with REQ-6.5 refusing to send another.
                if let Err(error) = LoginCodeRepository::delete(&state.db, code.id).await {
                    tracing::error!(error = %error, "could not roll back a login code whose delivery failed");
                }
                outcome("send_failed");
                false
            }
        }
    }
}

/// Stamp when this account last had a code issued, so RDU can answer "I never
/// got a code" without an address reaching a log (REQ-6.10).
async fn stamp_issued(state: &AppState, user_id: Uuid, now: DateTime<Utc>) {
    if let Err(error) = UserRepository::record_code_issued(&state.db, user_id, now).await {
        tracing::warn!(error = %error, "could not stamp when a login code was issued");
    }
}

/// Spend a code, and return the session it bought.
///
/// Every failure is `Err(())`: the caller has one message for all of them, and
/// the distinction lives in the log rather than in the response.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "auth verify_code",
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
    )
)]
async fn verify(
    state: &AppState,
    headers: &HeaderMap,
    token: &str,
    submitted: &str,
    now: DateTime<Utc>,
) -> Result<Session, ()> {
    let span = tracing::Span::current();
    let outcome = |value: &'static str| span.record("auth.outcome", value);

    let code = match LoginCodeRepository::find_by_browser_token(&state.db, token).await {
        Ok(Some(code)) => code,
        Ok(None) => {
            // Either an unknown address (whose browser was handed a token that
            // binds to nothing, by design) or a code that has been cleaned up.
            outcome("unknown_binding");
            tracing::info!("a code was submitted against a binding that matches no code");
            return Err(());
        }
        Err(error) => {
            outcome("lookup_failed");
            tracing::error!(error = %error, "could not look up the code for a binding");
            return Err(());
        }
    };
    span.record("auth.subject", tracing::field::display(code.user_id));

    if code.consumed_at.is_some() {
        outcome("code_consumed");
        tracing::warn!("a code that has already been used was submitted again");
        return Err(());
    }
    if now >= code.expires_at {
        outcome("code_expired");
        tracing::info!("an expired code was submitted");
        return Err(());
    }
    let user = match UserRepository::find_by_id(&state.db, code.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            outcome("account_gone");
            tracing::warn!("a code was submitted for an account that no longer exists");
            return Err(());
        }
        Err(error) => {
            outcome("lookup_failed");
            tracing::error!(error = %error, "could not look up the account behind a code");
            return Err(());
        }
    };

    if locked_out(&user, &state.auth, now) {
        outcome("locked_out");
        tracing::warn!(
            failed_logins = user.failed_logins,
            "refused a code: the account is throttled after consecutive failures"
        );
        return Err(());
    }

    // REQ-6.4's three strikes, claimed before anything is compared. The claim is
    // the limit: reading `attempts` and incrementing it afterwards let every
    // simultaneous submission past the check at once, which is what made twenty
    // parallel guesses cost three strikes' worth of budget and none of the
    // protection. A refusal here also covers a code consumed in a parallel
    // request.
    match LoginCodeRepository::claim_attempt(&state.db, code.id, secret::MAX_CODE_ATTEMPTS).await {
        Ok(Attempt::Claimed) => {}
        Ok(Attempt::Exhausted) => {
            // REQ-6.4 doing its job.
            outcome("code_invalidated");
            tracing::warn!("a code with its three attempts spent was submitted");
            return Err(());
        }
        Ok(Attempt::AlreadySpent) => {
            // Usually one person with two tabs open, not somebody guessing —
            // kept apart from `code_invalidated` so support is not sent after a
            // mistyping problem that does not exist.
            outcome("code_consumed");
            tracing::info!("a code was submitted that had already been spent");
            return Err(());
        }
        Ok(Attempt::Unknown) => {
            outcome("code_vanished");
            tracing::warn!("a code resolved by its binding was gone by the time an attempt was claimed");
            return Err(());
        }
        Err(error) => {
            outcome("claim_failed");
            tracing::error!(error = %error, "could not claim an attempt against the code");
            return Err(());
        }
    }

    if !secret::code_matches(submitted, &code.code) {
        // The per-code strike is already spent by the claim above. The
        // account-level counter is the one that survives invalidation and resend,
        // and it decays over the lockout window so a capped account is not
        // re-locked forever by one attempt per window.
        match UserRepository::record_failed_login(&state.db, user.id, now, now - delta(state.auth.lockout)).await {
            Ok(failed) => {
                outcome("wrong_code");
                tracing::warn!(failed_logins = failed, "a wrong code was submitted");
            }
            Err(error) => {
                outcome("wrong_code");
                tracing::error!(error = %error, "could not count a wrong entry against the account");
            }
        }
        return Err(());
    }

    // Single use, decided by the store: the `consumed_at IS NULL` in the update
    // is what stops two simultaneous submissions of one code both winning.
    match LoginCodeRepository::consume(&state.db, code.id, now).await {
        Ok(true) => {}
        Ok(false) => {
            outcome("replayed");
            tracing::warn!("a correct code lost the race to be consumed, so it was already spent");
            return Err(());
        }
        Err(error) => {
            outcome("consume_failed");
            tracing::error!(error = %error, "could not consume a correct code");
            return Err(());
        }
    }

    // Only a success clears the account counter (NIST SP 800-63B-4).
    if let Err(error) = UserRepository::clear_failed_logins(&state.db, user.id).await {
        tracing::warn!(error = %error, "could not clear the account's failure counter after a successful sign-in");
    }
    // The account's *unspent* codes are now noise bound to browsers nobody is
    // using. The one just consumed stays: it is the resend cooldown's only
    // anchor and the daily send cap's only evidence, and deleting it let a user
    // sign in and immediately be sent another code.
    if let Err(error) = LoginCodeRepository::delete_unconsumed_for_user(&state.db, user.id).await {
        tracing::warn!(error = %error, "could not clear the account's remaining login codes");
    }
    // Session fixation: whatever session this browser arrived with is deleted
    // rather than reused, so a value planted before authentication cannot
    // survive into it.
    let _ = session::end(&state.db, headers).await;

    let session = match session::begin(&state.db, &state.auth, user.id, now).await {
        Ok(session) => session,
        Err(error) => {
            // The code is already spent and bought nothing. Left that way the
            // user is told it was invalid, re-entering it fails, and the
            // cooldown refuses them another — locked out for up to a cooldown by
            // an error that was never theirs. Nobody was authenticated, so
            // reopening it is free.
            outcome("session_failed");
            tracing::error!(error = %error, "could not create a session for a correct code");
            if let Err(error) = LoginCodeRepository::unconsume(&state.db, code.id).await {
                tracing::error!(
                    error = %error,
                    "could not reopen the code after a failed sign-in; the user cannot retry until it expires"
                );
            }
            return Err(());
        }
    };

    outcome("signed_in");
    tracing::info!("signed in");
    Ok(session)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::Request;
    use axum::Router;
    use editor_core::records::Role;
    use editor_core::repository::SessionRepository;
    use tower::ServiceExt;

    use super::*;
    use crate::auth::cookie;
    use crate::test_support::{
        body_string, capture_logs, cookie_set, count_rows, get, location, post, state_with, test_app, test_state,
        urlencode, with_cookie, RecordingMailer,
    };

    const KNOWN: &str = "depositor@example.test";
    const UNKNOWN: &str = "nobody@example.test";

    async fn a_user(state: &AppState, email: &str) -> Uuid {
        crate::test_support::a_user(state, email, "A Depositor", Role::Depositor, &[])
            .await
            .id
    }

    /// `POST /login` for `email`, carrying `binding` as the login cookie if given.
    async fn request_code(app: &Router, email: &str, binding: Option<&str>) -> Response {
        let form = format!("email={}", urlencode(email));
        let mut request = post("/login", &form);
        if let Some(binding) = binding {
            request = with_cookie(request, cookie::LOGIN, binding);
        }
        app.clone().oneshot(request).await.expect("the request should complete")
    }

    /// `POST /login/code` with `code`, bound by `binding`.
    async fn submit_code(app: &Router, code: &str, binding: &str) -> Response {
        let request = with_cookie(post("/login/code", &format!("code={code}")), cookie::LOGIN, binding);
        app.clone().oneshot(request).await.expect("the request should complete")
    }

    /// Drive a full sign-in and return the session cookie.
    async fn sign_in(app: &Router, mailer: &RecordingMailer, email: &str) -> String {
        let issued = request_code(app, email, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding must be handed out");
        let code = mailer.last_code().expect("a code must have been sent");
        let response = submit_code(app, &code, &binding).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "sign-in should redirect");
        cookie_set(&response, cookie::SESSION).expect("a session cookie must be set")
    }

    // ---- REQ-6.2: the response tells an attacker nothing ---------------------

    #[tokio::test]
    async fn test_a_known_and_an_unknown_address_get_byte_for_byte_the_same_answer() {
        // The whole anti-enumeration property in one assertion. Status, location
        // and whether a cookie is set must all match; only the mail differs, and
        // only the address's owner sees that.
        let (state, mailer) = test_state("enumerate").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let known = request_code(&app, KNOWN, None).await;
        let unknown = request_code(&app, UNKNOWN, None).await;

        assert_eq!(known.status(), unknown.status());
        assert_eq!(location(&known), location(&unknown));
        assert_eq!(location(&known).as_deref(), Some("/login/code"));
        // Both hand out a binding, and the two are different tokens of the same
        // shape — one addresses a code, the other addresses nothing.
        let known_binding = cookie_set(&known, cookie::LOGIN).expect("known address gets a binding");
        let unknown_binding = cookie_set(&unknown, cookie::LOGIN).expect("unknown address gets one too");
        assert_eq!(known_binding.len(), unknown_binding.len());
        assert_ne!(known_binding, unknown_binding);

        assert_eq!(mailer.sent().len(), 1, "exactly one address has an account");
        assert_eq!(mailer.sent()[0].to, KNOWN);
    }

    #[tokio::test]
    async fn test_a_second_post_is_as_quiet_for_a_known_address_as_for_an_unknown_one() {
        // The subtle half. If the cookie were re-set only when a code was issued,
        // a second post inside the cooldown would go quiet for a known address
        // and not for an unknown one — and that difference is the enumeration
        // oracle the identical first response was supposed to close.
        let (state, _) = state_with("enumerate-twice", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::from_secs(60);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let known_first = request_code(&app, KNOWN, None).await;
        let known_binding = cookie_set(&known_first, cookie::LOGIN).unwrap();
        let unknown_first = request_code(&app, UNKNOWN, None).await;
        let unknown_binding = cookie_set(&unknown_first, cookie::LOGIN).unwrap();

        let known_again = request_code(&app, KNOWN, Some(&known_binding)).await;
        let unknown_again = request_code(&app, UNKNOWN, Some(&unknown_binding)).await;

        assert_eq!(known_again.status(), unknown_again.status());
        assert_eq!(location(&known_again), location(&unknown_again));

        // Both get a fresh binding. The earlier version of this handler set one
        // only when a code had been issued or the browser presented none, which
        // made "was a cookie set?" a one-request answer to "does this address
        // have an account?" — see `issue`.
        let known_cookie = cookie_set(&known_again, cookie::LOGIN).expect("a binding either way");
        let unknown_cookie = cookie_set(&unknown_again, cookie::LOGIN).expect("a binding either way");
        assert_eq!(known_cookie.len(), unknown_cookie.len());
        assert_ne!(known_cookie, unknown_cookie);
        assert_ne!(known_cookie, known_binding, "and it is a fresh token, not the one presented");
    }

    #[tokio::test]
    async fn test_a_forged_binding_cannot_tell_a_known_address_from_an_unknown_one() {
        // The attacker supplies the cookie themselves — `cookie::read` accepts any
        // non-empty value, so no prior request is needed. If whether a cookie
        // comes back depends on whether the address has an account, REQ-6.2 is
        // defeated in a single request, with no timing measurement.
        let (state, _) = test_state("enumerate-forged").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        const FORGED: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let known = request_code(&app, KNOWN, Some(FORGED)).await;
        let unknown = request_code(&app, UNKNOWN, Some(FORGED)).await;

        assert_eq!(known.status(), unknown.status());
        assert_eq!(location(&known), location(&unknown));
        assert_eq!(
            cookie_set(&known, cookie::LOGIN).is_some(),
            cookie_set(&unknown, cookie::LOGIN).is_some(),
            "whether a binding comes back must not depend on whether the address has an account"
        );
        assert_eq!(
            cookie_set(&known, cookie::LOGIN).map(|c| c.len()),
            cookie_set(&unknown, cookie::LOGIN).map(|c| c.len()),
            "and neither must its shape"
        );
    }

    #[tokio::test]
    async fn test_simultaneous_wrong_guesses_cannot_outrun_the_three_strike_limit() {
        // REQ-6.4 is one of exactly two controls standing between a ~19.93-bit
        // secret and a guesser. Reading `attempts` and then incrementing it
        // leaves a window in which every simultaneous submission passes the
        // check, so the limit has to BE the increment.
        //
        // The attacker holds a binding legitimately: they posted the victim's
        // address themselves, so the code is bound to their browser even though
        // it was mailed to the victim.
        let (state, mailer) = test_state("strikes-race").await;
        let user_id = a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let correct = mailer.last_code().expect("a code");
        let wrong = if correct == "000000" { "111111" } else { "000000" };

        let mut handles = Vec::new();
        for _ in 0..20 {
            let app = app.clone();
            let binding = binding.clone();
            let wrong = wrong.to_string();
            handles.push(tokio::spawn(async move {
                let request = with_cookie(post("/login/code", &format!("code={wrong}")), cookie::LOGIN, &binding);
                app.oneshot(request).await.expect("the request should complete").status()
            }));
        }
        for handle in handles {
            handle.await.expect("the task should not panic");
        }

        let code = LoginCodeRepository::find_by_browser_token(&state.db, &binding)
            .await
            .expect("the lookup should succeed")
            .expect("the code should still be there");
        // Exactly the limit, not merely "no more than". A `<=` here would also
        // pass against an implementation that never counted at all.
        assert_eq!(
            code.attempts,
            secret::MAX_CODE_ATTEMPTS,
            "twenty simultaneous guesses must spend exactly the limit"
        );

        // The account counter is bounded by the same claim — at most three
        // comparisons ever happen — but not asserted exactly, and the reason is
        // worth writing down. Under `Source::Memory` SQLite uses shared-cache
        // *table* locks, so a reader transaction can make a concurrent write
        // return `SQLITE_LOCKED`, which `busy_timeout` does not retry; one of
        // the three writes then loses. Production uses WAL on a file database,
        // where readers never block writers, so the mechanism does not exist
        // there — see `db::Source`.
        let user = UserRepository::find_by_id(&state.db, user_id).await.unwrap().unwrap();
        assert!(
            (1..=secret::MAX_CODE_ATTEMPTS).contains(&user.failed_logins),
            "the account counter recorded {} failures from one code",
            user.failed_logins
        );
    }

    #[tokio::test]
    async fn test_an_unknown_address_stores_nothing_and_sends_nothing() {
        let (state, mailer) = test_state("unknown").await;
        let app = test_app(&state);

        request_code(&app, UNKNOWN, None).await;

        assert!(mailer.sent().is_empty(), "REQ-6.2: nothing may be sent");
        assert_eq!(count_rows(&state.db, "login_codes").await, 0);
        assert_eq!(count_rows(&state.db, "users").await, 0);
    }

    #[tokio::test]
    async fn test_a_binding_from_an_unknown_address_can_never_be_spent() {
        let (state, _) = test_state("unknown-binding").await;
        let app = test_app(&state);

        let response = request_code(&app, UNKNOWN, None).await;
        let binding = cookie_set(&response, cookie::LOGIN).unwrap();

        // Every code is wrong, because there is no code.
        let attempt = submit_code(&app, "000000", &binding).await;
        assert_eq!(attempt.status(), StatusCode::OK);
        assert!(body_string(attempt).await.contains("not valid"));
    }

    // ---- The happy path -----------------------------------------------------

    #[tokio::test]
    async fn test_a_code_signs_the_user_in_and_rotates_the_cookies() {
        let (state, mailer) = test_state("sign-in").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let code = mailer.last_code().expect("a six-digit code should have been sent");
        assert_eq!(code.len(), 6);

        let signed_in = submit_code(&app, &code, &binding).await;
        assert_eq!(signed_in.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&signed_in).as_deref(), Some("/"));
        let session = cookie_set(&signed_in, cookie::SESSION).expect("a session cookie");
        assert!(!session.is_empty());
        assert_eq!(
            cookie_set(&signed_in, cookie::LOGIN).as_deref(),
            Some(""),
            "the binding has done its job and must not outlive the code it addressed"
        );

        // And the session actually works: `/projects` is behind the guard, so
        // reaching it at all is the proof, and the header names the account.
        let projects = app
            .clone()
            .oneshot(with_cookie(get("/projects"), cookie::SESSION, &session))
            .await
            .unwrap();
        assert_eq!(projects.status(), StatusCode::OK);
        assert!(body_string(projects).await.contains("A Depositor"));
    }

    #[tokio::test]
    async fn test_the_destination_survives_the_whole_sign_in_and_is_where_the_user_lands() {
        // The point of `next`: someone who follows a link into a project and is
        // sent to sign in ends up on the project, not on the root wondering
        // where they were going.
        let (state, mailer) = test_state("sign-in-next").await;
        let _ = crate::test_support::a_user(&state, KNOWN, "A Depositor", Role::Depositor, &["0801"]).await;
        let app = test_app(&state);

        // The guard names the destination.
        let guarded = app.clone().oneshot(get("/projects/0801")).await.unwrap();
        assert_eq!(location(&guarded).as_deref(), Some("/login?next=/projects/0801"));

        // The address form carries it on its action.
        let form = app.clone().oneshot(get("/login?next=/projects/0801")).await.unwrap();
        assert!(body_string(form).await.contains(r#"action="/login?next=/projects/0801""#));

        // Issuing carries it on to the code screen.
        let issued = app
            .clone()
            .oneshot(post("/login?next=/projects/0801", &format!("email={}", urlencode(KNOWN))))
            .await
            .unwrap();
        assert_eq!(location(&issued).as_deref(), Some("/login/code?next=/projects/0801"));
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        // And spending the code lands on it.
        let signed_in = app
            .clone()
            .oneshot(with_cookie(
                post("/login/code?next=/projects/0801", &format!("code={code}")),
                cookie::LOGIN,
                &binding,
            ))
            .await
            .unwrap();
        assert_eq!(location(&signed_in).as_deref(), Some("/projects/0801"));
        let session = cookie_set(&signed_in, cookie::SESSION).expect("a session");

        let arrived = app
            .clone()
            .oneshot(with_cookie(get("/projects/0801"), cookie::SESSION, &session))
            .await
            .unwrap();
        assert_eq!(arrived.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_a_destination_pointing_off_site_is_dropped_rather_than_followed() {
        // The reachable form of the open redirect: the query is whatever a link
        // in a phishing mail put there, and this is the one redirect a browser
        // that has just authenticated will follow.
        let (state, mailer) = test_state("sign-in-open-redirect").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = app
            .clone()
            .oneshot(post("/login?next=https://evil.example", &format!("email={}", urlencode(KNOWN))))
            .await
            .unwrap();
        assert_eq!(
            location(&issued).as_deref(),
            Some("/login/code"),
            "a destination outside the service must not be carried"
        );
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        let signed_in = app
            .clone()
            .oneshot(with_cookie(
                post("/login/code?next=https://evil.example", &format!("code={code}")),
                cookie::LOGIN,
                &binding,
            ))
            .await
            .unwrap();
        assert_eq!(
            location(&signed_in).as_deref(),
            Some("/"),
            "and must not be followed even when it survives to the last step"
        );
    }

    #[tokio::test]
    async fn test_an_already_signed_in_visitor_is_sent_to_the_destination_rather_than_the_root() {
        // A link followed in a browser that still has a session should behave
        // like one followed in a browser that does not.
        let (state, mailer) = test_state("already-signed-in").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        for uri in ["/login?next=/projects/0801", "/login/code?next=/projects/0801"] {
            let response = app
                .clone()
                .oneshot(with_cookie(get(uri), cookie::SESSION, &session))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{uri}");
            assert_eq!(location(&response).as_deref(), Some("/projects/0801"), "{uri}");
        }
    }

    // ---- The development code reveal ---------------------------------------

    /// App state shaped like the PR preview: no relay, no volume, reveal on.
    async fn revealing_state(label: &str) -> (AppState, RecordingMailer) {
        let (mut state, mailer) = test_state(label).await;
        state.reveal_login_code = true;
        (state, mailer)
    }

    #[tokio::test]
    async fn test_a_throwaway_deployment_shows_the_code_on_the_page() {
        // The whole point: a reviewer opens the preview, types the address, and
        // the code is in front of them. No log access, nothing to be told.
        let (state, mailer) = revealing_state("reveal-on").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        let page = app
            .clone()
            .oneshot(with_cookie(get("/login/code"), cookie::LOGIN, &binding))
            .await
            .unwrap();
        let body = body_string(page).await;
        assert!(body.contains(&code), "the code must be on the page: {body}");
        assert!(body.contains("no mail relay"), "and it must say why: {body}");

        // And it is the real code, not a fixed one — it still signs the user in.
        let signed_in = submit_code(&app, &code, &binding).await;
        assert_eq!(signed_in.status(), StatusCode::SEE_OTHER);
        assert!(cookie_set(&signed_in, cookie::SESSION).is_some());
    }

    #[tokio::test]
    async fn test_the_ordinary_deployment_shows_nothing() {
        // Every deployment that mails a code renders this same page. The block
        // appearing where it should not is the failure that matters.
        let (state, mailer) = test_state("reveal-off").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        let page = app
            .clone()
            .oneshot(with_cookie(get("/login/code"), cookie::LOGIN, &binding))
            .await
            .unwrap();
        let body = body_string(page).await;
        assert!(!body.contains(&code), "the code must not reach the page: {body}");
        assert!(!body.contains("no mail relay"), "{body}");
    }

    #[tokio::test]
    async fn test_a_browser_sees_only_the_code_its_own_binding_owns() {
        // The lookup is by the token the request carries, so one browser cannot
        // read another's code by visiting the page.
        let (state, mailer) = revealing_state("reveal-scope").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        // A second browser inventing a token sees nothing.
        let other = app
            .clone()
            .oneshot(with_cookie(get("/login/code"), cookie::LOGIN, "a-token-that-binds-to-nothing"))
            .await
            .unwrap();
        let body = body_string(other).await;
        assert!(!body.contains(&code), "another browser must not see it: {body}");
    }

    #[tokio::test]
    async fn test_a_spent_or_expired_code_is_not_shown() {
        // Once used, it authenticates nobody — putting it back on a page would
        // be a credential on screen that buys nothing.
        let (state, mailer) = revealing_state("reveal-spent").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");
        submit_code(&app, &code, &binding).await;

        let page = app
            .clone()
            .oneshot(with_cookie(get("/login/code"), cookie::LOGIN, &binding))
            .await
            .unwrap();
        assert!(!body_string(page).await.contains(&code));
    }

    #[tokio::test]
    async fn test_a_rejected_entry_shows_the_code_again() {
        // The reader mistyped a code they can see; hiding it now is the one
        // moment it is actually needed.
        let (state, mailer) = revealing_state("reveal-retry").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        let wrong = submit_code(&app, "000000", &binding).await;
        assert_eq!(wrong.status(), StatusCode::OK);
        let body = body_string(wrong).await;
        assert!(body.contains(&code), "{body}");
        assert!(body.contains(CODE_REJECTED), "{body}");
    }

    #[tokio::test]
    async fn test_the_reveal_puts_no_address_in_a_log() {
        // REQ-6.10 is unaffected by any of this: the code reaches the page, the
        // address reaches neither the page nor a log.
        let (state, _) = revealing_state("reveal-no-address").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let (logs, guard) = capture_logs();
        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).expect("a binding");
        let page = app
            .clone()
            .oneshot(with_cookie(get("/login/code"), cookie::LOGIN, &binding))
            .await
            .unwrap();
        let body = body_string(page).await;
        drop(guard);

        let lines = logs.lines();
        assert!(!lines.is_empty(), "the capture itself must be working");
        assert!(!lines.iter().any(|line| line.contains(KNOWN)), "{lines:?}");
        assert!(!body.contains(KNOWN), "the address stays off the page too: {body}");
    }

    #[tokio::test]
    async fn test_signing_in_clears_the_account_failure_counter() {
        // NIST SP 800-63B-4: only a success may reset it.
        let (state, mailer) = test_state("clear-counter").await;
        let user_id = a_user(&state, KNOWN).await;
        UserRepository::record_failed_login(&state.db, user_id, Utc::now(), Utc::now() - TimeDelta::hours(1))
            .await
            .unwrap();
        let app = test_app(&state);

        sign_in(&app, &mailer, KNOWN).await;

        let user = UserRepository::find_by_id(&state.db, user_id).await.unwrap().unwrap();
        assert_eq!(user.failed_logins, 0);
        assert_eq!(user.failed_login_at, None);
    }

    #[tokio::test]
    async fn test_signing_in_stamps_when_the_code_was_issued() {
        // REQ-6.10's diagnosis route: RDU answers "I never got a code" from this
        // rather than from a log with an address in it.
        let (state, _) = test_state("stamp").await;
        let user_id = a_user(&state, KNOWN).await;
        let app = test_app(&state);

        request_code(&app, KNOWN, None).await;

        let user = UserRepository::find_by_id(&state.db, user_id).await.unwrap().unwrap();
        assert!(user.last_code_at.is_some());
    }

    // ---- Guessing, replay and binding --------------------------------------

    #[tokio::test]
    async fn test_a_wrong_code_counts_against_both_the_code_and_the_account() {
        let (state, mailer) = test_state("wrong-code").await;
        let user_id = a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let correct = mailer.last_code().unwrap();
        let wrong = if correct == "000000" { "111111" } else { "000000" };

        let response = submit_code(&app, wrong, &binding).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("not valid"));

        let user = UserRepository::find_by_id(&state.db, user_id).await.unwrap().unwrap();
        assert_eq!(user.failed_logins, 1, "the account counter must move");
        assert!(user.failed_login_at.is_some(), "and carry the instant a lockout measures from");
        let code = LoginCodeRepository::find_active_for_user(&state.db, user_id, Utc::now())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code.attempts, 1, "and so must the per-code counter");
    }

    #[tokio::test]
    async fn test_three_wrong_entries_kill_the_code_even_for_the_right_digits() {
        // REQ-6.4. The check is before the comparison, so the fourth attempt
        // fails whatever it carries.
        let (state, mailer) = test_state("three-strikes").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let correct = mailer.last_code().unwrap();
        let wrong = if correct == "000000" { "111111" } else { "000000" };

        for _ in 0..3 {
            assert_eq!(submit_code(&app, wrong, &binding).await.status(), StatusCode::OK);
        }
        let with_the_right_code = submit_code(&app, &correct, &binding).await;
        assert_eq!(
            with_the_right_code.status(),
            StatusCode::OK,
            "not a redirect — it must not sign in"
        );
        assert_eq!(cookie_set(&with_the_right_code, cookie::SESSION), None);
    }

    #[tokio::test]
    async fn test_a_code_cannot_be_spent_twice() {
        // Replay resistance, NIST §3.1.3.2.
        let (state, mailer) = test_state("replay").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let code = mailer.last_code().unwrap();

        assert_eq!(submit_code(&app, &code, &binding).await.status(), StatusCode::SEE_OTHER);
        let replayed = submit_code(&app, &code, &binding).await;
        assert_eq!(replayed.status(), StatusCode::OK);
        assert_eq!(cookie_set(&replayed, cookie::SESSION), None);
    }

    #[tokio::test]
    async fn test_a_code_is_useless_in_a_browser_that_did_not_ask_for_it() {
        // The interception defence: a code read out of the mailbox by anyone
        // else has no binding to go with it.
        let (state, mailer) = test_state("binding").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        request_code(&app, KNOWN, None).await;
        let code = mailer.last_code().unwrap();

        // Another browser: it has been through `POST /login` for some address, so
        // it holds a well-formed binding — just not this one.
        let other = request_code(&app, UNKNOWN, None).await;
        let other_binding = cookie_set(&other, cookie::LOGIN).unwrap();

        let stolen = submit_code(&app, &code, &other_binding).await;
        assert_eq!(stolen.status(), StatusCode::OK);
        assert_eq!(cookie_set(&stolen, cookie::SESSION), None);

        // With no binding at all it does not even reach the comparison.
        let bare = app.clone().oneshot(post("/login/code", &format!("code={code}"))).await.unwrap();
        assert_eq!(bare.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&bare).as_deref(), Some("/login"));
    }

    // ---- Quota and throttling ----------------------------------------------

    #[tokio::test]
    async fn test_the_cooldown_refuses_a_second_code_without_touching_the_first() {
        // REQ-6.5, and the reason the binding is not re-issued: the browser that
        // asked keeps the code it can actually spend.
        let (state, mailer) = state_with("cooldown", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::from_secs(60);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let first = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&first, cookie::LOGIN).unwrap();
        let code = mailer.last_code().unwrap();

        let second = request_code(&app, KNOWN, Some(&binding)).await;
        assert_eq!(mailer.sent().len(), 1, "the cooldown must stop the second mail");

        // The browser still gets a fresh token — the response has to look the
        // same as it would for an address with no account — and the code it
        // already owns follows it there, so the user is not stranded holding a
        // code they can no longer spend.
        let rotated = cookie_set(&second, cookie::LOGIN).expect("a binding comes back either way");
        assert_ne!(rotated, binding);
        assert_eq!(
            submit_code(&app, &code, &rotated).await.status(),
            StatusCode::SEE_OTHER,
            "the code already in the user's mailbox must still work"
        );
    }

    #[tokio::test]
    async fn test_a_binding_cannot_be_moved_by_a_browser_that_does_not_hold_it() {
        // The rebind is what lets every response carry a fresh cookie without
        // stranding a live code. Its authorisation is holding the old token, so
        // an attacker posting the victim's address must not acquire the binding
        // of the code on its way to them.
        let (state, mailer) = state_with("rebind-hostile", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::from_secs(600);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let victim = request_code(&app, KNOWN, None).await;
        let victim_binding = cookie_set(&victim, cookie::LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        // The attacker posts the same address inside the cooldown, presenting a
        // token they made up.
        let attacker = request_code(&app, KNOWN, Some("a-token-the-attacker-invented")).await;
        let attacker_binding = cookie_set(&attacker, cookie::LOGIN).expect("a binding either way");

        assert_eq!(
            submit_code(&app, &code, &attacker_binding).await.status(),
            StatusCode::OK,
            "the attacker's token must bind to nothing"
        );
        assert_eq!(
            submit_code(&app, &code, &victim_binding).await.status(),
            StatusCode::SEE_OTHER,
            "and the victim's binding must be untouched"
        );
    }

    #[tokio::test]
    async fn test_signing_in_does_not_reset_the_resend_cooldown() {
        // REQ-6.5 is measured from the last code *issued*, and the cooldown's
        // only anchor is the row itself. Deleting every code on a successful
        // sign-in therefore deleted the anchor: request, sign in, request again,
        // and a second mail went out immediately.
        let (state, mailer) = state_with("cooldown-after-signin", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::from_secs(600);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        sign_in(&app, &mailer, KNOWN).await;
        assert_eq!(mailer.sent().len(), 1);

        request_code(&app, KNOWN, None).await;
        assert_eq!(
            mailer.sent().len(),
            1,
            "a completed sign-in must not hand the account a fresh cooldown"
        );
    }

    #[tokio::test]
    async fn test_a_spent_code_still_counts_against_the_daily_cap() {
        // The cap counts rows in `login_codes`. Deleting them on sign-in made
        // every completed login invisible to the control protecting the shared
        // relay quota.
        let (state, mailer) = state_with("cap-after-signin", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.daily_cap = 2;
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        sign_in(&app, &mailer, KNOWN).await;
        sign_in(&app, &mailer, KNOWN).await;
        assert_eq!(mailer.sent().len(), 2, "two codes were sent");

        request_code(&app, KNOWN, None).await;
        assert_eq!(
            mailer.sent().len(),
            2,
            "the third must be refused: two codes were already sent inside the window"
        );
    }

    #[tokio::test]
    async fn test_the_global_daily_cap_stops_sending_and_says_so_loudly() {
        // The relay quota is shared, so an attacker looping resend across the
        // known addresses would otherwise lock out everyone, RDU included.
        let (state, mailer) = state_with("daily-cap", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.daily_cap = 1;
        })
        .await;
        a_user(&state, KNOWN).await;
        a_user(&state, "second@example.test").await;
        let app = test_app(&state);

        let (logs, _guard) = capture_logs();
        request_code(&app, KNOWN, None).await;
        let over = request_code(&app, "second@example.test", None).await;

        assert_eq!(mailer.sent().len(), 1, "the cap must hold");
        // Still indistinguishable from a success.
        assert_eq!(over.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&over).as_deref(), Some("/login/code"));
        assert!(
            logs.lines().iter().any(|line| line.contains("daily send cap")),
            "the cap must alarm rather than fail silently: {:?}",
            logs.lines()
        );
    }

    #[tokio::test]
    async fn test_a_throttled_account_gets_no_further_codes() {
        // Issuing to a locked-out account spends relay quota on someone who
        // cannot use it, which is exactly what an attacker driving the counter
        // up would like.
        let (state, mailer) = state_with("throttled", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.max_failed = 2;
            auth.lockout = Duration::from_secs(3600);
        })
        .await;
        let user_id = a_user(&state, KNOWN).await;
        UserRepository::record_failed_login(&state.db, user_id, Utc::now(), Utc::now() - TimeDelta::hours(1))
            .await
            .unwrap();
        UserRepository::record_failed_login(&state.db, user_id, Utc::now(), Utc::now() - TimeDelta::hours(1))
            .await
            .unwrap();
        let app = test_app(&state);

        let response = request_code(&app, KNOWN, None).await;

        assert!(mailer.sent().is_empty(), "a throttled account must not be sent codes");
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "and must not be told so");
        assert_eq!(location(&response).as_deref(), Some("/login/code"));
    }

    // ---- Delivery failure ---------------------------------------------------

    #[tokio::test]
    async fn test_a_failed_send_rolls_the_code_and_its_cooldown_back() {
        // Otherwise the user waits out a cooldown for a code they never got, and
        // REQ-6.5 refuses to send them another.
        let (state, mailer) = state_with("send-fails", RecordingMailer::failing(), |auth| {
            auth.cooldown = Duration::from_secs(600);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let response = request_code(&app, KNOWN, None).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "the user is told nothing different");
        assert_eq!(count_rows(&state.db, "login_codes").await, 0, "no code may be left behind");

        // And the cooldown went with it, so the next attempt is allowed through.
        request_code(&app, KNOWN, None).await;
        assert_eq!(mailer.sent().len(), 2, "the rolled-back cooldown must not block a retry");
    }

    #[tokio::test]
    async fn test_break_glass_keeps_the_code_alive_and_writes_it_to_the_log() {
        // A relay broken for hours otherwise locks out every user including RDU.
        // Off by default, because it puts a live credential in the log pipeline.
        let (state, mailer) = state_with("break-glass", RecordingMailer::failing(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.break_glass = true;
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let (logs, _guard) = capture_logs();
        let response = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&response, cookie::LOGIN).expect("the browser stays bound to the logged code");

        assert_eq!(count_rows(&state.db, "login_codes").await, 1, "the code must survive");
        assert_eq!(mailer.sent().len(), 1, "the relay was still tried first");

        let logged = logs
            .lines()
            .iter()
            .find_map(|line| {
                line.split("login.code=")
                    .nth(1)
                    .map(|rest| rest.chars().take(6).collect::<String>())
            })
            .expect("the undelivered code must reach the log, or break-glass delivers nothing");
        assert_eq!(
            logged,
            mailer.last_code().unwrap(),
            "the logged code is the one that was minted"
        );
        assert_eq!(submit_code(&app, &logged, &binding).await.status(), StatusCode::SEE_OTHER);
    }

    // ---- Sessions -----------------------------------------------------------

    #[tokio::test]
    async fn test_sign_out_deletes_the_session_and_clears_the_cookie() {
        let (state, mailer) = test_state("sign-out").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        let response = app
            .clone()
            .oneshot(with_cookie(post("/logout", ""), cookie::SESSION, &session))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/login"));
        assert_eq!(cookie_set(&response, cookie::SESSION).as_deref(), Some(""));
        assert_eq!(count_rows(&state.db, "sessions").await, 0, "REQ-6.6: the row goes too");
    }

    #[tokio::test]
    async fn test_sign_out_without_a_session_still_clears_and_redirects() {
        // A stale tab must land on the login page, not on an error.
        let (state, _) = test_state("sign-out-empty").await;
        let app = test_app(&state);

        let response = app.clone().oneshot(post("/logout", "")).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(cookie_set(&response, cookie::SESSION).as_deref(), Some(""));
    }

    #[tokio::test]
    async fn test_a_session_past_its_absolute_expiry_is_refused_and_deleted() {
        let (state, mailer) = test_state("absolute-expiry").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        // Age it past its absolute expiry, which is never extended.
        let expired = Utc::now() - chrono::TimeDelta::minutes(1);
        state
            .db
            .write({
                let id = session.clone();
                move |tx| {
                    tx.execute(
                        "UPDATE sessions SET expires_at = ?2 WHERE id = ?1",
                        rusqlite::params![id, expired],
                    )
                }
            })
            .await
            .unwrap();

        let refused = app
            .clone()
            .oneshot(with_cookie(get("/projects"), cookie::SESSION, &session))
            .await
            .unwrap();
        assert_eq!(
            refused.status(),
            StatusCode::SEE_OTHER,
            "an expired session must not reach a guarded page"
        );
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects"));
        assert_eq!(
            SessionRepository::find(&state.db, &session).await.unwrap(),
            None,
            "and must not linger"
        );
    }

    #[tokio::test]
    async fn test_an_idle_session_is_refused_even_before_its_absolute_expiry() {
        let (state, mailer) = state_with("idle-expiry", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.session_idle = Duration::from_secs(60);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        let stale = Utc::now() - chrono::TimeDelta::minutes(5);
        state
            .db
            .write({
                let id = session.clone();
                move |tx| {
                    tx.execute(
                        "UPDATE sessions SET last_seen_at = ?2 WHERE id = ?1",
                        rusqlite::params![id, stale],
                    )
                }
            })
            .await
            .unwrap();

        let refused = app
            .clone()
            .oneshot(with_cookie(get("/projects"), cookie::SESSION, &session))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects"));
        assert_eq!(SessionRepository::find(&state.db, &session).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_signing_in_replaces_the_session_the_browser_arrived_with() {
        // Fixation: a value planted before authentication must not survive into
        // the authenticated session.
        let (state, mailer) = test_state("fixation").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let first = sign_in(&app, &mailer, KNOWN).await;

        // Sign in again from the same browser, still carrying the first session.
        let issued = app
            .clone()
            .oneshot(with_cookie(
                post("/login", &format!("email={}", urlencode(KNOWN))),
                cookie::SESSION,
                &first,
            ))
            .await
            .unwrap();
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let code = mailer.last_code().unwrap();
        let response = app
            .clone()
            .oneshot(with_cookie(
                with_cookie(post("/login/code", &format!("code={code}")), cookie::LOGIN, &binding),
                cookie::SESSION,
                &first,
            ))
            .await
            .unwrap();

        let second = cookie_set(&response, cookie::SESSION).expect("a new session");
        assert_ne!(second, first, "the session id must be a fresh one");
        assert_eq!(
            SessionRepository::find(&state.db, &first).await.unwrap(),
            None,
            "and the one it arrived with must be gone, not merely unused"
        );
    }

    // ---- Method and CSRF discipline ----------------------------------------

    #[tokio::test]
    async fn test_every_state_changing_route_refuses_get() {
        // The `Sec-Fetch-Site` control exempts GET by necessity, so a
        // state-changing GET is a state-changing request nothing protects.
        let (state, _) = test_state("methods").await;
        let app = test_app(&state);

        assert_eq!(
            app.clone().oneshot(get("/logout")).await.unwrap().status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        // `/login` and `/login/code` answer GET with their forms, so the check
        // there is that the GET does not issue or spend anything.
        let (login_state, mailer) = test_state("methods-login").await;
        a_user(&login_state, KNOWN).await;
        let login_app = test_app(&login_state);
        login_app.clone().oneshot(get("/login")).await.unwrap();
        login_app.clone().oneshot(get("/login/code")).await.unwrap();
        assert!(mailer.sent().is_empty(), "a GET must not send a code");
        assert_eq!(count_rows(&login_state.db, "login_codes").await, 0);
        assert_eq!(count_rows(&login_state.db, "sessions").await, 0);
    }

    #[tokio::test]
    async fn test_the_login_posts_are_refused_without_sec_fetch_site() {
        let (state, mailer) = test_state("csrf").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        for (uri, form) in [
            ("/login", format!("email={}", urlencode(KNOWN))),
            ("/login/code", "code=123456".to_string()),
            ("/logout", String::new()),
        ] {
            let request = Request::builder()
                .method("POST")
                .uri(uri)
                .header("x-forwarded-for", "203.0.113.7")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "{uri} must be refused without the header"
            );
        }
        assert!(mailer.sent().is_empty());
    }

    // ---- Forms and navigation ----------------------------------------------

    #[tokio::test]
    async fn test_a_malformed_address_is_reported_rather_than_silently_accepted() {
        // With REQ-6.2's identical response, a typo is otherwise indistinguishable
        // from success and the user waits for mail that was never going anywhere.
        let (state, mailer) = test_state("malformed").await;
        let app = test_app(&state);

        let response = app.clone().oneshot(post("/login", "email=nobody")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(body_string(response).await.contains("valid email address"));
        assert!(mailer.sent().is_empty());
    }

    #[tokio::test]
    async fn test_the_code_page_sends_a_browser_that_never_asked_back_to_the_form() {
        let (state, _) = test_state("code-page-bare").await;
        let app = test_app(&state);

        let response = app.clone().oneshot(get("/login/code")).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/login"));
    }

    #[tokio::test]
    async fn test_a_signed_in_user_is_sent_home_from_the_login_screens() {
        let (state, mailer) = test_state("already-signed-in").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        for uri in ["/login", "/login/code"] {
            let response = app
                .clone()
                .oneshot(with_cookie(get(uri), cookie::SESSION, &session))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{uri}");
            assert_eq!(location(&response).as_deref(), Some("/"), "{uri}");
        }
    }

    // ---- REQ-6.10 -----------------------------------------------------------

    #[tokio::test]
    async fn test_no_address_reaches_a_log_or_a_span_on_any_path() {
        // REQ-6.10, over every branch that touches an address: unknown, issued,
        // wrong code, signed in, throttled, and a relay that refuses — the last
        // one especially, because an SMTP reply routinely quotes the recipient.
        let (state, mailer) = state_with("no-address-in-logs", RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO;
            // Low enough that one wrong code throttles the account, so the
            // locked-out branch is actually driven rather than merely claimed.
            auth.max_failed = 1;
            auth.lockout = Duration::from_secs(3600);
        })
        .await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let (logs, guard) = capture_logs();

        request_code(&app, UNKNOWN, None).await;

        // Signed in first, because success is what clears the failure counter —
        // driving the lockout before it would make this path unreachable.
        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, cookie::LOGIN).unwrap();
        let code = mailer.last_code().unwrap();
        let session = submit_code(&app, &code, &binding).await;
        let session = cookie_set(&session, cookie::SESSION).expect("signed in");
        app.clone()
            .oneshot(with_cookie(post("/logout", ""), cookie::SESSION, &session))
            .await
            .unwrap();

        // A wrong code, which with `max_failed = 1` throttles the account …
        let reissued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&reissued, cookie::LOGIN).unwrap();
        let code = mailer.last_code().unwrap();
        let wrong = if code == "000000" { "111111" } else { "000000" };
        submit_code(&app, wrong, &binding).await;
        // … so this one takes the locked-out branch, in both handlers.
        submit_code(&app, wrong, &binding).await;
        request_code(&app, KNOWN, None).await;

        app.clone().oneshot(post("/login", "email=nobody")).await.unwrap();
        app.clone().oneshot(post("/login/code", "code=123456")).await.unwrap();

        // A relay that refuses, on the same capture. The two branches this test
        // still cannot reach are `claim_failed` and the rebind failure: both need
        // an injected database error, and nothing here injects one.
        let (failing_state, _) = state_with("no-address-in-logs-failing", RecordingMailer::failing(), |auth| {
            auth.cooldown = Duration::ZERO;
        })
        .await;
        a_user(&failing_state, KNOWN).await;
        let failing_app = test_app(&failing_state);
        request_code(&failing_app, KNOWN, None).await;

        drop(guard);
        let lines = logs.lines();
        assert!(!lines.is_empty(), "the capture itself must be working");
        assert!(
            lines.iter().any(|line| line.contains("auth.subject")),
            "the opaque correlation id is what replaces the address: {lines:?}"
        );
        for line in &lines {
            for forbidden in [KNOWN, UNKNOWN, "example.test", "depositor", "nobody"] {
                assert!(
                    !line.to_lowercase().contains(forbidden),
                    "an address (or part of one) reached a log: {line}"
                );
            }
        }
    }

    #[test]
    fn test_the_mail_body_carries_the_code_and_no_address() {
        let body = mail_body("123456");
        assert!(body.contains("123456"), "{body}");
        assert!(body.contains("ten minutes"), "{body}");
        // The recipient knows their own address; putting it in the body would
        // make the message quotable as proof the account exists.
        assert!(!body.contains('@'), "{body}");
    }
}
