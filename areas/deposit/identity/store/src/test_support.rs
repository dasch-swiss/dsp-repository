//! Test support for the crates that hold identity's store: plain-value seed helpers, a row count,
//! and [`FaultyStore`], which fails chosen calls of a real [`Database`].
//!
//! The helpers take strings and instants rather than `identity-core` records, so a crate that may
//! not name `identity-core` (the area's composition root, `areas/deposit/ADR-0004`) can seed with
//! them.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_core::records::{LoginCode, Role, Session, User};
use identity_core::repository::{
    Attempt, Issued, LoginCodeRepository, MailSendRepository, RepositoryError, Result, SessionRepository,
    UserRepository,
};
use uuid::Uuid;

use crate::{Database, Source};

/// A fresh in-memory store for one test.
pub async fn open_for_test(label: &str) -> Database {
    Database::open(Source::memory_for_test(label), 4, std::time::Duration::from_secs(5))
        .await
        .expect("the test identity store should open")
}

impl Database {
    /// Rows in one table, so a test can assert on state no repository method returns: what a
    /// rollback left behind, and what a cascade took.
    pub async fn count_rows(&self, table: &'static str) -> i64 {
        self.read(move |conn| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)))
            .await
            .expect("counting rows should succeed")
    }

    /// Move a session's absolute expiry, which no repository method can: it is set at creation
    /// and never extended.
    pub async fn set_session_expiry(&self, id: &str, expires_at: DateTime<Utc>) {
        let id = id.to_string();
        self.write(move |tx| {
            tx.execute(
                "UPDATE sessions SET expires_at = ?2 WHERE id = ?1",
                rusqlite::params![id, expires_at],
            )
        })
        .await
        .expect("moving the session's expiry should succeed");
    }
}

/// Insert an account and return its id. `role` is `"depositor"` or `"rdu"`.
pub async fn seed_user(db: &Database, email: &str, name: &str, role: &str, shortcodes: &[&str]) -> String {
    let user = User {
        id: Uuid::new_v4(),
        email: email.to_string(),
        name: name.to_string(),
        role: role.parse::<Role>().expect("the role should be depositor or rdu"),
        shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
        failed_logins: 0,
        failed_login_at: None,
        last_code_at: None,
        created_at: Utc::now(),
    };
    UserRepository::create(db, &user).await.expect("the seeded user should insert");
    user.id.to_string()
}

/// Insert a session for `user_id` that began at `now` and expires at `expires_at`, and return its
/// token, the session cookie's value.
pub async fn seed_session(db: &Database, user_id: &str, now: DateTime<Utc>, expires_at: DateTime<Utc>) -> String {
    let session = Session {
        id: Uuid::new_v4().simple().to_string(),
        user_id: Uuid::parse_str(user_id).expect("the user id should be a uuid"),
        created_at: now,
        last_seen_at: now,
        expires_at,
    };
    SessionRepository::create(db, &session)
        .await
        .expect("the seeded session should insert");
    session.id
}

/// An account as a test reads it back: the stored fields a handler can change or must leave alone.
/// `role` is `"depositor"` or `"rdu"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: &'static str,
    pub shortcodes: Vec<String>,
    pub failed_logins: u32,
    pub last_code_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserRow {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            role: user.role.as_str(),
            shortcodes: user.shortcodes,
            failed_logins: user.failed_logins,
            last_code_at: user.last_code_at,
            created_at: user.created_at,
        }
    }
}

pub async fn find_user(db: &Database, user_id: &str) -> Option<UserRow> {
    UserRepository::find_by_id(db, Uuid::parse_str(user_id).expect("the user id should be a uuid"))
        .await
        .expect("the user lookup should succeed")
        .map(UserRow::from)
}

pub async fn find_user_by_email(db: &Database, email: &str) -> Option<UserRow> {
    UserRepository::find_by_email(db, email)
        .await
        .expect("the user lookup should succeed")
        .map(UserRow::from)
}

/// Whether a session with this token is stored. Reading it does not extend it.
pub async fn session_exists(db: &Database, token: &str) -> bool {
    SessionRepository::find(db, token)
        .await
        .expect("the session lookup should succeed")
        .is_some()
}

pub async fn touch_session(db: &Database, token: &str, at: DateTime<Utc>) {
    SessionRepository::touch(db, token, at)
        .await
        .expect("touching the session should succeed");
}

pub async fn record_code_issued(db: &Database, user_id: &str, at: DateTime<Utc>) {
    UserRepository::record_code_issued(db, Uuid::parse_str(user_id).expect("the user id should be a uuid"), at)
        .await
        .expect("recording the code should succeed");
}

/// Count one failed login at `at` against the account. Failures before `decay_before` no longer
/// count.
pub async fn record_failed_login(db: &Database, user_id: &str, at: DateTime<Utc>, decay_before: DateTime<Utc>) {
    UserRepository::record_failed_login(
        db,
        Uuid::parse_str(user_id).expect("the user id should be a uuid"),
        at,
        decay_before,
    )
    .await
    .expect("recording the failed login should succeed");
}

/// Delete an account, as removing a depositor does.
pub async fn delete_user(db: &Database, user_id: &str) {
    UserRepository::delete(db, Uuid::parse_str(user_id).expect("the user id should be a uuid"))
        .await
        .expect("the user should delete");
}

/// Which calls a [`FaultyStore`] must fail.
///
/// One field per injectable call, named after the port method, so the compiler checks the name and
/// a new branch adds a field rather than a string. All false by default: a test sets exactly the
/// one branch it is driving.
#[derive(Default)]
pub struct Faults {
    pub claim_attempt: bool,
    pub rebind_browser_token: bool,
    pub record_failed_login: bool,
    /// `MailSendRepository::record`, named for the port, since `record` alone would not say which
    /// one.
    pub record_mail_send: bool,
}

