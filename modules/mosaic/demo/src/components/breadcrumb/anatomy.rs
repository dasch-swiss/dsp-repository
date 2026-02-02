use leptos::prelude::*;
use mosaic_tiles::breadcrumb::*;

#[component]
pub fn BreadcrumbAnatomy() -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem href="/home">"Home"</BreadcrumbItem>
            <BreadcrumbItem href="/products">"Products"</BreadcrumbItem>
            <BreadcrumbItem>"Current Page"</BreadcrumbItem>
        </Breadcrumb>
    }
}
