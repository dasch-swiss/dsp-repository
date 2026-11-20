//! One refresh against wiremock and temporary subtrees, with real time and time limits of a few
//! hundred milliseconds: paused tokio time would fire them at once while a task waits on a socket
//! or a blocking thread.

use std::collections::{BTreeMap, VecDeque};
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use shared_stand_in::{Generation, Provenance, ReadyToken, TOKEN_FILE};
use tempfile::TempDir;
use vre_bridge_client::{private_work_dir, Server, ServiceIdentity, Timeouts};
use vre_bridge_core::{
    inputs_digest, AllowlistEntry, FailureReason, Outcome, RawSettings, RefusalReason, ServerId, Settings,
    RESERVED_NAMES,
};
use vre_bridge_lift::{INTERIM_FORMAT_VERSION, TRANSFORMATION_VERSION};
use vre_bridge_store::{Lock, Subtrees, RIGHTS_RECORD_FILE};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::clock::FakeClock;
use crate::{sweep_dump_dirs, Busy, LogReport, RefreshReport, Refresher, Report, SNAPSHOT_FILE};

const KEY: &str = include_str!("../../client/testdata/ed25519-private.b64");
const IRI: &str = "http://rdfh.ch/projects/0803";
const ENCODED_IRI: &str = "http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0803";
const NOW: &str = "2026-11-20T01:00:00Z";
/// One day after `NOW`, as the client writes a cutoff.
const FORCED_CUTOFF: &str = "2026-11-21T01:00:00Z";
const AS_OF: &str = "2026-11-20T01:00:05.123456Z";
const OLD_AS_OF: &str = "2026-11-19T01:00:00Z";
/// `OLD_AS_OF` minus 60 seconds.
const OLD_CUTOFF: &str = "2026-11-19T00:59:00Z";
const OLD_GENERATION: &str = "20261119T010000Z-0000000a";

const ONTO: &str = "http://www.knora.org/ontology/0803/incunabula";
const DATA: &str = "http://www.knora.org/data/0803/incunabula";
const PUBLIC: &str = "V knora-admin:UnknownUser";
const MEMBERS_ONLY: &str = "CR knora-admin:Creator|V knora-admin:ProjectMember";
/// In the dump's `admin.nq`, which never leaves the zip.
const ADMIN_SECRET: &str = "ADMIN-SECRET-HASH";

// ---------------------------------------------------------------- dumps

/// A Page resource's lines in the data graph; every label holds `LITERAL`, which no log may show.
fn page(iri: &str, label: &str, permissions: &str) -> String {
    format!(
        "<{iri}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{ONTO}#Page> <{DATA}> .\n\
         <{iri}> <http://www.w3.org/2000/01/rdf-schema#label> \"{label}-LITERAL\" <{DATA}> .\n\
         <{iri}> <http://www.knora.org/ontology/knora-base#isDeleted> \"false\"^^<http://www.w3.org/2001/XMLSchema#boolean> <{DATA}> .\n\
         <{iri}> <http://www.knora.org/ontology/knora-base#hasPermissions> \"{permissions}\" <{DATA}> .\n"
    )
}

