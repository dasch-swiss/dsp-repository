use components::footer;
use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Footer component renderer for Component Store
pub struct FooterComponentRenderer;

impl ComponentRenderer for FooterComponentRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let config = footer::FooterConfig {
            company_name: "DaSCH",
            description: "Digital infrastructure for humanities research data preservation and discovery.",
            copyright_text: "© 2024 DaSCH, University of Basel. All rights reserved.",
            logo_light_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600",
            logo_dark_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500",
        };

        let markup = footer::footer(&config);
        Ok(markup)
    }
}
