use leptos::prelude::*;

use crate::components::project_detail::access_rights_section::AccessRightsSection;
use crate::components::project_detail::attributions_section::AttributionsSection;
use crate::components::project_detail::coverage_section::CoverageSection;
use crate::components::project_detail::disciplines_section::DisciplinesSection;
use crate::components::project_detail::funding_section::FundingSection;
use crate::components::project_detail::lang_utils::{
    group_by_language_as_badges, group_by_language_as_paragraphs, lang_map_to_views,
};
use crate::components::project_detail::link_list_section::LinkListSection;
use crate::components::project_detail::project_header::ProjectHeader;
use crate::components::project_detail::project_metadata::ProjectMetadata;
use crate::components::project_detail::publications_section::PublicationsSection;
use crate::components::*;
use crate::domain::Project;

#[component]
pub fn ProjectDetails(proj: Project) -> impl IntoView {
    let descriptions = lang_map_to_views(&proj.description);
    let abstracts = lang_map_to_views(&proj.abstract_text.clone().unwrap_or_default());
    let keywords_content = group_by_language_as_badges(&proj.keywords);
    let data_languages_content = proj
        .data_language
        .as_deref()
        .map(group_by_language_as_badges)
        .unwrap_or_default();
    let alt_names_content = proj
        .alternative_names
        .as_deref()
        .map(group_by_language_as_paragraphs)
        .unwrap_or_default();

    view! {
        <div class="space-y-6">
            <ProjectHeader
                shortcode=proj.shortcode.clone()
                name=proj.name.clone()
                status=proj.status.clone()
                short_description=proj.short_description.clone()
            />

            <TableOfContents />

            <div id="description" class="scroll-mt-52">
                <LanguageTabs
                    title="Description".to_string()
                    content=descriptions
                />
            </div>

            <ProjectMetadata
                start_date=proj.start_date.clone()
                end_date=proj.end_date.clone()
                data_publication_year=proj.data_publication_year.clone()
                url=proj.url.clone()
                type_of_data=proj.type_of_data.clone()
            />

            {(!keywords_content.is_empty())
                .then(|| {
                    view! {
                        <div id="keywords" class="scroll-mt-52">
                            <LanguageTabs
                                title="Keywords".to_string()
                                content=keywords_content
                            />
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

            <FundingSection funding=proj.funding.clone() />

            {proj
                .publications
                .as_ref()
                .map(|publications| {
                    view! {
                        <PublicationsSection publications=publications.clone() />
                    }
                })}

            {(!proj.legal_info.is_empty())
                .then(|| {
                    view! {
                        <div
                            id="legal-information"
                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                        >
                            <h3 class="text-xl font-bold mb-3">
                                "Legal Information"
                            </h3>
                            <LegalInfo legal_info=proj.legal_info.clone() />
                        </div>
                    }
                })}

            <HowToCite citation=proj.how_to_cite.clone() />

            <AccessRightsSection access_rights=proj.access_rights.clone() />

            {(!data_languages_content.is_empty())
                .then(|| {
                    view! {
                        <div id="data-languages" class="scroll-mt-52">
                            <LanguageTabs
                                title="Data Languages".to_string()
                                content=data_languages_content
                            />
                        </div>
                    }
                        .into_any()
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

            <AttributionsSection attributions=proj.attributions.clone() />
        </div>
    }
}
