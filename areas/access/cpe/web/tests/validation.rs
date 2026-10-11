//! Layer-4 integration tests for boot-time validation.
//!
//! Each test loads a synthetic fixture bundle from
//! `web/tests/projects/<name>/` and asserts the expected
//! `DiagnosticKind` set fires (or that a clean config produces `Ok`).
//!
//! The harness goes through the public `cpe_web::project::Project::load`
//! API (not `validate_with_data` directly, which is `pub(crate)`), exercising
//! the full load → validate pipeline.

use std::collections::HashSet;
use std::path::Path;

use cpe_core::diagnostic::{Diagnostic, DiagnosticKind, LabelTarget, Severity};
use cpe_core::validate::context::ValidationContext;
use cpe_web::project::{LoadError, Project};

// ── Fixture loader ────────────────────────────────────────────────────────────

/// The `(name, content)` pairs of a fixture's `narrative/` files, sorted by
/// name, as the bin passes them to `Project::load`; empty when the fixture has
/// no `narrative/` dir. Without the contents the narrative-content checks
/// (heading ids, figure sections) would validate every fixture clean.
fn fixture_narratives(fixture_dir: &Path, fixture_name: &str) -> Vec<(String, String)> {
    let narrative_dir = fixture_dir.join("narrative");
    if !narrative_dir.is_dir() {
        return Vec::new();
    }
    let mut pairs: Vec<(String, String)> = std::fs::read_dir(&narrative_dir)
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/narrative/: {e}"))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let name = entry.file_name().into_string().ok()?;
            let content = std::fs::read_to_string(entry.path())
                .unwrap_or_else(|e| panic!("failed to read {fixture_name}/narrative/{name}: {e}"));
            Some((name, content))
        })
        .collect();
    pairs.sort();
    pairs
}

