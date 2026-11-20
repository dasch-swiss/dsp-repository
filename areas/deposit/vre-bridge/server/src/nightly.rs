//! The nightly run: under the lock, once per local date, the sweep of leftover dump directories,
//! recovery, the removal of projects no longer allowlisted, then a refresh of each allowlisted
//! project, a busy one retried later in the same run.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use tokio::time::Instant;
use tracing::{error, info, warn, Instrument as _};
use vre_bridge_core::{AllowlistEntry, FailureReason, Outcome};
use vre_bridge_store::{Lock, RecoveryReport, RemovalReport, StoreError};

use crate::bridge::{log_recovery, sweep_and_log, Active};
use crate::refresh::spawn_blocking;
use crate::RefreshReport;

/// How often a nightly run tries the lock again while someone else holds it.
const LOCK_RETRY: Duration = Duration::from_secs(1);

/// How often a nightly run still waiting for the lock says so again.
const LOCK_WAIT_LOGGED: Duration = Duration::from_secs(600);

/// The `project` of a failed removal's report when the locations could not be listed: not a
/// shortcode, so it names no project.
const NO_PROJECT: &str = "(none)";

/// A project whose dump slot was busy, waiting for its next try.
struct Waiting<'a> {
    entry: &'a AllowlistEntry,
    not_before: DateTime<Utc>,
    /// Retries made so far.
    retries: u64,
    started: Instant,
}

/// What the run did before the projects.
struct Prepared {
    recovered: RecoveryReport,
    removed: Result<RemovalReport, StoreError>,
}

impl Active {
    /// One nightly run of local date `today`, the date it was scheduled for. It waits for the
    /// lock, then does nothing if the nightly-run record holds that date; otherwise it records the
    /// date before anything else, so a run cut short is not run again that night. The lock is
    /// held to the run's end, retries included.
    pub(crate) async fn nightly_run(self: &Arc<Self>, today: NaiveDate) {
        let span = tracing::info_span!("nightly_run", date = %today);
        async {
            let Some(lock) = self.lock_waiting().await else {
                return;
            };
            let Some(prepared) = self.prepare(&lock, today).await else {
                return;
            };
            info!("the nightly run started");
            log_recovery(&prepared.recovered);
            self.report_removal(prepared.removed);
            self.refresh_all(&lock).await;
            info!("the nightly run ended");
        }
        .instrument(span)
        .await;
    }

    /// Takes the lock, trying again every [`LOCK_RETRY`] with an async sleep in real time; `None`
    /// when it cannot be taken for another reason than a holder. The first wait is logged at info
    /// level, a long one at warning level every [`LOCK_WAIT_LOGGED`].
    async fn lock_waiting(&self) -> Option<Arc<Lock>> {
        let started = Instant::now();
        let mut next_log = started + LOCK_WAIT_LOGGED;
        let mut told = false;
        loop {
            let path = self.settings.lock_file.clone();
            match spawn_blocking(move || Lock::try_acquire(&path)).await {
                Ok(Ok(Some(lock))) => return Some(Arc::new(lock)),
                Ok(Ok(None)) => {
                    if !told {
                        info!("the lock is held; the nightly run waits for it");
                        told = true;
                    }
                    if Instant::now() >= next_log {
                        warn!(
                            waited_secs = started.elapsed().as_secs(),
                            "the nightly run still waits for the lock"
                        );
                        next_log += LOCK_WAIT_LOGGED;
                    }
                    tokio::time::sleep(LOCK_RETRY).await;
                }
                Ok(Err(e)) => {
                    error!(error = %e, "the lock cannot be taken; the nightly run is skipped");
                    return None;
                }
                Err(join) => {
                    error!(
                        panicked = join.is_panic(),
                        "taking the lock did not finish; the nightly run is skipped"
                    );
                    return None;
                }
            }
        }
    }

