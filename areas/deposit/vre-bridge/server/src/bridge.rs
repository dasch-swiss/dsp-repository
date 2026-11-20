//! [`VreBridge`]: what `deposit-server` builds from its configuration, starts once, and calls for
//! the operator's refresh.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::{fmt, io};

use chrono::{DateTime, TimeDelta, Utc};
use chrono_tz::Tz;
use opentelemetry::metrics::Meter;
use tokio::time::Instant;
use tracing::{error, info, warn};
use vre_bridge_client::{Server, ServiceIdentity, Timeouts, DUMP_DIR_PREFIX};
use vre_bridge_core::{AllowlistEntry, FailureReason, Outcome, RawSettings, ServerId, Settings, FILE_MAPPING};
use vre_bridge_store::{is_swept, Lock, RecoveryReport, Subtrees};

use crate::refresh::spawn_blocking;
use crate::report::{observe_active, OtelReport};
use crate::withhold::withhold;
use crate::{schedule, sweep_dump_dirs, Busy, Clock, RefreshReport, Refresher, Report, SystemClock, Vre};

/// The operator's refresh's exit status when the bridge is not configured or the project is not
/// on the allowlist (`EX_USAGE`).
pub const EXIT_UNUSABLE: u8 = 64;

/// The operator's refresh's exit status when the lock is held, by a nightly run or another
/// refresh (`EX_TEMPFAIL`): try again once it has ended.
pub const EXIT_LOCKED: u8 = 75;

/// Why the bridge does nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inactive {
    /// Neither output location is set.
    NotConfigured,
    /// A configuration error, naming the variable; it never holds a signing key.
    Invalid(String),
}

impl fmt::Display for Inactive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => f.write_str("no output location is configured"),
            Self::Invalid(reason) => f.write_str(reason),
        }
    }
}

/// The VRE bridge of one `deposit-server` process: active with checked settings, or inactive with
/// the reason. It never stops its process: an inactive bridge does nothing, and an active one
/// logs what fails.
pub struct VreBridge(State);

enum State {
    Active(Arc<Active>),
    Inactive(Inactive),
}

/// An active bridge's parts, shared by the start-time work, the scheduler, each nightly run and
/// the operator's refresh.
pub(crate) struct Active {
    pub(crate) refresher: Refresher,
    pub(crate) report: Arc<dyn Report>,
    pub(crate) settings: Arc<Settings>,
    pub(crate) subtrees: Subtrees,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) time_zone: Tz,
}

impl VreBridge {
    /// The bridge `raw` configures, inactive when no output location is set or a setting is
    /// invalid; the reason is logged. Each server's client is built here, so a rustls crypto
    /// provider must be installed first, and each signing key signs a probe token here. It reads
    /// the file system to check the paths, so it blocks: on a runtime, call it from
    /// `spawn_blocking`.
    ///
    /// Its instruments are made on `meter`, scope [`METER_SCOPE`](crate::METER_SCOPE): from the
    /// process's meter provider once that is installed, since instruments made before stay no-ops,
    /// or from a provider with no reader where nothing may be exported.
    #[must_use]
    pub fn from_raw(raw: RawSettings, meter: &Meter) -> Self {
        let state = match Settings::parse(raw) {
            Ok(None) => State::Inactive(Inactive::NotConfigured),
            Err(e) => State::Inactive(Inactive::Invalid(e.to_string())),
            Ok(Some(mut settings)) => {
                let checked = check(&mut settings).and_then(|time_zone| Ok((time_zone, servers(&settings)?)));
                match checked {
                    Ok((time_zone, servers)) => State::Active(Arc::new(Active::new(
                        settings,
                        time_zone,
                        Arc::new(servers),
                        Arc::new(OtelReport::new(meter)),
                        Arc::new(SystemClock),
                    ))),
                    Err(why) => State::Inactive(why),
                }
            }
        };
        match &state {
            State::Active(_) => info!("the VRE bridge is active"),
            State::Inactive(why @ Inactive::NotConfigured) => info!(reason = %why, "the VRE bridge is inactive"),
            State::Inactive(why @ Inactive::Invalid(_)) => {
                error!(reason = %why, "the VRE bridge is inactive: its configuration is invalid");
            }
        }
        observe_active(meter, matches!(state, State::Active(_)));
        Self(state)
    }

