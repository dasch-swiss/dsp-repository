use leptos::prelude::*;
use mosaic_tiles::icon::*;

#[component]
pub fn UsageExample() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h4 class="text-base font-semibold mb-3">"Buttons"</h4>
                <button class="inline-flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700">
                    <Icon icon=IconSearch class="w-4 h-4" />
                    "Search"
                </button>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-3">"Links"</h4>
                <a href="#" class="inline-flex items-center gap-2 text-primary-600 hover:underline">
                    "Visit Documentation"
                    <Icon icon=LinkExternal class="w-4 h-4" />
                </a>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-3">"Alerts"</h4>
                <div class="flex items-center gap-2 p-3 bg-info-50 border border-info-200 rounded-lg">
                    <Icon icon=Info class="w-5 h-5 text-info-600" />
                    <span class="text-sm text-info-800">"This is an informational message"</span>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-3">"Navigation"</h4>
                <div class="flex items-center gap-2">
                    <button class="p-2 border rounded hover:bg-neutral-50" aria-label="Previous">
                        <Icon icon=IconChevronLeft class="w-4 h-4 text-neutral-600" />
                    </button>
                    <span class="text-sm text-neutral-700">"Page 1 of 10"</span>
                    <button class="p-2 border rounded hover:bg-neutral-50" aria-label="Next">
                        <Icon icon=IconChevronRight class="w-4 h-4 text-neutral-600" />
                    </button>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-3">"Social Media"</h4>
                <div class="flex gap-4">
                    <a href="#" class="p-2 text-neutral-600 hover:text-neutral-900" aria-label="GitHub">
                        <Icon icon=IconGitHub class="w-6 h-6" />
                    </a>
                    <a href="#" class="p-2 text-neutral-600 hover:text-primary-600" aria-label="LinkedIn">
                        <Icon icon=IconLinkedIn class="w-6 h-6" />
                    </a>
                    <a href="#" class="p-2 text-neutral-600 hover:text-neutral-900" aria-label="X">
                        <Icon icon=IconX class="w-6 h-6" />
                    </a>
                </div>
            </div>
        </div>
    }
}
