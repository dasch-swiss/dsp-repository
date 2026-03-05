use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
    export function copy_text_to_clipboard(text) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text);
        }
    }
")]
extern "C" {
    fn copy_text_to_clipboard(text: &str);
}

#[island]
pub fn CopyButton(text: String) -> impl IntoView {
    let (tooltip_state, set_tooltip_state) = signal("Copy");
    let (show_tooltip, set_show_tooltip) = signal(false);

    let text_for_handler = text.clone();
    let handle_copy = move |_| {
        copy_text_to_clipboard(&text_for_handler);
        set_tooltip_state.set("Copied!");
        set_show_tooltip.set(true);
    };

    view! {
        <button
            class="btn btn-ghost px-1 py-0.5 text-xs tooltip tooltip-left flex-shrink-0"
            class:tooltip-open=move || show_tooltip.get()
            data-tip=move || tooltip_state.get()
            on:click=handle_copy
        >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                />
            </svg>
        </button>
    }
}
