use leptos::prelude::*;
use mosaic_tiles::breadcrumb::*;
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem href="/">
                <Icon icon=Grid class="w-4 h-4 inline mr-1" />
                "Home"
            </BreadcrumbItem>
            <BreadcrumbItem href="/settings">
                <Icon icon=Tune class="w-4 h-4 inline mr-1" />
                "Settings"
            </BreadcrumbItem>
            <BreadcrumbItem>
                <Icon icon=People class="w-4 h-4 inline mr-1" />
                "Profile"
            </BreadcrumbItem>
        </Breadcrumb>
    }
}
