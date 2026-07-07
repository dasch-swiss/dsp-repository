use leptos::prelude::*;

// See `super` module docs for why this is a sync lookup with a wasm32 stub.

/// Renders an organization name from the in-process org cache.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn OrganizationName(organization_id: String) -> impl IntoView {
    match dpe_core::load_organization(&organization_id) {
        Some(org) => view! { <span class="font-semibold">{org.name}</span> }.into_any(),
        None => view! { <span class="italic text-base-content/70">"Organization not found"</span> }.into_any(),
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn OrganizationName(organization_id: String) -> impl IntoView {
    let _ = organization_id;
}
