//! Card tile: a container with a variant plus header/body/footer partials.

use maud::{html, Markup};

#[derive(Clone, Copy, Debug, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Bordered,
    Elevated,
    AutoHover,
}

impl CardVariant {
    pub fn css_class(self) -> &'static str {
        match self {
            CardVariant::Default => "card-default",
            CardVariant::Bordered => "card-bordered",
            CardVariant::Elevated => "card-elevated",
            CardVariant::AutoHover => "card-autohover",
        }
    }
}

#[derive(Default)]
pub struct CardProps<'a> {
    pub variant: CardVariant,
    pub class: &'a str,
}

/// Render the outer `<div class="card …">` wrapping the given content.
pub fn card(props: CardProps, content: Markup) -> Markup {
    html! {
        div class=(format!("card {} {}", props.variant.css_class(), props.class)) {
            (content)
        }
    }
}

/// Render the card header partial.
pub fn card_header(content: Markup) -> Markup {
    html! { div class="card-header" { (content) } }
}

/// Render the card body partial with optional extra `class`.
pub fn card_body(class: &str, content: Markup) -> Markup {
    html! { div class=(format!("card-body {class}")) { (content) } }
}

/// Render the card footer partial.
pub fn card_footer(content: Markup) -> Markup {
    html! { div class="card-footer" { (content) } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping() {
        assert_eq!(CardVariant::Default.css_class(), "card-default");
        assert_eq!(CardVariant::Bordered.css_class(), "card-bordered");
        assert_eq!(CardVariant::Elevated.css_class(), "card-elevated");
        assert_eq!(CardVariant::AutoHover.css_class(), "card-autohover");
    }

    #[test]
    fn default_card_wraps_content() {
        let out = card(CardProps::default(), html! { "body" }).into_string();
        assert!(out.contains(r#"class="card card-default "#), "{out}");
        assert!(out.contains(">body</div>"));
    }

    #[test]
    fn card_carries_extra_class() {
        let out = card(CardProps { variant: CardVariant::Bordered, class: "mt-4" }, html! {}).into_string();
        assert!(out.contains("card card-bordered mt-4"), "{out}");
    }

    #[test]
    fn body_header_footer_partials() {
        assert!(card_header(html! { "h" }).into_string().contains(r#"class="card-header">h</div>"#));
        assert!(card_body("p-4", html! { "b" }).into_string().contains(r#"class="card-body p-4">b</div>"#));
        assert!(card_footer(html! { "f" }).into_string().contains(r#"class="card-footer">f</div>"#));
    }
}
