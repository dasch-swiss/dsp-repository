pub mod app;
pub mod components;

// Generate component documentation pages from TOML
demo_macro::generate_component_pages!();

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
