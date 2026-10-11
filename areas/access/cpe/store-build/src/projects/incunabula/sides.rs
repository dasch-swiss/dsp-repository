//! The side of a page a border strip stands on: named by the prefix of the strip's label, and
//! held against the side a Page links the strip on.

use super::{BORDER_STRIP, PAGE};
use crate::error::Rule;
use crate::hook::{Draft, Mapped, Refusals, Value};

/// The list of the two sides in the block, and the property a strip's side is written under.
const LIST: &str = "side";
const NAME: &str = "side";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    const BOTH: [Side; 2] = [Side::Left, Side::Right];

    /// The side a strip's label names by its prefix. A label with neither prefix names none.
    fn of_label(label: &str) -> Option<Side> {
        if label.starts_with("l_") {
            Some(Side::Left)
        } else if label.starts_with("r_") {
            Some(Side::Right)
        } else {
            None
        }
    }

    /// The name of a Page's link to its strip on this side.
    fn link_name(self) -> &'static str {
        match self {
            Side::Left => "strip-left",
            Side::Right => "strip-right",
        }
    }

    /// The side's node in [`LIST`].
    fn node(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// Refuses every Page whose strip on a side is no strip of that side. A strip of no side
/// stands on neither. A link to a strip that gets no row is passed over: the strip was refused.
pub(super) fn refuse_mismatches(draft: &Draft<'_>, refusals: &mut Refusals) {
    for page in draft.entries_of(PAGE) {
        for side in Side::BOTH {
            let Mapped::Values([Value::Link(target)]) = page.values(side.link_name()) else {
                continue;
            };
            let strip = draft.entry(target).filter(|strip| strip.is_kept());
            if strip.is_some_and(|strip| Side::of_label(strip.label()) != Some(side)) {
                refusals.refuse(Rule::StripSideMismatch, page.iri(), target);
            }
        }
    }
}

/// Writes the side of every strip whose label names one. A strip that gets no row gets none
/// for its side either.
pub(super) fn provide<'a>(draft: &mut Draft<'a>) {
    let mut sides: Vec<(&'a str, Side)> = Vec::new();
    for strip in draft.entries_of(BORDER_STRIP) {
        if let Some(side) = Side::of_label(strip.label()) {
            sides.push((strip.iri(), side));
        }
    }
    for (iri, side) in sides {
        draft.provide_list_value(iri, NAME, LIST, side.node());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_of_label_prefix_names_the_side_and_another_label_names_none() {
        assert_eq!(Side::of_label("l_1a"), Some(Side::Left));
        assert_eq!(Side::of_label("r_12"), Some(Side::Right));
        assert_eq!(Side::of_label("[missing]"), None);
        assert_eq!(Side::of_label("L_1"), None);
        assert_eq!(Side::of_label("left"), None);
    }
}
