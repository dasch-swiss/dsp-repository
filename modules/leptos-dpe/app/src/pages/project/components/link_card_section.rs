use leptos::prelude::*;

/// A reusable section that renders a list of card-style links.
/// Each item is a tuple of (href, name, description).
#[component]
pub fn LinkCardSection(title: String, items: Vec<(String, String, String)>) -> impl IntoView {
    (!items.is_empty()).then(|| {
        view! {
            <div>
                <h3 class="text-sm font-semibold text-gray-700 mb-2">{title}</h3>
                <div class="flex flex-col gap-2">
                    {items
                        .into_iter()
                        .map(|(href, name, description)| {
                            view! {
                                <a
                                    href=href
                                    class="block bg-gray-50 border border-gray-200 rounded p-3 hover:border-primary-400 transition-colors"
                                >
                                    <div class="font-medium text-gray-900">{name}</div>
                                    <div class="text-sm text-gray-600 line-clamp-2">
                                        {description}
                                    </div>
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
