use askama::Template;
use axum::response::Html;
use components::{banner, button, link, shell, tag, tile};
use maud::html;

use crate::skeleton::PlaygroundSkeleton;

pub fn home() -> Html<String> {
    let title = "Playground - Home";
    let body = html!(
        div {
            h1 class="playground-page-title" { "DSP Design System Playground" }
            p class="playground-section__description" { "Components and examples" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Components" }
                div class="playground-section__example" {
                    ul {
                        li { a href="/button" { "Button" } }
                        li { a href="/banner" { "Banner" } }
                        li { a href="/link" { "Link" } }
                        li { a href="/shell" { "Shell" } }
                        li { a href="/tag" { "Tag" } }
                        li { a href="/tile" { "Tile" } }
                    }
                }
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
            h1 class="playground-page-title" { "Button" }
            p class="playground-section__description" { "This is the button component" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Basic Button" }
                p class="playground-section__description" { "Default button component" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (button::button("Click me!"))
                }
            }

            // TODO: add variants
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn banner() -> Html<String> {
    let title = "Playground - Banner";
    let banner = banner::with_suffix("DaSCH", "Swiss National Data and Service Center for the Humanities");
    let banner_all = banner::full(
        "We are",
        "DaSCH",
        "The Swiss National Data and Service Center for the Humanities",
    );
    let body = html!(
        div {
            h1 class="playground-page-title" { "Banner" }
            p class="playground-section__description" { "This is the banner component" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Accent Only" }
                p class="playground-section__description" { "This is the banner component with accent only" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (banner::accent_only("DaSCH"))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Prefix and Accent" }
                p class="playground-section__description" { "This is the banner component with prefix and accent" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (banner::with_prefix("We are", "DaSCH"))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Accent and Suffix" }
                p class="playground-section__description" { "This is the banner component with accent and suffix" }
                p class="playground-section__description" { "This is what we will use on the website" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (banner)
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Prefix, Accent and Suffix" }
                p class="playground-section__description" { "This is the banner component with prefix, accent and suffix" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (banner_all)
                }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn shell() -> Html<String> {
    let title = "Playground - Shell";
    let body = html!(
        div {
            h1 class="playground-page-title" { "Shell" }
            p class="playground-section__description" { "This is the shell component" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Application Shell" }
                p class="playground-section__description" { "Navigation and layout wrapper" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (shell::shell())
                }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn tile() -> Html<String> {
    let title = "Playground - Tile";
    let body = html!(
        div {
            h1 class="playground-page-title" { "Tile" }
            p class="playground-section__description" { "This is the tile component with two variants" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Base Tile" }
                p class="playground-section__description" { "Basic tile for displaying information" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (tile::base(html! {
                        h4 { "Feature Title" }
                        p { "This is a base tile with some content. It can contain any markup." }
                    }))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Clickable Tile" }
                p class="playground-section__description" { "Tile that acts as a navigation link" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (tile::clickable("/", html! {
                        h4 { "Go to Home" }
                        p { "Click anywhere on this tile to navigate to the home page." }
                    }))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Composition Example" }
                p class="playground-section__description" { "Tiles can be composed with other components" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (tile::base(html! {
                        p { "A tile containing multiple button components" }
                        div {
                            (button::button("Primary"))
                            " "
                            (button::button_with_variant("Secondary", components::ButtonVariant::Secondary, false))
                        }
                    }))
                }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn tag() -> Html<String> {
    let title = "Playground - Tag";
    let body = html!(
        div {
            h1 class="playground-page-title" { "Tag" }
            p class="playground-section__description" { "Carbon Design System tag component wrapper" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Basic Tag" }
                p class="playground-section__description" { "Default gray tag component" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (tag::tag("Default"))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Tag Variants" }
                p class="playground-section__description" { "Different tag colors" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (tag::tag_with_variant("Gray", components::TagVariant::Gray))
                    " "
                    (tag::tag_with_variant("Blue", components::TagVariant::Blue))
                    " "
                    (tag::tag_with_variant("Green", components::TagVariant::Green))
                }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}

pub fn link() -> Html<String> {
    let title = "Playground - Link";
    let body = html!(
        div {
            h1 class="playground-page-title" { "Link" }
            p class="playground-section__description" { "Carbon Design System link component wrapper" }

            section class="playground-section" {
                h2 class="playground-section__title" { "Basic Link" }
                p class="playground-section__description" { "Default link component" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (link::link("Visit Carbon Design System", "https://carbondesignsystem.com"))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Internal Link" }
                p class="playground-section__description" { "Link to other pages in the application" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (link::link("Go to Button Component", "/button"))
                }
            }

            section class="playground-section" {
                h2 class="playground-section__title" { "Custom Test ID" }
                p class="playground-section__description" { "Link with custom test identifier" }
                div class="playground-section__example" {
                    div class="playground-section__example-title" { "Example" }
                    (link::link_with_testid("Custom Link", "/tag", Some("custom-test-link")))
                }
            }
        }
    );
    let scaffold = PlaygroundSkeleton::new(title.to_string(), body.into_string()).render().unwrap();
    Html(scaffold)
}
