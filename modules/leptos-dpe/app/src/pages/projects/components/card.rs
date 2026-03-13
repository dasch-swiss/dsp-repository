use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::icon::{Icon, OpenDocument};

use super::statusbadge::ProjectCardIndicators;
use crate::domain::{AccessRightsType, ProjectStatus};

#[component]
pub fn ProjectCard(
    title: String,
    content: String,
    status: ProjectStatus,
    access_rights: AccessRightsType,
    btn_target: String,
    shortcode: String,
    #[prop(optional)] keywords: Vec<String>,
) -> impl IntoView {
    let image_src = format!("/assets/images/{shortcode}.webp");
    view! {
        <a href=btn_target class="block h-full relative hover:z-10">
            <Card variant=CardVariant::AutoHover class="flex flex-col h-full ![overflow:visible]">
                <figure class="bg-neutral-900 relative rounded-t-[inherit]">
                    <div class="overflow-hidden rounded-t-[inherit]">
                        <img
                            src=image_src
                            alt=title.clone()
                            class="w-full h-48 object-cover"
                            onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"
                        />
                        <div class="w-full h-48 bg-gray-100 items-center justify-center hidden">
                            <Icon icon=OpenDocument class="w-12 h-12 text-gray-300" />
                        </div>
                    </div>
                    <ProjectCardIndicators status=status access_rights=access_rights />
                </figure>
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
