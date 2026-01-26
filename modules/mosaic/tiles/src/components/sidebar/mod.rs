use leptos::either::Either;
use leptos::prelude::*;

#[cfg(feature = "icon")]
use crate::components::icon::{Hamburger, Icon};

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
/// ```rust
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
#[component]
pub fn Sidebar(
    /// Optional signal to control sidebar open state externally
    #[prop(optional, into)]
    open: Option<RwSignal<bool>>,
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Child components
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let is_open = open.unwrap_or_else(|| RwSignal::new(false));

    provide_context(SidebarContext { is_open });

    view! {
        <aside class=move || {
            format!(
                "sidebar {} {}",
                if is_open.get() { "sidebar-open" } else { "sidebar-closed" },
                class.get().unwrap_or_default(),
            )
        }>
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </aside>
    }
}

/// Sidebar header component for displaying a title or navigation header.
///
/// # Example
///
/// ```rust
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
/// ```rust
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
/// ```rust
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
#[component]
pub fn SidebarTrigger(
    /// Optional signal to control sidebar state (for use outside Sidebar context)
    #[prop(optional)]
    is_open: Option<RwSignal<bool>>,
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Optional children (defaults to hamburger icon)
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    // Use provided signal or get from context
    let signal = if let Some(signal) = is_open {
        signal
    } else {
        let ctx = expect_context::<SidebarContext>();
        ctx.is_open
    };

    let toggle = move |_| {
        signal.update(|open| *open = !*open);
    };

    view! {
        <button
            class=move || format!("sidebar-trigger {}", class.get().unwrap_or_default())
            on:click=toggle
            aria-label="Toggle sidebar"
        >
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                #[cfg(feature = "icon")]
                { Either::Right(view! { <Icon icon=Hamburger class="w-6 h-6" /> }) }
                #[cfg(not(feature = "icon"))]
                { Either::Right(view! { <span class="sidebar-trigger-icon"></span> }) }
            }}
        </button>
    }
}
