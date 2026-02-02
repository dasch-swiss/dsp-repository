use leptos::prelude::*;
use mosaic_tiles::icon::*;
use mosaic_tiles::tabs::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <Tabs>
            <Tab name="icon-tabs" value="search" label="Search" icon=IconSearch checked=true>
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold flex items-center gap-2">
                        <Icon icon=IconSearch class="w-5 h-5" />
                        "Search"
                    </h3>
                    <p>"Find what you're looking for with our powerful search feature."</p>
                    <div class="mt-4">
                        <input
                            type="text"
                            placeholder="Search..."
                            class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md w-full"
                        />
                    </div>
                </div>
            </Tab>
            <Tab name="icon-tabs" value="github" label="GitHub" icon=IconGitHub>
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold flex items-center gap-2">
                        <Icon icon=IconGitHub class="w-5 h-5" />
                        "GitHub"
                    </h3>
                    <p>"View our repositories and contribute to open source projects."</p>
                    <a
                        href="https://github.com"
                        class="inline-block mt-4 px-4 py-2 bg-gray-900 dark:bg-gray-700 text-white rounded-md hover:bg-gray-800 dark:hover:bg-gray-600"
                    >
                        "Visit GitHub"
                    </a>
                </div>
            </Tab>
            <Tab name="icon-tabs" value="linkedin" label="LinkedIn" icon=IconLinkedIn>
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold flex items-center gap-2">
                        <Icon icon=IconLinkedIn class="w-5 h-5" />
                        "LinkedIn"
                    </h3>
                    <p>"Connect with us on LinkedIn for professional networking."</p>
                    <a
                        href="https://linkedin.com"
                        class="inline-block mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                    >
                        "Visit LinkedIn"
                    </a>
                </div>
            </Tab>
        </Tabs>
    }
}
