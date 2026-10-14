//! The editor's extractors: who may reach an editor route, answered by identity through the
//! [`Authenticator`](editor_ports::Authenticator) port.
//! The editor never reads identity's tables: it sees a [`Principal`].

use std::time::SystemTime;

use axum::extract::FromRequestParts;
use axum::http::header::COOKIE;
use axum::http::request::Parts;
use axum::http::Method;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use deposit_shell::sign_in::login_url;
use editor_ports::Principal;
use uuid::Uuid;

use crate::shell::EditorState;

/// Every `Cookie` header value on the request, walking all of them rather than the first:
/// HTTP/2 clients are free to split them.
fn cookie_values(headers: &axum::http::HeaderMap) -> Vec<&str> {
    headers.get_all(COOKIE).iter().filter_map(|value| value.to_str().ok()).collect()
}

/// Where to send this request back to once its owner has signed in.
///
/// `GET` only: a `POST`'s destination is a side effect rather than a page, and
/// sending the browser to a `GET` of the same path after sign-in would look like
/// the write happened.
fn destination(parts: &Parts) -> Option<&str> {
    if parts.method != Method::GET {
        return None;
    }
    deposit_shell::sign_in::safe_next(parts.uri.path())
}

/// The signed-in account, as the editor's handlers use it.
#[derive(Debug, Clone)]
pub(crate) struct Account {
    /// [`Principal::account_id`], parsed once into the `Uuid` the editor's records hold.
    pub(crate) id: Uuid,
    pub(crate) principal: Principal,
}

impl Account {
    pub(crate) fn signed_out_at(&self) -> DateTime<Utc> {
        self.principal.signed_out_at.into()
    }
}

/// A live session, or the request never reaches the handler: it is sent to sign in.
#[derive(Debug, Clone)]
pub(crate) struct Authenticated(pub(crate) Account);

impl FromRequestParts<EditorState> for Authenticated {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &EditorState) -> Result<Self, Self::Rejection> {
        let sign_in = || Redirect::to(&login_url(destination(parts))).into_response();
        let Some(principal) = state
            .authenticator
            .authenticate(&cookie_values(&parts.headers), SystemTime::now())
            .await
        else {
            return Err(sign_in());
        };
        match Uuid::parse_str(&principal.account_id) {
            Ok(id) => Ok(Self(Account { id, principal })),
            // Identity mints every id as a `Uuid`, so this is a broken adapter rather than a
            // visitor's doing; failing closed keeps an id the records cannot hold off every write.
            Err(error) => {
                tracing::error!(error = %error, "the session's account id is not a uuid; treating the request as unauthenticated");
                Err(sign_in())
            }
        }
    }
}

/// A live session belonging to an RDU member, or the request never reaches the handler.
///
/// No session redirects to sign in; a session that is not RDU's is a 403 page, because signing in
/// again will not fix it.
#[derive(Debug, Clone)]
pub(crate) struct Rdu(pub(crate) Account);

impl FromRequestParts<EditorState> for Rdu {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &EditorState) -> Result<Self, Self::Rejection> {
        let Authenticated(account) = Authenticated::from_request_parts(parts, state).await?;
        if !account.principal.is_rdu() {
            // No `http.route` field: the enclosing server span carries it (see identity's `guard`).
            tracing::info!("refused an RDU-only page to an account that is not RDU");
            return Err(crate::shell::forbidden(
                state,
                &account.principal.display_name,
                deposit_shell::RDU_ONLY,
            ));
        }
        Ok(Self(account))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::http::{Request, StatusCode};
    use editor_core::records::normalize_shortcode;
    use editor_ports::{FakeAuthenticator, Role};

    use super::*;
    use crate::fixtures::{body_string, location, test_state, TestState};

    fn parts(method: &str, uri: &str) -> Parts {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(())
            .expect("the request should build")
            .into_parts()
            .0
    }

    #[test]
    fn test_a_get_is_sent_back_to_where_it_was_going() {
        assert_eq!(destination(&parts("GET", "/projects/0801")), Some("/projects/0801"));
    }

    #[test]
    fn test_a_get_with_a_query_keeps_only_its_path() {
        assert_eq!(destination(&parts("GET", "/projects/0801?x=1")), Some("/projects/0801"));
    }

