use leptos::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use mosaic_tiles::button::ButtonVariant;
#[cfg(not(target_arch = "wasm32"))]
use mosaic_tiles::card::{Card, CardBody, CardVariant};
#[cfg(not(target_arch = "wasm32"))]
use mosaic_tiles::link::Link;

#[cfg(not(target_arch = "wasm32"))]
use super::card::ProjectCard;
#[cfg(not(target_arch = "wasm32"))]
use super::project_pagination::ProjectPagination;
use crate::domain::ProjectQuery;

/// Renders the filtered + paginated project list.
///
/// Looked up synchronously from the in-process project cache via
/// `dpe_web::domain::projects::filter_and_paginate` — no `Resource` /
/// `<Suspense>`. The previous async pattern wrapped the resolved query in a
/// `Resource::new(move || query.get(), ...)` whose source closure read the
/// `Memo<Result<ProjectQuery, _>>` owned by the parent `ProjectsPage`. Under
/// streaming SSR the resource's async derived re-evaluator could fire after
/// the parent owner had already been disposed, hitting a recurring
/// `tokio-rt-worker` panic at
/// `reactive_graph-0.2.11/src/traits.rs:394:39` ("Tried to access a reactive
/// value that has already been disposed.") — confirmed in production
/// backtraces post-PR #212. Resolving the query in `ProjectsPage` and
/// passing the plain value down removes the cross-owner Memo capture.
///
/// `dpe_core::all_projects` and `crate::domain::projects::filter_and_paginate`
/// are gated on non-wasm targets (disk-backed cache); DPE renders SSR-only,
/// but cargo-leptos still compiles `dpe-web` for `wasm32-unknown-unknown`
/// (lib-package), so an inert wasm stub is needed even though it never
/// renders.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn ProjectList(query: ProjectQuery) -> impl IntoView {
    use crate::domain::lang_value;
    use crate::domain::projects::filter_and_paginate;

    let page = filter_and_paginate(dpe_core::all_projects(), &query, None);
    let nr_pages = page.nr_pages;
    let total_items = page.total_items;

    if total_items == 0 {
        view! {
            <Card variant=CardVariant::Bordered>
                <CardBody>
                    <div class="text-center">
                        <h3 class="mb-4">"No projects found matching your criteria"</h3>
                        <Link href="/dpe/projects" as_button=ButtonVariant::Ghost>
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
                <div class="mb-4 text-sm text-gray-600">{format!("{} projects", total_items)}</div>
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
                                    btn_target=format!("/dpe/projects/{}", project.shortcode)
                                    shortcode=project.shortcode.clone()
                                    keywords=keywords
                                />
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <div class="flex justify-center">
                <ProjectPagination nr_pages=nr_pages query=query />
            </div>
        }
        .into_any()
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn ProjectList(query: ProjectQuery) -> impl IntoView {
    let _ = query;
}
