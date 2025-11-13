use maud::{html, Markup};

const LINK_BASE_CLASSES: &str = "flex w-full px-4 py-2 text-sm text-gray-700 \
                                 focus:bg-gray-100 focus:text-gray-900 focus:outline-hidden \
                                 dark:text-gray-200 dark:focus:bg-white/5 dark:focus:text-white";

const BUTTON_BASE_CLASSES: &str = "flex w-full px-4 py-2 text-sm text-gray-700 cursor-pointer \
                                   focus:bg-gray-100 focus:text-gray-900 focus:outline-hidden \
                                   dark:text-gray-200 dark:focus:bg-white/5 dark:focus:text-white";

const ICON_CLASSES: &str = "mr-3 size-5 text-gray-400";

const DIVIDER_CLASSES: &str = "my-1 border-gray-200 dark:border-gray-700";

/// Creates a link menu item for navigation
///
/// # Example
/// ```rust
/// use components::menu_item;
///
/// let item = menu_item::link_menu_item("View Profile", "/profile");
/// ```
pub fn link_menu_item(text: impl Into<String>, href: impl Into<String>) -> Markup {
    let text = text.into();
    let href = href.into();

    html! {
        a href=(href) class=(LINK_BASE_CLASSES) data-testid="menu-item-link" {
            (text)
        }
    }
}

/// Creates a link menu item with an icon
///
/// The icon will be automatically styled with appropriate spacing and color.
///
/// # Example
/// ```rust
/// use components::{menu_item, icon, IconType};
///
/// let item = menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", icon::icon(IconType::Star));
/// ```
pub fn link_menu_item_with_icon(text: impl Into<String>, href: impl Into<String>, icon: Markup) -> Markup {
    let text = text.into();
    let href = href.into();

    html! {
        a href=(href) class=(LINK_BASE_CLASSES) data-testid="menu-item-link" {
            span class=(ICON_CLASSES) { (icon) }
            span { (text) }
        }
    }
}

/// Creates a button menu item for actions
///
/// # Example
/// ```rust
/// use components::menu_item;
///
/// let item = menu_item::button_menu_item("Delete");
/// ```
pub fn button_menu_item(text: impl Into<String>) -> Markup {
    let text = text.into();

    html! {
        button type="button" class=(BUTTON_BASE_CLASSES) data-testid="menu-item-button" {
            (text)
        }
    }
}

/// Creates a button menu item with an icon
///
/// The icon will be automatically styled with appropriate spacing and color.
///
/// # Example
/// ```rust
/// use components::{menu_item, icon, IconType};
///
/// let item = menu_item::button_menu_item_with_icon("View source", icon::icon(IconType::Code));
/// ```
pub fn button_menu_item_with_icon(text: impl Into<String>, icon: Markup) -> Markup {
    let text = text.into();

    html! {
        button type="button" class=(BUTTON_BASE_CLASSES) data-testid="menu-item-button" {
            span class=(ICON_CLASSES) { (icon) }
            span { (text) }
        }
    }
}

/// Creates a horizontal divider for separating menu items
///
/// # Example
/// ```rust
/// use components::menu_item;
/// use maud::html;
///
/// let menu = html! {
///     div {
///         (menu_item::link_menu_item("Profile", "/profile"))
///         (menu_item::menu_item_divider())
///         (menu_item::button_menu_item("Sign Out"))
///     }
/// };
/// ```
pub fn menu_item_divider() -> Markup {
    html! {
        hr class=(DIVIDER_CLASSES);
    }
}