    /// An active bridge over the given parts, after the same checks as [`from_raw`](Self::from_raw)
    /// but the servers'.
    #[cfg(test)]
    pub(crate) fn with_parts(
        mut settings: Settings,
        vre: Arc<dyn Vre>,
        report: Arc<dyn Report>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self(match check(&mut settings) {
            Ok(time_zone) => State::Active(Arc::new(Active::new(settings, time_zone, vre, report, clock))),
            Err(why) => State::Inactive(why),
        })
    }

    #[cfg(test)]
    pub(crate) fn active(&self) -> &Arc<Active> {
        match &self.0 {
            State::Active(active) => active,
            State::Inactive(why) => panic!("the bridge is inactive: {why}"),
        }
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.0, State::Active(_))
    }

    /// Why the bridge does nothing; `None` when it is active.
    #[must_use]
    pub fn inactive_reason(&self) -> Option<&Inactive> {
        match &self.0 {
            State::Active(_) => None,
            State::Inactive(why) => Some(why),
        }
    }

    /// Starts an active bridge in the background and returns at once; an inactive one spawns
    /// nothing. Under the lock, if no one else holds it, the leftover dump directories are swept
    /// and the allowlisted projects recovered; then the nightly runs are scheduled. Called once,
    /// within a tokio runtime; without one it logs an error and does nothing.
    pub fn start(&self) {
        let State::Active(active) = &self.0 else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            error!("the VRE bridge is started outside a tokio runtime; it does nothing");
            return;
        };
        let active = Arc::clone(active);
        runtime.spawn(withhold(async move {
            active.start_work().await;
            schedule::run(active).await;
        }));
    }

    /// The operator's forced refresh of one allowlisted project; the exit status is the outcome's
    /// (`0` unchanged or refreshed, `2` refused, `1` failed), [`EXIT_UNUSABLE`] when the bridge is
    /// inactive or the project not allowlisted, [`EXIT_LOCKED`] when the lock is held. It never
    /// waits for the lock, and a busy dump slot fails it at once.
    pub async fn operator_refresh(&self, shortcode: &str) -> ExitCode {
        let active = match &self.0 {
            State::Active(active) => Arc::clone(active),
            State::Inactive(why) => {
                error!(reason = %why, "the VRE bridge is inactive; nothing is refreshed");
                return ExitCode::from(EXIT_UNUSABLE);
            }
        };
        let Some(entry) = active.entry(shortcode) else {
            error!(project = ?shortcode, "the project is not on the allowlist; nothing is refreshed");
            return ExitCode::from(EXIT_UNUSABLE);
        };
        let path = active.settings.lock_file.clone();
        let lock = match spawn_blocking(move || Lock::try_acquire(&path)).await {
            Ok(Ok(Some(lock))) => lock,
            Ok(Ok(None)) => {
                warn!(
                    project = %entry.shortcode,
                    "the lock is held by a nightly run or another refresh; try again once it has ended"
                );
                return ExitCode::from(EXIT_LOCKED);
            }
            Ok(Err(e)) => {
                error!(error = %e, "the lock cannot be taken; nothing is refreshed");
                return ExitCode::from(Outcome::Failed(FailureReason::Store).exit_code());
            }
            Err(_) => {
                error!("taking the lock panicked; nothing is refreshed");
                return ExitCode::from(Outcome::Failed(FailureReason::Internal).exit_code());
            }
        };
        let started = Instant::now();
        let outcome = match active.refresh_guarded(&Arc::new(lock), &entry, true, started).await {
            Ok(outcome) => outcome,
            Err(Busy) => active.give_up(&entry, started),
        };
        ExitCode::from(outcome.exit_code())
    }
}

impl Active {
    /// Every refresh goes through here: in its own task, awaited and never aborted, since
    /// dropping it would release the lock while a blocking write goes on. A refresh that does not
    /// finish is reported as `Failed(Internal)`.
    ///
    /// # Errors
    ///
    /// [`Busy`] as [`Refresher::refresh`]; the caller retries or gives up.
    pub(crate) async fn refresh_guarded(
        self: &Arc<Self>,
        lock: &Arc<Lock>,
        entry: &AllowlistEntry,
        operator: bool,
        started: Instant,
    ) -> Result<Outcome, Busy> {
        let refresh = tokio::spawn({
            let (active, lock, entry) = (Arc::clone(self), Arc::clone(lock), entry.clone());
            withhold(async move { active.refresher.refresh(&lock, &entry, operator).await })
        });
        match refresh.await {
            Ok(result) => result,
            Err(join) => {
                error!(project = %entry.shortcode, panicked = join.is_panic(), "the refresh did not finish");
                let outcome = Outcome::Failed(FailureReason::Internal);
                self.report_unrefreshed(entry, outcome, started);
                Ok(outcome)
            }
        }
    }
}

