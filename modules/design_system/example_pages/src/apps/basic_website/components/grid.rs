use maud::{html, Markup};

#[derive(Debug, Clone, Copy)]
pub enum GridColumns {
    Two,   // lg:grid-cols-2
    Three, // lg:grid-cols-3
    #[allow(dead_code)]
    Four, // lg:grid-cols-4
}

impl GridColumns {
    fn css_class(&self) -> &'static str {
        match self {
            GridColumns::Two => "grid grid-cols-1 gap-8 lg:grid-cols-2",
            GridColumns::Three => "grid grid-cols-1 gap-8 lg:grid-cols-3",
            GridColumns::Four => "grid grid-cols-1 gap-x-8 gap-y-16 lg:grid-cols-4",
        }
    }

    fn responsive_class(&self) -> &'static str {
        match self {
            GridColumns::Two => "lg:grid-cols-2",
            GridColumns::Three => "lg:grid-cols-3",
            GridColumns::Four => "lg:grid-cols-4",
        }
    }
}

/// Standard responsive grid
pub fn grid(columns: GridColumns, content: Markup) -> Markup {
    html! {
        div class=(columns.css_class()) {
            (content)
        }
    }
}

/// Grid with max-width constraint
pub fn grid_constrained(columns: GridColumns, content: Markup) -> Markup {
    html! {
        div class=(format!("mx-auto mt-16 grid max-w-2xl grid-cols-1 gap-8 lg:max-w-none {}", columns.responsive_class())) {
            (content)
        }
    }
}

/// Grid for cards (projects, news) with larger vertical gaps
pub fn card_grid(columns: GridColumns, content: Markup) -> Markup {
    html! {
        div class=(format!("mx-auto mt-16 grid max-w-2xl grid-cols-1 gap-x-8 gap-y-20 lg:mx-0 lg:max-w-none {}", columns.responsive_class())) {
            (content)
        }
    }
}

/// Grid for project listings (equal gaps)
pub fn project_grid(content: Markup) -> Markup {
    html! {
        div class="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3" {
            (content)
        }
    }
}

/// Grid for statistics (centered text, 4 columns)
pub fn stats_grid(content: Markup) -> Markup {
    html! {
        div class="grid grid-cols-1 gap-x-8 gap-y-16 text-center lg:grid-cols-4" {
            (content)
        }
    }
}
