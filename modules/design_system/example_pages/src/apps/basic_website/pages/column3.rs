use axum::extract::Path;
use maud::{html, Markup};

use super::column1::render_column1;
use super::column2::render_column2;
use crate::layout::page_layout;

/// Renders Column 3 content - OpenSeadragon viewer
pub fn render_column3() -> Markup {
    html! {
        div class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
            div id="openseadragon-viewer" style="width: 100%; height: 600px;" {}
        }
    }
}

/// Data page with column2 and column3 parameters
pub async fn column3(Path((column2, _column3)): Path<(String, String)>) -> Markup {
    let fruits = vec!["apple", "pear", "banana"];

    let content = html! {
        section class="py-24 sm:py-32" aria-labelledby="data-heading" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                h1 id="data-heading" class="text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white mb-12" {
                    "Data Page"
                }

                // Three column grid
                div class="grid grid-cols-1 gap-8 md:grid-cols-3" {
                    (render_column1(&fruits))
                    (render_column2(&column2, &fruits))
                    (render_column3())
                }
            }
        }
    };

    page_layout("Data - DaSCH Swiss", content)
}
