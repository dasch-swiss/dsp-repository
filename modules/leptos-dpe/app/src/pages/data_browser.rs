use leptos::prelude::*;

#[component]
pub fn DataBrowserPage() -> impl IntoView {
    view! {
        <h1 class="text-2xl mb-4">Data Browser</h1>
        <DataBrowserContainer />
    }
}

#[island]
fn DataBrowserContainer() -> impl IntoView {
    let (selected_element, set_selected_element) = signal(None::<usize>);

    let elements: Vec<String> = (1..=100).map(|i| format!("element {}", i)).collect();

    let handle_element_click = move |id: usize| {
        set_selected_element.set(Some(id));
        // Update URL without navigation
        if let Some(window) = web_sys::window() {
            if let Ok(history) = window.history() {
                let url = format!("/data-browser/element-{}", id);
                let _ = history.push_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some(&url)
                );
            }
        }
    };

    // Initialize from URL on mount
    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(pathname) = location.pathname() {
                if let Some(id_str) = pathname.strip_prefix("/data-browser/element-") {
                    if let Ok(id) = id_str.parse::<usize>() {
                        set_selected_element.set(Some(id));
                    }
                }
            }
        }
    });

    // Listen for browser back/forward button events
    Effect::new(move |_| {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        if let Some(window) = web_sys::window() {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::PopStateEvent| {
                if let Some(window) = web_sys::window() {
                    let location = window.location();
                    if let Ok(pathname) = location.pathname() {
                        if let Some(id_str) = pathname.strip_prefix("/data-browser/element-") {
                            if let Ok(id) = id_str.parse::<usize>() {
                                set_selected_element.set(Some(id));
                            }
                        } else if pathname == "/data-browser" || pathname == "/data-browser/" {
                            set_selected_element.set(None);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let _ = window.add_event_listener_with_callback(
                "popstate",
                closure.as_ref().unchecked_ref()
            );

            // Cleanup when effect is destroyed
            closure.forget();
        }
    });

    view! {
        <div class="flex">
            <div style="height: 70vh; overflow: auto">
                {elements.into_iter().enumerate().map(|(i, element)| {
                    let id = i + 1;
                    view!{
                        <div
                            on:click=move |_| handle_element_click(id)
                            class="cursor-pointer hover:bg-gray-100 p-2"
                        >
                            {element}
                        </div>
                    }
                }).collect_view()}
            </div>

            {move || selected_element.get().map(|id| view! {
                <div class="p-4 flex-1" style="background-color: #aae5ea;">
                    <h2>"data element " {id}</h2>
                </div>
            })}
        </div>
    }
}