    #[test]
    fn test_a_write_is_not_sent_back_to_itself() {
        // A POST's destination is a side effect, not a page.
        assert_eq!(destination(&parts("POST", "/projects/0801/sections/overview")), None);
        assert_eq!(destination(&parts("DELETE", "/collection/x/discard")), None);
    }

    fn principal(account_id: &str, role: Role) -> Principal {
        Principal {
            account_id: account_id.to_string(),
            display_name: "A Depositor".to_string(),
            role,
            shortcodes: vec!["0801".to_string()],
            signed_out_at: SystemTime::now(),
        }
    }

    async fn state_signed_in_as(principal: Principal) -> TestState {
        let (mut state, _) = test_state("editor-auth").await;
        state.authenticator = Arc::new(FakeAuthenticator::new().with_session("token", principal));
        state
    }

    fn session_parts(cookie: Option<&str>) -> Parts {
        let mut request = Request::builder().method("GET").uri("/projects/0801");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        request.body(()).expect("the request should build").into_parts().0
    }

    #[tokio::test]
    async fn test_a_session_yields_the_account_with_its_id_parsed() {
        let id = Uuid::new_v4();
        let state = state_signed_in_as(principal(&id.to_string(), Role::Depositor)).await;

        let Authenticated(account) = Authenticated::from_request_parts(&mut session_parts(Some("s=token")), &state)
            .await
            .expect("a live session");
        assert_eq!(account.id, id);
        assert_eq!(account.principal.display_name, "A Depositor");
    }

    #[tokio::test]
    async fn test_no_session_is_sent_to_sign_in_and_back() {
        let state = state_signed_in_as(principal(&Uuid::new_v4().to_string(), Role::Depositor)).await;

        let refused = Authenticated::from_request_parts(&mut session_parts(None), &state)
            .await
            .expect_err("no session");
        assert_eq!(refused.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects/0801"));
    }

    #[tokio::test]
    async fn test_an_account_id_that_is_not_a_uuid_is_no_session() {
        let state = state_signed_in_as(principal("not-a-uuid", Role::Rdu)).await;

        let refused = Authenticated::from_request_parts(&mut session_parts(Some("s=token")), &state)
            .await
            .expect_err("an id the records cannot hold");
        assert_eq!(refused.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects/0801"));
    }

    #[tokio::test]
    async fn test_a_depositor_gets_the_rdu_only_page_rather_than_sign_in() {
        let state = state_signed_in_as(principal(&Uuid::new_v4().to_string(), Role::Depositor)).await;

        let refused = Rdu::from_request_parts(&mut session_parts(Some("s=token")), &state)
            .await
            .expect_err("not RDU");
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
        let body = body_string(refused).await;
        assert!(body.contains(deposit_shell::RDU_ONLY), "{body}");
        assert!(body.contains("A Depositor"), "{body}");
    }

    #[tokio::test]
    async fn test_an_rdu_session_passes() {
        let state = state_signed_in_as(principal(&Uuid::new_v4().to_string(), Role::Rdu)).await;

        assert!(Rdu::from_request_parts(&mut session_parts(Some("s=token")), &state)
            .await
            .is_ok());
    }

    fn depositor(shortcodes: &[&str]) -> Principal {
        Principal {
            account_id: uuid::Uuid::nil().to_string(),
            display_name: "A".to_string(),
            role: Role::Depositor,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            signed_out_at: std::time::SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn the_assignment_comparison_agrees_with_the_storage_key() {
        // Two expressions of one rule: `may_reach` compares, `normalize_shortcode`
        // keys. If they ever disagreed, a depositor could reach a project whose
        // draft they cannot load — or load someone else's. Checked over the
        // shapes the published set actually contains, mixed case included.
        for (assigned, requested) in [
            ("0801", "0801"),
            ("080C", "080c"),
            ("080c", "080C"),
            ("0801a", "0801A"),
            ("085F", "085f"),
        ] {
            assert!(
                depositor(&[assigned]).may_reach(requested),
                "{assigned} should reach {requested}"
            );
            assert_eq!(
                normalize_shortcode(assigned),
                normalize_shortcode(requested),
                "{assigned} and {requested} must key the same"
            );
        }
        for (assigned, requested) in [("0801", "0803"), ("080C", "080E")] {
            assert!(!depositor(&[assigned]).may_reach(requested));
            assert_ne!(normalize_shortcode(assigned), normalize_shortcode(requested));
        }
    }
}
