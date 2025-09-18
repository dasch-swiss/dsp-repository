use askama::Template;
use maud::{Markup, PreEscaped};

use crate::tailwind_askama_experiment::{HeaderConfig, NavElement};

#[derive(Template)]
#[template(path = "tailwind_askama_experiment/header.html")]
struct HeaderTemplate<'a> {
    company_name: &'a str,
    logo_light_url: &'a str,
    logo_dark_url: &'a str,
    login_href: &'a str,
    nav_elements: &'a Vec<NavElement>,
}

pub fn askama_header(nav_elements: Vec<NavElement>, config: &HeaderConfig) -> Markup {
    let template = HeaderTemplate {
        company_name: config.company_name,
        logo_light_url: config.logo_light_url,
        logo_dark_url: config.logo_dark_url,
        login_href: config.login_href,
        nav_elements: &nav_elements,
    };

    PreEscaped(template.render().unwrap())
}
