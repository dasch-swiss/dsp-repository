use leptos::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::pages::project::components::project_details::ProjectDetails;

/// Renders project details for a given shortcode.
///
/// Looked up synchronously from the in-process project + contributor caches
/// (`dpe_web::domain::projects::get_project` and
/// `dpe_web::domain::contributors::get_contributors`) — no `Resource` /
/// `<Suspense>`. The previous async pattern wrapped the project + contributor
/// load in a `Resource::new(move || shortcode.clone(), ..)`; under streaming
/// SSR the resource was visited by `<Suspense>::dry_resolve` while the
/// owning scope was already being torn down, hitting the recurring
/// `tokio-rt-worker` panic at
/// `reactive_graph-0.2.11/src/traits.rs:394:39` ("Tried to access a reactive
/// value that has already been disposed."). The data layer is fully
/// in-memory and synchronous, so the `Resource` indirection added no value
/// and all the lifecycle risk.
///
/// Like the sibling components in `pages/project/components/`, this is
/// gated on non-wasm because `dpe_core::project_cache` and
/// `dpe_core::contributors` are non-wasm. DPE renders SSR-only, but
/// cargo-leptos still compiles `dpe-web` for `wasm32-unknown-unknown`
/// (lib-package), so an inert wasm stub is needed even though it never
/// renders.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn ProjectLoader(
    /// The project shortcode to load
    shortcode: String,
) -> impl IntoView {
    use crate::domain::{get_contributors, get_project};

    match get_project(&shortcode) {
        Some(project) => {
            let contributors = get_contributors(project.attributions.clone());
            view! { <ProjectDetails proj=project contributors=contributors /> }.into_any()
        }
        None => view! {
            <div class="text-center py-12">
                <h1 class="font-display text-3xl font-bold mb-4">"Project Not Found"</h1>
                <p class="text-lg">
                    "The project with shortcode " {shortcode} " could not be found."
                </p>
            </div>
        }
        .into_any(),
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn ProjectLoader(shortcode: String) -> impl IntoView {
    let _ = shortcode;
}
