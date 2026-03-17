use leptos::prelude::*;
use mosaic_tiles::breadcrumb::{Breadcrumb as TilesBreadcrumb, BreadcrumbItem};

#[component]
pub fn Breadcrumb(project_name: String) -> impl IntoView {
    let project_name_truncated = if project_name.len() > 100 {
        format!("{}...", &project_name[..50])
    } else {
        project_name
    };

    view! {
        <TilesBreadcrumb>
            <BreadcrumbItem href="/projects">"Projects"</BreadcrumbItem>
            <BreadcrumbItem>{project_name_truncated}</BreadcrumbItem>
        </TilesBreadcrumb>
    }
}
