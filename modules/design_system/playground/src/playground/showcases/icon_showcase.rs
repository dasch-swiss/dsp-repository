use components::{icon, menu_item, IconType};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example, ComponentRenderer, ComponentSection};

/// Icon component renderer for Examples and Variants tab
pub struct IconRenderer;

impl ComponentRenderer for IconRenderer {
    fn render_variant_with_code(
        &self,
        variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "icon".to_string(),
                variant: variant.to_string(),
            });
        }

        Ok(Some(vec![
            ComponentSection {
                title: "Icon Sizes",
                description: Some("Icons can be sized using Tailwind size classes"),
                examples: vec![
                    example(
                        "icon-size-4",
                        "Size 4",
                        r#"icon::icon_with_class(IconType::Star, "size-4")"#,
                        icon::icon_with_class(IconType::Star, "size-4"),
                    ),
                    example(
                        "icon-size-5",
                        "Size 5 (default)",
                        r#"icon::icon_with_class(IconType::Star, "size-5")"#,
                        icon::icon_with_class(IconType::Star, "size-5"),
                    ),
                    example(
                        "icon-size-6",
                        "Size 6",
                        r#"icon::icon_with_class(IconType::Star, "size-6")"#,
                        icon::icon_with_class(IconType::Star, "size-6"),
                    ),
                    example(
                        "icon-size-8",
                        "Size 8",
                        r#"icon::icon_with_class(IconType::Star, "size-8")"#,
                        icon::icon_with_class(IconType::Star, "size-8"),
                    ),
                    example(
                        "icon-size-12",
                        "Size 12",
                        r#"icon::icon_with_class(IconType::Star, "size-12")"#,
                        icon::icon_with_class(IconType::Star, "size-12"),
                    ),
                ],
            },
            ComponentSection {
                title: "Icon Colors",
                description: Some("Icons inherit currentColor and can be styled with text color classes"),
                examples: vec![
                    example(
                        "icon-yellow",
                        "Yellow Star",
                        r#"icon::icon_with_class(IconType::Star, "size-8 text-yellow-500")"#,
                        icon::icon_with_class(IconType::Star, "size-8 text-yellow-500"),
                    ),
                    example(
                        "icon-blue",
                        "Blue Star",
                        r#"icon::icon_with_class(IconType::Star, "size-8 text-blue-500")"#,
                        icon::icon_with_class(IconType::Star, "size-8 text-blue-500"),
                    ),
                    example(
                        "icon-green",
                        "Green Star",
                        r#"icon::icon_with_class(IconType::Star, "size-8 text-green-500")"#,
                        icon::icon_with_class(IconType::Star, "size-8 text-green-500"),
                    ),
                    example(
                        "icon-red",
                        "Red Star",
                        r#"icon::icon_with_class(IconType::Star, "size-8 text-red-500")"#,
                        icon::icon_with_class(IconType::Star, "size-8 text-red-500"),
                    ),
                    example(
                        "icon-purple",
                        "Purple Star",
                        r#"icon::icon_with_class(IconType::Star, "size-8 text-purple-500")"#,
                        icon::icon_with_class(IconType::Star, "size-8 text-purple-500"),
                    ),
                ],
            },
            ComponentSection {
                title: "Icons in Menu Components",
                description: Some(
                    "Some components like the menu component or the button component implement icons. In general there is no need to define a color or a size as it is inherited or already defined by the component. Consult the docs of those components on how to pass or build the desired icon.",
                ),
                examples: vec![
                    example(
                        "menu-with-star",
                        "Menu Item with Star Icon",
                        r#"menu_item::link_menu_item_with_icon(
    "Favorites",
    "/favorites",
    icon::icon(IconType::Star)
)"#,
                        menu_item::link_menu_item_with_icon(
                            "Favorites",
                            "/favorites",
                            icon::icon(IconType::Star),
                        ),
                    ),
                    example(
                        "menu-with-code",
                        "Menu Item with Code Icon",
                        r#"menu_item::link_menu_item_with_icon(
    "View Source",
    "/source",
    icon::icon(IconType::Code)
)"#,
                        menu_item::link_menu_item_with_icon(
                            "View Source",
                            "/source",
                            icon::icon(IconType::Code),
                        ),
                    ),
                    example(
                        "menu-with-flag",
                        "Menu Item with Flag Icon",
                        r#"menu_item::link_menu_item_with_icon(
    "Report",
    "/report",
    icon::icon(IconType::Flag)
)"#,
                        menu_item::link_menu_item_with_icon(
                            "Report",
                            "/report",
                            icon::icon(IconType::Flag),
                        ),
                    ),
                ],
            },
            ComponentSection {
                title: "Available Icons",
                description: Some("All available icons including Heroicons and social media icons"),
                examples: vec![
                    example(
                        "icon-star",
                        "Star",
                        r#"icon::icon(IconType::Star)"#,
                        icon::icon(IconType::Star),
                    ),
                    example(
                        "icon-code",
                        "Code",
                        r#"icon::icon(IconType::Code)"#,
                        icon::icon(IconType::Code),
                    ),
                    example(
                        "icon-flag",
                        "Flag",
                        r#"icon::icon(IconType::Flag)"#,
                        icon::icon(IconType::Flag),
                    ),
                    example(
                        "icon-hamburger",
                        "Hamburger",
                        r#"icon::icon(IconType::Hamburger)"#,
                        icon::icon(IconType::Hamburger),
                    ),
                    example(
                        "icon-close",
                        "Close",
                        r#"icon::icon(IconType::Close)"#,
                        icon::icon(IconType::Close),
                    ),
                    example(
                        "icon-chevron-down",
                        "ChevronDown",
                        r#"icon::icon(IconType::ChevronDown)"#,
                        icon::icon(IconType::ChevronDown),
                    ),
                    example(
                        "icon-facebook",
                        "Facebook",
                        r#"icon::icon(IconType::Facebook)"#,
                        icon::icon(IconType::Facebook),
                    ),
                    example(
                        "icon-instagram",
                        "Instagram",
                        r#"icon::icon(IconType::Instagram)"#,
                        icon::icon(IconType::Instagram),
                    ),
                    example(
                        "icon-x",
                        "X (Twitter)",
                        r#"icon::icon(IconType::X)"#,
                        icon::icon(IconType::X),
                    ),
                    example(
                        "icon-github",
                        "GitHub",
                        r#"icon::icon(IconType::GitHub)"#,
                        icon::icon(IconType::GitHub),
                    ),
                    example(
                        "icon-youtube",
                        "YouTube",
                        r#"icon::icon(IconType::YouTube)"#,
                        icon::icon(IconType::YouTube),
                    ),
                ],
            },
        ]))
    }
}
