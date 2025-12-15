use maud::{html, Markup};

/// Pagination component with page numbers and navigation
pub fn pagination(current_page: usize, total_pages: usize) -> Markup {
    html! {
        nav class="mt-12 flex items-center justify-between border-t border-gray-200 px-4 sm:px-0 dark:border-gray-700" aria-label="Pagination" {
            // Mobile pagination (Previous/Next only)
            div class="flex flex-1 justify-between sm:hidden" {
                @if current_page > 1 {
                    a href=(format!("?page={}", current_page - 1))
                      class="relative inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200" {
                        "Previous"
                    }
                } @else {
                    span class="relative inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-400 cursor-not-allowed dark:border-gray-600 dark:bg-gray-800" {
                        "Previous"
                    }
                }

                @if current_page < total_pages {
                    a href=(format!("?page={}", current_page + 1))
                      class="relative ml-3 inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200" {
                        "Next"
                    }
                } @else {
                    span class="relative ml-3 inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-400 cursor-not-allowed dark:border-gray-600 dark:bg-gray-800" {
                        "Next"
                    }
                }
            }

            // Desktop pagination (numbered pages)
            div class="hidden sm:flex sm:flex-1 sm:items-center sm:justify-center" {
                div class="mt-6" {
                    nav class="isolate inline-flex -space-x-px rounded-md shadow-sm" aria-label="Pagination" {
                        // Previous button
                        @if current_page > 1 {
                            a href=(format!("?page={}", current_page - 1))
                              class="relative inline-flex items-center rounded-l-md px-2 py-2 text-gray-400 ring-1 ring-inset ring-gray-300 hover:bg-gray-50 focus:z-20 dark:ring-gray-600 dark:hover:bg-gray-700" {
                                span class="sr-only" { "Previous" }
                                "‹"
                            }
                        } @else {
                            span class="relative inline-flex items-center rounded-l-md px-2 py-2 text-gray-300 ring-1 ring-inset ring-gray-300 cursor-not-allowed dark:ring-gray-600 dark:text-gray-600" {
                                span class="sr-only" { "Previous" }
                                "‹"
                            }
                        }

                        // Page numbers
                        @for page in 1..=total_pages {
                            @if page == current_page {
                                a href=(format!("?page={}", page)) aria-current="page"
                                  class="relative z-10 inline-flex items-center bg-indigo-600 px-4 py-2 text-sm font-semibold text-white focus:z-20 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600" {
                                    (page)
                                }
                            } @else {
                                a href=(format!("?page={}", page))
                                  class="relative inline-flex items-center px-4 py-2 text-sm font-semibold text-gray-900 ring-1 ring-inset ring-gray-300 hover:bg-gray-50 focus:z-20 dark:text-gray-100 dark:ring-gray-600 dark:hover:bg-gray-700" {
                                    (page)
                                }
                            }
                        }

                        // Next button
                        @if current_page < total_pages {
                            a href=(format!("?page={}", current_page + 1))
                              class="relative inline-flex items-center rounded-r-md px-2 py-2 text-gray-400 ring-1 ring-inset ring-gray-300 hover:bg-gray-50 focus:z-20 dark:ring-gray-600 dark:hover:bg-gray-700" {
                                span class="sr-only" { "Next" }
                                "›"
                            }
                        } @else {
                            span class="relative inline-flex items-center rounded-r-md px-2 py-2 text-gray-300 ring-1 ring-inset ring-gray-300 cursor-not-allowed dark:ring-gray-600 dark:text-gray-600" {
                                span class="sr-only" { "Next" }
                                "›"
                            }
                        }
                    }
                }
            }
        }
    }
}
