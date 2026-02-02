use leptos::either::Either;
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
    #[prop(optional)] variant: ButtonVariant,
    /// ID of a popover element to control
    #[prop(optional, into)]
    popovertarget: MaybeProp<String>,
    /// Action to perform on the popover: "toggle" | "show" | "hide"
    #[prop(optional, into)]
    popovertargetaction: MaybeProp<String>,
) -> impl IntoView {
    let btn_disabled = Memo::new(move |_| disabled.get().unwrap_or(false));

    // Check if we're inside a PopoverTrigger context (only when popover feature is enabled)
    #[cfg(feature = "popover")]
    let popover_context = use_context::<crate::popover::PopoverContext>();
    #[cfg(feature = "popover")]
    let trigger_context = use_context::<crate::popover::PopoverTriggerContext>();

    // Clone for the closure
    #[cfg(feature = "popover")]
    let popover_ctx_clone = popover_context.clone();
    #[cfg(feature = "popover")]
    let trigger_ctx_clone = trigger_context.clone();

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
            type=move || button_type.get().unwrap_or_default().to_string()
            popovertarget=move || {
                // If inside PopoverTrigger, use the context menu_id
                // Otherwise use the explicit popovertarget prop
                #[cfg(feature = "popover")]
                {
                    if trigger_ctx_clone.is_some() {
                        if let Some(ref ctx) = popover_ctx_clone {
                            let menu_id = ctx.menu_id.get();
                            if !menu_id.is_empty() {
                                return Some(menu_id);
                            }
                        }
                    }
                }
                popovertarget.get()
            }
            popovertargetaction=move || {
                // Default to "toggle" if inside PopoverTrigger
                #[cfg(feature = "popover")]
                if trigger_context.is_some() {
                    return popovertargetaction
                        .get()
                        .or(Some("toggle".to_string()));
                }
                popovertargetaction.get()
            }
        >
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </button>
    }
}
