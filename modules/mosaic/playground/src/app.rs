//! Router, document shell, navigation, and home page for the playground.
//!
//! Each component showcase is its own URL-addressable page (MPA, no client-side
//! routing). The active nav item is resolved server-side from the route path.

use std::path::PathBuf;

use axum::response::Html;
use axum::routing::get;
use axum::Router;
use maud::{html, Markup, DOCTYPE};
use tower_http::services::ServeDir;

use crate::showcase;

/// Sidebar entries under the "Components" heading: (path, label).
const COMPONENT_NAV: &[(&str, &str)] = &[
    ("/badge", "Badge"),
    ("/breadcrumb", "Breadcrumb"),
    ("/button", "Button"),
    ("/card", "Card"),
    ("/icon", "Icon"),
    ("/link", "Link"),
    ("/tabs", "Tabs"),
];

/// Build the Axum router. Static assets (logos, the compiled stylesheet) are
/// served from `public_dir`; everything else 404s through `ServeDir`.
pub fn router(public_dir: PathBuf) -> Router {
    Router::new()
        .route("/", get(|| async { render("/", "Mosaic Component Library", home()) }))
        .route(
            "/theme",
            get(|| async { render("/theme", "Design Tokens", showcase::theme::page()) }),
        )
        .route("/badge", get(|| async { render("/badge", "Badge", showcase::badge::page()) }))
        .route(
            "/breadcrumb",
            get(|| async { render("/breadcrumb", "Breadcrumb", showcase::breadcrumb::page()) }),
        )
        .route(
            "/button",
            get(|| async { render("/button", "Button", showcase::button::page()) }),
        )
        .route("/card", get(|| async { render("/card", "Card", showcase::card::page()) }))
        .route("/icon", get(|| async { render("/icon", "Icon", showcase::icon::page()) }))
        .route("/link", get(|| async { render("/link", "Link", showcase::link::page()) }))
        .route("/tabs", get(|| async { render("/tabs", "Tabs", showcase::tabs::page()) }))
        .fallback_service(ServeDir::new(public_dir))
}

/// Render a full page document into an `Html` response.
fn render(active: &str, title: &str, content: Markup) -> Html<String> {
    Html(document(active, title, content).into_string())
}

/// The `<head>`: charset/viewport, Google Fonts (Lora/Lato), the compiled
/// stylesheet, and the document title.
fn head(title: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="";
            link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,300;0,400;0,700;1,400&family=Lora:ital,wght@0,400;0,600;0,700;1,400&display=swap";
            link rel="stylesheet" href="/assets/app.css";
            link rel="icon" href="/favicon.png";
            title { (title) " — Mosaic" }
        }
    }
}

/// The full HTML document: `<head>` plus the topbar, sidebar, and main content.
fn document(active: &str, title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body class="font-body" {
                div class="min-h-screen bg-neutral-50 flex flex-col" {
                    (topbar())
                    div class="flex flex-1 overflow-hidden" {
                        (sidebar(active))
                        main class="flex-1 overflow-y-auto" {
                            div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8" {
                                (content)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The top bar with the logo, linking home.
fn topbar() -> Markup {
    html! {
        div class="bg-white border-b border-neutral-200 sticky top-0 z-40" {
            div class="flex items-center h-16 px-4 sm:px-6 lg:px-8" {
                a href="/" class="flex items-center hover:opacity-80 transition-opacity" {
                    img src="/mosaic_logo_sm.svg" alt="Mosaic Logo" class="h-6 w-6 mr-3";
                    h1 class="text-xl font-semibold text-neutral-900" { "Mosaic Tiles Demo" }
                }
            }
        }
    }
}

/// A single sidebar link, highlighted when it matches the active path.
fn nav_link(active: &str, href: &str, label: &str) -> Markup {
    let class = if active == href {
        "block px-3 py-2 rounded-md bg-neutral-100 text-neutral-900"
    } else {
        "block px-3 py-2 rounded-md text-neutral-700 hover:bg-neutral-100 hover:text-neutral-900"
    };
    html! {
        a href=(href) class=(class) { (label) }
    }
}

/// The sidebar navigation: Home, the Foundation group, and the Components group.
fn sidebar(active: &str) -> Markup {
    html! {
        aside class="w-64 bg-white border-r border-neutral-200 overflow-y-auto" {
            nav class="p-4" {
                div class="mb-4" { (nav_link(active, "/", "Home")) }
                div class="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2 px-3" {
                    "Foundation"
                }
                (nav_link(active, "/theme", "Design Tokens"))
                div class="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2 mt-4 px-3" {
                    "Components"
                }
                @for (href, label) in COMPONENT_NAV {
                    (nav_link(active, href, label))
                }
            }
        }
    }
}

/// The home page: a short description of the Maud + Axum architecture.
fn home() -> Markup {
    html! {
        div class="max-w-4xl mx-auto px-4 py-12" {
            div class="flex justify-center mb-3" {
                img src="/mosaic_logo.svg" alt="Mosaic Logo" class="h-[576px] w-[576px]";
            }

            p class="text-lg text-neutral-700 mb-8" {
                "Mosaic is a server-rendered component library for the DaSCH Service Platform. "
                "Each tile is a Rust function returning "
                code class="text-base bg-neutral-100 px-1 rounded" { "maud::Markup" }
                " — plain HTML, no client-side framework, no WASM."
            }

            div class="space-y-8" {
                section {
                    h2 class="text-2xl font-semibold mb-4" { "Architecture" }
                    ul class="space-y-2 text-neutral-700" {
                        li { "• Tiles render HTML on the server with Maud; this playground is a plain Axum MPA." }
                        li { "• Styled with Tailwind CSS v4 utility classes." }
                        li { "• Design tokens are defined in " code class="text-sm bg-neutral-100 px-1 rounded" { "tokens.css" } " via Tailwind " code class="text-sm bg-neutral-100 px-1 rounded" { "@theme" } "." }
                        li { "• Each component ships its own CSS next to its Rust source." }
                    }
                }

                section {
                    h2 class="text-2xl font-semibold mb-4" { "Component Structure" }
                    p class="text-neutral-700 mb-3" {
                        "Each component is a Rust module with a dedicated CSS file:"
                    }
                    pre class="bg-neutral-100 p-4 rounded-lg text-sm overflow-x-auto" {
                        code {
                            "tiles/src/components/[component_name]/\n"
                            "├── mod.rs               # fn -> maud::Markup\n"
                            "└── [component_name].css # Tailwind CSS styles"
                        }
                    }
                }

                section {
                    h2 class="text-2xl font-semibold mb-4" { "Available Components" }
                    p class="text-neutral-700 mb-3" {
                        "Use the navigation to explore component examples."
                    }
                }
            }
        }
    }
}
