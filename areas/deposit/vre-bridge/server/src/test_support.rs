//! What the bridge's tests share: the report they record, the captured logs, temporary
//! directories and settings, and a fake `Vre` for the tests that need no wiremock.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use shared_stand_in::{Generation, Provenance, ReadyToken, TOKEN_FILE};
use tempfile::TempDir;
use vre_bridge_client::{Changes, ClientError, DeleteOutcome, DumpZip, ExportStatus, ExportTrigger};
use vre_bridge_core::{inputs_digest, RawSettings, ServerId, Settings};
use vre_bridge_lift::{INTERIM_FORMAT_VERSION, TRANSFORMATION_VERSION};

use crate::clock::FakeClock;
use crate::{Clock, LogReport, RefreshReport, Report, Vre, VreBridge};

pub(crate) const KEY: &str = include_str!("../../client/testdata/ed25519-private.pem");

/// The `asOf` every change check of [`FakeVre`] answers.
pub(crate) const FAKE_AS_OF: &str = "2026-11-20T01:00:05Z";

// ---------------------------------------------------------------- the report

/// Records every report and writes its outcome line, as `LogReport` does.
#[derive(Default)]
pub(crate) struct Recording(Mutex<Vec<RefreshReport>>);

impl Report for Recording {
    fn refresh(&self, report: &RefreshReport) {
        LogReport.refresh(report);
        self.0.lock().unwrap().push(report.clone());
    }
}

impl Recording {
    pub(crate) fn reports(&self) -> Vec<RefreshReport> {
        self.0.lock().unwrap().clone()
    }

    /// Each report's project and outcome word, in order.
    pub(crate) fn outcomes(&self) -> Vec<(String, &'static str)> {
        self.reports().iter().map(|r| (r.project.clone(), r.outcome.as_str())).collect()
    }
}

// ---------------------------------------------------------------- logs

/// Everything any test logs, from every thread: the subscriber is the process's global one.
pub(crate) fn logs() -> Arc<Mutex<Vec<u8>>> {
    static LOGS: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
    LOGS.get_or_init(|| {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = {
            let buffer = Arc::clone(&buffer);
            move || Captured(Arc::clone(&buffer))
        };
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        buffer
    })
    .clone()
}

struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The captured lines that name `project` in their span or fields.
pub(crate) fn lines_of(project: &str) -> Vec<String> {
    let text = String::from_utf8(logs().lock().unwrap().clone()).unwrap();
    let marker = format!("project={project}");
    text.lines().filter(|line| line.contains(&marker)).map(str::to_owned).collect()
}

// ---------------------------------------------------------------- directories and settings

pub(crate) fn temp() -> TempDir {
    tempfile::tempdir().unwrap()
}

pub(crate) fn display(path: &Path) -> String {
    path.display().to_string()
}

/// The subtrees, inputs, work directory and lock directory of one test; the lock directory also
/// holds the test key's file.
pub(crate) struct Dirs {
    pub(crate) sync: TempDir,
    pub(crate) media: TempDir,
    pub(crate) inputs: TempDir,
    pub(crate) work: TempDir,
    pub(crate) lock_dir: TempDir,
}

impl Dirs {
    pub(crate) fn new() -> Self {
        logs();
        let dirs = Self {
            sync: temp(),
            media: temp(),
            inputs: temp(),
            work: temp(),
            lock_dir: temp(),
        };
        std::fs::write(dirs.key_file(), KEY).unwrap();
        dirs
    }

    pub(crate) fn lock_file(&self) -> PathBuf {
        self.lock_dir.path().join("lock")
    }

    pub(crate) fn key_file(&self) -> PathBuf {
        self.lock_dir.path().join("signing-key.pem")
    }

