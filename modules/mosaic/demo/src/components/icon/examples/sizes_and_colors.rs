use leptos::prelude::*;
use mosaic_tiles::icon::*;

#[component]
pub fn SizesAndColorsExample() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <div>
                <h4 class="text-base font-semibold mb-3">"Sizes"</h4>
                <p class="text-sm text-neutral-600 mb-4">
                    "Control icon size using Tailwind width and height classes."
                </p>
                <div class="space-y-3">
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
            </div>

            <div>
                <h4 class="text-base font-semibold mb-3">"Colors"</h4>
                <p class="text-sm text-neutral-600 mb-4">
                    "Icons use currentColor and inherit text color from parent or Tailwind classes."
                </p>
                <div class="flex gap-6">
                    <Icon icon=IconGitHub class="w-8 h-8 text-neutral-500" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-primary-600" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-danger-500" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-success-600" />
                    <Icon icon=IconGitHub class="w-8 h-8 text-accent-500" />
                </div>
            </div>
        </div>
    }
}
