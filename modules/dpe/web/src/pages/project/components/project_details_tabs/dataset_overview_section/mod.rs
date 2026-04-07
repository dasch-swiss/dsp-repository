mod coverage_section;
mod data_language_section;
mod disciplines_section;
mod link_card_section;
mod link_list_section;
mod publication_year;
mod type_of_data_section;

use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use crate::domain::Project;
use coverage_section::CoverageSection;
use data_language_section::DataLanguageSection;
use disciplines_section::DisciplinesSection;
use link_card_section::LinkCardSection;
use link_list_section::LinkListSection;
use publication_year::PublicationYear;
use type_of_data_section::TypeOfDataSection;

#[component]
pub fn DatasetOverviewSection(proj: Project) -> impl IntoView {
    // Collect all keyword values across all language maps
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

    view! {
        <div class="space-y-4">
            <TypeOfDataSection type_of_data=proj.type_of_data.clone() />

            <DataLanguageSection data_languages=data_languages />
            {proj
                .data_publication_year
                .clone()
                .map(|year| {
                    view! {
                        <div>
                            <PublicationYear year=year />
                        </div>
                    }
                })}

            {(!all_keywords.is_empty())
                .then(|| {
                    view! {
                        <div>
                            <h3 class="dpe-subtitle">"Keywords"</h3>
                            <div class="flex flex-wrap gap-2">
                                {all_keywords
                                    .into_iter()
                                    .map(|k| {
                                        view! {
                                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-neutral-100 text-neutral-700">
                                                {k}
                                            </span>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                        .into_any()
                })}

            <DisciplinesSection disciplines=proj.disciplines.clone() />

            <CoverageSection
                temporal_coverage=proj.temporal_coverage.clone()
                spatial_coverage=proj.spatial_coverage.clone()
            />

            <LinkCardSection
                title="Part of Cluster".to_string()
                items=cluster_items
                clickable=false
            />

            <LinkCardSection
                title="Collections".to_string()
                items=collection_items
                clickable=false
            />

            {proj
                .documentation_material
                .as_ref()
                .and_then(|d| if d.is_empty() { None } else { Some(d) })
                .map(|docs| {
                    view! {
                        <LinkListSection
                            title="Documentation Material".to_string()
                            items=docs.clone()
                            as_links=true
                        />
                    }
                })}

            {proj
                .additional_material
                .as_ref()
                .and_then(|a| if a.is_empty() { None } else { Some(a) })
                .map(|materials| {
                    view! {
                        <LinkListSection
                            title="Additional Material".to_string()
                            items=materials.clone()
                            as_links=true
                        />
                    }
                })}

            {proj
                .provenance
                .clone()
                .map(|prov| {
                    view! {
                        <Card variant=CardVariant::Bordered>
                            <CardBody>
                                <h3 class="text-base font-semibold mb-3">"Provenance"</h3>
                                <p>{prov}</p>
                            </CardBody>
                        </Card>
                    }
                })}
        </div>
    }
}
