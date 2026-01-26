use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::components::*;
use crate::domain::get_project;

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params_map();
    let shortcode = move || params.read().get("id").unwrap_or_default();

    let project = Resource::new(shortcode, |shortcode| async move {
        get_project(shortcode).await
    });

    view! {
        <Title text=move || format!("Project {}", shortcode()) />
        <div class="min-h-100 py-6">
            <Suspense fallback=move || {
                view! { <Loading /> }
            }>
                {move || {
                    project
                        .get()
                        .map(|result| match result {
                            Ok(Some(proj)) => {
                                let descriptions = proj
                                    .description
                                    .iter()
                                    .map(|(lang, text)| {
                                        (
                                            lang.clone(),
                                            view! { <p class="leading-relaxed">{text.clone()}</p> }
                                                .into_any(),
                                        )
                                    })
                                    .collect();
                                let abstracts = proj
                                    .abstract_text
                                    .clone()
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|(lang, text)| {
                                        (
                                            lang.clone(),
                                            view! { <p class="leading-relaxed">{text.clone()}</p> }
                                                .into_any(),
                                        )
                                    })
                                    .collect();
                                let keywords_by_lang: std::collections::HashMap<
                                    String,
                                    Vec<String>,
                                > = proj
                                    .keywords
                                    .iter()
                                    .flat_map(|kw_map| kw_map.iter())
                                    .fold(
                                        std::collections::HashMap::new(),
                                        |mut acc, (lang, text)| {
                                            acc.entry(lang.clone())
                                                .or_insert_with(Vec::new)
                                                .push(text.clone());
                                            acc
                                        },
                                    );
                                let keywords_content: std::collections::HashMap<String, AnyView> = keywords_by_lang
                                    .into_iter()
                                    .map(|(lang, keywords)| {
                                        (
                                            lang.clone(),

                                            // Group keywords by language

                                            view! {
                                                <div class="flex flex-wrap gap-2">
                                                    {keywords
                                                        .into_iter()
                                                        .map(|kw| {
                                                            view! { <span class="badge badge-primary">{kw}</span> }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                                .into_any(),
                                        )
                                    })
                                    .collect();
                                let data_languages_by_lang: std::collections::HashMap<
                                    String,
                                    Vec<String>,
                                > = proj
                                    .data_language
                                    .as_ref()
                                    .map(|langs| {
                                        langs
                                            .iter()
                                            .flat_map(|lang_map| lang_map.iter())
                                            .fold(
                                                std::collections::HashMap::new(),
                                                |mut acc, (lang, text)| {
                                                    acc.entry(lang.clone())
                                                        .or_insert_with(Vec::new)
                                                        .push(text.clone());
                                                    acc
                                                },
                                            )
                                    })
                                    .unwrap_or_default();
                                let data_languages_content: std::collections::HashMap<
                                    String,
                                    AnyView,
                                > = data_languages_by_lang
                                    .into_iter()
                                    .map(|(lang, languages)| {
                                        (
                                            lang.clone(),

                                            // Group data languages by language

                                            view! {
                                                <div class="flex flex-wrap gap-2">
                                                    {languages
                                                        .into_iter()
                                                        .map(|lang_name| {
                                                            view! {
                                                                <span class="badge badge-primary">{lang_name}</span>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                                .into_any(),
                                        )
                                    })
                                    .collect();
                                let alt_names_by_lang: std::collections::HashMap<
                                    String,
                                    Vec<String>,
                                > = proj
                                    .alternative_names
                                    .as_ref()
                                    .map(|names| {
                                        names
                                            .iter()
                                            .flat_map(|name_map| name_map.iter())
                                            .fold(
                                                std::collections::HashMap::new(),
                                                |mut acc, (lang, text)| {
                                                    acc.entry(lang.clone())
                                                        .or_insert_with(Vec::new)
                                                        .push(text.clone());
                                                    acc
                                                },
                                            )
                                    })
                                    .unwrap_or_default();
                                let alt_names_content: std::collections::HashMap<String, AnyView> = alt_names_by_lang
                                    .into_iter()
                                    .map(|(lang, names)| {
                                        (
                                            lang.clone(),

                                            // Group alternative names by language

                                            view! {
                                                <div class="space-y-2">
                                                    {names
                                                        .into_iter()
                                                        .map(|name| {
                                                            view! { <p>{name}</p> }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                                .into_any(),
                                        )
                                    })
                                    .collect();

                                view! {
                                    <div class="space-y-6">
                                        <div class="bg-base-200 p-6 rounded-lg">
                                            <div class="flex justify-between items-start">
                                                <div>
                                                    <div class="text-sm text-base-content/70 mb-2">
                                                        "Project " {proj.shortcode.clone()}
                                                    </div>
                                                    <h1 class="text-3xl font-bold mb-3">{proj.name.clone()}</h1>
                                                </div>
                                                <ProjectStatusBadge status=proj.status.clone() />
                                            </div>
                                            <p class="text-lg mt-4">{proj.short_description.clone()}</p>
                                        </div>

                                        <TableOfContents />

                                        <div id="description" class="scroll-mt-52">
                                            <LanguageTabs
                                                title="Description".to_string()
                                                content=descriptions
                                            />
                                        </div>

                                        <div class="grid md:grid-cols-2 gap-6">
                                            <div
                                                id="project-details"
                                                class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                            >
                                                <h3 class="text-xl font-bold mb-3">"Project Details"</h3>
                                                <div class="space-y-2">
                                                    <div>
                                                        <span class="font-semibold">"Start Date: "</span>
                                                        {proj.start_date.clone()}
                                                    </div>
                                                    <div>
                                                        <span class="font-semibold">"End Date: "</span>
                                                        {proj.end_date.clone()}
                                                    </div>
                                                    {proj
                                                        .data_publication_year
                                                        .as_ref()
                                                        .map(|year| {
                                                            view! {
                                                                <div>
                                                                    <span class="font-semibold">"Publication Year: "</span>
                                                                    {year.clone()}
                                                                </div>
                                                            }
                                                        })}
                                                    {(!proj.url.is_empty())
                                                        .then(|| {
                                                            view! {
                                                                <div>
                                                                    <span class="font-semibold">"URLs:"</span>
                                                                    <ul class="list-disc list-inside ml-2">
                                                                        {proj
                                                                            .url
                                                                            .iter()
                                                                            .map(|url| {
                                                                                view! {
                                                                                    <li>
                                                                                        <a href=url.clone() class="link link-primary">
                                                                                            {url.clone()}
                                                                                        </a>
                                                                                    </li>
                                                                                }
                                                                            })
                                                                            .collect_view()}
                                                                    </ul>
                                                                </div>
                                                            }
                                                                .into_any()
                                                        })}
                                                </div>
                                            </div>

                                            {proj
                                                .type_of_data
                                                .as_ref()
                                                .map(|types| {
                                                    view! {
                                                        <div
                                                            id="type-of-data"
                                                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                        >
                                                            <h3 class="text-xl font-bold mb-3">"Type of Data"</h3>
                                                            <div class="flex flex-wrap gap-2">
                                                                {types
                                                                    .iter()
                                                                    .map(|t| {
                                                                        view! {
                                                                            <span class="badge badge-primary">{t.clone()}</span>
                                                                        }
                                                                    })
                                                                    .collect_view()}
                                                            </div>
                                                        </div>
                                                    }
                                                })}
                                        </div>

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

                                        {(!proj.disciplines.is_empty())
                                            .then(|| {
                                                view! {
                                                    <div
                                                        id="disciplines"
                                                        class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                    >
                                                        <h3 class="text-xl font-bold mb-3">"Disciplines"</h3>
                                                        <ul class="list-disc list-inside space-y-1">
                                                            {proj
                                                                .disciplines
                                                                .iter()
                                                                .map(|d| match d {
                                                                    crate::domain::Discipline::Text(map) => {
                                                                        view! {
                                                                            <li>
                                                                                <a
                                                                                    href="http://www.snf.ch/SiteCollectionDocuments/allg_disziplinenliste.pdf"
                                                                                    class="link link-primary"
                                                                                >
                                                                                    {map
                                                                                        .iter()
                                                                                        .map(|(lang, text)| { format!("{} ({})", text, lang) })
                                                                                        .collect::<Vec<_>>()
                                                                                        .join(" / ")}
                                                                                </a>
                                                                            </li>
                                                                        }
                                                                            .into_any()
                                                                    }
                                                                    crate::domain::Discipline::Reference(ref_) => {
                                                                        view! {
                                                                            <li>
                                                                                <a href=ref_.url.clone() class="link link-primary">
                                                                                    {ref_.text.clone().unwrap_or_else(|| ref_.url.clone())}
                                                                                </a>
                                                                                <span class="text-sm text-base-content/70 ml-2">
                                                                                    "(" {ref_.type_.clone()} ")"
                                                                                </span>
                                                                            </li>
                                                                        }
                                                                            .into_any()
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </ul>
                                                    </div>
                                                }
                                                    .into_any()
                                            })}

                                        <div class="grid md:grid-cols-2 gap-6">
                                            {(!proj.temporal_coverage.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <div
                                                            id="temporal-coverage"
                                                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                        >
                                                            <h3 class="text-xl font-bold mb-3">"Temporal Coverage"</h3>
                                                            <div class="space-y-2">
                                                                {proj
                                                                    .temporal_coverage
                                                                    .iter()
                                                                    .map(|t| match t {
                                                                        crate::domain::TemporalCoverage::Text(map) => {
                                                                            view! {
                                                                                <div>
                                                                                    {map
                                                                                        .iter()
                                                                                        .map(|(lang, text)| { format!("{} ({})", text, lang) })
                                                                                        .collect::<Vec<_>>()
                                                                                        .join(" / ")}
                                                                                </div>
                                                                            }
                                                                                .into_any()
                                                                        }
                                                                        crate::domain::TemporalCoverage::Reference(ref_) => {
                                                                            view! {
                                                                                <div>
                                                                                    <a href=ref_.url.clone() class="link link-primary">
                                                                                        {ref_.text.clone().unwrap_or_else(|| ref_.url.clone())}
                                                                                    </a>
                                                                                    <span class="text-sm text-base-content/70 ml-2">
                                                                                        "(" {ref_.type_.clone()} ")"
                                                                                    </span>
                                                                                </div>
                                                                            }
                                                                                .into_any()
                                                                        }
                                                                    })
                                                                    .collect_view()}
                                                            </div>
                                                        </div>
                                                    }
                                                        .into_any()
                                                })}
                                            {(!proj.spatial_coverage.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <div
                                                            id="spatial-coverage"
                                                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                        >
                                                            <h3 class="text-xl font-bold mb-3">"Spatial Coverage"</h3>
                                                            <div class="space-y-2">
                                                                {proj
                                                                    .spatial_coverage
                                                                    .iter()
                                                                    .map(|s| {
                                                                        view! {
                                                                            <div>
                                                                                <a href=s.url.clone() class="link link-primary">
                                                                                    {s.text.clone().unwrap_or_else(|| s.url.clone())}
                                                                                </a>
                                                                                <span class="text-sm text-base-content/70 ml-2">
                                                                                    "(" {s.type_.clone()} ")"
                                                                                </span>
                                                                            </div>
                                                                        }
                                                                    })
                                                                    .collect_view()}
                                                            </div>
                                                        </div>
                                                    }
                                                        .into_any()
                                                })}
                                        </div>

                                        <div id="abstract" class="scroll-mt-52">
                                            <LanguageTabs
                                                title="Abstract".to_string()
                                                content=abstracts
                                            />
                                        </div>

                                        <div
                                            id="funding"
                                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                        >
                                            <h3 class="text-xl font-bold mb-3">"Funding"</h3>
                                            {match &proj.funding {
                                                crate::domain::Funding::Grants(grants) => {
                                                    view! {
                                                        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                            {grants
                                                                .iter()
                                                                .map(|grant| {
                                                                    view! {
                                                                        <div class="border-l-4 border-primary pl-4 space-y-2">
                                                                            {grant
                                                                                .name
                                                                                .as_ref()
                                                                                .map(|name| {
                                                                                    view! { <div class="font-semibold">{name.clone()}</div> }
                                                                                })}
                                                                            {grant
                                                                                .number
                                                                                .as_ref()
                                                                                .map(|number| {
                                                                                    view! {
                                                                                        <div class="text-sm">"Grant Number: " {number.clone()}</div>
                                                                                    }
                                                                                })}
                                                                            <div class="text-sm">
                                                                                "Funders: "
                                                                                {grant
                                                                                    .funders
                                                                                    .iter()
                                                                                    .enumerate()
                                                                                    .map(|(i, funder_id)| {
                                                                                        view! {
                                                                                            <span>
                                                                                                {if i > 0 { ", " } else { "" }}
                                                                                                <OrganizationName organization_id=funder_id.clone() />
                                                                                            </span>
                                                                                        }
                                                                                    })
                                                                                    .collect_view()}
                                                                            </div>
                                                                            {grant
                                                                                .url
                                                                                .as_ref()
                                                                                .map(|url| {
                                                                                    view! { <UrlBadge url=url.clone() /> }
                                                                                })}
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </div>
                                                    }
                                                        .into_any()
                                                }
                                                crate::domain::Funding::Text(text) => {
                                                    view! {
                                                        <div class="text-base-content/70">{text.clone()}</div>
                                                    }
                                                        .into_any()
                                                }
                                            }}
                                        </div>

                                        {proj
                                            .publications
                                            .as_ref()
                                            .map(|publications| {
                                                view! {
                                                    <div
                                                        id="publications"
                                                        class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                    >
                                                        <h3 class="text-xl font-bold mb-3">"Publications"</h3>
                                                        <div class="space-y-2">
                                                            {publications
                                                                .iter()
                                                                .map(|pub_| {
                                                                    view! {
                                                                        <div>
                                                                            {(!pub_.text.is_empty())
                                                                                .then(|| {
                                                                                    view! { <span>{pub_.text.clone()} " "</span> }.into_any()
                                                                                })}
                                                                            {pub_
                                                                                .pid
                                                                                .as_ref()
                                                                                .map(|pid| {
                                                                                    view! {
                                                                                        <a href=pid.url.clone() class="link link-primary ml-2">
                                                                                            {pid.text.clone().unwrap_or_else(|| pid.url.clone())}
                                                                                        </a>
                                                                                    }
                                                                                })}
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </div>
                                                    </div>
                                                }
                                            })}

                                        {(!proj.legal_info.is_empty())
                                            .then(|| {
                                                view! {
                                                    <div
                                                        id="legal-information"
                                                        class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                    >
                                                        <h3 class="text-xl font-bold mb-3">"Legal Information"</h3>
                                                        <LegalInfo legal_info=proj.legal_info.clone() />
                                                    </div>
                                                }
                                            })}

                                        <HowToCite citation=proj.how_to_cite.clone() />

                                        <div
                                            id="access-rights"
                                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                        >
                                            <h3 class="text-xl font-bold mb-3">"Access Rights"</h3>
                                            <div class="space-y-2">
                                                <div class="badge badge-primary badge-lg">
                                                    {match proj.access_rights.access_rights {
                                                        crate::domain::AccessRightsType::FullOpenAccess => {
                                                            "Full Open Access"
                                                        }
                                                        crate::domain::AccessRightsType::OpenAccessWithRestrictions => {
                                                            "Open Access with Restrictions"
                                                        }
                                                        crate::domain::AccessRightsType::EmbargoedAccess => {
                                                            "Embargoed Access"
                                                        }
                                                        crate::domain::AccessRightsType::MetadataOnlyAccess => {
                                                            "Metadata only Access"
                                                        }
                                                    }}
                                                </div>
                                                {proj
                                                    .access_rights
                                                    .embargo_date
                                                    .as_ref()
                                                    .map(|date| {
                                                        view! {
                                                            <div class="text-sm">"Embargo Date: " {date.clone()}</div>
                                                        }
                                                    })}
                                            </div>
                                        </div>

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
                                                    <div class="bg-base-100 p-6 rounded-lg">
                                                        <h3 class="text-xl font-bold mb-3">"Collections"</h3>
                                                        <ul class="list-disc list-inside">
                                                            {collections
                                                                .iter()
                                                                .map(|c| {
                                                                    view! { <li>{c.clone()}</li> }
                                                                })
                                                                .collect_view()}
                                                        </ul>
                                                    </div>
                                                }
                                            })}

                                        {proj
                                            .records
                                            .as_ref()
                                            .and_then(|r| if r.is_empty() { None } else { Some(r) })
                                            .map(|records| {
                                                view! {
                                                    <div class="bg-base-100 p-6 rounded-lg">
                                                        <h3 class="text-xl font-bold mb-3">"Records"</h3>
                                                        <ul class="list-disc list-inside">
                                                            {records
                                                                .iter()
                                                                .map(|r| {
                                                                    view! { <li>{r.clone()}</li> }
                                                                })
                                                                .collect_view()}
                                                        </ul>
                                                    </div>
                                                }
                                            })}

                                        {proj
                                            .documentation_material
                                            .as_ref()
                                            .and_then(|d| if d.is_empty() { None } else { Some(d) })
                                            .map(|docs| {
                                                view! {
                                                    <div class="bg-base-100 p-6 rounded-lg">
                                                        <h3 class="text-xl font-bold mb-3">
                                                            "Documentation Material"
                                                        </h3>
                                                        <ul class="list-disc list-inside">
                                                            {docs
                                                                .iter()
                                                                .map(|doc| {
                                                                    view! {
                                                                        <li>
                                                                            <a href=doc.clone() class="link link-primary">
                                                                                {doc.clone()}
                                                                            </a>
                                                                        </li>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </ul>
                                                    </div>
                                                }
                                            })}

                                        {proj
                                            .additional_material
                                            .as_ref()
                                            .and_then(|a| if a.is_empty() { None } else { Some(a) })
                                            .map(|materials| {
                                                view! {
                                                    <div class="bg-base-100 p-6 rounded-lg">
                                                        <h3 class="text-xl font-bold mb-3">
                                                            "Additional Material"
                                                        </h3>
                                                        <ul class="list-disc list-inside">
                                                            {materials
                                                                .iter()
                                                                .map(|material| {
                                                                    view! {
                                                                        <li>
                                                                            <a href=material.clone() class="link link-primary">
                                                                                {material.clone()}
                                                                            </a>
                                                                        </li>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </ul>
                                                    </div>
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

                                        {(!proj.attributions.is_empty())
                                            .then(|| {
                                                view! {
                                                    <div
                                                        id="attributions"
                                                        class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                                                    >
                                                        <h3 class="text-xl font-bold mb-3">"Attributions"</h3>
                                                        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
                                                            {proj
                                                                .attributions
                                                                .into_iter()
                                                                .map(|attr| view! { <Contributor attr /> })
                                                                .collect_view()}
                                                        </div>
                                                    </div>
                                                }
                                                    .into_any()
                                            })}
                                    </div>
                                }
                                    .into_any()
                            }
                            Ok(None) => {
                                view! {
                                    <div class="text-center py-12">
                                        <h1 class="text-3xl font-bold mb-4">"Project Not Found"</h1>
                                        <p class="text-lg">
                                            "The project with shortcode " {shortcode()}
                                            " could not be found."
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! {
                                    <div class="alert alert-error">
                                        <div>
                                            <h1 class="font-bold">"Error"</h1>
                                            <p>"Failed to load project: " {e.to_string()}</p>
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
