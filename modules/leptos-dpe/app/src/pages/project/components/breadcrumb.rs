use leptos::prelude::*;

#[component]
pub fn Breadcrumb(project_name: String) -> impl IntoView {
    let project_name_truncated = if project_name.len() > 100 {
        format!("{}...", &project_name[..50])
    } else {
        project_name
    };

    let link_class = "text-primary";

    view! {
        <div class="breadcrumbs text-sm">
            <ul>
                <li><a href="/" class=link_class>"Home"</a></li>
                <li><a href="/projects" class=link_class>"Projects"</a></li>
                <li>{project_name_truncated}</li>
            </ul>
        </div>
    }
}