/// Load a fixture by directory name. Returns the `DiagnosticKind` set from
/// the `ValidationReport` on failure, or `Ok(())` on success.
///
/// `project_id` is passed explicitly so the caller can drive reserved-id /
/// invalid-id cases independently of the directory name.
fn load_fixture(fixture_name: &str, project_id: &str) -> Result<(), HashSet<DiagnosticKind>> {
    let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/projects"));
    let fixture_dir = base.join(fixture_name);

    let config_src = std::fs::read_to_string(fixture_dir.join("config.kdl"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/config.kdl: {e}"));
    let data_sql = std::fs::read_to_string(fixture_dir.join("data.sql"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/data.sql: {e}"));

    let narratives = fixture_narratives(&fixture_dir, fixture_name);
    let narrative_files: Vec<String> = narratives.iter().map(|(name, _)| name.clone()).collect();
    let ctx = ValidationContext {
        project_id,
        reserved_ids: &["_static"],
        narrative_files: &narrative_files,
        // Truthful empty inventory: this config declares no `cover-image`.
        // `&[]` asserts "no asset files" — never "unknown" (see
        // `ValidationContext::asset_files`), so do not copy it blindly.
        asset_files: &[],
    };

    match Project::load(&ctx, &config_src, &data_sql, &narratives) {
        Ok(_) => Ok(()),
        Err(LoadError::Validation(report)) => {
            let kinds: HashSet<DiagnosticKind> = report.iter().map(|d| d.kind.clone()).collect();
            Err(kinds)
        }
        Err(other) => {
            panic!("fixture '{fixture_name}' failed with a non-validation error: {other:?}")
        }
    }
}

/// Assert a fixture validates clean.
fn assert_clean(fixture_name: &str, project_id: &str) {
    load_fixture(fixture_name, project_id)
        .unwrap_or_else(|kinds| panic!("'{fixture_name}' should validate clean but got: {kinds:?}"));
}

/// Assert a fixture produces a `ValidationReport` containing exactly the
/// expected kinds (and no others). Use this for single-kind fixtures.
fn assert_exact_kinds(fixture_name: &str, project_id: &str, expected: &[DiagnosticKind]) {
    let err_kinds = load_fixture(fixture_name, project_id)
        .unwrap_err_or_else(|| panic!("'{fixture_name}' should fail validation but validated clean"));
    let expected_set: HashSet<DiagnosticKind> = expected.iter().cloned().collect();
    assert_eq!(
        err_kinds, expected_set,
        "'{fixture_name}': kind set mismatch\n  got:      {err_kinds:?}\n  expected: {expected_set:?}"
    );
}

// Convenience trait so we can call `.unwrap_err_or_else` on `Result<(), E>`.
trait UnwrapErrOr<E> {
    fn unwrap_err_or_else<F: FnOnce() -> E>(self, f: F) -> E;
}
impl<E> UnwrapErrOr<E> for Result<(), E> {
    fn unwrap_err_or_else<F: FnOnce() -> E>(self, f: F) -> E {
        match self {
            Ok(()) => f(),
            Err(e) => e,
        }
    }
}

/// Load a fixture, returning the full `Vec<Diagnostic>` on validation failure.
///
/// Unlike `load_fixture` (which collapses to a `HashSet` of kinds), this
/// preserves duplicates so tests can assert exact diagnostic *counts* — e.g.
/// "exactly three `MalformedPropertyRow` diagnostics".
fn load_fixture_diagnostics(fixture_name: &str, project_id: &str) -> Result<(), Vec<Diagnostic>> {
    let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/projects"));
    let fixture_dir = base.join(fixture_name);

    let config_src = std::fs::read_to_string(fixture_dir.join("config.kdl"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/config.kdl: {e}"));
    let data_sql = std::fs::read_to_string(fixture_dir.join("data.sql"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/data.sql: {e}"));

    let narratives = fixture_narratives(&fixture_dir, fixture_name);
    let narrative_files: Vec<String> = narratives.iter().map(|(name, _)| name.clone()).collect();
    let ctx = ValidationContext {
        project_id,
        reserved_ids: &["_static"],
        narrative_files: &narrative_files,
        // Truthful empty inventory: this config declares no `cover-image`.
        // `&[]` asserts "no asset files" — never "unknown" (see
        // `ValidationContext::asset_files`), so do not copy it blindly.
        asset_files: &[],
    };

    match Project::load(&ctx, &config_src, &data_sql, &narratives) {
        Ok(_) => Ok(()),
        Err(LoadError::Validation(report)) => {
            let diagnostics: Vec<Diagnostic> = report.iter().cloned().collect();
            Err(diagnostics)
        }
        Err(other) => {
            panic!("fixture '{fixture_name}' failed with a non-validation error: {other:?}")
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn good_validates_clean() {
    assert_clean("good", "good");
}

#[test]
fn good_narrative_partial_variants_validates_clean() {
    // D3: a narrative file with only per-language variants (no bare "home.md")
    // validates clean as long as the default-language variant ("home.en.md")
    // exists — "fr" has no variant at all, which is fine (D1's render-time
    // fallback chain covers it; this is a boot-validation test, not a render
    // test).
    assert_clean("good-narrative-partial-variants", "good-narrative-partial-variants");
}

#[test]
fn broken_unexpected_toplevel() {
    assert_exact_kinds(
        "broken-unexpected-toplevel",
        "broken-unexpected-toplevel",
        &[DiagnosticKind::UnexpectedTopLevelNode],
    );
}

#[test]
fn broken_missing_title() {
    assert_exact_kinds(
        "broken-missing-title",
        "broken-missing-title",
        &[DiagnosticKind::MissingSiteTitle],
    );
}

#[test]
fn broken_missing_languages() {
    assert_exact_kinds(
        "broken-missing-languages",
        "broken-missing-languages",
        &[DiagnosticKind::MissingSiteLanguages],
    );
}

#[test]
fn broken_missing_required_language() {
    // languages "en" only → missing "de" → exactly one MissingRequiredLanguage.
    // Using a single-missing case avoids HashSet-collapses-duplicates ambiguity.
    assert_exact_kinds(
        "broken-missing-required-language",
        "broken-missing-required-language",
        &[DiagnosticKind::MissingRequiredLanguage],
    );
}

#[test]
fn broken_languages_wrong_type() {
    // languages 42 "de" "en" → InvalidFieldType for the integer entry.
    // Both required languages ("de", "en") are present as valid strings,
    // so no MissingRequiredLanguage fires. Proves wrong-type path is isolated.
    assert_exact_kinds(
        "broken-languages-wrong-type",
        "broken-languages-wrong-type",
        &[DiagnosticKind::InvalidFieldType],
    );
}

#[test]
fn broken_site_date_precision_word_wrong_type() {
    // `site { date-precision-word "yes" }` → InvalidFieldType: the flag is a
    // KDL boolean, and a wrong-typed value is reported, never read as `false`.
    assert_exact_kinds(
        "broken-site-date-precision-word-wrong-type",
        "broken-site-date-precision-word-wrong-type",
        &[DiagnosticKind::InvalidFieldType],
    );
}

#[test]
fn broken_narrative_companion_figures() {
    assert_exact_kinds(
        "broken-narrative-companion-figures",
        "broken-narrative-companion-figures",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_narrative_companion_wrong_type() {
    assert_exact_kinds(
        "broken-narrative-companion-wrong-type",
        "broken-narrative-companion-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_narrative_layout_not_first() {
    assert_exact_kinds(
        "broken-narrative-layout-not-first",
        "broken-narrative-layout-not-first",
        &[DiagnosticKind::NarrativeLayoutNotFirst { page: "/".to_owned() }],
    );
}

#[test]
fn broken_site_narrative_companion_unknown() {
    assert_exact_kinds(
        "broken-site-narrative-companion-unknown",
        "broken-site-narrative-companion-unknown",
        &[DiagnosticKind::UnknownSiteValue {
            key: "narrative-companion".to_owned(),
            value: "figures".to_owned(),
            allowed: vec!["rail".to_owned(), "text".to_owned()],
        }],
    );
}

#[test]
fn broken_heading_id_duplicate_on_page_fails_in_both_arms() {
    let duplicate = DiagnosticKind::DuplicateHeadingIdOnPage { id: "ways".to_owned(), page: "/".to_owned() };
    // The lowered arm; the fixture's second page reuses another id of the
    // same file, which must stay accepted.
    assert_exact_kinds(
        "broken-heading-id-duplicate-on-page",
        "broken-heading-id-duplicate-on-page",
        std::slice::from_ref(&duplicate),
    );
    // The raw arm: the config error and the duplicate in one boot pass.
    assert_exact_kinds(
        "broken-heading-id-duplicate-on-page-raw",
        "broken-heading-id-duplicate-on-page-raw",
        &[DiagnosticKind::MissingSiteTitle, duplicate],
    );
}

#[test]
fn broken_unknown_default_language() {
    assert_exact_kinds(
        "broken-unknown-default-language",
        "broken-unknown-default-language",
        &[DiagnosticKind::UnknownDefaultLanguage],
    );
}

#[test]
fn broken_duplicate_page_id() {
    assert_exact_kinds(
        "broken-duplicate-page-id",
        "broken-duplicate-page-id",
        &[DiagnosticKind::DuplicatePageId],
    );
}

#[test]
fn broken_duplicate_route() {
    assert_exact_kinds(
        "broken-duplicate-route",
        "broken-duplicate-route",
        &[DiagnosticKind::DuplicatePageRoute],
    );
}

#[test]
fn broken_duplicate_list_id() {
    assert_exact_kinds(
        "broken-duplicate-list-id",
        "broken-duplicate-list-id",
        &[DiagnosticKind::DuplicateResourceListId],
    );
}

#[test]
fn broken_unknown_component() {
    assert_exact_kinds(
        "broken-unknown-component",
        "broken-unknown-component",
        &[DiagnosticKind::UnknownComponent],
    );
}

#[test]
fn broken_missing_attr() {
    assert_exact_kinds(
        "broken-missing-attr",
        "broken-missing-attr",
        &[DiagnosticKind::MissingRequiredAttribute],
    );
}

#[test]
fn broken_attr_wrong_type() {
    assert_exact_kinds(
        "broken-attr-wrong-type",
        "broken-attr-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_unknown_facet_kind() {
    assert_exact_kinds(
        "broken-unknown-facet-kind",
        "broken-unknown-facet-kind",
        &[DiagnosticKind::UnknownFacetKind],
    );
}

#[test]
fn broken_pagination_conflict() {
    assert_exact_kinds(
        "broken-pagination-conflict",
        "broken-pagination-conflict",
        &[DiagnosticKind::PaginationConflict],
    );
}

#[test]
fn broken_unresolved_target() {
    assert_exact_kinds(
        "broken-unresolved-target",
        "broken-unresolved-target",
        &[DiagnosticKind::UnresolvedTarget],
    );
}

#[test]
fn broken_undeclared_param() {
    assert_exact_kinds(
        "broken-undeclared-param",
        "broken-undeclared-param",
        &[DiagnosticKind::UndeclaredRouteParam],
    );
}

#[test]
fn broken_chrome_incomplete() {
    assert_exact_kinds(
        "broken-chrome-incomplete",
        "broken-chrome-incomplete",
        &[DiagnosticKind::IncompleteChromeTranslation],
    );
}

#[test]
fn broken_narrative_missing() {
    assert_exact_kinds(
        "broken-narrative-missing",
        "broken-narrative-missing",
        &[DiagnosticKind::NarrativeFileNotFound],
    );
}

#[test]
fn broken_narrative_traversal() {
    assert_exact_kinds(
        "broken-narrative-traversal",
        "broken-narrative-traversal",
        &[DiagnosticKind::InvalidNarrativePath],
    );
}

#[test]
fn broken_narrative_no_default_variant() {
    // D3: only a non-default-language variant exists (home.fr.md); neither
    // the base file nor the default-language ("en") variant is present, so
    // NarrativeFileNotFound must still fire — the render-time fallback chain
    // has no guaranteed rung to land on.
    assert_exact_kinds(
        "broken-narrative-no-default-variant",
        "broken-narrative-no-default-variant",
        &[DiagnosticKind::NarrativeFileNotFound],
    );
}

#[test]
fn broken_reserved_id() {
    // project_id "_static" is in reserved_ids → ReservedProjectId.
    // Also InvalidProjectId fires because "_static" starts with underscore (fails kebab regex).
    // Both must fire deterministically; an impl forgetting the reserved check (emitting only
    // InvalidProjectId) would fail this assertion.
    assert_exact_kinds(
        "broken-reserved-id",
        "_static",
        &[DiagnosticKind::ReservedProjectId, DiagnosticKind::InvalidProjectId],
    );
}

#[test]
fn broken_invalid_id() {
    // project_id "Bad_Id" is not kebab-case → InvalidProjectId.
    assert_exact_kinds("broken-invalid-id", "Bad_Id", &[DiagnosticKind::InvalidProjectId]);
}

#[test]
fn broken_multi_accumulates_multiple_kinds() {
    // This fixture triggers several diagnostic kinds at once, proving that accumulation
    // runs to completion — including the narrative file-not-found pass.
    let err_kinds = load_fixture("broken-multi", "broken-multi")
        .unwrap_err_or_else(|| panic!("broken-multi should fail validation but validated clean"));

    assert!(
        err_kinds.len() > 1,
        "broken-multi must produce more than one distinct DiagnosticKind, got: {err_kinds:?}"
    );

    // Assert ALL five intended kinds are present — not just the first four.
    // NarrativeFileNotFound proves the narrative pass runs even after earlier errors.
    assert!(
        err_kinds.contains(&DiagnosticKind::UnexpectedTopLevelNode),
        "broken-multi must contain UnexpectedTopLevelNode, got: {err_kinds:?}"
    );
    assert!(
        err_kinds.contains(&DiagnosticKind::DuplicatePageId),
        "broken-multi must contain DuplicatePageId, got: {err_kinds:?}"
    );
    assert!(
        err_kinds.contains(&DiagnosticKind::UnknownComponent),
        "broken-multi must contain UnknownComponent, got: {err_kinds:?}"
    );
    assert!(
        err_kinds.contains(&DiagnosticKind::MissingRequiredAttribute),
        "broken-multi must contain MissingRequiredAttribute, got: {err_kinds:?}"
    );
    assert!(
        err_kinds.contains(&DiagnosticKind::NarrativeFileNotFound),
        "broken-multi must contain NarrativeFileNotFound (proves narrative pass runs to completion), got: {err_kinds:?}"
    );
}

// ── Step 4: data-dependent validation (Layer-4) ───────────────────────────────

/// Load a fixture by directory name, returning the loaded `Project` on success.
/// Panics on non-validation errors (unexpected for these fixtures).
fn load_fixture_project(fixture_name: &str, project_id: &str) -> Result<Project, HashSet<DiagnosticKind>> {
    let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/projects"));
    let fixture_dir = base.join(fixture_name);

    let config_src = std::fs::read_to_string(fixture_dir.join("config.kdl"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/config.kdl: {e}"));
    let data_sql = std::fs::read_to_string(fixture_dir.join("data.sql"))
        .unwrap_or_else(|e| panic!("failed to read {fixture_name}/data.sql: {e}"));

    let narratives = fixture_narratives(&fixture_dir, fixture_name);
    let narrative_files: Vec<String> = narratives.iter().map(|(name, _)| name.clone()).collect();
    let ctx = ValidationContext {
        project_id,
        reserved_ids: &["_static"],
        narrative_files: &narrative_files,
        // Truthful empty inventory: this config declares no `cover-image`.
        // `&[]` asserts "no asset files" — never "unknown" (see
        // `ValidationContext::asset_files`), so do not copy it blindly.
        asset_files: &[],
    };

    match Project::load(&ctx, &config_src, &data_sql, &narratives) {
        Ok(project) => Ok(project),
        Err(LoadError::Validation(report)) => {
            let kinds: HashSet<DiagnosticKind> = report.iter().map(|d| d.kind.clone()).collect();
            Err(kinds)
        }
        Err(other) => {
            panic!("fixture '{fixture_name}' failed with a non-validation error: {other:?}")
        }
    }
}

#[test]
fn broken_unknown_resource_hard_fails() {
    // A resource-detail with a literal id absent from data.sql → UnknownResource error.
    assert_exact_kinds(
        "broken-unknown-resource",
        "broken-unknown-resource",
        &[DiagnosticKind::UnknownResource],
    );
}

#[test]
fn broken_sparse_property_succeeds_with_warnings_only() {
    // A resource-detail naming show-properties and title-property that are not
    // present on the resource's class. This is advisory (SparseProperty is a
    // Warning), so the project must load successfully — validation is NOT
    // blocked by warnings.
    load_fixture("broken-sparse-property", "broken-sparse-property")
        .expect("broken-sparse-property should validate clean (warnings do not block load)");
}

#[test]
fn broken_sparse_property_warnings_are_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the SparseProperty
    // diagnostics so callers (the bin) can log them. This is the primary
    // assertion for AC6: warnings are not silently discarded.
    let project = load_fixture_project("broken-sparse-property", "broken-sparse-property")
        .expect("broken-sparse-property should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-sparse-property must produce at least one warning, got none"
    );

    // All surfaced diagnostics must be Warning-severity (no errors sneaking in).
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "load_warnings must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    // At least one SparseProperty kind must be present.
    let has_sparse = warnings.iter().any(|w| w.kind == DiagnosticKind::SparseProperty);
    assert!(
        has_sparse,
        "broken-sparse-property must contain a SparseProperty warning; got: {warnings:?}"
    );

    // The fixture has two offending properties: 'missing-prop' (show-properties)
    // and 'nonexistent-prop' (title-property). Both must be reported.
    let missing_prop_warned = warnings
        .iter()
        .any(|w| w.kind == DiagnosticKind::SparseProperty && w.message.contains("missing-prop"));
    let nonexistent_prop_warned = warnings
        .iter()
        .any(|w| w.kind == DiagnosticKind::SparseProperty && w.message.contains("nonexistent-prop"));
    assert!(
        missing_prop_warned,
        "expected a SparseProperty warning mentioning 'missing-prop': {warnings:?}"
    );
    assert!(
        nonexistent_prop_warned,
        "expected a SparseProperty warning mentioning 'nonexistent-prop': {warnings:?}"
    );
}

#[test]
fn good_baseline_has_no_load_warnings() {
    // The good/ fixture has a resource-detail that references a real resource
    // with real properties — no SparseProperty warnings should fire.
    // (If the good fixture has no resource-detail at all, warnings are also empty.)
    let project = load_fixture_project("good", "good").expect("good fixture should load successfully");
    assert!(
        project.warnings().is_empty(),
        "good fixture must produce no load warnings, got: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_sparse_property_has_no_error_diagnostics() {
    // Confirms no error-severity diagnostics are produced; the SparseProperty
    // warnings do not harden into errors. This assertion remains alongside the
    // new warnings test to catch any severity reclassification regression.
    load_fixture("broken-sparse-property", "broken-sparse-property")
        .unwrap_or_else(|kinds| panic!("'broken-sparse-property' should load clean but got errors: {kinds:?}"));
}

#[test]
fn good_baseline_unchanged_after_data_checks() {
    // The good/ baseline must still validate clean after data-dependent checks
    // are wired in — its data.sql contains the resource its config references
    // (none: the config has no resource-detail), so no data checks fire.
    let _ = Severity::Error; // use the import so it is not flagged as unused
    assert_clean("good", "good");
}

// ── Nav-route existence check (UnresolvedNavRoute) ────────────────────────────

#[test]
fn broken_nav_route_fires_exactly_one_unresolved_nav_route() {
    // One nav item points at "/" (exists) and one at "/nowhere" (no page).
    // Exactly one UnresolvedNavRoute diagnostic must fire.
    assert_exact_kinds("broken-nav-route", "broken-nav-route", &[DiagnosticKind::UnresolvedNavRoute]);
    // Count assertion: the kind-set check above would pass even if zero or two
    // diagnostics fired (HashSet collapses duplicates). Assert exactly 1.
    let diagnostics = load_fixture_diagnostics("broken-nav-route", "broken-nav-route")
        .unwrap_err_or_else(|| panic!("broken-nav-route should fail validation but validated clean"));
    let count = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::UnresolvedNavRoute)
        .count();
    assert_eq!(
        count, 1,
        "broken-nav-route must produce exactly 1 UnresolvedNavRoute diagnostic, got {count}: {diagnostics:?}"
    );
}

// ── G11: `site { not-found { browse } }` (Incunabula Slice 8) ─────────────────

#[test]
fn broken_not_found_browse_route_fires_exactly_one() {
    // "/books" is declared, "/nowhere" is not: exactly one diagnostic.
    assert_exact_kinds(
        "broken-not-found-browse-route",
        "broken-not-found-browse-route",
        &[DiagnosticKind::UnresolvedNotFoundBrowseRoute],
    );
    let diagnostics = load_fixture_diagnostics("broken-not-found-browse-route", "broken-not-found-browse-route")
        .unwrap_err_or_else(|| panic!("broken-not-found-browse-route should fail validation but validated clean"));
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::UnresolvedNotFoundBrowseRoute)
        .collect();
    assert_eq!(hits.len(), 1, "exactly one, got: {diagnostics:?}");
    assert!(
        hits[0].message.contains("'/nowhere'"),
        "message names the route: {}",
        hits[0].message
    );
}

#[test]
fn broken_not_found_too_many_browse_links_hard_fails() {
    assert_exact_kinds(
        "broken-not-found-too-many",
        "broken-not-found-too-many",
        &[DiagnosticKind::TooManyNotFoundBrowseLinks],
    );
}

#[test]
fn broken_not_found_wrong_typed_label_reports_instead_of_vanishing() {
    assert_exact_kinds(
        "broken-not-found-wrong-type",
        "broken-not-found-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

// ── Nav label written as a chrome-string id (no diagnostic) ─────────────────

#[test]
fn good_nav_chrome_id_label_loads_without_warnings() {
    // A nav item whose label IS a declared chrome-string id is the intended
    // bilingual-nav idiom (`labels::nav_label` resolves the id first), and
    // the fixture's second item is a plain literal. Neither form produces a
    // diagnostic: the former `NavLabelChromeIdCollision` Warning fired on the
    // idiom itself and was removed.
    let project = load_fixture_project("good-nav-chrome-id-label", "good-nav-chrome-id-label")
        .expect("good-nav-chrome-id-label should load successfully");
    assert!(
        project.warnings().is_empty(),
        "a nav label written as a declared chrome-string id must produce no \
         warnings, got: {:?}",
        project.warnings()
    );
}

// ── Desk header row budget (HeaderRowOverBudget) ────────────────────────────

#[test]
fn broken_header_row_over_budget_warns_for_the_long_language_only() {
    let project = load_fixture_project("broken-header-row-over-budget", "broken-header-row-over-budget")
        .expect("an over-budget desk row is a warning; the project must load");
    let warnings = project.warnings();
    assert_eq!(warnings.len(), 1, "one warning, for de: {warnings:?}");
    let w = &warnings[0];
    assert_eq!(w.severity, Severity::Warning);
    match &w.kind {
        DiagnosticKind::HeaderRowOverBudget { lang, estimated_px, budget_px } => {
            assert_eq!(lang, "de");
            assert_eq!(*budget_px, 1170);
            assert!(estimated_px > budget_px, "{w:?}");
        }
        other => panic!("expected HeaderRowOverBudget, got {other:?}"),
    }
}

// ── Property-row consistency check (MalformedPropertyRow) ────────────────────

#[test]
fn broken_property_row_accumulates_all_three_violations() {
    // Three violations across three value_type families (text, date, link)
    // must all be reported in one pass — proving accumulation and covering all
    // three expect() families in build_property_value.
    assert_exact_kinds(
        "broken-property-row",
        "broken-property-row",
        &[DiagnosticKind::MalformedPropertyRow],
    );
    // Count assertion: the kind-set check above collapses 1 vs. 3 diagnostics
    // to the same set. Assert exactly 3 — one per value_type family
    // (text → value_text NULL, date → date_start NULL, link → target_id NULL).
    let diagnostics = load_fixture_diagnostics("broken-property-row", "broken-property-row")
        .unwrap_err_or_else(|| panic!("broken-property-row should fail validation but validated clean"));
    let count = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::MalformedPropertyRow)
        .count();
    assert_eq!(
        count, 3,
        "broken-property-row must produce exactly 3 MalformedPropertyRow diagnostics \
         (one per value_type family: text, date, link), got {count}: {diagnostics:?}"
    );
}

// ── Annotation store consistency (MalformedAnnotation) ───────────────────────
// dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md

/// Each `broken-annotation-<rule>` fixture breaks exactly one rule, so it
/// reports exactly one `MalformedAnnotation`; its `-raw` sibling adds a
/// missing site title, so the raw arm must report both families.
fn assert_one_malformed_annotation(fixture: &str, extra: &[DiagnosticKind]) {
    let diagnostics = load_fixture_diagnostics(fixture, fixture)
        .unwrap_err_or_else(|| panic!("{fixture} should fail validation but validated clean"));
    let count = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::MalformedAnnotation)
        .count();
    assert_eq!(count, 1, "{fixture}: expected one MalformedAnnotation: {diagnostics:?}");
    let mut expected: HashSet<DiagnosticKind> = extra.iter().cloned().collect();
    expected.insert(DiagnosticKind::MalformedAnnotation);
    let got: HashSet<DiagnosticKind> = diagnostics.iter().map(|d| d.kind.clone()).collect();
    assert_eq!(got, expected, "{fixture}: kind set mismatch");
}

#[test]
fn broken_annotation_fixtures_fail_in_both_arms() {
    for rule in ["box", "points", "color", "link-empty"] {
        assert_one_malformed_annotation(&format!("broken-annotation-{rule}"), &[]);
        assert_one_malformed_annotation(&format!("broken-annotation-{rule}-raw"), &[DiagnosticKind::MissingSiteTitle]);
    }
}

// ── Stored data ARKs (MalformedDataArk, Slice 9) ─────────────────────────────

/// `broken-data-ark` stores six malformed ARKs (a version ARK and `http:` on
/// resources; a foreign host, a trailing `/`, a lowercase shortcode and `''`
/// on annotations) beside a well-formed and a NULL one in each table, so the
/// check reports exactly six, naming each row. Its `-raw` sibling adds a
/// missing site title, so the raw arm must report both families.
#[test]
fn broken_data_ark_fixtures_fail_in_both_arms() {
    for (fixture, extra) in [
        ("broken-data-ark", None),
        ("broken-data-ark-raw", Some(DiagnosticKind::MissingSiteTitle)),
    ] {
        let diagnostics = load_fixture_diagnostics(fixture, fixture)
            .unwrap_err_or_else(|| panic!("{fixture} should fail validation but validated clean"));
        let mut flagged: Vec<&str> = diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::MalformedDataArk)
            .map(|d| d.message.as_str())
            .collect();
        flagged.sort_unstable();
        assert_eq!(flagged.len(), 6, "{fixture}: {diagnostics:?}");
        for (row, message) in [
            "annotations 'a1'",
            "annotations 'a2'",
            "annotations 'a3'",
            "annotations 'a4'",
            "resources 'b1'",
            "resources 'p2'",
        ]
        .iter()
        .zip(&flagged)
        {
            assert!(message.starts_with(row), "{fixture}: {message}");
        }
        let mut expected: HashSet<DiagnosticKind> = extra.into_iter().collect();
        expected.insert(DiagnosticKind::MalformedDataArk);
        let got: HashSet<DiagnosticKind> = diagnostics.iter().map(|d| d.kind.clone()).collect();
        assert_eq!(got, expected, "{fixture}: kind set mismatch");
    }
}

#[test]
fn good_baseline_has_no_malformed_property_row_diagnostics() {
    // The good/ fixture has no properties rows at all — the check must remain
    // silent on a clean database.
    assert_clean("good", "good");
}

// ── Class→route map validation (AmbiguousClassRoute) ─────────────────────────

#[test]
fn broken_ambiguous_class_route_hard_fails() {
    // Two pages both declare class="Character" on their resource-detail with a
    // single :id param route → AmbiguousClassRoute hard-fail.
    assert_exact_kinds(
        "broken-ambiguous-class-route",
        "broken-ambiguous-class-route",
        &[DiagnosticKind::AmbiguousClassRoute {
            class: "Character".to_owned(),
            first_route: "/characters/:id".to_owned(),
            second_route: "/characters-alt/:id".to_owned(),
        }],
    );
}

#[test]
fn good_class_route_validates_clean_and_builds_map() {
    // A single, unambiguous class→route binding must:
    // 1. Validate without errors (load succeeds).
    // 2. Produce a class_routes() map with exactly the expected entry.
    let project = load_fixture_project("good-class-route", "good-class-route")
        .expect("good-class-route should load successfully");

    assert!(
        project.warnings().is_empty(),
        "good-class-route must produce no load warnings, got: {:?}",
        project.warnings()
    );

    let routes = project.class_routes();
    assert_eq!(
        routes.len(),
        1,
        "good-class-route must produce exactly 1 class→route entry, got: {routes:?}"
    );
    assert_eq!(
        routes.get("Character").map(String::as_str),
        Some("/characters/:id"),
        "class 'Character' must map to '/characters/:id', got: {routes:?}"
    );
}

#[test]
fn literal_id_page_excluded_from_class_routes_map() {
    // The good-class-route fixture has two pages that both carry
    // resource-detail with class="Character":
    //   1. /characters/:id  — parameterised (one :param segment) → INCLUDED
    //   2. /featured        — literal route  (zero :param segments) → EXCLUDED
    //
    // The class_routes() map must contain ONLY the parameterised binding;
    // the literal-id page must be silently excluded.
    let project = load_fixture_project("good-class-route", "good-class-route")
        .expect("good-class-route should load successfully");

    let routes = project.class_routes();

    // The literal-id page must NOT have added an extra entry.
    assert_eq!(
        routes.len(),
        1,
        "class_routes() must contain exactly 1 entry (literal-id page excluded), got: {routes:?}"
    );

    // The parameterised page binding must be present.
    assert_eq!(
        routes.get("Character").map(String::as_str),
        Some("/characters/:id"),
        "parameterised page must bind 'Character' → '/characters/:id', got: {routes:?}"
    );
}

#[test]
fn good_baseline_has_empty_class_routes_map() {
    // The good/ fixture has no parameterised resource-detail — class_routes()
    // must be empty.
    let project = load_fixture_project("good", "good").expect("good fixture should load successfully");
    assert!(
        project.class_routes().is_empty(),
        "good/ fixture has no class bindings — class_routes() must be empty, got: {:?}",
        project.class_routes()
    );
}

// ── FK-existence check (UnknownLinkTarget) ────────────────────────────────────

#[test]
fn broken_unknown_link_target_succeeds_with_warnings_only() {
    // A resource has a link property pointing at a target that does not exist.
    // UnknownLinkTarget is Warning severity — validation must succeed (not block
    // load). Mirrors the broken-sparse-property pattern.
    load_fixture("broken-unknown-link-target", "broken-unknown-link-target")
        .expect("broken-unknown-link-target should validate clean (warnings do not block load)");
}

#[test]
fn broken_unknown_link_target_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the UnknownLinkTarget
    // diagnostic so callers (the bin) can log it.
    let project = load_fixture_project("broken-unknown-link-target", "broken-unknown-link-target")
        .expect("broken-unknown-link-target should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-unknown-link-target must produce at least one warning, got none"
    );

    // All surfaced diagnostics must be Warning-severity.
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    // At least one UnknownLinkTarget kind must be present.
    let has_unknown_link = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::UnknownLinkTarget { .. }));
    assert!(
        has_unknown_link,
        "broken-unknown-link-target must contain an UnknownLinkTarget warning; got: {warnings:?}"
    );

    // The warning message must mention both the source and the missing target.
    let mentions_src = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::UnknownLinkTarget { .. }) && w.message.contains("src_resource"));
    let mentions_tgt = warnings.iter().any(|w| {
        matches!(&w.kind, DiagnosticKind::UnknownLinkTarget { .. }) && w.message.contains("nonexistent_target")
    });
    assert!(
        mentions_src,
        "UnknownLinkTarget warning must mention the source resource id 'src_resource': {warnings:?}"
    );
    assert!(
        mentions_tgt,
        "UnknownLinkTarget warning must mention the missing target id 'nonexistent_target': {warnings:?}"
    );
}

#[test]
fn broken_unknown_link_target_has_no_error_diagnostics() {
    // Confirms no error-severity diagnostics are produced; the UnknownLinkTarget
    // warning does not harden into an error.
    load_fixture("broken-unknown-link-target", "broken-unknown-link-target")
        .unwrap_or_else(|kinds| panic!("'broken-unknown-link-target' should load clean but got errors: {kinds:?}"));
}

#[test]
fn broken_unknown_link_target_fires_exactly_one_warning() {
    // The fixture has exactly ONE dangling link (src_resource → nonexistent_target).
    // The kind-set check in broken_unknown_link_target_warning_surfaced_on_project
    // passes even if zero or two diagnostics fired (warnings() preserves duplicates
    // via Project::warnings, but the kind-set collapses). Assert exactly 1 here.
    let project = load_fixture_project("broken-unknown-link-target", "broken-unknown-link-target")
        .expect("broken-unknown-link-target should load successfully (warnings only)");

    let warnings = project.warnings();
    let count = warnings
        .iter()
        .filter(|w| matches!(&w.kind, DiagnosticKind::UnknownLinkTarget { .. }))
        .count();
    assert_eq!(
        count, 1,
        "broken-unknown-link-target fixture has exactly one dangling link — must fire exactly 1 UnknownLinkTarget warning, got {count}: {warnings:?}"
    );
}

#[test]
fn good_baseline_has_no_unknown_link_target_warnings() {
    // The good/ fixture has no link properties at all — no UnknownLinkTarget
    // warnings should fire. Proves the check is silent on clean data.
    let project = load_fixture_project("good", "good").expect("good fixture should load successfully");
    let has_unknown_link = project
        .warnings()
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::UnknownLinkTarget { .. }));
    assert!(
        !has_unknown_link,
        "good/ fixture must produce no UnknownLinkTarget warnings, got: {:?}",
        project.warnings()
    );
}

// ── resource-list data checks ─────────────────────────────────────────────────

#[test]
fn broken_resource_list_class_hard_fails() {
    // A resource-list whose class does not exist in the data → UnknownResourceListClass error.
    assert_exact_kinds(
        "broken-resource-list-class",
        "broken-resource-list-class",
        &[DiagnosticKind::UnknownResourceListClass { class: "NoSuchClass".to_owned(), page: "/list".to_owned() }],
    );
}

#[test]
fn broken_resource_list_prop_hard_fails() {
    // A resource-list with where/sort properties that exist nowhere → UnknownResourceListProperty.
    // Two missing properties → two UnknownResourceListProperty diagnostics (hard-fail).
    let err_kinds = load_fixture("broken-resource-list-prop", "broken-resource-list-prop")
        .unwrap_err_or_else(|| panic!("broken-resource-list-prop should fail validation but validated clean"));

    // Both missing-property variants must be present.
    let has_no_such_prop = err_kinds.contains(&DiagnosticKind::UnknownResourceListProperty {
        property: "no-such-prop".to_owned(),
        class: "Thing".to_owned(),
        page: "/list".to_owned(),
    });
    let has_another_missing = err_kinds.contains(&DiagnosticKind::UnknownResourceListProperty {
        property: "another-missing-prop".to_owned(),
        class: "Thing".to_owned(),
        page: "/list".to_owned(),
    });
    assert!(
        has_no_such_prop,
        "broken-resource-list-prop must report UnknownResourceListProperty for 'no-such-prop': {err_kinds:?}"
    );
    assert!(
        has_another_missing,
        "broken-resource-list-prop must report UnknownResourceListProperty for 'another-missing-prop': {err_kinds:?}"
    );
    // No other unexpected kinds (only these two resource-list-prop errors should appear).
    assert_eq!(
        err_kinds.len(),
        2,
        "broken-resource-list-prop must produce exactly 2 diagnostic kinds: {err_kinds:?}"
    );

    // Count assertion: two properties → 2 diagnostics (not collapsed).
    let diagnostics = load_fixture_diagnostics("broken-resource-list-prop", "broken-resource-list-prop")
        .unwrap_err_or_else(|| panic!("broken-resource-list-prop should fail validation but validated clean"));
    let count = diagnostics
        .iter()
        .filter(|d| matches!(&d.kind, DiagnosticKind::UnknownResourceListProperty { .. }))
        .count();
    assert_eq!(
        count, 2,
        "broken-resource-list-prop has two missing properties → 2 UnknownResourceListProperty diagnostics, got {count}: {diagnostics:?}"
    );
}

#[test]
fn broken_resource_list_sparse_succeeds_with_warnings_only() {
    // A resource-list whose sort property exists on another class but not on the
    // listed class. SparseResourceListProperty is a Warning, so load must succeed.
    load_fixture("broken-resource-list-sparse", "broken-resource-list-sparse")
        .expect("broken-resource-list-sparse should validate clean (warnings do not block load)");
}

#[test]
fn broken_resource_list_sparse_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the SparseResourceListProperty
    // diagnostic so callers (the bin) can log it.
    let project = load_fixture_project("broken-resource-list-sparse", "broken-resource-list-sparse")
        .expect("broken-resource-list-sparse should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-resource-list-sparse must produce at least one warning, got none"
    );

    // All surfaced diagnostics must be Warning-severity.
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    // At least one SparseResourceListProperty kind must be present.
    let has_sparse = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }));
    assert!(
        has_sparse,
        "broken-resource-list-sparse must contain a SparseResourceListProperty warning; got: {warnings:?}"
    );

    // The warning must mention the property name and class.
    let mentions_prop = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }) && w.message.contains("name"));
    let mentions_class = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }) && w.message.contains("Widget"));
    assert!(
        mentions_prop,
        "SparseResourceListProperty warning must mention property 'name': {warnings:?}"
    );
    assert!(
        mentions_class,
        "SparseResourceListProperty warning must mention class 'Widget': {warnings:?}"
    );
}

