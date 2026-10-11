//! The DOM ids, DataStar expressions and id builders that more than one
//! component emits or reads, defined once so the renderer that writes an id
//! and the expression that queries it cannot drift.
//!
//! A leaf of the component modules, plain constants and one pure fn with no crate imports. A
//! DOM id the boot validator imports lives in `markup_contract` instead. The
//! validator's `RESERVED_HEADING_IDS` spells `PANEL_ID` and `PAGES_TITLE_ID`
//! as literals and the `annotation-` prefix of `entry_id` in
//! `RESERVED_HEADING_ID_PREFIXES`; the `reserved_heading_ids_hold_the_dom_ids`
//! test below fails when they drift apart.

/// Whether the header menu is open as a panel. Guarded: `:popover-open`
/// throws in a selector where popovers are unsupported. Spells
/// `markup_contract::HEADER_MENU_ID` as a literal, pinned by a test in
/// `component::annotation`.
pub(crate) const HEADER_MENU_OPEN_EXPR: &str =
    "(m => m && m.hidePopover && m.matches(':popover-open'))(document.getElementById('header-menu'))";

/// The `<details>` id of the crumb line's dataset panel (`data_links`); a
/// page has one crumb line. The register's Escape
/// (`annotation::esc_unpin_expr`) tests it.
pub(crate) const PANEL_ID: &str = "crumb-data";

/// Escape from any focus inside the open page picker (`compound_viewer`)
/// closes it and returns focus to its summary; the `.orderctl` idiom
/// (`resource_list::render_order_control`). Shared with the crumb dataset
/// panel and the header language menu, so a change here changes all three.
pub const PICKER_ESCAPE_EXPR: &str = "evt.key === 'Escape' && el.open && (evt.preventDefault(), el.open = false, el.querySelector('summary').focus({preventScroll: true}))";

/// Focus leaving the picker for another focusable element closes it (the
/// `.orderctl` idiom). Shared like [`PICKER_ESCAPE_EXPR`].
pub const PICKER_FOCUSOUT_EXPR: &str =
    "evt.relatedTarget && evt.relatedTarget.tabIndex >= 0 && !el.contains(evt.relatedTarget) && (el.open = false)";

/// The id of the `pages` head's `h2` (`compound_viewer`), which the reader
/// article is `aria-labelledby`: a swap's programmatic focus on
/// `#compound-viewer` then lands on a region named by the current page.
/// Fixed, like `id="compound-viewer"`: a page holds at most one navigator.
/// `annotation::ARRIVAL_INIT_EXPR` spells it as a literal and scrolls its
/// parent, so the `h2` must stay a direct child of `header.cv-member__head`
/// (the element `record.css` gives the arrival `scroll-margin-top`).
pub(crate) const PAGES_TITLE_ID: &str = "cv-title";

/// The fragment id of an annotation register entry: its annotation's or
/// link's id, the target a partner link lands on (`LinkResolver::partner_href`).
pub(crate) fn entry_id(id: &str) -> String {
    format!("annotation-{id}")
}

#[cfg(test)]
mod tests {
    use cpe_core::validate::{RESERVED_HEADING_IDS, RESERVED_HEADING_ID_PREFIXES};

    use super::*;

    /// The validator (in `cpe-core`) cannot import these ids, so it spells them as
    /// literals; this pins the literals to the constants.
    #[test]
    fn reserved_heading_ids_hold_the_dom_ids() {
        for id in [PANEL_ID, PAGES_TITLE_ID] {
            assert!(
                RESERVED_HEADING_IDS.contains(&id),
                "validate::RESERVED_HEADING_IDS lacks {id:?}"
            );
        }
        let entry = entry_id("x");
        assert!(
            RESERVED_HEADING_ID_PREFIXES.iter().any(|prefix| entry.starts_with(prefix)),
            "validate::RESERVED_HEADING_ID_PREFIXES lacks the prefix of {entry:?}"
        );
    }
}
