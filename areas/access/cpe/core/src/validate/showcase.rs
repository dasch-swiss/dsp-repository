//! Lowering and boot checks of Incunabula's showcase figures:
//! `woodcut-cases`, `edition-cards` and `strip-row`, each reached through its
//! `validate::project_components` row.
//!
//! The woodcut data checks keep every copied string verbatim from the store: a crop
//! `alt` and a `remark-excerpt` must be part of one line of a page
//! description, a `detail` `alt` the detail annotation's own remark, and a
//! `schramm=` number the first page's catalogue comment. Both
//! `validate_with_data` arms run them through [`check_cases`].

use std::collections::HashSet;

use super::attrs::{
    first_positional_string, get_string_attr, get_typed_integer_attr, get_typed_string_attr,
    require_first_positional_string, require_integer_attr, require_string_attr,
};
use super::context::ValidationContext;
use super::facts::{push_internal_query_error, LinkTargetRow, ProjectFacts, ShowcaseAnnotationRow};
use super::media_property::check_single_media_property;
use super::parts::{check_role_value_types, lower_imprint, part_roles};
use crate::diagnostic::{Diagnostic, DiagnosticKind, ValidationReport};
use crate::iiif::iiif_info_json_url;
use crate::ir::{CaseLayout, ChromeString, Component, EditionCard, Page, ShowcaseCase, ShowcaseCrop, ShowcaseRemark};
use crate::util::is_safe_external_url;

/// The Page property holding the description an `alt` or excerpt quotes.
const DESCRIPTION_PROPERTY: &str = "description";
/// The Page property holding the Schramm reference (`… Abb. 1124.`).
const SCHRAMM_PROPERTY: &str = "comment";
/// A case sets two to four impressions in its row.
const CROPS_PER_CASE: std::ops::RangeInclusive<usize> = 2..=4;
/// A page case joins exactly two pages.
const PAGES_PER_CASE: usize = 2;
/// The most tiles a `strip-row` lists: the store hydrates each one and binds
/// every id into one `IN (…)` query, so the row is a teaser, not a browse.
const MAX_STRIP_ROW_LIMIT: u32 = 100;

// ── Lowering ──────────────────────────────────────────────────────────────────

/// Lower a `woodcut-cases` node. `section=` is required here by presence
/// only: `lower_page_body`'s slot reader owns its type check. `media=` and
/// `layout=` are required; every case is lowered and checked even after an
/// earlier one failed, so one boot pass reports them all.
pub(crate) fn lower_woodcut_cases(
    raw: &crate::config::Component,
    _ctx: &ValidationContext<'_>,
    report: &mut ValidationReport,
) -> Option<Component> {
    require_section(raw, report);
    let media = require_string_attr(raw, "media", report);
    let layout = require_string_attr(raw, "layout", report).and_then(|value| {
        let layout = CaseLayout::parse(&value);
        if layout.is_none() {
            invalid_case(
                raw,
                None,
                format!("woodcut-cases layout \"{value}\" is not one of edition, four, compact"),
                report,
            );
        }
        layout
    });

    let mut cases = Vec::new();
    let mut keys: HashSet<String> = HashSet::new();
    let mut lowered_all = true;
    for child in &raw.children {
        let Some(case) = lower_case(child, layout, report) else {
            lowered_all = false;
            continue;
        };
        if !keys.insert(case.key().to_owned()) {
            invalid_case(
                child,
                Some(case.key()),
                format!(
                    "woodcut-cases has two cases keyed '{}' (a case's first crop, a page case's \
                     link); the key is the case's DOM id",
                    case.key()
                ),
                report,
            );
            continue;
        }
        cases.push(case);
    }
    if cases.is_empty() && lowered_all {
        invalid_case(raw, None, "woodcut-cases has no case".to_owned(), report);
    }

    Some(Component::WoodcutCases { media: media?, layout: layout?, cases })
}

/// `MissingRequiredAttribute` unless `raw` carries `section=`: a showcase
/// figure renders only inside a narrative section. By presence only:
/// `lower_page_body`'s slot reader owns its type check.
fn require_section(raw: &crate::config::Component, report: &mut ValidationReport) {
    if !raw.attrs.iter().any(|(k, _)| k == "section") {
        report.push(Diagnostic::error(
            DiagnosticKind::MissingRequiredAttribute,
            format!(
                "component '{}' is missing the required 'section' attribute: it renders only \
                 inside a narrative section",
                raw.name
            ),
            raw.span,
        ));
    }
}

