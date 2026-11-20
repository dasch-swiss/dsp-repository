//! One nightly run over a fake `Vre` and a fake clock: once per local date, its order, the busy
//! projects' retries, removal, and one project's failure leaving the others.

use std::process::ExitCode;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use vre_bridge_core::{FailureReason, Outcome};

use crate::clock::FakeClock;
use crate::refresh_tests::public_dump;
use crate::test_support::{lines_of, Dirs, FakeVre, Plays, Recording};
use crate::{VreBridge, EXIT_LOCKED};

const NOW: &str = "2026-11-20T01:00:00Z";
const OLD: &str = "20261119T010000Z-0000000a";
const OLDER: &str = "20261118T010000Z-0000000b";

fn utc(rfc3339: &str) -> DateTime<Utc> {
    rfc3339.parse().unwrap()
}

fn minutes(n: i64) -> DateTime<Utc> {
    utc(NOW) + TimeDelta::minutes(n)
}

/// The local date of [`NOW`] in Zurich.
fn today() -> NaiveDate {
    "2026-11-20".parse().unwrap()
}

/// A bridge over `dirs` with `allowlist`, a free-running clock at [`NOW`] and the fake VRE
/// `vre(clock)` builds.
fn bridge(
    dirs: &Dirs,
    allowlist: &str,
    vre: impl FnOnce(Arc<FakeClock>) -> FakeVre,
) -> (VreBridge, Arc<FakeVre>, Arc<FakeClock>, Arc<Recording>) {
    let clock = Arc::new(FakeClock::at(NOW));
    let vre = Arc::new(vre(Arc::clone(&clock)));
    let recording = Arc::new(Recording::default());
    let bridge = dirs.bridge(dirs.settings(allowlist), vre.clone(), recording.clone(), clock.clone());
    (bridge, vre, clock, recording)
}

#[tokio::test]
async fn test_two_runs_on_one_local_date_make_one_run() {
    let dirs = Dirs::new();
    let (bridge, vre, _, recording) = bridge(&dirs, "0803=prod", FakeVre::new);

    bridge.active().nightly_run(today()).await;
    bridge.active().nightly_run(today()).await;

    assert_eq!(vre.calls_of("lookup").len(), 1);
    assert_eq!(recording.outcomes(), [("0803".to_owned(), "failed")]);
}

#[tokio::test]
async fn test_the_run_sweeps_recovers_and_removes_before_the_first_project() {
    let dirs = Dirs::new();
    // A dump directory a killed process left.
    let leftover = dirs.work.path().join("vre-bridge-dump-x");
    std::fs::create_dir(&leftover).unwrap();
    std::fs::write(leftover.join("dump.zip"), "admin.nq").unwrap();
    // 0804's tokens diverge, as a refresh cut short between its two renames leaves them.
    dirs.seed(dirs.sync.path(), "0804", OLD);
    dirs.seed(dirs.media.path(), "0804", OLDER);
    // 0A01 left the allowlist with tokens, 0A02 without.
    dirs.seed(dirs.sync.path(), "0A01", OLD);
    dirs.seed(dirs.media.path(), "0A01", OLD);
    std::fs::create_dir_all(dirs.sync.path().join("0A02").join(OLD)).unwrap();

    let check = {
        let (sync, media) = (dirs.sync.path().to_owned(), dirs.media.path().to_owned());
        let leftover = leftover.clone();
        move || {
            assert!(!leftover.exists(), "not swept");
            let token = |root: &std::path::Path, sc: &str| std::fs::read(root.join(sc).join(".ready")).ok();
            assert_eq!(token(&media, "0804"), token(&sync, "0804"), "not recovered");
            assert!(
                token(&sync, "0A01").is_none() && token(&media, "0A01").is_none(),
                "not withdrawn"
            );
            assert!(!sync.join("0A02").exists(), "not deleted");
        }
    };
    let (bridge, vre, _, recording) = bridge(&dirs, "0803=prod,0804=prod", |clock| {
        FakeVre::new(clock).plays("0803", Plays::Checks(Box::new(check)))
    });

    bridge.active().nightly_run(today()).await;

    assert_eq!(vre.calls_of("lookup")[0].0, "0803");
    assert_eq!(
        recording.outcomes(),
        [
            ("0A01".to_owned(), "removed"),
            ("0803".to_owned(), "failed"),
            ("0804".to_owned(), "failed"),
        ]
    );
}

