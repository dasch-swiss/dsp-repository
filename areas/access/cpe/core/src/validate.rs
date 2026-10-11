//! Config-internal validation and lowering: raw IR → typed IR.
//!
//! The entry point for callers outside this module is [`load_config`]: it
//! parses the KDL into a `RawConfig`, then runs `validate_with_data` with a
//! [`ValidationContext`] from the caller. Validation accumulates all
//! diagnostics in a [`ValidationReport`], **never short-circuiting**.
//! A partial typed IR is built alongside; it is discarded on error and
//! returned on success.
//!
//! Covers all config-internal checks (top-level/site/project-id plus
//! component lowering and page-body validation). Data-dependent checks
//! (class/property existence, facet-kind↔value-type, …) run in
//! `validate_with_data`, in both of its arms, and read the store only
//! through `facts::ProjectFacts`. Topic-sized parts live in the children:
//! `context`, `attrs`, `parts`, `media_property`, `header_budget`, `timeline`,
//! `showcase` and `project_components`.

use crate::config::{ConfigError, RawConfig, RawLabelTarget};
use crate::diagnostic::{Diagnostic, DiagnosticKind, LabelTarget, Severity, Span, ValidationReport};
use crate::ir::{
    AnnotationShape, ChipKind, ChipSpec, ChromeString, ClassLabel, Component, CompoundLayout, Facet, FacetKind,
    FigureMore, FigureSlot, Gloss, GlossPart, HeadSlots, IncomingGroupBy, IncomingGrouping, IncomingLayout,
    IncomingLinksSpec, IncomingOrder, MemberIndex, MemberLinks, NarrativeCompanion, NarrativeFor, NarrativeSide,
    NavItem, NotFoundBrowse, OrderOption, Page, Pagination, PickerMode, PropertyGroup, PropertyLabel, ResourceListId,
    SiteConfig, Sort, SortDir, SortKey, TextAlign, TileRatio, ValidatedConfig, Value, WhereClause,
    DEFAULT_INCOMING_LINKS_LIMIT, GROUPED_INCOMING_LIMIT_CEILING,
};
use crate::theme::{Accent, Background, ThemeKnob, Typeface};
use crate::util::split_stem_ext;

mod attrs;
pub mod context;
pub mod facts;
mod header_budget;
mod media_property;
mod parts;
pub mod project_components;
mod remodel;
mod showcase;
mod timeline;
use attrs::{
    first_integer_arg, first_positional_string, get_attr, get_bool_attr, get_string_attr, get_typed_integer_attr,
    get_typed_string_attr, optional_string_attr, positional_string_args, require_first_positional_string,
    require_string_attr, value_type_name,
};
use context::ValidationContext;
use facts::{push_internal_query_error, ProjectFacts};
use media_property::check_single_media_property;
use parts::{check_role_value_types, lower_gloss_part, lower_imprint, part_roles};

// ── Public surface ────────────────────────────────────────────────────────────

/// Validate `raw` against `ctx`, lower valid parts to a [`ValidatedConfig`].
///
/// Every check **appends** to `report`; no early return on first failure.
/// Returns `(Some(ValidatedConfig), report)` iff `report.has_errors() ==
/// false` at the end; `(None, report)` otherwise (any partially-built typed
/// IR is discarded). The report is returned in **both** cases — including
/// success — because config-internal lowering can raise Warning-severity
/// diagnostics (e.g. `MissingClassPlural`, raised beside `check_nav_routes`)
/// that `validate_with_data` must surface via
/// `Project::warnings()` even when there are no errors.
fn validate_inner(raw: &RawConfig, ctx: &ValidationContext<'_>) -> (Option<ValidatedConfig>, ValidationReport) {
    let mut report = ValidationReport::new();

    // ── Project-id checks (span-less — id comes from directory name) ──────────
    check_project_id(ctx.project_id, ctx.reserved_ids, &mut report);

    // ── Top-level node checks ─────────────────────────────────────────────────
    check_unexpected_top_level(raw, &mut report);

    // ── Site checks ───────────────────────────────────────────────────────────
    let site_result = check_and_lower_site(raw, ctx, &mut report);

    // Default language for the narrative existence probe: a narrative `file`
    // must resolve to the base file OR the default-language variant — the two
    // rungs the render-time fallback chain guarantees will render. `None` only
    // when site lowering itself failed (already an error state); in that case
    // keep the more lenient "any variant" probe rather than inventing extra
    // noise on top of an already-broken site block.
    let default_language: Option<String> = site_result.as_ref().map(|s| s.default_language.clone());

    // ── Page checks ───────────────────────────────────────────────────────────
    let pages = check_and_lower_pages(raw, ctx, default_language.as_deref(), &mut report);

    // ── Nav-route existence checks ────────────────────────────────────────────
    // Only run when site lowering succeeded — the nav items live on SiteConfig.
    // `pages` is always available (partially lowered even when body errors fired).
    if let Some(ref site) = site_result {
        let declared_routes: Vec<&str> = pages.iter().map(|p| p.route.as_str()).collect();
        check_nav_routes(&site.nav, &declared_routes, &mut report);
        check_not_found_browse_routes(&site.not_found_browse, &declared_routes, &mut report);
        check_figure_more_routes(&pages, &declared_routes, &mut report);

        // ── Desk header row budget ────────────────────────────────────────────
        // Same placement rationale as `check_class_plurals` below: config-
        // internal, needs the lowered site title, nav and chrome.
        header_budget::check_header_row_budget(site, &mut report);

        // ── Class-plural completeness ─────────────────────────────────────────
        // Config-internal (no data needed), so it runs here — before the
        // `validate_with_data` fork — and therefore reaches both arms without
        // a `*_into_raw` sibling. It does sit inside the `site_result` guard
        // (class labels live on the lowered `SiteConfig`), so a config whose
        // `site {}` block itself failed to lower reports no `MissingClassPlural`
        // — acceptable for an advisory on a boot that hard-fails anyway.
        check_class_plurals(&site.class_labels, &mut report);

        // ── Property-group + incoming-links captions ─────────────────
        // Same placement rationale as `check_class_plurals`: config-internal,
        // needs the lowered `site.chrome`, runs before the fork.
        check_group_labels(&pages, &site.chrome, &mut report);

        // ── Ordering-option labels ───────────────────────────────────────
        // Same placement rationale: config-internal, needs `site.chrome`.
        check_order_labels(&pages, &site.chrome, &mut report);

        // ── Composed-gloss units ──────────────────────────────────────────────
        // Same placement rationale: config-internal, needs `site.chrome`.
        check_gloss_units(&pages, &site.chrome, &mut report);

        // ── Figure-block slot labels ──────────────────────────────────────────
        // Same placement rationale: config-internal, needs `site.chrome`.
        check_figure_labels(&pages, &site.chrome, &mut report);

        // ── Showcase remark labels ────────────────────────────────────────────
        // Same placement rationale: config-internal, needs `site.chrome`.
        showcase::check_showcase_labels(&pages, &site.chrome, &mut report);
    }

    // ── Class→route map validation ────────────────────────────────────────────
    // Detect ambiguous class bindings (two pages, same class, both parameterised).
    // The map itself is built during Project::load from the validated config via
    // `build_class_routes`, but we run the duplicate-detection pass here so it
    // participates in the full accumulated-error report.
    check_class_routes(&pages, &mut report);

    // ── Grid-mode without media ────────────────────────────────────────────────
    check_grid_without_media(&pages, &mut report);
    check_tile_ratio_without_grid(&pages, &mut report);

    // ── Narrative layout attributes on a later narrative ──────────────────────
    check_narrative_layout_first(&pages, &mut report);

    // ── Remodel block ─────────────────────────────────────────────────────────
    // Config-internal, so it runs before the `validate_with_data` fork and
    // reaches both arms. The languages come from the raw config, not the
    // lowered site, so a failed site still gets its list translations checked.
    if let Some(block) = &raw.remodel {
        // The web binary does not use the mapping: it is validated here and
        // dropped, and `load_remodel` is the entry that returns it. An error
        // still fails boot.
        let _ = remodel::lower(block, &declared_languages(raw), &mut report);
    }

    // ── Final result ──────────────────────────────────────────────────────────
    if report.has_errors() {
        return (None, report);
    }

    let site = site_result.expect("site check returned None but report has no errors — invariant violated");

    (Some(ValidatedConfig { site, pages }), report)
}

/// The site's declared languages, read silently from the raw config: a
/// wrong-typed entry is left out here, and site lowering reports it. Each
/// language stands once, at its first place: the site takes a language that is
/// written twice, and a mapping that held it twice would write a per-language
/// row twice.
fn declared_languages(raw: &RawConfig) -> Vec<&str> {
    let mut languages: Vec<&str> = Vec::new();
    for (value, _) in &raw.languages {
        if let Value::String(language) = value {
            if !languages.contains(&language.as_str()) {
                languages.push(language.as_str());
            }
        }
    }
    languages
}

// ── Data-dependent validation ─────────────────────────────────────────────────

/// Why loading a project's config failed: it did not parse, or it did not validate.
#[derive(Debug)]
pub enum ConfigLoadError {
    /// The KDL source did not parse.
    Parse(ConfigError),
    /// The config parsed, but the report holds at least one error.
    Validation(ValidationReport),
}

/// Parse `config_src` and validate it against the project's data. The raw
/// config never leaves this crate.
///
/// `ctx`, `facts` and `narratives` are passed to `validate_with_data`
/// unchanged.
pub fn load_config(
    config_src: &str,
    ctx: &ValidationContext<'_>,
    facts: &dyn ProjectFacts,
    narratives: &[(String, String)],
) -> Result<(ValidatedConfig, Vec<Diagnostic>), ConfigLoadError> {
    let raw = crate::config::parse(config_src).map_err(ConfigLoadError::Parse)?;
    validate_with_data(&raw, ctx, facts, narratives).map_err(ConfigLoadError::Validation)
}

/// Parse `config_src` and lower its `remodel` block: the store builder's entry.
///
/// Needs no store and no [`ValidationContext`], and checks nothing of the
/// config but what decides which block is the mapping: a second `remodel`
/// block is `UnexpectedTopLevelNode`, as it is at boot, so the builder never
/// builds from one block of a file that boot refuses for the other. A
/// config without a `remodel` block is `MissingRemodelBlock` and one that
/// declares no site language is `MissingSiteLanguages` (the lists' labels are
/// checked against the languages, and the builder holds a curated text per
/// language and a date's display text to them), both spanless; any error
/// comes back as `ConfigLoadError::Validation`.
pub fn load_remodel(config_src: &str) -> Result<crate::remodel::Remodel, ConfigLoadError> {
    let raw = crate::config::parse(config_src).map_err(ConfigLoadError::Parse)?;
    let languages = declared_languages(&raw);
    let mut report = ValidationReport::new();
    for (name, span) in &raw.unknown_top_level {
        if name == "remodel" {
            report.push(unexpected_top_level(name, *span));
        }
    }

    let remodel = match &raw.remodel {
        None => {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::MissingRemodelBlock,
                "config.kdl has no 'remodel' block",
            ));
            None
        }
        Some(block) => {
            if languages.is_empty() {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::MissingSiteLanguages,
                    "the site declares no language; the 'remodel' block's list labels are checked against them",
                ));
            }
            remodel::lower(block, &languages, &mut report)
        }
    };

    match remodel {
        Some(remodel) if !report.has_errors() => Ok(remodel),
        _ => Err(ConfigLoadError::Validation(report)),
    }
}

/// Run the full config-internal validation **plus** data-dependent checks.
///
/// `load_config` calls this as its second step, after the parse, once
/// `data.sql` has been loaded into the in-memory SQLite. It calls
/// `validate_inner(raw, ctx)` first to accumulate all config checks, then
/// appends data-dependent diagnostics for `resource-detail` components with
/// literal ids.
///
/// **Both arms run the same data-dependent check families, in the same
/// order.** When lowering succeeded the checks read the typed
/// `ValidatedConfig`; when it failed each has a `*_into_raw` sibling reading
/// the `RawConfig` instead, so a dual failure (a config-internal error **and**
/// a data defect) surfaces both in one boot pass. The deliberate exclusions
/// are the config-internal checks that run *before* this fork, inside
/// `validate_inner` (`check_nav_routes`, `check_class_routes`,
/// `check_grid_without_media`) — their diagnostics already land in the
/// `report` both arms consume, so a raw sibling would double-report — and
/// `build_media_properties`, which is a post-validation map-builder, not a
/// check.
///
/// Callers that lack a live database (unit tests, config-only linting) should
/// call `validate_inner` directly; the data-dependent checks are a no-op there.
///
/// `narratives` are the project's `(file name, content)` pairs, the ones
/// `Project::load` stores. The heading-id and figure-section checks read them
/// in both arms, like a data check; `&[]` makes those checks no-ops.
///
/// Never short-circuits — all checks run and all diagnostics are accumulated.
///
/// On success, returns `(ValidatedConfig, Vec<Diagnostic>)` where the
/// `Vec<Diagnostic>` contains all warning-severity diagnostics from both the
/// config-internal pass (e.g. `MissingClassPlural`) and the
/// data-dependent checks (empty when none). Hard-fail (error-severity) data
/// diagnostics still go in the `Err(report)` path.
pub(crate) fn validate_with_data(
    raw: &RawConfig,
    ctx: &ValidationContext<'_>,
    data: &dyn ProjectFacts,
    narratives: &[(String, String)],
) -> Result<(ValidatedConfig, Vec<Diagnostic>), ValidationReport> {
    // Run config-internal checks first. On failure we still want to attempt
    // the data checks (best-effort), but the typed IR is only available on
    // success.
    //
    // Strategy: call validate_inner() which returns (Some(config), report) or
    // (None, report). Unlike the public `validate` wrapper, this keeps the
    // report even on success, because config-internal lowering can also raise
    // Warning-severity diagnostics (`MissingClassPlural`) that
    // must reach `Project::warnings()`.
    match validate_inner(raw, ctx) {
        (Some(config), config_report) => {
            // Config-internal checks passed (no errors — config_report may
            // still carry Warning-severity diagnostics). Run data-dependent
            // checks next.
            let mut extra = ValidationReport::new();
            // Property-row consistency check: reads data, not config.
            // Runs in both branches so violations are reported regardless of
            // whether config checks also failed.
            check_malformed_property_rows(data, &mut extra);
            check_malformed_annotations(data, &mut extra);
            check_malformed_data_arks(data, &mut extra);
            check_resource_detail_data(&config, data, &mut extra);
            check_narrative_for_data(&config, data, &mut extra);
            check_incoming_links_data(&config, data, &mut extra);
            check_resource_list_data(&config, data, &mut extra);
            // `media="…"`: registration order is detail media ->
            // compound-viewer media -> resource-list media, each contributing
            // to (or checking against) one shared class->(property, page) map,
            // so the list check sees every earlier registration's
            // ConflictingMediaProperty potential.
            let mut media_map = check_resource_detail_media(&config, data, &mut extra);
            check_compound_viewer_media(&config, data, &mut media_map, &mut extra);
            check_resource_list_media(&config, &media_map, data, &mut extra);
            check_compound_viewer_data(&config, data, &mut extra);
            check_facet_panel_data(&config, data, &mut extra);
            check_search_bar_data(&config, data, &mut extra);
            check_labels_data(&config, data, &mut extra);
            // Per-project components, through the registry.
            check_project_component_data(&config, ctx.project_id, data, &mut extra);
            check_narrative_headings(&config, narratives, &mut extra);
            // FK-existence check: warn on every link whose target_id is absent
            // from resources. Runs after the malformed-row check so it can
            // assume target_id is non-NULL on all link rows (malformed NULL
            // rows would already be caught above).
            check_link_targets(data, &mut extra);
            if extra.has_errors() {
                // Hard data errors: return a failing report carrying all of the
                // data-check diagnostics (errors and warnings alike), plus any
                // config-internal warnings so the operator sees the full
                // picture. config_report has no errors here (checked above by
                // validate_inner returning Some), so this only adds warnings.
                let mut combined = ValidationReport::new();
                for d in config_report.iter() {
                    combined.push(d.clone());
                }
                for d in extra.iter() {
                    combined.push(d.clone());
                }
                Err(combined)
            } else {
                // No hard-fail data errors. Collect warning-severity
                // diagnostics from both the config-internal pass and the data
                // checks, and return them alongside the config so callers
                // (e.g. `Project::load`) can surface them to the operator.
                let warnings: Vec<Diagnostic> = config_report
                    .iter()
                    .chain(extra.iter())
                    .filter(|d| d.severity == Severity::Warning)
                    .cloned()
                    .collect();
                Ok((config, warnings))
            }
        }
        (None, mut report) => {
            // Config errors already present; still attempt data checks so the
            // operator sees the full picture in one pass.
            // Property-row consistency check runs here too (reads data, not config).
            check_malformed_property_rows(data, &mut report);
            check_malformed_annotations(data, &mut report);
            check_malformed_data_arks(data, &mut report);
            check_resource_detail_data_into_raw(raw, data, &mut report);
            check_narrative_for_data_into_raw(raw, data, &mut report);
            check_incoming_links_data_into_raw(raw, data, &mut report);
            check_resource_list_data_into_raw(raw, data, &mut report);
            // Mirrors the success arm's order: detail media -> compound-viewer
            // media -> resource-list media, all sharing one class->(property,
            // page) map. Reordering these silently disables
            // ConflictingMediaProperty for whichever pair got swapped —
            // nothing but the fixture tests catch it.
            let mut media_map = check_resource_detail_media_into_raw(raw, data, &mut report);
            check_compound_viewer_media_into_raw(raw, data, &mut media_map, &mut report);
            check_resource_list_media_into_raw(raw, &media_map, data, &mut report);
            check_compound_viewer_data_into_raw(raw, data, &mut report);
            check_facet_panel_data_into_raw(raw, data, &mut report);
            check_search_bar_data_into_raw(raw, data, &mut report);
            check_labels_data_into_raw(raw, data, &mut report);
            check_project_component_data_into_raw(raw, ctx.project_id, data, &mut report);
            check_narrative_headings_into_raw(raw, ctx.project_id, narratives, &mut report);
            check_link_targets(data, &mut report);
            Err(report)
        }
    }
}

/// A `resource-detail class="…"` existence check, mirroring
/// `UnknownResourceListClass`/`UnknownIncomingLinksClass`/
/// `UnknownTimelineClass`. `Ok(false)` pushes `UnknownResourceDetailClass`
/// (Error); `Err(e)` pushes `InternalQueryError`. Returns whether the class is
/// known — callers use this to skip cascading checks (property/media) that
/// would otherwise run against a class that doesn't exist.
fn check_resource_detail_class(
    class: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> bool {
    match data.class_exists(class) {
        Ok(true) => true,
        Ok(false) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownResourceDetailClass { class: class.to_owned(), page: route.to_owned() },
                message: format!(
                    "page '{route}': resource-detail declares class '{class}' which does not \
                     exist in the data"
                ),
                span: None,
                severity: Severity::Error,
            });
            false
        }
        Err(e) => {
            push_internal_query_error(
                report,
                "resource-detail class existence",
                format!(
                    "page '{route}': could not check existence of resource-detail class \
                     '{class}': {e}"
                ),
            );
            false
        }
    }
}

/// Run data-dependent checks on a successfully-lowered `ValidatedConfig`.
/// Appends diagnostics to `report`; never short-circuits.
fn check_resource_detail_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceDetail { id, class, show_properties, title_property, groups, head, .. } =
                component
            {
                // The class existence check runs first, regardless of the
                // `id` shape (literal / `$param` / absent) — a typo'd class
                // is a config mistake independent of how the resource itself
                // is addressed.
                let class_known = match class {
                    Some(class_val) => check_resource_detail_class(class_val, &page.route, data, report),
                    None => true,
                };

                // Group members and header-slot properties are both names the
                // record puts on the page, so both join the sparse check.
                let mut group_members: Vec<String> = groups.iter().flat_map(|g| g.properties.iter().cloned()).collect();
                if let Some(head) = head {
                    group_members.extend(head.alt_title.iter().cloned());
                    group_members.extend(head.author.iter().cloned());
                    group_members.extend(head.imprint.iter().map(|p| p.property.clone()));
                    if class_known {
                        if let Some(record_class) = resolved_detail_class(id, class, data) {
                            let imprint: Vec<(&str, bool)> =
                                head.imprint.iter().map(|p| (p.property.as_str(), p.unit.is_some())).collect();
                            check_head_slot_types(
                                &record_class,
                                head.alt_title.as_deref(),
                                head.author.as_deref(),
                                &imprint,
                                &page.route,
                                data,
                                report,
                            );
                        }
                    }
                }

                let Some(id_val) = id else {
                    continue;
                };

                // A `$param` reference names no single resource, so the
                // UnknownResource + resource-keyed sparse pair cannot run —
                // but the component's declared `class` is enough to run the
                // sparse half over the class itself.
                if id_val.starts_with('$') {
                    // Skip the properties-on-class check when the class is
                    // unknown (the cascade rule) — it already returns
                    // silently on an empty property set, but skip it
                    // explicitly rather than relying on that.
                    if let Some(class_val) = class {
                        if class_known {
                            check_resource_detail_properties_on_class(
                                class_val,
                                show_properties,
                                &group_members,
                                title_property,
                                data,
                                report,
                            );
                        }
                    }
                    continue;
                }

                check_single_resource_detail(id_val, show_properties, &group_members, title_property, data, report);
            }
        }
    }
}

/// Run data-dependent checks on the raw config when config-internal checks
/// already failed. We parse what we can from raw components.
fn check_resource_detail_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-detail" {
                continue;
            }

            // The class existence check runs before the `id_val … continue`
            // below, mirroring the success arm — it must not depend on `id`
            // being present. A wrong-typed `class` is `None` here (lowering
            // already reported it), so it's silently skipped.
            let class_val = get_string_attr(raw_comp, "class");
            let class_known = match &class_val {
                Some(c) => check_resource_detail_class(c, page_route, data, report),
                None => true,
            };

            // Extract id from raw attrs.
            let id_val = get_string_attr(raw_comp, "id");

            let id_val = match id_val {
                Some(s) => s,
                None => continue,
            };

            let is_param = id_val.starts_with('$');

            // Extract show-properties from raw children.
            let show_properties: Option<Vec<String>> =
                raw_comp.children.iter().find(|c| c.name == "show-properties").map(|c| {
                    c.args
                        .iter()
                        .filter_map(|v| {
                            if let crate::ir::Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                });

            // Extract every `group` child's members — positional string
            // args, label ignored (a missing label is lowering's Error, never
            // re-reported here).
            let mut group_members: Vec<String> = raw_comp
                .children
                .iter()
                .filter(|c| c.name == "group")
                .flat_map(positional_string_args)
                .collect();

            // Extract title-property from raw attrs.
            let title_property: Option<String> = get_string_attr(raw_comp, "title-property");

            // The header slots, read with the silent readers: an absent or
            // wrong-typed attribute is skipped (lowering already reported it),
            // and only the FIRST `imprint` counts, as in lowering.
            let alt_title = get_string_attr(raw_comp, "alt-title-property");
            let author = get_string_attr(raw_comp, "author-property");
            let imprint: Vec<(String, bool)> = raw_comp
                .children
                .iter()
                .find(|c| c.name == "imprint")
                .map(|block| {
                    block
                        .children
                        .iter()
                        .filter(|c| c.name == "part")
                        .filter_map(|c| {
                            let unit = c.attrs.iter().any(|(k, _)| k == "unit");
                            get_string_attr(c, "property").map(|p| (p, unit))
                        })
                        .collect()
                })
                .unwrap_or_default();
            group_members.extend(alt_title.iter().cloned());
            group_members.extend(author.iter().cloned());
            group_members.extend(imprint.iter().map(|(p, _)| p.clone()));
            if class_known && (alt_title.is_some() || author.is_some() || !imprint.is_empty()) {
                let raw_id = get_string_attr(raw_comp, "id");
                if let Some(record_class) = resolved_detail_class(&raw_id, &class_val, data) {
                    let imprint: Vec<(&str, bool)> = imprint.iter().map(|(p, u)| (p.as_str(), *u)).collect();
                    check_head_slot_types(
                        &record_class,
                        alt_title.as_deref(),
                        author.as_deref(),
                        &imprint,
                        page_route,
                        data,
                        report,
                    );
                }
            }

            if is_param {
                // Raw sibling of the success arm's parameterised branch: the
                // class-keyed sparse half only. An absent/wrong-typed `class`
                // silently continues (`get_string_attr` never reports). Skip
                // when the class is unknown (the cascade rule).
                if let Some(ref class_val) = class_val {
                    if class_known {
                        check_resource_detail_properties_on_class(
                            class_val,
                            &show_properties,
                            &group_members,
                            &title_property,
                            data,
                            report,
                        );
                    }
                }
                continue;
            }

            check_single_resource_detail(&id_val, &show_properties, &group_members, &title_property, data, report);
        }
    }
}

/// `UnknownNarrativeForResource` (Warning) for every `narrative-for` entry on a
/// lowered `resource-detail` whose resource id does not exist in the data. Runs
/// on `$param` pages too — the `narrative-for` id is always a literal.
fn check_narrative_for_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceDetail { narrative_for, .. } = component {
                for entry in narrative_for {
                    check_single_narrative_for(&entry.resource_id, &page.route, data, report);
                }
            }
        }
    }
}

/// Raw sibling of `check_narrative_for_data`, for the `validate_with_data`
/// failure arm. Extracts ids with the silent `first_positional_string` and
/// `continue`s on an absent/wrong-typed one — lowering already reported it.
fn check_narrative_for_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        let route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-detail" {
                continue;
            }
            for child in raw_comp.children.iter().filter(|c| c.name == "narrative-for") {
                let Some(resource_id) = first_positional_string(child) else {
                    continue;
                };
                check_single_narrative_for(&resource_id, route, data, report);
            }
        }
    }
}

/// The shared per-entry body of the `narrative-for` resource-existence check.
fn check_single_narrative_for(resource_id: &str, route: &str, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    match data.resource_exists(resource_id) {
        Ok(true) => {}
        Ok(false) => report.push(Diagnostic {
            kind: DiagnosticKind::UnknownNarrativeForResource {
                resource_id: resource_id.to_owned(),
                page: route.to_owned(),
            },
            message: format!(
                "page '{route}': resource-detail `narrative-for` names resource \
                 '{resource_id}', which does not exist in the data; the intro will never render"
            ),
            span: None,
            severity: Severity::Warning,
        }),
        Err(e) => push_internal_query_error(
            report,
            "narrative-for",
            format!(
                "resource-detail `narrative-for`: could not check resource existence \
                 for id '{resource_id}': {e}"
            ),
        ),
    }
}

/// Data-dependent checks for every lowered `resource-detail { incoming-links …
/// }` — see `check_single_incoming_links`. Runs on
/// `$param` pages too: the spec's class and properties are always literals.
fn check_incoming_links_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceDetail { id, class, incoming_links: Some(spec), .. } = component {
                let host_class =
                    resolved_detail_class(id, class, data).filter(|host| matches!(data.class_exists(host), Ok(true)));
                let rows = IncomingRowProperties {
                    group_by: match spec.grouping.as_ref().map(|g| &g.by) {
                        Some(IncomingGroupBy::Property(p)) => Some(p.as_str()),
                        Some(IncomingGroupBy::Via) | None => None,
                    },
                    row_label: spec.row_label.as_deref(),
                    row_aside: spec.row_aside.as_deref(),
                    sort_property: spec.sort_property.as_deref(),
                };
                check_single_incoming_links(
                    &spec.class,
                    &spec.properties,
                    &page.route,
                    host_class.as_deref(),
                    &rows,
                    data,
                    report,
                );
            }
        }
    }
}

/// Raw sibling of `check_incoming_links_data`, for the `validate_with_data`
/// failure arm. Reads only the FIRST `incoming-links` child (a second one is
/// lowering's `DuplicateIncomingLinks`, and the success arm never lowers it),
/// extracting `class` with the diagnostic-free `get_string_attr` and silently
/// `continue`-ing on an absent/wrong-typed one or on zero properties —
/// lowering already reported those.
fn check_incoming_links_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        let route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-detail" {
                continue;
            }
            let Some(child) = raw_comp.children.iter().find(|c| c.name == "incoming-links") else {
                continue;
            };
            let Some(class) = get_string_attr(child, "class") else {
                continue;
            };
            let mut properties = positional_string_args(child);
            let mut seen = std::collections::HashSet::new();
            properties.retain(|p| seen.insert(p.clone()));
            if properties.is_empty() {
                continue;
            }
            let detail_id = get_string_attr(raw_comp, "id");
            let detail_class = get_string_attr(raw_comp, "class");
            let host_class = resolved_detail_class(&detail_id, &detail_class, data)
                .filter(|host| matches!(data.class_exists(host), Ok(true)));
            // The row properties, read with the SILENT reader (a
            // wrong-typed one was already reported by lowering) in the success
            // arm's order; `group-by="via"` names no source property.
            let group_by = get_string_attr(child, "group-by").filter(|g| g != "via");
            let row_label = get_string_attr(child, "row-label");
            let row_aside = get_string_attr(child, "row-aside");
            let sort_property = get_string_attr(child, "sort-property");
            let rows = IncomingRowProperties {
                group_by: group_by.as_deref(),
                row_label: row_label.as_deref(),
                row_aside: row_aside.as_deref(),
                sort_property: sort_property.as_deref(),
            };
            check_single_incoming_links(&class, &properties, route, host_class.as_deref(), &rows, data, report);
        }
    }
}

/// The shared body of the `incoming-links` data checks. The class and the
/// properties are **query inputs** (like a `resource-list`'s `class`/`where`),
/// so absent-everywhere is an Error, not the `group`-member sparse Warning:
///
/// - `class` not in the data → `UnknownIncomingLinksClass` (Error); the per-property checks then
///   run the absent-everywhere tier only (every property would otherwise also be "sparse" on a
///   class with no resources);
/// - a property absent from the data entirely → `UnknownIncomingLinksProperty` (Error);
/// - present in the data but carried by no resource of `class` (the union over the class — "sparse"
///   in the codebase's documented sense) → `SparseIncomingLinksProperty` (Warning);
/// - carried by the class with any stored `value_type` other than `link` →
///   `IncomingLinksPropertyNotLink` (Error) — only `link` rows have a `target_id` to
///   reverse-resolve;
/// - all `link`-typed, and `host_class` is known (the resolved class of the page's own
///   `resource-detail`) — when the property's targets never actually resolve to a resource of
///   `host_class`, the list on this page will always be empty → `IncomingLinksPropertyMissesHost`
///   (Warning). A `host_class` of `None` (a class-less `$param` page) skips this tier entirely, and
///   a target set that is empty (every link dangling) also skips it — that is `UnknownLinkTarget`'s
///   business, not this one's.
///
/// Then the row properties (`rows`) — see `check_incoming_row_property`.
///
/// Every query failure is `InternalQueryError`, never dropped. Never
/// short-circuits across properties.
fn check_single_incoming_links(
    class: &str,
    properties: &[String],
    route: &str,
    host_class: Option<&str>,
    rows: &IncomingRowProperties<'_>,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let class_known = match data.class_exists(class) {
        Ok(b) => b,
        Err(e) => {
            push_internal_query_error(
                report,
                "incoming-links class existence",
                format!(
                    "page '{route}': resource-detail `incoming-links`: could not check class \
                     existence for '{class}': {e}"
                ),
            );
            return;
        }
    };
    if !class_known {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownIncomingLinksClass { class: class.to_owned(), page: route.to_owned() },
            message: format!(
                "page '{route}': resource-detail `incoming-links` references source class \
                 '{class}' which does not exist in the data"
            ),
            span: None,
            severity: Severity::Error,
        });
    }

    for prop in properties {
        let exists_anywhere = match data.property_exists_anywhere(prop) {
            Ok(b) => b,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "incoming-links property existence",
                    format!(
                        "page '{route}': resource-detail `incoming-links`: could not check \
                         property existence for '{prop}': {e}"
                    ),
                );
                continue;
            }
        };
        if !exists_anywhere {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownIncomingLinksProperty {
                    property: prop.clone(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                message: format!(
                    "page '{route}': resource-detail `incoming-links` property '{prop}' does \
                     not exist anywhere in the data (source class '{class}')"
                ),
                span: None,
                severity: Severity::Error,
            });
            continue;
        }
        if !class_known {
            // Already reported the class; skip the class-scoped tiers.
            continue;
        }
        let types = match data.property_value_types_on_class(class, prop) {
            Ok(t) => t,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "incoming-links property value type",
                    format!(
                        "page '{route}': resource-detail `incoming-links`: could not check \
                         the value type of property '{prop}' on class '{class}': {e}"
                    ),
                );
                continue;
            }
        };
        if types.is_empty() {
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseIncomingLinksProperty {
                    property: prop.clone(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                message: format!(
                    "page '{route}': resource-detail `incoming-links` property '{prop}' is \
                     not present on any resource of source class '{class}'; it can never \
                     contribute a row"
                ),
                span: None,
                severity: Severity::Warning,
            });
            continue;
        }
        // The lexicographically smallest offending type, for a stable message
        // (not arbitrary HashSet iteration order).
        if let Some(bad) = types.iter().filter(|t| t.as_str() != "link").min() {
            report.push(Diagnostic {
                kind: DiagnosticKind::IncomingLinksPropertyNotLink {
                    property: prop.clone(),
                    value_type: bad.clone(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                message: format!(
                    "page '{route}': resource-detail `incoming-links` property '{prop}' on \
                     source class '{class}' has value type '{bad}', but incoming-links \
                     requires 'link'"
                ),
                span: None,
                severity: Severity::Error,
            });
            continue;
        }
        // Every stored value is `link`. If this page's own class is known,
        // warn when the property's targets never actually resolve back to
        // the host class — such a list is guaranteed to always be empty.
        if let Some(host) = host_class {
            let target_classes = match data.link_target_classes_on_class(class, prop) {
                Ok(t) => t,
                Err(e) => {
                    push_internal_query_error(
                        report,
                        "incoming-links target classes",
                        format!(
                            "page '{route}': resource-detail `incoming-links`: could not \
                             check the target classes of property '{prop}' on source class \
                             '{class}': {e}"
                        ),
                    );
                    continue;
                }
            };
            if !target_classes.is_empty() && !target_classes.contains(host) {
                report.push(Diagnostic {
                    kind: DiagnosticKind::IncomingLinksPropertyMissesHost {
                        property: prop.clone(),
                        class: class.to_owned(),
                        host_class: host.to_owned(),
                        page: route.to_owned(),
                    },
                    message: format!(
                        "page '{route}': resource-detail `incoming-links` property '{prop}' \
                         on source class '{class}' never points at a '{host}' resource, so \
                         the list will always be empty on this page"
                    ),
                    span: None,
                    severity: Severity::Warning,
                });
            }
        }
    }

    let target = IncomingRowTarget { class, class_known, link_properties: properties, route };
    for (attribute, property, allowed) in rows.checks() {
        check_incoming_row_property(&target, attribute, property, allowed, data, report);
    }
}

/// The row properties of one `incoming-links` child, as fed
/// to `check_single_incoming_links` by BOTH `validate_with_data` arms. Each
/// `Some` names a property of the source resource; `group_by` is only a
/// property `group-by=` (the reserved `via` names none).
struct IncomingRowProperties<'a> {
    group_by: Option<&'a str>,
    row_label: Option<&'a str>,
    row_aside: Option<&'a str>,
    sort_property: Option<&'a str>,
}

/// `group-by`/`row-aside` accept a list value or text; `row-label`/
/// `sort-property` only text (any other type would lower cleanly and put every
/// row in the no-key group / sort it last).
const LIST_OR_TEXT: &[&str] = &["list_value", "text"];
const TEXT_ONLY: &[&str] = &["text"];

impl<'a> IncomingRowProperties<'a> {
    /// `(attribute, property, allowed value types)` for every set row
    /// property, in a fixed order (attribute order in the config docs).
    fn checks(&self) -> Vec<(&'static str, &'a str, &'static [&'static str])> {
        [
            ("group-by", self.group_by, LIST_OR_TEXT),
            ("row-label", self.row_label, TEXT_ONLY),
            ("row-aside", self.row_aside, LIST_OR_TEXT),
            ("sort-property", self.sort_property, TEXT_ONLY),
        ]
        .into_iter()
        .filter_map(|(attr, prop, allowed)| prop.map(|p| (attr, p, allowed)))
        .collect()
    }
}

/// The source side of one `incoming-links` list, shared by its row checks.
struct IncomingRowTarget<'a> {
    class: &'a str,
    class_known: bool,
    link_properties: &'a [String],
    route: &'a str,
}

/// One `incoming-links` row property, tiered like the link
/// properties but scoped to the source class:
///
/// - absent from the data, or (class known) carried by no resource of the source class →
///   `UnknownIncomingLinksRowProperty` (Error);
/// - stored with a type outside `allowed` → `IncomingLinksRowPropertyWrongType` (Error);
/// - some source resource linking through the declared properties lacks it →
///   `SparseIncomingLinksRowProperty` (Warning; those rows fall back).
///
/// An unknown source class runs the absent-everywhere tier only (already
/// reported as `UnknownIncomingLinksClass`).
fn check_incoming_row_property(
    target: &IncomingRowTarget<'_>,
    attribute: &str,
    prop: &str,
    allowed: &[&str],
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let IncomingRowTarget { class, class_known, link_properties, route } = *target;
    let unknown = |report: &mut ValidationReport, detail: &str| {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownIncomingLinksRowProperty {
                attribute: attribute.to_owned(),
                property: prop.to_owned(),
                class: class.to_owned(),
                page: route.to_owned(),
            },
            message: format!(
                "page '{route}': resource-detail `incoming-links` {attribute}='{prop}' {detail} \
                 (source class '{class}')"
            ),
            span: None,
            severity: Severity::Error,
        });
    };
    match data.property_exists_anywhere(prop) {
        Ok(true) => {}
        Ok(false) => {
            unknown(report, "does not exist anywhere in the data");
            return;
        }
        Err(e) => {
            push_internal_query_error(
                report,
                "incoming-links row property existence",
                format!(
                    "page '{route}': resource-detail `incoming-links`: could not check \
                     {attribute} property existence for '{prop}': {e}"
                ),
            );
            return;
        }
    }
    if !class_known {
        return;
    }
    let types = match data.property_value_types_on_class(class, prop) {
        Ok(t) => t,
        Err(e) => {
            push_internal_query_error(
                report,
                "incoming-links row property value type",
                format!(
                    "page '{route}': resource-detail `incoming-links`: could not check the \
                     value type of {attribute} property '{prop}' on class '{class}': {e}"
                ),
            );
            return;
        }
    };
    if types.is_empty() {
        unknown(report, "is not present on any resource of the source class");
        return;
    }
    if let Some(bad) = types.iter().filter(|t| !allowed.contains(&t.as_str())).min() {
        report.push(Diagnostic {
            kind: DiagnosticKind::IncomingLinksRowPropertyWrongType {
                attribute: attribute.to_owned(),
                property: prop.to_owned(),
                value_type: bad.clone(),
                class: class.to_owned(),
                page: route.to_owned(),
            },
            message: format!(
                "page '{route}': resource-detail `incoming-links` {attribute}='{prop}' on \
                 source class '{class}' has value type '{bad}', but {attribute} requires {}",
                allowed.join(" or ")
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }
    match data.linking_resource_lacks_property(class, link_properties, prop) {
        Ok(false) => {}
        Ok(true) => report.push(Diagnostic {
            kind: DiagnosticKind::SparseIncomingLinksRowProperty {
                attribute: attribute.to_owned(),
                property: prop.to_owned(),
                class: class.to_owned(),
                page: route.to_owned(),
            },
            message: format!(
                "page '{route}': resource-detail `incoming-links` {attribute}='{prop}' is \
                 missing on some '{class}' resources that link here; those rows fall back"
            ),
            span: None,
            severity: Severity::Warning,
        }),
        Err(e) => push_internal_query_error(
            report,
            "incoming-links row property coverage",
            format!(
                "page '{route}': resource-detail `incoming-links`: could not check the \
                 coverage of {attribute} property '{prop}' on class '{class}': {e}"
            ),
        ),
    }
}

/// Run data-dependent checks on all `resource-list` components in a
/// successfully-lowered `ValidatedConfig`. Appends diagnostics to `report`;
/// never short-circuits.
fn check_resource_list_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceList {
                class,
                wheres,
                sorts,
                gloss,
                chips,
                order_options,
                order_name_property,
                ..
            } = component
            {
                // Skip $param references — the class is resolved at request time,
                // not known statically at boot. Mirrors resource-detail's skip.
                if class.starts_with('$') {
                    continue;
                }
                // Gloss, chip, `order-by` and `order-name-property`
                // properties are validated for existence alongside where/sort
                // props (same two-tier hard-fail/sparse-warn treatment).
                let gloss_props: Vec<&str> = match gloss {
                    None => Vec::new(),
                    Some(Gloss::Property(p)) => vec![p.as_str()],
                    Some(Gloss::Composed { parts, teaser }) => {
                        parts.iter().map(|p| p.property.as_str()).chain(teaser.as_deref()).collect()
                    }
                };
                let display_props: Vec<&str> = gloss_props
                    .into_iter()
                    .chain(chips.iter().map(|c| c.property.as_str()))
                    .chain(order_options.iter().map(|o| o.property.as_str()))
                    .chain(order_name_property.as_deref())
                    .collect();
                check_single_resource_list(class, wheres, sorts, &display_props, &page.route, data, report);
                if let Some(Gloss::Composed { parts, teaser }) = gloss {
                    let parts: Vec<(&str, bool)> =
                        parts.iter().map(|p| (p.property.as_str(), p.unit.is_some())).collect();
                    check_gloss_part_types(class, &parts, teaser.as_deref(), &page.route, data, report);
                }
            }
        }
    }
}

/// The value types a teaser, an alt title or an author renders.
const TEXT_VALUE_TYPES: &[&str] = &["text", "richtext"];

/// `IncompatibleGlossPart` for one composed gloss, shared by both
/// `validate_with_data` arms. `parts` are `(property, has unit=)` in config
/// order. A property absent from the class is skipped here: its existence is
/// `check_single_resource_list`'s business.
fn check_gloss_part_types(
    class: &str,
    parts: &[(&str, bool)],
    teaser: Option<&str>,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let mut roles = part_roles(parts, "");
    if let Some(property) = teaser {
        roles.push((property, "teaser".to_owned(), TEXT_VALUE_TYPES));
    }
    check_role_value_types(class, &roles, "resource-list gloss", route, data, report);
}

/// `IncompatibleGlossPart` for one `resource-detail`'s header slots, shared
/// by both `validate_with_data` arms: the alt title and the author must be
/// text/richtext; an imprint part obeys the composed-gloss part rules.
/// `class` is the record's resolved class.
fn check_head_slot_types(
    class: &str,
    alt_title: Option<&str>,
    author: Option<&str>,
    imprint: &[(&str, bool)],
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let mut roles: Vec<(&str, String, &[&str])> = Vec::new();
    if let Some(property) = alt_title {
        roles.push((property, "alt-title".to_owned(), TEXT_VALUE_TYPES));
    }
    if let Some(property) = author {
        roles.push((property, "author".to_owned(), TEXT_VALUE_TYPES));
    }
    roles.extend(part_roles(imprint, "imprint "));
    check_role_value_types(class, &roles, "resource-detail", route, data, report);
}

/// Run data-dependent checks on `resource-list` components from the raw config
/// when config-internal checks already failed. We parse what we can from raw.
fn check_resource_list_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-list" {
                continue;
            }

            // Extract class from raw attrs (required field).
            let class = match get_string_attr(raw_comp, "class") {
                Some(c) => c,
                None => continue, // class is required; if absent, skip
            };

            // Skip $param references — resolved at request time, not statically known.
            if class.starts_with('$') {
                continue;
            }

            // Extract where clauses and sort properties from raw children.
            let mut wheres: Vec<crate::ir::WhereClause> = Vec::new();
            let mut sorts: Vec<crate::ir::Sort> = Vec::new();
            // Gloss, chip, `order-by` and `order-name-property` property names
            // (validated for existence like where/sort). Read straight from
            // `attrs` for the same borrow reason as the child arm below.
            let mut display_props: Vec<&str> = Vec::new();
            if let Some((_, crate::ir::Value::String(prop))) =
                raw_comp.attrs.iter().find(|(k, _)| k == "order-name-property")
            {
                display_props.push(prop.as_str());
            }
            // A composed gloss's `(part property, has unit=)` and teaser.
            let mut gloss_parts: Vec<(&str, bool)> = Vec::new();
            let mut gloss_teaser: Option<&str> = None;

            for child in &raw_comp.children {
                if child.name == "gloss" {
                    for grandchild in &child.children {
                        let Some((_, crate::ir::Value::String(prop))) =
                            grandchild.attrs.iter().find(|(k, _)| k == "property")
                        else {
                            continue;
                        };
                        match grandchild.name.as_str() {
                            "part" => {
                                gloss_parts.push((prop.as_str(), grandchild.attrs.iter().any(|(k, _)| k == "unit")))
                            }
                            "teaser" if gloss_teaser.is_none() => {
                                gloss_teaser = Some(prop.as_str());
                            }
                            _ => continue,
                        }
                        if !display_props.contains(&prop.as_str()) {
                            display_props.push(prop.as_str());
                        }
                    }
                }
                match child.name.as_str() {
                    "gloss" | "chip" | "order-by" => {
                        // Deliberately NOT `get_string_attr` (unlike every
                        // other raw attr read in this fn): `display_props` is
                        // a `Vec<&str>` borrowing from `child`, and
                        // `get_string_attr` hands back an owned `String` that
                        // would be dropped at the end of this arm.
                        if let Some((_, crate::ir::Value::String(prop))) =
                            child.attrs.iter().find(|(k, _)| k == "property")
                        {
                            if !display_props.contains(&prop.as_str()) {
                                display_props.push(prop.as_str());
                            }
                        }
                    }
                    "where" => {
                        let prop = get_string_attr(child, "property");
                        if let Some(property) = prop {
                            // We only need the property name for data checks;
                            // synthesise an Equals clause as the simplest valid form.
                            wheres.push(crate::ir::WhereClause::Equals {
                                property,
                                value: crate::ir::Value::String(String::new()),
                            });
                        }
                    }
                    "sort" => {
                        let prop = get_string_attr(child, "property");
                        if let Some(property) = prop {
                            // Config `sort` is always a property (never Class/Id —
                            // those only arrive at request time via `?sort=`).
                            sorts.push(crate::ir::Sort {
                                key: crate::ir::SortKey::Property(property),
                                direction: crate::ir::SortDir::Asc,
                            });
                        }
                    }
                    _ => {}
                }
            }

            let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");
            check_single_resource_list(&class, &wheres, &sorts, &display_props, page_route, data, report);
            if !gloss_parts.is_empty() {
                check_gloss_part_types(&class, &gloss_parts, gloss_teaser, page_route, data, report);
            }
        }
    }
}

/// Run data-dependent checks on all `compound-viewer` components in a
/// successfully-lowered `ValidatedConfig`. Appends diagnostics to `report`;
/// never short-circuits.
fn check_compound_viewer_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::CompoundViewer { source, member_links, .. } = component {
                // Before the `$param` skip: the member classes come from the
                // data either way.
                if let Some(links) = member_links {
                    check_single_member_links(source, &links.properties, &page.route, data, report);
                }
                // Skip $param references — the source is resolved at request time,
                // not known statically at boot. Same limitation as the link FK check
                // on parameterised pages.
                if source.starts_with('$') {
                    continue;
                }
                check_single_compound_viewer(source, data, report);
            }
        }
    }
}

/// Run data-dependent checks on `compound-viewer` components from the raw
/// config when config-internal checks already failed. We parse what we can
/// from raw components.
fn check_compound_viewer_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        for raw_comp in &raw_page.body {
            if raw_comp.name != "compound-viewer" {
                continue;
            }

            // Extract source from raw attrs (required field).
            let source = match get_string_attr(raw_comp, "source") {
                Some(s) => s,
                None => continue, // source is required; if absent, skip (lowering already caught it)
            };

            if let Some(links) = raw_comp.children.iter().find(|c| c.name == "member-links") {
                let route = raw_page.route.as_deref().unwrap_or("<unknown page>");
                let properties = positional_string_args(links);
                check_single_member_links(&source, &properties, route, data, report);
            }

            // Skip $param references — resolved at request time, not statically known.
            if source.starts_with('$') {
                continue;
            }

            check_single_compound_viewer(&source, data, report);
        }
    }
}

/// The `compound-viewer` `member-links` data checks,
/// shared by both arms. The member classes come from the data (a literal
/// `source`'s own parts, or every compound member for a `$param`), as for
/// `check_single_compound_viewer_media`. A property on no member class is the
/// Warning `SparseProperty`; a property that is not `link`-typed is
/// `IncompatibleGlossPart` with the role `member link` (the row renders only
/// link values). No member classes yet ⇒ nothing to check.
fn check_single_member_links(
    source: &str,
    properties: &[String],
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let parent = (!source.starts_with('$')).then_some(source);
    let classes = match data.compound_member_classes(parent) {
        Ok(c) => c,
        Err(e) => {
            push_internal_query_error(
                report,
                "compound member classes",
                format!(
                    "page '{page_route}': could not resolve the member classes of \
                     compound-viewer source '{source}': {e}"
                ),
            );
            return;
        }
    };
    if classes.is_empty() {
        return;
    }
    for property in properties {
        let mut present = false;
        for class in &classes {
            match data.property_value_types_on_class(class, property) {
                Ok(types) => present |= !types.is_empty(),
                // One failed query stands for them all: report it once and
                // stop, rather than once per (property, class) pair.
                Err(e) => {
                    push_internal_query_error(
                        report,
                        "member-links property value type",
                        format!(
                            "page '{page_route}': compound-viewer member-links: could not check \
                             property '{property}' on class '{class}': {e}"
                        ),
                    );
                    return;
                }
            }
        }
        if !present {
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseProperty,
                message: format!(
                    "page '{page_route}': compound-viewer member-links property '{property}' \
                     is not present on any member of source '{source}'"
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }
    let roles: Vec<(&str, String, &[&str])> = properties
        .iter()
        .map(|p| (p.as_str(), "member link".to_owned(), &["link"][..]))
        .collect();
    for class in &classes {
        check_role_value_types(class, &roles, "compound-viewer member-links", page_route, data, report);
    }
}

/// Perform the EmptyCompoundSource check for one `compound-viewer` component.
/// Appends to `report`; never short-circuits.
fn check_single_compound_viewer(source: &str, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    let has_parts = match data.compound_has_parts(source) {
        Ok(b) => b,
        Err(e) => {
            // Unexpected query error — the query itself failed to execute, not
            // "the compound source was checked and found to have no parts"
            // (that genuine-empty case below stays EmptyCompoundSource/Warning,
            // advisory). A query failure is a hard error, distinct from that
            // advisory case, so it gets its own InternalQueryError kind rather
            // than borrowing EmptyCompoundSource.
            push_internal_query_error(
                report,
                "compound membership",
                format!(
                    "compound-viewer source '{source}': could not check compound \
                     membership: {e}"
                ),
            );
            return;
        }
    };

    if !has_parts {
        report.push(Diagnostic {
            kind: DiagnosticKind::EmptyCompoundSource { source: source.to_owned() },
            message: format!("compound-viewer source '{source}' has no compound members"),
            span: None,
            severity: Severity::Warning,
        });
    }
}

/// Run `compound-viewer media="…"` validation across all pages of an
/// already-validated config.
///
/// Unlike `check_compound_viewer_data`, a `$param` source is NOT skipped: its
/// member classes come from the data (every `compound_parts` child's class),
/// so it is still checkable at boot. See `check_single_compound_viewer_media`.
///
/// `media_map` is the shared class→(property, page) map — the same one
/// `check_resource_detail_media` returned and `check_resource_list_media`
/// will check against next. Each resolved member class either registers
/// (first wins) or, if the class is already registered with a *different*
/// property, raises `ConflictingMediaProperty`.
fn check_compound_viewer_media(
    config: &ValidatedConfig,
    data: &dyn ProjectFacts,
    media_map: &mut std::collections::HashMap<String, (String, String)>,
    report: &mut ValidationReport,
) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::CompoundViewer { source, media: Some(ref media_prop), .. } = component {
                check_single_compound_viewer_media(source, media_prop, &page.route, data, media_map, report);
            }
        }
    }
}

/// Raw-fallback sibling of `check_compound_viewer_media` (the both-arms rule,
/// `dsp-incubator cpe/CLAUDE.md`). `source` is required, so an
/// absent/wrong-typed one was already reported by lowering's
/// `require_string_attr` — silent `continue`. `media` is read through the
/// silent `get_string_attr` (the `*_into_raw` contract); a
/// present-but-wrong-typed `media` is reported by lowering
/// (`get_typed_string_attr`), so this fn skips it too — one mistake, one
/// diagnostic.
fn check_compound_viewer_media_into_raw(
    raw: &RawConfig,
    data: &dyn ProjectFacts,
    media_map: &mut std::collections::HashMap<String, (String, String)>,
    report: &mut ValidationReport,
) {
    for raw_page in &raw.pages {
        let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "compound-viewer" {
                continue;
            }
            let Some(media_prop) = get_string_attr(raw_comp, "media") else {
                continue;
            };
            let Some(source) = get_string_attr(raw_comp, "source") else {
                continue;
            };
            check_single_compound_viewer_media(&source, &media_prop, page_route, data, media_map, report);
        }
    }
}

/// The shared per-component body of the `compound-viewer media="…"` check.
///
/// Resolves the member class(es) from the data — a literal `source` → the
/// distinct classes of its own `compound_parts` children; a `$param` source →
/// the distinct classes of **every** `compound_parts` child in the project —
/// and runs `check_single_media_property` (the `resource-detail` media
/// four-check) once per class, then registers the class into `media_map`
/// (first wins) or raises `ConflictingMediaProperty` against an
/// already-registered different property (`register_media_class`).
///
/// An empty class set (no members yet, or a literal source with no parts —
/// the latter already `EmptyCompoundSource`) skips silently. Deliberately
/// NOT `resource-detail`'s `UnresolvableMediaClass` hard-fail: an Error here
/// would abort the whole binary (dsp-incubator cpe/docs/adr/0002-deployment-topology.md) over a
/// compound that simply has nothing to show yet.
fn check_single_compound_viewer_media(
    source: &str,
    media_prop: &str,
    page_route: &str,
    data: &dyn ProjectFacts,
    media_map: &mut std::collections::HashMap<String, (String, String)>,
    report: &mut ValidationReport,
) {
    let parent = if source.starts_with('$') { None } else { Some(source) };
    let classes = match data.compound_member_classes(parent) {
        Ok(c) => c,
        Err(e) => {
            push_internal_query_error(
                report,
                "compound member classes",
                format!(
                    "page '{page_route}': could not resolve the member classes of \
                     compound-viewer source '{source}': {e}"
                ),
            );
            return;
        }
    };
    for class in &classes {
        check_single_media_property(class, media_prop, page_route, data, report);
        register_media_class(media_map, class, media_prop, page_route, "compound-viewer", report);
    }
}

/// Check `class` against the shared class→(property, page) media map,
/// pushing `ConflictingMediaProperty` when the class is already registered
/// with a *different* property. `component` names the declaration kind
/// raising the diagnostic (e.g. `"compound-viewer"`, `"resource-list"`),
/// making the message source-neutral without a dedicated `DiagnosticKind`
/// field for it. Returns `true` when
/// `class` was already present in `media_map` (conflicting or not) — the
/// signal `register_media_class` needs to decide whether to insert.
/// Check-only: never mutates `media_map`. Shared by `register_media_class`
/// (which additionally inserts on first sight) and the two `resource-list`
/// media checks (which never register).
fn check_media_conflict(
    media_map: &std::collections::HashMap<String, (String, String)>,
    class: &str,
    media_prop: &str,
    page_route: &str,
    component: &str,
    report: &mut ValidationReport,
) -> bool {
    let Some((registered_prop, registered_page)) = media_map.get(class) else {
        return false;
    };
    if registered_prop != media_prop {
        report.push(Diagnostic {
            kind: DiagnosticKind::ConflictingMediaProperty {
                class: class.to_owned(),
                registered_property: registered_prop.clone(),
                declared_property: media_prop.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "page '{page_route}': {component} media=\"{media_prop}\" conflicts with \
                 class '{class}''s media property \"{registered_prop}\" \
                 (declared on page '{registered_page}') — the two must agree"
            ),
            span: None,
            severity: Severity::Error,
        });
    }
    true
}

/// Register `class` into the shared class→(property, page) media map, or
/// raise `ConflictingMediaProperty` when the class is already registered
/// with a *different* property. First registration wins (mirrors
/// `build_class_routes`'s and `check_resource_detail_media`'s own
/// first-wins rule) — a repeat registration with the SAME property is not a
/// conflict and leaves the earlier (page, property) pair in place. Delegates
/// the conflict check itself to `check_media_conflict`, then inserts on
/// first sight (`check_media_conflict` never mutates `media_map`).
///
/// `component` names the declaration kind raising the diagnostic (e.g.
/// `"compound-viewer"`), making the message source-neutral without a
/// dedicated `DiagnosticKind` field for it (the message's `{component}`
/// placeholder).
fn register_media_class(
    media_map: &mut std::collections::HashMap<String, (String, String)>,
    class: &str,
    media_prop: &str,
    page_route: &str,
    component: &str,
    report: &mut ValidationReport,
) {
    if check_media_conflict(media_map, class, media_prop, page_route, component, report) {
        return;
    }
    media_map.insert(class.to_owned(), (media_prop.to_owned(), page_route.to_owned()));
}

/// Run data-dependent checks on all `facet-panel` components in a
/// successfully-lowered `ValidatedConfig`. Appends diagnostics to `report`;
/// never short-circuits.
fn check_facet_panel_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        // Build a map of resource-list id → class for target resolution.
        // We need this to resolve the facet-panel's target to the list's class.
        let list_class_map: std::collections::HashMap<&str, &str> = page
            .body
            .iter()
            .filter_map(|c| {
                if let Component::ResourceList { id: Some(ref id), ref class, .. } = c {
                    Some((id.as_str(), class.as_str()))
                } else {
                    None
                }
            })
            .collect();

        for component in &page.body {
            if let Component::FacetPanel { ref target, ref facets } = component {
                // Resolve the target to the matching resource-list's class.
                let target_class = match list_class_map.get(target.as_str()) {
                    Some(c) => *c,
                    None => continue, // No matching list; UnresolvedTarget already fired.
                };

                // Skip $param references — class resolved at request time, not statically.
                if target_class.starts_with('$') {
                    continue;
                }

                for facet in facets {
                    check_single_facet(target_class, facet, &page.route, data, report);
                }
            }
        }
    }
}

/// Run data-dependent checks on `facet-panel` components from the raw config
/// when config-internal checks already failed. We parse what we can from raw.
fn check_facet_panel_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        // Build a map of resource-list id → class from this page's raw components.
        let list_class_map: std::collections::HashMap<String, String> = raw_page
            .body
            .iter()
            .filter_map(|raw_comp| {
                if raw_comp.name != "resource-list" {
                    return None;
                }
                let id = get_string_attr(raw_comp, "id")?;
                let class = get_string_attr(raw_comp, "class")?;
                Some((id, class))
            })
            .collect();

        for raw_comp in &raw_page.body {
            if raw_comp.name != "facet-panel" {
                continue;
            }

            // Extract target from raw attrs (required field).
            let target = match get_string_attr(raw_comp, "target") {
                Some(t) => t,
                None => continue, // target is required; if absent, skip
            };

            // Resolve target to a class.
            let target_class = match list_class_map.get(&target) {
                Some(c) => c.as_str(),
                None => continue, // No matching list; skip.
            };

            // Skip $param references.
            if target_class.starts_with('$') {
                continue;
            }

            let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");

            // Extract facets from raw children.
            for child in &raw_comp.children {
                if child.name != "facet" {
                    continue;
                }

                let property = get_string_attr(child, "property");
                let kind_str = get_string_attr(child, "kind");

                let (Some(property), Some(kind_str)) = (property, kind_str) else {
                    continue; // Required fields missing; lowering already caught it.
                };

                let kind = match FacetKind::parse_config_str(&kind_str) {
                    Some(k) => k,
                    None => continue, // Unknown kind; config-internal check already caught it.
                };

                let facet = Facet { property, kind };
                check_single_facet(target_class, &facet, page_route, data, report);
            }
        }
    }
}

/// Run data-dependent checks on all `search-bar` components in a
/// successfully-lowered `ValidatedConfig`. Appends diagnostics to `report`;
/// never short-circuits. Mirrors `check_facet_panel_data` — resolve each
/// search-bar's `target` to its sibling `resource-list`'s class via a
/// per-page `id → class` map, then check every `searches` property name
/// independently via `check_single_search_property` (mirrors how
/// `check_facet_panel_data` calls `check_single_facet` once per facet — a
/// search-bar can surface multiple diagnostics, one per bad property).
fn check_search_bar_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for page in &config.pages {
        // Build a map of resource-list id → class for target resolution.
        let list_class_map: std::collections::HashMap<&str, &str> = page
            .body
            .iter()
            .filter_map(|c| {
                if let Component::ResourceList { id: Some(ref id), ref class, .. } = c {
                    Some((id.as_str(), class.as_str()))
                } else {
                    None
                }
            })
            .collect();

        for component in &page.body {
            if let Component::SearchBar { ref target, ref searches, .. } = component {
                // Resolve the target to the matching resource-list's class.
                let target_class = match list_class_map.get(target.as_str()) {
                    Some(c) => *c,
                    None => continue, // No matching list; UnresolvedTarget already fired.
                };

                // Skip $param references — class resolved at request time, not statically.
                if target_class.starts_with('$') {
                    continue;
                }

                for prop in searches {
                    check_single_search_property(target_class, prop, &page.route, data, report);
                }
            }
        }
    }
}

/// Run data-dependent checks on `search-bar` components from the raw config
/// when config-internal checks already failed. We parse what we can from raw.
/// Mirrors `check_facet_panel_data_into_raw`, but `searches` is a list-valued
/// attribute (attr + trailing positional args), not child nodes — the same
/// reconstruction `lower_search_bar` itself performs, not the facet
/// child-iteration pattern.
fn check_search_bar_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for raw_page in &raw.pages {
        // Build a map of resource-list id → class from this page's raw components.
        let list_class_map: std::collections::HashMap<String, String> = raw_page
            .body
            .iter()
            .filter_map(|raw_comp| {
                if raw_comp.name != "resource-list" {
                    return None;
                }
                let id = get_string_attr(raw_comp, "id")?;
                let class = get_string_attr(raw_comp, "class")?;
                Some((id, class))
            })
            .collect();

        for raw_comp in &raw_page.body {
            if raw_comp.name != "search-bar" {
                continue;
            }

            // Extract target from raw attrs (required field).
            let target = match get_string_attr(raw_comp, "target") {
                Some(t) => t,
                None => continue, // target is required; if absent, skip
            };

            // Resolve target to a class.
            let target_class = match list_class_map.get(&target) {
                Some(c) => c.as_str(),
                None => continue, // No matching list; skip.
            };

            // Skip $param references.
            if target_class.starts_with('$') {
                continue;
            }

            let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");

            // `searches` is a list-valued attribute: reconstruct it as
            // `[attrs["searches"]] ++ positional args` — the same
            // reconstruction `lower_search_bar` performs, since there are no
            // child nodes to iterate here (unlike `facet-panel`'s `facet`
            // children).
            let first_search = get_string_attr(raw_comp, "searches");
            let Some(first_search) = first_search else {
                continue; // searches is required; lowering already caught it.
            };
            let mut searches = vec![first_search];
            for arg in &raw_comp.args {
                if let crate::ir::Value::String(s) = arg {
                    searches.push(s.clone());
                }
            }

            for prop in &searches {
                check_single_search_property(target_class, prop, page_route, data, report);
            }
        }
    }
}

/// Query keys the host reads from the request query string for its own
/// purposes (theme/appearance overrides, i18n, pagination, sort, membership
/// index, DataStar's own reserved key, resets, the search box's own `?q=`
/// reader, and the record viewer's thumb-strip fold state `?strip=` —
/// reserved so a project could never author a colliding
/// property name, …). Facet values are read back at request time via the facet's
/// `property` name used *as* a query key (see the host's facet-narrowing
/// wiring), so a facet property colliding with one of these would be
/// silently shadowed by (or shadow) the host's own handling of that key. `"q"`
/// specifically guards against a **facet** `property` attribute literally
/// named `q` colliding with the search box's own `?q=` reader — it is not a
/// `search-bar` `searches` property-name concern (those are checked
/// elsewhere, by `check_search_bar_data`, for existence/type, not reservation).
///
/// MUST stay in sync with the query keys read in `dsp-incubator cpe/bin/src/lib.rs`,
/// `dsp-incubator cpe/bin/src/router.rs` and `cpe-components` (grid-mode's `?view=` is
/// read there, by `component::resource_list::parse_view_param`). This const duplicates the
/// knowledge deliberately.
const RESERVED_FACET_PROPERTIES: &[&str] = &[
    "accent",
    "appearance",
    "background",
    "datastar",
    "img",
    "lang",
    "member",
    "page",
    "q",
    "reset",
    "sort",
    "strip",
    "typeface",
    "view",
];

/// Perform facet data-dependent checks for one facet.
///
/// Checks (in priority order, one fires per facet at most):
/// - `ReservedFacetProperty` (Error) — property shadows a reserved query key (pure config — checked
///   before anything else, independent of `kind`)
/// - `UnsupportedFacetKind` (Error) — kind != List
/// - `UnknownFacetProperty` (Error) — property absent everywhere
/// - `IncompatibleFacetKind` (Error) — list-kind on a non-list-value property
/// - `SparseFacetProperty` (Warning) — property absent on target class
///
/// Appends to `report`. Stops further checks for the current facet after the
/// first applicable condition fires (conditions are mutually exclusive per facet).
fn check_single_facet(
    target_class: &str,
    facet: &Facet,
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // Guard 0 (pure config, no data): a facet property must not shadow a
    // reserved query key the host reads for its own purposes. Checked first —
    // it needs no query and applies regardless of `kind`.
    if RESERVED_FACET_PROPERTIES.contains(&facet.property.as_str()) {
        report.push(Diagnostic {
            kind: DiagnosticKind::ReservedFacetProperty {
                property: facet.property.clone(),
                page: page_route.to_owned(),
            },
            message: format!(
                "facet-panel on page '{page_route}': facet property '{}' collides with a \
                 reserved query key the engine reads at runtime — rename the property; \
                 reserved keys are: {}",
                facet.property,
                RESERVED_FACET_PROPERTIES.join(", ")
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Only the `list` kind is implemented; all others are
    // valid IR kinds but unsupported here → hard-fail one diagnostic per facet.
    if facet.kind != FacetKind::List {
        let kind_str = match facet.kind {
            FacetKind::Text => "text",
            FacetKind::List => "list",
            FacetKind::DateDecade => "date-decade",
            FacetKind::DateYear => "date-year",
            FacetKind::Boolean => "boolean",
            FacetKind::Range => "range",
        };
        report.push(Diagnostic {
            kind: DiagnosticKind::UnsupportedFacetKind { kind: kind_str.to_owned(), page: page_route.to_owned() },
            message: format!(
                "facet-panel on page '{page_route}': facet property '{}' uses kind \
                 '{kind_str}' which is not yet implemented (only 'list' is supported \
                 in this slice)",
                facet.property
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // `list` kind — three mutually exclusive checks in priority order.
    let prop = &facet.property;

    // Check 1: property must exist anywhere in the data (hard-fail).
    let exists_anywhere = match data.property_exists_anywhere(prop) {
        Ok(b) => b,
        Err(e) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownFacetProperty {
                    property: prop.to_owned(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "facet-panel on page '{page_route}': could not check property \
                     existence for '{prop}': {e}"
                ),
                span: None,
                severity: Severity::Error,
            });
            return;
        }
    };

    if !exists_anywhere {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownFacetProperty {
                property: prop.to_owned(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "facet-panel on page '{page_route}': facet property '{prop}' does not \
                 exist anywhere in the data (target class '{target_class}')"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 2: if present on the class, verify value_type compatibility.
    // Empty types set means property is absent on this class — handled in check 3.
    let types = match data.property_value_types_on_class(target_class, prop) {
        Ok(t) => t,
        Err(e) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::IncompatibleFacetKind {
                    property: prop.to_owned(),
                    kind: "list".to_owned(),
                    value_type: String::new(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "facet-panel on page '{page_route}': could not check value type \
                     for property '{prop}' on class '{target_class}': {e}"
                ),
                span: None,
                severity: Severity::Error,
            });
            return;
        }
    };

    if !types.is_empty() && !types.contains("list_value") {
        // Property is present on the class but with the wrong value type.
        // Pick the minimum for a deterministic, stable message (not arbitrary
        // HashSet iteration order).
        let actual_type = types.iter().min().expect("types is non-empty — checked above").clone();
        report.push(Diagnostic {
            kind: DiagnosticKind::IncompatibleFacetKind {
                property: prop.to_owned(),
                kind: "list".to_owned(),
                value_type: actual_type.clone(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "facet-panel on page '{page_route}': facet property '{prop}' on class \
                 '{target_class}' has value type '{actual_type}', but kind 'list' \
                 requires 'list_value'"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 3: property must be present on this class (warn when sparse).
    let class_props = match data.properties_present_on_class(target_class) {
        Ok(set) => set,
        Err(e) => {
            // Query error: use SparseFacetProperty as the nearest diagnostic kind
            // (mirrors the analogous resource-list pattern with SparseProperty).
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseFacetProperty {
                    property: prop.to_owned(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "facet-panel on page '{page_route}': could not fetch properties \
                     for class '{target_class}': {e}"
                ),
                span: None,
                severity: Severity::Warning,
            });
            return;
        }
    };

    if !class_props.contains(prop.as_str()) {
        report.push(Diagnostic {
            kind: DiagnosticKind::SparseFacetProperty {
                property: prop.to_owned(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "facet-panel on page '{page_route}': facet property '{prop}' is not \
                 present on any resource of class '{target_class}'"
            ),
            span: None,
            severity: Severity::Warning,
        });
    }
}

/// Perform search-bar data-dependent checks for one `searches` property name.
///
/// Mirrors `check_single_facet`'s three-check structure and priority order
/// (checks 1 and 2 hard-fail; check 3 is an advisory sparse-warn), with the
/// facet-specific parts swapped for search-bar equivalents. Unlike
/// `check_single_facet`, there is no reserved-property guard (a search
/// property is never itself read back as a query key — only the fixed `?q=`
/// key is) and no `kind` concept (search has exactly one behaviour: FTS5 over
/// text/richtext).
///
/// Checks (in priority order, one fires per property at most):
/// - `UnknownSearchProperty` (Error) — property absent everywhere
/// - `IncompatibleSearchProperty` (Error) — present on the class with a value type that is neither
///   `text` nor `richtext`
/// - `SparseSearchProperty` (Warning) — present somewhere but absent on the target class
///
/// Appends to `report`. Stops further checks for the current property after
/// the first applicable condition fires (conditions are mutually exclusive
/// per property).
fn check_single_search_property(
    target_class: &str,
    prop: &str,
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // Check 1: property must exist anywhere in the data (hard-fail).
    let exists_anywhere = match data.property_exists_anywhere(prop) {
        Ok(b) => b,
        Err(e) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownSearchProperty {
                    property: prop.to_owned(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "search-bar on page '{page_route}': could not check property \
                     existence for '{prop}': {e}"
                ),
                span: None,
                severity: Severity::Error,
            });
            return;
        }
    };

    if !exists_anywhere {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownSearchProperty {
                property: prop.to_owned(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "search-bar on page '{page_route}': searchable property '{prop}' does not \
                 exist anywhere in the data (target class '{target_class}')"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 2: if present on the class, verify value_type compatibility.
    // Empty types set means property is absent on this class — handled in check 3.
    let types = match data.property_value_types_on_class(target_class, prop) {
        Ok(t) => t,
        Err(e) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::IncompatibleSearchProperty {
                    property: prop.to_owned(),
                    value_type: String::new(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "search-bar on page '{page_route}': could not check value type \
                     for property '{prop}' on class '{target_class}': {e}"
                ),
                span: None,
                severity: Severity::Error,
            });
            return;
        }
    };

    if !types.is_empty() && !types.contains("text") && !types.contains("richtext") {
        // Property is present on the class but with the wrong value type.
        // Pick the minimum for a deterministic, stable message (not arbitrary
        // HashSet iteration order).
        let actual_type = types.iter().min().expect("types is non-empty — checked above").clone();
        report.push(Diagnostic {
            kind: DiagnosticKind::IncompatibleSearchProperty {
                property: prop.to_owned(),
                value_type: actual_type.clone(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "search-bar on page '{page_route}': searchable property '{prop}' on class \
                 '{target_class}' has value type '{actual_type}', but search requires \
                 'text' or 'richtext'"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 3: property must be present on this class (warn when sparse).
    let class_props = match data.properties_present_on_class(target_class) {
        Ok(set) => set,
        Err(e) => {
            // Query error: use SparseSearchProperty as the nearest diagnostic
            // kind (mirrors check_single_facet's analogous Err arm — a query
            // failure, not a genuine sparse finding, but this is the facet
            // precedent and this mirrors it exactly).
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseSearchProperty {
                    property: prop.to_owned(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "search-bar on page '{page_route}': could not fetch properties \
                     for class '{target_class}': {e}"
                ),
                span: None,
                severity: Severity::Warning,
            });
            return;
        }
    };

    if !class_props.contains(prop) {
        report.push(Diagnostic {
            kind: DiagnosticKind::SparseSearchProperty {
                property: prop.to_owned(),
                target_class: target_class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "search-bar on page '{page_route}': searchable property '{prop}' is not \
                 present on any resource of class '{target_class}'"
            ),
            span: None,
            severity: Severity::Warning,
        });
    }
}

/// Perform the UnknownResourceListClass + UnknownResourceListProperty +
/// SparseResourceListProperty checks for one `resource-list` component.
/// Appends to `report`; never short-circuits.
fn check_single_resource_list<'a>(
    class: &str,
    wheres: &'a [crate::ir::WhereClause],
    sorts: &'a [crate::ir::Sort],
    display_props: &[&'a str],
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // Check 1: class must exist in the data (hard-fail).
    let class_known = match data.class_exists(class) {
        Ok(b) => b,
        Err(e) => {
            // Query failure, not "class confirmed missing" — don't reuse
            // UnknownResourceListClass (the genuine-condition push below).
            push_internal_query_error(
                report,
                "resource-list class existence",
                format!(
                    "resource-list on page '{page_route}': could not check class existence \
                     for '{class}': {e}"
                ),
            );
            return;
        }
    };

    if !class_known {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownResourceListClass { class: class.to_owned(), page: page_route.to_owned() },
            message: format!(
                "resource-list on page '{page_route}' references class '{class}' \
                 which does not exist in the data"
            ),
            span: None,
            severity: Severity::Error,
        });
        // When the class is unknown, skip per-property sparse checks: the class
        // has no properties so every property would fire SparseResourceListProperty,
        // producing noisy redundant diagnostics. The UnknownResourceListClass error
        // is the headline finding; the operator should fix the class name first.
        // Still run the property_exists_anywhere check (hard-fail tier) since
        // a property that exists nowhere is a separate authoring error.
        let prop_names = collect_all_props(wheres, sorts, display_props);
        for prop in prop_names {
            check_resource_list_property_hard_fail_only(prop, class, page_route, data, report);
        }
        return;
    }

    // Check 2: for each where/sort/gloss/chip property, two-tier check.
    let prop_names = collect_all_props(wheres, sorts, display_props);
    if prop_names.is_empty() {
        return;
    }

    // Fetch the full set of properties present on this class (for sparse check).
    let class_props = match data.properties_present_on_class(class) {
        Ok(set) => set,
        Err(e) => {
            // Unexpected query error — push a query-failure Warning (not a
            // SparseResourceListProperty with an empty property name, which
            // would be semantically wrong) and skip the sparse checks.
            // Mirrors the analogous Err arm in `check_resource_detail_data`
            // which uses `DiagnosticKind::SparseProperty` for the query-failure
            // case.
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseProperty,
                message: format!(
                    "resource-list on page '{page_route}': could not fetch properties \
                     for class '{class}': {e}"
                ),
                span: None,
                severity: Severity::Warning,
            });
            return;
        }
    };

    for prop in prop_names {
        // Hard-fail tier: property exists nowhere in the data.
        let exists_anywhere = match data.property_exists_anywhere(prop) {
            Ok(b) => b,
            Err(e) => {
                // Query failure, not "property confirmed absent" — don't reuse
                // UnknownResourceListProperty (the genuine-condition push below).
                push_internal_query_error(
                    report,
                    "resource-list property existence",
                    format!(
                        "resource-list on page '{page_route}': could not check property \
                         existence for '{prop}': {e}"
                    ),
                );
                continue; // continue to next property (accumulating)
            }
        };

        if !exists_anywhere {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownResourceListProperty {
                    property: prop.to_owned(),
                    class: class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "resource-list on page '{page_route}': referenced property '{prop}' \
                     does not exist anywhere in the data (class '{class}')"
                ),
                span: None,
                severity: Severity::Error,
            });
        } else if !class_props.contains(prop) {
            // Sparse warn tier: exists somewhere but not on this class.
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseResourceListProperty {
                    property: prop.to_owned(),
                    class: class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "resource-list on page '{page_route}': referenced property '{prop}' \
                     is not present on any resource of class '{class}'"
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }

    // Check 3 (Guard 1 — `IncompatibleWhereValue`, Warning): for each `where`
    // Equals/Range value, verify the value's kind can possibly match the
    // property's actual storage on this class. Only runs for a property
    // confirmed present on the class (`class_props.contains`) — the
    // absent/sparse cases are already covered by checks above. `In` and
    // `TargetIn` clauses are out of scope (see `IncompatibleWhereValue`'s doc
    // comment in `diagnostic.rs`).
    for wc in wheres {
        match wc {
            WhereClause::Equals { property, value } => {
                check_where_value_compatibility(class, property, value, &class_props, page_route, data, report);
            }
            WhereClause::Range { property, start, end } => {
                for value in [start, end].into_iter().flatten() {
                    check_where_value_compatibility(class, property, value, &class_props, page_route, data, report);
                }
            }
            WhereClause::In { .. } | WhereClause::TargetIn { .. } => {}
            // `Search` carries no config-time-checkable value — its
            // target-class + searchable-property checks are a validation
            // concern owned by `check_search_bar_data`, not this
            // generic where-value-compatibility pass.
            WhereClause::Search { .. } => {}
        }
    }
}

/// The `properties` column the store's `build_where` compares for a `where`
/// value of this kind. Mirrors the store's `value_col_and_param` type→column
/// mapping (kept in sync by hand — that function is private to the store and
/// additionally binds a query parameter this caller doesn't need).
fn where_value_column(value: &Value) -> &'static str {
    match value {
        Value::String(_) | Value::Null => "value_text",
        Value::Integer(_) | Value::Bool(_) => "value_int",
        Value::Float(_) => "value_real",
    }
}

/// Human-readable label for a `where`-clause value's kind, for diagnostic
/// messages.
fn value_kind_label(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}

/// The `properties` column a given `value_type` populates, per the canonical
/// table documented on the store's `malformed_property_rows`. Unrecognised
/// `value_type` strings return `None` (defensive; the schema `CHECK`
/// constraint prevents this in practice).
fn column_for_value_type(value_type: &str) -> Option<&'static str> {
    match value_type {
        "text" | "richtext" => Some("value_text"),
        "integer" | "boolean" => Some("value_int"),
        "decimal" => Some("value_real"),
        "date" => Some("date_start"),
        "link" | "list_value" => Some("target_id"),
        _ => None,
    }
}

/// Push an `IncompatibleWhereValue` warning when `value`'s kind can never
/// match `property`'s actual storage on `class` — i.e. none of the
/// `value_type`s the property has on this class populate the column `value`'s
/// kind would compare against. A no-op when the property is not confirmed
/// present on the class, when the type-lookup query fails, or when at least
/// one stored type is compatible.
fn check_where_value_compatibility(
    class: &str,
    property: &str,
    value: &Value,
    class_props: &std::collections::HashSet<String>,
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    if !class_props.contains(property) {
        // Absent/sparse on this class — UnknownResourceListProperty /
        // SparseResourceListProperty already cover it above.
        return;
    }

    let types = match data.property_value_types_on_class(class, property) {
        Ok(t) if !t.is_empty() => t,
        _ => return, // query error, or (defensively) empty despite class_props — nothing to compare
    };

    let target_col = where_value_column(value);
    if types.iter().any(|vt| column_for_value_type(vt) == Some(target_col)) {
        return; // at least one stored type could populate the compared column
    }

    let mut stored_types: Vec<&str> = types.iter().map(String::as_str).collect();
    stored_types.sort_unstable();
    let stored_types = stored_types.join("/");

    report.push(Diagnostic {
        kind: DiagnosticKind::IncompatibleWhereValue {
            property: property.to_owned(),
            class: class.to_owned(),
            page: page_route.to_owned(),
        },
        message: format!(
            "resource-list on page '{page_route}': where property '{property}' on class \
             '{class}' compares a {} value, but every resource stores '{property}' as \
             '{stored_types}' — this filter can never match",
            value_kind_label(value)
        ),
        span: None,
        severity: Severity::Warning,
    });
}

/// When the class is unknown, only run the hard-fail (exists-nowhere) tier
/// for each property — skip the sparse check entirely to avoid noise.
fn check_resource_list_property_hard_fail_only(
    prop: &str,
    class: &str,
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let exists_anywhere = match data.property_exists_anywhere(prop) {
        Ok(b) => b,
        Err(e) => {
            // Query failure, not "property confirmed absent" — mirrors the
            // analogous branch in check_single_resource_list.
            push_internal_query_error(
                report,
                "resource-list property existence",
                format!(
                    "resource-list on page '{page_route}': could not check property \
                     existence for '{prop}': {e}"
                ),
            );
            return;
        }
    };
    if !exists_anywhere {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownResourceListProperty {
                property: prop.to_owned(),
                class: class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "resource-list on page '{page_route}': referenced property '{prop}' \
                 does not exist anywhere in the data (class '{class}')"
            ),
            span: None,
            severity: Severity::Error,
        });
    }
}

/// Collect unique property names from all `where` and `sort` clauses.
/// Uses a `Vec` (not `HashSet`) to preserve order; duplicates are deduplicated
/// by checking before inserting.
///
/// `SortKey::Class`/`SortKey::Id` sorts are skipped — they are not EAV
/// property names and must not trigger `UnknownResourceListProperty` /
/// `SparseResourceListProperty` validation. In practice only
/// `SortKey::Property` ever reaches this function from static config (see
/// `SortKey`'s doc comment), but the match is exhaustive for correctness.
fn collect_where_sort_props<'a>(wheres: &'a [crate::ir::WhereClause], sorts: &'a [crate::ir::Sort]) -> Vec<&'a str> {
    let mut names: Vec<&'a str> = Vec::new();
    for wc in wheres {
        let prop = match wc {
            crate::ir::WhereClause::Equals { property, .. }
            | crate::ir::WhereClause::In { property, .. }
            | crate::ir::WhereClause::Range { property, .. }
            | crate::ir::WhereClause::TargetIn { property, .. } => property.as_str(),
            // `Search` has no `property` field (it's a table+query pair) and
            // isn't a real config property reference — skip this iteration,
            // contributing no name to the where/sort existence-check list.
            crate::ir::WhereClause::Search { .. } => continue,
        };
        if !names.contains(&prop) {
            names.push(prop);
        }
    }
    for s in sorts {
        let prop = match &s.key {
            SortKey::Property(p) => Some(p.as_str()),
            SortKey::Class | SortKey::Id => None,
        };
        if let Some(prop) = prop {
            if !names.contains(&prop) {
                names.push(prop);
            }
        }
    }
    names
}

/// Collect the deduplicated set of property names a `resource-list` references
/// for boot-time existence validation: the `where`/`sort` properties (via
/// [`collect_where_sort_props`]) plus the `display_props` (gloss + chip
/// property names). All share the caller's lifetime, so the returned `&str`s
/// borrow from `wheres`/`sorts`/`display_props` without allocation.
fn collect_all_props<'a>(
    wheres: &'a [crate::ir::WhereClause],
    sorts: &'a [crate::ir::Sort],
    display_props: &[&'a str],
) -> Vec<&'a str> {
    let mut names = collect_where_sort_props(wheres, sorts);
    for &p in display_props {
        if !names.contains(&p) {
            names.push(p);
        }
    }
    names
}

/// Check that every `properties` row has its required value column populated
/// for its `value_type`. One `MalformedPropertyRow` diagnostic per violating row.
/// Appends to `report`; never short-circuits.
///
/// This check reads data, not config, so it runs in both branches of
/// `validate_with_data` (Ok(config) and Err(report)).
fn check_malformed_property_rows(data: &dyn ProjectFacts, report: &mut ValidationReport) {
    let rows = match data.malformed_property_rows() {
        Ok(r) => r,
        Err(e) => {
            // Unexpected query error on a freshly loaded DB — the query itself
            // failed, not "a malformed row was found". Don't reuse
            // MalformedPropertyRow (the genuine-condition push below).
            push_internal_query_error(
                report,
                "property-row consistency",
                format!("could not check property-row consistency: {e}"),
            );
            return;
        }
    };

    for (resource_id, name, value_type, missing_column) in rows {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::MalformedPropertyRow,
            format!(
                "resource '{resource_id}': property '{name}' has value_type='{value_type}' \
                 but '{missing_column}' is NULL — every '{value_type}' row must have a \
                 non-NULL '{missing_column}'"
            ),
        ));
    }
}

/// Slack on every annotation coordinate comparison, so a box that touches an
/// image edge after a float round-trip still counts as inside. The store builder's
/// geometry module (`areas/access/cpe/store-build/src/generic/geometry.rs`)
/// checks with the same value, and a test there reads the line below; keep the two
/// equal.
const ANNOTATION_TOLERANCE: f64 = 1e-9;

/// The rules one `annotations` row breaks (dsp-incubator
/// cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md), one message each; empty when
/// the row is well formed. Pure, so the tolerance edges are unit-tested without a database.
fn annotation_row_violations(row: &facts::AnnotationCheckRow) -> Vec<String> {
    let t = ANNOTATION_TOLERANCE;
    let id = &row.id;
    let mut out = Vec::new();
    let (x, y, w, h) = (row.x, row.y, row.w, row.h);
    let inside_image = x >= -t && y >= -t && x + w <= 1.0 + t && y + h <= 1.0 + t;
    if !(w > 0.0 && h > 0.0 && inside_image) {
        out.push(format!(
            "annotation '{id}': box x={x} y={y} w={w} h={h} must lie inside the image \
             (0..1) with w and h above 0"
        ));
    }
    let shape = AnnotationShape::from_db(&row.shape);
    match (shape, row.points.as_deref()) {
        (None, _) => out.push(format!("annotation '{id}': unknown shape '{}'", row.shape)),
        (Some(AnnotationShape::Polygon), None) => {
            out.push(format!("annotation '{id}': a polygon needs points"));
        }
        (Some(AnnotationShape::Polygon), Some(text)) => match crate::ir::parse_annotation_points(text) {
            None => out.push(format!(
                "annotation '{id}': points '{text}' must be whitespace-separated x,y pairs"
            )),
            Some(vertices) if vertices.len() < 3 => out.push(format!(
                "annotation '{id}': a polygon needs at least 3 vertices, found {}",
                vertices.len()
            )),
            Some(vertices) => {
                let outside = vertices
                    .iter()
                    .any(|&(px, py)| px < x - t || px > x + w + t || py < y - t || py > y + h + t);
                if outside {
                    out.push(format!("annotation '{id}': every polygon vertex must lie inside its box"));
                }
            }
        },
        (Some(AnnotationShape::Rectangle | AnnotationShape::Circle), Some(_)) => {
            out.push(format!("annotation '{id}': only a polygon carries points, not a {}", row.shape))
        }
        (Some(AnnotationShape::Rectangle | AnnotationShape::Circle), None) => {}
    }
    if let Some(color) = &row.color {
        let well_formed = color.len() == 7
            && color.starts_with('#')
            && color[1..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !well_formed {
            out.push(format!("annotation '{id}': colour '{color}' must be lowercase #rrggbb"));
        }
    }
    out
}

/// Check every `annotations` row and every `annotation_links` row against
/// the rules the DDL does not enforce (dsp-incubator
/// cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md). One `MalformedAnnotation`
/// per violated rule. Reads data, not config, so both `validate_with_data`
/// arms call it.
fn check_malformed_annotations(data: &dyn ProjectFacts, report: &mut ValidationReport) {
    let rows = match data.malformed_annotation_rows() {
        Ok(r) => r,
        Err(e) => {
            push_internal_query_error(
                report,
                "annotation consistency",
                format!("could not check annotation consistency: {e}"),
            );
            return;
        }
    };
    let annotation_messages = rows.annotations.iter().flat_map(annotation_row_violations);
    let link_messages = rows
        .empty_links
        .iter()
        .map(|id| format!("annotation link '{id}' must have at least one target"));
    for message in annotation_messages.chain(link_messages) {
        report.push(Diagnostic::error_spanless(DiagnosticKind::MalformedAnnotation, message));
    }
}

#[cfg(test)]
mod annotation_check_tests {
    use super::annotation_row_violations;
    use super::facts::AnnotationCheckRow;

    fn row(shape: &str, (x, y, w, h): (f64, f64, f64, f64), points: Option<&str>) -> AnnotationCheckRow {
        AnnotationCheckRow {
            id: "a1".to_owned(),
            shape: shape.to_owned(),
            x,
            y,
            w,
            h,
            points: points.map(str::to_owned),
            color: Some("#ff3333".to_owned()),
        }
    }

    fn violations(r: &AnnotationCheckRow) -> usize {
        annotation_row_violations(r).len()
    }

    #[test]
    fn well_formed_shapes_pass() {
        assert_eq!(violations(&row("rectangle", (0.0, 0.0, 1.0, 1.0), None)), 0);
        assert_eq!(violations(&row("circle", (0.2, 0.3, 0.1, 0.05), None)), 0);
        let poly = row("polygon", (0.1, 0.1, 0.4, 0.4), Some("0.1,0.1 0.5,0.1 0.3,0.5"));
        assert_eq!(violations(&poly), 0);
        let mut no_colour = row("rectangle", (0.1, 0.1, 0.1, 0.1), None);
        no_colour.color = None;
        assert_eq!(violations(&no_colour), 0, "a NULL colour is allowed");
    }

    #[test]
    fn box_edges_are_tolerant_to_1e9_only() {
        assert_eq!(violations(&row("rectangle", (0.5, 0.0, 0.5 + 1e-10, 0.2), None)), 0);
        assert_eq!(violations(&row("rectangle", (-1e-10, 0.0, 0.2, 0.2), None)), 0);
        assert_eq!(violations(&row("rectangle", (0.5, 0.0, 0.5 + 1e-8, 0.2), None)), 1);
        assert_eq!(violations(&row("rectangle", (0.0, -1e-8, 0.2, 0.2), None)), 1);
        assert_eq!(violations(&row("rectangle", (0.9, 0.0, 0.2, 0.2), None)), 1);
    }

    #[test]
    fn empty_box_fails() {
        assert_eq!(violations(&row("rectangle", (0.1, 0.1, 0.0, 0.2), None)), 1);
        assert_eq!(violations(&row("circle", (0.1, 0.1, 0.2, -0.1), None)), 1);
    }

    #[test]
    fn polygon_points_are_required_parsed_counted_and_contained() {
        let b = (0.1, 0.1, 0.4, 0.4);
        assert_eq!(violations(&row("polygon", b, None)), 1);
        assert_eq!(violations(&row("polygon", b, Some("0.1,0.1 0.5;0.1 0.3,0.5"))), 1);
        assert_eq!(violations(&row("polygon", b, Some("0.1,0.1 0.5,0.1"))), 1);
        assert_eq!(
            violations(&row("polygon", b, Some("0.1,0.1 0.5,0.1 0.3,0.6"))),
            1,
            "a vertex below the box"
        );
        assert_eq!(
            violations(&row("polygon", b, Some("0.1,0.1 0.5,0.1 0.3,0.5000000001"))),
            0,
            "inside within tolerance"
        );
    }

    #[test]
    fn points_on_a_non_polygon_fail() {
        assert_eq!(violations(&row("rectangle", (0.1, 0.1, 0.4, 0.4), Some("0.1,0.1 0.5,0.5"))), 1);
    }

    #[test]
    fn colour_must_be_lowercase_rrggbb() {
        for bad in ["#FF3333", "ff3333", "#ff333", "#ff33333", "#gg3333", "red"] {
            let mut r = row("rectangle", (0.1, 0.1, 0.1, 0.1), None);
            r.color = Some(bad.to_owned());
            assert_eq!(violations(&r), 1, "{bad} must be rejected");
        }
    }

    #[test]
    fn each_broken_rule_is_reported_once() {
        let mut r = row("polygon", (0.9, 0.1, 0.4, 0.4), Some("0,0 1,1"));
        r.color = Some("#FFF".to_owned());
        assert_eq!(violations(&r), 3, "box, vertex count and colour");
    }
}

/// Is `s` a plain DSP data ARK URL, i.e.
/// `^https://ark\.dasch\.swiss/ark:/72163/1/[0-9A-F]{4}/[A-Za-z0-9_=]+$`?
/// Hand-written: this crate has no regex dependency. A version ARK's
/// `.<timestamp>` suffix, `http:`, another host, a trailing `/`, a lowercase
/// shortcode, an unescaped `-` and `''` all fail. `dsp-incubator
/// cpe/tools/dao-lift/src/ark.rs` holds the same matcher for `cpe-export`'s own
/// check.
pub(crate) fn is_plain_data_ark(s: &str) -> bool {
    let Some((shortcode, id)) = s.strip_prefix(crate::ir::DATA_ARK_PREFIX).and_then(|rest| rest.split_once('/')) else {
        return false;
    };
    shortcode.len() == 4
        && shortcode.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        && !id.is_empty()
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'=')
}

/// Check every stored `resources.ark` and `annotations.ark` with
/// [`is_plain_data_ark`]. One `MalformedDataArk` per bad row. Reads data, not
/// config, so both `validate_with_data` arms call it.
fn check_malformed_data_arks(data: &dyn ProjectFacts, report: &mut ValidationReport) {
    let rows = match data.malformed_data_ark_rows() {
        Ok(r) => r,
        Err(e) => {
            push_internal_query_error(report, "data ARK shape", format!("could not check the stored data ARKs: {e}"));
            return;
        }
    };
    for row in rows.iter().filter(|r| !is_plain_data_ark(&r.ark)) {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::MalformedDataArk,
            format!(
                "{} '{}': ark {:?} is not a plain DSP data ARK \
                 ({}<SHORTCODE>/<id>, no version suffix); store NULL when there is none",
                row.table,
                row.id,
                row.ark,
                crate::ir::DATA_ARK_PREFIX
            ),
        ));
    }
}

#[cfg(test)]
mod data_ark_check_tests {
    use super::is_plain_data_ark;

    #[test]
    fn plain_data_arks_pass() {
        for ok in [
            // `=` (an escaped `-`) and `_` in the id
            "https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7",
            "https://ark.dasch.swiss/ark:/72163/1/0803/XtSmDsS4V0C2=FjlF4Zy_wT",
            "https://ark.dasch.swiss/ark:/72163/1/0001/bmfk1DMHRBiR4=_6HXpEFA=",
            // an uppercase hex shortcode
            "https://ark.dasch.swiss/ark:/72163/1/081C/abc",
        ] {
            assert!(is_plain_data_ark(ok), "{ok} must pass");
        }
    }

    #[test]
    fn everything_else_fails() {
        for bad in [
            "",
            // a lowercase or three-character shortcode
            "https://ark.dasch.swiss/ark:/72163/1/081c/abc",
            "https://ark.dasch.swiss/ark:/72163/1/803/abc",
            // a non-hex shortcode
            "https://ark.dasch.swiss/ark:/72163/1/08G3/abc",
            // `-` in the id (unescaped)
            "https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3-JfqVZOkd7hUQ26kTg7",
            // a version ARK
            "https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7.20180604T085622Z",
            "http://ark.dasch.swiss/ark:/72163/1/0803/abc",
            // a trailing `/`
            "https://ark.dasch.swiss/ark:/72163/1/0803/abc/",
            // a foreign host
            "https://ark.example.org/ark:/72163/1/0803/abc",
            "https://ark.dasch.swiss.example.org/ark:/72163/1/0803/abc",
            // another version or assigned number
            "https://ark.dasch.swiss/ark:/72163/2/0803/abc",
            "https://ark.dasch.swiss/ark:/99999/1/0803/abc",
            // no id, no shortcode
            "https://ark.dasch.swiss/ark:/72163/1/0803/",
            "https://ark.dasch.swiss/ark:/72163/1/0803",
            // surrounding whitespace, a query
            " https://ark.dasch.swiss/ark:/72163/1/0803/abc",
            "https://ark.dasch.swiss/ark:/72163/1/0803/abc ",
            "https://ark.dasch.swiss/ark:/72163/1/0803/abc?x=1",
            "https://ark.dasch.swiss/ark:/72163/1/0803/abc\n",
            // an extra segment, a mixed-case, five-character or empty shortcode
            "https://ark.dasch.swiss/ark:/72163/1/0803/abc/def",
            "https://ark.dasch.swiss/ark:/72163/1/08a3/abc",
            "https://ark.dasch.swiss/ark:/72163/1/08031/abc",
            "https://ark.dasch.swiss/ark:/72163/1//abc",
            // non-ASCII in the id
            "https://ark.dasch.swiss/ark:/72163/1/0803/ab\u{e9}",
        ] {
            assert!(!is_plain_data_ark(bad), "{bad:?} must fail");
        }
    }
}

/// Check that every `link`-typed property value's `target_id` exists in
/// `resources`. One `UnknownLinkTarget` (Warning) diagnostic per dangling pair.
/// Appends to `report`; never short-circuits.
///
/// Runs after `check_malformed_property_rows` so all `link` rows with a
/// non-NULL `target_id` are already known to be well-formed.
fn check_link_targets(data: &dyn ProjectFacts, report: &mut ValidationReport) {
    let pairs = match data.dangling_link_targets() {
        Ok(p) => p,
        Err(e) => {
            // Unexpected query error on a freshly loaded DB — the query itself
            // failed, not "a dangling link target was found". It gets its own
            // InternalQueryError kind: not the sibling
            // `check_malformed_property_rows`'s MalformedPropertyRow, and not
            // the misleading Warning-severity UnknownLinkTarget with empty ids.
            push_internal_query_error(
                report,
                "link target existence",
                format!("could not check link target existence: {e}"),
            );
            return;
        }
    };

    for (source_id, target_id) in pairs {
        report.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::UnknownLinkTarget { source_id: source_id.clone(), target_id: target_id.clone() },
            message: format!("resource '{source_id}' has a link to unknown resource '{target_id}'"),
            span: None,
        });
    }
}

/// Data-aware existence check for declared `labels {}` entries: a property or
/// class label naming a property/class absent from the project's data is
/// advisory (typo-catching), not a hard-fail — the label is simply unused.
/// Appends `UnknownLabel` (Warning) diagnostics; never short-circuits.
fn check_labels_data(config: &ValidatedConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for pl in &config.site.property_labels {
        check_property_label_exists(&pl.property, data, report);
    }
    for cl in &config.site.class_labels {
        check_class_label_exists(&cl.class, data, report);
    }
}

/// Existence check for one declared `labels { property "…" }` key. Shared by
/// `check_labels_data` (success path) and `check_labels_data_into_raw`.
fn check_property_label_exists(key: &str, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    match data.property_exists_anywhere(key) {
        Ok(true) => {}
        Ok(false) => report.push(Diagnostic {
            kind: DiagnosticKind::UnknownLabel { target: LabelTarget::Property, key: key.to_owned() },
            message: format!("property label '{key}' names a property absent from the project data"),
            span: None,
            severity: Severity::Warning,
        }),
        Err(e) => push_internal_query_error(
            report,
            "label property existence",
            format!("could not check property existence for label '{key}': {e}"),
        ),
    }
}

/// Existence check for one declared `labels { class "…" }` key. Shared by
/// `check_labels_data` (success path) and `check_labels_data_into_raw`.
fn check_class_label_exists(key: &str, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    match data.class_exists(key) {
        Ok(true) => {}
        Ok(false) => report.push(Diagnostic {
            kind: DiagnosticKind::UnknownLabel { target: LabelTarget::Class, key: key.to_owned() },
            message: format!("class label '{key}' names a class absent from the project data"),
            span: None,
            severity: Severity::Warning,
        }),
        Err(e) => push_internal_query_error(
            report,
            "label class existence",
            format!("could not check class existence for label '{key}': {e}"),
        ),
    }
}

/// Raw-fallback sibling of `check_labels_data`. `raw.labels` is a top-level
/// field, populated even when `check_and_lower_site` failed and produced no
/// `SiteConfig` — which is what makes this sibling possible at all.
///
/// An entry with no `key` is skipped silently (lowering already reported the
/// missing key). Repeated keys emit one diagnostic each, exactly as the
/// success path does (the lowered `Vec`s keep both entries) — deliberately no
/// dedupe here, since that would make the two arms diverge.
fn check_labels_data_into_raw(raw: &RawConfig, data: &dyn ProjectFacts, report: &mut ValidationReport) {
    for label in &raw.labels {
        let Some(key) = label.key.as_deref() else {
            continue;
        };
        match label.target {
            crate::config::RawLabelTarget::Property => check_property_label_exists(key, data, report),
            crate::config::RawLabelTarget::Class => check_class_label_exists(key, data, report),
        }
    }
}

/// Success-arm data checks for every lowered per-project component, each
/// through its `project_components` row for `project_id` (dsp-incubator
/// cpe/docs/adr/0011-per-project-components.md). Lowering only produces a per-project variant for a
/// project it is registered for, so the `(project, name)` lookup always hits; a miss is
/// skipped rather than panicking.
fn check_project_component_data(
    config: &ValidatedConfig,
    project_id: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for page in &config.pages {
        for component in &page.body {
            let Some(row) = component
                .registry_name()
                .and_then(|name| project_components::lookup(project_id, name))
            else {
                continue;
            };
            (row.check_data)(component, &page.route, data, report);
        }
    }
}

/// Raw sibling of `check_project_component_data`, for the `validate_with_data`
/// failure arm. Filters on `(project, name)`: a node whose name is not
/// registered for `project_id` is skipped, so a project declaring a component
/// registered only for another project gets exactly the `UnknownComponent`
/// lowering reported — never a second, data-level report for the same mistake.
fn check_project_component_data_into_raw(
    raw: &RawConfig,
    project_id: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for raw_page in &raw.pages {
        let route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if let Some(row) = project_components::lookup(project_id, &raw_comp.name) {
                (row.check_data_into_raw)(raw_comp, route, data, report);
            }
        }
    }
}

/// Ids the engine renders on its own elements, and the prefixes of the ids it
/// derives (`annotation-<id>`, `crumb-<region>`, …). A narrative heading id
/// equal to one, or starting with a prefix, would duplicate an id on some
/// page (`InvalidHeadingId`). Built from the engine's `id=` / `#id` markup;
/// a new engine-owned id or id prefix belongs here too. The literals that
/// spell a `component::dom` constant are pinned to it by
/// `component::dom`'s `reserved_heading_ids_hold_the_dom_ids` test.
#[doc(hidden)]
pub const RESERVED_HEADING_IDS: &[&str] = &[
    "main",
    "q",
    "compound-viewer",
    "cv-picker",
    "cv-annotations",
    "cv-title",
    "cv-status",
    "annotations",
    "anno-cap",
    "crumb-data",
    "record-figure",
    "orderctl",
    "orderctl-status",
    "browse-count",
    "viewtoggle-lbl",
    crate::markup_contract::RAIL_CAPTION_ID,
    crate::markup_contract::HEADER_CONTROLS_ID,
    crate::markup_contract::HEADER_MENU_ID,
];
#[doc(hidden)]
pub const RESERVED_HEADING_ID_PREFIXES: &[&str] = &[
    "annotation-",
    "case-",
    "crumb-",
    "share-",
    "filters-",
    "facets-",
    "cv-pk-",
    "rv-",
    "sec-",
];

/// The present variants of narrative `file` in `narratives`: the base file
/// and every `<stem>.<lang>.<ext>`, as `(name, content)`.
fn narrative_variants<'a>(file: &str, narratives: &'a [(String, String)]) -> Vec<(&'a str, &'a str)> {
    let (stem, ext) = split_stem_ext(file);
    narratives
        .iter()
        .filter(|(name, _)| {
            name == file
                || (!ext.is_empty()
                    && name
                        .strip_prefix(&format!("{stem}."))
                        .and_then(|rest| rest.strip_suffix(&format!(".{ext}")))
                        .is_some_and(|lang| !lang.is_empty() && !lang.contains('.')))
        })
        .map(|(name, content)| (name.as_str(), content.as_str()))
        .collect()
}

/// `InvalidHeadingId` for every heading id of every present variant of
/// narrative `file`: malformed, repeated within the variant, or reserved.
fn check_single_heading_ids(file: &str, narratives: &[(String, String)], report: &mut ValidationReport) {
    for (name, content) in narrative_variants(file, narratives) {
        let mut seen: Vec<String> = Vec::new();
        for (_, id) in crate::util::heading_ids(content) {
            let well_formed = !id.is_empty()
                && id
                    .split('-')
                    .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
            let problem = if !well_formed {
                Some("is not lowercase ASCII letters and digits joined by '-'")
            } else if seen.contains(&id) {
                Some("is used by an earlier heading of the same file")
            } else if RESERVED_HEADING_IDS.contains(&id.as_str())
                || RESERVED_HEADING_ID_PREFIXES.iter().any(|p| id.starts_with(p))
            {
                Some("collides with an id the engine renders itself")
            } else {
                None
            };
            if let Some(problem) = problem {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::InvalidHeadingId { id: id.clone(), file: name.to_owned() },
                    format!("narrative '{name}': heading id '{id}' {problem}"),
                ));
            }
            seen.push(id);
        }
    }
}

/// `UnknownFigureSection` / `FigureSectionHoldsInviteList` for one slot's
/// `section` against every present variant of the narrative before it: the id
/// must be a top-level `##` that figure mode splits at
/// (`narrative_outline::section_headings`, the renderer's own reading).
fn check_single_figure_section(
    section: &str,
    file: &str,
    route: &str,
    narratives: &[(String, String)],
    report: &mut ValidationReport,
) {
    for (name, content) in narrative_variants(file, narratives) {
        let headings = crate::narrative_outline::section_headings(content);
        match headings.iter().find(|h| h.id == section) {
            Some(h) if h.holds_invite_list => report.push(Diagnostic::error_spanless(
                DiagnosticKind::FigureSectionHoldsInviteList {
                    section: section.to_owned(),
                    file: name.to_owned(),
                    page: route.to_owned(),
                },
                format!(
                    "page '{route}': section=\"{section}\" names a heading of '{name}' that an \
                     invite list follows; an invite section is not split into figure parts, so \
                     a figure block cannot attach there"
                ),
            )),
            Some(_) => {}
            None => {
                let elsewhere = crate::util::heading_ids(content)
                    .into_iter()
                    .find(|(_, id)| id == section)
                    .map(|(level, _)| {
                        let place = if level == pulldown_cmark::HeadingLevel::H2 {
                            "a '##' nested in a quote or list".to_owned()
                        } else {
                            format!("an {level} heading")
                        };
                        format!(
                            "; it sits on {place}, and only a top-level '##' starts a figure \
                             section"
                        )
                    })
                    .unwrap_or_default();
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::UnknownFigureSection {
                        section: section.to_owned(),
                        file: name.to_owned(),
                        page: route.to_owned(),
                    },
                    format!(
                        "page '{route}': section=\"{section}\" is not the id of a '##' heading in \
                         '{name}'{elsewhere}"
                    ),
                ));
            }
        }
    }
}

/// `DuplicateHeadingIdOnPage` for one page: a heading id carried by two of
/// its narratives in the content one visitor is served. `files` are the
/// page's narrative files in body order, NOT deduplicated (the same file
/// twice collides with itself).
///
/// The comparison is per declared language, over the content the renderer's
/// fallback resolves for it (`<stem>.<lang>.<ext>`, else the default-language
/// variant, else the base file), so a base-only file collides with another
/// file's variant. With no declared language (a raw config whose `languages`
/// failed) only the base files are compared. One diagnostic per id.
fn check_page_heading_ids_unique(
    route: &str,
    files: &[&str],
    languages: &[&str],
    default_language: Option<&str>,
    narratives: &[(String, String)],
    report: &mut ValidationReport,
) {
    let content_of = |name: &str| narratives.iter().find(|(n, _)| n == name).map(|(_, content)| content.as_str());
    // The rungs of `component::narrative::resolve_narrative_content`, in its
    // order and with its variant naming; that function reads a
    // `NarrativeStore`, which boot does not have yet. Change both together.
    let resolve = |file: &str, lang: Option<&str>| {
        let variant = |l: &str| crate::util::variant_filename(file, l).and_then(|name| content_of(&name));
        lang.and_then(variant)
            .or_else(|| default_language.and_then(variant))
            .or_else(|| content_of(file))
    };
    let passes: Vec<Option<&str>> = if languages.is_empty() {
        vec![None]
    } else {
        languages.iter().copied().map(Some).collect()
    };

    // Each shared id once, with the two files it was first seen in.
    let mut collisions: Vec<(String, &str, &str)> = Vec::new();
    for lang in passes {
        let mut seen: Vec<(String, &str)> = Vec::new();
        for file in files {
            let mut ids: Vec<String> = resolve(file, lang)
                .map(crate::util::heading_ids)
                .unwrap_or_default()
                .into_iter()
                .map(|(_, id)| id)
                .collect();
            // A repeat inside one file is `InvalidHeadingId`'s to report.
            ids.sort();
            ids.dedup();
            for id in &ids {
                if collisions.iter().any(|(known_id, ..)| known_id == id) {
                    continue;
                }
                if let Some((_, earlier)) = seen.iter().find(|(seen_id, _)| seen_id == id) {
                    collisions.push((id.clone(), earlier, file));
                }
            }
            seen.extend(ids.into_iter().map(|id| (id, *file)));
        }
    }
    for (id, earlier, file) in collisions {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::DuplicateHeadingIdOnPage { id: id.clone(), page: route.to_owned() },
            format!(
                "page '{route}': heading id '{id}' is carried by narrative '{earlier}' and again \
                 by '{file}'; an id must be unique on a page"
            ),
        ));
    }
}

/// The narrative-content checks on a lowered config: `InvalidHeadingId` once
/// per narrative file a `narrative` component names, each figure-block
/// slot's section against the nearest `narrative` before it, and
/// `DuplicateHeadingIdOnPage` per page.
fn check_narrative_headings(config: &ValidatedConfig, narratives: &[(String, String)], report: &mut ValidationReport) {
    let languages: Vec<&str> = config.site.languages.iter().map(String::as_str).collect();
    let mut files: Vec<&str> = Vec::new();
    let mut pages: Vec<(&str, Vec<&str>)> = Vec::new();
    for page in &config.pages {
        let mut narrative: Option<&str> = None;
        let mut page_files: Vec<&str> = Vec::new();
        for (i, component) in page.body.iter().enumerate() {
            if let Component::Narrative { file, .. } = component {
                narrative = Some(file);
                page_files.push(file);
                if !files.contains(&file.as_str()) {
                    files.push(file);
                }
            }
            if let (Some(slot), Some(file)) = (page.figure_slot(i), narrative) {
                check_single_figure_section(&slot.section, file, &page.route, narratives, report);
            }
        }
        pages.push((&page.route, page_files));
    }
    for file in files {
        check_single_heading_ids(file, narratives, report);
    }
    for (route, page_files) in &pages {
        check_page_heading_ids_unique(
            route,
            page_files,
            &languages,
            Some(&config.site.default_language),
            narratives,
            report,
        );
    }
}

/// Raw sibling of `check_narrative_headings`, for the failure arm: reads
/// `file=` and `section=` with the silent `get_string_attr`, and a `section=`
/// only on a component registered for `project_id`, the components lowering
/// reads a slot from.
fn check_narrative_headings_into_raw(
    raw: &RawConfig,
    project_id: &str,
    narratives: &[(String, String)],
    report: &mut ValidationReport,
) {
    // Declared languages, read silently: lowering reports a wrong-typed one.
    let languages: Vec<&str> = raw
        .languages
        .iter()
        .filter_map(|(value, _)| match value {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    let default_language = raw.default_language.as_deref().or_else(|| languages.first().copied());

    let mut files: Vec<String> = Vec::new();
    let mut pages: Vec<(&str, Vec<String>)> = Vec::new();
    for raw_page in &raw.pages {
        let route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        let mut narrative: Option<String> = None;
        let mut page_files: Vec<String> = Vec::new();
        for raw_comp in &raw_page.body {
            if raw_comp.name == "narrative" {
                narrative = get_string_attr(raw_comp, "file");
                if let Some(file) = &narrative {
                    page_files.push(file.clone());
                    if !files.contains(file) {
                        files.push(file.clone());
                    }
                }
                continue;
            }
            if project_components::lookup(project_id, &raw_comp.name).is_none() {
                continue;
            }
            if let (Some(section), Some(file)) = (get_string_attr(raw_comp, "section"), &narrative) {
                check_single_figure_section(&section, file, route, narratives, report);
            }
        }
        pages.push((route, page_files));
    }
    for file in &files {
        check_single_heading_ids(file, narratives, report);
    }
    for (route, page_files) in &pages {
        let page_files: Vec<&str> = page_files.iter().map(String::as_str).collect();
        check_page_heading_ids_unique(route, &page_files, &languages, default_language, narratives, report);
    }
}

/// The property names one `resource-detail` puts on the page: its
/// `show-properties` list, every `group` member and header-slot property
/// (flattened by the caller, from the lowered component or from the raw
/// children), plus its `title-property`. Shared by the literal-id and the parameterised
/// sparse checks so the two can never disagree about which names are covered.
fn detail_property_names<'a>(
    show_properties: &'a Option<Vec<String>>,
    group_members: &'a [String],
    title_property: &'a Option<String>,
) -> Vec<&'a str> {
    let mut names: Vec<&str> = Vec::new();
    if let Some(props) = show_properties {
        for p in props {
            names.push(p.as_str());
        }
    }
    for p in group_members {
        names.push(p.as_str());
    }
    if let Some(tp) = title_property {
        names.push(tp.as_str());
    }
    names
}

/// The sparse-`show-properties` check for a **parameterised**
/// `resource-detail` (`/books/:id`), computed over the component's declared
/// `class` instead of over one resource's class.
///
/// A `$param` id names no single resource, so neither `UnknownResource` nor
/// the id-keyed `properties_present_on_class_of` lookup applies — but the
/// declared `class` is exactly as good an input for the sparse half, and it is
/// the common case (Incunabula's, HdM's and Alice's detail pages are all
/// parameterised).
///
/// Same `SparseProperty` semantics as the literal-id path: a property absent
/// from **every** resource of the class warns; a merely uneven one does not.
/// A class with no property rows at all (an undeclared or not-yet-translated
/// class) is skipped rather than warning once per listed property — the class
/// itself is another check's business.
fn check_resource_detail_properties_on_class(
    class: &str,
    show_properties: &Option<Vec<String>>,
    group_members: &[String],
    title_property: &Option<String>,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let names_to_check = detail_property_names(show_properties, group_members, title_property);
    if names_to_check.is_empty() {
        return;
    }

    let present = match data.properties_present_on_class(class) {
        Ok(set) => set,
        Err(e) => {
            push_internal_query_error(
                report,
                "resource-detail class properties",
                format!("resource-detail: could not fetch properties for class '{class}': {e}"),
            );
            return;
        }
    };

    if present.is_empty() {
        return;
    }

    for name in names_to_check {
        if !present.contains(name) {
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseProperty,
                message: format!(
                    "resource-detail property '{name}' is not present on any resource \
                     of class '{class}'"
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }
}

/// Perform the UnknownResource + SparseProperty checks for one literal-id
/// `resource-detail`. Appends to `report` on findings.
fn check_single_resource_detail(
    id: &str,
    show_properties: &Option<Vec<String>>,
    group_members: &[String],
    title_property: &Option<String>,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // Check 1: resource must exist (hard-fail).
    let exists = match data.resource_exists(id) {
        Ok(b) => b,
        Err(e) => {
            push_internal_query_error(
                report,
                "resource-detail resource existence",
                format!("resource-detail: could not check resource existence for id '{id}': {e}"),
            );
            return;
        }
    };

    if !exists {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownResource,
            message: format!("resource-detail references literal id '{id}' which does not exist in the data"),
            span: None,
            severity: Severity::Error,
        });
        // No point checking property names when the resource is absent.
        return;
    }

    // Check 2: SparseProperty (warn) — only when properties to check are given.
    let names_to_check = detail_property_names(show_properties, group_members, title_property);
    if names_to_check.is_empty() {
        return;
    }

    let present = match data.properties_present_on_class_of(id) {
        Ok(set) => set,
        Err(e) => {
            push_internal_query_error(
                report,
                "resource-detail properties",
                format!("resource-detail: could not fetch properties for class of '{id}': {e}"),
            );
            return;
        }
    };

    for name in names_to_check {
        if !present.contains(name) {
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseProperty,
                message: format!(
                    "resource-detail property '{name}' is not present on any resource \
                     of the same class as '{id}'"
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }
}

// ── Project-id checks ─────────────────────────────────────────────────────────

fn check_project_id(project_id: &str, reserved_ids: &[&str], report: &mut ValidationReport) {
    // ReservedProjectId — span-less (id is the directory name, not in config.kdl).
    if reserved_ids.contains(&project_id) {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::ReservedProjectId,
            format!("project id '{project_id}' is reserved"),
        ));
    }

    // InvalidProjectId — must match ^[a-z0-9][a-z0-9-]*$
    if !is_valid_project_id(project_id) {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::InvalidProjectId,
            format!(
                "project id '{project_id}' is not valid kebab-case \
                 (must match ^[a-z0-9][a-z0-9-]*$)"
            ),
        ));
    }
}

/// Whether `id` can be a project id: `^[a-z0-9][a-z0-9-]*$`. A host applies it
/// to a folder name before it takes the folder as a project, since the id is
/// also a URL segment.
pub fn is_valid_project_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let mut chars = id.chars();
    let first = chars.next().expect("id is non-empty — checked above");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

// ── Top-level node checks ─────────────────────────────────────────────────────

fn check_unexpected_top_level(raw: &RawConfig, report: &mut ValidationReport) {
    for (name, span) in &raw.unknown_top_level {
        report.push(unexpected_top_level(name, *span));
    }
}

fn unexpected_top_level(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        DiagnosticKind::UnexpectedTopLevelNode,
        format!(
            "unexpected top-level node '{name}'; only 'site', 'labels', 'page' and 'remodel' are allowed (each but 'page' once)"
        ),
        span,
    )
}

// ── Site checks ───────────────────────────────────────────────────────────────

/// Validate and lower the site metadata. Returns `Some(SiteConfig)` when
/// no errors were found; `None` when at least one error was pushed.
fn check_and_lower_site(
    raw: &RawConfig,
    ctx: &ValidationContext<'_>,
    report: &mut ValidationReport,
) -> Option<SiteConfig> {
    let error_count_before = error_count(report);

    // ── title ────────────────────────────────────────────────────────────────
    if raw.title.is_none() {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::MissingSiteTitle,
            "site block is missing a 'title' child",
        ));
    }

    // ── languages ─────────────────────────────────────────────────────────────
    // Check for absent or empty languages list first.
    let mut valid_languages: Vec<String> = Vec::new();

    if raw.languages.is_empty() {
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::MissingSiteLanguages,
            "site block is missing a 'languages' child or the list is empty",
        ));
    } else {
        // Check each entry's type. Collect only string entries as valid.
        for (value, span) in &raw.languages {
            match value {
                Value::String(s) => {
                    valid_languages.push(s.clone());
                }
                other => {
                    let type_name = value_type_name(other);
                    report.push(Diagnostic::error(
                        DiagnosticKind::InvalidFieldType,
                        format!("languages entries must be strings, got {type_name}"),
                        *span,
                    ));
                }
            }
        }

        // MissingRequiredLanguage — at least "de" or "en" must be present.
        if !valid_languages.is_empty() {
            let has_de = valid_languages.iter().any(|l| l == "de");
            let has_en = valid_languages.iter().any(|l| l == "en");
            if !has_de {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::MissingRequiredLanguage,
                    "languages list must include 'de'",
                ));
            }
            if !has_en {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::MissingRequiredLanguage,
                    "languages list must include 'en'",
                ));
            }
        }
    }

    // ── default-language ──────────────────────────────────────────────────────
    let default_language: Option<String> = if let Some(dl) = &raw.default_language {
        if !valid_languages.is_empty() && !valid_languages.contains(dl) {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::UnknownDefaultLanguage,
                format!("default-language '{dl}' is not in the declared languages list"),
            ));
            None
        } else {
            Some(dl.clone())
        }
    } else {
        None
    };

    // ── chrome ────────────────────────────────────────────────────────────────
    let mut validated_chrome: Vec<ChromeString> = Vec::new();
    for cs in &raw.chrome {
        let id = match &cs.id {
            Some(id) => id.clone(),
            None => {
                // No id: skip lowering but don't add an error here (chrome id
                // validation is not this check's scope).
                continue;
            }
        };

        // IncompleteChromeTranslation — check against valid_languages only.
        if !valid_languages.is_empty() {
            for lang in &valid_languages {
                let has_translation = cs.translations.iter().any(|(l, _)| l == lang);
                if !has_translation {
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::IncompleteChromeTranslation,
                        format!("chrome string '{id}' is missing a translation for language '{lang}'"),
                    ));
                }
            }
        }

        let translations: Vec<(String, String)> = cs.translations.clone();
        validated_chrome.push(ChromeString { id, translations });
    }

    // ── labels ────────────────────────────────────────────────────────────────
    let mut validated_property_labels: Vec<PropertyLabel> = Vec::new();
    let mut validated_class_labels: Vec<ClassLabel> = Vec::new();
    for rl in &raw.labels {
        let key = match &rl.key {
            Some(key) => key.clone(),
            None => {
                // No key: skip lowering but don't add an error here (mirrors
                // the chrome id skip above).
                continue;
            }
        };

        // IncompleteLabelTranslation — check against valid_languages only.
        let (target, kind_word) = match rl.target {
            RawLabelTarget::Property => (LabelTarget::Property, "property"),
            RawLabelTarget::Class => (LabelTarget::Class, "class"),
        };
        if !valid_languages.is_empty() {
            for lang in &valid_languages {
                let has_translation = rl.translations.iter().any(|(l, _)| l == lang);
                if !has_translation {
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::IncompleteLabelTranslation {
                            target: target.clone(),
                            key: key.clone(),
                            lang: lang.clone(),
                        },
                        format!("{kind_word} label '{key}' is missing a translation for language '{lang}'"),
                    ));
                }
            }
        }

        // IncompleteLabelTranslation (plural). Same completeness rule as the
        // singular translations above, applied to `rl.plural`, but ONLY when a
        // `plural` block is present at all (`rl.plural.is_empty()` ⇒
        // undeclared, not a violation — skip the check entirely) and ONLY for
        // `class` entries (properties have no plural concept; `rl.plural` is
        // simply never read for the `Property` arm). Reuses
        // `IncompleteLabelTranslation` rather than a new `DiagnosticKind` — the
        // message text says "PLURAL" so an operator can tell which block
        // failed.
        if rl.target == RawLabelTarget::Class && !rl.plural.is_empty() && !valid_languages.is_empty() {
            for lang in &valid_languages {
                let has_translation = rl.plural.iter().any(|(l, _)| l == lang);
                if !has_translation {
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::IncompleteLabelTranslation {
                            target: target.clone(),
                            key: key.clone(),
                            lang: lang.clone(),
                        },
                        format!("class label '{key}' is missing a PLURAL translation for language '{lang}'"),
                    ));
                }
            }
        }

        match rl.target {
            RawLabelTarget::Property => {
                validated_property_labels.push(PropertyLabel { property: key, translations: rl.translations.clone() })
            }
            RawLabelTarget::Class => validated_class_labels.push(ClassLabel {
                class: key,
                translations: rl.translations.clone(),
                plural: rl.plural.clone(),
            }),
        }
    }

    // ── theme ─────────────────────────────────────────────────────────────────
    // Lower raw key/value pairs into a `ThemeKnob`. Unknown keys or values
    // push `UnknownThemeValue` diagnostics without short-circuiting — the full
    // theme block is always processed so all errors are reported at once.
    let mut theme_accent: Option<Accent> = None;
    let mut theme_background: Option<Background> = None;
    let mut theme_typeface: Option<Typeface> = None;

    for tv in &raw.theme {
        match &tv.value {
            Some((Value::String(s), _span)) => match tv.name.as_str() {
                "accent" => match Accent::parse_config_str(s) {
                    Some(a) => theme_accent = Some(a),
                    None => {
                        report.push(Diagnostic::error_spanless(
                            DiagnosticKind::UnknownThemeValue {
                                key: "accent".to_owned(),
                                value: s.clone(),
                                allowed: vec![
                                    "ochre".to_owned(),
                                    "green".to_owned(),
                                    "teal".to_owned(),
                                    "red".to_owned(),
                                ],
                            },
                            format!(
                                "theme: unknown accent value '{s}'; \
                                         allowed: ochre, green, teal, red"
                            ),
                        ));
                    }
                },
                "background" => match Background::parse_config_str(s) {
                    Some(b) => theme_background = Some(b),
                    None => {
                        report.push(Diagnostic::error_spanless(
                            DiagnosticKind::UnknownThemeValue {
                                key: "background".to_owned(),
                                value: s.clone(),
                                allowed: vec!["neutral".to_owned(), "tint".to_owned()],
                            },
                            format!(
                                "theme: unknown background value '{s}'; \
                                         allowed: neutral, tint"
                            ),
                        ));
                    }
                },
                "typeface" => match Typeface::parse_config_str(s) {
                    Some(t) => theme_typeface = Some(t),
                    None => {
                        let allowed: Vec<String> =
                            Typeface::all().iter().map(|t| t.as_data_attr().to_owned()).collect();
                        let allowed_list = allowed.join(", ");
                        report.push(Diagnostic::error_spanless(
                            DiagnosticKind::UnknownThemeValue { key: "typeface".to_owned(), value: s.clone(), allowed },
                            format!("theme: unknown typeface value '{s}'; allowed: {allowed_list}"),
                        ));
                    }
                },
                other_key => {
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::UnknownThemeValue {
                            key: other_key.to_owned(),
                            value: s.clone(),
                            allowed: vec![],
                        },
                        format!(
                            "theme: unknown key '{other_key}'; \
                                 recognised keys: accent, background, typeface"
                        ),
                    ));
                }
            },
            Some((other, span)) => {
                let type_name = value_type_name(other);
                report.push(Diagnostic::error(
                    DiagnosticKind::InvalidFieldType,
                    format!("theme value for '{}' must be a string, got {type_name}", tv.name),
                    *span,
                ));
            }
            None => {
                // No value — theme var with no value is unusual but
                // treated as an unknown-key error since the key alone
                // carries no useful information.
            }
        }
    }

    let validated_theme = ThemeKnob {
        accent: theme_accent.unwrap_or_default(),
        background: theme_background.unwrap_or_default(),
        typeface: theme_typeface.unwrap_or_default(),
    };

    // ── nav ───────────────────────────────────────────────────────────────────
    let validated_nav: Vec<NavItem> = raw
        .nav
        .iter()
        .filter_map(|ni| {
            let label = ni.label.clone()?;
            let route = ni.route.clone()?;
            Some(NavItem { label, route })
        })
        .collect();

    // ── nav label / chrome-id "collision": deliberately NOT a diagnostic ──────
    // A nav item's `label` is "chrome-string id OR literal" by design, and
    // `labels::nav_label` resolves the id first. A label that matches a
    // chrome-string id the project ITSELF declares is therefore the intended
    // bilingual-nav idiom, not an accident — every nav item in all three
    // shipped projects is written that way. A `NavLabelChromeIdCollision`
    // Warning on that idiom cannot distinguish it from an accident, because the
    // only ids it can compare against are the project's own declared ones: it
    // is pure boot-log noise. Don't add one without a set of ids the project
    // did NOT declare to compare against.

    // ── cover-image ───────────────────────────────────────────────────────────
    if let Some(path) = &raw.cover_image {
        // InvalidCoverImagePath — bare relative path only: no ".." segment,
        // no leading '/', and no empty or dot-prefixed segment. This MUST
        // match the segment-rejection rules the runtime route
        // (`serve_project_asset` in `dsp-incubator cpe/bin/src/router.rs`)
        // independently enforces — otherwise a path can boot-validate
        // successfully yet
        // 404 at request time (e.g. "assets/.cover.webp" or
        // "assets//cover.webp").
        let has_leading_slash = path.starts_with('/');
        let has_bad_segment = path.split('/').any(|seg| seg == ".." || seg.is_empty() || seg.starts_with('.'));
        if has_leading_slash || has_bad_segment {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::InvalidCoverImagePath,
                format!(
                    "cover-image path '{path}' must be a plain relative path (no '..', \
                     no leading '/', and no empty or dot-prefixed segment)"
                ),
            ));
        } else if !ctx.asset_files.iter().any(|f| f == path) {
            // CoverImageNotFound — `ctx.asset_files` is the project's
            // COMPLETE `assets/` inventory, unconditionally. An empty
            // inventory therefore means "this project has no asset files"
            // (no `assets/` dir, or an empty one — `DiscoveredProject::
            // asset_files()` returns `Vec::new()` for both, and in both
            // the truthful answer to "does this file exist?" is no), so a
            // declared `cover-image` is missing and must hard-fail.
            //
            // This DELIBERATELY diverges from the `narrative_files`
            // empty-slice guard below: that inventory is genuinely
            // optional ("caller found no narrative/ dir ⇒ don't
            // constrain"), whereas failing open here would let a project
            // declaring `site.cover-image` with no `assets/` directory
            // boot green and serve a broken <img> on the site index.
            // Don't "restore consistency" by reintroducing an
            // `is_empty()` guard.
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::CoverImageNotFound,
                format!("cover-image path '{path}' not found in the project's assets/ directory"),
            ));
        }
    }

    // ── date-precision-word ───────────────────────────────────────────────────
    // `site { date-precision-word #true }`. Absent (or a bare node with no
    // argument) ⇒ `false`. A non-boolean argument is `InvalidFieldType` — the
    // kind for a wrong-typed `site {}` child (the `languages` precedent) —
    // never a silent `false`.
    let date_precision_word = match &raw.date_precision_word {
        None => false,
        Some((Value::Bool(b), _)) => *b,
        Some((other, span)) => {
            let type_name = value_type_name(other);
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidFieldType,
                format!("date-precision-word must be a boolean (#true / #false), got {type_name}"),
                *span,
            ));
            false
        }
    };

    // ── footer-attribution ─────────────────────────────────────────────────────
    // Copy override: a chrome-id-or-literal string (the `description`
    // precedent), resolved at render time; a non-string argument is
    // `InvalidFieldType`, never a silent absence. Absent ⇒ `None` ⇒ the
    // engine's own copy.
    let footer_attribution = lower_site_string(&raw.footer_attribution, "footer-attribution", report);

    // ── narrative-companion / narrative-side / text-align ─────────────────────
    let narrative_companion = lower_site_choice(
        &raw.narrative_companion,
        "narrative-companion",
        &NarrativeCompanion::ALLOWED,
        NarrativeCompanion::parse_config_str,
        report,
    );
    let narrative_side = lower_site_choice(
        &raw.narrative_side,
        "narrative-side",
        &NarrativeSide::ALLOWED,
        NarrativeSide::parse_config_str,
        report,
    );
    let text_align = lower_site_choice(
        &raw.text_align,
        "text-align",
        &TextAlign::ALLOWED,
        TextAlign::parse_config_str,
        report,
    );

    // ── not-found { browse } ──────────────────────────────────────────────────
    // Route existence is checked in `validate_inner`
    // beside `check_nav_routes` (it needs the lowered page routes); the
    // cap of three is checked here.
    if raw.not_found_browse.len() > NOT_FOUND_BROWSE_MAX {
        let message = format!(
            "not-found declares {} browse links; at most {NOT_FOUND_BROWSE_MAX} are allowed",
            raw.not_found_browse.len()
        );
        report.push(match raw.not_found_span {
            Some(span) => Diagnostic::error(DiagnosticKind::TooManyNotFoundBrowseLinks, message, span),
            None => Diagnostic::error_spanless(DiagnosticKind::TooManyNotFoundBrowseLinks, message),
        });
    }
    let not_found_browse: Vec<NotFoundBrowse> = raw
        .not_found_browse
        .iter()
        .filter_map(|c| {
            // Reporting readers: a missing/wrong-typed `route` and a
            // wrong-typed `label` each push a diagnostic, never a silent drop.
            let route = require_string_attr(c, "route", report);
            let label = get_typed_string_attr(c, "label", report);
            Some(NotFoundBrowse { route: route?, label })
        })
        .collect();

    // If we introduced any new errors, we cannot safely lower.
    if error_count(report) > error_count_before {
        return None;
    }

    let title = raw
        .title
        .clone()
        .expect("title is Some — we checked above and returned None on error");
    let resolved_default = default_language
        .or_else(|| valid_languages.first().cloned())
        .expect("valid_languages is non-empty — checked above");

    Some(SiteConfig {
        title,
        description: raw.description.clone(),
        cover_image: raw.cover_image.clone(),
        image_credit: raw.image_credit.clone(),
        eyebrow: raw.eyebrow.clone(),
        languages: valid_languages,
        default_language: resolved_default,
        nav: validated_nav,
        chrome: validated_chrome,
        property_labels: validated_property_labels,
        class_labels: validated_class_labels,
        theme: validated_theme, // ThemeKnob (default when theme block absent)
        date_precision_word,
        footer_attribution,
        not_found_browse,
        narrative_companion,
        narrative_side,
        text_align,
    })
}

/// The cap on `site { not-found { browse } }` entries: at most three,
/// never the whole nav.
const NOT_FOUND_BROWSE_MAX: usize = 3;

/// Lower an optional string-valued `site {}` child captured raw by
/// `config::walk_site`: absent ⇒ `None`; a string ⇒ `Some`; any other KDL
/// value ⇒ `InvalidFieldType` (span-located) and `None`.
fn lower_site_string(raw: &Option<(Value, Span)>, name: &str, report: &mut ValidationReport) -> Option<String> {
    match raw {
        None => None,
        Some((Value::String(s), _)) => Some(s.clone()),
        Some((other, span)) => {
            let type_name = value_type_name(other);
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidFieldType,
                format!("{name} must be a string, got {type_name}"),
                *span,
            ));
            None
        }
    }
}

/// Lower a closed-vocabulary `site {}` child: absent ⇒ the default; a
/// non-string ⇒ `InvalidFieldType`; a string outside `allowed` ⇒
/// `UnknownSiteValue`. Either error also yields the default, so lowering
/// goes on and reports the rest.
fn lower_site_choice<T: Default>(
    raw: &Option<(Value, Span)>,
    name: &str,
    allowed: &[&str],
    parse: fn(&str) -> Option<T>,
    report: &mut ValidationReport,
) -> T {
    let Some((_, span)) = raw else {
        return T::default();
    };
    let Some(value) = lower_site_string(raw, name, report) else {
        return T::default();
    };
    parse(&value).unwrap_or_else(|| {
        report.push(Diagnostic::error(
            DiagnosticKind::UnknownSiteValue {
                key: name.to_owned(),
                value: value.clone(),
                allowed: allowed.iter().map(|a| (*a).to_owned()).collect(),
            },
            format!("site: unknown {name} value '{value}'; allowed: {}", allowed.join(", ")),
            *span,
        ));
        T::default()
    })
}

// ── Page checks ───────────────────────────────────────────────────────────────

/// Validate pages (id/route uniqueness) and lower to the typed `Page` IR.
/// Page bodies are lowered via `lower_page_body`.
///
/// `default_language` — the project's resolved default language (`None` only
/// when site lowering itself failed); threaded through to `lower_page_body`
/// for the narrative existence probe.
fn check_and_lower_pages(
    raw: &RawConfig,
    ctx: &ValidationContext<'_>,
    default_language: Option<&str>,
    report: &mut ValidationReport,
) -> Vec<Page> {
    // Collect ids and routes for duplicate detection. No span stored — the
    // page node span is not on RawPage, so duplicates use span-less diagnostics
    // (consistent with ReservedProjectId).
    let mut seen_ids: Vec<String> = Vec::new();
    let mut seen_routes: Vec<String> = Vec::new();
    let mut pages: Vec<Page> = Vec::new();

    // Collect all resource-list ids across the entire project for
    // DuplicateResourceListId detection.
    let mut seen_list_ids: Vec<String> = Vec::new();

    for raw_page in &raw.pages {
        let id = match &raw_page.id {
            Some(id) => id.clone(),
            None => {
                // Missing id — not checked here; the page is skipped.
                continue;
            }
        };
        let route = match &raw_page.route {
            Some(r) => r.clone(),
            None => {
                // Missing route — not checked here.
                continue;
            }
        };

        // ReservedRoute — the engine reserves `_`-prefixed route segments for
        // engine-registered routes (`_static`, `_assets`, the `_<component>`
        // fragment segments). Those routes are registered by the host and never
        // appear in config, so this only guards config-declared routes.
        if route_first_segment_is_reserved(&route) {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::ReservedRoute,
                format!(
                    "page route '{route}' uses a reserved leading-underscore \
                     segment; the '_'-prefixed route namespace is reserved for \
                     the engine (e.g. _static, _assets)"
                ),
            ));
        }

        // DuplicatePageId — span-less (page node span not stored on RawPage).
        let dup_id = seen_ids.iter().any(|existing_id| existing_id == &id);
        if dup_id {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::DuplicatePageId,
                format!("duplicate page id '{id}'"),
            ));
        } else {
            seen_ids.push(id.clone());
        }

        // DuplicatePageRoute — span-less (same reason as DuplicatePageId).
        let dup_route = seen_routes.iter().any(|existing_route| existing_route == &route);
        if dup_route {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::DuplicatePageRoute,
                format!("duplicate page route '{route}'"),
            ));
        } else {
            seen_routes.push(route.clone());
        }

        // Extract route params — `:param` segments in the route.
        let params = extract_route_params(&route);

        // Lower the page body components. Route was present and parseable
        // above, so $param resolution is always enabled (skip_param_check =
        // false).
        let (mut body, figure_slots) = lower_page_body(
            &raw_page.body,
            &params,
            false,
            ctx,
            default_language,
            &mut seen_list_ids,
            report,
        );

        // DuplicateSingletonComponent — the engine assumes at most one
        // `resource-detail` (fixed record-viewer id `record-figure`), at
        // most one `facet-panel` (`_facets` fragment handler assumes exactly
        // one panel per page), at most one `resource-list` (the surf-head's
        // result count is a fixed `id="browse-count"`), and at most one
        // `search-bar` (its input is a fixed `id="q"`) per page. A second
        // instance of any of these would collide on that fixed id/handler at
        // render time, so this is a boot hard-fail. Checked regardless of
        // whether a `resource-list` declares an `id` at all — `id` is
        // optional and orthogonal to this count (contrast with
        // `DuplicateResourceListId`, which only fires when two *declared*
        // ids collide).
        for (component_name, count) in [
            (
                "resource-detail",
                body.iter().filter(|c| matches!(c, Component::ResourceDetail { .. })).count(),
            ),
            (
                "facet-panel",
                body.iter().filter(|c| matches!(c, Component::FacetPanel { .. })).count(),
            ),
            (
                "resource-list",
                body.iter().filter(|c| matches!(c, Component::ResourceList { .. })).count(),
            ),
            (
                "search-bar",
                body.iter().filter(|c| matches!(c, Component::SearchBar { .. })).count(),
            ),
            // The reader's fixed `id="compound-viewer"` / `id="cv-picker"`
            // assume one navigator per page.
            (
                "compound-viewer",
                body.iter().filter(|c| matches!(c, Component::CompoundViewer { .. })).count(),
            ),
        ] {
            if count > 1 {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::DuplicateSingletonComponent {
                        component: component_name.to_owned(),
                        page: route.clone(),
                    },
                    format!(
                        "page '{route}' declares {count} '{component_name}' components; \
                         at most one is supported per page"
                    ),
                ));
            }
        }

        // DuplicateGroupedProperty — a property in two `group`s, or in a
        // `group` and `show-properties`, would render twice on one record.
        // Config-internal ⇒ checked here, before the success/raw fork, so both
        // validation outcomes inherit it.
        check_duplicate_grouped_properties(&body, &route, report);

        // DuplicateNarrativeFor — two `narrative-for` entries for one resource
        // id leave undecidable which intro renders. Config-internal ⇒ before
        // the fork, like the above.
        check_duplicate_narrative_for(&body, &route, report);

        // A showcase case key is a DOM id (`case-<key>`); lowering rejects a
        // repeat inside one block, this one across the page's blocks.
        showcase::check_duplicate_case_keys(&body, report);

        // CompoundViewerWithPagedList — the compound-viewer reads
        // `?page=<member-id|name|prefix>`, a paginating resource-list reads
        // `?page=N`. On one page they would silently share the key, so the
        // combination is a boot hard-fail. `PerPage` is also the lowered
        // default when neither `per-page` nor `limit` is declared; only `limit`
        // (no pager) is compatible.
        let has_compound_viewer = body.iter().any(|c| matches!(c, Component::CompoundViewer { .. }));
        let has_paged_list = body
            .iter()
            .any(|c| matches!(c, Component::ResourceList { pagination: Pagination::PerPage(_), .. }));
        if has_compound_viewer && has_paged_list {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::CompoundViewerWithPagedList { page: route.clone() },
                format!(
                    "page '{route}' declares both a 'compound-viewer' and a paginating \
                     'resource-list' (per-page); both read the '?page=' query key \
                     (member address vs. page number), so they cannot share a page — \
                     give the resource-list a 'limit' or move one of them to another page"
                ),
            ));
        }

        // PagesLayoutWithoutMedia — the `pages` layout is scan-first; without
        // `media=` it has no scan to show. Config-internal ⇒ before the
        // success/raw fork.
        if body
            .iter()
            .any(|c| matches!(c, Component::CompoundViewer { layout: CompoundLayout::Pages, media: None, .. }))
        {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::PagesLayoutWithoutMedia { page: route.clone() },
                format!(
                    "page '{route}' declares a 'compound-viewer' with layout=\"pages\" but no \
                     'media' attribute; the pages layout shows each member's scan, so it needs \
                     media=\"<member image link property>\" (or use layout=\"contents\")"
                ),
            ));
        }

        // PagesLayoutOnly (book-page Choice E) — the picker lives in the
        // `pages` pager row, the member-links row sits under it and the
        // annotation register follows the pages reader; any other layout has
        // nowhere to show them, so each is rejected rather than ignored.
        // Config-internal ⇒ before the success/raw fork.
        for c in &body {
            let Component::CompoundViewer {
                picker,
                member_links,
                annotations,
                layout: CompoundLayout::Contents,
                ..
            } = c
            else {
                continue;
            };
            let misplaced = [
                (
                    picker.is_some(),
                    "picker",
                    "a 'picker' attribute",
                    "the page picker lives in the pages reader's pager row",
                ),
                (
                    member_links.is_some(),
                    "member-links",
                    "a 'member-links' child",
                    "the member-links row sits under the pages reader's pager",
                ),
                (
                    *annotations,
                    "annotations",
                    "an 'annotations' attribute",
                    "the annotation register belongs to the pages reader's page",
                ),
            ];
            for (present, feature, what, why) in misplaced {
                if !present {
                    continue;
                }
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::PagesLayoutOnly { feature: feature.to_owned(), page: route.clone() },
                    format!(
                        "page '{route}' declares a 'compound-viewer' with {what} but \
                         layout=\"contents\"; {why}, so use layout=\"pages\" or drop the \
                         {feature}"
                    ),
                ));
            }
        }

        // CompoundMediaOwnership (dsp-incubator
        // cpe/docs/adr/0012-one-member-navigator-declared-layout.md) — a `resource-detail`
        // with `media=` on the same compound the page's navigator shows would render the
        // compound's images twice. The navigator owns them. Config-internal ⇒ before the
        // success/raw fork.
        let owns_media = body.iter().any(|c| {
            matches!(
                c,
                Component::ResourceDetail { id, media: Some(_), .. }
                    if pairs_with_navigator(id.as_deref(), &body)
            )
        });
        if owns_media {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::CompoundMediaOwnership { page: route.clone() },
                format!(
                    "page '{route}' declares a 'resource-detail' with 'media' on the same \
                     resource its 'compound-viewer' navigates; the navigator owns the \
                     compound's images — drop 'media' from the resource-detail"
                ),
            ));
        }

        // MultipleImageViewers (dsp-incubator
        // cpe/docs/adr/0012-one-member-navigator-declared-layout.md) — a `pages` reader
        // mounts the record viewer's unit, and so does any `resource-detail` with `media=`;
        // the unit's signals are page-global, so two on one page drive each other. Reported
        // only when `CompoundMediaOwnership` has not already fired (one mistake, one
        // diagnostic). Config-internal ⇒ before the success/raw fork.
        let has_pages_reader = body
            .iter()
            .any(|c| matches!(c, Component::CompoundViewer { layout: CompoundLayout::Pages, .. }));
        let has_detail_media = body
            .iter()
            .any(|c| matches!(c, Component::ResourceDetail { media: Some(_), .. }));
        if has_pages_reader && has_detail_media && !owns_media {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::MultipleImageViewers { page: route.clone() },
                format!(
                    "page '{route}' declares a 'compound-viewer' with layout=\"pages\" and a \
                     'resource-detail' with 'media'; both mount the record viewer, whose signals \
                     (rail, expand, plate, the Adjust values, the zoom flags) are page-global, so \
                     the two viewers would drive each other — one image viewer per page: \
                     drop 'media' from the resource-detail"
                ),
            ));
        }

        // The resource-detail `navigator` flag — the same
        // `pairs_with_navigator` call `CompoundMediaOwnership` makes above, so
        // the two cannot drift. Computed first (immutable borrow), then
        // written.
        let navigator_flags: Vec<bool> = body
            .iter()
            .map(|c| match c {
                Component::ResourceDetail { id, .. } => pairs_with_navigator(id.as_deref(), &body),
                _ => false,
            })
            .collect();
        for (c, flag) in body.iter_mut().zip(navigator_flags) {
            if let Component::ResourceDetail { navigator, .. } = c {
                *navigator = flag;
            }
        }

        // The compound-viewer `paired` flag — the mirror image of `navigator` above:
        // whether some `resource-detail` on this page shares this compound-viewer's
        // `source` expression. `true` ⇒ the empty (`total == 0`) branch must not
        // repeat that record's title/eyebrow/properties (dsp-incubator
        // cpe/docs/adr/0012-one-member-navigator-declared-layout.md). A distinct pass
        // from `navigator_flags` above (a different component variant), but through
        // the SAME `pairs_with_navigator` predicate, so the two flags can never
        // disagree for a given pairing (`render_body` relies on it).
        let paired_flags: Vec<bool> = body
            .iter()
            .map(|c| match c {
                Component::CompoundViewer { .. } => body.iter().any(|other| {
                    matches!(other, Component::ResourceDetail { id, .. }
                        if pairs_with_navigator(id.as_deref(), std::slice::from_ref(c)))
                }),
                _ => false,
            })
            .collect();
        for (c, flag) in body.iter_mut().zip(paired_flags) {
            if let Component::CompoundViewer { paired, .. } = c {
                *paired = flag;
            }
        }

        pages.push(Page { id, route, params, body, figure_slots });
    }

    pages
}

/// Whether a `resource-detail` whose `id` expression is `detail_id` shows the
/// same compound as a sibling `compound-viewer` in the lowered page `body` —
/// i.e. whether that record is paired with a member navigator on its page
/// (dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md).
///
/// **Single source of truth** for the pairing: `CompoundMediaOwnership`,
/// the resource-detail `navigator` flag and the compound-viewer `paired`
/// flag all call this on the same lowered body, so they cannot drift.
///
/// The comparison is on the **expression**, not the resolved id: the same
/// `$param` string, or the same literal. An absent `id` never pairs.
fn pairs_with_navigator(detail_id: Option<&str>, body: &[Component]) -> bool {
    let Some(detail_id) = detail_id else {
        return false;
    };
    body.iter().any(|c| {
        matches!(
            c,
            Component::CompoundViewer { source, .. } if source == detail_id
        )
    })
}

/// Return `true` if any path segment of `route` begins with `_`.
///
/// The engine reserves the `_`-prefixed route namespace for engine-registered
/// routes (`<base>/<id>/_static/…`, `<base>/<id>/_assets/…`, the `_<component>`
/// fragment segments). A config page must not declare a route whose first (or
/// any) segment starts with `_`. Splitting on `/` and ignoring empty segments
/// means a leading slash (`"/_foo"`) is handled
/// correctly. `:param` segments never start with `_`, so they are unaffected.
fn route_first_segment_is_reserved(route: &str) -> bool {
    route.split('/').filter(|seg| !seg.is_empty()).any(|seg| seg.starts_with('_'))
}

/// Extract declared route parameters from a route pattern.
///
/// Splits on `/`, collects segments starting with `:`, strips the `:`.
/// e.g. `"/books/:id/chapters/:chapter"` → `["id", "chapter"]`.
pub(crate) fn extract_route_params(route: &str) -> Vec<String> {
    route
        .split('/')
        .filter(|seg| seg.starts_with(':'))
        .map(|seg| seg[1..].to_owned())
        .collect()
}

// ── Component lowering ───────────────────────────────────────────────

/// The six generic v1 component names, valid in every project. Per-project
/// custom components (e.g. Incunabula's `timeline`) are not listed here: they
/// are valid only for the project `project_components` registers them
/// for (dsp-incubator cpe/docs/adr/0011-per-project-components.md).
const V1_COMPONENTS: &[&str] = &[
    "narrative",
    "resource-list",
    "resource-detail",
    "compound-viewer",
    "facet-panel",
    "search-bar",
];

/// The `UnknownComponent` message: names both the generic v1 set and the
/// per-project components registered for `project_id` (if any), so a
/// component registered for a *different* project reads as unknown here.
fn unknown_component_message(name: &str, project_id: &str) -> String {
    let generic = V1_COMPONENTS.join(", ");
    let registered = project_components::names_for(project_id);
    if registered.is_empty() {
        format!(
            "unknown component '{name}'; expected one of: {generic} \
             (no per-project components are registered for project '{project_id}')"
        )
    } else {
        format!(
            "unknown component '{name}'; expected one of: {generic}, \
             or a component registered for project '{project_id}': {}",
            registered.join(", ")
        )
    }
}

/// Lower a page's raw component list into typed [`Component`]s.
///
/// - Unknown components → `UnknownComponent` diagnostic, component skipped.
/// - Components with validation errors → diagnostics pushed, component skipped.
/// - Sibling components are always processed regardless of earlier failures.
///
/// `seen_list_ids` accumulates `resource-list` ids across pages for
/// `DuplicateResourceListId` detection.
///
/// `skip_param_check`: when `true`, `$param` resolution is skipped for
/// this page's components (used when the page route is absent/unparseable).
///
/// `default_language` — threaded straight through to the `narrative`
/// component arm's existence probe; see `lower_narrative`.
///
/// Returns the typed body and, index-aligned with it, each component's
/// figure-block slot (`Page::figure_slots`): a component and its slot are
/// pushed or dropped together.
fn lower_page_body(
    raw_components: &[crate::config::Component],
    page_params: &[String],
    skip_param_check: bool,
    ctx: &ValidationContext<'_>,
    default_language: Option<&str>,
    seen_list_ids: &mut Vec<String>,
    report: &mut ValidationReport,
) -> (Vec<Component>, Vec<Option<FigureSlot>>) {
    // Collect all resource-list ids declared in this page's raw IR up front,
    // so that UnresolvedTarget resolution doesn't cascade on a list that exists
    // but failed another check.
    let page_list_ids: Vec<String> = raw_components
        .iter()
        .filter(|c| c.name == "resource-list")
        .filter_map(|c| get_string_attr(c, "id"))
        .collect();

    let mut typed = Vec::new();
    let mut slots = Vec::new();
    // Raw names, not lowered ones: a narrative that failed its own checks
    // must not add a second, cascading `FigureSectionWithoutNarrative`.
    let mut follows_narrative = false;

    for raw_comp in raw_components {
        let comp_span = raw_comp.span;
        let is_narrative = raw_comp.name == "narrative";

        // Allowlist: a generic v1 component, or one registered for THIS
        // project (`(project, name)` — registered elsewhere is unknown here).
        let registered = project_components::lookup(ctx.project_id, &raw_comp.name);
        if !V1_COMPONENTS.contains(&raw_comp.name.as_str()) && registered.is_none() {
            report.push(Diagnostic::error(
                DiagnosticKind::UnknownComponent,
                unknown_component_message(&raw_comp.name, ctx.project_id),
                comp_span,
            ));
            continue;
        }

        let before = error_count(report);

        let lowered = match raw_comp.name.as_str() {
            "narrative" => lower_narrative(raw_comp, ctx, default_language, report),
            "resource-list" => lower_resource_list(raw_comp, page_params, skip_param_check, seen_list_ids, report),
            "resource-detail" => {
                lower_resource_detail(raw_comp, page_params, skip_param_check, ctx, default_language, report)
            }
            "compound-viewer" => lower_compound_viewer(raw_comp, page_params, skip_param_check, report),
            "facet-panel" => lower_facet_panel(raw_comp, &page_list_ids, report),
            "search-bar" => lower_search_bar(raw_comp, page_params, skip_param_check, &page_list_ids, report),
            // Not a generic name, so the allowlist above passed it on its
            // registry row; lower through that row. `None` cannot occur (the
            // allowlist `continue`d), and lowers to nothing rather than panic.
            _ => registered.and_then(|row| (row.lower)(raw_comp, ctx, report)),
        };
        // The slot attributes are generic to every per-project component;
        // a v1 component carries none.
        let slot = registered.and_then(|_| lower_figure_slot(raw_comp, follows_narrative, report));

        // Only push the component if it produced no new errors.
        if error_count(report) == before {
            if let Some(c) = lowered {
                typed.push(c);
                slots.push(slot);
            }
        }
        follows_narrative |= is_narrative;
    }

    (typed, slots)
}

/// Lower a per-project component's figure-block slot attributes (dsp-incubator
/// cpe/docs/adr/0011-per-project-components.md): `section=` names a `##` heading id of the nearest
/// preceding `narrative`; `count=` and `more-label=` are chrome-string ids; `more-route=` a page
/// route. `None` when `section=` is absent. Reports, so lowering only.
///
/// `count=`/`more-*` without `section=`, and one of `more-label=`/
/// `more-route=` without the other, are `MissingRequiredAttribute`; a
/// `section=` with no `narrative` before it is
/// `FigureSectionWithoutNarrative`. Whether the heading exists is checked
/// against the narrative content in `validate_with_data`.
fn lower_figure_slot(
    raw: &crate::config::Component,
    follows_narrative: bool,
    report: &mut ValidationReport,
) -> Option<FigureSlot> {
    let has = |key: &str| raw.attrs.iter().any(|(k, _)| k == key);
    let missing = |key: &str, because: &str, report: &mut ValidationReport| {
        report.push(Diagnostic::error(
            DiagnosticKind::MissingRequiredAttribute,
            format!("component '{}' is missing the '{key}' attribute: {because}", raw.name),
            raw.span,
        ));
    };

    let section = get_typed_string_attr(raw, "section", report);
    let count = get_typed_string_attr(raw, "count", report);
    let more_label = get_typed_string_attr(raw, "more-label", report);
    let more_route = get_typed_string_attr(raw, "more-route", report);

    if has("more-label") != has("more-route") {
        let (absent, present) = if has("more-label") {
            ("more-route", "more-label")
        } else {
            ("more-label", "more-route")
        };
        missing(
            absent,
            &format!("'{present}' needs it, the more link has a label and a route"),
            report,
        );
    }
    if !has("section") && ["count", "more-label", "more-route"].iter().any(|k| has(k)) {
        missing(
            "section",
            "'count', 'more-label' and 'more-route' only describe a figure-block slot",
            report,
        );
    }

    let section = section?;
    if !follows_narrative {
        report.push(Diagnostic::error(
            DiagnosticKind::FigureSectionWithoutNarrative { section: section.clone() },
            format!(
                "component '{}' declares section=\"{section}\" but no 'narrative' precedes it \
                 on its page; a figure block renders inside the narrative before it",
                raw.name
            ),
            raw.span,
        ));
    }
    let more = more_label.zip(more_route).map(|(label, route)| FigureMore { label, route });
    Some(FigureSlot { section, count, more })
}

/// Count current error-severity diagnostics in the report.
fn error_count(report: &ValidationReport) -> usize {
    report
        .iter()
        .filter(|d| d.severity == crate::diagnostic::Severity::Error)
        .count()
}

// ── Per-component lowering helpers ────────────────────────────────────────────

fn lower_narrative(
    raw: &crate::config::Component,
    ctx: &ValidationContext<'_>,
    default_language: Option<&str>,
    report: &mut ValidationReport,
) -> Option<Component> {
    let file = require_string_attr(raw, "file", report);
    let companion = lower_narrative_choice(
        raw,
        "companion",
        &NarrativeCompanion::ALLOWED,
        NarrativeCompanion::parse_config_str,
        report,
    );
    let side = lower_narrative_choice(raw, "side", &NarrativeSide::ALLOWED, NarrativeSide::parse_config_str, report);
    let file = file?;
    if !check_narrative_file(&file, ctx, default_language, raw.span, report) {
        return None;
    }
    Some(Component::Narrative { file, companion, side })
}

/// Read a closed-vocabulary `narrative` attribute: absent ⇒ `None`; a
/// non-string or a string outside `allowed` ⇒ `InvalidAttributeType`.
fn lower_narrative_choice<T>(
    raw: &crate::config::Component,
    key: &str,
    allowed: &[&str],
    parse: fn(&str) -> Option<T>,
    report: &mut ValidationReport,
) -> Option<T> {
    let value = get_typed_string_attr(raw, key, report)?;
    let parsed = parse(&value);
    if parsed.is_none() {
        let allowed = allowed.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(" or ");
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidAttributeType,
            format!("narrative '{key}' must be {allowed}, got \"{value}\""),
            raw.span,
        ));
    }
    parsed
}

/// `NarrativeLayoutNotFirst` for every `narrative` after its page's first
/// that sets `companion=` or `side=`: the page's layout is read from the
/// first one only, so the attribute would be ignored.
fn check_narrative_layout_first(pages: &[Page], report: &mut ValidationReport) {
    for page in pages {
        let narratives = page.body.iter().filter_map(|c| match c {
            Component::Narrative { file, companion, side } => Some((file, companion, side)),
            _ => None,
        });
        for (file, companion, side) in narratives.skip(1) {
            if companion.is_some() || side.is_some() {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::NarrativeLayoutNotFirst { page: page.route.clone() },
                    format!(
                        "page '{}': narrative '{file}' sets 'companion' or 'side', but it is not \
                         the page's first narrative; move the attribute to the first one",
                        page.route
                    ),
                ));
            }
        }
    }
}

/// The two boot checks every narrative `file="…"` value runs — shared by the
/// page-level `narrative` component and `resource-detail`'s `narrative-for`
/// child, so one mistake is one kind: `InvalidNarrativePath` (path separator /
/// `..`) and `NarrativeFileNotFound` (neither the base file nor the
/// default-language variant is in the inventory; skipped when the inventory is
/// empty — "unknown, not checked").
///
/// Returns `true` when the file passed both checks; on `false` exactly one
/// Error was pushed.
fn check_narrative_file(
    file: &str,
    ctx: &ValidationContext<'_>,
    default_language: Option<&str>,
    span: crate::diagnostic::Span,
    report: &mut ValidationReport,
) -> bool {
    // InvalidNarrativePath — path separator, bare "..", or ".." as a path
    // segment. Note: `"..".split('.').any(|s| s == "..")` never matches because
    // splitting ".." by '.' yields ["", "", ""] — so `file == ".."` must be
    // checked explicitly alongside the separator-split check.
    let has_path_sep = file.contains('/') || file.contains('\\');
    let has_dotdot = file == ".." || file.split('/').any(|s| s == "..") || file.split('\\').any(|s| s == "..");
    if has_path_sep || has_dotdot {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidNarrativePath,
            format!(
                "narrative file '{file}' contains a path separator or '..'; \
                 only bare filenames are allowed"
            ),
            span,
        ));
        return false;
    }

    // NarrativeFileNotFound — must be in the inventory.
    // An empty slice means the caller found no narrative/ dir, so existence
    // is not checked (the host supplies the inventory; no dir → no constraint).
    if !ctx.narrative_files.is_empty() {
        let base_exists = ctx.narrative_files.iter().any(|f| f == file);
        if !base_exists {
            // The narrative file must resolve to the base file (checked above)
            // OR the default-language variant specifically — the two rungs the
            // render-time fallback chain (requested → default → base)
            // guarantees will always render. Non-default-language variants are
            // optional: they're a nice-to-have that falls back to the default
            // variant at render time, so their absence isn't a boot-time error.
            // This is a narrower probe than an "any language variant" check.
            let default_variant_exists = match default_language {
                Some(default_lang) => match crate::util::variant_filename(file, default_lang) {
                    Some(candidate) => ctx.narrative_files.iter().any(|f| f == &candidate),
                    // No extension → no variant form is possible; only the
                    // base counts, and base_exists is already false here.
                    None => false,
                },
                // No resolved default language (site lowering itself failed)
                // — fall back to the lenient "any variant" probe so this
                // check doesn't pile a second, less-precise error on top of
                // an already-broken site block.
                None => {
                    let (stem, _ext) = split_stem_ext(file);
                    ctx.narrative_files.iter().any(|f| f.starts_with(&format!("{stem}.")))
                }
            };
            if !default_variant_exists {
                report.push(Diagnostic::error(
                    DiagnosticKind::NarrativeFileNotFound,
                    format!("narrative file '{file}' not found in the narrative/ directory"),
                    span,
                ));
                return false;
            }
        }
    }

    true
}

/// Check that every nav item's route matches a declared page route.
///
/// `nav_items` — the lowered nav from `SiteConfig`; each item has a `label`
/// and a `route` that has already passed through the site lowering (i.e. only
/// items with both label and route present are in the vec).
///
/// `declared_routes` — the `route` field of every `Page` that was produced by
/// `check_and_lower_pages`. Route strings are collected from the lowered `Page`
/// structs (not re-derived from raw config) so we use the exact same string
/// the renderer will compare against for `is_active`.
///
/// One `UnresolvedNavRoute` diagnostic per offending nav item — never
/// short-circuits.
fn check_nav_routes(nav_items: &[crate::ir::NavItem], declared_routes: &[&str], report: &mut ValidationReport) {
    for item in nav_items {
        check_route_declared(
            &format!("nav item '{}'", item.label),
            &item.route,
            declared_routes,
            DiagnosticKind::UnresolvedNavRoute,
            report,
        );
    }
}

/// `site { not-found { browse route="…" } }` route existence — the same
/// exact-match rule and message shape as `check_nav_routes`, reporting
/// `UnresolvedNotFoundBrowseRoute`. Config-internal, so like `check_nav_routes`
/// it runs before the `validate_with_data` fork and needs no `*_into_raw`
/// sibling.
fn check_not_found_browse_routes(entries: &[NotFoundBrowse], declared_routes: &[&str], report: &mut ValidationReport) {
    for entry in entries {
        check_route_declared(
            "not-found browse link",
            &entry.route,
            declared_routes,
            DiagnosticKind::UnresolvedNotFoundBrowseRoute,
            report,
        );
    }
}

/// The shared exact-string route-existence check behind `check_nav_routes`
/// and `check_not_found_browse_routes`: push `kind` when `route` is not one
/// of `declared_routes`. `what` names the offending item in the message.
fn check_route_declared(
    what: &str,
    route: &str,
    declared_routes: &[&str],
    kind: DiagnosticKind,
    report: &mut ValidationReport,
) {
    if !declared_routes.contains(&route) {
        report.push(Diagnostic::error_spanless(
            kind,
            format!(
                "{what} has route '{route}' which does not match any declared page route; \
                 declared routes: [{}]",
                declared_routes.join(", ")
            ),
        ));
    }
}

/// Check that every declared `labels { class "…" }` entry also declares a
/// `plural { … }` block.
///
/// Config-internal (needs no data), so this runs inside `validate_inner`
/// before the `validate_with_data` fork — like `check_nav_routes` and
/// `check_class_routes`, its diagnostics reach both arms and it needs no
/// `*_into_raw` sibling.
///
/// A class label without a plural is not illegal —
/// `labels::resolve_class_plural` degrades to the singular label — but the
/// degraded copy reads wrong on every plural-bearing surface ("Alle Buch
/// durchsuchen"), so one `MissingClassPlural` **Warning** per offending class
/// (never short-circuits). Only the presence of the block is checked here;
/// per-language completeness of a declared plural is not (a declared plural
/// missing one language still resolves to another declared plural, which is
/// a better answer than the singular).
fn check_class_plurals(class_labels: &[crate::ir::ClassLabel], report: &mut ValidationReport) {
    for label in class_labels {
        if label.plural.is_empty() {
            report.push(Diagnostic {
                kind: DiagnosticKind::MissingClassPlural { class: label.class.clone() },
                message: format!(
                    "class label '{}' declares no `plural {{ … }}` block; the plural-bearing \
                     surfaces (search no-match copy, browse breadcrumb/eyebrow, resource-detail \
                     class crumb, 404 'browse all' exit) will fall back to the singular label",
                    label.class
                ),
                span: None,
                severity: Severity::Warning,
            });
        }
    }
}

/// Extract the class→route binding contributed by a single page, if any.
///
/// A page contributes a binding when **all** of:
/// - Its route has **exactly one** `:param` segment.
/// - Its body contains a `resource-detail` with a declared `class` (`Some`).
/// - That `resource-detail`'s `id` is NOT a literal (i.e. `id` is `None` or starts with `$`).
///
/// Returns `Some((class, route))` for the first such component on the page,
/// or `None` when the page contributes no binding. Both `check_class_routes`
/// and `build_class_routes` use this as their single source of truth for the
/// contribution predicate.
fn page_class_binding(page: &Page) -> Option<(&str, &str)> {
    let param_count = page.route.split('/').filter(|seg| seg.starts_with(':')).count();
    if param_count != 1 {
        return None;
    }

    for component in &page.body {
        if let Component::ResourceDetail { id, class: Some(ref class), .. } = component {
            // Skip literal-id resource-details (non-`$param` ids).
            // A `$param` id (or `None` id) is the parameterised case.
            let is_literal = id.as_deref().map(|s| !s.starts_with('$')).unwrap_or(false);
            if is_literal {
                continue;
            }
            return Some((class.as_str(), page.route.as_str()));
        }
    }
    None
}

/// Check for ambiguous class→route bindings across all pages.
///
/// A page contributes a binding when:
/// - Its route has **exactly one** `:param` segment, AND
/// - Its body contains a `resource-detail` with a declared `class` (`Some`).
///
/// Pages whose `resource-detail` has a literal (non-`$param`) id, or no declared
/// class, or a route without exactly one param, do NOT contribute.
///
/// If two contributing pages bind the **same class**, one `AmbiguousClassRoute`
/// diagnostic is pushed per collision (accumulated, never short-circuited).
///
/// This check runs inside `validate()` so the diagnostic participates in the
/// standard boot hard-fail / `--check` reporting. The map itself is built by
/// `build_class_routes` from the already-validated config (deduplication already
/// done by this check), so there is exactly one source of truth.
fn check_class_routes(pages: &[Page], report: &mut ValidationReport) {
    // class → first_route seen; on duplicate, emit the diagnostic.
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for page in pages {
        if let Some((class, route)) = page_class_binding(page) {
            if let Some(existing_route) = seen.get(class) {
                if existing_route.as_str() != route {
                    // Collision: same class bound to two different routes.
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::AmbiguousClassRoute {
                            class: class.to_owned(),
                            first_route: existing_route.clone(),
                            second_route: route.to_owned(),
                        },
                        format!(
                            "class '{class}' is bound to two different parameterised routes: \
                             '{existing_route}' and '{route}'; the class→route map requires an \
                             unambiguous binding — remove the 'class' attribute from one of \
                             the conflicting resource-detail components"
                        ),
                    ));
                }
                // Either collision or exact-same-route duplicate — either way,
                // don't insert again (first registration wins for diagnostics).
            } else {
                seen.insert(class.to_owned(), route.to_owned());
            }
        }
    }
}

/// Check for `resource-list` components with `grid=#true` but no `media`
/// attribute.
///
/// A config-internal pages walk, mirroring `check_class_routes`'s shape — no
/// `ValidatedConfig`/data-accessor needed, so it runs inside `validate_inner`
/// alongside `check_class_routes`, meaning BOTH the data-validated and
/// raw-fallback validation paths inherit it for free. Deliberately NOT in
/// `lower_resource_list` (no page route in scope there) and NOT in
/// `check_resource_list_media` (data-dependent, `media: Some(..)`-guarded,
/// and skipped on the raw-fallback path — the exact gap this check avoids).
///
/// Pushes one `DiagnosticKind::GridWithoutMedia` (Error) per offending
/// `resource-list`, accumulated — never short-circuits.
fn check_grid_without_media(pages: &[Page], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            if let Component::ResourceList { class, grid: true, media: None, .. } = component {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::GridWithoutMedia { class: class.clone(), page: page.route.clone() },
                    format!(
                        "resource-list on page '{}' has 'grid=#true' but no 'media' attribute; \
                         grid mode renders a wall of image tiles and requires a 'media' property \
                         naming the class's embedded-image set — add 'media=\"…\"' or remove 'grid=#true'",
                        page.route
                    ),
                ));
            }
        }
    }
}

/// `TileRatioWithoutGrid` (Error) for every `resource-list` with a
/// `tile-ratio` and no `grid=#true`: without the grid view the ratio would be
/// read and never used. Config-internal like [`check_grid_without_media`],
/// so both validation arms inherit it.
fn check_tile_ratio_without_grid(pages: &[Page], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            if let Component::ResourceList { class, grid: false, tile_ratio: Some(_), .. } = component {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::TileRatioWithoutGrid { class: class.clone(), page: page.route.clone() },
                    format!(
                        "resource-list on page '{}' has a 'tile-ratio' but no 'grid=#true'; \
                         the ratio shapes the grid view's tiles — add 'grid=#true' or remove \
                         'tile-ratio'",
                        page.route
                    ),
                ));
            }
        }
    }
}

/// Build the class→route map from an already-validated config.
///
/// Applies the same contribution rules as `check_class_routes` (via the shared
/// `page_class_binding` predicate: exactly one `:param` in the route,
/// `resource-detail` with a declared `class`, not a literal id). Because
/// `check_class_routes` runs first during `validate()`, any ambiguous bindings
/// are already reported as hard-fail diagnostics; by the time this function is
/// called (inside `Project::load` after validation passed) there are no
/// duplicates and the map is unambiguous.
///
/// Returns `HashMap<class, route_template>` — e.g. `"Character" → "/characters/:id"`.
/// The routes are project-relative (no `/<project-id>` prefix); the renderer
/// prepends the project id when building link hrefs.
pub fn build_class_routes(pages: &[Page]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for page in pages {
        if let Some((class, route)) = page_class_binding(page) {
            // First registration wins (duplicates already caught by check_class_routes).
            map.entry(class.to_owned()).or_insert_with(|| route.to_owned());
        }
    }

    map
}

/// Build the class→list-route map.
///
/// Mirrors [`build_class_routes`]'s shape but reads `Component::ResourceList
/// { class, .. }` on **parameter-less** pages (a browse page has zero `:param`
/// segments — the sibling detail map requires exactly one). Returns
/// `HashMap<class, route>` — e.g. `"Character" → "/characters"` — project-
/// relative, no `/<project-id>` prefix (the renderer prepends it, same as
/// `class_routes`).
///
/// **First registration wins — deliberately laxer than `build_class_routes`'s
/// sibling `check_class_routes` hard-fail (`AmbiguousClassRoute`).** Two
/// browse pages over the same class is a legitimate config (e.g. a curated
/// subset page and a full browse page for the same Class) — this map is
/// **advisory**: it powers the "Browse all {class}" link and breadcrumb
/// class crumb, never routing or link resolution for `link` property values.
/// There is no corresponding `check_class_list_routes` hard-fail pass and none
/// is needed — an arbitrary-but-deterministic choice between two equally
/// valid browse pages is an acceptable outcome for an advisory map, unlike
/// the detail map where two bindings really would be ambiguous routing.
///
/// `resource-list` IS singleton-validated **per page**
/// (`DiagnosticKind::DuplicateSingletonComponent` — a page can have at most
/// one), so the per-page scan for a class binding is trivial; first-wins only
/// arbitrates *across* pages.
pub fn build_class_list_routes(pages: &[Page]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for page in pages {
        let param_count = page.route.split('/').filter(|seg| seg.starts_with(':')).count();
        if param_count != 0 {
            continue;
        }

        for component in &page.body {
            if let Component::ResourceList { class, .. } = component {
                // First registration wins across pages (see doc comment above).
                map.entry(class.clone()).or_insert_with(|| page.route.clone());
                // A page's `resource-list` is boot-singleton-validated, but
                // guard the scan anyway: stop at the first hit on this page.
                break;
            }
        }
    }

    map
}

// ── `media="…"` — the embedded-media axis ─────────────────────────────────────
//
// A `resource-detail`/`compound-viewer`/`resource-list` `media` attribute
// names the host→image `link` property carrying a resource's (or a list
// row's) embedded-media set. Pieces, mirroring the `class_routes` shape
// above:
//   - `resolved_detail_class` — the class-resolution rule shared by the per-component check and the
//     map builder.
//   - `media_property::check_single_media_property` — the four-check data-dependent validator
//     (existence / link-type / sparse / target-representations), mirroring `check_single_facet`'s
//     structure.
//   - `check_resource_detail_media` / `check_compound_viewer_media` / `check_resource_list_media` —
//     walk `config.pages`, in that order, each running the per-component check;
//     `check_resource_detail_media` builds the initial class→(media-property, page) map,
//     `check_compound_viewer_media` merges its own resolved member classes into the SAME map via
//     `register_media_class` (first wins; a different property already registered ⇒
//     `ConflictingMediaProperty`), and `check_resource_list_media` finally checks (without
//     registering) against the fully merged map.
//   - `build_media_properties` — builds the actual map (called from `Project::load`, mirroring
//     `build_class_routes`), safe to call only after validation has passed (no
//     `UnresolvableMediaClass`/ `ConflictingMediaProperty` diagnostics remain).

/// Resolve the class a `resource-detail` component's `media` (or any other
/// class-dependent attribute) applies to: the component's own `class`
/// attribute wins; otherwise, for a literal (non-`$param`) `id`, the class is
/// looked up from the data. Returns `None` when neither source resolves a
/// class (a `$param`/absent `id` page with no `class` attribute — the
/// `UnresolvableMediaClass` case) or when a literal `id` does not exist in
/// the data (already flagged elsewhere as `UnknownResource`; not re-flagged
/// here).
fn resolved_detail_class(id: &Option<String>, class: &Option<String>, data: &dyn ProjectFacts) -> Option<String> {
    if let Some(c) = class {
        return Some(c.clone());
    }
    let id_val = id.as_deref()?;
    if id_val.starts_with('$') {
        return None;
    }
    data.class_of(id_val).ok().flatten()
}

/// Run `resource-detail media="…"` validation across all pages of an
/// already-validated config, and build the class→media-property map used
/// both to check `resource-list` consistency here and by
/// `build_media_properties` (called after validation passes).
///
/// Returns the class→(media-property, page-route) map, so
/// `check_resource_list_media` can check for conflicts without a second,
/// separate data-dependent walk.
fn check_resource_detail_media(
    config: &ValidatedConfig,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> std::collections::HashMap<String, (String, String)> {
    let mut map: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceDetail { id, class, media: Some(ref media_prop), .. } = component {
                let resolved = resolved_detail_class(id, class, data);
                match resolved {
                    Some(resolved_class) => {
                        // A typo'd, declared `class` attribute is already
                        // flagged as `UnknownResourceDetailClass` by
                        // `check_resource_detail_class` — skip the media
                        // four-check and map registration to avoid cascading
                        // a `SparseMediaProperty`/etc. against a nonexistent
                        // class (the cascade rule). A class resolved via
                        // a literal id's `class_of` is unaffected: that
                        // lookup only ever returns a class that exists.
                        let class_is_unknown_declared =
                            class.as_deref().is_some_and(|c| matches!(data.class_exists(c), Ok(false)));
                        if class_is_unknown_declared {
                            continue;
                        }
                        check_single_media_property(&resolved_class, media_prop, &page.route, data, report);
                        map.entry(resolved_class)
                            .or_insert_with(|| (media_prop.clone(), page.route.clone()));
                    }
                    None => {
                        // Only the genuinely-unresolvable case (no `class`
                        // attribute AND `id` is absent/`$param`) is flagged
                        // here — a literal `id` that doesn't exist in the
                        // data is already flagged by `UnknownResource`
                        // elsewhere, so it is silently skipped to avoid
                        // duplicate diagnostics for the same root cause.
                        let literal_id_missing = id.as_deref().map(|s| !s.starts_with('$')).unwrap_or(false);
                        if !literal_id_missing {
                            report.push(Diagnostic {
                                kind: DiagnosticKind::UnresolvableMediaClass { page: page.route.clone() },
                                message: format!(
                                    "page '{}': resource-detail declares 'media=\"{media_prop}\"' \
                                     but has no resolvable class — add a 'class=\"…\"' attribute \
                                     (a class-less $param page cannot be validated)",
                                    page.route
                                ),
                                span: None,
                                severity: Severity::Error,
                            });
                        }
                    }
                }
            }
        }
    }

    map
}

/// Raw-fallback sibling of `check_resource_detail_media`: run the same
/// `media="…"` validation off the **raw** config when config-internal lowering
/// already failed, so a dual failure (config error + media defect) surfaces
/// both in one boot pass.
///
/// Extraction follows the `*_into_raw` contract — attributes are read via the
/// silent `get_string_attr`, so an absent `media`/`id` means a plain
/// `continue`. A present-but-wrong-typed `class` or `id`, though, is reported
/// by lowering (`get_typed_string_attr`) — this fn guards
/// against double-reporting that case; see the guard below. The check body
/// itself is the shared `check_single_media_property` helper, identical to
/// the success path.
///
/// Returns the class→(media-property, page-route) map for
/// `check_resource_list_media_into_raw`'s `ConflictingMediaProperty` check.
fn check_resource_detail_media_into_raw(
    raw: &RawConfig,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) -> std::collections::HashMap<String, (String, String)> {
    let mut map: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

    for raw_page in &raw.pages {
        let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-detail" {
                continue;
            }
            // No `media` attribute, or a wrong-typed one — which lowering
            // reports (`get_typed_string_attr`), so the success arm
            // sees `None` and skips this component too. Not this check's
            // business either way.
            let Some(media_prop) = get_string_attr(raw_comp, "media") else {
                continue;
            };

            // A present-but-wrong-typed `class` or `id` is reported by lowering
            // (`get_typed_string_attr`) — `continue` rather
            // than also emitting `UnresolvableMediaClass` here (one mistake,
            // one diagnostic). `get_string_attr` returns `None` for BOTH
            // "absent" and "present but wrong-typed", so distinguish the two
            // explicitly via `get_attr`.
            if get_attr(raw_comp, "class").is_some_and(|v| !matches!(v, Value::String(_)))
                || get_attr(raw_comp, "id").is_some_and(|v| !matches!(v, Value::String(_)))
            {
                continue;
            }

            let id = get_string_attr(raw_comp, "id");
            let class = get_string_attr(raw_comp, "class");

            match resolved_detail_class(&id, &class, data) {
                Some(resolved_class) => {
                    // Skip when the declared `class` attribute names a
                    // nonexistent class — already flagged by
                    // `check_resource_detail_class`, mirroring the success
                    // arm's cascade suppression. A class resolved via a
                    // literal id's `class_of` is unaffected.
                    let class_is_unknown_declared =
                        class.as_deref().is_some_and(|c| matches!(data.class_exists(c), Ok(false)));
                    if class_is_unknown_declared {
                        continue;
                    }
                    check_single_media_property(&resolved_class, &media_prop, page_route, data, report);
                    // First-wins, mirroring the success path.
                    map.entry(resolved_class)
                        .or_insert_with(|| (media_prop.clone(), page_route.to_owned()));
                }
                None => {
                    // A present-but-wrong-typed `class`/`id` was already
                    // filtered out above (lowering reports it via
                    // `get_typed_string_attr`) — this branch is only
                    // reached for a genuinely absent/unresolvable class, so
                    // no double-report risk here. Covered by the fixture pair
                    // `broken-media-wrong-typed-class{,-raw}`.
                    //
                    // Mirrors the success path: only the genuinely
                    // unresolvable case is flagged — a literal `id` absent
                    // from the data is already `UnknownResource`.
                    let literal_id_missing = id.as_deref().map(|s| !s.starts_with('$')).unwrap_or(false);
                    if !literal_id_missing {
                        report.push(Diagnostic {
                            kind: DiagnosticKind::UnresolvableMediaClass { page: page_route.to_owned() },
                            message: format!(
                                "page '{page_route}': resource-detail declares \
                                 'media=\"{media_prop}\"' but has no resolvable class — add a \
                                 'class=\"…\"' attribute (a class-less $param page cannot be \
                                 validated)"
                            ),
                            span: None,
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }
    }

    map
}

/// Run `resource-list media="…"` validation across all pages of an
/// already-validated config: the per-component `check_single_media_property`
/// four-check, plus a consistency check against `media_map` (the shared
/// class→(media-property, page-route) map, by this point populated by
/// `check_resource_detail_media` **and** `check_compound_viewer_media` — a
/// `resource-list` naming a different `media` property than an
/// already-registered `resource-detail`/`compound-viewer` binding for the
/// same class is a `ConflictingMediaProperty` hard-fail. A list-only
/// declaration (no matching earlier entry) is always fine. This check never
/// registers into `media_map` itself — a `resource-list`'s own `media` never
/// feeds `build_media_properties` (see its doc comment), and nothing runs
/// after this check that would need the registration.
fn check_resource_list_media(
    config: &ValidatedConfig,
    media_map: &std::collections::HashMap<String, (String, String)>,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for page in &config.pages {
        for component in &page.body {
            if let Component::ResourceList { class, media: Some(ref media_prop), .. } = component {
                // Skip $param references — the class is resolved at request
                // time, not known statically at boot (mirrors the other
                // resource-list data checks' skip rule).
                if class.starts_with('$') {
                    continue;
                }

                check_single_media_property(class, media_prop, &page.route, data, report);

                check_media_conflict(media_map, class, media_prop, &page.route, "resource-list", report);
            }
        }
    }
}

/// Raw-fallback sibling of `check_resource_list_media`: same check, inputs
/// extracted from the **raw** config (see `check_resource_detail_media_into_raw`
/// for the contract). `media_map` must come from
/// `check_resource_detail_media_into_raw` **and**
/// `check_compound_viewer_media_into_raw` running first, in that order —
/// otherwise `ConflictingMediaProperty` silently never fires for whichever
/// registration is missing.
fn check_resource_list_media_into_raw(
    raw: &RawConfig,
    media_map: &std::collections::HashMap<String, (String, String)>,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for raw_page in &raw.pages {
        let page_route = raw_page.route.as_deref().unwrap_or("<unknown page>");
        for raw_comp in &raw_page.body {
            if raw_comp.name != "resource-list" {
                continue;
            }
            let Some(media_prop) = get_string_attr(raw_comp, "media") else {
                continue;
            };
            // `class` is required on `resource-list`; an absent or wrong-typed
            // one was already reported by lowering.
            let Some(class) = get_string_attr(raw_comp, "class") else {
                continue;
            };
            // Skip $param references — resolved at request time.
            if class.starts_with('$') {
                continue;
            }

            check_single_media_property(&class, &media_prop, page_route, data, report);

            check_media_conflict(media_map, &class, &media_prop, page_route, "resource-list", report);
        }
    }
}

/// Build the class→media-property map from an already-validated config +
/// live data. Mirrors `build_class_routes`'s contract: safe to call only
/// after `check_resource_detail_media`/`check_compound_viewer_media`/
/// `check_resource_list_media` have run with no errors (no
/// `UnresolvableMediaClass`/`ConflictingMediaProperty` diagnostics), by which
/// point the map is unambiguous.
///
/// `resource-detail`, then `compound-viewer` declarations contribute. A
/// `resource-list`-only `media` declaration with no
/// matching earlier binding does not appear in this map — it drives that
/// list's own row thumbs directly, with nothing to suppress.
pub fn build_media_properties(pages: &[Page], data: &dyn ProjectFacts) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for page in pages {
        for component in &page.body {
            if let Component::ResourceDetail { id, class, media: Some(ref media_prop), .. } = component {
                if let Some(resolved_class) = resolved_detail_class(id, class, data) {
                    // First registration wins (duplicates already caught by
                    // AmbiguousClassRoute for the parameterised case; two
                    // literal-id pages resolving the same class with
                    // different media properties is an unhandled corner case).
                    map.entry(resolved_class).or_insert_with(|| media_prop.clone());
                }
            }
        }
    }

    // `compound-viewer media="…"`: resolve each member class the
    // same way `check_single_compound_viewer_media` does, and `or_insert`
    // it — a query error here means validation would already have raised
    // `InternalQueryError` for the same query, so it is safe to skip.
    for page in pages {
        for component in &page.body {
            if let Component::CompoundViewer { source, media: Some(ref media_prop), .. } = component {
                let parent = if source.starts_with('$') {
                    None
                } else {
                    Some(source.as_str())
                };
                let Ok(classes) = data.compound_member_classes(parent) else {
                    continue;
                };
                for class in classes {
                    map.entry(class).or_insert_with(|| media_prop.clone());
                }
            }
        }
    }

    map
}

fn lower_resource_list(
    raw: &crate::config::Component,
    page_params: &[String],
    skip_param_check: bool,
    seen_list_ids: &mut Vec<String>,
    report: &mut ValidationReport,
) -> Option<Component> {
    // `class` is required.
    let class = require_string_attr(raw, "class", report)?;

    // Check $param usage in class.
    if !skip_param_check {
        check_param_refs_in_value(&class, page_params, raw.span, report);
    }

    // Optional `id`. A present-but-wrong-typed value is `InvalidAttributeType`
    // (`get_typed_string_attr`), never silently absent.
    let id: Option<ResourceListId> = get_typed_string_attr(raw, "id", report);

    // DuplicateResourceListId — check project-wide.
    if let Some(ref list_id) = id {
        if seen_list_ids.contains(list_id) {
            report.push(Diagnostic::error(
                DiagnosticKind::DuplicateResourceListId,
                format!("duplicate resource-list id '{list_id}' within the project"),
                raw.span,
            ));
            return None;
        }
        seen_list_ids.push(list_id.clone());
    }

    // Lower child nodes: where, sort, per-page, limit, gloss, chip.
    let mut wheres = Vec::new();
    let mut sorts = Vec::new();
    let mut per_page: Option<u32> = None;
    let mut limit: Option<u32> = None;
    let mut gloss: Option<Gloss> = None;
    let mut chips: Vec<ChipSpec> = Vec::new();
    let mut order_options: Vec<OrderOption> = Vec::new();

    for child in &raw.children {
        match child.name.as_str() {
            "order-by" => {
                if let Some(option) = lower_order_by(child, &order_options, report) {
                    order_options.push(option);
                }
            }
            "where" => {
                if let Some(wc) = lower_where_clause(child, page_params, skip_param_check, report) {
                    wheres.push(wc);
                }
            }
            "sort" => {
                if let Some(s) = lower_sort(child, report) {
                    sorts.push(s);
                }
            }
            "gloss" => {
                // `gloss property="…"` (one property's first value) or a
                // composed `gloss { part …; teaser … }` block. A second `gloss`
                // node is a config error.
                if let Some(lowered) = lower_gloss(child, report) {
                    if gloss.is_some() {
                        report.push(Diagnostic::error(
                            DiagnosticKind::InvalidAttributeType,
                            "resource-list has more than one 'gloss'; only one is allowed",
                            child.span,
                        ));
                    } else {
                        gloss = Some(lowered);
                    }
                }
            }
            "chip" => {
                // `chip property="…" kind="term"|"keyword"` — repeatable; each names a
                // property whose values render as row chips. An absent `kind` derives the
                // shape from cardinality at render time (single list value → .term, multi
                // → .keyword); an explicit `kind` overrides that for every label
                // of
                // the property.
                if let Some(prop) = require_string_attr(child, "property", report) {
                    let kind = match get_typed_string_attr(child, "kind", report) {
                        None => None,
                        Some(ref s) if s == "term" => Some(ChipKind::Term),
                        Some(ref s) if s == "keyword" => Some(ChipKind::Keyword),
                        Some(other) => {
                            report.push(Diagnostic::error(
                                DiagnosticKind::InvalidAttributeType,
                                format!("chip 'kind' must be \"term\" or \"keyword\", got \"{other}\""),
                                child.span,
                            ));
                            None
                        }
                    };
                    if !chips.iter().any(|c: &ChipSpec| c.property == prop) {
                        chips.push(ChipSpec { property: prop, kind });
                    }
                }
            }
            "per-page" => {
                // `per-page <int>` — positional integer arg; must be 1..=u32::MAX.
                match first_integer_arg(child) {
                    Some(n) if (1..=u32::MAX as i128).contains(&n) => per_page = Some(n as u32),
                    Some(_) => {
                        report.push(Diagnostic::error(
                            DiagnosticKind::InvalidAttributeType,
                            "per-page value must be a positive integer \u{2264} 4294967295",
                            child.span,
                        ));
                    }
                    None => {
                        report.push(Diagnostic::error(
                            DiagnosticKind::InvalidAttributeType,
                            "per-page requires a positional integer argument",
                            child.span,
                        ));
                    }
                }
            }
            "limit" => {
                // `limit <int>` — positional integer arg; must be 1..=u32::MAX.
                match first_integer_arg(child) {
                    Some(n) if (1..=u32::MAX as i128).contains(&n) => limit = Some(n as u32),
                    Some(_) => {
                        report.push(Diagnostic::error(
                            DiagnosticKind::InvalidAttributeType,
                            "limit value must be a positive integer \u{2264} 4294967295",
                            child.span,
                        ));
                    }
                    None => {
                        report.push(Diagnostic::error(
                            DiagnosticKind::InvalidAttributeType,
                            "limit requires a positional integer argument",
                            child.span,
                        ));
                    }
                }
            }
            _ => {} // Unknown child nodes ignored at this layer.
        }
    }

    // PaginationConflict — both per-page and limit present.
    if per_page.is_some() && limit.is_some() {
        report.push(Diagnostic::error(
            DiagnosticKind::PaginationConflict,
            "resource-list has both 'per-page' and 'limit'; they are mutually exclusive",
            raw.span,
        ));
        return None;
    }

    let pagination = if let Some(n) = limit {
        Pagination::Limit(n)
    } else if let Some(n) = per_page {
        Pagination::PerPage(n)
    } else {
        Pagination::default() // PerPage(20)
    };

    // `media` is an optional string attribute naming the host→image `link`
    // property that carries this list's browse-row thumbs.
    // Data-existence/type/target-class checks happen in
    // `check_resource_list_media`; parsing here just captures the raw value.
    // A present-but-wrong-typed value is `InvalidAttributeType`
    // (`get_typed_string_attr`), never silently absent.
    let media = get_typed_string_attr(raw, "media", report);

    // `grid` is an optional KDL boolean attribute opting this list into grid
    // mode. Absent ⇒ `false`. A wrong-typed value (e.g. `grid="true"` as a
    // string, or `grid=1` as an integer) is itself reported by `get_bool_attr`
    // as `InvalidAttributeType` — it must not silently lower to `false`, or the
    // toggle would just never appear with no boot error. `grid: true` without
    // `media` is checked separately by `check_grid_without_media`.
    let grid = get_bool_attr(raw, "grid", report);

    // `tile-ratio="W:H"`: a string outside `TileRatio::parse`'s grammar is
    // reported here; a ratio without `grid=#true` by
    // `check_tile_ratio_without_grid`.
    let tile_ratio = get_typed_string_attr(raw, "tile-ratio", report).and_then(|text| {
        let ratio = TileRatio::parse(&text);
        if ratio.is_none() {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component 'resource-list' attribute 'tile-ratio' must be two integers \
                     from 1 to 9999 joined by a colon (\"3:4\", \"1:9\"), got \"{text}\""
                ),
                raw.span,
            ));
        }
        ratio
    });

    // `order-name-label`/`order-name-desc-label` — optional
    // chrome-string-id overrides (id-only, never literal text) for the
    // "Name A–Z"/"Name Z–A" ordering-control labels, resolved at render time
    // exactly like `search-bar`'s `placeholder`/`scope`. Diagnostic-free when
    // absent; a wrong-typed value (`order-name-label=#true`) is reported as
    // `InvalidAttributeType` rather than silently vanishing.
    let order_name_label = get_typed_string_attr(raw, "order-name-label", report);
    let order_name_desc_label = get_typed_string_attr(raw, "order-name-desc-label", report);
    // `order-default-label` relabels the built-in "As curated" row on the same
    // silent-fallback tier as the pair above. `order-name-property` swaps the
    // property the built-in `name`/`name-desc` rows order by; its existence on
    // the class is checked with the data (`check_resource_list_data`).
    let order_default_label = get_typed_string_attr(raw, "order-default-label", report);
    let order_name_property = get_typed_string_attr(raw, "order-name-property", report);

    Some(Component::ResourceList {
        id,
        class,
        wheres,
        sorts,
        pagination,
        gloss,
        chips,
        media,
        grid,
        tile_ratio,
        order_name_label,
        order_name_desc_label,
        order_default_label,
        order_name_property,
        order_options,
    })
}

/// The built-in `?sort=` tokens a declared `order-by key=` may not reuse:
/// `default` (the "As curated" row) plus every token `Sort::parse_wire`
/// recognises. MUST stay in sync with `Sort::parse_wire` and the ordering
/// control's built-in rows.
const RESERVED_ORDER_KEYS: &[&str] = &["default", "name", "name-desc", "class", "id"];

/// Lower one `resource-list { gloss … }` child.
///
/// - `gloss property="…"` with no children ⇒ [`Gloss::Property`] (a missing or wrong-typed
///   `property` reports as before, through `require_string_attr`).
/// - No `property=` and child nodes ⇒ [`Gloss::Composed`]: each `part property="…" unit="…"` and at
///   most one `teaser property="…"`; any other child is `UnknownChildNode { component: "gloss" }`.
///
/// `InvalidAttributeType` for: `property=` together with children, a
/// composed block with no `part`, a second `teaser`, a repeated part
/// property. `unit=` ids and the parts' value types are checked later
/// (`check_gloss_units`, `check_gloss_part_types`).
fn lower_gloss(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<Gloss> {
    let has_property = raw.attrs.iter().any(|(k, _)| k == "property");
    if raw.children.is_empty() || has_property {
        let property = require_string_attr(raw, "property", report)?;
        if !raw.children.is_empty() {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                "gloss takes either property=\"…\" or part/teaser children, not both",
                raw.span,
            ));
            return None;
        }
        return Some(Gloss::Property(property));
    }

    let mut parts: Vec<GlossPart> = Vec::new();
    let mut teaser: Option<String> = None;
    let mut ok = true;
    for child in &raw.children {
        match child.name.as_str() {
            "part" => match lower_gloss_part(child, &parts, "gloss", report) {
                Some(part) => parts.push(part),
                None => ok = false,
            },
            "teaser" => {
                let Some(property) = require_string_attr(child, "property", report) else {
                    ok = false;
                    continue;
                };
                if teaser.is_some() {
                    report.push(Diagnostic::error(
                        DiagnosticKind::InvalidAttributeType,
                        "gloss has more than one 'teaser'; only one is allowed",
                        child.span,
                    ));
                    ok = false;
                    continue;
                }
                teaser = Some(property);
            }
            _ => {
                report.push(Diagnostic::error(
                    DiagnosticKind::UnknownChildNode { component: "gloss".to_owned(), child: child.name.clone() },
                    format!("gloss has an unknown child node '{}'; expected part or teaser", child.name),
                    child.span,
                ));
                ok = false;
            }
        }
    }
    if parts.is_empty() {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidAttributeType,
            "a composed gloss needs at least one 'part'",
            raw.span,
        ));
        return None;
    }
    ok.then_some(Gloss::Composed { parts, teaser })
}

/// Lower a `resource-detail`'s header slots: the `alt-title-property=` and
/// `author-property=` attributes (reporting `get_typed_string_attr`, the
/// `title-property` idiom) and at most one `imprint { part … }` child, whose
/// parts follow the composed-gloss `part` grammar ([`lower_gloss_part`]). An
/// `imprint` with no `part` is `InvalidAttributeType`; any other child in it
/// is `UnknownChildNode { component: "imprint" }`; a second `imprint` is
/// `InvalidAttributeType`. `None` when no slot is declared. `unit=` ids and
/// the slots' value types are checked later (`check_gloss_units`,
/// `check_head_slot_types`).
///
/// Reports, so never call it from a `*_into_raw` sibling.
fn lower_head_slots(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<HeadSlots> {
    let alt_title = get_typed_string_attr(raw, "alt-title-property", report);
    let author = get_typed_string_attr(raw, "author-property", report);
    let mut imprints = raw.children.iter().filter(|c| c.name == "imprint");
    let imprint = imprints.next().map(|block| lower_imprint(block, report)).unwrap_or_default();
    for extra in imprints {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidAttributeType,
            "resource-detail has more than one 'imprint'; only one is allowed",
            extra.span,
        ));
    }
    if alt_title.is_none() && author.is_none() && imprint.is_empty() {
        return None;
    }
    Some(HeadSlots { alt_title, author, imprint })
}

/// Lower one `resource-list { order-by key="…" property="…" direction="…"
/// label="…" }` child. `earlier` is the list's already-lowered options, for the
/// duplicate check.
///
/// Every attribute goes through a *reporting* reader: `key`,
/// `property` and `label` are required (`require_string_attr`), `direction`
/// is optional (`get_typed_string_attr`, `asc` when absent) — so a
/// wrong-typed value (`key=#true`) reports `InvalidAttributeType` instead of
/// vanishing. A key outside `[a-z0-9-]+` is `InvalidOrderKey`, a built-in
/// key `ReservedOrderKey`, a repeat within the list `DuplicateOrderKey`; each
/// such option is dropped (the boot fails anyway). The `label` id is checked
/// against the chrome block later, by `check_order_labels`.
fn lower_order_by(
    raw: &crate::config::Component,
    earlier: &[OrderOption],
    report: &mut ValidationReport,
) -> Option<OrderOption> {
    let key = require_string_attr(raw, "key", report);
    let property = require_string_attr(raw, "property", report);
    let label = require_string_attr(raw, "label", report);
    let direction = match get_typed_string_attr(raw, "direction", report) {
        None => Some(SortDir::Asc),
        Some(ref s) if s == "asc" => Some(SortDir::Asc),
        Some(ref s) if s == "desc" => Some(SortDir::Desc),
        Some(other) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!("order-by 'direction' must be \"asc\" or \"desc\", got \"{other}\""),
                raw.span,
            ));
            None
        }
    };
    let key = key?;

    let well_formed = !key.is_empty() && key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !well_formed {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidOrderKey { key: key.clone() },
            format!(
                "resource-list order-by key '{key}' must be non-empty and use only \
                 lowercase ASCII letters, digits and '-'"
            ),
            raw.span,
        ));
        return None;
    }
    if RESERVED_ORDER_KEYS.contains(&key.as_str()) {
        report.push(Diagnostic::error(
            DiagnosticKind::ReservedOrderKey { key: key.clone() },
            format!(
                "resource-list order-by key '{key}' is a built-in ordering \
                 (default, name, name-desc, class, id); choose another key"
            ),
            raw.span,
        ));
        return None;
    }
    if earlier.iter().any(|o| o.key == key) {
        report.push(Diagnostic::error(
            DiagnosticKind::DuplicateOrderKey { key: key.clone() },
            format!("resource-list declares more than one order-by with key '{key}'"),
            raw.span,
        ));
        return None;
    }

    Some(OrderOption {
        key,
        property: property?,
        direction: direction?,
        label: label?,
    })
}

/// `UnknownOrderLabel` for every `resource-list { order-by label="…" }` whose
/// id is not a declared `chrome { string }` id. Config-internal but needs the
/// lowered chrome block, so `validate_inner` runs it beside
/// `check_group_labels`, inside the `site_result` guard and before the
/// `validate_with_data` fork — both arms see it, no `*_into_raw` sibling
/// needed. A hard-fail: the engine has no default word to fall back to.
fn check_order_labels(pages: &[Page], chrome: &[ChromeString], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            let Component::ResourceList { order_options, .. } = component else {
                continue;
            };
            for option in order_options {
                if chrome.iter().any(|cs| cs.id == option.label) {
                    continue;
                }
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::UnknownOrderLabel { label: option.label.clone(), page: page.route.clone() },
                    format!(
                        "page '{}': resource-list order-by '{}' label '{}' is not a declared \
                         chrome string id",
                        page.route, option.key, option.label
                    ),
                ));
            }
        }
    }
}

/// Every composed-gloss and imprint `unit=` must be a declared chrome-string
/// id (`UnknownGlossUnit`), and every one of its translations must carry the
/// `{n}` placeholder the number is substituted into
/// (`GlossUnitMissingPlaceholder`, one per offending language).
fn check_gloss_units(pages: &[Page], chrome: &[ChromeString], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            let (parts, surface): (&[GlossPart], &str) = match component {
                Component::ResourceList { gloss: Some(Gloss::Composed { parts, .. }), .. } => {
                    (parts, "resource-list gloss")
                }
                Component::ResourceDetail { head: Some(head), .. } => (&head.imprint, "resource-detail imprint"),
                Component::EditionCards { imprint, .. } => (imprint, "edition-cards imprint"),
                _ => continue,
            };
            for unit in parts.iter().filter_map(|p| p.unit.as_deref()) {
                let Some(string) = chrome.iter().find(|cs| cs.id == unit) else {
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::UnknownGlossUnit { unit: unit.to_owned(), page: page.route.clone() },
                        format!(
                            "page '{}': {surface} unit '{unit}' is not a declared \
                             chrome string id",
                            page.route
                        ),
                    ));
                    continue;
                };
                for (lang, text) in &string.translations {
                    if text.contains("{n}") {
                        continue;
                    }
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::GlossUnitMissingPlaceholder { unit: unit.to_owned(), lang: lang.clone() },
                        format!(
                            "{surface} unit '{unit}' has no '{{n}}' placeholder in its \
                             '{lang}' translation \"{text}\", so the number would never render"
                        ),
                    ));
                }
            }
        }
    }
}

/// `UnknownFigureLabel` for every figure-block slot `count=` or `more-label=`
/// that names no declared chrome string. A hard-fail, unlike the advisory
/// `UnknownGroupLabel`: a missing label silently drops the count line or the
/// more link rather than falling back. Config-internal, run inside the
/// `site_result` guard before the fork (the `check_gloss_units` placement).
fn check_figure_labels(pages: &[Page], chrome: &[ChromeString], report: &mut ValidationReport) {
    for page in pages {
        for slot in page.figure_slots.iter().flatten() {
            let labels = [
                ("count", slot.count.as_deref()),
                ("more-label", slot.more.as_ref().map(|m| m.label.as_str())),
            ];
            for (attribute, label) in labels {
                let Some(label) = label else {
                    continue;
                };
                if chrome.iter().any(|cs| cs.id == label) {
                    continue;
                }
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::UnknownFigureLabel { label: label.to_owned(), page: page.route.clone() },
                    format!(
                        "page '{}': the figure block of section '{}' has {attribute}=\"{label}\", \
                         which is not a declared chrome string id",
                        page.route, slot.section
                    ),
                ));
            }
        }
    }
}

/// `UnresolvedFigureMoreRoute` for every figure-block slot `more-route=` that
/// is parameterised (a more link cannot fill `:id`) or matches no declared
/// page route — the `check_nav_routes` rule. Config-internal, run after the
/// pages are lowered and before the fork.
fn check_figure_more_routes(pages: &[Page], declared_routes: &[&str], report: &mut ValidationReport) {
    for page in pages {
        for slot in page.figure_slots.iter().flatten() {
            let Some(more) = &slot.more else {
                continue;
            };
            // A more link has no resource to fill a `:param` with.
            if more.route.contains(':') {
                report.push(Diagnostic::error_spanless(
                    DiagnosticKind::UnresolvedFigureMoreRoute { route: more.route.clone(), page: page.route.clone() },
                    format!(
                        "page '{}': the figure block of section '{}' has more-route=\"{}\", a \
                         parameterised route; a more link needs a route without ':param'",
                        page.route, slot.section, more.route
                    ),
                ));
                continue;
            }
            check_route_declared(
                &format!("page '{}': the figure block of section '{}'", page.route, slot.section),
                &more.route,
                declared_routes,
                DiagnosticKind::UnresolvedFigureMoreRoute { route: more.route.clone(), page: page.route.clone() },
                report,
            );
        }
    }
}

/// `DuplicateGroupedProperty` for every `resource-detail` on one page: a
/// property named by a `group` that is already named by `show-properties`, by
/// an earlier group, or earlier in the same group. One diagnostic per
/// repeated property name (never short-circuits). Duplicates *within*
/// `show-properties` alone are out of scope.
fn check_duplicate_grouped_properties(body: &[Component], route: &str, report: &mut ValidationReport) {
    for component in body {
        let Component::ResourceDetail { show_properties, groups, .. } = component else {
            continue;
        };
        let mut seen: Vec<&str> = show_properties.iter().flatten().map(String::as_str).collect();
        let mut reported: Vec<&str> = Vec::new();
        for member in groups.iter().flat_map(|g| g.properties.iter()) {
            let member = member.as_str();
            if seen.contains(&member) {
                if !reported.contains(&member) {
                    reported.push(member);
                    report.push(Diagnostic::error_spanless(
                        DiagnosticKind::DuplicateGroupedProperty {
                            property: member.to_owned(),
                            page: route.to_owned(),
                        },
                        format!(
                            "page '{route}': resource-detail property '{member}' appears in \
                             more than one `group` (or in a `group` and `show-properties`); \
                             each property may be shown at most once"
                        ),
                    ));
                }
            } else {
                seen.push(member);
            }
        }
    }
}

/// `DuplicateNarrativeFor` for every `resource-detail` on one page: one
/// diagnostic per resource id named by more than one `narrative-for` entry
/// (never short-circuits).
fn check_duplicate_narrative_for(body: &[Component], route: &str, report: &mut ValidationReport) {
    for component in body {
        let Component::ResourceDetail { narrative_for, .. } = component else {
            continue;
        };
        let mut seen: Vec<&str> = Vec::new();
        let mut reported: Vec<&str> = Vec::new();
        for entry in narrative_for {
            let id = entry.resource_id.as_str();
            if !seen.contains(&id) {
                seen.push(id);
                continue;
            }
            if reported.contains(&id) {
                continue;
            }
            reported.push(id);
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::DuplicateNarrativeFor { resource_id: id.to_owned(), page: route.to_owned() },
                format!(
                    "page '{route}': resource-detail declares more than one `narrative-for` \
                     entry for resource '{id}'; each resource may have at most one intro"
                ),
            ));
        }
    }
}

/// The grouped `incoming-links note-label=` chrome check: an
/// undeclared id is `UnknownIncomingLinksNoteLabel` (Warning — the list
/// renders without its note, the `UnknownIncomingLinksLabel` tier); a declared
/// one is `IncomingLinksTextMissingPlaceholder` (Error) once per language whose
/// text lacks `{n}` (the `check_gloss_units` tier). Only a grouped spec carries
/// a note label, so an ungrouped `note-label=` is never checked (it is ignored).
fn check_incoming_links_note_label(note: &str, route: &str, chrome: &[ChromeString], report: &mut ValidationReport) {
    let Some(string) = chrome.iter().find(|cs| cs.id == note) else {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownIncomingLinksNoteLabel { label: note.to_owned(), page: route.to_owned() },
            message: format!(
                "page '{route}': resource-detail incoming-links note-label '{note}' is not a \
                 declared chrome string id; the list renders without its note"
            ),
            span: None,
            severity: Severity::Warning,
        });
        return;
    };
    for (lang, text) in &string.translations {
        if text.contains("{n}") {
            continue;
        }
        report.push(Diagnostic::error_spanless(
            DiagnosticKind::IncomingLinksTextMissingPlaceholder { label: note.to_owned(), lang: lang.clone() },
            format!(
                "incoming-links note-label '{note}' has no '{{n}}' placeholder in its '{lang}' \
                 translation \"{text}\", so the row count would never render"
            ),
        ));
    }
}

/// `UnknownGroupLabel` for every `resource-detail` `group` whose `label=` is
/// not a declared `chrome { string }` id. Config-internal but needs the
/// lowered chrome block, so `validate_inner` runs it inside its
/// `site_result` guard, before the `validate_with_data` fork (the
/// `check_class_plurals` placement) — both arms see it, no `*_into_raw`
/// sibling needed. Advisory: the group renders under
/// `ChromeMsg::PropertyGroupTitle` instead.
///
/// The same pass also checks the `incoming-links` child's `label=`: an
/// undeclared id is `UnknownIncomingLinksLabel` (Warning) — the list falls back
/// to the source class's capitalised plural. A group's `empty=` and
/// `links-label=` ids follow the same rule (`UnknownGroupEmptyLabel`,
/// `UnknownGroupLinksLabel`).
fn check_group_labels(pages: &[Page], chrome: &[ChromeString], report: &mut ValidationReport) {
    for page in pages {
        for component in &page.body {
            // A `compound-viewer`'s `member-links` caption follows the
            // `group` rule: an undeclared id is `UnknownGroupLabel`.
            if let Component::CompoundViewer { member_links: Some(links), .. } = component {
                if !chrome.iter().any(|cs| cs.id == links.label) {
                    report.push(Diagnostic {
                        kind: DiagnosticKind::UnknownGroupLabel {
                            label: links.label.clone(),
                            page: page.route.clone(),
                        },
                        message: format!(
                            "page '{}': compound-viewer member-links label '{}' is not a \
                             declared chrome string id; the row falls back to the default \
                             caption",
                            page.route, links.label
                        ),
                        span: None,
                        severity: Severity::Warning,
                    });
                }
                continue;
            }
            let Component::ResourceDetail { groups, incoming_links, .. } = component else {
                continue;
            };
            if let Some(spec) = incoming_links {
                if !chrome.iter().any(|cs| cs.id == spec.label) {
                    report.push(Diagnostic {
                        kind: DiagnosticKind::UnknownIncomingLinksLabel {
                            label: spec.label.clone(),
                            page: page.route.clone(),
                        },
                        message: format!(
                            "page '{}': resource-detail incoming-links label '{}' is not a \
                             declared chrome string id; the list falls back to the source \
                             class's plural as its caption",
                            page.route, spec.label
                        ),
                        span: None,
                        severity: Severity::Warning,
                    });
                }
                if let Some(note) = spec.grouping.as_ref().and_then(|g| g.note_label.as_deref()) {
                    check_incoming_links_note_label(note, &page.route, chrome, report);
                }
            }
            for group in groups {
                if let Some(links) = group.links_label.as_deref() {
                    if !chrome.iter().any(|cs| cs.id == links) {
                        report.push(Diagnostic {
                            kind: DiagnosticKind::UnknownGroupLinksLabel {
                                label: links.to_owned(),
                                page: page.route.clone(),
                            },
                            message: format!(
                                "page '{}': resource-detail group links-label '{}' is not a \
                                 declared chrome string id; the links row falls back to the \
                                 default caption",
                                page.route, links
                            ),
                            span: None,
                            severity: Severity::Warning,
                        });
                    }
                }
                if let Some(empty) = group.empty.as_deref() {
                    if !chrome.iter().any(|cs| cs.id == empty) {
                        report.push(Diagnostic {
                            kind: DiagnosticKind::UnknownGroupEmptyLabel {
                                label: empty.to_owned(),
                                page: page.route.clone(),
                            },
                            message: format!(
                                "page '{}': resource-detail group empty label '{}' is not a \
                                 declared chrome string id; an all-absent group renders nothing",
                                page.route, empty
                            ),
                            span: None,
                            severity: Severity::Warning,
                        });
                    }
                }
                if chrome.iter().any(|cs| cs.id == group.label) {
                    continue;
                }
                report.push(Diagnostic {
                    kind: DiagnosticKind::UnknownGroupLabel { label: group.label.clone(), page: page.route.clone() },
                    message: format!(
                        "page '{}': resource-detail group label '{}' is not a declared \
                         chrome string id; the group falls back to the default caption",
                        page.route, group.label
                    ),
                    span: None,
                    severity: Severity::Warning,
                });
            }
        }
    }
}

fn lower_resource_detail(
    raw: &crate::config::Component,
    page_params: &[String],
    skip_param_check: bool,
    ctx: &ValidationContext<'_>,
    default_language: Option<&str>,
    report: &mut ValidationReport,
) -> Option<Component> {
    // Reject any child node this component does not recognise — the only
    // names `lower_resource_detail` and its child lowerers ever look up by
    // `c.name ==`. Config-internal, so it reaches both `validate_with_data`
    // arms through the shared report before the fork. Keep lowering the rest
    // of the component regardless — every other mistake in it must still
    // report, and this fn must still return `Some`.
    const RESOURCE_DETAIL_CHILDREN: &[&str] =
        &["show-properties", "group", "narrative-for", "incoming-links", "imprint"];
    for child in &raw.children {
        if !RESOURCE_DETAIL_CHILDREN.contains(&child.name.as_str()) {
            report.push(Diagnostic::error(
                DiagnosticKind::UnknownChildNode {
                    component: "resource-detail".to_owned(),
                    child: child.name.clone(),
                },
                format!(
                    "resource-detail has an unknown child node '{}'; expected one of show-properties, group, narrative-for, incoming-links, imprint",
                    child.name
                ),
                child.span,
            ));
        }
    }

    // `id` is optional at this layer (required only outside a `:id` route,
    // which is a data-dependent check). A present-but-wrong-typed
    // value is `InvalidAttributeType` (`get_typed_string_attr`), never
    // silently absent.
    let id = get_typed_string_attr(raw, "id", report);

    if !skip_param_check {
        if let Some(ref val) = id {
            check_param_refs_in_value(val, page_params, raw.span, report);
        }
    }

    // `show-properties` is a child node with positional string args.
    let show_properties = raw.children.iter().find(|c| c.name == "show-properties").map(|c| {
        c.args
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    });

    // `title-property` is an optional string attribute naming the property
    // whose value is used as the resource-detail page `<h1>` title. A
    // present-but-wrong-typed value is `InvalidAttributeType`
    // (`get_typed_string_attr`), never silently absent.
    let title_property = get_typed_string_attr(raw, "title-property", report);

    // `class` is an optional string attribute declaring the Class rendered by
    // this detail page — used to build the class→route map for clickable
    // cross-resource links. Absent ⇒ `None` (backward-compatible). A
    // present-but-wrong-typed value is `InvalidAttributeType`
    // (`get_typed_string_attr`), never silently absent.
    let class = get_typed_string_attr(raw, "class", report);

    // `media` is an optional string attribute naming the host→image `link`
    // property that feeds this resource's record viewer.
    // Data-existence/type/target-class/resolvable-class checks
    // happen in `check_resource_detail_media`; parsing here just captures
    // the raw value. A present-but-wrong-typed value is `InvalidAttributeType`
    // (`get_typed_string_attr`), never silently absent.
    let media = get_typed_string_attr(raw, "media", report);

    // `group` — repeatable child node, in declaration order: a required
    // `label=` chrome-string id plus positional string property names (the
    // `show-properties` shape). A missing or wrong-typed `label` is reported by
    // `require_string_attr` and that group is skipped. Duplicate members
    // (`DuplicateGroupedProperty`) are checked in `check_and_lower_pages`,
    // which knows the page route; the label's existence (`UnknownGroupLabel`)
    // in `validate_inner`, which has the lowered chrome block.
    let groups = raw
        .children
        .iter()
        .filter(|c| c.name == "group")
        .filter_map(|c| {
            let label = require_string_attr(c, "label", report)?;
            let empty = get_typed_string_attr(c, "empty", report);
            let links_label = get_typed_string_attr(c, "links-label", report);
            Some(PropertyGroup {
                label,
                properties: positional_string_args(c),
                empty,
                links_label,
            })
        })
        .collect();

    // `narrative-for` — repeatable child node: a required first positional
    // string arg (the literal resource id) plus a required `file=`. The file
    // runs the exact checks a page-level `narrative` does
    // (`check_narrative_file`). No `$param` interpolation in either value.
    // Duplicate ids (`DuplicateNarrativeFor`) are checked in
    // `check_and_lower_pages`, which knows the page route; resource existence
    // (`UnknownNarrativeForResource`) is data-dependent.
    let narrative_for = raw
        .children
        .iter()
        .filter(|c| c.name == "narrative-for")
        .filter_map(|c| {
            let resource_id = require_first_positional_string(c, report);
            let file = require_string_attr(c, "file", report);
            let file_ok = file
                .as_deref()
                .is_some_and(|f| check_narrative_file(f, ctx, default_language, c.span, report));
            match (resource_id, file) {
                (Some(resource_id), Some(file)) if file_ok => Some(NarrativeFor { resource_id, file }),
                _ => None,
            }
        })
        .collect();

    // `incoming-links` — at most one child node.
    let incoming_links = lower_incoming_links(raw, report);

    let head = lower_head_slots(raw, report);

    Some(Component::ResourceDetail {
        id,
        show_properties,
        title_property,
        class,
        media,
        groups,
        narrative_for,
        incoming_links,
        head,
        // A page-level fact: `check_and_lower_pages` sets it once the whole
        // body is lowered (it needs the sibling `compound-viewer`s).
        navigator: false,
    })
}

/// Lower a `resource-detail`'s `incoming-links` child: `label=` and `class=`
/// are required strings (`require_string_attr`);
/// `order=` is an optional closed vocabulary read with the REPORTING
/// `get_typed_string_attr` (`name` default · `id` · `compound-seq` — a
/// wrong value or a non-string is `InvalidAttributeType`, never a silent
/// fall-back to `name`); the positional string args are the link properties
/// (the `show-properties`/`group` filter), and zero of them is
/// `IncomingLinksWithoutProperty`. A second (or later) child is
/// `DuplicateIncomingLinks` and is not lowered. Every attribute of the first
/// child is read before bailing, so one child with several mistakes reports
/// all of them. The label's chrome-id existence is checked later, in
/// `validate_inner` (`check_group_labels`), which has the lowered chrome.
///
/// Grouping takes the optional `group-by=` (`via` or a source
/// property), `layout=` (`cards`|`columns`, default `columns`), `note-label=`
/// (a chrome id), `row-label=`, `row-aside=` and `sort-property=`, all read
/// with `get_typed_string_attr`. `layout=`/`note-label=` are ignored without
/// `group-by=`. With `group-by=` and no `limit=`, the limit resolves to
/// `GROUPED_INCOMING_LIMIT_CEILING`.
///
/// Reports, so never call it from a `*_into_raw` sibling.
fn lower_incoming_links(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<IncomingLinksSpec> {
    let mut children = raw.children.iter().filter(|c| c.name == "incoming-links");
    let first = children.next()?;
    for extra in children {
        report.push(Diagnostic::error(
            DiagnosticKind::DuplicateIncomingLinks,
            "resource-detail declares more than one `incoming-links` child; \
             each record may have at most one incoming-links list",
            extra.span,
        ));
    }

    let label = require_string_attr(first, "label", report);
    let class = require_string_attr(first, "class", report);

    // Grouping + row anatomy — every attribute read with the
    // REPORTING readers before any early return below, so one child with
    // several mistakes reports all of them. `layout=`/`note-label=` without
    // `group-by=` are read (a wrong type or an unknown `layout` value still
    // reports) but then ignored — documented, no "option without grouping"
    // diagnostic. Their data-dependent checks live in
    // `check_single_incoming_links` (both arms); the `note-label` chrome
    // check in `check_group_labels`.
    let group_by = optional_string_attr(first, "group-by", report).map(|opt| {
        opt.map(|s| {
            if s == "via" {
                IncomingGroupBy::Via
            } else {
                IncomingGroupBy::Property(s)
            }
        })
    });
    let layout_present = first.attrs.iter().any(|(k, _)| k == "layout");
    let layout = match get_typed_string_attr(first, "layout", report) {
        None if !layout_present => Some(IncomingLayout::default()),
        // Present but not a string: `get_typed_string_attr` already reported.
        None => None,
        Some(ref s) if s == "cards" => Some(IncomingLayout::Cards),
        Some(ref s) if s == "columns" => Some(IncomingLayout::Columns),
        Some(other) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!("incoming-links 'layout' must be \"cards\" or \"columns\", got \"{other}\""),
                first.span,
            ));
            None
        }
    };
    let note_label = optional_string_attr(first, "note-label", report);
    let row_label = optional_string_attr(first, "row-label", report);
    let row_aside = optional_string_attr(first, "row-aside", report);
    let sort_property = optional_string_attr(first, "sort-property", report);

    let order_present = first.attrs.iter().any(|(k, _)| k == "order");
    let order = match get_typed_string_attr(first, "order", report) {
        None if !order_present => Some(IncomingOrder::Name),
        // Present but not a string: `get_typed_string_attr` already reported.
        None => None,
        Some(ref s) if s == "name" => Some(IncomingOrder::Name),
        Some(ref s) if s == "id" => Some(IncomingOrder::Id),
        Some(ref s) if s == "compound-seq" => Some(IncomingOrder::CompoundSeq),
        Some(other) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "incoming-links 'order' must be \"name\", \"id\" or \"compound-seq\", \
                     got \"{other}\""
                ),
                first.span,
            ));
            None
        }
    };

    let mut properties = positional_string_args(first);
    let mut seen = std::collections::HashSet::new();
    properties.retain(|p| seen.insert(p.clone()));
    if properties.is_empty() {
        report.push(Diagnostic::error(
            DiagnosticKind::IncomingLinksWithoutProperty,
            "resource-detail `incoming-links` names no link property; list at least \
             one property as a positional string argument",
            first.span,
        ));
        return None;
    }

    // `limit=<int>` — optional, config-internal (no raw-fallback sibling
    // needed). Absent ⇒ DEFAULT_INCOMING_LINKS_LIMIT, or
    // GROUPED_INCOMING_LIMIT_CEILING when `group-by=` is set (resolved here
    // because the query cannot tell a defaulted 50 from an explicit
    // `limit=50`). Present ⇒ always honoured; must be a positive integer that
    // fits u32 (never an `as` cast — the `timeline::timeline_axis_year`
    // convention).
    let default_limit = if matches!(group_by, Ok(Some(_))) {
        GROUPED_INCOMING_LIMIT_CEILING
    } else {
        DEFAULT_INCOMING_LINKS_LIMIT
    };
    let limit_present = first.attrs.iter().any(|(k, _)| k == "limit");
    let limit = match get_typed_integer_attr(first, "limit", report) {
        None if !limit_present => Some(default_limit),
        // Present but not an integer: `get_typed_integer_attr` already reported.
        None => None,
        Some(n) if n >= 1 => match u32::try_from(n).ok() {
            Some(limit) => Some(limit),
            None => {
                report.push(Diagnostic::error(
                    DiagnosticKind::InvalidAttributeType,
                    "incoming-links limit must be a positive integer \u{2264} 4294967295",
                    first.span,
                ));
                None
            }
        },
        Some(_) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                "incoming-links limit must be a positive integer \u{2264} 4294967295",
                first.span,
            ));
            None
        }
    };

    let layout = layout?;
    let group_by = group_by.ok()?;
    let note_label = note_label.ok()?;
    let row_label = row_label.ok()?;
    let row_aside = row_aside.ok()?;
    let sort_property = sort_property.ok()?;
    let grouping = group_by.map(|by| IncomingGrouping { by, layout, note_label });

    Some(IncomingLinksSpec {
        label: label?,
        class: class?,
        properties,
        order: order?,
        limit: limit?,
        grouping,
        row_label,
        row_aside,
        sort_property,
    })
}

fn lower_compound_viewer(
    raw: &crate::config::Component,
    page_params: &[String],
    skip_param_check: bool,
    report: &mut ValidationReport,
) -> Option<Component> {
    // `source`, `layout` and `index` are all required. Each is read before
    // any bail-out so a declaration missing several reports all of them at
    // once, not only the first.
    let source = require_string_attr(raw, "source", report);

    // `layout` and `index` are required, closed-vocabulary attributes: absent ⇒
    // `MissingRequiredAttribute`, wrong type or unknown value ⇒
    // `InvalidAttributeType` naming the vocabulary (the `chip kind=`
    // precedent). Either error skips the whole component — no default layout or
    // index exists anywhere in the engine. Both readers REPORT, so neither may
    // be reached from a `*_into_raw` sibling (none is needed: the checks are
    // config-internal).
    let layout = require_string_attr(raw, "layout", report).and_then(|v| match v.as_str() {
        "contents" => Some(CompoundLayout::Contents),
        "pages" => Some(CompoundLayout::Pages),
        other => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!("compound-viewer 'layout' must be \"contents\" or \"pages\", got \"{other}\""),
                raw.span,
            ));
            None
        }
    });
    let index = require_string_attr(raw, "index", report).and_then(|v| match v.as_str() {
        "rail" => Some(MemberIndex::Rail),
        "collapsed" => Some(MemberIndex::Collapsed),
        "none" => Some(MemberIndex::Off),
        other => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "compound-viewer 'index' must be \"rail\", \"collapsed\" or \"none\", \
                     got \"{other}\""
                ),
                raw.span,
            ));
            None
        }
    });

    let source = source?;

    if !skip_param_check {
        check_param_refs_in_value(&source, page_params, raw.span, report);
    }

    // `media` is an optional string attribute naming the member→image `link`
    // property whose first target is shown as the member's scan. Read like
    // `resource-detail`'s own `media`, with the REPORTING
    // `get_typed_string_attr`: a present-but-wrong-typed value is
    // `InvalidAttributeType`, never silently absent. The data-dependent checks
    // live in `check_compound_viewer_media` and its raw sibling.
    let media = get_typed_string_attr(raw, "media", report);

    // `picker` (book-page Choice E): an optional closed vocabulary, read with
    // the REPORTING `get_typed_string_attr` — a wrong type or an unknown
    // value is `InvalidAttributeType` naming the vocabulary (the `layout=`
    // precedent), and the component lowers without a picker. A picker on a
    // non-`pages` layout is `PagesLayoutOnly`, checked per page in
    // `check_and_lower_pages`.
    let picker = get_typed_string_attr(raw, "picker", report).and_then(|v| match v.as_str() {
        "numbered" => Some(PickerMode::Numbered),
        "signatures" => Some(PickerMode::Signatures),
        other => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "compound-viewer 'picker' must be \"numbered\" or \"signatures\", \
                     got \"{other}\""
                ),
                raw.span,
            ));
            None
        }
    });

    // `member-links`: at most one child, the `group`
    // shape — a required `label=` chrome id plus positional property names.
    // The label's existence is checked in `check_group_labels`; the
    // properties' existence and `link` type are data-dependent
    // (`check_single_member_links`, both arms).
    let member_links = lower_member_links(raw, report);

    // `annotations=` (dsp-incubator
    // cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md): a reporting boolean read,
    // like `grid=`. On a non-`pages` layout it is `PagesLayoutOnly`, checked per page in
    // `check_and_lower_pages`.
    let annotations = get_bool_attr(raw, "annotations", report);

    // Bail only now, after every attribute has had its chance to report.
    let (layout, index) = (layout?, index?);

    Some(Component::CompoundViewer {
        source,
        media,
        picker,
        layout,
        index,
        // Set for real by `check_and_lower_pages`'s post-lowering pass
        // (mirrors `ResourceDetail.navigator` — a single-component lowering
        // fn cannot see its page siblings).
        paired: false,
        member_links,
        annotations,
    })
}

/// Lower a `compound-viewer`'s `member-links` child. A second child is
/// `UnknownChildNode` (only one row exists) and is not lowered; a missing or
/// wrong-typed `label=` is reported by `require_string_attr` and the row is
/// dropped. Reports, so never call it from a `*_into_raw` sibling.
fn lower_member_links(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<MemberLinks> {
    let mut children = raw.children.iter().filter(|c| c.name == "member-links");
    let first = children.next()?;
    for extra in children {
        report.push(Diagnostic::error(
            DiagnosticKind::UnknownChildNode {
                component: "compound-viewer".to_owned(),
                child: extra.name.clone(),
            },
            "compound-viewer declares more than one `member-links` child; a reader has one \
             member-links row",
            extra.span,
        ));
    }
    let label = require_string_attr(first, "label", report)?;
    Some(MemberLinks { label, properties: positional_string_args(first) })
}

fn lower_facet_panel(
    raw: &crate::config::Component,
    page_list_ids: &[String],
    report: &mut ValidationReport,
) -> Option<Component> {
    let target = require_string_attr(raw, "target", report)?;

    // UnresolvedTarget — target must match a resource-list id in the same page.
    if !page_list_ids.contains(&target) {
        report.push(Diagnostic::error(
            DiagnosticKind::UnresolvedTarget,
            format!("facet-panel target '{target}' does not match any resource-list id on this page"),
            raw.span,
        ));
        return None;
    }

    // Lower facet children.
    let mut facets = Vec::new();
    for child in &raw.children {
        if child.name == "facet" {
            if let Some(f) = lower_facet(child, report) {
                facets.push(f);
            }
        }
    }

    Some(Component::FacetPanel { target, facets })
}

fn lower_facet(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<Facet> {
    let property = require_string_attr(raw, "property", report)?;
    let kind_str = require_string_attr(raw, "kind", report)?;

    let kind = match FacetKind::parse_config_str(&kind_str) {
        Some(k) => k,
        None => {
            report.push(Diagnostic::error(
                DiagnosticKind::UnknownFacetKind,
                format!(
                    "unknown facet kind '{kind_str}'; expected one of: \
                     text, list, date-decade, date-year, boolean, range"
                ),
                raw.span,
            ));
            return None;
        }
    };

    Some(Facet { property, kind })
}

fn lower_search_bar(
    raw: &crate::config::Component,
    page_params: &[String],
    skip_param_check: bool,
    page_list_ids: &[String],
    report: &mut ValidationReport,
) -> Option<Component> {
    let target = require_string_attr(raw, "target", report)?;

    // UnresolvedTarget — target must match a resource-list id in the same page.
    if !page_list_ids.contains(&target) {
        report.push(Diagnostic::error(
            DiagnosticKind::UnresolvedTarget,
            format!("search-bar target '{target}' does not match any resource-list id on this page"),
            raw.span,
        ));
        return None;
    }

    // `searches` is a list-valued attribute: KDL parses `searches="a" "b"` as
    // named attr `searches="a"` PLUS trailing positional node args `"b"`, …
    // Reconstruct the full list as `[attrs["searches"]] ++ positional_args`.
    // `searches` is required, so the REPORTING `require_string_attr`: an
    // absent value is `MissingRequiredAttribute` (via `require_string_attr`
    // itself), a present-but-wrong-typed value is `InvalidAttributeType`,
    // never a misleading `MissingRequiredAttribute`.
    let first_search = require_string_attr(raw, "searches", report)?;
    let mut searches = vec![first_search];
    // Append positional args (the rest of the list-valued attribute).
    for arg in &raw.args {
        if let Value::String(s) = arg {
            searches.push(s.clone());
        }
    }

    // $param check for searches values.
    if !skip_param_check {
        for s in &searches {
            check_param_refs_in_value(s, page_params, raw.span, report);
        }
    }

    let placeholder = get_typed_string_attr(raw, "placeholder", report);
    let scope = get_typed_string_attr(raw, "scope", report);

    Some(Component::SearchBar { target, searches, placeholder, scope })
}

// ── Data-binding child node lowering ─────────────────────────────────────────

fn lower_where_clause(
    raw: &crate::config::Component,
    page_params: &[String],
    skip_param_check: bool,
    report: &mut ValidationReport,
) -> Option<WhereClause> {
    let property = require_string_attr(raw, "property", report)?;

    // Determine which form: equals, in, or range.
    let has_equals = get_attr(raw, "equals").is_some();
    let has_in = get_attr(raw, "in").is_some();
    let has_range_start = get_attr(raw, "range-start").is_some();
    let has_range_end = get_attr(raw, "range-end").is_some();

    if has_equals {
        let value = get_attr(raw, "equals").expect("has_equals checked above");
        if !skip_param_check {
            if let Value::String(ref s) = value {
                check_param_refs_in_value(s, page_params, raw.span, report);
            }
        }
        Some(WhereClause::Equals { property, value })
    } else if has_in {
        // `in` is a list-valued attribute: `in="v1" "v2"` → named attr "in" + positional args.
        let first = get_attr(raw, "in").expect("has_in checked above");
        let mut values = vec![first];
        for arg in &raw.args {
            values.push(arg.clone());
        }
        if !skip_param_check {
            for v in &values {
                if let Value::String(ref s) = v {
                    check_param_refs_in_value(s, page_params, raw.span, report);
                }
            }
        }
        Some(WhereClause::In { property, values })
    } else if has_range_start || has_range_end {
        let start = get_attr(raw, "range-start");
        let end = get_attr(raw, "range-end");
        if !skip_param_check {
            if let Some(Value::String(ref s)) = start {
                check_param_refs_in_value(s, page_params, raw.span, report);
            }
            if let Some(Value::String(ref s)) = end {
                check_param_refs_in_value(s, page_params, raw.span, report);
            }
        }
        Some(WhereClause::Range { property, start, end })
    } else {
        // No recognised filter form — skip silently (permissive parse).
        None
    }
}

fn lower_sort(raw: &crate::config::Component, report: &mut ValidationReport) -> Option<Sort> {
    let property = require_string_attr(raw, "property", report)?;

    let direction = match get_typed_string_attr(raw, "direction", report) {
        // Absent, or present-but-wrong-typed (`get_typed_string_attr` already
        // reported `InvalidAttributeType` in the latter case; `direction=1`
        // falls back to `Asc` the same as an absent `direction`).
        None => SortDir::Asc,
        Some(ref s) if s == "asc" => SortDir::Asc,
        Some(ref s) if s == "desc" => SortDir::Desc,
        Some(other) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!("sort 'direction' must be \"asc\" or \"desc\", got \"{other}\""),
                raw.span,
            ));
            return None;
        }
    };

    // Config `sort` is always a property (never Class/Id — those only
    // arrive at request time via `?sort=`, parsed by `Sort::parse_wire`).
    Some(Sort { key: SortKey::Property(property), direction })
}

// ── $param resolution ─────────────────────────────────────────────────────────

/// Check a string value for a leading `$` — if present, treat it as a
/// `$paramName` reference and validate against `page_params`.
///
/// Known limitation: a literal value beginning with `$` (e.g. `"$usd"`)
/// is treated as a param reference; there is no escaping in v1.
fn check_param_refs_in_value(value: &str, page_params: &[String], span: Span, report: &mut ValidationReport) {
    if let Some(param_name) = value.strip_prefix('$') {
        if !page_params.iter().any(|p| p == param_name) {
            report.push(Diagnostic::error(
                DiagnosticKind::UndeclaredRouteParam,
                format!(
                    "attribute references route param '${param_name}' \
                     which is not declared in the page route"
                ),
                span,
            ));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::diagnostic::DiagnosticKind;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Parse KDL source and run validation in one call. Panics on KDL syntax
    /// errors (all test fixtures should be syntactically valid).
    ///
    /// Wraps `validate_inner`, discarding the report on success — these unit
    /// tests only assert on the error path.
    fn run(src: &str, ctx: &ValidationContext<'_>) -> Result<ValidatedConfig, ValidationReport> {
        let raw = config::parse(src).expect("test fixture KDL must be syntactically valid");
        match validate_inner(&raw, ctx) {
            (Some(config), _report) => Ok(config),
            (None, report) => Err(report),
        }
    }

    /// Collect the kinds from a failed validation report.
    fn kinds(report: &ValidationReport) -> Vec<DiagnosticKind> {
        report.iter().map(|d| d.kind.clone()).collect()
    }

    // A minimal *valid* config used as a baseline.
    const GOOD_CONFIG: &str = r#"
site {
    title "My Project"
    languages "de" "en"
}
page id="home" route="/"
"#;

    // ── Project-id checks ─────────────────────────────────────────────────────

    #[test]
    fn valid_project_id_passes() {
        let ctx = ValidationContext {
            project_id: "my-project",
            reserved_ids: &["_static"],
            narrative_files: &[],
            asset_files: &[],
        };
        let result = run(GOOD_CONFIG, &ctx);
        assert!(result.is_ok(), "valid id should pass: {:?}", result);
    }

    #[test]
    fn reserved_project_id_fails() {
        let ctx = ValidationContext {
            project_id: "_static",
            reserved_ids: &["_static"],
            narrative_files: &[],
            asset_files: &[],
        };
        let err = run(GOOD_CONFIG, &ctx).expect_err("reserved id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::ReservedProjectId),
            "expected ReservedProjectId, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_project_id_uppercase_fails() {
        let ctx = ValidationContext {
            project_id: "MyProject",
            reserved_ids: &[],
            narrative_files: &[],
            asset_files: &[],
        };
        let err = run(GOOD_CONFIG, &ctx).expect_err("uppercase id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidProjectId),
            "expected InvalidProjectId, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_project_id_leading_hyphen_fails() {
        let ctx = ValidationContext {
            project_id: "-bad",
            reserved_ids: &[],
            narrative_files: &[],
            asset_files: &[],
        };
        let err = run(GOOD_CONFIG, &ctx).expect_err("leading-hyphen id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidProjectId),
            "expected InvalidProjectId, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_project_id_empty_fails() {
        let ctx = ValidationContext {
            project_id: "",
            reserved_ids: &[],
            narrative_files: &[],
            asset_files: &[],
        };
        let err = run(GOOD_CONFIG, &ctx).expect_err("empty id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidProjectId),
            "expected InvalidProjectId, got: {:?}",
            kinds(&err)
        );
    }

    // ── UnexpectedTopLevelNode ─────────────────────────────────────────────────

    #[test]
    fn unexpected_top_level_node_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
widget {}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown top-level node should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnexpectedTopLevelNode),
            "expected UnexpectedTopLevelNode, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn no_unexpected_top_level_node_passes() {
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(GOOD_CONFIG, &ctx);
        assert!(result.is_ok(), "good config should pass: {:?}", result);
    }

    // ── remodel block ─────────────────────────────────────────────────────────

    #[test]
    fn a_config_without_a_remodel_block_has_no_remodel_diagnostic_and_load_remodel_refuses_it() {
        let ctx = ValidationContext::for_tests("my-project");
        let raw = config::parse(GOOD_CONFIG).expect("fixture must parse");
        let (validated, report) = validate_inner(&raw, &ctx);
        assert!(validated.is_some(), "good config should pass: {:?}", kinds(&report));
        assert!(report.is_empty(), "no remodel diagnostic expected: {:?}", kinds(&report));

        match load_remodel(GOOD_CONFIG) {
            Err(ConfigLoadError::Validation(report)) => {
                assert_eq!(kinds(&report), vec![DiagnosticKind::MissingRemodelBlock]);
            }
            other => panic!("expected MissingRemodelBlock, got {other:?}"),
        }
    }

    #[test]
    fn a_second_remodel_block_is_an_unexpected_top_level_node() {
        const BLOCK: &str = r#"remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
"#;
        let src = format!(
            "site {{\n    title \"T\"\n    languages \"de\" \"en\"\n}}\n{BLOCK}{BLOCK}page id=\"home\" route=\"/\"\n"
        );
        let first = src.find("remodel").expect("fixture holds a first block");
        let second = src.rfind("remodel").expect("fixture holds a second block");
        assert_ne!(first, second);

        let ctx = ValidationContext::for_tests("my-project");
        let raw = config::parse(&src).expect("fixture must parse");
        let (_, report) = validate_inner(&raw, &ctx);

        assert_eq!(
            kinds(&report),
            vec![DiagnosticKind::UnexpectedTopLevelNode],
            "only the second block is unexpected"
        );
        let span = report.iter().next().and_then(|d| d.span).expect("the diagnostic is located");
        assert_eq!(
            span.offset, second,
            "the diagnostic starts at the second block's node: {span:?}"
        );
    }

    #[test]
    fn a_faulty_remodel_block_fails_the_validation_of_the_whole_config() {
        let src = r#"site {
    title "T"
    languages "de" "en"
}
remodel {
    shortcode "08G3"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let raw = config::parse(src).expect("fixture must parse");

        let (validated, report) = validate_inner(&raw, &ctx);

        assert!(validated.is_none(), "a faulty block fails the config");
        assert_eq!(kinds(&report), vec![DiagnosticKind::InvalidRemodelValue]);
    }

    #[test]
    fn load_remodel_refuses_a_second_remodel_block_as_the_whole_validation_does() {
        const BLOCK: &str = r#"remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
"#;
        let src = format!("site {{\n    title \"T\"\n    languages \"de\" \"en\"\n}}\n{BLOCK}{BLOCK}");
        let second = src.rfind("remodel").expect("fixture holds a second block");

        match load_remodel(&src) {
            Err(ConfigLoadError::Validation(report)) => {
                assert_eq!(kinds(&report), vec![DiagnosticKind::UnexpectedTopLevelNode]);
                let span = report.iter().next().and_then(|d| d.span);
                assert_eq!(span.map(|span| span.offset), Some(second));
            }
            other => panic!("expected UnexpectedTopLevelNode, got {other:?}"),
        }
    }

    #[test]
    fn load_remodel_holds_a_site_language_that_is_written_twice_once() {
        let src = r#"site {
    title "T"
    languages "de" "en" "de"
}
remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
"#;
        let remodel = load_remodel(src).unwrap_or_else(|e| panic!("the block lowers: {e:?}"));

        assert_eq!(remodel.languages, ["de", "en"]);
    }

    #[test]
    fn load_remodel_returns_the_parse_error_of_a_config_that_is_no_kdl() {
        match load_remodel("remodel {") {
            Err(ConfigLoadError::Parse(_)) => {}
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    /// The site has no title, an unknown node stands at the top level and the
    /// page names an unknown component: boot refuses each, and none decides
    /// what the mapping is.
    #[test]
    fn load_remodel_lowers_a_valid_block_beside_blocks_that_do_not_validate() {
        let src = r#"site {
    languages "de" "en"
}
widget {}
remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
page id="home" route="/" {
    no-such-component
}
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let raw = config::parse(src).expect("fixture must parse");
        let (validated, _) = validate_inner(&raw, &ctx);
        assert!(validated.is_none(), "the config as a whole is refused");

        let remodel = load_remodel(src).unwrap_or_else(|e| panic!("the block lowers: {e:?}"));

        assert_eq!(remodel.shortcode.as_str(), "0803");
    }

    #[test]
    fn load_remodel_refuses_a_config_that_declares_no_site_language() {
        let src = r#"site {
    title "T"
}
remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
page id="home" route="/"
"#;
        match load_remodel(src) {
            Err(ConfigLoadError::Validation(report)) => {
                assert!(
                    kinds(&report).contains(&DiagnosticKind::MissingSiteLanguages),
                    "expected MissingSiteLanguages, got: {:?}",
                    kinds(&report)
                );
            }
            other => panic!("expected MissingSiteLanguages, got {other:?}"),
        }
    }

    #[test]
    fn load_remodel_returns_the_lowered_block_of_a_valid_config() {
        let src = r#"site {
    title "T"
    languages "de" "en"
}
remodel {
    shortcode "0803"
    class "Book" iri="http://example.org/onto#Book" { id from="iri" }
}
page id="home" route="/"
"#;
        let remodel = match load_remodel(src) {
            Ok(remodel) => remodel,
            Err(e) => panic!("expected a lowered remodel, got {e:?}"),
        };
        assert_eq!(remodel.shortcode.as_str(), "0803");
        assert_eq!(remodel.classes.len(), 1);
        assert_eq!(remodel.classes[0].name.as_str(), "Book");
    }

    // ── MissingSiteTitle ──────────────────────────────────────────────────────

    #[test]
    fn missing_site_title_fails() {
        let src = r#"
site {
    languages "de" "en"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("missing title should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingSiteTitle),
            "expected MissingSiteTitle, got: {:?}",
            kinds(&err)
        );
    }

    // ── MissingSiteLanguages ──────────────────────────────────────────────────

    #[test]
    fn missing_languages_fails() {
        let src = r#"
site {
    title "T"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("missing languages should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingSiteLanguages),
            "expected MissingSiteLanguages, got: {:?}",
            kinds(&err)
        );
    }

    // ── MissingRequiredLanguage ───────────────────────────────────────────────

    #[test]
    fn missing_de_language_fails() {
        let src = r#"
site {
    title "T"
    languages "en" "fr"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("missing 'de' should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::MissingRequiredLanguage),
            "expected MissingRequiredLanguage, got: {:?}",
            k
        );
    }

    #[test]
    fn missing_en_language_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "fr"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("missing 'en' should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::MissingRequiredLanguage),
            "expected MissingRequiredLanguage, got: {:?}",
            k
        );
    }

    // ── InvalidFieldType ──────────────────────────────────────────────────────

    #[test]
    fn non_string_language_entry_fails() {
        // languages entry is an integer, not a string
        let src = r#"
site {
    title "T"
    languages 42 "en"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("non-string language should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::InvalidFieldType),
            "expected InvalidFieldType for non-string language, got: {:?}",
            k
        );
    }

    #[test]
    fn non_string_theme_value_fails() {
        // Non-string value for a theme key must produce `InvalidFieldType`.
        // (accent with an integer value, not a string.)
        let src = r##"
site {
    title "T"
    languages "de" "en"
    theme {
        accent 42
    }
}
page id="home" route="/"
"##;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("non-string theme value should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::InvalidFieldType),
            "expected InvalidFieldType for non-string theme value, got: {:?}",
            k
        );
    }

    // ── site { date-precision-word } lowering ─────

    #[test]
    fn date_precision_word_true_reaches_site_config() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    date-precision-word #true
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("#true is a valid date-precision-word");
        assert!(config.site.date_precision_word);
    }

    #[test]
    fn narrative_layout_site_values_reach_site_config() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    narrative-companion "text"
    narrative-side "start"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("both values are in their vocabulary");
        assert_eq!(config.site.narrative_companion, NarrativeCompanion::Text);
        assert_eq!(config.site.narrative_side, NarrativeSide::Start);
    }

    #[test]
    fn narrative_layout_site_values_default_to_rail_and_end() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("a site block without the keys is valid");
        assert_eq!(config.site.narrative_companion, NarrativeCompanion::Rail);
        assert_eq!(config.site.narrative_side, NarrativeSide::End);
        assert_eq!(config.site.text_align, crate::ir::TextAlign::Start);
    }

    #[test]
    fn narrative_side_site_value_non_string_is_invalid_field_type() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    narrative-side #true
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let report = run(src, &ctx).expect_err("a boolean narrative-side must fail");
        let kinds: Vec<&DiagnosticKind> = report.iter().map(|d| &d.kind).collect();
        assert_eq!(kinds, vec![&DiagnosticKind::InvalidFieldType]);
    }

    /// A `site {}` block holding `text_align_line`, lowered.
    fn lower_with_text_align(text_align_line: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            "site {{\n    title \"T\"\n    languages \"de\" \"en\"\n    {text_align_line}\n}}\n\
             page id=\"home\" route=\"/\"\n"
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    #[test]
    fn text_align_justify_reaches_site_config() {
        let config = lower_with_text_align(r#"text-align "justify""#).expect("\"justify\" is in the vocabulary");
        assert_eq!(config.site.text_align, TextAlign::Justify);
    }

    /// `"start"` states the default: it lowers to the same value as an absent
    /// key, which is what emits no `data-align`.
    #[test]
    fn text_align_start_lowers_to_the_default() {
        let config = lower_with_text_align(r#"text-align "start""#).expect("\"start\" is in the vocabulary");
        assert_eq!(config.site.text_align, TextAlign::Start);
        assert_eq!(config.site.text_align, TextAlign::default());
    }

    #[test]
    fn text_align_unknown_value_is_unknown_site_value() {
        let report =
            lower_with_text_align(r#"text-align "book""#).expect_err("a value outside the vocabulary must fail");
        let kinds: Vec<&DiagnosticKind> = report.iter().map(|d| &d.kind).collect();
        assert_eq!(
            kinds,
            vec![&DiagnosticKind::UnknownSiteValue {
                key: "text-align".to_owned(),
                value: "book".to_owned(),
                allowed: vec!["start".to_owned(), "justify".to_owned()],
            }]
        );
    }

    #[test]
    fn text_align_non_string_is_invalid_field_type() {
        let report = lower_with_text_align("text-align #true").expect_err("a boolean text-align must fail");
        let kinds: Vec<&DiagnosticKind> = report.iter().map(|d| &d.kind).collect();
        assert_eq!(kinds, vec![&DiagnosticKind::InvalidFieldType]);
    }

    /// `companion="rail"` is accepted on a page that ends up with fewer than
    /// two rail entries: the count depends on the narrative's content per
    /// language, so boot cannot reject it.
    #[test]
    fn narrative_companion_rail_lowers_without_a_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/" {
    narrative file="one-section.md" companion="rail"
}
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("companion=\"rail\" is valid on any page");
        assert_eq!(
            config.pages[0].body,
            vec![Component::Narrative {
                file: "one-section.md".to_owned(),
                companion: Some(NarrativeCompanion::Rail),
                side: None,
            }]
        );
    }

    /// `side=` beside `companion="text"` is accepted: the side takes effect
    /// once the companion is switched back to the rail.
    #[test]
    fn narrative_companion_text_with_side_lowers_without_a_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/" {
    narrative file="home.md" companion="text" side="start"
    narrative file="more.md"
}
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("companion=\"text\" side=\"start\" is valid");
        assert_eq!(
            config.pages[0].body[0],
            Component::Narrative {
                file: "home.md".to_owned(),
                companion: Some(NarrativeCompanion::Text),
                side: Some(NarrativeSide::Start),
            }
        );
    }

    #[test]
    fn date_precision_word_defaults_to_false() {
        let absent = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
"#;
        let explicit = r#"
site {
    title "T"
    languages "de" "en"
    date-precision-word #false
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        for src in [absent, explicit] {
            let config = run(src, &ctx).expect("valid config");
            assert!(!config.site.date_precision_word);
        }
    }

    #[test]
    fn date_precision_word_non_bool_is_invalid_field_type() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    date-precision-word "yes"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("a string date-precision-word must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidFieldType]);
    }

    // ── site { footer-attribution, not-found } ──

    #[test]
    fn g11_overrides_reach_site_config() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    footer-attribution "Scans: UB Basel"
    nav {
        item label="Books" route="/books"
    }
    not-found {
        browse route="/books"
        browse route="/strips" label="nf-strips"
    }
}
page id="home" route="/"
page id="books" route="/books"
page id="strips" route="/strips"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("valid G11 overrides");
        assert_eq!(config.site.footer_attribution.as_deref(), Some("Scans: UB Basel"));
        assert_eq!(
            config.site.not_found_browse,
            vec![
                NotFoundBrowse { route: "/books".to_owned(), label: None },
                NotFoundBrowse {
                    route: "/strips".to_owned(),
                    label: Some("nf-strips".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn g11_overrides_absent_leave_defaults() {
        // dsp-incubator cpe/docs/adr/0002-deployment-topology.md: absent config never fails boot
        // and changes nothing.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("valid config");
        assert!(config.site.footer_attribution.is_none());
        assert!(config.site.not_found_browse.is_empty());
    }

    #[test]
    fn g11_empty_not_found_block_is_valid_and_empty() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    not-found {
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("an empty not-found block is valid");
        assert!(config.site.not_found_browse.is_empty());
    }

    #[test]
    fn g11_non_string_footer_is_invalid_field_type() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    footer-attribution #true
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("a non-string override must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidFieldType]);
    }

    #[test]
    fn g11_browse_route_must_be_declared() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    not-found {
        browse route="/nowhere"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("undeclared browse route must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::UnresolvedNotFoundBrowseRoute]);
        assert!(
            err.iter().next().expect("one diagnostic").message.contains("'/nowhere'"),
            "message names the route: {}",
            err.iter().next().expect("one diagnostic").message
        );
    }

    #[test]
    fn g11_more_than_three_browse_links_is_an_error() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    not-found {
        browse route="/"
        browse route="/"
        browse route="/"
        browse route="/"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("four browse links must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::TooManyNotFoundBrowseLinks]);
        assert!(
            err.iter().next().expect("one diagnostic").span.is_some(),
            "reported at the not-found node"
        );
    }

    #[test]
    fn g11_browse_attribute_types_report() {
        // A missing route, a wrong-typed route and a wrong-typed label each
        // report — never a silent drop (the reporting-reader rule).
        let src = r#"
site {
    title "T"
    languages "de" "en"
    not-found {
        browse label="x"
        browse route=1
        browse route="/" label=#true
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("bad browse attributes must fail");
        assert_eq!(
            kinds(&err),
            vec![
                DiagnosticKind::MissingRequiredAttribute,
                DiagnosticKind::InvalidAttributeType,
                DiagnosticKind::InvalidAttributeType,
            ]
        );
    }

    // ── ThemeKnob lowering ────────────────────────────────────────────────────

    #[test]
    fn theme_knob_valid_accent_and_background_passes() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        accent "ochre"
        background "tint"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "valid theme block should pass: {:?}", result);
        let config = result.unwrap();
        use crate::theme::{Accent, Background};
        assert_eq!(config.site.theme.accent, Accent::Ochre, "accent should be Ochre");
        assert_eq!(config.site.theme.background, Background::Tint, "background should be Tint");
    }

    #[test]
    fn theme_knob_valid_typeface_passes() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        typeface "spectral"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "valid typeface should pass: {:?}", result);
        assert_eq!(result.unwrap().site.theme.typeface, Typeface::Spectral);
    }

    #[test]
    fn unknown_typeface_value_produces_error_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        typeface "comic-sans"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown typeface value should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "typeface" && value == "comic-sans"
            )),
            "expected UnknownThemeValue for typeface=comic-sans, got: {:?}",
            k
        );
    }

    // ── Reserved-route (`_`-prefixed) check ────────────────────────────────────

    #[test]
    fn reserved_underscore_route_fails() {
        // A config page must not claim a `_`-prefixed route — the namespace
        // is reserved for engine-registered routes.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
page id="cfg" route="/_foo"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("reserved `_`-route should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::ReservedRoute),
            "expected ReservedRoute for a `/_foo` page route, got: {:?}",
            k
        );
    }

    #[test]
    fn normal_routes_are_not_reserved() {
        assert!(!route_first_segment_is_reserved("/"));
        assert!(!route_first_segment_is_reserved("/about"));
        assert!(!route_first_segment_is_reserved("/characters/:id"));
        assert!(route_first_segment_is_reserved("/_foo"));
        assert!(route_first_segment_is_reserved("/_static/x"));
        // A `_` deeper in the path is also reserved (defensive).
        assert!(route_first_segment_is_reserved("/books/_x"));
    }

    #[test]
    fn absent_theme_block_yields_default_knob() {
        // GOOD_CONFIG has no theme block — should yield ThemeKnob::default().
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(GOOD_CONFIG, &ctx);
        assert!(result.is_ok(), "config without theme block should pass");
        let config = result.unwrap();
        use crate::theme::{Accent, Background};
        assert_eq!(
            config.site.theme.accent,
            Accent::Ochre,
            "absent theme should default to Ochre accent"
        );
        assert_eq!(
            config.site.theme.background,
            Background::Tint,
            "absent theme should default to Tint background"
        );
    }

    #[test]
    fn unknown_accent_value_produces_error_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        accent "purple"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown accent value should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "accent" && value == "purple"
            )),
            "expected UnknownThemeValue for accent=purple, got: {:?}",
            k
        );
    }

    #[test]
    fn unknown_background_value_produces_error_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        background "warm"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown background value should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "background" && value == "warm"
            )),
            "expected UnknownThemeValue for background=warm, got: {:?}",
            k
        );
    }

    #[test]
    fn unknown_theme_key_produces_error_diagnostic() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        font-family "Garamond"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown theme key should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, .. }
                    if key == "font-family"
            )),
            "expected UnknownThemeValue for key=font-family, got: {:?}",
            k
        );
    }

    #[test]
    fn unknown_theme_value_accumulates_with_other_errors() {
        // An unknown accent value AND a missing title should both appear —
        // the diagnostic accumulator must not short-circuit.
        let src = r#"
site {
    languages "de" "en"
    theme {
        accent "purple"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("multiple errors should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::MissingSiteTitle),
            "MissingSiteTitle must be present in accumulated errors: {:?}",
            k
        );
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "accent" && value == "purple"
            )),
            "UnknownThemeValue must be present in accumulated errors: {:?}",
            k
        );
        assert!(k.len() >= 2, "at least 2 errors must accumulate, got: {:?}", k);
    }

    /// Both a bad accent AND a bad background in the same `theme {}` block must each
    /// produce a diagnostic — the accumulator must run through the whole block without
    /// short-circuiting after the first unknown value.
    #[test]
    fn both_bad_theme_values_both_produce_diagnostics() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    theme {
        accent "purple"
        background "warm"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("two bad theme values should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "accent" && value == "purple"
            )),
            "UnknownThemeValue for accent=purple must be present in: {:?}",
            k
        );
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::UnknownThemeValue { key, value, .. }
                    if key == "background" && value == "warm"
            )),
            "UnknownThemeValue for background=warm must be present in: {:?}",
            k
        );
        assert!(
            k.len() >= 2,
            "both theme errors must accumulate (at least 2 diagnostics), got: {:?}",
            k
        );
    }

    // ── UnknownDefaultLanguage ────────────────────────────────────────────────

    #[test]
    fn unknown_default_language_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    default-language "fr"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("unknown default-language should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnknownDefaultLanguage),
            "expected UnknownDefaultLanguage, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn valid_default_language_passes() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    default-language "de"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "valid default-language should pass: {:?}", result);
        assert_eq!(result.unwrap().site.default_language, "de");
    }

    // ── DuplicatePageId ───────────────────────────────────────────────────────

    #[test]
    fn duplicate_page_id_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
page id="home" route="/about"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("duplicate page id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::DuplicatePageId),
            "expected DuplicatePageId, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn unique_page_ids_pass() {
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(GOOD_CONFIG, &ctx);
        assert!(result.is_ok(), "unique page ids should pass: {:?}", result);
    }

    // ── DuplicatePageRoute ────────────────────────────────────────────────────

    #[test]
    fn duplicate_page_route_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/"
page id="about" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("duplicate page route should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::DuplicatePageRoute),
            "expected DuplicatePageRoute, got: {:?}",
            kinds(&err)
        );
    }

    // ── IncompleteChromeTranslation ───────────────────────────────────────────

    #[test]
    fn incomplete_chrome_translation_fails() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    chrome {
        string "search-placeholder" {
            de "Suchen"
        }
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("incomplete chrome should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::IncompleteChromeTranslation),
            "expected IncompleteChromeTranslation, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn complete_chrome_translation_passes() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    chrome {
        string "search-placeholder" {
            de "Suchen"
            en "Search"
        }
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "complete chrome should pass: {:?}", result);
    }

    // ── IncompleteLabelTranslation ────────────────────────────────────────────

    #[test]
    fn incomplete_label_translation_fails_for_property() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    property "role" {
        de "Rolle"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("incomplete property label should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::IncompleteLabelTranslation {
                target: crate::diagnostic::LabelTarget::Property,
                key: "role".to_owned(),
                lang: "en".to_owned(),
            }),
            "expected IncompleteLabelTranslation, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn incomplete_label_translation_fails_for_class() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Character" {
        en "Character"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("incomplete class label should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::IncompleteLabelTranslation {
                target: crate::diagnostic::LabelTarget::Class,
                key: "Character".to_owned(),
                lang: "de".to_owned(),
            }),
            "expected IncompleteLabelTranslation, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn complete_label_translation_passes() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    property "role" {
        de "Rolle"
        en "Role"
    }
    class "Character" {
        de "Figur"
        en "Character"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "complete labels should pass: {:?}", result);
        let config = result.expect("checked ok above");
        assert_eq!(config.site.property_labels.len(), 1);
        assert_eq!(config.site.property_labels[0].property, "role");
        assert_eq!(config.site.class_labels.len(), 1);
        assert_eq!(config.site.class_labels[0].class, "Character");
    }

    // ── `plural` completeness ──

    #[test]
    fn incomplete_plural_label_translation_fails_for_class() {
        // A `plural` block missing a declared language must fire the SAME
        // `IncompleteLabelTranslation` kind as a missing singular
        // translation (reused, not a new DiagnosticKind).
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Character" {
        en "Character"
        de "Figur"
        plural {
            en "characters"
        }
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("incomplete plural block should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::IncompleteLabelTranslation {
                target: crate::diagnostic::LabelTarget::Class,
                key: "Character".to_owned(),
                lang: "de".to_owned(),
            }),
            "expected IncompleteLabelTranslation, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn complete_plural_label_translation_passes() {
        // A `plural` block declaring every site language must load clean and
        // populate `ClassLabel.plural`.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Character" {
        en "Character"
        de "Figur"
        plural {
            en "characters"
            de "Figuren"
        }
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(result.is_ok(), "complete plural block should pass: {:?}", result);
        let config = result.expect("checked ok above");
        assert_eq!(config.site.class_labels.len(), 1);
        assert_eq!(
            config.site.class_labels[0].plural,
            vec![
                ("en".to_owned(), "characters".to_owned()),
                ("de".to_owned(), "Figuren".to_owned())
            ]
        );
    }

    #[test]
    fn class_label_with_no_plural_block_still_loads_clean() {
        // A class label with NO `plural` block at all must LOAD (it raises the
        // advisory `MissingClassPlural` Warning, asserted by
        // `class_label_with_no_plural_block_warns`, never an Error). This
        // proves the plural-COMPLETENESS check never fires on an absent (not
        // merely incomplete) `plural` block.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Character" {
        en "Character"
        de "Figur"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(src, &ctx);
        assert!(
            result.is_ok(),
            "class label with no plural block should pass unaffected: {:?}",
            result
        );
        let config = result.expect("checked ok above");
        assert_eq!(config.site.class_labels.len(), 1);
        assert!(
            config.site.class_labels[0].plural.is_empty(),
            "no `plural` child declared ⇒ ClassLabel.plural must be empty"
        );
    }

    // ── MissingClassPlural (advisory) ─────────────────────────────────────────

    /// Run validation and return the report regardless of outcome — the
    /// Warning-only checks (`MissingClassPlural`) leave `run`'s `Ok` arm,
    /// which discards the report.
    fn report_of(src: &str, ctx: &ValidationContext<'_>) -> ValidationReport {
        let raw = config::parse(src).expect("test fixture KDL must be syntactically valid");
        validate_inner(&raw, ctx).1
    }

    #[test]
    fn class_label_with_no_plural_block_warns() {
        // A declared class with no `plural { … }` degrades to the singular on
        // every plural-bearing surface ("Alle Buch durchsuchen"), so it earns
        // one advisory Warning — never an Error.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Book" {
        en "Book"
        de "Buch"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(src, &ctx);
        assert!(!report.has_errors(), "MissingClassPlural must not block boot: {report:?}");
        let hits: Vec<_> = report
            .iter()
            .filter(|d| d.kind == DiagnosticKind::MissingClassPlural { class: "Book".to_owned() })
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "expected exactly 1 MissingClassPlural for 'Book', got: {:?}",
            kinds(&report)
        );
        assert_eq!(hits[0].severity, Severity::Warning, "MissingClassPlural must be advisory");
    }

    #[test]
    fn class_label_with_plural_block_does_not_warn() {
        // The non-firing case: a declared `plural { … }` block silences it.
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
labels {
    class "Book" {
        en "Book"
        de "Buch"
        plural {
            en "books"
            de "Bücher"
        }
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(src, &ctx);
        assert!(
            !kinds(&report)
                .iter()
                .any(|k| matches!(k, DiagnosticKind::MissingClassPlural { .. })),
            "a declared plural block must not warn, got: {:?}",
            kinds(&report)
        );
    }

    // ── Multi-error accumulation ──────────────────────────────────────────────

    #[test]
    fn multiple_site_errors_accumulate() {
        // Missing title AND missing languages — both errors should appear.
        let src = r#"
site {
    description "oops no title, no languages"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(src, &ctx).expect_err("multiple errors should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::MissingSiteTitle),
            "expected MissingSiteTitle in: {:?}",
            k
        );
        assert!(
            k.contains(&DiagnosticKind::MissingSiteLanguages),
            "expected MissingSiteLanguages in: {:?}",
            k
        );
        assert!(k.len() >= 2, "expected at least 2 errors, got: {:?}", k);
    }

    #[test]
    fn project_id_and_site_errors_accumulate() {
        // Invalid project id AND missing title — both errors should appear.
        let src = r#"
site {
    languages "de" "en"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext {
            project_id: "BAD_ID",
            reserved_ids: &[],
            narrative_files: &[],
            asset_files: &[],
        };
        let err = run(src, &ctx).expect_err("multiple errors should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::InvalidProjectId),
            "expected InvalidProjectId in: {:?}",
            k
        );
        assert!(
            k.contains(&DiagnosticKind::MissingSiteTitle),
            "expected MissingSiteTitle in: {:?}",
            k
        );
    }

    // ── Good config → Ok(ValidatedConfig) ─────────────────────────────────────

    #[test]
    fn good_config_returns_ok_validated_config() {
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(GOOD_CONFIG, &ctx);
        let validated = result.expect("good config should produce Ok(ValidatedConfig)");
        assert_eq!(validated.site.title, "My Project");
        assert_eq!(validated.site.languages, vec!["de", "en"]);
        // default_language falls back to first declared language
        assert_eq!(validated.site.default_language, "de");
        assert_eq!(validated.pages.len(), 1);
        let page = &validated.pages[0];
        assert_eq!(page.id, "home");
        assert_eq!(page.route, "/");
        assert!(page.body.is_empty(), "bodies empty until Step 3");
    }

    // ── Route param extraction ────────────────────────────────────────────────

    #[test]
    fn route_params_extracted_correctly() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="detail" route="/books/:id/chapters/:chapter"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let validated = run(src, &ctx).expect("valid config should pass");
        let page = &validated.pages[0];
        assert_eq!(page.params, vec!["id", "chapter"]);
    }

    #[test]
    fn route_with_no_params_gives_empty_params() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="home" route="/books"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let validated = run(src, &ctx).expect("valid config should pass");
        let page = &validated.pages[0];
        assert!(page.params.is_empty(), "no params in /books");
    }

    // ── is_valid_project_id ───────────────────────────────────────────────────

    #[test]
    fn valid_project_ids() {
        assert!(is_valid_project_id("a"));
        assert!(is_valid_project_id("abc"));
        assert!(is_valid_project_id("my-project"));
        assert!(is_valid_project_id("project123"));
        assert!(is_valid_project_id("0abc"));
        assert!(is_valid_project_id("alice-in-daschland"));
    }

    #[test]
    fn invalid_project_ids() {
        assert!(!is_valid_project_id(""));
        assert!(!is_valid_project_id("-bad"));
        assert!(!is_valid_project_id("Bad"));
        assert!(!is_valid_project_id("bad_id"));
        assert!(!is_valid_project_id("bad id"));
        assert!(!is_valid_project_id("BAD"));
        assert!(!is_valid_project_id("_static"));
    }

    // ── Component lowering tests ─────────────────────────────────────

    // Minimal good config helper for the lowering tests.
    const GOOD_SITE: &str = r#"
site {
    title "Test Project"
    languages "de" "en"
}
"#;

    fn good_ctx<'a>(narrative_files: &'a [String]) -> ValidationContext<'a> {
        ValidationContext {
            project_id: "my-project",
            reserved_ids: &[],
            narrative_files,
            asset_files: &[],
        }
    }

    // ── UnknownComponent ──────────────────────────────────────────────────────

    #[test]
    fn unknown_component_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    widget class="Foo"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("unknown component should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnknownComponent),
            "expected UnknownComponent, got: {:?}",
            kinds(&err)
        );
    }

    // ── MissingRequiredAttribute ──────────────────────────────────────────────

    #[test]
    fn missing_required_attr_narrative_fails() {
        // narrative without `file`
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("missing narrative file should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
            "expected MissingRequiredAttribute, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn missing_required_attr_resource_list_fails() {
        // resource-list without `class`
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("missing resource-list class should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
            "expected MissingRequiredAttribute, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn missing_required_attr_search_bar_fails() {
        // search-bar without required `searches`
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("missing search-bar searches should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::MissingRequiredAttribute),
            "expected MissingRequiredAttribute, got: {:?}",
            k
        );
    }

    #[test]
    fn missing_required_attr_compound_viewer_fails() {
        // compound-viewer without `source`
        let src = format!(
            r#"{}
page id="home" route="/:id" {{
    compound-viewer layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("missing compound-viewer source should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
            "expected MissingRequiredAttribute, got: {:?}",
            kinds(&err)
        );
    }

    // ── InvalidAttributeType ─────────────────────────────────────────────────

    #[test]
    fn invalid_attr_type_resource_list_class_integer_fails() {
        // class must be a string; passing an integer is a type error.
        // In KDL, `class=42` is an integer attribute.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class=42
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("integer class should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_attr_type_sort_direction_fails() {
        // sort direction must be "asc" or "desc".
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        sort property="title" direction="sideways"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("bad sort direction should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for direction, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_attr_type_grid_string_fails() {
        // `grid` must be a KDL boolean; a string value (`grid="true"`) must
        // itself be reported by `get_bool_attr` as InvalidAttributeType — not
        // silently lowered to `false` with no boot error.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" grid="true"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("string grid value should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for grid=\"true\", got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_attr_type_grid_integer_fails() {
        // `grid=1` (integer) must likewise be rejected, not silently
        // truthy-coerced.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" grid=1
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("integer grid value should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for grid=1, got: {:?}",
            kinds(&err)
        );
    }

    /// `order-name-label`/`order-name-desc-label` read via the reporting
    /// `get_typed_string_attr`, so a wrong-typed value reports instead of
    /// silently vanishing.
    #[test]
    fn invalid_attr_type_order_name_labels_fail() {
        for attr in ["order-name-label", "order-name-desc-label"] {
            let src = format!(
                r#"{}
page id="home" route="/" {{
    resource-list class="Book" {attr}=#true
}}
"#,
                GOOD_SITE
            );
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err("a boolean label id should fail");
            assert!(
                kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
                "expected InvalidAttributeType for {attr}=#true, got: {:?}",
                kinds(&err)
            );
        }
    }

    // ── Composed gloss lowering ───────────────────────────────────────────────

    /// A site declaring the unit string `gloss-extent` (`{n}` in both
    /// languages) and `bad-unit` (no `{n}` in `en`).
    const GLOSS_SITE: &str = r#"
site {
    title "Test Project"
    languages "de" "en"
    chrome {
        string "gloss-extent" {
            de "{n} Seiten"
            en "{n} pages"
        }
        string "bad-unit" {
            de "{n} Seiten"
            en "pages"
        }
    }
}
"#;

    /// Lower `body` (the `resource-list`'s children) and return the result.
    fn run_gloss(body: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            "{GLOSS_SITE}\npage id=\"home\" route=\"/\" {{\n    resource-list class=\"Book\" {{\n{body}\n    }}\n}}\n"
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    fn lowered_gloss(config: &ValidatedConfig) -> Option<Gloss> {
        match &config.pages[0].body[0] {
            Component::ResourceList { gloss, .. } => gloss.clone(),
            other => panic!("expected a resource-list, got {other:?}"),
        }
    }

    #[test]
    fn gloss_property_lowers_to_the_legacy_form() {
        let config = run_gloss(r#"gloss property="description""#).expect("valid");
        assert_eq!(lowered_gloss(&config), Some(Gloss::Property("description".to_owned())));
    }

    #[test]
    fn gloss_block_lowers_to_the_composed_form() {
        let config = run_gloss(
            r#"gloss {
            part property="publisher"
            part property="extent" unit="gloss-extent"
            teaser property="teaser"
        }"#,
        )
        .expect("valid");
        assert_eq!(
            lowered_gloss(&config),
            Some(Gloss::Composed {
                parts: vec![
                    GlossPart { property: "publisher".to_owned(), unit: None },
                    GlossPart {
                        property: "extent".to_owned(),
                        unit: Some("gloss-extent".to_owned()),
                    },
                ],
                teaser: Some("teaser".to_owned()),
            })
        );
    }

    #[test]
    fn gloss_errors_are_invalid_attribute_type() {
        for (what, body) in [
            (
                "property= and children",
                r#"gloss property="description" { part property="publisher" }"#,
            ),
            ("no part", r#"gloss { teaser property="teaser" }"#),
            (
                "a second teaser",
                r#"gloss {
                part property="publisher"
                teaser property="teaser"
                teaser property="note"
            }"#,
            ),
            (
                "a repeated part",
                r#"gloss {
                part property="publisher"
                part property="publisher"
            }"#,
            ),
            ("a wrong-typed unit", r#"gloss { part property="extent" unit=#true }"#),
        ] {
            let err = run_gloss(body).expect_err(what);
            assert_eq!(
                kinds(&err),
                vec![DiagnosticKind::InvalidAttributeType],
                "{what}: got {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn gloss_unknown_child_is_unknown_child_node() {
        let err = run_gloss(r#"gloss { part property="publisher"; prat property="x" }"#).expect_err("unknown child");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownChildNode { component: "gloss".to_owned(), child: "prat".to_owned() }]
        );
    }

    #[test]
    fn gloss_unit_must_be_a_declared_chrome_id() {
        let err = run_gloss(r#"gloss { part property="extent" unit="no-such-unit" }"#).expect_err("undeclared unit");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownGlossUnit { unit: "no-such-unit".to_owned(), page: "/".to_owned() }]
        );
    }

    #[test]
    fn gloss_unit_needs_the_placeholder_in_every_language() {
        let err = run_gloss(r#"gloss { part property="extent" unit="bad-unit" }"#).expect_err("unit without {n}");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::GlossUnitMissingPlaceholder { unit: "bad-unit".to_owned(), lang: "en".to_owned() }]
        );
    }

    // ── Record header slots ───────────────────────────────────────────────────

    /// Lower a `resource-detail` with `attrs` and the children `body`.
    fn run_head(attrs: &str, body: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            "{GLOSS_SITE}\npage id=\"book\" route=\"/books/:id\" {{\n    resource-detail id=\"$id\" class=\"Book\" {attrs} {{\n{body}\n    }}\n}}\n"
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    fn lowered_head(config: &ValidatedConfig) -> Option<HeadSlots> {
        match &config.pages[0].body[0] {
            Component::ResourceDetail { head, .. } => head.clone(),
            other => panic!("expected a resource-detail, got {other:?}"),
        }
    }

    #[test]
    fn head_slots_lower_every_form() {
        let config = run_head(
            r#"alt-title-property="alt-title" author-property="creator""#,
            r#"imprint {
            part property="pub-place"
            part property="extent" unit="gloss-extent"
        }"#,
        )
        .expect("valid");
        assert_eq!(
            lowered_head(&config),
            Some(HeadSlots {
                alt_title: Some("alt-title".to_owned()),
                author: Some("creator".to_owned()),
                imprint: vec![
                    GlossPart { property: "pub-place".to_owned(), unit: None },
                    GlossPart {
                        property: "extent".to_owned(),
                        unit: Some("gloss-extent".to_owned()),
                    },
                ],
            })
        );
        // One slot alone is enough; none at all lowers to `None`.
        let author_only = run_head(r#"author-property="creator""#, "").expect("valid");
        assert_eq!(
            lowered_head(&author_only),
            Some(HeadSlots { author: Some("creator".to_owned()), ..HeadSlots::default() })
        );
        let none = run_head("", r#"show-properties "name""#).expect("valid");
        assert_eq!(lowered_head(&none), None);
    }

    #[test]
    fn head_slot_mistakes_are_invalid_attribute_type() {
        for (what, attrs, body) in [
            ("a wrong-typed alt title", "alt-title-property=#true", ""),
            ("a wrong-typed author", "author-property=1", ""),
            ("an empty imprint", "", "imprint {}"),
            (
                "a repeated imprint part",
                "",
                r#"imprint { part property="pub-place"; part property="pub-place" }"#,
            ),
            (
                "a second imprint",
                "",
                r#"imprint { part property="pub-place" }
                imprint { part property="publisher" }"#,
            ),
        ] {
            let err = run_head(attrs, body).expect_err(what);
            assert_eq!(
                kinds(&err),
                vec![DiagnosticKind::InvalidAttributeType],
                "{what}: got {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn imprint_teaser_is_an_unknown_child_node() {
        let err = run_head("", r#"imprint { part property="pub-place"; teaser property="teaser" }"#)
            .expect_err("teaser in imprint");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownChildNode { component: "imprint".to_owned(), child: "teaser".to_owned() }]
        );
    }

    #[test]
    fn imprint_unit_must_be_a_declared_chrome_id() {
        let err =
            run_head("", r#"imprint { part property="extent" unit="no-such-unit" }"#).expect_err("undeclared unit");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownGlossUnit {
                unit: "no-such-unit".to_owned(),
                page: "/books/:id".to_owned(),
            }]
        );
    }

    #[test]
    fn invalid_attr_type_chip_kind_fails() {
        // `chip kind` must be "term" or "keyword" — a closed vocabulary,
        // no new DiagnosticKind.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        chip property="office" kind="chip"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("bad chip kind should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for chip kind, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn invalid_attr_type_chip_kind_wrong_typed_fails() {
        // A present-but-wrong-TYPED `kind` (a bool, not a wrong-valued string)
        // must also be `InvalidAttributeType`, not silently swallowed into the
        // cardinality-derived fallback with no boot error at all. Mirrors
        // `invalid_attr_type_grid_bool_fails`/`_integer_fails` above: this
        // lowering site reads `kind` with `get_typed_string_attr`, not
        // `get_string_attr`.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        chip property="office" kind=#true
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed chip kind should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for chip kind=#true, got: {:?}",
            kinds(&err)
        );
    }

    // ── Wrong-typed optional string attributes in lowering ──────────
    //
    // Every optional string attribute read inside a `lower_*` fn goes through
    // the REPORTING `get_typed_string_attr` (or, for `search-bar`'s required
    // `searches`, `require_string_attr`): absent ⇒ `None`, no diagnostic;
    // present-but-wrong-typed ⇒ `InvalidAttributeType`. These tests pin that
    // contract at every such site, plus the two guard tests for the "absent
    // stays clean" and "one cascade is accepted" halves.

    /// Every kind in the report is `InvalidAttributeType` (order-insensitive,
    /// any count ≥ 1) — used where a single config exercises more than one
    /// wrong-typed attribute in one component.
    fn assert_only_invalid_attribute_type(err: &ValidationReport) {
        let k = kinds(err);
        assert!(!k.is_empty(), "expected at least one diagnostic, got none");
        assert!(
            k.iter().all(|kind| *kind == DiagnosticKind::InvalidAttributeType),
            "expected every diagnostic to be InvalidAttributeType, got: {k:?}"
        );
    }

    #[test]
    fn resource_detail_wrong_typed_optional_strings_are_invalid_attribute_type() {
        for attr in ["id=1", "title-property=#true", "class=42", "media=7"] {
            let src = format!(
                r#"{}
page id="home" route="/" {{
    resource-detail {attr}
}}
"#,
                GOOD_SITE
            );
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err(&format!("resource-detail {attr} should fail validation"));
            assert_only_invalid_attribute_type(&err);
        }
    }

    #[test]
    fn resource_list_wrong_typed_id_and_media_are_invalid_attribute_type() {
        for attrs in ["id=1 class=\"Book\"", "class=\"Book\" media=#false"] {
            let src = format!(
                r#"{}
page id="home" route="/" {{
    resource-list {attrs}
}}
"#,
                GOOD_SITE
            );
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err(&format!("resource-list {attrs} should fail validation"));
            assert_only_invalid_attribute_type(&err);
        }
    }

    #[test]
    fn compound_viewer_wrong_typed_media_and_picker_are_invalid_attribute_type() {
        let err = run_compound_viewer(r#"source="$id" layout="pages" index="none" media=7 picker=#true"#)
            .expect_err("wrong-typed media/picker should fail");
        assert_only_invalid_attribute_type(&err);
    }

    #[test]
    fn compound_viewer_unknown_picker_names_the_vocabulary() {
        let err = run_compound_viewer(r#"source="$id" layout="pages" index="none" media="image" picker="gatherings""#)
            .expect_err("an unknown picker mode should fail");
        let d = err
            .iter()
            .find(|d| d.kind == DiagnosticKind::InvalidAttributeType)
            .unwrap_or_else(|| panic!("expected InvalidAttributeType, got: {:?}", kinds(&err)));
        assert!(
            d.message.contains(r#""numbered" or "signatures", got "gatherings""#),
            "{}",
            d.message
        );
    }

    #[test]
    fn compound_viewer_picker_on_contents_layout_fails() {
        let err = run_compound_viewer(r#"source="$id" layout="contents" index="rail" picker="numbered""#)
            .expect_err("a picker on the contents layout should fail");
        assert!(
            err.iter().any(|d| matches!(
                &d.kind,
                DiagnosticKind::PagesLayoutOnly { feature, page }
                    if feature == "picker" && page == "/view/:id"
            )),
            "expected PagesLayoutOnly for the picker, got: {:?}",
            kinds(&err)
        );
    }

    /// `member-links` renders only under the `pages` pager, so on the
    /// `contents` layout it fails like a `picker` does, naming its feature.
    #[test]
    fn compound_viewer_member_links_on_contents_layout_fails() {
        let src = format!(
            r#"{GOOD_SITE}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="contents" index="rail" {{
        member-links label="strips" "strip-left"
    }}
}}
"#
        );
        let err = run(&src, &ValidationContext::for_tests("my-project"))
            .expect_err("member-links on the contents layout should fail");
        let found: Vec<_> = err
            .iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::PagesLayoutOnly { .. }))
            .collect();
        assert_eq!(found.len(), 1, "got: {:?}", kinds(&err));
        assert_eq!(
            found[0].kind,
            DiagnosticKind::PagesLayoutOnly {
                feature: "member-links".to_owned(),
                page: "/view/:id".to_owned(),
            }
        );
        assert!(found[0].message.contains("a 'member-links' child"), "{}", found[0].message);
    }

    #[test]
    fn compound_viewer_pages_wrong_typed_media_reports_only_invalid_attribute_type() {
        // Pins `lower_page_body`'s drop-on-error: the wrong-typed `media`
        // makes lowering report an Error, so the whole component is dropped
        // before `check_grid_without_media`'s sibling check,
        // `check_and_lower_pages`'s `PagesLayoutWithoutMedia` pass, ever
        // sees it — no cascading second diagnostic.
        let err = run_compound_viewer(r#"source="$id" layout="pages" index="none" media=7"#)
            .expect_err("wrong-typed media on a pages-layout viewer should fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::InvalidAttributeType],
            "expected exactly one InvalidAttributeType, and NOT PagesLayoutWithoutMedia, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn resource_list_grid_wrong_typed_media_reports_only_invalid_attribute_type() {
        // Mirrors the compound-viewer/pages case above for `grid`/`media`.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" grid=#true media=7
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed media on a grid list should fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::InvalidAttributeType],
            "expected exactly one InvalidAttributeType, and NOT GridWithoutMedia, got: {:?}",
            kinds(&err)
        );
    }

    /// A one-list page whose `resource-list` carries `attrs`.
    fn run_tile_ratio(attrs: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src =
            format!("{GOOD_SITE}\npage id=\"home\" route=\"/\" {{\n    resource-list class=\"Book\" {attrs}\n}}\n");
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    #[test]
    fn tile_ratio_grammar_is_two_plain_integers_and_a_colon() {
        for bad in [
            "9",
            r#""0:5""#,
            r#""1:0""#,
            r#""1.5:2""#,
            r#""1 : 9""#,
            r#"" 1:9""#,
            r#""1:9:2""#,
            r#""abc""#,
            r#""10000:1""#,
            r#""-1:2""#,
            r#""+1:2""#,
            r#""01:9""#,
            r#""""#,
            r#""1:""#,
            r#"":9""#,
            r#""1/9""#,
            r#""٣:٤""#,
            r#""9999:10000""#,
            r#""00:1""#,
        ] {
            let err = run_tile_ratio(&format!("media=\"image\" grid=#true tile-ratio={bad}"))
                .expect_err("a ratio outside the grammar must fail");
            assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType], "tile-ratio={bad}");
        }
        for (good, css) in [
            ("1:9", "1 / 9"),
            ("3:4", "3 / 4"),
            ("16:9", "16 / 9"),
            ("9999:1", "9999 / 1"),
        ] {
            let config = run_tile_ratio(&format!("media=\"image\" grid=#true tile-ratio=\"{good}\""))
                .unwrap_or_else(|e| panic!("tile-ratio={good} must lower: {e:?}"));
            let Component::ResourceList { tile_ratio, .. } = &config.pages[0].body[0] else {
                panic!("expected a resource-list");
            };
            assert_eq!(tile_ratio.map(|r| r.to_string()).as_deref(), Some(css));
        }
    }

    #[test]
    fn tile_ratio_without_grid_is_an_error() {
        let err = run_tile_ratio(r#"tile-ratio="1:9""#).expect_err("a ratio needs grid=#true");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::TileRatioWithoutGrid { class: "Book".to_owned(), page: "/".to_owned() }]
        );
        // A ratio that does not lower is reported once, not also as
        // `TileRatioWithoutGrid`.
        for bad in [r#"tile-ratio="abc""#, "tile-ratio=9"] {
            let err = run_tile_ratio(bad).expect_err("a bad ratio must fail");
            assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType], "{bad}");
        }
        let config = run_tile_ratio(r#"media="image" grid=#true"#).expect("no ratio is fine");
        let Component::ResourceList { tile_ratio, .. } = &config.pages[0].body[0] else {
            panic!("expected a resource-list");
        };
        assert_eq!(*tile_ratio, None);
    }

    #[test]
    fn search_bar_wrong_typed_placeholder_and_scope_are_invalid_attribute_type() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="name" placeholder=1 scope=#true
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed placeholder/scope should fail");
        assert_only_invalid_attribute_type(&err);
    }

    #[test]
    fn search_bar_wrong_typed_searches_is_invalid_attribute_type_not_missing() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches=1
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed searches should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for searches=1, got: {k:?}"
        );
        assert!(
            !k.contains(&DiagnosticKind::MissingRequiredAttribute),
            "a wrong-typed searches must not report the misleading MissingRequiredAttribute: {k:?}"
        );
    }

    #[test]
    fn sort_wrong_typed_direction_is_invalid_attribute_type() {
        // `direction=1` must report, not fall back to `Asc` silently — it is
        // present, just not a string.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        sort property="name" direction=1
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed sort direction should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for direction=1, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn absent_optional_string_attributes_stay_diagnostic_free() {
        // Guards the "absent ⇒ no diagnostic" half of the contract above.
        // Each component sits on its own page — a compound-viewer beside a
        // default-paginated list is `CompoundViewerWithPagedList`.
        let src = format!(
            r#"{}
page id="a" route="/a" {{
    resource-detail id="X"
}}
page id="b" route="/b" {{
    resource-list id="l" class="C" {{
        sort property="name"
    }}
    search-bar target="l" searches="name"
}}
page id="c" route="/c" {{
    compound-viewer source="S" layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx);
        assert!(
            result.is_ok(),
            "no optional attribute should report when absent: {:?}",
            result.err().map(|e| kinds(&e))
        );
    }

    #[test]
    fn wrong_typed_resource_list_id_cascades_to_unresolved_target() {
        // Pins the accepted cascade: a wrong-typed list `id` is
        // `InvalidAttributeType` from lowering AND, because the
        // pre-scan `page_list_ids` (built from the raw config, never the
        // lowered one) never sees a string id for this list either, a
        // facet-panel targeting it is `UnresolvedTarget` too. Both are true
        // statements; the pre-scan can't do better without a string.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id=1 class="Book"
    facet-panel target="x"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("wrong-typed list id + facet-panel should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for id=1, got: {k:?}"
        );
        assert!(
            k.contains(&DiagnosticKind::UnresolvedTarget),
            "expected UnresolvedTarget for the facet-panel that can never resolve a wrong-typed \
             list id, got: {k:?}"
        );
    }

    #[test]
    fn chip_kind_absent_lowers_to_none() {
        // `chip property="x"` with no `kind` lowers to `kind: None` — the
        // existing cardinality-derived shape.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        chip property="office"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("valid chip with no kind should lower");
        let Component::ResourceList { chips, .. } = &config.pages[0].body[0] else {
            panic!("expected a ResourceList component");
        };
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].property, "office");
        assert_eq!(chips[0].kind, None);
    }

    #[test]
    fn chip_kind_term_and_keyword_lower_successfully() {
        // Both closed-vocabulary values lower without a diagnostic.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        chip property="office" kind="term"
        chip property="source" kind="keyword"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("valid chip kinds should lower");
        let Component::ResourceList { chips, .. } = &config.pages[0].body[0] else {
            panic!("expected a ResourceList component");
        };
        assert_eq!(chips.len(), 2);
        assert_eq!(chips[0].kind, Some(ChipKind::Term));
        assert_eq!(chips[1].kind, Some(ChipKind::Keyword));
    }

    // ── UnknownFacetKind ──────────────────────────────────────────────────────

    #[test]
    fn unknown_facet_kind_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    facet-panel target="books" {{
        facet property="genre" kind="rainbow"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("unknown facet kind should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnknownFacetKind),
            "expected UnknownFacetKind, got: {:?}",
            kinds(&err)
        );
    }

    // ── DuplicateResourceListId ───────────────────────────────────────────────

    #[test]
    fn duplicate_resource_list_id_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    resource-list id="books" class="Article"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("duplicate resource-list id should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::DuplicateResourceListId),
            "expected DuplicateResourceListId, got: {:?}",
            kinds(&err)
        );
    }

    // ── DuplicateSingletonComponent (resource-list / search-bar) ─────────────────
    //
    // Distinct from `DuplicateResourceListId` above: these two `resource-list`s
    // declare DIFFERENT ids (both lower successfully into `body`), so the guard
    // under test here is the page-wide component-count check, not the
    // declared-id-collision check.

    #[test]
    fn two_resource_lists_on_one_page_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    resource-list id="articles" class="Article"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("two resource-list components on one page should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::DuplicateSingletonComponent { component, page }
                    if component == "resource-list" && page == "/"
            )),
            "expected DuplicateSingletonComponent for resource-list, got: {:?}",
            k
        );
    }

    #[test]
    fn two_search_bars_on_one_page_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="name"
    search-bar target="books" searches="name"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("two search-bar components on one page should fail");
        let k = kinds(&err);
        assert!(
            k.iter().any(|d| matches!(
                d,
                DiagnosticKind::DuplicateSingletonComponent { component, page }
                    if component == "search-bar" && page == "/"
            )),
            "expected DuplicateSingletonComponent for search-bar, got: {:?}",
            k
        );
    }

    // ── CompoundViewerWithPagedList ─────────────────────

    fn has_compound_paged_list_error(report: &ValidationReport, route: &str) -> bool {
        report.iter().any(|d| {
            matches!(
                &d.kind,
                DiagnosticKind::CompoundViewerWithPagedList { page } if page == route
            )
        })
    }

    #[test]
    fn compound_viewer_with_per_page_list_on_one_page_fails() {
        let src = format!(
            r#"{}
page id="book" route="/books/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
    resource-list id="books" class="Book" {{
        per-page 10
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("compound-viewer + per-page list should fail");
        assert!(
            has_compound_paged_list_error(&err, "/books/:id"),
            "expected CompoundViewerWithPagedList, got: {:?}",
            kinds(&err)
        );
        let msg = &err
            .iter()
            .find(|d| matches!(d.kind, DiagnosticKind::CompoundViewerWithPagedList { .. }))
            .expect("diagnostic present")
            .message;
        assert!(
            msg.contains("compound-viewer") && msg.contains("resource-list"),
            "message must name both components, got: {msg}"
        );
        assert_eq!(
            err.iter()
                .find(|d| matches!(d.kind, DiagnosticKind::CompoundViewerWithPagedList { .. }))
                .map(|d| d.severity),
            Some(Severity::Error)
        );
    }

    #[test]
    fn compound_viewer_with_default_paginated_list_fails() {
        // No `per-page`/`limit` child: pagination lowers to the default
        // `PerPage(20)`, which still reads `?page=N`.
        let src = format!(
            r#"{}
page id="book" route="/books/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
    resource-list id="books" class="Book"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("compound-viewer + default-paged list should fail");
        assert!(
            has_compound_paged_list_error(&err, "/books/:id"),
            "expected CompoundViewerWithPagedList, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn compound_viewer_alone_passes() {
        let src = format!(
            r#"{}
page id="book" route="/books/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        run(&src, &ctx).expect("a lone compound-viewer must pass");
    }

    #[test]
    fn compound_viewer_with_limit_list_passes() {
        let src = format!(
            r#"{}
page id="book" route="/books/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
    resource-list id="books" class="Book" {{
        limit 5
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        run(&src, &ctx).expect("compound-viewer + limit (non-paginated) list must pass");
    }

    #[test]
    fn compound_viewer_and_paged_list_on_different_pages_pass() {
        let src = format!(
            r#"{}
page id="book" route="/books/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
}}
page id="books" route="/books" {{
    resource-list id="books" class="Book" {{
        per-page 10
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        run(&src, &ctx).expect("the two components on different pages must pass");
    }

    // ── PaginationConflict ────────────────────────────────────────────────────

    #[test]
    fn pagination_conflict_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        per-page 10
        limit 6
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("per-page+limit conflict should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::PaginationConflict),
            "expected PaginationConflict, got: {:?}",
            kinds(&err)
        );
    }

    // ── UnresolvedTarget ──────────────────────────────────────────────────────

    #[test]
    fn unresolved_target_facet_panel_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    facet-panel target="no-such-list" {{
        facet property="genre" kind="text"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("unresolved facet-panel target should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnresolvedTarget),
            "expected UnresolvedTarget, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn unresolved_target_search_bar_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="ghost-list" searches="title"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("unresolved search-bar target should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnresolvedTarget),
            "expected UnresolvedTarget, got: {:?}",
            kinds(&err)
        );
    }

    /// A resource-list that fails another check (missing `class`) still counts
    /// as a present target — no spurious cascade to UnresolvedTarget.
    #[test]
    fn unresolved_target_no_cascade_when_list_exists_but_failed_other_check() {
        // The resource-list "books" has no `class` (error), but it exists in
        // the page raw IR. The facet-panel's target="books" must NOT produce
        // an extra UnresolvedTarget on top.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books"
    facet-panel target="books" {{
        facet property="genre" kind="text"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("missing class should fail");
        let k = kinds(&err);
        // Exact kind-set assertion: only MissingRequiredAttribute; UnresolvedTarget absent.
        // A wrong impl that suppresses all errors (empty set) would fail
        // (empty ≠ {MissingRequiredAttribute}).
        // A wrong impl that cascades would fail
        // ({MissingRequiredAttribute, UnresolvedTarget} ≠ {MissingRequiredAttribute}).
        let expected: std::collections::HashSet<DiagnosticKind> =
            std::iter::once(DiagnosticKind::MissingRequiredAttribute).collect();
        let actual: std::collections::HashSet<DiagnosticKind> = k.into_iter().collect();
        assert_eq!(
            actual, expected,
            "expected exact kind set {{MissingRequiredAttribute}}, got: {actual:?}"
        );
    }

    // ── UndeclaredRouteParam ──────────────────────────────────────────────────

    #[test]
    fn undeclared_route_param_fails() {
        let src = format!(
            r#"{}
page id="detail" route="/books/:id" {{
    resource-detail id="$bookId"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("undeclared route param should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UndeclaredRouteParam),
            "expected UndeclaredRouteParam, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn undeclared_route_param_via_non_id_attr_fails() {
        // compound-viewer source="$missing" — the param "missing" is not on the route.
        let src = format!(
            r#"{}
page id="detail" route="/items/:id" {{
    compound-viewer source="$missing" layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("undeclared param in source should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UndeclaredRouteParam),
            "expected UndeclaredRouteParam in source, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn declared_route_param_passes() {
        let src = format!(
            r#"{}
page id="detail" route="/books/:id" {{
    resource-detail id="$id"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx);
        assert!(result.is_ok(), "declared param should pass: {:?}", result);
    }

    /// A page with a missing route must NOT cause `UndeclaredRouteParam` to
    /// skip checks on OTHER pages in the same project.
    #[test]
    fn missing_route_page_skip_does_not_suppress_param_error_on_other_page() {
        // page "bad-route" has no route — skip $param checks for it.
        // page "detail" has route "/books/:id" — $missing IS undeclared there.
        let src = format!(
            r#"{}
page id="no-route" {{
    resource-detail id="$whatever"
}}
page id="detail" route="/books/:id" {{
    compound-viewer source="$missing" layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        // The page "no-route" is skipped (no route → no page in result).
        // The page "detail" has a route, so $param checks run.
        // "$missing" is not declared on "/books/:id" → exactly one UndeclaredRouteParam.
        let err = run(&src, &ctx).expect_err("undeclared param on second page should fail");
        let k = kinds(&err);
        assert!(
            k.contains(&DiagnosticKind::UndeclaredRouteParam),
            "expected UndeclaredRouteParam from second page, got: {:?}",
            k
        );
        // There must be EXACTLY ONE UndeclaredRouteParam — the no-route page emitted none.
        let param_error_count = err.iter().filter(|d| d.kind == DiagnosticKind::UndeclaredRouteParam).count();
        assert_eq!(
            param_error_count, 1,
            "expected exactly 1 UndeclaredRouteParam (only from the routed page), got: {param_error_count}"
        );
    }

    // ── NarrativeFileNotFound ─────────────────────────────────────────────────

    #[test]
    fn narrative_file_not_found_fails() {
        let narrative_files: Vec<String> = vec!["home.md".into(), "home.de.md".into()];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="missing.md"
}}
"#,
            GOOD_SITE
        );
        let err = run(&src, &ctx).expect_err("missing narrative file should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::NarrativeFileNotFound),
            "expected NarrativeFileNotFound, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn narrative_file_found_passes() {
        let narrative_files: Vec<String> = vec!["home.md".into(), "home.de.md".into(), "home.en.md".into()];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="home.md"
}}
"#,
            GOOD_SITE
        );
        let result = run(&src, &ctx);
        assert!(result.is_ok(), "existing narrative file should pass: {:?}", result);
    }

    #[test]
    fn all_narrative_variants_present_passes() {
        // All language variants present: home.de.md and home.en.md.
        let narrative_files: Vec<String> = vec!["home.md".into(), "home.de.md".into(), "home.en.md".into()];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="home.md"
}}
"#,
            GOOD_SITE
        );
        let result = run(&src, &ctx);
        assert!(result.is_ok(), "all variants present should pass: {:?}", result);
    }

    // ── InvalidNarrativePath ──────────────────────────────────────────────────

    #[test]
    fn narrative_path_separator_fails() {
        let narrative_files: Vec<String> = vec!["home.md".into()];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="subdir/home.md"
}}
"#,
            GOOD_SITE
        );
        let err = run(&src, &ctx).expect_err("path separator in narrative file should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidNarrativePath),
            "expected InvalidNarrativePath, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn narrative_traversal_path_fails() {
        let narrative_files: Vec<String> = vec![];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="../etc/passwd"
}}
"#,
            GOOD_SITE
        );
        let err = run(&src, &ctx).expect_err("traversal path in narrative file should fail");
        let k = kinds(&err);
        // Should fail with either InvalidNarrativePath (path separator detected)
        // or the traversal check.
        assert!(
            k.contains(&DiagnosticKind::InvalidNarrativePath),
            "expected InvalidNarrativePath for traversal, got: {:?}",
            k
        );
    }

    // ── Bare ".." must trigger InvalidNarrativePath ─────────

    /// A narrative `file=".."` (bare double-dot, no slash) must be rejected.
    /// `file.split('.').any(|seg| seg == "..")` would never match it: splitting
    /// ".." by '.' yields ["", "", ""], none equal "..".
    #[test]
    fn narrative_bare_dotdot_fails() {
        let narrative_files: Vec<String> = vec![];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file=".."
}}
"#,
            GOOD_SITE
        );
        let err = run(&src, &ctx).expect_err("bare '..' narrative file must fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidNarrativePath),
            "expected InvalidNarrativePath for bare '..', got: {:?}",
            kinds(&err)
        );
    }

    // ── cover-image (site index card) ─────────────────────────────────────────

    /// Build a `ValidationContext` with custom `asset_files`, empty
    /// `narrative_files` (mirrors `good_ctx`, but for the asset-inventory axis).
    fn ctx_with_assets<'a>(asset_files: &'a [String]) -> ValidationContext<'a> {
        ValidationContext {
            project_id: "my-project",
            reserved_ids: &[],
            narrative_files: &[],
            asset_files,
        }
    }

    #[test]
    fn cover_image_absent_lowers_fine() {
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(GOOD_SITE, &ctx);
        assert!(result.is_ok(), "no cover-image should lower fine: {:?}", result);
        assert!(result.unwrap().site.cover_image.is_none());
    }

    #[test]
    fn cover_image_present_and_found_lowers() {
        let asset_files: Vec<String> = vec!["assets/cover.webp".into()];
        let ctx = ctx_with_assets(&asset_files);
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "assets/cover.webp"
}
"#;
        let result = run(src, &ctx).expect("existing cover-image should pass");
        assert_eq!(result.site.cover_image, Some("assets/cover.webp".to_owned()));
    }

    #[test]
    fn cover_image_not_found_fails() {
        let asset_files: Vec<String> = vec!["assets/other.webp".into()];
        let ctx = ctx_with_assets(&asset_files);
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "assets/cover.webp"
}
"#;
        let err = run(src, &ctx).expect_err("missing cover-image should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::CoverImageNotFound),
            "expected CoverImageNotFound, got: {:?}",
            kinds(&err)
        );
    }

    /// The inventory and the config value are compared verbatim, so both
    /// must carry the `assets/` segment (see `DiscoveredProject::
    /// asset_files`) — every test in this group uses that production
    /// shape. This one additionally pins the NESTED case: the path-shape
    /// validator and the runtime `<base>/{id}/_assets/{*path}` route both accept
    /// a subdirectory, so the inventory walk must reach it too.
    #[test]
    fn cover_image_matches_assets_prefixed_inventory() {
        let asset_files: Vec<String> = vec!["assets/cover.webp".into(), "assets/covers/nested.webp".into()];
        let ctx = ctx_with_assets(&asset_files);
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "assets/covers/nested.webp"
}
"#;
        let result = run(src, &ctx).expect("assets/-prefixed cover-image should pass");
        assert_eq!(result.site.cover_image, Some("assets/covers/nested.webp".to_owned()));
    }

    /// A project declaring `cover-image` while its `assets/` inventory is empty
    /// (typically: no `assets/` directory at all) must NOT boot green. A check
    /// that skips itself on an empty inventory fails open on exactly this case,
    /// the one it exists to catch, and the site index serves a broken `<img>`.
    #[test]
    fn cover_image_with_empty_asset_inventory_fails() {
        let ctx = ctx_with_assets(&[]);
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "assets/cover.webp"
}
"#;
        let err = run(src, &ctx).expect_err("cover-image with no assets/ should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::CoverImageNotFound),
            "expected CoverImageNotFound, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn cover_image_traversal_path_fails() {
        let ctx = ValidationContext::for_tests("my-project");
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "../escape.webp"
}
"#;
        let err = run(src, &ctx).expect_err("traversal cover-image path should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidCoverImagePath),
            "expected InvalidCoverImagePath, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn cover_image_leading_slash_fails() {
        let ctx = ValidationContext::for_tests("my-project");
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "/abs/path.webp"
}
"#;
        let err = run(src, &ctx).expect_err("leading-slash cover-image path should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidCoverImagePath),
            "expected InvalidCoverImagePath, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn cover_image_dot_prefixed_segment_fails() {
        // Mirrors the runtime route's (`serve_project_asset`) segment
        // rejection: a dot-prefixed segment must be rejected at boot too, or
        // a boot-valid cover-image path could still 404 at request time.
        let ctx = ValidationContext::for_tests("my-project");
        let src = r#"
site {
    title "Test Project"
    languages "de" "en"
    cover-image "assets/.cover.webp"
}
"#;
        let err = run(src, &ctx).expect_err("dot-prefixed segment cover-image path should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidCoverImagePath),
            "expected InvalidCoverImagePath, got: {:?}",
            kinds(&err)
        );
    }

    // ── searches list-valued attribute reconstruction ─────────────────────────

    #[test]
    fn searches_single_value_degenerate_list() {
        // `searches="title"` with NO positional args → searches = ["title"].
        // This exercises the degenerate list-reconstruction case where the
        // first_search is Some and raw.args is empty.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="title"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("single searches value should pass");
        let page = &result.pages[0];
        let search_bar = page
            .body
            .iter()
            .find(|c| matches!(c, Component::SearchBar { .. }))
            .expect("should have a SearchBar component");
        if let Component::SearchBar { searches, .. } = search_bar {
            assert_eq!(
                searches,
                &vec!["title".to_owned()],
                "searches with a single value must be a 1-element Vec"
            );
        }
    }

    #[test]
    fn searches_multi_value_reconstructs_correctly() {
        // `searches="title" "description"` → searches = ["title", "description"]
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="title" "description"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("valid searches list should pass");
        let page = &result.pages[0];
        let search_bar = page
            .body
            .iter()
            .find(|c| matches!(c, Component::SearchBar { .. }))
            .expect("should have a SearchBar component");
        if let Component::SearchBar { searches, .. } = search_bar {
            assert_eq!(
                searches,
                &vec!["title".to_owned(), "description".to_owned()],
                "searches should contain both values"
            );
        }
    }

    // ── RESERVED_FACET_PROPERTIES — "q" reservation ───────────────────────────

    #[test]
    fn reserved_facet_properties_contains_q() {
        // "q" is the search box's own query-string key (?q=); a facet property
        // literally named "q" would collide with it. Direct assertion on the
        // const's contents — simplest and sufficient, no fixture needed just
        // for this (the `broken-facet-reserved-property` fixture/
        // Layer-4 test proves the *check* fires for a reserved name,
        // using "sort"; this proves "q" is a member of the reserved set).
        assert!(
            RESERVED_FACET_PROPERTIES.contains(&"q"),
            "RESERVED_FACET_PROPERTIES must contain \"q\" (the search box's ?q= key)"
        );
    }

    #[test]
    fn reserved_facet_properties_contains_strip() {
        // "strip" is the record viewer's thumb-strip fold state (?strip=),
        // reserved so a facet property literally named
        // `strip` can never collide with the host's reader (mirrors the "q"
        // reservation above).
        assert!(
            RESERVED_FACET_PROPERTIES.contains(&"strip"),
            "RESERVED_FACET_PROPERTIES must contain \"strip\" (the record viewer's ?strip= key)"
        );
    }

    // ── search-bar `scope` attribute (chrome-string id, optional) ─────────────

    #[test]
    fn search_bar_scope_attr_parses() {
        // `scope="…"` is an optional chrome-string id, parsed exactly like
        // `placeholder`. When present it must lower to `Some(..)`.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="title" scope="books-scope"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("search-bar with scope should pass");
        let page = &result.pages[0];
        let search_bar = page
            .body
            .iter()
            .find(|c| matches!(c, Component::SearchBar { .. }))
            .expect("should have a SearchBar component");
        if let Component::SearchBar { scope, .. } = search_bar {
            assert_eq!(
                scope,
                &Some("books-scope".to_owned()),
                "declared scope attr must lower to Some(..)"
            );
        }
    }

    #[test]
    fn search_bar_scope_attr_absent_lowers_to_none() {
        // Omitting `scope` must lower to `None` (backward-compatible — mirrors
        // `placeholder`'s optional-attribute behaviour).
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list id="books" class="Book"
    search-bar target="books" searches="title"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("search-bar without scope should pass");
        let page = &result.pages[0];
        let search_bar = page
            .body
            .iter()
            .find(|c| matches!(c, Component::SearchBar { .. }))
            .expect("should have a SearchBar component");
        if let Component::SearchBar { scope, .. } = search_bar {
            assert_eq!(scope, &None, "omitted scope attr must lower to None");
        }
    }

    // ── Fully-good page lowers to exact typed IR ──────────────────────────────

    #[test]
    fn fully_good_page_lowers_to_exact_typed_ir() {
        let narrative_files: Vec<String> = vec!["home.md".into()];
        let ctx = good_ctx(&narrative_files);
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative file="home.md"
    resource-list id="books" class="Book" {{
        sort property="title"
        per-page 10
    }}
    facet-panel target="books" {{
        facet property="genre" kind="text"
    }}
    search-bar target="books" searches="title" "description" scope="books-scope"
}}
"#,
            GOOD_SITE
        );
        let result = run(&src, &ctx).expect("fully good config should pass");
        assert_eq!(result.pages.len(), 1);
        let page = &result.pages[0];
        assert_eq!(page.id, "home");
        assert_eq!(page.route, "/");
        assert_eq!(page.body.len(), 4);

        // Narrative
        assert_eq!(
            page.body[0],
            Component::Narrative { file: "home.md".to_owned(), companion: None, side: None }
        );

        // ResourceList
        assert_eq!(
            page.body[1],
            Component::ResourceList {
                id: Some("books".to_owned()),
                class: "Book".to_owned(),
                wheres: vec![],
                sorts: vec![Sort {
                    key: SortKey::Property("title".to_owned()),
                    direction: SortDir::Asc,
                }],
                pagination: Pagination::PerPage(10),
                gloss: None,
                chips: vec![],
                media: None,
                grid: false,
                tile_ratio: None,
                order_name_label: None,
                order_name_desc_label: None,
                order_default_label: None,
                order_name_property: None,
                order_options: vec![],
            }
        );

        // FacetPanel
        assert_eq!(
            page.body[2],
            Component::FacetPanel {
                target: "books".to_owned(),
                facets: vec![Facet { property: "genre".to_owned(), kind: FacetKind::Text }],
            }
        );

        // SearchBar
        assert_eq!(
            page.body[3],
            Component::SearchBar {
                target: "books".to_owned(),
                searches: vec!["title".to_owned(), "description".to_owned()],
                placeholder: None,
                scope: Some("books-scope".to_owned()),
            }
        );
    }

    #[test]
    fn resource_detail_with_show_properties_lowers_correctly() {
        let src = format!(
            r#"{}
page id="detail" route="/items/:id" {{
    resource-detail id="$id" {{
        show-properties "title" "author" "year"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("resource-detail with show-properties should pass");
        let page = &result.pages[0];
        assert_eq!(page.body.len(), 1);
        assert_eq!(
            page.body[0],
            Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: Some(vec!["title".to_owned(), "author".to_owned(), "year".to_owned(),]),
                title_property: None,
                class: None,
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            }
        );
    }

    // ── UnknownChildNode on resource-detail ──────────────────────────────────

    #[test]
    fn resource_detail_unknown_child_is_error() {
        let src = format!(
            r#"{}
page id="detail" route="/items/:id" {{
    resource-detail id="X" {{
        gruop label="a" "p"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("unknown child node must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownChildNode {
                component: "resource-detail".to_owned(),
                child: "gruop".to_owned(),
            }]
        );
    }

    #[test]
    fn resource_detail_each_unknown_child_reported() {
        let src = format!(
            r#"{}
page id="detail" route="/items/:id" {{
    resource-detail id="X" {{
        narative-for "R1" file="x.md"
        incomming-links label="a" class="C" "p"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("two unknown children must both fail");
        assert_eq!(
            kinds(&err),
            vec![
                DiagnosticKind::UnknownChildNode {
                    component: "resource-detail".to_owned(),
                    child: "narative-for".to_owned(),
                },
                DiagnosticKind::UnknownChildNode {
                    component: "resource-detail".to_owned(),
                    child: "incomming-links".to_owned(),
                },
            ],
            "must report in declaration order"
        );
    }

    #[test]
    fn resource_detail_known_children_are_not_flagged() {
        // One of each of the four known child names. `narrative-for`'s file
        // isn't in `for_tests`' empty inventory, so this legitimately fails
        // with `NarrativeFileNotFound` — unrelated to this check, and allowed
        // (assert on the report regardless of `run`'s Ok/Err outcome).
        let src = group_src(
            r#"        show-properties "title"
        group label="group-a" "creator"
        narrative-for "R1" file="x.md"
        incoming-links label="group-a" class="C" "p""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(
            !kinds(&report)
                .iter()
                .any(|k| matches!(k, DiagnosticKind::UnknownChildNode { .. })),
            "no UnknownChildNode expected among the four known names: {:?}",
            kinds(&report)
        );
    }

    #[test]
    fn resource_detail_unknown_child_does_not_suppress_siblings() {
        // Call `lower_resource_detail` directly with a local report: `run`
        // (via `lowered_incoming`'s pattern) `.expect()`s success and would
        // panic on the Error this fixture deliberately raises.
        let src = group_src(
            r#"        gruop label="group-a" "x"
        group label="group-a" "creator""#,
        );
        let raw = config::parse(&src).expect("test fixture KDL must be syntactically valid");
        let raw_comp = &raw.pages[0].body[0];
        let ctx = ValidationContext::for_tests("my-project");
        let mut report = ValidationReport::new();
        let lowered = lower_resource_detail(raw_comp, &[], true, &ctx, None, &mut report);
        let Some(Component::ResourceDetail { groups, .. }) = lowered else {
            panic!("expected Some(ResourceDetail), got {:?}", lowered);
        };
        assert_eq!(
            groups,
            vec![PropertyGroup {
                label: "group-a".to_owned(),
                properties: vec!["creator".to_owned()],
                empty: None,
                links_label: None,
            }],
            "the unknown child must not suppress lowering of its valid sibling"
        );
        assert_eq!(
            kinds(&report),
            vec![DiagnosticKind::UnknownChildNode {
                component: "resource-detail".to_owned(),
                child: "gruop".to_owned(),
            }]
        );
    }

    // ── `group` children on resource-detail ───────────────────────────────

    /// A site with one declared chrome id, `group-a`, for the `group` tests.
    const GROUP_SITE: &str = r#"
site {
    title "Test Project"
    languages "de" "en"
    chrome {
        string "group-a" {
            de "Gruppe A"
            en "Group A"
        }
        string "note-n" {
            de "Einträge · {n}"
            en "Entries · {n}"
        }
    }
}
"#;

    fn group_src(detail_body: &str) -> String {
        format!(
            "{GROUP_SITE}\npage id=\"detail\" route=\"/items/:id\" {{\n    resource-detail id=\"$id\" {{\n{detail_body}\n    }}\n}}\n"
        )
    }

    #[test]
    fn resource_detail_groups_lower_in_declaration_order() {
        let src = group_src(
            r#"        group label="group-a" "creator" "date"
        group label="group-a" "place""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("grouped resource-detail should pass");
        let Component::ResourceDetail { groups, show_properties, .. } = &result.pages[0].body[0] else {
            panic!("expected ResourceDetail, got {:?}", result.pages[0].body[0]);
        };
        assert_eq!(show_properties, &None, "show-properties stays optional");
        assert_eq!(
            groups,
            &vec![
                PropertyGroup {
                    label: "group-a".to_owned(),
                    properties: vec!["creator".to_owned(), "date".to_owned()],
                    empty: None,
                    links_label: None,
                },
                PropertyGroup {
                    label: "group-a".to_owned(),
                    properties: vec!["place".to_owned()],
                    empty: None,
                    links_label: None,
                },
            ]
        );
    }

    #[test]
    fn resource_detail_group_empty_lowers() {
        let src = group_src(r#"        group label="group-a" empty="note-n" "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("group with empty= should pass");
        let Component::ResourceDetail { groups, .. } = &result.pages[0].body[0] else {
            panic!("expected ResourceDetail");
        };
        assert_eq!(groups[0].empty.as_deref(), Some("note-n"));
    }

    #[test]
    fn resource_detail_group_links_label_lowers_and_flags_an_undeclared_id() {
        let src = group_src(r#"        group label="group-a" links-label="group-a" "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("group with links-label= should pass");
        let Component::ResourceDetail { groups, .. } = &result.pages[0].body[0] else {
            panic!("expected ResourceDetail");
        };
        assert_eq!(groups[0].links_label.as_deref(), Some("group-a"));

        let src = group_src(r#"        group label="group-a" links-label="nope" "creator""#);
        let report = report_of(&src, &ctx);
        assert!(!report.has_errors(), "an undeclared links-label is advisory: {report:?}");
        assert!(
            kinds(&report).contains(&DiagnosticKind::UnknownGroupLinksLabel {
                label: "nope".to_owned(),
                page: "/items/:id".to_owned(),
            }),
            "{:?}",
            kinds(&report)
        );
    }

    #[test]
    fn resource_detail_group_links_label_must_be_a_string() {
        let src = group_src(r#"        group label="group-a" links-label=1 "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(
            kinds(&report).iter().any(|k| matches!(k, DiagnosticKind::InvalidAttributeType)),
            "{:?}",
            kinds(&report)
        );
    }

    #[test]
    fn resource_detail_group_empty_wrong_type_is_an_error() {
        let src = group_src(r#"        group label="group-a" empty=3 "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("non-string empty= must fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn resource_detail_group_with_undeclared_empty_label_warns() {
        let src = group_src(r#"        group label="group-a" empty="empty-typo" "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(!report.has_errors(), "must not block boot: {report:?}");
        let hits: Vec<_> = report
            .iter()
            .filter(|d| {
                d.kind
                    == DiagnosticKind::UnknownGroupEmptyLabel {
                        label: "empty-typo".to_owned(),
                        page: "/items/:id".to_owned(),
                    }
            })
            .collect();
        assert_eq!(hits.len(), 1, "got: {:?}", kinds(&report));
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    #[test]
    fn resource_detail_group_with_declared_empty_label_is_clean() {
        let src = group_src(r#"        group label="group-a" empty="note-n" "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(report.is_empty(), "got: {:?}", kinds(&report));
    }

    #[test]
    fn resource_detail_group_without_label_is_an_error() {
        let src = group_src(r#"        group "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("an unlabelled group must fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
            "expected MissingRequiredAttribute, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn resource_detail_property_in_group_and_show_properties_is_duplicate() {
        let src = group_src(
            r#"        show-properties "name" "creator"
        group label="group-a" "creator" "date""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("group + show-properties overlap must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::DuplicateGroupedProperty {
                property: "creator".to_owned(),
                page: "/items/:id".to_owned(),
            }]
        );
    }

    #[test]
    fn resource_detail_property_in_two_groups_is_duplicate_reported_once() {
        let src = group_src(
            r#"        group label="group-a" "creator" "date"
        group label="group-a" "creator"
        group label="group-a" "creator""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("a property in two groups must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::DuplicateGroupedProperty {
                property: "creator".to_owned(),
                page: "/items/:id".to_owned(),
            }],
            "one diagnostic per repeated property"
        );
    }

    #[test]
    fn resource_detail_disjoint_group_and_show_properties_pass() {
        let src = group_src(
            r#"        show-properties "name"
        group label="group-a" "creator" "date""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(
            report.is_empty(),
            "disjoint groups + declared label: zero diagnostics, got: {:?}",
            kinds(&report)
        );
    }

    #[test]
    fn resource_detail_group_with_undeclared_label_warns() {
        let src = group_src(r#"        group label="group-typo" "creator""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(!report.has_errors(), "UnknownGroupLabel must not block boot: {report:?}");
        let hits: Vec<_> = report
            .iter()
            .filter(|d| {
                d.kind
                    == DiagnosticKind::UnknownGroupLabel {
                        label: "group-typo".to_owned(),
                        page: "/items/:id".to_owned(),
                    }
            })
            .collect();
        assert_eq!(hits.len(), 1, "got: {:?}", kinds(&report));
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    // ── `incoming-links` child on resource-detail ────

    /// `GROUP_SITE`'s one declared chrome id, `group-a`, doubles as the
    /// declared incoming-links caption id.
    fn incoming_src(child: &str) -> String {
        group_src(&format!("        {child}"))
    }

    fn lowered_incoming(src: &str) -> Option<IncomingLinksSpec> {
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(src, &ctx).expect("incoming-links config should pass");
        let Component::ResourceDetail { incoming_links, .. } = &config.pages[0].body[0] else {
            panic!("expected ResourceDetail, got {:?}", config.pages[0].body[0]);
        };
        incoming_links.clone()
    }

    #[test]
    fn incoming_links_lowers_with_explicit_order() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Page" order="compound-seq" "strip-left" "strip-right""#,
        ));
        assert_eq!(
            spec,
            Some(IncomingLinksSpec {
                label: "group-a".to_owned(),
                class: "Page".to_owned(),
                properties: vec!["strip-left".to_owned(), "strip-right".to_owned()],
                order: IncomingOrder::CompoundSeq,
                limit: DEFAULT_INCOMING_LINKS_LIMIT,
                grouping: None,
                row_label: None,
                row_aside: None,
                sort_property: None,
            })
        );
    }

    // ── Grouped incoming-links attributes ─────────────────

    #[test]
    fn incoming_links_lowers_grouping_and_row_attributes() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Performance" group-by="decade" layout="cards" note-label="note-n" row-label="date" row-aside="source" sort-property="sort-key" "work""#,
        ))
        .expect("spec");
        assert_eq!(
            spec.grouping,
            Some(IncomingGrouping {
                by: IncomingGroupBy::Property("decade".to_owned()),
                layout: IncomingLayout::Cards,
                note_label: Some("note-n".to_owned()),
            })
        );
        assert_eq!(spec.row_label.as_deref(), Some("date"));
        assert_eq!(spec.row_aside.as_deref(), Some("source"));
        assert_eq!(spec.sort_property.as_deref(), Some("sort-key"));
    }

    #[test]
    fn incoming_links_repeated_property_lowers_to_one_entry() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by="via" "librettist" "composer" "librettist""#,
        ))
        .expect("spec");
        assert_eq!(spec.properties, vec!["librettist", "composer"]);
    }

    #[test]
    fn incoming_links_group_by_via_defaults_layout_to_columns() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by="via" "librettist" "composer""#,
        ))
        .expect("spec");
        assert_eq!(
            spec.grouping,
            Some(IncomingGrouping {
                by: IncomingGroupBy::Via,
                layout: IncomingLayout::Columns,
                note_label: None,
            })
        );
    }

    #[test]
    fn incoming_links_grouped_without_limit_uses_ceiling() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by="via" "librettist""#,
        ))
        .expect("spec");
        assert_eq!(spec.limit, GROUPED_INCOMING_LIMIT_CEILING);
    }

    #[test]
    fn incoming_links_grouped_explicit_limit_is_honoured() {
        for (limit_src, want) in [("limit=50", 50), ("limit=7", 7)] {
            let spec = lowered_incoming(&incoming_src(&format!(
                r#"incoming-links label="group-a" class="Work" group-by="via" {limit_src} "librettist""#
            )))
            .expect("spec");
            assert_eq!(spec.limit, want, "{limit_src}");
        }
    }

    #[test]
    fn incoming_links_layout_and_note_without_group_by_are_ignored() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Page" layout="cards" note-label="nope" "strip-left""#,
        ))
        .expect("spec");
        assert_eq!(spec.grouping, None);
        assert_eq!(spec.limit, DEFAULT_INCOMING_LINKS_LIMIT);
        let report = report_of(
            &incoming_src(
                r#"incoming-links label="group-a" class="Page" layout="cards" note-label="nope" "strip-left""#,
            ),
            &ValidationContext::for_tests("my-project"),
        );
        assert!(report.is_empty(), "got: {:?}", kinds(&report));
    }

    #[test]
    fn incoming_links_unknown_layout_is_invalid_attribute_type() {
        let src =
            incoming_src(r#"incoming-links label="group-a" class="Work" group-by="via" layout="grid" "librettist""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("layout=\"grid\" must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn incoming_links_several_wrong_typed_attributes_report_all() {
        // One child, four mistakes, zero properties: every one is reported,
        // none short-circuited by the early `IncomingLinksWithoutProperty`.
        let src = incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by=1 layout=#true row-label=2 sort-property=#false"#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("must fail");
        assert_eq!(
            kinds(&err),
            vec![
                DiagnosticKind::InvalidAttributeType,
                DiagnosticKind::InvalidAttributeType,
                DiagnosticKind::InvalidAttributeType,
                DiagnosticKind::InvalidAttributeType,
                DiagnosticKind::IncomingLinksWithoutProperty,
            ]
        );
    }

    #[test]
    fn incoming_links_undeclared_note_label_warns() {
        let src = incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by="via" note-label="note-typo" "librettist""#,
        );
        let report = report_of(&src, &ValidationContext::for_tests("my-project"));
        assert!(!report.has_errors(), "must not block boot: {report:?}");
        let hits: Vec<_> = report
            .iter()
            .filter(|d| {
                d.kind
                    == DiagnosticKind::UnknownIncomingLinksNoteLabel {
                        label: "note-typo".to_owned(),
                        page: "/items/:id".to_owned(),
                    }
            })
            .collect();
        assert_eq!(hits.len(), 1, "got: {:?}", kinds(&report));
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    #[test]
    fn incoming_links_note_label_without_placeholder_is_an_error() {
        // `group-a` (GROUP_SITE's chrome string) carries no `{n}`.
        let src = incoming_src(
            r#"incoming-links label="group-a" class="Work" group-by="via" note-label="group-a" "librettist""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("a note without {n} must fail");
        assert!(
            !kinds(&err).is_empty()
                && kinds(&err).iter().all(|k| matches!(
                    k,
                    DiagnosticKind::IncomingLinksTextMissingPlaceholder { label, .. }
                        if label == "group-a"
                )),
            "got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn incoming_links_order_defaults_to_name_and_accepts_id() {
        let spec = lowered_incoming(&incoming_src(r#"incoming-links label="group-a" class="Page" "strip-left""#))
            .expect("spec");
        assert_eq!(spec.order, IncomingOrder::Name);
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Page" order="id" "strip-left""#,
        ))
        .expect("spec");
        assert_eq!(spec.order, IncomingOrder::Id);
    }

    #[test]
    fn resource_detail_without_incoming_links_lowers_none() {
        assert_eq!(lowered_incoming(&group_src("")), None);
    }

    #[test]
    fn incoming_links_limit_defaults_to_50() {
        let spec = lowered_incoming(&incoming_src(r#"incoming-links label="group-a" class="Page" "strip-left""#))
            .expect("spec");
        assert_eq!(spec.limit, DEFAULT_INCOMING_LINKS_LIMIT);
    }

    #[test]
    fn incoming_links_explicit_limit_lowers() {
        let spec = lowered_incoming(&incoming_src(
            r#"incoming-links label="group-a" class="Page" limit=5 "strip-left""#,
        ))
        .expect("spec");
        assert_eq!(spec.limit, 5);
    }

    #[test]
    fn incoming_links_zero_or_negative_limit_is_invalid_attribute_type() {
        for limit_src in ["limit=0", "limit=-1"] {
            let src = incoming_src(&format!(
                r#"incoming-links label="group-a" class="Page" {limit_src} "strip-left""#
            ));
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err(&format!("{limit_src} must fail"));
            assert_eq!(
                kinds(&err),
                vec![DiagnosticKind::InvalidAttributeType],
                "{limit_src}: got {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn incoming_links_non_integer_limit_is_invalid_attribute_type() {
        let src = incoming_src(r#"incoming-links label="group-a" class="Page" limit="5" "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("limit=\"5\" must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn incoming_links_unknown_order_value_is_invalid_attribute_type() {
        let src = incoming_src(r#"incoming-links label="group-a" class="Page" order="foo" "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("order=\"foo\" must fail, never fall back to name");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn incoming_links_non_string_order_is_invalid_attribute_type() {
        let src = incoming_src(r#"incoming-links label="group-a" class="Page" order=#true "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("order=#true must fail, never fall back to name");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn incoming_links_missing_label_and_class_report_both() {
        let src = incoming_src(r#"incoming-links "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("label + class are required");
        assert_eq!(
            kinds(&err),
            vec![
                DiagnosticKind::MissingRequiredAttribute,
                DiagnosticKind::MissingRequiredAttribute
            ]
        );
    }

    #[test]
    fn incoming_links_without_property_is_an_error() {
        let src = incoming_src(r#"incoming-links label="group-a" class="Page""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("zero properties must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::IncomingLinksWithoutProperty]);
        assert_eq!(err.iter().next().map(|d| d.severity), Some(Severity::Error));
    }

    #[test]
    fn two_incoming_links_children_is_an_error() {
        let src = group_src(
            r#"        incoming-links label="group-a" class="Page" "strip-left"
        incoming-links label="group-a" class="Page" "strip-right""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("a second incoming-links child must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::DuplicateIncomingLinks]);
        assert_eq!(err.iter().next().map(|d| d.severity), Some(Severity::Error));
    }

    #[test]
    fn incoming_links_with_undeclared_label_warns() {
        let src = incoming_src(r#"incoming-links label="pages-typo" class="Page" "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(
            !report.has_errors(),
            "UnknownIncomingLinksLabel must not block boot: {report:?}"
        );
        let hits: Vec<_> = report
            .iter()
            .filter(|d| {
                d.kind
                    == DiagnosticKind::UnknownIncomingLinksLabel {
                        label: "pages-typo".to_owned(),
                        page: "/items/:id".to_owned(),
                    }
            })
            .collect();
        assert_eq!(hits.len(), 1, "got: {:?}", kinds(&report));
        assert_eq!(hits[0].severity, Severity::Warning);
    }

    #[test]
    fn incoming_links_with_declared_label_is_clean() {
        let src = incoming_src(r#"incoming-links label="group-a" class="Page" "strip-left""#);
        let ctx = ValidationContext::for_tests("my-project");
        let report = report_of(&src, &ctx);
        assert!(report.is_empty(), "got: {:?}", kinds(&report));
    }

    #[test]
    fn detail_property_names_includes_group_members() {
        let show = Some(vec!["name".to_owned()]);
        let members = vec!["creator".to_owned(), "date".to_owned()];
        let title = Some("title".to_owned());
        assert_eq!(
            detail_property_names(&show, &members, &title),
            vec!["name", "creator", "date", "title"],
            "group members must be sparse-checked like show-properties"
        );
    }

    #[test]
    fn resource_detail_class_attr_is_lowered() {
        // The `class` attribute on `resource-detail` should be read into
        // `Component::ResourceDetail.class`. Absent ⇒ `None` (backward-compatible).
        let src_with_class = format!(
            r#"{}
page id="chars" route="/chars/:id" {{
    resource-detail id="$id" class="Character"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src_with_class, &ctx).expect("resource-detail with class should pass");
        let page = &result.pages[0];
        assert_eq!(page.body.len(), 1);
        assert_eq!(
            page.body[0],
            Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: None,
                title_property: None,
                class: Some("Character".to_owned()),
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            },
            "class attribute must be lowered into Component::ResourceDetail.class"
        );
    }

    #[test]
    fn resource_detail_class_attr_absent_gives_none() {
        // A resource-detail without a `class` attribute must produce class=None.
        let src_no_class = format!(
            r#"{}
page id="items" route="/items/:id" {{
    resource-detail id="$id"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src_no_class, &ctx).expect("resource-detail without class should pass");
        let page = &result.pages[0];
        assert_eq!(
            page.body[0],
            Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: None,
                title_property: None,
                class: None,
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            },
            "absent class attribute must give class=None"
        );
    }

    #[test]
    fn compound_viewer_lowers_correctly() {
        let src = format!(
            r#"{}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="contents" index="rail"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("compound-viewer should pass");
        let page = &result.pages[0];
        assert_eq!(
            page.body[0],
            Component::CompoundViewer {
                source: "$id".to_owned(),
                media: None,
                picker: None,
                layout: CompoundLayout::Contents,
                index: MemberIndex::Rail,
                paired: false,
                member_links: None,
                annotations: false,
            }
        );
    }

    #[test]
    fn compound_viewer_lowers_annotations() {
        let attrs = r#"source="$id" layout="pages" index="none" media="image" annotations=#true"#;
        let result = run_compound_viewer(attrs)
            .unwrap_or_else(|e| panic!("annotations=#true should lower, got: {:?}", kinds(&e)));
        assert!(matches!(
            result.pages[0].body[0],
            Component::CompoundViewer { annotations: true, .. }
        ));
        let errors = run_compound_viewer(r#"source="$id" layout="pages" index="none" media="image" annotations="yes""#)
            .expect_err("a string annotations= is InvalidAttributeType");
        assert!(
            kinds(&errors).contains(&DiagnosticKind::InvalidAttributeType),
            "{:?}",
            kinds(&errors)
        );
    }

    #[test]
    fn compound_viewer_lowers_picker() {
        for (value, mode) in [
            ("numbered", PickerMode::Numbered),
            ("signatures", PickerMode::Signatures),
        ] {
            let attrs = format!(r#"source="$id" layout="pages" index="none" media="image" picker="{value}""#);
            let result = run_compound_viewer(&attrs)
                .unwrap_or_else(|e| panic!("picker={value} should lower, got: {:?}", kinds(&e)));
            assert_eq!(
                result.pages[0].body[0],
                Component::CompoundViewer {
                    source: "$id".to_owned(),
                    media: Some("image".to_owned()),
                    picker: Some(mode),
                    layout: CompoundLayout::Pages,
                    index: MemberIndex::Off,
                    paired: false,
                    member_links: None,
                    annotations: false,
                }
            );
        }
    }

    /// The attributes `jump-to-member`/`jump-placeholder`, which the engine
    /// does not read, get no diagnostic: CPE has no unknown-attribute check, so
    /// one is ignored.
    #[test]
    fn compound_viewer_ignores_the_retired_jump_attributes() {
        let result = run_compound_viewer(
            r#"source="$id" layout="contents" index="rail" jump-to-member=#true jump-placeholder="e.g. a1r""#,
        )
        .unwrap_or_else(|e| panic!("stale jump attributes should be ignored, got: {:?}", kinds(&e)));
        assert!(matches!(
            result.pages[0].body[0],
            Component::CompoundViewer { picker: None, .. }
        ));
    }

    #[test]
    fn compound_viewer_lowers_member_links() {
        let src = format!(
            r#"{}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="pages" index="none" media="image" {{
        member-links label="strips" "strip-left" "strip-right"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("compound-viewer with member-links should pass");
        let Component::CompoundViewer { member_links, .. } = &result.pages[0].body[0] else {
            panic!("expected a compound-viewer: {:?}", result.pages[0].body[0]);
        };
        assert_eq!(
            member_links.as_ref(),
            Some(&MemberLinks {
                label: "strips".to_owned(),
                properties: vec!["strip-left".to_owned(), "strip-right".to_owned()],
            })
        );
    }

    #[test]
    fn compound_viewer_second_member_links_is_unknown_child() {
        let src = format!(
            r#"{}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="pages" index="none" media="image" {{
        member-links label="a" "strip-left"
        member-links label="b" "strip-right"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("two member-links rows should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::UnknownChildNode {
                component: "compound-viewer".to_owned(),
                child: "member-links".to_owned(),
            }),
            "got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn compound_viewer_member_links_without_label_is_missing_attribute() {
        let src = format!(
            r#"{}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="pages" index="none" media="image" {{
        member-links "strip-left"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("member-links without label should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
            "got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn compound_viewer_lowers_media_attribute() {
        let src = format!(
            r#"{}
page id="viewer" route="/view/:id" {{
    compound-viewer source="$id" layout="contents" index="rail" media="image"
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("compound-viewer with media should pass");
        assert_eq!(
            result.pages[0].body[0],
            Component::CompoundViewer {
                source: "$id".to_owned(),
                media: Some("image".to_owned()),
                picker: None,
                layout: CompoundLayout::Contents,
                index: MemberIndex::Rail,
                paired: false,
                member_links: None,
                annotations: false,
            }
        );
    }

    // ── compound-viewer layout= / index= ─────────

    /// Lower one `compound-viewer` declaration (`attrs` = everything after
    /// the node name) on a `$id` page.
    fn run_compound_viewer(attrs: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            r#"{GOOD_SITE}
page id="viewer" route="/view/:id" {{
    compound-viewer {attrs}
}}
"#
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    fn lowered_layout_index(attrs: &str) -> (CompoundLayout, MemberIndex) {
        let result =
            run_compound_viewer(attrs).unwrap_or_else(|e| panic!("`{attrs}` should lower, got: {:?}", kinds(&e)));
        match &result.pages[0].body[0] {
            Component::CompoundViewer { layout, index, .. } => (*layout, *index),
            other => panic!("expected CompoundViewer, got: {other:?}"),
        }
    }

    /// The failing report's messages of one kind, for asserting which
    /// attribute a `MissingRequiredAttribute` / `InvalidAttributeType` names.
    fn messages_of(report: &ValidationReport, kind: &DiagnosticKind) -> Vec<String> {
        report.iter().filter(|d| &d.kind == kind).map(|d| d.message.clone()).collect()
    }

    #[test]
    fn compound_viewer_lowers_every_layout_value() {
        assert_eq!(
            lowered_layout_index(r#"source="$id" layout="contents" index="rail""#).0,
            CompoundLayout::Contents
        );
        assert_eq!(
            lowered_layout_index(r#"source="$id" media="image" layout="pages" index="rail""#).0,
            CompoundLayout::Pages
        );
    }

    #[test]
    fn compound_viewer_lowers_every_index_value() {
        for (value, expected) in [
            ("rail", MemberIndex::Rail),
            ("collapsed", MemberIndex::Collapsed),
            ("none", MemberIndex::Off),
        ] {
            assert_eq!(
                lowered_layout_index(&format!(r#"source="$id" layout="contents" index="{value}""#)).1,
                expected,
                "index=\"{value}\""
            );
        }
    }

    #[test]
    fn compound_viewer_missing_layout_is_missing_required_attribute() {
        let err = run_compound_viewer(r#"source="$id" index="rail""#).expect_err("absent layout must fail");
        let msgs = messages_of(&err, &DiagnosticKind::MissingRequiredAttribute);
        assert_eq!(msgs.len(), 1, "got: {:?}", kinds(&err));
        assert!(msgs[0].contains("'layout'"), "got: {msgs:?}");
    }

    #[test]
    fn compound_viewer_missing_index_is_missing_required_attribute() {
        let err = run_compound_viewer(r#"source="$id" layout="contents""#).expect_err("absent index must fail");
        let msgs = messages_of(&err, &DiagnosticKind::MissingRequiredAttribute);
        assert_eq!(msgs.len(), 1, "got: {:?}", kinds(&err));
        assert!(msgs[0].contains("'index'"), "got: {msgs:?}");
    }

    #[test]
    fn compound_viewer_missing_layout_and_index_reports_both() {
        let err = run_compound_viewer(r#"source="$id""#).expect_err("both absent must fail");
        let msgs = messages_of(&err, &DiagnosticKind::MissingRequiredAttribute);
        assert_eq!(msgs.len(), 2, "both diagnostics at once, got: {msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("'layout'")), "got: {msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("'index'")), "got: {msgs:?}");
    }

    #[test]
    fn compound_viewer_unknown_layout_is_invalid_attribute_type_naming_vocabulary() {
        let err =
            run_compound_viewer(r#"source="$id" layout="scroll" index="rail""#).expect_err("unknown layout must fail");
        let msgs = messages_of(&err, &DiagnosticKind::InvalidAttributeType);
        assert_eq!(msgs.len(), 1, "got: {msgs:?}");
        assert!(
            msgs[0].contains("\"contents\"") && msgs[0].contains("\"pages\"") && msgs[0].contains("\"scroll\""),
            "the message names the vocabulary and the bad value, got: {msgs:?}"
        );
    }

    #[test]
    fn compound_viewer_unknown_index_is_invalid_attribute_type_naming_vocabulary() {
        let err =
            run_compound_viewer(r#"source="$id" layout="contents" index="off""#).expect_err("unknown index must fail");
        let msgs = messages_of(&err, &DiagnosticKind::InvalidAttributeType);
        assert_eq!(msgs.len(), 1, "got: {msgs:?}");
        assert!(
            msgs[0].contains("\"rail\"")
                && msgs[0].contains("\"collapsed\"")
                && msgs[0].contains("\"none\"")
                && msgs[0].contains("\"off\""),
            "the message names the vocabulary and the bad value, got: {msgs:?}"
        );
    }

    #[test]
    fn compound_viewer_wrong_typed_layout_and_index_are_invalid_attribute_type() {
        for attrs in [
            r#"source="$id" layout=#true index="rail""#,
            r#"source="$id" layout="contents" index=1"#,
        ] {
            let err = run_compound_viewer(attrs).expect_err("wrong-typed value must fail");
            assert_eq!(
                messages_of(&err, &DiagnosticKind::InvalidAttributeType).len(),
                1,
                "`{attrs}`, got: {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn compound_viewer_invalid_layout_or_index_skips_the_component() {
        // The component is skipped, never lowered with a default: a skipped
        // viewer takes part in no page-level check (here: a `pages`-less
        // duplicate that would otherwise trip DuplicateSingletonComponent).
        for attrs in [
            r#"source="$id" layout="scroll" index="rail""#,
            r#"source="$id" layout="contents" index="off""#,
            r#"source="$id" index="rail""#,
        ] {
            let err = run_compound_viewer(&format!(
                "{attrs}\n    compound-viewer source=\"$id\" layout=\"contents\" index=\"rail\""
            ))
            .expect_err("must fail");
            assert!(
                !kinds(&err)
                    .iter()
                    .any(|k| matches!(k, DiagnosticKind::DuplicateSingletonComponent { .. })),
                "`{attrs}` must be skipped, got: {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn two_compound_viewers_on_one_page_are_duplicate_singleton() {
        let err = run_compound_viewer(
            "source=\"$id\" layout=\"contents\" index=\"rail\"\n    \
             compound-viewer source=\"$id\" layout=\"contents\" index=\"none\"",
        )
        .expect_err("two compound-viewers must fail");
        assert!(
            err.iter().any(|d| matches!(
                &d.kind,
                DiagnosticKind::DuplicateSingletonComponent { component, page }
                    if component == "compound-viewer" && page == "/view/:id"
            )),
            "expected DuplicateSingletonComponent for compound-viewer, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn pages_layout_without_media_fails() {
        let err = run_compound_viewer(r#"source="$id" layout="pages" index="none""#)
            .expect_err("pages without media must fail");
        let found = err
            .iter()
            .find(|d| {
                matches!(
                    &d.kind,
                    DiagnosticKind::PagesLayoutWithoutMedia { page } if page == "/view/:id"
                )
            })
            .unwrap_or_else(|| panic!("expected PagesLayoutWithoutMedia, got: {:?}", kinds(&err)));
        assert_eq!(found.severity, Severity::Error);
        assert!(found.message.contains("media"), "got: {}", found.message);
    }

    #[test]
    fn pages_layout_with_media_and_contents_without_media_pass() {
        run_compound_viewer(r#"source="$id" media="image" layout="pages" index="none""#).expect("pages + media passes");
        run_compound_viewer(r#"source="$id" layout="contents" index="none""#).expect("contents without media passes");
    }

    /// A book page body: `resource-detail {detail}` beside `compound-viewer
    /// {viewer}`.
    fn run_detail_beside_viewer(detail: &str, viewer: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            r#"{GOOD_SITE}
page id="book" route="/books/:id" {{
    resource-detail class="Book" {detail}
    compound-viewer {viewer} layout="contents" index="rail"
}}
"#
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    fn has_media_ownership_error(report: &ValidationReport) -> bool {
        report.iter().any(|d| {
            matches!(
                &d.kind,
                DiagnosticKind::CompoundMediaOwnership { page } if page == "/books/:id"
            ) && d.severity == Severity::Error
        })
    }

    #[test]
    fn compound_media_ownership_fires_on_same_param_expression() {
        let err = run_detail_beside_viewer(r#"id="$id" media="image""#, r#"source="$id""#)
            .expect_err("a media-bearing record on the navigator's compound must fail");
        assert!(
            has_media_ownership_error(&err),
            "expected CompoundMediaOwnership, got: {:?}",
            kinds(&err)
        );
        let msg = &err
            .iter()
            .find(|d| matches!(d.kind, DiagnosticKind::CompoundMediaOwnership { .. }))
            .expect("diagnostic present")
            .message;
        assert!(msg.contains("drop 'media' from the resource-detail"), "got: {msg}");
    }

    #[test]
    fn compound_media_ownership_fires_on_same_literal() {
        let err =
            run_detail_beside_viewer(r#"id="B1" media="image""#, r#"source="B1""#).expect_err("same literal must fail");
        assert!(has_media_ownership_error(&err), "got: {:?}", kinds(&err));
    }

    #[test]
    fn compound_media_ownership_silent_on_different_expression_or_no_media() {
        for (detail, viewer) in [
            (r#"id="$id" media="image""#, r#"source="B1""#),
            (r#"id="B1" media="image""#, r#"source="B2""#),
            (r#"id="$id""#, r#"source="$id""#),
        ] {
            if let Err(err) = run_detail_beside_viewer(detail, viewer) {
                assert!(
                    !has_media_ownership_error(&err),
                    "`{detail}` + `{viewer}` must not pair, got: {:?}",
                    kinds(&err)
                );
            }
        }
    }

    /// A book page body: `resource-detail {detail}` beside a media-bearing
    /// `compound-viewer {viewer}` in the given `layout`.
    fn run_detail_beside_layout(detail: &str, viewer: &str, layout: &str) -> Result<ValidatedConfig, ValidationReport> {
        let src = format!(
            r#"{GOOD_SITE}
page id="book" route="/books/:id" {{
    resource-detail class="Book" {detail}
    compound-viewer {viewer} media="image" layout="{layout}" index="none"
}}
"#
        );
        run(&src, &ValidationContext::for_tests("my-project"))
    }

    fn multiple_viewer_errors(report: &ValidationReport) -> usize {
        report
            .iter()
            .filter(|d| {
                matches!(
                    &d.kind,
                    DiagnosticKind::MultipleImageViewers { page } if page == "/books/:id"
                ) && d.severity == Severity::Error
            })
            .count()
    }

    #[test]
    fn multiple_image_viewers_fires_for_pages_beside_a_different_expression_detail_media() {
        let err = run_detail_beside_layout(r#"id="B1" media="image""#, r#"source="$id""#, "pages")
            .expect_err("a pages reader beside a media-bearing record must fail");
        assert_eq!(multiple_viewer_errors(&err), 1, "got: {:?}", kinds(&err));
        assert!(!has_media_ownership_error(&err), "got: {:?}", kinds(&err));
        let msg = &err
            .iter()
            .find(|d| matches!(d.kind, DiagnosticKind::MultipleImageViewers { .. }))
            .expect("diagnostic present")
            .message;
        assert!(
            msg.contains("drop 'media' from the resource-detail")
                && msg.contains("expand")
                && msg.contains("page-global"),
            "the message names the shared signals and the fix, got: {msg}"
        );
    }

    #[test]
    fn multiple_image_viewers_silent_for_contents_or_a_detail_without_media() {
        for (detail, layout) in [
            (r#"id="B1" media="image""#, "contents"),
            (r#"id="B1""#, "pages"),
            (r#"id="$id""#, "pages"),
        ] {
            if let Err(err) = run_detail_beside_layout(detail, r#"source="$id""#, layout) {
                assert_eq!(
                    multiple_viewer_errors(&err),
                    0,
                    "`{detail}` + layout={layout} must not fire, got: {:?}",
                    kinds(&err)
                );
            }
        }
    }

    #[test]
    fn multiple_image_viewers_never_doubles_compound_media_ownership() {
        let err = run_detail_beside_layout(r#"id="$id" media="image""#, r#"source="$id""#, "pages")
            .expect_err("the paired case must fail");
        assert!(has_media_ownership_error(&err), "got: {:?}", kinds(&err));
        assert_eq!(
            multiple_viewer_errors(&err),
            0,
            "one mistake, one diagnostic, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn pairs_with_navigator_compares_expressions_and_never_pairs_absent_id() {
        let body = [Component::CompoundViewer {
            source: "$id".to_owned(),
            media: None,
            picker: None,
            layout: CompoundLayout::Contents,
            index: MemberIndex::Rail,
            paired: false,
            member_links: None,
            annotations: false,
        }];
        assert!(pairs_with_navigator(Some("$id"), &body));
        assert!(!pairs_with_navigator(Some("$other"), &body));
        assert!(!pairs_with_navigator(Some("id"), &body));
        assert!(!pairs_with_navigator(None, &body));
        assert!(!pairs_with_navigator(Some("$id"), &[]));
    }

    /// The lowered `navigator` flag of the page's (single) `resource-detail`.
    fn lowered_navigator(config: &ValidatedConfig) -> bool {
        config.pages[0]
            .body
            .iter()
            .find_map(|c| match c {
                Component::ResourceDetail { navigator, .. } => Some(*navigator),
                _ => None,
            })
            .expect("the page has a resource-detail")
    }

    #[test]
    fn navigator_flag_lowered_true_for_same_param_expression() {
        let config =
            run_detail_beside_viewer(r#"id="$id""#, r#"source="$id""#).expect("a paired record without media is valid");
        assert!(lowered_navigator(&config));
    }

    #[test]
    fn navigator_flag_lowered_false_for_a_different_expression() {
        let config = run_detail_beside_viewer(r#"id="$id""#, r#"source="B1""#).expect("an unpaired record is valid");
        assert!(!lowered_navigator(&config));
    }

    #[test]
    fn navigator_flag_lowered_false_without_a_compound_viewer() {
        let src = format!(
            r#"{GOOD_SITE}
page id="book" route="/books/:id" {{
    resource-detail id="$id" class="Book"
}}
"#
        );
        let config = run(&src, &ValidationContext::for_tests("my-project")).expect("a lone record is valid");
        assert!(!lowered_navigator(&config));
    }

    /// The lowered `paired` flag of the page's (single) `compound-viewer` —
    /// the mirror of `lowered_navigator`.
    fn lowered_paired(config: &ValidatedConfig) -> bool {
        config.pages[0]
            .body
            .iter()
            .find_map(|c| match c {
                Component::CompoundViewer { paired, .. } => Some(*paired),
                _ => None,
            })
            .expect("the page has a compound-viewer")
    }

    #[test]
    fn paired_flag_lowered_true_for_same_param_expression() {
        let config =
            run_detail_beside_viewer(r#"id="$id""#, r#"source="$id""#).expect("a paired record without media is valid");
        assert!(lowered_paired(&config));
    }

    #[test]
    fn paired_flag_lowered_false_for_a_different_expression() {
        let config = run_detail_beside_viewer(r#"id="$id""#, r#"source="B1""#).expect("an unpaired record is valid");
        assert!(!lowered_paired(&config));
    }

    #[test]
    fn where_clause_equals_lowers_correctly() {
        let src = format!(
            r#"{}
page id="featured" route="/featured" {{
    resource-list class="Book" {{
        where property="featured" equals=#true
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("where equals should pass");
        let page = &result.pages[0];
        if let Component::ResourceList { wheres, .. } = &page.body[0] {
            assert_eq!(wheres.len(), 1);
            assert_eq!(
                wheres[0],
                WhereClause::Equals { property: "featured".to_owned(), value: Value::Bool(true) }
            );
        } else {
            panic!("expected ResourceList");
        }
    }

    #[test]
    fn where_clause_range_lowers_correctly() {
        let src = format!(
            r#"{}
page id="range-test" route="/range" {{
    resource-list class="Book" {{
        where property="year" range-start=1480 range-end=1500
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("where range should pass");
        let page = &result.pages[0];
        if let Component::ResourceList { wheres, .. } = &page.body[0] {
            assert_eq!(wheres.len(), 1);
            assert_eq!(
                wheres[0],
                WhereClause::Range {
                    property: "year".to_owned(),
                    start: Some(Value::Integer(1480)),
                    end: Some(Value::Integer(1500)),
                }
            );
        } else {
            panic!("expected ResourceList");
        }
    }

    #[test]
    fn sort_direction_desc_lowers_correctly() {
        let src = format!(
            r#"{}
page id="sorted" route="/sorted" {{
    resource-list class="Book" {{
        sort property="title" direction="desc"
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("sort direction desc should pass");
        let page = &result.pages[0];
        if let Component::ResourceList { sorts, .. } = &page.body[0] {
            assert_eq!(sorts.len(), 1);
            assert_eq!(
                sorts[0],
                Sort {
                    key: SortKey::Property("title".to_owned()),
                    direction: SortDir::Desc,
                }
            );
        } else {
            panic!("expected ResourceList");
        }
    }

    #[test]
    fn limit_child_node_lowers_correctly() {
        let src = format!(
            r#"{}
page id="limited" route="/limited" {{
    resource-list class="Book" {{
        limit 6
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("limit should pass");
        let page = &result.pages[0];
        if let Component::ResourceList { pagination, .. } = &page.body[0] {
            assert_eq!(*pagination, Pagination::Limit(6));
        } else {
            panic!("expected ResourceList");
        }
    }

    // ── Per-page / limit integer bounds ────────────────────

    /// `per-page 0` must be rejected (0 is not a positive integer).
    #[test]
    fn per_page_zero_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        per-page 0
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("per-page 0 should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for per-page 0, got: {:?}",
            kinds(&err)
        );
    }

    /// `per-page 5000000000` (> u32::MAX = 4294967295) must be rejected.
    #[test]
    fn per_page_overflow_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        per-page 5000000000
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("per-page > u32::MAX should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for per-page overflow, got: {:?}",
            kinds(&err)
        );
    }

    /// `per-page 20` is a valid positive integer within u32 range and must pass.
    #[test]
    fn per_page_valid_passes() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        per-page 20
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let result = run(&src, &ctx).expect("per-page 20 should pass");
        let page = &result.pages[0];
        if let Component::ResourceList { pagination, .. } = &page.body[0] {
            assert_eq!(
                *pagination,
                Pagination::PerPage(20),
                "per-page 20 must lower to Pagination::PerPage(20)"
            );
        } else {
            panic!("expected ResourceList");
        }
    }

    /// `limit 0` must be rejected.
    #[test]
    fn limit_zero_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        limit 0
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("limit 0 should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for limit 0, got: {:?}",
            kinds(&err)
        );
    }

    /// `limit 5000000000` (> u32::MAX) must be rejected.
    #[test]
    fn limit_overflow_fails() {
        let src = format!(
            r#"{}
page id="home" route="/" {{
    resource-list class="Book" {{
        limit 5000000000
    }}
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("limit > u32::MAX should fail");
        assert!(
            kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
            "expected InvalidAttributeType for limit overflow, got: {:?}",
            kinds(&err)
        );
    }

    #[test]
    fn components_accumulate_errors_continue_past_failures() {
        // Two broken components in a row — both errors should appear.
        let src = format!(
            r#"{}
page id="home" route="/" {{
    narrative
    resource-list
}}
"#,
            GOOD_SITE
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("multiple component errors should fail");
        let k = kinds(&err);
        // Both components are missing required attrs — expect at least 2 errors.
        let missing_count = k.iter().filter(|k| **k == DiagnosticKind::MissingRequiredAttribute).count();
        assert!(missing_count >= 2, "expected at least 2 MissingRequiredAttribute, got: {:?}", k);
    }

    // ── `narrative-for` children on resource-detail ───────────────────────

    /// A `$id` book page whose `resource-detail` carries `detail_body`.
    fn narrative_for_src(detail_body: &str) -> String {
        format!(
            "{GOOD_SITE}\npage id=\"book\" route=\"/books/:id\" {{\n    resource-detail id=\"$id\" {{\n{detail_body}\n    }}\n}}\n"
        )
    }

    /// The Incunabula shape: DE + EN variants, no bare base file; default `de`.
    fn intro_inventory() -> Vec<String> {
        vec!["book-a.de.md".into(), "book-a.en.md".into(), "book-b.de.md".into()]
    }

    #[test]
    fn narrative_for_lowers_in_declaration_order() {
        let files = intro_inventory();
        let ctx = good_ctx(&files);
        let src = narrative_for_src(
            r#"        narrative-for "b" file="book-b.md"
        narrative-for "a" file="book-a.md""#,
        );
        let config = run(&src, &ctx).expect("variant-only intro files must pass");
        let Component::ResourceDetail { narrative_for, .. } = &config.pages[0].body[0] else {
            panic!("expected ResourceDetail, got {:?}", config.pages[0].body[0]);
        };
        assert_eq!(
            narrative_for,
            &vec![
                NarrativeFor { resource_id: "b".to_owned(), file: "book-b.md".to_owned() },
                NarrativeFor { resource_id: "a".to_owned(), file: "book-a.md".to_owned() },
            ]
        );
    }

    #[test]
    fn resource_detail_without_narrative_for_lowers_empty() {
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&narrative_for_src(""), &ctx).expect("plain detail must pass");
        let Component::ResourceDetail { narrative_for, .. } = &config.pages[0].body[0] else {
            panic!("expected ResourceDetail");
        };
        assert!(narrative_for.is_empty());
    }

    #[test]
    fn narrative_for_missing_file_is_narrative_file_not_found() {
        let files = intro_inventory();
        let ctx = good_ctx(&files);
        let src = narrative_for_src(r#"        narrative-for "a" file="book-missing.md""#);
        let err = run(&src, &ctx).expect_err("an absent intro file must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::NarrativeFileNotFound]);
    }

    #[test]
    fn narrative_for_only_non_default_variant_is_narrative_file_not_found() {
        // Only `book-c.en.md` exists: neither the base nor the default (`de`)
        // variant — the two rungs the render chain guarantees — is present.
        let files: Vec<String> = vec!["book-c.en.md".into()];
        let ctx = good_ctx(&files);
        let src = narrative_for_src(r#"        narrative-for "c" file="book-c.md""#);
        let err = run(&src, &ctx).expect_err("a non-default-only intro must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::NarrativeFileNotFound]);
    }

    #[test]
    fn narrative_for_path_separator_is_invalid_narrative_path() {
        let files = intro_inventory();
        let ctx = good_ctx(&files);
        let src = narrative_for_src(r#"        narrative-for "a" file="../book-a.md""#);
        let err = run(&src, &ctx).expect_err("a traversing intro path must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidNarrativePath]);
    }

    #[test]
    fn narrative_for_without_file_is_missing_required_attribute() {
        let ctx = ValidationContext::for_tests("my-project");
        let src = narrative_for_src(r#"        narrative-for "a""#);
        let err = run(&src, &ctx).expect_err("narrative-for without file= must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::MissingRequiredAttribute]);
    }

    #[test]
    fn narrative_for_without_resource_id_is_missing_required_attribute() {
        let ctx = ValidationContext::for_tests("my-project");
        let src = narrative_for_src(r#"        narrative-for file="book-a.md""#);
        let err = run(&src, &ctx).expect_err("narrative-for without an id must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::MissingRequiredAttribute]);
    }

    #[test]
    fn narrative_for_non_string_resource_id_is_invalid_attribute_type() {
        let ctx = ValidationContext::for_tests("my-project");
        let src = narrative_for_src(r#"        narrative-for 42 file="book-a.md""#);
        let err = run(&src, &ctx).expect_err("a non-string id must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn duplicate_narrative_for_is_an_error_reported_once() {
        let files = intro_inventory();
        let ctx = good_ctx(&files);
        let src = narrative_for_src(
            r#"        narrative-for "a" file="book-a.md"
        narrative-for "a" file="book-b.md"
        narrative-for "a" file="book-a.md"
        narrative-for "b" file="book-b.md""#,
        );
        let err = run(&src, &ctx).expect_err("two intros for one id must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::DuplicateNarrativeFor { resource_id: "a".to_owned(), page: "/books/:id".to_owned() }],
            "one diagnostic per repeated id"
        );
    }

    // ── `order-by` options on resource-list ──────────

    /// A site with two chrome strings the `order-by` tests can name as labels.
    const ORDER_SITE: &str = r#"
site {
    title "Test Project"
    languages "de" "en"
    chrome {
        string "order-book-date" {
            de "Datum"
            en "Date"
        }
        string "order-book-extent" {
            de "Umfang"
            en "Extent"
        }
    }
}
"#;

    fn order_src(children: &str) -> String {
        format!(
            "{ORDER_SITE}\npage id=\"home\" route=\"/\" {{\n    resource-list class=\"Book\" {{\n{children}\n    }}\n}}\n"
        )
    }

    #[test]
    fn order_by_lowers_all_attributes_in_config_order() {
        let src = order_src(
            r#"        sort property="pub-date"
        order-by key="date" property="pub-date" direction="asc" label="order-book-date"
        order-by key="extent" property="extent" direction="desc" label="order-book-extent""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("valid order-by options should lower");
        let Component::ResourceList { order_options, sorts, .. } = &config.pages[0].body[0] else {
            panic!("expected a ResourceList component");
        };
        assert_eq!(
            order_options,
            &vec![
                OrderOption {
                    key: "date".to_owned(),
                    property: "pub-date".to_owned(),
                    direction: SortDir::Asc,
                    label: "order-book-date".to_owned(),
                },
                OrderOption {
                    key: "extent".to_owned(),
                    property: "extent".to_owned(),
                    direction: SortDir::Desc,
                    label: "order-book-extent".to_owned(),
                },
            ]
        );
        // The default `sort` is untouched by the options.
        assert_eq!(sorts.len(), 1);
        // An option lowers to a plain property Sort — no new SortKey variant.
        assert_eq!(
            order_options[1].sort(),
            Sort {
                key: SortKey::Property("extent".to_owned()),
                direction: SortDir::Desc,
            }
        );
    }

    #[test]
    fn order_by_direction_defaults_to_asc() {
        let src = order_src(r#"        order-by key="date" property="pub-date" label="order-book-date""#);
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("order-by without direction should lower");
        let Component::ResourceList { order_options, .. } = &config.pages[0].body[0] else {
            panic!("expected a ResourceList component");
        };
        assert_eq!(order_options.len(), 1);
        assert_eq!(order_options[0].direction, SortDir::Asc);
    }

    #[test]
    fn resource_list_without_order_by_lowers_empty_options() {
        let src = order_src(r#"        sort property="pub-date""#);
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("plain resource-list should lower");
        let Component::ResourceList { order_options, .. } = &config.pages[0].body[0] else {
            panic!("expected a ResourceList component");
        };
        assert!(order_options.is_empty());
    }

    #[test]
    fn order_by_missing_required_attributes_fail() {
        for (child, attr) in [
            (r#"order-by property="pub-date" label="order-book-date""#, "key"),
            (r#"order-by key="date" label="order-book-date""#, "property"),
            (r#"order-by key="date" property="pub-date""#, "label"),
        ] {
            let src = order_src(&format!("        {child}"));
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err("an order-by missing an attribute must fail");
            assert!(
                kinds(&err).contains(&DiagnosticKind::MissingRequiredAttribute),
                "absent '{attr}' must be MissingRequiredAttribute, got: {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn order_by_wrong_typed_attributes_report_instead_of_vanishing() {
        // Every `order-by` attribute goes through a reporting
        // reader, so `key=#true` (a bool, not a string) is a boot error — not
        // a silently-dropped option.
        for child in [
            r#"order-by key=#true property="pub-date" label="order-book-date""#,
            r#"order-by key="date" property=1 label="order-book-date""#,
            r#"order-by key="date" property="pub-date" label=#false"#,
            r#"order-by key="date" property="pub-date" direction=#true label="order-book-date""#,
        ] {
            let src = order_src(&format!("        {child}"));
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err("a wrong-typed order-by attribute must fail");
            assert!(
                kinds(&err).contains(&DiagnosticKind::InvalidAttributeType),
                "`{child}` must be InvalidAttributeType, got: {:?}",
                kinds(&err)
            );
        }
    }

    #[test]
    fn order_by_bad_direction_value_fails() {
        let src =
            order_src(r#"        order-by key="date" property="pub-date" direction="up" label="order-book-date""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("direction outside asc/desc must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn order_by_reserved_keys_fail() {
        for key in RESERVED_ORDER_KEYS {
            let src = order_src(&format!(
                "        order-by key=\"{key}\" property=\"pub-date\" label=\"order-book-date\""
            ));
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err("a reserved order-by key must fail");
            assert_eq!(
                kinds(&err),
                vec![DiagnosticKind::ReservedOrderKey { key: (*key).to_owned() }],
                "key '{key}'"
            );
        }
    }

    #[test]
    fn order_by_invalid_keys_fail() {
        for key in ["", "Date", "pub date", "a&b", "dätum", "a_b"] {
            let src = order_src(&format!(
                "        order-by key=\"{key}\" property=\"pub-date\" label=\"order-book-date\""
            ));
            let ctx = ValidationContext::for_tests("my-project");
            let err = run(&src, &ctx).expect_err("a malformed order-by key must fail");
            assert_eq!(
                kinds(&err),
                vec![DiagnosticKind::InvalidOrderKey { key: key.to_owned() }],
                "key '{key}'"
            );
        }
    }

    #[test]
    fn order_by_well_formed_key_with_digits_and_hyphens_passes() {
        let src = order_src(r#"        order-by key="date-2" property="pub-date" label="order-book-date""#);
        let ctx = ValidationContext::for_tests("my-project");
        run(&src, &ctx).expect("a [a-z0-9-]+ key must lower");
    }

    #[test]
    fn order_by_duplicate_key_fails_once() {
        let src = order_src(
            r#"        order-by key="date" property="pub-date" label="order-book-date"
        order-by key="date" property="extent" label="order-book-extent""#,
        );
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("a duplicate order-by key must fail");
        assert_eq!(kinds(&err), vec![DiagnosticKind::DuplicateOrderKey { key: "date".to_owned() }]);
    }

    #[test]
    fn order_by_same_key_on_two_lists_passes() {
        // The duplicate check is per list, not project-wide.
        let src = format!(
            r#"{ORDER_SITE}
page id="a" route="/a" {{
    resource-list class="Book" {{
        order-by key="date" property="pub-date" label="order-book-date"
    }}
}}
page id="b" route="/b" {{
    resource-list class="Print" {{
        order-by key="date" property="pub-date" label="order-book-date"
    }}
}}
"#
        );
        let ctx = ValidationContext::for_tests("my-project");
        run(&src, &ctx).expect("the same key on two lists must lower");
    }

    #[test]
    fn order_by_undeclared_label_fails() {
        let src = order_src(r#"        order-by key="date" property="pub-date" label="order-typo""#);
        let ctx = ValidationContext::for_tests("my-project");
        let err = run(&src, &ctx).expect_err("an undeclared order-by label must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnknownOrderLabel { label: "order-typo".to_owned(), page: "/".to_owned() }]
        );
    }

    // ── timeline (Incunabula) ─────────────────────────────────────────

    fn timeline_src(attrs: &str) -> String {
        format!(
            r#"
site {{
    title "My Project"
    languages "de" "en"
}}
page id="home" route="/" {{
    timeline {attrs}
}}
"#
        )
    }

    #[test]
    fn timeline_lowers_all_attributes() {
        let src = timeline_src(
            r#"class="Book" date-property="pub-date" lane-property="office" extent-property="extent" axis-start=1470 axis-end=1506"#,
        );
        let ctx = ValidationContext::for_tests("incunabula");
        let config = run(&src, &ctx).expect("a complete timeline must lower");
        assert_eq!(
            config.pages[0].body,
            vec![Component::Timeline {
                class: "Book".to_owned(),
                date_property: "pub-date".to_owned(),
                lane_property: "office".to_owned(),
                extent_property: Some("extent".to_owned()),
                axis_start: 1470,
                axis_end: 1506,
            }]
        );
    }

    #[test]
    fn timeline_in_another_project_is_an_unknown_component() {
        // `(project, name)` keying: the timeline is registered for
        // Incunabula only, so a complete timeline node elsewhere is unknown,
        // and the message names both sets.
        let src = timeline_src(
            r#"class="Book" date-property="pub-date" lane-property="office" axis-start=1470 axis-end=1506"#,
        );
        let ctx = ValidationContext::for_tests("hdm");
        let err = run(&src, &ctx).expect_err("timeline is not registered for hdm");
        assert_eq!(kinds(&err), vec![DiagnosticKind::UnknownComponent]);
        let message = &err.iter().next().expect("one diagnostic").message;
        assert!(message.contains("search-bar"), "{message}");
        assert!(message.contains("'hdm'"), "{message}");
    }

    #[test]
    fn unknown_component_message_names_the_projects_registered_components() {
        let message = unknown_component_message("chart", "incunabula");
        assert!(message.contains("narrative"), "{message}");
        assert!(message.contains("timeline"), "{message}");
        assert!(message.contains("woodcut-cases"), "{message}");
        assert!(message.contains("edition-cards"), "{message}");
        assert!(message.contains("strip-row"), "{message}");
        let message = unknown_component_message("timeline", "alice-in-daschland");
        assert!(message.contains("no per-project components"), "{message}");
    }

    fn showcase_src(component: &str) -> String {
        format!(
            r#"
site {{
    title "My Project"
    languages "de" "en"
    chrome {{
        string "edition-german" {{
            de "Deutsch"
            en "German"
        }}
        string "gloss-extent" {{
            de "{{n}} Seiten"
            en "{{n}} pages"
        }}
    }}
}}
page id="home" route="/" {{
    narrative file="n.md"
    {component}
}}
"#
        )
    }

    #[test]
    fn edition_cards_and_strip_row_lower_in_order() {
        let src = showcase_src(
            r#"edition-cards section="editions" media="image" {
        book "dt" lang-label="edition-german"
        book "aug"
        imprint { part property="pub-place"; part property="extent" unit="gloss-extent" }
    }
    strip-row section="strips" class="BorderStrip" media="image" uses-property="page-uses" min-uses=9 limit=12"#,
        );
        let ctx = ValidationContext::for_tests("incunabula");
        let config = run(&src, &ctx).expect("both blocks lower");
        assert_eq!(
            config.pages[0].body[1..],
            [
                Component::EditionCards {
                    media: "image".to_owned(),
                    books: vec![
                        crate::ir::EditionCard {
                            book: "dt".to_owned(),
                            lang_label: Some("edition-german".to_owned()),
                        },
                        crate::ir::EditionCard { book: "aug".to_owned(), lang_label: None },
                    ],
                    imprint: vec![
                        GlossPart { property: "pub-place".to_owned(), unit: None },
                        GlossPart {
                            property: "extent".to_owned(),
                            unit: Some("gloss-extent".to_owned()),
                        },
                    ],
                },
                Component::StripRow {
                    class: "BorderStrip".to_owned(),
                    media: "image".to_owned(),
                    uses_property: "page-uses".to_owned(),
                    min_uses: 9,
                    limit: 12,
                },
            ]
        );
    }

    #[test]
    fn malformed_edition_cards_and_strip_rows_are_reported() {
        let ctx = ValidationContext::for_tests("incunabula");
        for (what, component, want) in [
            (
                "a strip row without a section",
                r#"strip-row class="S" media="image" uses-property="u" min-uses=1 limit=1"#,
                vec![DiagnosticKind::MissingRequiredAttribute],
            ),
            (
                "a strip row of no tile",
                r#"strip-row section="s" class="S" media="image" uses-property="u" min-uses=1 limit=0"#,
                vec![DiagnosticKind::InvalidAttributeType],
            ),
            (
                "a strip row over the tile cap",
                r#"strip-row section="s" class="S" media="image" uses-property="u" min-uses=1 limit=101"#,
                vec![DiagnosticKind::InvalidAttributeType],
            ),
            (
                "a negative minimum",
                r#"strip-row section="s" class="S" media="image" uses-property="u" min-uses=-1 limit=3"#,
                vec![DiagnosticKind::InvalidAttributeType],
            ),
            (
                "a book named twice",
                r#"edition-cards section="s" media="image" { book "dt"; book "dt" }"#,
                vec![DiagnosticKind::InvalidAttributeType],
            ),
            (
                "two imprints",
                r#"edition-cards section="s" media="image" { book "dt"; imprint { part property="a" }; imprint { part property="b" } }"#,
                vec![DiagnosticKind::InvalidAttributeType],
            ),
            (
                "no book",
                r#"edition-cards section="s" media="image" { imprint { part property="a" } }"#,
                vec![DiagnosticKind::MissingRequiredAttribute],
            ),
            (
                "an unknown child",
                r#"edition-cards section="s" media="image" { book "dt"; strip "l" }"#,
                vec![DiagnosticKind::UnknownChildNode {
                    component: "edition-cards".to_owned(),
                    child: "strip".to_owned(),
                }],
            ),
        ] {
            let err = run(&showcase_src(component), &ctx).expect_err(what);
            assert_eq!(kinds(&err), want, "{what}");
        }
    }

    #[test]
    fn no_registered_component_shadows_a_generic_one() {
        // The lowering `match` dispatches generic names first, so a registry
        // row reusing one would never be reached.
        for row in project_components::PROJECT_COMPONENTS {
            assert!(
                !V1_COMPONENTS.contains(&row.name),
                "registered component '{}' shadows a generic v1 component",
                row.name
            );
        }
    }

    #[test]
    fn timeline_without_extent_property_lowers_to_none() {
        let src = timeline_src(
            r#"class="Book" date-property="pub-date" lane-property="office" axis-start=1470 axis-end=1506"#,
        );
        let ctx = ValidationContext::for_tests("incunabula");
        let config = run(&src, &ctx).expect("extent-property is optional");
        assert!(matches!(
            &config.pages[0].body[0],
            Component::Timeline { extent_property: None, .. }
        ));
    }

    #[test]
    fn timeline_reports_every_attribute_defect_at_once() {
        // Missing class, string-typed axis-start, out-of-range axis-end:
        // three diagnostics from one node, no short-circuit.
        let src =
            timeline_src(r#"date-property="pub-date" lane-property="office" axis-start="1470" axis-end=9999999999"#);
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("a broken timeline must fail");
        let got = kinds(&err);
        assert_eq!(got.len(), 3, "{got:?}");
        assert!(got.contains(&DiagnosticKind::MissingRequiredAttribute));
        assert!(got.contains(&DiagnosticKind::InvalidAttributeType));
        assert!(got.contains(&DiagnosticKind::TimelineAxisOutOfRange { attribute: "axis-end".to_owned() }));
    }

    #[test]
    fn timeline_span_below_three_fails() {
        let src = timeline_src(
            r#"class="Book" date-property="pub-date" lane-property="office" axis-start=1470 axis-end=1472"#,
        );
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("a two-year axis must fail");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::InvalidTimelineAxis { axis_start: 1470, axis_end: 1472 }]
        );
    }

    // ── Figure-block slot ─────────────────────────────────────────────────────
    // The slot attributes of dsp-incubator cpe/docs/adr/0011-per-project-components.md.

    const TIMELINE_ATTRS: &str =
        r#"class="Book" date-property="pub-date" lane-property="office" axis-start=1470 axis-end=1506"#;

    /// A page whose body is `body`, with the `count`/`more` chrome strings
    /// and a `/strips` page for a more route to resolve against.
    fn slot_src(body: &str) -> String {
        format!(
            r#"
site {{
    title "T"
    languages "de" "en"
    chrome {{
        string "four-count" {{
            de "4 Fälle"
            en "4 cases"
        }}
        string "strips-all" {{
            de "Alle"
            en "All"
        }}
    }}
}}
page id="home" route="/" {{
{body}
}}
page id="strips" route="/strips"
"#
        )
    }

    #[test]
    fn figure_slot_lowers_aligned_with_the_body() {
        let src = slot_src(&format!(
            "    narrative file=\"woodcuts.md\"\n    timeline {TIMELINE_ATTRS}\n    timeline {TIMELINE_ATTRS} section=\"four\" count=\"four-count\" more-label=\"strips-all\" more-route=\"/strips\""
        ));
        let ctx = ValidationContext::for_tests("incunabula");
        let config = run(&src, &ctx).expect("a complete slot must lower");
        let page = &config.pages[0];
        assert_eq!(page.body.len(), 3);
        assert_eq!(page.figure_slots.len(), 3);
        assert_eq!(page.figure_slot(0), None);
        assert_eq!(page.figure_slot(1), None);
        assert_eq!(
            page.figure_slot(2),
            Some(&FigureSlot {
                section: "four".to_owned(),
                count: Some("four-count".to_owned()),
                more: Some(FigureMore { label: "strips-all".to_owned(), route: "/strips".to_owned() }),
            })
        );
    }

    #[test]
    fn figure_slot_drops_in_lockstep_with_its_component() {
        // The middle timeline fails lowering (no class): it and its slot both
        // go, so the third component keeps its own slot at index 1.
        let body = [
            "    narrative file=\"woodcuts.md\"".to_owned(),
            r#"    timeline date-property="pub-date" lane-property="office" axis-start=1470 axis-end=1506 section="lost""#.to_owned(),
            format!("    timeline {TIMELINE_ATTRS} section=\"kept\""),
        ]
        .join("\n");
        let mut report = ValidationReport::new();
        let raw = crate::config::parse(&slot_src(&body)).expect("parses");
        let ctx = ValidationContext::for_tests("incunabula");
        let (body, slots) = lower_page_body(&raw.pages[0].body, &[], false, &ctx, None, &mut Vec::new(), &mut report);
        assert_eq!(kinds(&report), vec![DiagnosticKind::MissingRequiredAttribute]);
        assert_eq!(body.len(), 2);
        assert_eq!(
            slots.iter().map(|s| s.as_ref().map(|s| s.section.as_str())).collect::<Vec<_>>(),
            vec![None, Some("kept")]
        );
    }

    #[test]
    fn figure_slot_without_a_preceding_narrative_fails() {
        let src = slot_src(&format!(
            "    timeline {TIMELINE_ATTRS} section=\"four\"\n    narrative file=\"woodcuts.md\""
        ));
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("a slot needs a narrative before it");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::FigureSectionWithoutNarrative { section: "four".to_owned() }]
        );
    }

    #[test]
    fn figure_slot_more_link_needs_both_halves_and_a_section() {
        for attrs in [
            r#"section="four" more-label="strips-all""#,
            r#"section="four" more-route="/strips""#,
            r#"count="four-count""#,
        ] {
            let src = slot_src(&format!(
                "    narrative file=\"woodcuts.md\"\n    timeline {TIMELINE_ATTRS} {attrs}"
            ));
            let ctx = ValidationContext::for_tests("incunabula");
            let err = run(&src, &ctx).expect_err(attrs);
            assert_eq!(kinds(&err), vec![DiagnosticKind::MissingRequiredAttribute], "{attrs}");
        }
    }

    #[test]
    fn figure_slot_wrong_typed_attribute_reports_once() {
        let src = slot_src(&format!(
            "    narrative file=\"woodcuts.md\"\n    timeline {TIMELINE_ATTRS} section=4"
        ));
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("section must be a string");
        assert_eq!(kinds(&err), vec![DiagnosticKind::InvalidAttributeType]);
    }

    #[test]
    fn heading_ids_colliding_with_engine_ids_are_rejected() {
        let rejected = [
            "cv-title",
            "cv-status",
            "header-controls",
            "header-menu",
            "case-x",
            "crumb-data-entry-1",
            "annotation-x",
        ];
        let md: String = rejected
            .iter()
            .map(|id| format!("## H {{#{id}}}\n\n"))
            .chain(std::iter::once("## Fine {#four}\n".to_owned()))
            .collect();
        let narratives = vec![("woodcuts.de.md".to_owned(), md)];
        let mut report = ValidationReport::new();
        check_single_heading_ids("woodcuts.md", &narratives, &mut report);
        let ids: Vec<String> = report
            .iter()
            .map(|d| match &d.kind {
                DiagnosticKind::InvalidHeadingId { id, .. } => id.clone(),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(ids, rejected.map(str::to_owned).to_vec());
    }

    /// The `DuplicateHeadingIdOnPage` ids `check_narrative_headings` reports
    /// for `pages` (KDL page nodes) over `narratives`, with their pages.
    fn duplicate_heading_ids(pages: &str, narratives: &[(&str, &str)]) -> Vec<(String, String)> {
        let src = format!("site {{\n    title \"T\"\n    languages \"de\" \"en\"\n}}\n{pages}");
        let ctx = ValidationContext::for_tests("my-project");
        let config = run(&src, &ctx).expect("the config itself is valid");
        let narratives: Vec<(String, String)> = narratives
            .iter()
            .map(|(name, content)| ((*name).to_owned(), (*content).to_owned()))
            .collect();
        let mut report = ValidationReport::new();
        check_narrative_headings(&config, &narratives, &mut report);
        report
            .iter()
            .map(|d| match &d.kind {
                DiagnosticKind::DuplicateHeadingIdOnPage { id, page } => (id.clone(), page.clone()),
                other => panic!("unexpected diagnostic {other:?}"),
            })
            .collect()
    }

    /// The fallback case: on the German page `a.md` serves its base file and
    /// `b.md` its German variant. No variant file is shared, yet the page
    /// holds the id twice.
    #[test]
    fn duplicate_heading_id_is_found_through_the_narrative_fallback() {
        let pages = r#"page id="home" route="/" {
    narrative file="a.md"
    narrative file="b.md"
}"#;
        let got = duplicate_heading_ids(pages, &[("a.md", "## A {#same}\n"), ("b.de.md", "## B {#same}\n")]);
        assert_eq!(got, vec![("same".to_owned(), "/".to_owned())]);
    }

    /// A variant that drops the id in one language still collides in the
    /// other, and is reported once.
    #[test]
    fn duplicate_heading_id_in_one_language_only_is_reported_once() {
        let pages = r#"page id="home" route="/" {
    narrative file="a.md"
    narrative file="b.md"
}"#;
        let got = duplicate_heading_ids(
            pages,
            &[
                ("a.md", "## A {#same}\n"),
                ("b.de.md", "## B {#other}\n"),
                ("b.en.md", "## B {#same}\n"),
            ],
        );
        assert_eq!(got, vec![("same".to_owned(), "/".to_owned())]);
    }

    #[test]
    fn the_same_narrative_twice_on_a_page_duplicates_its_heading_ids() {
        let pages = r#"page id="home" route="/" {
    narrative file="a.md"
    narrative file="a.md"
}"#;
        let got = duplicate_heading_ids(pages, &[("a.md", "## A {#same}\n")]);
        assert_eq!(got, vec![("same".to_owned(), "/".to_owned())]);
    }

    #[test]
    fn the_same_heading_id_on_two_pages_is_accepted() {
        let pages = r#"page id="home" route="/" {
    narrative file="a.md"
}
page id="other" route="/other" {
    narrative file="b.md"
    narrative file="c.md"
}"#;
        let got = duplicate_heading_ids(
            pages,
            &[
                ("a.md", "## A {#same}\n"),
                ("b.md", "## B {#same}\n"),
                ("c.md", "## C {#different}\n"),
            ],
        );
        assert_eq!(got, Vec::<(String, String)>::new());
    }

    /// `sec-<n>` is the id the engine generates for an id-less `##`, and
    /// `narr-rail-cap` the page rail's caption: an author may use neither.
    /// `section` shows the check accepts an ordinary id in the same file.
    #[test]
    fn engine_generated_heading_ids_are_reserved() {
        let narratives = vec![(
            "a.md".to_owned(),
            "## One {#sec-1}\n\n## Two {#narr-rail-cap}\n\n## Three {#section}\n".to_owned(),
        )];
        let mut report = ValidationReport::new();
        check_single_heading_ids("a.md", &narratives, &mut report);
        assert_eq!(
            kinds(&report),
            vec![
                DiagnosticKind::InvalidHeadingId { id: "sec-1".to_owned(), file: "a.md".to_owned() },
                DiagnosticKind::InvalidHeadingId { id: "narr-rail-cap".to_owned(), file: "a.md".to_owned() },
            ]
        );
    }

    /// A section of invite groups is an invite section: a figure block
    /// cannot attach to it, as it cannot to a section with a plain invite
    /// list. `plain` shows the check accepts an ordinary section of the file.
    #[test]
    fn a_figure_block_on_a_groups_section_is_rejected() {
        let narratives = vec![(
            "a.md".to_owned(),
            "## Offices {#offices}\n\n### Amerbach\n\n- [Book A](/a) \u{2014} first\n\n\
             ## Plain {#plain}\n\nText.\n"
                .to_owned(),
        )];
        let mut report = ValidationReport::new();
        check_single_figure_section("plain", "a.md", "/", &narratives, &mut report);
        assert_eq!(kinds(&report), vec![]);
        check_single_figure_section("offices", "a.md", "/", &narratives, &mut report);
        assert_eq!(
            kinds(&report),
            vec![DiagnosticKind::FigureSectionHoldsInviteList {
                section: "offices".to_owned(),
                file: "a.md".to_owned(),
                page: "/".to_owned(),
            }]
        );
    }

    #[test]
    fn narrative_variants_are_the_base_file_and_its_language_variants() {
        let pairs: Vec<(String, String)> = [
            "woodcuts.md",
            "woodcuts.de.md",
            "woodcuts.en.md",
            "woodcuts-old.de.md",
            "woodcuts.de.extra.md",
            "home.de.md",
        ]
        .iter()
        .map(|n| ((*n).to_owned(), String::new()))
        .collect();
        let names: Vec<&str> = narrative_variants("woodcuts.md", &pairs)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, vec!["woodcuts.md", "woodcuts.de.md", "woodcuts.en.md"]);
    }

    #[test]
    fn figure_slot_more_route_must_not_be_parameterised() {
        let src = slot_src(&format!(
            "    narrative file=\"woodcuts.md\"\n    timeline {TIMELINE_ATTRS} section=\"four\" more-label=\"strips-all\" more-route=\"/strips/:id\""
        ));
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("a more link cannot fill ':id'");
        assert_eq!(
            kinds(&err),
            vec![DiagnosticKind::UnresolvedFigureMoreRoute { route: "/strips/:id".to_owned(), page: "/".to_owned() }]
        );
    }

    #[test]
    fn figure_slot_labels_and_route_must_resolve() {
        let src = slot_src(&format!(
            "    narrative file=\"woodcuts.md\"\n    timeline {TIMELINE_ATTRS} section=\"four\" count=\"nope\" more-label=\"strips-all\" more-route=\"/nowhere\""
        ));
        let ctx = ValidationContext::for_tests("incunabula");
        let err = run(&src, &ctx).expect_err("an unknown label and route must fail");
        let got = kinds(&err);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got.contains(&DiagnosticKind::UnknownFigureLabel { label: "nope".to_owned(), page: "/".to_owned() }));
        assert!(got.contains(&DiagnosticKind::UnresolvedFigureMoreRoute {
            route: "/nowhere".to_owned(),
            page: "/".to_owned(),
        }));
    }
}
