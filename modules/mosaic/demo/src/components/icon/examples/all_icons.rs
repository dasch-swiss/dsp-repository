use leptos::prelude::*;
use mosaic_tiles::icon::*;

#[component]
pub fn AllIconsExample() -> impl IntoView {
    view! {
        <div class="grid grid-cols-4 gap-6">
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconChevronUp class="w-6 h-6" />
                <span class="text-sm text-gray-600">"ChevronUp"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconChevronDown class="w-6 h-6" />
                <span class="text-sm text-gray-600">"ChevronDown"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconChevronLeft class="w-6 h-6" />
                <span class="text-sm text-gray-600">"ChevronLeft"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconChevronRight class="w-6 h-6" />
                <span class="text-sm text-gray-600">"ChevronRight"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconSearch class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Search"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=LinkExternal class="w-6 h-6" />
                <span class="text-sm text-gray-600">"LinkExternal"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Info class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Info"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Mail class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Mail"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=CopyPaste class="w-6 h-6" />
                <span class="text-sm text-gray-600">"CopyPaste"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconLinkedIn class="w-6 h-6" />
                <span class="text-sm text-gray-600">"LinkedIn"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconX class="w-6 h-6" />
                <span class="text-sm text-gray-600">"X (Twitter)"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconGitHub class="w-6 h-6" />
                <span class="text-sm text-gray-600">"GitHub"</span>
            </div>
        </div>
    }
}