/// A store that delegates every call to a real [`Database`] and fails the ones [`Faults`] names.
///
/// Delegation rather than a store built from scratch, because the branches this exists for are
/// deep in a flow: reaching the failing `claim_attempt` means an account, a live code and a
/// browser binding all had to be found first. A fake answering everything from nothing would have
/// to reimplement the store to get there, and would then be proving things about the
/// reimplementation.
pub struct FaultyStore {
    inner: Arc<Database>,
    faults: Faults,
}

impl FaultyStore {
    pub fn new(inner: Arc<Database>, faults: Faults) -> Self {
        Self { inner, faults }
    }
}

/// The error an injected failure reports.
///
/// [`RepositoryError::Backend`] specifically: it is the variant a driver failure arrives as, so the
/// handler takes the same branch it would in production. The message says it was injected, so a
/// capture of a test that fails for another reason does not read like a real storage fault.
fn injected(call: &'static str) -> RepositoryError {
    RepositoryError::Backend(format!("injected failure in {call} (test)").into())
}

#[async_trait]
impl UserRepository for FaultyStore {
    async fn create(&self, user: &User) -> Result<()> {
        UserRepository::create(&*self.inner, user).await
    }

    async fn update(&self, user: &User) -> Result<()> {
        UserRepository::update(&*self.inner, user).await
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        UserRepository::delete(&*self.inner, id).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        UserRepository::find_by_id(&*self.inner, id).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        UserRepository::find_by_email(&*self.inner, email).await
    }

    async fn list(&self) -> Result<Vec<User>> {
        UserRepository::list(&*self.inner).await
    }

    async fn record_failed_login(&self, id: Uuid, at: DateTime<Utc>, decay_before: DateTime<Utc>) -> Result<u32> {
        if self.faults.record_failed_login {
            return Err(injected("UserRepository::record_failed_login"));
        }
        UserRepository::record_failed_login(&*self.inner, id, at, decay_before).await
    }

    async fn clear_failed_logins(&self, id: Uuid) -> Result<()> {
        UserRepository::clear_failed_logins(&*self.inner, id).await
    }

    async fn record_code_issued(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        UserRepository::record_code_issued(&*self.inner, id, at).await
    }
}

#[async_trait]
impl SessionRepository for FaultyStore {
    async fn create(&self, session: &Session) -> Result<()> {
        SessionRepository::create(&*self.inner, session).await
    }

    async fn find(&self, id: &str) -> Result<Option<Session>> {
        SessionRepository::find(&*self.inner, id).await
    }

    async fn touch(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
        SessionRepository::touch(&*self.inner, id, at).await
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        SessionRepository::delete(&*self.inner, id).await
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64> {
        SessionRepository::delete_expired(&*self.inner, now).await
    }
}

#[async_trait]
impl LoginCodeRepository for FaultyStore {
    async fn create(&self, code: &LoginCode) -> Result<()> {
        LoginCodeRepository::create(&*self.inner, code).await
    }

    async fn create_unless_issued_since(&self, code: &LoginCode, not_before: DateTime<Utc>) -> Result<Issued> {
        LoginCodeRepository::create_unless_issued_since(&*self.inner, code, not_before).await
    }

    async fn find_by_browser_token(&self, token: &str) -> Result<Option<LoginCode>> {
        LoginCodeRepository::find_by_browser_token(&*self.inner, token).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        LoginCodeRepository::delete(&*self.inner, id).await
    }

    async fn find_active_for_user(&self, user_id: Uuid, now: DateTime<Utc>) -> Result<Option<LoginCode>> {
        LoginCodeRepository::find_active_for_user(&*self.inner, user_id, now).await
    }

    async fn claim_attempt(&self, id: Uuid, max_attempts: u32) -> Result<Attempt> {
        if self.faults.claim_attempt {
            return Err(injected("LoginCodeRepository::claim_attempt"));
        }
        LoginCodeRepository::claim_attempt(&*self.inner, id, max_attempts).await
    }

    async fn rebind_browser_token(&self, presented: &str, replacement: &str) -> Result<bool> {
        if self.faults.rebind_browser_token {
            return Err(injected("LoginCodeRepository::rebind_browser_token"));
        }
        LoginCodeRepository::rebind_browser_token(&*self.inner, presented, replacement).await
    }

    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool> {
        LoginCodeRepository::consume(&*self.inner, id, at).await
    }

    async fn delete_unconsumed_for_user(&self, user_id: Uuid) -> Result<u64> {
        LoginCodeRepository::delete_unconsumed_for_user(&*self.inner, user_id).await
    }

    async fn unconsume(&self, id: Uuid) -> Result<bool> {
        LoginCodeRepository::unconsume(&*self.inner, id).await
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64> {
        LoginCodeRepository::delete_expired(&*self.inner, now).await
    }
}

#[async_trait]
impl MailSendRepository for FaultyStore {
    async fn record(&self, user_id: Uuid, sent_at: DateTime<Utc>) -> Result<()> {
        if self.faults.record_mail_send {
            return Err(injected("MailSendRepository::record"));
        }
        MailSendRepository::record(&*self.inner, user_id, sent_at).await
    }

    async fn count_since(&self, since: DateTime<Utc>) -> Result<u64> {
        MailSendRepository::count_since(&*self.inner, since).await
    }

    async fn count_for_user_since(&self, user_id: Uuid, since: DateTime<Utc>) -> Result<u64> {
        MailSendRepository::count_for_user_since(&*self.inner, user_id, since).await
    }

    async fn delete_before(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        MailSendRepository::delete_before(&*self.inner, cutoff).await
    }
}