#[test]
fn good_resource_list_validates_clean() {
    // A resource-list whose class and all where/sort properties exist on that
    // class → no resource-list diagnostics.
    let project = load_fixture_project("good-resource-list", "good-resource-list")
        .expect("good-resource-list should load successfully");
    assert!(
        project.warnings().is_empty(),
        "good-resource-list must produce no load warnings, got: {:?}",
        project.warnings()
    );
}

// ── IncompatibleWhereValue (Guard 1) ──────────────────────────────────────────

#[test]
fn broken_resource_list_incompatible_where_succeeds_with_warnings_only() {
    // A `where equals=5` (integer) against 'name', which class 'Thing' stores
    // only as value_text. IncompatibleWhereValue is a Warning — load must
    // still succeed.
    load_fixture(
        "broken-resource-list-incompatible-where",
        "broken-resource-list-incompatible-where",
    )
    .expect(
        "broken-resource-list-incompatible-where should validate clean \
         (warnings do not block load)",
    );
}

#[test]
fn broken_resource_list_incompatible_where_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the
    // IncompatibleWhereValue diagnostic so callers (the bin) can log it.
    let project = load_fixture_project(
        "broken-resource-list-incompatible-where",
        "broken-resource-list-incompatible-where",
    )
    .expect("broken-resource-list-incompatible-where should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-resource-list-incompatible-where must produce at least one warning, got none"
    );

    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    let has_incompatible = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::IncompatibleWhereValue { .. }));
    assert!(
        has_incompatible,
        "broken-resource-list-incompatible-where must contain an IncompatibleWhereValue \
         warning; got: {warnings:?}"
    );

    // The warning must mention the property name and class.
    let mentions_prop = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::IncompatibleWhereValue { .. }) && w.message.contains("name"));
    let mentions_class = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::IncompatibleWhereValue { .. }) && w.message.contains("Thing"));
    assert!(
        mentions_prop,
        "IncompatibleWhereValue warning must mention property 'name': {warnings:?}"
    );
    assert!(
        mentions_class,
        "IncompatibleWhereValue warning must mention class 'Thing': {warnings:?}"
    );
}

#[test]
fn good_resource_list_matching_where_type_no_incompatible_where_value_warning() {
    // good-resource-list's `where property="name" equals="foo"` compares a
    // string against a value_text-stored property — types match, so
    // IncompatibleWhereValue must NOT fire. good_resource_list_validates_clean
    // already asserts warnings().is_empty(), which subsumes this, but this
    // test isolates the specific kind for clarity when reading test output.
    let project = load_fixture_project("good-resource-list", "good-resource-list")
        .expect("good-resource-list should load successfully");
    let has_incompatible = project
        .warnings()
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::IncompatibleWhereValue { .. }));
    assert!(
        !has_incompatible,
        "good-resource-list must not produce an IncompatibleWhereValue warning \
         (the where value's type matches the property's stored type); got: {:?}",
        project.warnings()
    );
}

// ── ReservedFacetProperty (Guard 2) ───────────────────────────────────────────

#[test]
fn broken_facet_reserved_property_hard_fails() {
    // A facet-panel property="sort" collides with the reserved `?sort=` query
    // key the bin reads at runtime → ReservedFacetProperty hard-fail.
    assert_exact_kinds(
        "broken-facet-reserved-property",
        "broken-facet-reserved-property",
        &[DiagnosticKind::ReservedFacetProperty { property: "sort".to_owned(), page: "/list".to_owned() }],
    );
}

#[test]
fn broken_facet_reserved_view_hard_fails() {
    // A facet-panel property="view" collides with the reserved grid-mode
    // `?view=` query key the engine's resource-list renderer reads
    // (`parse_view_param`) → ReservedFacetProperty hard-fail.
    assert_exact_kinds(
        "broken-facet-reserved-view",
        "broken-facet-reserved-view",
        &[DiagnosticKind::ReservedFacetProperty { property: "view".to_owned(), page: "/list".to_owned() }],
    );
}

#[test]
fn facet_property_role_is_not_reserved() {
    // 'role' is not in the reserved query-key set (see RESERVED_FACET_PROPERTIES
    // in validate.rs) — ReservedFacetProperty must not fire for it. The
    // broken-facet-sparse fixture already uses property="role" and is expected
    // to load successfully (only the advisory SparseFacetProperty warning
    // fires); if ReservedFacetProperty misfired here (an Error), load would
    // fail instead of succeed.
    let project = load_fixture_project("broken-facet-sparse", "broken-facet-sparse")
        .expect("'role' must not be treated as a reserved facet property");
    let has_reserved = project
        .warnings()
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::ReservedFacetProperty { .. }));
    assert!(
        !has_reserved,
        "ReservedFacetProperty is Error-severity and must never appear in warnings(); \
         got: {:?}",
        project.warnings()
    );
}

// ── Sparse WHERE property (two-tier check applies to WHERE, not only SORT) ─────

#[test]
fn broken_resource_list_sparse_where_succeeds_with_warnings_only() {
    // A resource-list whose WHERE property exists on another class but not on the
    // listed class. SparseResourceListProperty is a Warning (not a hard-fail),
    // so the project must load successfully.
    // Mirrors broken_resource_list_sparse_succeeds_with_warnings_only but for
    // a WHERE clause instead of a sort clause — validates the two-tier check
    // applies to WHERE properties too.
    load_fixture("broken-resource-list-sparse-where", "broken-resource-list-sparse-where")
        .expect("broken-resource-list-sparse-where should validate clean (warnings do not block load)");
}

#[test]
fn broken_resource_list_sparse_where_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the SparseResourceListProperty
    // diagnostic for the WHERE property.
    let project = load_fixture_project("broken-resource-list-sparse-where", "broken-resource-list-sparse-where")
        .expect("broken-resource-list-sparse-where should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-resource-list-sparse-where must produce at least one warning, got none"
    );

    // All surfaced diagnostics must be Warning-severity.
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    // At least one SparseResourceListProperty kind must be present.
    let has_sparse = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }));
    assert!(
        has_sparse,
        "broken-resource-list-sparse-where must contain a SparseResourceListProperty warning; got: {warnings:?}"
    );

    // The warning must mention the property name and class.
    let mentions_prop = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }) && w.message.contains("name"));
    let mentions_class = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseResourceListProperty { .. }) && w.message.contains("Widget"));
    assert!(
        mentions_prop,
        "SparseResourceListProperty warning must mention property 'name': {warnings:?}"
    );
    assert!(
        mentions_class,
        "SparseResourceListProperty warning must mention class 'Widget': {warnings:?}"
    );
}

// ── EmptyCompoundSource check ─────────────────────────────────────────────────

#[test]
fn broken_empty_compound_source_succeeds_with_warnings_only() {
    // A compound-viewer with a literal source that has no compound_parts rows.
    // EmptyCompoundSource is a Warning (not a hard-fail), so the project must
    // load successfully — validation is NOT blocked by warnings.
    load_fixture("broken-empty-compound-source", "broken-empty-compound-source")
        .expect("broken-empty-compound-source should validate clean (warnings do not block load)");
}

#[test]
fn broken_empty_compound_source_warning_is_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the EmptyCompoundSource
    // diagnostic. Mirrors the SparseProperty and UnknownLinkTarget warning-surface
    // patterns.
    let project = load_fixture_project("broken-empty-compound-source", "broken-empty-compound-source")
        .expect("broken-empty-compound-source should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-empty-compound-source must produce at least one warning, got none"
    );

    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    let has_empty = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::EmptyCompoundSource { .. }));
    assert!(
        has_empty,
        "broken-empty-compound-source must contain an EmptyCompoundSource warning; \
         got: {warnings:?}"
    );

    // The warning message must name the source id.
    let mentions_source = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::EmptyCompoundSource { .. }) && w.message.contains("ST_001"));
    assert!(
        mentions_source,
        "EmptyCompoundSource warning must mention source id 'ST_001': {warnings:?}"
    );
}

#[test]
fn good_compound_viewer_no_empty_source_warning() {
    // A literal source WITH compound_parts rows must NOT fire EmptyCompoundSource.
    let project = load_fixture_project("good-compound-viewer", "good-compound-viewer")
        .expect("good-compound-viewer should load successfully");
    let has_empty = project
        .warnings()
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::EmptyCompoundSource { .. }));
    assert!(
        !has_empty,
        "good-compound-viewer must not produce an EmptyCompoundSource warning; \
         got: {:?}",
        project.warnings()
    );
}

#[test]
fn compound_viewer_param_source_no_false_positive() {
    // A $param source is runtime-data-dependent; the check is skipped at boot
    // so no EmptyCompoundSource warning fires even without compound_parts rows.
    let project = load_fixture_project("good-compound-viewer-param", "good-compound-viewer-param")
        .expect("good-compound-viewer-param should load successfully");
    let has_empty = project
        .warnings()
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::EmptyCompoundSource { .. }));
    assert!(
        !has_empty,
        "compound-viewer with $param source must not produce EmptyCompoundSource warning; \
         got: {:?}",
        project.warnings()
    );
}

// ── EmptyCompoundSource via Err/raw branch ────────────────────────────────────

#[test]
fn broken_empty_compound_source_raw_branch_surfaces_warning() {
    // Layer-4 test for check_compound_viewer_data_into_raw (the Err branch of
    // validate_with_data). The fixture has:
    //   (a) MissingSiteTitle — a config-internal error that forces validate()
    //       to return Err, routing validate_with_data into the Err/raw path.
    //   (b) compound-viewer source="EMPTY_ST" — a literal source that exists
    //       in data.sql but has no compound_parts rows.
    //
    // Expected: load fails (MissingSiteTitle is a hard error), AND the
    // EmptyCompoundSource warning is present in the returned diagnostics —
    // proving that check_compound_viewer_data_into_raw ran on the raw config
    // and surfaced the warning even though config-internal validation also
    // failed.
    let diagnostics = load_fixture_diagnostics("broken-empty-compound-source-raw", "broken-empty-compound-source-raw")
        .unwrap_err_or_else(|| {
            panic!(
                "broken-empty-compound-source-raw should fail validation \
                     (MissingSiteTitle is a hard error) but validated clean"
            )
        });

    // The config-internal error must be present (proves we are in the Err/raw branch).
    let has_missing_title = diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle));
    assert!(
        has_missing_title,
        "broken-empty-compound-source-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );

    // The EmptyCompoundSource warning must also be present — from the raw branch.
    let has_empty = diagnostics
        .iter()
        .any(|d| matches!(&d.kind, DiagnosticKind::EmptyCompoundSource { .. }));
    assert!(
        has_empty,
        "broken-empty-compound-source-raw must contain EmptyCompoundSource warning \
         (surfaced via check_compound_viewer_data_into_raw, the Err/raw path): {diagnostics:?}"
    );

    // The warning must name the source id ('EMPTY_ST').
    let mentions_source = diagnostics
        .iter()
        .any(|d| matches!(&d.kind, DiagnosticKind::EmptyCompoundSource { .. }) && d.message.contains("EMPTY_ST"));
    assert!(
        mentions_source,
        "EmptyCompoundSource warning (raw branch) must mention source id 'EMPTY_ST': {diagnostics:?}"
    );
}

// ── Facet data-dependent checks ───────────────────────────────────────────────

