use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::extract::Path;
use axum::response::{IntoResponse, Sse};
use datastar::prelude::PatchElements;
use maud::html;

use crate::pages::projects::all_projects;

/// SSE endpoint for dynamic statistics updates
/// Simulates growing repository data
///
/// DataStar features demonstrated:
/// - Server-Sent Events (SSE) for live updates
/// - Fragment merging with targeted element IDs
pub async fn stats_stream_handler() -> impl IntoResponse {
    Sse::new(stream! {
        let mut partners = 10;
        let mut projects = 65;
        let mut resources = 2_100_000;
        let mut audiovisuals = 2_500;


        loop {
            let last_updated = chrono::Utc::now().format("%H:%M:%S").to_string();

            // Render statistics as HTML fragments using Maud
            let fragment = html! {
                dd id="stat-partners" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                    (partners) "+"
                }
                dd id="stat-projects" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                    (projects) "+"
                }
                dd id="stat-resources" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                    @let millions = resources as f64 / 1_000_000.0;
                    (format!("{:.1}M+", millions))
                }
                dd id="stat-audiovisuals" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                    @let thousands = audiovisuals as f64 / 1_000.0;
                    (format!("{:.1}K+", thousands))
                }
                span id="stats-last-updated" class="text-sm text-gray-500 dark:text-gray-500" {
                    "Last updated: " (last_updated)
                }
            };
            let sse = PatchElements::new(fragment.into_string()).write_as_axum_sse_event();
            yield Ok::<_, Infallible>(sse);

            // Wait 2 seconds
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Randomly increment values (simulating growth)
            if rand::random::<f32>() > 0.7 {
                partners += 1;
            }
            if rand::random::<f32>() > 0.5 {
                projects += rand::random_range(1..3);
            }
            if rand::random::<f32>() > 0.2 {
                resources += rand::random_range(10_000..300_000);
            }
            if rand::random::<f32>() > 0.6 {
                audiovisuals += rand::random_range(50..500);
            }
        }
    })
}

/// SSE endpoint for system status updates
/// Simulates service health checks with varying response times and status
///
/// DataStar features demonstrated:
/// - Server-Sent Events (SSE) for live updates
/// - Fragment merging to update multiple elements simultaneously
/// - Dynamic HTML generation with status badges
pub async fn status_stream_handler() -> impl IntoResponse {
    Sse::new(stream! {
        loop {
            // Generate random response times and statuses
            let archive_status = if rand::random::<f32>() > 0.95 { "degraded" } else { "operational" };
            let archive_response = rand::random::<u32>() % 200 + 20;

            let search_status = if rand::random::<f32>() > 0.98 { "down" } else { "operational" };
            let search_response = rand::random::<u32>() % 300 + 50;

            let metadata_status = if rand::random::<f32>() > 0.90 { "degraded" } else { "operational" };
            let metadata_response = rand::random::<u32>() % 400 + 100;

            let export_status = if rand::random::<f32>() > 0.85 { "degraded" } else { "operational" };
            let export_response = rand::random::<u32>() % 500 + 150;

            let primary_connections = rand::random::<u32>() % 15 + 5;
            let replica_connections = rand::random::<u32>() % 10 + 3;

            let storage_used = 2100 + (rand::random::<u32>() % 100);
            let storage_total = 10000;
            let storage_percent = (storage_used as f64 / storage_total as f64 * 100.0) as u32;

            let last_check = chrono::Utc::now().format("%H:%M:%S").to_string();

            // Render status as HTML fragments using Maud
            let fragment = html! {
                // Service statuses
                div id="service-archive" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Archive Service" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Response: " (archive_response) "ms" }
                    }
                    (render_status_badge(archive_status))
                }
                div id="service-search" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Search API" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Response: " (search_response) "ms" }
                    }
                    (render_status_badge(search_status))
                }
                div id="service-metadata" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Metadata Service" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Response: " (metadata_response) "ms" }
                    }
                    (render_status_badge(metadata_status))
                }
                div id="service-export" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Export Service" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Response: " (export_response) "ms" }
                    }
                    (render_status_badge(export_status))
                }

                // Database connections
                div id="db-primary" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Primary Database" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Active connections: " (primary_connections) "/50" }
                    }
                    (render_status_badge("operational"))
                }
                div id="db-replica" class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800" {
                    div {
                        div class="font-semibold text-gray-900 dark:text-white" { "Read Replica" }
                        div class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Active connections: " (replica_connections) "/50" }
                    }
                    (render_status_badge("operational"))
                }

                // Storage
                div id="storage-info" class="rounded-lg border border-gray-200 bg-white p-6 dark:border-gray-700 dark:bg-gray-800" {
                    div class="mb-2 flex items-center justify-between" {
                        span class="font-semibold text-gray-900 dark:text-white" { "Object Storage" }
                        span class="text-sm text-gray-600 dark:text-gray-400" {
                            (storage_used) " GB / " (storage_total) " GB (" (storage_percent) "%)"
                        }
                    }
                    div class="mt-3 h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700" {
                        div class="h-full bg-indigo-600 transition-all duration-500" style=(format!("width: {}%", storage_percent)) {}
                    }
                }

                // Last check time
                span id="status-last-check" class="text-gray-500 dark:text-gray-500" {
                    " • Last check: " (last_check)
                }
            };

            let sse =  PatchElements::new(fragment).into();
            yield Ok::<_, Infallible>(sse);
            // yield MergeFragments::new(fragment.into_string()).into();

            // Wait 2 seconds between updates
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}

