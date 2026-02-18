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
          <figure>
            <img
                src="https://dasch.swiss/projects/0854.webp"
                alt="Shoes" />
            </figure>

            <div class="card-body">
                <div class="flex justify-end">
                    <ProjectStatusBadge status=status size=BadgeSize::Small />
                </div>
                <h2 class="card-title">{title}</h2>
                <p>{content}</p>
            </div>
        </a>
    }
}
