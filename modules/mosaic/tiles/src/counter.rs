use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <Button on_click= move |_| set_count.update(|n| *n += 1)>
            "Click Me: " {count}
        </Button>
    }
}

#[component]
pub fn Button(
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional, into)] on_click: Option<Callback<MouseEvent>>,
) -> impl IntoView {
    let btn_disabled = Memo::new(move |_| disabled.get());
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
        on:click=on_click
    >
    {
        if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </button>
    }
}
