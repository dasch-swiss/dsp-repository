use cpe_core::url::Base;
use cpe_core::util::{is_safe_external_url, MARKDOWN_OPTIONS};
use maud::{Markup, PreEscaped};
use pulldown_cmark::{html as cmark_html, Event, LinkType, Parser, Tag};

/// The events of `src` without raw HTML (`Event::Html`, `Event::InlineHtml`):
/// content must not inject arbitrary HTML. Tables are enabled.
fn events(src: &str) -> impl Iterator<Item = Event<'_>> {
    Parser::new_ext(src, MARKDOWN_OPTIONS).filter(|event| !matches!(event, Event::Html(_) | Event::InlineHtml(_)))
}

/// Render a Markdown string to an HTML fragment (no wrapper element).
///
/// For trusted, project-authored Markdown (a record's narrative intro):
/// link and image destinations are **not** scheme-filtered, so `#anchor` and
/// relative links stay. A destination that starts with one `/` is a path of
/// CPE and gets `base` in front (`narrative_outline::rebase_links`); the
/// content names no mount. For a richtext property value, use
/// `render_richtext_fragment`.
pub fn render_markdown_fragment(base: &Base, src: &str) -> Markup {
    let mut events: Vec<Event<'_>> = events(src).collect();
    cpe_core::narrative_outline::rebase_links(&mut events, base);
    let mut html_out = String::new();
    cmark_html::push_html(&mut html_out, events.into_iter());
    PreEscaped(html_out)
}

/// Render a richtext property value to an HTML fragment (no wrapper
/// element).
///
/// Richtext values are effectively untrusted content, so every link and
/// image destination is held to [`is_safe_external_url`] (the allow-list
/// `resource_detail`'s plain-text-URL and thumbnail paths use): an unsafe
/// link destination becomes `#` (as `LinkType::Inline`, so an email autolink
/// does not emit `href="mailto:#"`), an unsafe image `src` becomes `""`,
/// which never fetches. The `TagEnd` is passed through, so tags stay
/// balanced. The allow-list accepts only `http://`, `https://` and
/// single-`/`-rooted destinations, so `mailto:`, `#anchor` and relative
/// links are neutralised too, by design: this is a guard against unsafe
/// schemes, and no shipped richtext data contains a link of any of those
/// shapes. A value is archive data, not narrative: it gets no base.
pub(crate) fn render_richtext_fragment(src: &str) -> Markup {
    let rewritten = events(src).map(|event| match event {
        Event::Start(Tag::Link { link_type, dest_url, title, id }) if !is_safe_external_url(&dest_url) => {
            Event::Start(Tag::Link { link_type: LinkType::Inline, dest_url: "#".into(), title, id })
        }
        Event::Start(Tag::Image { link_type, dest_url, title, id }) if !is_safe_external_url(&dest_url) => {
            Event::Start(Tag::Image { link_type, dest_url: "".into(), title, id })
        }
        other => other,
    });
    let mut html_out = String::new();
    cmark_html::push_html(&mut html_out, rewritten);
    PreEscaped(html_out)
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
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "# Hello").into_string();
        assert!(out.contains("<h1>Hello</h1>"), "expected <h1>Hello</h1>, got: {out}");
    }

    /// Heading attributes are an editorial-narrative option only
    /// (`util::EDITORIAL_MARKDOWN_OPTIONS`): here a `{#id}` stays text.
    #[test]
    fn fragment_keeps_a_heading_attribute_as_text() {
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "## Intro {#intro}").into_string();
        assert!(
            out.contains("<h2>Intro {#intro}</h2>"),
            "expected the literal {{#intro}} and no id, got: {out}"
        );
    }

    #[test]
    fn fragment_renders_emphasis() {
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "some *italic* text").into_string();
        assert!(out.contains("<em>italic</em>"), "expected <em>italic</em>, got: {out}");
    }

    #[test]
    fn fragment_renders_blockquote() {
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "> a quote").into_string();
        assert!(out.contains("<blockquote>"), "expected <blockquote>, got: {out}");
    }

    #[test]
    fn fragment_renders_gfm_table() {
        let src = "| a | b |\n|---|---|\n| 1 | 2 |";
        let out = render_markdown_fragment(&cpe_core::url::ROOT, src).into_string();
        assert!(out.contains("<table>"), "expected <table>, got: {out}");
    }

    /// Security guard: block-level `<script>` tags are stripped.
    /// `Event::Html` events are filtered before `push_html`.
    #[test]
    fn fragment_drops_script_block_html() {
        let src = "<script>alert(1)</script>";
        let out = render_markdown_fragment(&cpe_core::url::ROOT, src).into_string();
        assert!(
            !out.contains("<script"),
            "script tag must be dropped by raw-HTML filter, got: {out}"
        );
    }

    /// Security guard: block-level `<style>` tags are stripped.
    #[test]
    fn fragment_drops_style_block_html() {
        let src = "<style>body { color: red; }</style>";
        let out = render_markdown_fragment(&cpe_core::url::ROOT, src).into_string();
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
        let out = render_markdown_fragment(&cpe_core::url::ROOT, src).into_string();
        assert!(
            !out.contains("onclick"),
            "onclick attr must be dropped by raw-HTML filter, got: {out}"
        );
    }

    /// Confirm the fragment produces no outer wrapper element.
    #[test]
    fn fragment_has_no_section_wrapper() {
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "hello").into_string();
        assert!(
            !out.contains("<section"),
            "render_markdown_fragment must produce no section wrapper, got: {out}"
        );
    }

    /// Pin: render_markdown_fragment (the trusted-prose path) stays
    /// unsanitized — anchor and relative links are kept verbatim.
    #[test]
    fn markdown_fragment_keeps_anchor_and_relative_links() {
        let out = render_markdown_fragment(&cpe_core::url::ROOT, "[a](#x) [b](notes.html)").into_string();
        assert!(out.contains(r##"href="#x""##), "expected #x href, got: {out}");
        assert!(out.contains(r#"href="notes.html""#), "expected notes.html href, got: {out}");
    }

    #[test]
    fn a_link_with_one_leading_slash_gets_the_base() {
        let base = Base::parse("/cpe").expect("/cpe is a base");
        let out = render_markdown_fragment(&base, "[a](/incunabula/books) ![i](/incunabula/_assets/assets/x.webp)")
            .into_string();
        assert!(out.contains(r#"href="/cpe/incunabula/books""#), "got: {out}");
        assert!(out.contains(r#"src="/cpe/incunabula/_assets/assets/x.webp""#), "got: {out}");
    }

    #[test]
    fn under_a_base_any_other_link_is_left_as_it_is() {
        let base = Base::parse("/cpe").expect("/cpe is a base");
        let src = "[a](//evil.example/x) [b](https://example.org/a) [c](#x) [d](notes.html) <a@b.example>";
        let out = render_markdown_fragment(&base, src).into_string();
        for kept in [
            r#"href="//evil.example/x""#,
            r#"href="https://example.org/a""#,
            r##"href="#x""##,
            r#"href="notes.html""#,
            r#"href="mailto:a@b.example""#,
        ] {
            assert!(out.contains(kept), "expected {kept}, got: {out}");
        }
        assert!(!out.contains("/cpe"), "got: {out}");
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