/// Lower an `edition-cards` node: `section=` (presence) and `media=` are
/// required; each `book "<id>" lang-label="…"` child is a card, in order,
/// and at most one `imprint { part … }` child gives the imprint line. A
/// repeated book is `InvalidAttributeType`, a block with no book
/// `MissingRequiredAttribute`. The chrome ids are checked later
/// ([`check_showcase_labels`]).
pub(crate) fn lower_edition_cards(
    raw: &crate::config::Component,
    _ctx: &ValidationContext<'_>,
    report: &mut ValidationReport,
) -> Option<Component> {
    require_section(raw, report);
    let media = require_string_attr(raw, "media", report);
    let mut books: Vec<EditionCard> = Vec::new();
    let mut imprint = None;
    let mut cards_ok = true;
    for child in &raw.children {
        match child.name.as_str() {
            "book" => {
                let book = require_first_positional_string(child, report);
                let lang_label = get_typed_string_attr(child, "lang-label", report);
                let Some(book) = book else {
                    cards_ok = false;
                    continue;
                };
                if books.iter().any(|c| c.book == book) {
                    report.push(Diagnostic::error(
                        DiagnosticKind::InvalidAttributeType,
                        format!("edition-cards names the book '{book}' more than once"),
                        child.span,
                    ));
                    continue;
                }
                books.push(EditionCard { book, lang_label });
            }
            "imprint" if imprint.is_none() => imprint = Some(lower_imprint(child, report)),
            "imprint" => report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                "edition-cards has more than one 'imprint'; only one is allowed",
                child.span,
            )),
            other => unknown_child(child, "edition-cards", other, report),
        }
    }
    if books.is_empty() && cards_ok {
        report.push(Diagnostic::error(
            DiagnosticKind::MissingRequiredAttribute,
            "edition-cards needs at least one 'book'",
            raw.span,
        ));
    }
    Some(Component::EditionCards { media: media?, books, imprint: imprint.unwrap_or_default() })
}

/// Lower a `strip-row` node: `section=` (presence), `class=`, `media=`,
/// `uses-property=`, `min-uses=` (≥ 0) and `limit=` (1 to
/// [`MAX_STRIP_ROW_LIMIT`]) are all required.
pub(crate) fn lower_strip_row(
    raw: &crate::config::Component,
    _ctx: &ValidationContext<'_>,
    report: &mut ValidationReport,
) -> Option<Component> {
    require_section(raw, report);
    let class = require_string_attr(raw, "class", report);
    let media = require_string_attr(raw, "media", report);
    let uses_property = require_string_attr(raw, "uses-property", report);
    let min_uses =
        ranged_integer(raw, "min-uses", 0..=i128::from(i64::MAX), report).and_then(|n| i64::try_from(n).ok());
    let limit =
        ranged_integer(raw, "limit", 1..=i128::from(MAX_STRIP_ROW_LIMIT), report).and_then(|n| u32::try_from(n).ok());
    Some(Component::StripRow {
        class: class?,
        media: media?,
        uses_property: uses_property?,
        min_uses: min_uses?,
        limit: limit?,
    })
}

/// A required integer attribute inside `range`; out of range is
/// `InvalidAttributeType`.
fn ranged_integer(
    raw: &crate::config::Component,
    key: &str,
    range: std::ops::RangeInclusive<i128>,
    report: &mut ValidationReport,
) -> Option<i128> {
    let n = require_integer_attr(raw, key, report)?;
    if range.contains(&n) {
        return Some(n);
    }
    report.push(Diagnostic::error(
        DiagnosticKind::InvalidAttributeType,
        format!(
            "{} attribute '{key}' must be between {} and {}, got {n}",
            raw.name,
            range.start(),
            range.end()
        ),
        raw.span,
    ));
    None
}

/// Lower one child of a `woodcut-cases` node. `layout` is `None` when the
/// block's own layout did not lower (or, in the raw arm, is not read): the
/// compact-only rule is then skipped.
fn lower_case(
    raw: &crate::config::Component,
    layout: Option<CaseLayout>,
    report: &mut ValidationReport,
) -> Option<ShowcaseCase> {
    match raw.name.as_str() {
        "case" => lower_crop_case(raw, layout, report),
        "page-case" => lower_page_case(raw, report),
        other => {
            unknown_child(raw, "woodcut-cases", other, report);
            None
        }
    }
}

