use maud::{html, Markup};

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// shell component renderer for Examples and Variants tab
pub struct ShellRenderer;

impl ComponentRenderer for ShellRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        Ok(html! {
            div class="p-8" {
                p class="text-gray-600 dark:text-gray-400" {
                    "Examples and variants for this component will be added in the future."
                }
            }
        })
    }
}