#[test]
fn broken_facet_unsupported_kind_hard_fails() {
    // A facet-panel with kind="date-decade" — a valid FacetKind but not
    // implemented in this slice. UnsupportedFacetKind is a hard-fail.
    assert_exact_kinds(
        "broken-facet-unsupported-kind",
        "broken-facet-unsupported-kind",
        &[DiagnosticKind::UnsupportedFacetKind { kind: "date-decade".to_owned(), page: "/list".to_owned() }],
    );
}

#[test]
fn broken_facet_incompatible_kind_hard_fails() {
    // A facet-panel with kind="list" on a 'text' property. The list kind
    // requires value_type='list_value'. IncompatibleFacetKind is a hard-fail.
    assert_exact_kinds(
        "broken-facet-incompatible-kind",
        "broken-facet-incompatible-kind",
        &[DiagnosticKind::IncompatibleFacetKind {
            property: "role".to_owned(),
            kind: "list".to_owned(),
            value_type: "text".to_owned(),
            target_class: "Thing".to_owned(),
            page: "/list".to_owned(),
        }],
    );
}

#[test]
fn broken_facet_unknown_property_hard_fails() {
    // A facet-panel whose property doesn't exist anywhere in the data.
    // UnknownFacetProperty is a hard-fail.
    assert_exact_kinds(
        "broken-facet-unknown-property",
        "broken-facet-unknown-property",
        &[DiagnosticKind::UnknownFacetProperty {
            property: "no-such-prop".to_owned(),
            target_class: "Thing".to_owned(),
            page: "/list".to_owned(),
        }],
    );
}

#[test]
fn broken_facet_multi_accumulates_multiple_kinds() {
    // A facet-panel with two broken facets:
    //   - "ghost-prop" → UnknownFacetProperty
    //   - "role" (text property) → IncompatibleFacetKind
    // Both errors must be reported in a single pass (accumulation, not short-circuit).
    let err_kinds = load_fixture("broken-facet-multi", "broken-facet-multi")
        .unwrap_err_or_else(|| panic!("'broken-facet-multi' should fail validation but validated clean"));

    let has_unknown = err_kinds
        .iter()
        .any(|k| matches!(k, DiagnosticKind::UnknownFacetProperty { property, .. } if property == "ghost-prop"));
    let has_incompatible = err_kinds
        .iter()
        .any(|k| matches!(k, DiagnosticKind::IncompatibleFacetKind { property, .. } if property == "role"));
    assert!(
        has_unknown,
        "broken-facet-multi must contain UnknownFacetProperty for 'ghost-prop': {err_kinds:?}"
    );
    assert!(
        has_incompatible,
        "broken-facet-multi must contain IncompatibleFacetKind for 'role': {err_kinds:?}"
    );
    assert_eq!(
        err_kinds.len(),
        2,
        "broken-facet-multi must contain exactly 2 diagnostic kinds: {err_kinds:?}"
    );
}

#[test]
fn broken_facet_sparse_succeeds_with_warnings_only() {
    // A facet-panel with a 'list' facet whose property exists on another class
    // but not on the target class. SparseFacetProperty is a Warning, so the
    // project must load successfully — validation is NOT blocked by warnings.
    load_fixture("broken-facet-sparse", "broken-facet-sparse")
        .expect("broken-facet-sparse should validate clean (warnings do not block load)");
}

#[test]
fn broken_facet_sparse_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the SparseFacetProperty
    // diagnostic so callers (the bin) can log it.
    let project = load_fixture_project("broken-facet-sparse", "broken-facet-sparse")
        .expect("broken-facet-sparse should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-facet-sparse must produce at least one warning, got none"
    );

    // All surfaced diagnostics must be Warning-severity.
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    // At least one SparseFacetProperty kind must be present.
    let has_sparse = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseFacetProperty { .. }));
    assert!(
        has_sparse,
        "broken-facet-sparse must contain a SparseFacetProperty warning; got: {warnings:?}"
    );

    // The warning must mention the property name and target class.
    let mentions_prop = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseFacetProperty { .. }) && w.message.contains("role"));
    let mentions_class = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseFacetProperty { .. }) && w.message.contains("Thing"));
    assert!(
        mentions_prop,
        "SparseFacetProperty warning must mention property 'role': {warnings:?}"
    );
    assert!(
        mentions_class,
        "SparseFacetProperty warning must mention class 'Thing': {warnings:?}"
    );
}

// ── Good-facet clean baseline ─────────────────────────────────────────────────

#[test]
fn good_facet_validates_clean_no_diagnostics() {
    // A well-formed facet-panel: resource-list class="Thing" + facet-panel targeting
    // it with property="category" kind="list", where 'category' is a 'list_value'
    // property present on every Thing resource (universal).
    //
    // Must load with NO errors and NO warnings:
    //   - No IncompatibleFacetKind  (kind="list" is correct for list_value)
    //   - No UnknownFacetProperty   (property exists in the data)
    //   - No SparseFacetProperty    (property is universal — present on every Thing)
    //
    // This regression test catches a validator that wrongly fires on a correct
    // 'list' facet — the inverse of the broken-facet-* fixtures.
    let project =
        load_fixture_project("good-facet", "good-facet").expect("good-facet should load successfully (no errors)");

    let warnings = project.warnings();
    assert!(
        warnings.is_empty(),
        "good-facet must produce no load warnings (SparseFacetProperty must not \
         fire on a universal property), got: {warnings:?}"
    );
}

#[test]
fn good_facet_loads_without_error() {
    // Companion to good_facet_validates_clean_no_diagnostics: assert_clean uses
    // the non-project-returning load_fixture path and confirms hard-fail
    // diagnostics (IncompatibleFacetKind, UnknownFacetProperty) do not fire.
    assert_clean("good-facet", "good-facet");
}

// ── Search-bar data checks ────────────────────────────────────────────────────

#[test]
fn broken_search_unknown_property_hard_fails() {
    // A search-bar whose searchable property doesn't exist anywhere in the
    // data. UnknownSearchProperty is a hard-fail. Mirrors
    // broken_facet_unknown_property_hard_fails.
    assert_exact_kinds(
        "broken-search-unknown-property",
        "broken-search-unknown-property",
        &[DiagnosticKind::UnknownSearchProperty {
            property: "no-such-prop".to_owned(),
            target_class: "Thing".to_owned(),
            page: "/list".to_owned(),
        }],
    );
}

#[test]
fn broken_search_incompatible_property_hard_fails() {
    // A search-bar over a property whose value_type is 'integer' — search
    // requires 'text' or 'richtext'. IncompatibleSearchProperty is a
    // hard-fail. Mirrors broken_facet_incompatible_kind_hard_fails.
    assert_exact_kinds(
        "broken-search-incompatible-property",
        "broken-search-incompatible-property",
        &[DiagnosticKind::IncompatibleSearchProperty {
            property: "views".to_owned(),
            value_type: "integer".to_owned(),
            target_class: "Thing".to_owned(),
            page: "/list".to_owned(),
        }],
    );
}

#[test]
fn broken_search_sparse_succeeds_with_warnings_only() {
    // A search-bar property that exists on another class but not on the
    // target class. SparseSearchProperty is a Warning, so the project must
    // load successfully — validation is NOT blocked by warnings. Mirrors
    // broken_facet_sparse_succeeds_with_warnings_only.
    load_fixture("broken-search-sparse", "broken-search-sparse")
        .expect("broken-search-sparse should validate clean (warnings do not block load)");
}

#[test]
fn broken_search_sparse_warning_surfaced_on_project() {
    // After load succeeds, Project::warnings() must carry the
    // SparseSearchProperty diagnostic so callers (the bin) can log it.
    // Mirrors broken_facet_sparse_warning_surfaced_on_project.
    let project = load_fixture_project("broken-search-sparse", "broken-search-sparse")
        .expect("broken-search-sparse should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-search-sparse must produce at least one warning, got none"
    );

    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "warnings() must only contain Warning-severity diagnostics, got: {w:?}"
        );
    }

    let has_sparse = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseSearchProperty { .. }));
    assert!(
        has_sparse,
        "broken-search-sparse must contain a SparseSearchProperty warning; got: {warnings:?}"
    );

    let mentions_prop = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseSearchProperty { .. }) && w.message.contains("description"));
    let mentions_class = warnings
        .iter()
        .any(|w| matches!(&w.kind, DiagnosticKind::SparseSearchProperty { .. }) && w.message.contains("Thing"));
    assert!(
        mentions_prop,
        "SparseSearchProperty warning must mention property 'description': {warnings:?}"
    );
    assert!(
        mentions_class,
        "SparseSearchProperty warning must mention class 'Thing': {warnings:?}"
    );
}

#[test]
fn good_search_validates_clean_no_diagnostics() {
    // good-search (added for FTS5 boot indexing, step 4) declares a
    // search-bar over "name" (text) and "description" (richtext), both
    // present on every 'Thing' resource — the happy path for the new
    // data-dependent checks. Must load with NO warnings:
    //   - No UnknownSearchProperty       (both properties exist in the data)
    //   - No IncompatibleSearchProperty  (text/richtext are both compatible)
    //   - No SparseSearchProperty        (both properties are universal)
    let project =
        load_fixture_project("good-search", "good-search").expect("good-search should load successfully (no errors)");

    let warnings = project.warnings();
    assert!(
        warnings.is_empty(),
        "good-search must produce no load warnings, got: {warnings:?}"
    );
}

// ── Duplicate singleton component check (DuplicateSingletonComponent) ────────

#[test]
fn broken_duplicate_resource_detail_hard_fails() {
    // Two resource-detail components on the same page → DuplicateSingletonComponent
    // for "resource-detail". Both literal ids ("r1", "r2") exist in the data, so
    // no unrelated UnknownResource diagnostic fires alongside it.
    assert_exact_kinds(
        "broken-duplicate-resource-detail",
        "broken-duplicate-resource-detail",
        &[DiagnosticKind::DuplicateSingletonComponent {
            component: "resource-detail".to_owned(),
            page: "/detail".to_owned(),
        }],
    );
}

#[test]
fn broken_duplicate_facet_panel_hard_fails() {
    // Two facet-panel components on the same page, both targeting the same
    // resource-list, both with clean facet data (no IncompatibleFacetKind,
    // UnknownFacetProperty, or SparseFacetProperty noise) → the only diagnostic
    // is DuplicateSingletonComponent for "facet-panel".
    assert_exact_kinds(
        "broken-duplicate-facet-panel",
        "broken-duplicate-facet-panel",
        &[DiagnosticKind::DuplicateSingletonComponent {
            component: "facet-panel".to_owned(),
            page: "/list".to_owned(),
        }],
    );
}

// ── Label existence check (UnknownLabel) ──────────────────────────────────────

#[test]
fn broken_label_unknown_property_succeeds_with_warnings_only() {
    // A `labels { property "…" }` entry naming a property absent from the
    // project's data. This is advisory (UnknownLabel is a Warning), so the
    // project must load successfully — validation is NOT blocked by warnings.
    load_fixture("broken-label-unknown-property", "broken-label-unknown-property")
        .expect("broken-label-unknown-property should validate clean (warnings do not block load)");
}

#[test]
fn broken_label_unknown_property_warnings_are_surfaced_on_project() {
    let project = load_fixture_project("broken-label-unknown-property", "broken-label-unknown-property")
        .expect("broken-label-unknown-property should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-label-unknown-property must produce at least one warning, got none"
    );

    let has_unknown_label = warnings.iter().any(|w| {
        w.kind
            == DiagnosticKind::UnknownLabel {
                target: LabelTarget::Property,
                key: "unknown-field".to_owned(),
            }
            && w.message.contains("unknown-field")
    });
    assert!(
        has_unknown_label,
        "expected an UnknownLabel warning mentioning 'unknown-field': {warnings:?}"
    );
}

#[test]
fn broken_label_unknown_property_has_no_error_diagnostics() {
    load_fixture("broken-label-unknown-property", "broken-label-unknown-property")
        .unwrap_or_else(|kinds| panic!("'broken-label-unknown-property' should load clean but got errors: {kinds:?}"));
}

#[test]
fn broken_label_unknown_class_succeeds_with_warnings_only() {
    // A `labels { class "…" }` entry naming a class absent from the project's
    // data. Advisory only — the project must load successfully.
    load_fixture("broken-label-unknown-class", "broken-label-unknown-class")
        .expect("broken-label-unknown-class should validate clean (warnings do not block load)");
}

#[test]
fn broken_label_unknown_class_warnings_are_surfaced_on_project() {
    let project = load_fixture_project("broken-label-unknown-class", "broken-label-unknown-class")
        .expect("broken-label-unknown-class should load successfully (warnings only)");

    let warnings = project.warnings();
    assert!(
        !warnings.is_empty(),
        "broken-label-unknown-class must produce at least one warning, got none"
    );

    let has_unknown_label = warnings.iter().any(|w| {
        w.kind == DiagnosticKind::UnknownLabel { target: LabelTarget::Class, key: "UnknownClass".to_owned() }
            && w.message.contains("UnknownClass")
    });
    assert!(
        has_unknown_label,
        "expected an UnknownLabel warning mentioning 'UnknownClass': {warnings:?}"
    );
}

#[test]
fn broken_label_unknown_class_has_no_error_diagnostics() {
    load_fixture("broken-label-unknown-class", "broken-label-unknown-class")
        .unwrap_or_else(|kinds| panic!("'broken-label-unknown-class' should load clean but got errors: {kinds:?}"));
}

#[test]
fn good_label_known_produces_no_unknown_label_warnings() {
    // True-negative counterpart to the two broken-label-unknown-* fixtures
    // above: a `labels {}` block naming a property AND a class that both
    // DO exist in the data must produce zero UnknownLabel warnings. Without
    // this test, a `check_labels_data` that warned unconditionally on every
    // declared label (regardless of data presence) would still pass every
    // other test in this file.
    let project = load_fixture_project("good-label-known", "good-label-known")
        .expect("good-label-known should load successfully");

    let warnings = project.warnings();
    let has_unknown_label = warnings.iter().any(|w| matches!(w.kind, DiagnosticKind::UnknownLabel { .. }));
    assert!(
        !has_unknown_label,
        "a labels block naming a real property ('name') and a real class \
         ('Thing') must not produce any UnknownLabel warning, got: {warnings:?}"
    );
}

// ── `media="…"` — embedded-media plan step 5 ─────────────────────────

#[test]
fn good_media_validates_clean_and_builds_map() {
    // A clean media declaration (link-typed, present, targeting an
    // image-like class) on both resource-detail and resource-list must
    // validate without errors and produce a media_properties() map with
    // exactly the expected entry.
    let project = load_fixture_project("good-media", "good-media").expect("good-media should load successfully");

    assert!(
        project.warnings().is_empty(),
        "good-media must produce no load warnings, got: {:?}",
        project.warnings()
    );

    let media = project.media_properties();
    assert_eq!(
        media.len(),
        1,
        "good-media must produce exactly 1 class→media-property entry, got: {media:?}"
    );
    assert_eq!(
        media.get("Character").map(String::as_str),
        Some("portrait"),
        "class 'Character' must map to media property 'portrait', got: {media:?}"
    );
}

