//! In-process cache mapping ChronOntology period URLs to W3CDTF date ranges.
//!
//! ChronOntology periods are mirrored into `chronontology-periods.json` (a single
//! object keyed by bare period id, e.g. `"0vGXxVln724L"`). DSP project metadata
//! references periods by full URL (`https://chronontology.dainst.org/period/<id>`),
//! so the lookup strips the `/period/` prefix to reconcile the two.
//!
//! Loaded from disk once on first access, like the other `*_cache` modules.
//!
//! Note: This module is gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::utils::get_data_dir;
use super::w3cdtf::{to_w3cdtf_range, W3cdtfRange};

const PERIODS_FILE: &str = "chronontology-periods.json";

static PERIODS: OnceLock<HashMap<String, W3cdtfRange>> = OnceLock::new();

/// Raw shape of one period in `chronontology-periods.json`. Only the timespan is
/// read; all other fields (names, relations, ...) are ignored at runtime.
#[derive(Debug, Deserialize)]
struct PeriodRaw {
    #[serde(rename = "hasTimespan", default)]
    has_timespan: Vec<TimespanRaw>,
}

#[derive(Debug, Deserialize)]
struct TimespanRaw {
    #[serde(default)]
    begin: Option<BoundRaw>,
    #[serde(default)]
    end: Option<BoundRaw>,
}

#[derive(Debug, Deserialize)]
struct BoundRaw {
    #[serde(default)]
    at: Option<String>,
}

/// Return a reference to the cached period-range map, loading it on first call.
pub fn all_periods() -> &'static HashMap<String, W3cdtfRange> {
    PERIODS.get_or_init(load_all_periods)
}

/// Strip a ChronOntology URL down to its bare period id. Falls back to the input
/// unchanged when it does not contain the `/period/` segment.
fn period_id(url: &str) -> &str {
    url.rsplit("/period/").next().unwrap_or(url).trim_end_matches('/')
}

/// Look up a ChronOntology period URL in `periods`, returning its W3CDTF range.
/// Pure over the given map so it can be unit-tested without the process cache.
pub fn timespan_for_in(periods: &HashMap<String, W3cdtfRange>, url: &str) -> Option<W3cdtfRange> {
    periods.get(period_id(url)).cloned()
}

/// Look up a ChronOntology period URL (from the cache), returning its W3CDTF range.
pub fn timespan_for(url: &str) -> Option<W3cdtfRange> {
    timespan_for_in(all_periods(), url)
}

/// Convert one raw period into a W3CDTF range, taking the first timespan that
/// yields a range. (Current data has no multi-timespan periods; "first" is a
/// defensive default.)
fn range_for(period: &PeriodRaw) -> Option<W3cdtfRange> {
    period.has_timespan.iter().find_map(|ts| {
        let begin = ts.begin.as_ref().and_then(|b| b.at.as_deref());
        let end = ts.end.as_ref().and_then(|b| b.at.as_deref());
        to_w3cdtf_range(begin, end)
    })
}

fn load_all_periods() -> HashMap<String, W3cdtfRange> {
    use std::fs;
    use std::path::PathBuf;

    let path = PathBuf::from(get_data_dir()).join(PERIODS_FILE);

    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(file = ?path, error = %e, "failed to read ChronOntology periods file");
            return HashMap::new();
        }
    };

    let raw: HashMap<String, PeriodRaw> = match serde_json::from_str(&json) {
        Ok(raw) => raw,
        Err(e) => {
            tracing::warn!(file = ?path, error = %e, "failed to parse ChronOntology periods file");
            return HashMap::new();
        }
    };

    raw.into_iter()
        .filter_map(|(id, period)| range_for(&period).map(|range| (id, range)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> HashMap<String, W3cdtfRange> {
        let mut map = HashMap::new();
        map.insert("0vGXxVln724L".to_string(), to_w3cdtf_range(Some("98"), Some("117")).unwrap());
        map
    }

    #[test]
    fn resolves_full_url_by_stripping_period_prefix() {
        let periods = fixtures();
        let range = timespan_for_in(&periods, "https://chronontology.dainst.org/period/0vGXxVln724L");
        assert_eq!(range.map(String::from), Some("0098/0117".to_string()));
    }

    #[test]
    fn resolves_bare_id_directly() {
        let periods = fixtures();
        let range = timespan_for_in(&periods, "0vGXxVln724L");
        assert_eq!(range.map(String::from), Some("0098/0117".to_string()));
    }

    #[test]
    fn unknown_id_is_none() {
        let periods = fixtures();
        assert_eq!(
            timespan_for_in(&periods, "https://chronontology.dainst.org/period/unknown"),
            None
        );
    }

    #[test]
    fn period_id_strips_prefix_and_trailing_slash() {
        assert_eq!(period_id("https://chronontology.dainst.org/period/ABC123"), "ABC123");
        assert_eq!(period_id("https://chronontology.dainst.org/period/ABC123/"), "ABC123");
        assert_eq!(period_id("ABC123"), "ABC123");
    }

    #[test]
    fn range_for_skips_period_without_usable_timespan() {
        let period = PeriodRaw { has_timespan: vec![] };
        assert_eq!(range_for(&period), None);
    }

    #[test]
    fn range_for_takes_first_usable_timespan() {
        let period = PeriodRaw {
            has_timespan: vec![
                TimespanRaw { begin: None, end: None },
                TimespanRaw {
                    begin: Some(BoundRaw { at: Some("1300".to_string()) }),
                    end: Some(BoundRaw { at: Some("1500".to_string()) }),
                },
            ],
        };
        assert_eq!(range_for(&period).map(String::from), Some("1300/1500".to_string()));
    }
}
