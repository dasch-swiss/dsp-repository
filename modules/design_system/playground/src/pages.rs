use askama::Template;
use axum::response::Html;
use components::{banner, button};
use maud::{html, PreEscaped};

use crate::skeleton::PlaygroundSkeleton;

pub fn home() -> Html<String> {
    let title = "Playground - Home";
    let body = html!(
        div {
            h1 { "DSP Design System Playground" }
            p { "Components and examples" }
            h2 { "Components" }
            ul {
                li { a href="/button" { "Button" } }
                li { a href="/banner" { "Banner" } }
                li { a href="/shell" { "Shell" } }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn button() -> Html<String> {
    let title = "Playground - Button";
    let body = html!(
        div {
            h2 { "Button" }
            p { "This is the button component" }
            div {(PreEscaped(button::button()))}
            // TODO: add variants
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn banner() -> Html<String> {
    let title = "Playground - Banner";
    let banner = banner::banner(
        None,
        "DaSCH".to_string(),
        Some("Swiss National Data and Service Center for the Humanities".to_string()),
    );
    let banner_all = banner::banner(
        Some("We are".to_string()),
        "DaSCH".to_string(),
        Some("The Swiss National Data and Service Center for the Humanities".to_string()),
    );
    let body = html!(
        div {
            h2 { "Banner" }
            p { "This is the banner component" }
            h3 { "Accent Only" }
            p { "This is the banner component with accent only" }
            div {(PreEscaped(banner::banner(None, "DaSCH".to_string(), None)))}
            h3 { "Prefix and Accent" }
            p { "This is the banner component with prefix and accent" }
            div {(PreEscaped(banner::banner(Some("We are".to_string()), "DaSCH".to_string(), None)))}
            h3 { "Accent and Suffix" }
            p { "This is the banner component with accent and suffix" }
            p { "This is what we will use on the website" }
            div {(PreEscaped(banner))}
            h3 { "Prefix, Accent and Suffix" }
            p { "This is the banner component with prefix, accent and suffix" }
            div {(PreEscaped(banner_all))}
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn shell() -> Html<String> {
    let title = "Playground - Shell";
    let body = html!(
        div {
            h2 { "Shell" }
            p { "This is the shell component" }
            div {(PreEscaped(components::shell::shell()))}
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}
