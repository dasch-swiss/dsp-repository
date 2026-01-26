use leptos::either::Either;
use leptos::prelude::*;

#[cfg(feature = "icon")]
use crate::icon::IconData;

/// Tabs container component that manages a set of tabs with CSS-only switching.
///
/// Tabs use radio buttons for CSS-only tab switching, providing an accessible
/// and performant solution without JavaScript.
///
/// # Examples
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::tabs::*;
/// use mosaic_tiles::icon::*;
///
/// view! {
///     <Tabs>
///         <Tab
///             name="my-tabs"
///             value="tab1"
///             label="First Tab"
///             checked=true
///         >
///             <p>"Content of first tab"</p>
///         </Tab>
///         <Tab
///             name="my-tabs"
///             value="tab2"
///             label="Second Tab"
///             icon=IconSearch
///         >
///             <p>"Content of second tab"</p>
///         </Tab>
///     </Tabs>
/// };
/// ```
#[component]
pub fn Tabs(
    /// The Tab components
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="tabs">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}

/// Individual tab component with label, optional icon, and content panel.
///
/// Each Tab renders a radio input, label, and content panel. The radio inputs
/// enable CSS-only tab switching. All tabs in a group must share the same `name`
/// prop to function as a mutually exclusive set.
///
/// # Props
///
/// * `name` - Radio group name (must be the same for all tabs in a group)
/// * `value` - Unique identifier for this tab
/// * `label` - The tab label text
/// * `icon` - Optional icon to display before the label (requires "icon" feature)
/// * `checked` - Whether this tab is initially selected
/// * `children` - The content to display when this tab is active
///
/// # Examples
///
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::tabs::*;
/// use mosaic_tiles::icon::*;
///
/// // Tab with icon
/// view! {
///     <Tab
///         name="tabs"
///         value="search"
///         label="Search"
///         icon=IconSearch
///         checked=true
///     >
///         <p>"Search content"</p>
///     </Tab>
/// };
///
/// // Simple tab without icon
/// view! {
///     <Tab
///         name="tabs"
///         value="settings"
///         label="Settings"
///     >
///         <p>"Settings content"</p>
///     </Tab>
/// };
/// ```
#[component]
pub fn Tab(
    /// Radio group name - must be the same for all tabs in a group
    #[prop(into)]
    name: String,
    /// Unique identifier for this tab
    #[prop(into)]
    value: String,
    /// The tab label text
    #[prop(into)]
    label: String,
    /// Optional icon to display before the label
    #[cfg(feature = "icon")]
    #[prop(optional)]
    icon: Option<IconData>,
    /// Whether this tab is initially selected
    #[prop(optional, into)]
    checked: MaybeProp<bool>,
    /// The content to display when this tab is active
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let is_checked = Memo::new(move |_| checked.get().unwrap_or(false));
    let input_id = format!("{}-{}", name, value);

    view! {
        <input
            type="radio"
            class="tab-input"
            id=input_id.clone()
            name=name
            value=value
            checked=is_checked.get()
        />
        <label class="tab-label" for=input_id>
            {#[cfg(feature = "icon")]
            if let Some(icon_data) = icon {
                Either::Left(
                    view! {
                        <svg
                            class="tab-icon"
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox=icon_data.view_box
                            fill="currentColor"
                            inner_html=icon_data.data
                        ></svg>
                    },
                )
            } else {
                Either::Right(())
            }}
            {#[cfg(not(feature = "icon"))]
            ()}
            <span>{label}</span>
        </label>
        <div class="tab-panel">
            {if let Some(children) = children {
                Either::Left(children())
            } else {
                Either::Right(())
            }}
        </div>
    }
}