#[test]
fn broken_media_unknown_property_hard_fails() {
    assert_exact_kinds(
        "broken-media-unknown-property",
        "broken-media-unknown-property",
        &[DiagnosticKind::UnknownMediaProperty {
            property: "no-such-prop".to_owned(),
            class: "Character".to_owned(),
            page: "/characters/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_media_incompatible_type_hard_fails() {
    assert_exact_kinds(
        "broken-media-incompatible-type",
        "broken-media-incompatible-type",
        &[DiagnosticKind::IncompatibleMediaProperty {
            property: "bio".to_owned(),
            class: "Character".to_owned(),
            value_type: "text".to_owned(),
            page: "/characters/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_media_sparse_succeeds_with_warnings_only() {
    // SparseMediaProperty is advisory — the project must still load.
    let project = load_fixture_project("broken-media-sparse", "broken-media-sparse")
        .expect("broken-media-sparse should load with warnings only, not hard-fail");

    assert!(
        project.warnings().iter().any(|w| matches!(
            &w.kind,
            DiagnosticKind::SparseMediaProperty { property, class, page }
                if property == "portrait" && class == "Character" && page == "/characters/:id"
        )),
        "expected a SparseMediaProperty warning, got: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_media_target_not_imagelike_hard_fails() {
    assert_exact_kinds(
        "broken-media-target-not-imagelike",
        "broken-media-target-not-imagelike",
        &[DiagnosticKind::MediaPropertyTargetNotImagelike {
            property: "colleague".to_owned(),
            class: "Character".to_owned(),
            target_class: "Character".to_owned(),
            page: "/characters/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_media_unresolvable_class_hard_fails() {
    assert_exact_kinds(
        "broken-media-unresolvable-class",
        "broken-media-unresolvable-class",
        &[DiagnosticKind::UnresolvableMediaClass { page: "/things/:id".to_owned() }],
    );
}

#[test]
fn broken_media_conflicting_property_hard_fails() {
    assert_exact_kinds(
        "broken-media-conflicting",
        "broken-media-conflicting",
        &[DiagnosticKind::ConflictingMediaProperty {
            class: "Character".to_owned(),
            registered_property: "portrait".to_owned(),
            declared_property: "headshot".to_owned(),
            page: "/characters".to_owned(),
        }],
    );
}

// ── `compound-viewer media="…"` (Incunabula Slice 2 step 6b) ──────────────────

#[test]
fn good_compound_media_loads_clean() {
    let project = load_fixture_project("good-compound-media", "good-compound-media")
        .expect("good-compound-media must load without errors");
    assert!(
        project.warnings().is_empty(),
        "good-compound-media must load with no warnings, got: {:?}",
        project.warnings()
    );
}

#[test]
fn good_compound_media_no_members_skips_rather_than_errors() {
    // `$param` source + no compound_parts rows anywhere ⇒ empty member-class
    // set ⇒ the check is skipped. An Error here would abort the whole binary.
    let project = load_fixture_project("good-compound-media-no-members", "good-compound-media-no-members")
        .expect("an empty member-class set must skip the media check, not hard-fail");
    assert!(
        project.warnings().is_empty(),
        "no media diagnostic of any severity may fire on an empty member set, got: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_compound_media_incompatible_type_hard_fails() {
    // `$param` source: the member class comes from every compound_parts
    // child in the data ({Page}), not from a `class=` attribute.
    assert_exact_kinds(
        "broken-compound-media-incompatible-type",
        "broken-compound-media-incompatible-type",
        &[DiagnosticKind::IncompatibleMediaProperty {
            property: "note".to_owned(),
            class: "Page".to_owned(),
            value_type: "text".to_owned(),
            page: "/books/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_compound_media_incompatible_type_raw_branch_surfaces_error() {
    // Layer-4 test for check_compound_viewer_media_into_raw (literal source,
    // Err/raw branch): MissingSiteTitle forces the raw path; the raw sibling
    // must still report the wrong media property in the same pass.
    let diagnostics = load_fixture_diagnostics(
        "broken-compound-media-incompatible-type-raw",
        "broken-compound-media-incompatible-type-raw",
    )
    .unwrap_err_or_else(|| {
        panic!(
            "broken-compound-media-incompatible-type-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
        )
    });
    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "must contain MissingSiteTitle (forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| matches!(
            &d.kind,
            DiagnosticKind::IncompatibleMediaProperty { property, class, page, .. }
                if property == "note" && class == "Page" && page == "/book"
        )),
        "must ALSO contain IncompatibleMediaProperty from \
         check_compound_viewer_media_into_raw: {diagnostics:?}"
    );
}

// ── `compound-viewer media="…"` feeds the class->media map and
//    ConflictingMediaProperty (engine-hygiene batch 3, item 8) ────────────────

#[test]
fn broken_compound_media_conflicts_with_detail_registered_class() {
    // The resource-detail registers Page->"image" first; the compound-viewer
    // (a separate page) declares "scan" for the same class — a conflict.
    assert_exact_kinds(
        "broken-compound-media-conflicting",
        "broken-compound-media-conflicting",
        &[DiagnosticKind::ConflictingMediaProperty {
            class: "Page".to_owned(),
            registered_property: "image".to_owned(),
            declared_property: "scan".to_owned(),
            page: "/book".to_owned(),
        }],
    );
}

#[test]
fn broken_compound_media_conflicts_with_detail_registered_class_raw_branch_surfaces_error() {
    // Layer-4 test for the detail->compound-viewer media-map threading in
    // the Err/raw branch: check_resource_detail_media_into_raw must run
    // BEFORE check_compound_viewer_media_into_raw for the conflict to fire.
    let diagnostics =
        load_fixture_diagnostics("broken-compound-media-conflicting-raw", "broken-compound-media-conflicting-raw")
            .unwrap_err_or_else(|| {
                panic!(
                    "broken-compound-media-conflicting-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
                )
            });
    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-compound-media-conflicting-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::ConflictingMediaProperty {
                class: "Page".to_owned(),
                registered_property: "image".to_owned(),
                declared_property: "scan".to_owned(),
                page: "/book".to_owned(),
            }),
        "broken-compound-media-conflicting-raw must ALSO contain ConflictingMediaProperty — \
         its absence means the raw detail-media map never reached the raw \
         compound-viewer-media check: {diagnostics:?}"
    );
}

#[test]
fn broken_list_conflicts_with_compound_registered_class() {
    // No resource-detail page at all: the compound-viewer registers
    // Page->"image" first, and the resource-list (a separate page)
    // declares "scan" for the same class — a conflict, proving the list
    // check runs against the MERGED map (detail + compound-viewer), not
    // just the detail-only one.
    assert_exact_kinds(
        "broken-compound-list-media-conflicting",
        "broken-compound-list-media-conflicting",
        &[DiagnosticKind::ConflictingMediaProperty {
            class: "Page".to_owned(),
            registered_property: "image".to_owned(),
            declared_property: "scan".to_owned(),
            page: "/pages".to_owned(),
        }],
    );
}

#[test]
fn broken_list_conflicts_with_compound_registered_class_raw_branch_surfaces_error() {
    // Layer-4 test for the compound-viewer->list media-map threading in the
    // Err/raw branch, with no resource-detail page in the mix at all:
    // check_compound_viewer_media_into_raw must run BEFORE
    // check_resource_list_media_into_raw for the conflict to fire.
    let diagnostics = load_fixture_diagnostics(
        "broken-compound-list-media-conflicting-raw",
        "broken-compound-list-media-conflicting-raw",
    )
    .unwrap_err_or_else(|| {
        panic!(
            "broken-compound-list-media-conflicting-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
        )
    });
    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-compound-list-media-conflicting-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::ConflictingMediaProperty {
                class: "Page".to_owned(),
                registered_property: "image".to_owned(),
                declared_property: "scan".to_owned(),
                page: "/pages".to_owned(),
            }),
        "broken-compound-list-media-conflicting-raw must ALSO contain ConflictingMediaProperty — \
         its absence means the raw compound-viewer-media map never reached the raw \
         list-media check: {diagnostics:?}"
    );
}

// ── `compound-viewer layout=`/`index=`, ownership, and the `pages` reader
//    (Incunabula Slice 3b, step 8) ─────────────────────────────────────────────

#[test]
fn broken_compound_layout_missing() {
    // `layout` is required; omitting it is MissingRequiredAttribute alone —
    // the whole component is dropped, so no other check can fire.
    assert_exact_kinds(
        "broken-compound-layout-missing",
        "broken-compound-layout-missing",
        &[DiagnosticKind::MissingRequiredAttribute],
    );
}

#[test]
fn broken_compound_layout_unknown() {
    // `layout` is a closed vocabulary ("contents" | "pages"); an unrecognized
    // value is InvalidAttributeType alone.
    assert_exact_kinds(
        "broken-compound-layout-unknown",
        "broken-compound-layout-unknown",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_picker_unknown_mode() {
    assert_exact_kinds(
        "broken-picker-unknown-mode",
        "broken-picker-unknown-mode",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_picker_wrong_type() {
    assert_exact_kinds(
        "broken-picker-wrong-type",
        "broken-picker-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_picker_contents_layout() {
    assert_exact_kinds(
        "broken-picker-contents-layout",
        "broken-picker-contents-layout",
        &[DiagnosticKind::PagesLayoutOnly { feature: "picker".to_owned(), page: "/books/:id".to_owned() }],
    );
}

#[test]
fn broken_member_links_contents_layout() {
    assert_exact_kinds(
        "broken-member-links-contents-layout",
        "broken-member-links-contents-layout",
        &[DiagnosticKind::PagesLayoutOnly {
            feature: "member-links".to_owned(),
            page: "/books/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_annotations_not_pages() {
    assert_exact_kinds(
        "broken-annotations-not-pages",
        "broken-annotations-not-pages",
        &[DiagnosticKind::PagesLayoutOnly {
            feature: "annotations".to_owned(),
            page: "/books/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_compound_index_missing() {
    // `index` is required (Q1); omitting it is MissingRequiredAttribute
    // alone.
    assert_exact_kinds(
        "broken-compound-index-missing",
        "broken-compound-index-missing",
        &[DiagnosticKind::MissingRequiredAttribute],
    );
}

#[test]
fn broken_compound_index_unknown() {
    // `index` is a closed vocabulary ("rail" | "collapsed" | "none"); an
    // unrecognized value is InvalidAttributeType alone.
    assert_exact_kinds(
        "broken-compound-index-unknown",
        "broken-compound-index-unknown",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_pages_layout_without_media() {
    // The `pages` layout is scan-first; without `media=` it has no scan to
    // show (D3). A `$id` param source keeps this config-internal, isolating
    // PagesLayoutWithoutMedia.
    assert_exact_kinds(
        "broken-pages-layout-without-media",
        "broken-pages-layout-without-media",
        &[DiagnosticKind::PagesLayoutWithoutMedia { page: "/books/:id".to_owned() }],
    );
}

#[test]
fn broken_compound_media_ownership() {
    // A resource-detail with `media=` on the same resource as its page's
    // compound-viewer navigator would render the compound's images twice;
    // the navigator owns them (D2,
    // dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md).
    assert_exact_kinds(
        "broken-compound-media-ownership",
        "broken-compound-media-ownership",
        &[DiagnosticKind::CompoundMediaOwnership { page: "/books/:id".to_owned() }],
    );
}

#[test]
fn broken_two_compound_viewers() {
    // Two compound-viewer components on one page share the fixed
    // id="compound-viewer"/id="cv-picker" ids and page-global reader signals —
    // at most one navigator per page.
    assert_exact_kinds(
        "broken-two-compound-viewers",
        "broken-two-compound-viewers",
        &[DiagnosticKind::DuplicateSingletonComponent {
            component: "compound-viewer".to_owned(),
            page: "/books/:id".to_owned(),
        }],
    );
}

#[test]
fn broken_pages_beside_record_viewer_multiple_image_viewers_alone() {
    // A `pages` reader mounts the record viewer's own unit, and so does any
    // resource-detail with `media=`; two on one page would drive each
    // other's page-global signals. The resource-detail's id is a DIFFERENT
    // expression from the compound-viewer's source, so CompoundMediaOwnership
    // does not also fire — this isolates MultipleImageViewers alone.
    assert_exact_kinds(
        "broken-pages-beside-record-viewer",
        "broken-pages-beside-record-viewer",
        &[DiagnosticKind::MultipleImageViewers { page: "/books/:id".to_owned() }],
    );
}

#[test]
fn good_compound_pages_loads_clean() {
    // layout="pages" index="none" media="image" paired with a same-id
    // resource-detail carrying NO media (the navigator owns the images) —
    // zero diagnostics of any severity.
    let project = load_fixture_project("good-compound-pages", "good-compound-pages")
        .expect("good-compound-pages must load without errors");
    assert!(
        project.warnings().is_empty(),
        "good-compound-pages must load with no warnings, got: {:?}",
        project.warnings()
    );
}

// ── `media="…"` via Err/raw branch ────────────────────────────────────────────

#[test]
fn broken_media_unknown_property_raw_branch_surfaces_error() {
    // Layer-4 test for check_resource_detail_media_into_raw (the Err branch of
    // validate_with_data). The fixture has:
    //   (a) MissingSiteTitle — a config-internal error forcing the Err/raw path.
    //   (b) resource-detail media="no-such-prop" — absent everywhere in the data.
    //
    // Expected: BOTH diagnostics in one returned Vec — the accumulating
    // validator must not stop at the config-internal error.
    let diagnostics =
        load_fixture_diagnostics("broken-media-unknown-property-raw", "broken-media-unknown-property-raw")
            .unwrap_err_or_else(|| {
                panic!(
                    "broken-media-unknown-property-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
                )
            });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-media-unknown-property-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| matches!(&d.kind, DiagnosticKind::UnknownMediaProperty { .. })),
        "broken-media-unknown-property-raw must ALSO contain UnknownMediaProperty \
         from check_resource_detail_media_into_raw: {diagnostics:?}"
    );
}

#[test]
fn broken_media_conflicting_raw_branch_surfaces_error() {
    // Layer-4 test for the detail→list media-map threading in the Err/raw
    // branch. ConflictingMediaProperty can only fire if
    // check_resource_detail_media_into_raw ran BEFORE
    // check_resource_list_media_into_raw and its map was passed along — an
    // ordering dependency the compiler cannot see. This is the only test
    // standing between that bug and a green CI.
    let diagnostics = load_fixture_diagnostics("broken-media-conflicting-raw", "broken-media-conflicting-raw")
        .unwrap_err_or_else(|| {
            panic!(
                "broken-media-conflicting-raw should fail validation \
                     (MissingSiteTitle is a hard error) but validated clean"
            )
        });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-media-conflicting-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::ConflictingMediaProperty {
                class: "Character".to_owned(),
                registered_property: "portrait".to_owned(),
                declared_property: "headshot".to_owned(),
                page: "/characters".to_owned(),
            }),
        "broken-media-conflicting-raw must ALSO contain ConflictingMediaProperty — \
         its absence means the raw detail-media map never reached the raw \
         list-media check: {diagnostics:?}"
    );
}

#[test]
fn broken_media_wrong_typed_class_reports_invalid_attribute_type_once() {
    // Step 7 (report wrong-typed optional string attributes) inverts this
    // pair's premise: `lower_resource_detail` now reads `class` with the
    // REPORTING `get_typed_string_attr`, so `class=42` itself produces
    // `InvalidAttributeType` — and the raw-fallback guard in
    // `check_resource_detail_media_into_raw` now skips the wrong-typed
    // `class`/`id` case instead of also emitting `UnresolvableMediaClass`
    // (one mistake, one diagnostic). Both fixtures now contain
    // `InvalidAttributeType` and NOT `UnresolvableMediaClass`.
    //
    // The two fixtures differ only in whether `site { title }` is present,
    // i.e. only in which arm of validate_with_data runs.
    let success_arm = load_fixture("broken-media-wrong-typed-class", "broken-media-wrong-typed-class")
        .unwrap_err_or_else(|| panic!("broken-media-wrong-typed-class should fail validation but validated clean"));
    assert!(
        success_arm.contains(&DiagnosticKind::InvalidAttributeType),
        "success arm must report InvalidAttributeType for a wrong-typed class: {success_arm:?}"
    );
    assert!(
        !success_arm.contains(&DiagnosticKind::UnresolvableMediaClass { page: "/characters/:id".to_owned() }),
        "success arm must NOT also report UnresolvableMediaClass — lowering already named the \
         root cause, so the media check must not cascade into a second diagnostic: \
         {success_arm:?}"
    );

    let raw_arm = load_fixture_diagnostics("broken-media-wrong-typed-class-raw", "broken-media-wrong-typed-class-raw")
        .unwrap_err_or_else(|| {
            panic!(
                "broken-media-wrong-typed-class-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
            )
        });
    assert!(
        raw_arm.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-media-wrong-typed-class-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {raw_arm:?}"
    );
    assert!(
        raw_arm.iter().any(|d| matches!(&d.kind, DiagnosticKind::InvalidAttributeType)),
        "the raw arm must report InvalidAttributeType exactly as the success arm does — config- \
         internal lowering runs once, before the fork, and reaches both arms: {raw_arm:?}"
    );
    assert!(
        !raw_arm
            .iter()
            .any(|d| d.kind == DiagnosticKind::UnresolvableMediaClass { page: "/characters/:id".to_owned() }),
        "the raw arm must NOT also report UnresolvableMediaClass — the new guard in \
         check_resource_detail_media_into_raw skips a present-but-wrong-typed class/id: \
         {raw_arm:?}"
    );
}

// ── UnknownLabel via Err/raw branch ───────────────────────────────────────────

#[test]
fn broken_label_unknown_property_raw_branch_surfaces_warning() {
    // Layer-4 test for check_labels_data_into_raw. The source fixture
    // (broken-label-unknown-property) validates CLEAN — UnknownLabel is only a
    // Warning. This variant hard-fails on the missing site title while still
    // surfacing the warning, which is the whole point: `raw.labels` is a
    // top-level field that survives a failed site lowering.
    let diagnostics =
        load_fixture_diagnostics("broken-label-unknown-property-raw", "broken-label-unknown-property-raw")
            .unwrap_err_or_else(|| {
                panic!(
                    "broken-label-unknown-property-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
                )
            });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-label-unknown-property-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::UnknownLabel {
                target: LabelTarget::Property,
                key: "unknown-field".to_owned(),
            }),
        "broken-label-unknown-property-raw must ALSO contain the UnknownLabel warning \
         from check_labels_data_into_raw: {diagnostics:?}"
    );
}

#[test]
fn good_baseline_has_empty_media_properties_map() {
    // The good/ fixture declares no `media` attributes — media_properties()
    // must be empty.
    let project = load_fixture_project("good", "good").expect("good fixture should load successfully");
    assert!(
        project.media_properties().is_empty(),
        "good/ fixture has no media declarations — media_properties() must be empty, got: {:?}",
        project.media_properties()
    );
}

// ── `grid="…"` — grid-mode plan step 3 (GridWithoutMedia) ─────────────────────

#[test]
fn broken_grid_without_media_hard_fails() {
    // resource-list opts into grid mode (grid=#true) but declares no `media`
    // attribute → GridWithoutMedia hard-fail. Pure config check — fires
    // regardless of data (`check_grid_without_media`, a pages walk hooked
    // into `validate_inner` alongside `check_class_routes`).
    assert_exact_kinds(
        "broken-grid-without-media",
        "broken-grid-without-media",
        &[DiagnosticKind::GridWithoutMedia { class: "Thing".to_owned(), page: "/list".to_owned() }],
    );
}

// ── SparseProperty on a PARAMETERISED resource-detail ─────────────────────────

#[test]
fn broken_sparse_property_param_warns_over_the_declared_class() {
    // A parameterised resource-detail (`/things/:id`) names no single
    // resource, so the literal-id pair (UnknownResource + the id-keyed sparse
    // lookup) cannot run — it used to be skipped entirely, leaving the common
    // detail-page shape with no show-properties coverage at all. The declared
    // `class` now drives the sparse half.
    let project = load_fixture_project("broken-sparse-property-param", "broken-sparse-property-param")
        .expect("broken-sparse-property-param should load (SparseProperty is advisory)");

    let warnings = project.warnings();
    for w in warnings {
        assert_eq!(
            w.severity,
            Severity::Warning,
            "the parameterised sparse check must never harden into an error: {w:?}"
        );
    }

    let sparse: Vec<_> = warnings.iter().filter(|w| w.kind == DiagnosticKind::SparseProperty).collect();
    assert_eq!(
        sparse.len(),
        2,
        "expected exactly 2 SparseProperty warnings (show-properties + \
         title-property), got: {warnings:?}"
    );
    assert!(
        sparse.iter().any(|w| w.message.contains("missing-prop")),
        "expected a SparseProperty warning naming 'missing-prop': {warnings:?}"
    );
    assert!(
        sparse.iter().any(|w| w.message.contains("nonexistent-prop")),
        "expected a SparseProperty warning naming 'nonexistent-prop': {warnings:?}"
    );
    assert!(
        sparse.iter().all(|w| w.message.contains("'Thing'")),
        "the message must name the declared class, not a resource id: {warnings:?}"
    );
}

#[test]
fn good_sparse_property_param_does_not_warn() {
    // The non-firing case: every listed property is present on the class.
    // `subtitle` is on 1 of 2 Things — uneven, but present, which is NOT a
    // SparseProperty (same union semantics the literal-id path has always
    // used).
    let project = load_fixture_project("good-sparse-property-param", "good-sparse-property-param")
        .expect("good-sparse-property-param should load successfully");
    assert!(
        project.warnings().is_empty(),
        "a parameterised detail whose properties all exist on the class must \
         produce no warnings, got: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_sparse_property_param_raw_branch_surfaces_warning() {
    // The raw sibling (check_resource_detail_data_into_raw's `$param` branch):
    // when config-internal lowering fails, the class-keyed sparse check must
    // still run off the RawConfig so a dual failure surfaces both halves in
    // one boot pass.
    let diagnostics = load_fixture_diagnostics("broken-sparse-property-param-raw", "broken-sparse-property-param-raw")
        .unwrap_err_or_else(|| {
            panic!(
                "broken-sparse-property-param-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
            )
        });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-sparse-property-param-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::SparseProperty && d.message.contains("missing-prop")),
        "broken-sparse-property-param-raw must ALSO surface the class-keyed \
         SparseProperty warning: {diagnostics:?}"
    );
}

// ── Incunabula Slice 3 (step 6): group / narrative-for boot validation ───────

#[test]
fn broken_group_duplicate_property_hard_fails() {
    // A `group` names a property also named by `show-properties`.
    assert_exact_kinds(
        "broken-group-duplicate-property",
        "broken-group-duplicate-property",
        &[DiagnosticKind::DuplicateGroupedProperty { property: "name".to_owned(), page: "/detail".to_owned() }],
    );
}

#[test]
fn broken_group_unknown_label_succeeds_with_warning() {
    // A `group label=` id that is not a declared chrome string id is
    // advisory (UnknownGroupLabel is a Warning) — load must succeed.
    let project = load_fixture_project("broken-group-unknown-label", "broken-group-unknown-label")
        .expect("broken-group-unknown-label should load successfully (warning only)");
    let warnings = project.warnings();
    assert!(
        warnings.iter().any(|w| w.kind
            == DiagnosticKind::UnknownGroupLabel { label: "not-declared".to_owned(), page: "/detail".to_owned() }),
        "expected an UnknownGroupLabel warning naming 'not-declared': {warnings:?}"
    );
}

#[test]
fn broken_group_links_label_succeeds_with_warning() {
    // A `group links-label=` id that is not a declared chrome string id is
    // advisory (UnknownGroupLinksLabel is a Warning) — load must succeed.
    let project = load_fixture_project("broken-group-links-label", "broken-group-links-label")
        .expect("broken-group-links-label should load successfully (warning only)");
    let kinds: Vec<_> = project.warnings().iter().map(|w| &w.kind).collect();
    assert_eq!(
        kinds,
        vec![&DiagnosticKind::UnknownGroupLinksLabel { label: "not-declared".to_owned(), page: "/detail".to_owned() }],
        "exactly the links-label warning"
    );
}

#[test]
fn broken_narrative_for_duplicate_hard_fails() {
    // Two `narrative-for` entries name the same resource id.
    assert_exact_kinds(
        "broken-narrative-for-duplicate",
        "broken-narrative-for-duplicate",
        &[DiagnosticKind::DuplicateNarrativeFor { resource_id: "r1".to_owned(), page: "/detail".to_owned() }],
    );
}

#[test]
fn broken_narrative_for_unknown_resource_succeeds_with_warning() {
    // A `narrative-for` id that does not exist in the data is advisory
    // (UnknownNarrativeForResource is a Warning) — load must succeed.
    let project =
        load_fixture_project("broken-narrative-for-unknown-resource", "broken-narrative-for-unknown-resource")
            .expect("broken-narrative-for-unknown-resource should load successfully (warning only)");
    let warnings = project.warnings();
    assert!(
        warnings.iter().any(|w| w.kind
            == DiagnosticKind::UnknownNarrativeForResource {
                resource_id: "ghost".to_owned(),
                page: "/detail".to_owned(),
            }),
        "expected an UnknownNarrativeForResource warning naming 'ghost': {warnings:?}"
    );
}

#[test]
fn broken_narrative_for_unknown_resource_raw_branch_surfaces_warning() {
    // Layer-4 test for check_narrative_for_data_into_raw (the Err branch of
    // validate_with_data). The fixture has:
    //   (a) MissingSiteTitle — a config-internal error that forces validate()
    //       to return Err, routing validate_with_data into the Err/raw path.
    //   (b) narrative-for "ghost" file="a.md" — a literal id that does not
    //       exist in data.sql.
    //
    // Expected: load fails (MissingSiteTitle is a hard error), AND the
    // UnknownNarrativeForResource warning is present in the returned
    // diagnostics — proving check_narrative_for_data_into_raw ran on the raw
    // config and surfaced the warning even though config-internal validation
    // also failed.
    let diagnostics = load_fixture_diagnostics(
        "broken-narrative-for-unknown-resource-raw",
        "broken-narrative-for-unknown-resource-raw",
    )
    .unwrap_err_or_else(|| {
        panic!(
            "broken-narrative-for-unknown-resource-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
        )
    });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-narrative-for-unknown-resource-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::UnknownNarrativeForResource {
                resource_id: "ghost".to_owned(),
                page: "/detail".to_owned(),
            }),
        "broken-narrative-for-unknown-resource-raw must ALSO surface the \
         UnknownNarrativeForResource warning: {diagnostics:?}"
    );
}

#[test]
fn broken_narrative_for_file_not_found_hard_fails() {
    // narrative-for's file= attribute runs the same NarrativeFileNotFound
    // check the page-level `narrative` component does (distinct from
    // broken-narrative-missing, which exercises the page-level component).
    assert_exact_kinds(
        "broken-narrative-for-file-not-found",
        "broken-narrative-for-file-not-found",
        &[DiagnosticKind::NarrativeFileNotFound],
    );
}

#[test]
fn broken_group_sparse_member_succeeds_with_warning() {
    // A group member absent from the resource's class is advisory
    // (SparseProperty is a Warning) — load must succeed. Pins step 1's
    // detail_property_names extension to group members.
    let project = load_fixture_project("broken-group-sparse-member", "broken-group-sparse-member")
        .expect("broken-group-sparse-member should load successfully (warning only)");
    let warnings = project.warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.kind == DiagnosticKind::SparseProperty && w.message.contains("missing-prop")),
        "expected a SparseProperty warning naming 'missing-prop': {warnings:?}"
    );
}

#[test]
fn broken_group_sparse_member_raw_branch_surfaces_warning() {
    // Layer-4 test for check_resource_detail_data_into_raw's `group`
    // positional-args extraction (step 1), on the literal-id branch: the
    // fixture has MissingSiteTitle (forcing the Err/raw path) plus a group
    // member absent from the class. Both diagnostics must appear in one
    // returned Vec.
    let diagnostics = load_fixture_diagnostics("broken-group-sparse-member-raw", "broken-group-sparse-member-raw")
        .unwrap_err_or_else(|| {
            panic!(
                "broken-group-sparse-member-raw should fail validation \
             (MissingSiteTitle is a hard error) but validated clean"
            )
        });

    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-group-sparse-member-raw must contain MissingSiteTitle \
         (the config-internal error that forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::SparseProperty && d.message.contains("missing-prop")),
        "broken-group-sparse-member-raw must ALSO surface the SparseProperty \
         warning (via the raw arm's group-args extraction): {diagnostics:?}"
    );
}

#[test]
fn good_group_narrative_for_validates_clean() {
    // Positive path: a group (all members present, valid label) AND a
    // narrative-for entry (valid resource id, valid file) both resolve
    // cleanly — zero diagnostics.
    assert_clean("good-group-narrative-for", "good-group-narrative-for");
}

// ── Incunabula Slice 5 (step 6): incoming-links boot validation ──────────────

/// Assert a `-raw` fixture fails with BOTH the forcing `MissingSiteTitle` and
/// `expected` — proving `check_incoming_links_data_into_raw` ran on the
/// Err/raw branch.
fn assert_raw_arm_surfaces(fixture: &str, expected: DiagnosticKind) {
    let diagnostics = load_fixture_diagnostics(fixture, fixture)
        .unwrap_err_or_else(|| panic!("{fixture} should fail validation (MissingSiteTitle) but validated clean"));
    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "{fixture} must contain MissingSiteTitle (forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind == expected),
        "{fixture} must ALSO contain {expected:?} — its absence means the raw \
         incoming-links sibling never ran: {diagnostics:?}"
    );
}

fn strips_page() -> String {
    "/strips/:id".to_owned()
}

#[test]
fn broken_incoming_links_misses_host_warns_in_both_arms() {
    // Success arm: 'strip-left' is fully link-typed on Page, but every
    // stored target is a Woodcut, never this page's own host class
    // (BorderStrip) — the list would always be empty. Advisory only, so
    // the project must still boot.
    let project = load_fixture_project("broken-incoming-links-misses-host", "broken-incoming-links-misses-host")
        .expect("broken-incoming-links-misses-host should load successfully (warning only)");
    let warnings = project.warnings();
    assert!(
        warnings.iter().any(|w| w.kind
            == DiagnosticKind::IncomingLinksPropertyMissesHost {
                property: "strip-left".to_owned(),
                class: "Page".to_owned(),
                host_class: "BorderStrip".to_owned(),
                page: strips_page(),
            }),
        "expected IncomingLinksPropertyMissesHost among warnings: {warnings:?}"
    );

    // Raw arm: the same warning must surface beside the forcing MissingSiteTitle.
    assert_raw_arm_surfaces(
        "broken-incoming-links-misses-host-raw",
        DiagnosticKind::IncomingLinksPropertyMissesHost {
            property: "strip-left".to_owned(),
            class: "Page".to_owned(),
            host_class: "BorderStrip".to_owned(),
            page: strips_page(),
        },
    );
}

#[test]
fn incoming_links_targeting_host_does_not_warn() {
    // A correctly-configured incoming-links (targets match the host class)
    // must never fire IncomingLinksPropertyMissesHost.
    let project = load_fixture_project("good-incoming-links-targets-host", "good-incoming-links-targets-host")
        .expect("good-incoming-links-targets-host should validate clean");
    assert!(
        !project
            .warnings()
            .iter()
            .any(|w| matches!(w.kind, DiagnosticKind::IncomingLinksPropertyMissesHost { .. })),
        "no IncomingLinksPropertyMissesHost expected: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_incoming_links_unknown_class_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-unknown-class",
        "broken-incoming-links-unknown-class",
        &[DiagnosticKind::UnknownIncomingLinksClass { class: "Paeg".to_owned(), page: strips_page() }],
    );
}

#[test]
fn broken_incoming_links_unknown_class_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces(
        "broken-incoming-links-unknown-class-raw",
        DiagnosticKind::UnknownIncomingLinksClass { class: "Paeg".to_owned(), page: strips_page() },
    );
}

#[test]
fn broken_incoming_links_unknown_property_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-unknown-property",
        "broken-incoming-links-unknown-property",
        &[DiagnosticKind::UnknownIncomingLinksProperty {
            property: "strip-middle".to_owned(),
            class: "Page".to_owned(),
            page: strips_page(),
        }],
    );
}

#[test]
fn broken_incoming_links_unknown_property_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces(
        "broken-incoming-links-unknown-property-raw",
        DiagnosticKind::UnknownIncomingLinksProperty {
            property: "strip-middle".to_owned(),
            class: "Page".to_owned(),
            page: strips_page(),
        },
    );
}

#[test]
fn broken_incoming_links_sparse_succeeds_with_warning() {
    // `side` exists in the data (on BorderStrip) but on no Page — advisory.
    let project = load_fixture_project("broken-incoming-links-sparse", "broken-incoming-links-sparse")
        .expect("broken-incoming-links-sparse should load successfully (warning only)");
    let warnings = project.warnings();
    let hits: Vec<_> = warnings
        .iter()
        .filter(|w| {
            w.kind
                == DiagnosticKind::SparseIncomingLinksProperty {
                    property: "side".to_owned(),
                    class: "Page".to_owned(),
                    page: strips_page(),
                }
        })
        .collect();
    assert_eq!(hits.len(), 1, "expected one SparseIncomingLinksProperty: {warnings:?}");
    assert_eq!(hits[0].severity, Severity::Warning);
}

#[test]
fn broken_incoming_links_sparse_raw_branch_surfaces_warning() {
    assert_raw_arm_surfaces(
        "broken-incoming-links-sparse-raw",
        DiagnosticKind::SparseIncomingLinksProperty {
            property: "side".to_owned(),
            class: "Page".to_owned(),
            page: strips_page(),
        },
    );
}

#[test]
fn broken_incoming_links_not_link_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-not-link",
        "broken-incoming-links-not-link",
        &[DiagnosticKind::IncomingLinksPropertyNotLink {
            property: "name".to_owned(),
            value_type: "text".to_owned(),
            class: "Page".to_owned(),
            page: strips_page(),
        }],
    );
}

#[test]
fn broken_incoming_links_not_link_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces(
        "broken-incoming-links-not-link-raw",
        DiagnosticKind::IncomingLinksPropertyNotLink {
            property: "name".to_owned(),
            value_type: "text".to_owned(),
            class: "Page".to_owned(),
            page: strips_page(),
        },
    );
}

