use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::components::mobile_filters_button::MobileFiltersButton;
use super::components::project_filters::ProjectFilters;
use super::components::project_list::ProjectList;
use super::components::project_search_input::ProjectSearchInput;
use crate::domain::{list_data_languages, list_type_of_data, ProjectQuery};

#[component]
pub fn ProjectsPage() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let status_items = current_query.status_filter_items();
    let access_rights_items = current_query.access_rights_filter_items();

    let available_types = Resource::new(|| (), |_| async { list_type_of_data().await });
    let available_languages = Resource::new(|| (), |_| async { list_data_languages().await });

    let cq = current_query.clone();

    let dialog_open = current_query.dialog.unwrap_or(false);
    let open_dialog_href = format!("/dpe/projects{}", current_query.clone().with_dialog(true).to_query_string());
    let close_dialog_href = format!("/dpe/projects{}", current_query.clone().with_dialog(false).to_query_string());

    view! {
        <Suspense>
            {move || {
                let status = status_items.clone();
                let status_mobile = status_items.clone();
                let access = access_rights_items.clone();
                let access_mobile = access_rights_items.clone();
                let open_href = open_dialog_href.clone();
                let close_href = close_dialog_href.clone();
                let type_of_data = cq.type_of_data();
                let data_language = cq.data_language();
                let type_of_data_items = available_types
                    .get()
                    .and_then(|r| r.ok())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| {
                        let checked = type_of_data.contains(&t);
                        let href = format!(
                            "/dpe/projects{}",
                            cq.with_type_of_data_toggled(&t).to_query_string(),
                        );
                        (t, checked, href)
                    })
                    .collect::<Vec<_>>();
                let type_of_data_items_mobile = type_of_data_items.clone();
                let data_language_items = available_languages
                    .get()
                    .and_then(|r| r.ok())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(code, display)| {
                        let checked = data_language.contains(&code);
                        let href = format!(
                            "/dpe/projects{}",
                            cq.with_data_language_toggled(&code).to_query_string(),
                        );
                        (display, checked, href)
                    })
                    .collect::<Vec<_>>();
                let data_language_items_mobile = data_language_items.clone();
                view! {
                    <div class="flex gap-4">
                        <div class="hidden lg:block lg:w-72 2xl:w-80 flex-shrink-0">
                            <ProjectFilters
                                status_items=status
                                type_of_data_items=type_of_data_items
                                data_language_items=data_language_items
                                access_rights_items=access
                            />
                        </div>
                        <div class="flex-1 flex flex-col gap-2">
                            <Card variant=CardVariant::Bordered class="overflow-visible">
                                <CardBody>
                                    <div class="flex gap-4">
                                        <div class="flex-1">
                                            <ProjectSearchInput />
                                        </div>
                                        <div class="lg:hidden">
                                            <MobileFiltersButton
                                                status_items=status_mobile
                                                type_of_data_items=type_of_data_items_mobile
                                                data_language_items=data_language_items_mobile
                                                access_rights_items=access_mobile
                                                dialog_open=dialog_open
                                                open_dialog_href=open_href
                                                close_dialog_href=close_href
                                            />
                                        </div>
                                    </div>
                                </CardBody>
                            </Card>
                            <ProjectList query=query />
                        </div>
                    </div>
                }
            }}
        </Suspense>
    }
}
