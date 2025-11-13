use components::{icon, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Icon component renderer for Component Store
pub struct IconComponentRenderer;

impl ComponentRenderer for IconComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        match variant {
            "close" => Ok(html! {
                (icon::icon(IconType::Close))
            }),
            _ => Err(PlaygroundError::InvalidVariant { component: "icon".to_string(), variant: variant.to_string() }),
        }
    }
}
