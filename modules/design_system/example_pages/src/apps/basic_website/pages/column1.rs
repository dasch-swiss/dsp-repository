use maud::{html, Markup};

use crate::layout::page_layout;

/// Data page - base route showing only column 1
pub async fn column1() -> Markup {
    let fruits = vec!["apple", "pear", "banana"];

    let content = html! {
        section class="py-24 sm:py-32" aria-labelledby="data-heading" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                h1 id="data-heading" class="text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white mb-12" {
                    "Data Page"
                }

                // Single column grid
                div class="grid grid-cols-1 gap-8 md:grid-cols-1" {
                    // Column 1 - always visible
                    divs class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
                        h2 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white" {
                            "Column 1"
                        }
                        ul class="space-y-2" {
                            @for fruit in &fruits {
                                li {
                                    a
                                        href=(format!("/data/{}", fruit))
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
        }
    };

    page_layout("Data - DaSCH Swiss", content)
}
