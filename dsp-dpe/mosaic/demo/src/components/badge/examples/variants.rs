use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeVariant};

#[component]
pub fn VariantsExample() -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-3 items-center">
            <Badge variant=BadgeVariant::Primary>"Primary"</Badge>
            <Badge variant=BadgeVariant::Secondary>"Secondary"</Badge>
            <Badge variant=BadgeVariant::Success>"Success"</Badge>
            <Badge variant=BadgeVariant::Warning>"Warning"</Badge>
            <Badge variant=BadgeVariant::Danger>"Danger"</Badge>
            <Badge variant=BadgeVariant::Info>"Info"</Badge>
        </div>
    }
}
