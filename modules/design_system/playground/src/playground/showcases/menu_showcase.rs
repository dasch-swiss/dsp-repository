use components::{button, icon, menu, menu_item, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Menu component renderer
pub struct MenuRenderer;

impl ComponentRenderer for MenuRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "menu".to_string(),
                variant: variant.to_string(),
            });
        }

        // Create sample icons for demonstration
        let star_icon = icon::icon(IconType::Star);
        let code_icon = icon::icon(IconType::Code);
        let flag_icon = icon::icon(IconType::Flag);

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu with Text Button Trigger" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "A complete menu showcasing links, buttons, icons, and dividers. The menu automatically positions itself optimally based on available screen space."
                    }
                    (menu::menu()
                        .with_id("demo-menu")
                        .with_trigger(
                            button::button("Open Menu")
                                .with_id("demo-menu-trigger")
                                .popovertarget("demo-menu")
                                .build()
                        )
                        .with_item(menu_item::link_menu_item("Profile", "/profile"))
                        .with_item(menu_item::link_menu_item("Settings", "/settings"))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", star_icon.clone()))
                        .with_item(menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                        .with_item(menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item_with_icon("Delete", flag_icon.clone()))
                        .build())
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu with Icon Button Trigger" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Using an icon button trigger for a compact UI. Icon buttons are keyboard accessible and semantically correct."
                    }
                    (menu::menu()
                        .with_id("icon-menu")
                        .with_trigger(
                            button::icon_button(icon::icon(IconType::Hamburger))
                                .with_id("icon-menu-trigger")
                                .popovertarget("icon-menu")
                                .build()
                        )
                        .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
                        .with_item(menu_item::link_menu_item("Profile", "/profile"))
                        .with_item(menu_item::link_menu_item("Settings", "/settings"))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item("Sign Out"))
                        .build())
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Programmatic Trigger" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Menus can be triggered programmatically via DataStar onclick handlers or JavaScript"
                    }
                    (button::button("Open Programmatically")
                        .with_id("programmatic-trigger")
                        .onclick("document.getElementById('programmatic-menu').showPopover()")
                        .build())
                    (menu::menu()
                        .with_id("programmatic-menu")
                        .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
                        .with_item(menu_item::link_menu_item("Analytics", "/analytics"))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item("Refresh"))
                        .build())
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