#[tokio::test]
async fn test_a_busy_project_is_retried_after_the_wait_then_failed() {
    let dirs = Dirs::new();
    let (bridge, vre, clock, recording) =
        bridge(&dirs, "0803=prod", |clock| FakeVre::new(clock).plays("0803", Plays::Busy));

    bridge.active().nightly_run(today()).await;

    // The defaults: 3 retries, 20 minutes apart.
    let exports: Vec<_> = vre.calls_of("export").into_iter().map(|(_, at)| at).collect();
    assert_eq!(exports, [minutes(0), minutes(20), minutes(40), minutes(60)]);
    assert_eq!(clock.slept(), [minutes(20), minutes(40), minutes(60)]);
    let reports = recording.reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Failed(FailureReason::Busy));
}

#[tokio::test]
async fn test_the_retry_queue_sleeps_to_the_earliest_and_retries_only_the_projects_due() {
    let dirs = Dirs::new();
    let mut settings = dirs.settings("0803=prod,0804=prod");
    settings.dump_retries = 2;
    let clock = Arc::new(FakeClock::at(NOW));
    // Each lookup of 0804 takes 10 minutes, so the two projects fall due at different times.
    let vre = Arc::new(
        FakeVre::new(Arc::clone(&clock))
            .plays("0803", Plays::Busy)
            .plays("0804", Plays::Busy)
            .lookup_takes("0804", TimeDelta::minutes(10)),
    );
    let recording = Arc::new(Recording::default());
    let bridge = dirs.bridge(settings, vre.clone(), recording.clone(), clock.clone());

    bridge.active().nightly_run(today()).await;

    let exports = vre.calls_of("export");
    let expected = [
        ("0803", minutes(0)),
        ("0804", minutes(10)),
        ("0803", minutes(20)),
        ("0804", minutes(40)),
        ("0803", minutes(40)),
        ("0804", minutes(70)),
    ];
    assert_eq!(exports, expected.map(|(sc, at)| (sc.to_owned(), at)));
    // At 40 minutes 0803 is due without a sleep: 0804's lookup moved the clock there.
    assert_eq!(clock.slept(), [minutes(20), minutes(30), minutes(60)]);
    assert_eq!(
        recording.outcomes(),
        [("0803".to_owned(), "failed"), ("0804".to_owned(), "failed")]
    );
}

#[tokio::test]
async fn test_an_operator_refresh_between_retries_finds_the_lock_held() {
    let dirs = Dirs::new();
    let mut settings = dirs.settings("0803=prod");
    settings.dump_retries = 1;
    let (clock, mut sleeps) = FakeClock::gated(NOW);
    let clock = Arc::new(clock);
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)).plays("0803", Plays::Busy));
    let recording = Arc::new(Recording::default());
    let bridge = Arc::new(dirs.bridge(settings, vre.clone(), recording.clone(), clock.clone()));
    let run = tokio::spawn({
        let bridge = Arc::clone(&bridge);
        async move { bridge.active().nightly_run(today()).await }
    });

    assert_eq!(sleeps.recv().await, Some(minutes(20)));
    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(EXIT_LOCKED));
    clock.release(1);
    run.await.unwrap();

    assert_eq!(vre.calls_of("export").len(), 2);
    assert_eq!(recording.outcomes(), [("0803".to_owned(), "failed")]);
}

