//! Sample records for a throwaway deployment, so its surfaces can be exercised.
//!
//! The editor's review surfaces are only reachable when something is waiting to
//! be reviewed, and nothing in the service creates a submission yet — submit is
//! still to come. Until it lands, a PR preview renders an empty queue and a
//! "nothing to review" page for every project, which makes the surface
//! unreviewable by anyone who is not reading the tests.
//!
//! This inserts one pending submission and one draft at startup so the queue,
//! the diff, the per-field controls and the take-over banner are all reachable.
//! It is sample data and says so: the accounts it mints are at `.invalid`,
//! which [RFC 2606] guarantees can never resolve, so no address here can
//! receive mail even if a relay were configured.
//!
//! ## When it runs, and why that is safe
//!
//! Only under [`Config::is_throwaway`](crate::config::Config::is_throwaway) —
//! not `PROD`, no SMTP relay, no `EDITOR_DB_DIR`. All three, not any one: the
//! same predicate already decides whether a login code may be shown in the
//! interface, and it describes exactly one thing, a deployment whose entire
//! database dies with the process. A deployment with durable state never
//! reaches this module, so there is no environment in which sample data can
//! outlive a restart or sit beside a real record.
//!
//! It is also skipped whenever a submission already exists, so it seeds a fresh
//! database and never writes over what somebody is looking at.
//!
//! **Delete this module when submit lands.** At that point the queue fills the
//! honest way and sample data is one more thing to explain.
//!
//! [RFC 2606]: https://www.rfc-editor.org/rfc/rfc2606

use chrono::{DateTime, Duration, Utc};
use editor_core::draft::ProjectDraft;
use editor_core::published::PublishedProjects;
use editor_core::records::{normalize_shortcode, DraftRecord, Role, Submission, SubmissionState, User};
use editor_core::repository::{DraftRepository, Repositories, RepositoryError, SubmissionRepository, UserRepository};
use serde_json::json;
use uuid::Uuid;

/// The sample depositor. `.invalid` is reserved by RFC 2606 and resolves
/// nowhere, so this address cannot receive mail under any configuration.
const DEPOSITOR: &str = "sample-depositor@example.invalid";
const DEPOSITOR_NAME: &str = "Sample Depositor";

/// What the sample submission changes, so the diff has something to show.
///
/// One scalar and one language map: between them they exercise both in-place
/// editors, and the language map is the row whose rendering is easiest to get
/// wrong.
fn changes() -> serde_json::Value {
    json!({
        "name": "A sample edit to this project's name",
        "abstract": {
            "en": "A sample edit to this project's abstract, so the review diff has a language map to show.",
            "de": "Eine Beispieländerung, damit der Vergleich eine Sprachkarte zeigt."
        }
    })
}

/// Insert the sample records, unless something is already there.
///
/// Returns the shortcodes it seeded, or `None` when it did nothing. Every
/// failure is the caller's to log and ignore: sample data that will not insert
/// is a worse preview, not a reason to refuse to start.
pub(crate) async fn seed(
    db: &dyn Repositories,
    published: &PublishedProjects,
    now: DateTime<Utc>,
) -> Result<Option<(String, String)>, RepositoryError> {
    if !SubmissionRepository::list(db).await?.is_empty() {
        return Ok(None);
    }
    // Two published projects, in the set's own (shortcode) order, so the same
    // image always seeds the same pair and a screenshot keeps meaning something.
    // Without a published set there is nothing to diff against, and a sample
    // submission whose every field reads "new" would demonstrate the degenerate
    // case rather than the ordinary one.
    let mut shortcodes = published.summaries().map(|project| project.shortcode.to_string());
    let (Some(under_review), Some(in_progress)) = (shortcodes.next(), shortcodes.next()) else {
        return Ok(None);
    };

    let depositor = ensure_depositor(db, &[&under_review, &in_progress], now).await?;

    let mut draft = published.get(&under_review).map(ProjectDraft::from_raw).unwrap_or_default();
    for (field, value) in changes().as_object().expect("the sample changes are an object") {
        draft.set(field, value.clone());
    }
    SubmissionRepository::create(
        db,
        &Submission {
            id: Uuid::new_v4(),
            shortcode: normalize_shortcode(&under_review),
            payload: serde_json::to_string(&draft).map_err(|error| RepositoryError::Corrupt(error.to_string()))?,
            state: SubmissionState::Submitted,
            submitted_by: Some(depositor),
            // Backdated, so the queue's "oldest first" order is visible with one
            // submission and its "submitted" column is not the deployment time.
            submitted_at: now - Duration::hours(26),
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        },
    )
    .await?;

    // A second project with an unsubmitted draft, so the queue's other table —
    // the one that exists because RDU can help somebody who is stuck — is not
    // empty either.
    let mut started = published.get(&in_progress).map(ProjectDraft::from_raw).unwrap_or_default();
    started.set("name", json!("A sample draft nobody has submitted yet"));
    DraftRepository::upsert(
        db,
        &DraftRecord {
            shortcode: normalize_shortcode(&in_progress),
            payload: serde_json::to_string(&started).map_err(|error| RepositoryError::Corrupt(error.to_string()))?,
            updated_by: Some(depositor),
            reviewer_note: None,
            created_at: now - Duration::hours(3),
            updated_at: now - Duration::hours(2),
        },
    )
    .await?;

    Ok(Some((under_review, in_progress)))
}

