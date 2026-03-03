use leptos::either::Either;
use leptos::prelude::*;

#[cfg(feature = "button")]
use crate::components::button::ButtonVariant;

#[component]
pub fn Link(
    /// The URL to navigate to
    #[prop(into)]
    href: String,
    /// Render the link as a button component with the given variant
    #[prop(optional, into)]
    as_button: MaybeProp<ButtonVariant>,
    /// Optional target attribute (e.g., "_blank", "_self")
    #[prop(optional, into)]
    target: Option<String>,
    /// Optional rel attribute (e.g., "noopener noreferrer")
    #[prop(optional, into)]
    rel: Option<String>,
    /// Toggle whether the link is disabled
    #[prop(optional, into)]
    disabled: MaybeProp<bool>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let is_disabled = Memo::new(move |_| disabled.get().unwrap_or(false));
    let button_variant = as_button.get();

    #[cfg(feature = "button")]
    if let Some(variant) = button_variant {
        return view! {
            <a
                href=move || {
                    if is_disabled.get() {
                        None
                    } else {
                        Some(href.clone())
                    }
                }
                class=variant.css_class()
                target=target
                rel=rel
                aria-disabled=move || if is_disabled.get() { Some("true") } else { None }
                tabindex=move || if is_disabled.get() { Some("-1") } else { None }
            >
                {if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }}
            </a>
        }
        .into_any();
    }

    view! {
        <a
            href=move || {
                if is_disabled.get() {
                    None
                } else {
                    Some(href.clone())
                }
            }
            class=move || {
                format!("link {}", if is_disabled.get() { "link-disabled" } else { "" })
            }
            target=target
            rel=rel
            aria-disabled=move || if is_disabled.get() { Some("true") } else { None }
        >
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </a>
    }
    .into_any()
}
