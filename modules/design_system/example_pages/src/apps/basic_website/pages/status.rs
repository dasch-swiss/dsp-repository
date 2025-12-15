use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::layout::page_layout;

/// System Status page
pub async fn status() -> Markup {
    let content = html! {
        (hero::hero("System Status")
            .with_description("Real-time status of repository services and infrastructure.")
            .with_id("status-heading")
            .build())

        // DataStar: data-init triggers SSE connection for real-time status updates
        div data-init="@get('/api/status/stream')" {
            section class="bg-white py-24 sm:py-32 dark:bg-gray-900" aria-labelledby="status-heading" {
                div class="mx-auto max-w-7xl px-6 lg:px-8" {
                    // Live indicator
                    div class="mt-8 flex items-center justify-center gap-2 text-sm" {
                        span class="relative flex h-3 w-3" {
                            span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" {}
                            span class="relative inline-flex h-3 w-3 rounded-full bg-green-500" {}
                        }
                        span class="text-gray-600 dark:text-gray-400" { "Live updates" }
                        span id="status-last-check" class="text-gray-500 dark:text-gray-500" {}
                    }

                    // Services Section
                    div class="mx-auto mt-16 max-w-4xl" {
                        h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6" {
                            "Repository Services"
                        }

                        div class="space-y-4" {
                            // Archive Service - placeholder
                            div id="service-archive" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Archive Service" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }

                            // Search API - placeholder
                            div id="service-search" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Search API" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }

                            // Metadata Service - placeholder
                            div id="service-metadata" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Metadata Service" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }

                            // Export Service - placeholder
                            div id="service-export" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Export Service" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }
                        }
                    }

                    // Database Section
                    div class="mx-auto mt-12 max-w-4xl" {
                        h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6" {
                            "Database Connections"
                        }

                        div class="space-y-4" {
                            // Primary Database - placeholder
                            div id="db-primary" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Primary Database" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }

                            // Read Replica - placeholder
                            div id="db-replica" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                                div {
                                    div class="font-semibold text-gray-900 dark:text-white" { "Read Replica" }
                                    div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                                }
                            }
                        }
                    }

                    // Storage Section
                    div class="mx-auto mt-12 max-w-4xl" {
                        h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6" {
                            "Storage"
                        }

                        // Storage info - placeholder
                        div id="storage-info" class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
                            div class="mb-2 flex items-center justify-between" {
                                span class="font-semibold text-gray-900 dark:text-white" { "Object Storage" }
                                span class="text-sm text-gray-600 dark:text-gray-400" { "Loading..." }
                            }
                            div class="mt-3 h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700" {
                                div class="h-full bg-indigo-600 transition-all duration-500" style="width: 0%" {}
                            }
                        }
                    }
                }
            }
        }
    };

    page_layout("System Status - DaSCH Swiss", content)
}
