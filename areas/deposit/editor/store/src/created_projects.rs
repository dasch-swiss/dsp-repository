//! [`CreatedProjectRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::proposals::ProposalStatus;
use editor_core::records::{CreatedProject, DraftRecord};
use editor_core::repository::{CreatedProjectRepository, Removal, Result};
use rusqlite::{params, Row};

use super::mapping::{optional_uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "created project";

const SELECT: &str = "SELECT shortcode, name, created_by, created_at FROM created_projects";

/// Whether `?1` has work a removal would destroy; [`CreatedProjectRepository::removable`] and
/// [`CreatedProjectRepository::remove`] both read it.
const PENDING: &str = "SELECT EXISTS (SELECT 1 FROM submissions WHERE shortcode = ?1) \
                       OR EXISTS (SELECT 1 FROM approved_records WHERE shortcode = ?1)";

fn map_row(row: &Row<'_>) -> rusqlite::Result<CreatedProject> {
    Ok(CreatedProject {
        shortcode: row.get(0)?,
        name: row.get(1)?,
        created_by: optional_uuid_column(row, 2)?,
        created_at: row.get(3)?,
    })
}

#[async_trait]
impl CreatedProjectRepository for Database {
    async fn create(&self, project: &CreatedProject, draft: &DraftRecord) -> Result<()> {
        let project = project.clone();
        let draft = draft.clone();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO created_projects (shortcode, name, created_by, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    project.shortcode,
                    project.name,
                    project.created_by.map(|id| id.to_string()),
                    project.created_at,
                ],
            )?;
            // A plain insert, unlike `DraftRepository::upsert`: an existing draft is a conflict.
            tx.execute(
                "INSERT INTO drafts (shortcode, payload, updated_by, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    draft.shortcode,
                    draft.payload,
                    draft.updated_by.map(|id| id.to_string()),
                    draft.created_at,
                    draft.updated_at,
                ],
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn find(&self, shortcode: &str) -> Result<Option<CreatedProject>> {
        let shortcode = shortcode.to_string();
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE shortcode = ?1"), params![shortcode], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn list(&self) -> Result<Vec<CreatedProject>> {
        Ok(self
            .read(|conn| {
                let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY created_at, shortcode"))?;
                let rows = stmt.query_map([], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn delete_registration(&self, shortcode: &str) -> Result<bool> {
        let shortcode = shortcode.to_string();
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM created_projects WHERE shortcode = ?1", params![shortcode]))
            .await?;
        Ok(deleted > 0)
    }

    async fn removable(&self, shortcode: &str) -> Result<bool> {
        let shortcode = shortcode.to_string();
        let pending: bool = self
            .read(move |conn| conn.query_row(PENDING, params![shortcode], |row| row.get(0)))
            .await?;
        Ok(!pending)
    }

    async fn remove(&self, shortcode: &str, at: DateTime<Utc>) -> Result<Removal> {
        let shortcode = shortcode.to_string();
        Ok(self
            .write(move |tx| {
                let registered: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM created_projects WHERE shortcode = ?1)",
                    params![shortcode],
                    |row| row.get(0),
                )?;
                if !registered {
                    return Ok(Removal::NotFound);
                }
                let pending: bool = tx.query_row(PENDING, params![shortcode], |row| row.get(0))?;
                if pending {
                    return Ok(Removal::Pending);
                }
                // Withdrawn rather than deleted: a proposal's row keeps its allocated id taken. An
                // accepted, unretired one is withdrawn too, or the next approved record under the
                // shortcode would carry it.
                tx.execute(
                    "UPDATE entity_proposals SET status = ?2, updated_at = ?3 \
                     WHERE shortcode = ?1 AND (status IN (?4, ?5) OR (status = ?6 AND retired_at IS NULL))",
                    params![
                        shortcode,
                        ProposalStatus::Withdrawn.as_str(),
                        at,
                        ProposalStatus::Draft.as_str(),
                        ProposalStatus::Submitted.as_str(),
                        ProposalStatus::Accepted.as_str(),
                    ],
                )?;
                tx.execute("DELETE FROM review_rounds WHERE shortcode = ?1", params![shortcode])?;
                tx.execute("DELETE FROM drafts WHERE shortcode = ?1", params![shortcode])?;
                tx.execute("DELETE FROM created_projects WHERE shortcode = ?1", params![shortcode])?;
                Ok(Removal::Removed)
            })
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation};
    use editor_core::records::{Submission, SubmissionState};
    use editor_core::repository::{DraftRepository, EntityProposalRepository, RepositoryError};
    use uuid::Uuid;

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
    }

    fn project(shortcode: &str, hour: u32) -> CreatedProject {
        CreatedProject {
            shortcode: shortcode.to_string(),
            name: format!("Project {shortcode}"),
            created_by: Some(Uuid::new_v4()),
            created_at: at(hour),
        }
    }

    fn seed(shortcode: &str) -> DraftRecord {
        DraftRecord {
            shortcode: shortcode.to_string(),
            payload: format!(r#"{{"shortcode":"{shortcode}"}}"#),
            updated_by: None,
            created_at: at(10),
            updated_at: at(10),
        }
    }

    #[tokio::test]
    async fn test_create_writes_the_registration_and_its_draft() {
        let db = test_db("created-create").await;
        let created = project("1111", 10);
        db.create(&created, &seed("1111")).await.unwrap();

        assert_eq!(CreatedProjectRepository::find(&db, "1111").await.unwrap(), Some(created));
        assert_eq!(
            DraftRepository::find(&db, "1111").await.unwrap().map(|d| d.payload),
            Some(seed("1111").payload)
        );
    }

    #[tokio::test]
    async fn test_create_over_an_existing_draft_is_a_conflict_and_writes_nothing() {
        // A depositor assigned the shortcode can save a draft before RDU creates the project;
        // the creation must not overwrite it, nor leave a registration behind.
        let db = test_db("created-over-draft").await;
        let mut theirs = seed("1111");
        theirs.payload = r#"{"name":"the depositor's work"}"#.to_string();
        db.upsert(&theirs).await.unwrap();

        let result = db.create(&project("1111", 10), &seed("1111")).await;
        assert!(matches!(result, Err(RepositoryError::Conflict { .. })), "{result:?}");
        assert_eq!(count(&db, "created_projects").await, 0);
        assert_eq!(
            DraftRepository::find(&db, "1111").await.unwrap().unwrap().payload,
            theirs.payload
        );
    }

    #[tokio::test]
    async fn test_creating_a_shortcode_twice_is_a_conflict() {
        let db = test_db("created-twice").await;
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();
        DraftRepository::delete(&db, "1111").await.unwrap();

        let result = db.create(&project("1111", 11), &seed("1111")).await;
        assert!(matches!(result, Err(RepositoryError::Conflict { .. })), "{result:?}");
    }

    #[tokio::test]
    async fn test_list_returns_every_registration_oldest_first() {
        let db = test_db("created-list").await;
        db.create(&project("2222", 12), &seed("2222")).await.unwrap();
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();

        let shortcodes: Vec<_> = CreatedProjectRepository::list(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.shortcode)
            .collect();
        assert_eq!(shortcodes, ["1111", "2222"]);
    }

    #[tokio::test]
    async fn test_delete_registration_keeps_the_draft() {
        let db = test_db("created-delete").await;
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();

        assert!(CreatedProjectRepository::delete_registration(&db, "1111").await.unwrap());
        assert_eq!(count(&db, "created_projects").await, 0);
        assert_eq!(count(&db, "drafts").await, 1);
        assert!(!CreatedProjectRepository::delete_registration(&db, "1111").await.unwrap());
    }

    fn proposal(entity_id: &str, status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: "1111".to_string(),
            entity_id: entity_id.to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::Change,
            payload: r#"{"name":"placeholder"}"#.to_string(),
            status,
            proposed_by: None,
            created_at: at(10),
            updated_at: at(10),
            decision: None,
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    #[tokio::test]
    async fn test_remove_deletes_the_project_and_withdraws_its_live_proposals() {
        // A project created again under the shortcode must not find the removed one's proposals or
        // rounds.
        let db = test_db("created-remove").await;
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();
        let live = proposal("person-001", ProposalStatus::Draft);
        let accepted = proposal("person-002", ProposalStatus::Accepted);
        let mut retired = proposal("person-003", ProposalStatus::Accepted);
        retired.retired_at = Some(at(11));
        for row in [&live, &accepted, &retired] {
            EntityProposalRepository::create_change(&db, row).await.unwrap();
        }
        // A round of an earlier, rejected submission: nothing pending, but the form would show it.
        db.write(|tx| {
            tx.execute(
                "INSERT INTO review_rounds (id, shortcode, submission_id, outcome, at) \
                 VALUES (?1, '1111', ?2, 'rejected', ?3)",
                params![Uuid::new_v4().to_string(), Uuid::new_v4().to_string(), at(11)],
            )
        })
        .await
        .unwrap();

        assert_eq!(db.remove("1111", at(12)).await.unwrap(), Removal::Removed);

        assert_eq!(count(&db, "created_projects").await, 0);
        assert_eq!(count(&db, "drafts").await, 0);
        assert_eq!(count(&db, "review_rounds").await, 0);
        for (row, status) in [
            (&live, ProposalStatus::Withdrawn),
            (&accepted, ProposalStatus::Withdrawn),
            (&retired, ProposalStatus::Accepted),
        ] {
            let found = EntityProposalRepository::find(&db, row.id).await.unwrap().unwrap();
            assert_eq!(found.status, status, "{}", row.entity_id);
        }
        assert_eq!(db.remove("1111", at(13)).await.unwrap(), Removal::NotFound);
    }

    #[tokio::test]
    async fn test_remove_with_a_submission_writes_nothing() {
        // The submission arrives after the handler's own check; the transaction's check decides.
        let db = test_db("created-remove-pending").await;
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();
        let live = proposal("person-001", ProposalStatus::Draft);
        EntityProposalRepository::create_change(&db, &live).await.unwrap();
        editor_core::repository::SubmissionRepository::create(
            &db,
            &Submission {
                id: Uuid::new_v4(),
                shortcode: "1111".to_string(),
                payload: seed("1111").payload,
                state: SubmissionState::Submitted,
                submitted_by: None,
                submitted_at: at(11),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
                review_state: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(db.remove("1111", at(12)).await.unwrap(), Removal::Pending);

        assert_eq!(count(&db, "created_projects").await, 1);
        assert_eq!(count(&db, "drafts").await, 1);
        assert_eq!(
            EntityProposalRepository::find(&db, live.id).await.unwrap().unwrap().status,
            ProposalStatus::Submitted,
            "removal leaves the proposal untouched"
        );
    }

    #[tokio::test]
    async fn test_remove_with_an_approved_record_writes_nothing() {
        let db = test_db("created-remove-approved").await;
        db.create(&project("1111", 10), &seed("1111")).await.unwrap();
        let live = proposal("person-001", ProposalStatus::Draft);
        EntityProposalRepository::create_change(&db, &live).await.unwrap();
        editor_core::repository::ApprovedRecordRepository::create(
            &db,
            &editor_core::records::ApprovedRecord {
                id: Uuid::new_v4(),
                shortcode: "1111".to_string(),
                payload: seed("1111").payload,
                approved_by: None,
                approved_at: at(11),
            },
        )
        .await
        .unwrap();

        assert!(!db.removable("1111").await.unwrap());
        assert_eq!(db.remove("1111", at(12)).await.unwrap(), Removal::Pending);

        assert_eq!(count(&db, "created_projects").await, 1);
        assert_eq!(count(&db, "drafts").await, 1);
        assert_eq!(
            EntityProposalRepository::find(&db, live.id).await.unwrap().unwrap().status,
            ProposalStatus::Draft
        );
    }
}