impl Active {
    fn new(
        settings: Settings,
        time_zone: Tz,
        vre: Arc<dyn Vre>,
        report: Arc<dyn Report>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let settings = Arc::new(settings);
        let subtrees = Subtrees::new(settings.sync_dir.clone(), settings.media_dir.clone());
        Self {
            refresher: Refresher::new(
                vre,
                Arc::clone(&report),
                subtrees.clone(),
                Arc::clone(&settings),
                Arc::clone(&clock),
            ),
            report,
            settings,
            subtrees,
            clock,
            time_zone,
        }
    }

    fn entry(&self, shortcode: &str) -> Option<AllowlistEntry> {
        self.settings
            .allowlist
            .entries()
            .iter()
            .find(|entry| entry.shortcode == shortcode)
            .cloned()
    }

    pub(crate) fn shortcodes(&self) -> Vec<String> {
        self.settings
            .allowlist
            .entries()
            .iter()
            .map(|entry| entry.shortcode.clone())
            .collect()
    }

    /// Reports a project whose dump slot stayed busy as failed, and returns that outcome.
    pub(crate) fn give_up(&self, entry: &AllowlistEntry, started: Instant) -> Outcome {
        let outcome = Outcome::Failed(FailureReason::Busy);
        self.report_unrefreshed(entry, outcome, started);
        outcome
    }

    fn report_unrefreshed(&self, entry: &AllowlistEntry, outcome: Outcome, started: Instant) {
        self.report.refresh(&RefreshReport {
            project: entry.shortcode.clone(),
            server: Some(entry.server.clone()),
            outcome,
            duration: started.elapsed(),
            sizes: None,
        });
    }

    /// Returns once the clock reads `t` or later.
    pub(crate) async fn sleep_until(&self, t: DateTime<Utc>) {
        while self.clock.now() < t {
            self.clock.sleep_until(t).await;
        }
    }

    /// `d` from now; `None` past the clock's range.
    pub(crate) fn later_by(&self, d: std::time::Duration) -> Option<DateTime<Utc>> {
        self.clock.now().checked_add_signed(TimeDelta::from_std(d).ok()?)
    }

    /// The start-time work: under the lock, if no one holds it, the sweep of leftover dump
    /// directories and the recovery of every allowlisted project.
    async fn start_work(&self) {
        let path = self.settings.lock_file.clone();
        let work_dir = self.settings.work_dir.clone();
        let subtrees = self.subtrees.clone();
        let shortcodes = self.shortcodes();
        let done = spawn_blocking(move || {
            let Some(_lock) = Lock::try_acquire(&path)? else {
                return Ok(None);
            };
            sweep_and_log(&work_dir);
            Ok::<_, vre_bridge_store::StoreError>(Some(subtrees.recover(&shortcodes)))
        })
        .await;
        match done {
            Ok(Ok(Some(recovered))) => log_recovery(&recovered),
            Ok(Ok(None)) => warn!("the lock is held elsewhere; the start-time sweep and recovery are skipped"),
            Ok(Err(e)) => error!(error = %e, "the lock cannot be taken; the start-time sweep and recovery are skipped"),
            Err(join) => error!(panicked = join.is_panic(), "the start-time sweep and recovery did not finish"),
        }
    }
}

/// Deletes the leftover dump directories in `work_dir`; the caller holds the lock. Blocking.
pub(crate) fn sweep_and_log(work_dir: &Path) {
    match sweep_dump_dirs(work_dir) {
        Ok(0) => {}
        Ok(swept) => warn!(swept, "leftover dump directories deleted from the work directory"),
        Err(e) => error!(error = %e, "sweeping the work directory failed"),
    }
}

pub(crate) fn log_recovery(report: &RecoveryReport) {
    if report.deleted_generations > 0 {
        info!(
            deleted_generations = report.deleted_generations,
            "recovery deleted generations no token names"
        );
    }
    for (project, e) in &report.failed {
        error!(project = %project, error = %e, "recovering the project failed");
    }
}

const SYNC_DIR: &str = "EDITOR_VRE_BRIDGE_SYNC_DIR";
const MEDIA_DIR: &str = "EDITOR_VRE_BRIDGE_MEDIA_DIR";
const WORK_DIR: &str = "EDITOR_VRE_BRIDGE_WORK_DIR";
const LOCK_FILE: &str = "EDITOR_VRE_BRIDGE_LOCK_FILE";
const TIME_ZONE: &str = "EDITOR_VRE_BRIDGE_TIME_ZONE";
const SIGNING_KEYS: &str = "EDITOR_VRE_BRIDGE_SIGNING_KEYS";

