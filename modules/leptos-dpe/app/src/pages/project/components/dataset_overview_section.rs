use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use crate::domain::{lang_value, Project};
use crate::pages::project::components::coverage_section::CoverageSection;
use crate::pages::project::components::data_language_section::DataLanguageSection;
use crate::pages::project::components::disciplines_section::DisciplinesSection;
use crate::pages::project::components::link_card_section::LinkCardSection;
use crate::pages::project::components::link_list_section::LinkListSection;
use crate::pages::project::components::publication_year::PublicationYear;
use crate::pages::project::components::type_of_data_section::TypeOfDataSection;

#[component]
pub fn DatasetOverviewSection(proj: Project) -> impl IntoView {
    // Collect all keyword values across all language maps
    let all_keywords: Vec<String> = proj.keywords.iter().flat_map(|map| map.values().cloned()).collect();
    let data_languages: Vec<String> = proj
        .data_language
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|map| lang_value(map).cloned())
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
            <PublicationYear year=proj.data_publication_year.clone() />

            {(!all_keywords.is_empty())
                .then(|| {
                    view! {
                        <div class="scroll-mt-52">
                            <h3 class="dpe-subtitle">"Keywords"</h3>
                            <div class="flex flex-wrap gap-1.5">
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

            <LinkCardSection title="Part of Cluster".to_string() items=cluster_items />

            <LinkCardSection title="Collections".to_string() items=collection_items />

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
                                <p class="text-sm">{prov}</p>
                            </CardBody>
                        </Card>
                    }
                })}
        </div>
    }
}
