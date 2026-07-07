use leptos::prelude::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ButtonGroupSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ButtonGroupOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A group of buttons visually connected together.
/// Wraps regular Button components and applies connected styling.
/// Use regular Button components as children.
#[component]
pub fn ButtonGroup(
    /// The size of the button group
    #[prop(optional)]
    size: ButtonGroupSize,
    /// The orientation of the button group (horizontal or vertical)
    #[prop(optional)]
    orientation: ButtonGroupOrientation,
    /// The button elements to display in the group
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div
            class=move || {
                format!(
                    "btn-group {} {}",
                    match size {
                        ButtonGroupSize::Small => "btn-group-sm",
                        ButtonGroupSize::Medium => "btn-group-md",
                        ButtonGroupSize::Large => "btn-group-lg",
                    },
                    match orientation {
                        ButtonGroupOrientation::Horizontal => "btn-group-horizontal",
                        ButtonGroupOrientation::Vertical => "btn-group-vertical",
                    },
                )
            }

            role="group"
        >
            {if let Some(children) = children {
                leptos::either::Either::Left(children())
            } else {
                leptos::either::Either::Right(())
            }}
        </div>
    }
}
