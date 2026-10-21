//! The editor's ports, answered from identity's store: [`LiveAuthenticator`] and
//! [`LiveUserDirectory`].
//!
//! Both hold identity's repository bundle rather than [`crate::Database`], so a test can put a
//! fault-injecting store in front of them.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use editor_ports::{Authenticator, PortFuture, Principal, Unavailable, UserDirectory};
use identity_core::login::AuthConfig;
use identity_core::records::Role;
use identity_core::repository::{IdentityRepositories, UserRepository};
use identity_core::session;
use uuid::Uuid;

/// Who a request's session belongs to, by identity-core's session rules.
pub struct LiveAuthenticator {
    store: Arc<dyn IdentityRepositories>,
    auth: AuthConfig,
}

impl LiveAuthenticator {
    pub fn new(store: Arc<dyn IdentityRepositories>, auth: AuthConfig) -> Self {
        Self { store, auth }
    }
}

impl Authenticator for LiveAuthenticator {
    fn authenticate<'a>(&'a self, cookie_headers: &'a [&'a str], now: SystemTime) -> PortFuture<'a, Option<Principal>> {
        Box::pin(async move {
            let viewer = session::current(&*self.store, &self.auth, cookie_headers, DateTime::<Utc>::from(now)).await?;
            let user = viewer.user;
            Some(Principal {
                account_id: user.id.to_string(),
                display_name: user.name,
                role: match user.role {
                    Role::Depositor => editor_ports::Role::Depositor,
                    Role::Rdu => editor_ports::Role::Rdu,
                },
                assigned_shortcodes: user.shortcodes,
                signed_out_at: viewer.signed_out_at.into(),
            })
        })
    }
}

/// The names behind account ids, from identity's account table.
pub struct LiveUserDirectory {
    store: Arc<dyn IdentityRepositories>,
}

impl LiveUserDirectory {
    pub fn new(store: Arc<dyn IdentityRepositories>) -> Self {
        Self { store }
    }
}

