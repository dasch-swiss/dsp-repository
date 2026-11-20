use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};

use cpe_ports::{ArchiveProjection, ChangeListener};
use shared_stand_in::{Generation, Provenance, ReadyToken};

use crate::LiveArchiveProjection;

const INTERVAL: Duration = Duration::from_millis(50);
/// Long enough for several rounds, so a missing announcement is not just a late one.
const QUIET: Duration = Duration::from_millis(400);
const WITHIN: Duration = Duration::from_secs(2);

#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl Recording {
    fn announced(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    /// Waits until `count` announcements arrived, or [`WITHIN`] passed.
    fn wait_for(&self, count: usize) -> Vec<String> {
        let deadline = Instant::now() + WITHIN;
        while self.announced().len() < count && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        self.announced()
    }
}

impl ChangeListener for Recording {
    fn changed(&self, shortcode: &str) {
        self.0.lock().unwrap().push(shortcode.to_string());
    }
}

fn token_bytes(generation: &str) -> Vec<u8> {
    ReadyToken {
        provenance: Provenance {
            generation: Generation::parse(generation).expect("a generation name"),
            as_of: "2026-11-20T01:02:03Z".to_string(),
            server: "prod".to_string(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: format!("sha256:{}", "0".repeat(64)),
        },
        previous_generation: None,
    }
    .to_bytes()
}

/// Writes the token in `dir` as the bridge does, by a rename, so no round sees half a token.
fn write_token(dir: &Path, bytes: &[u8]) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(".ready.tmp"), bytes).unwrap();
    fs::rename(dir.join(".ready.tmp"), dir.join(".ready")).unwrap();
}

const FIRST: &str = "20261120T010203Z-9f3a1c2e";
const SECOND: &str = "20261121T010203Z-0b1d2e3f";

fn subscribed(root: &Path) -> (LiveArchiveProjection, Arc<Recording>) {
    let adapter = LiveArchiveProjection::new(root).with_poll_interval(INTERVAL);
    let recording = Arc::new(Recording::default());
    adapter.subscribe(recording.clone());
    (adapter, recording)
}

#[test]
fn test_subscribe_tokens_present_before_are_not_announced() {
    let dir = tempfile::tempdir().unwrap();
    write_token(&dir.path().join("0803"), &token_bytes(FIRST));

    let (_adapter, recording) = subscribed(dir.path());
    thread::sleep(QUIET);

    assert!(recording.announced().is_empty(), "{:?}", recording.announced());
}

#[test]
fn test_subscribe_token_written_after_is_announced() {
    let dir = tempfile::tempdir().unwrap();
    let (_adapter, recording) = subscribed(dir.path());

    write_token(&dir.path().join("0803"), &token_bytes(FIRST));

    assert_eq!(recording.wait_for(1), ["0803"]);
}

#[test]
fn test_subscribe_token_rewritten_with_other_bytes_is_announced_again() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("0803");
    write_token(&project, &token_bytes(FIRST));
    let (_adapter, recording) = subscribed(dir.path());

    write_token(&project, &token_bytes(SECOND));
    assert_eq!(recording.wait_for(1), ["0803"]);
    write_token(&project, &token_bytes(FIRST));

    assert_eq!(recording.wait_for(2), ["0803", "0803"]);
}

#[test]
fn test_subscribe_token_rewritten_with_identical_bytes_is_not_announced() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("0803");
    write_token(&project, &token_bytes(FIRST));
    let (_adapter, recording) = subscribed(dir.path());

    write_token(&project, &token_bytes(FIRST));
    thread::sleep(QUIET);

    assert!(recording.announced().is_empty(), "{:?}", recording.announced());
}

#[test]
fn test_subscribe_token_removed_is_announced() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("0803");
    write_token(&project, &token_bytes(FIRST));
    let (_adapter, recording) = subscribed(dir.path());

    fs::remove_file(project.join(".ready")).unwrap();

    assert_eq!(recording.wait_for(1), ["0803"]);
}

