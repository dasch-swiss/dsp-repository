//! When the nightly runs happen: the next `EDITOR_VRE_BRIDGE_NIGHTLY_AT` in
//! `EDITOR_VRE_BRIDGE_TIME_ZONE`, always in the future, so a missed run is never caught up.

use std::sync::Arc;

use chrono::{DateTime, LocalResult, NaiveDateTime, NaiveTime, TimeDelta, TimeZone as _, Utc};
use chrono_tz::Tz;
use tracing::{error, info};

use crate::bridge::Active;
use crate::withhold::withhold;

/// How far a local time in a daylight-saving gap is moved forward per step until it exists.
const GAP_STEP: TimeDelta = TimeDelta::minutes(15);

/// More steps than any gap of a real time zone needs.
const GAP_STEPS: usize = 24 * 4;

/// The first nightly run strictly after `now`: `at` on the local date of `now` or a later one. A
/// local time that occurs twice takes its first occurrence; one that does not occur is moved
/// forward by [`GAP_STEP`] until it does. `None` only past chrono's range.
pub(crate) fn next_run(now: DateTime<Utc>, at: NaiveTime, time_zone: Tz) -> Option<DateTime<Utc>> {
    let today = now.with_timezone(&time_zone).date_naive();
    today
        .iter_days()
        .take(3)
        .filter_map(|day| instant(day.and_time(at), time_zone))
        .find(|t| *t > now)
}

fn instant(mut local: NaiveDateTime, time_zone: Tz) -> Option<DateTime<Utc>> {
    for _ in 0..=GAP_STEPS {
        match time_zone.from_local_datetime(&local) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => return Some(t.with_timezone(&Utc)),
            LocalResult::None => local = local.checked_add_signed(GAP_STEP)?,
        }
    }
    None
}

/// Runs the nightly runs forever, each as its own task awaited to its end: a panicking run is
/// logged and the next night still runs.
pub(crate) async fn run(active: Arc<Active>) {
    loop {
        let Some(next) = next_run(active.clock.now(), active.settings.nightly_at, active.time_zone) else {
            error!("no next nightly run can be computed; the VRE bridge schedules no more runs");
            return;
        };
        info!(at = %next.to_rfc3339(), "the next nightly run is scheduled");
        active.sleep_until(next).await;
        // The scheduled date, not the date the run gets the lock on: a run that waits past
        // midnight still records the night it belongs to.
        let date = next.with_timezone(&active.time_zone).date_naive();
        let pass = tokio::spawn({
            let active = Arc::clone(&active);
            withhold(async move { active.nightly_run(date).await })
        });
        if let Err(join) = pass.await {
            error!(
                date = %date,
                panicked = join.is_panic(),
                "the nightly run did not finish; the next one is scheduled as usual"
            );
        }
    }
}
