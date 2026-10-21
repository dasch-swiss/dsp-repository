use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::{Actor, Authenticator, PortFuture, Unavailable, UserDirectory};

/// Answers with the [`Actor`] registered for a cookie's value, whatever the cookie is called.
///
/// Ignores `now` and expires nothing: session lifetimes are identity's rules, tested against
/// `identity-store`'s `LiveAuthenticator`.
#[derive(Debug, Default)]
pub struct FakeAuthenticator {
    sessions: Mutex<HashMap<String, Actor>>,
}

impl FakeAuthenticator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A request carrying a cookie whose value is `token` authenticates as `actor`.
    #[must_use]
    pub fn with_session(self, token: &str, actor: Actor) -> Self {
        self.insert_session(token, actor);
        self
    }

    /// [`Self::with_session`] for a fake already shared behind an `Arc`.
    pub fn insert_session(&self, token: &str, actor: Actor) {
        self.sessions
            .lock()
            .expect("the session map lock should not be poisoned")
            .insert(token.to_string(), actor);
    }
}

impl Authenticator for FakeAuthenticator {
    fn authenticate<'a>(&'a self, cookie_headers: &'a [&'a str], _now: SystemTime) -> PortFuture<'a, Option<Actor>> {
        let sessions = self.sessions.lock().expect("the session map lock should not be poisoned");
        let found = cookie_headers
            .iter()
            .flat_map(|header| header.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find_map(|(_, value)| sessions.get(value))
            .cloned();
        Box::pin(async move { found })
    }
}

#[derive(Debug, Default)]
pub struct FakeUserDirectory {
    names: Mutex<HashMap<String, String>>,
    failing: bool,
}

impl FakeUserDirectory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_account(self, account_id: &str, display_name: &str) -> Self {
        self.insert_account(account_id, display_name);
        self
    }

    /// [`Self::with_account`] for a fake already shared behind an `Arc`.
    pub fn insert_account(&self, account_id: &str, display_name: &str) {
        self.names
            .lock()
            .expect("the name map lock should not be poisoned")
            .insert(account_id.to_string(), display_name.to_string());
    }

    #[must_use]
    pub fn failing() -> Self {
        Self { names: Mutex::new(HashMap::new()), failing: true }
    }
}

#[derive(Debug)]
struct FakeFailure;

impl fmt::Display for FakeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the fake was told to fail")
    }
}

impl std::error::Error for FakeFailure {}

impl UserDirectory for FakeUserDirectory {
    fn display_names<'a>(
        &'a self,
        account_ids: &'a [String],
    ) -> PortFuture<'a, Result<Vec<(String, String)>, Unavailable>> {
        let answer = if self.failing {
            Err(Unavailable::new(FakeFailure))
        } else {
            let names = self.names.lock().expect("the name map lock should not be poisoned");
            Ok(account_ids
                .iter()
                .filter_map(|id| names.get(id).map(|name| (id.clone(), name.clone())))
                .collect())
        };
        Box::pin(async move { answer })
    }
}
