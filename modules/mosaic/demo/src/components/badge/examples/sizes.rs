use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};

#[component]
pub fn SizesExample() -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-3 items-center">
            <Badge size=BadgeSize::Small variant=BadgeVariant::Primary>
                "Small"
            </Badge>
            <Badge size=BadgeSize::Medium variant=BadgeVariant::Primary>
                "Medium"
            </Badge>
            <Badge size=BadgeSize::Large variant=BadgeVariant::Primary>
                "Large"
            </Badge>
        </div>
    }
}
