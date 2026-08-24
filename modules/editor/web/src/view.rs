//! Hand-written HTML document shell. Composes a page's content with the global
//! header, footer and `<head>`.

use maud::{html, Markup, DOCTYPE};

use crate::components;

/// Who a page is rendered for, when anyone is signed in.
///
/// A named type rather than a second `Option<&str>` parameter beside
/// `traceparent`: two adjacent `Option<&str>` arguments are silently
/// interchangeable, and the swap would put a trace id in the header where a
/// name belongs and leave the correlation meta tag holding a person's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewer<'a> {
    /// The signed-in user's display name. Never the address — the header is on
    /// every page and in every screenshot of one.
    pub name: &'a str,
}

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
/// unhashed in dev, content-hashed in release. `viewer` is `None` on every page
/// reachable without a session, which is what keeps the sign-out control off
/// the login screens.
pub fn page(
    title: &str,
    traceparent: Option<&str>,
    css_href: &str,
    viewer: Option<Viewer<'_>>,
    content: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title, traceparent, css_href))
            body class="font-body" {
                div class="bg-gray-50 min-h-screen flex flex-col gap-4" {
                    (components::header(viewer))
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
        let out = page("My Title", None, "/assets/app.css", None, content).into_string();
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
        let out = page("t", None, "/assets/app.css", None, content).into_string();
        let header_at = out.find("DaSCH Metadata Editor").expect("header renders");
        let main_at = out.find("<main").expect("main renders");
        let footer_at = out.find("<footer").expect("footer renders");
        assert!(header_at < main_at && main_at < footer_at, "{out}");
    }

    #[test]
    fn emits_traceparent_meta_only_when_present() {
        let with = page("t", Some("00-abc-def-01"), "/assets/app.css", None, html! {}).into_string();
        assert!(with.contains(r#"<meta name="traceparent" content="00-abc-def-01">"#), "{with}");
        let without = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!without.contains("traceparent"), "{without}");
    }

    #[test]
    fn uses_the_href_it_is_given_so_the_hashed_stylesheet_is_not_bypassed() {
        // The release stylesheet is content-hashed and discovered at startup; a
        // hardcoded /assets/app.css here would serve a 404 in production.
        let out = page("t", None, "/assets/app.1a2b3c4d.css", None, html! {}).into_string();
        assert!(out.contains(r#"href="/assets/app.1a2b3c4d.css""#), "{out}");
        assert!(!out.contains(r#"href="/assets/app.css""#), "{out}");
    }

    #[test]
    fn loads_datastar_and_the_telemetry_beacon_as_modules() {
        // Both are vendored, so the paths are local; `type="module"` matters
        // because telemetry.js imports web-vitals relatively.
        let out = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(out.contains(r#"<script type="module" src="/vendor/datastar.js">"#), "{out}");
        assert!(out.contains(r#"<script type="module" src="/telemetry.js">"#), "{out}");
    }

    #[test]
    fn the_shell_carries_the_viewer_into_the_header() {
        // The sign-out control lives in the header, so `page` is where a signed-in
        // session becomes visible — and where a login screen stays anonymous.
        let signed_in =
            page("t", None, "/assets/app.css", Some(Viewer { name: "A Depositor" }), html! {}).into_string();
        assert!(signed_in.contains("A Depositor"), "{signed_in}");
        assert!(signed_in.contains("/logout"), "{signed_in}");

        let anonymous = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!anonymous.contains("/logout"), "{anonymous}");
    }

    #[test]
    fn escapes_the_title() {
        let out = page("<script>alert(1)</script>", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
