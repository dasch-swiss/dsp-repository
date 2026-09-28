use maud::{html, Markup};

use crate::RenderContext;

/// Renders a placeholder value ("MISSING" or "CALCULATED") styled in red when
/// `ctx.show_placeholder_values` is true. Otherwise renders nothing.
pub fn placeholder_value(value: &str, ctx: &RenderContext) -> Markup {
    html! {
        @if ctx.show_placeholder_values {
            span class="text-danger-600 font-mono text-xs" { (value) }
        }
    }
}

/// Returns true if the value should be rendered — either it is not a placeholder,
/// or placeholders are currently visible.
pub fn should_render_value(value: &str, ctx: &RenderContext) -> bool {
    !shared_metadata::is_placeholder(value) || ctx.show_placeholder_values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_values_always_render() {
        let ctx = RenderContext { show_placeholder_values: false };
        assert!(should_render_value("A real project name", &ctx));
        assert!(should_render_value("2020-01-01", &ctx));
    }

    #[test]
    fn placeholder_only_renders_when_context_says_to_show_placeholders() {
        let shown = RenderContext { show_placeholder_values: true };
        let hidden = RenderContext { show_placeholder_values: false };
        assert!(should_render_value("MISSING", &shown));
        assert!(!should_render_value("MISSING", &hidden));
    }

    #[test]
    fn placeholder_value_renders_only_when_shown() {
        let shown = RenderContext { show_placeholder_values: true };
        let hidden = RenderContext { show_placeholder_values: false };
        assert_ne!(placeholder_value("MISSING", &shown).into_string(), "");
        assert_eq!(placeholder_value("MISSING", &hidden).into_string(), "");
    }
}