/// The three HdM Slice 3 row-property diagnostics
/// `broken-incoming-links-row-props{,-raw}` must raise.
fn row_props_kinds() -> Vec<DiagnosticKind> {
    let (class, page) = ("Work".to_owned(), "/people/:id".to_owned());
    vec![
        DiagnosticKind::UnknownIncomingLinksRowProperty {
            attribute: "sort-property".to_owned(),
            property: "nope".to_owned(),
            class: class.clone(),
            page: page.clone(),
        },
        DiagnosticKind::IncomingLinksRowPropertyWrongType {
            attribute: "row-aside".to_owned(),
            property: "count".to_owned(),
            value_type: "integer".to_owned(),
            class: class.clone(),
            page: page.clone(),
        },
        DiagnosticKind::SparseIncomingLinksRowProperty {
            attribute: "row-label".to_owned(),
            property: "title".to_owned(),
            class,
            page,
        },
    ]
}

#[test]
fn broken_incoming_links_row_props_hard_fails_with_all_three() {
    // `group-by="category"` is text on every linking Work, so it adds nothing.
    assert_exact_kinds(
        "broken-incoming-links-row-props",
        "broken-incoming-links-row-props",
        &row_props_kinds(),
    );
}

#[test]
fn broken_incoming_links_row_props_raw_branch_surfaces_all_three() {
    for kind in row_props_kinds() {
        assert_raw_arm_surfaces("broken-incoming-links-row-props-raw", kind);
    }
}

