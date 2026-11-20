use std::fs;

use chrono::NaiveDate;
use tempfile::TempDir;

use crate::{is_swept, sweep_work_dir, Lock, StoreError};

#[test]
fn test_is_swept_names_what_the_sweep_deletes() {
    let work = std::path::Path::new("/work");
    for (path, swept) in [
        ("/work/vre-bridge-dump-x", true),
        ("/work/vre-bridge-dump-x/sync", true),
        ("/work/other/vre-bridge-dump-x", false),
        ("/work", false),
        ("/elsewhere/vre-bridge-dump-x", false),
    ] {
        assert_eq!(is_swept(work, "vre-bridge-dump-", std::path::Path::new(path)), swept, "{path}");
    }
    assert!(!is_swept(work, "", std::path::Path::new("/work/anything")));
}

#[test]
fn test_try_acquire_held_by_another_handle_returns_none_until_dropped() {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join("vre-bridge.lock");

    let held = Lock::try_acquire(&path).expect("open").expect("free");
    assert!(Lock::try_acquire(&path).expect("open").is_none());
    drop(held);
    assert!(Lock::try_acquire(&path).expect("open").is_some());
}

#[test]
fn test_nightly_record_round_trips_a_date() {
    let dir = TempDir::new().expect("a temporary directory");
    let lock = Lock::try_acquire(&dir.path().join("vre-bridge.lock"))
        .expect("open")
        .expect("free");
    let night = NaiveDate::from_ymd_opt(2026, 11, 20).expect("a date");
    let next = NaiveDate::from_ymd_opt(2026, 11, 21).expect("a date");

    assert!(!lock.nightly_done_on(night), "no record yet");
    lock.record_nightly(night).expect("record");
    assert!(lock.nightly_done_on(night));
    assert!(!lock.nightly_done_on(next));
    lock.record_nightly(next).expect("record");
    assert!(lock.nightly_done_on(next));
    assert_eq!(
        fs::read_to_string(dir.path().join("vre-bridge.lock.nightly")).expect("the record"),
        "2026-11-21\n"
    );
}

#[test]
fn test_sweep_work_dir_deletes_prefixed_entries_and_unlinks_symlinks_without_following() {
    let dir = TempDir::new().expect("a temporary directory");
    let work = dir.path().join("work");
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir_all(work.join("vre-bridge-dump-x/data")).expect("create");
    fs::write(work.join("vre-bridge-dump-x/data/admin.nq"), b"x").expect("write");
    fs::create_dir(work.join("other")).expect("create");
    fs::create_dir(&elsewhere).expect("create");
    fs::write(elsewhere.join("keep"), b"k").expect("write");
    std::os::unix::fs::symlink(&elsewhere, work.join("vre-bridge-dump-y")).expect("symlink");

    let deleted = sweep_work_dir(&work, "vre-bridge-dump-").expect("sweep");

    assert_eq!(deleted, 2);
    assert!(fs::symlink_metadata(work.join("vre-bridge-dump-x")).is_err());
    assert!(fs::symlink_metadata(work.join("vre-bridge-dump-y")).is_err());
    assert!(work.join("other").is_dir());
    assert!(elsewhere.join("keep").is_file(), "the symlink's target is untouched");
}

#[test]
fn test_sweep_work_dir_entry_that_cannot_be_deleted_does_not_stop_the_sweep() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("a temporary directory");
    let work = dir.path().join("work");
    let locked = work.join("vre-bridge-dump-a/locked");
    fs::create_dir_all(&locked).expect("create");
    fs::write(locked.join("admin.nq"), b"x").expect("write");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).expect("chmod");
    if fs::write(locked.join("probe"), b"x").is_ok() {
        // Running as root, which a mode does not stop.
        return;
    }
    // Many others, so that some come after the failing one in whatever order the listing has.
    for i in 0..20 {
        fs::create_dir(work.join(format!("vre-bridge-dump-{i:02}"))).expect("create");
    }

    let result = sweep_work_dir(&work, "vre-bridge-dump-");

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).expect("chmod back");
    assert!(matches!(result, Err(StoreError::Io { .. })), "{result:?}");
    for i in 0..20 {
        assert!(!work.join(format!("vre-bridge-dump-{i:02}")).exists(), "{i}");
    }
}

#[test]
fn test_sweep_work_dir_empty_prefix_refused() {
    let dir = TempDir::new().expect("a temporary directory");
    assert!(matches!(sweep_work_dir(dir.path(), ""), Err(StoreError::Internal(_))));
}
