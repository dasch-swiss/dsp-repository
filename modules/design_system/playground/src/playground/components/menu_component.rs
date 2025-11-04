use components::{button, icon, menu, menu_item, ComponentBuilder, IconType};
use maud::Markup;

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Menu component renderer for Component Store (isolated component variants)
pub struct MenuComponentRenderer;

impl ComponentRenderer for MenuComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Create sample icons for demonstration
        let star_icon = icon::icon(IconType::Star);
        let code_icon = icon::icon(IconType::Code);

        match variant {
            "text-trigger" => Ok(menu::menu()
                .with_id("text-menu")
                .with_trigger(
                    button::button("Open Menu")
                        .with_id("text-menu-trigger")
                        .popovertarget("text-menu")
                        .build(),
                )
                .with_item(menu_item::link_menu_item("Profile", "/profile"))
                .with_item(menu_item::link_menu_item("Settings", "/settings"))
                .with_item(menu_item::menu_item_divider())
                .with_item(menu_item::link_menu_item_with_icon(
                    "Add to favorites",
                    "/favorites",
                    star_icon.clone(),
                ))
                .with_item(menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()))
                .with_item(menu_item::menu_item_divider())
                .with_item(menu_item::button_menu_item("Delete"))
                .build()),
            "icon-trigger" => Ok(menu::menu()
                .with_id("icon-menu")
                .with_trigger(
                    button::icon_button(icon::icon(IconType::Hamburger))
                        .with_id("icon-menu-trigger")
                        .popovertarget("icon-menu")
                        .build(),
                )
                .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
                .with_item(menu_item::link_menu_item("Profile", "/profile"))
                .with_item(menu_item::link_menu_item("Settings", "/settings"))
                .with_item(menu_item::menu_item_divider())
                .with_item(menu_item::button_menu_item("Sign Out"))
                .build()),
            _ => Err(PlaygroundError::InvalidVariant { component: "menu".to_string(), variant: variant.to_string() }),
        }
    }
}