    /// Settings over these directories; `allowlist` as `SC=server,…`, each server configured
    /// with the test key's file. Creates each allowlisted project's inputs directory, empty unless
    /// the test fills it.
    pub(crate) fn raw(&self, allowlist: &str) -> RawSettings {
        let entries = allowlist.split(',').filter_map(|entry| entry.split_once('='));
        for (shortcode, _) in entries.clone() {
            std::fs::create_dir_all(self.inputs.path().join(shortcode)).unwrap();
        }
        let servers: std::collections::BTreeSet<&str> = entries.map(|(_, server)| server).collect();
        let list = |f: &dyn Fn(&str) -> String| servers.iter().map(|id| f(id)).collect::<Vec<_>>().join(",");
        RawSettings {
            sync_dir: Some(display(self.sync.path())),
            media_dir: Some(display(self.media.path())),
            lock_file: Some(display(&self.lock_file())),
            allowlist: Some(allowlist.to_owned()),
            servers: Some(list(&|id| format!("{id}=https://{id}.example.test"))),
            key_ids: Some(list(&|id| format!("{id}=test-kid"))),
            signing_key_files: Some(list(&|id| format!("{id}={}", display(&self.key_file()))).into()),
            service_name: Some("dsp-repository-test".to_owned()),
            inputs_dir: Some(display(self.inputs.path())),
            work_dir: Some(display(self.work.path())),
            ..RawSettings::default()
        }
    }

    pub(crate) fn settings(&self, allowlist: &str) -> Settings {
        Settings::parse(self.raw(allowlist)).unwrap().unwrap()
    }

    /// Writes a token naming `generation`, and that generation's directory in both subtrees,
    /// into the subtree at `root`.
    pub(crate) fn seed(&self, root: &Path, shortcode: &str, generation: &str) {
        for subtree in [self.sync.path(), self.media.path()] {
            std::fs::create_dir_all(subtree.join(shortcode).join(generation)).unwrap();
        }
        let token = ReadyToken {
            provenance: Provenance {
                generation: Generation::parse(generation).unwrap(),
                as_of: "2026-11-19T01:00:00Z".to_owned(),
                server: "prod".to_owned(),
                interim_format_version: INTERIM_FORMAT_VERSION,
                transformation_version: TRANSFORMATION_VERSION,
                inputs_digest: inputs_digest(&self.inputs.path().join(shortcode)).unwrap(),
            },
            previous_generation: None,
        };
        std::fs::write(root.join(shortcode).join(TOKEN_FILE), token.to_bytes()).unwrap();
    }

    /// The project's token files' bytes, `sync`'s first.
    pub(crate) fn token_bytes(&self, shortcode: &str) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let read = |root: &Path| std::fs::read(root.join(shortcode).join(TOKEN_FILE)).ok();
        (read(self.sync.path()), read(self.media.path()))
    }

    /// An active bridge over these directories, `vre`, `report` and `clock`.
    pub(crate) fn bridge(
        &self,
        settings: Settings,
        vre: Arc<dyn Vre>,
        report: Arc<dyn Report>,
        clock: Arc<dyn Clock>,
    ) -> VreBridge {
        let bridge = VreBridge::with_parts(settings, vre, report, clock);
        assert!(bridge.is_active(), "{:?}", bridge.inactive_reason());
        bridge
    }
}

// ---------------------------------------------------------------- a fake VRE

const IRI_PREFIX: &str = "http://rdfh.ch/projects/";

/// What [`FakeVre`] does for one project.
pub(crate) enum Plays {
    /// The project lookup fails.
    Fails,
    /// The lookup panics.
    Panics,
    /// The lookup runs the check, then fails.
    Checks(Box<dyn Fn() + Send + Sync>),
    /// Another caller's export holds the dump slot.
    Busy,
    /// The export completes with this dump zip.
    Dump(Vec<u8>),
}

/// One call to [`FakeVre`]: what was called, for which project, at which clock time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) what: &'static str,
    pub(crate) project: String,
    pub(crate) at: DateTime<Utc>,
}

/// A VRE server per project, played from a script; a project without one fails its lookup.
pub(crate) struct FakeVre {
    clock: Arc<FakeClock>,
    plays: BTreeMap<String, Plays>,
    lookup_takes: BTreeMap<String, TimeDelta>,
    calls: Mutex<Vec<Call>>,
}

