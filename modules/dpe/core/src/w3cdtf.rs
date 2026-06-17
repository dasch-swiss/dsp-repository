//! W3CDTF / RKMS-ISO8601 date-range formatting for DataCite output.
//!
//! DataCite's `date` property requires a structured W3CDTF value (a single year
//! or a `start/end` interval), not free text. ChronOntology timespans and DSP
//! project start/end dates both feed through here so the two callers produce
//! identical formatting.
//!
//! Year handling:
//! - Numeric years are zero-padded to at least four digits (`98` -> `0098`) and keep a leading `-`
//!   for BCE (`-54` -> `-0054`). Years already wider than four digits pass through unchanged
//!   (ChronOntology has geological periods down to `-4600000000`), which W3CDTF / `xs:gYear`
//!   permit.
//! - The literals `null`, `"not specified"` and `"present"` (all present in the ChronOntology data)
//!   are treated as a missing/open bound.

/// A formatted, W3CDTF-valid temporal range ready to drop into a DataCite
/// `date` value. Always either a single year or a `start/end` interval (with
/// `..` for an open side).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W3cdtfRange(String);

impl W3cdtfRange {
    /// The formatted range string (e.g. `0098/0117`, `-0054`, `0030/..`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<W3cdtfRange> for String {
    fn from(range: W3cdtfRange) -> Self {
        range.0
    }
}

/// Sentinels seen in ChronOntology `begin.at` / `end.at` that denote an absent
/// bound rather than a year.
fn is_open_bound(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("not specified") || trimmed.eq_ignore_ascii_case("present")
}

/// Parse a single bound into a `(year, w3cdtf_string)` pair, or `None` if the
/// bound is absent/open or not a valid year.
fn parse_year(raw: Option<&str>) -> Option<(i64, String)> {
    let raw = raw?;
    if is_open_bound(raw) {
        return None;
    }
    let year: i64 = raw.trim().parse().ok()?;
    Some((year, format_year(year)))
}

/// Format a (possibly negative) year as W3CDTF, zero-padding the magnitude to at
/// least four digits and preserving the BCE sign.
fn format_year(year: i64) -> String {
    let sign = if year < 0 { "-" } else { "" };
    format!("{sign}{:04}", year.unsigned_abs())
}

/// Build a W3CDTF range from a begin/end pair of raw bound strings.
///
/// Returns `None` when both bounds are absent, or when both are present but
/// reversed (`begin > end`) — a reversed interval is never emitted; the caller
/// falls back to a name-only `dateInformation`.
pub fn to_w3cdtf_range(begin: Option<&str>, end: Option<&str>) -> Option<W3cdtfRange> {
    let begin = parse_year(begin);
    let end = parse_year(end);

    match (begin, end) {
        (Some((by, bs)), Some((ey, es))) => {
            if by > ey {
                tracing::warn!(begin = by, end = ey, "temporal range has begin after end; skipping");
                return None;
            }
            if bs == es {
                Some(W3cdtfRange(bs))
            } else {
                Some(W3cdtfRange(format!("{bs}/{es}")))
            }
        }
        (Some((_, bs)), None) => Some(W3cdtfRange(format!("{bs}/.."))),
        (None, Some((_, es))) => Some(W3cdtfRange(format!("../{es}"))),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(begin: Option<&str>, end: Option<&str>) -> Option<String> {
        to_w3cdtf_range(begin, end).map(|r| r.as_str().to_string())
    }

    #[test]
    fn pads_short_years_to_four_digits() {
        assert_eq!(range(Some("98"), Some("117")), Some("0098/0117".to_string()));
    }

    #[test]
    fn keeps_bce_sign_and_pads() {
        assert_eq!(range(Some("-54"), Some("-51")), Some("-0054/-0051".to_string()));
    }

    #[test]
    fn passes_long_years_through_unchanged() {
        assert_eq!(range(Some("-150000"), Some("-80000")), Some("-150000/-80000".to_string()));
    }

    #[test]
    fn equal_bounds_collapse_to_single_year() {
        assert_eq!(range(Some("1300"), Some("1300")), Some("1300".to_string()));
    }

    #[test]
    fn open_end_present_literal() {
        assert_eq!(range(Some("1900"), Some("present")), Some("1900/..".to_string()));
    }

    #[test]
    fn open_end_not_specified_literal() {
        assert_eq!(range(Some("1900"), Some("not specified")), Some("1900/..".to_string()));
    }

    #[test]
    fn open_begin() {
        assert_eq!(range(None, Some("1900")), Some("../1900".to_string()));
    }

    #[test]
    fn both_missing_is_none() {
        assert_eq!(range(None, None), None);
        assert_eq!(range(Some("not specified"), None), None);
    }

    #[test]
    fn reversed_bounds_rejected() {
        assert_eq!(range(Some("1900"), Some("1800")), None);
    }

    #[test]
    fn whitespace_and_empty_treated_as_open() {
        assert_eq!(range(Some("  "), Some("1900")), Some("../1900".to_string()));
    }

    #[test]
    fn non_numeric_bound_is_open() {
        // Anything that isn't a recognised year or sentinel is treated as absent.
        assert_eq!(range(Some("circa 1900"), Some("1950")), Some("../1950".to_string()));
    }
}
