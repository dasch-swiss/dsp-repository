// TODO: verify styling against carbon
// TODO: add accessibility features (proper focus indicators, keyboard navigation)
use maud::{html, Markup};

#[derive(Debug, Clone)]
pub enum TileVariant {
    Base,
    Clickable,
}

impl TileVariant {
    fn css_class(&self) -> &'static str {
        match self {
            TileVariant::Base => "dsp-tile",
            TileVariant::Clickable => "dsp-tile dsp-tile--clickable",
        }
    }

    fn test_id(&self) -> &'static str {
        match self {
            TileVariant::Base => "tile-base",
            TileVariant::Clickable => "tile-clickable",
        }
    }
}

pub fn base(content: Markup) -> Markup {
    base_with_testid(content, TileVariant::Base.test_id())
}

pub fn base_with_testid(content: Markup, test_id: &str) -> Markup {
    html! {
        div class=(TileVariant::Base.css_class()) data-testid=(test_id) {
            (content)
        }
    }
}

pub fn clickable(href: impl Into<String>, content: Markup) -> Markup {
    clickable_with_testid(href, content, TileVariant::Clickable.test_id())
}

pub fn clickable_with_testid(href: impl Into<String>, content: Markup, test_id: &str) -> Markup {
    let href = href.into();
    html! {
        a class=(TileVariant::Clickable.css_class()) href=(href) data-testid=(test_id) {
            (content)
        }
    }
}
