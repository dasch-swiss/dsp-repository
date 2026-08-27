//! Process-global cache over the ChronOntology period table.
//!
//! The loading and lookup logic is [`platform_metadata::chronontology`], shared
//! with the editor. What stays here is the `OnceLock` and the DPE data
//! directory it reads: `DPE_DATA_DIR` is one service's configuration and does
//! not belong in a shared crate.
use std::collections::HashMap;
use std::sync::OnceLock;

use platform_metadata::chronontology;
use platform_metadata::w3cdtf::W3cdtfRange;

use super::utils::get_data_dir;

static PERIODS: OnceLock<HashMap<String, W3cdtfRange>> = OnceLock::new();

/// Return a reference to the cached period-range map, loading it on first call.
pub fn all_periods() -> &'static HashMap<String, W3cdtfRange> {
    PERIODS.get_or_init(load_all_periods)
}

/// Look up a ChronOntology period URL (from the cache), returning its W3CDTF range.
pub fn timespan_for(url: &str) -> Option<W3cdtfRange> {
    chronontology::timespan_for_in(all_periods(), url)
}

fn load_all_periods() -> HashMap<String, W3cdtfRange> {
    chronontology::load_from(std::path::Path::new(get_data_dir()))
}
