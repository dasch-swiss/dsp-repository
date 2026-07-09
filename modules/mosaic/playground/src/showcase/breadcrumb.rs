//! Breadcrumb showcase.

use maud::{html, Markup};
use mosaic_tiles::breadcrumb::{breadcrumb, breadcrumb_current, breadcrumb_item};
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
        (breadcrumb_item("/", "Home"))
        (breadcrumb_item("/documentation", "Documentation"))
        (breadcrumb_current("Breadcrumb"))
    })
}

fn nested() -> Markup {
    breadcrumb(html! {
        (breadcrumb_item("/", "Home"))
        (breadcrumb_item("/products", "Products"))
        (breadcrumb_item("/products/electronics", "Electronics"))
        (breadcrumb_item("/products/electronics/laptops", "Laptops"))
        (breadcrumb_current("Product Details"))
    })
}

fn with_icons() -> Markup {
    let home = html! {
        (icon(Grid, "w-4 h-4 inline mr-1"))
        "Home"
    };
    let settings = html! {
        (icon(Tune, "w-4 h-4 inline mr-1"))
        "Settings"
    };
    let profile = html! {
        (icon(People, "w-4 h-4 inline mr-1"))
        "Profile"
    };
    breadcrumb(html! {
        (breadcrumb_item("/", home))
        (breadcrumb_item("/settings", settings))
        (breadcrumb_current(profile))
    })
}
