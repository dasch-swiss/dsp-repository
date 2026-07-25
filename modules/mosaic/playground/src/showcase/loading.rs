//! Loading showcase.

use maud::{html, Markup};
use mosaic_tiles::loading::loading;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("Loading", "A centered loading spinner for pending content.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("loading-basic", "Spinner", "The default centered spinner.", basic()))
    }
}

fn basic() -> Markup {
    html! {
        (loading())
    }
}
