//! The desk-row budget: a boot warning (`HeaderRowOverBudget`) when a
//! project's one-row desk header would need more width than the desk
//! breakpoint leaves it.
//!
//! The estimate mirrors the desk form in `dsp-incubator
//! cpe/static/cpe/shell.css` (the desk query's block): a change to a size, gap
//! or face there moves the estimate, and `estimate_matches_the_measured_rows`
//! pins it to widths measured in Chrome.

use crate::diagnostic::{Diagnostic, DiagnosticKind, Severity, ValidationReport};
use crate::ir::SiteConfig;
use crate::labels::{nav_label, site_title};
use crate::markup_contract::HEADER_DESK_BREAKPOINT_PX;

/// The fluid middle of `--gutter` (`tokens.css`): `3.2vw + 0.4rem`.
const HEADER_DESK_GUTTER_VW: f64 = 3.2;
const HEADER_DESK_GUTTER_BASE_PX: f64 = 0.4 * 16.0;
/// The row's content box at the desk breakpoint, where it is narrowest: the
/// window less two gutters, rounded down (1,170px at 1264px). It holds while
/// the breakpoint is below the frame cap (`--shell-max`, 2200px) and inside the
/// gutter's fluid range (425px to 2300px), both from
/// `dsp-incubator cpe/static/cpe/tokens.css`: a change to either token changes
/// this formula. It assumes overlay scrollbars: a
/// classic scrollbar takes about 15px more.
const HEADER_DESK_BUDGET_PX: u32 = (HEADER_DESK_BREAKPOINT_PX as f64
    - 2.0 * (HEADER_DESK_GUTTER_VW / 100.0 * HEADER_DESK_BREAKPOINT_PX as f64 + HEADER_DESK_GUTTER_BASE_PX))
    as u32;

// Text is a per-face average advance (em) times the character count, so it
// holds for Latin-script text only. IBM Plex Mono's advance is a fixed
// 0.6em; the Newsreader title's average and the two px constants marked
// below are calibrated by `estimate_matches_the_measured_rows`, for
// Newsreader and Plex Mono: a project with another `typeface` gets the same
// constants.
const MONO_ADVANCE_EM: f64 = 0.6;
const TITLE_ADVANCE_EM: f64 = 0.41;

/// `.brand__mark`: "CPE" at `--size-2` (14px), 0.18em tracking, 0.5em
/// inline padding.
const MARK_PX: f64 = 3.0 * (MONO_ADVANCE_EM + 0.18) * 14.0 + 2.0 * 0.5 * 14.0;
/// `.brand__sep`, the "/" between mark and title (calibrated).
const SEP_PX: f64 = 7.5;
/// `.brand`'s gap (`--space-3`), twice: mark–sep, sep–title.
const BRAND_GAPS_PX: f64 = 2.0 * 12.0;
/// `.brand__project` at `--size-4` (19px), plus its 0.15em italic overhang.
const TITLE_SIZE_PX: f64 = 19.0;
const TITLE_OVERHANG_PX: f64 = 0.15 * TITLE_SIZE_PX;
/// The desk row's gap (`--space-6`), twice: brand–nav, nav–controls. The
/// nav element renders even with no items, so both gaps always apply.
const ROW_GAPS_PX: f64 = 2.0 * 32.0;
/// A nav label: `--size-1` (12.5px), uppercase, 0.06em tracking.
const NAV_SIZE_PX: f64 = 12.5;
const NAV_TRACKING_EM: f64 = 0.06;
/// The nav's gap between labels.
const NAV_GAP_PX: f64 = 24.0;
/// The language summary (code and caret; the name is visually hidden;
/// calibrated), plus the controls' 8px gap.
const LANG_SUMMARY_PX: f64 = 51.0 + 8.0;
/// The appearance box.
const APPEARANCE_PX: f64 = 32.0;

