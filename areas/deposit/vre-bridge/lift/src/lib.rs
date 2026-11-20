//! The VRE bridge's lift: one project's dump as the interim snapshot that
//! `FORMAT.md` beside this crate specifies. [`lift`] selects the dump's current
//! data, refuses the project unless the anonymous user may view all of it (the
//! public guard), and writes the selection with its access rights.

// Ported from dsp-incubator's dao-lift, which pre-dates these two (purely
// stylistic) lints: allowed crate-wide rather than rewriting ported code, so the
// diff against the source stays readable; `-D warnings` stays on for everything else.
#![allow(clippy::collapsible_if, clippy::too_many_arguments)]
// A lift failure is a `LiftError`: production code calls no `expect`, `unwrap`,
// `panic!` or `unreachable!`.
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unreachable)
)]

mod dump;
mod error;
mod guard;
#[cfg(test)]
mod guard_tests;
mod model;
mod selection;
mod snapshot;
#[cfg(test)]
mod test_support;

use std::path::Path;

pub use error::LiftError;
pub use guard::{Problem, SubjectKind, Violation};
pub use selection::{SelectedFile, SelectedFiles};
pub use snapshot::{Handling, Kind, Snapshot, Stats, ValueKind, ViolationReport};

use crate::dump::Dump;

/// The interim format version a snapshot follows, recorded in each Ready token.
/// Raised when a reader would read a snapshot differently (`FORMAT.md` §0).
pub const INTERIM_FORMAT_VERSION: u32 = 1;

/// The version of the transformation from a dump to a snapshot. Raised with any
/// change to the bytes written for the same dump; the golden test fails until it is.
pub const TRANSFORMATION_VERSION: u32 = 1;

/// A lifted project: its snapshot, what the run counted, for the caller to log, and
/// the current file values of the snapshot's resources, for a DaTEI file mapping.
#[derive(Debug)]
pub struct Lifted {
    pub snapshot: Snapshot,
    pub stats: Stats,
    pub files: SelectedFiles,
}

/// The snapshot of the unzipped dump at `dump_dir`; reads only its `data.nq` and
/// `ontology-*.nq`. Blocks, and holds the whole dump in memory.
pub fn lift(dump_dir: &Path) -> Result<Lifted, LiftError> {
    let dump = Dump::load(dump_dir).map_err(|e| LiftError::Dump(e.to_string()))?;
    lift_dump(&dump)
}

/// Select, guard, then build: the snapshot holds exactly the selection the guard passed.
fn lift_dump(dump: &Dump) -> Result<Lifted, LiftError> {
    let read = snapshot::Read::of(dump)?;
    let selection = selection::select(&read.index, &read.subjects);
    let (rights, violations) = guard::check(&read.index, &read.subjects, &selection);
    if !violations.is_empty() {
        return Err(LiftError::Refused { violations });
    }
    let files = selection::files(&read.index, &read.subjects, &selection);
    let mut stats = Stats::default();
    let snapshot = snapshot::build(read, &selection, &rights, &mut stats)?;
    Ok(Lifted { snapshot, stats, files })
}
