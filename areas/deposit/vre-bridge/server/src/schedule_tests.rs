//! The nightly run's schedule: the next run in Europe/Zurich across daylight-saving changes, and
//! the scheduler over a gated fake clock.

use std::sync::Arc;

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use chrono_tz::Tz;
use tokio::sync::mpsc::UnboundedReceiver;
use vre_bridge_store::Lock;

use crate::clock::FakeClock;
use crate::schedule::next_run;
use crate::test_support::{Dirs, FakeVre, Plays, Recording};

fn utc(rfc3339: &str) -> DateTime<Utc> {
    rfc3339.parse().unwrap()
}

fn hh_mm(text: &str) -> NaiveTime {
    NaiveTime::parse_from_str(text, "%H:%M").unwrap()
}

const ZURICH: Tz = chrono_tz::Europe::Zurich;

#[test]
fn test_next_run_at_03_00_is_tomorrow_02_00() {
    // 03:00 CET.
    let next = next_run(utc("2026-11-20T02:00:00Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(next, Some(utc("2026-11-21T01:00:00Z")));
}

#[test]
fn test_next_run_before_its_time_is_today() {
    let next = next_run(utc("2026-11-20T00:59:59Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(next, Some(utc("2026-11-20T01:00:00Z")));
}

#[test]
fn test_next_run_at_its_own_instant_is_the_next_day() {
    let next = next_run(utc("2026-11-20T01:00:00Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(next, Some(utc("2026-11-21T01:00:00Z")));
}

/// On 2027-03-28 Zurich's clocks go from 02:00 CET to 03:00 CEST: 02:00 to 02:45 do not occur,
/// so a run time among them moves forward in steps of 15 minutes until it occurs.
#[test]
fn test_next_run_in_the_spring_gap_steps_forward_to_03_00_cest() {
    let next = next_run(utc("2027-03-27T12:00:00Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(next, Some(utc("2027-03-28T01:00:00Z")));
    // 02:55 does not occur either; 03:10 CEST does.
    let next = next_run(utc("2027-03-27T12:00:00Z"), hh_mm("02:40"), ZURICH);
    assert_eq!(next, Some(utc("2027-03-28T01:10:00Z")));
    let after = next_run(utc("2027-03-28T01:00:00Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(after, Some(utc("2027-03-29T00:00:00Z")));
}

/// On 2027-10-31 Zurich's clocks go from 03:00 CEST back to 02:00 CET: 02:00 to 02:59 occur
/// twice, and the run takes the first.
#[test]
fn test_next_run_in_the_autumn_overlap_takes_the_first_occurrence() {
    let next = next_run(utc("2027-10-30T12:00:00Z"), hh_mm("02:00"), ZURICH);
    assert_eq!(next, Some(utc("2027-10-31T00:00:00Z")));
    let next = next_run(utc("2027-10-30T12:00:00Z"), hh_mm("02:30"), ZURICH);
    assert_eq!(next, Some(utc("2027-10-31T00:30:00Z")));
    // The second 02:30 is past the first, so the next run is the next day's.
    let next = next_run(utc("2027-10-31T01:30:00Z"), hh_mm("02:30"), ZURICH);
    assert_eq!(next, Some(utc("2027-11-01T01:30:00Z")));
}

// ---------------------------------------------------------------- the scheduler

/// A started bridge over a gated clock at `now`, with an allowlist of one project whose lookup
/// does `plays`.
fn started(
    dirs: &Dirs,
    now: &str,
    nightly_at: &str,
    plays: Plays,
) -> (Arc<FakeClock>, UnboundedReceiver<DateTime<Utc>>, Arc<FakeVre>) {
    let (clock, sleeps) = FakeClock::gated(now);
    let clock = Arc::new(clock);
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)).plays("0803", plays));
    let mut settings = dirs.settings("0803=prod");
    settings.nightly_at = hh_mm(nightly_at);
    let bridge = dirs.bridge(settings, vre.clone(), Arc::new(Recording::default()), clock.clone());
    bridge.start();
    (clock, sleeps, vre)
}

#[tokio::test(flavor = "current_thread")]
async fn test_start_at_03_00_schedules_tomorrow_02_00_not_now() {
    let dirs = Dirs::new();
    let (_clock, mut sleeps, vre) = started(&dirs, "2026-11-20T02:00:00Z", "02:00", Plays::Fails);

    assert_eq!(sleeps.recv().await, Some(utc("2026-11-21T01:00:00Z")));
    assert!(vre.calls().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn test_restart_at_the_second_02_30_with_the_record_of_that_date_schedules_the_next_day() {
    let dirs = Dirs::new();
    let date = "2027-10-31".parse().unwrap();
    let lock = Lock::try_acquire(&dirs.lock_file()).unwrap().unwrap();
    lock.record_nightly(date).unwrap();
    drop(lock);

    // The second 02:30 of 2027-10-31, in CET.
    let (_clock, mut sleeps, vre) = started(&dirs, "2027-10-31T01:30:00Z", "02:30", Plays::Fails);
    assert_eq!(sleeps.recv().await, Some(utc("2027-11-01T01:30:00Z")));
    assert!(vre.calls().is_empty());

    // A run of that date started then does nothing either.
    let clock = Arc::new(FakeClock::at("2027-10-31T01:30:00Z"));
    let vre = Arc::new(FakeVre::new(Arc::clone(&clock)).plays("0803", Plays::Fails));
    let bridge = dirs.bridge(dirs.settings("0803=prod"), vre.clone(), Arc::new(Recording::default()), clock);
    bridge.active().nightly_run(date).await;
    assert!(vre.calls().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn test_a_run_that_gets_the_lock_after_midnight_records_its_scheduled_date() {
    let dirs = Dirs::new();
    // 23:00 CET; the run is at 23:59.
    let (clock, mut sleeps, vre) = started(&dirs, "2026-11-20T22:00:00Z", "23:59", Plays::Fails);

    assert_eq!(sleeps.recv().await, Some(utc("2026-11-20T22:59:00Z")));
    // The run starts at 01:30 CET the next day (00:30Z), as after waiting for the lock.
    clock.advance(TimeDelta::minutes(150));
    clock.release(1);
    assert_eq!(sleeps.recv().await, Some(utc("2026-11-21T22:59:00Z")));
    clock.release(1);
    assert_eq!(sleeps.recv().await, Some(utc("2026-11-22T22:59:00Z")));

    // Both nights ran: the first did not take the second's date.
    assert_eq!(vre.calls_of("lookup").len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn test_a_night_whose_refresh_panics_is_followed_by_the_next_night() {
    let dirs = Dirs::new();
    let (clock, mut sleeps, vre) = started(&dirs, "2026-11-20T12:00:00Z", "02:00", Plays::Panics);

    assert_eq!(sleeps.recv().await, Some(utc("2026-11-21T01:00:00Z")));
    clock.release(1);
    assert_eq!(sleeps.recv().await, Some(utc("2026-11-22T01:00:00Z")));
    clock.release(1);
    assert_eq!(sleeps.recv().await, Some(utc("2026-11-23T01:00:00Z")));

    assert_eq!(vre.calls_of("lookup").len(), 2);
}
