//! One refresh of one project: recovery, the decision, the change check, the export, the dump's
//! download and unpacking, the lift, the new generation in both subtrees, the report.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use shared_stand_in::{Generation, Provenance, ReadyToken};
use tempfile::TempDir;
use tokio::task::JoinError;
use tokio::time::Instant;
use tracing::{error, info, warn};
use vre_bridge_client::{
    private_work_dir, unpack, ClientError, DeleteOutcome, ExportStatus, ExportTrigger, DUMP_DIR_PREFIX,
};
use vre_bridge_core::{
    decide, inputs_digest, AllowlistEntry, Decision, FailureReason, Outcome, RefusalReason, Running, ServerId, Settings,
};
use vre_bridge_lift::{LiftError, Lifted, Violation};
use vre_bridge_store::{new_generation_name, Lock, StoreError, Subtree, Subtrees, TokenState};

use crate::{Clock, RefreshReport, RefreshSizes, Report, Vre};

/// The interim snapshot's name in a `sync` generation. `sync-store` reads the same name, its own
/// `SNAPSHOT_FILE` in `areas/access/sync/store/src/lib.rs`; `shared-stand-in` holds no constant
/// for it, so the two change together.
pub const SNAPSHOT_FILE: &str = "snapshot.nq";

/// How far ahead of the bridge's clock a forced refresh's change check asks: a cutoff in the
/// future lists no resource and yields only `asOf`, since the endpoint has no paging.
const FORCED_CUTOFF_AHEAD: TimeDelta = TimeDelta::days(1);

/// The public guard's violations logged one line each; the rest are counted in the total.
const LOGGED_VIOLATIONS: usize = 50;

/// Consecutive failed status polls tolerated within the export's time limit: one dropped
/// connection does not end an export that may have run for an hour.
const STATUS_POLL_ATTEMPTS: u32 = 3;

/// A fresh generation name collides with an existing directory only by chance; drawn this often.
const GENERATION_ATTEMPTS: u32 = 3;

/// Another caller's export holds the server's dump slot. Not an outcome: never reported or
/// counted; the caller retries, and once it gives up reports `Failed(FailureReason::Busy)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

impl std::fmt::Display for Busy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("another caller's export holds the dump slot")
    }
}

impl std::error::Error for Busy {}

/// Why waiting for an export ended without its end.
enum Waited {
    TimedOut,
    Client(ClientError),
}

fn left_behind(id: &str, reason: &dyn std::fmt::Display) {
    error!(export = %id, reason = %reason, "the export could not be deleted; the next refresh deletes it");
}

/// Why a refresh stopped before writing.
enum Stop {
    Busy,
    Ended(Outcome),
}

impl From<Outcome> for Stop {
    fn from(outcome: Outcome) -> Self {
        Stop::Ended(outcome)
    }
}

struct Done {
    outcome: Outcome,
    sizes: Option<RefreshSizes>,
}

/// The project's state when the refresh started, which the new tokens build on.
struct Start {
    running: Running,
    decision: Decision,
    previous: Option<Generation>,
}

type LiftFn = fn(&Path) -> Result<Lifted, LiftError>;

/// Refreshes one project at a time; its callers hold the [`Lock`].
pub struct Refresher {
    vre: Arc<dyn Vre>,
    report: Arc<dyn Report>,
    subtrees: Subtrees,
    settings: Arc<Settings>,
    clock: Arc<dyn Clock>,
    /// `vre_bridge_lift::lift`; a test puts a panicking one here.
    lift: LiftFn,
}