/// A dump zip as dsp-api builds it, with `data_nq` as its data and an `admin.nq` beside it.
fn dump_zip(data_nq: &str) -> Vec<u8> {
    let ontology = format!(
        "<{ONTO}#Page> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> <{ONTO}> .\n"
    );
    let admin = format!(
        "<http://rdfh.ch/users/u> <http://example.org/hash> \"{ADMIN_SECRET}\" <http://www.knora.org/data/admin> .\n"
    );
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in [
        ("bag-info.txt", "External-Identifier: 0803"),
        ("data/rdf/admin.nq", admin.as_str()),
        ("data/rdf/data.nq", data_nq),
        ("data/rdf/ontology-0.nq", ontology.as_str()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn public_dump() -> Vec<u8> {
    dump_zip(&page("http://rdfh.ch/0803/ZEBRA", "PUBLIC", PUBLIC))
}

fn refused_dump(pages: usize) -> Vec<u8> {
    let data: String = (0..pages)
        .map(|i| page(&format!("http://rdfh.ch/0803/P{i:03}"), &format!("REFUSED-{i}"), MEMBERS_ONLY))
        .collect();
    dump_zip(&data)
}

fn malformed_dump() -> Vec<u8> {
    dump_zip(&format!(
        "{}<http://rdfh.ch/0803/B> \"MALFORMED-LITERAL\" broken .\n",
        page("http://rdfh.ch/0803/A", "PUBLIC", PUBLIC)
    ))
}

// ---------------------------------------------------------------- DSP-API

fn export_path(id: &str, suffix: &str) -> String {
    format!("/v3/projects/{ENCODED_IRI}/exports/{id}{suffix}")
}

fn task(id: &str, status: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": id, "status": status}))
}

fn export_exists(id: &str) -> ResponseTemplate {
    ResponseTemplate::new(409).set_body_json(serde_json::json!({
        "errors": [{"code": "export_exists", "message": "an export exists", "details": {"id": id}}]
    }))
}

fn export_in_progress() -> ResponseTemplate {
    ResponseTemplate::new(409).set_body_json(serde_json::json!({
        "errors": [{"code": "export_in_progress", "message": "the export is running"}]
    }))
}

/// Answers each request with the next response; the last one repeats.
struct Sequence(Mutex<VecDeque<ResponseTemplate>>);

impl Sequence {
    fn of(responses: impl IntoIterator<Item = ResponseTemplate>) -> Self {
        Self(Mutex::new(responses.into_iter().collect()))
    }
}

impl Respond for Sequence {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let mut queue = self.0.lock().unwrap();
        if queue.len() > 1 {
            queue.pop_front().unwrap()
        } else {
            queue.front().unwrap().clone()
        }
    }
}

/// Answers `before` until `seen` counts a request, then `after`.
struct Until {
    seen: Arc<AtomicUsize>,
    before: ResponseTemplate,
    after: ResponseTemplate,
}

impl Respond for Until {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.seen.load(Ordering::SeqCst) == 0 {
            self.before.clone()
        } else {
            self.after.clone()
        }
    }
}

/// Counts its requests into `seen` and answers with `inner`.
struct Counting<R> {
    seen: Arc<AtomicUsize>,
    inner: R,
}

impl<R: Respond> Respond for Counting<R> {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        self.seen.fetch_add(1, Ordering::SeqCst);
        self.inner.respond(request)
    }
}

async fn mount(dsp: &MockServer, http_method: &str, at: impl Into<String>, response: impl Respond + 'static) {
    Mock::given(method(http_method))
        .and(path(at.into()))
        .respond_with(response)
        .mount(dsp)
        .await;
}

async fn project_lookup(dsp: &MockServer, shortcode: &str) {
    let body = serde_json::json!({"project": {"id": IRI, "shortcode": shortcode}});
    mount(
        dsp,
        "GET",
        format!("/admin/projects/shortcode/{shortcode}"),
        ResponseTemplate::new(200).set_body_json(body),
    )
    .await;
}

/// The change check, answering `count` resources only for `cutoff`.
async fn change_check(dsp: &MockServer, cutoff: &str, count: usize) {
    Mock::given(method("GET"))
        .and(path("/v2/resources/info"))
        .and(query_param("cutoff", cutoff))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"resources": [], "count": count, "asOf": AS_OF})),
        )
        .mount(dsp)
        .await;
}

async fn export_started(dsp: &MockServer, id: &str) {
    let started = ResponseTemplate::new(202).set_body_json(serde_json::json!({"id": id, "status": "in_progress"}));
    mount(dsp, "POST", format!("/v3/projects/{ENCODED_IRI}/exports"), started).await;
}

/// The export `id` completes at once, downloads as `zip` and is deleted.
async fn export_lifecycle(dsp: &MockServer, id: &str, zip: Vec<u8>) {
    mount(dsp, "GET", export_path(id, ""), task(id, "completed")).await;
    mount(
        dsp,
        "GET",
        export_path(id, "/download"),
        ResponseTemplate::new(200).set_body_bytes(zip),
    )
    .await;
    mount(dsp, "DELETE", export_path(id, ""), ResponseTemplate::new(204)).await;
}

/// A forced refresh of `shortcode` that gets `zip`.
async fn forced(dsp: &MockServer, shortcode: &str, zip: Vec<u8>) {
    project_lookup(dsp, shortcode).await;
    change_check(dsp, FORCED_CUTOFF, 0).await;
    export_started(dsp, "exp1").await;
    export_lifecycle(dsp, "exp1", zip).await;
}

