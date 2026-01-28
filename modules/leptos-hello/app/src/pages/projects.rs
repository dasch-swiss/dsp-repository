use crate::components::{Loading, ProjectSearch};
use crate::domain::{list_projects, ProjectQuery};
use crate::ProjectCard;
use leptos::prelude::*;
use leptos_router::hooks::use_query;

#[component]
pub fn ProjectsPage() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();

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

    let current_query = query.get().unwrap_or_default();

    view! {
        <div class="flex flex-col gap-4 py-4">
            // Everything inside Suspense to avoid reading resource outside
            <Suspense fallback=move || {
                view! { <Loading /> }
            }>
                {move || {
                    projects
                        .get()
                        .map(|result| match result {
                            Ok(page) => {
                                view! {
                                    // ProjectSearch with pagination info
                                    <ProjectSearch
                                        nr_pages=page.nr_pages
                                        total_items=page.total_items
                                        query=current_query.clone()
                                    />

                                    // Project cards grid
                                    <div class="flex flex-wrap gap-4">
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
                                                        shortcode=project.shortcode.clone()
                                                    />
                                                }
                                            })
                                            .collect_view()}
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
    }
}
