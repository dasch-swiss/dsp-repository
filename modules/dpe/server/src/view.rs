//! Hand-written HTML document shell. Composes the `dpe-web` page content with
//! the global header, footer, `<head>`, and the vendored Datastar + telemetry
//! scripts.

use maud::{html, Markup, DOCTYPE};

/// Extra markup for the end of `<head>`: the machine-readable metadata a page
/// carries, or nothing.
///
/// A newtype rather than a bare `Markup` so that swapping it with `page`'s
/// same-shaped `content` parameter is a compile error instead of a JSON-LD
/// block rendered into `<main>`.
pub(crate) struct HeadExtras(pub(crate) Markup);

/// The `<head>`: charset/viewport, the conditional `traceparent` correlation
/// meta tag, Google Fonts (Lora/Lato), the compiled stylesheet, the conditional
/// Fathom analytics script, the document title, and any page-specific extras.
fn head(
    title: &str,
    traceparent: Option<&str>,
    css_href: &str,
    fathom_site_id: Option<&str>,
    head_extras: HeadExtras,
) -> Markup {
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
            @if let Some(site_id) = fathom_site_id {
                script
                    src="https://cdn.usefathom.com/script.js"
                    data-site=(site_id)
                    data-spa="auto"
                    data-excluded-domains="localhost,repository.dev.dasch.swiss,repository.test.dasch.swiss,repository.stage.dasch.swiss"
                    defer {}
            }
            title { (title) }
            (head_extras.0)
        }
    }
}

/// The full HTML document: `<head>` plus the body shell (header, the page
/// `content` in `<main>`, footer) and the vendored module scripts.
pub(crate) fn page(
    title: &str,
    traceparent: Option<&str>,
    css_href: &str,
    fathom_site_id: Option<&str>,
    head_extras: HeadExtras,
    content: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title, traceparent, css_href, fathom_site_id, head_extras))
            body class="font-body" {
                div class="bg-gray-50 min-h-screen flex flex-col gap-4" {
                    (dpe_web::components::header())
                    main class="flex-1 dpe-max-layout-width mx-auto px-4 w-full overflow-x-clip" {
                        (content)
                    }
                    (dpe_web::components::footer())
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

    fn no_extras() -> HeadExtras {
        HeadExtras(html! {})
    }

    #[test]
    fn renders_doctype_html_head_and_body_shell() {
        let content = html! {
            p { "content" }
        };
        let out = page("My Title", None, "/assets/app.css", None, no_extras(), content).into_string();
        assert!(out.starts_with("<!DOCTYPE html><html lang=\"en\">"), "{out}");
        assert!(out.contains("<title>My Title</title>"), "{out}");
        assert!(out.contains(r#"<link rel="stylesheet" href="/assets/app.css">"#), "{out}");
        assert!(out.contains(r#"<body class="font-body">"#), "{out}");
        assert!(
            out.contains(
                r#"<main class="flex-1 dpe-max-layout-width mx-auto px-4 w-full overflow-x-clip"><p>content</p></main>"#
            ),
            "{out}"
        );
        assert!(out.contains(r#"<script type="module" src="/vendor/datastar.js">"#), "{out}");
        assert!(out.contains(r#"<script type="module" src="/telemetry.js">"#), "{out}");
    }

    #[test]
    fn emits_traceparent_meta_when_present() {
        let with = page("t", Some("00-abc-def-01"), "/assets/app.css", None, no_extras(), html! {}).into_string();
        assert!(with.contains(r#"<meta name="traceparent" content="00-abc-def-01">"#), "{with}");
        let without = page("t", None, "/assets/app.css", None, no_extras(), html! {}).into_string();
        assert!(!without.contains("traceparent"), "{without}");
    }

    #[test]
    fn emits_fathom_script_only_with_site_id() {
        let with = page("t", None, "/assets/app.css", Some("ABCDEF"), no_extras(), html! {}).into_string();
        assert!(with.contains(r#"src="https://cdn.usefathom.com/script.js""#), "{with}");
        assert!(with.contains(r#"data-site="ABCDEF""#), "{with}");
        assert!(with.contains(r#"data-spa="auto""#), "{with}");
        assert!(with.contains("data-excluded-domains="), "{with}");
        let without = page("t", None, "/assets/app.css", None, no_extras(), html! {}).into_string();
        assert!(!without.contains("usefathom"), "{without}");
    }

    /// Last in `<head>`, after the title and the scripts, so a page can add to
    /// the head without displacing anything already there.
    #[test]
    fn head_extras_render_at_the_end_of_the_head() {
        let extras = HeadExtras(html! {
            meta name="DC.title" content="A Project";
        });
        let out = page("t", None, "/assets/app.css", Some("ABCDEF"), extras, html! {}).into_string();
        let title = out.find("<title>t</title>").expect("a title");
        let extra = out.find(r#"<meta name="DC.title""#).expect("the extra");
        let head_end = out.find("</head>").expect("the end of the head");
        assert!(title < extra && extra < head_end, "{out}");
    }

    #[test]
    fn includes_google_fonts() {
        let out = page("t", None, "/assets/app.css", None, no_extras(), html! {}).into_string();
        assert!(
            out.contains(r#"<link rel="preconnect" href="https://fonts.googleapis.com">"#),
            "{out}"
        );
        assert!(out.contains("family=Lora"), "{out}");
        assert!(out.contains("family=Lato"), "{out}");
    }
}