async fn requests(dsp: &MockServer, http_method: &str, at: &str) -> usize {
    dsp.received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.method.as_str() == http_method && r.url.path() == at)
        .count()
}

// ---------------------------------------------------------------- the bridge

#[derive(Default)]
struct Recording(Mutex<Vec<RefreshReport>>);

impl Report for Recording {
    fn refresh(&self, report: &RefreshReport) {
        self.0.lock().unwrap().push(report.clone());
    }
}

impl Recording {
    fn reports(&self) -> Vec<RefreshReport> {
        self.0.lock().unwrap().clone()
    }
}

/// The subtrees, inputs, work directory and lock of one test, and a VRE server per id.
struct Env {
    servers: BTreeMap<String, String>,
    sync: TempDir,
    media: TempDir,
    inputs: TempDir,
    work: TempDir,
    lock_dir: TempDir,
    settings: Settings,
    recording: Arc<Recording>,
}

impl Env {
    /// `allowlist` as `SC=server,…`; `servers` maps each server id to its base URL.
    fn new(allowlist: &str, servers: &[(&str, &str)]) -> Self {
        // Every test's logs go to the capture the leak check reads.
        logs();
        let (sync, media, inputs, work, lock_dir) = (temp(), temp(), temp(), temp(), temp());
        let list =
            |f: &dyn Fn(&str, &str) -> String| servers.iter().map(|(id, url)| f(id, url)).collect::<Vec<_>>().join(",");
        let raw = RawSettings {
            sync_dir: Some(display(sync.path())),
            media_dir: Some(display(media.path())),
            lock_file: Some(display(&lock_dir.path().join("lock"))),
            allowlist: Some(allowlist.to_owned()),
            servers: Some(list(&|id, url| format!("{id}={url}"))),
            key_ids: Some(list(&|id, _| format!("{id}=test-kid"))),
            signing_keys: Some(list(&|id, _| format!("{id}={}", KEY.trim())).into()),
            service_name: Some("dsp-repository-test".to_owned()),
            inputs_dir: Some(display(inputs.path())),
            work_dir: Some(display(work.path())),
            ..RawSettings::default()
        };
        let mut settings = Settings::parse(raw).unwrap().unwrap();
        settings.export_poll = Duration::from_millis(20);
        settings.export_timeout = Duration::from_millis(300);
        let servers = servers.iter().map(|(id, url)| ((*id).to_owned(), (*url).to_owned())).collect();
        Self {
            servers,
            sync,
            media,
            inputs,
            work,
            lock_dir,
            settings,
            recording: Arc::default(),
        }
    }

    fn one(shortcode: &str, dsp: &MockServer) -> Self {
        Self::new(&format!("{shortcode}=prod"), &[("prod", &dsp.uri())])
    }

    fn refresher(&self, report: Arc<dyn Report>) -> Refresher {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let timeouts = Timeouts {
            connect: Duration::from_millis(500),
            read: Duration::from_millis(500),
            request: Duration::from_millis(500),
            download: Duration::from_millis(800),
            file: Duration::from_millis(500),
        };
        let vre: BTreeMap<ServerId, Server> = self
            .settings
            .servers
            .iter()
            .map(|(id, s)| {
                let identity =
                    ServiceIdentity::new(&self.settings.service_name, &s.key_id, s.signing_key.expose()).unwrap();
                (id.clone(), Server::new(&s.base_url, identity, timeouts).unwrap())
            })
            .collect();
        assert_eq!(vre.len(), self.servers.len());
        Refresher::new(
            Arc::new(vre),
            report,
            Subtrees::new(self.sync.path().to_owned(), self.media.path().to_owned()),
            Arc::new(self.settings.clone()),
            Arc::new(FakeClock::at(NOW)),
        )
    }

    fn entry(&self, shortcode: &str) -> AllowlistEntry {
        self.settings
            .allowlist
            .entries()
            .iter()
            .find(|e| e.shortcode == shortcode)
            .unwrap()
            .clone()
    }

    async fn refresh_with(&self, refresher: &Refresher, shortcode: &str, operator: bool) -> Result<Outcome, Busy> {
        let lock = Lock::try_acquire(&self.lock_dir.path().join("lock")).unwrap().unwrap();
        let outcome = refresher.refresh(&lock, &self.entry(shortcode), operator).await;
        self.assert_work_dir_empty();
        outcome
    }

