use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use super::project_loader::ProjectLoader;

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params_map();
    let shortcode = move || params.read().get("id").unwrap_or_default();

    view! {
        <Title text=move || format!("Project {}", shortcode()) />
        <div class="min-h-100">
            <ProjectLoader shortcode=shortcode() />
        </div>
    }
}
