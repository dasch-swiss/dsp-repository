use axum::extract::Query;
use components::{hero, ComponentBuilder};
use maud::{html, Markup};
use serde::Deserialize;

pub use super::project_data::all_projects;
use crate::components::{
    content_section_compact_labeled, flex_between, pagination, project_card, project_grid, status_badge_active,
    ProjectCard,
};
use crate::layout::page_layout;

#[derive(Deserialize)]
pub struct ProjectsPageQuery {
    #[serde(default = "default_page")]
    page: usize,
}

fn default_page() -> usize {
    1
}

/// Renders project cards for a given page with status badges
pub fn render_project_cards(projects: &[ProjectCard], start_idx: usize) -> Markup {
    project_grid(html! {
        @for (idx, project) in projects.iter().enumerate() {
            // Calculate the actual project index in the full list
            @let project_idx = start_idx + idx;
            // Project card with "Active" badge and click handler
            // DataStar: data-on:click sets signal and fetches content via @get()
            div
                class="relative cursor-pointer transition-transform hover:scale-105"
                "data-on:click"=(format!("$drawerOpen = true; @get('/api/project/{}')", project_idx))
            {
                div class="absolute left-4 top-4 z-10" {
                    (status_badge_active())
                }
                (project_card(project))
            }
        }
    })
}

/// Projects page
pub async fn projects(Query(query): Query<ProjectsPageQuery>) -> Markup {
    let all_projects = all_projects();
    let total_projects = all_projects.len();
    let per_page = 12;
    let total_pages = total_projects.div_ceil(per_page);

    // Clamp page to valid range
    let page = query.page.max(1).min(total_pages);

    // Calculate start and end indices for this page
    let start_idx = (page - 1) * per_page;
    let end_idx = (start_idx + per_page).min(total_projects);
    let projects = &all_projects[start_idx..end_idx];

    let content = html! {
        // Initialize drawer state signal
        // DataStar: signals provide reactive state management
        div
            "data-signals"=r#"{"drawerOpen": false}"#
        {
            // Hero/Introduction section
            (hero::hero("Discover research projects")
                .with_description("Explore cutting-edge humanities research projects hosted on our platform. Each project represents a unique contribution to Swiss cultural heritage and academic knowledge.")
                .with_id("projects-intro-heading")
                .build())

            // Projects listing section
            (content_section_compact_labeled("projects-list-heading", html! {
                // Status indicator
                (flex_between(html! {
                    div class="mb-8 border-b border-gray-200 pb-4 dark:border-gray-700" {
                        p class="text-sm text-gray-600 dark:text-gray-400" {
                            "Showing " (start_idx + 1) " to " (end_idx) " of " (total_projects) " projects"
                        }
                    }
                }))

                // Project grid
                (render_project_cards(projects, start_idx))

                // Pagination
                (pagination(page, total_pages))
            }))

            // Drawer overlay backdrop
            // DataStar: conditional classes (data-class:*) control visibility based on signal
            div
                "data-on:click"="$drawerOpen = false"
                "data-class:opacity-0"="!$drawerOpen"
                "data-class:pointer-events-none"="!$drawerOpen"
                "data-class:opacity-100"="$drawerOpen"
                "data-class:pointer-events-auto"="$drawerOpen"
                class="fixed inset-0 z-40 bg-gray-500/75 opacity-0 pointer-events-none transition-opacity duration-300 dark:bg-gray-800/75"
                aria-hidden="true"
            {}

            // Drawer panel
            // DataStar: conditional classes for slide-in animation
            div
                "data-class:translate-x-full"="!$drawerOpen"
                "data-class:translate-x-0"="$drawerOpen"
                class="fixed inset-y-0 right-0 z-50 w-full max-w-2xl translate-x-full overflow-hidden bg-white shadow-2xl ring-1 ring-gray-900/10 transition-transform duration-300 ease-in-out dark:bg-gray-900 dark:ring-white/10"
                role="dialog"
                aria-modal="true"
                aria-labelledby="drawer-title"
            {
                // Content will be loaded here via DataStar
                div id="project-detail" class="flex h-full items-center justify-center" {
                    // Loading placeholder
                    div class="text-gray-500 dark:text-gray-400" {
                        "Loading..."
                    }
                }
            }
        }
    };

    page_layout("Projects - DaSCH Swiss", content)
}
