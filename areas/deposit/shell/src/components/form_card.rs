use maud::{html, Markup};
use mosaic_tiles::card::{card, CardVariant};
use mosaic_tiles::ComponentBuilder;

/// The one white card with heading, fields and actions that the login screens and other
/// single-form pages share. It does not centre itself; the caller places it.
pub fn form_card(heading: &str, body: Markup) -> Markup {
    let inner = html! {
        h1 class="font-display font-semibold text-2xl text-neutral-900" { (heading) }
        (body)
    };
    card(inner)
        .variant(CardVariant::Bordered)
        .class("w-full max-w-md p-6 flex flex-col gap-4")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_heading_is_the_h1_and_precedes_the_body() {
        let out = form_card(
            "Sign in",
            html! {
                p { "fields" }
            },
        )
        .into_string();
        let h1 = out.find("<h1").expect("h1 renders");
        let body = out.find("<p>fields</p>").expect("body renders");
        assert!(h1 < body, "{out}");
        assert!(out.contains(">Sign in</h1>"), "{out}");
    }

    #[test]
    fn test_the_heading_is_escaped() {
        let out = form_card("<script>alert(1)</script>", html! {}).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
