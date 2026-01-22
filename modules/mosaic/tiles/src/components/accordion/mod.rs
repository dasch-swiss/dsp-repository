use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use crate::components::icon::{Icon, IconChevronRight};

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
                <Icon
                    icon=IconChevronRight
                    class=Signal::derive(move || {
                        if is_open.get() {
                            "accordion-icon accordion-icon-open".to_string()
                        } else {
                            "accordion-icon".to_string()
                        }
                    })
                />
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