    async fn refresh(&self, shortcode: &str, operator: bool) -> Result<Outcome, Busy> {
        let refresher = self.refresher(self.recording.clone());
        self.refresh_with(&refresher, shortcode, operator).await
    }

    fn assert_work_dir_empty(&self) {
        let left: Vec<_> = std::fs::read_dir(self.work.path()).unwrap().collect();
        assert!(left.is_empty(), "the work directory holds {left:?}");
    }

    /// The token files' bytes, `sync`'s first.
    fn token_bytes(&self, shortcode: &str) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let read = |root: &Path| std::fs::read(root.join(shortcode).join(TOKEN_FILE)).ok();
        (read(self.sync.path()), read(self.media.path()))
    }

    fn token(&self, root: &Path, shortcode: &str) -> ReadyToken {
        ReadyToken::parse(&std::fs::read(root.join(shortcode).join(TOKEN_FILE)).unwrap()).unwrap()
    }

    /// Writes a token naming `generation`, and the generation's directory, into one subtree.
    fn seed(&self, root: &Path, shortcode: &str, generation: &str, as_of: &str) {
        let project = root.join(shortcode);
        std::fs::create_dir_all(project.join(generation)).unwrap();
        let token = ReadyToken {
            provenance: Provenance {
                generation: Generation::parse(generation).unwrap(),
                as_of: as_of.to_owned(),
                server: "prod".to_owned(),
                interim_format_version: INTERIM_FORMAT_VERSION,
                transformation_version: TRANSFORMATION_VERSION,
                inputs_digest: inputs_digest(&self.inputs.path().join(shortcode)).unwrap(),
            },
            previous_generation: None,
        };
        std::fs::write(project.join(TOKEN_FILE), token.to_bytes()).unwrap();
    }

    /// Seeds agreeing tokens, runs an operator's refresh and checks it ends as `expected` with
    /// both tokens' bytes as they were.
    async fn assert_ends_untouched(&self, shortcode: &str, expected: Outcome) {
        self.seed_both(shortcode);
        let before = self.token_bytes(shortcode);

        assert_eq!(self.refresh(shortcode, true).await, Ok(expected));

        assert_eq!(self.token_bytes(shortcode), before);
        assert!(before.0.is_some() && before.1.is_some());
    }

    /// Agreeing tokens in both subtrees, as an earlier refresh leaves them.
    fn seed_both(&self, shortcode: &str) {
        self.seed(self.sync.path(), shortcode, OLD_GENERATION, OLD_AS_OF);
        self.seed(self.media.path(), shortcode, OLD_GENERATION, OLD_AS_OF);
    }
}

fn temp() -> TempDir {
    tempfile::tempdir().unwrap()
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

// ---------------------------------------------------------------- logs

/// Everything any test logs, from every thread: the subscriber is the process's global one.
fn logs() -> Arc<Mutex<Vec<u8>>> {
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
fn lines_of(project: &str) -> Vec<String> {
    let text = String::from_utf8(logs().lock().unwrap().clone()).unwrap();
    let marker = format!("project={project}");
    text.lines().filter(|line| line.contains(&marker)).map(str::to_owned).collect()
}

// ---------------------------------------------------------------- change check

#[tokio::test]
async fn test_forced_without_tokens_asks_one_day_ahead_and_takes_t_from_as_of() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0803", public_dump()).await;
    let env = Env::one("0803", &dsp);

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    let token = env.token(env.sync.path(), "0803");
    assert_eq!(token.provenance.as_of, AS_OF);
    assert!(
        token.provenance.generation.as_str().starts_with("20261120T010005Z-"),
        "{token:?}"
    );
    let checks: Vec<_> = dsp
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/v2/resources/info")
        .collect();
    assert_eq!(checks.len(), 1);
    assert!(checks[0].url.query_pairs().any(|(k, v)| k == "cutoff" && v == FORCED_CUTOFF));
}

#[tokio::test]
async fn test_operator_refresh_asks_one_day_ahead_whatever_t_is() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0803", public_dump()).await;
    let env = Env::one("0803", &dsp);
    env.seed_both("0803");

    let outcome = env.refresh("0803", true).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    assert_eq!(env.token(env.media.path(), "0803").provenance.as_of, AS_OF);
}

// ---------------------------------------------------------------- recovery

