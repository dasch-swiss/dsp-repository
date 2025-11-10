use components::{button, icon, menu, menu_item, ComponentBuilder, IconType};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example_with_description, ComponentRenderer, ComponentSection};

/// Menu component renderer for Examples and Variants tab
pub struct MenuRenderer;

impl ComponentRenderer for MenuRenderer {
    fn render_variant_with_code(
        &self,
        variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "menu".to_string(),
                variant: variant.to_string(),
            });
        }

        // Create sample icons for examples
        let star_icon = icon::icon(IconType::Star);
        let code_icon = icon::icon(IconType::Code);
        let flag_icon = icon::icon(IconType::Flag);

        Ok(Some(vec![
            ComponentSection {
                title: "Using a verctor of menu items",
                description: Some("Build menus by passing a vector of items using with_items()"),
                examples: vec![example_with_description(
                    "menu-with-items",
                    "Menu Built with with_items()",
                    "Create a menu by passing a vector of menu items",
                    r#"// Build the items vector separately
let items = vec![
    // Navigation links
    menu_item::link_menu_item("Edit", "/edit"),
    menu_item::link_menu_item("Duplicate", "/duplicate"),

    menu_item::menu_item_divider(),

    // Action buttons with icons
    menu_item::button_menu_item_with_icon(
        "Share",
        star_icon.clone()
    ),
    menu_item::button_menu_item_with_icon(
        "Archive",
        code_icon.clone()
    ),

    menu_item::menu_item_divider(),

    // Destructive action
    menu_item::button_menu_item_with_icon(
        "Delete",
        flag_icon.clone()
    ),
];

// Create the menu with the items vector
menu::menu()
    .with_id("items-menu")
    .with_trigger(
        button::button("Actions")
            .with_id("items-menu-trigger")
            .popovertarget("items-menu")
            .build()
    )
    .with_items(items)
    .build()"#,
                    {
                        // Build the items vector separately
                        let items = vec![
                            // Navigation links
                            menu_item::link_menu_item("Edit", "/edit"),
                            menu_item::link_menu_item("Duplicate", "/duplicate"),
                            menu_item::menu_item_divider(),
                            // Action buttons with icons
                            menu_item::button_menu_item_with_icon("Share", star_icon.clone()),
                            menu_item::button_menu_item_with_icon("Archive", code_icon.clone()),
                            menu_item::menu_item_divider(),
                            // Destructive action
                            menu_item::button_menu_item_with_icon("Delete", flag_icon.clone()),
                        ];

                        // Create the menu with the items vector
                        menu::menu()
                            .with_id("items-menu")
                            .with_trigger(
                                button::button("Actions")
                                    .with_id("items-menu-trigger")
                                    .popovertarget("items-menu")
                                    .build(),
                            )
                            .with_items(items)
                            .build()
                    },
                )],
            },
            ComponentSection {
                title: "Basic Menus",
                description: Some("Menus with different trigger button styles"),
                examples: vec![
                    example_with_description(
                        "text-trigger-menu",
                        "Menu with Text Button Trigger",
                        "A menu triggered by a text button",
                        r#"menu::menu()
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
    .with_item(menu_item::link_menu_item_with_icon(
        "Add to favorites",
        "/favorites",
        star_icon.clone()
    ))
    .with_item(menu_item::link_menu_item_with_icon(
        "View source",
        "/source",
        code_icon.clone()
    ))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item_with_icon(
        "Delete",
        flag_icon.clone()
    ))
    .build()"#,
                        menu::menu()
                            .with_id("demo-menu")
                            .with_trigger(
                                button::button("Open Menu")
                                    .with_id("demo-menu-trigger")
                                    .popovertarget("demo-menu")
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
                            .with_item(menu_item::button_menu_item_with_icon("Delete", flag_icon.clone()))
                            .build(),
                    ),
                    example_with_description(
                        "icon-trigger-menu",
                        "Menu with Icon Button Trigger",
                        "A menu triggered by an icon button (hamburger menu)",
                        r#"menu::menu()
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
    .build()"#,
                        menu::menu()
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
                            .build(),
                    ),
                ],
            },
        ]))
    }
}