fn row_props_more_kinds() -> Vec<DiagnosticKind> {
    let (class, page) = ("Work".to_owned(), "/people/:id".to_owned());
    let wrong = |attribute: &str| DiagnosticKind::IncomingLinksRowPropertyWrongType {
        attribute: attribute.to_owned(),
        property: "count".to_owned(),
        value_type: "integer".to_owned(),
        class: class.clone(),
        page: page.clone(),
    };
    let unknown = |attribute: &str, property: &str| DiagnosticKind::UnknownIncomingLinksRowProperty {
        attribute: attribute.to_owned(),
        property: property.to_owned(),
        class: class.clone(),
        page: page.clone(),
    };
    vec![
        unknown("group-by", "nope-group"),
        wrong("row-label"),
        unknown("row-aside", "void"),
        wrong("sort-property"),
    ]
}

fn row_props_typed_kinds() -> Vec<DiagnosticKind> {
    let (class, page) = ("Work".to_owned(), "/people/:id".to_owned());
    vec![
        DiagnosticKind::IncomingLinksRowPropertyWrongType {
            attribute: "group-by".to_owned(),
            property: "count".to_owned(),
            value_type: "integer".to_owned(),
            class: class.clone(),
            page: page.clone(),
        },
        DiagnosticKind::SparseIncomingLinksRowProperty {
            attribute: "row-aside".to_owned(),
            property: "note".to_owned(),
            class,
            page,
        },
    ]
}

#[test]
fn broken_incoming_links_row_props_more_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-row-props-more",
        "broken-incoming-links-row-props-more",
        &row_props_more_kinds(),
    );
}

#[test]
fn broken_incoming_links_row_props_more_raw_branch_surfaces_all() {
    for kind in row_props_more_kinds() {
        assert_raw_arm_surfaces("broken-incoming-links-row-props-more-raw", kind);
    }
}

#[test]
fn broken_incoming_links_row_props_typed_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-row-props-typed",
        "broken-incoming-links-row-props-typed",
        &row_props_typed_kinds(),
    );
}

#[test]
fn broken_incoming_links_row_props_typed_raw_branch_surfaces_all() {
    for kind in row_props_typed_kinds() {
        assert_raw_arm_surfaces("broken-incoming-links-row-props-typed-raw", kind);
    }
}

#[test]
fn broken_incoming_links_no_property_hard_fails() {
    assert_exact_kinds(
        "broken-incoming-links-no-property",
        "broken-incoming-links-no-property",
        &[DiagnosticKind::IncomingLinksWithoutProperty],
    );
}

#[test]
fn broken_incoming_links_duplicate_hard_fails() {
    // Two `incoming-links` children on one `resource-detail` — at most one
    // reverse-link list per record. Config-internal (checked in
    // `lower_resource_detail`, span-located, not page-keyed).
    assert_exact_kinds(
        "broken-incoming-links-duplicate",
        "broken-incoming-links-duplicate",
        &[DiagnosticKind::DuplicateIncomingLinks],
    );
}

#[test]
fn broken_incoming_links_unknown_label_succeeds_with_warning() {
    // An `incoming-links label=` id that is not a declared chrome string id
    // is advisory (UnknownIncomingLinksLabel is a Warning) — load must
    // succeed.
    let project = load_fixture_project("broken-incoming-links-unknown-label", "broken-incoming-links-unknown-label")
        .expect("broken-incoming-links-unknown-label should load successfully (warning only)");
    let warnings = project.warnings();
    assert!(
        warnings.iter().any(|w| w.kind
            == DiagnosticKind::UnknownIncomingLinksLabel { label: "not-declared".to_owned(), page: strips_page() }),
        "expected an UnknownIncomingLinksLabel warning naming 'not-declared': {warnings:?}"
    );
}

// ── `resource-detail class` existence check (Backlog hardening batch 3 item 2) ─

fn characters_detail_page() -> String {
    "/characters/:id".to_owned()
}

#[test]
fn broken_resource_detail_unknown_class_is_error_in_both_arms() {
    let expected = DiagnosticKind::UnknownResourceDetailClass {
        class: "Charakter".to_owned(),
        page: characters_detail_page(),
    };

    // Success arm: exactly this one kind — no SparseProperty must accompany
    // it (the class is unknown, so the properties-on-class check is skipped
    // entirely).
    assert_exact_kinds(
        "broken-resource-detail-unknown-class",
        "broken-resource-detail-unknown-class",
        std::slice::from_ref(&expected),
    );

    // Raw arm: same kind, alongside MissingSiteTitle (which forces the raw
    // branch), and still no SparseProperty.
    let diagnostics = load_fixture_diagnostics(
        "broken-resource-detail-unknown-class-raw",
        "broken-resource-detail-unknown-class-raw",
    )
    .unwrap_err_or_else(|| {
        panic!(
            "broken-resource-detail-unknown-class-raw should fail validation \
             (MissingSiteTitle) but validated clean"
        )
    });
    assert!(
        diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-resource-detail-unknown-class-raw must contain MissingSiteTitle \
         (forces the Err/raw branch): {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|d| d.kind == expected),
        "broken-resource-detail-unknown-class-raw must ALSO contain {expected:?} — its \
         absence means the raw resource-detail class-existence check never ran: {diagnostics:?}"
    );
    assert!(
        !diagnostics.iter().any(|d| matches!(&d.kind, DiagnosticKind::SparseProperty)),
        "broken-resource-detail-unknown-class-raw must NOT contain SparseProperty — the \
         properties-on-class check must be skipped when the class is unknown: {diagnostics:?}"
    );
}

#[test]
fn broken_resource_detail_unknown_class_with_media_reports_once_in_both_arms() {
    let expected = DiagnosticKind::UnknownResourceDetailClass {
        class: "Charakter".to_owned(),
        page: characters_detail_page(),
    };
    let is_cascaded_media_kind = |k: &DiagnosticKind| {
        matches!(
            k,
            DiagnosticKind::SparseMediaProperty { .. }
                | DiagnosticKind::UnknownMediaProperty { .. }
                | DiagnosticKind::IncompatibleMediaProperty { .. }
                | DiagnosticKind::MediaPropertyTargetNotImagelike { .. }
        )
    };

    // Success arm.
    let diagnostics = load_fixture_diagnostics(
        "broken-resource-detail-unknown-class-media",
        "broken-resource-detail-unknown-class-media",
    )
    .unwrap_err_or_else(|| {
        panic!("broken-resource-detail-unknown-class-media should fail validation but validated clean")
    });
    assert!(
        diagnostics.iter().any(|d| d.kind == expected),
        "broken-resource-detail-unknown-class-media must contain {expected:?}: {diagnostics:?}"
    );
    assert!(
        !diagnostics.iter().any(|d| is_cascaded_media_kind(&d.kind)),
        "broken-resource-detail-unknown-class-media must NOT contain a cascaded media \
         diagnostic — the media four-check must be skipped when the class is unknown: \
         {diagnostics:?}"
    );

    // Raw arm.
    let diagnostics_raw = load_fixture_diagnostics(
        "broken-resource-detail-unknown-class-media-raw",
        "broken-resource-detail-unknown-class-media-raw",
    )
    .unwrap_err_or_else(|| {
        panic!(
            "broken-resource-detail-unknown-class-media-raw should fail validation \
             (MissingSiteTitle) but validated clean"
        )
    });
    assert!(
        diagnostics_raw
            .iter()
            .any(|d| matches!(&d.kind, DiagnosticKind::MissingSiteTitle)),
        "broken-resource-detail-unknown-class-media-raw must contain MissingSiteTitle \
         (forces the Err/raw branch): {diagnostics_raw:?}"
    );
    assert!(
        diagnostics_raw.iter().any(|d| d.kind == expected),
        "broken-resource-detail-unknown-class-media-raw must ALSO contain {expected:?}: \
         {diagnostics_raw:?}"
    );
    assert!(
        !diagnostics_raw.iter().any(|d| is_cascaded_media_kind(&d.kind)),
        "broken-resource-detail-unknown-class-media-raw must NOT contain a cascaded media \
         diagnostic: {diagnostics_raw:?}"
    );
}

// ── CompoundViewerWithPagedList Layer-4 coverage ─────────────────────────────
// `diagnostic.rs` ~513 — validate.rs already has a unit-level test
// (`compound_viewer_with_per_page_list_on_one_page_fails`); this is the
// missing Layer-4 fixture-driven mirror of the other `broken-*` families.

#[test]
fn broken_compound_with_paged_list_hard_fails() {
    assert_exact_kinds(
        "broken-compound-with-paged-list",
        "broken-compound-with-paged-list",
        &[DiagnosticKind::CompoundViewerWithPagedList { page: "/books/:id".to_owned() }],
    );
}

// ── G9: `order-by` options on resource-list (Incunabula Slice 8) ──────────────

fn books_page() -> String {
    "/books".to_owned()
}

#[test]
fn broken_order_by_reserved_key_hard_fails() {
    assert_exact_kinds(
        "broken-order-by-reserved-key",
        "broken-order-by-reserved-key",
        &[DiagnosticKind::ReservedOrderKey { key: "name".to_owned() }],
    );
}

#[test]
fn broken_order_by_duplicate_key_hard_fails() {
    assert_exact_kinds(
        "broken-order-by-duplicate-key",
        "broken-order-by-duplicate-key",
        &[DiagnosticKind::DuplicateOrderKey { key: "date".to_owned() }],
    );
}

#[test]
fn broken_order_by_invalid_key_hard_fails() {
    assert_exact_kinds(
        "broken-order-by-invalid-key",
        "broken-order-by-invalid-key",
        &[DiagnosticKind::InvalidOrderKey { key: "Date".to_owned() }],
    );
}

#[test]
fn broken_order_by_unknown_label_hard_fails() {
    assert_exact_kinds(
        "broken-order-by-unknown-label",
        "broken-order-by-unknown-label",
        &[DiagnosticKind::UnknownOrderLabel { label: "order-typo".to_owned(), page: books_page() }],
    );
}

#[test]
fn broken_order_by_wrong_typed_key_reports_instead_of_vanishing() {
    // `key=#true` must be a boot error, not a silently-dropped option
    // (Slice 8 decision 11a — the reporting readers).
    assert_exact_kinds(
        "broken-order-by-wrong-type",
        "broken-order-by-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_order_by_unknown_property_hard_fails() {
    assert_exact_kinds(
        "broken-order-by-unknown-property",
        "broken-order-by-unknown-property",
        &[DiagnosticKind::UnknownResourceListProperty {
            property: "no-such-prop".to_owned(),
            class: "Book".to_owned(),
            page: books_page(),
        }],
    );
}

#[test]
fn broken_order_by_unknown_property_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces(
        "broken-order-by-unknown-property-raw",
        DiagnosticKind::UnknownResourceListProperty {
            property: "no-such-prop".to_owned(),
            class: "Book".to_owned(),
            page: books_page(),
        },
    );
}

#[test]
fn broken_order_name_property_unknown_hard_fails() {
    assert_exact_kinds(
        "broken-order-name-property-unknown",
        "broken-order-name-property-unknown",
        &[DiagnosticKind::UnknownResourceListProperty {
            property: "no-such-prop".to_owned(),
            class: "Book".to_owned(),
            page: books_page(),
        }],
    );
}

#[test]
fn broken_order_name_property_unknown_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces(
        "broken-order-name-property-unknown-raw",
        DiagnosticKind::UnknownResourceListProperty {
            property: "no-such-prop".to_owned(),
            class: "Book".to_owned(),
            page: books_page(),
        },
    );
}

#[test]
fn broken_order_default_label_wrong_type_reports_instead_of_vanishing() {
    assert_exact_kinds(
        "broken-order-default-label-wrong-type",
        "broken-order-default-label-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

// ── Composed row gloss ────────────────────────────────────────────────────────

fn incompatible_gloss(property: &str, role: &str, value_type: &str) -> DiagnosticKind {
    DiagnosticKind::IncompatibleGlossPart {
        property: property.to_owned(),
        role: role.to_owned(),
        value_type: value_type.to_owned(),
        class: "Book".to_owned(),
        page: books_page(),
    }
}

#[test]
fn broken_gloss_incompatible_part_hard_fails() {
    assert_exact_kinds(
        "broken-gloss-incompatible-part",
        "broken-gloss-incompatible-part",
        &[
            incompatible_gloss("office", "part", "list_value"),
            incompatible_gloss("pub-place", "unit", "text"),
        ],
    );
}

#[test]
fn broken_gloss_incompatible_part_raw_branch_surfaces_error() {
    for expected in [
        incompatible_gloss("office", "part", "list_value"),
        incompatible_gloss("pub-place", "unit", "text"),
    ] {
        assert_raw_arm_surfaces("broken-gloss-incompatible-part-raw", expected);
    }
}

#[test]
fn broken_gloss_unknown_unit_hard_fails() {
    assert_exact_kinds(
        "broken-gloss-unknown-unit",
        "broken-gloss-unknown-unit",
        &[DiagnosticKind::UnknownGlossUnit { unit: "gloss-extent".to_owned(), page: books_page() }],
    );
}

#[test]
fn broken_gloss_unit_missing_placeholder_hard_fails() {
    assert_exact_kinds(
        "broken-gloss-unit-missing-placeholder",
        "broken-gloss-unit-missing-placeholder",
        &[DiagnosticKind::GlossUnitMissingPlaceholder { unit: "gloss-extent".to_owned(), lang: "en".to_owned() }],
    );
}

#[test]
fn broken_gloss_block_shape_errors_hard_fail() {
    for fixture in ["broken-gloss-property-and-parts", "broken-gloss-no-part"] {
        assert_exact_kinds(fixture, fixture, &[DiagnosticKind::InvalidAttributeType]);
    }
    assert_exact_kinds(
        "broken-gloss-unknown-child",
        "broken-gloss-unknown-child",
        &[DiagnosticKind::UnknownChildNode { component: "gloss".to_owned(), child: "prat".to_owned() }],
    );
}

// ── Record header slots ───────────────────────────────────────────────────────

fn incompatible_head_slot(property: &str, role: &str, value_type: &str) -> DiagnosticKind {
    DiagnosticKind::IncompatibleGlossPart {
        property: property.to_owned(),
        role: role.to_owned(),
        value_type: value_type.to_owned(),
        class: "Book".to_owned(),
        page: "/books/:id".to_owned(),
    }
}

fn head_slot_incompatibilities() -> [DiagnosticKind; 3] {
    [
        incompatible_head_slot("extent", "alt-title", "integer"),
        incompatible_head_slot("office", "author", "list_value"),
        incompatible_head_slot("pub-place", "imprint unit", "text"),
    ]
}

#[test]
fn broken_head_slot_incompatible_hard_fails() {
    assert_exact_kinds(
        "broken-head-slot-incompatible",
        "broken-head-slot-incompatible",
        &head_slot_incompatibilities(),
    );
}

#[test]
fn broken_head_slot_incompatible_raw_branch_surfaces_error() {
    for expected in head_slot_incompatibilities() {
        assert_raw_arm_surfaces("broken-head-slot-incompatible-raw", expected);
    }
}

// ── compound-viewer member-links (book-page step 6a) ─────────────────────────

fn member_link_not_link() -> DiagnosticKind {
    DiagnosticKind::IncompatibleGlossPart {
        property: "caption".to_owned(),
        role: "member link".to_owned(),
        value_type: "text".to_owned(),
        class: "Page".to_owned(),
        page: "/books/:id".to_owned(),
    }
}

#[test]
fn broken_member_links_not_link_hard_fails() {
    assert_exact_kinds(
        "broken-member-links-not-link",
        "broken-member-links-not-link",
        &[member_link_not_link()],
    );
}

#[test]
fn broken_member_links_not_link_raw_branch_surfaces_error() {
    assert_raw_arm_surfaces("broken-member-links-not-link-raw", member_link_not_link());
}

#[test]
fn broken_member_links_warnings_load_with_both_warnings() {
    let project = load_fixture_project("broken-member-links-warnings", "broken-member-links-warnings")
        .expect("an unknown caption and an absent property are advisory");
    let kinds: Vec<&DiagnosticKind> = project.warnings().iter().map(|w| &w.kind).collect();
    assert!(
        kinds.contains(&&DiagnosticKind::UnknownGroupLabel {
            label: "no-such-caption".to_owned(),
            page: "/books/:id".to_owned(),
        }),
        "expected UnknownGroupLabel: {kinds:?}"
    );
    assert!(
        project
            .warnings()
            .iter()
            .any(|w| w.kind == DiagnosticKind::SparseProperty && w.message.contains("'strip-top'")),
        "expected SparseProperty for strip-top: {:?}",
        project.warnings()
    );
    assert_eq!(project.warnings().len(), 2, "{:?}", project.warnings());
}

#[test]
fn broken_imprint_unknown_child_hard_fails() {
    assert_exact_kinds(
        "broken-imprint-unknown-child",
        "broken-imprint-unknown-child",
        &[DiagnosticKind::UnknownChildNode { component: "imprint".to_owned(), child: "teaser".to_owned() }],
    );
}

#[test]
fn broken_order_by_sparse_property_loads_with_warning() {
    let project = load_fixture_project("broken-order-by-sparse", "broken-order-by-sparse")
        .expect("broken-order-by-sparse should load (SparseResourceListProperty is a Warning)");
    let expected = DiagnosticKind::SparseResourceListProperty {
        property: "extent".to_owned(),
        class: "Book".to_owned(),
        page: books_page(),
    };
    assert!(
        project.warnings().iter().any(|w| w.kind == expected),
        "expected {expected:?} among the warnings: {:?}",
        project.warnings()
    );
}

// ── timeline (Incunabula Slice 4) ─────────────────────────────────────────────
//
// Every timeline fixture is loaded as project "incunabula": step 8 keys the
// component on `(project, name)`, so these fixtures pass through step 8
// unchanged only if they are loaded as Incunabula from the start.

const TIMELINE_PROJECT: &str = "incunabula";

fn home_page() -> String {
    "/".to_owned()
}

/// Assert a timeline fixture fails with exactly `expected` — as a multiset:
/// same kinds, same counts (order-insensitive).
fn assert_timeline_errors(fixture: &str, expected: &[DiagnosticKind]) {
    let diagnostics = load_fixture_diagnostics(fixture, TIMELINE_PROJECT)
        .unwrap_err_or_else(|| panic!("{fixture} should fail validation but validated clean"));
    let mut got: Vec<String> = diagnostics.iter().map(|d| format!("{:?}", d.kind)).collect();
    let mut want: Vec<String> = expected.iter().map(|k| format!("{k:?}")).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "{fixture}: diagnostic multiset mismatch\n  full: {diagnostics:?}");
}

fn timeline_incompatible(property: &str, value_type: &str, expected: &str) -> DiagnosticKind {
    DiagnosticKind::IncompatibleTimelineProperty {
        property: property.to_owned(),
        value_type: value_type.to_owned(),
        expected: expected.to_owned(),
        class: "Book".to_owned(),
        page: home_page(),
    }
}

#[test]
fn good_timeline_validates_clean_with_no_timeline_warning() {
    let project = load_fixture_project("good-timeline", TIMELINE_PROJECT)
        .unwrap_or_else(|kinds| panic!("good-timeline should validate clean but got: {kinds:?}"));
    assert!(
        !project
            .warnings()
            .iter()
            .any(|w| matches!(w.kind, DiagnosticKind::SparseTimelineProperty { .. })),
        "a fully-carried timeline must raise no SparseTimelineProperty: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_timeline_unknown_class_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-unknown-class",
        &[DiagnosticKind::UnknownTimelineClass { class: "Bok".to_owned(), page: home_page() }],
    );
}

#[test]
fn broken_timeline_unknown_property_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-unknown-property",
        &[DiagnosticKind::UnknownTimelineProperty {
            property: "publication-date".to_owned(),
            class: "Book".to_owned(),
            page: home_page(),
        }],
    );
}

