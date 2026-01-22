use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn Accordion(
    /// Optional children content (AccordionItem components)
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="accordion">
            {
                if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }
            }
        </div>
    }
}

#[component]
pub fn AccordionItem(
    /// The title/header text for the accordion item
    title: String,
    /// Whether the item starts open or closed
    #[prop(optional)]
    default_open: bool,
    /// Optional children content (the accordion body)
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(default_open);

    let toggle = move |_: MouseEvent| {
        set_is_open.update(|open| *open = !*open);
    };

    view! {
        <div class="accordion-item">
            <button
                class="accordion-header"
                on:click=toggle
                aria-expanded=move || is_open.get()
            >
                <span class="accordion-title">{title}</span>
                <svg
                    class=move || {
                        if is_open.get() {
                            "accordion-icon accordion-icon-open"
                        } else {
                            "accordion-icon"
                        }
                    }
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
            </button>
            <div
                class=move || {
                    if is_open.get() {
                        "accordion-content accordion-content-open"
                    } else {
                        "accordion-content"
                    }
                }
            >
                <div class="accordion-body">
                    {
                        if let Some(children) = children {
                            Either::Left(children())
                        } else {
                            Either::Right(())
                        }
                    }
                </div>
            </div>
        </div>
    }
}