#[tokio::test]
async fn test_operator_refresh_of_diverged_tokens_points_media_back_first() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0803", public_dump()).await;
    let env = Env::one("0803", &dsp);
    let media_only = "20261118T010000Z-0000000b";
    env.seed(env.sync.path(), "0803", OLD_GENERATION, OLD_AS_OF);
    env.seed(env.media.path(), "0803", media_only, OLD_AS_OF);
    std::fs::create_dir_all(env.media.path().join("0803").join(OLD_GENERATION)).unwrap();

    let outcome = env.refresh("0803", true).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    let media = env.token(env.media.path(), "0803");
    assert_eq!(media.previous_generation.as_ref().map(Generation::as_str), Some(OLD_GENERATION));
    // Without recovery first, the replaced media token would have kept its generation.
    assert!(!env.media.path().join("0803").join(media_only).exists());
    assert!(env.media.path().join("0803").join(OLD_GENERATION).exists());
}

// ---------------------------------------------------------------- success

#[tokio::test]
async fn test_unchanged_project_leaves_its_tokens_and_starts_no_export() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, OLD_CUTOFF, 0).await;
    let env = Env::one("0803", &dsp);
    env.seed_both("0803");
    let before = env.token_bytes("0803");

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Unchanged));
    assert_eq!(env.token_bytes("0803"), before);
    assert_eq!(requests(&dsp, "POST", &format!("/v3/projects/{ENCODED_IRI}/exports")).await, 0);
    let reports = env.recording.reports();
    assert_eq!(reports.len(), 1);
    assert_eq!((reports[0].outcome, reports[0].sizes), (Outcome::Unchanged, None));
}

#[tokio::test]
async fn test_changed_project_is_refreshed_into_both_subtrees_and_its_export_deleted() {
    let dsp = MockServer::start().await;
    let zip = public_dump();
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, OLD_CUTOFF, 2).await;
    export_started(&dsp, "exp1").await;
    export_lifecycle(&dsp, "exp1", zip.clone()).await;
    let env = Env::one("0803", &dsp);
    std::fs::create_dir_all(env.inputs.path().join("0803")).unwrap();
    std::fs::write(env.inputs.path().join("0803").join("config.kdl"), "project \"0803\"\n").unwrap();
    env.seed_both("0803");
    // The seeded tokens carry this digest; the refresh must compute the same.
    let digest = inputs_digest(&env.inputs.path().join("0803")).unwrap();

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    let (sync, media) = env.token_bytes("0803");
    assert_eq!(sync, media);
    let token = env.token(env.sync.path(), "0803");
    assert_eq!(token.provenance.as_of, AS_OF);
    assert_eq!(token.provenance.inputs_digest, digest);
    assert_eq!(token.provenance.server, "prod");
    assert_eq!(token.previous_generation.as_ref().map(Generation::as_str), Some(OLD_GENERATION));
    let generation = token.provenance.generation.as_str();
    let snapshot = std::fs::read(env.sync.path().join("0803").join(generation).join(SNAPSHOT_FILE)).unwrap();
    assert!(String::from_utf8_lossy(&snapshot).contains("PUBLIC-LITERAL"));
    let media_dir = env.media.path().join("0803").join(generation);
    let rights = std::fs::read(media_dir.join(RIGHTS_RECORD_FILE)).unwrap();
    let config = std::fs::read(media_dir.join("config.kdl")).unwrap();
    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
    let reports = env.recording.reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Refreshed);
    let sizes = reports[0].sizes.unwrap();
    assert_eq!(sizes.dump, zip.len() as u64);
    assert_eq!(sizes.snapshot, snapshot.len() as u64);
    assert_eq!(sizes.media, (rights.len() + config.len()) as u64);
}

// ---------------------------------------------------------------- refusal and failure

#[tokio::test]
async fn test_guard_refusal_leaves_tokens_deletes_the_export_and_logs_fifty_violations_and_a_total() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0A01", refused_dump(60)).await;
    let env = Env::one("0A01", &dsp);

    env.assert_ends_untouched("0A01", Outcome::Refused(RefusalReason::PublicGuard))
        .await;

    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
    let lines = lines_of("0A01");
    let violations = lines.iter().filter(|l| l.contains("the public guard refuses an item")).count();
    assert_eq!(violations, 50, "{lines:#?}");
    assert!(lines
        .iter()
        .any(|l| l.contains("the public guard refused the project") && l.contains("total=60")));
}

