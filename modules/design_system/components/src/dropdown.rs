use maud::Markup;

use crate::button::{self, ButtonVariant};
use crate::icon::{self, IconType};
use crate::menu::MenuBuilder;
use crate::ComponentBuilder;

/// Creates a dropdown with a secondary button and chevron down icon
///
/// # Arguments
/// * `id` - The dropdown ID (used for both menu and trigger IDs)
/// * `label` - The button text label
/// * `menu` - The menu builder with items configured
///
/// # Example
/// ```rust
/// use components::{dropdown::dropdown_secondary, menu::menu, menu_item, ComponentBuilder};
///
/// let dropdown = dropdown_secondary(
///     "actions-dropdown",
///     "Options",
///     menu()
///         .with_item(menu_item::link_menu_item("Edit", "/edit"))
///         .with_item(menu_item::link_menu_item("Delete", "/delete"))
/// );
/// ```
#[must_use]
pub fn dropdown_secondary(id: impl Into<String>, label: impl Into<String>, menu: MenuBuilder) -> Markup {
    let id = id.into();
    let trigger_id = format!("{}-trigger", id);

    let trigger_button = button::button(label)
        .with_id(&trigger_id)
        .variant(ButtonVariant::Secondary)
        .with_trailing_icon(icon::icon(IconType::ChevronDown))
        .popovertarget(&id)
        .build();

    menu.with_id(id).with_trigger(trigger_button).build()
}

/// Creates a dropdown with a more_vert icon button (three vertical dots)
///
/// # Arguments
/// * `id` - The dropdown ID (used for both menu and trigger IDs)
/// * `menu` - The menu builder with items configured
///
/// # Example
/// ```rust
/// use components::{dropdown::dropdown_more_vert, menu::menu, menu_item, ComponentBuilder};
///
/// let dropdown = dropdown_more_vert(
///     "more-dropdown",
///     menu()
///         .with_item(menu_item::link_menu_item("Settings", "/settings"))
///         .with_item(menu_item::link_menu_item("Help", "/help"))
/// );
/// ```
#[must_use]
pub fn dropdown_more_vert(id: impl Into<String>, menu: MenuBuilder) -> Markup {
    let id = id.into();
    let trigger_id = format!("{}-trigger", id);

    let trigger_button = button::icon_button(icon::icon(IconType::MoreVert))
        .with_id(&trigger_id)
        .popovertarget(&id)
        .build();

    menu.with_id(id).with_trigger(trigger_button).build()
}

/// Creates a dropdown with a hamburger icon button
///
/// # Arguments
/// * `id` - The dropdown ID (used for both menu and trigger IDs)
/// * `menu` - The menu builder with items configured
///
/// # Example
/// ```rust
/// use components::{dropdown::dropdown_hamburger, menu::menu, menu_item, ComponentBuilder};
///
/// let dropdown = dropdown_hamburger(
///     "nav-dropdown",
///     menu()
///         .with_item(menu_item::link_menu_item("Home", "/"))
///         .with_item(menu_item::link_menu_item("About", "/about"))
/// );
/// ```
#[must_use]
pub fn dropdown_hamburger(id: impl Into<String>, menu: MenuBuilder) -> Markup {
    let id = id.into();
    let trigger_id = format!("{}-trigger", id);

    let trigger_button = button::icon_button(icon::icon(IconType::Hamburger))
        .with_id(&trigger_id)
        .popovertarget(&id)
        .build();

    menu.with_id(id).with_trigger(trigger_button).build()
}
