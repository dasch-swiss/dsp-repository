use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::prelude::*;

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
pub fn HowToCite(citation: String) -> impl IntoView {
    let (tooltip_state, set_tooltip_state) = signal("Copy");
    let (show_tooltip, set_show_tooltip) = signal(false);

    let citation_for_handler = citation.clone();
    let handle_copy = move |_| {
        copy_to_clipboard(&citation_for_handler);
        set_tooltip_state.set("Copied!");
        set_show_tooltip.set(true);
    };

    view! {
        <div id="how-to-cite" class="bg-base-100 p-6 rounded-lg scroll-mt-52">
            <h3 class="text-xl font-bold mb-3">"How to Cite"</h3>
                <button
                    class="btn btn-sm btn-primary tooltip tooltip-left"
                    class:tooltip-open=move || show_tooltip.get()
                    data-tip=move || tooltip_state.get()
                    on:click=handle_copy
                >copy</button>
        </div>
    }
}
