//! Which panics' payloads a panic hook must not log: those raised by the bridge's own work, whose
//! message may quote the data it handles, a dump's contents included.

use std::cell::Cell;
use std::future::Future;

thread_local! {
    static WITHHELD: Cell<bool> = const { Cell::new(false) };
}

/// Whether the code running on this thread is the bridge's work. A panic hook asks it while the
/// panic unwinds from that work, and then logs no payload.
#[must_use]
pub fn panic_payload_withheld() -> bool {
    WITHHELD.get()
}

/// Runs `f` with this thread's panics withheld.
pub(crate) fn withheld<T>(f: impl FnOnce() -> T) -> T {
    let _scope = Scope(WITHHELD.replace(true));
    f()
}

/// `future`, with every poll of it withheld, on whichever runtime thread polls it. A task it
/// spawns is not covered: wrap that one too.
pub(crate) fn withhold<F: Future>(future: F) -> impl Future<Output = F::Output> {
    let mut future = Box::pin(future);
    std::future::poll_fn(move |cx| withheld(|| future.as_mut().poll(cx)))
}

/// Restores the flag as it was, on return and on unwind alike.
struct Scope(bool);

impl Drop for Scope {
    fn drop(&mut self) {
        WITHHELD.set(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withheld_only_inside_and_restored_after() {
        assert!(!panic_payload_withheld());
        assert!(withheld(panic_payload_withheld));
        assert!(withheld(|| withheld(|| true) && panic_payload_withheld()));
        assert!(!panic_payload_withheld());
    }

    #[test]
    fn test_withheld_restored_after_a_panic() {
        let caught = std::panic::catch_unwind(|| withheld(|| panic!("a payload")));
        assert!(caught.is_err());
        assert!(!panic_payload_withheld());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_withhold_covers_every_poll_and_the_bridge_blocking_work() {
        let polled = tokio::spawn(withhold(async {
            let before = panic_payload_withheld();
            tokio::task::yield_now().await;
            before && panic_payload_withheld()
        }));
        assert!(polled.await.unwrap());

        let blocking = crate::refresh::spawn_blocking(panic_payload_withheld);
        assert!(blocking.await.unwrap());

        let unwrapped = tokio::spawn(async { panic_payload_withheld() });
        assert!(!unwrapped.await.unwrap());
    }
}
