//! `VreBridge`: what makes it inactive, its start-time work, and the operator's refresh.

use std::process::ExitCode;
use std::sync::Arc;

use vre_bridge_core::{FailureReason, Outcome, RawSettings};
use vre_bridge_store::Lock;

use crate::clock::FakeClock;
use crate::refresh_tests::public_dump;
use crate::test_support::{display, temp, Dirs, FakeVre, Plays, Recording, KEY};
use crate::{Inactive, VreBridge, EXIT_LOCKED, EXIT_UNUSABLE};

const NOW: &str = "2026-11-20T12:00:00Z";
const OLD: &str = "20261119T010000Z-0000000a";
const OLDER: &str = "20261118T010000Z-0000000b";

fn reason(bridge: &VreBridge) -> String {
    match bridge.inactive_reason() {
        Some(Inactive::Invalid(reason)) => reason.clone(),
        other => panic!("not inactive for an invalid setting: {other:?}"),
    }
}

// ---------------------------------------------------------------- from_raw

#[test]
fn test_from_raw_with_valid_settings_is_active() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dirs = Dirs::new();
    assert!(VreBridge::from_raw(dirs.raw("0803=prod")).is_active());
}

#[tokio::test(flavor = "current_thread")]
async fn test_from_raw_without_output_locations_is_inactive_and_start_spawns_nothing() {
    let dirs = Dirs::new();
    let leftover = dirs.work.path().join("vre-bridge-dump-x");
    std::fs::create_dir(&leftover).unwrap();
    let raw = RawSettings { sync_dir: None, media_dir: None, ..dirs.raw("0803=prod") };

    let bridge = VreBridge::from_raw(raw);
    bridge.start();
    tokio::task::yield_now().await;

    assert_eq!(bridge.inactive_reason(), Some(&Inactive::NotConfigured));
    assert_eq!(tokio::runtime::Handle::current().metrics().num_alive_tasks(), 0);
    assert!(leftover.exists());
    assert!(!dirs.lock_file().exists());
}

#[test]
fn test_from_raw_with_the_work_dir_inside_an_output_location_is_inactive() {
    let dirs = Dirs::new();
    let inside = dirs.sync.path().join("work");
    std::fs::create_dir(&inside).unwrap();
    let raw = RawSettings { work_dir: Some(display(&inside)), ..dirs.raw("0803=prod") };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(reason.contains("EDITOR_VRE_BRIDGE_WORK_DIR"), "{reason}");
    assert!(reason.contains("EDITOR_VRE_BRIDGE_SYNC_DIR"), "{reason}");
}

#[test]
fn test_from_raw_with_the_output_location_as_work_dir_is_inactive() {
    let dirs = Dirs::new();
    let raw = RawSettings {
        work_dir: Some(display(dirs.media.path())),
        ..dirs.raw("0803=prod")
    };

    assert!(reason(&VreBridge::from_raw(raw)).contains("EDITOR_VRE_BRIDGE_WORK_DIR"));
}

#[test]
fn test_from_raw_with_a_symlinked_work_dir_is_inactive() {
    let dirs = Dirs::new();
    let link = dirs.lock_dir.path().join("work");
    std::os::unix::fs::symlink(dirs.work.path(), &link).unwrap();
    let raw = RawSettings { work_dir: Some(display(&link)), ..dirs.raw("0803=prod") };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_WORK_DIR") && reason.contains("symlink"),
        "{reason}"
    );
}

#[test]
fn test_from_raw_with_a_dangling_lock_file_symlink_is_inactive() {
    let dirs = Dirs::new();
    let link = dirs.lock_dir.path().join("lock-link");
    std::os::unix::fs::symlink(dirs.sync.path().join("lock"), &link).unwrap();
    let raw = RawSettings { lock_file: Some(display(&link)), ..dirs.raw("0803=prod") };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_LOCK_FILE") && reason.contains("symlink"),
        "{reason}"
    );
}

#[test]
fn test_from_raw_with_an_output_location_the_sweep_would_delete_is_inactive() {
    let dirs = Dirs::new();
    let swept = dirs.work.path().join("vre-bridge-dump-sync");
    std::fs::create_dir(&swept).unwrap();
    let raw = RawSettings { sync_dir: Some(display(&swept)), ..dirs.raw("0803=prod") };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_SYNC_DIR") && reason.contains("sweep"),
        "{reason}"
    );
}

