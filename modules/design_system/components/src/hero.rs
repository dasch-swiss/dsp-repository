use maud::{html, Markup};

use crate::builder_common::ComponentBuilder;

/// Hero section builder with flexible configuration
pub struct HeroBuilder {
    headline: String,
    description: Option<String>,
    announcement_text: Option<String>,
    announcement_link_text: Option<String>,
    announcement_href: Option<String>,
    primary_button_text: Option<String>,
    primary_button_href: Option<String>,
    secondary_button_text: Option<String>,
    secondary_button_href: Option<String>,
    image_url: Option<String>,
    image_alt: Option<String>,
    centered: bool,
    id: Option<String>,
    test_id: Option<String>,
}

impl ComponentBuilder for HeroBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        let has_image = self.image_url.is_some();
        let has_buttons = self.primary_button_text.is_some() || self.secondary_button_text.is_some();
        let has_announcement = self.announcement_text.is_some();

        // Compute IDs for attributes
        let heading_id = self
            .id
            .clone()
            .or_else(|| self.test_id.clone().map(|t| format!("{}-heading", t)));
        let section_id = self.id.clone();
        let test_id = self.test_id.as_deref().unwrap_or("hero");

        if self.centered || !has_image {
            // Simple centered layout (no image)
            html! {
                section
                    class="bg-white dark:bg-gray-900"
                    aria-labelledby=[heading_id.as_deref()]
                    id=[section_id.as_deref()]
                    data-testid=(test_id)
                {
                    div class="relative isolate px-6 pt-14 lg:px-8" {
                        div class="mx-auto max-w-2xl py-32 sm:py-48 lg:py-56" {
                            div class="text-center" {
                                h1 id=[heading_id.as_deref()] class="text-4xl font-bold tracking-tight text-gray-900 sm:text-6xl dark:text-white" {
                                    (self.headline)
                                }
                                @if let Some(desc) = self.description {
                                    p class="mt-6 text-lg leading-8 text-gray-600 dark:text-gray-400" {
                                        (desc)
                                    }
                                }
                                @if has_buttons {
                                    div class="mt-10 flex items-center justify-center gap-x-6" {
                                        @if let Some(text) = self.primary_button_text {
                                            a href=(self.primary_button_href.unwrap_or_default())
                                              class="rounded-md bg-indigo-600 px-3.5 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600" {
                                                (text)
                                            }
                                        }
                                        @if let Some(text) = self.secondary_button_text {
                                            a href=(self.secondary_button_href.unwrap_or_default())
                                              class="text-sm font-semibold leading-6 text-gray-900 dark:text-white" {
                                                (text) " " span aria-hidden="true" { "→" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Complex layout with image on right side
            html! {
                section
                    class="relative"
                    aria-label="Hero"
                    role="banner"
                    id=[section_id.as_deref()]
                    data-testid=(test_id)
                {
                    div class="mx-auto max-w-7xl bg-gray-900" {
                        div class="relative z-10 pt-14 lg:w-full lg:max-w-2xl" {
                            // Decoration SVG
                            svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"
                                class="absolute inset-y-0 right-8 hidden h-full w-80 translate-x-1/2 transform fill-white lg:block dark:fill-gray-900" {
                                polygon points="0,0 90,0 50,100 0,100";
                            }
                            div class="relative px-6 py-32 sm:py-40 lg:px-8 lg:py-56 lg:pr-0" {
                                div class="mx-auto max-w-2xl lg:mx-0 lg:max-w-xl" {
                                    // Announcement banner
                                    @if has_announcement {
                                        div class="hidden sm:mb-10 sm:flex" {
                                            div class="relative rounded-full px-3 py-1 text-sm/6 text-gray-500 ring-1 ring-gray-900/10 hover:ring-gray-900/20 dark:text-gray-400 dark:ring-white/10 dark:hover:ring-white/20" {
                                                (self.announcement_text.unwrap_or_default()) " "
                                                a href=(self.announcement_href.unwrap_or_default())
                                                  class="font-semibold whitespace-nowrap text-indigo-600 dark:text-indigo-400" {
                                                    span aria-hidden="true" class="absolute inset-0" {}
                                                    (self.announcement_link_text.unwrap_or_default()) " "
                                                    span aria-hidden="true" { "→" }
                                                }
                                            }
                                        }
                                    }
                                    // Headline and description
                                    h1 id=[heading_id.as_deref()] class="text-5xl font-semibold tracking-tight text-pretty text-gray-900 sm:text-7xl dark:text-white" {
                                        (self.headline)
                                    }
                                    @if let Some(desc) = self.description {
                                        p class="mt-8 text-lg font-medium text-pretty text-gray-500 sm:text-xl/8 dark:text-gray-400" {
                                            (desc)
                                        }
                                    }
                                    // Buttons
                                    @if has_buttons {
                                        div class="mt-10 flex items-center gap-x-6" {
                                            @if let Some(text) = self.primary_button_text {
                                                a href=(self.primary_button_href.unwrap_or_default())
                                                  class="rounded-md bg-indigo-600 px-3.5 py-2.5 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500" {
                                                    (text)
                                                }
                                            }
                                            @if let Some(text) = self.secondary_button_text {
                                                a href=(self.secondary_button_href.unwrap_or_default())
                                                  class="text-sm/6 font-semibold text-gray-900 dark:text-white" {
                                                    (text) " " span aria-hidden="true" { "→" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Hero image
                    @if let Some(url) = self.image_url {
                        div class="bg-gray-50 lg:absolute lg:inset-y-0 lg:right-0 lg:w-1/2 dark:bg-gray-800" {
                            img src=(url) alt=(self.image_alt.unwrap_or_default())
                                class="aspect-3/2 object-cover lg:aspect-auto lg:size-full";
                        }
                    }
                }
            }
        }
    }
}

impl HeroBuilder {
    pub fn new(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            description: None,
            announcement_text: None,
            announcement_link_text: None,
            announcement_href: None,
            primary_button_text: None,
            primary_button_href: None,
            secondary_button_text: None,
            secondary_button_href: None,
            image_url: None,
            image_alt: None,
            centered: false,
            id: None,
            test_id: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_announcement(
        mut self,
        text: impl Into<String>,
        link_text: impl Into<String>,
        href: impl Into<String>,
    ) -> Self {
        self.announcement_text = Some(text.into());
        self.announcement_link_text = Some(link_text.into());
        self.announcement_href = Some(href.into());
        self
    }

    pub fn with_primary_button(mut self, text: impl Into<String>, href: impl Into<String>) -> Self {
        self.primary_button_text = Some(text.into());
        self.primary_button_href = Some(href.into());
        self
    }

    pub fn with_secondary_button(mut self, text: impl Into<String>, href: impl Into<String>) -> Self {
        self.secondary_button_text = Some(text.into());
        self.secondary_button_href = Some(href.into());
        self
    }

    pub fn with_image(mut self, url: impl Into<String>, alt: impl Into<String>) -> Self {
        self.image_url = Some(url.into());
        self.image_alt = Some(alt.into());
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }
}

/// Create a new hero section
#[must_use = "call .build() to render the component"]
pub fn hero(headline: impl Into<String>) -> HeroBuilder {
    HeroBuilder::new(headline)
}
