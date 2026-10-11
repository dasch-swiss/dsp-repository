//! A Book's `sort-key`: what the site orders its titles by. The editors' key where they curated
//! one, else the Book's name.

use super::BOOK;
use crate::error::Rule;
use crate::generic::text::py_strip;
use crate::hook::{Draft, Mapped, Refusals, Value, View};

/// The curated key the editors write a sort key under, and the property it is written as.
const KEY: &str = "sort_key";
const NAME: &str = "sort-key";

/// Writes the sort key of every Book that has a name.
///
/// The site orders titles ignoring case in ASCII only: a key that starts outside ASCII would
/// be filed after every other, and is refused. What the editors curated under the key is
/// checked for every Book; a Book without a name gets no key and is not refused for one.
pub(super) fn provide<'a>(draft: &mut Draft<'a>, refusals: &mut Refusals) {
    let mut keys: Vec<(&'a str, &'a str)> = Vec::new();
    for book in draft.entries_of(BOOK) {
        let curated = curated_key(&book, refusals);
        let name = match book.values("name") {
            Mapped::Values([Value::Text(name, _)]) => *name,
            Mapped::Values(_) => continue,
            // The label is the name of a Book whose curated title was refused, when the label
            // may be a name: the mapping does not read it while a curated title stands in.
            Mapped::Refused(_) if !book.label_is_text() => continue,
            Mapped::Refused(_) => book.label(),
        };
        let key = curated.unwrap_or(name);
        if py_strip(key).starts_with(|first: char| first.is_ascii()) {
            keys.push((book.iri(), key));
        } else {
            refusals.refuse(Rule::SortKeyNotAscii, book.iri(), key);
        }
    }
    for (iri, key) in keys {
        draft.provide_text(iri, NAME, key, None);
    }
}

/// The one sort key the editors curated for `book`; `None` when they curated none, and when
/// what they curated is refused: the Book's name is the key then.
///
/// A key is checked for whitespace around it before anything else, so one that is whitespace
/// alone is refused and not taken for none. The language of a key is not read.
fn curated_key<'a>(book: &View<'_, 'a>, refusals: &mut Refusals) -> Option<&'a str> {
    let served = book.curated_served(KEY);
    let padded = served.iter().find(|(text, _)| py_strip(text) != *text);
    if let Some((padded, _)) = padded {
        refusals.refuse(Rule::PaddedSortKey, book.iri(), padded);
        return None;
    }
    match book.curated(KEY)?.as_slice() {
        [] => None,
        [(key, _)] => Some(*key),
        _ => {
            refusals.refuse(Rule::CuratedCount, book.iri(), KEY);
            None
        }
    }
}
