//! Catalogue addresses that are dead in the archive, and the addresses the editors curated to be
//! written in their place.

use super::BOOK;
use crate::error::Rule;
use crate::generic::text::py_strip;
use crate::hook::{Draft, Mapped, Refusals, Value, View};

/// The property a Book's catalogue address is written under.
const NAME: &str = "catalogue-uri";

/// The curated keys of an override: the address to write, and the address it replaces. The
/// override holds only while the archive has exactly the second: once the address is mended
/// there, the build refuses the Book until the editors take both values out.
const REPLACEMENT_KEY: &str = "catalogue_uri";
const DEAD_KEY: &str = "catalogue_uri_replaces";

/// Puts the curated replacement in place of every overridden Book's address.
///
/// The override is tested on the address as served. A Book whose address is no longer the dead
/// one is refused and left without an address, so that the address it has is not checked a
/// second time. A Book whose address the mapping refused for its kind or its number is passed
/// over: that refusal is the report.
pub(super) fn apply_overrides<'a>(draft: &mut Draft<'a>, refusals: &mut Refusals) {
    let mut overrides: Vec<(&'a str, Vec<Value<'a>>)> = Vec::new();
    for book in draft.entries_of(BOOK) {
        let Some((dead, replacement)) = curated_pair(&book, refusals) else {
            continue;
        };
        if matches!(book.values(NAME), Mapped::Refused(_)) {
            continue;
        }
        let served = match book.served(NAME).as_slice() {
            [Value::Text(address, _)] => Some(*address),
            _ => None,
        };
        if served.map(py_strip) == Some(dead) {
            overrides.push((book.iri(), vec![Value::Text(replacement, None)]));
        } else {
            let found = served.unwrap_or_default();
            refusals.refuse(Rule::StaleCatalogueOverride, book.iri(), found);
            overrides.push((book.iri(), Vec::new()));
        }
    }
    for (iri, values) in overrides {
        draft.override_values(iri, NAME, values);
    }
}

/// The override the editors curated for `book`, as (the dead address, its replacement); `None`
/// when they curated none, and when what they curated is refused.
///
/// Both keys are read as served: each value is an address, which a curated text read otherwise
/// may not hold. A replacement without the address it replaces could not be retired, so one
/// key without the other is refused. The language of a value is not read.
fn curated_pair<'a>(book: &View<'_, 'a>, refusals: &mut Refusals) -> Option<(&'a str, &'a str)> {
    let replacement = book.curated_served(REPLACEMENT_KEY);
    let dead = book.curated_served(DEAD_KEY);
    let mut too_many = false;
    for (key, values) in [(REPLACEMENT_KEY, &replacement), (DEAD_KEY, &dead)] {
        if values.len() > 1 {
            refusals.refuse(Rule::CuratedCount, book.iri(), key);
            too_many = true;
        }
    }
    if too_many {
        return None;
    }
    let missing = match (dead.first(), replacement.first()) {
        (Some((dead, _)), Some((replacement, _))) => return Some((*dead, *replacement)),
        (None, None) => return None,
        (None, Some(_)) => DEAD_KEY,
        (Some(_), None) => REPLACEMENT_KEY,
    };
    refusals.refuse(Rule::UnpairedCatalogueOverride, book.iri(), missing);
    None
}
