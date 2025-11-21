use components::logo_cloud::{logo_cloud, Logo};
use maud::{html, Markup};

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example, ComponentRenderer, ComponentSection};

/// logo_cloud component renderer for Examples and Variants tab
pub struct LogoCloudRenderer;

impl ComponentRenderer for LogoCloudRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Fallback rendering if code-view is not used
        Ok(html! {
            div class="p-8 flex flex-col gap-12" {
                (render_default_example())
                (render_single_logo_example())
                (render_ten_logos_example())
            }
        })
    }

    fn render_variant_with_code(
        &self,
        _variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        let sections = vec![ComponentSection {
            title: "Logo Cloud Examples",
            description: Some("Display a collection of partner or client logos in a responsive grid"),
            examples: vec![
                example(
                    "default",
                    "Default (5 logos)",
                    r#"logo_cloud(
    "Trusted by the world's most innovative teams",
    vec![
        Logo::new("https://tailwindcss.com/.../transistor-logo.svg", "Transistor", 158, 48),
        Logo::new("https://tailwindcss.com/.../reform-logo.svg", "Reform", 158, 48),
        Logo::new("https://tailwindcss.com/.../tuple-logo.svg", "Tuple", 158, 48),
        Logo::new("https://tailwindcss.com/.../savvycal-logo.svg", "SavvyCal", 158, 48),
        Logo::new("https://tailwindcss.com/.../statamic-logo.svg", "Statamic", 158, 48),
    ]
)"#,
                    render_default_example(),
                ),
                example(
                    "single",
                    "Single logo",
                    r#"logo_cloud(
    "Powered by",
    vec![
        Logo::new("https://tailwindcss.com/.../transistor-logo.svg", "Transistor", 158, 48),
    ]
)"#,
                    render_single_logo_example(),
                ),
                example(
                    "many",
                    "Many logos (10)",
                    r#"logo_cloud(
    "Trusted by leading organizations",
    vec![
        Logo::new("https://tailwindcss.com/.../transistor-logo.svg", "Transistor", 158, 48),
        // ... 8 more logos ...
        Logo::new("https://tailwindcss.com/.../statamic-logo.svg", "Statamic", 158, 48),
    ]
)"#,
                    render_ten_logos_example(),
                ),
            ],
        }];

        Ok(Some(sections))
    }
}

fn render_default_example() -> Markup {
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

    logo_cloud("Trusted by the world's most innovative teams", logos)
}

fn render_single_logo_example() -> Markup {
    let logos = vec![Logo::new(
        "https://tailwindcss.com/plus-assets/img/logos/158x48/transistor-logo-gray-900.svg",
        "Transistor",
        158,
        48,
    )];

    logo_cloud("Powered by", logos)
}

fn render_ten_logos_example() -> Markup {
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

    logo_cloud("Trusted by leading organizations", logos)
}
