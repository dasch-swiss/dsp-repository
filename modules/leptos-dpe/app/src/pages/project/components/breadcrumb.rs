use leptos::prelude::*;

#[component]
pub fn Breadcrumb(project_name: String) -> impl IntoView {
    // Truncate project name to 50 characters with ellipsis
    let project_name_truncated = if project_name.len() > 100 {
        format!("{}...", &project_name[..50])
    } else {
        project_name
    };

    view! {
        <nav class="flex items-center gap-2 text-sm text-gray-600">
            <a href="/" class="hover:text-gray-900 transition-colors">
                "Home"
            </a>
            <span class="text-gray-400">">"</span>
            <a href="/projects" class="hover:text-gray-900 transition-colors">
                "Projects"
            </a>
            <span class="text-gray-400">">"</span>
            <span class="text-gray-900 font-medium truncate">
                {project_name_truncated}
            </span>
        </nav>
    }
}
