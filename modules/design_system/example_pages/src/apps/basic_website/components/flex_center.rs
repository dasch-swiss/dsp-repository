use maud::{html, Markup};

/// Flex container with centered content
#[allow(dead_code)]
pub fn flex_center(content: Markup) -> Markup {
    html! {
        div class="flex justify-center" {
            (content)
        }
    }
}

/// Flex container with centered content and top margin
#[allow(dead_code)]
pub fn flex_center_with_margin(content: Markup) -> Markup {
    html! {
        div class="mt-10 flex justify-center" {
            (content)
        }
    }
}

/// Flex container with centered content and custom gap
#[allow(dead_code)]
pub fn flex_center_gap(gap_class: &str, content: Markup) -> Markup {
    html! {
        div class=(format!("flex justify-center {}", gap_class)) {
            (content)
        }
    }
}

/// Flex container with items center alignment
#[allow(dead_code)]
pub fn flex_items_center(content: Markup) -> Markup {
    html! {
        div class="flex items-center justify-center" {
            (content)
        }
    }
}

/// Flex container with space between
pub fn flex_between(content: Markup) -> Markup {
    html! {
        div class="flex items-center justify-between" {
            (content)
        }
    }
}
