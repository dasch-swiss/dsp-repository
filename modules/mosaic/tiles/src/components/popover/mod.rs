use leptos::context::Provider;
use leptos::either::Either;
use leptos::prelude::*;

/// Context for managing popover IDs to connect trigger and content
#[derive(Clone)]
pub struct PopoverContext {
    pub menu_id: RwSignal<String>,
}

/// Context provided by PopoverTrigger to its children
#[derive(Clone)]
pub struct PopoverTriggerContext {}

/// Main Popover container component that manages popover state.
///
/// The Popover component provides context for PopoverTrigger and PopoverContent
/// to communicate using the native browser Popover API.
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
    let menu_id = RwSignal::new(String::new());
    let context = PopoverContext { menu_id };

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
/// The trigger element will use the native popover API to toggle visibility.
/// When wrapping a Button component, the Button will automatically receive
/// the popovertarget attribute to control the associated popover.
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
    let trigger_context = PopoverTriggerContext {};

    view! {
        <Provider value=trigger_context>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </Provider>
    }
}

/// PopoverContent displays the content using the native browser Popover API.
///
/// The content is automatically shown/hidden by the browser when the associated
/// trigger is activated. The native API handles positioning, z-index, and light dismiss.
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
    /// Optional additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Optional explicit ID for the popover (for islands mode or custom control)
    #[prop(optional, into)]
    id: MaybeProp<String>,
) -> impl IntoView {
    let context = expect_context::<PopoverContext>();

    // Generate ID once using StoredValue to ensure consistency across SSR and hydration
    let menu_id = StoredValue::new(id.get().unwrap_or_else(|| format!("popover-{}", uuid::Uuid::new_v4())));

    // Set the menu_id in context so the trigger can access it
    Effect::new(move |_| {
        context.menu_id.set(menu_id.get_value());
    });

    view! {
        <div
            class=move || {
                format!(
                    "popover-content {}",
                    class.get().unwrap_or_default(),
                )
            }
            id=menu_id.get_value()
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
