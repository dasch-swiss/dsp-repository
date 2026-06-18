//! Hand-written HTML document shell, replacing `leptos_meta` + the Leptos
//! `shell`/`App`. Composes the `dpe-web` page content with the global header,
//! footer, `<head>`, and the vendored Datastar + telemetry scripts.

use maud::{html, Markup, DOCTYPE};

/// The `<head>`: charset/viewport, the conditional `traceparent` correlation
/// meta tag, Google Fonts (Lora/Lato), the compiled stylesheet, the conditional
/// Fathom analytics script, and the document title.
fn head(title: &str, traceparent: Option<&str>, css_href: &str, fathom_site_id: Option<&str>) -> Markup {
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
            link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,300;0,400;0,700;1,400&family=Lora:ital,wght@0,400;0,600;0,700;1,400&display=swap";
            link rel="stylesheet" href=(css_href);
            @if let Some(site_id) = fathom_site_id {
                script src="https://cdn.usefathom.com/script.js"
                       data-site=(site_id)
                       data-spa="auto"
                       data-excluded-domains="localhost,repository.dev.dasch.swiss,repository.test.dasch.swiss,repository.stage.dasch.swiss"
                       defer {}
            }
            title { (title) }
        }
    }
}

/// The full HTML document: `<head>` plus the body shell (header, the page
/// `content` in `<main>`, footer) and the vendored module scripts.
pub fn page(
    title: &str,
    traceparent: Option<&str>,
    css_href: &str,
    fathom_site_id: Option<&str>,
    content: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title, traceparent, css_href, fathom_site_id))
            body class="font-body" {
                div class="bg-gray-50 min-h-screen flex flex-col gap-4" {
                    (dpe_web::components::header())
                    main class="flex-1 dpe-max-layout-width mx-auto px-4 w-full" {
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

    #[test]
    fn renders_doctype_html_head_and_body_shell() {
        let out = page("My Title", None, "/assets/app.css", None, html! { p { "content" } }).into_string();
        assert!(out.starts_with("<!DOCTYPE html><html lang=\"en\">"), "{out}");
        assert!(out.contains("<title>My Title</title>"), "{out}");
        assert!(out.contains(r#"<link rel="stylesheet" href="/assets/app.css">"#), "{out}");
        assert!(out.contains(r#"<body class="font-body">"#), "{out}");
        assert!(
            out.contains(r#"<main class="flex-1 dpe-max-layout-width mx-auto px-4 w-full"><p>content</p></main>"#),
            "{out}"
        );
        assert!(out.contains(r#"<script type="module" src="/vendor/datastar.js">"#), "{out}");
        assert!(out.contains(r#"<script type="module" src="/telemetry.js">"#), "{out}");
    }

    #[test]
    fn emits_traceparent_meta_when_present() {
        let with = page("t", Some("00-abc-def-01"), "/assets/app.css", None, html! {}).into_string();
        assert!(with.contains(r#"<meta name="traceparent" content="00-abc-def-01">"#), "{with}");
        let without = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!without.contains("traceparent"), "{without}");
    }

    #[test]
    fn emits_fathom_script_only_with_site_id() {
        let with = page("t", None, "/assets/app.css", Some("ABCDEF"), html! {}).into_string();
        assert!(with.contains(r#"src="https://cdn.usefathom.com/script.js""#), "{with}");
        assert!(with.contains(r#"data-site="ABCDEF""#), "{with}");
        assert!(with.contains(r#"data-spa="auto""#), "{with}");
        assert!(with.contains("data-excluded-domains="), "{with}");
        let without = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(!without.contains("usefathom"), "{without}");
    }

    #[test]
    fn includes_google_fonts() {
        let out = page("t", None, "/assets/app.css", None, html! {}).into_string();
        assert!(
            out.contains(r#"<link rel="preconnect" href="https://fonts.googleapis.com">"#),
            "{out}"
        );
        assert!(out.contains("family=Lora"), "{out}");
        assert!(out.contains("family=Lato"), "{out}");
    }
}
