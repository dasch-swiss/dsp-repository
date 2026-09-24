//! Where DPE's published data set lives in this repository's checkout.
//!
//! One name per toolchain reads through here (or its `justfile`/Playwright
//! equivalent), so a future move of `modules/dpe` changes this file alone.

use std::path::{Path, PathBuf};

/// DPE's published data set, relative to the repository root.
pub const DPE_DATA_DIR: &str = "modules/dpe/server/data";

/// [`DPE_DATA_DIR`] resolved against this checkout, for tests and dev tooling.
///
/// Resolves via the build machine's `CARGO_MANIFEST_DIR`, so it is meaningless
/// in a deployed image; `EditorConfig` deliberately carries no default for
/// `EDITOR_DATA_DIR` (see `server/src/config.rs`).
pub fn checkout_dpe_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..").join(DPE_DATA_DIR)
}
