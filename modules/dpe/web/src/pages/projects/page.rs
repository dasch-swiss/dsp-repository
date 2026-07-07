use leptos::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use leptos_router::hooks::use_query;
#[cfg(not(target_arch = "wasm32"))]
use mosaic_tiles::card::{Card, CardBody, CardVariant};

#[cfg(not(target_arch = "wasm32"))]
use super::components::mobile_filters_button::MobileFiltersButton;
#[cfg(not(target_arch = "wasm32"))]
use super::components::project_filters::ProjectFilters;
#[cfg(not(target_arch = "wasm32"))]
use super::components::project_list::ProjectList;
#[cfg(not(target_arch = "wasm32"))]
use super::components::project_search_input::ProjectSearchInput;
#[cfg(not(target_arch = "wasm32"))]
use crate::domain::ProjectQuery;

/// Projects index page.
///
/// Resolved synchronously from the in-memory project cache — no
/// `Resource` / `<Suspense>`. The previous async pattern wrapped the
/// type-of-data and data-language facet sources in
/// `Resource::new(|| (), |_| async { list_..().await })`; under streaming
/// SSR even those `()`-sourced resources were visited by
/// `<Suspense>::dry_resolve`, hitting the recurring
/// `tokio-rt-worker` panic at
/// `reactive_graph-0.2.11/src/traits.rs:394:39` ("Tried to access a
/// reactive value that has already been disposed."). The data layer is
/// fully in-memory and synchronous, so the `Resource` indirection added
/// no value and all the lifecycle risk.
///
/// `dpe_web::domain::list_type_of_data`, `list_data_languages`, and
/// `ProjectQuery` accessors that read filter state are gated on non-wasm
/// (matching the underlying caches); DPE renders SSR-only, but
/// cargo-leptos still compiles `dpe-web` for `wasm32-unknown-unknown`
/// (lib-package), so an inert wasm stub is needed even though it never
/// renders.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn ProjectsPage() -> impl IntoView {
    use crate::domain::{list_data_languages, list_type_of_data};

    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let status_items = current_query.status_filter_items();
    let access_rights_items = current_query.access_rights_filter_items();

    let dialog_open = current_query.dialog.unwrap_or(false);
    let open_dialog_href = format!("/dpe/projects{}", current_query.clone().with_dialog(true).to_query_string());
    let close_dialog_href = format!("/dpe/projects{}", current_query.clone().with_dialog(false).to_query_string());

    let type_of_data_selected = current_query.type_of_data();
    let type_of_data_items: Vec<(String, bool, String)> = list_type_of_data()
        .into_iter()
        .map(|t| {
            let checked = type_of_data_selected.contains(&t);
            let href = format!("/dpe/projects{}", current_query.with_type_of_data_toggled(&t).to_query_string(),);
            (t, checked, href)
        })
        .collect();

    let data_language_selected = current_query.data_language();
    let data_language_items: Vec<(String, bool, String)> = list_data_languages()
        .into_iter()
        .map(|(code, display)| {
            let checked = data_language_selected.contains(&code);
            let href = format!(
                "/dpe/projects{}",
                current_query.with_data_language_toggled(&code).to_query_string(),
            );
            (display, checked, href)
        })
        .collect();

    view! {
        <div class="flex gap-4">
            <div class="hidden lg:block lg:w-72 2xl:w-80 flex-shrink-0">
                <ProjectFilters
                    status_items=status_items.clone()
                    type_of_data_items=type_of_data_items.clone()
                    data_language_items=data_language_items.clone()
                    access_rights_items=access_rights_items.clone()
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
                                    status_items=status_items
                                    type_of_data_items=type_of_data_items
                                    data_language_items=data_language_items
                                    access_rights_items=access_rights_items
                                    dialog_open=dialog_open
                                    open_dialog_href=open_dialog_href
                                    close_dialog_href=close_dialog_href
                                />
                            </div>
                        </div>
                    </CardBody>
                </Card>
                <ProjectList query=current_query />
            </div>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn ProjectsPage() -> impl IntoView {}