/// The sample depositor, created if it is not already there.
///
/// It exists so the queue's "last editor" column names somebody rather than
/// reading "Account removed", and so the depositor's own side of the form can
/// be signed in to.
async fn ensure_depositor(
    db: &dyn UserRepository,
    shortcodes: &[&str],
    now: DateTime<Utc>,
) -> Result<Uuid, RepositoryError> {
    if let Some(existing) = UserRepository::find_by_email(db, DEPOSITOR).await? {
        return Ok(existing.id);
    }
    let user = User {
        id: Uuid::new_v4(),
        email: DEPOSITOR.to_string(),
        name: DEPOSITOR_NAME.to_string(),
        role: Role::Depositor,
        shortcodes: shortcodes.iter().map(|code| (*code).to_string()).collect(),
        failed_logins: 0,
        failed_login_at: None,
        last_code_at: None,
        created_at: now,
    };
    UserRepository::create(db, &user).await?;
    Ok(user.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{published_corpus, test_state};

    #[tokio::test]
    async fn seeding_fills_both_of_the_queues_tables() {
        let (state, _) = test_state("seed-fills").await;
        let published = published_corpus();

        let seeded = seed(&*state.db, &published, Utc::now())
            .await
            .expect("seeding should succeed")
            .expect("a corpus with two projects seeds");

        let submissions = SubmissionRepository::list(&*state.db).await.unwrap();
        assert_eq!(submissions.len(), 1);
        assert_eq!(submissions[0].shortcode, normalize_shortcode(&seeded.0));
        let drafts = DraftRepository::list(&*state.db).await.unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].shortcode, normalize_shortcode(&seeded.1));
    }

    #[tokio::test]
    async fn the_sample_submission_changes_a_scalar_and_a_language_map() {
        // The point of the sample: a diff with nothing in it demonstrates
        // nothing, and the language map is the row whose rendering is easiest
        // to get wrong.
        let (state, _) = test_state("seed-diff").await;
        let published = published_corpus();
        let (under_review, _) = seed(&*state.db, &published, Utc::now()).await.unwrap().unwrap();

        let submission = SubmissionRepository::find_by_shortcode(&*state.db, &normalize_shortcode(&under_review))
            .await
            .unwrap()
            .unwrap();
        let submitted: ProjectDraft = serde_json::from_str(&submission.payload).unwrap();
        let original = ProjectDraft::from_raw(published.get(&under_review).unwrap());

        let changed: Vec<String> = editor_core::review::diff(Some(&original), &submitted)
            .into_iter()
            .filter(editor_core::review::FieldDiff::changed)
            .map(|row| row.field)
            .collect();
        assert!(changed.contains(&"name".to_string()), "{changed:?}");
        assert!(changed.contains(&"abstract".to_string()), "{changed:?}");
    }

    #[tokio::test]
    async fn seeding_a_database_that_already_holds_a_submission_does_nothing() {
        // It must never write over what somebody is looking at, and a restarted
        // preview must not accumulate a second sample.
        let (state, _) = test_state("seed-idempotent").await;
        let published = published_corpus();
        seed(&*state.db, &published, Utc::now()).await.unwrap();

        assert!(seed(&*state.db, &published, Utc::now()).await.unwrap().is_none());
        assert_eq!(SubmissionRepository::list(&*state.db).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn seeding_without_a_published_set_does_nothing() {
        // A deployment with no data directory has nothing to diff against, and
        // a sample whose every field reads "new" would demonstrate the
        // degenerate case rather than the ordinary one.
        let (state, _) = test_state("seed-no-corpus").await;
        let empty = PublishedProjects::default();

        assert!(seed(&*state.db, &empty, Utc::now()).await.unwrap().is_none());
        assert!(SubmissionRepository::list(&*state.db).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_sample_depositor_can_reach_the_projects_it_is_seeded_for() {
        // Otherwise signing in as them lands on a list with nothing in it, and
        // the depositor's half of the flow cannot be walked at all.
        let (state, _) = test_state("seed-depositor").await;
        let published = published_corpus();
        let (under_review, in_progress) = seed(&*state.db, &published, Utc::now()).await.unwrap().unwrap();

        let depositor = UserRepository::find_by_email(&*state.db, DEPOSITOR)
            .await
            .unwrap()
            .expect("the sample depositor exists");
        assert_eq!(depositor.role, Role::Depositor);
        assert!(depositor.may_reach(&under_review));
        assert!(depositor.may_reach(&in_progress));
    }
}
