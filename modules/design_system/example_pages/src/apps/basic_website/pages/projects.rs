use axum::extract::Query;
use components::{hero, ComponentBuilder};
use maud::{html, Markup};
use serde::Deserialize;

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

/// All available projects (30 total for 3 pages of 12 each, with 6 on last page)
pub fn all_projects() -> Vec<ProjectCard> {
    vec![
        ProjectCard {
            title: "Bernoulli-Euler Online (BEOL)",
            description: "Mathematics history digital edition of the correspondence and manuscripts of the Bernoulli mathematicians and Leonhard Euler.",
            image_url: "https://images.unsplash.com/photo-1635070041078-e363dbe005cb?w=400",
        },
        ProjectCard {
            title: "Healing Arts",
            description: "Medical knowledge in manuscript traditions from medieval Europe and the Islamic world.",
            image_url: "https://images.unsplash.com/photo-1532938911079-1b06ac7ceec7?w=400",
        },
        ProjectCard {
            title: "Operative TV",
            description: "Audiovisual closed-circuits study documenting the evolution of television technology.",
            image_url: "https://images.unsplash.com/photo-1522869635100-9f4c5e86aa37?w=400",
        },
        ProjectCard {
            title: "Hôtel de Musique Bern",
            description: "Historical performance database documenting musical life in 18th-century Bern.",
            image_url: "https://images.unsplash.com/photo-1507838153414-b4b713384a76?w=400",
        },
        ProjectCard {
            title: "I-GEOARCHive",
            description: "Geoarchaeological data repository for Swiss archaeological research.",
            image_url: "https://images.unsplash.com/photo-1541888946425-d81bb19240f5?w=400",
        },
        ProjectCard {
            title: "Côté chaire, côté rue",
            description: "Academic and popular religious literature in the French-speaking world during the Reformation.",
            image_url: "https://images.unsplash.com/photo-1481627834876-b7833e8f5570?w=400",
        },
        ProjectCard {
            title: "Medieval Manuscripts",
            description: "Digital archive of illuminated manuscripts from Swiss monasteries.",
            image_url: "https://images.unsplash.com/photo-1524995997946-a1c2e315a42f?w=400",
        },
        ProjectCard {
            title: "Swiss Dialect Archive",
            description: "Comprehensive collection of Swiss German dialect recordings and transcriptions.",
            image_url: "https://images.unsplash.com/photo-1516280440614-37939bbacd81?w=400",
        },
        ProjectCard {
            title: "Alpine Architecture",
            description: "Documentation of traditional Alpine building techniques and structures.",
            image_url: "https://images.unsplash.com/photo-1513635269975-59663e0ac1ad?w=400",
        },
        ProjectCard {
            title: "Reformation Pamphlets",
            description: "Digital collection of 16th-century religious pamphlets and broadsides.",
            image_url: "https://images.unsplash.com/photo-1455390582262-044cdead277a?w=400",
        },
        ProjectCard {
            title: "Swiss Folklore Collection",
            description: "Ethnographic records of Swiss customs, legends, and oral traditions.",
            image_url: "https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=400",
        },
        ProjectCard {
            title: "Historical Maps of Switzerland",
            description: "Georeferenced historical maps from the 16th to 20th centuries.",
            image_url: "https://images.unsplash.com/photo-1524661135-423995f22d0b?w=400",
        },
        // Page 2 projects (13-24)
        ProjectCard {
            title: "Swiss Textile Heritage",
            description: "Documentation of traditional Swiss textile production and craftsmanship.",
            image_url: "https://images.unsplash.com/photo-1610701596007-11502861dcfa?w=400",
        },
        ProjectCard {
            title: "Alpine Literature Archive",
            description: "Digital collection of literature about and from the Swiss Alps.",
            image_url: "https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=400",
        },
        ProjectCard {
            title: "Swiss Musical Heritage",
            description: "Traditional Swiss music recordings and manuscripts.",
            image_url: "https://images.unsplash.com/photo-1511379938547-c1f69419868d?w=400",
        },
        ProjectCard {
            title: "Helvetic Correspondence",
            description: "Letters and correspondence between Swiss scholars and intellectuals.",
            image_url: "https://images.unsplash.com/photo-1455390582262-044cdead277a?w=400",
        },
        ProjectCard {
            title: "Swiss Botanical Illustrations",
            description: "Historical botanical drawings and descriptions from Swiss collections.",
            image_url: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=400",
        },
        ProjectCard {
            title: "Urban Development Documentation",
            description: "Visual documentation of Swiss urban development through the centuries.",
            image_url: "https://images.unsplash.com/photo-1449824913935-59a10b8d2000?w=400",
        },
        ProjectCard {
            title: "Swiss Legal Manuscripts",
            description: "Historical legal documents and manuscripts from Swiss archives.",
            image_url: "https://images.unsplash.com/photo-1589829085413-56de8ae18c73?w=400",
        },
        ProjectCard {
            title: "Traditional Swiss Crafts",
            description: "Documentation of traditional Swiss craftsmanship and techniques.",
            image_url: "https://images.unsplash.com/photo-1452860606245-08befc0ff44b?w=400",
        },
        ProjectCard {
            title: "Swiss Religious Art",
            description: "Digital archive of religious art from Swiss churches and monasteries.",
            image_url: "https://images.unsplash.com/photo-1578301978162-7aae4d755744?w=400",
        },
        ProjectCard {
            title: "Alpine Photography Collection",
            description: "Historical photographs documenting life in the Swiss Alps.",
            image_url: "https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=400",
        },
        ProjectCard {
            title: "Swiss Immigration Records",
            description: "Historical records of immigration to and from Switzerland.",
            image_url: "https://images.unsplash.com/photo-1552664730-d307ca884978?w=400",
        },
        ProjectCard {
            title: "Swiss Scientific Instruments",
            description: "Catalog of historical scientific instruments from Swiss collections.",
            image_url: "https://images.unsplash.com/photo-1532094349884-543bc11b234d?w=400",
        },
        // Page 3 projects (25-30)
        ProjectCard {
            title: "Swiss Theater Archive",
            description: "Historical documents and recordings from Swiss theater productions.",
            image_url: "https://images.unsplash.com/photo-1503095396549-807759245b35?w=400",
        },
        ProjectCard {
            title: "Alpine Ecology Research",
            description: "Long-term ecological research data from Swiss Alpine regions.",
            image_url: "https://images.unsplash.com/photo-1469474968028-56623f02e42e?w=400",
        },
        ProjectCard {
            title: "Swiss Wine Heritage",
            description: "Documentation of Swiss viticulture history and traditions.",
            image_url: "https://images.unsplash.com/photo-1506377247377-2a5b3b417ebb?w=400",
        },
        ProjectCard {
            title: "Swiss Clockmaking History",
            description: "Archive of Swiss clockmaking craftsmanship and innovation.",
            image_url: "https://images.unsplash.com/photo-1509048191080-d2984bad6ae5?w=400",
        },
        ProjectCard {
            title: "Swiss Educational History",
            description: "Historical documents on the development of Swiss education system.",
            image_url: "https://images.unsplash.com/photo-1503676260728-1c00da094a0b?w=400",
        },
        ProjectCard {
            title: "Swiss Culinary Traditions",
            description: "Historical recipes and culinary traditions from Swiss regions.",
            image_url: "https://images.unsplash.com/photo-1504674900247-0877df9cc836?w=400",
        },
    ]
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
