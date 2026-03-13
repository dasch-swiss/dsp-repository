use leptos::prelude::*;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::link::Link;

use super::card::ProjectCard;
use super::project_pagination::ProjectPagination;
use crate::components::loading::Loading;
use crate::domain::{lang_value, list_projects, ProjectQuery};

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
                None,
                query_data.type_of_data,
                query_data.data_language,
                query_data.access_rights,
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
                projects
                    .get()
                    .map(|result| match result {
                        Ok(page) => {
                            let nr_pages = page.nr_pages;
                            let total_items = page.total_items;
                            if total_items == 0 {

                                view! {
                                    <Card variant=CardVariant::Bordered>
                                        <CardBody>
                                            <div class="text-center">
                                                <h3 class="mb-4">
                                                    "No projects found matching your criteria"
                                                </h3>
                                                <Link href="/projects" as_button=ButtonVariant::Ghost>
                                                    "Clear your filters"
                                                </Link>
                                            </div>
                                        </CardBody>
                                    </Card>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div>
                                        <div class="mb-4 text-sm text-gray-600">
                                            {format!("{} projects", total_items)}
                                        </div>
                                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                            {page
                                                .items
                                                .into_iter()
                                                .map(|project| {
                                                    let keywords: Vec<String> = project
                                                        .keywords
                                                        .iter()
                                                        .filter_map(|map| lang_value(map).cloned())
                                                        .collect();
                                                    view! {
                                                        <ProjectCard
                                                            title=project.name.clone()
                                                            content=project.short_description.clone()
                                                            status=project.status.clone()
                                                            access_rights=project.access_rights.access_rights.clone()
                                                            btn_target=format!("/projects/{}", project.shortcode)
                                                            shortcode=project.shortcode.clone()
                                                            keywords=keywords
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
