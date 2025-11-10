use components::{icon, menu_item, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Menu Item component renderer for Component Store
pub struct MenuItemComponentRenderer;

impl ComponentRenderer for MenuItemComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "menu-item".to_string(),
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
                    h3 class="text-lg font-semibold mb-3" { "Link Menu Items" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item("View Profile", "#"))
                            (menu_item::link_menu_item("Settings", "#"))
                            (menu_item::link_menu_item("Help Center", "#"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Link Menu Items with Icons" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("Add to favorites", "#", star_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Embed", "#", code_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Report content", "#", flag_icon.clone()))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Menu Items" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::button_menu_item("Delete"))
                            (menu_item::button_menu_item("Archive"))
                            (menu_item::button_menu_item("Mark as read"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Menu Items with Icons" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Copy link", flag_icon.clone()))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu Item Divider" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Use dividers to separate groups of menu items"
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item("Item 1", "#"))
                            (menu_item::link_menu_item("Item 2", "#"))
                            (menu_item::menu_item_divider())
                            (menu_item::link_menu_item("Item 3", "#"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Mixed Menu (Links and Buttons)" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "A typical menu combining navigation links and action buttons"
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("View Details", "/details", star_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Edit", "/edit", code_icon.clone()))
                            (menu_item::menu_item_divider())
                            (menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                            (menu_item::menu_item_divider())
                            (menu_item::button_menu_item_with_icon("Delete", flag_icon))
                        }
                    }
                }
            }
        };

        Ok(markup)
    }
}
