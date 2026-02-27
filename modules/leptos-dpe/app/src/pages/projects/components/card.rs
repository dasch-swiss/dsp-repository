use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

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
    let layout_class = match view {
        ProjectView::List => "flex flex-row h-full",
        ProjectView::Grid => "flex flex-col h-full",
    };

    let figure_style = match view {
        ProjectView::List => "min-width: 300px; width: 300px;",
        ProjectView::Grid => "",
    };

    view! {
        <a href=btn_target class="block h-full">
            <Card variant=CardVariant::AutoHover class=layout_class>
                <figure class="relative bg-neutral-900 overflow-hidden" style=figure_style>
                    <img src="https://dasch.swiss/projects/0854.webp" alt="Shoes" />

                    <div class="absolute bottom-1 right-1">
                        <ProjectStatusBadge status=status size=BadgeSize::Small />
                    </div>
                </figure>

                <CardBody>
                    <h2 class="font-semibold text-lg text-ellipsis">{title}</h2>
                    <p>{content}</p>
                    <div>
                        <span class="badge badge-sm badge-neutral badge-outline">"Badge"</span>
                    </div>
                </CardBody>
            </Card>
        </a>
    }
}
