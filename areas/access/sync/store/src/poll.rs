//! The Ready-token poll behind [`ArchiveProjection::subscribe`](cpe_ports::ArchiveProjection):
//! every interval, the token of each known project, and of each project known the round before, is
//! read and compared with what the last round saw, and each project whose token appeared, changed
//! or went is announced. A poll, not a file-system watch: the subtree may be NFS.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpe_ports::ChangeListener;
use shared_stand_in::{known_projects, read_token, InvalidToken, ReadyToken, TokenError};

type Listeners = Arc<Mutex<Vec<Arc<dyn ChangeListener>>>>;

/// The poll thread of one adapter. Dropping it stops the thread and waits for it to end.
pub(crate) struct Poll {
    listeners: Listeners,
    /// Dropped to stop the thread: its `recv_timeout` then returns at once.
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Poll {
    /// Reads every token as the baseline, announcing nothing for it, then starts the thread that
    /// polls every `interval` and tells `listener` of each change from then on.
    pub(crate) fn start(root: PathBuf, interval: Duration, listener: Arc<dyn ChangeListener>) -> Self {
        let listeners: Listeners = Arc::new(Mutex::new(vec![listener]));
        let mut tokens = Tokens::new(root);
        tokens.round();
        let (stop, stopped) = mpsc::channel::<()>();
        let polled = Arc::clone(&listeners);
        let spawned = thread::Builder::new().name("sync-ready-token-poll".to_string()).spawn(move || {
            // `Disconnected`, the stop, ends the loop; nothing is ever sent.
            while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(interval) {
                let Ok(changed) = catch_unwind(AssertUnwindSafe(|| tokens.round())) else {
                    tracing::error!("a round of the Ready-token poll panicked; the next one runs as usual");
                    continue;
                };
                for shortcode in changed {
                    announce(&polled, &shortcode);
                }
            }
        });
        let thread = match spawned {
            Ok(thread) => Some(thread),
            Err(error) => {
                tracing::error!(%error, "the Ready-token poll cannot start: no changed project is announced");
                None
            }
        };
        Self { listeners, stop: Some(stop), thread }
    }

    pub(crate) fn add(&self, listener: Arc<dyn ChangeListener>) {
        self.listeners.lock().unwrap_or_else(PoisonError::into_inner).push(listener);
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            // A listener that held the last handle on the adapter drops it on the poll thread,
            // which cannot join itself; it ends on its own once the stop is seen.
            if thread.thread().id() == thread::current().id() {
                return;
            }
            if thread.join().is_err() {
                tracing::error!("the Ready-token poll panicked");
            }
        }
    }
}

/// A listener that panics is logged and skipped, so the poll goes on for the others.
fn announce(listeners: &Listeners, shortcode: &str) {
    let listeners = listeners.lock().unwrap_or_else(PoisonError::into_inner).clone();
    for listener in listeners {
        if catch_unwind(AssertUnwindSafe(|| listener.changed(shortcode))).is_err() {
            tracing::error!(shortcode, "a change listener panicked");
        }
    }
}

/// What the poll last saw of each project's token.
struct Tokens {
    root: PathBuf,
    seen: BTreeMap<String, Seen>,
    /// Projects whose token could not be read in the last round they were polled; logged once
    /// on failing and once on recovering, not every round.
    failing: BTreeSet<String>,
    root_failing: bool,
}

/// A token, compared by its content: two tokens are equal exactly when their canonical encodings
/// are.
#[derive(PartialEq)]
enum Seen {
    Token(ReadyToken),
    /// A token that does not parse is still a change: its reader answers `Unavailable`.
    Invalid(InvalidToken),
}

impl Tokens {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            seen: BTreeMap::new(),
            failing: BTreeSet::new(),
            root_failing: false,
        }
    }

    /// The shortcodes whose token appeared, changed or went since the last round. A project whose
    /// token cannot be read is skipped and keeps what the last round saw of it; while the root
    /// cannot be listed, the round announces nothing, and a baseline taken then is empty, so the
    /// first round that lists the root announces every project it finds.
    fn round(&mut self) -> Vec<String> {
        let listed = match known_projects(&self.root) {
            Ok(listed) => listed,
            Err(error) => {
                if !self.root_failing {
                    tracing::warn!(%error, root = %self.root.display(), "the Ready-token poll cannot list the subtree");
                }
                self.root_failing = true;
                return Vec::new();
            }
        };
        if std::mem::take(&mut self.root_failing) {
            tracing::info!(root = %self.root.display(), "the Ready-token poll lists the subtree again");
        }
        let polled: BTreeSet<String> = listed.into_iter().chain(self.seen.keys().cloned()).collect();
        let mut changed = Vec::new();
        for shortcode in polled {
            let now = match read_token(&self.root, &shortcode) {
                Ok(token) => token.map(Seen::Token),
                Err(TokenError::Invalid(invalid)) => Some(Seen::Invalid(invalid)),
                Err(TokenError::Unreadable(kind)) => {
                    if self.failing.insert(shortcode.clone()) {
                        tracing::warn!(shortcode, ?kind, "the Ready-token poll cannot read the project's token");
                    }
                    continue;
                }
            };
            if self.failing.remove(&shortcode) {
                tracing::info!(shortcode, "the Ready-token poll reads the project's token again");
            }
            if self.seen.get(&shortcode) != now.as_ref() {
                match now {
                    Some(now) => self.seen.insert(shortcode.clone(), now),
                    None => self.seen.remove(&shortcode),
                };
                changed.push(shortcode);
            }
        }
        changed
    }
}