#[tokio::test]
async fn test_failed_export_is_failed_and_deleted() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    export_started(&dsp, "exp1").await;
    mount(&dsp, "GET", export_path("exp1", ""), task("exp1", "failed")).await;
    mount(&dsp, "DELETE", export_path("exp1", ""), ResponseTemplate::new(204)).await;
    let env = Env::one("0803", &dsp);

    env.assert_ends_untouched("0803", Outcome::Failed(FailureReason::Export)).await;

    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
}

#[tokio::test]
async fn test_server_refusing_the_service_identity_is_refused() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    mount(
        &dsp,
        "POST",
        format!("/v3/projects/{ENCODED_IRI}/exports"),
        ResponseTemplate::new(403),
    )
    .await;
    let env = Env::one("0803", &dsp);

    env.assert_ends_untouched("0803", Outcome::Refused(RefusalReason::Unauthorized))
        .await;
}

#[tokio::test]
async fn test_dump_breaking_the_closed_world_is_refused() {
    let dsp = MockServer::start().await;
    // A resource without a label.
    let unlabelled = page("http://rdfh.ch/0803/A", "X", PUBLIC)
        .lines()
        .filter(|line| !line.contains("rdf-schema#label"))
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    forced(&dsp, "0803", dump_zip(&unlabelled)).await;
    let env = Env::one("0803", &dsp);

    env.assert_ends_untouched("0803", Outcome::Refused(RefusalReason::ClosedWorld))
        .await;
}

#[tokio::test]
async fn test_export_left_undeleted_does_not_change_a_refreshed_outcome() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    export_started(&dsp, "exp1").await;
    mount(&dsp, "GET", export_path("exp1", ""), task("exp1", "completed")).await;
    mount(
        &dsp,
        "GET",
        export_path("exp1", "/download"),
        ResponseTemplate::new(200).set_body_bytes(public_dump()),
    )
    .await;
    mount(&dsp, "DELETE", export_path("exp1", ""), ResponseTemplate::new(500)).await;
    let env = Env::one("0803", &dsp);

    assert_eq!(env.refresh("0803", false).await, Ok(Outcome::Refreshed));
    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
}

#[tokio::test]
async fn test_unreachable_server_is_failed() {
    // Bound and dropped: nothing listens on the port.
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let env = Env::new("0803=prod", &[("prod", &format!("http://127.0.0.1:{port}"))]);
    env.seed_both("0803");
    let before = env.token_bytes("0803");

    let outcome = env.refresh("0803", true).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Unreachable)));
    assert_eq!(env.token_bytes("0803"), before);
}

#[tokio::test]
async fn test_panic_in_the_blocking_lift_is_failed() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0803", public_dump()).await;
    let env = Env::one("0803", &dsp);
    env.seed_both("0803");
    let before = env.token_bytes("0803");
    let refresher = env.refresher(env.recording.clone()).with_lift(|_| panic!("the lift broke"));

    let outcome = env.refresh_with(&refresher, "0803", true).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Internal)));
    assert_eq!(env.token_bytes("0803"), before);
    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
}

#[tokio::test]
async fn test_dump_over_its_cap_is_failed_and_its_export_deleted() {
    let dsp = MockServer::start().await;
    let zip = public_dump();
    forced(&dsp, "0803", zip.clone()).await;
    let mut env = Env::one("0803", &dsp);
    env.settings.max_dump_bytes = zip.len() as u64 - 1;

    env.assert_ends_untouched("0803", Outcome::Failed(FailureReason::Dump)).await;

    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
}

#[tokio::test]
async fn test_dump_unpacking_over_its_cap_is_failed() {
    let dsp = MockServer::start().await;
    forced(&dsp, "0803", public_dump()).await;
    let mut env = Env::one("0803", &dsp);
    env.settings.max_unpacked_bytes = 10;

    env.assert_ends_untouched("0803", Outcome::Failed(FailureReason::Dump)).await;
}

#[tokio::test]
async fn test_missing_inputs_directory_is_failed_before_any_request() {
    let dsp = MockServer::start().await;
    let mut env = Env::one("0803", &dsp);
    env.settings.inputs_dir = env.inputs.path().join("missing");

    let outcome = env.refresh("0803", true).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Inputs)));
    assert!(dsp.received_requests().await.unwrap().is_empty());
}

// ---------------------------------------------------------------- 409

