//! The one negotiation step a landing page takes: an `Accept` header and the
//! representations on offer in, HTML or a redirect out.
//!
//! A landing page is never *rendered* differently by header (ADR-0004). It may
//! redirect to a machine-readable representation, which ADR-0005 carves out,
//! and this is the whole of that decision: a pure function, no dependency, no
//! HTTP type. The consuming service reads the header, calls this, and answers.
//!
//! The candidate list is [`UrlLayout::candidates`], built from the same pairs as
//! the representation `describedby` links, so a page can never redirect to a
//! representation it does not advertise. It is never hand-written at a call
//! site. The other `describedby` links — the OAI records — are not candidates:
//! a `describedby` target is not negotiable by being one.
//!
//! # The rules
//!
//! Written down rather than implied, because every one of them is a choice:
//!
//! - Media types compare case-insensitively, and a candidate matches only an **exact** type. A
//!   wildcard range counts towards HTML and never towards a candidate, which is why `application/*`
//!   gets HTML.
//! - Parameters other than `q` are ignored, so `text/html;level=1;q=0.7` is HTML at 0.7.
//! - A missing `q` counts as 1. A `q` that is not a number in `0..=1` skips that entry, and so does
//!   an entry with no media range; the rest are still considered. A repeated `q` in one entry is
//!   read left to right, so the last one wins.
//! - HTML's effective `q` is the **maximum** over every entry matching `text/html`, `text/*` or
//!   `*/*`, with no specificity weighting. No such entry means HTML is not asked for at all, which
//!   is `q` 0.
//! - A candidate wins only when its `q` is **strictly greater** than HTML's. Browsers send
//!   `text/html` at 1 and `*/*` at 0.8, so nothing changes for people, and an explicit tie goes to
//!   the page.
//! - Among candidates that beat HTML at the same `q`, the one listed first in the header wins — the
//!   header is read in order and the best is replaced only on a strictly greater `q`.
//! - At most [`MAX_RANGES`] media ranges are read, and a header longer than [`MAX_HEADER_BYTES`]
//!   counts as absent, so a hostile header costs bounded work and always yields [`Decision::Html`].
//!
//! This decision never produces a 4xx. There is no such thing as an `Accept`
//! this route cannot answer.

use crate::signposting::Candidate;

/// Media ranges read from one header. Past this the rest are ignored: a real
/// client sends a handful, and the answer cannot improve by reading more.
const MAX_RANGES: usize = 20;

/// Headers longer than this are treated as absent — HTML, no parsing.
const MAX_HEADER_BYTES: usize = 2048;

/// The ranges that ask for the landing page itself.
const HTML_RANGES: [&str; 3] = ["text/html", "text/*", "*/*"];

/// What the landing page answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Render the landing page.
    Html,
    /// Redirect to this representation.
    Redirect(String),
}

/// The landing page's answer for one `Accept` header.
pub fn decide(accept: Option<&str>, candidates: &[Candidate]) -> Decision {
    let Some(header) = accept.filter(|header| header.len() <= MAX_HEADER_BYTES) else {
        return Decision::Html;
    };

    let entries: Vec<(&str, f64)> = header.split(',').take(MAX_RANGES).filter_map(parse_entry).collect();

    let html_quality = entries
        .iter()
        .filter(|(range, _)| HTML_RANGES.iter().any(|html| range.eq_ignore_ascii_case(html)))
        .map(|(_, quality)| *quality)
        .fold(0.0_f64, f64::max);

    let mut best: Option<(f64, &str)> = None;
    for (range, quality) in &entries {
        if *quality <= html_quality {
            continue;
        }
        let Some(candidate) = candidates
            .iter()
            .find(|candidate| range.eq_ignore_ascii_case(&candidate.media_type))
        else {
            continue;
        };
        // Strictly greater, so an equal `q` leaves the earlier entry in place:
        // that is what makes the first-listed candidate win a tie.
        if best.is_none_or(|(best_quality, _)| *quality > best_quality) {
            best = Some((*quality, candidate.url.as_str()));
        }
    }

    match best {
        Some((_, url)) => Decision::Redirect(url.to_string()),
        None => Decision::Html,
    }
}

