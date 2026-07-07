use leptos::prelude::*;

#[component]
pub fn TypographyExample() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <div>
                <h4 class="text-sm font-semibold text-neutral-500 mb-3">
                    "font-display"
                    <span class="font-normal text-neutral-400">
                        " — Lora, Georgia, Times New Roman, serif"
                    </span>
                </h4>
                <div style="font-family: var(--font-display)">
                    <p class="text-4xl mb-2">"The quick brown fox jumps over the lazy dog"</p>
                    <p class="text-2xl mb-2">"Heading level two in display font"</p>
                    <p class="text-lg">"Smaller heading in Lora with serif fallbacks"</p>
                </div>
            </div>

            <div>
                <h4 class="text-sm font-semibold text-neutral-500 mb-3">
                    "font-body"
                    <span class="font-normal text-neutral-400">
                        " — Lato, Helvetica Neue, Arial, sans-serif"
                    </span>
                </h4>
                <div style="font-family: var(--font-body)">
                    <p class="text-base mb-2">
                        "Body text set in Lato. This is the default font for paragraph content, form labels, and UI elements. The fallback chain ensures readable sans-serif text even without web fonts loaded."
                    </p>
                    <p class="text-sm text-neutral-600">
                        "Smaller body text for captions, metadata, and secondary information."
                    </p>
                </div>
            </div>
        </div>
    }
}