#[tokio::test]
async fn test_own_export_in_the_slot_is_finished_deleted_and_the_trigger_asked_again() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    let started = ResponseTemplate::new(202).set_body_json(serde_json::json!({"id": "exp1", "status": "in_progress"}));
    mount(
        &dsp,
        "POST",
        format!("/v3/projects/{ENCODED_IRI}/exports"),
        Sequence::of([export_exists("old"), started]),
    )
    .await;
    mount(&dsp, "GET", export_path("old", ""), task("old", "completed")).await;
    mount(&dsp, "DELETE", export_path("old", ""), ResponseTemplate::new(204)).await;
    export_lifecycle(&dsp, "exp1", public_dump()).await;
    let env = Env::one("0803", &dsp);

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    assert_eq!(requests(&dsp, "DELETE", &export_path("old", "")).await, 1);
    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 1);
}

#[tokio::test]
async fn test_own_export_in_the_slot_twice_is_failed() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    mount(
        &dsp,
        "POST",
        format!("/v3/projects/{ENCODED_IRI}/exports"),
        export_exists("old"),
    )
    .await;
    mount(&dsp, "GET", export_path("old", ""), task("old", "completed")).await;
    mount(&dsp, "DELETE", export_path("old", ""), ResponseTemplate::new(204)).await;
    let env = Env::one("0803", &dsp);

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Server)));
    assert_eq!(requests(&dsp, "POST", &format!("/v3/projects/{ENCODED_IRI}/exports")).await, 2);
    assert_eq!(requests(&dsp, "DELETE", &export_path("old", "")).await, 1);
}

#[tokio::test]
async fn test_own_export_still_running_after_the_time_limit_is_failed() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    mount(
        &dsp,
        "POST",
        format!("/v3/projects/{ENCODED_IRI}/exports"),
        export_exists("old"),
    )
    .await;
    mount(&dsp, "GET", export_path("old", ""), task("old", "in_progress")).await;
    mount(&dsp, "DELETE", export_path("old", ""), export_in_progress()).await;
    let env = Env::one("0803", &dsp);

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Timeout)));
    assert_eq!(requests(&dsp, "POST", &format!("/v3/projects/{ENCODED_IRI}/exports")).await, 1);
}

#[tokio::test]
async fn test_another_callers_export_in_the_slot_is_busy_and_not_reported() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    mount(
        &dsp,
        "POST",
        format!("/v3/projects/{ENCODED_IRI}/exports"),
        export_exists("theirs"),
    )
    .await;
    mount(&dsp, "GET", export_path("theirs", ""), ResponseTemplate::new(404)).await;
    let env = Env::one("0803", &dsp);
    env.seed_both("0803");
    let before = env.token_bytes("0803");

    let outcome = env.refresh("0803", true).await;

    assert_eq!(outcome, Err(Busy));
    assert_eq!(env.token_bytes("0803"), before);
    assert!(env.recording.reports().is_empty());
    assert_eq!(requests(&dsp, "DELETE", &export_path("theirs", "")).await, 0);
}

// ---------------------------------------------------------------- delete

#[tokio::test]
async fn test_timed_out_export_is_deleted_once_it_ends_and_the_refresh_stays_failed() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    export_started(&dsp, "exp1").await;
    // The export runs until the bridge first tries to delete it, so past the first wait's limit.
    let deletes = Arc::new(AtomicUsize::new(0));
    let ends = Until {
        seen: Arc::clone(&deletes),
        before: task("exp1", "in_progress"),
        after: task("exp1", "completed"),
    };
    mount(&dsp, "GET", export_path("exp1", ""), ends).await;
    let delete = Counting {
        seen: deletes,
        inner: Sequence::of([export_in_progress(), ResponseTemplate::new(204)]),
    };
    mount(&dsp, "DELETE", export_path("exp1", ""), delete).await;
    let env = Env::one("0803", &dsp);

    let outcome = env.refresh("0803", false).await;

    assert_eq!(outcome, Ok(Outcome::Failed(FailureReason::Timeout)));
    assert_eq!(requests(&dsp, "DELETE", &export_path("exp1", "")).await, 2);
    assert_eq!(requests(&dsp, "GET", &export_path("exp1", "/download")).await, 0);
}

