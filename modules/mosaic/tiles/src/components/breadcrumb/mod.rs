use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn Breadcrumb(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <nav aria-label="Breadcrumb" class="breadcrumb">
            <ol class="breadcrumb-list">
                {if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }}
            </ol>
        </nav>
    }
}

#[component]
pub fn BreadcrumbItem(
    /// Optional href for the breadcrumb item. If provided, renders as a link.
    /// If omitted, renders as plain text (typically for the current page).
    #[prop(optional, into)]
    href: Option<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <li class="breadcrumb-item">
            {if let Some(href) = href {
                Either::Left(view! {
                    <a href=href class="breadcrumb-link">
                        {if let Some(children) = children {
                            Either::Left(children())
                        } else {
                            Either::Right(())
                        }}
                    </a>
                })
            } else {
                Either::Right(view! {
                    <span class="breadcrumb-current" aria-current="page">
                        {if let Some(children) = children {
                            Either::Left(children())
                        } else {
                            Either::Right(())
                        }}
                    </span>
                })
            }}
        </li>
    }
}
