//! The DOM ids and the header's desk breakpoint that both the renderers and
//! the boot validator read: the validator reserves the ids
//! (`validate::RESERVED_HEADING_IDS`) and budgets the desk row at the
//! breakpoint (`validate::header_budget`), the components emit them.
//!
//! A domain leaf with no crate-internal imports, in `cpe-core` (which cannot
//! depend on the component crate), so the validator can read the
//! contract without importing a renderer. A DOM id or size the validator
//! reads is defined here and the renderer imports it; an id several components
//! share goes to `component::dom`, and an id only one renderer uses stays with
//! that renderer. `component::dom::HEADER_MENU_OPEN_EXPR` spells
//! `HEADER_MENU_ID` inside a JS string, pinned by a test in
//! `component::annotation`.

/// The DOM id of the header controls (`div.header__controls`). The full page
/// renders it once, and the bin's `_compound` handler patches it, so the
/// language and appearance hrefs follow the reader's `?page=`.
pub const HEADER_CONTROLS_ID: &str = "header-controls";

/// The DOM id of the header menu (`div[popover]`), the burger's
/// `popovertarget`. It wraps the nav and `#header-controls` and is never a
/// patch target, so a `_compound` swap leaves an open panel open.
pub const HEADER_MENU_ID: &str = "header-menu";

/// The desk breakpoint in CSS px (79rem): from this window width the header
/// is one row with the nav and the controls in it. Three readers move with
/// it: `shell.css`'s `min-width` desk query and its `not all and` complement,
/// which are literals that `component::tests::header_desk_breakpoint_matches_shell_css`
/// pins to this constant; `component::header_menu_resize_expr`; and the
/// boot budget (`validate/header_budget.rs`), which is computed at this width.
pub const HEADER_DESK_BREAKPOINT_PX: u32 = 1264;
// The stylesheet and the resize handler spell the breakpoint in whole rem.
const _: () = assert!(
    HEADER_DESK_BREAKPOINT_PX.is_multiple_of(16),
    "shell.css and matchMedia spell the desk breakpoint in whole rem: use a multiple of 16"
);

/// The id of the page rail's caption, which names its `nav`. Boot reserves
/// it (`validate::RESERVED_HEADING_IDS`), so no heading can take it.
pub const RAIL_CAPTION_ID: &str = "narr-rail-cap";
