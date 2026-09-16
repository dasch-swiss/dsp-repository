//! Session lifecycle: minting one on a successful login, validating the one a
//! request carries, and ending it.

use std::time::Duration;

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use editor_core::records::{Session, User};
use editor_core::repository::{Repositories, RepositoryError, SessionRepository, UserRepository};
use uuid::Uuid;

use super::{cookie, delta, secret, AuthConfig};

/// How stale `last_seen_at` may get before a request writes it forward.
///
/// Advancing it on every request would put a write on the single writer
/// connection in front of every page view.
///
/// This is the **one** state change a `GET` performs, against the router's "no
/// `GET` mutates state" discipline. It is sound because the write carries nothing
/// the requester supplied, is idempotent, and touches the requester's own row, so
/// there is nothing for a cross-site request to achieve by triggering it.
const TOUCH_INTERVAL: Duration = Duration::from_secs(60);

/// Mint a session for `user_id`.
///
/// The id is a fresh 256-bit token every time, so nothing the browser held before
/// authentication can become the authenticated session id and a fixated pre-auth
/// value has nowhere to go.
pub(crate) async fn begin(
    db: &dyn SessionRepository,
    auth: &AuthConfig,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Session, RepositoryError> {
    let session = Session {
        id: secret::token(),
        user_id,
        created_at: now,
        last_seen_at: now,
        // Absolute, set once and never extended — that is the difference between
        // it and the idle timeout, and the reason a stolen cookie has a deadline
        // even while it is being used.
        expires_at: now + delta(auth.session_absolute),
    };
    SessionRepository::create(db, &session).await?;
    Ok(session)
}

/// The signed-in user, if the request carries a live session.
///
/// Fails closed on every error: a database that cannot answer produces `None`
/// rather than a guess.
pub(crate) async fn current(
    db: &dyn Repositories,
    auth: &AuthConfig,
    headers: &HeaderMap,
    now: DateTime<Utc>,
) -> Option<Viewer> {
    let id = cookie::read(headers, cookie::SESSION)?;

    let session = match SessionRepository::find(db, &id).await {
        Ok(Some(session)) => session,
        Ok(None) => return None,
        Err(error) => {
            tracing::warn!(error = %error, "could not read the session; treating the request as unauthenticated");
            return None;
        }
    };

    // Both deadlines delete the row: a session that can never be used again is
    // only a source of confusion in the table, and `delete_expired` runs on
    // nobody's schedule yet.
    let idle_deadline = session.last_seen_at + delta(auth.session_idle);
    if now >= session.expires_at || now >= idle_deadline {
        let _ = SessionRepository::delete(db, &id).await;
        return None;
    }

    let user = match UserRepository::find_by_id(db, session.user_id).await {
        Ok(Some(user)) => user,
        // The foreign key cascades, so a session without its user should be
        // impossible. Refusing rather than trusting it costs nothing and means
        // account removal cannot leave a usable session behind by any route.
        Ok(None) => {
            let _ = SessionRepository::delete(db, &id).await;
            return None;
        }
        Err(error) => {
            tracing::warn!(error = %error, "could not read the session's user; treating the request as unauthenticated");
            return None;
        }
    };

    if now - session.last_seen_at >= delta(TOUCH_INTERVAL) {
        if let Err(error) = SessionRepository::touch(db, &id, now).await {
            // Not fatal: the session is valid, and the only cost of a failed
            // touch is that the idle timeout measures from slightly further back.
            tracing::warn!(error = %error, "could not advance the session's last-seen time");
        }
    }

    // The earlier of the two deadlines is the one a page can honestly warn about:
    // the absolute expiry never moves, while the idle one advances on every
    // request, including an autosave.
    Some(Viewer { user, signed_out_at: session.expires_at.min(idle_deadline) })
}

/// Who is signed in, and when that stops being true.
#[derive(Debug, Clone)]
pub(crate) struct Viewer {
    pub(crate) user: User,
    /// The earlier of the absolute expiry and the idle timeout.
    pub(crate) signed_out_at: DateTime<Utc>,
}

/// What [`end`] did.
///
/// Three states rather than a boolean: "there was no session" and "the delete
/// failed" are opposite facts, and collapsing them makes the sign-out log assert
/// the session was already gone while the row is still there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ended {
    /// The row is gone. Carries the account it belonged to, so sign-out is
    /// traceable by the same opaque id as the rest of the flow.
    Deleted(Uuid),
    /// The request carried no session cookie, or none that matched a row.
    NoSession,
    /// The delete failed. The session is still live.
    Failed,
}

pub(crate) async fn end(db: &dyn SessionRepository, headers: &HeaderMap) -> Ended {
    let Some(id) = cookie::read(headers, cookie::SESSION) else {
        return Ended::NoSession;
    };
    // Read before delete so the account can be named in the log.
    let user_id = match SessionRepository::find(db, &id).await {
        Ok(Some(session)) => Some(session.user_id),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(error = %error, "could not read the session being signed out");
            None
        }
    };
    match SessionRepository::delete(db, &id).await {
        Ok(true) => user_id.map_or(Ended::NoSession, Ended::Deleted),
        Ok(false) => Ended::NoSession,
        Err(error) => {
            tracing::warn!(error = %error, "could not delete the session on sign-out");
            Ended::Failed
        }
    }
}
