use leptos::either::Either;
use leptos::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Soft,
}

#[derive(Debug, Clone, Default)]
pub enum ButtonType {
    #[default]
    Button,
    Reset,
    Submit,
}
impl ButtonVariant {
    pub fn css_class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "btn btn-primary",
            ButtonVariant::Secondary => "btn btn-secondary",
            ButtonVariant::Outline => "btn btn-outline",
            ButtonVariant::Ghost => "btn btn-ghost",
            ButtonVariant::Soft => "btn btn-soft",
        }
    }
}

impl std::fmt::Display for ButtonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ButtonType::Submit => "submit",
            ButtonType::Button => "button",
            ButtonType::Reset => "reset",
        };
        write!(f, "{}", s)
    }
}

#[component]
pub fn Button(
    /// Toggle whether or not the input is disabled.
    #[prop(optional, into)]
    disabled: MaybeProp<bool>,
    /// The type of the button. Defaults to `button`:
    /// "button|submit|reset"
    /// https://www.w3schools.com/TAGs/att_button_type.asp
    #[prop(optional, into)]
    button_type: MaybeProp<ButtonType>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] variant: ButtonVariant,
) -> impl IntoView {
    view! {
        <button
            class=move || {
                format!(
                    "{} {}",
                    "btn",
                    match variant {
                        ButtonVariant::Primary => "btn-primary",
                        ButtonVariant::Secondary => "btn-secondary",
                        ButtonVariant::Outline => "btn-outline",
                        ButtonVariant::Ghost => "btn-ghost",
                        ButtonVariant::Soft => "btn-soft",
                    },
                )
            }
            disabled=move || disabled.get_untracked()
            prop:disabled=move || disabled.get()
            type=move || button_type.get().unwrap_or_default().to_string()
        >
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </button>
    }
}
