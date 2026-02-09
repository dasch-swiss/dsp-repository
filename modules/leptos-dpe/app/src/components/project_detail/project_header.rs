use leptos::prelude::*;

use crate::components::ProjectStatusBadge;
use crate::domain::ProjectStatus;

#[component]
pub fn ProjectHeader(
    shortcode: String,
    name: String,
    status: ProjectStatus,
    short_description: String,
) -> impl IntoView {
    view! {
        <div class="bg-base-200 p-6 rounded-lg">
            <div class="flex justify-between items-start">
                <div>
                    <div class="text-sm text-base-content/70 mb-2">
                        "Project " {shortcode}
                    </div>
                    <h1 class="text-3xl font-bold mb-3">{name}</h1>
                </div>
                <ProjectStatusBadge status />
            </div>
            <p class="text-lg mt-4">{short_description}</p>
        </div>
    }
}
