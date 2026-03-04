use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::components::mobile_filters_button::MobileFiltersButton;
use super::components::project_filters::ProjectFilters;
use super::components::project_list::ProjectList;
use super::components::project_search_input::ProjectSearchInput;
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectsPage() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();

    view! {
        <div class="flex gap-4">
            <div class="hidden lg:block lg:w-72 2xl:w-80 flex-shrink-0">
                <ProjectFilters />
            </div>

            <div class="flex-1 flex flex-col gap-2">
                <Card variant=CardVariant::Bordered class="overflow-visible">
                    <CardBody>
                        <div class="flex gap-4">
                            <div class="flex-1">
                                <ProjectSearchInput />
                            </div>
                            <div class="lg:hidden">
                                <MobileFiltersButton />
                            </div>
                        </div>
                    </CardBody>
                </Card>

                <ProjectList query=query />
            </div>
        </div>
    }
}
