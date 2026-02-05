mod components;
mod reactive;

pub use components::theme_provider::ThemeProvider;
pub use components::*;
pub use reactive::Reactive;

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
