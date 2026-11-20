//! Following the archive projection's announcements: what [`crate::Cpe::follow`] runs on its
//! thread.
//!
//! Every re-activation loads every project again, whichever shortcode was announced, and is stored
//! whole or not at all; a per-project rebuild is DEV-7989.

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use cpe_ports::{ArchiveProjection, ChangeListener, DepositedFiles};

use crate::{activation, discovery, Cpe};

/// The listener [`Cpe::follow`] subscribes: it hands each shortcode to the thread and returns.
pub(crate) struct Announcements(pub(crate) Sender<String>);

impl ChangeListener for Announcements {
    fn changed(&self, shortcode: &str) {
        // Fails only once the thread is gone, and then nobody is left to follow.
        let _ = self.0.send(shortcode.to_owned());
    }
}

/// What the thread woke up for.
enum Wake {
    Announced(String),
    RetryDue,
    Closed,
}

fn wait(announced: &Receiver<String>, retry_at: Option<Instant>) -> Wake {
    let Some(at) = retry_at else {
        return announced.recv().map_or(Wake::Closed, Wake::Announced);
    };
    match announced.recv_timeout(at.saturating_duration_since(Instant::now())) {
        Ok(shortcode) => Wake::Announced(shortcode),
        Err(RecvTimeoutError::Timeout) => Wake::RetryDue,
        Err(RecvTimeoutError::Disconnected) => Wake::Closed,
    }
}

/// Re-activate `cpe` on each announcement for a shortcode a project folder declares, until every
/// sender of `announced` is gone; the announcements that queued up meanwhile are handled by one
/// re-activation. A pending retry is dropped once the senders are gone.
pub(crate) fn run(
    cpe: &Cpe,
    announced: &Receiver<String>,
    projection: &dyn ArchiveProjection,
    files: &dyn DepositedFiles,
    retry_after: Duration,
) {
    let mut retry_at: Option<Instant> = None;
    loop {
        match wait(announced, retry_at) {
            Wake::Closed => return,
            Wake::RetryDue => {
                tracing::info!("CPE tries its held-back re-activation once more");
                retry_at = None;
                reactivate(cpe, projection, files);
            }
            Wake::Announced(first) => {
                let shortcodes: BTreeSet<String> = std::iter::once(first).chain(announced.try_iter()).collect();
                if !names_a_project_folder(cpe, &shortcodes) {
                    tracing::debug!(?shortcodes, "an announcement names no project folder's shortcode; ignored");
                    continue;
                }
                tracing::info!(?shortcodes, "the archive projection announced a change: CPE re-activates");
                retry_at = (!reactivate(cpe, projection, files)).then(|| Instant::now() + retry_after);
            }
        }
    }
}

/// Whether a project folder declares one of `shortcodes`.
fn names_a_project_folder(cpe: &Cpe, shortcodes: &BTreeSet<String>) -> bool {
    discovery::discover_for_activation(&cpe.config.projects_dir)
        .projects
        .into_iter()
        .any(|project| project.shortcode.is_some_and(|shortcode| shortcodes.contains(&shortcode)))
}

