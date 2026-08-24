//! Account bootstrap: the RDU members that configuration says must always
//! exist (REQ-7.1, REQ-7.2).
//!
//! Depositor accounts are created by RDU through the interface, which is
//! DEV-6910's work. This module exists because RDU members are the exception —
//! "defined in configuration and shall always exist without provisioning" — and
//! because without it there is no account in a fresh database, so the login flow
//! cannot be exercised anywhere except a test.
//!
//! ## What it does not do
//!
//! An address removed from the configuration is **reported, not revoked**. The
//! removal policy — demote and keep the row, or delete it and its sessions,
//! drafts and submissions — is a role-model decision that belongs with the rest
//! of account management, and inventing half of it here would pre-empt it. A
//! startup warning names any `rdu` account the configuration no longer lists, so
//! the gap is visible rather than silent.

use chrono::{DateTime, Utc};
use editor_core::records::{Role, User};
use editor_core::repository::{RepositoryError, UserRepository};
use uuid::Uuid;

use crate::db::Database;

/// Create or promote an account for every configured RDU address, and report
/// any `rdu` account the configuration no longer names.
///
/// Returns how many accounts were created or promoted.
pub(crate) async fn ensure_rdu(
    db: &Database,
    addresses: &[String],
    now: DateTime<Utc>,
) -> Result<usize, RepositoryError> {
    let mut changed = 0;

    for address in addresses {
        match UserRepository::find_by_email(db, address).await? {
            None => {
                let user = User {
                    id: Uuid::new_v4(),
                    email: address.clone(),
                    // There is no name in the configuration, and inventing a
                    // pretty one would be a guess. The local part is what the
                    // person calls themselves in the address they gave, and RDU
                    // can change it once account editing exists.
                    name: default_name(address),
                    role: Role::Rdu,
                    // Empty by design: RDU access is role-based, not per-project
                    // (REQ-4.2).
                    shortcodes: Vec::new(),
                    failed_logins: 0,
                    failed_login_at: None,
                    last_code_at: None,
                    created_at: now,
                };
                UserRepository::create(db, &user).await?;
                tracing::info!(auth.subject = %user.id, "created an RDU account from configuration");
                changed += 1;
            }
            Some(user) if user.role != Role::Rdu => {
                // Configuration wins. An operator listing an address is an
                // explicit statement about who administers the service, and
                // leaving the row as a depositor would silently ignore it.
                let promoted = User { role: Role::Rdu, ..user };
                UserRepository::update(db, &promoted).await?;
                tracing::warn!(auth.subject = %promoted.id, "promoted an existing account to RDU from configuration");
                changed += 1;
            }
            Some(_) => {}
        }
    }

    report_unlisted(db, addresses).await?;
    Ok(changed)
}

/// The display name for an address with no name attached: its local part.
fn default_name(address: &str) -> String {
    address.split('@').next().unwrap_or(address).to_string()
}

