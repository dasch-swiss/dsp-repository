use leptos::either::Either;
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
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
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
    view! {
        <details class="accordion-item" open=default_open.then_some(true)>
            <summary class="accordion-header">
                <span class="accordion-title">{title}</span>
                <Icon icon=IconChevronRight class="accordion-icon" />
            </summary>
            <div class="accordion-content">
                <div class="accordion-body">
                    {if let Some(children) = children {
                        Either::Left(children())
                    } else {
                        Either::Right(())
                    }}
                </div>
            </div>
        </details>
    }
}
