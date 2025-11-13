use components::{dropdown, icon, menu, menu_item, IconType};
use maud::Markup;

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Dropdown component renderer for Component Store (isolated component variants)
pub struct DropdownComponentRenderer;

impl ComponentRenderer for DropdownComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Create sample icons for menu items
        let star_icon = icon::icon(IconType::Star);
        let code_icon = icon::icon(IconType::Code);

        match variant {
            "secondary" => {
                let items = vec![
                    menu_item::link_menu_item("Edit", "/edit"),
                    menu_item::link_menu_item("Duplicate", "/duplicate"),
                    menu_item::menu_item_divider(),
                    menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", star_icon.clone()),
                    menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()),
                    menu_item::menu_item_divider(),
                    menu_item::button_menu_item("Delete"),
                ];

                Ok(dropdown::dropdown_secondary(
                    "secondary-dropdown",
                    "Options",
                    menu::menu().with_items(items),
                ))
            }
            "more-vert" => {
                let items = vec![
                    menu_item::link_menu_item("Settings", "/settings"),
                    menu_item::link_menu_item("Help", "/help"),
                    menu_item::menu_item_divider(),
                    menu_item::button_menu_item("Sign Out"),
                ];

                Ok(dropdown::dropdown_more_vert(
                    "more-vert-dropdown",
                    menu::menu().with_items(items),
                ))
            }
            "hamburger" => {
                let items = vec![
                    menu_item::link_menu_item("Home", "/"),
                    menu_item::link_menu_item("About", "/about"),
                    menu_item::link_menu_item("Services", "/services"),
                    menu_item::menu_item_divider(),
                    menu_item::link_menu_item("Contact", "/contact"),
                ];

                Ok(dropdown::dropdown_hamburger(
                    "hamburger-dropdown",
                    menu::menu().with_items(items),
                ))
            }
            _ => Err(PlaygroundError::InvalidVariant {
                component: "dropdown".to_string(),
                variant: variant.to_string(),
            }),
        }
    }
}
