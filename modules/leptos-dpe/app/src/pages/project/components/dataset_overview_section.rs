use leptos::prelude::*;

use crate::pages::project::components::coverage_section::CoverageSection;
use crate::pages::project::components::disciplines_section::DisciplinesSection;
use crate::pages::project::components::lang_utils::lang_map_to_views;
use crate::pages::project::components::link_list_section::LinkListSection;
use crate::pages::project::components::publication_year::PublicationYear;
use crate::pages::project::components::type_of_data_section::TypeOfDataSection;
use crate::domain::Project;

#[component]
pub fn DatasetOverviewSection(proj: Project) -> impl IntoView {
    let _descriptions = lang_map_to_views(&proj.description);
    let english_keywords: Vec<String> = proj.keywords.iter().filter_map(|map| map.get("en").cloned()).collect();
    let data_languages: Vec<String> = proj
        .data_language
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|map| map.get("en").cloned())
        .collect();
    view! {
        <div class="space-y-4">
        <TypeOfDataSection type_of_data=proj.type_of_data.clone() />

        {(!data_languages.is_empty())
            .then(|| {
                view! {
                    <div id="data-languages" class="scroll-mt-52">
                        <h3 class="dpe-subtitle">"Data Languages"</h3>
                        <div class="flex flex-wrap gap-2">
                            {data_languages
                                .iter()
                                .map(|l| {
                                    view! {
                                        <span class="badge badge-primary text-xs">{l.clone()}</span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                }
                    .into_any()
            })}
                <PublicationYear year=proj.data_publication_year.clone() />

        {(!english_keywords.is_empty())
            .then(|| {
                view! {
                    <div id="keywords" class="scroll-mt-52">
                        <h3 class="dpe-subtitle">"Keywords"</h3>
                        <div class="flex flex-wrap gap-2">
                            {english_keywords
                                .iter()
                                .map(|k| {
                                    view! {
                                        <span class="badge badge-primary text-xs">{k.clone()}</span>
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

        {proj
            .collections
            .as_ref()
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .map(|collections| {
                view! {
                    <LinkListSection
                        title="Collections".to_string()
                        items=collections.clone()
                    />
                }
            })}

        {proj
            .records
            .as_ref()
            .and_then(|r| if r.is_empty() { None } else { Some(r) })
            .map(|records| {
                view! {
                    <LinkListSection
                        title="Records".to_string()
                        items=records.clone()
                    />
                }
            })}

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
            .as_ref()
            .map(|prov| {
                view! {
                    <div class="bg-base-100 p-6 rounded-lg">
                        <h3 class="text-base font-semibold mb-3">"Provenance"</h3>
                        <p class="text-sm">{prov.clone()}</p>
                    </div>
                }
            })}
        </div>
    }
}
