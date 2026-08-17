//! Hand-written HTML document shell. Composes a page's content with the global
//! header, footer and `<head>`.

use maud::{html, Markup, DOCTYPE};

use crate::components;

/// The `<head>`: charset/viewport, the conditional `traceparent` correlation
/// meta tag, Google Fonts (Lora/Lato, matching the Mosaic design tokens), the
/// compiled stylesheet, and the document title.
///
/// No analytics script: the editor is authenticated and its observability comes
/// from OTel plus the first-party telemetry beacon, not a third-party tracker.
fn head(title: &str, traceparent: Option<&str>, css_href: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            @if let Some(tp) = traceparent {
                meta name="traceparent" content=(tp);
            }
            // Google Fonts: Lora (display) and Lato (body) for the design tokens.
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="";
            link
                rel="stylesheet"
                href="https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,300;0,400;0,700;1,400&family=Lora:ital,wght@0,400;0,600;0,700;1,400&display=swap";
            link rel="stylesheet" href=(css_href);
            title { (title) }
        }
    }
}

/// The full HTML document: `<head>` plus the body shell (header, the page
/// `content` in `<main>`, footer).
///
/// `title` is the document title as it should appear; callers add the " — DaSCH
/// Metadata Editor" suffix themselves rather than having it appended here, so a
/// page can opt out.
///
/// `traceparent` is the current server span, rendered as a meta tag for
/// client-side trace correlation. `css_href` is resolved once at startup —
/// unhashed in dev, content-hashed in release.
pub fn page(title: &str, traceparent: Option<&str>, css_href: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title, traceparent, css_href))
            body class="font-body" {
                div class="bg-gray-50 min-h-screen flex flex-col gap-4" {
                    (components::header())
                    main class="flex-1 max-w-[1536px] mx-auto px-4 w-full" { (content) }
                    (components::footer())
                }
                script type="module" src="/vendor/datastar.js" {}
                script type="module" src="/telemetry.js" {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use maud::html;

    use super::*;

    #[test]
    fn renders_doctype_html_head_and_body_shell() {
        let content = html! {
            p { "content" }
        };
        let out = page("My Title", None, "/assets/app.css", content).into_string();
        assert!(out.starts_with("<!DOCTYPE html><html lang=\"en\">"), "{out}");
        assert!(out.contains("<title>My Title</title>"), "{out}");
        assert!(out.contains(r#"<link rel="stylesheet" href="/assets/app.css">"#), "{out}");
        assert!(out.contains(r#"<body class="font-body">"#), "{out}");
        assert!(out.contains("<p>content</p>"), "{out}");
    }

    #[test]
    fn wraps_content_in_main_between_header_and_footer() {
        let content = html! {
            p { "content" }
        };
        let out = page("t", None, "/assets/app.css", content).into_string();
        let header_at = out.find("DaSCH Metadata Editor").expect("header renders");
        let main_at = out.find("<main").expect("main renders");
        let footer_at = out.find("<footer").expect("footer renders");
        assert!(header_at < main_at && main_at < footer_at, "{out}");
    }

    #[test]
    fn emits_traceparent_meta_only_when_present() {
        let with = page("t", Some("00-abc-def-01"), "/assets/app.css", html! {}).into_string();
        assert!(with.contains(r#"<meta name="traceparent" content="00-abc-def-01">"#), "{with}");
        let without = page("t", None, "/assets/app.css", html! {}).into_string();
        assert!(!without.contains("traceparent"), "{without}");
    }

    #[test]
    fn uses_the_href_it_is_given_so_the_hashed_stylesheet_is_not_bypassed() {
        // The release stylesheet is content-hashed and discovered at startup; a
        // hardcoded /assets/app.css here would serve a 404 in production.
        let out = page("t", None, "/assets/app.1a2b3c4d.css", html! {}).into_string();
        assert!(out.contains(r#"href="/assets/app.1a2b3c4d.css""#), "{out}");
        assert!(!out.contains(r#"href="/assets/app.css""#), "{out}");
    }

    #[test]
    fn loads_datastar_and_the_telemetry_beacon_as_modules() {
        // Both are vendored, so the paths are local; `type="module"` matters
        // because telemetry.js imports web-vitals relatively.
        let out = page("t", None, "/assets/app.css", html! {}).into_string();
        assert!(out.contains(r#"<script type="module" src="/vendor/datastar.js">"#), "{out}");
        assert!(out.contains(r#"<script type="module" src="/telemetry.js">"#), "{out}");
    }

    #[test]
    fn escapes_the_title() {
        let out = page("<script>alert(1)</script>", None, "/assets/app.css", html! {}).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