impl Refresher {
    #[must_use]
    pub fn new(
        vre: Arc<dyn Vre>,
        report: Arc<dyn Report>,
        subtrees: Subtrees,
        settings: Arc<Settings>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            vre,
            report,
            subtrees,
            settings,
            clock,
            lift: vre_bridge_lift::lift,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_lift(mut self, lift: LiftFn) -> Self {
        self.lift = lift;
        self
    }

    /// Refreshes `entry`'s project from its server; `operator` forces it. A refused or failed
    /// refresh leaves the project's tokens as they were, and every outcome is reported.
    ///
    /// The future must run to its end, never be dropped early: a blocking step, the generation's
    /// writing included, cannot be stopped, and would go on after the caller released the lock.
    /// Each wait for an export is bounded by `EDITOR_VRE_BRIDGE_EXPORT_TIMEOUT_SECS`, and a refresh
    /// may wait several times: for its own leftover export, its new one, and once more to delete
    /// it.
    ///
    /// # Errors
    ///
    /// [`Busy`] when another caller's export holds the server's dump slot; nothing is reported.
    #[tracing::instrument(skip_all, fields(project = %entry.shortcode, server = %entry.server))]
    pub async fn refresh(&self, _held: &Lock, entry: &AllowlistEntry, operator: bool) -> Result<Outcome, Busy> {
        let started = Instant::now();
        let done = match self.run(entry, operator).await {
            Ok(done) => done,
            Err(Stop::Busy) => {
                info!("another caller's export holds the dump slot");
                return Err(Busy);
            }
            Err(Stop::Ended(outcome)) => Done { outcome, sizes: None },
        };
        self.report.refresh(&RefreshReport {
            project: entry.shortcode.clone(),
            server: entry.server.clone(),
            outcome: done.outcome,
            duration: started.elapsed(),
            sizes: done.sizes,
        });
        Ok(done.outcome)
    }

    async fn run(&self, entry: &AllowlistEntry, operator: bool) -> Result<Done, Stop> {
        let start = self.start(entry, operator).await?;
        let server = &entry.server;
        let iri = self
            .vre
            .project_iri(server, &entry.shortcode)
            .await
            .map_err(|e| client_failure("the project lookup", &e))?;

        let cutoff = match start.decision {
            Decision::Forced(reason) => {
                info!(reason = reason.as_str(), "the refresh is forced");
                self.clock.now() + FORCED_CUTOFF_AHEAD
            }
            Decision::CheckChanges { cutoff } => cutoff,
        };
        let changes = self
            .vre
            .changes(server, &iri, cutoff)
            .await
            .map_err(|e| client_failure("the change check", &e))?;
        if matches!(start.decision, Decision::CheckChanges { .. }) && changes.count == 0 {
            return Ok(Done { outcome: Outcome::Unchanged, sizes: None });
        }
        let t = DateTime::parse_from_rfc3339(&changes.as_of)
            .map(|t| t.with_timezone(&Utc))
            .map_err(|_| {
                error!("the change check's asOf is not an RFC 3339 time");
                Outcome::Failed(FailureReason::Server)
            })?;

        let work = Arc::new(
            self.blocking("creating the private work directory", FailureReason::Internal, {
                let parent = self.settings.work_dir.clone();
                move || private_work_dir(&parent)
            })
            .await?,
        );
        let done = self.dump_and_write(entry, &iri, t, start, &work).await;
        remove_work_dir(work).await;
        done
    }

    /// Recovers the project, reads its tokens and digests its shipped inputs. A project without
    /// its inputs directory is refused here, before anything is asked of the server; it is looked
    /// for on every refresh, so a directory a later release ships is found without a restart.
    async fn start(&self, entry: &AllowlistEntry, operator: bool) -> Result<Start, Stop> {
        let (sync, media) = self
            .blocking("recovering the project", FailureReason::Store, {
                let subtrees = self.subtrees.clone();
                let shortcode = entry.shortcode.clone();
                move || subtrees.recover_project(&shortcode).map(|()| subtrees.read_tokens(&shortcode))
            })
            .await?;

        let inputs = self.settings.inputs_dir.clone();
        let dir = inputs.join(&entry.shortcode);
        let digest = self
            .blocking("digesting the shipped inputs", FailureReason::Inputs, {
                let dir = dir.clone();
                move || {
                    // A wrong inputs directory fails the bridge; it refuses no project.
                    if !inputs.is_dir() {
                        return Err(format!("the inputs directory {} is not a directory", inputs.display()));
                    }
                    // A symlink, or anything but a directory, fails in the digest.
                    match std::fs::symlink_metadata(&dir) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                        _ => {}
                    }
                    inputs_digest(&dir).map(Some).map_err(|e| e.to_string())
                }
            })
            .await?;
        let Some(digest) = digest else {
            warn!(path = %dir.display(), "the project has no shipped-inputs directory");
            return Err(Outcome::Refused(RefusalReason::InputsMissing).into());
        };

        let (sync, media) = (valid(sync), valid(media));
        let running = Running::new(digest);
        let decision = decide(sync.as_ref(), media.as_ref(), entry, &running, operator);
        let previous = sync.map(|token| token.provenance.generation);
        Ok(Start { running, decision, previous })
    }