/// The estimated desk-row content width for one language: brand, nav and
/// controls with the row's gaps. `languages` is the project's declared
/// language count: the summary renders only for two or more.
fn estimate_desk_row_px(title: &str, nav_labels: &[&str], languages: usize) -> u32 {
    let title_px = title.chars().count() as f64 * TITLE_ADVANCE_EM * TITLE_SIZE_PX + TITLE_OVERHANG_PX;
    let nav_px: f64 = nav_labels
        .iter()
        .map(|label| label.to_uppercase().chars().count() as f64 * (MONO_ADVANCE_EM + NAV_TRACKING_EM) * NAV_SIZE_PX)
        .sum::<f64>()
        + nav_labels.len().saturating_sub(1) as f64 * NAV_GAP_PX;
    let lang_px = if languages > 1 { LANG_SUMMARY_PX } else { 0.0 };
    let total = MARK_PX + BRAND_GAPS_PX + SEP_PX + title_px + ROW_GAPS_PX + nav_px + lang_px + APPEARANCE_PX;
    total.ceil() as u32
}

/// Push one `HeaderRowOverBudget` warning per declared language whose
/// estimated desk row is wider than [`HEADER_DESK_BUDGET_PX`]. The site
/// still renders (the title ellipsises), so this is advisory.
pub(super) fn check_header_row_budget(site: &SiteConfig, report: &mut ValidationReport) {
    for code in &site.languages {
        let lang = crate::lang::resolve(std::slice::from_ref(code), site);
        let title = site_title(site, &lang);
        let labels: Vec<&str> = site.nav.iter().map(|item| nav_label(site, item, &lang)).collect();
        let estimated_px = estimate_desk_row_px(title, &labels, site.languages.len());
        if estimated_px > HEADER_DESK_BUDGET_PX {
            report.push(Diagnostic {
                kind: DiagnosticKind::HeaderRowOverBudget {
                    lang: code.clone(),
                    estimated_px,
                    budget_px: HEADER_DESK_BUDGET_PX,
                },
                message: format!(
                    "the desk header row for '{code}' is estimated at {estimated_px}px, over the \
                     {HEADER_DESK_BUDGET_PX}px it has at the {HEADER_DESK_BREAKPOINT_PX}px desk \
                     breakpoint; the project title will be cut short there. Shorten the site title \
                     or the nav labels"
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_the_window_less_two_gutters() {
        assert_eq!(HEADER_DESK_BUDGET_PX, 1170);
    }

    /// Brand + nav + controls + both gaps, measured in Chrome on the shipped
    /// configs (2026-10-08): Incunabula DE 1,035px, Alice DE 961px, and the
    /// widest shipped row, Alice FR, 1,134px. The estimate must stay within
    /// ±5% of each.
    #[test]
    fn estimate_matches_the_measured_rows() {
        let cases: [(&str, &[&str], usize, f64); 3] = [
            (
                "Die Bilderfolgen der Basler Frühdrucke",
                &["Startseite", "Bücher", "Narrenschiff", "Holzschnitte", "Randleisten"],
                2,
                1035.0,
            ),
            (
                "Alice in DaSCHland",
                &[
                    "Startseite",
                    "Wunderland",
                    "Durch den Spiegel",
                    "Figuren",
                    "Orte",
                    "Kapitel",
                ],
                4,
                961.0,
            ),
            (
                "Alice in DaSCHland",
                &[
                    "Accueil",
                    "Pays des merveilles",
                    "De l'autre côté du miroir",
                    "Personnages",
                    "Lieux",
                    "Chapitres",
                ],
                4,
                1134.0,
            ),
        ];
        for (title, nav, languages, measured) in cases {
            let estimated = f64::from(estimate_desk_row_px(title, nav, languages));
            let off = (estimated - measured).abs() / measured;
            assert!(
                off <= 0.05,
                "{title}: estimated {estimated}px, measured {measured}px ({:.1}% off)",
                off * 100.0
            );
        }
    }

    #[test]
    fn a_single_language_has_no_summary() {
        let two = estimate_desk_row_px("T", &["A"], 2);
        let one = estimate_desk_row_px("T", &["A"], 1);
        assert_eq!(f64::from(two - one), LANG_SUMMARY_PX);
    }
}
