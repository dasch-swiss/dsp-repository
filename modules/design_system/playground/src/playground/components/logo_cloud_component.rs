use components::logo_cloud::{logo_cloud, Logo};
use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// LogoCloud component renderer for Component Store
pub struct LogoCloudComponentRenderer;

impl ComponentRenderer for LogoCloudComponentRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Reference example with 5 logos matching the Tailwind reference
        let logos = vec![
            Logo::new(
                "https://tailwindcss.com/plus-assets/img/logos/158x48/transistor-logo-gray-900.svg",
                "Transistor",
                158,
                48,
            ),
            Logo::new(
                "https://tailwindcss.com/plus-assets/img/logos/158x48/reform-logo-gray-900.svg",
                "Reform",
                158,
                48,
            ),
            Logo::new(
                "https://tailwindcss.com/plus-assets/img/logos/158x48/tuple-logo-gray-900.svg",
                "Tuple",
                158,
                48,
            ),
            Logo::new(
                "https://tailwindcss.com/plus-assets/img/logos/158x48/savvycal-logo-gray-900.svg",
                "SavvyCal",
                158,
                48,
            ),
            Logo::new(
                "https://tailwindcss.com/plus-assets/img/logos/158x48/statamic-logo-gray-900.svg",
                "Statamic",
                158,
                48,
            ),
        ];

        let markup = logo_cloud("Trusted by the world's most innovative teams", logos);
        Ok(markup)
    }
}
