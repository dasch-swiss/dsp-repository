use leptos::prelude::*;

// Regular component for filters and search - uses simple links that reload the page
#[component]
pub fn ProjectFilters(ongoing: bool, search: String) -> impl IntoView {
    // Build URL with updated ongoing parameter
    let toggle_ongoing_url = if search.is_empty() {
        format!("/projects?ongoing={}", !ongoing)
    } else {
        format!("/projects?ongoing={}&search={}", !ongoing, urlencoding::encode(&search))
    };

    view! {
        <div class="flex flex-col gap-4">
            // Status filter checkbox
            <div class="flex gap-4 items-center">
                <span class="font-semibold">"Filter by Status:"</span>
                <a href=toggle_ongoing_url class="flex items-center gap-2 cursor-pointer hover:opacity-80">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-primary pointer-events-none"
                        checked=ongoing
                    />
                    <span>"Ongoing"</span>
                </a>
            </div>

            // Search form
            <form
                method="get"
                action="/projects"
                class="flex gap-4 items-center"
            >
                <input type="hidden" name="ongoing" value=ongoing.to_string() />
                <span class="font-semibold">"Search:"</span>
                <input
                    type="text"
                    name="search"
                    class="input input-bordered input-primary w-full max-w-xs"
                    placeholder="Search projects..."
                    value=search
                />
                <button
                    type="submit"
                    class="btn btn-primary btn-sm"
                >
                    "Search"
                </button>
            </form>

            // Reset link
            <div>
                <a href="/projects" class="btn btn-primary btn-sm">
                    "Reset Search and Filter"
                </a>
            </div>
        </div>
    }
}
