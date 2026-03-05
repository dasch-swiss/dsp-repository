use leptos::prelude::*;
use mosaic_tiles::breadcrumb::*;

#[component]
pub fn NestedExample() -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem href="/">"Home"</BreadcrumbItem>
            <BreadcrumbItem href="/products">"Products"</BreadcrumbItem>
            <BreadcrumbItem href="/products/electronics">"Electronics"</BreadcrumbItem>
            <BreadcrumbItem href="/products/electronics/laptops">"Laptops"</BreadcrumbItem>
            <BreadcrumbItem>"Product Details"</BreadcrumbItem>
        </Breadcrumb>
    }
}
