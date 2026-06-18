//! Design-tokens showcase: color scales and typography. There is no tile here —
//! the page renders the tokens directly from `tokens.css` via CSS custom
//! properties.

use maud::{html, Markup};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Design Tokens",
        "DaSCH brand colors, typography, and neutral scale defined as CSS custom properties via Tailwind v4 @theme.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("theme-colors", "Color Scales", "Semantic color tokens (50–950) derived from DaSCH brand values in OKLCH. Each row shows the full scale for one semantic color.", colors()))
        (example("theme-typography", "Typography", "Font tokens: font-display (Lora) for headings and font-body (Lato) for body text, with fallback stacks.", typography()))
    }
}

const STOPS: &[&str] = &[
    "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950",
];

const SCALES: &[(&str, &str)] = &[
    ("primary", "#336790"),
    ("secondary", "#74A2CF"),
    ("success", "#31837B"),
    ("danger", "#9E484D"),
    ("warning", "#E39E22"),
    ("info", "#74A2CF"),
    ("accent", "#706DA6"),
    ("neutral", "#3B4856"),
];

fn color_scale(name: &str, base_hex: &str) -> Markup {
    html! {
        div class="mb-6" {
            div class="flex items-baseline gap-2 mb-2" {
                span class="text-sm font-semibold" { (name) }
                span class="text-xs text-neutral-500 font-mono" { (base_hex) }
            }
            div class="flex gap-1" {
                @for stop in STOPS {
                    @let is_light = matches!(*stop, "50" | "100" | "200" | "300");
                    @let text_color = if is_light { "#1a1a1a" } else { "#ffffff" };
                    @let style = format!(
                        "background-color: var(--color-{name}-{stop}); color: {text_color}; min-width: 4rem; min-height: 4rem"
                    );
                    div class="flex flex-col items-center justify-end p-2 rounded" style=(style) {
                        span class="text-xs font-mono" { (stop) }
                    }
                }
            }
        }
    }
}

fn colors() -> Markup {
    html! {
        div {
            p class="text-sm text-neutral-600 mb-4" {
                "Colors are defined as CSS custom properties (e.g. "
                code class="text-sm bg-neutral-100 px-1 rounded" { "var(--color-primary-500)" }
                ") and are also available as Tailwind utilities (e.g. "
                code class="text-sm bg-neutral-100 px-1 rounded" { "bg-primary-500" } ")."
            }
            @for (name, hex) in SCALES {
                (color_scale(name, hex))
            }
        }
    }
}

fn typography() -> Markup {
    html! {
        div class="space-y-8" {
            div {
                h4 class="text-sm font-semibold text-neutral-500 mb-3" {
                    "font-display"
                    span class="font-normal text-neutral-400" { " — Lora, Georgia, Times New Roman, serif" }
                }
                div style="font-family: var(--font-display)" {
                    p class="text-4xl mb-2" { "The quick brown fox jumps over the lazy dog" }
                    p class="text-2xl mb-2" { "Heading level two in display font" }
                    p class="text-lg" { "Smaller heading in Lora with serif fallbacks" }
                }
            }
            div {
                h4 class="text-sm font-semibold text-neutral-500 mb-3" {
                    "font-body"
                    span class="font-normal text-neutral-400" { " — Lato, Helvetica Neue, Arial, sans-serif" }
                }
                div style="font-family: var(--font-body)" {
                    p class="text-base mb-2" {
                        "Body text set in Lato. This is the default font for paragraph content, form labels, and UI elements. The fallback chain ensures readable sans-serif text even without web fonts loaded."
                    }
                    p class="text-sm text-neutral-600" {
                        "Smaller body text for captions, metadata, and secondary information."
                    }
                }
            }
        }
    }
}
