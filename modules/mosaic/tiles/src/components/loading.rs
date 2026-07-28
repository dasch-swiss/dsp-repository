//! Loading-spinner tile (promoted from DPE).

use maud::{html, Markup};

/// Render a centered loading spinner. Carries `role="status"` and a
/// visually-hidden "Loading" label so assistive technology announces the
/// pending state (WCAG 4.1.3).
#[must_use]
pub fn loading() -> Markup {
    html! {
        div class="flex items-center justify-center w-100 h-100" role="status" {
            span class="loading loading-spinner loading-xl" {}
            span class="sr-only" { "Loading" }
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
            out.contains(r#"<div class="flex items-center justify-center w-100 h-100""#),
            "{out}"
        );
        assert!(
            out.contains(r#"<span class="loading loading-spinner loading-xl"></span>"#),
            "{out}"
        );
        assert!(out.contains(r#"role="status""#), "spinner must announce loading state: {out}");
        assert!(out.contains(r#"<span class="sr-only">Loading</span>"#), "{out}");
    }
}
