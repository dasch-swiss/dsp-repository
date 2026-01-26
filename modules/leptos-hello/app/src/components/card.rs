use leptos::prelude::*;

use crate::components::{statusbadge::BadgeSize, ProjectStatusBadge};
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
        <div class="card bg-base-300 shadow w-96">
            <div class="card-body">
                <div class="flex justify-end">
                    <ProjectStatusBadge status=status size=BadgeSize::Small />
                </div>
                <h2 class="card-title">{title}</h2>
                <p>{content}</p>
                <div class="card-actions justify-end">
                    <button class="btn btn-primary">
                        <a href=btn_target>{btn_text}</a>
                    </button>
                </div>
            </div>
        </div>
    }
}