/// Load every project again, and serve the result if no project held it back. Returns whether it
/// was stored. A panic keeps what was served and the thread.
fn reactivate(cpe: &Cpe, projection: &dyn ArchiveProjection, files: &dyn DepositedFiles) -> bool {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let next = activation::from_ports(&cpe.config, projection, files);
        // `cpe-reactivate` is the only thread that stores once the first activation is done.
        let held_back = cpe
            .activated()
            .map(|current| activation::held_back_by(&current, &next))
            .unwrap_or_default();
        if held_back.is_empty() {
            cpe.store(next);
        }
        held_back
    }));
    match outcome {
        Ok(held_back) if held_back.is_empty() => {
            tracing::info!("CPE re-activated");
            true
        }
        Ok(held_back) => {
            tracing::error!(
                project_ids = held_back.join(", ").as_str(),
                "CPE's re-activation is held back: these projects were served and would no longer be; what was \
                 served before is still served"
            );
            false
        }
        Err(payload) => {
            tracing::error!(
                panic = activation::panic_message(payload.as_ref()),
                "CPE's re-activation panicked; what was served before is still served"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Mutex, PoisonError, RwLock};
    use std::time::{Duration, Instant};

    use axum::http::StatusCode;
    use cpe_ports::{
        ArchiveProjection, ChangeListener, DepositedFiles, FakeArchiveProjection, FakeDepositedFiles, FilesError,
        ProjectSnapshot, ProjectionError, Provenance,
    };

    use crate::activation::tests::{configs, projects, snapshot, things_of, THINGS};
    use crate::tests::{assert_unavailable, config_for, get};
    use crate::Cpe;

    /// Answers to `provenance`, in order, the last one for good.
    type Answers = Mutex<Vec<Option<Provenance>>>;

    /// The next of `answers`; `None` when a test set none.
    fn next_answer(answers: &Answers) -> Option<Option<Provenance>> {
        let mut answers = answers.lock().unwrap_or_else(PoisonError::into_inner);
        match answers.len() {
            0 => None,
            1 => Some(answers[0].clone()),
            _ => Some(answers.remove(0)),
        }
    }

    /// A projection whose fake a test replaces while CPE follows it; it keeps the listeners
    /// itself, so a replacement does not lose them.
    #[derive(Default)]
    struct Projection {
        fake: RwLock<FakeArchiveProjection>,
        listeners: Mutex<Vec<Arc<dyn ChangeListener>>>,
        /// Overrides the fake's provenance when set.
        provenance: Answers,
        /// Replaces the fake, and is announced, once the next `snapshot` has read the current one.
        after_next_snapshot: Mutex<Option<FakeArchiveProjection>>,
        panics: AtomicBool,
        snapshots: AtomicUsize,
    }

    impl Projection {
        fn new(fake: FakeArchiveProjection) -> Self {
            Self { fake: RwLock::new(fake), ..Self::default() }
        }

        fn swap(&self, fake: FakeArchiveProjection) {
            *self.fake.write().unwrap_or_else(PoisonError::into_inner) = fake;
        }

        fn announce(&self, shortcode: &str) {
            let listeners = self.listeners.lock().unwrap_or_else(PoisonError::into_inner).clone();
            for listener in listeners {
                listener.changed(shortcode);
            }
        }

        fn snapshots(&self) -> usize {
            self.snapshots.load(Ordering::SeqCst)
        }
    }

    impl ArchiveProjection for Projection {
        fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
            assert!(!self.panics.load(Ordering::SeqCst), "the projection panics for {shortcode}");
            // Counted under the lock, so a test that sees the count and then swaps waits for this
            // read.
            let answer = {
                let fake = self.fake.read().unwrap_or_else(PoisonError::into_inner);
                self.snapshots.fetch_add(1, Ordering::SeqCst);
                fake.snapshot(shortcode)
            };
            let next = self.after_next_snapshot.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(next) = next {
                self.swap(next);
                self.announce(shortcode);
            }
            answer
        }

        fn provenance(&self, shortcode: &str) -> Result<Option<Provenance>, ProjectionError> {
            match next_answer(&self.provenance) {
                Some(answer) => Ok(answer),
                None => self.fake.read().unwrap_or_else(PoisonError::into_inner).provenance(shortcode),
            }
        }

        fn subscribe(&self, listener: Arc<dyn ChangeListener>) {
            self.listeners.lock().unwrap_or_else(PoisonError::into_inner).push(listener);
        }

        fn location_present(&self, shortcode: &str) -> bool {
            self.fake
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .location_present(shortcode)
        }
    }

    /// The configs of `things` and `other`, each with provenance `G1`.
    struct Files {
        fake: FakeDepositedFiles,
        /// Overrides the fake's provenance when set.
        provenance: Answers,
    }

    impl Files {
        fn new() -> Self {
            Self {
                fake: configs(&[("0001", THINGS), ("0003", &things_of("0003"))])
                    .with_provenance("0001", provenance("G1"))
                    .with_provenance("0003", provenance("G1")),
                provenance: Answers::default(),
            }
        }
    }

    impl DepositedFiles for Files {
        fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError> {
            self.fake.file(shortcode, name)
        }

        fn provenance(&self, shortcode: &str) -> Result<Option<Provenance>, FilesError> {
            match next_answer(&self.provenance) {
                Some(answer) => Ok(answer),
                None => self.fake.provenance(shortcode),
            }
        }
    }

    fn provenance(generation: &str) -> Provenance {
        Provenance {
            generation: generation.to_owned(),
            as_of: "2026-11-20T01:02:03Z".to_owned(),
            server: "prod".to_owned(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: format!("sha256:{}", "0".repeat(64)),
        }
    }

    /// `things` (0001) and `other` (0003), each with one lamp named `name`, provenance `G1`.
    fn lamps(name: &str) -> FakeArchiveProjection {
        FakeArchiveProjection::default()
            .with_project(snapshot("0001", name))
            .with_project(snapshot("0003", name))
            .with_provenance("0001", provenance("G1"))
            .with_provenance("0003", provenance("G1"))
    }

    /// CPE serving `things` and `other` from [`lamps`] named "Brass lamp".
    fn activated() -> (tempfile::TempDir, Cpe, Projection, Files) {
        let root = projects(&[("things", "0001"), ("other", "0003")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let projection = Projection::new(lamps("Brass lamp"));
        let files = Files::new();
        cpe.activate(&projection, &files);
        (root, cpe, projection, files)
    }

    /// Runs the follow loop over `announcements` and returns once it has handled them.
    fn run(cpe: &Cpe, projection: &Projection, files: &Files, announcements: &[&str]) {
        let (announce, announced) = mpsc::channel();
        for shortcode in announcements {
            announce.send((*shortcode).to_owned()).expect("the receiver is here");
        }
        drop(announce);
        super::run(cpe, &announced, projection, files, Duration::from_millis(10));
    }

    /// The name the lamp of the project at `prefix` is shown with, or the status that answered.
    async fn lamp(cpe: &Cpe, prefix: &str) -> Result<&'static str, StatusCode> {
        let (status, _, body) = get(cpe, &format!("/{prefix}/things/lamp")).await;
        if status != StatusCode::OK {
            return Err(status);
        }
        Ok(["Brass lamp", "Copper kettle"]
            .into_iter()
            .find(|name| body.contains(name))
            .expect("the page names the lamp"))
    }

    async fn assert_both_brass(cpe: &Cpe) {
        assert_eq!(lamp(cpe, "things").await, Ok("Brass lamp"));
        assert_eq!(lamp(cpe, "other").await, Ok("Brass lamp"));
    }

    /// Waits up to ten seconds for the project at `prefix` to show `name`.
    async fn eventually(cpe: &Cpe, prefix: &str, name: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while lamp(cpe, prefix).await != Ok(name) {
            assert!(Instant::now() < deadline, "{prefix} does not show {name}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Waits up to ten seconds for `projection` to have been asked for `count` snapshots.
    fn snapshots_reach(projection: &Projection, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while projection.snapshots() < count {
            assert!(Instant::now() < deadline, "{} snapshots, not {count}", projection.snapshots());
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[tokio::test]
    async fn an_announcement_for_a_folders_shortcode_reactivates_every_project() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle"));

        run(&cpe, &projection, &files, &["0001"]);

        assert_eq!(lamp(&cpe, "things").await, Ok("Copper kettle"));
        assert_eq!(lamp(&cpe, "other").await, Ok("Copper kettle"));
    }

    #[tokio::test]
    async fn an_announcement_for_a_shortcode_without_a_folder_runs_no_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle"));
        let before = projection.snapshots();

        run(&cpe, &projection, &files, &["0002", "0803"]);

        assert_eq!(projection.snapshots(), before);
        assert_both_brass(&cpe).await;
    }

    /// `sync`'s token goes first, so `media` still serves the provenance it had.
    #[tokio::test]
    async fn a_project_sync_no_longer_knows_is_no_longer_presented_and_the_others_are_refreshed() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(
            FakeArchiveProjection::default()
                .with_project(snapshot("0001", "Copper kettle"))
                .with_provenance("0001", provenance("G1")),
        );

        run(&cpe, &projection, &files, &["0003"]);

        assert_eq!(lamp(&cpe, "things").await, Ok("Copper kettle"));
        let (status, headers, _) = get(&cpe, "/other/").await;
        assert_unavailable(status, &headers, "/other/");
        assert_eq!(cpe.projects().len(), 1);
    }

    /// A lost mount looks like every token removed; the missing location tells it apart.
    #[tokio::test]
    async fn a_project_gone_with_its_location_keeps_the_previous_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(
            FakeArchiveProjection::default()
                .with_project(snapshot("0001", "Copper kettle"))
                .with_provenance("0001", provenance("G1"))
                .with_lost_location("0003"),
        );

        run(&cpe, &projection, &files, &["0003"]);

        assert_both_brass(&cpe).await;
    }

    #[tokio::test]
    async fn provenance_that_differs_between_the_ports_keeps_the_previous_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle").with_provenance("0001", provenance("G2")));

        run(&cpe, &projection, &files, &["0001"]);

        assert_both_brass(&cpe).await;
    }

    /// Both ports agree at each read, and the second read differs from the first. The answers are
    /// not per project: `other`, read first, takes both and holds the re-activation back.
    #[tokio::test]
    async fn provenance_that_changes_while_a_project_loads_keeps_the_previous_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle"));
        for answers in [&projection.provenance, &files.provenance] {
            *answers.lock().unwrap() = vec![Some(provenance("G1")), Some(provenance("G2"))];
        }

        run(&cpe, &projection, &files, &["0001"]);

        assert_both_brass(&cpe).await;
    }

    #[tokio::test]
    async fn a_project_that_loaded_before_and_now_fails_keeps_the_previous_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle").with_unavailable("0001"));

        run(&cpe, &projection, &files, &["0001"]);

        assert_both_brass(&cpe).await;
    }

    #[tokio::test]
    async fn a_panic_while_a_project_loads_keeps_the_previous_activation() {
        let (_root, cpe, projection, files) = activated();
        projection.swap(lamps("Copper kettle"));
        projection.panics.store(true, Ordering::SeqCst);

        run(&cpe, &projection, &files, &["0001"]);

        assert_both_brass(&cpe).await;
    }

    /// The change is announced from inside the first activation's first snapshot read.
    #[tokio::test]
    async fn a_change_made_during_the_first_activation_is_followed() {
        let root = projects(&[("things", "0001"), ("other", "0003")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let projection = Arc::new(Projection::new(lamps("Brass lamp")));
        *projection.after_next_snapshot.lock().unwrap() = Some(lamps("Copper kettle"));

        cpe.follow(projection.clone(), Arc::new(Files::new()), Duration::from_secs(60));

        eventually(&cpe, "other", "Copper kettle").await;
        assert_eq!(lamp(&cpe, "things").await, Ok("Copper kettle"));
    }

    #[tokio::test]
    async fn a_held_back_reactivation_is_tried_once_more_after_the_retry_interval() {
        let root = projects(&[("things", "0001"), ("other", "0003")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let projection = Arc::new(Projection::new(lamps("Brass lamp")));
        let retry_after = Duration::from_millis(300);
        cpe.follow(projection.clone(), Arc::new(Files::new()), retry_after);
        assert_both_brass(&cpe).await;

        // Two snapshot reads per activation. `other` is read first, then `things`, which fails.
        projection.swap(lamps("Copper kettle").with_unavailable("0001"));
        projection.announce("0001");
        snapshots_reach(&projection, 4);
        projection.swap(lamps("Copper kettle"));

        eventually(&cpe, "things", "Copper kettle").await;
        assert_eq!(projection.snapshots(), 6, "one retry, with no announcement");

        projection.swap(lamps("Brass lamp").with_unavailable("0001"));
        projection.announce("0001");
        std::thread::sleep(retry_after * 5);
        assert_eq!(projection.snapshots(), 10, "held back on the announcement and on its one retry");
        assert_eq!(lamp(&cpe, "other").await, Ok("Copper kettle"));
    }
}
