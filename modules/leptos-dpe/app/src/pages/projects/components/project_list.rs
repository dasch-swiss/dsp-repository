use leptos::prelude::*;

use super::card::ProjectCard;
use super::project_pagination::ProjectPagination;
use crate::components::loading::Loading;
use crate::domain::{list_projects, ProjectQuery, ProjectView};

#[component]
pub fn ProjectList(query: Memo<Result<ProjectQuery, leptos_router::params::ParamsError>>) -> impl IntoView {
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
                query_data.view,
                None,
            )
            .await
        },
    );

    view! {
        // Everything inside Suspense to avoid reading resource outside
        <Suspense fallback=move || {
            view! { <Loading /> }
        }>
            {move || {
                let current_query = query.get().unwrap_or_default();
                let view = current_query.view();
                let grid_class = match view {
                    ProjectView::List => "grid grid-cols-1 gap-4",
                    ProjectView::Grid => "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                };
                projects
                    .get()
                    .map(|result| match result {
                        Ok(page) => {
                            let nr_pages = page.nr_pages;
                            let total_items = page.total_items;
                            if total_items == 0 {

                                view! {
                                    <div class="card bg-base-100 border border-gray-200 p-8 text-center">
                                        <h3 class="mb-4">
                                            "No projects found matching your criteria"
                                        </h3>
                                        <div class="text-center">
                                            <a href="/projects" class="btn btn-ghost">
                                                "Clear your filters"
                                            </a>
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div>
                                        <div class="mb-2">
                                            {format!("{} projects", total_items)}
                                        </div>
                                        <div class=grid_class>
                                            {
                                                let view_value = view;
                                                page.items
                                                    .into_iter()
                                                    .map(move |project| {
                                                        view! {
                                                            <ProjectCard
                                                                title=project.name.clone()
                                                                content=project.short_description.clone()
                                                                status=project.status.clone()
                                                                btn_target=format!("/projects/{}", project.shortcode)
                                                                view=view_value
                                                            />
                                                        }
                                                    })
                                                    .collect_view()
                                            }
                                        </div>
                                    </div>

                                    <div class="flex justify-center">
                                        <ProjectPagination nr_pages=nr_pages query=current_query />
                                    </div>
                                }
                                    .into_any()
                            }
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
    }
}
