use maud::{html, Markup};

use crate::icon::{icon_with_class, IconType};

#[derive(Debug, Clone)]
pub struct FooterConfig {
    pub company_name: &'static str,
    pub description: &'static str,
    pub copyright_text: &'static str,
    pub logo_light_url: &'static str,
    pub logo_dark_url: &'static str,
}

fn render_footer_logo(config: &FooterConfig) -> Markup {
    html! {
        img src=(config.logo_light_url) alt=(config.company_name) class="h-9 dark:hidden";
        img src=(config.logo_dark_url) alt=(config.company_name) class="h-9 not-dark:hidden";
    }
}

fn render_footer_social_links() -> Markup {
    html! {
        div class="flex gap-x-6" {
            a href="#" class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300" {
                span class="sr-only" { "Facebook" }
                (icon_with_class(IconType::Facebook, "size-6"))
            }
            a href="#" class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300" {
                span class="sr-only" { "Instagram" }
                (icon_with_class(IconType::Instagram, "size-6"))
            }
            a href="#" class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300" {
                span class="sr-only" { "X" }
                (icon_with_class(IconType::X, "size-6"))
            }
            a href="#" class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300" {
                span class="sr-only" { "GitHub" }
                (icon_with_class(IconType::GitHub, "size-6"))
            }
            a href="#" class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300" {
                span class="sr-only" { "YouTube" }
                (icon_with_class(IconType::YouTube, "size-6"))
            }
        }
    }
}

pub fn footer(config: &FooterConfig) -> Markup {
    html! {
        footer class="bg-white dark:bg-gray-900" aria-labelledby="footer-heading" {
            h2 id="footer-heading" class="sr-only" { "Footer" }
            div class="mx-auto max-w-7xl px-6 pb-8 pt-16 sm:pt-24 lg:px-8 lg:pt-32" {
                div class="xl:grid xl:grid-cols-3 xl:gap-8" {
                    div class="space-y-8" {
                        (render_footer_logo(config))
                        p class="text-sm/6 text-balance text-gray-600 dark:text-gray-400" {
                            (config.description)
                        }
                        (render_footer_social_links())
                    }
                    div class="mt-16 grid grid-cols-2 gap-8 xl:col-span-2 xl:mt-0" {
                        div class="md:grid md:grid-cols-2 md:gap-8" {
                            div {
                                h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white" { "Solutions" }
                                ul role="list" class="mt-6 space-y-4" {
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Marketing" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Analytics" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Automation" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Commerce" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Insights" }
                                    }
                                }
                            }
                            div class="mt-10 md:mt-0" {
                                h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white" { "Support" }
                                ul role="list" class="mt-6 space-y-4" {
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Submit ticket" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Documentation" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Guides" }
                                    }
                                }
                            }
                        }
                        div class="md:grid md:grid-cols-2 md:gap-8" {
                            div {
                                h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white" { "Company" }
                                ul role="list" class="mt-6 space-y-4" {
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "About" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Blog" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Jobs" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Press" }
                                    }
                                }
                            }
                            div class="mt-10 md:mt-0" {
                                h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white" { "Legal" }
                                ul role="list" class="mt-6 space-y-4" {
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Terms of service" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "Privacy policy" }
                                    }
                                    li {
                                        a href="#" class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white" { "License" }
                                    }
                                }
                            }
                        }
                    }
                }
                div class="mt-16 border-t border-gray-900/10 pt-8 sm:mt-20 lg:mt-24 dark:border-white/10" {
                    p class="text-sm/6 text-gray-600 dark:text-gray-400" { (config.copyright_text) }
                }
            }
        }
    }
}