fn invalid(reason: String) -> Inactive {
    Inactive::Invalid(reason)
}

/// The checks `Settings::parse` leaves to the bridge: the time zone, which it returns; an IIIF
/// base for each server from which a project ships a file mapping; and the work directory and lock
/// file outside both output locations, neither a symlink, and no output location where the sweep
/// of the work directory would delete it. Paths are compared after `canonicalize`, so both output
/// locations and the work directory must exist; the settings keep the canonical work directory and
/// lock file, so a symlink placed later changes neither.
fn check(settings: &mut Settings) -> Result<Tz, Inactive> {
    let time_zone = settings
        .time_zone
        .parse::<Tz>()
        .map_err(|_| invalid(format!("{TIME_ZONE} is not an IANA time zone name")))?;
    let inputs = settings.inputs_dir.clone();
    settings
        .check_iiif_bases(|shortcode| std::fs::symlink_metadata(inputs.join(shortcode).join(FILE_MAPPING)).is_ok())
        .map_err(|e| invalid(e.to_string()))?;

    let resolve = |var: &str, path: &Path| {
        path.canonicalize()
            .map_err(|e| invalid(format!("{var} {} cannot be resolved: {e}", path.display())))
    };
    let roots = [
        (SYNC_DIR, resolve(SYNC_DIR, &settings.sync_dir)?),
        (MEDIA_DIR, resolve(MEDIA_DIR, &settings.media_dir)?),
    ];

    for (var, path) in [(WORK_DIR, &settings.work_dir), (LOCK_FILE, &settings.lock_file)] {
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(invalid(format!("{var} {} is a symlink", path.display())));
        }
    }
    let work_dir = resolve(WORK_DIR, &settings.work_dir)?;
    let lock_file = resolve_file(&settings.lock_file)
        .map_err(|e| invalid(format!("{LOCK_FILE} {} cannot be resolved: {e}", settings.lock_file.display())))?;

    for (var, path) in [(WORK_DIR, &work_dir), (LOCK_FILE, &lock_file)] {
        if let Some((root_var, _)) = roots.iter().find(|(_, root)| path.starts_with(root)) {
            return Err(invalid(format!("{var} is at or below the output location {root_var}")));
        }
    }
    let kept = roots.iter().map(|(var, path)| (*var, path)).chain([(LOCK_FILE, &lock_file)]);
    for (var, path) in kept {
        if is_swept(&work_dir, DUMP_DIR_PREFIX, path) {
            return Err(invalid(format!(
                "{var} lies in a {WORK_DIR} entry named {DUMP_DIR_PREFIX}*, which the sweep deletes"
            )));
        }
    }
    settings.work_dir = work_dir;
    settings.lock_file = lock_file;
    Ok(time_zone)
}

/// `path` canonicalized; a file that does not exist yet by its parent directory.
fn resolve_file(path: &Path) -> io::Result<PathBuf> {
    match path.canonicalize() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let name = path
                .file_name()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
            let parent = match path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent,
                _ => Path::new("."),
            };
            Ok(parent.canonicalize()?.join(name))
        }
        resolved => resolved,
    }
}

/// A client per server the allowlist uses. Each signing key signs a probe token here, so a key
/// that cannot sign makes the bridge inactive at start.
fn servers(settings: &Settings) -> Result<BTreeMap<ServerId, Server>, Inactive> {
    let timeouts = Timeouts {
        connect: Timeouts::default().connect.min(settings.request_timeout),
        read: settings.request_timeout,
        request: settings.request_timeout,
        download: settings.download_timeout,
        file: settings.file_timeout,
    };
    settings
        .servers
        .iter()
        .map(|(id, server)| {
            let identity = ServiceIdentity::new(&settings.service_name, &server.key_id, server.signing_key.expose())
                .map_err(|e| invalid(format!("{SIGNING_KEYS}: the key of server `{id}`: {e}")))?;
            let client_error = |e| invalid(format!("the client of server `{id}`: {e}"));
            let mut client = Server::new(&server.base_url, identity, timeouts).map_err(client_error)?;
            if let Some(iiif_base) = &server.iiif_base {
                client = client.with_file_base(iiif_base).map_err(client_error)?;
            }
            Ok((id.clone(), client))
        })
        .collect()
}
