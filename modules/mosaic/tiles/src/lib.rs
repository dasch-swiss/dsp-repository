mod components;

pub use components::theme_provider::ThemeProvider;
pub use components::*;

static CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/singlestage.css"));
