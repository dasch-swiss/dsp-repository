use leptos::prelude::*;

use crate::components::{Loading, ProjectPagination};
use crate::domain::models::Page;
use crate::domain::{list_projects, ProjectQuery};
use crate::ProjectCard;

#[component]
pub fn ProjectList(
    query: Memo<Result<ProjectQuery, leptos_router::params::ParamsError>>,
) -> impl IntoView {
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

    // TODO: These should come from the actual page data
    let nr_pages = 5;
    let total_items = 10;

    view! {
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
    }
}
