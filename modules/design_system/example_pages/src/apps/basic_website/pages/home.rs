use components::{hero, logo_cloud, ComponentBuilder, Logo};
use maud::{html, Markup};

use crate::components::{
    card_grid, content_section_gray_labeled, content_section_labeled, cta_link_centered, grid_constrained, news_card,
    project_card, section_header_simple, service_card, stats_grid, GridColumns, NewsCard, ProjectCard, ServiceCard,
};
use crate::layout::page_layout;

/// Home page
pub async fn home() -> Markup {
    // Define services
    let services = [
        ServiceCard {
            title: "Data Deposit & Archiving",
            description: "Active preservation for humanities research with searchable, editable, and citable data alongside persistent identifiers.",
            icon: "📦",
        },
        ServiceCard {
            title: "Training & Workshops",
            description: "Free workshops on data management planning, FAIR principles, and best practices for qualitative humanities data with tailored institutional sessions.",
            icon: "🎓",
        },
        ServiceCard {
            title: "Expert Consulting",
            description: "Professional consulting on data modeling, metadata standards, and project-specific workflows. First 8 hours free for research groups and scholars.",
            icon: "💡",
        },
    ];

    // Define featured projects
    let projects = [
        ProjectCard {
            title: "Bernoulli-Euler Online (BEOL)",
            description: "Mathematics influenced by the Bernoulli dynasty and Leonhard Euler.",
            image_url: "https://images.unsplash.com/photo-1635070041078-e363dbe005cb?w=400",
        },
        ProjectCard {
            title: "Côté chaire, côté rue",
            description:
                "Database resulting from transposition of a virtual exhibition about 16th-century Reformation.",
            image_url: "https://images.unsplash.com/photo-1481627834876-b7833e8f5570?w=400",
        },
        ProjectCard {
            title: "Healing Arts",
            description: "Entangled histories of botany, art and health through medieval herbals.",
            image_url: "https://images.unsplash.com/photo-1532938911079-1b06ac7ceec7?w=400",
        },
    ];

    // Define news items
    let news_items = [
        NewsCard {
            title: "FORS-DaSCH Webinar Series",
            date: "2024-11-15",
            description: "Announcement of upcoming webinar series on research data management.",
        },
        NewsCard {
            title: "Survey on Research Data Management",
            date: "2024-10-20",
            description: "Participation call for research data management survey.",
        },
        NewsCard {
            title: "SWISSUbase for Humanities Call",
            date: "2024-09-15",
            description: "Call for projects supporting datasets up to 10GB.",
        },
    ];

    // Define partner logos (using actual logos from dasch.swiss)
    let partners = vec![
        Logo {
            src: "https://dasch.swiss/partners/uni_basel.png".to_string(),
            alt: "University of Basel".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/snsf.png".to_string(),
            alt: "Swiss National Science Foundation".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/swissuniversities.png".to_string(),
            alt: "Swiss Universities".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/SAGW_Logo_sw_pos.jpg".to_string(),
            alt: "Swiss Academy of Humanities and Social Sciences".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/switch.png".to_string(),
            alt: "SWITCH".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/DARIAH-CH.avif".to_string(),
            alt: "DARIAH-CH".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/iiif.png".to_string(),
            alt: "IIIF".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/dariah-eu.png".to_string(),
            alt: "DARIAH-EU".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/re3data.png".to_string(),
            alt: "re3data".to_string(),
            width: 150,
            height: 50,
        },
        Logo {
            src: "https://dasch.swiss/partners/fors.png".to_string(),
            alt: "FORS".to_string(),
            width: 150,
            height: 50,
        },
    ];

    let content = html! {
        // Hero section
        (hero::hero("Swiss National Data and Service Center for the Humanities")
            .with_description("Preserve, manage, and share your humanities research data with Switzerland's trusted archiving infrastructure.")
            .with_primary_button("Contact", "/contact")
            .with_secondary_button("Data Deposit", "/services#data-deposit")
            .with_id("hero-heading")
            .build())

        // Partner logos section
        (logo_cloud::logo_cloud("Our Partners", partners))

        // Services section
        (content_section_labeled("services-heading", html! {
            (section_header_simple("OUR CORE SERVICES"))
            (grid_constrained(GridColumns::Three, html! {
                @for service in &services {
                    (service_card(service))
                }
            }))
        }))

        // Statistics section with live updates
        // DataStar: data-init triggers SSE connection when page loads
        div data-init="@get('/api/stats/stream')" {
            section class="bg-gray-50 py-24 sm:py-32 dark:bg-gray-800" aria-labelledby="stats-heading" {
                div class="mx-auto max-w-7xl px-6 lg:px-8" {
                    // Section header
                    h2 id="stats-heading" class="text-center text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white mb-12" {
                        "Platform Statistics"
                    }

                    // Live indicator
                    div class="mb-8 flex items-center justify-center gap-2 text-sm" {
                        span class="relative flex h-3 w-3" {
                            span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" {}
                            span class="relative inline-flex h-3 w-3 rounded-full bg-green-500" {}
                        }
                        span class="text-gray-600 dark:text-gray-400" { "Live updates" }
                        span id="stats-last-updated" class="text-gray-500 dark:text-gray-500" {}
                    }

                // Statistics grid
                (stats_grid(html! {
                    div class="mx-auto flex max-w-xs flex-col gap-y-4" {
                        dt class="text-base/7 text-gray-600 dark:text-gray-400" {
                            "Partners"
                        }
                        dd id="stat-partners" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                            "10+"
                        }
                    }
                    div class="mx-auto flex max-w-xs flex-col gap-y-4" {
                        dt class="text-base/7 text-gray-600 dark:text-gray-400" {
                            "Projects"
                        }
                        dd id="stat-projects" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                            "65+"
                        }
                    }
                    div class="mx-auto flex max-w-xs flex-col gap-y-4" {
                        dt class="text-base/7 text-gray-600 dark:text-gray-400" {
                            "Resources"
                        }
                        dd id="stat-resources" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                            "2.1M+"
                        }
                    }
                    div class="mx-auto flex max-w-xs flex-col gap-y-4" {
                        dt class="text-base/7 text-gray-600 dark:text-gray-400" {
                            "Audiovisuals"
                        }
                        dd id="stat-audiovisuals" class="order-first text-3xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white" {
                            "2.5K+"
                        }
                    }
                }))
                }
            }
        }

        // Featured projects section
        (content_section_labeled("projects-heading", html! {
            (section_header_simple("Featured Research Projects"))
            (grid_constrained(GridColumns::Three, html! {
                @for project in &projects {
                    (project_card(project))
                }
            }))
            (cta_link_centered("View All Projects", "/projects"))
        }))

        // DARIAH Integration Section
        (content_section_gray_labeled("dariah-heading", html! {
            div class="mx-auto max-w-2xl lg:text-center" {
                (section_header_simple("DARIAH ERIC Full Member"))
                p class="mt-6 text-lg leading-8 text-gray-600 dark:text-gray-400" {
                    "Switzerland as Full Member in DARIAH ERIC since 2023. DARIAH connects 22 countries across Europe in a research infrastructure for the Arts and Humanities."
                }
                div class="mt-10 space-y-4 text-left" {
                    p class="text-base text-gray-600 dark:text-gray-400" {
                        strong { "National Coordinator: " } "Prof. Dr. Rita Gautschy"
                    }
                    p class="text-base text-gray-600 dark:text-gray-400" {
                        strong { "National Coordination Officer: " } "Dr. Cristina Grisot"
                    }
                }
            }
        }))

        // News section
        (content_section_labeled("news-heading", html! {
            (section_header_simple("Latest News"))
            (card_grid(GridColumns::Three, html! {
                @for news in &news_items {
                    (news_card(news))
                }
            }))
            (cta_link_centered("View All News", "/news"))
        }))
    };

    page_layout("Home - DaSCH Swiss", content)
}