#[test]
fn test_subscribe_token_replaced_by_an_invalid_one_is_announced() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("0803");
    write_token(&project, &token_bytes(FIRST));
    let (_adapter, recording) = subscribed(dir.path());

    write_token(&project, b"{}\n");

    assert_eq!(recording.wait_for(1), ["0803"]);
}

#[test]
fn test_subscribe_token_of_a_non_shortcode_directory_is_never_announced() {
    let dir = tempfile::tempdir().unwrap();
    let (_adapter, recording) = subscribed(dir.path());

    for name in ["lost+found", "083c", "PROVENANCE"] {
        write_token(&dir.path().join(name), &token_bytes(FIRST));
    }
    write_token(&dir.path().join("0803"), &token_bytes(FIRST));

    // The shortcode's announcement shows a round ran after every token was written.
    assert_eq!(recording.wait_for(1), ["0803"]);
    thread::sleep(QUIET);
    assert_eq!(recording.announced(), ["0803"]);
}

#[test]
fn test_subscribe_every_listener_is_told() {
    let dir = tempfile::tempdir().unwrap();
    let (adapter, first) = subscribed(dir.path());
    let second = Arc::new(Recording::default());
    adapter.subscribe(second.clone());

    write_token(&dir.path().join("0803"), &token_bytes(FIRST));

    assert_eq!(first.wait_for(1), ["0803"]);
    assert_eq!(second.wait_for(1), ["0803"]);
}

struct Panicking;

impl ChangeListener for Panicking {
    fn changed(&self, _shortcode: &str) {
        panic!("a listener's own fault");
    }
}

#[test]
fn test_subscribe_panicking_listener_leaves_the_others_told_and_the_poll_running() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = LiveArchiveProjection::new(dir.path()).with_poll_interval(INTERVAL);
    adapter.subscribe(Arc::new(Panicking));
    let recording = Arc::new(Recording::default());
    adapter.subscribe(recording.clone());

    write_token(&dir.path().join("0803"), &token_bytes(FIRST));
    assert_eq!(recording.wait_for(1), ["0803"]);
    write_token(&dir.path().join("0804"), &token_bytes(FIRST));

    assert_eq!(recording.wait_for(2), ["0803", "0804"]);
}

/// Holds the only handle on the adapter it subscribed to, and drops it when told.
struct DropsTheAdapter {
    adapter: Mutex<Option<LiveArchiveProjection>>,
    dropped: Mutex<bool>,
}

impl ChangeListener for DropsTheAdapter {
    fn changed(&self, _shortcode: &str) {
        drop(self.adapter.lock().unwrap().take());
        *self.dropped.lock().unwrap() = true;
    }
}

#[test]
fn test_drop_on_the_poll_thread_from_a_listener_returns() {
    let dir = tempfile::tempdir().unwrap();
    let listener = Arc::new(DropsTheAdapter { adapter: Mutex::new(None), dropped: Mutex::new(false) });
    let adapter = LiveArchiveProjection::new(dir.path()).with_poll_interval(INTERVAL);
    adapter.subscribe(listener.clone());
    *listener.adapter.lock().unwrap() = Some(adapter);

    write_token(&dir.path().join("0803"), &token_bytes(FIRST));

    let deadline = Instant::now() + WITHIN;
    while !*listener.dropped.lock().unwrap() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(*listener.dropped.lock().unwrap(), "the drop on the poll thread did not return");
}

#[test]
fn test_drop_stops_the_poll_thread() {
    let dir = tempfile::tempdir().unwrap();
    let (adapter, recording) = subscribed(dir.path());

    drop(adapter);

    // The thread held the only other reference to the listener; `drop` returned after joining it.
    assert_eq!(Arc::strong_count(&recording), 1);
    write_token(&dir.path().join("0803"), &token_bytes(FIRST));
    thread::sleep(QUIET);
    assert!(recording.announced().is_empty(), "{:?}", recording.announced());
}
