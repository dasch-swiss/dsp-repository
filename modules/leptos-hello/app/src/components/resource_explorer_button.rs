use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

/// ResourceExplorerButton component that displays a button and opens a DaisyUI modal dialog.
///
/// This component provides a button that, when clicked, opens a modal dialog
/// using DaisyUI's modal component pattern.
#[component]
pub fn ResourceExplorerButton(
    /// The text to display on the button
    #[prop(default = "Explore Resources".to_string(), into)]
    button_text: String,
    /// Optional modal content
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);

    let open_modal = move |_e: MouseEvent| {
        is_open.set(true);
    };

    let close_modal = move |_e: MouseEvent| {
        is_open.set(false);
    };

    view! {
        <div>
            // Button to trigger the modal
            <button
                class="btn btn-primary"
                on:click=open_modal
            >
                {button_text}
            </button>

            // DaisyUI Modal
            <dialog
                class=move || if is_open.get() { "modal modal-open" } else { "modal" }
            >
                <div class="modal-box">
                    <h3 class="text-lg font-bold">"Resource Explorer"</h3>
                    <div class="py-4">
                        {if let Some(children) = children {
                            Either::Left(children())
                        } else {
                            Either::Right(view! {
                                <p>"This is a dummy dialog for exploring resources."</p>
                                <p>"Add your content here!"</p>
                            })
                        }}
                    </div>
                    <div class="modal-action">
                        <button class="btn" on:click=close_modal>
                            "Close"
                        </button>
                    </div>
                </div>
                // Backdrop - clicking it will close the modal
                <form method="dialog" class="modal-backdrop">
                    <button on:click=close_modal>"close"</button>
                </form>
            </dialog>
        </div>
    }
}
