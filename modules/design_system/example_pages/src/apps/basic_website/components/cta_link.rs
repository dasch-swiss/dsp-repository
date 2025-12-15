use maud::{html, Markup};

/// Call-to-action link with arrow
pub fn cta_link(text: impl Into<String>, href: impl Into<String>) -> Markup {
    let text = text.into();
    let href = href.into();

    html! {
        a href=(href) class="text-sm font-semibold leading-6 text-indigo-600 hover:text-indigo-500" {
            (text) " " span aria-hidden="true" { "→" }
        }
    }
}

/// Call-to-action link with arrow, centered with margin
pub fn cta_link_centered(text: impl Into<String>, href: impl Into<String>) -> Markup {
    html! {
        div class="mt-10 flex justify-center" {
            (cta_link(text, href))
        }
    }
}
