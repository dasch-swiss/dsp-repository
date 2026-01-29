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

    view! {
        <div class="flex">
            <div style="height: 70vh; overflow: auto">
                {elements.into_iter().enumerate().map(|(i, element)| {
                    let id = i + 1;
                    view!{
                        <div
                            on:click=move |_| {
                                set_selected_element.set(Some(id));
                            }
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
