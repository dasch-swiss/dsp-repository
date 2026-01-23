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

#[cfg(feature = "sidebar")]
pub mod sidebar {
    pub use crate::components::sidebar::*;
}

#[cfg(feature = "tabs")]
pub mod tabs {
    pub use crate::components::tabs::*;
}

static CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/singlestage.css"));
