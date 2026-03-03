use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeVariant};
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-3 items-center">
            <Badge variant=BadgeVariant::Info>
                <Icon icon=Info class="w-3 h-3 inline mr-1" />
                "New"
            </Badge>
            <Badge variant=BadgeVariant::Success>
                <Icon icon=IconChevronUp class="w-3 h-3 inline mr-1" />
                "Trending"
            </Badge>
            <Badge variant=BadgeVariant::Warning>
                <Icon icon=Mail class="w-3 h-3 inline mr-1" />
                "5 Messages"
            </Badge>
        </div>
    }
}
