use components::button::{self, ButtonVariant};
use components::{icon, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Button component renderer
pub struct ButtonRenderer;

impl ComponentRenderer for ButtonRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "button".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                div class="mb-4 p-4 bg-blue-50 dark:bg-blue-950 rounded-md" {
                    p class="text-sm text-blue-900 dark:text-blue-100" {
                        "💡 All buttons include DataStar onclick handlers. Open browser console to see click events."
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Variants" }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary - Main call-to-action" }
                            (button::button("Primary Button")
                                .with_id("primary-button")
                                .onclick("console.log('Primary button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary - Alternative action" }
                            (button::button("Secondary Button")
                                .with_id("secondary-button")
                                .variant(ButtonVariant::Secondary)
                                .onclick("console.log('Secondary button clicked!')")
                                .build())
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Icon Buttons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icon-only buttons for compact actions like closing dialogs or opening menus"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Default Icon Buttons" }
                            p class="text-xs text-gray-500 dark:text-gray-500 mb-2" { "Default icon buttons use subtle gray colors (text-gray-900 dark:text-gray-300)" }
                            div class="flex items-center gap-4" {
                                (button::icon_button(icon::icon(IconType::Hamburger))
                                    .with_id("hamburger-button")
                                    .onclick("console.log('Hamburger icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Close))
                                    .with_id("close-button")
                                    .onclick("console.log('Close icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::ChevronDown))
                                    .with_id("chevron-button")
                                    .onclick("console.log('ChevronDown icon clicked!')")
                                    .build())
                            }
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Icon Buttons with Custom Colors" }
                            p class="text-xs text-gray-500 dark:text-gray-500 mb-2" { "Use .color() to override with custom Tailwind color classes" }
                            div class="flex items-center gap-4" {
                                (button::icon_button(icon::icon(IconType::Star))
                                    .with_id("star-button")
                                    .color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
                                    .onclick("console.log('Star icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Code))
                                    .with_id("code-button")
                                    .color("text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-950")
                                    .onclick("console.log('Code icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Flag))
                                    .with_id("flag-button")
                                    .color("text-red-500 hover:bg-red-50 dark:hover:bg-red-950")
                                    .onclick("console.log('Flag icon clicked!')")
                                    .build())
                            }
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Disabled States" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Disabled buttons also implement onclick but do not trigger onclick events"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary Disabled" }
                            (button::button("Disabled Primary")
                                .with_id("disabled-primary")
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary Disabled" }
                            (button::button("Disabled Secondary")
                                .with_id("disabled-secondary")
                                .variant(ButtonVariant::Secondary)
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Icon Button Disabled" }
                            (button::icon_button(icon::icon(IconType::Close))
                                .with_id("disabled-icon-button")
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Buttons with Icons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Add leading or trailing icons to standard buttons for enhanced visual communication"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Leading Icon" }
                            (button::button("Download")
                                .with_id("download-button")
                                .with_leading_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Download button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Trailing Icon" }
                            (button::button("Next")
                                .with_id("next-button")
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Next button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Both Icons" }
                            (button::button("Code")
                                .with_id("code-action-button")
                                .with_leading_icon(icon::icon(IconType::Code))
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Code button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary with Icon" }
                            (button::button("Star")
                                .with_id("star-action-button")
                                .variant(ButtonVariant::Secondary)
                                .with_leading_icon(icon::icon(IconType::Star))
                                .onclick("console.log('Star button clicked!')")
                                .build())
                        }
                    }
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

// Example of using the macro for future code-view feature:
// This shows how you would structure examples when migrating to the
// ComponentSection-based approach:
//
// fn _button_variants_section() -> ComponentSection {
//     ComponentSection {
//         title: "Button Variants",
//         description: None,
//         examples: vec![
//             example!{
//                 id: "primary-button",
//                 name: "Primary Button",
//                 description: "Main call-to-action",
//                 code: {
//                     button::button("Primary Button")
//                         .with_id("primary-button")
//                         .onclick("console.log('Primary button clicked!')")
//                         .build()
//                 }
//             },
//             example!{
//                 id: "secondary-button",
//                 name: "Secondary Button",
//                 description: "Alternative action",
//                 code: {
//                     button::button("Secondary Button")
//                         .with_id("secondary-button")
//                         .variant(ButtonVariant::Secondary)
//                         .onclick("console.log('Secondary button clicked!')")
//                         .build()
//                 }
//             },
//         ],
//     }
// }