#[test]
fn broken_timeline_date_absent_from_class_hard_fails() {
    // The stated deviation: absent-from-class is an Error for the date (and
    // lane), not the sparse Warning `check_single_resource_list` uses.
    assert_timeline_errors(
        "broken-timeline-date-not-on-class",
        &[DiagnosticKind::MissingTimelineProperty {
            property: "scan-date".to_owned(),
            class: "Book".to_owned(),
            page: home_page(),
        }],
    );
}

#[test]
fn broken_timeline_date_wrong_type_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-date-wrong-type",
        &[timeline_incompatible("name", "text", "date")],
    );
}

#[test]
fn broken_timeline_lane_wrong_type_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-lane-wrong-type",
        &[timeline_incompatible("extent", "integer", "list_value")],
    );
}

#[test]
fn broken_timeline_extent_wrong_type_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-extent-wrong-type",
        &[timeline_incompatible("name", "text", "integer")],
    );
}

#[test]
fn broken_timeline_extent_attr_wrong_type_reports_instead_of_vanishing() {
    // `extent-property=1` must be a boot error, not a silently-dropped
    // extent segment — the reporting reader `get_typed_string_attr`.
    assert_timeline_errors(
        "broken-timeline-extent-attr-wrong-type",
        &[DiagnosticKind::InvalidAttributeType],
    );
}

#[test]
fn broken_timeline_extent_absent_from_class_loads_with_warning() {
    let project = load_fixture_project("broken-timeline-extent-sparse", TIMELINE_PROJECT)
        .expect("an extent absent from the class is only a Warning");
    let expected = DiagnosticKind::SparseTimelineProperty {
        property: "page-count".to_owned(),
        class: "Book".to_owned(),
        page: home_page(),
    };
    let hits = project.warnings().iter().filter(|w| w.kind == expected).count();
    assert_eq!(
        hits,
        1,
        "expected exactly one {expected:?} among the warnings: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_timeline_axis_start_missing_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-axis-start-missing",
        &[DiagnosticKind::MissingRequiredAttribute],
    );
}

#[test]
fn broken_timeline_axis_start_string_hard_fails() {
    assert_timeline_errors("broken-timeline-axis-start-string", &[DiagnosticKind::InvalidAttributeType]);
}

#[test]
fn broken_timeline_axis_reversed_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-axis-reversed",
        &[DiagnosticKind::InvalidTimelineAxis { axis_start: 1506, axis_end: 1470 }],
    );
}

#[test]
fn broken_timeline_axis_out_of_range_hard_fails() {
    assert_timeline_errors(
        "broken-timeline-axis-out-of-range",
        &[DiagnosticKind::TimelineAxisOutOfRange { attribute: "axis-end".to_owned() }],
    );
}

/// Step 8's `(project, name)` keying: a project that is not Incunabula but
/// declares `timeline` gets exactly `{UnknownComponent}` — no timeline data
/// diagnostics from the raw arm, even when the data would also break the
/// timeline's checks.
#[test]
fn timeline_declared_by_another_project_is_exactly_unknown_component() {
    for fixture in ["good-timeline", "broken-timeline-date-wrong-type"] {
        let diagnostics = load_fixture_diagnostics(fixture, "alice-in-daschland")
            .unwrap_err_or_else(|| panic!("{fixture} loaded as another project should fail validation"));
        let got: Vec<String> = diagnostics.iter().map(|d| format!("{:?}", d.kind)).collect();
        assert_eq!(
            got,
            vec![format!("{:?}", DiagnosticKind::UnknownComponent)],
            "{fixture}: expected exactly UnknownComponent\n  full: {diagnostics:?}"
        );
    }
}

#[test]
fn broken_timeline_date_wrong_type_raw_branch_surfaces_both() {
    // The raw arm must run the timeline data check too: one boot pass reports
    // the config half and the data half together.
    assert_timeline_errors(
        "broken-timeline-date-wrong-type-raw",
        &[
            DiagnosticKind::MissingSiteTitle,
            timeline_incompatible("name", "text", "date"),
        ],
    );
}

// ── Figure-block slot (Incunabula Slice 7) ────────────────────────────────────
//
// The slotted component is the registered `timeline`, so these fixtures load
// as "incunabula" (`assert_timeline_errors`).

#[test]
fn good_figure_validates_clean() {
    load_fixture_project("good-figure", TIMELINE_PROJECT)
        .unwrap_or_else(|kinds| panic!("good-figure should validate clean but got: {kinds:?}"));
}

#[test]
fn broken_figure_no_narrative_fails() {
    assert_timeline_errors(
        "broken-figure-no-narrative",
        &[DiagnosticKind::FigureSectionWithoutNarrative { section: "four".to_owned() }],
    );
}

#[test]
fn broken_figure_more_route_fails() {
    assert_timeline_errors(
        "broken-figure-more-route",
        &[DiagnosticKind::UnresolvedFigureMoreRoute { route: "/nowhere".to_owned(), page: home_page() }],
    );
}

fn unknown_section(section: &str, file: &str) -> DiagnosticKind {
    DiagnosticKind::UnknownFigureSection {
        section: section.to_owned(),
        file: file.to_owned(),
        page: home_page(),
    }
}

fn invite_section(file: &str) -> DiagnosticKind {
    DiagnosticKind::FigureSectionHoldsInviteList {
        section: "explore".to_owned(),
        file: file.to_owned(),
        page: home_page(),
    }
}

fn invalid_heading_id(id: &str) -> DiagnosticKind {
    DiagnosticKind::InvalidHeadingId { id: id.to_owned(), file: "woodcuts.de.md".to_owned() }
}

#[test]
fn broken_figure_unknown_section_fails_per_variant_and_level() {
    // 'four' is a `##` id in DE only; 'deep' sits on a `###` in both.
    let expected = [
        unknown_section("four", "woodcuts.en.md"),
        unknown_section("deep", "woodcuts.de.md"),
        unknown_section("deep", "woodcuts.en.md"),
    ];
    assert_timeline_errors("broken-figure-unknown-section", &expected);
    let mut raw = expected.to_vec();
    raw.push(DiagnosticKind::MissingSiteTitle);
    assert_timeline_errors("broken-figure-unknown-section-raw", &raw);
}

#[test]
fn broken_figure_invite_list_fails() {
    let expected = [invite_section("woodcuts.de.md"), invite_section("woodcuts.en.md")];
    assert_timeline_errors("broken-figure-invite-list", &expected);
    let mut raw = expected.to_vec();
    raw.push(DiagnosticKind::MissingSiteTitle);
    assert_timeline_errors("broken-figure-invite-list-raw", &raw);
}

#[test]
fn broken_figure_heading_id_rejects_malformed_and_repeated_ids() {
    let expected = [invalid_heading_id("Bad_Id"), invalid_heading_id("four")];
    assert_timeline_errors("broken-figure-heading-id", &expected);
    let mut raw = expected.to_vec();
    raw.push(DiagnosticKind::MissingSiteTitle);
    assert_timeline_errors("broken-figure-heading-id-raw", &raw);
}

#[test]
fn broken_figure_reserved_id_rejects_engine_ids() {
    assert_timeline_errors(
        "broken-figure-reserved-id",
        &[invalid_heading_id("main"), invalid_heading_id("annotation-x")],
    );
}

#[test]
fn broken_figure_label_fails() {
    assert_timeline_errors(
        "broken-figure-label",
        &[DiagnosticKind::UnknownFigureLabel { label: "no-such-string".to_owned(), page: home_page() }],
    );
}

// ── woodcut-cases (Incunabula Slice 7) ───────────────────────────────────────
//
// Every fixture is good-showcase with one change, loaded as "incunabula"; a
// `-raw` sibling also drops the site title, so the raw arm must report the
// data defect beside `MissingSiteTitle`.

fn s(v: &str) -> String {
    v.to_owned()
}

/// Assert `fixture` fails with exactly `expected`, and its `-raw` sibling
/// with `expected` plus `MissingSiteTitle`.
fn assert_showcase_errors_both_arms(fixture: &str, expected: &[DiagnosticKind]) {
    assert_timeline_errors(fixture, expected);
    let mut raw = expected.to_vec();
    raw.push(DiagnosticKind::MissingSiteTitle);
    assert_timeline_errors(&format!("{fixture}-raw"), &raw);
}

#[test]
fn good_showcase_validates_clean() {
    let project = load_fixture_project("good-showcase", TIMELINE_PROJECT)
        .unwrap_or_else(|kinds| panic!("good-showcase should validate clean but got: {kinds:?}"));
    assert!(
        project.warnings().is_empty(),
        "good-showcase raises no warning: {:?}",
        project.warnings()
    );
}

#[test]
fn woodcut_cases_declared_by_another_project_is_exactly_unknown_component() {
    let diagnostics = load_fixture_diagnostics("broken-showcase-schramm", "hdm")
        .unwrap_err_or_else(|| panic!("woodcut-cases is not registered for hdm"));
    let got: Vec<String> = diagnostics.iter().map(|d| format!("{:?}", d.kind)).collect();
    assert_eq!(got, vec![format!("{:?}", DiagnosticKind::UnknownComponent)]);
}

#[test]
fn broken_showcase_annotation_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-annotation",
        &[DiagnosticKind::UnknownShowcaseAnnotation { annotation: s("an-zz"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_link_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-link",
        &[DiagnosticKind::UnknownShowcaseLink { link: s("L-zz"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_link_mismatch_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-link-mismatch",
        &[
            DiagnosticKind::ShowcaseLinkMismatch { link: s("L-page"), case: s("an-c8r"), page: home_page() },
            DiagnosticKind::ShowcaseLinkMismatch { link: s("L-page"), case: s("L-page"), page: home_page() },
        ],
    );
}

#[test]
fn broken_showcase_page_fails_once_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-page",
        &[DiagnosticKind::UnknownShowcasePage { id: s("pg-zz"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_excerpt_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-excerpt",
        &[
            DiagnosticKind::ShowcaseExcerptMismatch {
                text: s("Holzschnitt zu Kap. 14."),
                case: s("an-c1v"),
                page: home_page(),
            },
            DiagnosticKind::ShowcaseExcerptMismatch {
                text: s("Anbetung des Kindes, 75 x 61 mm."),
                case: s("L-page"),
                page: home_page(),
            },
        ],
    );
}

#[test]
fn broken_showcase_detail_alt_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-detail-alt",
        &[DiagnosticKind::ShowcaseExcerptMismatch {
            text: s("Handschriftliche Notiz"),
            case: s("an-c1v"),
            page: home_page(),
        }],
    );
}

#[test]
fn broken_showcase_schramm_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-schramm",
        &[DiagnosticKind::ShowcaseSchrammMismatch { schramm: 112, case: s("an-c1v"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_dimensions_fails_in_both_arms() {
    let missing = DiagnosticKind::MissingCropDimensions { annotation: s("an-c8r"), page: home_page() };
    assert_showcase_errors_both_arms("broken-showcase-dimensions", &[missing.clone(), missing]);
}

#[test]
fn broken_showcase_crop_uri_fails_in_both_arms() {
    let not_iiif = |annotation: &str| DiagnosticKind::CropImageNotIiif { annotation: s(annotation), page: home_page() };
    assert_showcase_errors_both_arms(
        "broken-showcase-crop-uri",
        &[
            not_iiif("an-c1v"),
            not_iiif("an-c1v"),
            not_iiif("an-c8r"),
            not_iiif("an-c8r"),
            not_iiif("an-note"),
        ],
    );
}

#[test]
fn broken_showcase_label_fails() {
    assert_timeline_errors(
        "broken-showcase-label",
        &[DiagnosticKind::UnknownShowcaseLabel { label: s("remark-zz"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_case_reports_every_malformed_case() {
    let invalid = |case: Option<&str>| DiagnosticKind::InvalidShowcaseCase { case: case.map(str::to_owned) };
    assert_timeline_errors(
        "broken-showcase-case",
        &[
            invalid(Some("an-c1v")),
            invalid(Some("an-c8r")),
            invalid(Some("L-page")),
            invalid(Some("an-c1v")),
            invalid(None),
            invalid(Some("an-c1v")),
            // A key repeated inside one block: once, from lowering.
            invalid(Some("an-c8r")),
            DiagnosticKind::MissingRequiredAttribute,
            DiagnosticKind::MissingRequiredAttribute,
        ],
    );
}

// ── edition-cards and strip-row (Incunabula Slice 7) ─────────────────────────
//
// Every fixture is good-showcase-cards with one change, loaded as
// "incunabula"; a `-raw` sibling also drops the site title.

#[test]
fn good_showcase_cards_validates_clean() {
    let project = load_fixture_project("good-showcase-cards", TIMELINE_PROJECT)
        .unwrap_or_else(|kinds| panic!("good-showcase-cards should validate clean but got: {kinds:?}"));
    assert!(
        project.warnings().is_empty(),
        "good-showcase-cards raises no warning: {:?}",
        project.warnings()
    );
}

#[test]
fn broken_showcase_book_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-book",
        &[DiagnosticKind::UnknownShowcaseBook { book: s("bk-zz"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_class_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-class",
        &[DiagnosticKind::UnknownShowcaseClass { class: s("Strip"), page: home_page() }],
    );
}

#[test]
fn broken_showcase_property_fails_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-property",
        &[DiagnosticKind::IncompatibleShowcaseProperty {
            property: s("name"),
            role: s("uses-property"),
            class: s("BorderStrip"),
            page: home_page(),
        }],
    );
}

#[test]
fn broken_showcase_media_fails_on_the_book_class_in_both_arms() {
    assert_showcase_errors_both_arms(
        "broken-showcase-media",
        &[
            DiagnosticKind::IncompatibleMediaProperty {
                property: s("name"),
                class: s("Book"),
                value_type: s("text"),
                page: home_page(),
            },
            DiagnosticKind::IncompatibleGlossPart {
                property: s("pub-place"),
                role: s("imprint unit"),
                value_type: s("text"),
                class: s("Book"),
                page: home_page(),
            },
        ],
    );
}

#[test]
fn broken_showcase_unit_fails_on_its_chrome_ids() {
    assert_timeline_errors(
        "broken-showcase-unit",
        &[
            DiagnosticKind::GlossUnitMissingPlaceholder { unit: s("gloss-extent"), lang: s("de") },
            DiagnosticKind::UnknownShowcaseLabel { label: s("edition-zz"), page: home_page() },
        ],
    );
}