impl UserDirectory for LiveUserDirectory {
    fn display_names<'a>(
        &'a self,
        account_ids: &'a [String],
    ) -> PortFuture<'a, Result<Vec<(String, String)>, Unavailable>> {
        Box::pin(async move {
            // Compared as `Uuid`s, so the spelling of an id cannot make a known account read as
            // removed.
            let wanted: HashSet<Uuid> = account_ids.iter().filter_map(|id| Uuid::parse_str(id).ok()).collect();
            // One query rather than one per id: a review queue asks for every row's names at once.
            let users = UserRepository::list(&*self.store).await.map_err(Unavailable::new)?;
            Ok(users
                .into_iter()
                .filter(|user| wanted.contains(&user.id))
                .map(|user| (user.id.to_string(), user.name))
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeDelta;
    use identity_core::cookie::SESSION;
    use identity_core::repository::SessionRepository;

    use super::*;
    use crate::test_support::{open_for_test, seed_user};
    use crate::Database;

    fn auth() -> AuthConfig {
        AuthConfig {
            cooldown: Duration::ZERO,
            max_failed: 5,
            lockout: Duration::from_secs(900),
            daily_cap: 100,
            account_daily_cap: 10,
            session_absolute: Duration::from_secs(8 * 3600),
            session_idle: Duration::from_secs(3600),
            break_glass: false,
        }
    }

    async fn store(label: &str) -> Arc<Database> {
        Arc::new(open_for_test(label).await)
    }

    /// A depositor assigned to `0801`, signed in at `now`; returns the account id and the token.
    async fn signed_in(db: &Arc<Database>, now: DateTime<Utc>) -> (String, String) {
        let id = seed_user(db, "a.depositor@example.test", "A Depositor", "depositor", &["0801"]).await;
        let user_id = Uuid::parse_str(&id).unwrap();
        let token = session::begin(&**db, &auth(), user_id, now).await.unwrap().id;
        (id, token)
    }

    fn cookie(token: &str) -> String {
        format!("{SESSION}={token}")
    }

    async fn authenticate(db: &Arc<Database>, headers: &[&str], now: DateTime<Utc>) -> Option<Principal> {
        LiveAuthenticator::new(db.clone(), auth())
            .authenticate(headers, now.into())
            .await
    }

    #[tokio::test]
    async fn test_a_live_session_answers_with_its_account() {
        let db = store("live-session").await;
        let now = Utc::now();
        let (id, token) = signed_in(&db, now).await;

        let principal = authenticate(&db, &[&cookie(&token)], now).await.expect("a live session");
        assert_eq!(principal.account_id, id);
        assert_eq!(principal.display_name, "A Depositor");
        assert_eq!(principal.role, editor_ports::Role::Depositor);
        assert_eq!(principal.assigned_shortcodes, vec!["0801".to_string()]);
        // The idle deadline comes first on a fresh session: it is the one the page warns about.
        assert_eq!(
            DateTime::<Utc>::from(principal.signed_out_at),
            now + identity_core::login::delta(auth().session_idle)
        );
    }

    #[tokio::test]
    async fn test_the_session_cookie_is_found_on_a_split_cookie_header() {
        // HTTP/2 clients may send one `Cookie` header per cookie.
        let db = store("live-split").await;
        let now = Utc::now();
        let (_, token) = signed_in(&db, now).await;

        let session = cookie(&token);
        assert!(authenticate(&db, &["other=1", &session], now).await.is_some());
    }

    #[tokio::test]
    async fn test_no_cookie_and_an_unknown_token_are_both_no_session() {
        let db = store("live-none").await;
        let now = Utc::now();
        signed_in(&db, now).await;

        assert!(authenticate(&db, &[], now).await.is_none());
        assert!(authenticate(&db, &[&cookie("not-a-session")], now).await.is_none());
    }

    #[tokio::test]
    async fn test_a_session_past_its_absolute_expiry_is_refused_and_deleted() {
        let db = store("live-absolute").await;
        let now = Utc::now();
        let (_, token) = signed_in(&db, now).await;
        db.set_session_expiry(&token, now - TimeDelta::minutes(1)).await;

        assert!(authenticate(&db, &[&cookie(&token)], now).await.is_none());
        assert_eq!(
            SessionRepository::find(&*db, &token).await.unwrap(),
            None,
            "and must not linger"
        );
    }

    #[tokio::test]
    async fn test_an_idle_session_is_refused_even_before_its_absolute_expiry() {
        let db = store("live-idle").await;
        let now = Utc::now();
        let (_, token) = signed_in(&db, now).await;
        SessionRepository::touch(&*db, &token, now - TimeDelta::hours(2)).await.unwrap();

        assert!(authenticate(&db, &[&cookie(&token)], now).await.is_none());
        assert_eq!(SessionRepository::find(&*db, &token).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_a_session_in_use_advances_its_idle_deadline() {
        let db = store("live-touch").await;
        let start = Utc::now() - TimeDelta::minutes(30);
        let (_, token) = signed_in(&db, start).await;
        let now = Utc::now();

        authenticate(&db, &[&cookie(&token)], now).await.expect("still live");
        let session = SessionRepository::find(&*db, &token).await.unwrap().unwrap();
        assert_eq!(session.last_seen_at, now, "the request touched the session");

        // The deadline a request is told is measured from the last-seen time it found, so the
        // next request is the one that sees the advanced deadline.
        let later = now + TimeDelta::seconds(1);
        let principal = authenticate(&db, &[&cookie(&token)], later).await.expect("still live");
        assert_eq!(
            DateTime::<Utc>::from(principal.signed_out_at),
            now + identity_core::login::delta(auth().session_idle)
        );
    }

    #[tokio::test]
    async fn test_the_absolute_expiry_is_the_deadline_once_it_comes_first() {
        let db = store("live-absolute-first").await;
        let now = Utc::now();
        let (_, token) = signed_in(&db, now).await;
        let expires = now + TimeDelta::minutes(5);
        db.set_session_expiry(&token, expires).await;

        let principal = authenticate(&db, &[&cookie(&token)], now).await.expect("still live");
        assert_eq!(DateTime::<Utc>::from(principal.signed_out_at), expires);
    }

    #[tokio::test]
    async fn test_a_removed_account_has_no_session() {
        let db = store("live-removed").await;
        let now = Utc::now();
        let (id, token) = signed_in(&db, now).await;
        UserRepository::delete(&*db, Uuid::parse_str(&id).unwrap()).await.unwrap();

        assert!(authenticate(&db, &[&cookie(&token)], now).await.is_none());
    }

    #[tokio::test]
    async fn test_an_rdu_session_answers_with_the_rdu_role() {
        let db = store("live-rdu").await;
        let now = Utc::now();
        let id = seed_user(&db, "rdu@dasch.swiss", "An Admin", "rdu", &[]).await;
        let token = session::begin(&*db, &auth(), Uuid::parse_str(&id).unwrap(), now)
            .await
            .unwrap()
            .id;

        let principal = authenticate(&db, &[&cookie(&token)], now).await.expect("a live session");
        assert!(principal.is_rdu());
        assert!(principal.may_reach("0801"));
    }

    #[tokio::test]
    async fn test_the_directory_names_the_accounts_it_still_knows() {
        let db = store("live-directory").await;
        let first = seed_user(&db, "a@example.test", "A Depositor", "depositor", &[]).await;
        seed_user(&db, "b@example.test", "B Depositor", "depositor", &[]).await;
        let removed = Uuid::new_v4().to_string();

        // Asked in upper case: an id is compared as a `Uuid`, not as text.
        let names = LiveUserDirectory::new(db.clone())
            .display_names(&[first.to_uppercase(), removed])
            .await
            .unwrap();
        // A removed account is absent rather than an error, and an id nobody asked for is left out.
        assert_eq!(names, vec![(first, "A Depositor".to_string())]);
    }
}
