//! Wall-clock time, injected so that tests set it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The bridge's wall clock.
#[async_trait]
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;

    /// Returns once `t` is reached, at once if it has passed. A monotonic sleep drifts across a
    /// host suspend or a clock step, so a caller checks `now() >= t` again after it returns.
    async fn sleep_until(&self, t: DateTime<Utc>);
}

/// The system's clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

#[async_trait]
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    async fn sleep_until(&self, t: DateTime<Utc>) {
        if let Ok(wait) = (t - Utc::now()).to_std() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// A clock that stands still until a sleep moves it to the sleep's end.
#[cfg(test)]
pub(crate) struct FakeClock(std::sync::Mutex<DateTime<Utc>>);

#[cfg(test)]
impl FakeClock {
    pub(crate) fn at(rfc3339: &str) -> Self {
        Self(std::sync::Mutex::new(rfc3339.parse().unwrap()))
    }
}

#[cfg(test)]
#[async_trait]
impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }

    async fn sleep_until(&self, t: DateTime<Utc>) {
        let mut now = self.0.lock().unwrap();
        *now = (*now).max(t);
    }
}
