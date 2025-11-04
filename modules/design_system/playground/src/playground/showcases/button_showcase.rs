use components::button::{self, ButtonVariant};
use components::{icon, ComponentBuilder, IconType};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example, ComponentRenderer, ComponentSection};

/// Button component renderer
pub struct ButtonRenderer;

impl ComponentRenderer for ButtonRenderer {
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
                    example(
                        "primary-button",
                        "Primary Button",
                        r#"button::button("Sample Button")
    .with_id("primary-button")
    .with_test_id("button-primary")
    .onclick("console.log('Primary button clicked!')")
    .build()"#,
                        button::button("Sample Button")
                            .with_id("primary-button")
                            .with_test_id("button-primary")
                            .onclick("console.log('Primary button clicked!')")
                            .build(),
                    ),
                    example(
                        "secondary-button",
                        "Secondary Button",
                        r#"button::button("Secondary Button")
    .with_id("secondary-button")
    .variant(ButtonVariant::Secondary)
    .onclick("console.log('Secondary button clicked!')")
    .build()"#,
                        button::button("Secondary Button")
                            .with_id("secondary-button")
                            .variant(ButtonVariant::Secondary)
                            .onclick("console.log('Secondary button clicked!')")
                            .build(),
                    ),
                ],
            },
            ComponentSection {
                title: "Icon Buttons",
                description: Some("Icon-only buttons for compact actions like closing dialogs or opening menus"),
                examples: vec![
                    example(
                        "icon-hamburger",
                        "Hamburger Icon Button",
                        r#"button::icon_button(icon::icon(IconType::Hamburger))
    .with_id("hamburger-button")
    .onclick("console.log('Hamburger icon clicked!')")
    .build()"#,
                        button::icon_button(icon::icon(IconType::Hamburger))
                            .with_id("hamburger-button")
                            .onclick("console.log('Hamburger icon clicked!')")
                            .build(),
                    ),
                    example(
                        "icon-close",
                        "Close Icon Button",
                        r#"button::icon_button(icon::icon(IconType::Close))
    .with_id("close-button")
    .onclick("console.log('Close icon clicked!')")
    .build()"#,
                        button::icon_button(icon::icon(IconType::Close))
                            .with_id("close-button")
                            .onclick("console.log('Close icon clicked!')")
                            .build(),
                    ),
                    example(
                        "icon-star-colored",
                        "Colored Star Icon Button",
                        r#"button::icon_button(icon::icon(IconType::Star))
    .with_id("star-button")
    .with_color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
    .onclick("console.log('Star icon clicked!')")
    .build()"#,
                        button::icon_button(icon::icon(IconType::Star))
                            .with_id("star-button")
                            .with_color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
                            .onclick("console.log('Star icon clicked!')")
                            .build(),
                    ),
                ],
            },
            ComponentSection {
                title: "Disabled States",
                description: Some("Disabled buttons do not trigger onclick events"),
                examples: vec![
                    example(
                        "disabled-primary",
                        "Disabled Primary",
                        r#"button::button("Disabled Primary")
    .with_id("disabled-primary")
    .disabled()
    .onclick("console.log('This should not fire!')")
    .build()"#,
                        button::button("Disabled Primary")
                            .with_id("disabled-primary")
                            .disabled()
                            .onclick("console.log('This should not fire!')")
                            .build(),
                    ),
                    example(
                        "disabled-secondary",
                        "Disabled Secondary",
                        r#"button::button("Disabled Secondary")
    .with_id("disabled-secondary")
    .variant(ButtonVariant::Secondary)
    .disabled()
    .onclick("console.log('This should not fire!')")
    .build()"#,
                        button::button("Disabled Secondary")
                            .with_id("disabled-secondary")
                            .variant(ButtonVariant::Secondary)
                            .disabled()
                            .onclick("console.log('This should not fire!')")
                            .build(),
                    ),
                ],
            },
            ComponentSection {
                title: "Buttons with Icons",
                description: Some("Add leading or trailing icons for enhanced visual communication"),
                examples: vec![
                    example(
                        "button-leading-icon",
                        "Button with Leading Icon",
                        r#"button::button("Download")
    .with_id("download-button")
    .with_leading_icon(icon::icon(IconType::ChevronDown))
    .onclick("console.log('Download button clicked!')")
    .build()"#,
                        button::button("Download")
                            .with_id("download-button")
                            .with_leading_icon(icon::icon(IconType::ChevronDown))
                            .onclick("console.log('Download button clicked!')")
                            .build(),
                    ),
                    example(
                        "button-trailing-icon",
                        "Button with Trailing Icon",
                        r#"button::button("Next")
    .with_id("next-button")
    .with_trailing_icon(icon::icon(IconType::ChevronDown))
    .onclick("console.log('Next button clicked!')")
    .build()"#,
                        button::button("Next")
                            .with_id("next-button")
                            .with_trailing_icon(icon::icon(IconType::ChevronDown))
                            .onclick("console.log('Next button clicked!')")
                            .build(),
                    ),
                    example(
                        "button-both-icons",
                        "Button with Both Icons",
                        r#"button::button("Code")
    .with_id("code-action-button")
    .with_leading_icon(icon::icon(IconType::Code))
    .with_trailing_icon(icon::icon(IconType::ChevronDown))
    .onclick("console.log('Code button clicked!')")
    .build()"#,
                        button::button("Code")
                            .with_id("code-action-button")
                            .with_leading_icon(icon::icon(IconType::Code))
                            .with_trailing_icon(icon::icon(IconType::ChevronDown))
                            .onclick("console.log('Code button clicked!')")
                            .build(),
                    ),
                ],
            },
        ]))
    }
}
