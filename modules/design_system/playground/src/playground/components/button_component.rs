use components::button::{self, ButtonVariant};
use components::{icon, ComponentBuilder, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Button component renderer for Component Store (isolated component variants)
pub struct ButtonComponentRenderer;

impl ComponentRenderer for ButtonComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        match variant {
            "primary" => Ok(html! {
                (button::button("Sample Button")
                    .with_id("primary-button")
                    .with_test_id("button-primary")
                    .onclick("console.log('Primary button clicked!')")
                    .build())
            }),
            "secondary" => Ok(html! {
                (button::button("Secondary Button")
                    .with_id("secondary-button")
                    .variant(ButtonVariant::Secondary)
                    .onclick("console.log('Secondary button clicked!')")
                    .build())

            }),
            "icon-hamburger" => Ok(html! {
                (button::icon_button(icon::icon(IconType::Hamburger))
                    .with_id("hamburger-button")
                    .onclick("console.log('Hamburger icon clicked!')")
                    .build())
            }),
            "icon-close" => Ok(html! {
                (button::icon_button(icon::icon(IconType::Close))
                    .with_id("close-button")
                    .onclick("console.log('Close icon clicked!')")
                    .build())

            }),
            "disabled-primary" => Ok(html! {
                (button::button("Disabled Primary")
                    .with_id("disabled-primary")
                    .disabled()
                    .onclick("console.log('This should not fire!')")
                    .build())

            }),
            _ => Err(PlaygroundError::InvalidVariant {
                component: "button".to_string(),
                variant: variant.to_string(),
            }),
        }
    }
}
