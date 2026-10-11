use cpe_core::util::{is_safe_external_url, MARKDOWN_OPTIONS};
use maud::{Markup, PreEscaped};
use pulldown_cmark::{html as cmark_html, Event, LinkType, Parser, Tag};

/// Shared CommonMark pipeline: parse `src`, filter raw HTML events, and
/// return the rendered HTML string. Tables are enabled via `ENABLE_TABLES`.
/// Raw HTML events (`Event::Html` and `Event::InlineHtml`) are stripped —
/// content must not inject arbitrary HTML.
///
/// When `sanitize_links` is `true`, every `Tag::Link`/`Tag::Image` start
/// event's `dest_url` is checked against [`is_safe_external_url`] (the same
/// allow-list `resource_detail`'s plain-text-URL and thumbnail paths use):
/// an unsafe link destination is rewritten to `#` (forcing `LinkType::Inline`
/// so an email autolink doesn't emit `href="mailto:#"`), an unsafe image
/// `src` is rewritten to `""` (an empty `src` never fetches). Everything
/// else — including the corresponding `TagEnd`, so tags stay balanced — is
/// passed through unchanged. This is used for richtext property values,
/// which are effectively untrusted content; project-authored narrative
/// Markdown (intros, home pages) is trusted prose and is never sanitized.
fn markdown_to_html(src: &str, sanitize_links: bool) -> String {
    let parser = Parser::new_ext(src, MARKDOWN_OPTIONS);
    let filtered = parser.filter(|event| !matches!(event, Event::Html(_) | Event::InlineHtml(_)));
    let mut html_out = String::new();
    if sanitize_links {
        let rewritten = filtered.map(|event| match event {
            Event::Start(Tag::Link { link_type, dest_url, title, id }) if !is_safe_external_url(&dest_url) => {
                Event::Start(Tag::Link { link_type: LinkType::Inline, dest_url: "#".into(), title, id })
            }
            Event::Start(Tag::Image { link_type, dest_url, title, id }) if !is_safe_external_url(&dest_url) => {
                Event::Start(Tag::Image { link_type, dest_url: "".into(), title, id })
            }
            other => other,
        });
        cmark_html::push_html(&mut html_out, rewritten);
    } else {
        cmark_html::push_html(&mut html_out, filtered);
    }
    html_out
}

/// Render a Markdown string to an HTML fragment (no wrapper element).
///
/// Uses the CommonMark+tables+raw-HTML-filtering pipeline: parse `src`,
/// strip all `Event::Html` and `Event::InlineHtml` events (block-level and
/// inline raw HTML), then emit with `push_html`. Link/image destinations are
/// **not** scheme-filtered — this is for trusted, project-authored Markdown
/// (narrative intros, home pages), where `#anchor`/relative links are
/// intentional. For richtext property values (untrusted-ish content), use
/// `render_richtext_fragment` instead.
pub fn render_markdown_fragment(src: &str) -> Markup {
    PreEscaped(markdown_to_html(src, false))
}

