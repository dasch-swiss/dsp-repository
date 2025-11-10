use components::button::{self, ButtonVariant};
use components::{icon, ComponentBuilder, IconType};
use maud::{html, Markup};

use crate::example;
use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{ComponentRenderer, ComponentSection};

/// Button component renderer
pub struct ButtonRenderer;

impl ComponentRenderer for ButtonRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // This method is kept for backward compatibility but is no longer used.
        // The button showcase uses render_variant_with_code() instead.
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "button".to_string(),
                variant: variant.to_string(),
            });
        }

        Ok(html! {
            div class="p-8" {
                p class="text-gray-600" { "This view is deprecated. Use code-view toggle instead." }
            }
        })
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }
    fn render_variant_with_code(
        &self,
        variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "button".to_string(),
                variant: variant.to_string(),
            });
        }

        Ok(Some(vec![
            ComponentSection {
                title: "Main button Variants",
                description: None,
                examples: vec![
                    example! {
                        id: "primary-button",
                        name: "Primary Button",
                        code: {
                            button::button("Primary Button")
                                .with_id("primary-button")
                                .onclick("console.log('Primary button clicked!')")
                                .build()
                        }
                    },
                    example! {
                        id: "secondary-button",
                        name: "Secondary Button",
                        code: {
                            button::button("Secondary Button")
                                .with_id("secondary-button")
                                .variant(ButtonVariant::Secondary)
                                .onclick("console.log('Secondary button clicked!')")
                                .build()
                        }
                    },
                ],
            },
            ComponentSection {
                title: "Icon Buttons",
                description: Some("Icon-only buttons for compact actions like closing dialogs or opening menus"),
                examples: vec![
                    example! {
                        id: "icon-hamburger",
                        name: "Hamburger Icon Button",
                        code: {
                            button::icon_button(icon::icon(IconType::Hamburger))
                                .with_id("hamburger-button")
                                .onclick("console.log('Hamburger icon clicked!')")
                                .build()
                        }
                    },
                    example! {
                        id: "icon-close",
                        name: "Close Icon Button",
                        code: {
                            button::icon_button(icon::icon(IconType::Close))
                                .with_id("close-button")
                                .onclick("console.log('Close icon clicked!')")
                                .build()
                        }
                    },
                    example! {
                        id: "icon-star-colored",
                        name: "Colored Star Icon Button",
                        code: {
                            button::icon_button(icon::icon(IconType::Star))
                                .with_id("star-button")
                                .with_color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
                                .onclick("console.log('Star icon clicked!')")
                                .build()
                        }
                    },
                ],
            },
            ComponentSection {
                title: "Disabled States",
                description: Some("Disabled buttons do not trigger onclick events"),
                examples: vec![
                    example! {
                        id: "disabled-primary",
                        name: "Disabled Primary",
                        code: {
                            button::button("Disabled Primary")
                                .with_id("disabled-primary")
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build()
                        }
                    },
                    example! {
                        id: "disabled-secondary",
                        name: "Disabled Secondary",
                        code: {
                            button::button("Disabled Secondary")
                                .with_id("disabled-secondary")
                                .variant(ButtonVariant::Secondary)
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build()
                        }
                    },
                ],
            },
            ComponentSection {
                title: "Buttons with Icons",
                description: Some("Add leading or trailing icons for enhanced visual communication"),
                examples: vec![
                    example! {
                        id: "button-leading-icon",
                        name: "Button with Leading Icon",
                        code: {
                            button::button("Download")
                                .with_id("download-button")
                                .with_leading_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Download button clicked!')")
                                .build()
                        }
                    },
                    example! {
                        id: "button-trailing-icon",
                        name: "Button with Trailing Icon",
                        code: {
                            button::button("Next")
                                .with_id("next-button")
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Next button clicked!')")
                                .build()
                        }
                    },
                    example! {
                        id: "button-both-icons",
                        name: "Button with Both Icons",
                        code: {
                            button::button("Code")
                                .with_id("code-action-button")
                                .with_leading_icon(icon::icon(IconType::Code))
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Code button clicked!')")
                                .build()
                        }
                    },
                ],
            },
        ]))
    }
}