fn render_status_badge(status: &str) -> maud::Markup {
    html! {
        @if status == "operational" {
            span class="inline-flex items-center gap-1.5 rounded-full bg-green-50 px-2 py-1 text-xs font-semibold text-green-700 ring-1 ring-inset ring-green-600/20 dark:bg-green-900 dark:text-green-300" {
                span class="h-1.5 w-1.5 rounded-full bg-green-600 dark:bg-green-400" {}
                "Operational"
            }
        } @else if status == "degraded" {
            span class="inline-flex items-center gap-1.5 rounded-full bg-yellow-50 px-2 py-1 text-xs font-semibold text-yellow-700 ring-1 ring-inset ring-yellow-600/20 dark:bg-yellow-900 dark:text-yellow-300" {
                span class="h-1.5 w-1.5 rounded-full bg-yellow-600 dark:bg-yellow-400" {}
                "Degraded"
            }
        } @else {
            span class="inline-flex items-center gap-1.5 rounded-full bg-red-50 px-2 py-1 text-xs font-semibold text-red-700 ring-1 ring-inset ring-red-600/20 dark:bg-red-900 dark:text-red-300" {
                span class="h-1.5 w-1.5 rounded-full bg-red-600 dark:bg-red-400" {}
                "Down"
            }
        }
    }
}

/// Project detail endpoint
/// Returns an HTML fragment with project details for the drawer
///
/// DataStar features demonstrated:
/// - Dynamic content loading via @get()
/// - Fragment merging for drawer content
/// - Event handling with data-on:click for drawer close
pub async fn project_detail_handler(Path(id): Path<usize>) -> impl IntoResponse {
    let projects = all_projects();

    // Get project by index, or return not found message
    let project = projects.get(id);

    let fragment = match project {
        Some(proj) => html! {
            div id="project-detail" class="flex h-full flex-col" {
                // Header with close button
                div class="flex items-center justify-between border-b border-gray-200 p-6 dark:border-gray-700" {
                    h2 class="text-2xl font-bold text-gray-900 dark:text-white" {
                        "Project Details"
                    }
                    button
                        class="rounded-lg p-2 text-gray-400 hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700 dark:hover:text-gray-300"
                        "data-on:click"="$drawerOpen = false"
                        aria-label="Close drawer"
                    {
                        svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" {}
                        }
                    }
                }

                // Content area with scrolling
                div class="flex-1 overflow-y-auto p-6" {
                    // Project image
                    div class="mb-6" {
                        img
                            class="h-64 w-full rounded-lg object-cover shadow-md"
                            src=(proj.image_url)
                            alt=(proj.title);
                    }

                    // Project title
                    h3 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white" {
                        (proj.title)
                    }

                    // Status badge
                    div class="mb-6" {
                        span class="inline-flex items-center gap-1.5 rounded-full bg-green-50 px-3 py-1 text-sm font-semibold text-green-700 ring-1 ring-inset ring-green-600/20 dark:bg-green-900 dark:text-green-300" {
                            span class="h-2 w-2 rounded-full bg-green-600 dark:bg-green-400" {}
                            "Active"
                        }
                    }

                    // Description
                    div class="mb-6" {
                        h4 class="mb-2 font-semibold text-gray-900 dark:text-white" {
                            "Description"
                        }
                        p class="text-gray-700 dark:text-gray-300" {
                            (proj.description)
                        }
                    }

                    // Additional details (placeholder)
                    div class="mb-6" {
                        h4 class="mb-2 font-semibold text-gray-900 dark:text-white" {
                            "Additional Information"
                        }
                        dl class="space-y-2 text-sm" {
                            div class="flex justify-between" {
                                dt class="text-gray-600 dark:text-gray-400" { "Project ID:" }
                                dd class="font-medium text-gray-900 dark:text-white" { (format!("{:04}", id)) }
                            }
                            div class="flex justify-between" {
                                dt class="text-gray-600 dark:text-gray-400" { "Status:" }
                                dd class="font-medium text-gray-900 dark:text-white" { "Active" }
                            }
                            div class="flex justify-between" {
                                dt class="text-gray-600 dark:text-gray-400" { "Resources:" }
                                dd class="font-medium text-gray-900 dark:text-white" { (format!("{}", 150 + id * 50)) }
                            }
                            div class="flex justify-between" {
                                dt class="text-gray-600 dark:text-gray-400" { "Last Updated:" }
                                dd class="font-medium text-gray-900 dark:text-white" { "2024-12-15" }
                            }
                        }
                    }

                    // Actions
                    div class="flex gap-3" {
                        a
                            href="#"
                            class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                        {
                            "Visit Project"
                        }
                        button
                            class="rounded-md bg-white px-4 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 dark:bg-gray-800 dark:text-white dark:ring-gray-700 dark:hover:bg-gray-700"
                        {
                            "Export Data"
                        }
                    }
                }
            }
        },
        None => html! {
            div id="project-detail" class="flex h-full items-center justify-center p-6" {
                div class="text-center" {
                    p class="text-lg font-semibold text-gray-900 dark:text-white" {
                        "Project not found"
                    }
                    p class="mt-2 text-sm text-gray-600 dark:text-gray-400" {
                        "The requested project could not be found."
                    }
                    button
                        class="mt-4 rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500"
                        "data-on:click"="$drawerOpen = false"
                    {
                        "Close"
                    }
                }
            }
        },
    };

    fragment
}
