//! [`LiveOpenWork`]: identity's [`OpenWork`] port, answered from the editor's tables and the
//! published set.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use editor_core::published::PublishedProjects;
use editor_core::records::normalize_shortcode;
use editor_core::repository::{CreatedProjectRepository, DraftRepository, EditorRepositories, SubmissionRepository};
use identity_ports::{OpenWork, PortFuture, Unavailable};
use uuid::Uuid;

/// The editor work still open under an account, for the depositor-removal
/// confirmation, and the projects the editor knows, for the account screens.
///
/// Over the editor's repository bundle rather than [`crate::Database`], so a test can put a
/// fault-injecting store in front of it.
pub struct LiveOpenWork {
    store: Arc<dyn EditorRepositories>,
    published: PublishedProjects,
}

impl LiveOpenWork {
    /// `data_dir` is the editor's data directory. Its published set is read once, here, as
    /// `Editor::start` reads it, so both capabilities know the same projects until the next
    /// restart. A file that fails to load is skipped; the editor's startup logs each one.
    pub fn new(store: Arc<dyn EditorRepositories>, data_dir: Option<&Path>) -> Self {
        let published = data_dir
            .map(|dir| PublishedProjects::load_from(&dir.join("projects")).0)
            .unwrap_or_default();
        Self { store, published }
    }
}

/// `None` for an id that is not a `Uuid`: no editor row can hold it, so nothing is open under it.
fn account(account_id: &str) -> Option<Uuid> {
    Uuid::parse_str(account_id).ok()
}

impl OpenWork for LiveOpenWork {
    fn drafts<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
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

    fn submissions<'a>(&'a self, account_id: &'a str) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
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
                .filter(|submission| submission.submitted_by == Some(id) && submission.state.is_pending())
                .map(|submission| submission.shortcode)
                .collect())
        })
    }

    fn unknown_projects<'a>(&'a self, shortcodes: &'a [String]) -> PortFuture<'a, Result<Vec<String>, Unavailable>> {
        Box::pin(async move {
            let created: BTreeSet<String> = CreatedProjectRepository::list(&*self.store)
                .await
                .map_err(Unavailable::new)?
                .into_iter()
                .map(|project| normalize_shortcode(&project.shortcode))
                .collect();
            Ok(shortcodes
                .iter()
                .filter(|shortcode| {
                    self.published.get(shortcode).is_none() && !created.contains(&normalize_shortcode(shortcode))
                })
                .cloned()
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use editor_core::records::{CreatedProject, DraftRecord};

    use super::*;
    use crate::test_support::{open_for_test, seed_draft, seed_submission};

    const AUTHOR: &str = "6f1c1d1e-0000-4000-8000-000000000001";
    const SOMEONE_ELSE: &str = "6f1c1d1e-0000-4000-8000-000000000002";

    async fn work(label: &str) -> (Arc<crate::Database>, LiveOpenWork) {
        let db = Arc::new(open_for_test(label).await);
        (db.clone(), LiveOpenWork::new(db, None))
    }

    fn codes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    async fn create_project(db: &crate::Database, shortcode: &str) {
        let now = Utc::now();
        let project = CreatedProject {
            shortcode: shortcode.to_string(),
            name: "A created project".to_string(),
            created_by: None,
            created_at: now,
        };
        let draft = DraftRecord {
            shortcode: shortcode.to_string(),
            payload: "{}".to_string(),
            updated_by: None,
            created_at: now,
            updated_at: now,
        };
        CreatedProjectRepository::create(db, &project, &draft).await.unwrap();
    }

    #[tokio::test]
    async fn test_a_published_or_created_shortcode_is_known_whatever_its_case() {
        let db = Arc::new(open_for_test("known-projects").await);
        let work = LiveOpenWork::new(db.clone(), Some(&editor_core::checkout_dpe_data_dir()));
        create_project(&db, "9a01").await;

        // `080C` is in the committed corpus.
        let asked = codes(&["080c", "9A01", "9999", "080C", "0zz0"]);
        assert_eq!(work.unknown_projects(&asked).await.unwrap(), codes(&["9999", "0zz0"]));
    }

    #[tokio::test]
    async fn test_without_a_data_directory_only_created_projects_are_known() {
        let (db, work) = work("known-without-data").await;
        create_project(&db, "9a01").await;

        let asked = codes(&["080C", "9a01"]);
        assert_eq!(work.unknown_projects(&asked).await.unwrap(), codes(&["080C"]));
    }

    #[tokio::test]
    async fn test_the_drafts_are_the_ones_the_account_saved_last() {
        let (db, work) = work("open-drafts").await;
        let now = Utc::now();
        seed_draft(&db, "0801", "{}", Some(AUTHOR), now, now).await;
        seed_draft(&db, "0803", "{}", Some(SOMEONE_ELSE), now, now).await;
        seed_draft(&db, "0804", "{}", None, now, now).await;

        assert_eq!(work.drafts(AUTHOR).await.unwrap(), vec!["0801".to_string()]);
    }

    #[tokio::test]
    async fn test_every_submission_short_of_approved_is_open() {
        let (db, work) = work("open-submissions").await;
        let now = Utc::now();
        seed_submission(&db, "0801", "{}", "submitted", Some(AUTHOR), now, None, None).await;
        seed_submission(&db, "0803", "{}", "in_review", Some(AUTHOR), now, Some(SOMEONE_ELSE), Some(now)).await;
        seed_submission(&db, "0804", "{}", "approved", Some(AUTHOR), now, Some(SOMEONE_ELSE), Some(now)).await;
        seed_submission(&db, "0805", "{}", "submitted", Some(SOMEONE_ELSE), now, None, None).await;

        let mut open = work.submissions(AUTHOR).await.unwrap();
        open.sort();
        assert_eq!(open, vec!["0801".to_string(), "0803".to_string()]);
    }

    #[tokio::test]
    async fn test_an_id_that_is_not_a_uuid_has_no_open_work() {
        let (db, work) = work("open-not-a-uuid").await;
        let now = Utc::now();
        seed_draft(&db, "0801", "{}", Some(AUTHOR), now, now).await;

        assert!(work.drafts("not-a-uuid").await.unwrap().is_empty());
        assert!(work.submissions("not-a-uuid").await.unwrap().is_empty());
    }
}