    async fn dump_and_write(
        &self,
        entry: &AllowlistEntry,
        iri: &str,
        t: DateTime<Utc>,
        start: Start,
        work: &Arc<TempDir>,
    ) -> Result<Done, Stop> {
        let server = &entry.server;
        let id = self.start_export(server, iri).await?;
        let downloaded = self.download(server, iri, &id, work).await;
        self.delete_export(server, iri, &id).await;
        let zip = downloaded?;

        let max_unpacked = self.settings.max_unpacked_bytes;
        let dump_dir = self
            .blocking("unpacking the dump", FailureReason::Dump, {
                let work = Arc::clone(work);
                let zip = zip.path.clone();
                // The zip holds `admin.nq`: gone as soon as what is needed is out of it.
                move || {
                    let unpacked = unpack(&zip, &work, max_unpacked);
                    let _ = std::fs::remove_file(&zip);
                    unpacked.map(|dump| dump.path().to_owned())
                }
            })
            .await?;
        let snapshot = self.lift(dump_dir).await?;

        let token = Token {
            as_of: t.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
            server: server.as_str().to_owned(),
            running: start.running,
            previous: start.previous,
        };
        let sizes = self
            .blocking("writing the generation", FailureReason::Store, {
                let subtrees = self.subtrees.clone();
                let shortcode = entry.shortcode.clone();
                let inputs = self.settings.inputs_dir.join(&entry.shortcode);
                move || write_generation(&subtrees, &shortcode, t, &snapshot, &inputs, token)
            })
            .await?;
        Ok(Done {
            outcome: Outcome::Refreshed,
            sizes: Some(RefreshSizes { dump: zip.size, snapshot: sizes.sync, media: sizes.media }),
        })
    }

    /// Triggers the export. The bridge's own leftover export in the slot is waited for, deleted
    /// and the trigger asked again, once per refresh.
    async fn start_export(&self, server: &ServerId, iri: &str) -> Result<String, Stop> {
        let trigger = |what| async move { self.vre.export(server, iri).await.map_err(|e| client_failure(what, &e)) };
        let occupant = match trigger("triggering the export").await? {
            ExportTrigger::Started { id } => return Ok(id),
            ExportTrigger::Busy => return Err(Stop::Busy),
            ExportTrigger::Occupied { id } => id,
        };
        if !self.is_own(server, iri, &occupant).await? {
            return Err(Stop::Busy);
        }
        warn!(export = %occupant, "the dump slot holds the bridge's own export; deleting it once it ends");
        let finished = self.wait_finished(server, iri, &occupant).await;
        self.delete_export(server, iri, &occupant).await;
        finished?;
        let occupant = match trigger("triggering the export again").await? {
            ExportTrigger::Started { id } => return Ok(id),
            ExportTrigger::Busy => return Err(Stop::Busy),
            ExportTrigger::Occupied { id } => id,
        };
        if !self.is_own(server, iri, &occupant).await? {
            return Err(Stop::Busy);
        }
        error!(export = %occupant, "the dump slot holds the bridge's own export again");
        Err(Outcome::Failed(FailureReason::Server).into())
    }

