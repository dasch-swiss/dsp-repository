//! Loading-spinner tile (promoted from DPE).

use maud::{html, Markup};

/// Render a centered loading spinner.
pub fn loading() -> Markup {
    html! {
        div class="flex items-center justify-center w-100 h-100" {
            span class="loading loading-spinner loading-xl" {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_centered_spinner() {
        let out = loading().into_string();
        assert!(
            out.contains(r#"<div class="flex items-center justify-center w-100 h-100">"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<span class="loading loading-spinner loading-xl"></span>"#),
            "{out}"
        );
    }
}
