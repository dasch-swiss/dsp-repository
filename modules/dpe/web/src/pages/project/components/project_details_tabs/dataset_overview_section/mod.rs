mod coverage_section;
mod data_language_section;
mod disciplines_section;
mod link_card_section;
mod link_list_section;
mod publication_year;
mod type_of_data_section;

use coverage_section::coverage_section;
use data_language_section::data_language_section;
use disciplines_section::disciplines_section;
use dpe_core::Project;
use link_card_section::link_card_section;
use link_list_section::link_list_section;
use maud::{html, Markup};
use mosaic_tiles::card::{card, card_body, CardProps, CardVariant};
use publication_year::publication_year;
use type_of_data_section::type_of_data_section;

/// The "Overview" tab panel: type of data, data languages, publication year,
/// keywords, disciplines, coverage, clusters, collections, documentation /
/// additional material, and provenance.
pub fn dataset_overview_section(proj: &Project) -> Markup {
    let all_keywords: Vec<String> = proj.keywords.iter().flat_map(|map| map.values().cloned()).collect();
    let data_languages: Vec<String> = proj
        .data_language
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|code| dpe_core::language_display_name(code).to_string())
        .collect();
    let cluster_items: Vec<(String, String, String)> = proj
        .clusters
        .iter()
        .map(|c| (format!("/cluster/{}", c.id), c.name.clone(), c.description.clone()))
        .collect();
    let collection_items: Vec<(String, String, String)> = proj
        .collections
        .iter()
        .map(|c| (format!("/collection/{}", c.id), c.name.clone(), c.description.clone()))
        .collect();

    html! {
        div class="space-y-4" {
            (type_of_data_section(proj.type_of_data.as_deref()))
            (data_language_section(&data_languages))
            @if let Some(year) = &proj.data_publication_year {
                div { (publication_year(year)) }
            }
            @if !all_keywords.is_empty() {
                div {
                    h3 class="dpe-subtitle" { "Keywords" }
                    div class="flex flex-wrap gap-2" {
                        @for k in &all_keywords {
                            span
                                class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-neutral-100 text-neutral-700"
                            { (k) }
                        }
                    }
                }
            }
            (disciplines_section(&proj.disciplines))
            (coverage_section(&proj.temporal_coverage, &proj.spatial_coverage))
            (link_card_section("Part of Cluster", &cluster_items, false))
            (link_card_section("Collections", &collection_items, false))
            @if let Some(docs) = proj.documentation_material.as_ref().filter(|d| !d.is_empty()) {
                (link_list_section("Documentation Material", docs, true))
            }
            @if let Some(materials) = proj.additional_material.as_ref().filter(|a| !a.is_empty()) {
                (link_list_section("Additional Material", materials, true))
            }
            @if let Some(prov) = &proj.provenance {
                ({
                    card(
                        CardProps {
                            variant: CardVariant::Bordered,
                            class: "",
                        },
                        card_body(
                            "",
                            html! {
                                h3 class = "text-base font-semibold mb-3" { "Provenance" } p
                                { (prov) }
                            },
                        ),
                    )
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn renders_overview_sections_from_project() {
        let out = dataset_overview_section(&sample_project()).into_string();
        assert!(out.contains("Type of Data"), "{out}");
        assert!(out.contains("Data Languages"), "{out}");
        assert!(out.contains("Data Publication Year"), "{out}");
        assert!(out.contains("Keywords"), "{out}");
        assert!(out.contains("archaeology"), "keyword value: {out}");
    }
}