    /// Whether the export is the bridge's own: dsp-api answers its status only to its owner.
    async fn is_own(&self, server: &ServerId, iri: &str, id: &str) -> Result<bool, Stop> {
        let status = self
            .vre
            .export_status(server, iri, id)
            .await
            .map_err(|e| client_failure("asking for the occupying export", &e))?;
        Ok(status.is_some())
    }

    /// Waits for the export to complete, then downloads it into `work`.
    async fn download(
        &self,
        server: &ServerId,
        iri: &str,
        id: &str,
        work: &TempDir,
    ) -> Result<vre_bridge_client::DumpZip, Stop> {
        match self.wait_finished(server, iri, id).await? {
            ExportStatus::Completed => {}
            ExportStatus::Failed | ExportStatus::InProgress => {
                error!(export = %id, "the export failed on the server");
                return Err(Outcome::Failed(FailureReason::Export).into());
            }
        }
        self.vre
            .download(server, iri, id, work, self.settings.max_dump_bytes)
            .await
            .map_err(|e| client_failure("the dump download", &e))
    }

    /// [`poll_finished`](Self::poll_finished), its failure logged and turned into the outcome.
    async fn wait_finished(&self, server: &ServerId, iri: &str, id: &str) -> Result<ExportStatus, Stop> {
        self.poll_finished(server, iri, id).await.map_err(|waited| match waited {
            Waited::TimedOut => {
                let limit_secs = self.settings.export_timeout.as_secs_f64();
                error!(export = %id, limit_secs, "the export did not end in time");
                Outcome::Failed(FailureReason::Timeout).into()
            }
            Waited::Client(e) => client_failure("polling the export", &e),
        })
    }

    /// Polls every `EDITOR_VRE_BRIDGE_EXPORT_POLL_SECS` until the export is no longer in progress,
    /// for at most `EDITOR_VRE_BRIDGE_EXPORT_TIMEOUT_SECS`; a failed poll is repeated up to
    /// [`STATUS_POLL_ATTEMPTS`] times in a row. Logs only the repeated polls.
    async fn poll_finished(&self, server: &ServerId, iri: &str, id: &str) -> Result<ExportStatus, Waited> {
        let poll = async {
            let mut failures = 0;
            loop {
                match self.vre.export_status(server, iri, id).await {
                    Ok(Some(ExportStatus::InProgress)) => failures = 0,
                    Ok(Some(status)) => return Ok(status),
                    Ok(None) => return Err(Waited::Client(ClientError::ExportGone)),
                    Err(e) if is_transient(&e) && failures + 1 < STATUS_POLL_ATTEMPTS => {
                        failures += 1;
                        warn!(export = %id, error = %e, "polling the export failed; polling again");
                    }
                    Err(e) => return Err(Waited::Client(e)),
                }
                tokio::time::sleep(self.settings.export_poll).await;
            }
        };
        tokio::time::timeout(self.settings.export_timeout, poll)
            .await
            .unwrap_or(Err(Waited::TimedOut))
    }

    /// Deletes the export, so the server's dump slot is free again. A running export cannot be
    /// deleted: it is waited for once more. An export left behind is logged with its id; the next
    /// refresh's trigger finds it as its own and deletes it. The outcome is not changed.
    async fn delete_export(&self, server: &ServerId, iri: &str, id: &str) {
        match self.vre.delete_export(server, iri, id).await {
            Ok(DeleteOutcome::Deleted | DeleteOutcome::Gone) => return,
            Ok(DeleteOutcome::StillRunning) => {}
            Err(e) => return left_behind(id, &e),
        }
        warn!(export = %id, "the export is still running; deleting it once it ends");
        match self.poll_finished(server, iri, id).await {
            Ok(_) => {}
            Err(Waited::Client(ClientError::ExportGone)) => return,
            Err(Waited::Client(e)) => return left_behind(id, &e),
            Err(Waited::TimedOut) => return left_behind(id, &"it did not end in time"),
        }
        match self.vre.delete_export(server, iri, id).await {
            Ok(DeleteOutcome::Deleted | DeleteOutcome::Gone) => {}
            Ok(DeleteOutcome::StillRunning) => left_behind(id, &"it is still running"),
            Err(e) => left_behind(id, &e),
        }
    }