fn lower_crop_case(
    raw: &crate::config::Component,
    layout: Option<CaseLayout>,
    report: &mut ValidationReport,
) -> Option<ShowcaseCase> {
    let schramm = match get_typed_integer_attr(raw, "schramm", report) {
        None => Ok(None),
        Some(n) => match u32::try_from(n) {
            Ok(n) if n > 0 => Ok(Some(n)),
            _ => {
                report.push(Diagnostic::error(
                    DiagnosticKind::InvalidAttributeType,
                    format!("case attribute 'schramm' must be a positive integer, got {n}"),
                    raw.span,
                ));
                Err(())
            }
        },
    };
    let title = get_typed_string_attr(raw, "title", report);

    let mut crops = Vec::new();
    let mut details = Vec::new();
    let mut children_ok = true;
    for child in &raw.children {
        let target = match child.name.as_str() {
            "crop" => &mut crops,
            "detail" => &mut details,
            other => {
                unknown_child(child, "case", other, report);
                children_ok = false;
                continue;
            }
        };
        match lower_crop(child, report) {
            Some(crop) => target.push(crop),
            None => children_ok = false,
        }
    }
    let key = crops.first().map(|c: &ShowcaseCrop| c.annotation.clone());
    let remark = lower_remark(raw, key.as_deref(), report);
    let mut shape_ok = true;
    if children_ok && !CROPS_PER_CASE.contains(&crops.len()) {
        invalid_case(
            raw,
            key.as_deref(),
            format!(
                "a case sets {} to {} crops in its row, this one has {}",
                CROPS_PER_CASE.start(),
                CROPS_PER_CASE.end(),
                crops.len()
            ),
            report,
        );
        shape_ok = false;
    }
    if details.len() > 1 {
        invalid_case(raw, key.as_deref(), "a case has at most one detail".to_owned(), report);
        shape_ok = false;
    }
    if layout == Some(CaseLayout::Compact) && !details.is_empty() {
        invalid_case(raw, key.as_deref(), "a compact block shows no detail crop".to_owned(), report);
        shape_ok = false;
    }

    if !(children_ok && shape_ok) {
        return None;
    }
    Some(ShowcaseCase::Crops {
        schramm: schramm.ok()?,
        title,
        remark: remark.ok()?,
        crops,
        detail: details.pop(),
    })
}

fn lower_page_case(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<ShowcaseCase> {
    let link = require_string_attr(raw, "link", report);
    let remark = lower_remark(raw, link.as_deref(), report);
    let mut pages = Vec::new();
    let mut children_ok = true;
    for child in &raw.children {
        if child.name != "page" {
            unknown_child(child, "page-case", &child.name, report);
            children_ok = false;
            continue;
        }
        match require_first_positional_string(child, report) {
            Some(id) => pages.push(id),
            None => children_ok = false,
        }
    }
    if children_ok && pages.len() != PAGES_PER_CASE {
        invalid_case(
            raw,
            link.as_deref(),
            format!("a page case joins {PAGES_PER_CASE} pages, this one names {}", pages.len()),
            report,
        );
        return None;
    }
    if !children_ok {
        return None;
    }
    Some(ShowcaseCase::Pages { link: link?, remark: remark.ok()?, pages })
}

/// One `crop "<annotation id>" alt="…"` (or `detail`) node.
fn lower_crop(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<ShowcaseCrop> {
    let annotation = require_first_positional_string(raw, report);
    let alt = get_typed_string_attr(raw, "alt", report);
    Some(ShowcaseCrop { annotation: annotation?, alt })
}

/// A case's remark source: at most one of `remark-link=`, `remark=` and
/// `remark-excerpt=`. `Err` when one is wrong-typed or several are set (both
/// reported). `key` is the case's key as lowered, for the diagnostic.
fn lower_remark(
    raw: &crate::config::Component,
    key: Option<&str>,
    report: &mut ValidationReport,
) -> Result<Option<ShowcaseRemark>, ()> {
    let sources = [
        ("remark-link", ShowcaseRemark::Link as fn(String) -> ShowcaseRemark),
        ("remark", ShowcaseRemark::Chrome),
        ("remark-excerpt", ShowcaseRemark::Excerpt),
    ];
    let mut found = Vec::new();
    let mut typed_ok = true;
    for (key, make) in sources {
        if !raw.attrs.iter().any(|(k, _)| k == key) {
            continue;
        }
        match get_typed_string_attr(raw, key, report) {
            Some(value) => found.push(make(value)),
            None => typed_ok = false,
        }
    }
    if found.len() > 1 {
        invalid_case(
            raw,
            key,
            "a case takes at most one of remark-link, remark and remark-excerpt".to_owned(),
            report,
        );
        return Err(());
    }
    if !typed_ok {
        return Err(());
    }
    Ok(found.pop())
}

fn invalid_case(raw: &crate::config::Component, key: Option<&str>, message: String, report: &mut ValidationReport) {
    report.push(Diagnostic::error(
        DiagnosticKind::InvalidShowcaseCase { case: key.map(str::to_owned) },
        message,
        raw.span,
    ));
}

fn unknown_child(raw: &crate::config::Component, component: &str, child: &str, report: &mut ValidationReport) {
    report.push(Diagnostic::error(
        DiagnosticKind::UnknownChildNode { component: component.to_owned(), child: child.to_owned() },
        format!("{component} has an unknown child node '{child}'"),
        raw.span,
    ));
}

// ── Config-internal page passes ───────────────────────────────────────────────

/// `InvalidShowcaseCase` for a case key used by two blocks of one page (a
/// repeat inside one block is lowering's). Config-internal, run per page in
/// `check_and_lower_pages`.
pub(super) fn check_duplicate_case_keys(body: &[Component], report: &mut ValidationReport) {
    let mut seen: HashSet<&str> = HashSet::new();
    for component in body {
        let Component::WoodcutCases { cases, .. } = component else {
            continue;
        };
        for case in cases {
            if !seen.insert(case.key()) {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::InvalidShowcaseCase { case: Some(case.key().to_owned()) },
                    format!(
                        "two showcase cases on one page are keyed '{}'; the key is the case's \
                         DOM id",
                        case.key()
                    ),
                ));
            }
        }
    }
}

