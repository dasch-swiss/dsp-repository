use maud::{html, Markup};

use crate::layout::page_layout;

/// Data page with three columns
pub async fn data() -> Markup {
    let fruits = vec!["apple", "pear", "banana"];

    let content = html! {
        section class="py-24 sm:py-32" aria-labelledby="data-heading" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                h1 id="data-heading" class="text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white mb-12" {
                    "Data Page"
                }

                // Three column grid
                div class="grid grid-cols-1 gap-8 md:grid-cols-3" {
                    @for i in 0..3 {
                        div class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
                            h2 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white" {
                                "Column " (i + 1)
                            }
                            ul class="space-y-2" {
                                @for fruit in &fruits {
                                    li class="text-gray-700 dark:text-gray-300" {
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
