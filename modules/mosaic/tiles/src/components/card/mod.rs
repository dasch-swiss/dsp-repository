use leptos::either::Either;
use leptos::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Bordered,
    Elevated,
    AutoHover,
}

#[component]
pub fn Card(
    /// Main content of the card
    #[prop(optional)]
    children: Option<Children>,
    /// Visual variant of the card
    #[prop(optional)]
    variant: CardVariant,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let card_class = move || {
        let variant_class = match variant {
            CardVariant::Default => "card-default",
            CardVariant::Bordered => "card-bordered",
            CardVariant::Elevated => "card-elevated",
            CardVariant::AutoHover => "card-autohover",
        };

        let additional_class = class.get().unwrap_or_default();

        format!("card {} {}", variant_class, additional_class)
    };

    view! {
        <div class=card_class>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

#[component]
pub fn CardHeader(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <div class="card-header">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

#[component]
pub fn CardBody(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <div class="card-body">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

#[component]
pub fn CardFooter(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <div class="card-footer">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}