/// The showcase figures' chrome ids: `UnknownShowcaseLabel` for a case
/// `remark=` or an edition card's `lang-label=` that names no declared
/// chrome string (the text would silently drop). An edition imprint's
/// `unit=` is `check_gloss_units`' (the record header's rule).
/// Config-internal, run in `validate_inner` beside `check_figure_labels`.
pub(super) fn check_showcase_labels(pages: &[Page], chrome: &[ChromeString], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            // `(attribute, chrome id, what declares it)`, each to resolve.
            let mut labels: Vec<(&str, &str, String)> = Vec::new();
            match component {
                Component::WoodcutCases { cases, .. } => {
                    for case in cases {
                        if let Some(ShowcaseRemark::Chrome(label)) = case.remark() {
                            labels.push(("remark", label, format!("showcase case '{}'", case.key())));
                        }
                    }
                }
                Component::EditionCards { books, .. } => {
                    for card in books {
                        if let Some(label) = &card.lang_label {
                            labels.push(("lang-label", label, format!("edition card '{}'", card.book)));
                        }
                    }
                }
                _ => continue,
            }
            for (attribute, label, owner) in labels {
                if chrome.iter().any(|cs| cs.id == label) {
                    continue;
                }
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::UnknownShowcaseLabel { label: label.to_owned(), page: page.route.clone() },
                    format!(
                        "page '{}': {owner} has {attribute}=\"{label}\", which is not a declared \
                         chrome string id",
                        page.route
                    ),
                ));
            }
        }
    }
}

// ── Data checks ───────────────────────────────────────────────────────────────

/// The success-arm data checks of one lowered `woodcut-cases` on page
/// `route` (its registry row's `check_data`); other variants are ignored.
pub(crate) fn check_woodcut_cases_data(
    component: &Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    if let Component::WoodcutCases { media, cases, .. } = component {
        check_cases(Some(media), cases, route, data, report);
    }
}

/// Raw sibling of [`check_woodcut_cases_data`], for the failure arm. Each
/// case is lowered into a scratch report that is dropped: lowering already
/// reported every defect, and a case that does not lower is skipped. A raw
/// node without `media=` skips only the image-size check.
pub(crate) fn check_woodcut_cases_data_into_raw(
    raw: &crate::config::Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let mut scratch = ValidationReport::new();
    let cases: Vec<ShowcaseCase> = raw
        .children
        .iter()
        .filter_map(|child| lower_case(child, None, &mut scratch))
        .collect();
    let media = get_string_attr(raw, "media");
    check_cases(media.as_deref(), &cases, route, data, report);
}

