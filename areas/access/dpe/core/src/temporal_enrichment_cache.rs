//! [`Corpus`]'s cache over the offline temporal-coverage enrichment table.
//!
//! The loading and lookup logic is
//! [`shared_metadata::temporal_enrichment`], shared with the editor. What
//! stays here is the cache and the DPE data directory it reads.
use std::collections::HashMap;

use shared_metadata::temporal_enrichment::{self, EnrichedDate};

use super::corpus::Corpus;

impl Corpus {
    pub fn all_enriched(&'static self) -> &'static HashMap<String, EnrichedDate> {
        self.enrichment_cache
            .get_or_init(|| temporal_enrichment::load_from(std::path::Path::new(&self.settings.data_dir)))
    }

    /// Look up an enriched entry (from the cache) by its normalized key.
    pub fn enriched_for(&'static self, key: &str) -> Option<EnrichedDate> {
        temporal_enrichment::enriched_for_in(self.all_enriched(), key)
    }
}

#[cfg(test)]
mod tests {
    /// A single W3CDTF year: optional `-`, then 4+ digits.
    fn is_w3cdtf_year(s: &str) -> bool {
        let digits = s.strip_prefix('-').unwrap_or(s);
        digits.len() >= 4 && digits.chars().all(|c| c.is_ascii_digit())
    }

    /// Accepts a single year, a `begin/end` range, or an RKMS-ISO8601 open range
    /// (`year/` or `/year`).
    fn is_w3cdtf(s: &str) -> bool {
        match s.split_once('/') {
            None => is_w3cdtf_year(s),
            Some(("", end)) => is_w3cdtf_year(end),     // /1900
            Some((begin, "")) => is_w3cdtf_year(begin), // 1900/
            Some((begin, end)) => is_w3cdtf_year(begin) && is_w3cdtf_year(end),
        }
    }

    /// Loads the real committed enrichment table through the production
    /// `load_from` and asserts every filled date is valid W3CDTF. This is the
    /// guard against a typo'd range or broken JSON in the committed data file.
    ///
    /// It lives here rather than beside `load_from` in `shared-metadata`,
    /// because the file it reads is DPE's data directory and a shared crate
    /// takes no path into a service.
    #[test]
    fn committed_enrichment_table_is_valid() {
        let data_dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data"));
        let table = shared_metadata::temporal_enrichment::load_from(data_dir);

        assert!(!table.is_empty(), "committed enrichment table should load and be non-empty");
        for (key, entry) in &table {
            if let Some(ref date) = entry.date {
                assert!(is_w3cdtf(date), "entry {key:?} has malformed W3CDTF date {date:?}");
            }
            assert!(!entry.original_name.is_empty(), "entry {key:?} has empty original_name");
        }
    }
}
