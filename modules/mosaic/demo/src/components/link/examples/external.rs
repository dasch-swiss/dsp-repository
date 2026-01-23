use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn ExternalExample() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <p class="text-sm text-gray-600">
                "External links should use "
                <code class="bg-gray-100 px-1 rounded">"target=\"_blank\""</code>
                " and "
                <code class="bg-gray-100 px-1 rounded">"rel=\"noopener noreferrer\""</code>
                " for security."
            </p>
            <div class="flex gap-4 items-center">
                <Link href="/link" target="_blank" rel="noopener noreferrer">
                    "Documentation"
                </Link>
                <Link href="/link" target="_blank" rel="noopener noreferrer">
                    "GitHub Repository"
                </Link>
                <Link href="/link" target="_blank" rel="noopener noreferrer" as_button=true>
                    "External Button Link"
                </Link>
            </div>
        </div>
    }
}
