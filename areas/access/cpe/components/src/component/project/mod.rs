//! Per-project custom components — **not generic** — and the renderer half of
//! their `(project, name)` registry (`dsp-incubator cpe/docs/adr/0011-per-project-components.md`).
//!
//! Everything under `component::project::<project-id>` is a component written
//! for one project, not part of the generic v1 component library beside it.
//! The module path is the whole signal: a component living here is the
//! per-project escape hatch, registered through the `(project, name)` seam that `dsp-incubator
//! cpe/docs/adr/0011-per-project-components.md` records.
//!
//! Like every other component, a per-project component takes pre-fetched data
//! (no DB access) and renders synchronously.
//!
//! Rendering, a figure-block slot's `{n}` and the dataset-panel listing find a
//! component's functions through its `PROJECT_RENDERERS` row, keyed on the
//! rendering project's id and the variant's `registry_name`. The validator
//! half, and the rule that keeps the two tables in step, is
//! `validate::project_components`.

use cpe_core::ir;
use cpe_core::view_data::ComponentData;
use maud::{html, Markup};

use crate::component::context::RenderContext;

pub(crate) mod incunabula;

/// Render one lowered component from its pre-fetched data slot. `site` gives
/// the renderer the project's chrome strings and labels, which
/// `RenderContext` does not carry.
type RenderFn = fn(&ir::Component, &RenderContext<'_>, &ComponentData, &ir::SiteConfig) -> Markup;

/// The number a figure-block slot's `{n}` stands for, read from the
/// component's pre-fetched data slot; `None` leaves the label as written.
type SlotNFn = fn(&ComponentData) -> Option<usize>;

/// What a slotted component lists in the crumb line's "Show in dataset"
/// panel, from its pre-fetched data slot: its case groups in render order,
/// each entry only with a data ARK (`data_arks`). An empty `Vec` lists
/// nothing. `site` gives the class labels the entries' roles read.
pub(crate) type DataListingFn = fn(
    &ir::Component,
    &ComponentData,
    &ir::SiteConfig,
    &cpe_core::lang::Lang,
    &ir::DataArks,
) -> Vec<crate::component::data_entries::DataGroup>;

/// One registered per-project component, as the renderer sees it.
struct ProjectRenderer {
    /// The project id the component is valid for; it is unknown elsewhere.
    project: &'static str,
    /// The KDL node name (and `ir::Component::registry_name`).
    name: &'static str,
    render: RenderFn,
    /// Fills `{n}` in the component's slot `count=` and `more-label=` text
    /// (`render_body`). `None` ⇒ the labels render as written.
    slot_n: Option<SlotNFn>,
    /// What the component lists in the dataset panel when it renders in a
    /// figure-block slot (`data_links::page_data_panel`, grouped under its
    /// section's heading). `None` ⇒ it lists nothing.
    data_listing: Option<DataListingFn>,
}

/// Every registered per-project component.
const PROJECT_RENDERERS: &[ProjectRenderer] = &[
    ProjectRenderer {
        project: "incunabula",
        name: "timeline",
        render: incunabula::timeline::render_registered,
        slot_n: None,
        data_listing: None,
    },
    ProjectRenderer {
        project: "incunabula",
        name: "woodcut-cases",
        render: incunabula::showcase::render_registered,
        slot_n: None,
        data_listing: Some(incunabula::showcase::woodcut_cases_listing),
    },
    ProjectRenderer {
        project: "incunabula",
        name: "edition-cards",
        render: incunabula::showcase::render_edition_cards_registered,
        slot_n: None,
        // The edition cards' Books are not listed, nor are the strip row's
        // BorderStrips.
        data_listing: None,
    },
    ProjectRenderer {
        project: "incunabula",
        name: "strip-row",
        render: incunabula::showcase::render_strip_row_registered,
        slot_n: Some(incunabula::showcase::strip_row_slot_n),
        data_listing: None,
    },
];

/// The row registered for `(project, name)`, if any.
fn lookup(project: &str, name: &str) -> Option<&'static ProjectRenderer> {
    PROJECT_RENDERERS.iter().find(|row| row.project == project && row.name == name)
}

/// Render a per-project component through its registry row, found by the
/// rendering project's id and the variant's `registry_name`. Renders nothing
/// when no row matches — unreachable after boot validation, which only lowers
/// a per-project component for a project it is registered for.
pub fn render(c: &ir::Component, ctx: &RenderContext<'_>, data: &ComponentData, site: &ir::SiteConfig) -> Markup {
    match c.registry_name().and_then(|name| lookup(ctx.project_id, name)) {
        Some(row) => (row.render)(c, ctx, data, site),
        None => html! {},
    }
}

/// The `{n}` of the slot of `c` (a per-project component of `project`), from
/// its data slot, through its row's `ProjectRenderer::slot_n`.
pub fn slot_n(project: &str, c: &ir::Component, data: &ComponentData) -> Option<usize> {
    let row = lookup(project, c.registry_name()?)?;
    row.slot_n?(data)
}

/// The dataset-panel listing hook of `c` (a per-project component of
/// `project`), through its row's [`ProjectRenderer::data_listing`].
pub(crate) fn data_listing(project: &str, c: &ir::Component) -> Option<DataListingFn> {
    lookup(project, c.registry_name()?)?.data_listing
}

#[cfg(test)]
mod tests {
    use cpe_core::validate::project_components::registry_keys;

    use super::*;

    #[test]
    fn renderer_rows_match_validator_rows() {
        let renderers: Vec<(&str, &str)> = PROJECT_RENDERERS.iter().map(|row| (row.project, row.name)).collect();
        let validators: Vec<(&str, &str)> = registry_keys();
        assert_eq!(
            renderers, validators,
            "component::project::PROJECT_RENDERERS (left) and \
             validate::project_components::PROJECT_COMPONENTS (right) must list the \
             same (project, name) keys in the same order: add the missing row"
        );
    }

    /// The third copy of a row's name is `ir::Component::registry_name`: a
    /// variant whose name has no row renders nothing and skips its data checks.
    #[test]
    fn registry_names_match_row_names() {
        use std::collections::BTreeSet;
        // One value per per-project variant; a new one is added here too.
        let variants = [
            ir::Component::Timeline {
                class: String::new(),
                date_property: String::new(),
                lane_property: String::new(),
                extent_property: None,
                axis_start: 0,
                axis_end: 0,
            },
            ir::Component::WoodcutCases {
                media: String::new(),
                layout: ir::CaseLayout::Edition,
                cases: Vec::new(),
            },
            ir::Component::EditionCards { media: String::new(), books: Vec::new(), imprint: Vec::new() },
            ir::Component::StripRow {
                class: String::new(),
                media: String::new(),
                uses_property: String::new(),
                min_uses: 0,
                limit: 0,
            },
        ];
        let variant_names: BTreeSet<&str> = variants
            .iter()
            .map(|c| c.registry_name().expect("a per-project variant has a registry name"))
            .collect();
        let row_names: BTreeSet<&str> = PROJECT_RENDERERS.iter().map(|row| row.name).collect();
        assert_eq!(
            variant_names, row_names,
            "ir::Component::registry_name (left) and the registry rows' names (right) \
             must be the same set"
        );
    }
}
