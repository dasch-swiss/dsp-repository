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
    let type_items = Memo::new(move |_| -> Vec<(String, bool, String)> {
        available_types
            .get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let checked = cq.type_of_data().contains(&t);
                let href = format!("/projects{}", cq.with_type_of_data_toggled(&t).to_query_string());
                (t, checked, href)
            })
            .collect()
    });

    let cq2 = current_query.clone();
    let language_items = Memo::new(move |_| -> Vec<(String, bool, String)> {
        available_languages
            .get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .into_iter()
            .map(|l| {
                let checked = cq2.data_language().contains(&l);
                let href =
                    format!("/projects{}", cq2.with_data_language_toggled(&l).to_query_string());
                (l, checked, href)
            })
            .collect()
    });

    let status_items_mobile = status_items.clone();
    let access_rights_items_mobile = access_rights_items.clone();

    let dialog_open = current_query.dialog.unwrap_or(false);
    let open_dialog_href =
        format!("/projects{}", current_query.clone().with_dialog(true).to_query_string());
    let close_dialog_href =
        format!("/projects{}", current_query.clone().with_dialog(false).to_query_string());

    view! {
        <div class="flex gap-4">
            <div class="hidden lg:block lg:w-72 2xl:w-80 flex-shrink-0">
                <ProjectFilters
                    status_items=status_items
                    type_of_data_items=type_items.get()
                    data_language_items=language_items.get()
                    access_rights_items=access_rights_items
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
                                    status_items=status_items_mobile
                                    type_of_data_items=type_items.get()
                                    data_language_items=language_items.get()
                                    access_rights_items=access_rights_items_mobile
                                    dialog_open=dialog_open
                                    open_dialog_href=open_dialog_href
                                    close_dialog_href=close_dialog_href
                                />
                            </div>
                        </div>
                    </CardBody>
                </Card>

                <ProjectList query=query />
            </div>
        </div>
    }
}
