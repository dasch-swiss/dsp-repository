//! The validator half of the `(project, name)` registry of per-project custom
//! components (dsp-incubator cpe/docs/adr/0011-per-project-components.md).
//!
//! A per-project component is valid only for the project it is registered
//! for: the validator's allowlist accepts a component name when it is one of
//! the generic v1 components **or** has a row here for the project being
//! loaded, and lowering and both arms of the data-dependent checks find the
//! component's functions through the same row. Adding a per-project component
//! is one row here plus one in the renderer half
//! (`component::project::PROJECT_RENDERERS`), with the same `(project, name)`
//! key at the same position; `component::project`'s
//! `renderer_rows_match_validator_rows` test holds the two tables together,
//! and its `registry_names_match_row_names` holds `registry_name` to them.
//! It does not touch `validate.rs`'s allowlist or its lowering `match`.
//!
//! What a new per-project component still adds (the seam's known limits):
//! an `ir::Component` variant (the row's `lower` returns one), an explicit
//! `render_component` arm delegating to `component::project::render` (kept
//! explicit on purpose, so a new *generic* variant still needs its own arm to
//! compile), an `ir::Component::registry_name` arm, a fetch arm in the
//! store's page builder (`query::page`; renderers have no DB access), and possibly a
//! `ComponentData` variant (in `view_data`). A config-internal check that
//! spans a whole page or every page is outside the row too and is called by
//! name from `validate.rs`: the showcase figures have two,
//! `showcase::check_showcase_labels` (in `validate_inner`, the chrome ids of
//! `woodcut-cases` and `edition-cards`) and `showcase::check_duplicate_case_keys`
//! (in `check_and_lower_pages`).
//!
//! The registry is split by crate: this half, in `cpe-core`, names only
//! validation types, and the renderer half, in `cpe-components`,
//! only render types, so the validator never imports a renderer. A second
//! project reusing a component is one more row in each half pointing at the
//! same functions.

use super::context::ValidationContext;
use super::facts::ProjectFacts;
use crate::diagnostic::ValidationReport;
use crate::ir;

/// Lower one raw component node into its typed IR variant. Reports its own
/// attribute defects; `None` means the node did not lower.
pub(crate) type LowerFn =
    fn(&crate::config::Component, &ValidationContext<'_>, &mut ValidationReport) -> Option<ir::Component>;

/// The success-arm data check for one lowered component on page `route`.
pub(crate) type CheckDataFn = fn(&ir::Component, &str, &dyn ProjectFacts, &mut ValidationReport);

/// The raw-arm sibling of [`CheckDataFn`], run on the raw node when
/// config-internal lowering already failed. Must stay silent on attribute
/// defects lowering already reported (`dsp-incubator cpe/CLAUDE.md`,
/// "`validate_with_data`'s two arms are symmetric").
pub(crate) type CheckDataIntoRawFn = fn(&crate::config::Component, &str, &dyn ProjectFacts, &mut ValidationReport);

/// One registered per-project component, as the validator sees it.
pub(crate) struct ProjectComponent {
    /// The project id the component is valid for; it is unknown elsewhere.
    pub(crate) project: &'static str,
    /// The KDL node name (and `ir::Component::registry_name`).
    pub(crate) name: &'static str,
    pub(crate) lower: LowerFn,
    pub(crate) check_data: CheckDataFn,
    pub(crate) check_data_into_raw: CheckDataIntoRawFn,
}

/// Every registered per-project component.
pub(crate) const PROJECT_COMPONENTS: &[ProjectComponent] = &[
    ProjectComponent {
        project: "incunabula",
        name: "timeline",
        lower: super::timeline::lower_timeline,
        check_data: super::timeline::check_timeline_data,
        check_data_into_raw: super::timeline::check_timeline_data_into_raw,
    },
    ProjectComponent {
        project: "incunabula",
        name: "woodcut-cases",
        lower: super::showcase::lower_woodcut_cases,
        check_data: super::showcase::check_woodcut_cases_data,
        check_data_into_raw: super::showcase::check_woodcut_cases_data_into_raw,
    },
    ProjectComponent {
        project: "incunabula",
        name: "edition-cards",
        lower: super::showcase::lower_edition_cards,
        check_data: super::showcase::check_edition_cards_data,
        check_data_into_raw: super::showcase::check_edition_cards_data_into_raw,
    },
    ProjectComponent {
        project: "incunabula",
        name: "strip-row",
        lower: super::showcase::lower_strip_row,
        check_data: super::showcase::check_strip_row_data,
        check_data_into_raw: super::showcase::check_strip_row_data_into_raw,
    },
];

/// The row registered for `(project, name)`, if any.
pub(crate) fn lookup(project: &str, name: &str) -> Option<&'static ProjectComponent> {
    PROJECT_COMPONENTS.iter().find(|row| row.project == project && row.name == name)
}

/// The `(project, name)` key of every registered row, in registry order.
#[doc(hidden)]
pub fn registry_keys() -> Vec<(&'static str, &'static str)> {
    PROJECT_COMPONENTS.iter().map(|row| (row.project, row.name)).collect()
}

/// The names registered for `project`, in registry order.
pub(crate) fn names_for(project: &str) -> Vec<&'static str> {
    PROJECT_COMPONENTS
        .iter()
        .filter(|row| row.project == project)
        .map(|row| row.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_keyed_on_project_and_name() {
        assert!(lookup("incunabula", "timeline").is_some());
        assert!(lookup("incunabula", "woodcut-cases").is_some());
        assert!(lookup("incunabula", "edition-cards").is_some());
        assert!(lookup("incunabula", "strip-row").is_some());
        assert!(lookup("hdm", "woodcut-cases").is_none());
        assert!(lookup("hdm", "strip-row").is_none());
        assert!(lookup("hdm", "timeline").is_none());
        assert!(lookup("incunabula", "narrative").is_none());
    }

    #[test]
    fn registry_rows_are_unique_per_project_and_name() {
        for (i, a) in PROJECT_COMPONENTS.iter().enumerate() {
            for b in &PROJECT_COMPONENTS[i + 1..] {
                assert!(
                    !(a.project == b.project && a.name == b.name),
                    "duplicate registry row ({}, {})",
                    a.project,
                    a.name
                );
            }
        }
    }

    #[test]
    fn names_for_lists_only_the_projects_own_rows() {
        assert_eq!(
            names_for("incunabula"),
            vec!["timeline", "woodcut-cases", "edition-cards", "strip-row"]
        );
        assert!(names_for("alice-in-daschland").is_empty());
    }
}
