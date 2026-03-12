use leptos::prelude::*;

use crate::domain::{get_contributors, get_project};
use crate::pages::project::components::project_details::ProjectDetails;

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

    // Load project and contributors in a single resource so they resolve together.
    let resource = Resource::new(
        move || shortcode.clone(),
        |shortcode| async move {
            let proj = get_project(shortcode).await?;
            let contributors = match &proj {
                Some(p) => get_contributors(p.attributions.clone()).await.unwrap_or_default(),
                None => vec![],
            };
            Ok::<_, leptos::server_fn::error::ServerFnError>((proj, contributors))
        },
    );

    view! {
        {move || {
            resource
                .get()
                .map(|result| match result {
                    Ok((Some(proj), contributors)) => {
                        view! { <ProjectDetails proj=proj contributors=contributors /> }.into_any()
                    }
                    Ok((None, _)) => {
                        view! {
                            <div class="text-center py-12">
                                <h1 class="font-display text-3xl font-bold mb-4">
                                    "Project Not Found"
                                </h1>
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
                                    <h1 class="font-display font-bold">"Error"</h1>
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
