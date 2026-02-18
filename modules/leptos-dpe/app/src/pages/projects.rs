use leptos::prelude::*;
use leptos_router::hooks::use_query;

use crate::components::{Loading, ProjectFilters, ProjectPagination, ProjectSearchInput};
use crate::domain::{list_projects, ProjectQuery};
use crate::ProjectCard;

#[component]
pub fn ProjectsPage() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();
    let nr_pages = 5;
    let total_items = 10;

    // Create resource that depends on query parameters
    let projects = Resource::new(
        move || query.get(),
        |q| async move {
            let query_data = q.unwrap_or_default();
            list_projects(
                query_data.ongoing,
                query_data.finished,
                query_data.search,
                query_data.page,
                None,
            )
            .await
        },
    );

    view! {
        <div class="flex gap-4">
            <ProjectFilters />

            <div class="flex-1 flex flex-col gap-4">
                // Everything inside Suspense to avoid reading resource outside
                <Suspense fallback=move || {
                    view! { <Loading /> }
                }>
                    {move || {
                        let current_query = query.get().unwrap_or_default();
                        projects
                            .get()
                            .map(|result| match result {
                                Ok(page) => {
                                    view! {
                                        <ProjectSearchInput />

                                        <div>
                                            <div class="mb-2">{format!("{} projects", total_items)}</div>
                                            <div class="grid grid-cols-3 gap-4">
                                                {page
                                                    .items
                                                    .into_iter()
                                                    .map(|project| {
                                                        view! {
                                                            <ProjectCard
                                                                title=project.name.clone()
                                                                content=project.short_description.clone()
                                                                status=project.status.clone()
                                                                btn_text="View Project".to_string()
                                                                btn_target=format!("/projects/{}", project.shortcode)
                                                            />
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        </div>

                                        <div class="flex justify-center">
                                            <ProjectPagination nr_pages=nr_pages query=current_query />
                                        </div>

                                    }
                                        .into_any()
                                }
                                Err(e) => {
                                    view! {
                                        <div class="alert alert-error">
                                            <span>"Failed to load projects: "{e.to_string()}</span>
                                        </div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </div>
        </div>
    }
}
