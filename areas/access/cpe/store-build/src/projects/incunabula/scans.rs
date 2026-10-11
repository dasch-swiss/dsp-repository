//! Calibration scans. A Page whose pagenum is the sentinel alone is a shot of a page beside a
//! colour chart: no page of its Book, and dropped.

use super::PAGE;
use crate::error::Rule;
use crate::generic::text::py_strip;
use crate::hook::{Draft, Mapped, Refusals, Value};

/// The texts an editor may have written about a Page.
const TEXTS: [&str; 2] = ["description", "comment"];
/// A Page's links to border strips.
const STRIP_LINKS: [&str; 2] = ["strip-left", "strip-right"];

/// Drops every calibration scan.
///
/// Dropping one must lose nothing an editor wrote: a scan that has a description or a comment
/// that is not empty once stripped, or a border strip, is refused, once per property. The
/// content is read as served, so a text the mapping refused still counts as one.
pub(super) fn drop_calibration_scans<'a>(draft: &mut Draft<'a>, refusals: &mut Refusals) {
    let Some(sentinel) = draft.sentinel() else {
        return;
    };
    let mut scans: Vec<&'a str> = Vec::new();
    for page in draft.entries_of(PAGE) {
        let Mapped::Values([Value::Text(pagenum, _)]) = page.values("name") else {
            continue;
        };
        if py_strip(pagenum) != sentinel {
            continue;
        }
        for name in TEXTS {
            let says_something = |value: &Value<'_>| match value {
                Value::Text(text, _) => !py_strip(text).is_empty(),
                _ => false,
            };
            if page.served(name).iter().any(says_something) {
                refusals.refuse(Rule::CalibrationScanContent, page.iri(), name);
            }
        }
        for name in STRIP_LINKS {
            if !page.served(name).is_empty() {
                refusals.refuse(Rule::CalibrationScanContent, page.iri(), name);
            }
        }
        scans.push(page.iri());
    }
    for iri in scans {
        draft.drop(iri);
    }
}
