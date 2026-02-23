use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::prelude::*;

use crate::components::InfoCard;

#[wasm_bindgen(inline_js = "
    export function copy_to_clipboard(text) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text);
        }
    }
")]
extern "C" {
    fn copy_to_clipboard(text: &str);
}

#[island]
pub fn HowToCite(permalink: String, citation: String) -> impl IntoView {
    let (permalink_tooltip_state, set_permalink_tooltip_state) = signal("Copy");
    let (show_permalink_tooltip, set_show_permalink_tooltip) = signal(false);
    let (citation_tooltip_state, set_citation_tooltip_state) = signal("Copy");
    let (show_citation_tooltip, set_show_citation_tooltip) = signal(false);

    let permalink_for_handler = permalink.clone();
    let handle_permalink_copy = move |_| {
        copy_to_clipboard(&permalink_for_handler);
        set_permalink_tooltip_state.set("Copied!");
        set_show_permalink_tooltip.set(true);
    };

    let citation_for_handler = citation.clone();
    let handle_citation_copy = move |_| {
        copy_to_clipboard(&citation_for_handler);
        set_citation_tooltip_state.set("Copied!");
        set_show_citation_tooltip.set(true);
    };

    view! {
        <div class="space-y-2">
            <h3 class="font-semibold">"Permalink"</h3>
            <InfoCard>
                <div class="flex items-center justify-between gap-3">
                    <a href={permalink.clone()} class="text-blue-600 hover:underline break-all flex-1">
                        {permalink.clone()}
                    </a>
                    <button
                        class="btn btn-xs btn-ghost tooltip tooltip-left flex-shrink-0"
                        class:tooltip-open=move || show_permalink_tooltip.get()
                        data-tip=move || permalink_tooltip_state.get()
                        on:click=handle_permalink_copy
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                        </svg>
                    </button>
                </div>
            </InfoCard>
        </div>

        <div class="space-y-2">
            <h3 class="font-semibold">"Citation"</h3>
            <InfoCard>
                <div class="space-y-3 flex items-center">
                    <div class="flex-1">{citation.clone()}</div>
                    <button
                        class="btn btn-xs btn-ghost tooltip tooltip-left"
                        class:tooltip-open=move || show_citation_tooltip.get()
                        data-tip=move || citation_tooltip_state.get()
                        on:click=handle_citation_copy
                    >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                        </svg>
        </button>
                </div>
            </InfoCard>
        </div>
    }
}
