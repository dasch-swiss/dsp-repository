use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::statusbadge::ProjectCardIndicators;
use crate::domain::{AccessRightsType, ProjectStatus, ProjectView};

#[component]
pub fn ProjectCard(
    title: String,
    content: String,
    status: ProjectStatus,
    access_rights: AccessRightsType,
    btn_target: String,
    view: ProjectView,
    #[prop(optional)] keywords: Vec<String>,
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
                <div class="relative">
                    <figure class="bg-neutral-900 overflow-hidden" style=figure_style>
                        <img src="https://dasch.swiss/projects/0854.webp" alt="Alice from Alice in Wonderland walks through a futuristic arched hall covered in glowing binary code toward a doorway labeled \"DasCHland,\" with plants and computer monitors along the sides." />
                    </figure>
                    <ProjectCardIndicators status=status access_rights=access_rights />
                </div>

                <CardBody>
                    <h2 class="font-display font-semibold text-lg text-ellipsis">{title}</h2>
                    <p class="text-sm text-gray-600 line-clamp-4 mt-2">{content}</p>
                    <div class="flex flex-wrap gap-1 mt-3">
                        {keywords
                            .into_iter()
                            .map(|kw| {
                                view! {
                                    <Badge variant=BadgeVariant::Secondary size=BadgeSize::Small>
                                        {kw}
                                    </Badge>
                                }
                            })
                            .collect_view()}
                    </div>
                </CardBody>
            </Card>
        </a>
    }
}
