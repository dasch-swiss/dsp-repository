//! [`EntityProposalRepository`] against SQLite.
//!
//! [`create_new`](Database::create_new) is the one method here that matters
//! beyond CRUD: it selects the ids already taken and inserts the new proposal
//! inside one `write` closure, which is `Database::write`'s `BEGIN IMMEDIATE`.
//! Two proposals for the same kind, submitted at once, still serialise through
//! the single writer connection — so the second one's `SELECT` always sees the
//! first one's `INSERT`, and the two can never compute the same next id. That
//! is the guard REQ-3.6 needs and REQ-5.4 does not give: REQ-5.4 renumbers
//! against the repository on collision, but nothing in it stops two proposals
//! inside the editor from allocating the same id before either reaches it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::proposals::{
    format_entity_id, next_entity_id, EntityProposal, ProposalDecision, ProposalKind, ProposalOperation, ProposalStatus,
};
use editor_core::records::normalize_shortcode;
use editor_core::repository::{EntityProposalRepository, RepositoryError, Result};
use rusqlite::{params, Row, Transaction};
use uuid::Uuid;

use super::mapping::{optional_parsed_column, optional_uuid_column, parsed_column, uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "entity proposal";

const SELECT: &str = "SELECT id, shortcode, entity_id, kind, operation, payload, status, decision, proposed_by, \
                      created_at, updated_at, decided_by, decided_at FROM entity_proposals";

fn map_row(row: &Row<'_>) -> rusqlite::Result<EntityProposal> {
    Ok(EntityProposal {
        id: uuid_column(row, 0)?,
        shortcode: row.get(1)?,
        entity_id: row.get(2)?,
        kind: parsed_column::<ProposalKind>(row, 3)?,
        operation: parsed_column::<ProposalOperation>(row, 4)?,
        payload: row.get(5)?,
        status: parsed_column::<ProposalStatus>(row, 6)?,
        decision: optional_parsed_column::<ProposalDecision>(row, 7)?,
        proposed_by: optional_uuid_column(row, 8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        decided_by: optional_uuid_column(row, 11)?,
        decided_at: row.get(12)?,
    })
}

fn insert_proposal(tx: &Transaction<'_>, proposal: &EntityProposal) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO entity_proposals (id, shortcode, entity_id, kind, operation, payload, status, decision, \
         proposed_by, created_at, updated_at, decided_by, decided_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            proposal.id.to_string(),
            proposal.shortcode,
            proposal.entity_id,
            proposal.kind.as_str(),
            proposal.operation.as_str(),
            proposal.payload,
            proposal.status.as_str(),
            proposal.decision.map(ProposalDecision::as_str),
            proposal.proposed_by.map(|id| id.to_string()),
            proposal.created_at,
            proposal.updated_at,
            proposal.decided_by.map(|id| id.to_string()),
            proposal.decided_at,
        ],
    )?;
    Ok(())
}

