#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // Import app to ensure islands are compiled into WASM
    #[allow(unused_imports)]
    use app::*;

    // initializes logging using the `log` crate
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_islands();
}