#[tokio::test]
async fn test_a_failed_status_poll_is_polled_again() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    export_started(&dsp, "exp1").await;
    let status = Sequence::of([
        ResponseTemplate::new(503),
        ResponseTemplate::new(502),
        task("exp1", "completed"),
    ]);
    mount(&dsp, "GET", export_path("exp1", ""), status).await;
    mount(
        &dsp,
        "GET",
        export_path("exp1", "/download"),
        ResponseTemplate::new(200).set_body_bytes(public_dump()),
    )
    .await;
    mount(&dsp, "DELETE", export_path("exp1", ""), ResponseTemplate::new(204)).await;
    let env = Env::one("0803", &dsp);

    assert_eq!(env.refresh("0803", false).await, Ok(Outcome::Refreshed));
}

#[tokio::test]
async fn test_a_rate_limited_status_poll_is_polled_again() {
    let dsp = MockServer::start().await;
    project_lookup(&dsp, "0803").await;
    change_check(&dsp, FORCED_CUTOFF, 0).await;
    export_started(&dsp, "exp1").await;
    let status = Sequence::of([ResponseTemplate::new(429), task("exp1", "completed")]);
    mount(&dsp, "GET", export_path("exp1", ""), status).await;
    mount(
        &dsp,
        "GET",
        export_path("exp1", "/download"),
        ResponseTemplate::new(200).set_body_bytes(public_dump()),
    )
    .await;
    mount(&dsp, "DELETE", export_path("exp1", ""), ResponseTemplate::new(204)).await;
    let env = Env::one("0803", &dsp);

    assert_eq!(env.refresh("0803", false).await, Ok(Outcome::Refreshed));
}

// ---------------------------------------------------------------- scope

#[tokio::test]
async fn test_each_project_goes_to_the_server_its_entry_names() {
    let (prod, stage) = (MockServer::start().await, MockServer::start().await);
    forced(&stage, "0804", public_dump()).await;
    let env = Env::new("0803=prod,0804=stage", &[("prod", &prod.uri()), ("stage", &stage.uri())]);

    let outcome = env.refresh("0804", false).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    assert!(prod.received_requests().await.unwrap().is_empty());
    assert_eq!(env.token(env.sync.path(), "0804").provenance.server, "stage");
}

// ---------------------------------------------------------------- logs

#[tokio::test]
async fn test_outcome_lines_name_the_refresh_and_no_log_holds_dump_content_a_key_or_a_token() {
    let logs = logs();
    let (refusing, malformed) = (MockServer::start().await, MockServer::start().await);
    forced(&refusing, "0A02", refused_dump(3)).await;
    forced(&malformed, "0A03", malformed_dump()).await;
    let env = Env::new("0A02=one,0A03=two", &[("one", &refusing.uri()), ("two", &malformed.uri())]);
    let refresher = env.refresher(Arc::new(LogReport));

    let first = env.refresh_with(&refresher, "0A02", false).await;
    let second = env.refresh_with(&refresher, "0A03", false).await;

    assert_eq!(first, Ok(Outcome::Refused(RefusalReason::PublicGuard)));
    assert_eq!(second, Ok(Outcome::Failed(FailureReason::Dump)));
    for (project, server, outcome, reason) in [
        ("0A02", "one", "refused", "public-guard"),
        ("0A03", "two", "failed", "dump"),
    ] {
        let lines = lines_of(project);
        let line = lines
            .iter()
            .find(|l| l.contains("refresh ended"))
            .unwrap_or_else(|| panic!("{lines:#?}"));
        for field in [
            format!("server={server}"),
            outcome.to_owned(),
            reason.to_owned(),
            "duration_secs=".to_owned(),
        ] {
            assert!(line.contains(&field), "{field} missing: {line}");
        }
    }
    let text = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    for leak in ["LITERAL", ADMIN_SECRET, KEY.trim(), "eyJ"] {
        assert!(!text.contains(leak), "the logs hold {leak:?}");
    }
}

// ---------------------------------------------------------------- names and the sweep

#[test]
fn test_reserved_names_cover_the_rights_record() {
    assert!(RESERVED_NAMES.contains(&RIGHTS_RECORD_FILE));
}

#[test]
fn test_sweep_deletes_a_private_work_directory() {
    let work = temp();
    let dump = private_work_dir(work.path()).unwrap().keep();

    assert_eq!(sweep_dump_dirs(work.path()).unwrap(), 1);
    assert!(!dump.exists());
}
