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
                <span class="text-sm text-gray-600">"IconLinkedIn"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconX class="w-6 h-6" />
                <span class="text-sm text-gray-600">"IconX"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=IconGitHub class="w-6 h-6" />
                <span class="text-sm text-gray-600">"IconGitHub"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=People class="w-6 h-6" />
                <span class="text-sm text-gray-600">"People"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Hamburger class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Hamburger"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Sidebar class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Sidebar"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Document class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Document"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Data class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Data"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Question class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Question"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=LockClosed class="w-6 h-6" />
                <span class="text-sm text-gray-600">"LockClosed"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=LockOpen class="w-6 h-6" />
                <span class="text-sm text-gray-600">"LockOpen"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Clock class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Clock"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Flag class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Flag"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=OpenDocument class="w-6 h-6" />
                <span class="text-sm text-gray-600">"OpenDocument"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Tune class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Tune"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=Grid class="w-6 h-6" />
                <span class="text-sm text-gray-600">"Grid"</span>
            </div>
            <div class="flex flex-col items-center gap-2 p-4 border rounded-lg">
                <Icon icon=DownloadFile class="w-6 h-6" />
                <span class="text-sm text-gray-600">"DownloadFile"</span>
            </div>
        </div>
    }
}
