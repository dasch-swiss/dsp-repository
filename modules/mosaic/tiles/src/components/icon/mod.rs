/// Icon component for displaying SVG icons with consistent styling.
///
/// This component provides access to 7 carefully selected icons:
/// - Navigation: ChevronUp, ChevronDown
/// - UI: Search, ArrowUpRight
/// - Social: LinkedIn, X (Twitter), GitHub
///
/// # Sizing
///
/// Icons automatically adapt to their context. Control size via:
/// - Explicit Tailwind classes: `class="w-5 h-5"`
/// - Parent context: Icons inherit sizing from parent elements
/// - Default: Icons use their intrinsic SVG dimensions
///
/// # Tree-Shaking
///
/// Only icons actually used in your code are included in the WASM bundle.
/// Unused icons are automatically eliminated at compile time.
///
/// # Examples
///
/// ```rust
/// use mosaic_tiles::icon::*;
///
/// // Default size
/// view! { <Icon icon=IconSearch /> }
///
/// // Explicit size
/// view! { <Icon icon=IconGitHub class="w-6 h-6 text-gray-600" /> }
///
/// // Size inherits from parent
/// view! {
///     <button class="text-lg">
///         <Icon icon=IconSearch class="inline mr-2" />
///         "Search"
///     </button>
/// }
/// ```
pub use icondata::{
    AiDownOutlined as IconChevronDown, AiInfoCircleOutlined as Info, AiLeftOutlined as IconChevronLeft,
    AiLinkedinOutlined as IconLinkedIn, AiMailOutlined as Mail, AiRightOutlined as IconChevronRight,
    AiSearchOutlined as IconSearch, AiUpOutlined as IconChevronUp, Icon as IconData, IoCopyOutline as CopyPaste,
    OcLinkExternalLg as LinkExternal, SiGithub as IconGitHub, SiX as IconX,
};
use leptos::prelude::*;

/// Icon component for rendering SVG icons with consistent styling.
///
/// # Props
///
/// * `icon` - The icon data (use IconChevronUp, IconSearch, etc.)
/// * `class` - Optional CSS classes for custom styling
///
/// # Examples
///
/// ```rust
/// use mosaic_tiles::icon::*;
///
/// // Basic usage
/// view! { <Icon icon=IconSearch /> }
///
/// // With size and color
/// view! { <Icon icon=IconGitHub class="w-6 h-6 text-gray-600 hover:text-gray-900" /> }
/// ```
#[component]
pub fn Icon(
    /// The icon data (use IconChevronUp, IconSearch, etc.)
    icon: IconData,
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    view! {
        <svg
            class=move || format!("icon {}", class.get().unwrap_or_default())
            xmlns="http://www.w3.org/2000/svg"
            viewBox=icon.view_box
            fill="currentColor"
            inner_html=icon.data
        ></svg>
    }
}
