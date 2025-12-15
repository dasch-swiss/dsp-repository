use maud::{html, Markup};

pub struct ProjectCard {
    pub title: &'static str,
    pub description: &'static str,
    pub image_url: &'static str,
}

/// Placeholder project card component
pub fn project_card(project: &ProjectCard) -> Markup {
    html! {
        article class="flex flex-col overflow-hidden rounded-lg shadow-lg" {
            div class="flex-shrink-0" {
                img class="h-48 w-full object-cover" src=(project.image_url) alt=(project.title);
            }
            div class="flex flex-1 flex-col justify-between bg-white p-6 dark:bg-gray-800" {
                div class="flex-1" {
                    h3 class="text-xl font-semibold text-gray-900 dark:text-white" {
                        (project.title)
                    }
                    p class="mt-3 text-base text-gray-500 dark:text-gray-400" {
                        (project.description)
                    }
                }
            }
        }
    }
}
