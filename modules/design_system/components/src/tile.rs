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
}

pub fn base(content: Markup) -> Markup {
    html! {
        div class=(TileVariant::Base.css_class()) {
            (content)
        }
    }
}

pub fn clickable(href: impl Into<String>, content: Markup) -> Markup {
    let href = href.into();
    html! {
        a class=(TileVariant::Clickable.css_class()) href=(href) {
            (content)
        }
    }
}
