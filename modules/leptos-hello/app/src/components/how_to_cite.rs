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

    view! {
        <div id="how-to-cite" class="bg-base-100 p-6 rounded-lg scroll-mt-52">
            <h3 class="text-xl font-bold mb-3">"How to Cite"</h3>
            <div class="relative bg-base-200 p-4 pr-12 rounded font-mono text-sm">
                {citation.clone()}
                <button
                    class="absolute top-2 right-2 btn btn-sm btn-ghost tooltip tooltip-left"
                    class:tooltip-open=move || show_tooltip.get()
                    data-tip=move || tooltip_state.get()
                    on:click={
                        let citation = citation.clone();
                        move |_| {
                            copy_to_clipboard(&citation);
                            set_tooltip_state.set("Copied!");
                            set_show_tooltip.set(true);
                            set_timeout(
                                move || {
                                    set_show_tooltip.set(false);
                                },
                                std::time::Duration::from_millis(1000),
                            );
                            set_timeout(
                                move || {
                                    set_tooltip_state.set("Copy");
                                },
                                std::time::Duration::from_millis(1300),
                            );
                        }
                    }
                >

                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        class="h-5 w-5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                        />
                    </svg>
                </button>
            </div>
        </div>
    }
}
