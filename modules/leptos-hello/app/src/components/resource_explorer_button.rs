use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::html::Dialog;
use web_sys::HtmlDialogElement;
use wasm_bindgen::JsCast;

use crate::components::{Loading, ProjectLoader};

/// ResourceExplorerButton component that displays a button and opens a DaisyUI modal dialog.
///
/// This component provides a button that, when clicked, opens a modal dialog
/// that loads the project page content.
#[island]
pub fn ResourceExplorerButton(shortcode: String) -> impl IntoView {
    let dialog_ref = NodeRef::<Dialog>::new();
    let (dialog_opened, set_dialog_opened) = signal(false);
    let shortcode = StoredValue::new(shortcode);

    let open_modal = move |_e: MouseEvent| {
        set_dialog_opened.set(true);
        if let Some(dialog_element) = dialog_ref.get() {
            if let Ok(dialog) = dialog_element.dyn_into::<HtmlDialogElement>() {
                let _ = dialog.show_modal();
            }
        }
    };

    view! {
        <div>
            // Button to trigger the modal
            <button
                class="btn btn-primary"
                on:click=open_modal
            >
                "View in dialog"
            </button>

            // DaisyUI Modal
            <dialog
                node_ref=dialog_ref
                class="modal"
            >
                <div class="modal-box max-w-7xl h-5/6 overflow-y-auto">
                    <div class="modal-action absolute top-2 right-2 mt-0">
                        <form method="dialog">
                            <button class="btn btn-sm btn-circle">"✕"</button>
                        </form>
                    </div>

                    {move || {
                        dialog_opened
                            .get()
                            .then(|| {
                                let shortcode_value = shortcode.get_value();
                                view! {
                                    <Suspense fallback=move || view! { <Loading /> }>
                                        <ProjectLoader shortcode=shortcode_value />
                                    </Suspense>
                                }
                            })
                    }}
                </div>
                // Backdrop - clicking it will close the modal
                <form method="dialog" class="modal-backdrop">
                    <button>"close"</button>
                </form>
            </dialog>
        </div>
    }
}
