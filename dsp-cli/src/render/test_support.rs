//! Shared test helpers for `src/render/` unit tests.
//!
//! `SharedBuf` uses `Arc<Mutex<Vec<u8>>>` because the renderer `with_writers`
//! constructors take two *owned* `impl Write` values and the tests pass
//! `out.clone()` / `err.clone()` — each clone bumps the Arc refcount so both
//! writer handles point at the same underlying buffer.
//!
//! The integration-test crate (`tests/`) has its own `support::SharedBuf` that
//! uses `Rc<RefCell<Vec<u8>>>` (single-threaded; separate crate boundary).
//! Do not unify the two — the interior-mutability types differ by design.
//!
//! `progress.rs` is excluded: its `SharedBuf` is `Rc<RefCell<…>>` for a
//! different purpose and stays local.

use std::io;
use std::sync::{Arc, Mutex};

use crate::render::MetaContext;

/// Shared write-buffer for capturing output in unit tests.
///
/// `Clone` bumps the `Arc` refcount; all clones share the same backing buffer.
#[derive(Clone)]
pub(crate) struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl SharedBuf {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    pub(crate) fn string(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Build a `MetaContext` with the given auth state and server label.
///
/// `filter_warning` is always `None` — add a bespoke constructor in the
/// calling test if you need a non-`None` warning.
pub(crate) fn make_meta(auth_state: &str, server: &str) -> MetaContext {
    MetaContext {
        server_label: server.to_string(),
        auth_state: auth_state.to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}
