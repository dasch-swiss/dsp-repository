//! Incunabula's hook: the four rules of the project that its `remodel` block cannot say.
//!
//! What they read and change is declared by the `hook` node of the project's block, and the
//! builder holds the hook to it: read the node, not this file, for the list.

use crate::hook::{Draft, Refusals};

mod catalogue;
mod scans;
mod sides;
mod sort_key;

// The classes, by their names in the block.
const BOOK: &str = "Book";
const PAGE: &str = "Page";
const BORDER_STRIP: &str = "BorderStrip";

/// The project's row in the builder's table of hooks.
///
/// The calibration scans go first: a rule that reads Pages reads the ones that stay.
pub(crate) fn remodel(draft: &mut Draft<'_>, refusals: &mut Refusals) {
    scans::drop_calibration_scans(draft, refusals);
    sort_key::provide(draft, refusals);
    catalogue::apply_overrides(draft, refusals);
    sides::refuse_mismatches(draft, refusals);
    sides::provide(draft);
}
