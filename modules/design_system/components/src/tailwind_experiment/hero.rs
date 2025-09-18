use maud::{html, Markup};

use crate::tailwind_experiment::HeroConfig;

fn render_hero_announcement(config: &HeroConfig) -> Markup {
    html! {
        div class="hidden sm:mb-10 sm:flex" {
            div class="relative rounded-full px-3 py-1 text-sm/6 text-gray-500 ring-1 ring-gray-900/10 hover:ring-gray-900/20 dark:text-gray-400 dark:ring-white/10 dark:hover:ring-white/20" {
                (config.announcement_text) " "
                a href=(config.announcement_href) class="font-semibold whitespace-nowrap text-indigo-600 dark:text-indigo-400" {
                    span aria-hidden="true" class="absolute inset-0" {}
                    (config.announcement_link_text) " "
                    span aria-hidden="true" { "→" }
                }
            }
        }
    }
}

fn render_hero_content(config: &HeroConfig) -> Markup {
    html! {
        h1 class="text-5xl font-semibold tracking-tight text-pretty text-gray-900 sm:text-7xl dark:text-white" {
            (config.headline)
        }
        p class="mt-8 text-lg font-medium text-pretty text-gray-500 sm:text-xl/8 dark:text-gray-400" {
            (config.description)
        }
    }
}

fn render_hero_buttons(config: &HeroConfig) -> Markup {
    html! {
        div class="mt-10 flex items-center gap-x-6" {
            a href=(config.primary_button_href) class="rounded-md bg-indigo-600 px-3.5 py-2.5 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500" {
                (config.primary_button_text)
            }
            a href=(config.secondary_button_href) class="text-sm/6 font-semibold text-gray-900 dark:text-white" {
                (config.secondary_button_text) " "
                span aria-hidden="true" { "→" }
            }
        }
    }
}

fn render_hero_decoration() -> Markup {
    html! {
        svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" class="absolute inset-y-0 right-8 hidden h-full w-80 translate-x-1/2 transform fill-white lg:block dark:fill-gray-900" {
            polygon points="0,0 90,0 50,100 0,100";
        }
    }
}

fn render_hero_image(config: &HeroConfig) -> Markup {
    html! {
        div class="bg-gray-50 lg:absolute lg:inset-y-0 lg:right-0 lg:w-1/2 dark:bg-gray-800" {
            img src=(config.image_url) alt=(config.image_alt) class="aspect-3/2 object-cover lg:aspect-auto lg:size-full";
        }
    }
}

pub fn tailwind_hero(config: &HeroConfig) -> Markup {
    html! {
        section class="relative" aria-label="Hero" role="banner" {
            div class="mx-auto max-w-7xl" {
                div class="relative z-10 pt-14 lg:w-full lg:max-w-2xl" {
                    (render_hero_decoration())
                    div class="relative px-6 py-32 sm:py-40 lg:px-8 lg:py-56 lg:pr-0" {
                        div class="mx-auto max-w-2xl lg:mx-0 lg:max-w-xl" {
                            (render_hero_announcement(config))
                            (render_hero_content(config))
                            (render_hero_buttons(config))
                        }
                    }
                }
            }
            (render_hero_image(config))
        }
    }
}
