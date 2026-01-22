use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
}

#[derive(Debug, Clone, Default)]
pub enum ButtonType {
    #[default]
    Button,
    Reset,
    Submit,
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
    #[prop(optional, into)] on_click: Option<Callback<MouseEvent>>,
    #[prop(optional)] variant: ButtonVariant,
) -> impl IntoView {
    let btn_disabled = Memo::new(move |_| disabled.get().unwrap_or(false));
    let on_click = move |e| {
        if btn_disabled.get() {
            return;
        }
        let Some(on_click) = on_click.as_ref() else {
            return;
        };
        on_click.run(e);
    };

    view! {
        <button
            class=move || {
                format!(
                    "{} {} {}",
                    "btn",
                    match variant {
                        ButtonVariant::Primary => "btn-primary",
                        ButtonVariant::Secondary => "btn-secondary",
                        ButtonVariant::Outline => "btn-outline",
                    },
                    if btn_disabled.get() { "btn-disabled" } else { "" },
                )
            }
            disabled=btn_disabled.get()
            on:click=on_click
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
