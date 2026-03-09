use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::project_filters_content::ProjectFiltersContent;
use crate::domain::{list_data_languages, list_type_of_data, ProjectQuery};

const ACCESS_RIGHTS_VALUES: &[&str] = &[
    "Full Open Access",
    "Open Access with Restrictions",
    "Embargoed Access",
    "Metadata only Access",
];

// Regular component for filters and search - uses simple links that reload the page
#[component]
pub fn ProjectFilters() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let ongoing = current_query.ongoing();
    let finished = current_query.finished();
    let search = current_query.search();
    let current_type_of_data = current_query.type_of_data();
    let type_of_data_param = current_query.type_of_data.clone();
    let current_data_language = current_query.data_language();
    let data_language_param = current_query.data_language.clone();
    let current_access_rights = current_query.access_rights();
    let access_rights_param = current_query.access_rights.clone();

    // Helper function to build URL with one status parameter toggled
    let build_url = |toggle_param: &str| {
        let new_query = ProjectQuery {
            ongoing: Some(if toggle_param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if toggle_param == "finished" { !finished } else { finished }),
            search: if search.is_empty() { None } else { Some(search.clone()) },
            page: Some(1),
            type_of_data: type_of_data_param.clone(),
            data_language: data_language_param.clone(),
            access_rights: access_rights_param.clone(),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Compute status filter items eagerly so no borrows are captured in the view
    let status_items: Vec<(String, bool, String)> =
        [("ongoing", "Ongoing", ongoing), ("finished", "Finished", finished)]
            .iter()
            .map(|(param, label, checked)| (label.to_string(), *checked, build_url(param)))
            .collect();

    // Load all available type_of_data and data_language values from the server
    let available_types = Resource::new(|| (), |_| async { list_type_of_data().await });
    let available_languages = Resource::new(|| (), |_| async { list_data_languages().await });

    // Build toggle URL helpers (capture clones to avoid move conflicts)
    let current_type_of_data_for_url = current_type_of_data.clone();
    let data_language_param2 = current_query.data_language.clone();
    let access_rights_param2 = current_query.access_rights.clone();
    let search2 = current_query.search();
    let build_type_url = move |value: &str| {
        let mut selected = current_type_of_data_for_url.clone();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        let type_param = if selected.is_empty() { None } else { Some(selected.join(",")) };
        let new_query = ProjectQuery {
            ongoing: Some(ongoing),
            finished: Some(finished),
            search: if search2.is_empty() { None } else { Some(search2.clone()) },
            page: Some(1),
            type_of_data: type_param,
            data_language: data_language_param2.clone(),
            access_rights: access_rights_param2.clone(),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let current_data_language_for_url = current_data_language.clone();
    let type_of_data_param3 = current_query.type_of_data.clone();
    let access_rights_param3 = current_query.access_rights.clone();
    let search3 = current_query.search();
    let build_language_url = move |value: &str| {
        let mut selected = current_data_language_for_url.clone();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        let lang_param = if selected.is_empty() { None } else { Some(selected.join(",")) };
        let new_query = ProjectQuery {
            ongoing: Some(ongoing),
            finished: Some(finished),
            search: if search3.is_empty() { None } else { Some(search3.clone()) },
            page: Some(1),
            type_of_data: type_of_data_param3.clone(),
            data_language: lang_param,
            access_rights: access_rights_param3.clone(),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let current_access_rights_for_url = current_access_rights.clone();
    let type_of_data_param4 = current_query.type_of_data.clone();
    let data_language_param4 = current_query.data_language.clone();
    let search4 = current_query.search();
    let build_access_rights_url = move |value: &str| {
        let mut selected = current_access_rights_for_url.clone();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        let ar_param = if selected.is_empty() { None } else { Some(selected.join(",")) };
        let new_query = ProjectQuery {
            ongoing: Some(ongoing),
            finished: Some(finished),
            search: if search4.is_empty() { None } else { Some(search4.clone()) },
            page: Some(1),
            type_of_data: type_of_data_param4.clone(),
            data_language: data_language_param4.clone(),
            access_rights: ar_param,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Access rights items are static (enum values)
    let access_rights_items: Vec<(String, bool, String)> = ACCESS_RIGHTS_VALUES
        .iter()
        .map(|&v| {
            let checked = current_access_rights.contains(&v.to_string());
            let url = build_access_rights_url(v);
            (v.to_string(), checked, url)
        })
        .collect();

    let clear_href = {
        let new_query = ProjectQuery {
            ongoing: None,
            finished: None,
            search: if search.is_empty() { None } else { Some(search.clone()) },
            page: Some(1),
            type_of_data: None,
            data_language: None,
            access_rights: None,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let status_items_clone = status_items.clone();
    let access_rights_items_clone = access_rights_items.clone();
    let clear_href_clone = clear_href.clone();

    view! {
        <Card variant=CardVariant::Bordered class="w-full">
            <CardBody>
                <Suspense fallback=move || {
                    view! {
                        <ProjectFiltersContent
                            status_items=status_items_clone.clone()
                            type_of_data_items=vec![]
                            data_language_items=vec![]
                            access_rights_items=access_rights_items_clone.clone()
                            clear_href=clear_href_clone.clone()
                        />
                    }
                }>
                    {move || {
                        let type_items: Vec<(String, bool, String)> = available_types
                            .get()
                            .and_then(|r| r.ok())
                            .unwrap_or_default()
                            .iter()
                            .map(|t| {
                                let checked = current_type_of_data.contains(t);
                                let url = build_type_url(t);
                                (t.clone(), checked, url)
                            })
                            .collect();
                        let language_items: Vec<(String, bool, String)> = available_languages
                            .get()
                            .and_then(|r| r.ok())
                            .unwrap_or_default()
                            .iter()
                            .map(|l| {
                                let checked = current_data_language.contains(l);
                                let url = build_language_url(l);
                                (l.clone(), checked, url)
                            })
                            .collect();
                        view! {
                            <ProjectFiltersContent
                                status_items=status_items.clone()
                                type_of_data_items=type_items
                                data_language_items=language_items
                                access_rights_items=access_rights_items.clone()
                                clear_href=clear_href.clone()
                            />
                        }
                    }}
                </Suspense>
            </CardBody>
        </Card>
    }
}
