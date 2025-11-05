use components::{icon, menu_item, IconType};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Icon component renderer
pub struct IconRenderer;

impl ComponentRenderer for IconRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "icon".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Custom Sizes" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icons can be sized using Tailwind size classes"
                    }
                    div class="flex items-center gap-6" {
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-4"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-4" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-5"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-5 (default)" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-6"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-6" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-8"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-8" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-12"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-12" }
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Custom Colors" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icons inherit currentColor and can be styled with text color classes"
                    }
                    div class="flex items-center gap-6" {
                        (icon::icon_with_class(IconType::Star, "size-8 text-yellow-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-blue-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-green-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-red-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-purple-500"))
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Icons in other components" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Some components like the menu component or the button component implement icons. In general there is no need to define a color or a size as it is inherited or already defined by the component. Consult the docs of those components on how to pass or build the desired icon."
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("Favorites", "/favorites", icon::icon(IconType::Star)))
                            (menu_item::link_menu_item_with_icon("View Source", "/source", icon::icon(IconType::Code)))
                            (menu_item::link_menu_item_with_icon("Report", "/report", icon::icon(IconType::Flag)))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Available Icons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "All available icons including Heroicons and social media icons"
                    }
                    div class="grid grid-cols-2 md:grid-cols-3 gap-4" {
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Star))
                            span class="text-sm font-medium" { "Star" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Code))
                            span class="text-sm font-medium" { "Code" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Flag))
                            span class="text-sm font-medium" { "Flag" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Hamburger))
                            span class="text-sm font-medium" { "Hamburger" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Close))
                            span class="text-sm font-medium" { "Close" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::ChevronDown))
                            span class="text-sm font-medium" { "ChevronDown" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Facebook))
                            span class="text-sm font-medium" { "Facebook" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Instagram))
                            span class="text-sm font-medium" { "Instagram" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::X))
                            span class="text-sm font-medium" { "X" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::GitHub))
                            span class="text-sm font-medium" { "GitHub" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::YouTube))
                            span class="text-sm font-medium" { "YouTube" }
                        }
                    }
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
