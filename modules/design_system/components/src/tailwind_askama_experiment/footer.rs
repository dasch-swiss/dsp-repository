use askama::Template;
use maud::{Markup, PreEscaped};

use crate::tailwind_askama_experiment::FooterConfig;

#[derive(Template)]
#[template(path = "tailwind_askama_experiment/footer.html")]
struct FooterTemplate<'a> {
    company_name: &'a str,
    description: &'a str,
    copyright_text: &'a str,
    logo_light_url: &'a str,
    logo_dark_url: &'a str,
}

pub fn askama_footer(config: &FooterConfig) -> Markup {
    let template = FooterTemplate {
        company_name: config.company_name,
        description: config.description,
        copyright_text: config.copyright_text,
        logo_light_url: config.logo_light_url,
        logo_dark_url: config.logo_dark_url,
    };

    PreEscaped(template.render().unwrap())
}
