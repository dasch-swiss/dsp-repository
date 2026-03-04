use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::statusbadge::ProjectCardIndicators;
use crate::domain::{AccessRightsType, ProjectStatus};

#[component]
pub fn ProjectCard(
    title: String,
    content: String,
    status: ProjectStatus,
    access_rights: AccessRightsType,
    btn_target: String,
    #[prop(optional)] keywords: Vec<String>,
) -> impl IntoView {
    view! {
        <a href=btn_target class="block h-full">
            <Card variant=CardVariant::AutoHover class="flex flex-col h-full">
                <div class="relative">
                    <figure class="bg-neutral-900 overflow-hidden">
                        <img src="https://dasch.swiss/projects/0854.webp" alt="Shoes" />
                    </figure>
                    <ProjectCardIndicators status=status access_rights=access_rights />
                </div>

                <CardBody>
                    <h2 class="font-display font-semibold text-lg text-ellipsis">{title}</h2>
                    <p class="text-sm text-gray-600 line-clamp-4 mt-2">{content}</p>
                    <div class="flex flex-wrap gap-1 mt-3">
                        {keywords
                            .into_iter()
                            .take(3)
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
