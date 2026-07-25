use dpe_core::Project;
use maud::{html, Markup};
use mosaic_tiles::badge::{badge, BadgeSize, BadgeVariant};
use mosaic_tiles::card::{card, card_body_with_class, CardVariant};
use mosaic_tiles::icon::{icon, OpenDocument};

use super::statusbadge::project_card_indicators;

/// A project tile for the projects grid: cover image (with fallback), title,
/// short description, and up to three keyword badges. The whole card links to
/// the project detail page. `keywords` are pre-resolved to display strings.
pub fn project_card(project: &Project, keywords: &[String]) -> Markup {
    let image_src = format!("/assets/images/{}.webp", project.shortcode);
    let btn_target = format!("/dpe/projects/{}", project.shortcode);

    let body_inner = html! {
        div class="flex flex-col flex-1" {
            h2 class="font-display font-bold text-lg line-clamp-2" { (project.name) }
            p class="text-sm text-gray-600 line-clamp-4 mt-2 flex-1" { (project.short_description) }
            div class="flex flex-wrap gap-1 mt-3" {
                @for kw in keywords.iter().take(3) {
                    ({
                        badge(kw.as_str())
                            .variant(BadgeVariant::Secondary)
                            .size(BadgeSize::Small)
                    })
                }
            }
        }
    };

    let card_content = html! {
        figure class="bg-neutral-900 relative rounded-t-[inherit]" {
            div class="overflow-hidden rounded-t-[inherit]" {
                img src=(image_src)
                    alt=(project.name)
                    class="w-full h-48 object-cover"
                    onerror="this.style.display='none';this.nextElementSibling.style.display='flex'";
                div class="w-full h-48 bg-gray-100 items-center justify-center hidden" {
                    (icon(OpenDocument, "w-12 h-12 text-gray-300"))
                }
            }
            ({
                project_card_indicators(
                    &project.status,
                    &project.access_rights.access_rights,
                )
            })
        }
        (card_body_with_class("flex-1 flex flex-col", body_inner))
    };

    html! {
        a href=(btn_target) class="block h-full relative hover:z-10" {
            ({
                card(card_content)
                    .variant(CardVariant::AutoHover)
                    .class("flex flex-col h-full ![overflow:visible]")
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn links_to_detail_page_with_cover_image() {
        let p = sample_project();
        let out = project_card(&p, &[]).into_string();
        assert!(out.contains(r#"href="/dpe/projects/0ABC""#), "{out}");
        assert!(out.contains(r#"src="/assets/images/0ABC.webp""#), "{out}");
        assert!(out.contains("Sample Research Project"), "{out}");
    }

    #[test]
    fn renders_up_to_three_keyword_badges() {
        let p = sample_project();
        let keywords = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let out = project_card(&p, &keywords).into_string();
        assert_eq!(out.matches("badge badge-secondary").count(), 3, "only first 3 keywords: {out}");
    }
}
