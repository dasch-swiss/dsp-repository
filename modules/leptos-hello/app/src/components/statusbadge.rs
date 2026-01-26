use crate::domain::ProjectStatus;
use leptos::prelude::*;

pub enum BadgeSize {
    Small,
    Default,
}

#[component]
pub fn ProjectStatusBadge(
    status: ProjectStatus,
    #[prop(default = BadgeSize::Default)] size: BadgeSize,
) -> impl IntoView {
    let is_ongoing = status == ProjectStatus::Ongoing;
    let is_small: bool = matches!(size, BadgeSize::Small);
    let status_text = match status {
        ProjectStatus::Ongoing => "Ongoing",
        ProjectStatus::Finished => "Finished",
    };
    view! {
        <span
            class="badge"
            class:badge-sm=is_small
            class:badge-success=!is_ongoing
            class:badge-warning=is_ongoing
        >
            {status_text}
        </span>
    }
}