/// Render a richtext property value to an HTML fragment (no wrapper
/// element).
///
/// Same pipeline as [`render_markdown_fragment`], plus scheme-filtering of
/// every link/image destination via [`is_safe_external_url`] — richtext
/// values are effectively untrusted content. The allow-list accepts only
/// `http://`, `https://` and single-`/`-rooted destinations, so `mailto:`,
/// `#anchor` and relative links are neutralised too, by design: this is a
/// guard against unsafe schemes, and no shipped richtext data contains a
/// link of any of those shapes.
pub(crate) fn render_richtext_fragment(src: &str) -> Markup {
    PreEscaped(markdown_to_html(src, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests target render_markdown_fragment, which serves only the
    // narrative-for book-intro path (the richtext property-value path is
    // render_richtext_fragment, tested separately below). The raw-HTML-filtering
    // tests are security-sensitive and must stay covered.

    #[test]
    fn fragment_renders_h1_heading() {
        let out = render_markdown_fragment("# Hello").into_string();
        assert!(out.contains("<h1>Hello</h1>"), "expected <h1>Hello</h1>, got: {out}");
    }

    /// Heading attributes are an editorial-narrative option only
    /// (`util::EDITORIAL_MARKDOWN_OPTIONS`): here a `{#id}` stays text.
    #[test]
    fn fragment_keeps_a_heading_attribute_as_text() {
        let out = render_markdown_fragment("## Intro {#intro}").into_string();
        assert!(
            out.contains("<h2>Intro {#intro}</h2>"),
            "expected the literal {{#intro}} and no id, got: {out}"
        );
    }

    #[test]
    fn fragment_renders_emphasis() {
        let out = render_markdown_fragment("some *italic* text").into_string();
        assert!(out.contains("<em>italic</em>"), "expected <em>italic</em>, got: {out}");
    }

    #[test]
    fn fragment_renders_blockquote() {
        let out = render_markdown_fragment("> a quote").into_string();
        assert!(out.contains("<blockquote>"), "expected <blockquote>, got: {out}");
    }

    #[test]
    fn fragment_renders_gfm_table() {
        let src = "| a | b |\n|---|---|\n| 1 | 2 |";
        let out = render_markdown_fragment(src).into_string();
        assert!(out.contains("<table>"), "expected <table>, got: {out}");
    }

    /// Security guard: block-level `<script>` tags are stripped.
    /// `Event::Html` events are filtered before `push_html`.
    #[test]
    fn fragment_drops_script_block_html() {
        let src = "<script>alert(1)</script>";
        let out = render_markdown_fragment(src).into_string();
        assert!(
            !out.contains("<script"),
            "script tag must be dropped by raw-HTML filter, got: {out}"
        );
    }

    /// Security guard: block-level `<style>` tags are stripped.
    #[test]
    fn fragment_drops_style_block_html() {
        let src = "<style>body { color: red; }</style>";
        let out = render_markdown_fragment(src).into_string();
        assert!(
            !out.contains("<style"),
            "style tag must be dropped by raw-HTML filter, got: {out}"
        );
    }

    /// Security guard: inline raw HTML (e.g. event handlers) is stripped.
    /// `Event::InlineHtml` events are filtered before `push_html`.
    #[test]
    fn fragment_drops_inline_html() {
        let src = "some text <span onclick=\"x\">hi</span> more";
        let out = render_markdown_fragment(src).into_string();
        assert!(
            !out.contains("onclick"),
            "onclick attr must be dropped by raw-HTML filter, got: {out}"
        );
    }

    /// Confirm the fragment produces no outer wrapper element.
    #[test]
    fn fragment_has_no_section_wrapper() {
        let out = render_markdown_fragment("hello").into_string();
        assert!(
            !out.contains("<section"),
            "render_markdown_fragment must produce no section wrapper, got: {out}"
        );
    }

    /// Pin: render_markdown_fragment (the trusted-prose path) stays
    /// unsanitized — anchor and relative links are kept verbatim.
    #[test]
    fn markdown_fragment_keeps_anchor_and_relative_links() {
        let out = render_markdown_fragment("[a](#x) [b](notes.html)").into_string();
        assert!(out.contains(r##"href="#x""##), "expected #x href, got: {out}");
        assert!(out.contains(r#"href="notes.html""#), "expected notes.html href, got: {out}");
    }

    #[test]
    fn richtext_neutralises_javascript_link() {
        let out = render_richtext_fragment("[x](javascript:alert(1))").into_string();
        assert!(!out.contains("javascript:"), "got: {out}");
        assert!(out.contains(r##"<a href="#">x</a>"##), "got: {out}");
    }

    #[test]
    fn richtext_neutralises_scheme_variants() {
        // JAVASCRIPT:x (case-insensitive)
        let out = render_richtext_fragment("[x](JAVASCRIPT:x)").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
        assert!(
            !out.contains("JAVASCRIPT:x") && !out.to_lowercase().contains("href=\"javascript:x\""),
            "got: {out}"
        );

        // vbscript:x
        let out = render_richtext_fragment("[x](vbscript:x)").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
        assert!(!out.contains(r#"href="vbscript:x""#), "got: {out}");

        // data:text/html,x
        let out = render_richtext_fragment("[x](data:text/html,x)").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
        assert!(!out.contains("href=\"data:"), "got: {out}");

        // entity-encoded &#106;avascript:x
        let out = render_richtext_fragment("[x](&#106;avascript:x)").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
        assert!(!out.contains("href=\"javascript:"), "got: {out}");

        // bracketed autolink with leading space: "< javascript:x>" is not a
        // valid autolink/link destination in CommonMark, so it renders as
        // literal text with no href at all — assert no href carries the
        // scheme.
        let out = render_richtext_fragment("< javascript:x>").into_string();
        assert!(!out.contains("href=\"javascript:"), "got: {out}");

        // reference-style link
        let out = render_richtext_fragment("[x][r]\n\n[r]: javascript:x").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
        assert!(!out.contains(r#"href="javascript:x""#), "got: {out}");

        // autolink — pulldown-cmark renders the URL as link TEXT (inert,
        // escaped); assert on the attribute, never on whole-output text
        // absence.
        let out = render_richtext_fragment("<javascript:alert(1)>").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");
    }

    #[test]
    fn richtext_neutralises_protocol_relative() {
        let out = render_richtext_fragment("[x](//evil.example/a)").into_string();
        assert!(out.contains(r##"href="#""##), "got: {out}");

        let out = render_richtext_fragment("![a](//evil.example/p.png)").into_string();
        assert!(out.contains(r#"src="""#), "got: {out}");
    }

    #[test]
    fn richtext_email_autolink_is_inert() {
        let out = render_richtext_fragment("<a@b.example>").into_string();
        assert!(!out.contains("mailto:"), "got: {out}");
        assert!(out.contains(r##"href="#""##), "got: {out}");
    }

    #[test]
    fn richtext_keeps_https_link() {
        let out = render_richtext_fragment("[x](https://example.org/a)").into_string();
        assert!(out.contains(r#"href="https://example.org/a""#), "got: {out}");
    }

    #[test]
    fn richtext_keeps_root_relative_link() {
        let out = render_richtext_fragment("[x](/p/alice/characters)").into_string();
        assert!(out.contains(r#"href="/p/alice/characters""#), "got: {out}");
    }

    #[test]
    fn richtext_neutralises_javascript_image() {
        let out = render_richtext_fragment("![alt](javascript:x)").into_string();
        assert!(out.contains(r#"src="""#), "got: {out}");
        assert!(out.contains(r#"alt="alt""#), "got: {out}");
        assert!(!out.contains("javascript:"), "got: {out}");
    }

    #[test]
    fn richtext_rewrite_keeps_tags_balanced() {
        let out = render_richtext_fragment("[x](javascript:alert(1))").into_string();
        assert_eq!(out.matches("<a").count(), 1, "got: {out}");
        assert_eq!(out.matches("</a>").count(), 1, "got: {out}");
    }
}
