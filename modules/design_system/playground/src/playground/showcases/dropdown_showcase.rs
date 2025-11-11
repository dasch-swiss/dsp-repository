use components::{dropdown, icon, menu, menu_item, IconType};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example_with_description, ComponentRenderer, ComponentSection};

/// Dropdown component renderer for Examples and Variants tab
pub struct DropdownRenderer;

impl ComponentRenderer for DropdownRenderer {
    fn render_variant_with_code(
        &self,
        variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "dropdown".to_string(),
                variant: variant.to_string(),
            });
        }

        // Create sample icons for menu items
        let star_icon = icon::icon(IconType::Star);
        let code_icon = icon::icon(IconType::Code);

        Ok(Some(vec![ComponentSection {
            title: "Dropdown Variants",
            description: Some("Three dropdown styles for different use cases"),
            examples: vec![
                example_with_description(
                    "dropdown-secondary",
                    "Secondary Button Dropdown",
                    "Dropdown with a labeled secondary button and chevron icon",
                    r#"let items = vec![
    menu_item::link_menu_item("Edit", "/edit"),
    menu_item::link_menu_item("Duplicate", "/duplicate"),
    menu_item::menu_item_divider(),
    menu_item::link_menu_item_with_icon(
        "Add to favorites",
        "/favorites",
        star_icon.clone()
    ),
    menu_item::link_menu_item_with_icon(
        "View source",
        "/source",
        code_icon.clone()
    ),
    menu_item::menu_item_divider(),
    menu_item::button_menu_item("Delete"),
];

dropdown::dropdown_secondary(
    "actions-dropdown",
    "Options",
    menu::menu().with_items(items)
)"#,
                    {
                        let items = vec![
                            menu_item::link_menu_item("Edit", "/edit"),
                            menu_item::link_menu_item("Duplicate", "/duplicate"),
                            menu_item::menu_item_divider(),
                            menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", star_icon.clone()),
                            menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()),
                            menu_item::menu_item_divider(),
                            menu_item::button_menu_item("Delete"),
                        ];

                        dropdown::dropdown_secondary("actions-dropdown", "Options", menu::menu().with_items(items))
                    },
                ),
                example_with_description(
                    "dropdown-more-vert",
                    "MoreVert Icon Dropdown",
                    "Dropdown with three vertical dots icon for overflow menus",
                    r#"let items = vec![
    menu_item::link_menu_item("Settings", "/settings"),
    menu_item::link_menu_item("Help", "/help"),
    menu_item::menu_item_divider(),
    menu_item::button_menu_item("Sign Out"),
];

dropdown::dropdown_more_vert(
    "more-dropdown",
    menu::menu().with_items(items)
)"#,
                    {
                        let items = vec![
                            menu_item::link_menu_item("Settings", "/settings"),
                            menu_item::link_menu_item("Help", "/help"),
                            menu_item::menu_item_divider(),
                            menu_item::button_menu_item("Sign Out"),
                        ];

                        dropdown::dropdown_more_vert("more-dropdown", menu::menu().with_items(items))
                    },
                ),
                example_with_description(
                    "dropdown-hamburger",
                    "Hamburger Icon Dropdown",
                    "Dropdown with hamburger menu icon for navigation menus",
                    r#"let items = vec![
    menu_item::link_menu_item("Home", "/"),
    menu_item::link_menu_item("About", "/about"),
    menu_item::link_menu_item("Services", "/services"),
    menu_item::menu_item_divider(),
    menu_item::link_menu_item("Contact", "/contact"),
];

dropdown::dropdown_hamburger(
    "nav-dropdown",
    menu::menu().with_items(items)
)"#,
                    {
                        let items = vec![
                            menu_item::link_menu_item("Home", "/"),
                            menu_item::link_menu_item("About", "/about"),
                            menu_item::link_menu_item("Services", "/services"),
                            menu_item::menu_item_divider(),
                            menu_item::link_menu_item("Contact", "/contact"),
                        ];

                        dropdown::dropdown_hamburger("nav-dropdown", menu::menu().with_items(items))
                    },
                ),
            ],
        }]))
    }
}
