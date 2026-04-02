use leptos::prelude::*;

/// A reusable section that renders a list of card-style links or plain cards.
/// Each item is a tuple of (href, name, description).
#[component]
pub fn LinkCardSection(
    title: String,
    items: Vec<(String, String, String)>,
    #[prop(default = true)] clickable: bool,
) -> impl IntoView {
    (!items.is_empty()).then(|| {
        view! {
            <div>
                <h3 class="dpe-subtitle">{title}</h3>
                <div class="flex flex-col gap-2">
                    {items
                        .into_iter()
                        .map(|(href, name, description)| {
                            if clickable {
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
                                    .into_any()
                            } else {
                                view! {
                                    <div class="block bg-gray-50 border border-gray-200 rounded p-3">
                                        <div class="font-medium text-gray-900">{name}</div>
                                        <div class="text-sm text-gray-600 line-clamp-2">
                                            {description}
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
