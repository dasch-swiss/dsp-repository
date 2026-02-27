use leptos::prelude::*;

use super::statusbadge::{BadgeSize, ProjectStatusBadge};
use crate::domain::{ProjectStatus, ProjectView};

#[component]
pub fn ProjectCard(
    title: String,
    content: String,
    status: ProjectStatus,
    btn_target: String,
    view: ProjectView,
) -> impl IntoView {
    let card_class = match view {
        ProjectView::List => "card bg-base-100 border border-gray-200 hover:shadow-sm flex-row",
        ProjectView::Grid => "card bg-base-100 border border-gray-200 hover:shadow-sm",
    };

    let figure_style = match view {
        ProjectView::List => "min-width: 300px; width: 300px; border-top-right-radius: 0; border-bottom-left-radius: inherit; min-width: 300px",
        ProjectView::Grid => "",
    };

    view! {
        <a href=btn_target class=card_class>
            <figure class="relative bg-neutral-900" style=figure_style>
                <img src="https://dasch.swiss/projects/0854.webp" alt="Shoes" />

                <div class="absolute bottom-1 right-1">
                    <ProjectStatusBadge status=status size=BadgeSize::Small />
                </div>
            </figure>

            <div class="card-body">
                <h2 class="card-title text-ellipsis">{title}</h2>
                <p>{content}</p>
                <div>
                    <span class="badge badge-sm badge-neutral badge-outline">Badge</span>
                </div>

            </div>
        </a>
    }
}