#[test]
fn test_from_raw_with_the_lock_file_where_the_sweep_would_delete_it_is_inactive() {
    let dirs = Dirs::new();
    let swept = dirs.work.path().join("vre-bridge-dump-lock");
    std::fs::create_dir(&swept).unwrap();
    let raw = RawSettings {
        lock_file: Some(display(&swept.join("lock"))),
        ..dirs.raw("0803=prod")
    };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_LOCK_FILE") && reason.contains("sweep"),
        "{reason}"
    );
}

#[test]
fn test_from_raw_with_the_lock_file_inside_an_output_location_is_inactive() {
    let dirs = Dirs::new();
    let lock = dirs.media.path().join("lock");
    let raw = RawSettings { lock_file: Some(display(&lock)), ..dirs.raw("0803=prod") };

    let reason = reason(&VreBridge::from_raw(raw));

    assert!(reason.contains("EDITOR_VRE_BRIDGE_LOCK_FILE"), "{reason}");
    assert!(reason.contains("EDITOR_VRE_BRIDGE_MEDIA_DIR"), "{reason}");
}

#[test]
fn test_from_raw_with_an_invalid_time_zone_is_inactive() {
    let dirs = Dirs::new();
    let raw = RawSettings {
        time_zone: Some("Europe/Atlantis".to_owned()),
        ..dirs.raw("0803=prod")
    };

    assert!(reason(&VreBridge::from_raw(raw)).contains("EDITOR_VRE_BRIDGE_TIME_ZONE"));
}

/// `dirs.raw("0803=prod")` with the key file of server `prod` at `path`.
fn with_key_file(dirs: &Dirs, path: &std::path::Path) -> RawSettings {
    RawSettings {
        signing_key_files: Some(format!("prod={}", display(path)).into()),
        ..dirs.raw("0803=prod")
    }
}

#[test]
fn test_from_raw_with_a_key_file_that_holds_no_key_that_signs_is_inactive_and_not_quoted() {
    let dirs = Dirs::new();
    // PEM whose base64 decodes, but to no PKCS#8 key.
    let body = "bm90LWEta2V5LWF0LWFsbA==";
    let file = dirs.lock_dir.path().join("not-a-key.pem");
    std::fs::write(
        &file,
        format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n"),
    )
    .unwrap();

    let reason = reason(&VreBridge::from_raw(with_key_file(&dirs, &file)));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES") && reason.contains("prod"),
        "{reason}"
    );
    let test_key = KEY.lines().nth(1).unwrap();
    assert!(!reason.contains(body) && !reason.contains(test_key), "{reason}");
}

#[test]
fn test_from_raw_with_a_missing_key_file_is_inactive() {
    let dirs = Dirs::new();
    let missing = dirs.lock_dir.path().join("missing.pem");

    let reason = reason(&VreBridge::from_raw(with_key_file(&dirs, &missing)));

    assert!(
        reason.contains("EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES") && reason.contains("`prod`"),
        "{reason}"
    );
}

#[test]
fn test_from_raw_with_the_key_file_inside_an_output_location_is_inactive_and_not_quoted() {
    let dirs = Dirs::new();
    let inside = dirs.media.path().join("key.pem");
    std::fs::write(&inside, KEY).unwrap();

    let reason = reason(&VreBridge::from_raw(with_key_file(&dirs, &inside)));

    assert!(reason.contains("EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES"), "{reason}");
    assert!(reason.contains("EDITOR_VRE_BRIDGE_MEDIA_DIR"), "{reason}");
    assert!(!reason.contains(KEY.lines().nth(1).unwrap()), "{reason}");
}

#[test]
fn test_from_raw_with_a_symlinked_key_file_reads_its_target() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dirs = Dirs::new();
    let link = dirs.work.path().join("key-link.pem");
    std::os::unix::fs::symlink(dirs.key_file(), &link).unwrap();

    assert!(VreBridge::from_raw(with_key_file(&dirs, &link)).is_active());
}

// ---------------------------------------------------------------- start

