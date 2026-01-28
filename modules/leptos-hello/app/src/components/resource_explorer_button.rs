use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::html::Dialog;
use web_sys::HtmlDialogElement;
use wasm_bindgen::JsCast;

/// ResourceExplorerButton component that displays a button and opens a DaisyUI modal dialog.
///
/// This component provides a button that, when clicked, opens a modal dialog
/// using DaisyUI's modal component pattern.
#[island]
pub fn ResourceExplorerButton() -> impl IntoView {
    let dialog_ref = NodeRef::<Dialog>::new();

    let open_modal = move |_e: MouseEvent| {
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
                "Explore Resources"
            </button>

            // DaisyUI Modal
            <dialog
                node_ref=dialog_ref
                class="modal"
            >
                <div class="modal-box">
                    <h3 class="text-lg font-bold">"Resource Explorer"</h3>
                    <div class="py-4">
                        <p>"This is a dummy dialog for exploring resources."</p>
                        <p>"Add your content here!"</p>
                    </div>
                    <div class="modal-action">
                        <form method="dialog">
                            <button class="btn">"Close"</button>
                        </form>
                    </div>
                </div>
                // Backdrop - clicking it will close the modal
                <form method="dialog" class="modal-backdrop">
                    <button>"close"</button>
                </form>
            </dialog>
        </div>
    }
}
