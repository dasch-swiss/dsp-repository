use leptos::prelude::*;

use crate::components::project_detail::coverage_section::CoverageSection;
use crate::components::project_detail::disciplines_section::DisciplinesSection;
use crate::components::project_detail::lang_utils::{group_by_language_as_paragraphs, lang_map_to_views};
use crate::components::project_detail::link_list_section::LinkListSection;
use crate::components::project_detail::project_metadata::ProjectMetadata;
use crate::components::project_detail::publications_section::PublicationsSection;
use crate::components::project_detail::type_of_data_section::TypeOfDataSection;
use crate::components::*;
use crate::domain::Project;

#[component]
pub fn DatasetOverviewSection(proj: Project) -> impl IntoView {
    let _descriptions = lang_map_to_views(&proj.description);
    let abstracts = lang_map_to_views(&proj.abstract_text.clone().unwrap_or_default());
    let english_keywords: Vec<String> = proj.keywords.iter().filter_map(|map| map.get("en").cloned()).collect();
    let data_languages: Vec<String> = proj
        .data_language
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|map| map.get("en").cloned())
        .collect();
    let alt_names_content = proj
        .alternative_names
        .as_deref()
        .map(group_by_language_as_paragraphs)
        .unwrap_or_default();

    view! {
        <TypeOfDataSection type_of_data=proj.type_of_data.clone() />

        {(!data_languages.is_empty())
            .then(|| {
                view! {
                    <div id="data-languages" class="scroll-mt-52">
                        <h3 class="text-xl font-bold mb-3">"Data Languages"</h3>
                        <div class="flex flex-wrap gap-2">
                            {data_languages
                                .iter()
                                .map(|l| {
                                    view! {
                                        <span class="badge badge-primary">{l.clone()}</span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                }
                    .into_any()
            })}

        <ProjectMetadata
            start_date=proj.start_date.clone()
            end_date=proj.end_date.clone()
            data_publication_year=proj.data_publication_year.clone()
            url=proj.url.clone()
        />

        {(!english_keywords.is_empty())
            .then(|| {
                view! {
                    <div id="keywords" class="scroll-mt-52">
                        <h3 class="text-xl font-bold mb-3">"Keywords"</h3>
                        <div class="flex flex-wrap gap-2">
                            {english_keywords
                                .iter()
                                .map(|k| {
                                    view! {
                                        <span class="badge badge-primary">{k.clone()}</span>
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

        <div id="abstract" class="scroll-mt-52">
            <LanguageTabs
                title="Abstract".to_string()
                content=abstracts
            />
        </div>

        {proj
            .publications
            .as_ref()
            .map(|publications| {
                view! {
                    <PublicationsSection publications=publications.clone() />
                }
            })}



        {(!alt_names_content.is_empty())
            .then(|| {
                view! {
                    <div id="alternative-names" class="scroll-mt-52">
                        <LanguageTabs
                            title="Alternative Names".to_string()
                            content=alt_names_content
                        />
                    </div>
                }
                    .into_any()
            })}

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
                        <h3 class="text-xl font-bold mb-3">"Provenance"</h3>
                        <p class="text-base">{prov.clone()}</p>
                    </div>
                }
            })}
    }
}