/// Warn about every `rdu` account the configuration no longer lists.
///
/// Deliberately a report and not a revocation — see the module docs.
async fn report_unlisted(db: &Database, addresses: &[String]) -> Result<(), RepositoryError> {
    let listed: Vec<String> = addresses.iter().map(|address| User::normalize_email(address)).collect();
    for user in UserRepository::list(db).await? {
        if user.role == Role::Rdu && !listed.contains(&user.email_normalized()) {
            tracing::warn!(
                auth.subject = %user.id,
                "an RDU account is not listed in EDITOR_RDU_EMAILS; configuration does not revoke it, so remove it \
                 by hand if the access should end"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;

    use super::*;
    use crate::db::Source;

    async fn test_db(label: &str) -> Database {
        Database::open(Source::memory_for_test(label), 2, Duration::from_secs(5))
            .await
            .expect("the test database should open")
    }

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    fn addresses(list: &[&str]) -> Vec<String> {
        list.iter().map(|address| (*address).to_string()).collect()
    }

    #[tokio::test]
    async fn test_a_configured_address_gets_an_rdu_account() {
        let db = test_db("rdu-create").await;

        assert_eq!(ensure_rdu(&db, &addresses(&["rdu@dasch.swiss"]), at(10)).await.unwrap(), 1);

        let user = UserRepository::find_by_email(&db, "rdu@dasch.swiss").await.unwrap().unwrap();
        assert_eq!(user.role, Role::Rdu);
        assert_eq!(user.name, "rdu");
        assert!(user.shortcodes.is_empty(), "RDU access is role-based, not per-project");
    }

    #[tokio::test]
    async fn test_the_bootstrap_is_idempotent_across_restarts() {
        // It runs on every start, so a second run must find nothing to do rather
        // than fail on the unique address or duplicate the account.
        let db = test_db("rdu-idempotent").await;
        let configured = addresses(&["rdu@dasch.swiss"]);

        assert_eq!(ensure_rdu(&db, &configured, at(10)).await.unwrap(), 1);
        assert_eq!(ensure_rdu(&db, &configured, at(11)).await.unwrap(), 0);
        assert_eq!(UserRepository::list(&db).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_an_address_is_matched_however_it_is_capitalised() {
        // Otherwise a restart with a differently-typed address creates a second
        // account, and the unique index turns that into a failed startup.
        let db = test_db("rdu-case").await;

        ensure_rdu(&db, &addresses(&["RDU@Dasch.Swiss"]), at(10)).await.unwrap();
        assert_eq!(ensure_rdu(&db, &addresses(&["rdu@dasch.swiss"]), at(11)).await.unwrap(), 0);
        assert_eq!(UserRepository::list(&db).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_an_existing_depositor_is_promoted_rather_than_ignored() {
        // Configuration is the statement of who administers the service; leaving
        // the row as a depositor would silently ignore it.
        let db = test_db("rdu-promote").await;
        let existing = User {
            id: Uuid::new_v4(),
            email: "someone@dasch.swiss".to_string(),
            name: "Someone".to_string(),
            role: Role::Depositor,
            shortcodes: vec!["0801".to_string()],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(&db, &existing).await.unwrap();

        assert_eq!(ensure_rdu(&db, &addresses(&["someone@dasch.swiss"]), at(10)).await.unwrap(), 1);

        let promoted = UserRepository::find_by_id(&db, existing.id).await.unwrap().unwrap();
        assert_eq!(promoted.role, Role::Rdu);
        assert_eq!(
            promoted.name, "Someone",
            "the promotion must not overwrite what is already known"
        );
        assert_eq!(promoted.shortcodes, vec!["0801".to_string()]);
    }

    #[tokio::test]
    async fn test_removing_an_address_leaves_the_account_and_says_so() {
        // The removal policy belongs with account management; what belongs here
        // is that the gap is visible rather than silent.
        let db = test_db("rdu-unlisted").await;
        ensure_rdu(&db, &addresses(&["rdu@dasch.swiss"]), at(10)).await.unwrap();

        let (logs, guard) = crate::test_support::capture_logs();
        assert_eq!(ensure_rdu(&db, &addresses(&[]), at(11)).await.unwrap(), 0);
        drop(guard);

        assert_eq!(UserRepository::list(&db).await.unwrap().len(), 1, "the account is not revoked");
        assert!(
            logs.lines().iter().any(|line| line.contains("EDITOR_RDU_EMAILS")),
            "the unlisted account must be reported: {:?}",
            logs.lines()
        );
    }

    #[tokio::test]
    async fn test_no_configured_addresses_is_a_legitimate_state() {
        // The PR preview and a fresh checkout both run with none.
        let db = test_db("rdu-empty").await;
        assert_eq!(ensure_rdu(&db, &[], at(10)).await.unwrap(), 0);
        assert!(UserRepository::list(&db).await.unwrap().is_empty());
    }

    #[test]
    fn test_the_default_name_is_the_local_part() {
        assert_eq!(default_name("a.user@dasch.swiss"), "a.user");
        assert_eq!(default_name("nonsense"), "nonsense");
    }
}