/// The shared body of both arms. `media` is `None` only in the raw arm.
fn check_cases(
    media: Option<&str>,
    cases: &[ShowcaseCase],
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for case in cases {
        match case {
            ShowcaseCase::Crops { schramm, remark, crops, detail, .. } => check_crop_case(
                CropCase {
                    key: case.key(),
                    schramm: *schramm,
                    remark: remark.as_ref(),
                    crops,
                    detail: detail.as_ref(),
                },
                media,
                route,
                data,
                report,
            ),
            ShowcaseCase::Pages { link, remark, pages } => {
                check_page_case(link, remark.as_ref(), pages, route, data, report)
            }
        }
    }
}

/// The parts of a `ShowcaseCase::Crops` its check reads.
struct CropCase<'a> {
    key: &'a str,
    schramm: Option<u32>,
    remark: Option<&'a ShowcaseRemark>,
    crops: &'a [ShowcaseCrop],
    detail: Option<&'a ShowcaseCrop>,
}

fn check_crop_case(
    case: CropCase<'_>,
    media: Option<&str>,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // The annotated resource of each crop that exists, in row order.
    let mut pages: Vec<Option<String>> = Vec::with_capacity(case.crops.len());
    for crop in case.crops {
        let row = check_crop(crop, media, route, data, report);
        if let (Some(row), Some(alt)) = (&row, &crop.alt) {
            let texts = texts_of(&row.resource_id, DESCRIPTION_PROPERTY, route, data, report);
            if !in_one_line(&texts, alt) {
                excerpt_mismatch(alt, case.key, route, report);
            }
        }
        pages.push(row.map(|r| r.resource_id));
    }
    if let Some(detail) = case.detail {
        let row = check_crop(detail, media, route, data, report);
        if let (Some(row), Some(alt)) = (row, &detail.alt) {
            if row.remark.as_deref().map(str::trim) != Some(alt.trim()) {
                excerpt_mismatch(alt, case.key, route, report);
            }
        }
    }

    let crop_ids: Vec<&str> = case.crops.iter().map(|c| c.annotation.as_str()).collect();
    match case.remark {
        Some(ShowcaseRemark::Link(link)) => {
            if let Some(targets) = link_targets_or_report(link, route, data, report) {
                let joins = targets
                    .iter()
                    .filter_map(|t| t.annotation_id.as_deref())
                    .any(|a| crop_ids.contains(&a));
                if !joins {
                    link_mismatch(link, case.key, route, report);
                }
            }
        }
        Some(ShowcaseRemark::Excerpt(excerpt)) => check_excerpt(
            excerpt,
            pages.iter().flatten().map(String::as_str),
            case.key,
            route,
            data,
            report,
        ),
        Some(ShowcaseRemark::Chrome(_)) | None => {}
    }

    if let (Some(n), Some(Some(first))) = (case.schramm, pages.first()) {
        let comments = texts_of(first, SCHRAMM_PROPERTY, route, data, report);
        if !comments.iter().any(|c| names_schramm(c, n)) {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::ShowcaseSchrammMismatch {
                    schramm: n,
                    case: case.key.to_owned(),
                    page: route.to_owned(),
                },
                format!(
                    "page '{route}': showcase case '{}' declares schramm={n}, but the comment of \
                     its first crop's page names no 'Abb. {n}'",
                    case.key
                ),
            ));
        }
    }
}

/// One crop's existence and, with `media`, its image size and URI shape. The
/// row when the annotation exists.
fn check_crop(
    crop: &ShowcaseCrop,
    media: Option<&str>,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> Option<ShowcaseAnnotationRow> {
    let row = match data.showcase_annotation(&crop.annotation, media.unwrap_or_default()) {
        Ok(row) => row,
        Err(e) => {
            push_internal_query_error(
                report,
                "showcase annotation",
                format!("page '{route}': could not read showcase annotation '{}': {e}", crop.annotation),
            );
            return None;
        }
    };
    let Some(row) = row else {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::UnknownShowcaseAnnotation { annotation: crop.annotation.clone(), page: route.to_owned() },
            format!(
                "page '{route}': showcase crop '{}' is not an annotation in the data",
                crop.annotation
            ),
        ));
        return None;
    };
    let sized = |v: Option<i64>| v.is_some_and(|v| v > 0);
    if media.is_none() {
        return Some(row);
    }
    if !(sized(row.width) && sized(row.height)) {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::MissingCropDimensions { annotation: crop.annotation.clone(), page: route.to_owned() },
            format!(
                "page '{route}': showcase crop '{}': the image of resource '{}' has no stored \
                 width and height, so the crop cannot be sized",
                crop.annotation, row.resource_id
            ),
        ));
    } else {
        let uri = row.uri.as_deref().unwrap_or_default();
        if !is_crop_source(uri) {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::CropImageNotIiif { annotation: crop.annotation.clone(), page: route.to_owned() },
                format!(
                    "page '{route}': showcase crop '{}': the image URI '{uri}' of resource '{}' \
                     is not a safe (http, https or root-relative) IIIF '/full/max/' URL, so no \
                     crop region can be requested",
                    crop.annotation, row.resource_id
                ),
            ));
        }
    }
    Some(row)
}