#[tokio::test]
async fn test_one_project_failing_leaves_the_next_one_refreshed() {
    let dirs = Dirs::new();
    let (bridge, _, _, recording) = bridge(&dirs, "0802=prod,0803=prod", |clock| {
        FakeVre::new(clock)
            .plays("0802", Plays::Fails)
            .plays("0803", Plays::Dump(public_dump()))
    });

    bridge.active().nightly_run(today()).await;

    assert_eq!(
        recording.outcomes(),
        [("0802".to_owned(), "failed"), ("0803".to_owned(), "refreshed")]
    );
    let (sync, media) = dirs.token_bytes("0803");
    assert!(sync.is_some() && sync == media);
}

#[tokio::test]
async fn test_a_panicking_refresh_fails_its_project_and_the_next_one_is_refreshed() {
    let dirs = Dirs::new();
    let (bridge, _, _, recording) = bridge(&dirs, "0802=prod,0803=prod", |clock| {
        FakeVre::new(clock)
            .plays("0802", Plays::Panics)
            .plays("0803", Plays::Dump(public_dump()))
    });

    bridge.active().nightly_run(today()).await;

    let reports = recording.reports();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].outcome, Outcome::Failed(FailureReason::Internal));
    assert_eq!((reports[1].project.as_str(), reports[1].outcome), ("0803", Outcome::Refreshed));
}

#[tokio::test]
async fn test_a_removal_is_reported_as_removed_and_logged_at_warning_level() {
    let dirs = Dirs::new();
    dirs.seed(dirs.sync.path(), "0A11", OLD);
    dirs.seed(dirs.media.path(), "0A11", OLD);
    let (bridge, vre, _, recording) = bridge(&dirs, "0803=prod", FakeVre::new);

    bridge.active().nightly_run(today()).await;

    let removed: Vec<_> = recording.reports().into_iter().filter(|r| r.project == "0A11").collect();
    assert_eq!(removed.len(), 1);
    assert_eq!((removed[0].outcome, removed[0].server.as_ref()), (Outcome::Removed, None));
    assert_eq!(dirs.token_bytes("0A11"), (None, None));
    assert!(dirs.sync.path().join("0A11").join(OLD).exists(), "the data goes in a later run");
    let lines = lines_of("0A11");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("WARN") && l.contains("withdrawn") && l.contains("outcome=\"removed\"")),
        "{lines:#?}"
    );
    // A shortcode not on the allowlist is never asked for.
    assert!(vre.calls().iter().all(|call| call.project == "0803"));
}

#[tokio::test]
async fn test_a_failed_removal_is_reported_as_failed_with_no_server() {
    use std::os::unix::fs::PermissionsExt as _;
    let dirs = Dirs::new();
    dirs.seed(dirs.sync.path(), "0A11", OLD);
    dirs.seed(dirs.media.path(), "0A11", OLD);
    let project = dirs.sync.path().join("0A11");
    std::fs::set_permissions(&project, std::fs::Permissions::from_mode(0o555)).unwrap();
    let (bridge, _, _, recording) = bridge(&dirs, "0803=prod", FakeVre::new);

    bridge.active().nightly_run(today()).await;

    std::fs::set_permissions(&project, std::fs::Permissions::from_mode(0o755)).unwrap();
    let removed: Vec<_> = recording.reports().into_iter().filter(|r| r.project == "0A11").collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(
        (removed[0].outcome, removed[0].server.as_ref()),
        (Outcome::Failed(FailureReason::Store), None)
    );
    assert!(dirs.token_bytes("0A11").0.is_some(), "the token stays");
}

#[tokio::test]
async fn test_an_empty_allowlist_removes_nothing() {
    let dirs = Dirs::new();
    dirs.seed(dirs.sync.path(), "0A12", OLD);
    dirs.seed(dirs.media.path(), "0A12", OLD);
    let before = dirs.token_bytes("0A12");
    let (bridge, vre, _, recording) = bridge(&dirs, "", FakeVre::new);

    bridge.active().nightly_run(today()).await;

    assert_eq!(dirs.token_bytes("0A12"), before);
    assert!(vre.calls().is_empty());
    assert!(recording.reports().is_empty());
}
