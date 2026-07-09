//! Card showcase.

use maud::{html, Markup, Render};
use mosaic_tiles::button::{button, ButtonVariant};
use mosaic_tiles::card::{card, card_body, card_footer, card_header, CardVariant};
use mosaic_tiles::icon::{icon, IconChevronRight, Info, LinkExternal, Mail};
use mosaic_tiles::ComponentBuilder;

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
        ({
            example(
                "card-variants",
                "Card Variants",
                "Available card styles: Simple, Bordered, Elevated, and AutoHover",
                variants(),
            )
        })
        ({
            example(
                "card-with_header_footer",
                "Card with Header and Footer",
                "Complete card with all sections and action buttons",
                with_header_footer(),
            )
        })
        ({
            example(
                "card-interactive",
                "Interactive Card",
                "Card with structured content for user profiles",
                interactive(),
            )
        })
        ({
            example(
                "card-with_images",
                "Cards with Images",
                "Various ways to incorporate images in cards",
                with_images(),
            )
        })
        ({
            example(
                "card-with_icons",
                "Cards with Icons",
                "Using icons to enhance card content",
                with_icons(),
            )
        })
    }
}

/// Shorthand: a card with a variant and body content.
fn c(variant: CardVariant, content: impl Render) -> Markup {
    card(content).variant(variant).build()
}

/// Shorthand: a button with a variant and label.
fn btn(variant: ButtonVariant, label: impl Render) -> impl Render {
    button(label).variant(variant)
}

fn variants() -> Markup {
    let specs = [
        (CardVariant::Default, "Simple", "This is a basic card with default styling."),
        (CardVariant::Bordered, "Bordered", "This card has a visible border."),
        (CardVariant::Elevated, "Elevated", "This card has a shadow to create elevation."),
        (
            CardVariant::AutoHover,
            "AutoHover",
            "This card starts with a bordered style and smoothly transitions to an elevated appearance when you hover over it.",
        ),
    ];
    let cards: Vec<Markup> = specs
        .into_iter()
        .map(|(variant, title, desc)| {
            let body = html! {
                h3 class="text-lg font-semibold mb-2" { (title) }
                p class="text-neutral-600" { (desc) }
            };
            c(variant, card_body(body))
        })
        .collect();
    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            @for card in &cards { (card) }
        }
    }
}

fn with_header_footer() -> Markup {
    let header = html! {
        h3 class="text-lg font-semibold" { "Card Title" }
        p class="text-sm text-neutral-500" { "Subtitle information" }
    };
    let body = html! {
        p class="text-neutral-700" {
            "This card demonstrates header and footer sections. "
            "Headers are great for titles and subtitles, while footers "
            "work well for actions or metadata."
        }
    };
    let footer = html! {
        div class="flex gap-2 justify-end" {
            ({
                btn(
                    ButtonVariant::Secondary,
                    html! {
                        "Cancel"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Primary,
                    html! {
                        "Save"
                    },
                )
            })
        }
    };
    let content = html! {
        (card_header(header))
        (card_body(body))
        (card_footer(footer))
    };
    c(CardVariant::Bordered, content)
}

fn interactive() -> Markup {
    let header = html! {
        h3 class="text-lg font-semibold" { "User Profile" }
    };
    let fields = [
        ("Name", "Jane Doe"),
        ("Email", "jane@example.com"),
        ("Role", "Administrator"),
    ];
    let body = html! {
        div class="space-y-4" {
            @for (label, value) in fields {
                div {
                    label class="block text-sm font-medium mb-1" { (label) }
                    p class="text-neutral-700" { (value) }
                }
            }
        }
    };
    let footer = html! {
        div class="text-sm text-neutral-500" { "Last updated: 2026-01-22" }
    };
    let content = html! {
        (card_header(header))
        (card_body(body))
        (card_footer(footer))
    };
    html! {
        div class="max-w-md" { (c(CardVariant::Elevated, content)) }
    }
}

fn with_images() -> Markup {
    let gradient_body = html! {
        h3 class="text-lg font-semibold mb-2" { "Image Card" }
        p class="text-neutral-600" { "Cards can feature images at the top for visual appeal." }
    };
    let gradient = html! {
        img src="/images/card-gradient.jpg"
            alt="Abstract blue and purple gradient"
            class="w-full h-48 object-cover";
        (card_body(gradient_body))
    };

    let abstract_body = html! {
        h3 class="text-lg font-semibold mb-2" { "Gallery Item" }
        p class="text-neutral-600 text-sm" { "Perfect for galleries or portfolios with actions." }
    };
    let abstract_footer = html! {
        div class="flex gap-2 justify-end" {
            ({
                btn(
                    ButtonVariant::Secondary,
                    html! {
                        "Share"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Primary,
                    html! {
                        "View"
                    },
                )
            })
        }
    };
    let abstract_card = html! {
        img src="/images/card-abstract.jpg"
            alt="Colorful abstract art"
            class="w-full h-48 object-cover";
        (card_body(abstract_body))
        (card_footer(abstract_footer))
    };

    let landscape_header = html! {
        h3 class="text-lg font-semibold" { "Featured Article" }
        p class="text-sm text-neutral-500" { "Published: Jan 22, 2026" }
    };
    let landscape_body = html! {
        p class="text-neutral-600" {
            "Combine images with headers and body content for rich card layouts."
        }
    };
    let landscape = html! {
        img src="/images/card-landscape.jpg"
            alt="Nature landscape"
            class="w-full h-48 object-cover rounded-t-lg";
        (card_header(landscape_header))
        (card_body(landscape_body))
    };

    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            (c(CardVariant::Bordered, gradient))
            (c(CardVariant::Elevated, abstract_card))
            (c(CardVariant::Bordered, landscape))
        }
    }
}

fn with_icons() -> Markup {
    let info_body = html! {
        div class="flex items-start gap-3" {
            div class="p-2 bg-info-100 rounded-lg" { (icon(Info, "w-5 h-5 text-info-600")) }
            div class="flex-1" {
                h3 class="text-lg font-semibold mb-2" { "Information" }
                p class="text-neutral-600 text-sm" {
                    "This card uses an icon to indicate informational content."
                }
            }
        }
    };

    let mail_header = html! {
        div class="flex items-center gap-2" {
            (icon(Mail, "w-5 h-5 text-neutral-600"))
            h3 class="text-lg font-semibold" { "Messages" }
        }
    };
    let mail_body = html! {
        p class="text-neutral-600" { "You have 3 new messages waiting for your review." }
    };
    let mail_footer_label = html! {
        "View Messages"
        (icon(IconChevronRight, "w-4 h-4 inline ml-1"))
    };
    let mail_footer = html! {
        (btn(ButtonVariant::Primary, mail_footer_label))
    };
    let mail = html! {
        (card_header(mail_header))
        (card_body(mail_body))
        (card_footer(mail_footer))
    };

    let docs_body = html! {
        h3 class="text-lg font-semibold mb-2" { "Documentation" }
        p class="text-neutral-600 text-sm mb-4" {
            "Learn more about our API and integration guides."
        }
        a   href="#"
            class="inline-flex items-center gap-1 text-primary-600 hover:text-primary-700 text-sm font-medium"
        { "Read the docs" (icon(LinkExternal, "w-4 h-4")) }
    };

    html! {
        div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3" {
            (c(CardVariant::Bordered, card_body(info_body)))
            (c(CardVariant::Elevated, mail))
            (c(CardVariant::Bordered, card_body(docs_body)))
        }
    }
}
