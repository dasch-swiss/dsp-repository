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
    render_project_header(proj, dpe_core::cover_image_url(&proj.shortcode).as_deref())
}

/// Render the hero against an already-resolved `cover`. Separated from the cache
/// lookup so both branches can be unit-tested without a process-global.
///
/// `cover` is `None` for a project with no cover image on disk, and then no
/// `<img>` is emitted at all and the placeholder is rendered directly. Deciding
/// this server-side is what makes the fallback work without JavaScript.
fn render_project_header(proj: &Project, cover: Option<&str>) -> Markup {
    // The two fields are independent in the data, so a credit can outlive its image,
    // and a credit with no cover credits nothing.
    let credit = cover.and(proj.image_credit.as_deref());
    let placeholder_icon = icon(OpenDocument, "w-12 h-12 text-gray-300");
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
                                .external()
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
                                .external()
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
                @match cover {
                    Some(src) => {
                        img src=(src)
                            alt=(proj.name)
                            class="w-full object-cover"
                            style="height: 320px"
                            onerror="this.style.display='none';this.nextElementSibling.style.display='flex'";
                        div class="w-full bg-gray-100 items-center justify-center hidden"
                            style="height: 320px"
                        { (placeholder_icon) }
                    }
                    None => {
                        div class="w-full bg-gray-100 flex items-center justify-center"
                            style="height: 320px"
                        { (placeholder_icon) }
                    }
                }
            }
            @if let Some(credit) = credit {
                figcaption class="px-3 py-1 text-right text-xs text-gray-500" { (credit) }
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

    const COVER: Option<&str> = Some("/assets/images/0ABC.webp");

    #[test]
    fn renders_title_image_and_primary_link() {
        let out = render_project_header(&sample_project(), COVER).into_string();
        assert!(out.contains("Sample Research Project"), "{out}");
        assert!(out.contains(r#"src="/assets/images/0ABC.webp""#), "{out}");
        // sample_project has a primary url → "Discover Project Data" button.
        assert!(out.contains(r#"href="https://example.org/project""#), "{out}");
        assert!(out.contains("Discover Project Data"), "{out}");
        // Both header buttons leave DPE, so they open in a new tab.
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains(r#"rel="noopener noreferrer""#), "{out}");
    }

    #[test]
    fn omits_the_img_entirely_when_the_project_has_no_cover() {
        // No cover means no `<img>` at all, as on the card.
        let out = render_project_header(&sample_project(), None).into_string();
        assert!(!out.contains("<img"), "no img element: {out}");
        assert!(!out.contains("/assets/images/"), "no cover URL: {out}");
        assert!(!out.contains("onerror"), "no JS-only fallback: {out}");
        // The placeholder takes its place at the hero's full height, visible.
        assert!(
            out.contains(r#"class="w-full bg-gray-100 flex items-center justify-center""#),
            "{out}"
        );
        // `flex`, not the `hidden` class list the with-cover branch emits. Matched on the
        // class list rather than the bare word, which also occurs in `aria-hidden`.
        assert!(!out.contains("justify-center hidden"), "placeholder is not hidden: {out}");
        // The rest of the hero is unaffected.
        assert!(out.contains("Sample Research Project"), "{out}");
        assert!(out.contains("Discover Project Data"), "{out}");
    }

    #[test]
    fn hides_the_placeholder_behind_the_img_when_a_cover_exists() {
        let out = render_project_header(&sample_project(), COVER).into_string();
        assert!(
            out.contains(r#"class="w-full bg-gray-100 items-center justify-center hidden""#),
            "{out}"
        );
        assert!(out.contains("this.nextElementSibling.style.display='flex'"), "{out}");
    }

    #[test]
    fn renders_image_credit_as_figcaption_when_present() {
        let proj = Project {
            image_credit: Some("© Fabrice Ducrest, Unil".to_string()),
            ..sample_project()
        };
        let out = render_project_header(&proj, COVER).into_string();
        assert!(out.contains("<figcaption"), "{out}");
        // Verbatim credit — "©" is preserved by Maud's auto-escaping splice.
        assert!(out.contains("© Fabrice Ducrest, Unil"), "{out}");
    }

    #[test]
    fn omits_the_image_credit_when_there_is_no_cover_to_credit() {
        // See the matching card test: cover and credit are independent fields
        // onboarded in separate steps, so a credit with no image is reachable.
        let proj = Project {
            image_credit: Some("© Fabrice Ducrest, Unil".to_string()),
            ..sample_project()
        };
        let out = render_project_header(&proj, None).into_string();
        assert!(!out.contains("<figcaption"), "{out}");
        assert!(!out.contains("© Fabrice Ducrest, Unil"), "{out}");
    }

    #[test]
    fn omits_figcaption_when_no_image_credit() {
        // sample_project() has image_credit: None.
        let out = render_project_header(&sample_project(), COVER).into_string();
        assert!(!out.contains("<figcaption"), "{out}");
    }
}
