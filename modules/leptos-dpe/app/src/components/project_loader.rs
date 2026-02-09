use leptos::prelude::*;

use crate::components::ProjectDetails;
use crate::domain::get_project;

/// ProjectLoader component that handles loading and error states for project data.
///
/// This component creates a Resource to load project data based on a shortcode
/// and handles all the different states (loading, success, not found, error) in one place.
#[component]
pub fn ProjectLoader(
    /// The project shortcode to load
    shortcode: String,
) -> impl IntoView {
    let shortcode_for_error = shortcode.clone();

    // Create resource that loads project data
    let resource = Resource::new(
        move || shortcode.clone(),
        |shortcode| async move { get_project(shortcode).await },
    );

    view! {
        {move || {
            resource
                .get()
                .map(|result| match result {
                    Ok(Some(proj)) => {
                        view! { <ProjectDetails proj=proj /> }.into_any()
                    }
                    Ok(None) => {
                        view! {
                            <div class="text-center py-12">
                                <h1 class="text-3xl font-bold mb-4">"Project Not Found"</h1>
                                <p class="text-lg">
                                    "The project with shortcode " {shortcode_for_error.clone()}
                                    " could not be found."
                                </p>
                            </div>
                        }
                            .into_any()
                    }
                    Err(e) => {
                        view! {
                            <div class="alert alert-error">
                                <div>
                                    <h1 class="font-bold">"Error"</h1>
                                    <p>"Failed to load project: " {e.to_string()}</p>
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                })
        }}
    }
}
