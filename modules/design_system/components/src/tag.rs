use maud::{html, Markup, PreEscaped};

#[derive(Debug, Clone)]
pub enum TagVariant {
    Gray,
    Blue,
    Green,
}

impl TagVariant {
    fn carbon_kind(&self) -> &'static str {
        match self {
            TagVariant::Gray => "gray",
            TagVariant::Blue => "blue",
            TagVariant::Green => "green",
        }
    }

    fn test_id(&self) -> &'static str {
        match self {
            TagVariant::Gray => "tag-gray",
            TagVariant::Blue => "tag-blue",
            TagVariant::Green => "tag-green",
        }
    }
}

pub fn tag(text: impl Into<String>) -> Markup {
    tag_with_variant(text, TagVariant::Gray)
}

pub fn tag_with_variant(text: impl Into<String>, variant: TagVariant) -> Markup {
    tag_with_variant_and_testid(text, variant, None)
}

// TODO: variants and attributes missing. Verify what we actually need for our design system.

pub fn tag_with_variant_and_testid(
    text: impl Into<String>,
    variant: TagVariant,
    custom_test_id: Option<&str>,
) -> Markup {
    let text = text.into();
    let test_id = custom_test_id.unwrap_or(variant.test_id());
    let kind = variant.carbon_kind();

    html! {
        (PreEscaped(format!(r#"
        <script>
        if (!window.carbonTag) {{
            window.carbonTag = true;
            import('https://1.www.s81c.com/common/carbon/web-components/version/v2.33.0/tag.min.js');
        }}
        </script>
        <cds-tag type="{}" data-testid="{}">{}</cds-tag>
        "#, kind, test_id, text)))
    }
}