/// Whether the renderer can request a region of `uri`: the crop renderer's
/// URL gate (`is_safe_external_url`) and `iiif_region_url`'s `/full/max/`
/// segment (the split `iiif_info_json_url` also makes).
fn is_crop_source(uri: &str) -> bool {
    is_safe_external_url(uri) && iiif_info_json_url(uri).is_some()
}

fn check_page_case(
    link: &str,
    remark: Option<&ShowcaseRemark>,
    pages: &[String],
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let mut known: Vec<&str> = Vec::with_capacity(pages.len());
    for page in pages {
        match data.resource_exists(page) {
            Ok(true) => known.push(page),
            Ok(false) => report.push(Diagnostic::error_spanless(
                DiagnosticKind::UnknownShowcasePage { id: page.clone(), page: route.to_owned() },
                format!("page '{route}': showcase page '{page}' is not a resource in the data"),
            )),
            Err(e) => push_internal_query_error(
                report,
                "showcase page existence",
                format!("page '{route}': could not check showcase page '{page}': {e}"),
            ),
        }
    }

    // An unknown page is already reported; comparing would add a second
    // diagnostic for the same mistake.
    let targets = link_targets_or_report(link, route, data, report);
    if let Some(targets) = targets.filter(|_| known.len() == pages.len()) {
        let mut targeted: Vec<&str> = targets.iter().filter_map(|t| t.resource_id.as_deref()).collect();
        let mut named: Vec<&str> = pages.iter().map(String::as_str).collect();
        targeted.sort_unstable();
        targeted.dedup();
        named.sort_unstable();
        if targeted != named {
            link_mismatch(link, link, route, report);
        }
    }

    match remark {
        Some(ShowcaseRemark::Link(remark_link)) => {
            if let Some(targets) = link_targets_or_report(remark_link, route, data, report) {
                let joins = targets
                    .iter()
                    .filter_map(|t| t.resource_id.as_deref())
                    .any(|r| pages.iter().any(|p| p == r));
                if !joins {
                    link_mismatch(remark_link, link, route, report);
                }
            }
        }
        Some(ShowcaseRemark::Excerpt(excerpt)) => {
            check_excerpt(excerpt, known.iter().copied(), link, route, data, report)
        }
        Some(ShowcaseRemark::Chrome(_)) | None => {}
    }
}

/// The success-arm data checks of one lowered `edition-cards` (its registry
/// row's `check_data`); other variants are ignored.
pub(crate) fn check_edition_cards_data(
    component: &Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    if let Component::EditionCards { media, books, imprint } = component {
        let books: Vec<&str> = books.iter().map(|c| c.book.as_str()).collect();
        let imprint: Vec<(&str, bool)> = imprint.iter().map(|p| (p.property.as_str(), p.unit.is_some())).collect();
        check_edition_cards(Some(media), &books, &imprint, route, data, report);
    }
}

/// Raw sibling of [`check_edition_cards_data`], for the failure arm: reads
/// what it can with the silent readers and skips the rest (lowering already
/// reported it). Only the first `imprint` counts, as in lowering.
pub(crate) fn check_edition_cards_data_into_raw(
    raw: &crate::config::Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let media = get_string_attr(raw, "media");
    let books: Vec<String> = raw
        .children
        .iter()
        .filter(|c| c.name == "book")
        .filter_map(first_positional_string)
        .collect();
    let parts: Vec<(String, bool)> = raw
        .children
        .iter()
        .find(|c| c.name == "imprint")
        .map(|block| {
            block
                .children
                .iter()
                .filter(|c| c.name == "part")
                .filter_map(|part| {
                    let property = get_string_attr(part, "property")?;
                    Some((property, part.attrs.iter().any(|(k, _)| k == "unit")))
                })
                .collect()
        })
        .unwrap_or_default();
    let books: Vec<&str> = books.iter().map(String::as_str).collect();
    let imprint: Vec<(&str, bool)> = parts.iter().map(|(p, u)| (p.as_str(), *u)).collect();
    check_edition_cards(media.as_deref(), &books, &imprint, route, data, report);
}