impl FakeVre {
    pub(crate) fn new(clock: Arc<FakeClock>) -> Self {
        Self {
            clock,
            plays: BTreeMap::new(),
            lookup_takes: BTreeMap::new(),
            calls: Mutex::default(),
        }
    }

    pub(crate) fn plays(mut self, shortcode: &str, plays: Plays) -> Self {
        self.plays.insert(shortcode.to_owned(), plays);
        self
    }

    /// Each lookup of the project moves the clock forward by `by`.
    pub(crate) fn lookup_takes(mut self, shortcode: &str, by: TimeDelta) -> Self {
        self.lookup_takes.insert(shortcode.to_owned(), by);
        self
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    /// The calls of one kind, as (project, clock time).
    pub(crate) fn calls_of(&self, what: &str) -> Vec<(String, DateTime<Utc>)> {
        self.calls()
            .into_iter()
            .filter(|call| call.what == what)
            .map(|call| (call.project, call.at))
            .collect()
    }

    fn record(&self, what: &'static str, project: &str) {
        let at = self.clock.now();
        self.calls.lock().unwrap().push(Call { what, project: project.to_owned(), at });
    }
}

fn shortcode(iri: &str) -> &str {
    iri.strip_prefix(IRI_PREFIX).unwrap()
}

fn unavailable(endpoint: &'static str) -> ClientError {
    ClientError::Status { endpoint, status: 503 }
}

#[async_trait]
impl Vre for FakeVre {
    async fn project_iri(&self, _server: &ServerId, sc: &str) -> Result<String, ClientError> {
        self.record("lookup", sc);
        if let Some(by) = self.lookup_takes.get(sc) {
            self.clock.advance(*by);
        }
        match self.plays.get(sc) {
            Some(Plays::Panics) => panic!("the fake VRE panics for {sc}"),
            Some(Plays::Checks(check)) => {
                check();
                Err(unavailable("lookup"))
            }
            Some(Plays::Busy | Plays::Dump(_)) => Ok(format!("{IRI_PREFIX}{sc}")),
            Some(Plays::Fails) | None => Err(unavailable("lookup")),
        }
    }

    async fn changes(&self, _server: &ServerId, iri: &str, _cutoff: DateTime<Utc>) -> Result<Changes, ClientError> {
        self.record("changes", shortcode(iri));
        Ok(Changes { as_of: FAKE_AS_OF.to_owned(), count: 1 })
    }

    async fn export(&self, _server: &ServerId, iri: &str) -> Result<ExportTrigger, ClientError> {
        let sc = shortcode(iri);
        self.record("export", sc);
        match self.plays.get(sc) {
            Some(Plays::Busy) => Ok(ExportTrigger::Busy),
            _ => Ok(ExportTrigger::Started { id: sc.to_owned() }),
        }
    }

    async fn export_status(
        &self,
        _server: &ServerId,
        _iri: &str,
        _id: &str,
    ) -> Result<Option<ExportStatus>, ClientError> {
        Ok(Some(ExportStatus::Completed))
    }

    async fn download(
        &self,
        _server: &ServerId,
        iri: &str,
        _id: &str,
        work: &TempDir,
        _max_bytes: u64,
    ) -> Result<DumpZip, ClientError> {
        let sc = shortcode(iri);
        self.record("download", sc);
        let Some(Plays::Dump(zip)) = self.plays.get(sc) else {
            return Err(unavailable("download"));
        };
        let path = work.path().join("dump.zip");
        std::fs::write(&path, zip).unwrap();
        Ok(DumpZip { path, size: zip.len() as u64 })
    }

    async fn delete_export(&self, _server: &ServerId, iri: &str, _id: &str) -> Result<DeleteOutcome, ClientError> {
        self.record("delete", shortcode(iri));
        Ok(DeleteOutcome::Deleted)
    }
}
