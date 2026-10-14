use std::collections::HashMap;
use std::fmt;
use std::time::SystemTime;

use crate::{Authenticator, PortFuture, Principal, Unavailable, UserDirectory};

/// Answers with the [`Principal`] registered for a cookie's value, whatever the cookie is called.
///
/// Ignores `now` and expires nothing: session lifetimes are identity's rules, tested against
/// `identity-store`'s `LiveAuthenticator`.
#[derive(Clone, Debug, Default)]
pub struct FakeAuthenticator {
    sessions: HashMap<String, Principal>,
}

impl FakeAuthenticator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A request carrying a cookie whose value is `token` authenticates as `principal`.
    #[must_use]
    pub fn with_session(mut self, token: &str, principal: Principal) -> Self {
        self.sessions.insert(token.to_string(), principal);
        self
    }
}

impl Authenticator for FakeAuthenticator {
    fn authenticate<'a>(
        &'a self,
        cookie_headers: &'a [&'a str],
        _now: SystemTime,
    ) -> PortFuture<'a, Option<Principal>> {
        let found = cookie_headers
            .iter()
            .flat_map(|header| header.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find_map(|(_, value)| self.sessions.get(value))
            .cloned();
        Box::pin(async move { found })
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeUserDirectory {
    names: HashMap<String, String>,
    failing: bool,
}

impl FakeUserDirectory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_account(mut self, account_id: &str, display_name: &str) -> Self {
        self.names.insert(account_id.to_string(), display_name.to_string());
        self
    }

    #[must_use]
    pub fn failing() -> Self {
        Self { names: HashMap::new(), failing: true }
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
            Ok(account_ids
                .iter()
                .filter_map(|id| self.names.get(id).map(|name| (id.clone(), name.clone())))
                .collect())
        };
        Box::pin(async move { answer })
    }
}