/// The shared body of both `edition-cards` arms: every book exists
/// (`UnknownShowcaseBook`), and on each distinct class of the books that do,
/// `media` passes the shared media checks (`check_single_media_property`: a
/// link to an image-like class) and every imprint part
/// renders (`IncompatibleGlossPart`, the record imprint's rule). `media` is
/// `None` only in the raw arm.
fn check_edition_cards(
    media: Option<&str>,
    books: &[&str],
    imprint: &[(&str, bool)],
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let mut classes: Vec<String> = Vec::new();
    for &book in books {
        match data.class_of(book) {
            Ok(Some(class)) => {
                if !classes.contains(&class) {
                    classes.push(class);
                }
            }
            Ok(None) => report.push(Diagnostic::error_spanless(
                DiagnosticKind::UnknownShowcaseBook { book: book.to_owned(), page: route.to_owned() },
                format!("page '{route}': edition card book '{book}' is not a resource in the data"),
            )),
            Err(e) => push_internal_query_error(
                report,
                "edition card book",
                format!("page '{route}': could not read edition card book '{book}': {e}"),
            ),
        }
    }
    for class in &classes {
        if let Some(media) = media {
            check_single_media_property(class, media, route, data, report);
        }
        check_role_value_types(class, &part_roles(imprint, "imprint "), "edition-cards", route, data, report);
    }
}

/// The success-arm data checks of one lowered `strip-row` (its registry
/// row's `check_data`); other variants are ignored.
pub(crate) fn check_strip_row_data(
    component: &Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    if let Component::StripRow { class, media, uses_property, .. } = component {
        check_strip_row(class, Some(media), Some(uses_property), route, data, report);
    }
}

/// Raw sibling of [`check_strip_row_data`]: a node without `class=` is
/// skipped, a missing `media=` or `uses-property=` skips only its check.
pub(crate) fn check_strip_row_data_into_raw(
    raw: &crate::config::Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let Some(class) = get_string_attr(raw, "class") else {
        return;
    };
    let media = get_string_attr(raw, "media");
    let uses = get_string_attr(raw, "uses-property");
    check_strip_row(&class, media.as_deref(), uses.as_deref(), route, data, report);
}

/// The shared body of both `strip-row` arms: the class exists
/// (`UnknownShowcaseClass`), and on it `uses-property` is an integer
/// (`IncompatibleShowcaseProperty`) and `media` passes the shared media checks
/// (`check_single_media_property`).
fn check_strip_row(
    class: &str,
    media: Option<&str>,
    uses_property: Option<&str>,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    match data.class_exists(class) {
        Ok(true) => {}
        Ok(false) => {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::UnknownShowcaseClass { class: class.to_owned(), page: route.to_owned() },
                format!("page '{route}': strip-row class '{class}' does not exist in the data"),
            ));
            return;
        }
        Err(e) => {
            push_internal_query_error(
                report,
                "strip-row class existence",
                format!("page '{route}': could not check strip-row class '{class}': {e}"),
            );
            return;
        }
    }
    if let Some(uses) = uses_property {
        check_property_type(class, uses, "uses-property", "integer", route, data, report);
    }
    if let Some(media) = media {
        check_single_media_property(class, media, route, data, report);
    }
}

/// `IncompatibleShowcaseProperty` unless every stored value of `property` on
/// `class` is of `value_type` and there is at least one.
fn check_property_type(
    class: &str,
    property: &str,
    role: &str,
    value_type: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let types = match data.property_value_types_on_class(class, property) {
        Ok(types) => types,
        Err(e) => {
            push_internal_query_error(
                report,
                "showcase property value type",
                format!(
                    "page '{route}': could not check the value type of '{property}' on class \
                     '{class}': {e}"
                ),
            );
            return;
        }
    };
    if !types.is_empty() && types.iter().all(|t| t == value_type) {
        return;
    }
    let found = if types.is_empty() {
        "no value".to_owned()
    } else {
        let mut found: Vec<&str> = types.iter().map(String::as_str).collect();
        found.sort_unstable();
        found.join(", ")
    };
    report.push(Diagnostic::error_spanless(
        DiagnosticKind::IncompatibleShowcaseProperty {
            property: property.to_owned(),
            role: role.to_owned(),
            class: class.to_owned(),
            page: route.to_owned(),
        },
        format!(
            "page '{route}': {role} '{property}' on class '{class}' must be a {value_type} \
             property, found {found}"
        ),
    ));
}

