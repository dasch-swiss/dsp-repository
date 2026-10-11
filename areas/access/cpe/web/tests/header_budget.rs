//! The vendored `cpe-components`' `style/tokens.css` holds the two values the vendored `cpe-core`'s
//! desk-row budget is computed from (`HEADER_DESK_BUDGET_PX` in its
//! `src/validate/header_budget.rs`). dsp-repository has no test that reads both, so the guard is
//! here and runs at every pin move.

use cpe_core::markup_contract::HEADER_DESK_BREAKPOINT_PX;

/// The budget formula reads `tokens.css`: no frame cap below 2200px, and
/// a gutter of `3.2vw + 0.4rem` between its 1.25rem floor (a 425px
/// window) and its 5rem cap (2300px).
#[test]
fn tokens_match_the_budget_formula() {
    const TOKENS_CSS: &str = include_str!("../../components/style/tokens.css");
    let change_both = "tokens.css and the HEADER_DESK_BUDGET_PX formula change together";
    assert!(TOKENS_CSS.contains("--shell-max: 137.5rem;"), "{change_both}");
    assert!(
        TOKENS_CSS.contains("--gutter: clamp(1.25rem, 3.2vw + 0.4rem, 5rem);"),
        "{change_both}"
    );
    assert!(
        (425..2200).contains(&HEADER_DESK_BREAKPOINT_PX),
        "outside this range the frame cap or a gutter clamp applies: {change_both}"
    );
}
