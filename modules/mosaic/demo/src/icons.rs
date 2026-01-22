use leptos::prelude::*;
use mosaic_tiles::icon::*;

#[component]
pub fn IconExamples() -> impl IntoView {
    view! {
        <div class="space-y-12 p-8">
            <div>
                <h1 class="text-3xl font-bold mb-2">"Icon Component"</h1>
                <p class="text-gray-600 dark:text-gray-400 mb-6">
                    "A collection of 10 carefully selected icons with automatic tree-shaking. Only icons used in your code are included in the WASM bundle."
                </p>
            </div>

            // All Icons Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"All Icons"</h2>
                <div class="grid grid-cols-4 gap-6">
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconChevronUp class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"ChevronUp"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconChevronDown class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"ChevronDown"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconSearch class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"Search"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=LinkExternal class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"LinkExternal"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=Info class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"Info"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=Mail class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"Mail"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=CopyPaste class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"CopyPaste"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconLinkedIn class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"LinkedIn"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconX class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"X (Twitter)"</span>
                    </div>
                    <div class="flex flex-col items-center gap-2 p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <Icon icon=IconGitHub class="w-6 h-6" />
                        <span class="text-sm text-gray-600 dark:text-gray-400">"GitHub"</span>
                    </div>
                </div>
            </section>

            // Size Examples Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Size Examples"</h2>
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                    "Control icon size using Tailwind width and height classes."
                </p>
                <div class="space-y-4">
                    <div class="flex items-center gap-4">
                        <Icon icon=IconSearch class="w-4 h-4" />
                        <code class="text-sm">"w-4 h-4 (16px)"</code>
                    </div>
                    <div class="flex items-center gap-4">
                        <Icon icon=IconSearch class="w-5 h-5" />
                        <code class="text-sm">"w-5 h-5 (20px)"</code>
                    </div>
                    <div class="flex items-center gap-4">
                        <Icon icon=IconSearch class="w-6 h-6" />
                        <code class="text-sm">"w-6 h-6 (24px)"</code>
                    </div>
                    <div class="flex items-center gap-4">
                        <Icon icon=IconSearch class="w-8 h-8" />
                        <code class="text-sm">"w-8 h-8 (32px)"</code>
                    </div>
                    <div class="flex items-center gap-4">
                        <Icon icon=IconSearch class="w-12 h-12" />
                        <code class="text-sm">"w-12 h-12 (48px)"</code>
                    </div>
                </div>
            </section>

            // Colors Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Color Customization"</h2>
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                    "Icons use currentColor and inherit text color from parent or Tailwind classes."
                </p>
                <div class="flex gap-6">
                    <Icon icon=IconGitHub class="w-8 h-8 text-gray-500" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-indigo-600" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-red-500" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-green-600" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-purple-500" />
                </div>
            </section>

            // Interactive Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Interactive States"</h2>
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                    "Icons work with hover, focus, and other state variants."
                </p>
                <div class="flex gap-4">
                    <button class="p-3 rounded-lg border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                        <Icon icon=IconSearch class="w-5 h-5 text-gray-600 dark:text-gray-400" />
                    </button>
                    <button class="p-3 rounded-lg border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                        <Icon icon=IconChevronUp class="w-5 h-5 text-gray-600 dark:text-gray-400" />
                    </button>
                    <button class="p-3 rounded-lg border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                        <Icon
                            icon=IconChevronDown
                            class="w-5 h-5 text-gray-600 dark:text-gray-400"
                        />
                    </button>
                </div>
            </section>

            // In Context Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Icons in Context"</h2>
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                    "Real-world usage examples showing icons with text and in buttons."
                </p>
                <div class="space-y-4">
                    // Button with icon
                    <button class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors">
                        <Icon icon=IconSearch class="w-4 h-4" />
                        "Search"
                    </button>
                </div>
                <div class="space-y-4">
                    // Link with external icon
                    <a
                        href="#"
                        class="inline-flex items-center gap-2 text-indigo-600 dark:text-indigo-400 hover:underline"
                    >
                        "Visit Documentation"
                        <Icon icon=LinkExternal class="w-4 h-4" />
                    </a>

                    // Navigation with chevron
                    <div class="flex items-center gap-2 text-gray-700 dark:text-gray-300">
                        <span>"Expandable Item"</span>
                        <Icon icon=IconChevronDown class="w-4 h-4" />
                    </div>

                    // Info alert
                    <div class="flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                        <Icon icon=Info class="w-5 h-5 text-blue-600 dark:text-blue-400" />
                        <span class="text-sm text-blue-800 dark:text-blue-300">
                            "This is an informational message"
                        </span>
                    </div>

                    // Contact with mail icon
                    <a
                        href="mailto:contact@example.com"
                        class="inline-flex items-center gap-2 text-gray-700 dark:text-gray-300 hover:text-indigo-600 dark:hover:text-indigo-400"
                    >
                        <Icon icon=Mail class="w-5 h-5" />
                        "contact@example.com"
                    </a>

                </div>
                <div>
                    // Copy button
                    <button class="inline-flex items-center gap-2 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors">
                        <Icon icon=CopyPaste class="w-4 h-4 text-gray-600 dark:text-gray-400" />
                        <span class="text-sm">"Copy to clipboard"</span>
                    </button>
                </div>
            </section>

            // Brand Icons Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Brand Icons"</h2>
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                    "Social media icons with appropriate styling."
                </p>
                <div class="flex gap-4">
                    <a
                        href="#"
                        class="p-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors"
                        aria-label="GitHub"
                    >
                        <Icon icon=IconGitHub class="w-6 h-6" />
                    </a>
                    <a
                        href="#"
                        class="p-2 text-gray-600 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors"
                        aria-label="LinkedIn"
                    >
                        <Icon icon=IconLinkedIn class="w-6 h-6" />
                    </a>
                    <a
                        href="#"
                        class="p-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors"
                        aria-label="X (Twitter)"
                    >
                        <Icon icon=IconX class="w-6 h-6" />
                    </a>
                </div>
            </section>

            // Code Example Section
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Usage"</h2>
                <div class="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-x-auto">
                    <pre class="text-sm">
                        <code>
                            "use mosaic_tiles::icon::*;
                            
                            // Basic usage
                            view! { <Icon icon=IconSearch /> }
                            
                            // With size and color
                            view! { <Icon icon=IconGitHub class=\"w-6 h-6 text-gray-600\" /> }
                            
                            // In a button
                            view! {
                                <button class=\"inline-flex items-center gap-2\">
                                    <Icon icon=IconSearch class=\"w-4 h-4\" />
                                    \"Search\"
                                </button>
                            }"
                        </code>
                    </pre>
                </div>
            </section>

            // Tree-Shaking Info
            <section>
                <h2 class="text-2xl font-semibold mb-4">"Tree-Shaking"</h2>
                <p class="text-gray-600 dark:text-gray-400">
                    "Only icons used in your code are compiled into the WASM bundle. Unused icons are automatically eliminated at compile time, keeping bundle sizes minimal."
                </p>
            </section>
        </div>
    }
}
