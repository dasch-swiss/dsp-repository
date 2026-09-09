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
    render_project_card(project, keywords, dpe_core::cover_image_url(&project.shortcode).as_deref())
}

/// Render a card against an already-resolved `cover`. Separated from the cache
/// lookup so both branches can be unit-tested without a process-global.
///
/// `cover` is `None` for a project with no cover image on disk, and then no
/// `<img>` is emitted at all and the placeholder is rendered directly. Deciding
/// this server-side is what makes the fallback work without JavaScript.
fn render_project_card(project: &Project, keywords: &[String], cover: Option<&str>) -> Markup {
    let btn_target = format!("/dpe/projects/{}", project.shortcode);
    // The two fields are independent in the data, so a credit can outlive its image.
    // A credit with no cover credits nothing, and on the card it sits overlaid on the
    // placeholder as if crediting that.
    let credit = cover.and(project.image_credit.as_deref());
    let placeholder_icon = icon(OpenDocument, "w-12 h-12 text-gray-300");

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
                @match cover {
                    Some(src) => {
                        img src=(src)
                            alt=(project.name)
                            class="w-full h-48 object-cover"
                            onerror="this.style.display='none';this.nextElementSibling.style.display='flex'";
                        div class="w-full h-48 bg-gray-100 items-center justify-center hidden" {
                            (placeholder_icon)
                        }
                    }
                    None => {
                        div class="w-full h-48 bg-gray-100 flex items-center justify-center" {
                            (placeholder_icon)
                        }
                    }
                }
            }
            @if let Some(credit) = credit {
                figcaption
                    class="absolute bottom-0 left-0 max-w-[70%] truncate rounded-tr px-2 py-1 text-xs text-gray-700 bg-white/90 backdrop-blur-sm"
                    title=(credit)
                { (credit) }
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

    const COVER: Option<&str> = Some("/assets/images/0ABC.webp");

    #[test]
    fn links_to_detail_page_with_cover_image() {
        let p = sample_project();
        let out = render_project_card(&p, &[], COVER).into_string();
        assert!(out.contains(r#"href="/dpe/projects/0ABC""#), "{out}");
        assert!(out.contains(r#"src="/assets/images/0ABC.webp""#), "{out}");
        assert!(out.contains("Sample Research Project"), "{out}");
    }

    #[test]
    fn omits_the_img_entirely_when_the_project_has_no_cover() {
        // No cover means no `<img>` at all, so nothing depends on a client-side handler.
        let out = render_project_card(&sample_project(), &[], None).into_string();
        assert!(!out.contains("<img"), "no img element: {out}");
        assert!(!out.contains("/assets/images/"), "no cover URL: {out}");
        assert!(!out.contains("onerror"), "no JS-only fallback: {out}");
        // The placeholder takes its place, visible rather than `hidden`.
        assert!(
            out.contains(r#"class="w-full h-48 bg-gray-100 flex items-center justify-center""#),
            "{out}"
        );
        // `flex`, not the `hidden` class list the with-cover branch emits. Matched on the
        // class list rather than the bare word, which also occurs in `aria-hidden`.
        assert!(!out.contains("justify-center hidden"), "placeholder is not hidden: {out}");
        // The card is still a working link to the project.
        assert!(out.contains(r#"href="/dpe/projects/0ABC""#), "{out}");
        assert!(out.contains("Sample Research Project"), "{out}");
    }

    #[test]
    fn hides_the_placeholder_behind_the_img_when_a_cover_exists() {
        // With a cover the placeholder is still emitted but starts hidden, so the
        // retained `onerror` handler has a sibling to reveal.
        let out = render_project_card(&sample_project(), &[], COVER).into_string();
        assert!(
            out.contains(r#"class="w-full h-48 bg-gray-100 items-center justify-center hidden""#),
            "{out}"
        );
        assert!(out.contains("this.nextElementSibling.style.display='flex'"), "{out}");
    }

    #[test]
    fn renders_up_to_three_keyword_badges() {
        let p = sample_project();
        let keywords = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let out = render_project_card(&p, &keywords, COVER).into_string();
        assert_eq!(out.matches("badge badge-secondary").count(), 3, "only first 3 keywords: {out}");
    }

    #[test]
    fn renders_truncated_image_credit_when_present() {
        let p = Project {
            image_credit: Some("© Someone, Somewhere".to_string()),
            ..sample_project()
        };
        let out = render_project_card(&p, &[], COVER).into_string();
        assert!(out.contains("© Someone, Somewhere"), "{out}");
        // overlaid on the image bottom, single-line clip, full text on hover
        assert!(out.contains("<figcaption"), "{out}");
        assert!(out.contains("truncate"), "{out}");
        assert!(out.contains(r#"title="© Someone, Somewhere""#), "{out}");
    }

    #[test]
    fn omits_the_image_credit_when_there_is_no_cover_to_credit() {
        // A credit can outlive its image, and would then be overlaid on the placeholder.
        let p = Project {
            image_credit: Some("© Someone, Somewhere".to_string()),
            ..sample_project()
        };
        let out = render_project_card(&p, &[], None).into_string();
        assert!(!out.contains("<figcaption"), "{out}");
        assert!(!out.contains("© Someone, Somewhere"), "{out}");
    }

    #[test]
    fn omits_image_credit_when_absent() {
        // sample_project() has image_credit: None → no caption on the card.
        let out = render_project_card(&sample_project(), &[], COVER).into_string();
        assert!(!out.contains("truncate"), "{out}");
    }
}
