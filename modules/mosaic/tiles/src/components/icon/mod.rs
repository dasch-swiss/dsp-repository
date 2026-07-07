/// Icon component for displaying SVG icons with consistent styling.
///
/// This component provides access to 7 carefully selected icons:
/// - Navigation: ChevronUp, ChevronDown
/// - UI: Search, ArrowUpRight
/// - Social: LinkedIn, X (Twitter), GitHub
/// - Other: Star
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
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::icon::*;
///
/// // Default size
/// view! { <Icon icon=IconSearch /> };
///
/// // Explicit size
/// view! { <Icon icon=IconGitHub class="w-6 h-6 text-neutral-600" /> };
///
/// // Size inherits from parent
/// view! {
///     <button class="text-lg">
///         <Icon icon=IconSearch class="inline mr-2" />
///         "Search"
///     </button>
/// };
/// ```
pub use icondata::{
    AiAppstoreOutlined as AppStore, AiArrowLeftOutlined as IconArrowLeft, AiArrowRightOutlined as IconArrowRight,
    AiBarsOutlined as Bars, AiClockCircleOutlined as Clock, AiDownOutlined as IconChevronDown,
    AiDownloadOutlined as Download, AiExportOutlined as Export, AiInfoCircleOutlined as Info,
    AiLeftOutlined as IconChevronLeft, AiLinkedinOutlined as IconLinkedIn, AiMailOutlined as Mail,
    AiQuestionCircleOutlined as Help, AiRightOutlined as IconChevronRight, AiSearchOutlined as Search,
    AiStarOutlined as Star, AiUnorderedListOutlined as List, AiUpOutlined as IconChevronUp, BiDataRegular as Data,
    BiGridRegular as Grid, BiSearchRegular as IconSearch, BsPeople as People, CgFileDocument as Document,
    CgLock as LockClosed, CgLockUnlock as LockOpen, Icon as IconData, IoCopyOutline as CopyPaste,
    IoFlagOutline as Flag, MdiFileDownloadOutline as DownloadFile, MdiTune as Tune, OcLinkExternalLg as LinkExternal,
    OcSidebarCollapseLg as Sidebar, OcThreeBarsSm as Hamburger, RiBookOpenDocumentLine as OpenDocument,
    SiGithub as IconGitHub, SiX as IconX,
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
/// ```rust,no_run
/// use leptos::prelude::*;
/// use mosaic_tiles::icon::*;
///
/// // Basic usage
/// view! { <Icon icon=IconSearch /> };
///
/// // With size and color
/// view! { <Icon icon=IconGitHub class="w-6 h-6 text-neutral-600 hover:text-neutral-900" /> };
/// ```
#[component]
pub fn Icon(
    /// The icon data (use IconChevronUp, IconSearch, etc.)
    icon: IconData,
    /// Optional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    // Read the class once at component creation rather than from inside a
    // `move ||` closure. All 101 call sites in this repo pass either no
    // `class` or a static string literal — none feed in a reactive signal —
    // so the wrapping closure only added a reactive subscription that turned
    // every `<Icon>` into a node visited by `<Suspense>::dry_resolve`. Under
    // streaming SSR that walk could fire after the owning scope had already
    // been disposed, hitting the recurring `tokio-rt-worker` panic at
    // `reactive_graph-0.2.11/src/traits.rs:394:39`. The actual panic surface
    // for DPE was eliminated by removing the `<Suspense>` boundaries on the
    // projects routes; this is a defensive cleanup that removes the
    // closure regardless of where Icon is rendered (e.g. mosaic-playground
    // islands).
    let class_str = format!("icon {}", class.get_untracked().unwrap_or_default());
    view! {
        <svg
            class=class_str
            xmlns="http://www.w3.org/2000/svg"
            viewBox=icon.view_box
            fill="currentColor"
            inner_html=icon.data
        ></svg>
    }
}
