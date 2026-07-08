use dpe_core::Project;
use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::card::{card, card_body, CardVariant};
use mosaic_tiles::icon::{icon, Export, OpenDocument};
use mosaic_tiles::link::link;
use mosaic_tiles::ComponentBuilder;

use super::description::description;

/// The project hero: cover image (with fallback), title, alternative names,
/// description, and primary/secondary "discover data" buttons.
pub fn project_header(proj: &Project) -> Markup {
    let image_src = format!("/assets/images/{}.webp", proj.shortcode);
    let desc = dpe_core::lang_value(&proj.description).cloned().unwrap_or_default();
    let alternative_names: Vec<String> = proj
        .alternative_names
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|m| dpe_core::lang_value(m).cloned())
        .collect();

    let body_inner = html! {
        div class="p-8 flex flex-row justify-center" {
            div class="max-w-3xl" {
                h2 class="font-bold font-display text-3xl text-ellipsis" { (proj.name) }
                @if !alternative_names.is_empty() {
                    p class="mt-1 text-sm text-gray-600" {
                        span { "Also known as: " }
                        @for name in &alternative_names {
                            span { (name) }
                        }
                    }
                }
                div class="mt-4" { (description(&desc)) }
                div class="mt-6 flex gap-4" {
                    @if let Some(u) = &proj.url {
                        @let label = u
                            .text
                            .clone()
                            .unwrap_or_else(|| "Discover Project Data".to_string());
                        ({
                            link(
                                    html! {
                                        (label) (icon(Export, "w-5 h-5"))
                                    },
                                    u.url.as_str(),
                                )
                                .as_button(ButtonVariant::Primary)
                        })
                    }
                    @if let Some(u) = &proj.secondary_url {
                        @let label = u
                            .text
                            .clone()
                            .unwrap_or_else(|| "External Project Website".to_string());
                        ({
                            link(
                                    html! {
                                        (label) (icon(Export, "w-5 h-5"))
                                    },
                                    u.url.as_str(),
                                )
                                .as_button(ButtonVariant::Outline)
                        })
                    }
                }
            }
        }
    };

    let card_content = html! {
        figure {
            div class="overflow-hidden" {
                img src=(image_src)
                    alt=(proj.name)
                    class="w-full object-cover"
                    style="height: 320px"
                    onerror="this.style.display='none';this.nextElementSibling.style.display='flex'";
                div class="w-full bg-gray-100 items-center justify-center hidden"
                    style="height: 320px"
                { (icon(OpenDocument, "w-12 h-12 text-gray-300")) }
            }
        }
        (card_body(body_inner))
    };

    card(card_content).variant(CardVariant::Bordered).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn renders_title_image_and_primary_link() {
        let out = project_header(&sample_project()).into_string();
        assert!(out.contains("Sample Research Project"), "{out}");
        assert!(out.contains(r#"src="/assets/images/0ABC.webp""#), "{out}");
        // sample_project has a primary url → "Discover Project Data" button.
        assert!(out.contains(r#"href="https://example.org/project""#), "{out}");
        assert!(out.contains("Discover Project Data"), "{out}");
    }
}
