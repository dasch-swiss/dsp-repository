//! Where DPE's published data set lives in this repository's checkout.
//!
//! One name per toolchain reads through here (or its `justfile`/Playwright
//! equivalent), so a future move of `areas/access/dpe` changes this file alone.

use std::path::{Path, PathBuf};

/// DPE's published data set, relative to the repository root.
pub const DPE_DATA_DIR: &str = "areas/access/dpe/server/data";

/// [`DPE_DATA_DIR`] resolved against this checkout, for tests and dev tooling.
///
/// Resolves via the build machine's `CARGO_MANIFEST_DIR`, so it is meaningless
/// in a deployed image; `EditorConfig` deliberately carries no default for
/// `EDITOR_DATA_DIR` (see `deposit-server`'s `config.rs`).
///
/// The repository root is taken lexically, four levels up, not by joining `..`:
/// under Bazel the variable is the relative `areas/deposit/editor/core`, and the
/// runfiles of another crate's test hold no such directory to walk out of.
pub fn checkout_dpe_data_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("the crate sits four levels below the repository root");
    root.join(DPE_DATA_DIR)
}
