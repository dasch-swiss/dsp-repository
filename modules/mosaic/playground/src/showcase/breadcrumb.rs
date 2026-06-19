//! Breadcrumb showcase.

use maud::{html, Markup};
use mosaic_tiles::breadcrumb::{breadcrumb, breadcrumb_item};
use mosaic_tiles::icon::{icon, Grid, People, Tune};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Breadcrumb",
        "Navigation component showing the current page's location within the site hierarchy.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "breadcrumb-basic",
                "Basic Usage",
                "Simple breadcrumb with links and current page",
                basic(),
            )
        })
        ({
            example(
                "breadcrumb-nested",
                "Deeply Nested Navigation",
                "Breadcrumb showing multiple levels of navigation",
                nested(),
            )
        })
        ({
            example(
                "breadcrumb-with_icons",
                "Breadcrumbs with Icons",
                "Breadcrumb items enhanced with icons",
                with_icons(),
            )
        })
    }
}

fn basic() -> Markup {
    breadcrumb(html! {
        ({
            breadcrumb_item(
                Some("/"),
                html! {
                    "Home"
                },
            )
        })
        ({
            breadcrumb_item(
                Some("/documentation"),
                html! {
                    "Documentation"
                },
            )
        })
        ({
            breadcrumb_item(
                None,
                html! {
                    "Breadcrumb"
                },
            )
        })
    })
}

fn nested() -> Markup {
    breadcrumb(html! {
        ({
            breadcrumb_item(
                Some("/"),
                html! {
                    "Home"
                },
            )
        })
        ({
            breadcrumb_item(
                Some("/products"),
                html! {
                    "Products"
                },
            )
        })
        ({
            breadcrumb_item(
                Some("/products/electronics"),
                html! {
                    "Electronics"
                },
            )
        })
        ({
            breadcrumb_item(
                Some("/products/electronics/laptops"),
                html! {
                    "Laptops"
                },
            )
        })
        ({
            breadcrumb_item(
                None,
                html! {
                    "Product Details"
                },
            )
        })
    })
}

fn with_icons() -> Markup {
    breadcrumb(html! {
        ({
            breadcrumb_item(
                Some("/"),
                html! {
                    (icon(Grid, "w-4 h-4 inline mr-1")) "Home"
                },
            )
        })
        ({
            breadcrumb_item(
                Some("/settings"),
                html! {
                    (icon(Tune, "w-4 h-4 inline mr-1")) "Settings"
                },
            )
        })
        ({
            breadcrumb_item(
                None,
                html! {
                    (icon(People, "w-4 h-4 inline mr-1")) "Profile"
                },
            )
        })
    })
}
