use leptos::either::Either;
use leptos::prelude::*;

/// Context for managing sidebar state
#[derive(Clone, Copy)]
struct SidebarContext {
    is_open: RwSignal<bool>,
}

/// Main Sidebar component that provides context for all child components.
///
/// # Props
///
/// * `open` - Optional signal to control sidebar open state externally
/// * `children` - Child components (SidebarHeader, SidebarContent, SidebarGroup)
///
/// # Example
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::sidebar::*;
///
/// view! {
///     <Sidebar>
///         <SidebarHeader>"Navigation"</SidebarHeader>
///         <SidebarContent>
///             <SidebarGroup>
///                 // Menu items here
///             </SidebarGroup>
///         </SidebarContent>
///     </Sidebar>
/// };
/// ```
#[island]
pub fn Sidebar(
    /// Child components
    children: Children,
) -> impl IntoView {
    view! {
        <div class="sidebar-wrapper">
            <aside class="sidebar ">{children()}</aside>
        </div>
    }
}

/// Sidebar header component for displaying a title or navigation header.
///
/// # Example
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::sidebar::*;
///
/// view! {
///     <SidebarHeader>"Navigation"</SidebarHeader>
/// };
/// ```
#[component]
pub fn SidebarHeader(
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Child components
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=move || {
            format!("sidebar-header {}", class.get().unwrap_or_default())
        }>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// Sidebar content component for the main scrollable content area.
///
/// # Example
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::sidebar::*;
///
/// view! {
///     <SidebarContent>
///         <SidebarGroup>
///             // Menu items
///         </SidebarGroup>
///     </SidebarContent>
/// };
/// ```
#[component]
pub fn SidebarContent(
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Child components
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=move || {
            format!("sidebar-content {}", class.get().unwrap_or_default())
        }>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// Sidebar group component for organizing related navigation items.
///
/// # Example
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::sidebar::*;
///
/// view! {
///     <SidebarGroup>
///         <a href="/dashboard">"Dashboard"</a>
///         <a href="/settings">"Settings"</a>
///     </SidebarGroup>
/// };
/// ```
#[component]
pub fn SidebarGroup(
    /// Optional group label/title
    #[prop(optional, into)]
    label: MaybeProp<String>,
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Child components
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=move || {
            format!("sidebar-group {}", class.get().unwrap_or_default())
        }>
            {move || {
                if let Some(label_text) = label.get() {
                    Either::Left(view! { <div class="sidebar-group-label">{label_text}</div> })
                } else {
                    Either::Right(())
                }
            }}
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// Separate trigger component for toggling the sidebar open/closed.
///
/// Can be used either:
/// - Inside a Sidebar component (will use context)
/// - Outside with an explicit `is_open` signal prop
///
/// # Example
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::sidebar::*;
///
/// // With context (inside Sidebar)
/// view! {
///     <SidebarTrigger />
/// };
///
/// // With explicit signal (outside Sidebar)
/// let is_open = RwSignal::new(false);
/// view! {
///     <SidebarTrigger is_open=is_open />
///     <Sidebar open=is_open>
///         // ...
///     </Sidebar>
/// };
/// ```
#[island]
pub fn SidebarTrigger() -> impl IntoView {
    // Get signal from context (must be inside Sidebar island)
    let ctx = expect_context::<SidebarContext>();
    let signal = ctx.is_open;

    let toggle = move |_| {
        signal.update(|open| *open = !*open);
    };

    view! {
        <button class="sidebar-trigger" on:click=toggle aria-label="Toggle sidebar">
            <span class="sidebar-trigger-icon"></span>
        </button>
    }
}