#[tokio::test(flavor = "current_thread")]
async fn test_start_recovers_diverging_tokens_and_sweeps_the_work_dir() {
    let dirs = Dirs::new();
    dirs.seed(dirs.sync.path(), "0803", OLD);
    dirs.seed(dirs.media.path(), "0803", OLDER);
    let leftover = dirs.work.path().join("vre-bridge-dump-x");
    std::fs::create_dir(&leftover).unwrap();
    let (clock, mut sleeps) = FakeClock::gated(NOW);
    let clock = Arc::new(clock);
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)));
    let bridge = dirs.bridge(dirs.settings("0803=prod"), vre.clone(), Arc::new(Recording::default()), clock);

    bridge.start();
    // The scheduler's first sleep comes after the start-time work.
    sleeps.recv().await.unwrap();

    let (sync, media) = dirs.token_bytes("0803");
    assert_eq!(media, sync);
    assert!(!leftover.exists());
    assert!(vre.calls().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn test_start_with_the_lock_held_elsewhere_neither_sweeps_nor_recovers() {
    let dirs = Dirs::new();
    dirs.seed(dirs.sync.path(), "0803", OLD);
    dirs.seed(dirs.media.path(), "0803", OLDER);
    let before = dirs.token_bytes("0803");
    let leftover = dirs.work.path().join("vre-bridge-dump-x");
    std::fs::create_dir(&leftover).unwrap();
    let held = Lock::try_acquire(&dirs.lock_file()).unwrap().unwrap();
    let (clock, mut sleeps) = FakeClock::gated(NOW);
    let clock = Arc::new(clock);
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)));
    let bridge = dirs.bridge(dirs.settings("0803=prod"), vre, Arc::new(Recording::default()), clock);

    bridge.start();
    sleeps.recv().await.unwrap();

    assert_eq!(dirs.token_bytes("0803"), before);
    assert!(leftover.exists());
    drop(held);
}

// ---------------------------------------------------------------- the operator's refresh

fn operator_bridge(dirs: &Dirs, plays: Plays) -> (VreBridge, Arc<FakeVre>, Arc<Recording>) {
    let clock = Arc::new(FakeClock::at(NOW));
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)).plays("0803", plays));
    let recording = Arc::new(Recording::default());
    let bridge = dirs.bridge(dirs.settings("0803=prod"), vre.clone(), recording.clone(), clock);
    (bridge, vre, recording)
}

#[tokio::test]
async fn test_operator_refresh_of_a_changed_project_exits_0() {
    let dirs = Dirs::new();
    let (bridge, _, recording) = operator_bridge(&dirs, Plays::Dump(public_dump()));

    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(0));
    assert_eq!(recording.outcomes(), [("0803".to_owned(), "refreshed")]);
}

#[tokio::test]
async fn test_operator_refresh_of_a_busy_dump_slot_fails_at_once() {
    let dirs = Dirs::new();
    let (bridge, vre, recording) = operator_bridge(&dirs, Plays::Busy);

    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(1));
    assert_eq!(vre.calls_of("export").len(), 1);
    let reports = recording.reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Failed(FailureReason::Busy));
}

#[tokio::test]
async fn test_operator_refresh_of_a_project_not_allowlisted_exits_64() {
    let dirs = Dirs::new();
    let (bridge, vre, _) = operator_bridge(&dirs, Plays::Busy);

    assert_eq!(bridge.operator_refresh("0804").await, ExitCode::from(EXIT_UNUSABLE));
    assert!(vre.calls().is_empty());
    assert!(!dirs.lock_file().exists());
}

#[tokio::test]
async fn test_operator_refresh_of_an_inactive_bridge_exits_64() {
    let bridge = VreBridge::from_raw(RawSettings {
        inputs_dir: Some(display(temp().path())),
        ..RawSettings::default()
    });

    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(EXIT_UNUSABLE));
}

#[tokio::test]
async fn test_operator_refresh_with_the_lock_held_exits_75() {
    let dirs = Dirs::new();
    let (bridge, vre, recording) = operator_bridge(&dirs, Plays::Busy);
    let _held = Lock::try_acquire(&dirs.lock_file()).unwrap().unwrap();

    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(EXIT_LOCKED));
    assert!(vre.calls().is_empty());
    assert!(recording.reports().is_empty());
}
