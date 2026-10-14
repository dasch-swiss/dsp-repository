use maud::{html, Markup};

pub fn not_found() -> Markup {
    html! {
        h1 class="font-display text-2xl mb-2" { "Page not found" }
        p { "The page you asked for does not exist." }
    }
}
