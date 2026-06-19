//! Badge tile with variant and size enums mapped to complete, literal classes.

use maud::{html, Markup};

#[derive(Clone, Copy, Debug, Default)]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Info,
}

impl BadgeVariant {
    pub fn css_class(self) -> &'static str {
        match self {
            BadgeVariant::Primary => "badge-primary",
            BadgeVariant::Secondary => "badge-secondary",
            BadgeVariant::Success => "badge-success",
            BadgeVariant::Warning => "badge-warning",
            BadgeVariant::Danger => "badge-danger",
            BadgeVariant::Info => "badge-info",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl BadgeSize {
    pub fn css_class(self) -> &'static str {
        match self {
            BadgeSize::Small => "badge-sm",
            BadgeSize::Medium => "badge-md",
            BadgeSize::Large => "badge-lg",
        }
    }
}

#[derive(Default)]
pub struct BadgeProps {
    pub variant: BadgeVariant,
    pub size: BadgeSize,
}

/// Render a `<span class="badge …">` wrapping the given content.
pub fn badge(props: BadgeProps, content: Markup) -> Markup {
    html! {
        span class=(format!("badge {} {}", props.variant.css_class(), props.size.css_class())) {
            (content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping() {
        assert_eq!(BadgeVariant::Primary.css_class(), "badge-primary");
        assert_eq!(BadgeVariant::Secondary.css_class(), "badge-secondary");
        assert_eq!(BadgeVariant::Success.css_class(), "badge-success");
        assert_eq!(BadgeVariant::Warning.css_class(), "badge-warning");
        assert_eq!(BadgeVariant::Danger.css_class(), "badge-danger");
        assert_eq!(BadgeVariant::Info.css_class(), "badge-info");
    }

    #[test]
    fn size_class_mapping() {
        assert_eq!(BadgeSize::Small.css_class(), "badge-sm");
        assert_eq!(BadgeSize::Medium.css_class(), "badge-md");
        assert_eq!(BadgeSize::Large.css_class(), "badge-lg");
    }

    #[test]
    fn default_badge_is_primary_medium() {
        let out = badge(
            BadgeProps::default(),
            html! {
                "New"
            },
        )
        .into_string();
        assert!(out.contains(r#"class="badge badge-primary badge-md""#), "{out}");
        assert!(out.contains(">New</span>"));
    }

    #[test]
    fn composes_variant_and_size() {
        let out = badge(BadgeProps { variant: BadgeVariant::Danger, size: BadgeSize::Large }, html! {}).into_string();
        assert!(out.contains(r#"class="badge badge-danger badge-lg""#), "{out}");
    }
}
