use leptos::leptos_dom::log;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use web_sys::{MouseEvent};

#[component]
pub fn DataBrowserPage() -> impl IntoView {
    let params = use_params_map();
    let selected_id = move || params.read().get("id");

    let elements: Vec<String> = (1..=100).map(|i| format!("element {}", i)).collect();

    view! {
        <h1 class="text-2xl mb-4">Data Browser</h1>
        <div class="flex">
            <DataBrowserList elements=elements />

            {move || selected_id().map(|id| view! {
                <div class="p-4 flex-1" style="background-color: #aae5ea;">
                    <h2>"Resource view for " {id}</h2>
                </div>
            })}
        </div>
    }
}

#[island]
fn DataBrowserList(elements: Vec<String>) -> impl IntoView {
    let navigate_to = move |url: String| {
        log!("navigate to {}", url);
        if let Some(window) = web_sys::window() {
            if let Some(history) = window.history().ok() {
                let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
                // Trigger a popstate event to update the UI
                if let Some(event) = web_sys::PopStateEvent::new("popstate").ok() {
                    let _ = window.dispatch_event(&event);
                }
            }
        }
    };

    view! {
        <div style="height: 70vh; overflow: auto">
            {elements.into_iter().enumerate().map(|(i, element)| {
                let id = i + 1;
                let url = format!("/data-browser/element-{}", id);
                view!{
                    <div
                        on:click=move |e: MouseEvent| {
                            e.prevent_default();
                            navigate_to(url.clone());
                        }
                    >
                        <div class="cursor-pointer hover:bg-gray-100 p-2">
                            {element}
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
