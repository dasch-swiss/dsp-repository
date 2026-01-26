use leptos::either::Either;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

/// Context for managing popover state
#[derive(Clone, Copy)]
struct PopoverContext {
    is_open: RwSignal<bool>,
}

/// Main Popover container component that manages popover state.
///
/// The Popover component provides context for PopoverTrigger and PopoverContent
/// to communicate and manage the open/close state of the popover.
///
/// # Examples
///
/// ```rust
/// use mosaic_tiles::popover::*;
/// use mosaic_tiles::button::*;
///
/// view! {
///     <Popover>
///         <PopoverTrigger>
///             <Button>"Open Popover"</Button>
///         </PopoverTrigger>
///         <PopoverContent>
///             <p>"Popover content goes here"</p>
///         </PopoverContent>
///     </Popover>
/// }
/// ```
#[component]
pub fn Popover(
    /// Optional children content (PopoverTrigger and PopoverContent)
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);
    let context = PopoverContext { is_open };

    provide_context(context);

    view! {
        <div class="popover">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// PopoverTrigger wraps the trigger element (typically a Button) that opens the popover.
///
/// The trigger element will toggle the popover visibility when clicked.
///
/// # Examples
///
/// ```rust
/// use mosaic_tiles::popover::*;
/// use mosaic_tiles::button::*;
///
/// view! {
///     <PopoverTrigger>
///         <Button>"Click me"</Button>
///     </PopoverTrigger>
/// }
/// ```
#[component]
pub fn PopoverTrigger(
    /// The trigger element (typically a Button)
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let context = expect_context::<PopoverContext>();

    let toggle = move |e: MouseEvent| {
        e.stop_propagation();
        context.is_open.update(|open| *open = !*open);
    };

    view! {
        <div class="popover-trigger" on:click=toggle>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// PopoverContent displays the content in a portal when the popover is open.
///
/// The content is rendered conditionally based on the popover state and is
/// positioned absolutely relative to the trigger.
///
/// # Examples
///
/// ```rust
/// use mosaic_tiles::popover::*;
///
/// view! {
///     <PopoverContent>
///         <div class="p-4">
///             <h3>"Title"</h3>
///             <p>"Content goes here"</p>
///         </div>
///     </PopoverContent>
/// }
/// ```
#[component]
pub fn PopoverContent(
    /// The content to display in the popover
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let context = expect_context::<PopoverContext>();

    let close_popover = move |_e: MouseEvent| {
        context.is_open.set(false);
    };

    let prevent_close = move |e: MouseEvent| {
        e.stop_propagation();
    };

    view! {
        <div class=move || {
            if context.is_open.get() {
                "popover-wrapper popover-wrapper-open"
            } else {
                "popover-wrapper"
            }
        }>
            <div class="popover-overlay" on:click=close_popover></div>
            <div class="popover-content" on:click=prevent_close>
                {if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }}
            </div>
        </div>
    }
}
