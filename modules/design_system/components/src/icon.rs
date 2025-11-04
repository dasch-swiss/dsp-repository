use maud::{html, Markup};

/// Available icon types from Heroicons
#[derive(Debug, Clone, Copy)]
pub enum IconType {
    /// Star icon - typically used for favorites/ratings
    Star,
    /// Code icon - typically used for source code/embed actions
    Code,
    /// Flag icon - typically used for reporting/flagging content
    Flag,
    /// Hamburger menu icon - typically used for mobile navigation
    Hamburger,
    /// Close/X icon - typically used for dismissing dialogs
    Close,
    /// Chevron down icon - typically used for dropdowns
    ChevronDown,
}

impl IconType {
    /// Returns the SVG path data for this icon
    fn path_data(&self) -> &'static str {
        match self {
            IconType::Star => "M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401Z",
            IconType::Code => "M6.28 5.22a.75.75 0 0 1 0 1.06L2.56 10l3.72 3.72a.75.75 0 0 1-1.06 1.06L.97 10.53a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0Zm7.44 0a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L17.44 10l-3.72-3.72a.75.75 0 0 1 0-1.06ZM11.377 2.011a.75.75 0 0 1 .612.867l-2.5 14.5a.75.75 0 0 1-1.478-.255l2.5-14.5a.75.75 0 0 1 .866-.612Z",
            IconType::Flag => "M3.5 2.75a.75.75 0 0 0-1.5 0v14.5a.75.75 0 0 0 1.5 0v-4.392l1.657-.348a6.449 6.449 0 0 1 4.271.572 7.948 7.948 0 0 0 5.965.524l2.078-.64A.75.75 0 0 0 18 12.25v-8.5a.75.75 0 0 0-.904-.734l-2.38.501a7.25 7.25 0 0 1-4.186-.363l-.502-.2a8.75 8.75 0 0 0-5.053-.439l-1.475.31V2.75Z",
            IconType::Hamburger => "M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5",
            IconType::Close => "M6 18 18 6M6 6l12 12",
            IconType::ChevronDown => "M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z",
        }
    }

    /// Returns the viewBox dimensions for this icon
    fn view_box(&self) -> &'static str {
        match self {
            IconType::Hamburger | IconType::Close => "0 0 24 24",
            _ => "0 0 20 20",
        }
    }

    /// Returns whether this icon uses stroke instead of fill
    fn uses_stroke(&self) -> bool {
        matches!(self, IconType::Hamburger | IconType::Close)
    }
}

const DEFAULT_SIZE_CLASS: &str = "size-5";

/// Creates an icon with default styling
///
/// # Example
/// ```rust
/// use components::icon::{icon, IconType};
///
/// let star = icon(IconType::Star);
/// ```
pub fn icon(icon_type: IconType) -> Markup {
    icon_with_class(icon_type, DEFAULT_SIZE_CLASS)
}

/// Creates an icon with custom CSS classes
///
/// # Example
/// ```rust
/// use components::icon::{icon_with_class, IconType};
///
/// let large_star = icon_with_class(IconType::Star, "size-8 text-yellow-500");
/// ```
pub fn icon_with_class(icon_type: IconType, css_class: impl Into<String>) -> Markup {
    let css_class = css_class.into();
    let path_data = icon_type.path_data();
    let view_box = icon_type.view_box();

    if icon_type.uses_stroke() {
        html! {
            svg
                viewBox=(view_box)
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                data-slot="icon"
                aria-hidden="true"
                class=(css_class)
            {
                path d=(path_data) stroke-linecap="round" stroke-linejoin="round";
            }
        }
    } else {
        html! {
            svg
                viewBox=(view_box)
                fill="currentColor"
                data-slot="icon"
                aria-hidden="true"
                class=(css_class)
            {
                path d=(path_data) clip-rule="evenodd" fill-rule="evenodd";
            }
        }
    }
}

/// Creates an icon specifically styled for menu items
///
/// This applies the standard menu item icon styling (size, margin, color).
///
/// # Example
/// ```rust
/// use components::icon::{icon_for_menu_item, IconType};
///
/// let menu_star = icon_for_menu_item(IconType::Star);
/// ```
pub fn icon_for_menu_item(icon_type: IconType) -> Markup {
    icon_with_class(icon_type, "mr-3 size-5 text-gray-400")
}