/// `ShowcaseExcerptMismatch` unless `excerpt` is part of one line of the
/// description of one of `pages` (the case's pages that exist).
fn check_excerpt<'p>(
    excerpt: &str,
    pages: impl IntoIterator<Item = &'p str>,
    case: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let found = pages
        .into_iter()
        .any(|page| in_one_line(&texts_of(page, DESCRIPTION_PROPERTY, route, data, report), excerpt));
    if !found {
        excerpt_mismatch(excerpt, case, route, report);
    }
}

/// The targets of `link`, or `None` after reporting it unknown (or a query
/// failure).
fn link_targets_or_report(
    link: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> Option<Vec<LinkTargetRow>> {
    match data.link_targets(link) {
        Ok(Some(targets)) => Some(targets),
        Ok(None) => {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::UnknownShowcaseLink { link: link.to_owned(), page: route.to_owned() },
                format!("page '{route}': showcase link '{link}' is not an annotation link in the data"),
            ));
            None
        }
        Err(e) => {
            push_internal_query_error(
                report,
                "showcase link",
                format!("page '{route}': could not read showcase link '{link}': {e}"),
            );
            None
        }
    }
}

/// The text values of `resource`'s `property`; a query failure is reported
/// and reads as no text.
fn texts_of(
    resource: &str,
    property: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> Vec<String> {
    data.property_texts(resource, property).unwrap_or_else(|e| {
        push_internal_query_error(
            report,
            "showcase page text",
            format!("page '{route}': could not read '{property}' of '{resource}': {e}"),
        );
        Vec::new()
    })
}

fn link_mismatch(link: &str, case: &str, route: &str, report: &mut ValidationReport) {
    report.push(Diagnostic::error_spanless(
        DiagnosticKind::ShowcaseLinkMismatch {
            link: link.to_owned(),
            case: case.to_owned(),
            page: route.to_owned(),
        },
        format!(
            "page '{route}': showcase link '{link}' does not join case '{case}': its targets are \
             not the case's crops or pages"
        ),
    ));
}

fn excerpt_mismatch(text: &str, case: &str, route: &str, report: &mut ValidationReport) {
    report.push(Diagnostic::error_spanless(
        DiagnosticKind::ShowcaseExcerptMismatch {
            text: text.to_owned(),
            case: case.to_owned(),
            page: route.to_owned(),
        },
        format!(
            "page '{route}': showcase case '{case}' quotes \"{text}\", which is not verbatim in \
             its source (a line of the page description, or the detail's own remark)"
        ),
    ));
}

/// Whether `excerpt` is part of one line (`\n`-separated, trimmed) of one of
/// `texts`. An empty excerpt never is.
fn in_one_line(texts: &[String], excerpt: &str) -> bool {
    !excerpt.trim().is_empty()
        && texts
            .iter()
            .flat_map(|t| t.split('\n'))
            .any(|line| line.trim().contains(excerpt))
}

/// Whether `comment` names `Abb. <n>` as a whole number (`Abb. 112` does not
/// match `Abb. 1124`).
fn names_schramm(comment: &str, n: u32) -> bool {
    let needle = format!("Abb. {n}");
    comment
        .match_indices(&needle)
        .any(|(i, _)| !comment[i + needle.len()..].starts_with(|c: char| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_schramm_matches_whole_numbers_only() {
        let comment = "Schramm, Bd. 22, Abb. 1124.";
        assert!(names_schramm(comment, 1124));
        assert!(!names_schramm(comment, 112));
        assert!(!names_schramm(comment, 124));
        assert!(names_schramm("Abb. 7", 7));
        assert!(names_schramm("Abb. 11 und Abb. 1124", 1124));
    }

    #[test]
    fn an_excerpt_must_sit_inside_one_trimmed_line() {
        let texts = ["Beginn Kapitel 13.\n  Holzschnitt zu Kap. 13: Von der Buhlerei.  \n11.7 x 8.4 cm.".to_owned()];
        assert!(in_one_line(&texts, "Holzschnitt zu Kap. 13: Von der Buhlerei."));
        assert!(in_one_line(&texts, "Von der Buhlerei"));
        assert!(!in_one_line(&texts, "Buhlerei.\n11.7"), "spans two lines");
        assert!(!in_one_line(&texts, "  "), "an empty excerpt never matches");
        assert!(!in_one_line(&[], "Buhlerei"));
    }
}
