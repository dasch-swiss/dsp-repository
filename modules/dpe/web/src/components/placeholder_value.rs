use maud::{html, Markup};

/// Renders a placeholder value ("MISSING" or "CALCULATED") styled in red when
/// `DPE_SHOW_PLACEHOLDER_VALUES` is true. Otherwise renders nothing.
pub fn placeholder_value(value: &str) -> Markup {
    html! {
        @if dpe_core::show_placeholder_values() {
            span class="text-danger-600 font-mono text-xs" { (value) }
        }
    }
}

/// Returns true if the value should be rendered — either it is not a placeholder,
/// or placeholders are currently visible.
pub fn should_render_value(value: &str) -> bool {
    !dpe_core::is_placeholder(value) || dpe_core::show_placeholder_values()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_values_always_render() {
        // A non-placeholder value renders regardless of the global flag.
        assert!(should_render_value("A real project name"));
        assert!(should_render_value("2020-01-01"));
    }

    #[test]
    fn placeholder_value_markup_is_empty_when_hidden() {
        // Default config hides placeholders, so nothing is rendered.
        if !dpe_core::show_placeholder_values() {
            assert_eq!(placeholder_value("MISSING").into_string(), "");
        }
    }
}
