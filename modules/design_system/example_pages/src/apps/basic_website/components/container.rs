use maud::{html, Markup};

#[derive(Debug, Clone, Copy)]
pub enum ContainerWidth {
    #[allow(dead_code)]
    Narrow, // max-w-2xl
    Medium, // max-w-4xl
    #[allow(dead_code)]
    Wide, // max-w-7xl (default)
}

impl ContainerWidth {
    fn css_class(&self) -> &'static str {
        match self {
            ContainerWidth::Narrow => "mx-auto max-w-2xl",
            ContainerWidth::Medium => "mx-auto max-w-4xl",
            ContainerWidth::Wide => "mx-auto max-w-7xl",
        }
    }
}

/// Container with constrained width
pub fn container(width: ContainerWidth, content: Markup) -> Markup {
    html! {
        div class=(width.css_class()) {
            (content)
        }
    }
}

/// Container with constrained width and centered text
#[allow(dead_code)]
pub fn container_centered(width: ContainerWidth, content: Markup) -> Markup {
    html! {
        div class=(format!("{} lg:text-center", width.css_class())) {
            (content)
        }
    }
}
