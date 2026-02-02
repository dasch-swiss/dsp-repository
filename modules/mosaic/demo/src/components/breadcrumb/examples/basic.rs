use leptos::prelude::*;
use mosaic_tiles::breadcrumb::*;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem href="/">"Home"</BreadcrumbItem>
            <BreadcrumbItem href="/documentation">"Documentation"</BreadcrumbItem>
            <BreadcrumbItem>"Breadcrumb"</BreadcrumbItem>
        </Breadcrumb>
    }
}
