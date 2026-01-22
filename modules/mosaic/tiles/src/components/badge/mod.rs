use leptos::either::Either;
use leptos::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Debug, Clone, Default)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[component]
pub fn Badge(
    /// The visual style variant of the badge
    #[prop(optional)]
    variant: BadgeVariant,
    /// The size of the badge
    #[prop(optional)]
    size: BadgeSize,
    /// Optional children content
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <span class=move || {
            format!(
                "badge {} {}",
                match variant {
                    BadgeVariant::Primary => "badge-primary",
                    BadgeVariant::Secondary => "badge-secondary",
                    BadgeVariant::Success => "badge-success",
                    BadgeVariant::Warning => "badge-warning",
                    BadgeVariant::Danger => "badge-danger",
                    BadgeVariant::Info => "badge-info",
                },
                match size {
                    BadgeSize::Small => "badge-sm",
                    BadgeSize::Medium => "badge-md",
                    BadgeSize::Large => "badge-lg",
                }
            )
        }>
            {
                if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }
            }
        </span>
    }
}
