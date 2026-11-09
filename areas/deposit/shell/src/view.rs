//! Hand-written HTML document shell. Composes a page's content with the global
//! header, footer and `<head>`.

use std::borrow::Cow;

use maud::{html, Markup, DOCTYPE};

use crate::components;

/// Who a page is rendered for, when anyone is signed in.
///
/// A named type rather than a second `Option<&str>` beside `traceparent`: two
/// adjacent `Option<&str>` arguments are silently interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewer<'a> {
    /// The signed-in user's display name. Never the address — the header is on
    /// every page and in every screenshot of one.
    pub name: &'a str,
    pub menu: Menu,
    /// The request path, without the query; the header uses it only for `aria-current`.
    pub path: &'a str,
}

/// The header navigation, by role. A depositor is never shown an RDU-only
/// destination, so the variant decides which links exist at all.
///
/// RDU's review-queue item carries the count of pending submissions; an unreadable
/// count (`None`) is shown without a number, never as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Depositor,
    Rdu { pending_reviews: Option<usize> },
}

impl Menu {
    /// The menu for a viewer, decided here once for every capability.
    ///
    /// `pending_reviews` is awaited only for RDU, so a depositor's page costs no
    /// count query; a failed read is logged and never fails the page.
    pub async fn for_role<E: std::fmt::Display>(
        is_rdu: bool,
        pending_reviews: impl std::future::Future<Output = Result<usize, E>>,
    ) -> Menu {
        if !is_rdu {
            return Menu::Depositor;
        }
        let pending_reviews = match pending_reviews.await {
            Ok(count) => Some(count),
            Err(error) => {
                tracing::warn!(error = %error, "the review-queue count could not be read; rendering the header without it");
                None
            }
        };
        Menu::Rdu { pending_reviews }
    }

    /// The menu's `(label, href)` list, in display order.
    pub fn items(self) -> Vec<(Cow<'static, str>, &'static str)> {
        let projects = (Cow::Borrowed("Projects"), "/projects");
        match self {
            Menu::Depositor => vec![projects],
            Menu::Rdu { pending_reviews } => vec![
                projects,
                (
                    match pending_reviews {
                        Some(n) => Cow::Owned(format!("Review queue ({n})")),
                        None => Cow::Borrowed("Review queue"),
                    },
                    "/review",
                ),
                (Cow::Borrowed("Approved records"), "/approved"),
                (Cow::Borrowed("Accounts"), "/depositors"),
            ],
        }
    }
}

/// The `<head>`. No analytics script: the editor is authenticated and its
/// observability is OTel plus the first-party telemetry beacon.
fn head(title: &str, traceparent: Option<&str>, css_href: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            @if let Some(tp) = traceparent {
                meta name="traceparent" content=(tp);
            }
            // Google Fonts: Lora (display), Lato (body) and JetBrains Mono (shortcodes and codes) for the design tokens.
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="";
            link
                rel="stylesheet"
                href="https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,300;0,400;0,700;1,400&family=Lora:ital,wght@0,400;0,600;0,700;1,400&family=JetBrains+Mono:wght@400;600&display=swap";
            link rel="stylesheet" href=(css_href);
            title { (title) }
        }
    }
}

/// The full HTML document: `<head>` plus the body shell.
///
/// Callers add the " — DaSCH Metadata Editor" suffix to `title` themselves, so
/// a page can opt out. `css_href` is resolved once at startup, unhashed in dev
/// and content-hashed in release. `viewer` is `None` on every page reachable
/// without a session, which keeps the sign-out control off the login screens.
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
                // First in the body and outside the flex column, so hiding it leaves the
                // column's `gap-4` untouched.
                a   href="#main"
                    class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:bg-white focus:px-3 focus:py-2 focus:font-bold focus:shadow-xs focus:outline-2 focus:outline-primary-600"
                { "Skip to content" }
                div class="bg-neutral-50 min-h-screen flex flex-col gap-4" {
                    (components::header(viewer))
                    main id="main" class="flex-1 max-w-[1536px] mx-auto px-4 w-full" { (content) }
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
    fn one_banner_header_follows_a_skip_link_that_targets_main() {
        let viewer = Viewer {
            name: "A Depositor",
            menu: Menu::Depositor,
            path: "/projects",
        };
        for viewer in [None, Some(viewer)] {
            let out = page("t", None, "/assets/app.css", viewer, html! {}).into_string();
            assert_eq!(out.matches("<header").count(), 1, "{out}");
            let body = &out[out.find("<body").expect("body renders")..];
            let first_link = body.find("<a").expect("a link renders");
            assert!(body[first_link..].starts_with(r##"<a href="#main""##), "{out}");
            assert!(first_link < body.find("<header").expect("header renders"), "{out}");
            assert_eq!(out.matches(r#"id="main""#).count(), 1, "{out}");
            assert!(out.contains(r#"<main id="main""#), "{out}");
        }
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
        let viewer = Viewer {
            name: "A Depositor",
            menu: Menu::Depositor,
            path: "/projects",
        };
        let signed_in = page("t", None, "/assets/app.css", Some(viewer), html! {}).into_string();
        assert!(signed_in.contains("A Depositor"), "{signed_in}");
        assert!(signed_in.contains("/logout"), "{signed_in}");

        let anonymous = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!anonymous.contains("/logout"), "{anonymous}");
    }

    #[test]
    fn the_two_nav_landmarks_are_told_apart_by_name() {
        let viewer = Viewer {
            name: "A Depositor",
            menu: Menu::Depositor,
            path: "/projects",
        };
        let out = page("t", None, "/assets/app.css", Some(viewer), html! {}).into_string();
        let navs: Vec<&str> = out.split("<nav").skip(1).collect();
        assert_eq!(navs.len(), 2, "{out}");
        let labels: Vec<&str> = navs
            .iter()
            .map(|nav| {
                let tag = &nav[..nav.find('>').expect("nav tag closes")];
                let after = tag
                    .split(r#"aria-label=""#)
                    .nth(1)
                    .unwrap_or_else(|| panic!("nav without aria-label: {tag}"));
                &after[..after.find('"').expect("label closes")]
            })
            .collect();
        assert_ne!(labels[0], labels[1], "{out}");
    }

    #[test]
    fn escapes_the_title() {
        let out = page("<script>alert(1)</script>", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
