//! [`Corpus`]'s cache over the ChronOntology period table.
//!
//! The loading and lookup logic is [`shared_metadata::chronontology`], shared
//! with the editor. What stays here is the cache and the DPE data directory it
//! reads.
use std::collections::HashMap;

use shared_metadata::chronontology;
use shared_metadata::w3cdtf::W3cdtfRange;

use super::corpus::Corpus;

impl Corpus {
    /// Return a reference to the cached period-range map, loading it on first call.
    pub fn all_periods(&'static self) -> &'static HashMap<String, W3cdtfRange> {
        self.periods_cache
            .get_or_init(|| chronontology::load_from(std::path::Path::new(&self.settings.data_dir)))
    }

    /// Look up a ChronOntology period URL (from the cache), returning its W3CDTF range.
    pub fn timespan_for(&'static self, url: &str) -> Option<W3cdtfRange> {
        chronontology::timespan_for_in(self.all_periods(), url)
    }
}
