//! Wall-clock time, injected so that tests set it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The bridge's wall clock.
#[async_trait]
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;

    /// Waits towards `t`, returning at once if it has passed. It may return before `t` (a
    /// monotonic sleep drifts across a host suspend or a clock step), so a caller sleeps again
    /// until `now() >= t`.
    async fn sleep_until(&self, t: DateTime<Utc>);
}

/// The longest single sleep of [`SystemClock`]; a caller loops until `now() >= t`.
const SLEEP_SLICE: std::time::Duration = std::time::Duration::from_secs(60);

/// The system's clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

#[async_trait]
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    /// Sleeps at most [`SLEEP_SLICE`] at a time, so a wake after a host suspend or a clock step
    /// is late by no more than that.
    async fn sleep_until(&self, t: DateTime<Utc>) {
        if let Ok(wait) = (t - Utc::now()).to_std() {
            tokio::time::sleep(wait.min(SLEEP_SLICE)).await;
        }
    }
}

/// A clock that stands still until a sleep moves it to the sleep's end, or a test moves it. A
/// gated one reports each sleep's end and waits for the test's `release` before it returns.
#[cfg(test)]
pub(crate) struct FakeClock {
    now: std::sync::Mutex<DateTime<Utc>>,
    slept: std::sync::Mutex<Vec<DateTime<Utc>>>,
    gate: Option<Gate>,
}

#[cfg(test)]
struct Gate {
    sleeps: tokio::sync::mpsc::UnboundedSender<DateTime<Utc>>,
    permits: tokio::sync::Semaphore,
}

#[cfg(test)]
impl FakeClock {
    pub(crate) fn at(rfc3339: &str) -> Self {
        Self {
            now: std::sync::Mutex::new(rfc3339.parse().unwrap()),
            slept: std::sync::Mutex::default(),
            gate: None,
        }
    }

    /// A clock whose sleeps each send their end to the receiver, then wait for a permit.
    pub(crate) fn gated(rfc3339: &str) -> (Self, tokio::sync::mpsc::UnboundedReceiver<DateTime<Utc>>) {
        let (sleeps, receiver) = tokio::sync::mpsc::unbounded_channel();
        let gate = Gate { sleeps, permits: tokio::sync::Semaphore::new(0) };
        (Self { gate: Some(gate), ..Self::at(rfc3339) }, receiver)
    }

    /// Lets `n` more sleeps return.
    pub(crate) fn release(&self, n: usize) {
        self.gate.as_ref().unwrap().permits.add_permits(n);
    }

    /// Each sleep's end, in order.
    pub(crate) fn slept(&self) -> Vec<DateTime<Utc>> {
        self.slept.lock().unwrap().clone()
    }

    pub(crate) fn advance(&self, by: chrono::TimeDelta) {
        *self.now.lock().unwrap() += by;
    }
}

#[cfg(test)]
#[async_trait]
impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }

    async fn sleep_until(&self, t: DateTime<Utc>) {
        self.slept.lock().unwrap().push(t);
        if let Some(gate) = &self.gate {
            let _ = gate.sleeps.send(t);
            gate.permits.acquire().await.unwrap().forget();
        }
        let mut now = self.now.lock().unwrap();
        *now = (*now).max(t);
    }
}
