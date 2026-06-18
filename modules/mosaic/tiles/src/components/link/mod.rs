//! Link tile: an anchor that can optionally be styled as a button.

use maud::{html, Markup};

use crate::components::button::ButtonVariant;

#[derive(Default)]
pub struct LinkProps<'a> {
    /// The URL to navigate to. Omitted from the rendered anchor when disabled.
    pub href: &'a str,
    /// Render the anchor styled as a button with the given variant.
    pub as_button: Option<ButtonVariant>,
    /// Optional `target` attribute (e.g. "_blank").
    pub target: Option<&'a str>,
    /// Optional `rel` attribute (e.g. "noopener noreferrer").
    pub rel: Option<&'a str>,
    /// Optional `aria-label` for icon-only or image-only links.
    pub aria_label: Option<&'a str>,
    /// Toggle whether the link is disabled.
    pub disabled: bool,
}

/// Render an `<a>` element. In button mode it carries the variant classes and a
/// `-1` tabindex when disabled; in link mode it carries the `link` classes.
pub fn link(props: LinkProps, content: Markup) -> Markup {
    let href = (!props.disabled).then_some(props.href);
    let aria_disabled = props.disabled.then_some("true");
    let tabindex = (props.as_button.is_some() && props.disabled).then_some("-1");
    let class = match props.as_button {
        Some(variant) => variant.css_class().to_string(),
        None => format!("link {}", if props.disabled { "link-disabled" } else { "" }),
    };
    html! {
        a href=[href]
          class=(class)
          target=[props.target]
          rel=[props.rel]
          aria-label=[props.aria_label]
          aria-disabled=[aria_disabled]
          tabindex=[tabindex] {
            (content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_renders_anchor_with_link_class() {
        let out = link(LinkProps { href: "/x", ..Default::default() }, html! { "Go" }).into_string();
        assert!(out.starts_with("<a "), "{out}");
        assert!(out.contains(r#"href="/x""#));
        assert!(out.contains(r#"class="link "#), "{out}");
        assert!(out.contains(">Go</a>"));
    }

    #[test]
    fn disabled_link_omits_href_and_marks_aria() {
        let out = link(LinkProps { href: "/x", disabled: true, ..Default::default() }, html! {}).into_string();
        assert!(!out.contains("href="), "disabled link must drop href: {out}");
        assert!(out.contains("link-disabled"), "{out}");
        assert!(out.contains(r#"aria-disabled="true""#), "{out}");
        assert!(!out.contains("tabindex="), "link mode has no tabindex: {out}");
    }

    #[test]
    fn as_button_uses_variant_class() {
        let out = link(
            LinkProps { href: "/x", as_button: Some(ButtonVariant::Primary), ..Default::default() },
            html! { "Act" },
        )
        .into_string();
        assert!(out.contains(r#"class="btn btn-primary""#), "{out}");
        assert!(!out.contains("link-disabled"));
    }

    #[test]
    fn disabled_button_link_sets_negative_tabindex() {
        let out = link(
            LinkProps { href: "/x", as_button: Some(ButtonVariant::Ghost), disabled: true, ..Default::default() },
            html! {},
        )
        .into_string();
        assert!(out.contains(r#"tabindex="-1""#), "{out}");
        assert!(!out.contains("href="), "{out}");
    }

    #[test]
    fn passes_through_target_rel_aria_label() {
        let out = link(
            LinkProps {
                href: "/x",
                target: Some("_blank"),
                rel: Some("noopener noreferrer"),
                aria_label: Some("External"),
                ..Default::default()
            },
            html! {},
        )
        .into_string();
        assert!(out.contains(r#"target="_blank""#));
        assert!(out.contains(r#"rel="noopener noreferrer""#));
        assert!(out.contains(r#"aria-label="External""#));
    }
}
