use axum::extract::Path;
use maud::{html, Markup};

use super::column1::render_column1;
use crate::layout::page_layout;

/// Renders Column 2 content - reusable across column2 and column3 pages
pub fn render_column2(column2_param: &str, fruits: &[&str]) -> Markup {
    html! {
        div class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
            h2 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white" {
                "Column 2 (" (column2_param) ")"
            }
            ul class="space-y-2" {
                @for fruit in fruits {
                    li {
                        a
                            href=(format!("/data/{}/{}", column2_param, fruit))
                            class="block rounded px-3 py-2 text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700"
                        {
                            (fruit)
                        }
                    }
                }
            }
        }
    }
}

/// Data page with column2 parameter
pub async fn column2(Path(column2): Path<String>) -> Markup {
    let fruits = vec!["apple", "pear", "banana"];

    let content = html! {
        section class="py-24 sm:py-32" aria-labelledby="data-heading" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                h1 id="data-heading" class="text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white mb-12" {
                    "Data Page"
                }

                // Two column grid
                div class="grid grid-cols-1 gap-8 md:grid-cols-2" {
                    (render_column1(&fruits))
                    (render_column2(&column2, &fruits))
                }
            }
        }
    };

    page_layout("Data - DaSCH Swiss", content)
}
