//! [`LiveAuthoredWork`]: identity's [`AuthoredWork`] port, answered from the editor's tables.

use std::sync::Arc;

use editor_core::records::SubmissionState;
use editor_core::repository::{DraftRepository, EditorRepositories, SubmissionRepository};
use identity_ports::{AuthoredWork, PortFuture, Unavailable};
use uuid::Uuid;

/// The editor work an account authored and that is still open, for the depositor-removal
/// confirmation.
///
/// Over the editor's repository bundle rather than [`crate::Database`], so a test can put a
/// fault-injecting store in front of it.
pub struct LiveAuthoredWork {
    store: Arc<dyn EditorRepositories>,
}

impl LiveAuthoredWork {
    pub fn new(store: Arc<dyn EditorRepositories>) -> Self {
        Self { store }
    }
}

/// `None` for an id that is not a `Uuid`: no editor row can hold it, so it authored nothing.
fn account(account_id: &str) -> Option<Uuid> {
    Uuid::parse_str(account_id).ok()
}

impl AuthoredWork for LiveAuthoredWork {
    fn open_drafts<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
        Box::pin(async move {
            let Some(id) = account(account_id) else {
                return Ok(Vec::new());
            };
            let drafts = DraftRepository::list(&*self.store).await.map_err(Unavailable::new)?;
            Ok(drafts
                .into_iter()
                .filter(|draft| draft.updated_by == Some(id))
                .map(|draft| draft.shortcode)
                .collect())
        })
    }

    fn open_submissions<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
        Box::pin(async move {
            let Some(id) = account(account_id) else {
                return Ok(Vec::new());
            };
            let submissions = SubmissionRepository::list(&*self.store).await.map_err(Unavailable::new)?;
            Ok(submissions
                .into_iter()
                // `list` returns every row regardless of state, so the state filter is this
                // adapter's to apply. Everything short of `Approved` qualifies: the warning is
                // about work that can no longer be *returned* to its depositor, and an `InReview`
                // submission is as unreturnable as a `Submitted` one. Filtering to `Submitted`
                // alone would hide the case where the loss matters most.
                .filter(|submission| {
                    submission.submitted_by == Some(id) && submission.state != SubmissionState::Approved
                })
                .map(|submission| submission.shortcode)
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::test_support::{open_for_test, seed_draft, seed_submission};

    const AUTHOR: &str = "6f1c1d1e-0000-4000-8000-000000000001";
    const SOMEONE_ELSE: &str = "6f1c1d1e-0000-4000-8000-000000000002";

    async fn work(label: &str) -> (Arc<crate::Database>, LiveAuthoredWork) {
        let db = Arc::new(open_for_test(label).await);
        (db.clone(), LiveAuthoredWork::new(db))
    }

    #[tokio::test]
    async fn test_the_drafts_are_the_ones_the_account_saved_last() {
        let (db, work) = work("authored-drafts").await;
        let now = Utc::now();
        seed_draft(&db, "0801", "{}", Some(AUTHOR), now, now).await;
        seed_draft(&db, "0803", "{}", Some(SOMEONE_ELSE), now, now).await;
        seed_draft(&db, "0804", "{}", None, now, now).await;

        assert_eq!(work.open_drafts(AUTHOR).await.unwrap(), vec!["0801".to_string()]);
    }

    #[tokio::test]
    async fn test_every_submission_short_of_approved_is_open() {
        let (db, work) = work("authored-submissions").await;
        let now = Utc::now();
        seed_submission(&db, "0801", "{}", "submitted", Some(AUTHOR), now, None, None).await;
        seed_submission(&db, "0803", "{}", "in_review", Some(AUTHOR), now, Some(SOMEONE_ELSE), Some(now)).await;
        seed_submission(&db, "0804", "{}", "approved", Some(AUTHOR), now, Some(SOMEONE_ELSE), Some(now)).await;
        seed_submission(&db, "0805", "{}", "submitted", Some(SOMEONE_ELSE), now, None, None).await;

        let mut open = work.open_submissions(AUTHOR).await.unwrap();
        open.sort();
        assert_eq!(open, vec!["0801".to_string(), "0803".to_string()]);
    }

    #[tokio::test]
    async fn test_an_id_that_is_not_a_uuid_authored_nothing() {
        let (db, work) = work("authored-not-a-uuid").await;
        let now = Utc::now();
        seed_draft(&db, "0801", "{}", Some(AUTHOR), now, now).await;

        assert!(work.open_drafts("not-a-uuid").await.unwrap().is_empty());
        assert!(work.open_submissions("not-a-uuid").await.unwrap().is_empty());
    }
}