    /// The record, the sweep, recovery and removal, in that order; `None` when the run is not
    /// to go on. The projects without a token are taken first, so removal deletes only a
    /// directory that had no token when the run began.
    async fn prepare(&self, lock: &Arc<Lock>, today: NaiveDate) -> Option<Prepared> {
        let lock = Arc::clone(lock);
        let work_dir = self.settings.work_dir.clone();
        let subtrees = self.subtrees.clone();
        let shortcodes = self.shortcodes();
        let prepared = spawn_blocking(move || {
            if lock.nightly_done_on(today) {
                return Ok(None);
            }
            lock.record_nightly(today)?;
            let tokenless = subtrees.tokenless_projects();
            sweep_and_log(&work_dir);
            let recovered = subtrees.recover(&shortcodes);
            let removed = subtrees.remove_unlisted(&shortcodes, &tokenless);
            Ok::<_, StoreError>(Some(Prepared { recovered, removed }))
        })
        .await;
        match prepared {
            Ok(Ok(Some(prepared))) => Some(prepared),
            Ok(Ok(None)) => {
                info!("this date's nightly run has started already; nothing to do");
                None
            }
            Ok(Err(e)) => {
                error!(error = %e, "the nightly-run record cannot be written; the nightly run is skipped");
                None
            }
            Err(join) => {
                error!(panicked = join.is_panic(), "the nightly run's preparation did not finish");
                None
            }
        }
    }

    /// Each withdrawn project is reported as outcome `removed`, which logs it at warning level. A
    /// removal that failed is reported as failed with reason `store`, one per project, or once for
    /// [`NO_PROJECT`] when the locations could not be listed, so the outcome metric sees it.
    fn report_removal(&self, removed: Result<RemovalReport, StoreError>) {
        let removed = match removed {
            Ok(removed) => removed,
            Err(e) => {
                error!(error = %e, "removing projects no longer allowlisted failed");
                self.report_removed(NO_PROJECT.to_owned(), Outcome::Failed(FailureReason::Store));
                return;
            }
        };
        for project in removed.withdrawn {
            self.report_removed(project, Outcome::Removed);
        }
        for project in &removed.deleted {
            info!(project = %project, "the data of a project no longer allowlisted is deleted");
        }
        for (project, e) in removed.failed {
            error!(project = %project, error = %e, "removing a project no longer allowlisted failed");
            self.report_removed(project, Outcome::Failed(FailureReason::Store));
        }
    }

    /// A removal's report: no server, since the project's allowlist entry is gone.
    fn report_removed(&self, project: String, outcome: Outcome) {
        self.report.refresh(&RefreshReport {
            project,
            server: None,
            outcome,
            duration: Duration::ZERO,
            sizes: None,
        });
    }

    /// Refreshes each allowlisted project once; a busy one is queued and retried after
    /// `EDITOR_VRE_BRIDGE_DUMP_RETRY_SECS`, up to `EDITOR_VRE_BRIDGE_DUMP_RETRIES` times, then
    /// reported as failed. Between tries the run sleeps until the earliest one is due.
    async fn refresh_all(self: &Arc<Self>, lock: &Arc<Lock>) {
        let mut queue = Vec::new();
        for entry in self.settings.allowlist.entries() {
            let started = Instant::now();
            if self.refresh_guarded(lock, entry, false, started).await.is_err() {
                self.requeue(&mut queue, entry, 0, started);
            }
        }
        while let Some(earliest) = queue.iter().map(|waiting| waiting.not_before).min() {
            self.sleep_until(earliest).await;
            let now = self.clock.now();
            let (due, later): (Vec<_>, Vec<_>) = queue.into_iter().partition(|waiting| waiting.not_before <= now);
            queue = later;
            for waiting in due {
                if self.refresh_guarded(lock, waiting.entry, false, waiting.started).await.is_err() {
                    self.requeue(&mut queue, waiting.entry, waiting.retries + 1, waiting.started);
                }
            }
        }
    }

    /// Queues a busy project for its next retry, or reports it failed once it has had them all.
    fn requeue<'a>(&self, queue: &mut Vec<Waiting<'a>>, entry: &'a AllowlistEntry, retries: u64, started: Instant) {
        if retries >= self.settings.dump_retries {
            self.give_up(entry, started);
            return;
        }
        let Some(not_before) = self.later_by(self.settings.dump_retry) else {
            error!(project = %entry.shortcode, "the retry time is out of the clock's range");
            self.give_up(entry, started);
            return;
        };
        warn!(project = %entry.shortcode, retries, "the dump slot is busy; the project is tried again later");
        queue.push(Waiting { entry, not_before, retries, started });
    }
}
