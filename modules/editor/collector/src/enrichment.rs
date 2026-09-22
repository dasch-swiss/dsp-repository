//! Keeping a collected project's `temporalCoverage` resolvable.
//!
//! `dpe-server validate` and `every_committed_temporal_coverage_resolves` both
//! fail on a free-text `temporalCoverage` that resolves to no date and carries
//! no reviewed row, so a project collected without one arrives with a red gate
//! through no reviewer's fault. A skeleton row (`date: null`,
//! `source: "unresolved"`) makes the gate pass and leaves the value visibly
//! unreviewed for whoever replaces it with a range.

use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};
use shared_metadata::project::ProjectRaw;
#[cfg(test)]
use shared_metadata::project::TemporalCoverage;
#[cfg(test)]
use shared_metadata::temporal_enrichment::EnrichedDate;
#[cfg(test)]
use shared_metadata::utils::Multilingual;
use shared_metadata::{chronontology, temporal_coverage, temporal_enrichment};

pub const ENRICHMENT_FILE: &str = "temporal-coverage-enrichment.json";

/// The indent and trailing newline the committed table uses.
const INDENT: &[u8] = b"  ";

/// Adds a skeleton row for every `temporalCoverage` value in `project` that
/// would otherwise count as a completeness gap.
///
/// Returns the names it added, empty when the table already covered every
/// value. The decision is `temporal_coverage::completeness_gap`, the same one
/// both enforcement points apply, so this cannot disagree with the gate it
/// exists to satisfy.
pub fn add_skeleton_rows(data_dir: &Path, project: &ProjectRaw) -> Result<Vec<String>, String> {
    let periods = chronontology::load_from(data_dir);
    let enrichment = temporal_enrichment::load_from(data_dir);

    let mut gaps: Vec<String> = project
        .temporal_coverage
        .iter()
        .filter_map(|coverage| temporal_coverage::completeness_gap(coverage, &periods, &enrichment))
        .collect();
    gaps.sort();
    gaps.dedup();

    if gaps.is_empty() {
        return Ok(Vec::new());
    }

    let path = data_dir.join(ENRICHMENT_FILE);
    // A `BTreeMap`, so the table stays in the key order the committed file
    // holds and an added row lands in place rather than at the end.
    let mut table: BTreeMap<String, Value> = match std::fs::read_to_string(&path) {
        Ok(json) => {
            serde_json::from_str(&json).map_err(|error| format!("{ENRICHMENT_FILE} does not parse: {error}"))?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => return Err(format!("could not read {ENRICHMENT_FILE}: {error}")),
    };

    for name in &gaps {
        table.insert(
            name.clone(),
            json!({"date": null, "original_name": name, "source": "unresolved"}),
        );
    }

    std::fs::write(&path, crate::json::render(&table, INDENT, ENRICHMENT_FILE)?)
        .map_err(|error| format!("could not write {ENRICHMENT_FILE}: {error}"))?;
    Ok(gaps)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn committed_data_dir() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dpe/server/data"))
    }

    /// Adding one row must not reformat the other 120. Held against the
    /// committed table rather than a fixture, since it is the file this writes.
    #[test]
    fn the_committed_table_round_trips_byte_identically() {
        let path = committed_data_dir().join(ENRICHMENT_FILE);
        let committed = std::fs::read_to_string(&path).expect("the committed table should be readable");
        let table: BTreeMap<String, Value> =
            serde_json::from_str(&committed).expect("the committed table should parse");

        assert_eq!(
            crate::json::render(&table, INDENT, ENRICHMENT_FILE).expect("the table renders"),
            committed
        );
    }

    /// The row's shape is what closes the gap: `completeness_gap` is the single
    /// decision `dpe-server validate` and `every_committed_temporal_coverage_resolves`
    /// both apply, so a row it still rejects would leave the collection pull
    /// request red.
    #[test]
    fn a_skeleton_row_is_the_shape_that_stops_counting_as_a_gap() {
        let name = "a free-text period nothing resolves";
        let coverage = TemporalCoverage::Text(Multilingual::from([("en".to_string(), name.to_string())]));
        let periods = HashMap::new();

        let mut table: HashMap<String, EnrichedDate> = HashMap::new();
        assert_eq!(
            temporal_coverage::completeness_gap(&coverage, &periods, &table).as_deref(),
            Some(name),
            "without a row the value is a gap"
        );

        let skeleton = json!({"date": null, "original_name": name, "source": "unresolved"});
        table.insert(
            name.to_string(),
            serde_json::from_value(skeleton).expect("the skeleton row parses"),
        );

        assert!(
            temporal_coverage::completeness_gap(&coverage, &periods, &table).is_none(),
            "the skeleton row closes the gap"
        );
    }
}