/// One `Accept` entry as its media range and quality, or `None` when the entry
/// is unusable and the rest of the header should still be read.
fn parse_entry(entry: &str) -> Option<(&str, f64)> {
    let mut parts = entry.split(';').map(str::trim);
    let range = parts.next().filter(|range| !range.is_empty())?;

    let mut quality = 1.0;
    for parameter in parts {
        // Only `q` is read. `level`, `charset` and anything else a client
        // invents say nothing about which representation it wants.
        let Some(value) = parameter
            .split_once('=')
            .filter(|(name, _)| name.eq_ignore_ascii_case("q"))
            .map(|(_, value)| value.trim())
        else {
            continue;
        };
        // Assigned, not returned: a repeated `q` in one entry is read to the
        // end and the last one wins, while any unusable `q` skips the entry.
        quality = value.parse::<f64>().ok().filter(|q| (0.0..=1.0).contains(q))?;
    }

    Some((range, quality))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_LD: &str = "application/ld+json";
    const DATACITE: &str = "application/vnd.datacite.datacite+json";

    fn candidates() -> Vec<Candidate> {
        vec![
            Candidate {
                media_type: JSON_LD.to_string(),
                url: "https://example.test/dpe/projects/0001/metadata.jsonld".to_string(),
            },
            Candidate {
                media_type: DATACITE.to_string(),
                url: "https://example.test/dpe/projects/0001/metadata.datacite.json".to_string(),
            },
        ]
    }

    fn decision(accept: Option<&str>) -> Decision {
        decide(accept, &candidates())
    }

    fn jsonld() -> Decision {
        Decision::Redirect("https://example.test/dpe/projects/0001/metadata.jsonld".to_string())
    }

    fn datacite() -> Decision {
        Decision::Redirect("https://example.test/dpe/projects/0001/metadata.datacite.json".to_string())
    }

    // --- one test per row of the plan's decision table ---

    #[test]
    fn a_browser_and_an_absent_header_get_the_page() {
        for accept in [None, Some("*/*"), Some("text/html,*/*;q=0.8")] {
            assert_eq!(decision(accept), Decision::Html, "{accept:?}");
        }
    }

    #[test]
    fn a_json_ld_request_is_redirected() {
        assert_eq!(decision(Some(JSON_LD)), jsonld());
    }

    /// The RDF list a FAIR assessor sends. Only the JSON-LD entry names a
    /// representation on offer, so it is the first one that matches anything.
    #[test]
    fn an_rdf_accept_list_lands_on_the_json_ld() {
        assert_eq!(
            decision(Some(
                "text/turtle, application/rdf+xml, application/n-triples, application/ld+json"
            )),
            jsonld()
        );
    }

    #[test]
    fn a_datacite_request_is_redirected() {
        assert_eq!(decision(Some(DATACITE)), datacite());
    }

    /// Nothing is on offer at `text/turtle`, and a type nobody offers is simply
    /// not a candidate.
    #[test]
    fn a_type_nobody_offers_gets_the_page() {
        assert_eq!(decision(Some("text/turtle")), Decision::Html);
    }

    /// `application/json` and `application/xml` are not aliases for JSON-LD.
    /// The representation is served for its own media type and no other.
    #[test]
    fn a_neighbouring_json_or_xml_type_is_not_an_alias() {
        for accept in ["application/json", "application/xml", "application/*"] {
            assert_eq!(decision(Some(accept)), Decision::Html, "{accept}");
        }
    }

    /// A tie goes to the page: a candidate must be preferred, not merely
    /// acceptable.
    #[test]
    fn an_explicit_tie_with_html_gets_the_page() {
        assert_eq!(decision(Some("text/html;q=1, application/ld+json;q=1")), Decision::Html);
    }

    #[test]
    fn a_header_asking_for_nothing_or_saying_nothing_gets_the_page() {
        for accept in ["*/*;q=0", ";;;", "", "q=0.5", "application/ld+json;q=bogus"] {
            assert_eq!(decision(Some(accept)), Decision::Html, "{accept:?}");
        }
    }

    // --- the written rules ---

    #[test]
    fn a_bad_entry_does_not_cost_the_good_one_beside_it() {
        assert_eq!(decision(Some("///;q=nope, application/ld+json")), jsonld());
        assert_eq!(decision(Some("application/ld+json, text/html;q=bogus")), jsonld());
    }

    /// HTML's `q` is the maximum over every range covering it, so the wildcard
    /// lifts the explicit entry rather than being overridden by it.
    #[test]
    fn htmls_quality_is_the_maximum_over_the_ranges_covering_it() {
        assert_eq!(decision(Some("text/html;q=0.5, text/*;q=0.9")), Decision::Html);
        assert_eq!(
            decision(Some("text/html;q=0.5, text/*;q=0.9, application/ld+json;q=0.95")),
            jsonld()
        );
        // 0.9 is not strictly greater than 0.9.
        assert_eq!(
            decision(Some("text/html;q=0.5, text/*;q=0.9, application/ld+json;q=0.9")),
            Decision::Html
        );
    }

    #[test]
    fn a_parameter_other_than_q_is_ignored() {
        assert_eq!(decision(Some("text/html;level=1;q=0.7")), Decision::Html);
        assert_eq!(decision(Some("text/html;level=1;q=0.7, application/ld+json;q=0.8")), jsonld());
    }

    #[test]
    fn a_missing_q_counts_as_one() {
        assert_eq!(decision(Some("application/ld+json, text/html;q=0.9")), jsonld());
    }

    /// Left to right, so the last `q` decides. The reverse order proves the
    /// rule is "last wins" and not "the highest wins".
    #[test]
    fn a_repeated_q_in_one_entry_is_read_to_the_end() {
        assert_eq!(decision(Some("application/ld+json;q=0.1;q=0.9, text/html;q=0.5")), jsonld());
        assert_eq!(
            decision(Some("application/ld+json;q=0.9;q=0.1, text/html;q=0.5")),
            Decision::Html
        );
    }

    #[test]
    fn a_q_outside_the_range_skips_its_entry() {
        assert_eq!(decision(Some("application/ld+json;q=2")), Decision::Html);
        assert_eq!(decision(Some("application/ld+json;q=-1")), Decision::Html);
    }

    #[test]
    fn media_types_compare_case_insensitively() {
        assert_eq!(decision(Some("APPLICATION/LD+JSON")), jsonld());
        assert_eq!(decision(Some("TEXT/HTML, application/ld+json")), Decision::Html);
    }

    /// Equal `q`, so the header's own order decides — and it decides for the
    /// entry, not for the candidate list's order.
    #[test]
    fn the_first_listed_candidate_wins_a_tie_between_candidates() {
        assert_eq!(decision(Some(&format!("{DATACITE}, {JSON_LD}"))), datacite());
        assert_eq!(decision(Some(&format!("{JSON_LD}, {DATACITE}"))), jsonld());
    }

    #[test]
    fn a_higher_q_beats_the_header_order() {
        assert_eq!(decision(Some(&format!("{DATACITE};q=0.5, {JSON_LD};q=0.9"))), jsonld());
    }

    // --- bounds ---

    #[test]
    fn a_header_over_the_byte_limit_is_treated_as_absent() {
        let mut header = "application/ld+json".to_string();
        while header.len() <= MAX_HEADER_BYTES {
            header.push_str(", text/plain");
        }
        assert_eq!(decision(Some(&header)), Decision::Html);
    }

    #[test]
    fn thousands_of_media_ranges_cost_bounded_work_and_yield_html() {
        let header = ["text/plain"; 5000].join(",");
        assert_eq!(decision(Some(&header)), Decision::Html);
    }

    /// Under the byte limit, but past the range limit: the candidate sits
    /// beyond the ranges that are read, so it is not seen.
    #[test]
    fn a_candidate_past_the_range_limit_is_not_read() {
        let mut ranges = vec!["text/plain"; MAX_RANGES];
        ranges.push(JSON_LD);
        let header = ranges.join(",");
        assert!(header.len() <= MAX_HEADER_BYTES, "{}", header.len());
        assert_eq!(decision(Some(&header)), Decision::Html);
    }

    #[test]
    fn an_empty_candidate_list_can_only_yield_html() {
        assert_eq!(decide(Some(JSON_LD), &[]), Decision::Html);
    }
}
