//! Card showcase.

use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonProps, ButtonVariant};
use mosaic_tiles::card::{card, card_body, card_footer, card_header, CardProps, CardVariant};
use mosaic_tiles::icon::{icon, IconChevronRight, Info, LinkExternal, Mail};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Card",
        "A flexible container component for grouping related content with optional header, body, and footer sections.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("card-variants", "Card Variants", "Available card styles: Simple, Bordered, Elevated, and AutoHover", variants()))
        (example("card-with_header_footer", "Card with Header and Footer", "Complete card with all sections and action buttons", with_header_footer()))
        (example("card-interactive", "Interactive Card", "Card with structured content for user profiles", interactive()))
        (example("card-with_images", "Cards with Images", "Various ways to incorporate images in cards", with_images()))
        (example("card-with_icons", "Cards with Icons", "Using icons to enhance card content", with_icons()))
    }
}

/// Shorthand: a card with a variant and body content.
fn c(variant: CardVariant, content: Markup) -> Markup {
    card(CardProps { variant, ..Default::default() }, content)
}

/// Shorthand: a primary button label.
fn btn(variant: ButtonVariant, label: Markup) -> Markup {
    button(ButtonProps { variant, ..Default::default() }, label)
}

fn variants() -> Markup {
    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            (c(CardVariant::Default, card_body("", html! {
                h3 class="text-lg font-semibold mb-2" { "Simple" }
                p class="text-neutral-600" { "This is a basic card with default styling." }
            })))
            (c(CardVariant::Bordered, card_body("", html! {
                h3 class="text-lg font-semibold mb-2" { "Bordered" }
                p class="text-neutral-600" { "This card has a visible border." }
            })))
            (c(CardVariant::Elevated, card_body("", html! {
                h3 class="text-lg font-semibold mb-2" { "Elevated" }
                p class="text-neutral-600" { "This card has a shadow to create elevation." }
            })))
            (c(CardVariant::AutoHover, card_body("", html! {
                h3 class="text-lg font-semibold mb-2" { "AutoHover" }
                p class="text-neutral-600" {
                    "This card starts with a bordered style and smoothly transitions to an elevated appearance when you hover over it."
                }
            })))
        }
    }
}

fn with_header_footer() -> Markup {
    c(
        CardVariant::Bordered,
        html! {
            (card_header(html! {
                h3 class="text-lg font-semibold" { "Card Title" }
                p class="text-sm text-neutral-500" { "Subtitle information" }
            }))
            (card_body("", html! {
                p class="text-neutral-700" {
                    "This card demonstrates header and footer sections. "
                    "Headers are great for titles and subtitles, while footers "
                    "work well for actions or metadata."
                }
            }))
            (card_footer(html! {
                div class="flex gap-2 justify-end" {
                    (btn(ButtonVariant::Secondary, html! { "Cancel" }))
                    (btn(ButtonVariant::Primary, html! { "Save" }))
                }
            }))
        },
    )
}

fn interactive() -> Markup {
    html! {
        div class="max-w-md" {
            (c(CardVariant::Elevated, html! {
                (card_header(html! { h3 class="text-lg font-semibold" { "User Profile" } }))
                (card_body("", html! {
                    div class="space-y-4" {
                        div {
                            label class="block text-sm font-medium mb-1" { "Name" }
                            p class="text-neutral-700" { "Jane Doe" }
                        }
                        div {
                            label class="block text-sm font-medium mb-1" { "Email" }
                            p class="text-neutral-700" { "jane@example.com" }
                        }
                        div {
                            label class="block text-sm font-medium mb-1" { "Role" }
                            p class="text-neutral-700" { "Administrator" }
                        }
                    }
                }))
                (card_footer(html! { div class="text-sm text-neutral-500" { "Last updated: 2026-01-22" } }))
            }))
        }
    }
}

fn with_images() -> Markup {
    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            (c(CardVariant::Bordered, html! {
                img src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba?w=400&h=250&fit=crop"
                    alt="Abstract blue and purple gradient" class="w-full h-48 object-cover";
                (card_body("", html! {
                    h3 class="text-lg font-semibold mb-2" { "Image Card" }
                    p class="text-neutral-600" { "Cards can feature images at the top for visual appeal." }
                }))
            }))
            (c(CardVariant::Elevated, html! {
                img src="https://images.unsplash.com/photo-1682687220795-796d3f6f7000?w=400&h=250&fit=crop"
                    alt="Colorful abstract art" class="w-full h-48 object-cover";
                (card_body("", html! {
                    h3 class="text-lg font-semibold mb-2" { "Gallery Item" }
                    p class="text-neutral-600 text-sm" { "Perfect for galleries or portfolios with actions." }
                }))
                (card_footer(html! {
                    div class="flex gap-2 justify-end" {
                        (btn(ButtonVariant::Secondary, html! { "Share" }))
                        (btn(ButtonVariant::Primary, html! { "View" }))
                    }
                }))
            }))
            (c(CardVariant::Bordered, html! {
                img src="https://images.unsplash.com/photo-1682695796497-31a44224d6d6?w=400&h=250&fit=crop"
                    alt="Nature landscape" class="w-full h-48 object-cover rounded-t-lg";
                (card_header(html! {
                    h3 class="text-lg font-semibold" { "Featured Article" }
                    p class="text-sm text-neutral-500" { "Published: Jan 22, 2026" }
                }))
                (card_body("", html! {
                    p class="text-neutral-600" { "Combine images with headers and body content for rich card layouts." }
                }))
            }))
        }
    }
}

fn with_icons() -> Markup {
    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            (c(CardVariant::Bordered, card_body("", html! {
                div class="flex items-start gap-3" {
                    div class="p-2 bg-info-100 rounded-lg" { (icon(Info, "w-5 h-5 text-info-600")) }
                    div class="flex-1" {
                        h3 class="text-lg font-semibold mb-2" { "Information" }
                        p class="text-neutral-600 text-sm" { "This card uses an icon to indicate informational content." }
                    }
                }
            })))
            (c(CardVariant::Elevated, html! {
                (card_header(html! {
                    div class="flex items-center gap-2" {
                        (icon(Mail, "w-5 h-5 text-neutral-600"))
                        h3 class="text-lg font-semibold" { "Messages" }
                    }
                }))
                (card_body("", html! {
                    p class="text-neutral-600" { "You have 3 new messages waiting for your review." }
                }))
                (card_footer(html! {
                    (btn(ButtonVariant::Primary, html! { "View Messages" (icon(IconChevronRight, "w-4 h-4 inline ml-1")) }))
                }))
            }))
            (c(CardVariant::Bordered, card_body("", html! {
                h3 class="text-lg font-semibold mb-2" { "Documentation" }
                p class="text-neutral-600 text-sm mb-4" { "Learn more about our API and integration guides." }
                a href="#" class="inline-flex items-center gap-1 text-primary-600 hover:text-primary-700 text-sm font-medium" {
                    "Read the docs"
                    (icon(LinkExternal, "w-4 h-4"))
                }
            })))
        }
    }
}
