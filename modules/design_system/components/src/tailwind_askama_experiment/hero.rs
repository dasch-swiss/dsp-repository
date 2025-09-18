use askama::Template;
use maud::{Markup, PreEscaped};

use crate::tailwind_askama_experiment::HeroConfig;

#[derive(Template)]
#[template(path = "tailwind_askama_experiment/hero.html")]
struct HeroTemplate<'a> {
    headline: &'a str,
    description: &'a str,
    announcement_text: &'a str,
    announcement_link_text: &'a str,
    announcement_href: &'a str,
    primary_button_text: &'a str,
    primary_button_href: &'a str,
    secondary_button_text: &'a str,
    secondary_button_href: &'a str,
    image_url: &'a str,
    image_alt: &'a str,
}

pub fn askama_hero(config: &HeroConfig) -> Markup {
    let template = HeroTemplate {
        headline: config.headline,
        description: config.description,
        announcement_text: config.announcement_text,
        announcement_link_text: config.announcement_link_text,
        announcement_href: config.announcement_href,
        primary_button_text: config.primary_button_text,
        primary_button_href: config.primary_button_href,
        secondary_button_text: config.secondary_button_text,
        secondary_button_href: config.secondary_button_href,
        image_url: config.image_url,
        image_alt: config.image_alt,
    };

    PreEscaped(template.render().unwrap())
}
