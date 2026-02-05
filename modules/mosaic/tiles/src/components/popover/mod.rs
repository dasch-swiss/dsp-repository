use leptos::context::Provider;
use leptos::either::Either;
use leptos::prelude::*;

/// Context providing the popover ID to children
#[derive(Clone)]
pub struct PopoverContext {
    pub id: String,
}

/// Marker context indicating we're inside a PopoverTrigger
#[derive(Clone)]
pub struct PopoverTriggerContext;

/// Main Popover container component that provides context for trigger and content.
///
/// The Popover component wraps PopoverTrigger and PopoverContent children.
/// The `id` prop is shared via context so the trigger button automatically
/// gets the correct `popovertarget` attribute.
///
/// # Examples
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::popover::*;
/// use mosaic_tiles::button::*;
///
/// view! {
///     <Popover id="my-popover">
///         <PopoverTrigger>
///             <Button>"Open Popover"</Button>
///         </PopoverTrigger>
///         <PopoverContent>
///             <p>"Popover content goes here"</p>
///         </PopoverContent>
///     </Popover>
/// };
/// ```
#[component]
pub fn Popover(
    /// Required ID for the popover - shared with trigger and content via context
    #[prop(into)]
    id: String,
    children: Children,
) -> impl IntoView {
    let context = PopoverContext { id };

    view! {
        <Provider value=context>
            <div class="popover">{ children() }</div>
        </Provider>
    }
}

/// PopoverTrigger wraps the trigger element (typically a Button) that opens the popover.
///
/// When a Button is placed inside PopoverTrigger, it automatically receives
/// the `popovertarget` attribute from the parent Popover's context.
///
/// # Examples
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::popover::*;
/// use mosaic_tiles::button::*;
///
/// view! {
///     <Popover id="my-popover">
///         <PopoverTrigger>
///             <Button>"Click me"</Button>
///         </PopoverTrigger>
///         <PopoverContent>
///             <p>"Content"</p>
///         </PopoverContent>
///     </Popover>
/// };
/// ```
#[component]
pub fn PopoverTrigger(children: Children) -> impl IntoView {
    view! { <Provider value=PopoverTriggerContext>{ children() }</Provider> }
}

/// PopoverContent displays the content using the native browser Popover API.
///
/// The content automatically receives its `id` from the parent Popover's context.
/// The native API handles positioning, z-index, and light dismiss.
///
/// # Examples
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::popover::*;
///
/// view! {
///     <Popover id="my-popover">
///         <PopoverTrigger>
///             <Button>"Open"</Button>
///         </PopoverTrigger>
///         <PopoverContent>
///             <div class="p-4">
///                 <h3>"Title"</h3>
///                 <p>"Content goes here"</p>
///             </div>
///         </PopoverContent>
///     </Popover>
/// };
/// ```
#[component]
pub fn PopoverContent(
    /// The content to display in the popover
    #[prop(optional)]
    children: Option<Children>,
    /// Optional additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let context = expect_context::<PopoverContext>();

    view! {
        <div
            class=format!("popover-content {}", class.get().unwrap_or_default())
            id=context.id.clone()
            popover="auto"
        >
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}
