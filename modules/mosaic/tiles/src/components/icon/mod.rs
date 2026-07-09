//! Icon component for rendering SVG icons with consistent styling.
//!
//! Icons are sourced from the `icondata` crate, which tree-shakes unused icons
//! at compile time. Sizing is controlled via Tailwind classes passed in `class`
//! or inherited from the parent context.
//!
//! ```
//! use mosaic_tiles::icon::{icon, IconSearch};
//!
//! let markup = icon(IconSearch, "w-5 h-5 text-neutral-600");
//! ```

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
use maud::{html, Markup, PreEscaped};

/// Render an SVG icon with the base `icon` class plus any `class` extras.
///
/// The SVG inner markup comes from the trusted `icondata` crate, so it is
/// emitted with `PreEscaped`. This is the only sanctioned `PreEscaped` site in
/// the tiles library — never feed user-controlled content through it.
///
/// Icons carry no accessible name of their own: they are decorative and the
/// name always comes from surrounding text or an `aria-label` on the enclosing
/// control. They are therefore emitted with `aria-hidden="true"` (and
/// `focusable="false"` to keep legacy Edge/IE from tab-focusing the SVG).
#[must_use]
pub fn icon(icon: IconData, class: &str) -> Markup {
    html! {
        svg class=(format!("icon {class}"))
            xmlns="http://www.w3.org/2000/svg"
            viewBox=[icon.view_box]
            fill="currentColor"
            aria-hidden="true"
            focusable="false"
        { (PreEscaped(icon.data)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_svg_with_base_and_extra_classes() {
        let out = icon(IconSearch, "w-5 h-5").into_string();
        assert!(out.starts_with("<svg"), "expected an <svg> root, got: {out}");
        assert!(out.contains(r#"class="icon w-5 h-5""#), "missing composed class: {out}");
        assert!(out.contains(r#"fill="currentColor""#));
        assert!(out.contains("viewBox="), "missing viewBox attribute: {out}");
        assert!(
            out.contains(r#"aria-hidden="true""#),
            "decorative icon must be aria-hidden: {out}"
        );
    }

    #[test]
    fn empty_class_keeps_base_class() {
        let out = icon(IconSearch, "").into_string();
        assert!(out.contains(r#"class="icon ""#), "expected bare base class: {out}");
    }
}