    /// Lifts the unpacked dump and renders the snapshot. A refusal is logged without a value:
    /// each guard violation names an IRI, a class and a reason, a closed-world refusal only its
    /// rules and counts.
    async fn lift(&self, dump_dir: PathBuf) -> Result<Vec<u8>, Stop> {
        let lift = self.lift;
        let lifted = spawn_blocking(move || {
            let lifted = lift(&dump_dir)?;
            let mut bytes = Vec::new();
            lifted
                .snapshot
                .write_to(&mut bytes)
                .map_err(|_| LiftError::Internal { what: "rendering the snapshot", subject: String::new() })?;
            Ok::<_, LiftError>(bytes)
        })
        .await;
        match lifted {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(LiftError::Refused { violations })) => {
                log_violations(&violations);
                Err(Outcome::Refused(RefusalReason::PublicGuard).into())
            }
            Ok(Err(LiftError::ClosedWorld(report))) => {
                let counts = report.counts();
                for (rule, count) in &counts {
                    warn!(rule = %rule, count, "the dump breaks the interim format");
                }
                let total: usize = counts.iter().map(|(_, count)| count).sum();
                warn!(total, "the interim format's closed world refused the project");
                Err(Outcome::Refused(RefusalReason::ClosedWorld).into())
            }
            Ok(Err(e @ LiftError::Dump(_))) => {
                error!(error = %e, "the dump cannot be lifted");
                Err(Outcome::Failed(FailureReason::Dump).into())
            }
            Ok(Err(e @ LiftError::Internal { .. })) => {
                error!(error = %e, "the lift failed");
                Err(Outcome::Failed(FailureReason::Internal).into())
            }
            Err(join) => Err(joined("the lift", &join, FailureReason::Internal)),
        }
    }

    /// Runs `f` on a blocking thread; its error, or a panic, ends the refresh as `failure`. The
    /// error is logged as it is, so it must hold no value of the dump (`LiftError` may: see
    /// `lift`).
    async fn blocking<T, E>(
        &self,
        what: &'static str,
        failure: FailureReason,
        f: impl FnOnce() -> Result<T, E> + Send + 'static,
    ) -> Result<T, Stop>
    where
        T: Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        match spawn_blocking(f).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => {
                error!(error = %e, "{what} failed");
                Err(Outcome::Failed(failure).into())
            }
            Err(join) => Err(joined(what, &join, FailureReason::Internal)),
        }
    }
}

/// What a new token records beyond its generation.
struct Token {
    as_of: String,
    server: String,
    running: Running,
    previous: Option<Generation>,
}

/// Writes the new generation into both subtrees and makes it current: shipped inputs and the
/// rights record into `media`, the snapshot into `sync`.
fn write_generation(
    subtrees: &Subtrees,
    shortcode: &str,
    t: DateTime<Utc>,
    snapshot: &[u8],
    inputs: &Path,
    token: Token,
) -> Result<vre_bridge_store::Sizes, StoreError> {
    let mut attempt = 1;
    let mut writer = loop {
        match subtrees.begin(shortcode, new_generation_name(&t)?) {
            Err(StoreError::GenerationExists(_)) if attempt < GENERATION_ATTEMPTS => attempt += 1,
            begun => break begun?,
        }
    };
    if inputs.is_dir() {
        writer.copy_tree(Subtree::Media, inputs)?;
    }
    writer.write_rights_record()?;
    writer.write(Subtree::Sync, SNAPSHOT_FILE, snapshot)?;
    let sizes = writer.sizes();
    let ready = ReadyToken {
        provenance: Provenance {
            generation: writer.generation().clone(),
            as_of: token.as_of,
            server: token.server,
            interim_format_version: token.running.interim_format_version,
            transformation_version: token.running.transformation_version,
            inputs_digest: token.running.inputs_digest,
        },
        previous_generation: token.previous,
    };
    writer.commit(&ready)?;
    Ok(sizes)
}