#[async_trait]
impl EntityProposalRepository for Database {
    async fn create_new(&self, proposal: &EntityProposal, published_floor: u32) -> Result<EntityProposal> {
        let mut proposal = proposal.clone();
        proposal.shortcode = normalize_shortcode(&proposal.shortcode);
        let kind = proposal.kind;
        self.write(move |tx| {
            // Every id of this kind, terminal rows included — `next_entity_id`'s docs say why a
            // rejected one still counts — plus `published_floor`, which stands in for the store
            // this layer cannot see.
            //
            // **Do not replace this with `SELECT MAX(entity_id)`.** `entity_id` is TEXT, so
            // SQLite compares it lexicographically — `format_entity_id`'s docs have the worked
            // example. Covered by `entity_proposals_kind_entity_id`, so reading every id of this
            // kind touches no table row.
            let mut stmt = tx.prepare("SELECT entity_id FROM entity_proposals WHERE kind = ?1")?;
            let mut taken: Vec<String> = stmt
                .query_map(params![kind.as_str()], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            taken.push(format_entity_id(kind, published_floor));
            proposal.entity_id = next_entity_id(kind, taken.iter().map(String::as_str));
            insert_proposal(tx, &proposal)?;
            Ok(proposal.clone())
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn create_change(&self, proposal: &EntityProposal) -> Result<()> {
        let mut proposal = proposal.clone();
        proposal.shortcode = normalize_shortcode(&proposal.shortcode);
        self.write(move |tx| insert_proposal(tx, &proposal))
            .await
            .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn update_payload(&self, id: Uuid, payload: &str, at: DateTime<Utc>) -> Result<()> {
        let payload = payload.to_string();
        let updated = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE entity_proposals SET payload = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id.to_string(), payload, at],
                )
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        if updated == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn set_decision(
        &self,
        id: Uuid,
        decision: Option<ProposalDecision>,
        by: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let updated = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE entity_proposals SET decision = ?2, decided_by = ?3, decided_at = ?4 WHERE id = ?1",
                    params![
                        id.to_string(),
                        decision.map(ProposalDecision::as_str),
                        by.map(|id| id.to_string()),
                        at
                    ],
                )
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        if updated == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<EntityProposal>> {
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE id = ?1"), params![id.to_string()], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<EntityProposal>> {
        let shortcode = normalize_shortcode(shortcode);
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(&format!("{SELECT} WHERE shortcode = ?1 ORDER BY created_at, id"))?;
                let rows = stmt.query_map(params![shortcode], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn list_live_for_entity(&self, entity_id: &str) -> Result<Vec<EntityProposal>> {
        let entity_id = entity_id.to_string();
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "{SELECT} WHERE entity_id = ?1 AND status IN ('{}', '{}') ORDER BY created_at, id",
                    ProposalStatus::Draft.as_str(),
                    ProposalStatus::Submitted.as_str(),
                ))?;
                let rows = stmt.query_map(params![entity_id], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn withdraw(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        let updated = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE entity_proposals SET status = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id.to_string(), ProposalStatus::Withdrawn.as_str(), at],
                )
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        if updated == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use editor_core::records::{Role, User};
    use editor_core::repository::UserRepository;

    use super::super::tests::test_db;
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 10, hour, 0, 0).unwrap()
    }

    async fn a_user(db: &Database, email: &str) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: email.to_string(),
            name: "A".to_string(),
            role: Role::Depositor,
            shortcodes: vec![],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    fn a_proposal(
        shortcode: &str,
        entity_id: &str,
        kind: ProposalKind,
        operation: ProposalOperation,
    ) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            entity_id: entity_id.to_string(),
            kind,
            operation,
            payload: r#"{"name":"placeholder"}"#.to_string(),
            status: ProposalStatus::Draft,
            proposed_by: None,
            created_at: at(10),
            updated_at: at(10),
            decision: None,
            decided_by: None,
            decided_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_new_allocates_one_past_the_published_floor() {
        let db = test_db("entity-proposals-allocate-empty").await;
        let proposal = EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Person, ProposalOperation::New),
            416,
        )
        .await
        .unwrap();
        assert_eq!(proposal.entity_id, "person-417");

        let second = EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Person, ProposalOperation::New),
            416,
        )
        .await
        .unwrap();
        assert_eq!(second.entity_id, "person-418");
    }

    #[tokio::test]
    async fn test_create_new_allocates_per_kind() {
        let db = test_db("entity-proposals-allocate-organization").await;
        let proposal = EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Organization, ProposalOperation::New),
            142,
        )
        .await
        .unwrap();
        assert_eq!(proposal.entity_id, "organization-143");
    }

    #[tokio::test]
    async fn test_a_second_new_proposal_naming_an_allocated_id_is_a_conflict() {
        // `entity_proposals_allocated_id`. Reachable through `create_change`,
        // which takes its `entity_id` from the caller rather than allocating —
        // exactly what a second `new` proposal racing the allocation would
        // otherwise collide on directly.
        let db = test_db("entity-proposals-allocated-conflict").await;
        EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Person, ProposalOperation::New),
            416,
        )
        .await
        .unwrap();

        let error = EntityProposalRepository::create_change(
            &db,
            &a_proposal("0803", "person-417", ProposalKind::Person, ProposalOperation::New),
        )
        .await
        .expect_err("a second proposal naming an already-allocated id must be refused");
        assert!(
            matches!(error, RepositoryError::Conflict { entity: "entity proposal" }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn test_an_allocated_id_is_not_reused_after_its_proposal_is_rejected() {
        // Decision 2 on the issue: a rejected row is terminal but stays in the
        // uniqueness index, so the id it took is spent for good.
        let db = test_db("entity-proposals-no-reuse-after-reject").await;
        let first = EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Person, ProposalOperation::New),
            416,
        )
        .await
        .unwrap();
        assert_eq!(first.entity_id, "person-417");

        db.write(move |tx| {
            tx.execute(
                "UPDATE entity_proposals SET status = 'rejected' WHERE id = ?1",
                params![first.id.to_string()],
            )
        })
        .await
        .unwrap();

        let second = EntityProposalRepository::create_new(
            &db,
            &a_proposal("0801", "", ProposalKind::Person, ProposalOperation::New),
            416,
        )
        .await
        .unwrap();
        assert_eq!(second.entity_id, "person-418", "person-417 must stay spent");
    }

    #[tokio::test]
    async fn test_two_projects_may_each_propose_a_change_to_one_entity() {
        let db = test_db("entity-proposals-change-two-projects").await;
        EntityProposalRepository::create_change(
            &db,
            &a_proposal(
                "0801",
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            ),
        )
        .await
        .unwrap();
        EntityProposalRepository::create_change(
            &db,
            &a_proposal(
                "0803",
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            ),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_a_second_live_change_proposal_for_one_project_is_a_conflict() {
        // `entity_proposals_live_per_entity`: a depositor must not be able to
        // hold two live proposals about the same entity, which the review
        // surface would then show as two independent decisions over one file.
        let db = test_db("entity-proposals-live-per-entity-conflict").await;
        let first = EntityProposalRepository::create_change(
            &db,
            &a_proposal(
                "0801",
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            ),
        )
        .await;
        assert!(first.is_ok());

        let error = EntityProposalRepository::create_change(
            &db,
            &a_proposal(
                "0801",
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            ),
        )
        .await
        .expect_err("a second live proposal for one project must be refused");
        assert!(
            matches!(error, RepositoryError::Conflict { entity: "entity proposal" }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn test_a_project_may_propose_again_once_its_live_change_is_withdrawn() {
        let db = test_db("entity-proposals-change-after-withdraw").await;
        let first = a_proposal(
            "0801",
            "organization-008",
            ProposalKind::Organization,
            ProposalOperation::Change,
        );
        EntityProposalRepository::create_change(&db, &first).await.unwrap();
        EntityProposalRepository::withdraw(&db, first.id, at(11)).await.unwrap();

        EntityProposalRepository::create_change(
            &db,
            &a_proposal(
                "0801",
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            ),
        )
        .await
        .expect("a project may propose again once its earlier live proposal is withdrawn");
    }

    #[tokio::test]
    async fn test_every_column_round_trips_through_find_decision_present() {
        let db = test_db("entity-proposals-round-trip-decision-some").await;
        let author = a_user(&db, "a@x.test").await;
        let reviewer = a_user(&db, "rdu@x.test").await;
        let proposal = EntityProposal {
            id: Uuid::new_v4(),
            shortcode: "0801".to_string(),
            entity_id: "person-417".to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::New,
            payload: r#"{"givenNames":["Ada"]}"#.to_string(),
            status: ProposalStatus::Submitted,
            proposed_by: Some(author),
            created_at: at(10),
            updated_at: at(11),
            decision: Some(ProposalDecision::Accept),
            decided_by: Some(reviewer),
            decided_at: Some(at(12)),
        };
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        assert_eq!(EntityProposalRepository::find(&db, proposal.id).await.unwrap(), Some(proposal));
    }

    #[tokio::test]
    async fn test_every_column_round_trips_through_find_decision_absent() {
        let db = test_db("entity-proposals-round-trip-decision-none").await;
        let proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        assert_eq!(EntityProposalRepository::find(&db, proposal.id).await.unwrap(), Some(proposal));
    }

    #[tokio::test]
    async fn test_list_for_shortcode_is_oldest_first_and_folds_shortcode_case() {
        let db = test_db("entity-proposals-list-for-shortcode").await;
        let mut second = a_proposal("080C", "person-418", ProposalKind::Person, ProposalOperation::New);
        second.created_at = at(12);
        let mut first = a_proposal("080C", "person-417", ProposalKind::Person, ProposalOperation::New);
        first.created_at = at(11);
        EntityProposalRepository::create_change(&db, &second).await.unwrap();
        EntityProposalRepository::create_change(&db, &first).await.unwrap();

        let ids: Vec<_> = EntityProposalRepository::list_for_shortcode(&db, "080c")
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec![first.id, second.id]);
    }

    #[tokio::test]
    async fn test_list_live_for_entity_excludes_terminal_statuses() {
        let db = test_db("entity-proposals-list-live-for-entity").await;
        for (shortcode, status) in [
            ("0801", ProposalStatus::Draft),
            ("0803", ProposalStatus::Submitted),
            ("0805", ProposalStatus::Accepted),
            ("0807", ProposalStatus::Rejected),
            ("0809", ProposalStatus::Withdrawn),
        ] {
            let mut proposal = a_proposal(
                shortcode,
                "organization-008",
                ProposalKind::Organization,
                ProposalOperation::Change,
            );
            proposal.status = status;
            db.write(move |tx| insert_proposal(tx, &proposal)).await.unwrap();
        }

        let live = EntityProposalRepository::list_live_for_entity(&db, "organization-008")
            .await
            .unwrap();
        let mut shortcodes: Vec<_> = live.into_iter().map(|p| p.shortcode).collect();
        shortcodes.sort();
        assert_eq!(shortcodes, vec!["0801".to_string(), "0803".to_string()]);
    }

    #[tokio::test]
    async fn test_a_known_status_reads_back_through_its_from_str() {
        // Pins that `parsed_column` is actually wired to `status` here, the way
        // `submissions.rs`'s equivalent test does. The unknown-status case
        // itself is `proposals.rs`'s
        // `test_unknown_stored_proposal_status_is_an_error_not_a_default` and is not
        // repeated here — reaching a corrupt row would mean defeating the `CHECK`
        // constraint on the write.
        let db = test_db("entity-proposals-status-round-trip").await;
        let mut proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        proposal.status = ProposalStatus::Submitted;
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.status, ProposalStatus::Submitted);
    }

    #[tokio::test]
    async fn test_update_payload_stamps_updated_at() {
        let db = test_db("entity-proposals-update-payload").await;
        let mut proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        proposal.updated_at = at(10);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        EntityProposalRepository::update_payload(&db, proposal.id, r#"{"name":"changed"}"#, at(13))
            .await
            .unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.payload, r#"{"name":"changed"}"#);
        assert_eq!(found.updated_at, at(13));
    }

    #[tokio::test]
    async fn test_update_payload_on_an_unknown_id_is_not_found() {
        let db = test_db("entity-proposals-update-payload-missing").await;
        let error = EntityProposalRepository::update_payload(&db, Uuid::new_v4(), "{}", at(10))
            .await
            .expect_err("an unknown proposal must not update");
        assert!(
            matches!(error, RepositoryError::NotFound { entity: "entity proposal" }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn test_set_decision_records_who_decided_what_and_leaves_status_alone() {
        let db = test_db("entity-proposals-set-decision").await;
        let reviewer = a_user(&db, "rdu@x.test").await;
        let proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        EntityProposalRepository::set_decision(
            &db,
            proposal.id,
            Some(ProposalDecision::Reject),
            Some(reviewer),
            at(13),
        )
        .await
        .unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.decision, Some(ProposalDecision::Reject));
        assert_eq!(found.decided_by, Some(reviewer));
        assert_eq!(found.decided_at, Some(at(13)));
        assert_eq!(
            found.status,
            ProposalStatus::Draft,
            "the status changes only when the round ends"
        );
    }

    #[tokio::test]
    async fn test_set_decision_clears_a_decision() {
        let db = test_db("entity-proposals-clear-decision").await;
        let mut proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        proposal.decision = Some(ProposalDecision::Accept);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        EntityProposalRepository::set_decision(&db, proposal.id, None, None, at(13))
            .await
            .unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.decision, None);
        assert_eq!(found.decided_by, None);
    }

    #[tokio::test]
    async fn test_set_decision_on_an_unknown_id_is_not_found() {
        let db = test_db("entity-proposals-set-decision-missing").await;
        let error =
            EntityProposalRepository::set_decision(&db, Uuid::new_v4(), Some(ProposalDecision::Accept), None, at(10))
                .await
                .expect_err("an unknown proposal must not update");
        assert!(
            matches!(error, RepositoryError::NotFound { entity: "entity proposal" }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn test_withdraw_tombstones_the_row_rather_than_deleting_it() {
        let db = test_db("entity-proposals-withdraw").await;
        let proposal = a_proposal("0801", "person-417", ProposalKind::Person, ProposalOperation::New);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        EntityProposalRepository::withdraw(&db, proposal.id, at(13)).await.unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.status, ProposalStatus::Withdrawn);
        assert_eq!(found.updated_at, at(13));
    }

    #[tokio::test]
    async fn test_withdraw_on_an_unknown_id_is_not_found() {
        let db = test_db("entity-proposals-withdraw-missing").await;
        let error = EntityProposalRepository::withdraw(&db, Uuid::new_v4(), at(10))
            .await
            .expect_err("an unknown proposal must not withdraw");
        assert!(
            matches!(error, RepositoryError::NotFound { entity: "entity proposal" }),
            "{error}"
        );
    }
}
