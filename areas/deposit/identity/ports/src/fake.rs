use std::collections::HashMap;
use std::fmt;

use crate::{AuthoredWork, PortFuture, Unavailable};

/// Answers with the shortcodes it was given per account, or fails the reads it was told to.
#[derive(Clone, Debug, Default)]
pub struct FakeAuthoredWork {
    drafts: HashMap<String, Vec<String>>,
    submissions: HashMap<String, Vec<String>>,
    failing_drafts: bool,
    failing_submissions: bool,
}

impl FakeAuthoredWork {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_drafts(mut self, account_id: &str, shortcodes: &[&str]) -> Self {
        self.drafts.insert(account_id.to_string(), owned(shortcodes));
        self
    }

    #[must_use]
    pub fn with_submissions(mut self, account_id: &str, shortcodes: &[&str]) -> Self {
        self.submissions.insert(account_id.to_string(), owned(shortcodes));
        self
    }

    #[must_use]
    pub fn failing_drafts(mut self) -> Self {
        self.failing_drafts = true;
        self
    }

    #[must_use]
    pub fn failing_submissions(mut self) -> Self {
        self.failing_submissions = true;
        self
    }
}

fn owned(shortcodes: &[&str]) -> Vec<String> {
    shortcodes.iter().map(|s| (*s).to_string()).collect()
}

fn answer(failing: bool, held: &HashMap<String, Vec<String>>, account_id: &str) -> Result<Vec<String>, Unavailable> {
    if failing {
        return Err(Unavailable::new(FakeFailure));
    }
    Ok(held.get(account_id).cloned().unwrap_or_default())
}

#[derive(Debug)]
struct FakeFailure;

impl fmt::Display for FakeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the fake was told to fail")
    }
}

impl std::error::Error for FakeFailure {}

impl AuthoredWork for FakeAuthoredWork {
    fn open_drafts<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
        let result = answer(self.failing_drafts, &self.drafts, account_id);
        Box::pin(async move { result })
    }

    fn open_submissions<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
        let result = answer(self.failing_submissions, &self.submissions, account_id);
        Box::pin(async move { result })
    }
}