/// Deletes every leftover private work directory directly below `work_dir`, as a killed process
/// leaves them; the caller holds the [`Lock`], so no running refresh's directory goes.
///
/// # Errors
///
/// As `vre_bridge_store::sweep_work_dir`.
pub fn sweep_dump_dirs(work_dir: &Path) -> vre_bridge_store::Result<usize> {
    vre_bridge_store::sweep_work_dir(work_dir, DUMP_DIR_PREFIX)
}

/// Deletes the private work directory with the dump in it, on a blocking thread.
async fn remove_work_dir(work: Arc<TempDir>) {
    let removed = spawn_blocking(move || match Arc::try_unwrap(work) {
        Ok(dir) => dir.close(),
        // Not reached: every blocking task that held a clone has ended. The last holder's drop
        // deletes it.
        Err(_) => Ok(()),
    })
    .await;
    match removed {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!(error = %e, "deleting the private work directory failed"),
        Err(_) => error!("deleting the private work directory panicked"),
    }
}

fn valid(state: TokenState) -> Option<ReadyToken> {
    match state {
        TokenState::Valid(token) => Some(token),
        TokenState::Missing | TokenState::Broken(_) => None,
    }
}

fn log_violations(violations: &[Violation]) {
    for violation in violations.iter().take(LOGGED_VIOLATIONS) {
        warn!(violation = %violation, "the public guard refuses an item");
    }
    warn!(total = violations.len(), "the public guard refused the project");
}

/// A panic's payload is not logged: it may quote the data being handled.
fn joined(what: &str, join: &JoinError, failure: FailureReason) -> Stop {
    if join.is_panic() {
        error!("{what} panicked");
    } else {
        error!("{what} was cancelled");
    }
    Outcome::Failed(failure).into()
}

/// A failed call's outcome, logged with the step. `ClientError` carries no secret and no data.
fn client_failure(step: &'static str, e: &ClientError) -> Stop {
    let outcome = client_outcome(e);
    match outcome {
        Outcome::Refused(_) => warn!(error = %e, "{step} was refused"),
        _ => error!(error = %e, "{step} failed"),
    }
    outcome.into()
}

fn client_outcome(e: &ClientError) -> Outcome {
    use FailureReason as F;
    match e {
        ClientError::Status { status: 401 | 403, .. } => Outcome::Refused(RefusalReason::Unauthorized),
        ClientError::Request { source, .. } if source.is_timeout() => Outcome::Failed(F::Timeout),
        ClientError::Request { .. } | ClientError::ShortBody { .. } => Outcome::Failed(F::Unreachable),
        ClientError::TimedOut { .. } => Outcome::Failed(F::Timeout),
        ClientError::Status { .. } | ClientError::Body { .. } | ClientError::ExportGone => Outcome::Failed(F::Server),
        ClientError::ExportFailed => Outcome::Failed(F::Export),
        ClientError::Io { .. }
        | ClientError::Zip(_)
        | ClientError::UnsafeEntry { .. }
        | ClientError::DuplicateEntry
        | ClientError::MissingData
        | ClientError::TooLarge { .. } => Outcome::Failed(F::Dump),
        ClientError::Identity(_)
        | ClientError::BaseUrl
        | ClientError::NoCryptoProvider
        | ClientError::NoServer
        | ClientError::WorkDirMode => Outcome::Failed(F::Internal),
    }
}

/// A poll worth repeating: the transport, the server's own error, or a rate limit (`429`).
fn is_transient(e: &ClientError) -> bool {
    match e {
        ClientError::Request { .. } => true,
        ClientError::Status { status, .. } => *status >= 500 || *status == 429,
        _ => false,
    }
}

/// `tokio::task::spawn_blocking` inside the caller's span, so what the store logs there names the
/// project and server.
fn spawn_blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> tokio::task::JoinHandle<T> {
    let span = tracing::Span::current();
    tokio::task::spawn_blocking(move || span.in_scope(f))
}
