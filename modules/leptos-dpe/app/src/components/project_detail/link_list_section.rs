use leptos::prelude::*;

#[component]
pub fn LinkListSection(title: String, items: Vec<String>, #[prop(default = false)] as_links: bool) -> impl IntoView {
    view! {
        <div class="bg-base-100 p-6 rounded-lg">
            <h3 class="text-xl font-bold mb-3">{title}</h3>
            <ul class="list-disc list-inside">
                {items
                    .iter()
                    .map(|item| {
                        if as_links {
                            view! {
                                <li>
                                    <a
                                        href=item.clone()
                                        class="link link-primary"
                                    >
                                        {item.clone()}
                                    </a>
                                </li>
                            }
                                .into_any()
                        } else {
                            view! { <li>{item.clone()}</li> }.into_any()
                        }
                    })
                    .collect_view()}
            </ul>
        </div>
    }
}
