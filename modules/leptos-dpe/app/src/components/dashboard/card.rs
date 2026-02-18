use leptos::prelude::*;

use crate::components::dashboard::statusbadge::BadgeSize;
use crate::components::ProjectStatusBadge;
use crate::domain::ProjectStatus;

#[component]
pub fn ProjectCard(
    title: String,
    content: String,
    status: ProjectStatus,
    btn_text: String,
    btn_target: String,
) -> impl IntoView {
    view! {
        <a href=btn_target class="card bg-base-100 shadow-sm">
          <figure class="relative">
            <img
                src="https://dasch.swiss/projects/0854.webp"
                alt="Shoes" />

            <div class="absolute bottom-1 right-1">
                        <ProjectStatusBadge status=status size=BadgeSize::Small />
            </div>
          </figure>

            <div class="card-body">
                <h2 class="card-title">{title}</h2>
                <p class="truncate">{content}</p>
        <div>
            <span class="badge badge-sm badge-neutral badge-outline">Badge</span>
        </div>

            </div>
        </a>
    }
}
