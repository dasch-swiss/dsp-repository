mod components;

pub use components::theme_provider::ThemeProvider;
pub use components::*;

#[cfg(feature = "icon")]
pub mod icon {
    pub use crate::components::icon::*;
}

static CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/singlestage.css"));
