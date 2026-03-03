mod components;

pub use components::theme_provider::ThemeProvider;
pub use components::*;

#[cfg(feature = "breadcrumb")]
pub mod breadcrumb {
    pub use crate::components::breadcrumb::*;
}

#[cfg(feature = "button_group")]
pub mod button_group {
    pub use crate::components::button_group::*;
}

#[cfg(feature = "icon")]
pub mod icon {
    pub use crate::components::icon::*;
}

#[cfg(feature = "link")]
pub mod link {
    pub use crate::components::link::*;
}

#[cfg(feature = "popover")]
pub mod popover {
    pub use crate::components::popover::*;
}

#[cfg(feature = "tabs")]
pub mod tabs {
    pub use crate::components::tabs::*;
}

static CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/singlestage.css"));

#[cfg(test)]
mod tests {
    use super::CSS;

    #[test]
    fn css_contains_brand_color_tokens() {
        assert!(CSS.contains("--color-primary-50"), "missing primary-50");
        assert!(CSS.contains("--color-primary-500"), "missing primary-500");
        assert!(CSS.contains("--color-primary-950"), "missing primary-950");
        assert!(CSS.contains("--color-secondary-500"), "missing secondary-500");
        assert!(CSS.contains("--color-success-500"), "missing success-500");
        assert!(CSS.contains("--color-danger-500"), "missing danger-500");
        assert!(CSS.contains("--color-warning-500"), "missing warning-500");
        assert!(CSS.contains("--color-info-500"), "missing info-500");
        assert!(CSS.contains("--color-accent-500"), "missing accent-500");
    }

    #[test]
    fn css_contains_neutral_scale() {
        assert!(CSS.contains("--color-neutral-50"), "missing neutral-50");
        assert!(CSS.contains("--color-neutral-500"), "missing neutral-500");
        assert!(CSS.contains("--color-neutral-950"), "missing neutral-950");
    }

    #[test]
    fn css_contains_font_tokens() {
        assert!(CSS.contains("--font-display"), "missing font-display");
        assert!(CSS.contains("--font-body"), "missing font-body");
    }

    #[test]
    fn css_contains_slate_brand() {
        assert!(CSS.contains("--color-slate-brand"), "missing slate-brand");
    }
}
