//! The resolution context a writer reads: everything resolution needs that the
//! consuming service owns, passed in rather than reached for.

use std::collections::HashMap;

use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::W3cdtfRange;
use shared_metadata::ContributorLookup;

/// Borrowed lookup tables a writer resolves against: contributor IDs,
/// ChronOntology period timespans, and the temporal-coverage enrichment table.
pub struct ResolveContext<'a> {
    pub lookup: &'a dyn ContributorLookup,
    pub periods: &'a HashMap<String, W3cdtfRange>,
    pub enriched: &'a HashMap<String, EnrichedDate>,
}

impl<'a> ResolveContext<'a> {
    pub fn new(
        lookup: &'a dyn ContributorLookup,
        periods: &'a HashMap<String, W3cdtfRange>,
        enriched: &'a HashMap<String, EnrichedDate>,
    ) -> Self {
        Self { lookup, periods, enriched }
    }
}
