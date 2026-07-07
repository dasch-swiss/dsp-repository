use leptos::prelude::*;

/// Renders a placeholder value ("MISSING" or "CALCULATED") styled in red
/// when `DPE_SHOW_PLACEHOLDER_VALUES` is true. Otherwise renders nothing.
#[component]
pub fn PlaceholderValue(value: String) -> impl IntoView {
    let show = show_placeholder_values_ssr();

    show.then(|| {
        view! { <span class="text-danger-600 font-mono text-xs">{value}</span> }
    })
}

/// Returns true if the value should be rendered — either it is not a placeholder,
/// or placeholders are currently visible.
pub fn should_render_value(value: &str) -> bool {
    if !dpe_core::is_placeholder(value) {
        return true;
    }
    show_placeholder_values_ssr()
}

/// SSR-safe wrapper: returns the flag on the server, always false on WASM.
fn show_placeholder_values_ssr() -> bool {
    #[cfg(feature = "ssr")]
    {
        dpe_core::show_placeholder_values()
    }
    #[cfg(not(feature = "ssr"))]
    {
        false
    }
}
