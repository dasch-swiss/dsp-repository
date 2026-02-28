use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::components::project_filters::ProjectFilters;
use super::components::project_list::ProjectList;
use super::components::project_search_input::ProjectSearchInput;
use super::components::project_view_toggle::ProjectViewToggle;
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectsPage() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();

    view! {
        <div class="flex gap-4">
            <ProjectFilters />

            <div class="flex-1 flex flex-col gap-4">
                <Card variant=CardVariant::Bordered>
                    <CardBody>
                        <div class="flex items-center gap-2">
                            <div class="flex-1">
                                <ProjectSearchInput />
                            </div>
                            <ProjectViewToggle />
                        </div>
                    </CardBody>
                </Card>

                <ProjectList query=query />
            </div>
        </div>
    }
}
