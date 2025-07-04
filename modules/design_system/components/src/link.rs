use maud::{html, Markup, PreEscaped};

pub fn link(text: impl Into<String>, href: impl Into<String>) -> Markup {
    link_with_testid(text, href, None)
}

pub fn link_with_testid(text: impl Into<String>, href: impl Into<String>, custom_test_id: Option<&str>) -> Markup {
    let text = text.into();
    let href = href.into();
    let test_id = custom_test_id.unwrap_or("link");

    html! {
        (PreEscaped(format!(r#"
        <script>
        if (!window.carbonLink) {{
            window.carbonLink = true;
            import('https://1.www.s81c.com/common/carbon/web-components/version/v2.33.0/link.min.js');
        }}
        </script>
        <cds-link href="{}" data-testid="{}">{}</cds-link>
        "#, href, test_id, text)))
    }
}
